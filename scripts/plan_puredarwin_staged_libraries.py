#!/usr/bin/env python3
"""Plan one downstream stage bound to an inspected staged-TAPI native linker.

An actual successful native plan/build receipt and currently verified outputs
are mandatory. No dependency realization, download or fixture build is run.
Keep reports private: they contain local paths and retained build evidence.
"""
import argparse
from datetime import datetime, timedelta
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

import inspect_puredarwin_native as inspection
import plan_puredarwin_libraries as libraries
import plan_puredarwin_staged_linker as native


base, linker, tapi = native.base, native.linker, native.tapi
RECIPE = Path(__file__).with_name("puredarwin_staged_libraries.nix")
FILES = {
    "recipe_sha256": RECIPE,
    "libraries_recipe_sha256": libraries.RECIPE,
    "libraries_plan_helper_sha256": Path(libraries.__file__),
    "staged_linker_recipe_sha256": native.RECIPE,
    "libtapi_recipe_sha256": tapi.RECIPE,
    "native_linker_recipe_sha256": linker.RECIPE,
    "stager_sha256": tapi.STAGER,
    "native_linker_plan_helper_sha256": Path(native.__file__),
    "libtapi_plan_helper_sha256": Path(tapi.__file__),
    "native_output_inspector_sha256": Path(inspection.__file__),
    "native_linker_base_helper_sha256": Path(linker.__file__),
    "fixture_helper_sha256": Path(base.__file__),
}
NATIVE_FIELDS = ("upstream", "previous_bounded", "rebound_unbounded", "candidate", "staged_libtapi",
                 "unchanged_recipe_fields", "build_inputs", "configure_phase_sha256", "build_phase_sha256",
                 "ninja_patch_sha256", "changed")
PRESERVED = {"sdk", "native_linker", "baseline_cross", "baseline_compiler_rt", "baseline_libsystem", "selected_recipe"}


def success_check(record, argv, *, bounded=False):
    native.require(record["argv"] == argv and inspection.zero(record["exit_code"]) and record["timed_out"] is False,
                   "Successful inspection command or outcome changed")
    native.require(record["output_limit_reached"] is False
                   and record["output_limit_bytes"] == (inspection.OUTPUT_LIMIT if bounded else None),
                   "Successful inspection output bound is missing or exceeded")
    for field in ("stdout_sha256", "stderr_sha256"):
        native.require(isinstance(record[field], str) and re.fullmatch(r"[0-9a-f]{64}", record[field]) is not None,
                       "Successful inspection stream checksum is missing")


def validate_realization(plan, built):
    historical = plan["commands"]["realize"]
    native.require(isinstance(historical, list) and historical and all(isinstance(item, str) for item in historical),
                   "Original plan realization argv is missing")
    old_link = Path(historical[-1])
    native.require(old_link.is_absolute() and ".." not in old_link.parts, "Original plan out-link must be absolute")
    expected_old = linker.build_commands(plan["candidate"], old_link.parent)["realize"]
    native.require(historical == expected_old, "Original plan realization arguments or bounds changed")
    actual = built["realization"]
    executable, link = Path(actual["nix_executable"]), Path(actual["out_link"])
    native.require(executable.is_absolute() and link.is_absolute() and ".." not in executable.parts
                   and ".." not in link.parts and actual["out_link_was_absent"] is True
                   and str(link) != plan["candidate"]["out"], "Native realization needs explicit local executable and new output link")
    expected = inspection.NIX + expected_old[len(linker.NIX):]
    expected[0], expected[-1] = str(executable), str(link)
    native.require(actual["argv"] == expected, "Native realization command differs from the selected candidate or one-job bounds")
    phases = [item for item in built["phases"] if item["name"] == inspection.BUILD_PHASE]
    native.require(len(phases) == 1 and phases[0]["argv"] == expected, "Native realization phase argv does not match its record")


