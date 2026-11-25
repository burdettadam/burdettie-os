#!/usr/bin/env python3
"""Plan an opt-in libtapi source-staging derivation; never realize outputs.

The complete pinned source and SDK must already be realized. Evaluation stays
offline. --query-substituters permits only the dry-run's cache metadata queries.
Staging estimates do not qualify compilation, linking, or peak disk capacity.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import build_puredarwin_fixtures as base
import plan_puredarwin_linker as linker
import stage_libtapi_source as staging


RECIPE = Path(__file__).with_name("puredarwin_libtapi.nix")
STAGER = Path(staging.__file__).resolve()
UNCHANGED_FIELDS = frozenset({"source", "patches", "post_patch", "build_phase", "native_build_inputs",
                              "build_inputs", "configure_phase", "install_phase", "dont_install",
                              "cmake_driver", "pre_unpack", "post_unpack"})


def evaluate(source, recipe, stager):
    # Pass code as a Nix string in an argv element, never through a shell.
    string = json.dumps(stager).replace("${", "\\${")
    expression = "p: (" + recipe + ") { packages = p; stager = " + string + "; }"
    return linker.evaluate(source, expression)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--expected-sdk-output", required=True)
    parser.add_argument("--expected-sdk-derivation", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--query-substituters", action="store_true")
    args = parser.parse_args(argv)
    report = dict(schema_version=1, scope="libtapi_source_staging_plan_only", status="blocked",
                  dependency_realization="not_run", dependency_downloads="not_run", linking="not_run",
                  guest_execution="not_run", burdettie_runtime_execution="not_run", security_enforcement="not_run",
                  substituter_queries_enabled=args.query_substituters,
                  planning_mode="substituter_metadata" if args.query_substituters else "offline_graph_only",
                  peak_space_qualified=False)
    output = None
    try:
        directory = args.output_dir.absolute()
        if directory.exists() or directory.is_symlink() or not directory.parent.is_dir():
            raise base.BuildError("Output must be a new directory with an existing parent")
        directory.mkdir(mode=0o700)
        output = directory
        if base.platform.system() != "Linux" or base.platform.machine() != "x86_64":
            raise base.BuildError("This plan requires an x86_64 Linux host")
        if base.shutil.which("nix") is None:
            raise base.BuildError("Nix is unavailable; no tool was installed")
        if os.environ.get("NIX_REMOTE", "") not in ("", "local", "daemon"):
            raise base.BuildError("A remote Nix store is not supported")
        source = args.puredarwin_source.resolve(strict=True)
        report["source"] = base.source_identity(source)
        report["nix_version"] = base.command(["nix", "--version"]).strip()
        base.require_nix_version(report["nix_version"])
        recipe, stager = RECIPE.read_bytes(), STAGER.read_bytes()
        for path, content in ((RECIPE, recipe), (STAGER, stager)):
            (output / path.name).write_bytes(content)
        report["recipe_sha256"] = hashlib.sha256(recipe).hexdigest()
        expected_stager_hash = hashlib.sha256(stager).hexdigest()
        report["stager_sha256"] = expected_stager_hash
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        identities, evaluation = evaluate(source, recipe.decode(), stager.decode())
        if identities["stager_sha256"] != expected_stager_hash:
            raise base.BuildError("Evaluated stager differs from its retained source snapshot")
        report.update(identities)
        expected = {"drv": args.expected_sdk_derivation, "out": args.expected_sdk_output}
        report["expected_sdk"] = expected
        if identities["sdk"] != expected or not Path(expected["out"]).is_dir():
            raise base.BuildError("Realized SDK identity does not match the baseline")
        report["sdk"]["identity_matches_expected"] = True
        if any(identities["upstream"][key] == identities["candidate"][key] for key in ("drv", "out")):
            raise base.BuildError("Staging derivation and output must differ from upstream")
        if (set(identities["unchanged_recipe_fields"]) != UNCHANGED_FIELDS
                or not all(value is True for value in identities["unchanged_recipe_fields"].values())):
            raise base.BuildError("An unrelated recipe field changed")
        full_source = Path(identities["full_libtapi_source"]["out"])
        if not full_source.is_dir():
            raise base.BuildError("Complete libtapi source is not realized; no download or build was started")
        manifest = staging.inspect_source(full_source, block_size=os.statvfs(output).f_frsize)
        manifest_path = output / "source-manifest.json"
        manifest_path.write_bytes(staging.encoded(manifest))
        report["full_libtapi_source"].update(
            realized=True, source_manifest_sha256=base.sha256(manifest_path),
            source_inventory_sha256=manifest["source_inventory_sha256"], manifest_file=manifest_path.name,
            staging_estimate=manifest["summary"])
        commands = linker.build_commands(identities["candidate"], output, args.query_substituters)
        commands["realize"][-1] = str(output / "result-libtapi")
        report["commands"] = {"evaluate": evaluation, **commands}
        report["dry_run"] = linker.dry_run(commands["dry_run"], output)
        if report["dry_run"]["exit_code"] != 0:
            report["status"] = "fail"
            raise base.BuildError("Dry-run failed or timed out; inspect dry-run.log")
        report["status"] = "planned_not_built"
    except (base.BuildError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        report["error"] = str(error)
    encoded = json.dumps(report, indent=2) + "\n"
    if output is not None:
        (output / "result.json").write_text(encoded)
    print(encoded, end="")
    return 0 if report["status"] == "planned_not_built" else (1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    sys.exit(main())
