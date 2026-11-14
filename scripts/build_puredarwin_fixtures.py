#!/usr/bin/env python3
"""Cross-build source-owned M1 fixtures using already-realized pinned Nix inputs.

Default: no-build input preflight. --build compiles two small C programs, never a
dependency, image or SDK. No package install, download, guest launch or runtime
execution is performed. Reports include private local paths; keep them local.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import sys

from collect_m1_inventory import InventoryError, command_string, parse_macho


PUREDARWIN_REVISION = "0163b2db225b85e3bf96fc25646ea290105ac298"
LOCK_SHA256 = "00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0"
NIXPKGS_REVISION = "fd1462031fdee08f65fd0b4c6b64e22239a77870"
PACKAGES = ("apple-sdk", "darwin-cross-toolchain", "native-ld", "libsystem")
TRIPLE = "x86_64-apple-darwin20.4"
DEPLOYMENT = "26.5"
ROOT = Path(__file__).resolve().parents[1]
FIXTURES = {"simple_cli": "fixtures/simple_cli/main.c",
            "m1_abi_probe": "fixtures/m1_abi_probe/probe.c"}


class BuildError(ValueError):
    pass


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def command(argv, *, env=None, timeout=60):
    result = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True,
                            text=True, env=env, timeout=timeout, check=False)
    if result.returncode:
        raise BuildError(f"{Path(argv[0]).name} failed ({result.returncode}): "
                         + result.stderr[-4000:])
    return result.stdout


def lock_identity(raw):
    if hashlib.sha256(raw).hexdigest() != LOCK_SHA256:
        raise BuildError("PureDarwin flake.lock differs from the selected revision")
    lock = json.loads(raw)
    # nodes.nixpkgs belongs to another input; follow the root's input reference.
    node = lock["nodes"][lock["root"]]["inputs"]["nixpkgs"]
    revision = lock["nodes"][node]["locked"]["rev"]
    if revision != NIXPKGS_REVISION:
        raise BuildError("Unexpected root nixpkgs input")
    return {"sha256": LOCK_SHA256, "nixpkgs_node": node, "nixpkgs_revision": revision}


def source_identity(source):
    git = ["git", "-C", str(source)]
    revision = command(git + ["rev-parse", "HEAD"]).strip()
    if revision != PUREDARWIN_REVISION:
        raise BuildError("PureDarwin checkout is not at the selected revision")
    if command(git + ["status", "--porcelain", "--untracked-files=no"]).strip():
        raise BuildError("PureDarwin tracked files or submodules have local changes")
    identity = {"revision": revision, "lock": lock_identity((source / "flake.lock").read_bytes())}
    submodules = command(git + ["submodule", "status", "--recursive"])
    if not submodules.strip() or any(line[0] != " " for line in submodules.splitlines()):
        raise BuildError("Pinned PureDarwin submodules are missing or at a different revision")
    identity["submodules"] = [line.strip().split()[:2] for line in submodules.splitlines()]
    return identity


def resolve_packages(source):
    if shutil.which("nix") is None:
        raise BuildError("Nix is unavailable; no inputs were installed or fetched")
    if os.environ.get("NIX_REMOTE", "") not in ("", "local", "daemon"):
        raise BuildError("A remote Nix store is not supported by this local preflight")
    flake = "git+" + source.as_uri() + f"?rev={PUREDARWIN_REVISION}&submodules=1"
    expression = "p: builtins.mapAttrs (n: v: { out = v.outPath; drv = v.drvPath; }) { inherit (p) "
    expression += " ".join(PACKAGES) + "; }"
    raw = command(["nix", "--extra-experimental-features", "nix-command flakes", "eval",
                   "--offline", "--no-write-lock-file", "--no-update-lock-file",
                   "--option", "allow-import-from-derivation", "false", "--json",
                   flake + "#packages.x86_64-linux", "--apply", expression])
    packages = json.loads(raw)
    if set(packages) != set(PACKAGES):
        raise BuildError("Unexpected Nix package selection")
    for name, record in packages.items():
        if not Path(record["out"]).is_dir():
            raise BuildError(f"Pinned {name} output is not realized; no dependency build was started")
    return packages


def require_nix_version(version):
    match = re.search(r"\b(\d+)\.(\d+)(?:\.\d+)?(?:\S*)$", version.strip())
    if match is None or tuple(map(int, match.groups())) < (2, 27):
        raise BuildError("Nix 2.27 or later is required for the pinned self.submodules and relative-path inputs")


def tool_paths(packages):
    outputs = {name: Path(record["out"]) for name, record in packages.items()}
    paths = {
        "compiler": outputs["darwin-cross-toolchain"] / "bin" / (TRIPLE + "-clang"),
        "nm": outputs["darwin-cross-toolchain"] / "bin" / (TRIPLE + "-nm"),
        "linker": outputs["native-ld"] / "bin/ld",
        "sdk": outputs["apple-sdk"] / "Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
        "libsystem": outputs["libsystem"],
    }
    required = [paths["sdk"] / "usr/include/stdio.h", paths["sdk"] / "usr/include/pthread.h",
                paths["libsystem"] / "usr/lib/libSystem.B.dylib",
                paths["libsystem"] / "usr/lib/system/libdyld.dylib"]
    if any(not path.is_file() for path in required):
        raise BuildError("Selected SDK headers or source-built libSystem/dyld are unavailable")
    if any(not os.access(paths[name], os.X_OK) for name in ("compiler", "linker", "nm")):
        raise BuildError("Pinned compiler/linker/inspection tools are not executable")
    return paths


def compile_command(paths, source, output):
    system = paths["libsystem"]
    return [str(paths["compiler"]), "-arch", "x86_64", "-std=c11", "-O0", "-g0",
            "-Wall", "-Wextra", "-Werror", "-U_FORTIFY_SOURCE", "-D_FORTIFY_SOURCE=0",
            "-I" + str(system / "usr/include"), "-fuse-ld=" + str(paths["linker"]),
            "-nostdlib", "-Wl,-Z", "-L" + str(system / "usr/lib"),
            "-Wl,-dylib_file,/usr/lib/system/libdyld.dylib:" + str(system / "usr/lib/system/libdyld.dylib"),
            "-Wl,-dylinker_install_name,/usr/lib/dyld",
            f"-Wl,-platform_version,macos,{DEPLOYMENT},{DEPLOYMENT}",
            str(source), "-o", str(output), "-lSystem"]


def inspect_fixture(path):
    with path.open("rb") as stream:
        metadata = parse_macho(stream, path.stat().st_size)
        if metadata["format"] != "thin" or metadata["slices"][0]["architecture"] != "x86_64":
            raise BuildError("Fixture must be a thin x86_64 Mach-O executable")
        info = metadata["slices"][0]
        if info["file_type"] != 2 or info["encrypted"]:
            raise BuildError("Fixture must be an unencrypted MH_EXECUTE")
        if info["rpaths"]:
            raise BuildError("Unexpected runtime library search path")
        if [(x["name"], x["load_kind"]) for x in info["dependencies"]] != [
                ("/usr/lib/libSystem.B.dylib", "required")]:
            raise BuildError("Unexpected direct dependency; inspect the link inputs")
        expected = {"command": "LC_BUILD_VERSION", "platform_id": 1,
                    "minimum_os": "26.5.0", "sdk": "26.5.0"}
        if info["platform_versions"] != [expected]:
            raise BuildError("Fixture deployment/SDK commands differ from the pinned target")
        stream.seek(0)
        header = stream.read(32)
        if header[:4] != b"\xcf\xfa\xed\xfe":
            raise BuildError("Expected little-endian 64-bit Mach-O")
        count, size = struct.unpack_from("<II", header, 16)
        commands = stream.read(size)
    offset, dylinkers, entries = 0, [], 0
    for _ in range(count):
        kind, size = struct.unpack_from("<II", commands, offset)
        data = commands[offset:offset + size]
        if kind == 0xE:  # LC_LOAD_DYLINKER is not collected by the app inventory.
            if size < 12:
                raise BuildError("Truncated dynamic-linker command")
            dylinkers.append(command_string(data, struct.unpack_from("<I", data, 8)[0], 12))
        if kind == 0x80000028:
            if size != 24:
                raise BuildError("Invalid LC_MAIN command")
            entries += 1
        offset += size
    if dylinkers != ["/usr/lib/dyld"] or entries != 1:
        raise BuildError("Expected exactly one /usr/lib/dyld command and LC_MAIN")
    return {"sha256": sha256(path), "macho": metadata, "dylinker": dylinkers[0],
            "guest_execution": "not_run", "burdettie_runtime_execution": "not_run"}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--build", action="store_true", help="Compile only with existing pinned inputs")
    parser.add_argument("--output-dir", type=Path, help="New private directory; required with --build")
    args = parser.parse_args(argv)
    if args.build and args.output_dir is None:
        parser.error("--build requires --output-dir")
    report = {"schema_version": 1, "scope": "cross_build_and_static_inspection_only",
              "status": "blocked", "guest_execution": "not_run",
              "burdettie_runtime_execution": "not_run", "security_enforcement": "not_run"}
    output = None
    try:
        if platform.system() != "Linux" or platform.machine() != "x86_64":
            raise BuildError("This recipe qualifies only an x86_64 Linux build host")
        source = args.puredarwin_source.resolve(strict=True)
        report["source"] = source_identity(source)
        if shutil.which("nix") is None:
            raise BuildError("Nix is unavailable; no inputs were installed or fetched")
        report["nix_version"] = command(["nix", "--version"]).strip()
        require_nix_version(report["nix_version"])
        report["packages"] = packages = resolve_packages(source)
        paths = tool_paths(packages)
        report["recipe_sha256"] = sha256(Path(__file__))
        report["burdettie_revision"] = command(["git", "-C", str(ROOT), "rev-parse", "HEAD"]).strip()
        report["compiler_version"] = command([str(paths["compiler"]), "--version"]).strip()
        report["input_sha256"] = {name: sha256(paths[name]) for name in ("compiler", "linker")}
        report["input_sha256"]["libSystem.B.dylib"] = sha256(paths["libsystem"] / "usr/lib/libSystem.B.dylib")
        report["input_sha256"]["libdyld.dylib"] = sha256(paths["libsystem"] / "usr/lib/system/libdyld.dylib")
        if args.build:
            candidate = args.output_dir.absolute()
            if candidate.exists() or not candidate.parent.is_dir():
                raise BuildError("Output must be a new directory with an existing parent")
            candidate.mkdir(mode=0o700)
            output = candidate
            report["status"] = "fail"
            report["artifacts"] = {}
            report["commands"] = {}
            env = os.environ.copy()
            for key in ("SDKROOT", "MACOSX_DEPLOYMENT_TARGET", "CPATH", "C_INCLUDE_PATH",
                        "LIBRARY_PATH", "CFLAGS", "CPPFLAGS", "LDFLAGS"):
                env.pop(key, None)
            env.update({"DARWIN_SDK_ROOT": str(paths["sdk"]), "LC_ALL": "C"})
            for name, relative in FIXTURES.items():
                fixture = ROOT / relative
                target = output / name
                # Freeze the small source input so a concurrent edit cannot
                # make the report hash describe different bytes from the build.
                snapshot = output / (name + ".c")
                snapshot.write_bytes(fixture.read_bytes())
                source_digest = sha256(snapshot)
                report["commands"][name] = compile_command(paths, snapshot, target)
                command(report["commands"][name], env=env)
                artifact = inspect_fixture(target)
                artifact["source_path"] = relative
                artifact["source_sha256"] = source_digest
                artifact["undefined_symbols"] = command([str(paths["nm"]), "-u", str(target)]).splitlines()
                report["artifacts"][name] = artifact
            report["status"] = "built_and_statically_checked"
        else:
            report["status"] = "ready_to_build"
    except (BuildError, InventoryError, OSError, ValueError, KeyError,
            subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    encoded = json.dumps(report, indent=2) + "\n"
    if output is not None and output.is_dir():
        (output / "result.json").write_text(encoded)
    print(encoded, end="")
    return 0 if report["status"] in ("ready_to_build", "built_and_statically_checked") else (
        1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    raise SystemExit(main())
