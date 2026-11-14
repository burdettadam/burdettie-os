"""Real Nix rebind checks plus successful-receipt and live-integrity gates."""
import base64
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
import plan_puredarwin_staged_linker as plan
from test_stage_libtapi_source import PATCH, make_source


FIXTURE = '''let
  serial = value:
    if builtins.isList value then map serial value
    else if builtins.isAttrs value then
      if value ? outPath then value.outPath else builtins.mapAttrs (_: serial) value
    else value;
  mk = name: attrs: attrs // {
    outPath = "/nix/store/" + builtins.substring 0 32 (builtins.hashString "sha256"
      (builtins.toJSON (serial attrs))) + "-" + name;
    drvPath = (mk name attrs).outPath + ".drv";
    overrideAttrs = change: mk name (attrs // change (mk name attrs));
  };
  sdk = { drvPath = "/nix/store/sdk.drv"; outPath = "/nix/store/sdk"; };
  bootstrap = { outPath = "/nix/store/bootstrap-toolchain"; };
  zlib = { outPath = "/nix/store/zlib"; };
  tapi = mk "libtapi" {
    pname = "apple-libtapi"; version = "1300.6.5";
    src = { outPath = "/nix/store/full-source"; drvPath = "/nix/store/full-source.drv";
      rev = "640b4623929c923c0468143ff2a363a48665fa54";
      outputHash = "sha256-eXzQRB07AcH9nBgPoGpIQZbC4O/bIzkdbjJ442qM9VA="; };
    patches = [ PATCH_PATH ]; postPatch = "patchShebangs build.sh install.sh tools";
    buildPhase = "JOBS=$NIX_BUILD_CORES ./build.sh ./install.sh";
    dontInstall = true; dontUseCmakeConfigure = true;
    nativeBuildInputs = [ { pname = "python3"; outPath = "/nix/store/python3"; } ];
  };
  mkNative = libtapi: (mk "native-ld" {
    src = "/nix/store/narrow-native-source";
    nativeBuildInputs = [ bootstrap ]; buildInputs = [ zlib libtapi ];
    NIX_DARWIN_TOOLCHAIN_DIR = bootstrap.outPath + "/bin";
    configurePhase = "Release x86_64-apple-darwin20.4 SDK=${sdk.outPath}/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
      + " -DPUREDARWIN_LIBTAPI_INCLUDE_DIR=${libtapi.outPath}/include"
      + " -DPUREDARWIN_LIBTAPI_LIBRARY=${libtapi.outPath}/lib/libtapi.so";
    buildPhase = NATIVE_BUILD;
    installPhase = "install original ld and strip";
  }) // { override = args: (mkNative args.libtapi).overrideAttrs (_: { DRIFT }); };
in { native-ld = mkNative tapi; apple-sdk = sdk; }
'''


class StagedLinkerOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.nix = os.environ.get("NIX_BIN") or shutil.which("nix")
        if not cls.nix:
            raise RuntimeError("Set NIX_BIN: the real Nix rebind tests must not be skipped")

    def evaluate(self, *, drift="", build="ninja -C build-nix-native host_ld host_strip"):
        with mock.patch.object(plan.linker, "evaluate", return_value=({}, [])) as evaluate:
            plan.evaluate(Path("/unused"), plan.RECIPE.read_text(), plan.tapi.RECIPE.read_text(),
                          plan.linker.RECIPE.read_text(), plan.tapi.STAGER.read_text())
        with tempfile.NamedTemporaryFile(mode="w", suffix=".patch") as patch:
            patch.write(PATCH)
            patch.flush()
            fixture = FIXTURE.replace("PATCH_PATH", json.dumps(patch.name)).replace("DRIFT", drift)
            fixture = fixture.replace("NATIVE_BUILD", json.dumps(build))
            expression = "(" + evaluate.call_args.args[1] + ") (" + fixture + ")"
            # Only the synthetic package fixture needs its temporary patch file.
            return subprocess.run([self.nix, "--extra-experimental-features", "nix-command flakes",
                                   "eval", "--store", "dummy://", "--impure", "--json", "--expr", expression],
                                  capture_output=True, text=True, timeout=30, check=False)

    def test_exact_rebind_retains_bootstrap_source_sdk_and_flags(self):
        result = self.evaluate()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(set(report["unchanged_recipe_fields"]), plan.UNCHANGED_FIELDS)
        self.assertTrue(all(report["unchanged_recipe_fields"].values()))
        self.assertEqual(report["changed"], {"libtapi_build_inputs": 1, "libtapi_configure_paths": 2, "ninja_commands": 1})
        old, new = report["build_inputs"]["upstream"], report["build_inputs"]["candidate"]
        self.assertEqual(old[0], new[0])
        self.assertEqual(old[1], report["staged_libtapi"]["upstream"]["out"])
        self.assertEqual(new[1], report["staged_libtapi"]["candidate"]["out"])
        identities = [tuple(sorted(report[name].items())) for name in
                      ("upstream", "previous_bounded", "rebound_unbounded", "candidate")]
        self.assertEqual(len(set(identities)), 4)
        self.assertEqual(report["sdk"], report["staged_libtapi"]["sdk"])

    def test_unrelated_rebind_changes_or_missing_duplicate_job_command_fail(self):
        for change in (
            {"drift": 'nativeBuildInputs = [];'},
            {"drift": 'buildInputs = [ zlib ];'},
            {"drift": 'NIX_DARWIN_TOOLCHAIN_DIR = "/wrong-final-toolchain";'},
            {"drift": 'configurePhase = "Debug changed-target";'},
            {"drift": 'installPhase = "changed";'},
            {"drift": 'src = "/different-source";'},
            {"build": "ninja another-target"},
            {"build": "ninja -C build-nix-native host_ld host_strip\nninja -C build-nix-native host_ld host_strip"},
        ):
            with self.subTest(change=change):
                self.assertNotEqual(self.evaluate(**change).returncode, 0)


class StagedLinkerReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = make_source(self.root)
        self.sdk = {"drv": "/nix/store/sdk.drv", "out": str(self.root / "sdk")}
        Path(self.sdk["out"]).mkdir()
        self.candidate = {"drv": "/nix/store/staged-libtapi.drv", "out": str(self.root / "libtapi")}
        self.upstream = {"drv": "/nix/store/original-libtapi.drv", "out": "/nix/store/original-libtapi"}
        self.source_id = {"revision": plan.base.PUREDARWIN_REVISION, "lock": {"sha256": plan.base.LOCK_SHA256}, "submodules": []}
        elf = bytearray(20)
        elf[:4], elf[4:6], elf[16:18], elf[18:20] = b"\x7fELF", b"\x02\x01", b"\x03\x00", b"\x3e\x00"
        contents = {"include/tapi/" + name: ("header " + name).encode() for name in plan.HEADERS}
        contents["lib/libtapi.so.12"] = bytes(elf) + b"owned synthetic library fixture"
        output = Path(self.candidate["out"])
        for relative, data in contents.items():
            path = output / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        (output / "lib/libtapi.so").symlink_to("libtapi.so.12")
        self.inspection = {
            "files": {name: {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()} for name, data in contents.items()},
            "symlinks": {"lib/libtapi.so": "libtapi.so.12"}, "elf_files": ["lib/libtapi.so.12"],
            "references": ["/nix/store/native-runtime"], "source_reference_absent": True,
            "temporary_build_paths_absent": True, "headers_are_regular_files": True,
        }
        nar = "sha256-" + base64.b64encode(hashlib.sha256(b"synthetic NAR identity").digest()).decode()
        self.path_info = {output.as_posix(): {"deriver": self.candidate["drv"], "narHash": nar, "narSize": 1234,
                                            "references": list(self.inspection["references"])}}
        raw = (json.dumps(self.path_info) + "\n").encode()
        self.build_dir = self.root / "successful-build"
        self.build_dir.mkdir()
        (self.build_dir / "output-path-info.json").write_bytes(raw)
        self.inspection["registered_output"] = {
            "path": str(output), "deriver": self.candidate["drv"], "nar_hash": nar, "nar_size": 1234,
            "path_info_sha256": hashlib.sha256(raw).hexdigest()}
        source_inventory = plan.tapi.staging.inspect_source(self.source)["source_inventory_sha256"]
        self.original_plan = {
            "schema_version": 1, "scope": "libtapi_source_staging_plan_only", "status": "planned_not_built",
            "dry_run": {"exit_code": 0, "timed_out": False}, "planning_mode": "substituter_metadata",
            "substituter_queries_enabled": True, "source": self.source_id,
            "sdk": dict(self.sdk, identity_matches_expected=True), "burdettie_revision": "a" * 40,
            "recipe_sha256": plan.base.sha256(plan.tapi.RECIPE), "stager_sha256": plan.base.sha256(plan.tapi.STAGER),
            "unchanged_recipe_fields": {key: True for key in plan.tapi.UNCHANGED_FIELDS},
            "candidate": self.candidate, "upstream": self.upstream,
            "full_libtapi_source": {"drv": "/nix/store/source.drv", "out": str(self.source), "rev": "pinned",
                                     "output_hash": "pinned-hash", "retained_unchanged": True,
                                     "source_inventory_sha256": source_inventory},
            "unpack_phase_sha256": "0" * 64, "upstream_patch_files": [],
            "upstream_build_phase_sha256": "1" * 64, "upstream_post_patch_sha256": "2" * 64,
        }
        self.built = {
            "schema_version": 1, "scope": "staged_libtapi_build", "status": "staged_libtapi_built",
            "realization_command_succeeded": True, "output_inspection_status": "pass",
            "burdettie_revision": self.original_plan["burdettie_revision"], "source_revision": self.source_id["revision"],
            "helper_sha256": plan.base.sha256(Path(plan.tapi.__file__)),
            "phases": [{"name": "staged-libtapi-build", "status": "pass", "exit_code": 0}],
            "candidate": copy.deepcopy(self.candidate), "upstream": copy.deepcopy(self.upstream),
            "full_libtapi_source": copy.deepcopy(self.original_plan["full_libtapi_source"]), "realized_output": str(output),
            "staging_receipt": {"status": "staged", "source_unchanged": True,
                                "source_inventory_matches_inspected_plan": True, "source_inventory_sha256": source_inventory},
            "output_inspection": self.inspection,
        }
        self.records = {
            "sdk": dict(self.sdk), "candidate": {"drv": "/nix/store/new-native.drv", "out": "/nix/store/new-native"},
            "unchanged_recipe_fields": {key: True for key in plan.UNCHANGED_FIELDS},
            "staged_libtapi": {key: copy.deepcopy(self.original_plan[key]) for key in
                ("candidate", "upstream", "full_libtapi_source", "unchanged_recipe_fields", "stager_sha256",
                 "unpack_phase_sha256", "upstream_patch_files", "upstream_build_phase_sha256", "upstream_post_patch_sha256")},
        }
        self.records["staged_libtapi"]["sdk"] = dict(self.sdk)
        self.current = copy.deepcopy(self.path_info)
        self.current[self.sdk["out"]] = {"deriver": self.sdk["drv"], "narHash": nar, "narSize": 4567, "references": []}

    def run_plan(self, *, name="new-plan", query=False, verify_code=0, dry_code=0, wrong_plan_hash=False):
        plan_path, build_path = self.root / "old-plan.json", self.build_dir / "result.json"
        plan_path.write_text(json.dumps(self.original_plan))
        plan_hash = plan.base.sha256(plan_path)
        self.built["plan_result_sha256"] = "f" * 64 if wrong_plan_hash else plan_hash
        build_path.write_text(json.dumps(self.built))

        def metadata(argv, **kwargs):
            self.assertIn("--offline", argv)
            self.assertNotIn("--repair", argv)
            self.assertNotIn("--no-trust", argv)
            self.assertNotIn("build", argv)
            if "path-info" in argv:
                return subprocess.CompletedProcess(argv, 0, json.dumps(self.current).encode(), b"")
            self.assertIn("verify", argv)
            return subprocess.CompletedProcess(argv, verify_code, b"", b"verification failure" if verify_code else b"")

        with mock.patch.object(plan.base.platform, "system", return_value="Linux"), \
                mock.patch.object(plan.base.platform, "machine", return_value="x86_64"), \
                mock.patch.object(plan.base.shutil, "which", return_value="nix"), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": "local"}), \
                mock.patch.object(plan.base, "source_identity", return_value=copy.deepcopy(self.source_id)), \
                mock.patch.object(plan.base, "command", side_effect=lambda a: "nix (Nix) 2.28.3" if a[0] == "nix" else "b" * 40), \
                mock.patch.object(plan, "evaluate", return_value=(copy.deepcopy(self.records), ["offline-eval"])) as evaluate, \
                mock.patch.object(plan.subprocess, "run", side_effect=metadata) as reads, \
                mock.patch.object(plan.linker, "dry_run", return_value={"exit_code": dry_code, "timed_out": False}) as dry, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            argv = ["--puredarwin-source", str(self.root), "--libtapi-plan", str(plan_path), "--libtapi-plan-sha256", plan_hash,
                    "--libtapi-build-result", str(build_path), "--libtapi-build-result-sha256", plan.base.sha256(build_path),
                    "--expected-sdk-output", self.sdk["out"], "--expected-sdk-derivation", self.sdk["drv"],
                    "--output-dir", str(self.root / name)]
            code = plan.main(argv + (["--query-substituters"] if query else []))
            return code, json.loads(stdout.getvalue()), reads, dry, evaluate

    def test_successful_receipts_still_require_fresh_integrity_before_plan_only_dry_run(self):
        code, report, reads, dry, _ = self.run_plan()
        self.assertEqual((code, report["status"]), (0, "planned_not_built"))
        self.assertEqual(reads.call_count, 2)
        self.assertEqual(dry.call_count, 1)
        self.assertIn("--offline", dry.call_args.args[0])
        self.assertIn("--dry-run", dry.call_args.args[0])
        self.assertTrue(report["current_output_inspection"]["matches_successful_receipt"])
        self.assertTrue(report["source_inventory_matches_successful_build"])
        self.assertEqual(report["successful_libtapi"]["burdettie_revision"], "a" * 40)
        self.assertEqual(report["burdettie_revision"], "b" * 40)
        self.assertEqual(report["read_checks"]["store-verify"]["exit_code"], 0)
        for field in ("dependency_realization", "dependency_downloads", "linking", "guest_execution",
                      "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(report[field], "not_run")
        self.assertFalse(report["peak_space_qualified"])

    def test_pending_or_failed_build_cannot_be_used_even_when_output_exists(self):
        for status in ("running", "fail", "planned_not_built"):
            self.built["status"] = status
            code, report, reads, dry, evaluate = self.run_plan(name=status)
            self.assertEqual((code, report["status"]), (2, "blocked"))
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()

    def test_build_must_bind_the_exact_plan_and_unchanged_recipe(self):
        code, _, reads, dry, _ = self.run_plan(wrong_plan_hash=True)
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        self.original_plan["stager_sha256"] = "f" * 64
        code, _, reads, dry, _ = self.run_plan(name="changed-stager")
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()

    def test_incomplete_successful_phase_or_registration_contract_is_blocked(self):
        original = copy.deepcopy(self.built)
        for name, mutate in (
            ("phase", lambda b: b.update(phases=[])),
            ("deriver", lambda b: b["output_inspection"]["registered_output"].update(deriver="/wrong")),
            ("nar", lambda b: b["output_inspection"]["registered_output"].update(nar_hash="sha256-invalid")),
            ("source-observation", lambda b: b["staging_receipt"].update(source_unchanged=False)),
        ):
            self.built = copy.deepcopy(original)
            mutate(self.built)
            code, _, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual(code, 2)
            reads.assert_not_called()
            dry.assert_not_called()

    def test_re_evaluated_candidate_or_source_inventory_mismatch_blocks_use(self):
        original = self.records["staged_libtapi"]["candidate"]["drv"]
        self.records["staged_libtapi"]["candidate"]["drv"] = "/different"
        code, _, reads, dry, _ = self.run_plan()
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        self.records["staged_libtapi"]["candidate"]["drv"] = original
        (self.source / ".metadata").chmod(0o644)
        (self.source / ".metadata").write_text("changed input")
        code, _, reads, dry, _ = self.run_plan(name="changed-source")
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()

    def test_fresh_nar_or_sdk_deriver_mismatch_is_failure(self):
        original = copy.deepcopy(self.current)
        for name, path, key, value in (
            ("nar", self.candidate["out"], "narSize", 999),
            ("sdk-deriver", self.sdk["out"], "deriver", "/wrong-sdk.drv"),
        ):
            self.current = copy.deepcopy(original)
            self.current[path][key] = value
            code, report, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, report["status"]), (1, "fail"))
            self.assertEqual(reads.call_count, 1)
            dry.assert_not_called()

    def test_store_verification_failure_is_retained_and_stops_the_dry_run(self):
        code, report, _, dry, _ = self.run_plan(verify_code=1)
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertEqual(report["read_checks"]["store-verify"]["exit_code"], 1)
        self.assertEqual((self.root / "new-plan/store-verify.stderr").read_bytes(), b"verification failure")
        dry.assert_not_called()

    def test_changed_file_or_escaping_output_link_fails_even_with_matching_metadata(self):
        header = Path(self.candidate["out"]) / "include/tapi/tapi.h"
        original = header.read_bytes()
        header.write_bytes(b"changed installed header")
        code, report, _, dry, _ = self.run_plan()
        self.assertEqual((code, report["status"]), (1, "fail"))
        dry.assert_not_called()
        header.write_bytes(original)
        link = Path(self.candidate["out"]) / "lib/libtapi.so"
        link.unlink()
        link.symlink_to(self.source / "build.sh")
        code, report, _, dry, _ = self.run_plan(name="escaping-link")
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertIn("escapes", report["error"])
        dry.assert_not_called()

    def test_changed_raw_path_info_and_existing_outputs_are_preserved_and_rejected(self):
        info = self.build_dir / "output-path-info.json"
        info.write_text("{}")
        code, _, reads, dry, _ = self.run_plan()
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        retained = (self.root / "new-plan/result.json").read_bytes()
        code, _, reads, dry, _ = self.run_plan()
        self.assertEqual(code, 2)
        self.assertEqual((self.root / "new-plan/result.json").read_bytes(), retained)

    def test_substituter_metadata_is_explicit_and_failed_dry_run_stays_failure(self):
        code, report, _, dry, _ = self.run_plan(query=True)
        self.assertEqual(code, 0)
        self.assertNotIn("--offline", dry.call_args.args[0])
        self.assertEqual(report["planning_mode"], "substituter_metadata")
        code, report, _, _, _ = self.run_plan(name="dry-failure", dry_code=1)
        self.assertEqual((code, report["status"]), (1, "fail"))

    def test_duplicate_receipt_fields_are_rejected(self):
        path = self.root / "duplicate.json"
        path.write_text('{"status":"fail","status":"staged_libtapi_built"}')
        with self.assertRaisesRegex(plan.base.BuildError, "Duplicate"):
            plan.read_json(path, plan.base.sha256(path))

    def test_production_evaluation_stays_offline_and_lock_preserving(self):
        with mock.patch.object(plan.base, "command", return_value="{}") as command:
            plan.evaluate(Path("/pinned-source"), "args: {}", "args: {}", "p: {}", "# script\n")
        argv = command.call_args.args[0]
        for flag in ("--offline", "--no-update-lock-file", "--no-write-lock-file"):
            self.assertIn(flag, argv)
        self.assertNotIn("--impure", argv)
        self.assertEqual(argv[argv.index("allow-import-from-derivation") + 1], "false")


if __name__ == "__main__":
    unittest.main()
