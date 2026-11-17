"""Plan-only gates and actual Nix evaluation against synthetic package records."""
import contextlib
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import plan_puredarwin_linker as plan


NIX_BIN = os.environ.get("NIX_BIN") or shutil.which("nix")


@unittest.skipUnless(NIX_BIN, "Set NIX_BIN or install Nix for the overlay evaluation tests")
class NixOverlayTests(unittest.TestCase):
    def evaluate(self, command="ninja -C build-nix-native host_ld host_strip", alteration=""):
        # Only the package/override machinery is synthetic; Nix evaluates the
        # complete committed recipe and its fail-closed assertions unchanged.
        fixture = '''let
          sdk = { drvPath = "/nix/store/sdk.drv"; outPath = "/nix/store/sdk"; };
          package = {
            drvPath = "/nix/store/original.drv"; outPath = "/nix/store/original";
            buildPhase = COMMAND;
            configurePhase = "export DARWIN_SDK_ROOT=/nix/store/sdk/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
            installPhase = "install untouched"; src = "/nix/store/source";
            nativeBuildInputs = [ { outPath = "/nix/store/ninja"; } ]; buildInputs = [];
            overrideAttrs = change: package // (change package) // {
              drvPath = "/nix/store/patched.drv"; outPath = "/nix/store/patched";
              ALTERATION
            };
          };
          in { native-ld = package; apple-sdk = sdk; }'''
        fixture = fixture.replace("COMMAND", json.dumps(command)).replace("ALTERATION", alteration)
        expression = "(" + plan.RECIPE.read_text() + ") (" + fixture + ")"
        return subprocess.run([NIX_BIN, "--extra-experimental-features", "nix-command flakes",
                               "eval", "--store", "dummy://", "--json", "--expr", expression],
                              text=True, capture_output=True, timeout=20, check=False)

    def test_exact_one_job_patch_retains_sdk_and_unrelated_fields(self):
        result = self.evaluate()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        before = "ninja -C build-nix-native host_ld host_strip"
        after = "ninja -j1 -C build-nix-native host_ld host_strip"
        self.assertEqual(report["patch_sha256"], hashlib.sha256((before + "\n" + after + "\n").encode()).hexdigest())
        self.assertEqual(report["candidate_build_phase_sha256"], hashlib.sha256(after.encode()).hexdigest())
        self.assertNotEqual(report["upstream"]["drv"], report["candidate"]["drv"])
        self.assertEqual(report["sdk"], {"drv": "/nix/store/sdk.drv", "out": "/nix/store/sdk"})
        self.assertTrue(all(report["unchanged"].values()))

    def test_missing_or_repeated_upstream_command_is_rejected(self):
        for command in ("ninja -C other", "ninja -C build-nix-native host_ld host_strip\n" * 2):
            with self.subTest(command=command):
                self.assertNotEqual(self.evaluate(command).returncode, 0)

    def test_unrelated_source_dependency_or_configure_change_is_rejected(self):
        for change in ('src = "/different";', 'nativeBuildInputs = [];',
                       'buildInputs = [ { outPath = "/added"; } ];',
                       'configurePhase = "changed";', 'installPhase = "changed";'):
            with self.subTest(change=change):
                self.assertNotEqual(self.evaluate(alteration=change).returncode, 0)


class LinkerPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.sdk = self.root / "sdk"
        self.sdk.mkdir()
        self.identities = {
            "sdk": {"drv": "/nix/store/sdk.drv", "out": str(self.sdk)},
            "upstream": {"drv": "/nix/store/original.drv", "out": "/nix/store/original"},
            "candidate": {"drv": "/nix/store/patched.drv", "out": "/nix/store/patched"},
            "patch_sha256": "patch-hash", "unchanged": {"source": True},
        }

    def run_plan(self, *, name="plan", expected_drv="/nix/store/sdk.drv", returncode=0,
                 timeout=False, query_substituters=False):
        with mock.patch.object(plan.base.platform, "system", return_value="Linux"), \
                mock.patch.object(plan.base.platform, "machine", return_value="x86_64"), \
                mock.patch.object(plan.base.shutil, "which", return_value="nix"), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": ""}), \
                mock.patch.object(plan.base, "source_identity", return_value={"revision": "pinned"}), \
                mock.patch.object(plan.base, "command", side_effect=lambda a: "nix (Nix) 2.28.3" if a[0] == "nix" else "revision"), \
                mock.patch.object(plan, "evaluate", return_value=(copy.deepcopy(self.identities), ["offline-eval"])), \
                mock.patch.object(plan.subprocess, "run") as run, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            run.return_value = subprocess.CompletedProcess([], returncode, "plan stdout\n", "plan stderr\n")
            if timeout:
                run.side_effect = subprocess.TimeoutExpired(["nix"], 180, output=b"partial plan\n")
            args = ["--puredarwin-source", str(self.root), "--expected-sdk-output", str(self.sdk),
                    "--expected-sdk-derivation", expected_drv, "--output-dir", str(self.root / name)]
            code = plan.main(args + (["--query-substituters"] if query_substituters else []))
            return code, json.loads(stdout.getvalue()), run

    def test_default_only_plans_and_reports_retained_bounded_realization(self):
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (0, "planned_not_built"))
        self.assertEqual(run.call_count, 1)
        executed = run.call_args.args[0]
        self.assertIn("--dry-run", executed)
        self.assertIn("--offline", executed)
        self.assertEqual(executed[executed.index("--max-jobs") + 1], "0")
        self.assertEqual(report["planning_mode"], "offline_graph_only")
        self.assertFalse(report["substituter_queries_enabled"])
        realize = report["commands"]["realize"]
        for option, value in (("--max-jobs", "1"), ("--cores", "1")):
            self.assertEqual(realize[realize.index(option) + 1], value)
        for argv in (executed, realize):
            for key, value in (("builders", ""), ("min-free", "0"), ("max-substitution-jobs", "2")):
                index = argv.index(key)
                self.assertEqual(argv[index - 1:index + 2], ["--option", key, value])
        self.assertIn("--keep-failed", realize)
        self.assertIn("--print-build-logs", realize)
        self.assertEqual(realize[-2:], ["--out-link", str(self.root / "plan/result-linker")])
        self.assertTrue(report["sdk"]["identity_matches_expected"])
        self.assertEqual(report["dry_run"]["exit_code"], 0)
        for key in ("dependency_realization", "dependency_downloads", "linking", "guest_execution",
                    "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(report[key], "not_run")
        self.assertEqual((self.root / "plan" / plan.RECIPE.name).read_bytes(), plan.RECIPE.read_bytes())

    def test_substituter_query_is_explicit_metadata_only_and_never_qualifies_peak_space(self):
        code, report, run = self.run_plan(query_substituters=True)
        self.assertEqual(code, 0)
        executed = run.call_args.args[0]
        self.assertNotIn("--offline", executed)
        self.assertIn("--dry-run", executed)
        self.assertEqual(executed[executed.index("--max-jobs") + 1], "0")
        self.assertEqual(report["planning_mode"], "substituter_metadata")
        self.assertTrue(report["substituter_queries_enabled"])
        self.assertFalse(report["peak_space_qualified"])
        self.assertEqual(report["dependency_downloads"], "not_run")

    def test_sdk_mismatch_or_unrealized_output_blocks_before_dry_run(self):
        code, report, run = self.run_plan(expected_drv="/different.drv")
        self.assertEqual((code, report["status"]), (2, "blocked"))
        run.assert_not_called()
        self.sdk.rmdir()
        code, report, run = self.run_plan(name="missing-sdk")
        self.assertEqual((code, report["status"]), (2, "blocked"))
        run.assert_not_called()

    def test_unchanged_candidate_identity_is_rejected(self):
        self.identities["candidate"] = self.identities["upstream"].copy()
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (2, "blocked"))
        run.assert_not_called()

    def test_retained_output_is_never_overwritten(self):
        output = self.root / "plan"
        output.mkdir()
        (output / "result.json").write_text("retained evidence")
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (2, "blocked"))
        self.assertEqual((output / "result.json").read_text(), "retained evidence")
        run.assert_not_called()

    def test_failed_and_timed_out_plans_keep_output_and_fail(self):
        for name, timeout in (("failed", False), ("timeout", True)):
            code, report, _ = self.run_plan(name=name, returncode=1, timeout=timeout)
            self.assertEqual((code, report["status"]), (1, "fail"))
            self.assertEqual(report["dry_run"]["timed_out"], timeout)
            self.assertEqual(report["dry_run"]["exit_code"], None if timeout else 1)
            self.assertTrue((self.root / name / "dry-run.log").read_text())

    def test_evaluation_is_offline_and_locks_remain_immutable(self):
        with mock.patch.object(plan.base, "command", return_value="{}") as command:
            plan.evaluate(Path("/source"), "p: p")
        argv = command.call_args.args[0]
        for flag in ("--offline", "--no-write-lock-file", "--no-update-lock-file", "allow-import-from-derivation"):
            self.assertIn(flag, argv)
        self.assertIn("eval", argv)
        self.assertNotIn("build", argv)
        for key, value in (("builders", ""), ("min-free", "0")):
            index = argv.index(key)
            self.assertEqual(argv[index - 1:index + 2], ["--option", key, value])


if __name__ == "__main__":
    unittest.main()
