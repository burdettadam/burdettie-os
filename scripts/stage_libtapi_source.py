#!/usr/bin/env python3
"""Stage the complete pinned libtapi source without copying unchanged file data.

This is used only by the opt-in libtapi unpack recipe. Every source directory
remains a real directory, original symlinks retain their text, and regular files
are linked to the immutable input unless patching or header installation needs
a private copy. It never resumes or changes an earlier build directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat


GNU_CONFIG_FILES = frozenset({"src/llvm/cmake/config.guess"})
COPY_FILES = GNU_CONFIG_FILES | frozenset({"build.sh", "install.sh",
                       "src/llvm/include/llvm/TextAPI/MachO/Architecture.def"})
COPY_TREES = ("tools", "src/tapi/include")


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def copied(path):
    return path in COPY_FILES or any(path.startswith(tree + "/") for tree in COPY_TREES)


def inspect_source(source, *, block_size=None):
    """Read and hash the full input; do not create staging or realize anything."""
    source = source.resolve(strict=True)
    if not source.is_dir():
        raise ValueError("Source must be an existing directory")
    for name in COPY_FILES | frozenset(COPY_TREES):
        mode = (source / name).lstat().st_mode
        expected = stat.S_ISDIR if name in COPY_TREES else stat.S_ISREG
        if not expected(mode):
            raise ValueError("Required writable input has an unexpected type: " + name)
    block_size = block_size or os.statvfs(source).f_frsize
    entries, seen_inodes = [], set()
    counts = {key: 0 for key in ("directories", "regular_files", "original_symlinks",
                                "copied_regular_files", "linked_regular_files")}
    allocated = logical = copied_bytes = estimate = 0

    def add(path):
        nonlocal allocated, logical, copied_bytes, estimate
        info = path.lstat()
        name = path.relative_to(source).as_posix()
        entry = {"path": name, "mode": stat.S_IMODE(info.st_mode)}
        inode = (info.st_dev, info.st_ino)
        if inode not in seen_inodes:
            allocated += info.st_blocks * 512
            seen_inodes.add(inode)
        if stat.S_ISDIR(info.st_mode):
            entry["kind"] = "directory"
            counts["directories"] += 1
            estimate += max(block_size, info.st_blocks * 512)
        elif stat.S_ISLNK(info.st_mode):
            entry.update(kind="original_symlink", target=os.readlink(path))
            counts["original_symlinks"] += 1
            estimate += max(1, (len(os.fsencode(entry["target"])) + block_size - 1) // block_size) * block_size
        elif stat.S_ISREG(info.st_mode):
            copy = copied(name)
            entry.update(kind="regular_file", bytes=info.st_size, sha256=digest(path),
                         staging="copy" if copy else "immutable_file_symlink")
            counts["regular_files"] += 1
            counts["copied_regular_files" if copy else "linked_regular_files"] += 1
            logical += info.st_size
            if copy:
                copied_bytes += info.st_size
                estimate += ((info.st_size + block_size - 1) // block_size) * block_size
            else:
                estimate += max(1, (len(os.fsencode(path)) + block_size - 1) // block_size) * block_size
        else:
            raise ValueError("Unsupported source entry type: " + name)
        entries.append(entry)

    add(source)
    for directory, directories, files in os.walk(source, followlinks=False):
        directories.sort()
        for name in sorted(directories + files):
            add(Path(directory) / name)
    entries.sort(key=lambda entry: entry["path"])
    config_scripts = {entry["path"] for entry in entries
                      if Path(entry["path"]).name in ("config.guess", "config.sub")}
    if config_scripts != GNU_CONFIG_FILES:
        raise ValueError("GNU config hook inputs differ from the pinned source")
    return {
        "schema_version": 1, "source_directory": str(source), "entries": entries,
        "source_inventory_sha256": hashlib.sha256(encoded(entries)).hexdigest(),
        "summary": {**counts, "gnu_config_scripts": sorted(config_scripts),
                    "source_allocated_bytes": allocated,
                    "logical_file_bytes": logical, "copied_file_bytes": copied_bytes,
                    "filesystem_block_bytes": block_size,
                    "staging_allocation_estimate_bytes": estimate,
                    "estimate_scope": "staging only; excludes build, output and additional filesystem metadata",
                    "estimate_method": "source directory blocks, rounded copies, at least one block per symlink",
                    "peak_space_qualified": False},
    }


def stage(source, destination, manifest_path):
    source = source.resolve(strict=True)
    destination = destination.absolute()
    manifest_path = manifest_path.absolute()
    for path in (destination, manifest_path):
        if path.exists() or path.is_symlink():
            raise ValueError("Refusing an existing output: " + str(path))
        if path.resolve().is_relative_to(source):
            raise ValueError("Outputs must be outside the immutable source")
        if not path.parent.is_dir():
            raise ValueError("Output parent must already exist: " + str(path.parent))
    if destination == manifest_path or manifest_path.is_relative_to(destination):
        raise ValueError("Manifest must be outside the staged source")
    manifest = inspect_source(source, block_size=os.statvfs(destination.parent).f_frsize)
    destination.mkdir(mode=0o700)
    for entry in manifest["entries"]:
        original, target = source / entry["path"], destination / entry["path"]
        if entry["kind"] == "directory":
            if entry["path"] != ".":
                target.mkdir()
            target.chmod(entry["mode"] | stat.S_IRWXU)
        elif entry["kind"] == "original_symlink":
            target.symlink_to(entry["target"])
        elif entry["staging"] == "copy":
            # Never hardlink a file that a subsequent patch or hook can modify.
            with original.open("rb") as reader, target.open("xb") as writer:
                shutil.copyfileobj(reader, writer)
            shutil.copystat(original, target, follow_symlinks=False)
            target.chmod(entry["mode"] | stat.S_IWUSR)
            if digest(target) != entry["sha256"]:
                raise ValueError("Source changed while copying: " + entry["path"])
        else:
            target.symlink_to(original)
    after = inspect_source(source, block_size=manifest["summary"]["filesystem_block_bytes"])
    if after["source_inventory_sha256"] != manifest["source_inventory_sha256"]:
        raise ValueError("Immutable source changed during staging")
    manifest.update(status="staged", source_unchanged=True,
                    staged_allocated_bytes=sum((destination / e["path"]).lstat().st_blocks * 512
                                               for e in manifest["entries"]))
    payload = encoded(manifest)
    with manifest_path.open("xb") as output:
        output.write(payload)
    # The full manifest stays in this new build directory. Retain this compact
    # receipt in Nix's build log even when successful-build cleanup removes it.
    return {key: manifest[key] for key in ("status", "source_inventory_sha256", "source_unchanged",
                                           "staged_allocated_bytes", "summary")} | {
        "manifest_sha256": hashlib.sha256(payload).hexdigest(),
        "manifest_file": manifest_path.name,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args(argv)
    result = stage(args.source, args.destination, args.manifest)
    print("BURDETTIE_LIBTAPI_STAGING " + json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
