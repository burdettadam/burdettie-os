"""Small synthetic Nix/receipt fixtures; no live package or build is qualified."""
import contextlib
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan_puredarwin_staged_libraries as plan
import test_plan_puredarwin_staged_linker as staged_tests


GRAPH = '''
  originalNative = mkNative tapi;
  mkCross = ld: drift: (mk "darwin-cross-toolchain-nix" ({
    name = "darwin-cross-toolchain-nix"; buildCommand = "join original wrappers";
    paths = map (name: "/nix/store/" + builtins.substring 0 32 (builtins.hashString "sha256"
      (name + (if builtins.elem name [ "clang" "clang++" "ld" ] then ld.outPath else "unchanged")))
      + "-x86_64-apple-darwin20.4-" + name)
      [ "clang" "clang++" "ar" "ranlib" "strip" "nm" "objdump" "otool" "dsymutil" "install_name_tool" "lipo" "ld" "xcrun" "dsymutil" ];
  } // drift)) // { override = args: mkCross args.nativeLd { CROSS_DRIFT }; };
  cross = mkCross originalNative {};
  llvm = { outPath = "/nix/store/full-llvm-source"; drvPath = "/nix/store/full-llvm-source.drv";
    rev = "llvmorg-21.1.8"; outputHash = "unchanged-complete-source"; };
  mkRt = cc: drift: (mk "compiler-rt" ({
    src = llvm; postPatch = "original visibility patch";
    configurePhase = "Release x86_64-apple-darwin20.4 macosx26.5 ${cc.outPath}";
    buildPhase = RT_BUILD; installPhase = "ninja -C build install";
    nativeBuildInputs = []; buildInputs = [];
  } // drift)) // { override = args: mkRt args.darwinCrossToolchain { RT_DRIFT }; };
  crt = mkRt cross {};
  mkSystem = ld: cc: rt: drift: (mk "libsystem" ({
    src = "/nix/store/libsystem-source"; postPatch = "original patch";
    nativeBuildInputs = [ cc ]; buildInputs = [ zlib ];
    configurePhase = "Debug x86_64 ${ld.outPath} ${cc.outPath} ${rt.outPath}";
    buildPhase = "ninja -C build-nix  libSystem_B_stub dyld";
    installPhase = "install original libSystem/dyld";
    NIX_DARWIN_HOST = "x86_64-apple-darwin20.4";
    NIX_DARWIN_TOOLCHAIN_DIR = cc.outPath + "/bin";
  } // drift)) // { override = args: mkSystem args.nativeLd args.darwinCrossToolchain args.compilerRt { SYSTEM_DRIFT }; };
in { native-ld = originalNative; apple-sdk = sdk; darwin-cross-toolchain = cross;
  compiler-rt = crt; libsystem = mkSystem originalNative cross crt {}; }
'''


class StagedLibrariesOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.nix = os.environ.get("NIX_BIN") or shutil.which("nix")
        if not cls.nix:
            raise RuntimeError("Set NIX_BIN: real Nix composition tests must not be skipped")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.patch = Path(temporary.name) / "upstream.patch"
        self.patch.write_text(staged_tests.PATCH)

    def evaluate(self, stage="libsystem", *, cross_drift="", rt_drift="", system_drift="", narrow=False,
                 rt_build="ninja -C build", native_build="ninja -C build-nix-native host_ld host_strip"):
        snapshots = {key: path.read_bytes() for key, path in plan.FILES.items()}
        with mock.patch.object(plan.base, "command", return_value="{}") as call:
            plan.evaluate(Path("/synthetic-source"), snapshots, stage)
        expression = call.call_args.args[0][-1]
        # Replace longer placeholders first: the native fixture's DRIFT must
        # not consume the names of the downstream negative controls.
        fixture = staged_tests.FIXTURE.replace("in { native-ld = mkNative tapi; apple-sdk = sdk; }", GRAPH)
        for before, after in (("CROSS_DRIFT", cross_drift), ("RT_DRIFT", rt_drift),
                              ("SYSTEM_DRIFT", system_drift), ("DRIFT", ""),
                              ("NATIVE_BUILD", json.dumps(native_build)), ("RT_BUILD", json.dumps(rt_build))):
            fixture = fixture.replace(before, after)
        if narrow:
            fixture = fixture.replace("compiler-rt = crt; libsystem = mkSystem originalNative cross crt {};",
                                      'compiler-rt = throw "unused compiler-rt"; libsystem = throw "unused libSystem";')
        fixture = fixture.replace("PATCH_PATH", json.dumps(str(self.patch)))
        return subprocess.run([self.nix, "--extra-experimental-features", "nix-command flakes",
                               "eval", "--store", "dummy://", "--impure", "--json", "--expr",
                               "(" + expression + ") (" + fixture + ")"],
                              capture_output=True, text=True, timeout=30, check=False)

    def test_all_stages_keep_four_distinct_identities_and_the_exact_bounded_linker(self):
        candidate = cross = None
        for stage in plan.libraries.STAGES:
            with self.subTest(stage=stage):
                result = self.evaluate(stage)
                self.assertEqual(result.returncode, 0, result.stderr)
                value = json.loads(result.stdout)
                self.assertEqual(value["stage"], stage)
                for key in ("drv", "out"):
                    self.assertEqual(len({value[name][key] for name in
                                          ("upstream", "previous_bounded", "rebound_unbounded", "candidate")}), 4)
                self.assertTrue(all(item in (True, None) for item in value["preserved"].values()))
                if candidate is None:
                    candidate = value["native_linker"]["candidate"]
                    cross = value["cross_toolchain"]["candidate"]
                self.assertEqual(value["native_linker"]["candidate"], candidate)
                self.assertEqual(value["cross_toolchain"]["candidate"], cross)
                self.assertEqual(value["native_linker"]["sdk"], value["sdk"])
                if stage != "cross-toolchain":
                    self.assertTrue(value["compiler_rt"]["full_source"]["retained_unchanged"])
                    self.assertEqual(value["compiler_rt"]["staging"]["copied"], ["compiler-rt"])

    def test_cross_stage_does_not_evaluate_llvm_or_libsystem(self):
        result = self.evaluate("cross-toolchain", narrow=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertIsNone(value["compiler_rt"])
        self.assertIsNone(value["baseline_checks"]["compiler_rt"])
        self.assertIsNone(value["baseline_checks"]["libsystem"])

    def test_source_mode_target_install_and_job_count_drift_fail_closed(self):
        for change in (
            {"cross_drift": 'buildCommand = "changed wrappers";'},
            {"rt_drift": 'src = { outPath = "/changed-llvm"; };'},
            {"rt_drift": 'configurePhase = "Debug changed target";'},
            {"rt_drift": 'installPhase = "changed install";'},
            {"system_drift": 'src = "/changed-source";'},
            {"system_drift": 'configurePhase = "Release changed target";'},
            {"system_drift": 'NIX_DARWIN_HOST = "changed-target";'},
            {"system_drift": 'installPhase = "changed install";'},
            {"rt_build": "ninja another-directory"},
            {"rt_build": "ninja -C build\nninja -C build"},
            {"native_build": "ninja -j1 -C build-nix-native host_ld host_strip"},
        ):
            with self.subTest(change=change):
                self.assertNotEqual(self.evaluate(**change).returncode, 0)


def owned_elf(kind=3):
    value = bytearray(64)
    value[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HHI", value, 16, kind, 62, 1)
    struct.pack_into("<H", value, 52, 64)
    return bytes(value) + b"owned synthetic header, never executed"


def owned_dynamic_elf(*, interpreter="/synthetic-loader/bin/ld-linux", needed=("libtapi.so.12",), rpath=None, runpath=None):
    strings = bytearray(b"\0")
    indices = {}
    for value in (*needed, rpath, runpath):
        if value is not None and value not in indices:
            indices[value] = len(strings)
            strings.extend(value.encode() + b"\0")
    tags = [(5, 0x400000 + 768), (10, len(strings))] + [(1, indices[value]) for value in needed]
    tags += ([] if rpath is None else [(15, indices[rpath])]) + ([] if runpath is None else [(29, indices[runpath])])
    tags.append((0, 0))
    value = bytearray(768 + len(strings))
    value[:64] = owned_elf()[:64]
    struct.pack_into("<Q", value, 32, 64)
    struct.pack_into("<HH", value, 54, 56, 3)
    interp = interpreter.encode() + b"\0"
    struct.pack_into("<IIQQQQQQ", value, 64, 1, 5, 0, 0x400000, 0, len(value), len(value), 4096)
    struct.pack_into("<IIQQQQQQ", value, 120, 3, 4, 256, 0x400100, 0, len(interp), len(interp), 1)
    struct.pack_into("<IIQQQQQQ", value, 176, 2, 4, 512, 0x400200, 0, 16 * len(tags), 16 * len(tags), 8)
    value[256:256 + len(interp)] = interp
    for i, item in enumerate(tags):
        struct.pack_into("<qQ", value, 512 + 16 * i, *item)
    value[768:] = strings
    return bytes(value)


class NativeElfBindingTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.tapi = self.root / "staged"
        (self.tapi / "lib").mkdir(parents=True)
        (self.tapi / "lib/libtapi.so.12git").write_bytes(owned_elf())
        (self.tapi / "lib/libtapi.so.12").symlink_to("libtapi.so.12git")
        self.elf = self.root / "ld.real"

    def bind(self, **kwargs):
        self.elf.write_bytes(owned_dynamic_elf(**kwargs))
        return plan.inspection.tapi_binding(self.elf, str(self.tapi))

    def test_runpath_rpath_direct_needed_and_actual_git_soname_resolve_to_staged_bytes(self):
        directory = str(self.tapi / "lib")
        for args, mode in (({"runpath": directory}, "DT_RUNPATH"), ({"rpath": directory}, "DT_RPATH"),
                           ({"needed": (str(self.tapi / "lib/libtapi.so.12"),)}, "absolute_DT_NEEDED"),
                           ({"needed": ("libtapi.so.12git",), "runpath": directory}, "DT_RUNPATH")):
            value = self.bind(**args)
            self.assertEqual(value["resolution_mode"], mode)
            self.assertEqual(value["resolved_path"], str(self.tapi / "lib/libtapi.so.12git"))
            self.assertEqual(value["sha256"], plan.base.sha256(self.tapi / "lib/libtapi.so.12git"))

    def test_first_search_match_and_runpath_precedence_are_enforced(self):
        alternate = self.root / "other"
        alternate.mkdir()
        (alternate / "libtapi.so.12").write_bytes(owned_elf())
        with self.assertRaisesRegex(plan.base.BuildError, "outside"):
            self.bind(runpath=str(alternate) + ":" + str(self.tapi / "lib"))
        value = self.bind(rpath=str(alternate), runpath=str(self.tapi / "lib"))
        self.assertEqual(value["resolution_mode"], "DT_RUNPATH")
        self.assertEqual(value["search_directories"], [str(self.tapi / "lib")])

    def test_missing_or_ambiguous_dependencies_and_unresolved_search_modes_are_rejected(self):
        for args in ({}, {"needed": ("libz.so.1",)}, {"needed": ("libtapi.so.12", "libtapi.so.12git")},
                     {"runpath": "$ORIGIN/../lib"}, {"runpath": ":" + str(self.tapi / "lib")},
                     {"runpath": str(self.root / "missing")}, {"needed": ("relative/libtapi.so.12",)}):
            with self.subTest(args=args), self.assertRaises(plan.base.BuildError):
                self.bind(**args)

    def test_truncated_program_headers_dynamic_ranges_and_string_tables_are_rejected(self):
        original = owned_dynamic_elf(runpath=str(self.tapi / "lib"))
        for mutate in (
            lambda b: struct.pack_into("<Q", b, 32, len(b) + 1),
            lambda b: struct.pack_into("<H", b, 56, 129),
            lambda b: struct.pack_into("<Q", b, 520, 0x800000),
            lambda b: struct.pack_into("<Q", b, 536, 1024 * 1024 + 1),
            lambda b: struct.pack_into("<Q", b, 552, len(b)),
            lambda b: b.__setitem__(slice(-1, None), b"x"),
        ):
            data = bytearray(original)
            mutate(data)
            self.elf.write_bytes(data)
            with self.assertRaises(plan.base.BuildError):
                plan.inspection.elf_dynamic(self.elf)

    def test_actual_loader_resolution_is_normalized_and_cannot_name_another_library(self):
        value = self.bind(runpath=str(self.tapi / "lib"))
        for address in ("1234", "abcdef"):
            data = ("\tlibtapi.so.12 => " + str(self.tapi / "lib/libtapi.so.12") + " (0x" + address + ")\n").encode()
            self.assertEqual(plan.inspection.observed_tapi_resolution(data, value)["resolved_path"], value["resolved_path"])
        other = self.root / "libtapi.so.12"
        other.write_bytes(owned_elf())
        with self.assertRaises(plan.base.BuildError):
            plan.inspection.observed_tapi_resolution(("libtapi.so.12 => " + str(other) + " (0x1)\n").encode(), value)

    def test_host_probe_output_limit_is_enforced_by_a_real_child_without_a_compiler(self):
        directory = self.root / "bounded-output"
        directory.mkdir()
        report = {}
        with self.assertRaises(plan.base.BuildError):
            plan.inspection.checked([sys.executable, "-c", "import os; os.write(1,b'x'*70000)"],
                                    "owned-output-probe", directory, report, bounded=True, timeout=5)
        self.assertLessEqual((directory / "owned-output-probe.stdout").stat().st_size, plan.inspection.OUTPUT_LIMIT)
        self.assertTrue(report["read_checks"]["owned-output-probe"]["output_limit_reached"])


class StagedLibraryReceiptTests(unittest.TestCase):
    def setUp(self):
        self.fixture = staged_tests.StagedLinkerReceiptTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.sdk = self.fixture.sdk
        self.loader = self.root / "loader-output"
        (self.loader / "lib").mkdir(parents=True)
        self.interpreter = self.loader / "lib/ld-linux-x86-64.so.2"
        self.interpreter.write_bytes(owned_elf())
        self.interpreter.chmod(0o755)
        version_header = Path(self.fixture.candidate["out"]) / "include/tapi/Version.inc"
        version_header.write_bytes(b"#define TAPI_VERSION 1300.6.5\n#define TAPI_VERSION_MAJOR 1300U\n"
                                   b"#define TAPI_VERSION_MINOR 6U\n#define TAPI_VERSION_PATCH 5U\n")
        self.fixture.inspection["files"]["include/tapi/Version.inc"] = {
            "bytes": version_header.stat().st_size, "sha256": plan.base.sha256(version_header)}
        self.native_out = self.root / "native-output"
        (self.native_out / "bin").mkdir(parents=True)
        for name, data in (("ld", b'#!/bin/sh\nexec "$(dirname "$0")/ld.real" "$@"\n'),
                           ("ld.real", owned_dynamic_elf(interpreter=str(self.interpreter),
                            runpath=str(Path(self.fixture.candidate["out"]) / "lib"))), ("strip", owned_elf(2))):
            path = self.native_out / "bin" / name
            path.write_bytes(data)
            path.chmod(0o755)
        self.native_candidate = {"drv": "/nix/store/native-candidate.drv", "out": str(self.native_out)}
        self.dependencies = {
            "libtapi": plan.native.registered_identity(self.fixture.path_info[self.fixture.candidate["out"]], self.fixture.candidate["out"]),
            "sdk": plan.native.registered_identity(self.fixture.current[self.sdk["out"]], self.sdk["out"]),
        }
        self.native_info = {str(self.native_out): {"deriver": self.native_candidate["drv"],
                            "narHash": self.dependencies["libtapi"]["nar_hash"], "narSize": 9876,
                            "references": [self.fixture.candidate["out"], str(self.loader)]}}
        self.fixture.current[str(self.loader)] = {"deriver": "/nix/store/owned-loader.drv",
                                                  "narHash": self.dependencies["sdk"]["nar_hash"],
                                                  "narSize": 7890, "references": []}
        self.native_plan_dir, self.native_build_dir = self.root / "prior-native-plan", self.root / "prior-native-build"
        self.native_plan_dir.mkdir()
        self.native_build_dir.mkdir()
        self.verify_code = self.smoke_code = 0
        self.smoke_timeout = False
        self.smoke_bytes = (b"@(#)PROGRAM:ld  PROJECT:ld64-LD64_VERSION_NUM\nBUILD synthetic\n"
                            b"configured to support archs: x86_64 arm64\n"
                            b"TAPI support using: TAPI version 1300.6.5 (1300.6.5)\n")
        self.loader_code = 0
        self.loader_target = Path(self.fixture.candidate["out"]) / "lib/libtapi.so.12"
        self.mutate_during_smoke = False
        with mock.patch.object(plan.inspection.subprocess, "run", side_effect=self.reads):
            expected = plan.inspection.inspect_native_output(self.native_candidate, self.dependencies,
                       self.fixture.upstream["out"], self.native_build_dir, {})
        old_plan = self.native_plan_dir / "libtapi-plan.json"
        old_plan.write_text(json.dumps(self.fixture.original_plan))
        self.fixture.built["plan_result_sha256"] = plan.base.sha256(old_plan)
        old_build = self.native_plan_dir / "libtapi-build-result.json"
        old_build.write_text(json.dumps(self.fixture.built))
        (self.native_plan_dir / "libtapi-output-path-info.json").write_bytes(
            (self.fixture.build_dir / "output-path-info.json").read_bytes())
        self.native_plan = {
            "schema_version": 1, "scope": "staged_libtapi_native_linker_plan_only", "status": "planned_not_built",
            "planning_mode": "substituter_metadata", "substituter_queries_enabled": True,
            "dry_run": {"exit_code": 0, "timed_out": False}, "source": self.fixture.source_id,
            "sdk": dict(self.sdk, identity_matches_expected=True), "burdettie_revision": "c" * 40,
            "recipe_sha256": plan.base.sha256(plan.native.RECIPE),
            **{key: plan.base.sha256(plan.FILES[key]) for key in ("libtapi_recipe_sha256", "native_linker_recipe_sha256",
                                                               "stager_sha256", "libtapi_plan_helper_sha256")},
            "upstream": {"drv": "/nix/store/native-original.drv", "out": "/nix/store/native-original"},
            "previous_bounded": {"drv": "/nix/store/native-old-bounded.drv", "out": "/nix/store/native-old-bounded"},
            "rebound_unbounded": {"drv": "/nix/store/native-rebound.drv", "out": "/nix/store/native-rebound"},
            "candidate": self.native_candidate, "staged_libtapi": self.fixture.records["staged_libtapi"],
            "unchanged_recipe_fields": {key: True for key in plan.native.UNCHANGED_FIELDS},
            "build_inputs": {"upstream": [self.fixture.upstream["out"]], "candidate": [self.fixture.candidate["out"]]},
            "configure_phase_sha256": {"upstream": "a" * 64, "candidate": "b" * 64},
            "build_phase_sha256": {"upstream": "c" * 64, "candidate": "d" * 64},
            "ninja_patch_sha256": "e" * 64,
            "changed": {"libtapi_build_inputs": 1, "libtapi_configure_paths": 2, "ninja_commands": 1},
            "source_inventory_matches_successful_build": True,
            "current_output_inspection": {"matches_successful_receipt": True},
            "read_checks": {key: {"exit_code": 0, "timed_out": False} for key in ("path-info", "store-verify")},
            "current_registered_outputs": self.dependencies,
            "successful_libtapi": {"plan_sha256": plan.base.sha256(old_plan), "build_result_sha256": plan.base.sha256(old_build),
                                   "burdettie_revision": self.fixture.original_plan["burdettie_revision"],
                                   "registered_output": self.fixture.inspection["registered_output"]},
        }
        self.native_plan["commands"] = plan.linker.build_commands(self.native_candidate, self.native_plan_dir, True)
        fields = ("recipe_sha256", "libtapi_recipe_sha256", "native_linker_recipe_sha256", "stager_sha256",
                  "libtapi_plan_helper_sha256", "source", "sdk", "successful_libtapi", *plan.NATIVE_FIELDS)
        self.native_build = {key: copy.deepcopy(self.native_plan[key]) for key in fields}
        self.native_build.update({
            "schema_version": 1, "scope": plan.inspection.BUILD_SCOPE, "status": plan.inspection.BUILD_SUCCESS,
            "realization_command_succeeded": True, "output_inspection_status": "pass",
            "started_at": "2000-01-01T00:00:00Z", "finished_at": "2000-01-01T00:00:01Z",
            "plan_burdettie_revision": "c" * 40, "burdettie_revision": "d" * 40,
            "helper_sha256": plan.base.sha256(Path(plan.native.__file__)),
            "output_inspector_sha256": plan.base.sha256(Path(plan.inspection.__file__)),
            "driver_sha256": hashlib.sha256(b"owned synthetic reviewed driver").hexdigest(),
            "phases": [{"name": plan.inspection.BUILD_PHASE, "exit_code": 0, "status": "pass", "timed_out": False}],
            "realized_output": str(self.native_out), "output_inspection": expected,
        })
        realization = plan.inspection.NIX + self.native_plan["commands"]["realize"][len(plan.linker.NIX):]
        realization[0], realization[-1] = "/nix/store/owned-nix/bin/nix", str(self.native_build_dir / "result-linker")
        self.native_build["realization"] = {"argv": realization, "nix_executable": realization[0], "out_link": realization[-1],
                                             "out_link_was_absent": True}
        self.native_build["phases"][0]["argv"] = list(realization)
        self.driver_hash = self.native_build["driver_sha256"]
        self.records = {"stage": "cross-toolchain", "sdk": dict(self.sdk),
                        "native_linker": {**{key: copy.deepcopy(self.native_plan[key]) for key in plan.NATIVE_FIELDS}, "sdk": dict(self.sdk)},
                        "preserved": {key: (None if key in ("baseline_compiler_rt", "baseline_libsystem") else True) for key in plan.PRESERVED}}
        for name in ("upstream", "previous_bounded", "rebound_unbounded", "candidate"):
            self.records[name] = {"drv": "/nix/store/cross-" + name + ".drv", "out": "/nix/store/cross-" + name}

    def reads(self, argv, **kwargs):
        if "path-info" in argv:
            self.assertIn("--offline", argv)
            self.assertEqual(argv[argv.index("--store") + 1], "local")
            data = self.native_info if str(self.native_out) in argv else self.fixture.current
            return subprocess.CompletedProcess(argv, 0, json.dumps(data).encode(), b"")
        if "verify" in argv:
            self.assertIn("--offline", argv)
            for forbidden in ("--repair", "--no-trust", "--no-contents", "--no-check-sigs"):
                self.assertNotIn(forbidden, argv)
            return subprocess.CompletedProcess(argv, self.verify_code, b"", b"verify failure" if self.verify_code else b"")
        self.assertEqual(kwargs["env"], {"PATH": "/usr/bin:/bin", "LC_ALL": "C"})
        self.assertEqual(kwargs["timeout"], 10)
        self.assertTrue(callable(kwargs["preexec_fn"]))

        def completed(code, stdout, stderr):
            kwargs["stdout"].write(stdout)
            kwargs["stderr"].write(stderr)
            return subprocess.CompletedProcess(argv, code)

        if "--list" in argv:
            self.assertEqual(argv, [str(self.interpreter), "--list", str(self.native_out / "bin/ld.real")])
            data = ("\tlibtapi.so.12 => " + str(self.loader_target) + " (0x1234)\n").encode()
            return completed(self.loader_code, data, b"")
        self.assertEqual(argv, [str(self.native_out / "bin/ld"), "-v"])
        if self.smoke_timeout:
            kwargs["stdout"].write(b"partial smoke")
            kwargs["stderr"].write(b"timeout")
            raise subprocess.TimeoutExpired(argv, 10, output=b"partial smoke", stderr=b"timeout")
        if self.mutate_during_smoke:
            (self.native_out / "bin/strip").write_bytes(owned_elf() + b"changed")
        return completed(self.smoke_code, b"", b"smoke failure" if self.smoke_code else self.smoke_bytes)

    def run_plan(self, *, name="new-plan", query=False, dry_code=0, wrong_plan_hash=False, final_write_error=False,
                 missing_receipt=False, changed_planner=False):
        prior = self.native_plan_dir / "result.json"
        prior.write_text(json.dumps(self.native_plan))
        self.native_build["plan_result_sha256"] = "f" * 64 if wrong_plan_hash else plan.base.sha256(prior)
        built = self.native_build_dir / "result.json"
        built.write_text(json.dumps(self.native_build))
        original_open = Path.open

        def open_file(path, *args, **kwargs):
            if final_write_error and path == self.root / name / "result.json":
                raise OSError("synthetic retained-result failure")
            return original_open(path, *args, **kwargs)

        code_changed = False
        original_sha256 = plan.base.sha256

        def checksum(path):
            return "0" * 64 if code_changed and Path(path) == Path(plan.__file__) else original_sha256(path)

        def dry_run(*unused):
            nonlocal code_changed
            # Model a concurrent code change without modifying checked-in
            # source. Only the post-command checksum read is different.
            code_changed = changed_planner
            return {"exit_code": dry_code, "timed_out": False}

        with mock.patch.object(plan.base.platform, "system", return_value="Linux"), \
                mock.patch.object(plan.base.platform, "machine", return_value="x86_64"), \
                mock.patch.object(plan.base.shutil, "which", return_value="nix"), \
                mock.patch.object(plan.base, "sha256", side_effect=checksum), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": "local"}), \
                mock.patch.object(plan.base, "source_identity", return_value=copy.deepcopy(self.fixture.source_id)), \
                mock.patch.object(plan.base, "command", side_effect=lambda a: "nix (Nix) 2.28.3" if a[0] == "nix" else "f" * 40), \
                mock.patch.object(plan, "evaluate", return_value=(copy.deepcopy(self.records), ["offline-local-eval"])) as evaluate, \
                mock.patch.object(plan.inspection.subprocess, "run", side_effect=self.reads) as reads, \
                mock.patch.object(plan.linker, "dry_run", side_effect=dry_run) as dry, \
                mock.patch.object(Path, "open", open_file), contextlib.redirect_stdout(io.StringIO()) as stdout:
            argv = ["--stage", "cross-toolchain", "--puredarwin-source", str(self.root),
                    "--native-linker-plan", str(prior), "--native-linker-plan-sha256", plan.base.sha256(prior),
                    "--native-linker-build-result", str(built), "--native-linker-build-result-sha256", plan.base.sha256(built),
                    "--native-linker-driver-sha256", self.driver_hash,
                    "--expected-sdk-output", self.sdk["out"], "--expected-sdk-derivation", self.sdk["drv"],
                    "--output-dir", str(self.root / name)]
            if missing_receipt:
                built.unlink()
            code = plan.main(argv + (["--query-substituters"] if query else []))
        return code, json.loads(stdout.getvalue()), reads, dry, evaluate

    def test_success_is_metadata_only_after_fresh_registration_verification_inventory_and_smoke(self):
        code, value, reads, dry, _ = self.run_plan()
        self.assertEqual((code, value["status"]), (0, "planned_not_built"))
        self.assertEqual(reads.call_count, 5)
        self.assertEqual(dry.call_count, 1)
        self.assertTrue(value["source_inventory_matches_successful_build"])
        self.assertEqual(value["current_native_inspection"]["status"], "pass")
        self.assertEqual(value["current_native_inspection"]["loader_resolution"]["resolved_path"],
                         str((Path(self.fixture.candidate["out"]) / "lib/libtapi.so.12").resolve()))
        self.assertEqual(value["successful_native_linker"]["plan_burdettie_revision"], "c" * 40)
        self.assertEqual(value["successful_native_linker"]["build_burdettie_revision"], "d" * 40)
        self.assertEqual(value["burdettie_revision"], "f" * 40)
        self.assertEqual(value["libraries_plan_helper_sha256"], plan.base.sha256(Path(plan.libraries.__file__)))
        for key in ("dependency_realization", "dependency_downloads", "fixture_linking", "guest_execution",
                    "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(value[key], "not_run")
        self.assertFalse(value["peak_space_qualified"])
        command = dry.call_args.args[0]
        for flag in ("--offline", "--dry-run", "--no-link"):
            self.assertIn(flag, command)
        self.assertEqual(command[command.index("--max-jobs") + 1], "0")
        self.assertEqual(command[command.index("--store") + 1], "local")
        realize = value["commands"]["realize"]
        self.assertEqual(realize[realize.index("--cores") + 1], "1")
        self.assertEqual(realize[realize.index("--max-jobs") + 1], "1")
        self.assertEqual(realize[-2:], ["--out-link", str(self.root / "new-plan/result-cross-toolchain")])

    def test_missing_running_failed_or_metadata_only_native_evidence_blocks_before_evaluation(self):
        for status in ("running", "fail", "planned_not_built", "metadata_plan_passed_not_built"):
            self.native_build["status"] = status
            code, value, reads, dry, evaluate = self.run_plan(name=status)
            self.assertEqual((code, value["status"]), (2, "blocked"))
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()
        del self.native_build["status"]
        code, _, reads, dry, evaluate = self.run_plan(name="missing-status")
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()

    def test_incomplete_phase_timestamp_registration_reference_or_smoke_evidence_blocks(self):
        original = copy.deepcopy(self.native_build)
        for name, mutate in (
            ("phase", lambda b: b.update(phases=[])),
            ("phase-timeout", lambda b: b["phases"][0].update(timed_out=True)),
            ("phase-timeout-missing", lambda b: b["phases"][0].pop("timed_out")),
            ("timestamp", lambda b: b.update(finished_at="1999-12-31T23:59:59Z")),
            ("timestamp-type", lambda b: b.update(finished_at=1)),
            ("deriver", lambda b: b["output_inspection"]["registered_output"].update(deriver="/wrong")),
            ("references", lambda b: b["output_inspection"].update(references=[])),
            ("smoke", lambda b: b["output_inspection"]["smoke_test"].update(exit_code=1)),
            ("inspector", lambda b: b.update(output_inspector_sha256="0" * 64)),
            ("driver", lambda b: b.update(driver_sha256="0" * 64)),
        ):
            with self.subTest(name=name):
                self.native_build = copy.deepcopy(original)
                mutate(self.native_build)
                code, _, reads, dry, evaluate = self.run_plan(name=name)
                self.assertEqual(code, 2)
                reads.assert_not_called()
                dry.assert_not_called()
                evaluate.assert_not_called()

    def test_missing_actual_native_receipt_blocks_before_evaluation_or_live_commands(self):
        code, value, reads, dry, evaluate = self.run_plan(missing_receipt=True)
        self.assertEqual((code, value["status"]), (2, "blocked"))
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()

    def test_boolean_and_other_noninteger_success_codes_are_rejected_in_receipts(self):
        original_plan, original_build = copy.deepcopy(self.native_plan), copy.deepcopy(self.native_build)
        for name, mutate in (
            ("plan-bool", lambda: self.native_plan["dry_run"].update(exit_code=False)),
            ("path-info-bool", lambda: self.native_plan["read_checks"]["path-info"].update(exit_code=False)),
            ("phase-bool", lambda: self.native_build["phases"][0].update(exit_code=False)),
            ("phase-float", lambda: self.native_build["phases"][0].update(exit_code=0.0)),
            ("phase-string", lambda: self.native_build["phases"][0].update(exit_code="0")),
            ("phase-null", lambda: self.native_build["phases"][0].update(exit_code=None)),
            ("verify-bool", lambda: self.native_build["output_inspection"]["store_verification"].update(exit_code=False)),
            ("smoke-bool", lambda: self.native_build["output_inspection"]["smoke_test"].update(exit_code=False)),
            ("loader-bool", lambda: self.native_build["output_inspection"]["loader_check"].update(exit_code=False)),
        ):
            self.native_plan, self.native_build = copy.deepcopy(original_plan), copy.deepcopy(original_build)
            mutate()
            code, value, reads, dry, evaluate = self.run_plan(name=name)
            self.assertEqual((code, value["status"]), (2, "blocked"))
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()

    def test_boolean_success_in_earlier_tapi_receipts_is_not_inherited(self):
        plan_path, build_path = (self.native_plan_dir / name for name in ("libtapi-plan.json", "libtapi-build-result.json"))
        original_plan, original_build = plan_path.read_bytes(), build_path.read_bytes()
        original_chain = copy.deepcopy(self.native_plan["successful_libtapi"])
        for name, target, mutate in (
            ("tapi-plan-bool", "libtapi-plan.json", lambda value: value["dry_run"].update(exit_code=False)),
            ("tapi-phase-bool", "libtapi-build-result.json", lambda value: value["phases"][0].update(exit_code=False)),
        ):
            path = self.native_plan_dir / target
            plan_path.write_bytes(original_plan)
            build_path.write_bytes(original_build)
            value = json.loads(path.read_bytes())
            mutate(value)
            path.write_text(json.dumps(value))
            tapi_build = json.loads(build_path.read_bytes())
            tapi_build["plan_result_sha256"] = plan.base.sha256(plan_path)
            build_path.write_text(json.dumps(tapi_build))
            chain = dict(original_chain, plan_sha256=plan.base.sha256(plan_path), build_result_sha256=plan.base.sha256(build_path))
            self.native_plan["successful_libtapi"] = copy.deepcopy(chain)
            self.native_build["successful_libtapi"] = copy.deepcopy(chain)
            code, value, reads, dry, evaluate = self.run_plan(name=name)
            self.assertEqual((code, value["status"]), (2, "blocked"))
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()
        plan_path.write_bytes(original_plan)
        build_path.write_bytes(original_build)
        self.native_plan["successful_libtapi"] = copy.deepcopy(original_chain)
        self.native_build["successful_libtapi"] = copy.deepcopy(original_chain)

    def test_realization_candidate_local_store_bounds_and_new_link_must_match(self):
        original = copy.deepcopy(self.native_build)
        changes = [("cores", "--cores", "2"), ("jobs", "--max-jobs", "2"),
                   ("substitution", "max-substitution-jobs", "3"), ("builders", "builders", "ssh://elsewhere"),
                   ("garbage-collection", "min-free", "1"), ("store", "--store", "daemon")]
        for name, token, value in changes:
            self.native_build = copy.deepcopy(original)
            argv = self.native_build["realization"]["argv"]
            argv[argv.index(token) + 1] = value
            self.native_build["phases"][0]["argv"] = list(argv)
            code, _, reads, dry, evaluate = self.run_plan(name=name)
            self.assertEqual(code, 2)
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()
        for name, mutate in (
            ("candidate", lambda b: b["realization"]["argv"].__setitem__(b["realization"]["argv"].index(self.native_candidate["drv"] + "^out"), "/other.drv^out")),
            ("keep-failed", lambda b: b["realization"]["argv"].remove("--keep-failed")),
            ("logs", lambda b: b["realization"]["argv"].remove("--print-build-logs")),
            ("json", lambda b: b["realization"]["argv"].remove("--json")),
            ("existing-link", lambda b: b["realization"].update(out_link_was_absent=False)),
            ("relative-executable", lambda b: b["realization"].update(nix_executable="nix")),
            ("missing-phase-argv", lambda b: b["phases"][0].pop("argv")),
        ):
            self.native_build = copy.deepcopy(original)
            mutate(self.native_build)
            if name != "missing-phase-argv":
                self.native_build["phases"][0]["argv"] = list(self.native_build["realization"]["argv"])
            code, _, reads, dry, evaluate = self.run_plan(name=name)
            self.assertEqual(code, 2)
            reads.assert_not_called()
            dry.assert_not_called()
            evaluate.assert_not_called()

    def test_original_plan_command_is_validated_in_its_unchanged_historical_form(self):
        self.native_plan["commands"]["realize"][1:1] = ["--store", "local"]
        code, _, reads, dry, evaluate = self.run_plan()
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()

    def test_wrong_plan_hash_or_modified_sidecar_is_rejected(self):
        code, _, reads, dry, evaluate = self.run_plan(wrong_plan_hash=True)
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()
        (self.native_build_dir / "output-path-info.json").write_text("{}")
        code, _, reads, dry, evaluate = self.run_plan(name="sidecar")
        self.assertEqual(code, 2)
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()

    def test_current_native_nar_deriver_dependency_or_reference_drift_fails_before_smoke(self):
        original = copy.deepcopy(self.native_info)
        for name, key, value in (("nar", "narSize", 42), ("deriver", "deriver", "/other.drv"),
                                 ("reference", "references", [self.fixture.upstream["out"]])):
            self.native_info = copy.deepcopy(original)
            self.native_info[str(self.native_out)][key] = value
            code, report, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, report["status"]), (1, "fail"))
            self.assertEqual(reads.call_count, 1)
            dry.assert_not_called()
        self.native_info = original
        self.fixture.current[self.sdk["out"]]["narSize"] = 1
        code, report, reads, dry, _ = self.run_plan(name="sdk-nar")
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertEqual(reads.call_count, 2)
        dry.assert_not_called()

    def test_default_store_verification_failure_is_retained_and_not_bypassed(self):
        self.verify_code = 1
        code, report, reads, dry, _ = self.run_plan()
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertEqual(reads.call_count, 3)
        self.assertEqual((self.root / "new-plan/native-store-verify.stderr").read_bytes(), b"verify failure")
        dry.assert_not_called()

    def test_loader_nar_and_actual_resolution_are_checked_before_version_execution(self):
        old = self.fixture.current[str(self.loader)]["narSize"]
        self.fixture.current[str(self.loader)]["narSize"] = 1
        code, value, reads, dry, _ = self.run_plan(name="loader-nar")
        self.assertEqual((code, value["status"]), (1, "fail"))
        self.assertEqual(reads.call_count, 2)
        dry.assert_not_called()
        self.fixture.current[str(self.loader)]["narSize"] = old
        alternate = self.root / "wrong-libtapi.so"
        alternate.write_bytes(owned_elf())
        self.loader_target = alternate
        code, value, reads, dry, _ = self.run_plan(name="wrong-loader-resolution")
        self.assertEqual((code, value["status"]), (1, "fail"))
        self.assertEqual(reads.call_count, 4)
        self.assertNotIn("native-ld-version", value["read_checks"])
        dry.assert_not_called()

    def test_boolean_live_exit_and_dry_run_codes_remain_failures(self):
        for name, field, expected_reads in (("verify-bool-live", "verify_code", 3),
                                            ("loader-bool-live", "loader_code", 4),
                                            ("smoke-bool-live", "smoke_code", 5)):
            setattr(self, field, False)
            code, value, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, value["status"]), (1, "fail"))
            self.assertEqual(reads.call_count, expected_reads)
            dry.assert_not_called()
            setattr(self, field, 0)
        code, value, _, _, _ = self.run_plan(name="dry-bool-live", dry_code=False)
        self.assertEqual((code, value["status"]), (1, "fail"))

    def test_file_mode_link_or_elf_change_is_failure_before_smoke(self):
        target = self.native_out / "bin/strip"
        original = target.read_bytes()
        for name, mutate in (
            ("mode", lambda: target.chmod(0o644)),
            ("bytes", lambda: target.write_bytes(original + b"changed")),
            ("elf", lambda: target.write_bytes(b"not an ELF file")),
        ):
            mutate()
            code, report, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, report["status"]), (1, "fail"))
            self.assertEqual(reads.call_count, 1)
            dry.assert_not_called()
            target.write_bytes(original)
            target.chmod(0o755)
        (self.native_out / "unexpected-link").symlink_to(self.fixture.source / "build.sh")
        code, report, reads, dry, _ = self.run_plan(name="link")
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertIn("escapes", report["error"])
        dry.assert_not_called()

    def test_smoke_failure_timeout_changed_version_and_mutation_remain_failures(self):
        for name, attribute, value in (("exit", "smoke_code", 1), ("timeout", "smoke_timeout", True),
                                       ("version", "smoke_bytes", b"different version"),
                                       ("mutation", "mutate_during_smoke", True)):
            old = getattr(self, attribute)
            setattr(self, attribute, value)
            code, report, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, report["status"]), (1, "fail"))
            self.assertEqual(reads.call_count, 5)
            self.assertIn("native-ld-version", report["read_checks"])
            dry.assert_not_called()
            setattr(self, attribute, old)

    def test_cyclic_native_or_tapi_output_link_is_a_retained_failure(self):
        for name, directory in (("native-cycle", self.native_out), ("tapi-cycle", Path(self.fixture.candidate["out"]))):
            link = directory / "cyclic-link"
            link.symlink_to("cyclic-link")
            code, value, _, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, value["status"]), (1, "fail"))
            self.assertIn("error", value)
            self.assertTrue((self.root / name / "result.json").is_file())
            dry.assert_not_called()
            link.unlink()

    def test_evaluated_native_or_downstream_identity_mismatch_fails_without_dry_run(self):
        original = copy.deepcopy(self.records)
        for name, mutate in (
            ("native", lambda: self.records["native_linker"]["candidate"].update(drv="/wrong.drv")),
            ("downstream", lambda: self.records.update(candidate=copy.deepcopy(self.records["previous_bounded"]))),
            ("flags", lambda: self.records["preserved"].update(selected_recipe=False)),
        ):
            self.records = copy.deepcopy(original)
            mutate()
            code, value, reads, dry, _ = self.run_plan(name=name)
            self.assertEqual((code, value["status"]), (1, "fail"))
            reads.assert_not_called()
            dry.assert_not_called()

    def test_changed_source_or_tapi_bytes_fail_before_native_execution(self):
        source = self.fixture.source / ".metadata"
        original = source.read_bytes()
        original_mode = source.stat().st_mode & 0o777
        source.chmod(0o644)
        source.write_bytes(b"changed source")
        code, value, reads, dry, _ = self.run_plan(name="source")
        self.assertEqual((code, value["status"]), (1, "fail"))
        reads.assert_not_called()
        dry.assert_not_called()
        source.write_bytes(original)
        source.chmod(original_mode)
        header = Path(self.fixture.candidate["out"]) / "include/tapi/tapi.h"
        header.write_bytes(b"changed installed TAPI header")
        code, value, reads, dry, _ = self.run_plan(name="tapi")
        self.assertEqual((code, value["status"]), (1, "fail"))
        reads.assert_not_called()
        dry.assert_not_called()

    def test_metadata_queries_are_explicit_and_failed_dry_run_stays_failed(self):
        code, value, _, dry, _ = self.run_plan(query=True)
        self.assertEqual(code, 0)
        self.assertEqual(value["planning_mode"], "substituter_metadata")
        self.assertNotIn("--offline", dry.call_args.args[0])
        code, value, _, dry, _ = self.run_plan(name="dry-fail", dry_code=1)
        self.assertEqual((code, value["status"]), (1, "fail"))

    def test_existing_output_and_failed_final_evidence_write_cannot_report_success(self):
        self.assertEqual(self.run_plan()[0], 0)
        path = self.root / "new-plan/result.json"
        retained = path.read_bytes()
        code, value, reads, dry, evaluate = self.run_plan()
        self.assertEqual((code, value["status"]), (2, "blocked"))
        self.assertEqual(path.read_bytes(), retained)
        reads.assert_not_called()
        dry.assert_not_called()
        evaluate.assert_not_called()
        code, value, _, _, _ = self.run_plan(name="evidence-write", final_write_error=True)
        self.assertEqual((code, value["status"]), (1, "fail"))
        self.assertIn("Could not retain final result", value["error"])

    def test_planner_mutation_after_commands_is_a_failure(self):
        code, value, _, dry, _ = self.run_plan(changed_planner=True)
        self.assertEqual((code, value["status"]), (1, "fail"))
        self.assertEqual(dry.call_count, 1)
        self.assertIn("changed during", value["error"])

    def test_production_evaluation_is_direct_local_offline_and_lock_preserving(self):
        snapshots = {key: path.read_bytes() for key, path in plan.FILES.items()}
        with mock.patch.object(plan.base, "command", return_value="{}") as call:
            plan.evaluate(Path("/synthetic-source"), snapshots, "cross-toolchain")
        argv = call.call_args.args[0]
        for flag in ("--offline", "--no-write-lock-file", "--no-update-lock-file"):
            self.assertIn(flag, argv)
        self.assertNotIn("--impure", argv)
        self.assertEqual(argv[argv.index("--store") + 1], "local")
        self.assertEqual(argv[argv.index("allow-import-from-derivation") + 1], "false")


if __name__ == "__main__":
    unittest.main()
