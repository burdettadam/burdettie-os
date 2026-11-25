"""Real pthread executions and negative controls; no synthetic passing fixture."""
from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import validate_m1_thread_state as check

SOURCE = Path(__file__).resolve().parents[2] / "fixtures/m1_thread_state_probe/probe.c"

# Interposition is owned test code linked into separate executables. The shipped
# fixture has no fault/skip/shortened-count switch. No live process is patched.
FAULTS = {
    "setspecific": ("pthread_setspecific", """
int m1_fault(pthread_key_t key, const void *value) {
    (void)key; (void)value; return 0;
}"""),
    "once": ("pthread_once", """
int m1_fault(pthread_once_t *once, void (*callback)(void)) {
    (void)once; (void)callback; return 0;
}"""),
    "destructor": ("pthread_key_create", """
int m1_fault(pthread_key_t *key, void (*destroy)(void *)) {
    (void)destroy; return pthread_key_create(key, NULL);
}"""),
    "create_partial": ("pthread_create", """
int m1_fault(pthread_t *thread, const pthread_attr_t *attr,
             void *(*start)(void *), void *arg) {
    static int attempts;
    if (attempts++ == 0) return pthread_create(thread, attr, start, arg);
    return EAGAIN;
}"""),
    "create_none": ("pthread_create", """
int m1_fault(pthread_t *thread, const pthread_attr_t *attr,
             void *(*start)(void *), void *arg) {
    (void)thread; (void)attr; (void)start; (void)arg; return EAGAIN;
}"""),
    "join_value": ("pthread_join", """
int m1_fault(pthread_t thread, void **value) {
    int rc = pthread_join(thread, value);
    if (rc == 0 && value != NULL) *value = NULL;
    return rc;
}"""),
    "key_create": ("pthread_key_create", """
int m1_fault(pthread_key_t *key, void (*destroy)(void *)) {
    (void)key; (void)destroy; return EAGAIN;
}"""),
    "timedwait": ("pthread_cond_timedwait", """
int m1_fault(pthread_cond_t *cond, pthread_mutex_t *mutex, const struct timespec *deadline) {
    (void)cond; (void)mutex; (void)deadline;
    /* Preserve the caller's lock as a real timed wait must on return. */
    return ETIMEDOUT;
}"""),
}


def expected_report():
    return dict(check.EXPECTED, scope="m1_pthread_state_execution", platform="linux",
                architecture="x86_64", status="pass")


class ReportTests(unittest.TestCase):
    def test_valid_shape_has_no_execution_authentication_claim(self):
        self.assertEqual(check.validate(json.dumps(expected_report()).encode(), 0, "linux", "x86_64"),
                         expected_report())

    def test_failed_signal_timeout_or_boolean_exit_is_not_pass(self):
        raw = json.dumps(expected_report()).encode()
        for code in (1, 2, -9, -15, 124, True, False, None):
            with self.subTest(code=code), self.assertRaises(check.EvidenceError):
                check.validate(raw, code, "linux", "x86_64")

    def test_every_counter_must_match_and_be_integer(self):
        for name, value in check.EXPECTED.items():
            for replacement in (value + 1, value - 1, str(value), float(value), True, None):
                altered = expected_report(); altered[name] = replacement
                with self.subTest(name=name, value=replacement), self.assertRaises(check.EvidenceError):
                    check.validate(json.dumps(altered).encode(), 0, "linux", "x86_64")

    def test_target_scope_status_and_unexpected_fields_rejected(self):
        for key, value in (("platform", "darwin"), ("architecture", "arm64"),
                           ("scope", "other"), ("status", "fail"), ("guest_boot", "pass")):
            altered = expected_report(); altered[key] = value
            with self.subTest(key=key), self.assertRaises(check.EvidenceError):
                check.validate(json.dumps(altered).encode(), 0, "linux", "x86_64")
        for key in expected_report():
            altered = expected_report(); del altered[key]
            with self.subTest(missing=key), self.assertRaises(check.EvidenceError):
                check.validate(json.dumps(altered).encode(), 0, "linux", "x86_64")

    def test_invalid_json_duplicate_keys_trailing_records_and_bounds(self):
        good = json.dumps(expected_report()).encode()
        for raw in (b"", b"[]", b"\xff", good + good, b"x" * (check.MAX_REPORT + 1),
                    good.replace(b'"schema_version": 1', b'"schema_version": 1,"schema_version": 1'),
                    good.replace(b'"failures": 0', b'"failures": NaN')):
            with self.subTest(raw=raw[:40]), self.assertRaises((ValueError, UnicodeError)):
                check.validate(raw, 0, "linux", "x86_64")

    def test_cli_checks_retained_file_and_does_not_modify_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "result.json"; raw = json.dumps(expected_report()).encode(); path.write_bytes(raw)
            argv = ["--report", str(path), "--exit-code", "0", "--platform", "linux", "--architecture", "x86_64"]
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(check.main(argv), 0)
                argv[3] = "1"
                self.assertEqual(check.main(argv), 1)
            self.assertEqual(path.read_bytes(), raw)

    def test_cli_refuses_symlink_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "result.json"; path.write_text(json.dumps(expected_report()))
            link = Path(tmp) / "link"; link.symlink_to(path)
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(check.main(["--report", str(link), "--exit-code", "0", "--platform", "linux",
                                             "--architecture", "x86_64"]), 1)


class ThreadExecutionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Missing trusted host tools must fail, not become skipped execution.
        cls.compiler = shutil.which("cc")
        if cls.compiler is None:
            raise RuntimeError("A trusted host cc compiler is required")
        cls.temp = tempfile.TemporaryDirectory(prefix="m1-thread-state-")
        cls.root = Path(cls.temp.name)
        cls.platform_name = {"Linux": "linux", "Darwin": "darwin"}.get(platform.system())
        cls.arch = {"x86_64": "x86_64", "arm64": "arm64", "aarch64": "arm64"}.get(platform.machine())
        if cls.platform_name is None or cls.arch is None:
            raise RuntimeError("Unsupported reference host")
        cls.compile("reference")

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    @classmethod
    def compile(cls, name, optimization="-O2", fault=None, shared_errno=False):
        flags = [cls.compiler, "-std=c11", optimization, "-Wall", "-Wextra", "-Werror", "-pthread"]
        obj = cls.root / (name + ".o"); executable = cls.root / name
        if fault:
            symbol, body = fault
            flags += ["-D" + symbol + "=m1_fault"]
            shim = cls.root / (name + "_shim.c")
            shim.write_text("#define _POSIX_C_SOURCE 200809L\n#include <pthread.h>\n#include <errno.h>\n#include <time.h>\n" + body)
        if shared_errno:
            header = cls.root / (name + ".h")
            header.write_text("#define _POSIX_C_SOURCE 200809L\n#include <errno.h>\n#undef errno\nint *m1_fault_errno(void);\n#define errno (*m1_fault_errno())\n")
            flags += ["-include", str(header)]
            shim = cls.root / (name + "_shim.c")
            shim.write_text("int *m1_fault_errno(void) { static int shared; return &shared; }\n")
        subprocess.run(flags + ["-c", str(SOURCE), "-o", str(obj)], check=True, capture_output=True, timeout=15)
        subprocess.run([cls.compiler, "-pthread", str(obj)] + ([str(shim)] if fault or shared_errno else [])
                       + ["-o", str(executable)], check=True, capture_output=True, timeout=15)
        return executable

    def execute(self, binary):
        # Every owned executable is a separate process; no shared globals leak
        # between tests. Timeout is a test failure, not expected negative output.
        result = subprocess.run([str(binary)], capture_output=True, timeout=6)
        self.assertLess(len(result.stdout) + len(result.stderr), check.MAX_REPORT)
        return result

    def assert_negative(self, result):
        self.assertEqual(result.returncode, 1, result.stderr.decode(errors="replace"))
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "fail")
        self.assertGreater(report["failures"], 0)
        self.assertEqual(report["workers_started"], report["workers_joined"])
        with self.assertRaises(check.EvidenceError):
            check.validate(result.stdout, result.returncode, self.platform_name, self.arch)
        # Relabeling exit/status cannot hide the actual failed counters.
        report["status"] = "pass"
        with self.assertRaises(check.EvidenceError):
            check.validate(json.dumps(report).encode(), 0, self.platform_name, self.arch)
        return report

    def test_real_threads_repeat_twenty_complete_executions(self):
        for attempt in range(20):
            with self.subTest(attempt=attempt):
                result = self.execute(self.root / "reference")
                self.assertEqual(result.stderr, b"")
                check.validate(result.stdout, result.returncode, self.platform_name, self.arch)

    def test_unoptimized_execution(self):
        result = self.execute(self.compile("unoptimized", optimization="-O0"))
        check.validate(result.stdout, result.returncode, self.platform_name, self.arch)

    def test_noop_setspecific_cannot_pass(self):
        report = self.assert_negative(self.execute(self.compile("setspecific", fault=FAULTS["setspecific"])))
        self.assertEqual(report["destructors"], 0)

    def test_noop_once_cannot_pass(self):
        report = self.assert_negative(self.execute(self.compile("once", fault=FAULTS["once"])))
        self.assertEqual(report["once_calls"], 0)

    def test_missing_destructors_cannot_pass(self):
        report = self.assert_negative(self.execute(self.compile("destructor", fault=FAULTS["destructor"])))
        self.assertEqual(report["destructors"], 0)
        self.assertEqual(report["rounds"], 192)

    def test_partial_create_failure_joins_every_started_worker(self):
        report = self.assert_negative(self.execute(self.compile("create_partial", fault=FAULTS["create_partial"])))
        self.assertEqual(report["workers_started"], 1)
        self.assertEqual(report["workers_joined"], 1)
        self.assertEqual(report["destructors"], 1)

    def test_total_create_failure_is_not_empty_success(self):
        report = self.assert_negative(self.execute(self.compile("create_none", fault=FAULTS["create_none"])))
        self.assertEqual(report["workers_started"], 0)

    def test_wrong_join_value_cannot_pass(self):
        report = self.assert_negative(self.execute(self.compile("join_value", fault=FAULTS["join_value"])))
        self.assertEqual(report["workers_joined"], 6)
        self.assertEqual(report["destructors"], 6)

    def test_key_creation_failure_cannot_pass(self):
        report = self.assert_negative(self.execute(self.compile("key_create", fault=FAULTS["key_create"])))
        self.assertEqual(report["pthread_errors"], 2)

    def test_wait_error_stops_and_joins_workers(self):
        report = self.assert_negative(self.execute(self.compile("timedwait", fault=FAULTS["timedwait"])))
        self.assertLess(report["rounds"], 192)
        self.assertEqual(report["workers_joined"], 6)

    def test_shared_errno_storage_is_detected_without_data_race(self):
        # All writes in the fixture occur under mutex. The fault intentionally
        # aliases storage, but introduces no concurrent unsynchronized writes.
        report = self.assert_negative(self.execute(self.compile("shared_errno", shared_errno=True)))
        self.assertEqual(report["errno_address_checks"], 384)
        self.assertGreaterEqual(report["failures"], 384)


if __name__ == "__main__":
    unittest.main()
