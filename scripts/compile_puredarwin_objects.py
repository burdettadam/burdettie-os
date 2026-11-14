#!/usr/bin/env python3
"""Compile source-owned Mach-O objects with an already-realized SDK/compiler.

Default: offline preflight. --compile writes only to a new private directory.
Never installs, fetches, builds a dependency, links, or executes a guest.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

import build_puredarwin_fixtures as base


def resolve_inputs(source):
    if base.shutil.which("nix") is None:
        raise base.BuildError("Nix is unavailable; no inputs were installed")
    if os.environ.get("NIX_REMOTE", "") not in ("", "local", "daemon"):
        raise base.BuildError("A remote Nix store is not supported")
    version = base.command(["nix", "--version"]).strip()
    base.require_nix_version(version)
    expression = ('s: let cs = builtins.filter (i: i ? cc) s.nativeBuildInputs; '
                  'c = assert builtins.length cs == 1; (builtins.head cs).cc; '
                  'id = p: { out = p.outPath; drv = p.drvPath; }; '
                  'in { sdk = id s; compiler = id c; }')
    records = json.loads(base.command([
        "nix", "--extra-experimental-features", "nix-command flakes", "eval", "--offline",
        "--no-write-lock-file", "--no-update-lock-file", "--option",
        "allow-import-from-derivation", "false", "--json",
        base.source_flake_url(source) + "#packages.x86_64-linux.apple-sdk", "--apply", expression]))
    if set(records) != {"sdk", "compiler"} or any(not Path(p["out"]).is_dir() for p in records.values()):
        raise base.BuildError("Pinned SDK/compiler outputs are not realized; no build was started")
    return version, records


def inspect_object(path):
    with path.open("rb") as stream:
        metadata = base.parse_macho(stream, path.stat().st_size)
    if (metadata["format"] != "thin" or metadata["slices"][0]["architecture"] != "x86_64"
            or metadata["slices"][0]["file_type"] != 1):
        raise base.BuildError("Expected a thin x86_64 MH_OBJECT, not an executable")
    return {"sha256": base.sha256(path), "macho": metadata,
            "symbol_inspection": "not_run", "undefined_symbols": None}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--compile", action="store_true")
    parser.add_argument("--output-dir", type=Path, help="New private directory; required with --compile")
    parser.add_argument("--llvm-nm", type=Path, help="Optional already-installed LLVM nm; never acquired")
    args = parser.parse_args(argv)
    if args.compile and args.output_dir is None:
        parser.error("--compile requires --output-dir")
    report = dict(schema_version=1, scope="object_compilation_only", status="blocked",
                  linking="not_run", guest_execution="not_run",
                  burdettie_runtime_execution="not_run", security_enforcement="not_run", llvm_nm=None)
    output = None
    try:
        if base.platform.system() != "Linux" or base.platform.machine() != "x86_64":
            raise base.BuildError("This probe requires an x86_64 Linux host")
        source = args.puredarwin_source.resolve(strict=True)
        report["source"] = base.source_identity(source)
        report["nix_version"], report["inputs"] = resolve_inputs(source)
        compiler = Path(report["inputs"]["compiler"]["out"]) / "bin/clang"
        sdk = Path(report["inputs"]["sdk"]["out"]) / "Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
        report["compiler_version"] = base.command([str(compiler), "--version"]).strip()
        if not re.search(r"clang version 21\.", report["compiler_version"]):
            raise base.BuildError("Selected SDK compiler is not the expected LLVM 21")
        report["compiler_sha256"] = base.sha256(compiler)
        report["sdk_files_sha256"] = {p: base.sha256(sdk / p) for p in (
            "usr/include/stdio.h", "usr/include/pthread.h", "usr/lib/libSystem.tbd")}
        report["recipe_sha256"] = base.sha256(Path(__file__))
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        if args.llvm_nm:
            args.llvm_nm = args.llvm_nm.absolute()
            version = base.command([str(args.llvm_nm), "--version"]).strip()
            if "LLVM" not in version:
                raise base.BuildError("--llvm-nm must identify an existing LLVM tool")
            report["llvm_nm"] = dict(path=str(args.llvm_nm), version=version, sha256=base.sha256(args.llvm_nm))
        if args.compile:
            candidate = args.output_dir.absolute()
            if candidate.exists() or not candidate.parent.is_dir():
                raise base.BuildError("Output must be a new directory with an existing parent")
            candidate.mkdir(mode=0o700)
            output = candidate
            report.update(status="fail", artifacts={}, commands={})
            env = os.environ.copy()
            for key in ("SDKROOT", "MACOSX_DEPLOYMENT_TARGET", "CPATH", "C_INCLUDE_PATH",
                        "LIBRARY_PATH", "CFLAGS", "CPPFLAGS", "LDFLAGS"):
                env.pop(key, None)
            env["LC_ALL"] = "C"
            for name, relative in base.FIXTURES.items():
                snapshot, target = output / (name + ".c"), output / (name + ".o")
                snapshot.write_bytes((base.ROOT / relative).read_bytes())
                command = [str(compiler), "--target=x86_64-apple-macosx26.5", "-isysroot", str(sdk),
                           "-std=c11", "-O0", "-g0", "-Wall", "-Wextra", "-Werror",
                           "-U_FORTIFY_SOURCE", "-D_FORTIFY_SOURCE=0", "-c", str(snapshot), "-o", str(target)]
                report["commands"][name] = command
                base.command(command, env=env)
                artifact = inspect_object(target)
                artifact.update(source_path=relative, source_sha256=base.sha256(snapshot))
                if args.llvm_nm:
                    artifact.update(symbol_inspection="collected", undefined_symbols=base.command(
                        [str(args.llvm_nm), "--undefined-only", str(target)]).splitlines())
                report["artifacts"][name] = artifact
            report["status"] = "objects_compiled_and_inspected"
        else:
            report["status"] = "ready_to_compile"
    except (base.BuildError, base.InventoryError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    encoded = json.dumps(report, indent=2) + "\n"
    if output is not None:
        (output / "result.json").write_text(encoded)
    print(encoded, end="")
    return 0 if report["status"] in ("ready_to_compile", "objects_compiled_and_inspected") else (
        1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    sys.exit(main())
