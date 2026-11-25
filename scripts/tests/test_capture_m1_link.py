"""Owned child-process tests, not selected-library qualification."""
import contextlib
import copy
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import capture_m1_link as capture
import inspect_puredarwin_compiler_rt as archive
from m1_evidence import digest, encoded
from test_inspect_puredarwin_compiler_rt import ar_member, macho


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.out = self.root / 'attempt'
        self.ar = self.root / 'libcompiler_rt.a'
        self.ar.write_bytes(archive.AR_MAGIC + ar_member('probe.o', macho()))
        self.obj = self.root / 'main.o'; self.obj.write_bytes(macho())
        self.tool = self.root / 'owned-ld'
        self.tool.write_text('#!' + sys.executable + '\n' + '''
import pathlib, sys
args = sys.argv[1:]
output = pathlib.Path(args[args.index('-o') + 1])
mapfile = pathlib.Path(args[args.index('-map') + 1])
ar = next(x for x in args if x.endswith('/libcompiler_rt.a'))
output.write_bytes(b'owned simulated output, not a qualified Mach-O dylib')
mapfile.write_text(f'# Path: {output}\\n# Arch: x86_64\\n# Object files:\\n'
                  f'[  0] linker synthesized\\n[  1] {ar}(probe.o)\\n'
                  '# Sections:\\n0x1000 0x20 __TEXT __text\\n# Symbols:\\n'
                  '0x1000 0x10 [  1] _owned_probe\\n')
print('owned linker invoked')
''')
        self.tool.chmod(0o700)
        exp = encoded({'schema_version': 1, 'scope': 'm1_compiler_rt_expectations',
            'source_revision': archive.SOURCE_REVISION, 'target_triple': 'x86_64-apple-darwin20.4',
            'archive_sha256': digest(self.ar.read_bytes()), 'member_names': ['probe.o']})
        self.ir = self.root / 'inspection.json'
        self.ir.write_bytes(encoded(archive.inspect_archive(self.ar.read_bytes(), exp, digest(exp))))
        self.request = self.root / 'request.json'
        self.req = {'schema_version': 1, 'scope': 'm1_direct_link_request',
            'source_revision': archive.SOURCE_REVISION, 'input_origin': 'owned_fixture',
            'argv': [str(self.tool), '-dylib', '-arch', 'x86_64', '-platform_version', 'macos', '26.5', '26.5',
                     str(self.obj), str(self.ar), '-o', str(self.out / 'linked.dylib'),
                     '-map', str(self.out / 'link.map')],
            'cwd': str(self.root), 'linker_sha256': digest(self.tool.read_bytes()),
            'archive_path': str(self.ar),
            'archive_inspection': {'path': str(self.ir), 'sha256': digest(self.ir.read_bytes())},
            'inputs': {str(p): digest(p.read_bytes()) for p in (self.obj, self.ar)}, 'timeout_seconds': 2}

    def execute(self):
        self.request.write_bytes(encoded(self.req))
        return capture.run(self.request, digest(self.request.read_bytes()), self.out)

    def replace_tool(self, source):
        self.tool.write_text('#!' + sys.executable + '\n' + source)
        self.req['linker_sha256'] = digest(self.tool.read_bytes())

    def failed(self, result, message):
        self.assertEqual(result['status'], 'fail')
        self.assertRegex(result['error'], message)
        self.assertFalse((self.out / 'link-verification.json').exists())

    def test_owned_actual_process_captures_inputs_exit_map_and_receipts(self):
        result = self.execute()
        self.assertEqual(result['status'], 'observed_link_and_selection_checked')
        self.assertEqual(result['selected_members'], ['probe.o'])
        self.assertEqual(result['phase']['exit_code'], 0)
        self.assertFalse(result['capture_execution_authenticated'])
        self.assertFalse(result['selected_libsystem_qualified'])
        self.assertEqual(json.loads((self.out / 'result.json').read_text()), result)
        for path, rec in result['inputs'].items():
            self.assertEqual((self.out / rec['snapshot']).read_bytes(), Path(path).read_bytes())
        self.assertIn('owned linker', (self.out / 'link.stdout.log').read_text())

    def test_missing_manifest_input_blocks_before_child(self):
        self.req['inputs'].pop(str(self.obj))
        self.failed(self.execute(), 'every explicit')
        self.assertFalse(self.out.exists())

    def test_extra_manifest_input_is_not_ignored(self):
        self.req['inputs'][str(self.tool)] = digest(self.tool.read_bytes())
        self.failed(self.execute(), 'every explicit')

    def test_changed_input_hash_blocks_before_child(self):
        self.obj.write_bytes(b'changed object')
        result = self.execute()
        self.failed(result, 'input digest mismatch')
        self.assertEqual(result['phase'], {})

    def test_changed_tool_hash_blocks_before_child(self):
        self.tool.write_text(self.tool.read_text() + '# changed\n')
        result = self.execute()
        self.failed(result, 'linker digest')
        self.assertEqual(result['phase'], {})

    def test_response_file_or_implicit_library_input_is_rejected(self):
        for extra in (['@hidden'], ['-lSystem'], ['-L', '/tmp'], ['-syslibroot', '/tmp']):
            with self.subTest(extra=extra):
                self.req['argv'] += extra
                self.failed(self.execute(), 'response|implicit')
                del self.req['argv'][-len(extra):]

    def test_output_cannot_escape_new_evidence_directory(self):
        idx = self.req['argv'].index('-o') + 1
        self.req['argv'][idx] = str(self.root / 'prior.dylib')
        self.failed(self.execute(), 'o path')

    def test_symlink_or_fifo_input_cannot_block(self):
        self.obj.unlink(); self.obj.symlink_to(self.ar)
        self.failed(self.execute(), 'symlink')
        self.obj.unlink(); os.mkfifo(self.obj)
        self.failed(self.execute(), 'regular file')

    def test_failed_archive_receipt_blocks_before_child(self):
        ir = json.loads(self.ir.read_text()); ir['status'] = 'fail'
        self.ir.write_bytes(encoded(ir)); self.req['archive_inspection']['sha256'] = digest(self.ir.read_bytes())
        result = self.execute()
        self.failed(result, 'receipt identity')
        self.assertEqual(result['phase'], {})

    def test_boolean_archive_count_is_not_a_valid_inventory(self):
        ir = json.loads(self.ir.read_text()); ir['member_count'] = True
        self.ir.write_bytes(encoded(ir)); self.req['archive_inspection']['sha256'] = digest(self.ir.read_bytes())
        self.failed(self.execute(), 'inventory')

    def test_actual_nonzero_exit_keeps_logs_without_success_capture(self):
        self.replace_tool('import sys; print("link failed", file=sys.stderr); sys.exit(7)\n')
        result = self.execute()
        self.failed(result, 'exit 7')
        self.assertEqual(result['phase']['exit_code'], 7)
        self.assertIn('link failed', (self.out / 'link.stderr.log').read_text())
        self.assertFalse((self.out / 'link-capture.json').exists())

    def test_actual_signal_is_not_success(self):
        self.replace_tool('import os, signal; os.kill(os.getpid(), signal.SIGTERM)\n')
        result = self.execute()
        self.failed(result, 'exit -15')
        self.assertEqual(result['phase']['exit_code'], -signal.SIGTERM)

    def test_timeout_kills_owned_group_and_retains_failure(self):
        self.replace_tool('import time; print("started", flush=True); time.sleep(10)\n')
        self.req['timeout_seconds'] = 1
        result = self.execute()
        self.failed(result, 'deadline')
        self.assertTrue(result['phase']['timed_out'])
        self.assertIn('started', (self.out / 'link.stdout.log').read_text())

    def test_no_unbounded_log_file_after_overflow(self):
        self.replace_tool('import os; os.write(1, b"x" * 32768)\n')
        with mock.patch.object(capture, 'MAX_LOG', 1024):
            result = self.execute()
        self.failed(result, 'log bound')
        self.assertTrue(result['phase']['logs_truncated'])
        self.assertEqual((self.out / 'link.stdout.log').stat().st_size, 1024)

    def test_input_mutation_by_owned_child_prevents_capture(self):
        self.tool.write_text(self.tool.read_text() + f'pathlib.Path({str(self.obj)!r}).write_bytes(b"changed")\n')
        self.req['linker_sha256'] = digest(self.tool.read_bytes())
        result = self.execute()
        self.failed(result, 'input changed')
        self.assertEqual(result['phase']['exit_code'], 0)
        self.assertFalse((self.out / 'link-capture.json').exists())

    def test_zero_exit_without_output_is_not_success(self):
        self.replace_tool('pass\n')
        result = self.execute()
        self.failed(result, 'No such file')
        self.assertEqual(result['phase']['exit_code'], 0)

    def test_symlink_map_is_rejected_after_zero_exit(self):
        self.tool.write_text(self.tool.read_text() + 'mapfile.unlink(); mapfile.symlink_to(output)\n')
        self.req['linker_sha256'] = digest(self.tool.read_bytes())
        self.failed(self.execute(), 'symbolic links|symlink')

    def test_existing_directory_and_evidence_are_preserved(self):
        self.out.mkdir(); (self.out / 'result.json').write_bytes(b'keep')
        self.failed(self.execute(), 'must be new')
        self.assertEqual((self.out / 'result.json').read_bytes(), b'keep')

    def test_boolean_timeout_and_schema_are_rejected(self):
        for field in ('schema_version', 'timeout_seconds'):
            original = self.req[field]; self.req[field] = True
            self.failed(self.execute(), 'invalid request|timeout')
            self.req[field] = original

    def test_duplicate_request_keys_rejected(self):
        raw = encoded(self.req).replace(b'"schema_version": 1', b'"schema_version": 1, "schema_version": 1')
        self.request.write_bytes(raw)
        self.failed(capture.run(self.request, digest(raw), self.out), 'duplicate JSON key')

    def test_unrelated_force_load_and_symbol_list_need_exact_input_hashes(self):
        extra = self.root / 'libother.a'; extra.write_bytes(self.ar.read_bytes())
        symbols = self.root / 'exports'; symbols.write_bytes(b'_owned_probe\n')
        self.req['argv'] += ['-force_load', str(extra), '-exported_symbols_list', str(symbols)]
        self.req['inputs'].update({str(p): digest(p.read_bytes()) for p in (extra, symbols)})
        self.assertEqual(self.execute()['status'], 'observed_link_and_selection_checked')

    def test_publish_failure_never_reports_success(self):
        original = capture.publish
        def fail_terminal(path, raw):
            if path.name == 'result.json':
                raise OSError('owned terminal write failure')
            return original(path, raw)
        with mock.patch.object(capture, 'publish', side_effect=fail_terminal):
            result = self.execute()
        self.assertEqual(result['status'], 'fail')
        self.assertIn('terminal evidence retention failed', result['error'])
        self.assertFalse((self.out / 'result.json').exists())

    def test_manifest_size_bounds_fail_before_link(self):
        with mock.patch.object(capture, 'MAX_TOTAL', 1):
            result = self.execute()
        self.failed(result, 'total explicit input size')
        self.assertEqual(result['phase'], {})

    def test_child_limits_preserve_stricter_inherited_bounds(self):
        with mock.patch.object(capture.resource, 'getrlimit', return_value=(10, 20)), \
             mock.patch.object(capture.resource, 'setrlimit') as setting:
            capture.child_limits()
        self.assertEqual(setting.call_args_list[0].args[1], (0, 0))
        self.assertEqual(setting.call_args_list[1].args[1], (10, 10))
        self.assertEqual(setting.call_args_list[2].args[1], (10, 10))

    def test_parent_sigterm_reaps_owned_child_and_retains_failure(self):
        self.replace_tool('import pathlib, time; pathlib.Path("started").write_text("yes"); time.sleep(10)\n')
        self.request.write_bytes(encoded(self.req))
        command = [sys.executable, str(Path(capture.__file__)), '--request', str(self.request),
                   '--request-sha256', digest(self.request.read_bytes()), '--output-dir', str(self.out)]
        with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
            import time
            deadline = time.monotonic() + 5
            while not (self.root / 'started').exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue((self.root / 'started').exists())
            process.send_signal(signal.SIGTERM)
            stdout, stderr = process.communicate(timeout=5)
            self.assertEqual(process.returncode, 1, stderr)
        result = json.loads(stdout)
        self.failed(result, 'signal 15')
        self.assertEqual(result['phase']['exit_code'], -signal.SIGKILL)
        self.assertEqual(json.loads((self.out / 'result.json').read_text()), result)

    def test_wrong_request_digest_does_not_create_output(self):
        self.request.write_bytes(encoded(self.req))
        self.failed(capture.run(self.request, '0' * 64, self.out), 'request digest')
        self.assertFalse(self.out.exists())


if __name__ == '__main__':
    unittest.main()
