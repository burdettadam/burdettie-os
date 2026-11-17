#!/usr/bin/env python3
"""Plan a one-job native linker build while reusing the pinned realized SDK.

Always plan-only: offline evaluation and dry-run, never dependency realization.
Default dry-run is an offline graph, not a binary-cache capacity estimate. Use
--query-substituters to permit cache metadata queries without realizing outputs.
Use a new private output directory. Execute the reported realization argv only
inside the owner's existing CPU, memory, disk, task and time limits.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

import build_puredarwin_fixtures as base


RECIPE = Path(__file__).with_name("puredarwin_native_linker.nix")
NIX = ["nix", "--extra-experimental-features", "nix-command flakes",
       "--option", "builders", "", "--option", "min-free", "0"]


def evaluate(source, expression):
    argv = NIX + ["eval", "--offline", "--no-write-lock-file", "--no-update-lock-file",
                  "--option", "allow-import-from-derivation", "false", "--json",
                  base.source_flake_url(source) + "#packages.x86_64-linux", "--apply", expression]
    return json.loads(base.command(argv)), argv


def build_commands(candidate, output, query_substituters=False):
    common = NIX + ["build", candidate["drv"] + "^out", "--cores", "1",
                    "--option", "max-substitution-jobs", "2"]
    return {
        "dry_run": common + ([] if query_substituters else ["--offline"])
                   + ["--dry-run", "--no-link", "--max-jobs", "0"],
        "realize": common + ["--max-jobs", "1", "--keep-failed", "--print-build-logs", "--json",
                              "--out-link", str(output / "result-linker")],
    }


def dry_run(argv, output):
    try:
        result = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True,
                                text=True, timeout=180, check=False)
        code, timed_out, streams = result.returncode, False, (result.stdout, result.stderr)
    except subprocess.TimeoutExpired as error:
        code, timed_out, streams = None, True, (error.stdout, error.stderr)
    text = "".join(s.decode(errors="replace") if isinstance(s, bytes) else s or "" for s in streams)
    log = output / "dry-run.log"
    log.write_text(text)
    return {"exit_code": code, "timed_out": timed_out, "log_sha256": base.sha256(log)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--puredarwin-source", type=Path, required=True)
    parser.add_argument("--expected-sdk-output", required=True)
    parser.add_argument("--expected-sdk-derivation", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--query-substituters", action="store_true",
                        help="Permit binary-cache metadata queries in the dry-run; never realize outputs")
    args = parser.parse_args(argv)
    report = dict(schema_version=1, scope="native_linker_plan_only", status="blocked",
                  dependency_realization="not_run", linking="not_run", guest_execution="not_run",
                  burdettie_runtime_execution="not_run", security_enforcement="not_run",
                  dependency_downloads="not_run", substituter_queries_enabled=args.query_substituters,
                  planning_mode="substituter_metadata" if args.query_substituters else "offline_graph_only",
                  peak_space_qualified=False)
    output = None
    try:
        candidate_dir = args.output_dir.absolute()
        if candidate_dir.exists() or not candidate_dir.parent.is_dir():
            raise base.BuildError("Output must be a new directory with an existing parent")
        candidate_dir.mkdir(mode=0o700)
        output = candidate_dir
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
        recipe = RECIPE.read_bytes()
        (output / RECIPE.name).write_bytes(recipe)
        report["recipe_sha256"] = hashlib.sha256(recipe).hexdigest()
        report["burdettie_revision"] = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
        identities, evaluation = evaluate(source, recipe.decode())
        report.update(identities)
        expected = {"out": args.expected_sdk_output, "drv": args.expected_sdk_derivation}
        report["expected_sdk"] = expected
        if identities["sdk"] != expected:
            raise base.BuildError("SDK derivation/output identity differs from the realized baseline")
        if not Path(expected["out"]).is_dir():
            raise base.BuildError("Expected SDK output is not realized; no build was started")
        if any(identities["upstream"][key] == identities["candidate"][key] for key in ("drv", "out")):
            raise base.BuildError("Bounded linker derivation and output must differ from upstream")
        report["sdk"]["identity_matches_expected"] = True
        report["commands"] = {"evaluate": evaluation, **build_commands(
            identities["candidate"], output, args.query_substituters)}
        # Store both streams so a failed/offline-incomplete plan remains reviewable.
        report["dry_run"] = dry_run(report["commands"]["dry_run"], output)
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
