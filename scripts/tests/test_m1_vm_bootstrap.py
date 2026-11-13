"""Protocol and failure-path tests; these do not boot or qualify PureDarwin."""

import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


spec = importlib.util.spec_from_file_location(
    "m1_vm_bootstrap", Path(__file__).resolve().parents[1] / "m1_vm_bootstrap.py")
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class BootstrapProtocolTests(unittest.TestCase):
    def test_echoed_command_and_partial_line_are_not_execution_evidence(self):
        for make in (
            lambda: bootstrap.shell_challenge("0123456789abcdef" * 2),
            lambda: bootstrap.data_challenge("write", "a" * 32, "b" * 32),
            lambda: bootstrap.data_challenge("read", "a" * 32, "b" * 32),
        ):
            command, response = make()
            self.assertNotIn(response, command.encode())
            self.assertFalse(bootstrap.complete_line(command.encode() + b"\r\n", response))
            self.assertFalse(bootstrap.complete_line(response, response))
            self.assertFalse(bootstrap.complete_line(b"prefix" + response + b"\n", response))
            self.assertFalse(bootstrap.complete_line(response + b"suffix\n", response))
            self.assertTrue(bootstrap.complete_line(b"echoed input\r\n" + response + b"\r\n", response))

    def test_persistence_command_rejects_missing_and_changed_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            nonce = "0123456789abcdef" * 2
            command, response = bootstrap.data_challenge("read", nonce, "b" * 32)
            command = command.replace("/var/tmp", str(directory))
            missing = subprocess.run(["sh", "-c", command], capture_output=True)
            self.assertFalse(bootstrap.complete_line(missing.stdout, response))
            data_file = directory / f"burdettie-m1-{nonce}.txt"
            data_file.write_bytes((nonce + "\n").encode())
            valid = subprocess.run(["sh", "-c", command], capture_output=True)
            self.assertTrue(bootstrap.complete_line(valid.stdout, response))
            data_file.write_bytes((nonce + "\n\n").encode())
            changed = subprocess.run(["sh", "-c", command], capture_output=True)
            self.assertFalse(bootstrap.complete_line(changed.stdout, response))

    def test_qemu_has_no_network_or_monitor_listener_and_uses_writable_disk(self):
        command = bootstrap.qemu_command("qemu", Path("/test"), 2048, 2)
        for option, value in (("-nic", "none"), ("-monitor", "none"), ("-display", "none"),
                              ("-serial", "stdio"), ("-accel", "tcg,thread=single")):
            self.assertEqual(command[command.index(option) + 1], value)
        self.assertNotIn("snapshot=on", " ".join(command))
        self.assertNotIn("hostfwd", " ".join(command))
        self.assertIn("id=root,format=raw,file=/test/guest.raw", command)

    def test_existing_output_is_preserved_and_blocked(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            sentinel = directory / "result.json"
            sentinel.write_text("keep this evidence")
            args = argparse.Namespace(output_dir=directory)
            report = bootstrap.run(args)
            self.assertEqual(report["status"], "blocked")
            self.assertEqual(sentinel.read_text(), "keep this evidence")

    @unittest.skipUnless(sys.platform == "linux" and hasattr(os, "sched_getaffinity"), "Linux preflight")
    def test_evidence_read_error_after_execution_is_failure_not_preflight_block(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            image = directory / "input.img"
            image.write_bytes(b"test")
            image_digest = bootstrap.sha256(image)
            real_sha256 = bootstrap.sha256

            def digest(path):
                if path.suffix == ".log":
                    raise OSError("simulated evidence read failure")
                return real_sha256(path)

            def executed_phase(command, output, phase, *args):
                (output / (phase + "-serial.log")).write_text("synthetic executed evidence")
                return {"status": "pass", "termination": {"process_exited": True}}

            args = argparse.Namespace(output_dir=directory / "run", image=image,
                                      firmware_code=image, firmware_vars=image,
                                      memory_mib=1024, vcpus=1, timeout=30)
            with mock.patch.object(bootstrap, "IMAGE_BYTES", 4), \
                    mock.patch.object(bootstrap, "IMAGE_SHA256", image_digest), \
                    mock.patch.object(bootstrap, "sha256", side_effect=digest), \
                    mock.patch.object(bootstrap, "memory_available", return_value=8 * 1024 * bootstrap.MIB), \
                    mock.patch.object(bootstrap.shutil, "which", return_value=sys.executable), \
                    mock.patch.object(bootstrap.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, b"QEMU test\n")), \
                    mock.patch.object(bootstrap, "run_phase", side_effect=executed_phase):
                report = bootstrap.run(args)
            self.assertEqual(len(report["phases"]), 2)
            self.assertEqual(report["status"], "fail")
            self.assertIn("evidence read failure", report["error"])

    @unittest.skipUnless(sys.platform == "linux" and hasattr(os, "sched_getaffinity"), "Linux process controls")
    def test_echo_only_console_times_out_and_process_is_reaped(self):
        emulator = "import sys\nsys.stdout.write('# '); sys.stdout.flush()\nfor line in sys.stdin:\n sys.stdout.write(line); sys.stdout.flush()\n"
        with tempfile.TemporaryDirectory() as temporary:
            result = bootstrap.run_phase(
                [sys.executable, "-u", "-c", emulator], Path(temporary), "write", "a" * 32,
                1, 1024, [min(os.sched_getaffinity(0))])
            self.assertEqual(result["status"], "fail")
            self.assertFalse(result["shell_verified"])
            self.assertFalse(result["data_verified"])
            self.assertEqual(result["reason"], "timeout_waiting_for_shell_response")
            self.assertTrue(result["termination"]["process_exited"])

    @unittest.skipUnless(sys.platform == "linux" and hasattr(os, "sched_getaffinity"), "Linux process controls")
    def test_synthetic_console_protocol_requires_second_process_readback(self):
        # This ordinary Linux subprocess only tests the serial protocol. It
        # supplies a synthetic uname and redirects guest paths to a temp dir.
        emulator = """import os, subprocess, sys
root = sys.argv[1]
sys.stdout.write('# '); sys.stdout.flush()
for line in sys.stdin:
    sys.stdout.write(line); sys.stdout.flush()
    line = line.replace('/var/tmp', root).replace('&& sync', '&& :')
    prefix = 'uname() { if [ "$1" = "-s" ]; then echo Darwin; else echo x86_64; fi; }; '
    subprocess.run(['sh', '-c', prefix + line], check=False)
    sys.stdout.write('# '); sys.stdout.flush()
"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            command = [sys.executable, "-u", "-c", emulator, temporary]
            arguments = ("0123456789abcdef" * 2, 3, 1024, [min(os.sched_getaffinity(0))])
            first = bootstrap.run_phase(command, directory, "write", *arguments)
            self.assertEqual(first["status"], "pass", first)
            self.assertTrue(first["termination"]["process_exited"])
            second = bootstrap.run_phase(command, directory, "read", *arguments)
            self.assertEqual(second["status"], "pass", second)
            self.assertTrue(second["termination"]["process_exited"])
            self.assertNotEqual(first["shell_expected"], second["shell_expected"])
            self.assertFalse(second["kernel_banner_observed"])


if __name__ == "__main__":
    unittest.main()
