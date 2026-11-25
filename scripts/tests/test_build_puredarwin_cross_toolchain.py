"""Owned subprocesses and synthetic receipts; no Nix realization is performed."""
import contextlib
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import pwd
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_puredarwin_cross_toolchain as build


def encoded(value):
    return (json.dumps(value, sort_keys=True) + "\n").encode()


def identity(number, name):
    prefix = (str(number) * 32)[:32]
    output = "/nix/store/" + prefix + "-" + name
    return {"drv": output + ".drv", "out": output}


def registered(value):
    return {"path": value["out"], "deriver": value["drv"],
            "nar_hash": "sha256-" + "A" * 43 + "=", "nar_size": 42, "path_info_sha256": "d" * 64}


def plan_fixture(code, directory):
    sdk, candidate, native = identity(1, "sdk"), identity(5, "cross"), identity(6, "native")
    selected = registered(native)
    observed = {"schema_version": 1, "scope": build.inspection.INSPECTION_SCOPE, "status": "pass",
                "registered_output": selected, "dependencies": {"sdk": registered(sdk), "libtapi": registered(identity(7, "tapi"))},
                "files": {}, "symlinks": {}, "elf_files": {}, "references": [],
                "tapi_dynamic_binding": {}, "version_identity": {}, "loader_registered_output": {},
                "loader_resolution": {}, "smoke_test": {}}
    return {"schema_version": 1, "scope": "staged_native_linker_library_plan_only", "stage": "cross-toolchain",
            "status": "planned_not_built", "planning_mode": "offline_graph_only", "substituter_queries_enabled": False,
            "helper_sha256": code[Path(build.planner.__file__).name],
            **{key: code[path.name] for key, path in build.planner.FILES.items()},
            **{key: "not_run" for key in ("dependency_realization", "dependency_downloads", "fixture_linking",
                                        "guest_execution", "burdettie_runtime_execution", "security_enforcement")},
            "peak_space_qualified": False, "source_inventory_matches_successful_build": True,
            "current_libtapi_inspection": {"matches_successful_receipt": True},
            "burdettie_revision": "a" * 40, "nix_version": "nix (Nix) 2.28.3", "source": {"revision": "b" * 40},
            "sdk": sdk, "expected_sdk": sdk, "upstream": identity(2, "upstream"),
            "previous_bounded": identity(3, "previous"), "rebound_unbounded": identity(4, "rebound"), "candidate": candidate,
            "native_linker": {"candidate": native}, "compiler_rt": None, "cross_toolchain": {"candidate": candidate},
            "preserved": {name: None if name in ("baseline_compiler_rt", "baseline_libsystem") else True
                          for name in build.planner.PRESERVED},
            "baseline_checks": {"compiler_rt": None, "libsystem": None, "cross_toolchain": {
                "name": True, "build_command": True, "wrapper_names": True, "only_linker_wrappers_changed": True}},
            "successful_native_linker": {"plan_sha256": "b" * 64, "build_result_sha256": "c" * 64,
                "driver_sha256": "a" * 64, "registered_output": selected},
            "current_native_inspection": observed,
            "dry_run": {"exit_code": 0, "timed_out": False, "log_sha256": hashlib.sha256(b"owned dry-run").hexdigest()},
            "commands": {**build.planned_commands(candidate, directory), "evaluate": ["nix", "eval", "--offline", "owned-expression"]}}


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.code = {name: "e" * 64 for name in build.CODE}
        self.plan = plan_fixture(self.code, self.root)
        (self.root / "dry-run.log").write_bytes(b"owned dry-run")

    def validate(self, value=None):
        build.validate_plan(value or self.plan, self.code, "a" * 64, self.root)

    def test_frozen_metadata_plan_is_accepted_without_promoting_it_to_build_success(self):
        self.validate()
        self.assertEqual(self.plan["status"], "planned_not_built")
        self.assertEqual(self.plan["dependency_realization"], "not_run")

    def test_failed_timed_out_or_wrong_type_dry_runs_are_rejected(self):
        for value in (False, True, "0", 0.0, 1, None):
            altered = copy.deepcopy(self.plan)
            altered["dry_run"]["exit_code"] = value
            with self.subTest(value=value), self.assertRaises(build.base.BuildError):
                self.validate(altered)
        self.plan["dry_run"]["timed_out"] = True
        with self.assertRaises(build.base.BuildError):
            self.validate()

    def test_wrong_scope_status_stage_network_mode_or_recipe_is_rejected(self):
        for field, value in (("scope", build.SCOPE), ("status", build.SUCCESS), ("stage", "compiler-rt"),
                             ("planning_mode", "substituter_metadata"), ("substituter_queries_enabled", True),
                             ("helper_sha256", "0" * 64), ("recipe_sha256", "0" * 64),
                             ("dependency_realization", "pass"), ("peak_space_qualified", True)):
            altered = copy.deepcopy(self.plan)
            altered[field] = value
            with self.subTest(field=field), self.assertRaises(build.base.BuildError):
                self.validate(altered)

    def test_reviewed_native_driver_and_registered_native_or_sdk_drift_are_rejected(self):
        for mutate in (
            lambda v: v["successful_native_linker"].update(driver_sha256="0" * 64),
            lambda v: v["current_native_inspection"].update(status="fail"),
            lambda v: v["current_native_inspection"]["registered_output"].update(nar_size=999),
            lambda v: v["current_native_inspection"]["dependencies"]["sdk"].update(deriver=identity(8, "other")["drv"]),
            lambda v: v["baseline_checks"]["cross_toolchain"].update(extra=True),
            lambda v: v["preserved"].update(sdk=False),
        ):
            changed = copy.deepcopy(self.plan)
            # The fixture aliases registration objects; break that alias before mutation.
            changed["current_native_inspection"]["registered_output"] = copy.deepcopy(changed["current_native_inspection"]["registered_output"])
            mutate(changed)
            with self.assertRaises(build.base.BuildError):
                self.validate(changed)

    def test_frozen_argv_cannot_supply_an_executable_extra_output_or_weaker_job_bound(self):
        for mutate in (
            lambda a: a.__setitem__(0, "/tmp/unreviewed-nix"),
            lambda a: a.__setitem__(a.index("--max-jobs") + 1, "4"),
            lambda a: a.extend(["--no-check-sigs"]),
            lambda a: a.__setitem__(a.index("local"), "daemon"),
            lambda a: a.__setitem__(-1, "/tmp/../unsafe/result-cross-toolchain"),
        ):
            altered = copy.deepcopy(self.plan)
            mutate(altered["commands"]["realize"])
            with self.assertRaises(build.base.BuildError):
                self.validate(altered)

    def test_candidate_paths_are_canonical_and_distinct(self):
        for candidate in ({"drv": "--repair", "out": identity(5, "cross")["out"]},
                          {"drv": identity(5, "cross")["drv"], "out": "/nix/store/../escape"},
                          self.plan["upstream"]):
            altered = copy.deepcopy(self.plan)
            altered["candidate"] = candidate
            with self.assertRaises(build.base.BuildError):
                self.validate(altered)

    def test_missing_or_changed_retained_dry_run_log_is_rejected(self):
        (self.root / "dry-run.log").write_bytes(b"changed")
        with self.assertRaises(build.base.BuildError):
            self.validate()

    def test_fresh_plan_must_keep_source_candidate_recipe_and_native_inventory(self):
        for field in ("source", "candidate", "sdk", "native_linker", "recipe_sha256", "successful_native_linker"):
            fresh = copy.deepcopy(self.plan)
            fresh[field] = {"changed": True}
            with self.subTest(field=field), self.assertRaises(build.base.BuildError):
                build.compare_plans(self.plan, fresh)
        fresh = copy.deepcopy(self.plan)
        fresh["current_native_inspection"]["files"] = {"changed": True}
        with self.assertRaises(build.base.BuildError):
            build.compare_plans(self.plan, fresh)

    def test_raw_serialization_or_loader_address_variance_is_not_content_identity(self):
        fresh = copy.deepcopy(self.plan)
        fresh["current_native_inspection"]["registered_output"] = copy.deepcopy(fresh["current_native_inspection"]["registered_output"])
        fresh["current_native_inspection"]["registered_output"]["path_info_sha256"] = "f" * 64
        fresh["current_native_inspection"]["loader_check"] = {"stdout_sha256": "f" * 64}
        build.compare_plans(self.plan, fresh)

    def test_realize_command_is_constructed_with_offline_direct_local_and_one_job(self):
        argv = build.realize_argv(self.plan["candidate"], Path("/reviewed/nix"), self.root)
        self.assertEqual(argv[0], "/reviewed/nix")
        self.assertIn("--offline", argv)
        self.assertEqual(argv[argv.index("--store") + 1], "local")
        self.assertEqual(argv[argv.index("--max-jobs") + 1], "1")
        self.assertEqual(argv[argv.index("--cores") + 1], "1")
        self.assertEqual(argv[-1], str(self.root / "result-cross-toolchain"))

    def test_receipt_checksum_duplicates_and_symlinks_are_rejected(self):
        path = self.root / "record.json"
        data = b'{"status":"pass","status":"fail"}'
        path.write_bytes(data)
        with self.assertRaises(build.base.BuildError):
            build.read_record(path, hashlib.sha256(data).hexdigest())
        path.write_bytes(b"{}")
        with self.assertRaises(build.base.BuildError):
            build.read_record(path, "0" * 64)
        linked = self.root / "linked.json"
        linked.symlink_to(path)
        with self.assertRaises(build.base.BuildError):
            build.read_record(linked, build.base.sha256(path))

    def test_exclusive_evidence_directory_rejects_existing_path_and_dangling_symlink(self):
        for name, symlink in (("existing", False), ("dangling", True)):
            path = self.root / name
            path.symlink_to(self.root / "absent") if symlink else path.mkdir()
            with self.assertRaises(FileExistsError):
                build.new_directory(path)

    def test_new_directory_rejects_store_and_noncanonical_parents(self):
        linked = self.root / "parent"
        linked.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(build.base.BuildError):
            build.new_directory(linked / "evidence")
        with mock.patch.object(build.cross, "STORE", self.root), self.assertRaises(build.base.BuildError):
            build.new_directory(self.root / "evidence")

    def test_reviewed_manifest_rejects_missing_extra_and_changed_helpers(self):
        source = self.root / "helper.py"
        source.write_text("owned helper\n")
        manifest = {"schema_version": 1, "scope": "reviewed_cross_toolchain_source",
                    "sha256": {source.name: build.base.sha256(source)}}
        target = self.root / "manifest.json"
        target.write_bytes(encoded(manifest))
        with mock.patch.object(build, "CODE", {source.name: source}):
            build.reviewed_code(target, build.base.sha256(target))
            source.write_text("changed helper\n")
            with self.assertRaises(build.base.BuildError):
                build.reviewed_code(target, build.base.sha256(target))
            manifest["sha256"]["extra.py"] = "1" * 64
            target.write_bytes(encoded(manifest))
            with self.assertRaises(build.base.BuildError):
                build.reviewed_code(target, build.base.sha256(target))