def validate_native_receipts(plan, built, plan_hash, source, sdk, hashes, expected_driver):
    native.require(plan["schema_version"] == 1 and plan["scope"] == "staged_libtapi_native_linker_plan_only"
                   and plan["status"] == "planned_not_built" and inspection.zero(plan["dry_run"]["exit_code"])
                   and plan["dry_run"]["timed_out"] is False
                   and plan["planning_mode"] == "substituter_metadata" and plan["substituter_queries_enabled"] is True,
                   "An inspected successful staged-linker metadata plan is required")
    native.require(plan["source"] == source and native.identity(plan["sdk"]) == sdk
                   and plan["sdk"]["identity_matches_expected"] is True, "Native plan source or SDK changed")
    native.require(plan["recipe_sha256"] == hashes["staged_linker_recipe_sha256"], "Staged-linker recipe changed")
    for field in ("libtapi_recipe_sha256", "native_linker_recipe_sha256", "stager_sha256", "libtapi_plan_helper_sha256"):
        native.require(plan[field] == hashes[field], "Native plan no longer binds " + field)
    native.require(set(plan["unchanged_recipe_fields"]) == native.UNCHANGED_FIELDS
                   and all(value is True for value in plan["unchanged_recipe_fields"].values()),
                   "Native plan's unchanged-field proof is incomplete")
    native.require(plan["source_inventory_matches_successful_build"] is True
                   and plan["current_output_inspection"]["matches_successful_receipt"] is True
                   and set(plan["read_checks"]) == {"path-info", "store-verify"}
                   and all(inspection.zero(item["exit_code"]) and item["timed_out"] is False for item in plan["read_checks"].values()),
                   "Native plan lacks successful prior source/output verification")
    native.require(built["schema_version"] == 1 and built["scope"] == inspection.BUILD_SCOPE
                   and built["status"] == inspection.BUILD_SUCCESS
                   and built["realization_command_succeeded"] is True and built["output_inspection_status"] == "pass",
                   "A successful inspected native build is required; pending or failed evidence cannot be used")
    native.require(all(isinstance(built[field], str) for field in ("started_at", "finished_at")),
                   "Native build timestamps must be UTC strings")
    started, finished = (datetime.fromisoformat(built[field].replace("Z", "+00:00"))
                         for field in ("started_at", "finished_at"))
    native.require(started.utcoffset() == timedelta(0) and finished.utcoffset() == timedelta(0)
                   and finished >= started, "Native build lacks a valid terminal UTC interval")
    native.require(built["plan_result_sha256"] == plan_hash
                   and built["plan_burdettie_revision"] == plan["burdettie_revision"]
                   and re.fullmatch(r"[0-9a-f]{40}", built["burdettie_revision"]) is not None,
                   "Native build does not bind the exact inspected plan/revisions")
    native.require(built["helper_sha256"] == hashes["native_linker_plan_helper_sha256"]
                   and built["output_inspector_sha256"] == hashes["native_output_inspector_sha256"],
                   "Native build helper or shared inspector differs from the reviewed code")
    native.require(re.fullmatch(r"[0-9a-f]{64}", expected_driver) is not None and built["driver_sha256"] == expected_driver,
                   "Native build driver checksum differs from the independently reviewed driver")
    for field in ("recipe_sha256", "libtapi_recipe_sha256", "native_linker_recipe_sha256", "stager_sha256",
                  "libtapi_plan_helper_sha256", "source", "sdk", "successful_libtapi", *NATIVE_FIELDS):
        native.require(built[field] == plan[field], "Native build differs from its inspected plan: " + field)
    native.require(built["realized_output"] == plan["candidate"]["out"], "Native output differs from the candidate")
    native.require(built["phases"] and all(item["status"] == "pass" and inspection.zero(item["exit_code"])
                   and item["timed_out"] is False for item in built["phases"])
                   and any(item["name"] == inspection.BUILD_PHASE for item in built["phases"]),
                   "Successful native realization phase is missing")
    validate_realization(plan, built)
    result = built["output_inspection"]
    native.require(result["schema_version"] == 1 and result["scope"] == inspection.INSPECTION_SCOPE
                   and result["status"] == "pass" and result["staged_libtapi_reference_present"] is True
                   and result["original_libtapi_reference_absent"] is True
                   and result["fixture_linking"] == "not_run" and result["guest_execution"] == "not_run",
                   "Native output inspection is incomplete or has the wrong scope")
    native.require(result["dependencies"] == plan["current_registered_outputs"], "Native inspection dependency identities changed")
    candidate, dependencies = plan["candidate"], result["dependencies"]
    registered = result["registered_output"]
    native.require(registered["path"] == candidate["out"] and registered["deriver"] == candidate["drv"],
                   "Native successful registration has the wrong path or deriver")
    expected_refs = inspection.references(result["references"], dependencies["libtapi"]["path"],
                                         plan["staged_libtapi"]["upstream"]["out"])
    native.require(result["references"] == expected_refs, "Native references must be sorted")
    success_check(result["store_verification"], inspection.NIX + ["store", "verify", "--offline", candidate["out"],
                  dependencies["libtapi"]["path"], dependencies["sdk"]["path"], result["loader_registered_output"]["path"]])
    success_check(result["smoke_test"], [str(Path(candidate["out"]) / "bin/ld"), "-v"], bounded=True)
    success_check(result["loader_check"], [result["tapi_dynamic_binding"]["dynamic"]["interpreter"], "--list",
                  str(Path(candidate["out"]) / "bin/ld.real")], bounded=True)
    binding = result["tapi_dynamic_binding"]
    native.require(result["loader_resolution"] == {key: binding[key] for key in ("needed", "resolved_path", "sha256")},
                   "Observed loader resolution differs from the inspected TAPI library")
    native.require(result["tapi_dynamic_binding"]["resolved_path"].startswith(dependencies["libtapi"]["path"] + "/")
                   and result["version_identity"]["program"] == "ld"
                   and result["version_identity"]["project"] == "ld64"
                   and result["version_identity"]["tapi_version"] == inspection.TAPI_VERSION,
                   "Native inspection lacks the selected dynamic binding and source-defined version identity")
    return result


