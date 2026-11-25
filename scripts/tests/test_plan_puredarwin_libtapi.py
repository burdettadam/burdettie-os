"""Require real Nix evaluation and test the opt-in, plan-only contract."""
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
sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan_puredarwin_libtapi as plan
from test_stage_libtapi_source import PATCH, make_source


UNCHANGED = {name: True for name in ("source", "patches", "post_patch", "build_phase",
                                    "native_build_inputs", "build_inputs", "configure_phase", "install_phase",
                                    "dont_install", "cmake_driver", "pre_unpack", "post_unpack")}
FIXTURE = '''let
  mk = attrs: attrs // {
    outPath = "/nix/store/" + builtins.substring 0 32 (builtins.hashString "sha256" (builtins.toJSON attrs)) + "-libtapi";
    drvPath = (mk attrs).outPath + ".drv";
    overrideAttrs = change: mk (attrs // change (mk attrs) // { DRIFT });
  };
  tapi = mk {
    pname = "apple-libtapi"; version = "1300.6.5";
    src = { outPath = "/nix/store/full-source"; drvPath = "/nix/store/full-source.drv";
      rev = "640b4623929c923c0468143ff2a363a48665fa54";
      outputHash = "sha256-eXzQRB07AcH9nBgPoGpIQZbC4O/bIzkdbjJ442qM9VA="; };
    patches = [ "/nix/store/libtapi-riscv.patch" ];
    postPatch = "patchShebangs build.sh install.sh tools";
    buildPhase = "runHook preBuild; export JOBS=$NIX_BUILD_CORES; ./build.sh; ./install.sh; runHook postBuild";
    dontInstall = true; dontUseCmakeConfigure = true;
    nativeBuildInputs = [ { pname = "python3"; outPath = "/nix/store/python3"; } ];
    EXTRA
  };
in { native-ld = { buildInputs = [ tapi ]; };
  apple-sdk = { drvPath = "/nix/store/sdk.drv"; outPath = "/nix/store/sdk"; }; }
'''


class LibtapiOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.nix = os.environ.get("NIX_BIN") or shutil.which("nix")
        if not cls.nix:
            raise RuntimeError("Set NIX_BIN: actual Nix overlay tests are required, never skipped")

    def evaluate(self, *, drift="", extra="", replacement=None, stager=None):
        fixture = FIXTURE.replace("DRIFT", drift).replace("EXTRA", extra)
        if replacement:
            fixture = fixture.replace(*replacement)
        with mock.patch.object(plan.linker, "evaluate", return_value=({}, [])) as evaluate:
            plan.evaluate(Path("/unused"), plan.RECIPE.read_text(), stager or plan.STAGER.read_text())
        with tempfile.NamedTemporaryFile(mode="w", suffix=".patch") as patch:
            patch.write(PATCH)
            patch.flush()
            fixture = fixture.replace("/nix/store/libtapi-riscv.patch", patch.name)
            expression = "(" + evaluate.call_args.args[1] + ") (" + fixture + ")"
            # This synthetic package's patch is a local temporary file. The
            # production helper still uses pure flake evaluation; only this
            # test expression needs permission to read its fixture file.
            return subprocess.run([self.nix, "--extra-experimental-features", "nix-command flakes",
                                   "eval", "--store", "dummy://", "--impure", "--json", "--expr", expression],
                                  capture_output=True, text=True, timeout=30, check=False)

    def test_only_unpack_changes_and_source_sdk_dependency_identities_remain(self):
        result = self.evaluate()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["unchanged_recipe_fields"], UNCHANGED)
        self.assertNotEqual(report["candidate"], report["upstream"])
        self.assertEqual(report["sdk"]["out"], "/nix/store/sdk")
        self.assertEqual(report["full_libtapi_source"]["out"], "/nix/store/full-source")
        self.assertTrue(report["full_libtapi_source"]["retained_unchanged"])
        self.assertEqual(report["stager_sha256"], hashlib.sha256(plan.STAGER.read_bytes()).hexdigest())
        self.assertEqual(report["unpack_phase_sha256"], hashlib.sha256(report["unpack_phase"].encode()).hexdigest())
        self.assertEqual(report["upstream_patch_files"][0]["sha256"], hashlib.sha256(PATCH.encode()).hexdigest())
        self.assertEqual(report["upstream_post_patch_sha256"],
                         hashlib.sha256(b"patchShebangs build.sh install.sh tools").hexdigest())

    def test_upstream_source_hook_or_dependency_drift_fails_closed(self):
        changes = [
            {"drift": 'buildPhase = "changed flags";'},
            {"drift": 'postPatch = "disabled";'},
            {"drift": 'patches = [];'},
            {"drift": 'nativeBuildInputs = [];'},
            {"extra": 'unpackPhase = "unexpected existing override";'},
            {"replacement": ('pname = "python3"', 'pname = "another-tool"')},
            {"replacement": ('buildInputs = [ tapi ]', 'buildInputs = [ tapi tapi ]')},
            {"replacement": ("640b4623929c923c0468143ff2a363a48665fa54", "different-revision")},
            {"replacement": ("sha256-eXzQRB07AcH9nBgPoGpIQZbC4O/bIzkdbjJ442qM9VA=", "different-hash")},
        ]
        for change in changes:
            with self.subTest(change=change):
                self.assertNotEqual(self.evaluate(**change).returncode, 0)

    def test_evaluated_unpack_actually_stages_source_and_runs_hooks(self):
        result = self.evaluate()
        self.assertEqual(result.returncode, 0, result.stderr)
        unpack = json.loads(result.stdout)["unpack_phase"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = make_source(root)
            build = root / "new-build"
            build.mkdir()
            script = 'runHook() { printf "hook:%s\\n" "$1"; }\n' + unpack + '\nprintf "root:%s\\n" "$sourceRoot"\n'
            result = subprocess.run(["bash", "-euo", "pipefail", "-c", script], cwd=build,
                                    env=dict(os.environ, src=str(source)), capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("hook:preUnpack", result.stdout)
            self.assertIn("hook:postUnpack", result.stdout)
            self.assertIn("root:source", result.stdout)
            self.assertIn("BURDETTIE_LIBTAPI_STAGING", result.stdout)
            manifest = json.loads((build / "libtapi-staging.json").read_text())
            self.assertEqual(manifest["source_inventory_sha256"], plan.staging.inspect_source(source)["source_inventory_sha256"])

    def test_nix_string_preserves_literal_interpolation_bytes(self):
        stager = plan.STAGER.read_text() + '\n# Literal ${not_a_nix_variable}\n'
        result = self.evaluate(stager=stager)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["stager_sha256"], hashlib.sha256(stager.encode()).hexdigest())


class LibtapiPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = make_source(self.root)
        (self.root / "sdk").mkdir()
        self.identities = {
            "sdk": {"drv": "/nix/store/sdk.drv", "out": str(self.root / "sdk")},
            "upstream": {"drv": "/nix/store/original.drv", "out": "/nix/store/original"},
            "candidate": {"drv": "/nix/store/staged.drv", "out": "/nix/store/staged"},
            "full_libtapi_source": {"drv": "/nix/store/source.drv", "out": str(self.source)},
            "stager_sha256": hashlib.sha256(plan.STAGER.read_bytes()).hexdigest(),
            "unchanged_recipe_fields": dict(UNCHANGED),
        }

    def run_plan(self, *, output="plan", query=False, dry_code=0):
        with mock.patch.object(plan.base.platform, "system", return_value="Linux"), \
                mock.patch.object(plan.base.platform, "machine", return_value="x86_64"), \
                mock.patch.object(plan.base.shutil, "which", return_value="nix"), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": ""}), \
                mock.patch.object(plan.base, "source_identity", return_value={"revision": "pinned"}), \
                mock.patch.object(plan.base, "command", side_effect=lambda a: "nix (Nix) 2.28.3" if a[0] == "nix" else "revision"), \
                mock.patch.object(plan, "evaluate", return_value=(copy.deepcopy(self.identities), ["offline-eval"])), \
                mock.patch.object(plan.linker, "dry_run", return_value={"exit_code": dry_code, "timed_out": False}) as dry_run, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            argv = ["--puredarwin-source", str(self.root), "--expected-sdk-output", str(self.root / "sdk"),
                    "--expected-sdk-derivation", "/nix/store/sdk.drv", "--output-dir", str(self.root / output)]
            code = plan.main(argv + (["--query-substituters"] if query else []))
            return code, json.loads(stdout.getvalue()), dry_run

    def test_default_is_offline_plan_only_with_complete_manifest_and_snapshots(self):
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (0, "planned_not_built"))
        self.assertEqual(run.call_count, 1)
        self.assertIn("--offline", run.call_args.args[0])
        self.assertIn("--dry-run", run.call_args.args[0])
        self.assertIn("--max-jobs", run.call_args.args[0])
        self.assertEqual(report["commands"]["realize"][-2:], ["--out-link", str(self.root / "plan/result-libtapi")])
        self.assertTrue(report["sdk"]["identity_matches_expected"])
        self.assertFalse(report["peak_space_qualified"])
        for name in ("dependency_realization", "dependency_downloads", "linking", "guest_execution",
                     "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(report[name], "not_run")
        for path in (plan.RECIPE, plan.STAGER):
            self.assertEqual((self.root / "plan" / path.name).read_bytes(), path.read_bytes())
        manifest = self.root / "plan/source-manifest.json"
        self.assertEqual(report["full_libtapi_source"]["source_manifest_sha256"], plan.base.sha256(manifest))
        self.assertEqual(json.loads(manifest.read_text())["entries"], plan.staging.inspect_source(self.source)["entries"])

    def test_explicit_substituter_query_has_no_realization_or_capacity_claim(self):
        code, report, run = self.run_plan(query=True)
        self.assertEqual(code, 0)
        self.assertNotIn("--offline", run.call_args.args[0])
        self.assertEqual(report["planning_mode"], "substituter_metadata")
        self.assertEqual(report["dependency_downloads"], "not_run")
        self.assertFalse(report["full_libtapi_source"]["staging_estimate"]["peak_space_qualified"])

    def test_identity_input_and_stager_mismatches_block_before_dry_run(self):
        for field, key, value in (("sdk", "drv", "/different"),
                                  ("candidate", "out", self.identities["upstream"]["out"]),
                                  ("full_libtapi_source", "out", str(self.root / "missing")),
                                  ("unchanged_recipe_fields", "build_phase", False)):
            original = self.identities[field][key]
            self.identities[field][key] = value
            code, report, run = self.run_plan(output=field)
            self.assertEqual((code, report["status"]), (2, "blocked"))
            run.assert_not_called()
            self.identities[field][key] = original
        self.identities["stager_sha256"] = "wrong"
        code, _, run = self.run_plan(output="stager")
        self.assertEqual(code, 2)
        run.assert_not_called()

    def test_incomplete_unchanged_field_contract_is_rejected(self):
        self.identities["unchanged_recipe_fields"].pop("post_unpack")
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (2, "blocked"))
        run.assert_not_called()

    def test_existing_output_and_failed_dry_run_are_not_hidden(self):
        code, report, _ = self.run_plan(dry_code=1)
        self.assertEqual((code, report["status"]), (1, "fail"))
        retained = (self.root / "plan/result.json").read_bytes()
        code, _, run = self.run_plan()
        self.assertEqual(code, 2)
        run.assert_not_called()
        self.assertEqual((self.root / "plan/result.json").read_bytes(), retained)

    def test_production_evaluation_remains_offline_and_disables_realization(self):
        with mock.patch.object(plan.base, "command", return_value="{}") as command:
            plan.evaluate(Path("/pinned-source"), "{ packages, stager }: {}", "# source bytes\n")
        argv = command.call_args.args[0]
        for flag in ("--offline", "--no-write-lock-file", "--no-update-lock-file"):
            self.assertIn(flag, argv)
        self.assertIn("allow-import-from-derivation", argv)
        self.assertEqual(argv[argv.index("allow-import-from-derivation") + 1], "false")
        self.assertEqual(argv[argv.index("builders") + 1], "")
        self.assertEqual(argv[argv.index("min-free") + 1], "0")
        self.assertNotIn("--impure", argv)
        self.assertNotIn("build", argv)


if __name__ == "__main__":
    unittest.main()