class GuardTests(unittest.TestCase):
    def cgroup(self):
        return {"path": "/sys/fs/cgroup/owned.service", "cpu.max": "25000 100000", "cpu.weight": "10",
                "memory.high": str(1280 * 1024 ** 2), "memory.max": str(1536 * 1024 ** 2),
                "memory.swap.max": "0", "pids.max": "128", "memory.events": "oom 0\noom_kill 0\n"}

    def service(self):
        return {"Id": "owned.service", "Type": "exec", "ActiveState": "active", "SubState": "running",
                "User": pwd.getpwuid(os.geteuid()).pw_name, "MainPID": str(os.getpid()), "ControlPID": "0",
                "ControlGroup": "/owned.service", "KillMode": "control-group", "ExitType": "main",
                "RemainAfterExit": "no", "SendSIGKILL": "yes", "KillSignal": "15", "FinalKillSignal": "9",
                "Delegate": "no", "RuntimeMaxUSec": "10min", "TimeoutStopUSec": "20s", "PrivateNetwork": "yes",
                "NetworkNamespacePath": "", "JoinsNamespaceOf": "", "NoNewPrivileges": "yes", "Nice": "15", "IOWeight": "10"}

    def resource(self):
        return {"available_memory_bytes": 3 * build.GIB, "filesystems": {
            "/owned/store": {"free_bytes": 10 * build.GIB, "free_inodes": 200000}}}

    def test_cpu_memory_swap_tasks_must_all_be_bounded(self):
        build.check_cgroup(self.cgroup())
        for name, value in (("cpu.max", "max 100000"), ("cpu.max", "80000 100000"),
                            ("memory.max", "max"), ("memory.max", str(4 * build.GIB)),
                            ("memory.high", "max"), ("cpu.weight", "11"), ("memory.swap.max", "1"),
                            ("pids.max", "129"), ("memory.events", "oom 0")):
            state = self.cgroup()
            state[name] = value
            with self.subTest(name=name), self.assertRaises(build.base.BuildError):
                build.check_cgroup(state)

    def test_fresh_capacity_requires_disk_memory_and_inode_reserves(self):
        build.check_resources(self.resource())
        for mutate in (
            lambda r: r.update(available_memory_bytes=build.GIB),
            lambda r: r["filesystems"]["/owned/store"].update(free_bytes=build.DISK_RESERVE),
            lambda r: r["filesystems"]["/owned/store"].update(free_inodes=99999),
            lambda r: r["filesystems"]["/owned/store"].update(free_bytes=True),
        ):
            value = self.resource()
            mutate(value)
            with self.assertRaises(build.base.BuildError):
                build.check_resources(value)

    def test_live_capacity_catches_filesystem_growth_even_above_absolute_reserve(self):
        initial = self.resource()
        current = copy.deepcopy(initial)
        current["filesystems"]["/owned/store"]["free_bytes"] -= build.MAX_GROWTH + 1
        with self.assertRaisesRegex(build.base.BuildError, "growth"):
            build.check_resources(current, initial)

    def test_guard_rechecks_cgroup_identity_oom_and_overall_timeout(self):
        with mock.patch.object(build, "cgroup_state", return_value=self.cgroup()) as state, \
                mock.patch.object(build, "resources", return_value=self.resource()), \
                mock.patch.object(build, "service_state", return_value=self.service()), \
                mock.patch.object(build, "cgroup_processes", return_value={os.getpid()}), \
                mock.patch.object(build.time, "monotonic", return_value=0) as clock:
            guard = build.Guard([Path("/owned/store")], "owned.service")
            guard.check()
            state.return_value = {**self.cgroup(), "memory.events": "oom 1\noom_kill 0\n"}
            with self.assertRaisesRegex(build.base.BuildError, "OOM"):
                guard.check()
            state.return_value = self.cgroup()
            clock.return_value = build.TOTAL_SECONDS + 1
            with self.assertRaisesRegex(build.base.BuildError, "timeout"):
                guard.check()

    def test_service_must_terminate_all_descendants_when_its_main_process_exits(self):
        build.check_service(self.service(), "owned.service", self.cgroup())
        for key, value in (("KillMode", "process"), ("ExitType", "cgroup"), ("RemainAfterExit", "yes"),
                           ("SendSIGKILL", "no"), ("Type", "forking"), ("Delegate", "yes"),
                           ("MainPID", str(os.getpid() + 1)), ("ControlGroup", "/unrelated.service"),
                           ("RuntimeMaxUSec", "infinity"), ("TimeoutStopUSec", "60s"), ("PrivateNetwork", "no"),
                           ("NetworkNamespacePath", "/host-network"), ("JoinsNamespaceOf", "other.service"),
                           ("NoNewPrivileges", "no"), ("Nice", "0"), ("IOWeight", "100"), ("IOWeight", "[not set]")):
            state = self.service()
            state[key] = value
            with self.subTest(key=key), self.assertRaises(build.base.BuildError):
                build.check_service(state, "owned.service", self.cgroup())

    def test_cgroup_census_covers_nested_groups_and_rejects_shared_work(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        root.chmod(0o755)
        nested = root / "nested"
        nested.mkdir()
        nested.chmod(0o755)
        (root / "cgroup.procs").write_text(str(os.getpid()) + "\n")
        (nested / "cgroup.procs").write_text("123456\n")
        real_stat = Path.stat

        def owned(path, **kwargs):
            value = real_stat(path, **kwargs)
            # The test host may be unprivileged; this fixture describes root-owned cgroups.
            return type("Owned", (), {"st_uid": 0, "st_mode": value.st_mode})()

        with mock.patch.object(build.os, "access", return_value=False), mock.patch.object(Path, "stat", owned):
            self.assertEqual(build.cgroup_processes(root), {os.getpid(), 123456})
        with mock.patch.object(build, "cgroup_state", return_value=self.cgroup()), \
                mock.patch.object(build, "resources", return_value=self.resource()), \
                mock.patch.object(build, "service_state", return_value=self.service()), \
                mock.patch.object(build, "cgroup_processes", return_value={os.getpid(), 123456}), \
                self.assertRaisesRegex(build.base.BuildError, "another process"):
            build.Guard([Path("/owned/store")], "owned.service")


class RunnerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.guard, self.report = mock.Mock(), {}
        self.runner = build.Runner(self.root, self.guard, self.report, dict(os.environ))

    def child(self, script, timeout=3):
        return self.runner.run("owned-child", [sys.executable, "-c", script], timeout)

    def test_actual_success_and_stream_hashes_are_recorded(self):
        result = self.child("import sys; print('owned'); print('error stream', file=sys.stderr)")
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["exit_code"], 0)
        self.assertFalse(result["timed_out"])
        for suffix in ("stdout", "stderr"):
            self.assertEqual(result[suffix + "_sha256"], build.base.sha256(self.root / ("owned-child." + suffix)))

    def test_actual_nonzero_exit_is_a_failed_phase(self):
        with self.assertRaises(build.base.BuildError):
            self.child("raise SystemExit(7)")
        self.assertEqual(self.report["phases"][0]["exit_code"], 7)
        self.assertEqual(self.report["phases"][0]["status"], "fail")

    def test_actual_timeout_stops_only_the_created_process_group(self):
        with self.assertRaises(build.base.BuildError):
            self.child("import time; time.sleep(30)", timeout=0.1)
        self.assertTrue(self.report["phases"][0]["timed_out"])
        self.assertLess(self.report["phases"][0]["exit_code"], 0)

    def test_log_limit_is_enforced_without_unbounded_capture(self):
        with mock.patch.object(build, "MAX_LOG", 1024), self.assertRaises(build.base.BuildError):
            self.child("import os; os.write(1, b'x' * 8192)")
        self.assertTrue(self.report["phases"][0]["output_limit_reached"])
        self.assertLessEqual(sum((self.root / ("owned-child." + key)).stat().st_size for key in ("stdout", "stderr")), 1024)

    def test_resource_failure_during_child_execution_is_retained(self):
        self.guard.check.side_effect = [None, build.base.BuildError("reserve reached")]
        with self.assertRaisesRegex(build.base.BuildError, "reserve"):
            self.child("import time; time.sleep(30)")
        self.assertEqual(self.report["phases"][0]["status"], "fail")
        self.assertLess(self.report["phases"][0]["exit_code"], 0)

    def test_child_process_group_cleanup_never_uses_a_uid_or_process_name(self):
        process = mock.Mock(pid=234567, returncode=0)
        process.poll.return_value = 0
        with mock.patch.object(build.os, "killpg", side_effect=ProcessLookupError) as kill:
            build.stop_group(process)
        kill.assert_called_once_with(234567, signal.SIGTERM)

    def test_detached_descendant_prevents_phase_success_even_when_leader_exits_zero(self):
        marker = self.root / "detached.pid"

        def idle():
            if marker.exists():
                descendant = int(marker.read_text())
                os.kill(descendant, 0)
                raise build.base.BuildError("operation cgroup still contains another process")

        self.guard.require_idle.side_effect = idle
        try:
            with self.assertRaisesRegex(build.base.BuildError, "another process"):
                self.child("import pathlib,subprocess,sys; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)'], "
                           "start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); "
                           "pathlib.Path(" + repr(str(marker)) + ").write_text(str(p.pid)); print(p.pid)")
            result = self.report["phases"][0]
            self.assertEqual(result["exit_code"], 0)
            self.assertEqual(result["status"], "fail")
        finally:
            if marker.exists():
                try:
                    os.kill(int(marker.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass

    def test_sigterm_reaches_child_group_cleanup(self):
        result = self.root / "signal-result.json"
        ready = self.root / "ready"
        program = (
            "import sys,signal,json,pathlib; from unittest import mock; "
            "sys.path.insert(0,sys.argv[1]); import build_puredarwin_cross_toolchain as b; "
            "signal.signal(signal.SIGTERM,b.interrupted); root=pathlib.Path(sys.argv[2]); report={}; "
            "runner=b.Runner(root,mock.Mock(),report,dict(b.os.environ));\n"
            "try:\n runner.run('signal-child',[sys.executable,'-c',"
            "\"import os,pathlib,time; pathlib.Path('" + str(ready) + "').write_text(str(os.getpid())); time.sleep(30)\"],30)\n"
            "except b.OperationInterrupted:\n (root/'signal-result.json').write_text(json.dumps(report))\n")
        producer = subprocess.Popen([sys.executable, "-c", program, str(build.SCRIPT.parent), str(self.root)])
        try:
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(ready.exists())
            producer.send_signal(signal.SIGTERM)
            self.assertEqual(producer.wait(timeout=5), 0)
            observed = json.loads(result.read_text())["phases"][0]
            self.assertEqual(observed["status"], "fail")
            self.assertLess(observed["exit_code"], 0)
            with self.assertRaises(ProcessLookupError):
                os.kill(int(ready.read_text()), 0)
        finally:
            if producer.poll() is None:
                producer.kill()
                producer.wait(timeout=5)
            if ready.exists():
                try:
                    os.killpg(int(ready.read_text()), signal.SIGKILL)
                except ProcessLookupError:
                    pass


class OrchestrationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.source, self.native_dir = self.root / "source", self.root / "native"
        self.source.mkdir()
        self.native_dir.mkdir()
        self.code_dir = self.root / "code"
        self.code_dir.mkdir()
        self.code = {}
        for name in build.CODE:
            path = self.code_dir / name
            path.write_text("owned fixture for " + name)
            self.code[name] = path
        self.code_hashes = {name: build.base.sha256(path) for name, path in self.code.items()}
        self.manifest = self.root / "reviewed.json"
        self.manifest.write_bytes(encoded({"schema_version": 1, "scope": "reviewed_cross_toolchain_source", "sha256": self.code_hashes}))
        self.plan_dir = self.root / "retained-plan"
        self.plan_dir.mkdir()
        self.plan = plan_fixture(self.code_hashes, self.plan_dir)
        for name, field in (("plan.json", "plan_sha256"), ("build.json", "build_result_sha256")):
            path = self.native_dir / name
            path.write_bytes(b"{}")
            self.plan["successful_native_linker"][field] = build.base.sha256(path)
        self.plan_path = self.plan_dir / "result.json"
        self.plan_path.write_bytes(encoded(self.plan))
        (self.plan_dir / "dry-run.log").write_bytes(b"owned dry-run")
        self.nix = self.root / "nix"
        self.nix.write_text("owned Nix placeholder, never executed")
        self.nix.chmod(0o700)
        self.output = self.root / "build"
        self.argv = ["--cross-plan", str(self.plan_path), "--cross-plan-sha256", build.base.sha256(self.plan_path),
                     "--native-linker-plan", str(self.native_dir / "plan.json"),
                     "--native-linker-build-result", str(self.native_dir / "build.json"),
                     "--native-linker-driver-sha256", "a" * 64, "--puredarwin-source", str(self.source),
                     "--nix-executable", str(self.nix), "--nix-sha256", build.base.sha256(self.nix),
                     "--reviewed-code", str(self.manifest), "--reviewed-code-sha256", build.base.sha256(self.manifest),
                     "--output-dir", str(self.output), "--service-unit", "owned.service"]
        self.executed, self.failure, self.mutation = [], None, None

    def fake_run(self, runner, name, argv, timeout):
        self.executed.append((name, argv))
        item = {"name": name, "argv": argv, "status": "pass", "exit_code": 0, "timed_out": False}
        runner.report["phases"].append(item)
        if self.failure == name:
            item.update(status="fail", exit_code=9)
            raise build.base.BuildError("owned failed " + name)
        if name == "fresh-plan":
            directory = Path(argv[argv.index("--output-dir") + 1])
            directory.mkdir()
            value = copy.deepcopy(self.plan)
            value["commands"].update(build.planned_commands(value["candidate"], directory))
            if self.mutation == "candidate":
                value["candidate"] = identity(9, "drift")
                value["cross_toolchain"] = {"candidate": value["candidate"]}
                value["commands"].update(build.planned_commands(value["candidate"], directory))
            if self.mutation == "helper":
                self.code[next(iter(self.code))].write_text("mutated after planner")
            if self.mutation == "source":
                self.source_changed = True
            (directory / "result.json").write_bytes(encoded(value))
            (directory / "dry-run.log").write_bytes(b"owned dry-run")
        elif name == "cross-toolchain-build":
            link = Path(argv[-1])
            if self.mutation == "output-link":
                link.write_text("not a result symlink")
            else:
                link.symlink_to(self.plan["candidate"]["out"])
            (runner.directory / (name + ".stdout")).write_bytes(encoded([{
                "drvPath": self.plan["candidate"]["drv"], "outputs": {"out": self.plan["candidate"]["out"]}}]))
        elif name == "cross-output-inspection":
            directory = Path(argv[-1])
            directory.mkdir()
            result = {"schema_version": 1, "scope": build.cross.INSPECTION_SCOPE, "status": "pass",
                      "registered_output": registered(self.plan["candidate"])}
            if self.mutation == "inspection":
                result["status"] = "fail"
            (directory / "inspection.json").write_bytes(encoded(result))
        return item

    def run_main(self, realize=True):
        output = io.StringIO()
        self.source_changed = False
        guard = mock.Mock(initial={"owned": True}, cgroup={"owned": True}, service={"owned": True})
        guard.check.return_value = {"owned": True}

        def command(argv, **unused):
            if argv[-1] == "--version":
                return "nix (Nix) 2.28.3"
            return "" if "status" in argv else "f" * 40

        def source_identity(path):
            return {"revision": "changed"} if self.source_changed else self.plan["source"]

        with mock.patch.object(build, "CODE", self.code), \
                mock.patch.object(build, "Guard", return_value=guard), \
                mock.patch.object(build, "cgroup_state", return_value={"owned": True}), \
                mock.patch.object(build.os, "geteuid", return_value=1234), \
                mock.patch.object(build.base, "source_identity", side_effect=source_identity), \
                mock.patch.object(build.base, "command", side_effect=command), \
                mock.patch.object(build.Runner, "run", autospec=True, side_effect=self.fake_run), \
                contextlib.redirect_stdout(output):
            code = build.main(self.argv + (["--realize"] if realize else []))
        return code, json.loads(output.getvalue())

    def test_success_needs_planner_realization_and_installed_output_inspection(self):
        code, result = self.run_main()
        self.assertEqual(code, 0, result)
        self.assertEqual(result["status"], build.SUCCESS)
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan", "cross-toolchain-build", "cross-output-inspection"])
        self.assertTrue(result["realization_command_succeeded"])
        self.assertEqual(result["cross_toolchain_output_inspection_status"], "pass")
        self.assertEqual(result["guest_execution"], "not_run")
        self.assertEqual(result["compiler_rt_qualification"], "not_run")
        self.assertEqual(result["automatic_stop_guard"], "not_checked_by_this_helper")
        self.assertEqual(json.loads((self.output / "result.json").read_text()), result)

    def test_default_stops_after_fresh_preflight_and_never_claims_built(self):
        code, result = self.run_main(realize=False)
        self.assertEqual(code, 0, result)
        self.assertEqual(result["status"], build.READY)
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan"])
        self.assertFalse(result["realization_command_succeeded"])
        self.assertEqual(result["cross_toolchain_realization"], "not_run")

    def test_failed_planner_never_reaches_realization(self):
        self.failure = "fresh-plan"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "fail")
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan"])

    def test_failed_realization_never_reaches_inspector_or_success(self):
        self.failure = "cross-toolchain-build"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "fail")
        self.assertFalse(result["realization_command_succeeded"])
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan", "cross-toolchain-build"])

    def test_failed_inspector_preserves_successful_build_but_never_promotes_overall_status(self):
        self.failure = "cross-output-inspection"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "fail")
        self.assertTrue(result["realization_command_succeeded"])
        self.assertEqual(result["cross_toolchain_output_inspection_status"], "fail")

    def test_fresh_candidate_drift_blocks_realization(self):
        self.mutation = "candidate"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan"])
        self.assertIn("differs", result["error"])

    def test_helper_mutation_after_planning_blocks_realization(self):
        self.mutation = "helper"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan"])
        self.assertIn("helper bytes changed", result["error"])

    def test_source_mutation_after_planning_blocks_realization(self):
        self.mutation = "source"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual([name for name, _ in self.executed], ["fresh-plan"])
        self.assertIn("source changed", result["error"])

    def test_output_link_must_be_new_and_name_the_exact_candidate(self):
        self.mutation = "output-link"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertFalse(result["realization_command_succeeded"])
        self.assertNotIn("cross-output-inspection", [name for name, _ in self.executed])

    def test_bad_inspection_receipt_cannot_be_promoted(self):
        self.mutation = "inspection"
        code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["cross_toolchain_output_inspection_status"], "fail")

    def test_existing_result_directory_preserves_earlier_evidence(self):
        self.output.mkdir()
        (self.output / "result.json").write_text("earlier evidence")
        code, result = self.run_main()
        self.assertEqual(code, 2)
        self.assertEqual(result["status"], "blocked")
        self.assertEqual((self.output / "result.json").read_text(), "earlier evidence")
        self.assertEqual(self.executed, [])

    def test_nonstandard_nix_basename_cannot_fall_back_to_another_path_executable(self):
        altered = self.root / "other-name"
        self.nix.rename(altered)
        self.argv[self.argv.index("--nix-executable") + 1] = str(altered)
        code, result = self.run_main()
        self.assertEqual(code, 2)
        self.assertIn("exact reviewed executable", result["error"])
        self.assertEqual(self.executed, [])

    def test_environment_scrubs_ambient_python_git_nix_and_loader_overrides(self):
        with mock.patch.dict(os.environ, {"PYTHONPATH": "/unsafe", "LD_PRELOAD": "/unsafe", "NIX_CONFIG": "unsafe",
                                         "GIT_CONFIG_COUNT": "10", "GIT_CONFIG_KEY_9": "unsafe"}):
            value = build.child_environment(self.nix, self.root)
            for key in ("PYTHONPATH", "LD_PRELOAD", "NIX_CONFIG", "GIT_CONFIG_KEY_9"):
                self.assertNotIn(key, value)
            self.assertEqual(value["NIX_USER_CONF_FILES"], "/dev/null")
            self.assertEqual(value["GIT_CONFIG_GLOBAL"], "/dev/null")
            with build.controlled_environment(value):
                self.assertEqual(dict(os.environ), value)
            self.assertEqual(os.environ["PYTHONPATH"], "/unsafe")

    def test_atomic_receipt_does_not_publish_success_after_flush_failure(self):
        self.output.mkdir()
        with mock.patch.object(build.os, "fsync", side_effect=OSError("owned flush failure")), self.assertRaises(OSError):
            build.publish_result(self.output, {"status": build.SUCCESS})
        self.assertFalse((self.output / "result.json").exists())
        self.assertTrue((self.output / "result.json.partial").exists())

    def test_publication_failure_reports_failure_and_does_not_create_terminal_success(self):
        with mock.patch.object(build, "publish_result", side_effect=OSError("owned publication failure")):
            code, result = self.run_main()
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "fail")
        self.assertFalse((self.output / "result.json").exists())


if __name__ == "__main__":
    unittest.main()
