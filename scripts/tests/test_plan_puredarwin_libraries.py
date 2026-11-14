"""Real Nix graph/phase assertions plus plan-only and source-staging checks."""
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
import plan_puredarwin_libraries as plan


NIX_BIN = os.environ.get("NIX_BIN") or shutil.which("nix")
FIXTURE = '''let
  mk = name: attrs: attrs // {
    inherit name;
    outPath = "/nix/store/" + builtins.substring 0 32 (builtins.hashString "sha256" (builtins.toJSON attrs)) + "-" + name;
    drvPath = (mk name attrs).outPath + ".drv";
    overrideAttrs = change: mk name (attrs // change (mk name attrs));
  };
  sdk = { drvPath = "/nix/store/sdk.drv"; outPath = "/nix/store/sdk"; };
  native = mk "native-ld" {
    src = "/nix/store/native-source"; nativeBuildInputs = []; buildInputs = [];
    configurePhase = "SDK=/nix/store/sdk/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk Release";
    buildPhase = "ninja -C build-nix-native host_ld host_strip";
    installPhase = "install native linker";
  };
  mkCross = ld: (mk "darwin-cross-toolchain-nix" {
    buildCommand = "symlink wrappers";
    paths = map (name: "/nix/store/" + builtins.substring 0 32 (builtins.hashString "sha256"
      (name + (if builtins.elem name [ "clang" "clang++" "ld" ] then ld.outPath else "unchanged")))
      + "-x86_64-apple-darwin20.4-" + name)
      [ "clang" "clang++" "ar" "ranlib" "strip" "nm" "objdump" "otool" "dsymutil" "install_name_tool" "lipo" "ld" "xcrun" "bare-dsymutil" ];
  }) // { override = args: (mkCross args.nativeLd).overrideAttrs (_: { CROSS_DRIFT }); };
  cross = mkCross native;
  llvm = { outPath = "/nix/store/full-llvm-source"; drvPath = "/nix/store/full-llvm-source.drv";
    rev = "llvmorg-21.1.8"; outputHash = "original-full-source-hash"; };
  mkRt = toolchain: (mk "compiler-rt" {
    src = llvm; postPatch = "patch compiler-rt/cmake/Modules/CompilerRTDarwinUtils.cmake";
    configurePhase = "Release x86_64-apple-darwin20.4 macosx26.5 ${toolchain.outPath}";
    buildPhase = RT_BUILD;
    installPhase = "ninja -C build install";
    nativeBuildInputs = []; buildInputs = [];
  }) // { override = args: (mkRt args.darwinCrossToolchain).overrideAttrs (_: { RT_DRIFT }); };
  rt = mkRt cross;
  mkLib = ld: toolchain: crt: (mk "libsystem" {
    src = "/nix/store/libsystem-source"; nativeBuildInputs = [ toolchain.outPath ]; buildInputs = [];
    configurePhase = "Debug x86_64 ${ld.outPath} ${toolchain.outPath} ${crt.outPath}";
    buildPhase = "ninja -C build-nix  libSystem_B_stub dyld";
    installPhase = "install libSystem dyld";
    NIX_DARWIN_HOST = "x86_64-apple-darwin20.4";
    NIX_DARWIN_TOOLCHAIN_DIR = toolchain.outPath + "/bin";
  }) // { override = args: (mkLib args.nativeLd args.darwinCrossToolchain args.compilerRt).overrideAttrs (_: { LIB_DRIFT }); };
in { native-ld = native; apple-sdk = sdk; darwin-cross-toolchain = cross;
  compiler-rt = rt; libsystem = mkLib native cross rt; }
'''


