#!/usr/bin/env python3
"""Plan a native linker explicitly bound to a successfully built staged libtapi.

Requires exact retained plan/build receipts and fresh registered-output checks.
Never builds, repairs, registers, downloads dependencies, or executes the library.
--query-substituters permits only the final dry-run's cache metadata queries.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

import build_puredarwin_fixtures as base
import plan_puredarwin_linker as linker
import plan_puredarwin_libtapi as tapi


RECIPE = Path(__file__).with_name("puredarwin_staged_linker.nix")
UNCHANGED_FIELDS = frozenset({"source", "native_build_inputs", "bootstrap_toolchain", "install_phase",
                              "other_configure_flags", "original_build_before_job_bound", "other_build_inputs", "sdk"})
HEADERS = ("APIVersion.h", "Defines.h", "LinkerInterfaceFile.h", "PackedVersion32.h",
           "Symbol.h", "tapi.h", "Version.h", "Version.inc")


def require(condition, message):
    if not condition:
        raise base.BuildError(message)


def identity(record):
    return {key: record[key] for key in ("drv", "out")}


def read_json(path, expected_hash):
    require(re.fullmatch(r"[0-9a-f]{64}", expected_hash) is not None, "Invalid expected receipt checksum")
    with path.open("rb") as stream:
        raw = stream.read(16 * 1024 * 1024 + 1)
    require(len(raw) <= 16 * 1024 * 1024, "Receipt exceeds the bounded read size")
    require(hashlib.sha256(raw).hexdigest() == expected_hash, "Retained receipt checksum mismatch: " + path.name)

    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "Duplicate JSON field in retained evidence")
            result[key] = value
        return result

    value = json.loads(raw.decode("utf-8"), object_pairs_hook=unique)
    require(isinstance(value, dict), "Retained receipt must be a JSON object")
    return value, raw


def registered_identity(entry, path):
    value = {"path": path, "deriver": entry["deriver"], "nar_hash": entry["narHash"], "nar_size": entry["narSize"]}
    require(isinstance(value["nar_hash"], str) and value["nar_hash"].startswith("sha256-"), "Expected a SHA256 NAR identity")
    encoded = value["nar_hash"][7:]
    decoded = base64.b64decode(encoded, validate=True)
    require(len(decoded) == 32 and base64.b64encode(decoded).decode() == encoded, "Invalid SHA256 NAR identity")
    require(type(value["nar_size"]) is int and value["nar_size"] > 0, "Invalid NAR size")
    return value


def validate_receipts(plan, built, plan_hash, source, sdk, hashes):
    require(plan["schema_version"] == 1 and plan["scope"] == "libtapi_source_staging_plan_only"
            and plan["status"] == "planned_not_built" and plan["dry_run"]["exit_code"] == 0
            and plan["dry_run"].get("timed_out") is False
            and plan["planning_mode"] == "substituter_metadata" and plan["substituter_queries_enabled"] is True,
            "A successful inspected libtapi plan is required")
    require(plan["source"] == source and identity(plan["sdk"]) == sdk
            and plan["sdk"]["identity_matches_expected"] is True, "Original plan source or SDK changed")
    require(plan["recipe_sha256"] == hashes["libtapi_recipe_sha256"]
            and plan["stager_sha256"] == hashes["stager_sha256"], "Published libtapi recipe or stager changed")
    require(set(plan["unchanged_recipe_fields"]) == tapi.UNCHANGED_FIELDS
            and all(value is True for value in plan["unchanged_recipe_fields"].values()), "Original unpack-only contract is incomplete")
    require(built["schema_version"] == 1 and built["scope"] == "staged_libtapi_build"
            and built["status"] == "staged_libtapi_built"
            and built["realization_command_succeeded"] is True and built["output_inspection_status"] == "pass",
            "The staged libtapi build must have passed; pending or failed builds cannot be used")
    require(built["plan_result_sha256"] == plan_hash and built["burdettie_revision"] == plan["burdettie_revision"]
            and built["source_revision"] == source["revision"]
            and built["helper_sha256"] == hashes["libtapi_plan_helper_sha256"], "Build receipt does not bind the exact inspected plan/code")
    require(built["phases"] and all(p["status"] == "pass" and p["exit_code"] == 0 for p in built["phases"])
            and any(p["name"] == "staged-libtapi-build" for p in built["phases"]), "Successful realization phase is missing")
    for name in ("candidate", "upstream", "full_libtapi_source"):
        require(built[name] == plan[name], "Build receipt differs from planned " + name)
    require(built["realized_output"] == plan["candidate"]["out"], "Successful output differs from the planned candidate")
    observed = built["staging_receipt"]
    require(observed["status"] == "staged" and observed["source_unchanged"] is True
            and observed["source_inventory_matches_inspected_plan"] is True
            and observed["source_inventory_sha256"] == plan["full_libtapi_source"]["source_inventory_sha256"],
            "Build receipt lacks the observed unchanged-source staging inventory")
    inspection = built["output_inspection"]
    for name in ("source_reference_absent", "temporary_build_paths_absent", "headers_are_regular_files"):
        require(inspection[name] is True, "Successful output inspection is incomplete: " + name)
    registered = inspection["registered_output"]
    require(registered["path"] == plan["candidate"]["out"]
            and registered["deriver"] == plan["candidate"]["drv"], "Successful output has the wrong registered deriver")
    return registered


def evaluate(source, recipe, libtapi_recipe, native_recipe, stager):
    string = json.dumps(stager).replace("${", "\\${")
    expression = ("p: (" + recipe + ") { packages = p; libtapiRecipe = (" + libtapi_recipe
                  + "); nativeLinkerRecipe = (" + native_recipe + "); stager = " + string + "; }")
    return linker.evaluate(source, expression)


def read_command(argv, name, output, report):
    report["commands"][name] = argv
    try:
        process = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True, timeout=90, check=False)
        code, timed_out, stdout, stderr = process.returncode, False, process.stdout, process.stderr
    except subprocess.TimeoutExpired as error:
        code, timed_out, stdout, stderr = None, True, error.stdout or b"", error.stderr or b""
    for suffix, content in (("stdout", stdout), ("stderr", stderr)):
        (output / (name + "." + suffix)).write_bytes(content)
    report.setdefault("read_checks", {})[name] = {
        "exit_code": code, "timed_out": timed_out,
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(), "stderr_sha256": hashlib.sha256(stderr).hexdigest()}
    require(code == 0, name + " failed; retained stdout/stderr contain the actual result")
    return stdout


def inspect_files(output, expected):
    files, links, elf = {}, {}, []
    for path in sorted(output.rglob("*")):
        relative = path.relative_to(output).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISLNK(mode):
            links[relative] = os.readlink(path)
            require(path.resolve(strict=True).is_relative_to(output), "Library output link escapes its package")
        elif stat.S_ISREG(mode):
            files[relative] = {"bytes": path.stat().st_size, "sha256": base.sha256(path)}
            with path.open("rb") as stream:
                header = stream.read(20)
            if header.startswith(b"\x7fELF"):
                require(header[4:6] == b"\x02\x01" and header[18:20] == b"\x3e\x00", "Library ELF architecture changed")
                if relative.startswith("lib/libtapi"):
                    require(header[16:18] == b"\x03\x00", "libtapi must be an ELF shared library")
                elf.append(relative)
        else:
            require(stat.S_ISDIR(mode), "Unexpected output file type")
    require(files == expected["files"] and links == expected["symlinks"] and elf == expected["elf_files"],
            "Current output differs from the successful build's file inventory")
    require(any(name.startswith("lib/libtapi") for name in elf), "Native libtapi ELF library is missing")
    for name in HEADERS:
        path = output / "include/tapi" / name
        require(path.is_file() and not path.is_symlink(), "Public header is missing or became a symlink: " + name)
    return {"files": files, "symlinks": links, "elf_files": elf, "matches_successful_receipt": True}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--libtapi-plan", type=Path, required=True)
    parser.add_argument("--libtapi-plan-sha256", required=True)
    parser.add_argument("--libtapi-build-result", type=Path, required=True)
    parser.add_argument("--libtapi-build-result-sha256", required=True)
    parser.add_argument("--expected-sdk-output", required=True)
    parser.add_argument("--expected-sdk-derivation", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--query-substituters", action="store_true")
    args = parser.parse_args(argv)
    report = dict(schema_version=1, scope="staged_libtapi_native_linker_plan_only", status="blocked",
                  dependency_realization="not_run", dependency_downloads="not_run", linking="not_run",
                  guest_execution="not_run", burdettie_runtime_execution="not_run", security_enforcement="not_run",
                  planning_mode="substituter_metadata" if args.query_substituters else "offline_graph_only",
                  substituter_queries_enabled=args.query_substituters, peak_space_qualified=False, commands={})
    output = None
    try:
        directory = args.output_dir.absolute()
        require(not directory.exists() and not directory.is_symlink() and directory.parent.is_dir(),
                "Output must be a new directory with an existing parent")
        directory.mkdir(mode=0o700)
        output = directory
        require(base.platform.system() == "Linux" and base.platform.machine() == "x86_64", "An x86_64 Linux host is required")
        require(base.shutil.which("nix") is not None, "Nix is unavailable; nothing was installed")
        require(os.environ.get("NIX_REMOTE", "") in ("", "local", "daemon"), "A remote Nix store is not supported")
        source = args.puredarwin_source.resolve(strict=True)
        report["source"] = base.source_identity(source)
        report["nix_version"] = base.command(["nix", "--version"]).strip()
        base.require_nix_version(report["nix_version"])
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        files = {"recipe_sha256": RECIPE, "libtapi_recipe_sha256": tapi.RECIPE,
                 "native_linker_recipe_sha256": linker.RECIPE, "stager_sha256": tapi.STAGER,
                 "libtapi_plan_helper_sha256": Path(tapi.__file__)}
        snapshots = {key: path.read_bytes() for key, path in files.items()}
        hashes = {key: hashlib.sha256(content).hexdigest() for key, content in snapshots.items()}
        report.update(hashes)
        for key, path in files.items():
            (output / path.name).write_bytes(snapshots[key])
        plan, plan_bytes = read_json(args.libtapi_plan, args.libtapi_plan_sha256)
        built, built_bytes = read_json(args.libtapi_build_result, args.libtapi_build_result_sha256)
        (output / "libtapi-plan.json").write_bytes(plan_bytes)
        (output / "libtapi-build-result.json").write_bytes(built_bytes)
        sdk = {"drv": args.expected_sdk_derivation, "out": args.expected_sdk_output}
        report["expected_sdk"] = sdk
        expected_registered = validate_receipts(plan, built, args.libtapi_plan_sha256, report["source"], sdk, hashes)
        prior_info, prior_bytes = read_json(args.libtapi_build_result.parent / "output-path-info.json",
                                            expected_registered["path_info_sha256"])
        (output / "libtapi-output-path-info.json").write_bytes(prior_bytes)
        tapi_out = plan["candidate"]["out"]
        require(set(prior_info) == {tapi_out}, "Successful path-info must identify exactly the libtapi output")
        registered = registered_identity(prior_info[tapi_out], tapi_out)
        require(registered == {key: expected_registered[key] for key in registered}, "Registered-output receipt and raw Nix metadata disagree")
        require(prior_info[tapi_out]["references"] == built["output_inspection"]["references"],
                "Successful reference inventory and raw Nix metadata disagree")
        report["successful_libtapi"] = {"plan_sha256": args.libtapi_plan_sha256,
                                        "build_result_sha256": args.libtapi_build_result_sha256,
                                        "burdettie_revision": plan["burdettie_revision"],
                                        "registered_output": expected_registered}
        records, evaluation = evaluate(source, *(snapshots[key].decode("utf-8") for key in
            ("recipe_sha256", "libtapi_recipe_sha256", "native_linker_recipe_sha256", "stager_sha256")))
        report["commands"]["evaluate"] = evaluation
        fresh_tapi = records["staged_libtapi"]
        for key in ("candidate", "upstream", "unchanged_recipe_fields", "stager_sha256", "unpack_phase_sha256",
                    "upstream_patch_files", "upstream_build_phase_sha256", "upstream_post_patch_sha256"):
            require(fresh_tapi[key] == plan[key], "Re-evaluated libtapi no longer matches the successful plan: " + key)
        for key in ("drv", "out", "rev", "output_hash", "retained_unchanged"):
            require(fresh_tapi["full_libtapi_source"][key] == plan["full_libtapi_source"][key], "Complete libtapi source identity changed")
        require(records["sdk"] == sdk and fresh_tapi["sdk"] == sdk and Path(sdk["out"]).is_dir(), "SDK identity or realized output changed")
        require(set(records["unchanged_recipe_fields"]) == UNCHANGED_FIELDS
                and all(value is True for value in records["unchanged_recipe_fields"].values()), "Native rebind changed an unrelated field")
        report.update(records)
        report["sdk"]["identity_matches_expected"] = True
        require(Path(tapi_out).is_dir(), "Successful libtapi output is no longer present")
        source_manifest = tapi.staging.inspect_source(Path(plan["full_libtapi_source"]["out"]))
        require(source_manifest["source_inventory_sha256"] == plan["full_libtapi_source"]["source_inventory_sha256"],
                "Retained complete source bytes changed after the successful build")
        (output / "source-manifest.json").write_bytes(tapi.staging.encoded(source_manifest))
        report["source_inventory_matches_successful_build"] = True
        # From this point, a failed live integrity check is a failed attempt,
        # rather than a rejected prerequisite or a successful plan.
        report["status"] = "fail"
        raw = read_command(linker.NIX + ["path-info", "--offline", "--json", tapi_out, sdk["out"]],
                           "path-info", output, report)
        current = json.loads(raw)
        require(set(current) == {tapi_out, sdk["out"]}, "Fresh path-info does not identify both required outputs")
        require(registered_identity(current[tapi_out], tapi_out) == registered, "Current libtapi derivation or NAR identity changed")
        sdk_registered = registered_identity(current[sdk["out"]], sdk["out"])
        require(sdk_registered["deriver"] == sdk["drv"], "Realized SDK has the wrong registered deriver")
        refs = current[tapi_out]["references"]
        require(isinstance(refs, list) and all(isinstance(value, str) for value in refs), "Invalid current store reference inventory")
        require(sorted(refs) == sorted(built["output_inspection"]["references"])
                and plan["full_libtapi_source"]["out"] not in refs, "Current output references differ from the successful inspection")
        read_command(linker.NIX + ["store", "verify", "--offline", tapi_out, sdk["out"]],
                     "store-verify", output, report)
        report["current_output_inspection"] = inspect_files(Path(tapi_out), built["output_inspection"])
        report["current_registered_outputs"] = {"libtapi": registered, "sdk": sdk_registered}
        commands = linker.build_commands(records["candidate"], output, args.query_substituters)
        report["commands"].update(commands)
        report["dry_run"] = linker.dry_run(commands["dry_run"], output)
        require(report["dry_run"]["exit_code"] == 0, "Native linker dry-run failed or timed out; inspect dry-run.log")
        report["status"] = "planned_not_built"
    except (base.BuildError, OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    payload = json.dumps(report, indent=2) + "\n"
    if output is not None:
        (output / "result.json").write_text(payload)
    print(payload, end="")
    return 0 if report["status"] == "planned_not_built" else (1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    sys.exit(main())
