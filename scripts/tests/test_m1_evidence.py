import contextlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import m1_evidence as evidence
import inspect_puredarwin_compiler_rt as archive
from test_inspect_puredarwin_compiler_rt import ar_member, macho


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def test_duplicate_keys_and_nonfinite_json_rejected(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}', b'[]', b'{} ' * 20):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                evidence.load_json(raw, 100)

    def test_regular_file_content_and_size_bound(self):
        path = self.root / 'input'; path.write_bytes(b'owned input')
        self.assertEqual(evidence.read_regular(path, 20), b'owned input')
        with self.assertRaisesRegex(evidence.EvidenceError, 'bounded'):
            evidence.read_regular(path, 4)

    def test_read_refuses_empty_directory_fifo_and_symlink(self):
        normal = self.root / 'normal'; normal.write_bytes(b'example')
        for name in ('empty', 'dir', 'fifo', 'symlink'):
            path = self.root / name
            if name == 'empty': path.touch()
            elif name == 'dir': path.mkdir()
            elif name == 'fifo': os.mkfifo(path)
            else: path.symlink_to(normal)
            with self.subTest(name=name), self.assertRaises((OSError, evidence.EvidenceError)):
                evidence.read_regular(path, 100)

    def test_same_bytes_path_replacement_detected(self):
        path = self.root / 'input'; path.write_bytes(b'original')
        original = evidence.os.fstat; calls = 0
        def replace(fd):
            nonlocal calls
            calls += 1
            result = original(fd)
            if calls == 2:
                path.rename(self.root / 'retained-original')
                path.write_bytes(b'original')
            return result
        with mock.patch.object(evidence.os, 'fstat', side_effect=replace), self.assertRaisesRegex(evidence.EvidenceError, 'replaced'):
            evidence.read_regular(path, 100)

    def test_atomic_publish_retains_partial_and_refuses_overwrite(self):
        out = self.root / 'result.json'; data = b'{"status":"pass"}\n'
        evidence.publish(out, data)
        self.assertEqual(out.read_bytes(), data)
        self.assertEqual((self.root / 'result.json.partial').read_bytes(), data)
        self.assertEqual(out.stat().st_mode & 0o777, 0o600)
        with self.assertRaises(evidence.EvidenceError): evidence.publish(out, b'new')
        self.assertEqual(out.read_bytes(), data)

    def test_file_and_directory_sync_failure_cannot_publish_success(self):
        for failure_call in (1, 2):
            path = self.root / ('result' + str(failure_call)); calls = 0
            original = evidence.os.fsync
            def fail(fd):
                nonlocal calls
                calls += 1
                if calls == failure_call: raise OSError('sync failure')
                return original(fd)
            with mock.patch.object(evidence.os, 'fsync', side_effect=fail), self.assertRaises(OSError):
                evidence.publish(path, b'owned')
            self.assertFalse(path.exists())
            self.assertTrue(path.with_name(path.name + '.partial').exists())

    def test_publication_collision_preserves_winner(self):
        path = self.root / 'result'; original = evidence.os.link
        def race(src, dst, **kw):
            Path(dst).write_bytes(b'prior winner')
            return original(src, dst, **kw)
        with mock.patch.object(evidence.os, 'link', side_effect=race), self.assertRaises(FileExistsError):
            evidence.publish(path, b'new data')
        self.assertEqual(path.read_bytes(), b'prior winner')

    def test_existing_partial_and_symlink_output_not_overwritten(self):
        out = self.root / 'result'; part = self.root / 'result.partial'; part.write_bytes(b'retained')
        with self.assertRaises(FileExistsError): evidence.publish(out, b'new')
        self.assertEqual(part.read_bytes(), b'retained')
        link = self.root / 'link'; link.symlink_to(out)
        with self.assertRaises(evidence.EvidenceError): evidence.publish(link, b'new')
        self.assertTrue(link.is_symlink())

    def test_missing_or_symlink_parent_rejected(self):
        missing = self.root / 'missing' / 'result'
        with self.assertRaises(evidence.EvidenceError): evidence.publish(missing, b'new')
        link = self.root / 'linked-parent'; link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(evidence.EvidenceError): evidence.publish(link / 'result', b'new')


class ArchiveReceiptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.path = self.root / 'libcompiler_rt.a'
        self.path.write_bytes(archive.AR_MAGIC + ar_member('probe.o', macho()))
        self.expect = self.root / 'expect.json'
        self.contract = {'schema_version': 1, 'scope': 'm1_compiler_rt_expectations',
            'source_revision': archive.SOURCE_REVISION, 'target_triple': 'x86_64-apple-darwin20.4',
            'archive_sha256': archive.digest(self.path.read_bytes()), 'member_names': ['probe.o']}
        self.expect.write_bytes(evidence.encoded(self.contract))
        self.output = self.root / 'result'

    def args(self):
        return ['--archive', str(self.path), '--expectations', str(self.expect),
                '--expectations-sha256', evidence.digest(self.expect.read_bytes()), '--output-dir', str(self.output)]

    def run_cli(self):
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            rc = archive.main(self.args())
        return rc, json.loads(stream.getvalue())

    def test_receipt_v2_reports_symbol_inventory_honestly(self):
        rc, report = self.run_cli()
        self.assertEqual(rc, 0); self.assertEqual(report['schema_version'], 2)
        self.assertNotEqual(report['symbol_inventory'], 'not_performed')
        self.assertFalse(report['members'][0]['symbol_table_present'])
        self.assertFalse(report['build_provenance_verified'])
        self.assertEqual(json.loads((self.output / 'result.json').read_text()), report)

    def test_output_failure_never_emits_pass(self):
        with mock.patch.object(archive, 'publish', side_effect=OSError('injected disk failure')):
            rc, report = self.run_cli()
        self.assertEqual(rc, 1); self.assertEqual(report['status'], 'fail')
        self.assertFalse((self.output / 'result.json').exists())

    def test_fsync_failure_never_publishes_pass(self):
        with mock.patch.object(evidence.os, 'fsync', side_effect=OSError('disk full')):
            rc, report = self.run_cli()
        self.assertEqual(rc, 1); self.assertEqual(report['status'], 'fail')
        self.assertFalse((self.output / 'result.json').exists())

    def test_boolean_schema_and_nonobject_expectations_fail_cleanly(self):
        for contract in ({**self.contract, 'schema_version': True}, []):
            self.output = self.root / ('case' + str(len(list(self.root.iterdir()))))
            self.expect.write_bytes((json.dumps(contract)+'\n').encode())
            rc, report = self.run_cli()
            self.assertEqual(rc, 1); self.assertEqual(report['status'], 'fail')

    def test_archive_changed_during_inspection_fails(self):
        original = archive.inspect_archive
        def mutate(*args):
            result = original(*args); self.path.write_bytes(self.path.read_bytes() + b'changed'); return result
        with mock.patch.object(archive, 'inspect_archive', side_effect=mutate):
            rc, report = self.run_cli()
        self.assertEqual(rc, 1); self.assertIn('changed during inspection', report['error'])

    def test_existing_output_preserved(self):
        self.output.mkdir(); marker = self.output / 'result.json'; marker.write_bytes(b'preserve')
        rc, report = self.run_cli()
        self.assertEqual(rc, 1); self.assertEqual(marker.read_bytes(), b'preserve')

    def test_wrong_archive_hash_has_retained_failure(self):
        self.path.write_bytes(self.path.read_bytes() + b'bad')
        rc, report = self.run_cli()
        self.assertEqual(rc, 1); self.assertIn('digest differs', report['error'])
        self.assertEqual(json.loads((self.output / 'failure.json').read_text())['status'], 'fail')


if __name__ == '__main__': unittest.main()
