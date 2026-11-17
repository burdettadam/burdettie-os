"""Metadata diagnostics must never echo a finding's secret-bearing fields."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "summarize_gitleaks", Path(__file__).resolve().parents[1] / "summarize_gitleaks.py"
)
assert SPEC is not None and SPEC.loader is not None
SUMMARY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUMMARY)


def finding(**updates):
    value = {
        "RuleID": "generic-api-key",
        "Commit": "a" * 40,
        "File": "fixtures/example.txt",
        "StartLine": 7,
        "EndLine": 8,
        "Match": "PRIVATE_MATCH_SENTINEL",
        "Secret": "PRIVATE_VALUE_SENTINEL",
        "Message": "PRIVATE_MESSAGE_SENTINEL",
        "Author": "PRIVATE_AUTHOR_SENTINEL",
        "Email": "PRIVATE_EMAIL_SENTINEL",
        "Extra": {"nested": "PRIVATE_EXTRA_SENTINEL"},
    }
    value.update(updates)
    return value


class SummaryTests(unittest.TestCase):
    def test_allowlisted_location_fields_only(self):
        result = SUMMARY.summarize(json.dumps([finding()]).encode())
        self.assertEqual(result, {
            "finding_count": 1,
            "locations": [{"rule": "generic-api-key", "commit": "a" * 40,
                           "file": "fixtures/example.txt", "first_line": 7, "last_line": 8}],
        })
        self.assertNotIn("PRIVATE", json.dumps(result))

    def test_empty_report(self):
        self.assertEqual(SUMMARY.summarize(b"[]"), {"finding_count": 0, "locations": []})

    def test_multiple_findings_are_not_deduplicated(self):
        result = SUMMARY.summarize(json.dumps([finding(), finding()]).encode())
        self.assertEqual(result["finding_count"], 2)
        self.assertEqual(len(result["locations"]), 2)

    def test_wrong_shapes_are_rejected(self):
        for value in [None, {}, "PRIVATE", [None], [1]]:
            with self.subTest(value=value), self.assertRaises(SUMMARY.InvalidReport):
                SUMMARY.summarize(json.dumps(value).encode())

    def test_duplicate_fields_are_rejected(self):
        with self.assertRaises(SUMMARY.InvalidReport):
            SUMMARY.summarize(b'[{"Secret":"PRIVATE_ONE","Secret":"PRIVATE_TWO"}]')

    def test_malformed_and_non_utf8_inputs_are_rejected(self):
        for value in [b"PRIVATE_INVALID_JSON", b"\xff", b"[" * 2000]:
            with self.subTest(value=value[:20]), self.assertRaises(SUMMARY.InvalidReport):
                SUMMARY.summarize(value)

    def test_rule_and_commit_are_validated(self):
        for updates in [{"RuleID": "::warning::PRIVATE"}, {"RuleID": ""},
                        {"Commit": "PRIVATE_COMMIT"}, {"Commit": None}]:
            with self.subTest(updates=updates), self.assertRaises(SUMMARY.InvalidReport):
                SUMMARY.summarize(json.dumps([finding(**updates)]).encode())

    def test_paths_reject_control_characters_and_traversal(self):
        for path in ["", "/private/file", "../file", "a/../file", "x\\file", "a\n::error::PRIVATE",
                     "a\x1b[0m", "a\x7f", "nonascii-\u200b", "a" * 2049, None]:
            with self.subTest(path=path), self.assertRaises(SUMMARY.InvalidReport):
                SUMMARY.summarize(json.dumps([finding(File=path)]).encode())

    def test_line_range_rejects_bools_and_inverted_ranges(self):
        for first, last in [(True, 8), (1, True), (0, 8), (8, 7), (1.5, 8), ("7", 8)]:
            with self.subTest(first=first, last=last), self.assertRaises(SUMMARY.InvalidReport):
                SUMMARY.summarize(json.dumps([finding(StartLine=first, EndLine=last)]).encode())

    def test_size_and_count_bounds(self):
        with self.assertRaises(SUMMARY.InvalidReport):
            SUMMARY.summarize(b" " * (SUMMARY.MAX_BYTES + 1))
        with self.assertRaises(SUMMARY.InvalidReport):
            SUMMARY.summarize(json.dumps([{}] * (SUMMARY.MAX_FINDINGS + 1)).encode())

    def test_cli_success_does_not_print_secrets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            path.write_text(json.dumps([finding()]), encoding="utf-8")
            output, errors = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertEqual(SUMMARY.main([str(path)]), 0)
            self.assertNotIn("PRIVATE", output.getvalue() + errors.getvalue())
            self.assertEqual(json.loads(output.getvalue())["finding_count"], 1)

    def test_cli_failure_does_not_echo_path_or_parser_input(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "PRIVATE_PATH"
            for content in [None, b"PRIVATE_JSON"]:
                if content is not None:
                    path.write_bytes(content)
                output, errors = io.StringIO(), io.StringIO()
                with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                    self.assertEqual(SUMMARY.main([str(path)]), 1)
                self.assertEqual(output.getvalue(), "")
                self.assertNotIn("PRIVATE", errors.getvalue())

    def test_usage_error(self):
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(SUMMARY.main([]), 2)


if __name__ == "__main__":
    unittest.main()
