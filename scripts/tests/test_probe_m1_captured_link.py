"""Negative-link acceptance must match an actual retained undefined-symbol failure."""
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import probe_m1_captured_link as probe
from m1_evidence import EvidenceError, digest, encoded


class NegativeProbeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.stderr = b'ld: error: undefined symbol: _owned_missing_for_negative_link\n'
        (self.root / 'link.stderr.log').write_bytes(self.stderr)
        self.result = {'status': 'fail', 'phase': {'exit_code': 1, 'timed_out': False,
            'logs_truncated': False, 'logs': {'stderr': {'sha256': digest(self.stderr)}}}}

    def runcheck(self):
        (self.root / 'result.json').write_bytes(encoded(self.result))
        return probe.expected_negative(self.result, self.root)

    def test_expected_negative_keeps_failure_and_passes_only_probe_assertion(self):
        self.assertTrue(self.runcheck())
        self.assertEqual(self.result['status'], 'fail')

    def test_wrong_nonzero_error_is_not_expected_rejection(self):
        (self.root / 'link.stderr.log').write_bytes(b'linker lacks platform support\n')
        with self.assertRaisesRegex(EvidenceError, 'another reason'): self.runcheck()

    def test_timeout_signal_success_and_boolean_exit_are_not_expected_negative(self):
        for value in (0, -15, True, None):
            self.result['phase']['exit_code'] = value
            with self.subTest(value=value), self.assertRaises(EvidenceError): self.runcheck()
        self.result['phase'].update(exit_code=1, timed_out=True)
        with self.assertRaises(EvidenceError): self.runcheck()

    def test_tampered_stderr_or_success_capture_is_rejected(self):
        self.result['phase']['logs']['stderr']['sha256'] = '0' * 64
        with self.assertRaisesRegex(EvidenceError, 'inconsistent'): self.runcheck()
        self.result['phase']['logs']['stderr']['sha256'] = digest(self.stderr)
        (self.root / 'link-capture.json').write_text('{}')
        with self.assertRaisesRegex(EvidenceError, 'successful capture'): self.runcheck()

    def test_missing_terminal_receipt_is_not_negative_evidence(self):
        with self.assertRaises(OSError): probe.expected_negative(self.result, self.root)

    def test_existing_probe_directory_is_preserved(self):
        saved = self.root / 'captured-links'; saved.mkdir()
        (saved / 'result.json').write_bytes(b'keep')
        with contextlib.redirect_stdout(io.StringIO()):
            rc = probe.main(['--input-dir', str(self.root), '--clang', '/missing', '--linker', '/missing'])
        self.assertEqual(rc, 1)
        self.assertEqual((saved / 'result.json').read_bytes(), b'keep')


if __name__ == '__main__': unittest.main()
