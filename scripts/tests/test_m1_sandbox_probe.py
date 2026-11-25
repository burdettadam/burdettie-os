"""Small Linux harness tests; synthetic reports never attest guest enforcement."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("sandbox_probe", ROOT / "scripts/run_m1_sandbox_probe.py")
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)
NONCE = "source_owned_test"


def synthetic_report():
    """Construct explicit unit input, not a simulated kernel or an execution receipt."""
    length = len(f"Burdettie sandbox probe nonce={NONCE}\n")

    def phase(worker, child, restricted):
        roles = []
        for role_name in ("worker", "descendant"):
            cases = []
            for index, (name, operation) in enumerate(probe.CASES):
                row = {"case": name, "operation": operation, "open_rc": 0, "open_errno": 0,
                       "io_rc": length if index in (0, 3, 6) else 1, "io_errno": 0,
                       "content_ok": 1, "sync_errno": 0, "close_errno": 0}
                if restricted and index >= 3:
                    row.update(content_ok=0, io_rc=-2 if index < 6 else -1)
                    row.update(open_rc=-1 if index < 6 else 0,
                               open_errno=1 if index < 6 else 0, io_errno=0 if index < 6 else 1)
                cases.append(row)
            roles.append({"role": role_name, "cases": cases})
        return {"received": 1, "worker_pid": worker, "worker_exit": 0, "worker_signal": 0,
                "native_api": int(restricted), "profile_rc": 0 if restricted else -2,
                "profile_errno": 0, "setup_errno": 0, "descendant_pid": child,
                "descendant_exit": 0, "roles": roles}

    return {"schema_version": 1, "scope": "native_puredarwin_file_probe", "nonce": NONCE,
            "status": "passed_limited_probe", "exit_code": 0, "parent_pid": 101,
            "process_group": 101, "seed_bytes": length, "denial_errno": 1,
            "baseline_pass": 1, "forbidden_files_unchanged": 1, "setup_errno": 0,
            "interrupted_signal": 0, "qualified_capabilities": [],
            "baseline": phase(102, 103, False), "restricted": phase(104, 105, True)}


def validate(value, code=0, nonce=NONCE, pid=101):
    return probe.validate_report(json.dumps(value).encode(), nonce, code, pid)


class TranscriptTests(unittest.TestCase):
    def test_limited_success_still_cannot_qualify_backend(self):
        outcome = validate(synthetic_report())["assessment"]
        self.assertFalse(outcome["selected_guest_qualified"])
        self.assertEqual(outcome["qualified_capabilities"], [])
        self.assertEqual(outcome["baseline_operations_observed"], 16)
        self.assertEqual(outcome["restricted_operations_observed"], 16)
        self.assertEqual(outcome["failed_required_observations"], [])

    def test_retained_descriptor_bypass_is_nonzero_even_with_fresh_open_denials(self):
        value = synthetic_report()
        for role in value["restricted"]["roles"]:
            for index in (6, 7):
                role["cases"][index] = copy.deepcopy(value["baseline"]["roles"][0]["cases"][index])
        with self.assertRaisesRegex(probe.InvalidReport, "disagrees"):
            validate(value)
        value.update(status="coverage_gap", exit_code=1, forbidden_files_unchanged=0)
        outcome = validate(value, 1)["assessment"]
        self.assertTrue(outcome["required_operation_outcomes"]["descendant"]["deny_read_open"])
        self.assertFalse(outcome["required_operation_outcomes"]["descendant"]["preopened_read"])
        self.assertIn("worker.preopened_write", outcome["failed_required_observations"])
        self.assertIn("forbidden_file_content_or_absence", outcome["failed_required_observations"])

    def test_allowed_sibling_failure_cannot_be_a_pass(self):
        value = synthetic_report()
        value["restricted"]["roles"][1]["cases"][0].update(open_rc=-1, open_errno=1, io_rc=-2, content_ok=0)
        with self.assertRaises(probe.InvalidReport):
            validate(value)
        value.update(status="coverage_gap", exit_code=1)
        self.assertIn("descendant.allow_read", validate(value, 1)["assessment"]["failed_required_observations"])

    def test_wrong_nonce_observed_exit_or_actual_parent_rejected(self):
        for kwargs in ({"nonce": "another_run"}, {"code": 1}, {"pid": 999}):
            with self.subTest(kwargs=kwargs), self.assertRaises(probe.InvalidReport):
                validate(synthetic_report(), **kwargs)

    def test_missing_or_failed_descendant_rejected(self):
        for field, invalid in (("descendant_pid", 0), ("descendant_exit", 3), ("worker_exit", 3),
                               ("worker_signal", 9), ("received", 0), ("setup_errno", 5)):
            value = synthetic_report()
            value["restricted"][field] = invalid
            with self.subTest(field=field), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_absent_api_or_successful_check_alone_cannot_pass(self):
        for mutation in ({"native_api": 0}, {"profile_rc": -1, "profile_errno": 78},
                         {"roles": []}, {"profile_errno": 1}):
            value = synthetic_report()
            value["restricted"].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_missing_duplicate_or_reordered_case_rejected(self):
        for mutation in ("missing", "duplicate", "operation", "role"):
            value = synthetic_report()
            role = value["restricted"]["roles"][1]
            if mutation == "missing":
                role["cases"].pop()
            elif mutation == "duplicate":
                role["cases"][7] = role["cases"][6]
            elif mutation == "operation":
                role["cases"][6]["operation"] = "read"
            else:
                role["role"] = "worker"
            with self.subTest(mutation=mutation), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_unattempted_io_and_wrong_errno_do_not_prove_denial(self):
        for index, change in ((3, {"io_rc": -1, "io_errno": 1}),
                              (6, {"io_rc": -2, "io_errno": 0}),
                              (7, {"io_errno": 13}), (3, {"open_errno": 2})):
            value = synthetic_report()
            value["restricted"]["roles"][0]["cases"][index].update(change)
            with self.subTest(index=index, change=change), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_baseline_sentinel_and_summary_bound(self):
        for field, invalid in (("content_ok", 0), ("io_rc", 1), ("sync_errno", 5)):
            value = synthetic_report()
            value["baseline"]["roles"][0]["cases"][1 if field == "sync_errno" else 0][field] = invalid
            with self.subTest(field=field), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_forged_capability_or_bool_integer_rejected(self):
        for change in ({"qualified_capabilities": ["file-grants"]}, {"baseline_pass": True},
                       {"schema_version": True}, {"process_group": 102}):
            value = synthetic_report()
            value.update(change)
            with self.subTest(change=change), self.assertRaises(probe.InvalidReport):
                validate(value)

    def test_duplicate_json_trailing_output_bad_utf8_and_size_rejected(self):
        raw = json.dumps(synthetic_report()).encode()
        for bad in (raw.replace(b'"schema_version": 1', b'"schema_version": 1, "schema_version": 1'),
                    raw + b"\nextra", b"\xff", b" " * (probe.LOG_LIMIT + 1)):
            with self.subTest(prefix=bad[:20]), self.assertRaises(probe.InvalidReport):
                probe.validate_report(bad, NONCE, 0)


class LocalHarnessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # This CI lane deliberately executes the Linux API-unavailable branch.
        # A Darwin execution needs explicit selected-guest provenance and must
        # not be silently substituted for this harness test.
        if sys.platform != "linux":
            raise RuntimeError("local harness tests require Linux; use an explicit guest capture on Darwin")
        cc = shutil.which("cc")
        if cc is None:
            raise RuntimeError("cc is required; the C fixture test must not skip")
        cls.directory = Path(tempfile.mkdtemp(prefix="m1-sandbox-probe-tests-",
                                             dir=os.environ.get("BURDETTIE_SANDBOX_TEST_ARTIFACTS")))
        print(f"retained sandbox harness artifacts: {cls.directory}", flush=True)
        cls.binary = cls.directory / "probe"
        command = [cc, "-std=c11", "-O0", "-g0", "-Wall", "-Wextra", "-Werror",
                   str(ROOT / "fixtures/m1_sandbox_probe/probe.c"), "-o", str(cls.binary)]
        result = subprocess.run(command, capture_output=True, timeout=30)
        (cls.directory / "compile.stdout").write_bytes(result.stdout)
        (cls.directory / "compile.stderr").write_bytes(result.stderr)
        (cls.directory / "compile.json").write_text(json.dumps({"argv": command, "exit_code": result.returncode}) + "\n")
        if result.returncode != 0:
            raise AssertionError(result.stderr.decode(errors="replace"))
        cls.digest = probe.sha256_file(cls.binary)
        cls.receipt = probe.run_probe(cls.binary, cls.digest, cls.directory / "actual-linux", NONCE)

    def test_actual_baseline_worker_and_descendant_then_blocked_api(self):
        result = self.receipt
        self.assertEqual(result["status"], "blocked_policy", result)
        self.assertEqual(result["actual_exit"], 2)
        self.assertEqual(result["exit_code"], 2)
        report = result["validation"]["report"]
        self.assertEqual(report["baseline_pass"], 1)
        self.assertEqual(report["setup_errno"], 0)
        self.assertEqual(report["baseline"]["worker_exit"], 0)
        self.assertEqual(report["baseline"]["descendant_exit"], 0)
        self.assertEqual(report["restricted"]["native_api"], 0)
        self.assertEqual(report["restricted"]["worker_exit"], 2)
        self.assertEqual(report["restricted"]["profile_rc"], -1)
        self.assertEqual(report["restricted"]["descendant_pid"], 0)
        self.assertEqual(result["validation"]["assessment"]["baseline_operations_observed"], 16)
        self.assertEqual(result["validation"]["assessment"]["restricted_operations_observed"], 0)
        self.assertFalse(result["selected_guest_qualified"])

    def test_actual_nonce_files_and_all_logs_retained(self):
        directory = self.directory / "actual-linux"
        seed = f"Burdettie sandbox probe nonce={NONCE}\n".encode()
        for role in ("worker", "descendant"):
            for name in ("read", "write", "pre-read", "pre-write"):
                self.assertEqual((directory / "fixtures/restricted/deny" / f"{role}-{name}").read_bytes(), seed)
            self.assertFalse((directory / "fixtures/restricted/deny" / f"{role}-create").exists())
            self.assertEqual((directory / "fixtures/baseline/allow" / f"{role}-create").read_bytes(), b"M")
        self.assertEqual(probe.sha256_file(directory / "probe"), self.digest)
        self.assertEqual(probe.sha256_file(directory / "stdout.log"), self.receipt["stdout_sha256"])
        self.assertTrue((directory / "result.json").is_file())

    def test_existing_output_is_never_reused_or_modified(self):
        output = self.directory / "actual-linux"
        before = {str(p.relative_to(output)): probe.sha256_file(p) for p in output.rglob("*") if p.is_file()}
        with self.assertRaises(FileExistsError):
            probe.run_probe(self.binary, self.digest, output, "second_nonce")
        after = {str(p.relative_to(output)): probe.sha256_file(p) for p in output.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_wrong_binary_hash_blocks_before_execution_and_preserves_snapshot(self):
        output = self.directory / "wrong-hash"
        result = probe.run_probe(self.binary, "0" * 64, output, NONCE)
        self.assertEqual(result["status"], "blocked_executable")
        self.assertEqual(result["exit_code"], 2)
        self.assertFalse((output / "fixtures").exists())
        self.assertEqual(probe.sha256_file(output / "probe"), self.digest)

    def test_darwin_api_branch_matches_pinned_declarations_without_linking(self):
        # Declaration-only syntax check: no fake implementation or sandbox call.
        header = self.directory / "sandbox.h"
        header.write_text("#include <stdint.h>\n"
                          "int sandbox_init_with_parameters(const char *, uint64_t, const char *const [], char **);\n"
                          "void sandbox_free_error(char *);\n")
        command = [shutil.which("cc"), "-std=c11", "-Wall", "-Wextra", "-Werror", "-D__APPLE__",
                   "-I", str(self.directory), "-fsyntax-only", str(ROOT / "fixtures/m1_sandbox_probe/probe.c")]
        result = subprocess.run(command, capture_output=True, timeout=30)
        (self.directory / "darwin-syntax.stdout").write_bytes(result.stdout)
        (self.directory / "darwin-syntax.stderr").write_bytes(result.stderr)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))

    def helper(self, name, body):
        path = self.directory / name
        path.write_text("#!/usr/bin/python3\n" + body)
        path.chmod(0o700)
        return path, probe.sha256_file(path)

    def test_timeout_targets_own_group_and_retains_partial_output(self):
        program, digest = self.helper("wait-helper", "import os, time\n"
                                      "child = os.fork()\n"
                                      "if child: print(child, flush=True)\n"
                                      "while True: time.sleep(1)\n")
        unrelated = subprocess.Popen(["/bin/sleep", "10"], start_new_session=True)
        try:
            output = self.directory / "timeout"
            result = probe.run_probe(program, digest, output, NONCE, timeout=0.2)
            self.assertEqual(result["status"], "timed_out", result)
            self.assertEqual(result["exit_code"], 2)
            self.assertTrue(result["leader_exit_observed"])
            self.assertEqual(result["descendant_termination"], "unverified")
            self.assertIsNone(unrelated.poll())
            child = int((output / "stdout.log").read_text().strip())
            # Linux can retain an orphan zombie until its namespace reaper runs;
            # it must not retain a running descendant. No global process search.
            state = Path(f"/proc/{child}/stat")
            for _ in range(50):
                if not state.exists() or state.read_text().split(") ", 1)[1].startswith("Z "):
                    break
                time.sleep(0.01)
            self.assertTrue(not state.exists() or state.read_text().split(") ", 1)[1].startswith("Z "))
        finally:
            unrelated.terminate()
            unrelated.wait(timeout=3)

    def test_output_bound_retains_failed_log(self):
        program, digest = self.helper("output-helper", "import sys\nsys.stdout.buffer.write(b'x' * 131072)\n")
        output = self.directory / "output-limit"
        result = probe.run_probe(program, digest, output, NONCE)
        self.assertEqual(result["status"], "output_limit", result)
        self.assertEqual(result["exit_code"], 2)
        self.assertEqual((output / "stdout.log").stat().st_size, probe.LOG_LIMIT)
        self.assertLessEqual((output / "stderr.log").stat().st_size, probe.LOG_LIMIT)

    def test_timeout_does_not_claim_descendant_termination(self):
        # Concrete negative control: the leader obeys TERM, but its child ignores
        # TERM and waits for this test's private release file (or a 5s bound).
        # We never signal a PGID after the leader has been reaped.
        program, digest = self.helper("leader-only-helper", "import os, signal, sys, time\n"
                                      "read_fd, write_fd = os.pipe()\nchild = os.fork()\n"
                                      "if not child:\n"
                                      "    os.close(read_fd)\n"
                                      "    signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
                                      "    os.write(write_fd, b'r'); os.close(write_fd)\n"
                                      "    until = time.monotonic() + 5\n"
                                      "    while time.monotonic() < until and not os.path.exists(sys.argv[1] + '-release'):\n"
                                      "        time.sleep(0.01)\n"
                                      "    os._exit(0)\n"
                                      "os.close(write_fd); os.read(read_fd, 1); os.close(read_fd)\n"
                                      "print(child, flush=True)\n"
                                      "while True: time.sleep(1)\n")
        output = self.directory / "leader-only"
        try:
            result = probe.run_probe(program, digest, output, NONCE, timeout=0.2)
            self.assertEqual(result["status"], "timed_out", result)
            child = int((output / "stdout.log").read_text().strip())
            state = Path(f"/proc/{child}/stat").read_text().split(") ", 1)[1]
            self.assertFalse(state.startswith("Z "), "negative control child did not remain live")
            self.assertNotIn("termination_confirmed", result)
            self.assertTrue(result["leader_exit_observed"])
            self.assertEqual(result["descendant_termination"], "unverified")
            self.assertEqual(result["exit_code"], 2)
        finally:
            (output / "fixtures-release").touch()

    def test_nonzero_child_exit_cannot_reuse_a_passing_json_body(self):
        program, digest = self.helper("wrong-exit-helper", "import json, os\n"
                                      + "value = " + repr(synthetic_report()) + "\n"
                                      "value['parent_pid'] = value['process_group'] = os.getpid()\n"
                                      "print(json.dumps(value))\nraise SystemExit(1)\n")
        result = probe.run_probe(program, digest, self.directory / "wrong-exit", NONCE)
        self.assertEqual(result["status"], "invalid_report")
        self.assertEqual(result["actual_exit"], 1)
        self.assertEqual(result["exit_code"], 2)
        self.assertIn("observed exit", result["error"])


if __name__ == "__main__":
    unittest.main()