def read_retained(path, checksum, output, filename):
    value, raw = native.read_json(path, checksum)
    with (output / filename).open("xb") as stream:
        stream.write(raw)
    return value


def evaluate(source, snapshots, stage):
    names = {"libtapiRecipe": "libtapi_recipe_sha256", "stagedLinkerRecipe": "staged_linker_recipe_sha256",
             "nativeLinkerRecipe": "native_linker_recipe_sha256", "librariesRecipe": "libraries_recipe_sha256"}
    expression = "p: (" + snapshots["recipe_sha256"].decode() + ") { packages = p; "
    expression += " ".join(name + " = (" + snapshots[key].decode() + ");" for name, key in names.items())
    expression += " stager = " + json.dumps(snapshots["stager_sha256"].decode()).replace("${", "\\${")
    expression += "; stage = " + json.dumps(stage) + "; }"
    argv = inspection.NIX + ["eval", "--offline", "--no-write-lock-file", "--no-update-lock-file",
                            "--option", "allow-import-from-derivation", "false", "--json",
                            base.source_flake_url(source) + "#packages.x86_64-linux", "--apply", expression]
    return json.loads(base.command(argv)), argv


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=libraries.STAGES, required=True)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--native-linker-plan", type=Path, required=True)
    parser.add_argument("--native-linker-plan-sha256", required=True)
    parser.add_argument("--native-linker-build-result", type=Path, required=True)
    parser.add_argument("--native-linker-build-result-sha256", required=True)
    parser.add_argument("--native-linker-driver-sha256", required=True)
    parser.add_argument("--expected-sdk-output", required=True)
    parser.add_argument("--expected-sdk-derivation", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--query-substituters", action="store_true")
    args = parser.parse_args(argv)
    report = dict(schema_version=1, scope="staged_native_linker_library_plan_only", stage=args.stage, status="blocked",
                  dependency_realization="not_run", dependency_downloads="not_run", fixture_linking="not_run",
                  guest_execution="not_run", burdettie_runtime_execution="not_run", security_enforcement="not_run",
                  planning_mode="substituter_metadata" if args.query_substituters else "offline_graph_only",
                  substituter_queries_enabled=args.query_substituters, peak_space_qualified=False, commands={})
    output = None
    try:
        directory = args.output_dir.absolute()
        native.require(not directory.exists() and not directory.is_symlink() and directory.parent.is_dir(),
                       "Output must be a new directory with an existing parent")
        directory.mkdir(mode=0o700)
        output = directory
        native.require(base.platform.system() == "Linux" and base.platform.machine() == "x86_64", "An x86_64 Linux host is required")
        native.require(base.shutil.which("nix") is not None, "Nix is unavailable; nothing was installed")
        native.require(os.environ.get("NIX_REMOTE", "") in ("", "local"), "Only a direct local Nix store is supported")
        source = args.puredarwin_source.resolve(strict=True)
        report["source"] = base.source_identity(source)
        report["nix_version"] = base.command(inspection.NIX + ["--version"]).strip()
        base.require_nix_version(report["nix_version"])
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        report["helper_sha256"] = base.sha256(Path(__file__))
        snapshots = {key: path.read_bytes() for key, path in FILES.items()}
        hashes = {key: hashlib.sha256(data).hexdigest() for key, data in snapshots.items()}
        report.update(hashes)
        for key, path in FILES.items():
            with (output / path.name).open("xb") as stream:
                stream.write(snapshots[key])
        plan = read_retained(args.native_linker_plan, args.native_linker_plan_sha256, output, "native-linker-plan.json")
        built = read_retained(args.native_linker_build_result, args.native_linker_build_result_sha256,
                              output, "native-linker-build-result.json")
        sdk = {"drv": args.expected_sdk_derivation, "out": args.expected_sdk_output}
        report["expected_sdk"] = sdk
        expected = validate_native_receipts(plan, built, args.native_linker_plan_sha256, report["source"], sdk, hashes,
                                            args.native_linker_driver_sha256)
        registered = expected["registered_output"]
        prior_info = read_retained(args.native_linker_build_result.parent / "output-path-info.json",
                                  registered["path_info_sha256"], output, "successful-native-path-info.json")
        native.require(set(prior_info) == {registered["path"]}, "Native sidecar must identify exactly its output")
        observed = native.registered_identity(prior_info[registered["path"]], registered["path"])
        native.require(observed == {key: registered[key] for key in observed}
                       and sorted(prior_info[registered["path"]]["references"]) == expected["references"],
                       "Native registration/reference receipt disagrees with its raw sidecar")
        successful_tapi = plan["successful_libtapi"]
        prior_dir = args.native_linker_plan.parent
        tapi_plan = read_retained(prior_dir / "libtapi-plan.json", successful_tapi["plan_sha256"], output, "libtapi-plan.json")
        tapi_build = read_retained(prior_dir / "libtapi-build-result.json", successful_tapi["build_result_sha256"],
                                  output, "libtapi-build-result.json")
        native.require(inspection.zero(tapi_plan["dry_run"]["exit_code"])
                       and all(inspection.zero(item["exit_code"]) and item.get("timed_out", False) is False for item in tapi_build["phases"]),
                       "Retained TAPI success contains a non-integer exit code or timed-out phase")
        tapi_registered = native.validate_receipts(tapi_plan, tapi_build, successful_tapi["plan_sha256"], report["source"], sdk, hashes)
        native.require(tapi_registered == successful_tapi["registered_output"]
                       and successful_tapi["burdettie_revision"] == tapi_plan["burdettie_revision"], "Native plan's TAPI chain changed")
        for key in ("candidate", "upstream", "unchanged_recipe_fields", "stager_sha256", "unpack_phase_sha256",
                    "upstream_patch_files", "upstream_build_phase_sha256", "upstream_post_patch_sha256"):
            native.require(plan["staged_libtapi"][key] == tapi_plan[key], "Native plan's selected TAPI differs: " + key)
        for key in ("drv", "out", "rev", "output_hash", "retained_unchanged"):
            native.require(plan["staged_libtapi"]["full_libtapi_source"][key] == tapi_plan["full_libtapi_source"][key],
                           "Native plan's complete TAPI source differs")
        tapi_info = read_retained(prior_dir / "libtapi-output-path-info.json", tapi_registered["path_info_sha256"],
                                 output, "libtapi-output-path-info.json")
        native.require(set(tapi_info) == {tapi_registered["path"]}, "TAPI sidecar output selection changed")
        tapi_identity = native.registered_identity(tapi_info[tapi_registered["path"]], tapi_registered["path"])
        native.require(tapi_identity == {key: tapi_registered[key] for key in tapi_identity}
                       and tapi_identity == expected["dependencies"]["libtapi"]
                       and tapi_info[tapi_registered["path"]]["references"] == tapi_build["output_inspection"]["references"],
                       "Retained TAPI registration or references differ")
        native.require(expected["dependencies"]["sdk"]["path"] == sdk["out"]
                       and expected["dependencies"]["sdk"]["deriver"] == sdk["drv"], "Retained SDK registration changed")
        report["successful_native_linker"] = {"plan_sha256": args.native_linker_plan_sha256,
                                              "build_result_sha256": args.native_linker_build_result_sha256,
                                              "plan_burdettie_revision": plan["burdettie_revision"],
                                              "build_burdettie_revision": built["burdettie_revision"],
                                              "driver_sha256": built["driver_sha256"],
                                              "registered_output": registered}
        # External evaluation/live inspection has begun. Every later error is
        # a failed attempt, including missing evidence or output-write errors.
        report["status"] = "fail"
        records, evaluation = evaluate(source, snapshots, args.stage)
        report["commands"]["evaluate"] = evaluation
        native.require(records["stage"] == args.stage and records["sdk"] == sdk, "Selected stage or SDK changed")
        for field in NATIVE_FIELDS:
            native.require(records["native_linker"][field] == plan[field], "Reconstructed native plan changed: " + field)
        native.require(records["native_linker"]["sdk"] == sdk, "Reconstructed native SDK changed")
        expected_preserved = {name: True for name in PRESERVED}
        if args.stage == "cross-toolchain":
            expected_preserved["baseline_compiler_rt"] = None
        if args.stage != "libsystem":
            expected_preserved["baseline_libsystem"] = None
        native.require(records["preserved"] == expected_preserved, "Downstream phase/flag preservation proof changed")
        for key in ("drv", "out"):
            native.require(len({records[name][key] for name in ("upstream", "previous_bounded", "rebound_unbounded", "candidate")}) == 4,
                           "Stage identities must distinguish every baseline and candidate")
        report.update(records)
        source_manifest = tapi.staging.inspect_source(Path(tapi_plan["full_libtapi_source"]["out"]))
        native.require(source_manifest["source_inventory_sha256"] == tapi_plan["full_libtapi_source"]["source_inventory_sha256"],
                       "Retained complete TAPI source changed")
        with (output / "source-manifest.json").open("xb") as stream:
            stream.write(tapi.staging.encoded(source_manifest))
        report["source_inventory_matches_successful_build"] = True
        report["current_libtapi_inspection"] = native.inspect_files(Path(tapi_registered["path"]), tapi_build["output_inspection"])
        report["current_native_inspection"] = inspection.inspect_native_output(
            plan["candidate"], expected["dependencies"], plan["staged_libtapi"]["upstream"]["out"], output, report, expected=expected)
        commands = linker.build_commands(records["candidate"], output, args.query_substituters)
        commands = {key: inspection.NIX + argv[len(linker.NIX):] for key, argv in commands.items()}
        commands["realize"][-1] = str(output / ("result-" + args.stage))
        report["commands"].update(commands)
        report["dry_run"] = linker.dry_run(commands["dry_run"], output)
        native.require(inspection.zero(report["dry_run"]["exit_code"]) and report["dry_run"]["timed_out"] is False,
                       "Downstream dry-run failed or timed out; inspect retained log")
        native.require(all(path.read_bytes() == snapshots[key] for key, path in FILES.items())
                       and base.sha256(Path(__file__)) == report["helper_sha256"],
                       "Planner or recipe bytes changed during the attempt")
        report["status"] = "planned_not_built"
    except (base.BuildError, OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    payload = json.dumps(report, indent=2) + "\n"
    if output is not None:
        try:
            with (output / "result.json").open("x") as stream:
                stream.write(payload)
        except OSError as error:
            report["status"] = "fail"
            report["error"] = "Could not retain final result: " + str(error)
            payload = json.dumps(report, indent=2) + "\n"
    print(payload, end="")
    return 0 if report["status"] == "planned_not_built" else (1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    sys.exit(main())