@unittest.skipUnless(NIX_BIN, "Set NIX_BIN or install Nix for the overlay evaluation tests")
class LibraryOverlayTests(unittest.TestCase):
    def evaluate(self, stage="compiler-rt", *, cross_drift="", rt_drift="", lib_drift="",
                 rt_build="ninja -C build", narrow=False):
        fixture = FIXTURE.replace("CROSS_DRIFT", cross_drift).replace("RT_DRIFT", rt_drift)
        fixture = fixture.replace("LIB_DRIFT", lib_drift).replace("RT_BUILD", json.dumps(rt_build))
        if narrow:
            fixture = fixture.replace("compiler-rt = rt; libsystem = mkLib native cross rt;",
                                      'compiler-rt = throw "unused compiler-rt"; libsystem = throw "unused libsystem";')
        expression = ("(" + plan.RECIPE.read_text() + ") { packages = (" + fixture
                      + "); nativeLinkerRecipe = (" + plan.linker.RECIPE.read_text()
                      + "); stage = " + json.dumps(stage) + "; }")
        return subprocess.run([NIX_BIN, "--extra-experimental-features", "nix-command flakes",
                               "eval", "--store", "dummy://", "--json", "--expr", expression],
                              text=True, capture_output=True, timeout=20, check=False)

    def test_all_stages_propagate_the_same_linker_and_preserve_source_flags(self):
        linker = None
        cross = None
        for stage in plan.STAGES:
            result = self.evaluate(stage)
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertTrue(all(report["preserved"].values()))
            self.assertNotEqual(report["candidate"], report["upstream"])
            self.assertEqual(report["stage"], stage)
            if linker is None:
                linker, cross = report["native_linker"], report["cross_toolchain"]["candidate"]
            self.assertEqual(report["native_linker"], linker)
            self.assertEqual(report["cross_toolchain"]["candidate"], cross)
            if stage != "cross-toolchain":
                self.assertEqual(report["compiler_rt"]["full_source"]["out"], "/nix/store/full-llvm-source")
                self.assertTrue(report["compiler_rt"]["full_source"]["retained_unchanged"])

    def test_cross_toolchain_plan_does_not_evaluate_library_inputs(self):
        result = self.evaluate("cross-toolchain", narrow=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(json.loads(result.stdout)["compiler_rt"])

    def test_missing_duplicate_commands_or_unrelated_changes_fail_closed(self):
        changes = [
            {"rt_build": "ninja other"}, {"rt_build": "ninja -C build\nninja -C build"},
            {"cross_drift": 'buildCommand = "changed";'},
            {"rt_drift": 'src = { outPath = "/different"; };'},
            {"rt_drift": 'postPatch = "different";'},
            {"rt_drift": 'configurePhase = "Release changed-target";'},
            {"lib_drift": 'src = "/different";'},
            {"lib_drift": 'configurePhase = "Release instead of Debug";'},
            {"lib_drift": 'NIX_DARWIN_HOST = "changed-target";'},
        ]
        for change in changes:
            with self.subTest(change=change):
                self.assertNotEqual(self.evaluate("libsystem", **change).returncode, 0)

    def test_actual_unpack_copies_only_compiler_rt_and_preserves_source_bytes(self):
        result = self.evaluate()
        self.assertEqual(result.returncode, 0, result.stderr)
        staging = json.loads(result.stdout)["compiler_rt"]["staging"]
        self.assertEqual(staging["unpack_phase_sha256"], hashlib.sha256(staging["unpack_phase"].encode()).hexdigest())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, build = root / "immutable-source", root / "build"
            (source / "compiler-rt/cmake").mkdir(parents=True)
            (source / "llvm").mkdir()
            original = source / "compiler-rt/cmake/module.cmake"
            original.write_text("original module")
            (source / "llvm/unused.cpp").write_text("uncopied source")
            (source / ".metadata").write_text("retained metadata")
            (source / "empty").mkdir()
            build.mkdir()
            script = "runHook() { :; }\n" + staging["unpack_phase"]
            env = dict(os.environ, src=str(source))
            subprocess.run(["bash", "-euo", "pipefail", "-c", script], cwd=build, env=env, check=True)
            writable = build / "source/compiler-rt/cmake/module.cmake"
            self.assertFalse((build / "source/compiler-rt").is_symlink())
            self.assertNotEqual(writable.stat().st_ino, original.stat().st_ino)
            writable.write_text("upstream postPatch applied")
            self.assertEqual(original.read_text(), "original module")
            for name in ("llvm", ".metadata", "empty"):
                self.assertTrue((build / "source" / name).is_symlink())
                self.assertEqual((build / "source" / name).resolve(), source / name)
            repeated = subprocess.run(["bash", "-euo", "pipefail", "-c", script],
                                      cwd=build, env=env, capture_output=True, check=False)
            self.assertNotEqual(repeated.returncode, 0)
            self.assertEqual(writable.read_text(), "upstream postPatch applied")


class LibraryPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ("sdk", "linker"):
            (self.root / name).mkdir()
        self.identities = {
            "stage": "compiler-rt",
            "sdk": {"drv": "/nix/store/sdk.drv", "out": str(self.root / "sdk")},
            "native_linker": {"drv": "/nix/store/linker.drv", "out": str(self.root / "linker")},
            "upstream": {"drv": "/nix/store/original.drv", "out": "/nix/store/original"},
            "candidate": {"drv": "/nix/store/patched.drv", "out": "/nix/store/patched"},
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
            args = ["--stage", "compiler-rt", "--puredarwin-source", str(self.root),
                    "--expected-sdk-output", str(self.root / "sdk"), "--expected-sdk-derivation", "/nix/store/sdk.drv",
                    "--expected-linker-output", str(self.root / "linker"), "--expected-linker-derivation", "/nix/store/linker.drv",
                    "--output-dir", str(self.root / output)]
            code = plan.main(args + (["--query-substituters"] if query else []))
            return code, json.loads(stdout.getvalue()), dry_run

    def test_default_is_plan_only_with_new_root_and_both_recipe_snapshots(self):
        code, report, run = self.run_plan()
        self.assertEqual((code, report["status"]), (0, "planned_not_built"))
        self.assertEqual(run.call_count, 1)
        self.assertIn("--offline", run.call_args.args[0])
        self.assertIn("--dry-run", run.call_args.args[0])
        self.assertEqual(report["commands"]["realize"][-2:], ["--out-link", str(self.root / "plan/result-compiler-rt")])
        for path in (plan.RECIPE, plan.linker.RECIPE):
            self.assertEqual((self.root / "plan" / path.name).read_bytes(), path.read_bytes())
        for name in ("sdk", "native_linker"):
            self.assertTrue(report[name]["identity_matches_expected"])
        for key in ("dependency_realization", "dependency_downloads", "fixture_linking", "guest_execution",
                    "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(report[key], "not_run")
        self.assertFalse(report["peak_space_qualified"])

    def test_explicit_metadata_plan_does_not_claim_download_or_space_qualification(self):
        code, report, run = self.run_plan(query=True)
        self.assertEqual(code, 0)
        self.assertNotIn("--offline", run.call_args.args[0])
        self.assertEqual(report["planning_mode"], "substituter_metadata")
        self.assertFalse(report["peak_space_qualified"])
        self.assertEqual(report["dependency_downloads"], "not_run")

    def test_expected_sdk_linker_and_stage_mismatch_block_before_dry_run(self):
        for name, key in (("sdk", "drv"), ("native_linker", "drv")):
            original = self.identities[name][key]
            self.identities[name][key] = "/different"
            code, report, run = self.run_plan(output=name)
            self.assertEqual((code, report["status"]), (2, "blocked"))
            run.assert_not_called()
            self.identities[name][key] = original
        self.identities["stage"] = "libsystem"
        code, _, run = self.run_plan(output="wrong-stage")
        self.assertEqual(code, 2)
        run.assert_not_called()

    def test_missing_realized_linker_and_existing_output_are_not_rebuilt_or_overwritten(self):
        (self.root / "linker").rmdir()
        code, _, run = self.run_plan()
        self.assertEqual(code, 2)
        run.assert_not_called()
        retained = (self.root / "plan/result.json").read_bytes()
        code, _, run = self.run_plan()
        self.assertEqual(code, 2)
        run.assert_not_called()
        self.assertEqual((self.root / "plan/result.json").read_bytes(), retained)

    def test_failed_plan_remains_nonzero(self):
        code, report, _ = self.run_plan(dry_code=1)
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertEqual(report["dry_run"]["exit_code"], 1)

    def test_evaluation_embeds_exact_recipes_and_stays_offline(self):
        with mock.patch.object(plan.base, "command", return_value="{}") as command:
            plan.evaluate(Path("/source"), "{ packages, nativeLinkerRecipe, stage }: {}", "p: {}", "compiler-rt")
        argv = command.call_args.args[0]
        self.assertIn('stage = "compiler-rt";', argv[-1])
        self.assertIn("nativeLinkerRecipe = (p: {});", argv[-1])
        for flag in ("--offline", "--no-write-lock-file", "--no-update-lock-file"):
            self.assertIn(flag, argv)


if __name__ == "__main__":
    unittest.main()
