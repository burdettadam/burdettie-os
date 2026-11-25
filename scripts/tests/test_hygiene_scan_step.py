"""Exercise the actual workflow shell: diagnostics must not turn failures green."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]


def scan_step():
    text = (ROOT / ".github/workflows/repository-hygiene.yml").read_text(encoding="utf-8")
    _, found, section = text.partition("      - name: Scan reachable history for secrets\n")
    if not found:
        raise AssertionError("Required secret-scanning step is missing")
    _, found, script = section.partition("        run: |\n")
    if not found:
        raise AssertionError("Required secret-scanning shell is missing")
    return textwrap.dedent(script)


class ScanStepTests(unittest.TestCase):
    def run_step(self, status, report):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            shutil.copyfile(ROOT / "scripts/summarize_gitleaks.py", root / "scripts/summarize_gitleaks.py")
            temporary = root / "runner"
            temporary.mkdir()
            fake = temporary / "gitleaks"
            fake.write_text(
                "#!/usr/bin/env python3\n"
                "import os, pathlib, sys\n"
                "arguments = sys.argv[1:]\n"
                "required = {'git', '--config', '.gitleaks.toml', '--redact', '--no-banner', '--log-opts=--all', '--report-format=json', '.'}\n"
                "if not required.issubset(arguments): sys.exit(97)\n"
                "path = next(a.split('=', 1)[1] for a in arguments if a.startswith('--report-path='))\n"
                "if os.environ['FAKE_REPORT'] != 'MISSING':\n"
                "    pathlib.Path(path).write_text(os.environ['FAKE_REPORT'])\n"
                "sys.exit(int(os.environ['FAKE_STATUS']))\n",
                encoding="utf-8",
            )
            fake.chmod(0o700)
            env = dict(os.environ, RUNNER_TEMP=str(temporary), FAKE_STATUS=str(status), FAKE_REPORT=report)
            result = subprocess.run(["bash", "-c", scan_step()], cwd=root, env=env,
                                    capture_output=True, text=True, timeout=10, check=False)
            self.assertEqual(list(temporary.glob("gitleaks-report.*")), [])
            self.assertNotIn("PRIVATE", result.stdout + result.stderr)
            return result

    def test_clean_scan_stays_successful(self):
        result = self.run_step(0, "[]")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["finding_count"], 0)

    def test_findings_stay_failed_and_only_locations_are_printed(self):
        report = json.dumps([{"RuleID": "test-rule", "Commit": "b" * 40, "File": "fixture.txt",
                              "StartLine": 1, "EndLine": 1, "Secret": "PRIVATE", "Match": "PRIVATE"}])
        result = self.run_step(1, report)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["finding_count"], 1)

    def test_original_nondefault_scanner_exit_is_preserved(self):
        self.assertEqual(self.run_step(7, "[]").returncode, 7)

    def test_invalid_report_cannot_turn_clean_scan_green(self):
        self.assertEqual(self.run_step(0, "PRIVATE_INVALID_JSON").returncode, 1)

    def test_missing_report_cannot_turn_clean_scan_green(self):
        self.assertEqual(self.run_step(0, "MISSING").returncode, 1)

    def test_scanner_error_wins_when_report_is_missing(self):
        self.assertEqual(self.run_step(9, "MISSING").returncode, 9)

    def test_scanner_error_wins_when_report_is_invalid(self):
        self.assertEqual(self.run_step(5, "PRIVATE_INVALID_JSON").returncode, 5)


if __name__ == "__main__":
    unittest.main()
