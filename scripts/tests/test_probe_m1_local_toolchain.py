import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import probe_m1_local_toolchain as probe
from m1_evidence import EvidenceError


class ProbeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.report = {'phases': []}

    def phase(self, source, timeout=2):
        return probe.run_phase([sys.executable, '-I', '-c', source], cwd=self.root, directory=self.root,
                               name='owned-child', report=self.report, timeout=timeout)

    def test_actual_success_captures_stdout_and_exit(self):
        result = self.phase('print("owned")')
        self.assertEqual(result['exit_code'], 0)
        self.assertEqual((self.root / result['stdout_file']).read_text(), 'owned\n')

    def test_actual_nonzero_child_remains_failed_with_logs(self):
        with self.assertRaisesRegex(EvidenceError, 'exit 3'):
            self.phase('import sys; print("retained failure"); sys.exit(3)')
        self.assertEqual(self.report['phases'][0]['status'], 'fail')
        self.assertEqual(self.report['phases'][0]['exit_code'], 3)
        self.assertIn('retained failure', (self.root / 'owned-child.stdout.log').read_text())

    def test_actual_timeout_is_not_success(self):
        with self.assertRaisesRegex(EvidenceError, 'deadline'):
            self.phase('import time; time.sleep(10)', timeout=0.05)
        result = self.report['phases'][0]
        self.assertTrue(result['timed_out']); self.assertNotEqual(result['exit_code'], 0)

    def test_missing_tool_is_retained_failure_not_a_skipped_pass(self):
        path = self.root / 'probe'
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            rc = probe.main(['--output-dir', str(path), '--clang', '/nonexistent/owned-clang'])
        report = json.loads(stream.getvalue())
        self.assertEqual(rc, 1); self.assertEqual(report['status'], 'fail')
        self.assertEqual(report['selected_source_build'], 'not_run')
        self.assertTrue((path / 'result.json').exists())

    def test_existing_output_is_never_reused(self):
        path = self.root / 'probe'; path.mkdir(); marker = path / 'marker'; marker.write_bytes(b'keep')
        with contextlib.redirect_stdout(io.StringIO()):
            rc = probe.main(['--output-dir', str(path)])
        self.assertEqual(rc, 1); self.assertEqual(marker.read_bytes(), b'keep')
        self.assertEqual(len(list(path.iterdir())), 1)

    def test_tool_identity_keeps_multicall_symlink_name(self):
        path = self.root / 'python-link'; path.symlink_to(sys.executable)
        result = probe.tool_identity(str(path))
        self.assertEqual(result['invoked_path'], str(path))
        self.assertEqual(result['resolved_path'], str(Path(sys.executable).resolve()))


if __name__ == '__main__': unittest.main()
