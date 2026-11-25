#!/usr/bin/env python3
"""Plan one downstream stage with the retained SDK and bounded native linker.

Never realizes dependencies or links a fixture. Evaluation stays offline and
does not update locks. --query-substituters permits dry-run cache metadata
queries only. The full immutable LLVM source remains a required input; reduced
writable staging does not establish that the source/store/build peak will fit.
Run each reported realization command only after inspecting its fresh plan and
applying the owner's existing CPU, memory, disk, task and time limits.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import plan_puredarwin_linker as linker


base = linker.base
RECIPE = Path(__file__).with_name("puredarwin_libraries.nix")
STAGES = ("cross-toolchain", "compiler-rt", "libsystem")


def evaluate(source, recipe, native_recipe, stage):
    expression = ("p: (" + recipe + ") { packages = p; nativeLinkerRecipe = ("
                  + native_recipe + "); stage = " + json.dumps(stage) + "; }")
    return linker.evaluate(source, expression)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=STAGES, required=True)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--expected-sdk-output", required=True)
    parser.add_argument("--expected-sdk-derivation", required=True)
    parser.add_argument("--expected-linker-output", required=True)
    parser.add_argument("--expected-linker-derivation", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--query-substituters", action="store_true",
                        help="Permit binary-cache metadata queries in the dry-run; never realize outputs")
    args = parser.parse_args(argv)
    report = dict(schema_version=1, scope="library_stage_plan_only", stage=args.stage,
                  status="blocked", dependency_realization="not_run", dependency_downloads="not_run",
                  fixture_linking="not_run", guest_execution="not_run",
                  burdettie_runtime_execution="not_run", security_enforcement="not_run",
                  substituter_queries_enabled=args.query_substituters, peak_space_qualified=False,
                  planning_mode="substituter_metadata" if args.query_substituters else "offline_graph_only")
    output = None
    try:
        directory = args.output_dir.absolute()
        if directory.exists() or not directory.parent.is_dir():
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
        recipes = {}
        for path, key in ((RECIPE, "recipe_sha256"), (linker.RECIPE, "native_linker_recipe_sha256")):
            content = path.read_bytes()
            (output / path.name).write_bytes(content)
            recipes[path.name] = content.decode()
            report[key] = hashlib.sha256(content).hexdigest()
        report["helper_sha256"] = base.sha256(Path(__file__))
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        identities, evaluation = evaluate(source, recipes[RECIPE.name], recipes[linker.RECIPE.name], args.stage)
        report.update(identities)
        expected = {
            "sdk": {"out": args.expected_sdk_output, "drv": args.expected_sdk_derivation},
            "native_linker": {"out": args.expected_linker_output, "drv": args.expected_linker_derivation},
        }
        report["expected"] = expected
        if identities["stage"] != args.stage:
            raise base.BuildError("Evaluated stage differs from the requested stage")
        for name, identity in expected.items():
            if identities[name] != identity:
                raise base.BuildError(name + " derivation/output differs from the retained baseline")
            if not Path(identity["out"]).is_dir():
                raise base.BuildError(name + " output is not realized; no build was started")
            report[name]["identity_matches_expected"] = True
        if any(identities["upstream"][key] == identities["candidate"][key] for key in ("drv", "out")):
            raise base.BuildError("Bounded stage derivation and output must differ from upstream")
        report["commands"] = {"evaluate": evaluation, **linker.build_commands(
            identities["candidate"], output, args.query_substituters)}
        report["commands"]["realize"][-1] = str(output / ("result-" + args.stage))
        report["dry_run"] = linker.dry_run(report["commands"]["dry_run"], output)
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
