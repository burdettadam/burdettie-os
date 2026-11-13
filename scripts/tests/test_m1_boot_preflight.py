"""Preflight checks must not confuse incomplete evidence with readiness."""

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "m1_boot_preflight.py"
SPEC = importlib.util.spec_from_file_location("m1_boot_preflight", SCRIPT)
preflight = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(preflight)


class BootPreflightTests(unittest.TestCase):
    def report(self, source, *, nix=True, head=None, dirty="", submodules="", root=None):
        calls = []

        def response(argv):
            calls.append(argv)
            if argv[-1] == "--version":
                output = "test tool version\n"
            elif argv[-1] == "--show-toplevel":
                output = str(root or source) + "\n"
            elif argv[-1] == "HEAD":
                output = (head or preflight.PUREDARWIN_REVISION) + "\n"
            elif "submodule" in argv:
                output = submodules
            else:
                output = dirty
            return {"available": True, "exit_code": 0, "stdout": output}

        def which(name):
            return "/test/" + name if name == "git" or (name == "nix" and nix) else None

        with patch.object(preflight.platform, "system", return_value="Linux"), \
             patch.object(preflight.platform, "machine", return_value="x86_64"), \
             patch.object(preflight.shutil, "which", side_effect=which), \
             patch.object(preflight, "command_output", side_effect=response):
            report = preflight.collect_report(source)
        return report, calls

    def source(self, directory):
        source = Path(directory)
        (source / "flake.lock").write_text("{}\n")
        return source

    def test_missing_nix_and_source_block(self):
        report, _ = self.report(None, nix=False)
        self.assertEqual(report["status"], "blocked")
        self.assertTrue(any("nix" in value for value in report["blockers"]))
        self.assertTrue(any("No pinned" in value for value in report["blockers"]))
        self.assertFalse(report["boot_executed"])

    def test_matching_clean_root_is_preflight_ready_only(self):
        with tempfile.TemporaryDirectory() as directory:
            report, calls = self.report(self.source(directory), submodules=" " + "a" * 40 + " external/example\n")
        self.assertEqual(report["status"], "ready_for_spike")
        self.assertFalse(report["build_executed"])
        self.assertFalse(report["boot_executed"])
        git_calls = [argv for argv in calls if "-C" in argv]
        self.assertTrue(git_calls)
        for argv in git_calls:
            self.assertIn("--no-optional-locks", argv)
            self.assertIn("core.fsmonitor=false", argv)
        status = next(argv for argv in git_calls if "--porcelain" in argv)
        self.assertIn("--untracked-files=all", status)
        self.assertIn("--ignore-submodules=none", status)

    def test_revision_mismatch_blocks(self):
        with tempfile.TemporaryDirectory() as directory:
            report, _ = self.report(self.source(directory), head="0" * 40)
        self.assertEqual(report["status"], "blocked")

    def test_dirty_source_blocks(self):
        with tempfile.TemporaryDirectory() as directory:
            for dirty in (" M flake.lock\n", "?? new-source.c\n", " m external/example\n"):
                with self.subTest(dirty=dirty):
                    report, _ = self.report(self.source(directory), dirty=dirty)
                    self.assertEqual(report["status"], "blocked")

    def test_child_directory_is_not_repository_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            child = root / "child"
            child.mkdir()
            report, _ = self.report(self.source(child), root=root)
        self.assertEqual(report["status"], "blocked")
        self.assertFalse(report["checkout"]["exact_repository_root"])

    def test_mismatched_submodule_after_4096_bytes_blocks(self):
        matching = (" " + "a" * 40 + " external/matching\n") * 100
        self.assertGreater(len(matching), 4096)
        with tempfile.TemporaryDirectory() as directory:
            for marker in ("-", "+", "U"):
                with self.subTest(marker=marker):
                    output = matching + marker + "b" * 40 + " external/bad\n"
                    report, _ = self.report(self.source(directory), submodules=output)
                    self.assertEqual(report["status"], "blocked")
                    self.assertFalse(report["checkout"]["submodules_match"])

    def test_command_output_preserves_validation_tail(self):
        output = " " * 5000 + "+mismatched-tail\n"
        completed = subprocess.CompletedProcess(["git"], 0, stdout=output, stderr="")
        with patch.object(preflight.subprocess, "run", return_value=completed):
            result = preflight.command_output(["git"])
        self.assertTrue(result["available"])
        self.assertEqual(result["stdout"], output)

    def test_output_cap_fails_closed(self):
        completed = subprocess.CompletedProcess(
            ["git"], 0, stdout="a" * (preflight.MAX_COMMAND_OUTPUT_BYTES + 1), stderr="")
        with patch.object(preflight.subprocess, "run", return_value=completed):
            result = preflight.command_output(["git"])
        self.assertFalse(result["available"])
        self.assertEqual(result["error"], "output_limit_exceeded")
        self.assertNotIn("stdout", result)


if __name__ == "__main__":
    unittest.main()
