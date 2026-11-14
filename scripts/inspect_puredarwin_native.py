#!/usr/bin/env python3
"""Inspect an already-realized staged-TAPI native linker; never realize inputs.

Shared by the downstream planner and a separately reviewed native build driver.
The caller must validate the source/plan/receipts first and classify any failure
here as a failed live inspection. All evidence files require a new directory.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import stat
import struct
import subprocess

import plan_puredarwin_staged_linker as native


base = native.base
NIX = native.linker.NIX + ["--store", "local"]
BUILD_SCOPE = "staged_libtapi_native_linker_build"
BUILD_SUCCESS = "staged_libtapi_native_linker_built"
BUILD_PHASE = "staged-native-linker-build"
INSPECTION_SCOPE = "linux_x86_64_staged_libtapi_native_linker"
TAPI_VERSION = "1300.6.5"
OUTPUT_LIMIT = 65536


def zero(value):
    return type(value) is int and value == 0


def checked(argv, name, directory, report, *, timeout=30, env=None, bounded=False):
    """Retain each checked read/smoke result, including failure and timeout."""
    report.setdefault("commands", {})[name] = argv
    files = {suffix: directory / (name + "." + suffix) for suffix in ("stdout", "stderr")}
    if bounded:
        # Applied only to read-only loader/version probes, never to Nix. Their
        # exclusive stdout/stderr files cannot grow beyond the per-file limit.
        def output_bound():
            resource.setrlimit(resource.RLIMIT_FSIZE, (OUTPUT_LIMIT, OUTPUT_LIMIT))

        with files["stdout"].open("xb") as out, files["stderr"].open("xb") as err:
            try:
                process = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                         timeout=timeout, env=env, preexec_fn=output_bound, check=False)
                code, timed_out = process.returncode, False
            except subprocess.TimeoutExpired:
                code, timed_out = None, True
        stdout, stderr = (files[suffix].read_bytes() for suffix in ("stdout", "stderr"))
    else:
        try:
            process = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True,
                                     timeout=timeout, env=env, check=False)
            code, timed_out, stdout, stderr = process.returncode, False, process.stdout, process.stderr
        except subprocess.TimeoutExpired as error:
            code, timed_out = None, True
            stdout, stderr = error.stdout or b"", error.stderr or b""
    limit_reached = bounded and max(len(stdout), len(stderr)) >= OUTPUT_LIMIT
    record = {"argv": argv, "exit_code": code, "timed_out": timed_out,
              "output_limit_bytes": OUTPUT_LIMIT if bounded else None, "output_limit_reached": limit_reached}
    for suffix, content in (("stdout", stdout), ("stderr", stderr)):
        if not bounded:
            with files[suffix].open("xb") as stream:
                stream.write(content)
        record[suffix + "_sha256"] = hashlib.sha256(content).hexdigest()
    report.setdefault("read_checks", {})[name] = record
    native.require(zero(code) and timed_out is False and not limit_reached, name + " failed; inspect retained evidence")
    return stdout, record


def inventory(output):
    """Return all output files/links and require the pinned host-tool layout."""
    native.require(output.is_dir() and not output.is_symlink(), "Native output must be a real directory")
    files, links, elfs = {}, {}, {}
    for path in sorted(output.rglob("*")):
        name = path.relative_to(output).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISLNK(mode):
            try:
                resolved = path.resolve(strict=True)
            except RuntimeError as error:
                raise base.BuildError("Native output link cannot be resolved: " + name) from error
            native.require(resolved.is_relative_to(output), "Native output link escapes its package")
            links[name] = {"target": os.readlink(path), "resolved": str(resolved)}
        elif stat.S_ISREG(mode):
            files[name] = {"bytes": path.stat().st_size, "sha256": base.sha256(path),
                           "mode": stat.S_IMODE(mode)}
            with path.open("rb") as stream:
                header = stream.read(64)
            if header.startswith(b"\x7fELF"):
                native.require(len(header) == 64 and header[4:7] == b"\x02\x01\x01",
                               "Native ELF must be complete 64-bit little-endian version 1")
                kind, machine, version = struct.unpack_from("<HHI", header, 16)
                native.require(kind in (2, 3) and machine == 62 and version == 1
                               and struct.unpack_from("<H", header, 52)[0] == 64,
                               "Unexpected native ELF type, architecture or header")
                elfs[name] = {"class": 64, "byte_order": "little", "machine": "x86_64", "type": kind}
        else:
            native.require(stat.S_ISDIR(mode), "Unexpected native output file type")
    for name in ("bin/ld", "bin/ld.real", "bin/strip"):
        native.require(name in files and files[name]["mode"] & 0o111,
                       "Native host tool must be a regular executable: " + name)
    native.require("bin/ld.real" in elfs and "bin/strip" in elfs and "bin/ld" not in elfs,
                   "Expected a linker wrapper and ELF native linker/strip")
    with (output / "bin/ld").open("rb") as stream:
        wrapper = stream.read(8192)
    native.require(wrapper.startswith(b"#!/") and b"ld.real" in wrapper,
                   "Native ld wrapper does not name ld.real")
    return {"files": files, "symlinks": links, "elf_files": elfs}


def references(value, tapi_output, original_tapi_output):
    native.require(isinstance(value, list) and all(isinstance(item, str) for item in value)
                   and len(value) == len(set(value)), "Invalid native store-reference inventory")
    native.require(tapi_output in value and original_tapi_output not in value,
                   "Native output does not reference only the selected staged TAPI")
    return sorted(value)


def elf_dynamic(path):
    """Read bounded ELF64LE program headers/dynamic strings, without sections."""
    length = path.stat().st_size
    with path.open("rb") as stream:
        def read(offset, size, limit=1024 * 1024):
            native.require(0 <= offset <= length and 0 <= size <= min(limit, length - offset),
                           "ELF range is truncated or exceeds the inspection bound")
            stream.seek(offset)
            value = stream.read(size)
            native.require(len(value) == size, "ELF changed during inspection")
            return value

        header = read(0, 64)
        native.require(header[:7] == b"\x7fELF\x02\x01\x01", "Expected ELF64 little-endian version 1")
        kind, machine, version = struct.unpack_from("<HHI", header, 16)
        native.require(kind in (2, 3) and machine == 62 and version == 1
                       and struct.unpack_from("<H", header, 52)[0] == 64, "Unsupported native ELF header")
        phoff = struct.unpack_from("<Q", header, 32)[0]
        entry_size, count = struct.unpack_from("<HH", header, 54)
        native.require(entry_size == 56 and 1 <= count <= 128 and phoff >= 64,
                       "Unsupported ELF program-header table")
        table = read(phoff, entry_size * count)
        segments = [struct.unpack_from("<IIQQQQQQ", table, i * 56) for i in range(count)]
        loads = [item for item in segments if item[0] == 1]
        native.require(loads and all(item[5] <= item[6] and item[2] + item[5] <= length for item in loads),
                       "Invalid ELF load segment")

        def file_offset(address, size):
            matches = [item[2] + address - item[3] for item in loads
                       if item[3] <= address and 0 <= address - item[3] <= item[5]
                       and size <= item[5] - (address - item[3])]
            native.require(len(matches) == 1, "ELF virtual range is unmapped or ambiguous")
            return matches[0]

        dynamic = [item for item in segments if item[0] == 2]
        interpreters = [item for item in segments if item[0] == 3]
        native.require(len(dynamic) == 1 and len(interpreters) == 1, "Native linker needs one dynamic segment and interpreter")
        interpreter = read(interpreters[0][2], interpreters[0][5], 4096)
        native.require(interpreter.endswith(b"\0") and interpreter.count(b"\0") == 1,
                       "Invalid ELF interpreter string")
        interpreter = interpreter[:-1].decode("utf-8")
        native.require(interpreter.startswith("/"), "ELF interpreter must be absolute")
        segment = dynamic[0]
        native.require(segment[5] % 16 == 0 and 16 <= segment[5] <= 65536
                       and file_offset(segment[3], segment[5]) == segment[2], "Invalid dynamic segment mapping")
        data = read(segment[2], segment[5], 65536)
        tags = []
        for offset in range(0, len(data), 16):
            tag, value = struct.unpack_from("<qQ", data, offset)
            if tag == 0:
                break
            # Audit/filter tags can redirect symbol binding. They are outside
            # this narrow source-owned executable contract.
            native.require(tag not in (0x6FFFFEFC, 0x6FFFFEFB, 0x7FFFFFFD, 0x7FFFFFFF),
                           "ELF audit/filter indirection is unsupported")
            tags.append((tag, value))
        else:
            raise base.BuildError("ELF dynamic table lacks DT_NULL")

        def single(tag, required=False):
            values = [value for key, value in tags if key == tag]
            native.require(len(values) <= 1 and (not required or len(values) == 1), "Duplicate or missing dynamic tag")
            return values[0] if values else None

        address, size = single(5, True), single(10, True)
        native.require(0 < size <= 1024 * 1024, "ELF string table exceeds the inspection bound")
        strings = read(file_offset(address, size), size)

        def string(offset):
            native.require(0 <= offset < len(strings), "ELF string offset is outside DT_STRTAB")
            end = strings.find(b"\0", offset)
            native.require(end != -1, "ELF dynamic string is not terminated")
            value = strings[offset:end].decode("utf-8")
            native.require(value and all(ord(char) >= 32 for char in value), "Invalid ELF dynamic string")
            return value

        needed = [string(value) for key, value in tags if key == 1]
        native.require(1 <= len(needed) <= 256 and len(needed) == len(set(needed)), "Invalid DT_NEEDED inventory")
        rpath, runpath = single(15), single(29)
        return {"interpreter": interpreter, "needed": needed,
                "rpath": None if rpath is None else string(rpath),
                "runpath": None if runpath is None else string(runpath)}


def tapi_binding(path, tapi_output):
    """Resolve the direct TAPI dependency in effective ELF search order."""
    dynamic = elf_dynamic(path)
    names = [name for name in dynamic["needed"]
             if re.fullmatch(r"libtapi\.so(?:\.[A-Za-z0-9._+-]+)?", Path(name).name)]
    native.require(len(names) == 1, "Native linker must need exactly one libtapi shared library")
    needed = names[0]
    directories = []
    if "/" in needed:
        native.require(needed.startswith("/") and "$" not in needed and ".." not in Path(needed).parts,
                       "Direct TAPI dependency must be an absolute fixed path")
        candidates, mode = [Path(needed)], "absolute_DT_NEEDED"
    else:
        mode = "DT_RUNPATH" if dynamic["runpath"] is not None else "DT_RPATH"
        value = dynamic["runpath"] if mode == "DT_RUNPATH" else dynamic["rpath"]
        native.require(value is not None, "TAPI resolution cannot rely on the host cache or default directories")
        directories = value.split(":")
        native.require(all(item.startswith("/") and "$" not in item and ".." not in Path(item).parts
                           for item in directories), "Relative, empty or token-expanded ELF search paths are unsupported")
        candidates = [Path(directory) / needed for directory in directories]
    selected = next((item for item in candidates if item.exists() or item.is_symlink()), None)
    native.require(selected is not None, "DT_NEEDED TAPI library cannot be resolved")
    resolved = selected.resolve(strict=True)
    native.require(resolved.is_file() and resolved.is_relative_to(Path(tapi_output)),
                   "ELF search order resolves TAPI outside the verified staged package")
    return {"dynamic": dynamic, "needed": needed, "resolution_mode": mode, "search_directories": directories,
            "selected_path": str(selected), "resolved_path": str(resolved), "sha256": base.sha256(resolved),
            "bytes": resolved.stat().st_size}


def version_identity(stdout, stderr, tapi_output):
    """Check source-defined ld/TAPI labels; retain an unresolved ld64 token."""
    native.require(len(stdout) + len(stderr) <= 65536, "Native version output exceeds the inspection bound")
    header = Path(tapi_output) / "include/tapi/Version.inc"
    native.require(header.is_file() and not header.is_symlink() and header.stat().st_size <= 4096,
                   "Installed TAPI version header must be a small regular file")
    data = header.read_bytes()
    versions = re.findall(rb"^#define\s+TAPI_VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)\s*$", data, re.M)
    native.require(versions == [TAPI_VERSION.encode()], "Installed TAPI version differs from the pinned build script")
    text = stderr.decode("utf-8")
    banners = re.findall(r"^@\(#\)PROGRAM:ld  PROJECT:ld64-([A-Za-z0-9_.+-]+)$", text, re.M)
    api = re.findall(r"^TAPI support using: (.*TAPI version " + re.escape(TAPI_VERSION)
                     + r" \(" + re.escape(TAPI_VERSION) + r"\))$", text, re.M)
    arches = re.findall(r"^configured to support archs: (.+)$", text, re.M)
    native.require(len(banners) == len(api) == len(arches) == 1 and "x86_64" in arches[0].split(),
                   "Native smoke output lacks the source-defined ld64, x86_64 or pinned TAPI identity")
    return {"program": "ld", "project": "ld64", "version_token": banners[0], "tapi_version": TAPI_VERSION,
            "tapi_full_version": api[0], "version_header_sha256": hashlib.sha256(data).hexdigest()}


def loader_output(interpreter, refs):
    path = Path(interpreter)
    native.require(path.is_absolute() and ".." not in path.parts, "ELF interpreter must be an absolute fixed path")
    owners = [value for value in refs if path.is_relative_to(Path(value))]
    native.require(len(owners) == 1 and path.resolve(strict=True).is_relative_to(Path(owners[0]))
                   and path.is_file() and os.access(path, os.X_OK), "ELF interpreter is not inside one referenced registered output")
    return owners[0]


def observed_tapi_resolution(raw, binding):
    native.require(len(raw) < OUTPUT_LIMIT, "Loader output exceeds the inspection bound")
    matches = []
    for line in raw.decode("utf-8").splitlines():
        mapped = re.fullmatch(r"\s*(\S+)\s+=>\s+(/\S+)\s+\(0x[0-9a-fA-F]+\)", line)
        direct = re.fullmatch(r"\s*(/\S+)\s+\(0x[0-9a-fA-F]+\)", line)
        name, path = (mapped.group(1), mapped.group(2)) if mapped else ((direct.group(1), direct.group(1)) if direct else ("", ""))
        if Path(name).name == Path(binding["needed"]).name:
            matches.append(str(Path(path).resolve(strict=True)))
    native.require(matches == [binding["resolved_path"]], "Actual ELF loader did not resolve the inspected staged TAPI library")
    return {"needed": binding["needed"], "resolved_path": matches[0], "sha256": binding["sha256"]}


def inspect_native_output(candidate, dependencies, original_tapi_output, directory, report, *, expected=None):
    """Check registration/content, inventory and a bounded host smoke test.

    dependencies contains exact previously observed registered identities for
    libtapi and sdk. expected is the successful native inspection when consuming
    an existing receipt; a producer omits it after successful realization.
    No filesystem mutation occurs in any Nix output and no repair is requested.
    """
    output = Path(candidate["out"])
    native.require(set(dependencies) == {"libtapi", "sdk"}, "Both verified dependency identities are required")
    raw, _ = checked(NIX + ["path-info", "--offline", "--json", str(output)],
                     "native-path-info", directory, report)
    with (directory / "output-path-info.json").open("xb") as stream:
        stream.write(raw)
    metadata = json.loads(raw)
    native.require(set(metadata) == {str(output)}, "Native path-info must identify exactly the selected output")
    registered = native.registered_identity(metadata[str(output)], str(output))
    native.require(registered["deriver"] == candidate["drv"], "Native output has the wrong registered deriver")
    if expected is not None:
        native.require(registered == {key: expected["registered_output"][key] for key in registered},
                       "Native NAR identity differs from the successful inspection")
    registered["path_info_sha256"] = hashlib.sha256(raw).hexdigest()
    refs = references(metadata[str(output)]["references"], dependencies["libtapi"]["path"], original_tapi_output)
    if expected is not None:
        native.require(refs == expected["references"], "Native store references changed")
    contents = inventory(output)
    if expected is not None:
        native.require(all(contents[key] == expected[key] for key in contents),
                       "Native file, link or ELF inventory differs from the successful inspection")
    binding = tapi_binding(output / "bin/ld.real", dependencies["libtapi"]["path"])
    if expected is not None:
        native.require(binding == expected["tapi_dynamic_binding"], "Native dynamic TAPI binding changed")
    loader = loader_output(binding["dynamic"]["interpreter"], refs)
    dependency_paths = [dependencies[name]["path"] for name in ("libtapi", "sdk")] + [loader]
    raw, _ = checked(NIX + ["path-info", "--offline", "--json"] + dependency_paths,
                     "native-dependency-path-info", directory, report)
    current = json.loads(raw)
    native.require(set(current) == set(dependency_paths), "Dependency path-info selection changed")
    for name, prior in dependencies.items():
        observed = native.registered_identity(current[prior["path"]], prior["path"])
        native.require(observed == {key: prior[key] for key in observed}, "Current " + name + " NAR/deriver changed")
    loader_registered = native.registered_identity(current[loader], loader)
    if expected is not None:
        native.require(loader_registered == expected["loader_registered_output"], "Current ELF loader NAR/deriver changed")
    _, verification = checked(NIX + ["store", "verify", "--offline", str(output)] + dependency_paths,
                              "native-store-verify", directory, report, timeout=90)
    # Loader overrides could make a smoke test use a different host library.
    env = {"PATH": "/usr/bin:/bin", "LC_ALL": "C"}
    raw, loader_check = checked([binding["dynamic"]["interpreter"], "--list", str(output / "bin/ld.real")],
                                "native-loader-list", directory, report, timeout=10, env=env, bounded=True)
    loader_resolution = observed_tapi_resolution(raw, binding)
    if expected is not None:
        native.require(loader_resolution == expected["loader_resolution"], "Observed ELF loader resolution changed")
    stdout, smoke = checked([str(output / "bin/ld"), "-v"], "native-ld-version", directory,
                            report, timeout=10, env=env, bounded=True)
    version = version_identity(stdout, (directory / "native-ld-version.stderr").read_bytes(), dependencies["libtapi"]["path"])
    if expected is not None:
        native.require(smoke == expected["smoke_test"] and version == expected["version_identity"], "Native linker smoke result changed")
    native.require(inventory(output) == contents, "Native output changed during its smoke test")
    native.require(tapi_binding(output / "bin/ld.real", dependencies["libtapi"]["path"]) == binding,
                   "Native TAPI binding changed during its smoke test")
    return {"schema_version": 1, "scope": INSPECTION_SCOPE, "status": "pass", **contents,
            "registered_output": registered, "references": refs,
            "staged_libtapi_reference_present": True, "original_libtapi_reference_absent": True,
            "dependencies": dependencies, "store_verification": verification, "smoke_test": smoke,
            "tapi_dynamic_binding": binding, "version_identity": version,
            "loader_registered_output": loader_registered, "loader_check": loader_check,
            "loader_resolution": loader_resolution,
            "fixture_linking": "not_run", "guest_execution": "not_run"}
