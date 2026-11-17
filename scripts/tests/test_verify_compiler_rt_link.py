"""Owned map/capture fixtures; these do not claim an actual selected link."""
import contextlib
import copy
import io
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import inspect_puredarwin_compiler_rt as archive
import verify_compiler_rt_link as check
from test_inspect_puredarwin_compiler_rt import ar_member, macho


class LinkTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.arpath = '/selected/store/libcompiler_rt.a'
        self.mappath = '/build/out.map'
        self.outpath = '/build/libowned.dylib'
        self.archive = archive.AR_MAGIC + ar_member('probe.o', macho())
        self.output = b'owned synthetic output bytes; not a qualified dylib'
        self.map = self.map_bytes()
        self.capture = {
            'schema_version': 2, 'scope': 'm1_compiler_rt_link_capture',
            'source_revision': check.SOURCE_REVISION, 'input_origin': 'owned_fixture',
            'argv_kind': 'ld64_direct', 'cwd': '/build', 'exit_code': 0, 'timed_out': False,
            'argv': ['/tools/ld', '-arch', 'x86_64', '-platform_version', 'macos', '26.5', '26.5',
                     '-dylib', 'main.o', self.arpath, '-map', 'out.map', '-o', 'libowned.dylib'],
        }

    def map_bytes(self, *, arpath=None, member='probe.o', path=None, arch='x86_64', extra=''):
        return (f'# Path: {path or self.outpath}\n# Arch: {arch}\n# Object files:\n'
                f'[  0] linker synthesized\n[  1] /build/main.o\n'
                f'[  2] {arpath or self.arpath}({member})\n{extra}'
                '# Sections:\n# Address Size Segment Section\n'
                '0x1000 0x20 __TEXT __text\n# Symbols:\n# Address Size File Name\n'
                '0x1000 0x10 [  1] _main\n0x1010 0x10 [  2] _owned_probe\n').encode()

    def inputs(self):
        exp = archive.encoded({'schema_version': 1, 'scope': 'm1_compiler_rt_expectations',
             'source_revision': archive.SOURCE_REVISION, 'target_triple': 'x86_64-apple-darwin20.4',
             'archive_sha256': archive.digest(self.archive),
             'member_names': [m['name'] for m in archive.parse_ar(self.archive)]})
        inspection = archive.inspect_archive(self.archive, exp, archive.digest(exp))
        cap = copy.deepcopy(self.capture)
        for key, path, data in [('archive', self.arpath, self.archive), ('link_map', self.mappath, self.map),
                                ('output', self.outpath, self.output)]:
            cap[key] = {'path': path, 'sha256': archive.digest(data)}
        return {'capture': archive.encoded(cap), 'map': self.map, 'archive': self.archive,
                'inspection': archive.encoded(inspection), 'output': self.output}

    def runcheck(self, raw=None):
        raw = self.inputs() if raw is None else raw
        hashes = {k: archive.digest(v) for k, v in raw.items()}
        return check.verify(raw['capture'], raw['map'], raw['archive'], raw['inspection'], raw['output'], hashes)

    def reject(self, pattern):
        with self.assertRaisesRegex(check.LinkEvidenceError, pattern):
            self.runcheck()

    def test_valid_ordinary_archive_selection(self):
        result = self.runcheck()
        self.assertEqual(result['selected_members'], ['probe.o'])
        self.assertTrue(result['archive_inventory_reparsed'])
        self.assertFalse(result['build_provenance_verified'])
        self.assertFalse(result['capture_execution_authenticated'])

    def test_wrong_archive_path_with_same_basename_rejected(self):
        self.map = self.map_bytes(arpath='/other/store/libcompiler_rt.a')
        self.reject('different compiler-rt archive path')

    def test_relative_map_archive_resolved_against_capture_cwd(self):
        self.map = self.map_bytes(arpath='../selected/store/libcompiler_rt.a')
        self.assertEqual(self.runcheck()['selected_members'], ['probe.o'])

    def test_wrong_map_output_and_architecture_rejected(self):
        for changes in ({'path': '/build/other.dylib'}, {'arch': 'arm64'}):
            with self.subTest(changes=changes):
                self.map = self.map_bytes(**changes)
                self.reject('identity|architecture')

    def test_captured_map_argument_must_name_actual_map(self):
        self.capture['argv'][self.capture['argv'].index('-map') + 1] = 'other.map'
        self.reject('map path')

    def test_captured_output_argument_must_name_actual_output(self):
        self.capture['argv'][-1] = 'other.dylib'
        self.reject('o path')

    def test_duplicate_map_or_output_flags_rejected(self):
        for flag, value in [('-map', self.mappath), ('-o', self.outpath)]:
            with self.subTest(flag=flag):
                original = self.capture['argv'][:]
                self.capture['argv'] += [flag, value]
                self.reject('duplicated')
                self.capture['argv'] = original

    def test_target_force_load_rejected(self):
        self.capture['argv'] += ['-force_load', self.arpath]
        self.reject('force-loaded')

    def test_unrelated_force_load_is_permitted(self):
        self.capture['argv'] += ['-force_load', '/selected/store/libother.a']
        self.assertEqual(self.runcheck()['other_archives_force_loaded'], 1)

    def test_missing_and_duplicate_ordinary_archive_rejected(self):
        original = self.capture['argv'][:]
        for replacement in ([a for a in original if a != self.arpath], original + [self.arpath]):
            self.capture['argv'] = replacement
            self.reject('exactly once')

    def test_option_operand_cannot_masquerade_as_archive_input(self):
        self.capture['argv'].insert(self.capture['argv'].index(self.arpath), '-L')
        self.reject('exactly once')

    def test_response_and_driver_and_filelist_inputs_rejected(self):
        for args in (['@extra.rsp'], ['-Wl,-force_load,' + self.arpath], ['-Xlinker', '-map'],
                     ['-filelist', '/tmp/list'], ['--whole-archive'], ['-all_load'], ['-ObjC']):
            with self.subTest(args=args):
                original = self.capture['argv'][:]
                self.capture['argv'] += args
                self.reject('unsupported|response|force-load')
                self.capture['argv'] = original

    def test_unresolved_symbol_suppression_rejected(self):
        self.capture['argv'] += ['-undefined', 'dynamic_lookup']
        self.reject('suppression')

    def test_other_archive_same_basename_is_ambiguous(self):
        self.capture['argv'] += ['/other/libcompiler_rt.a']
        self.reject('alternate compiler-rt')

    def test_member_not_in_archive_rejected(self):
        self.map = self.map_bytes(member='not_real.o')
        self.reject('absent from the inspected archive')

    def test_availability_member_rejected(self):
        name = b'os_version_check.c.o'
        payload = name + macho()
        header = b'#1/20'.ljust(16) + b'0'.ljust(12) + b'0'.ljust(6) + b'0'.ljust(6)
        header += b'100644'.ljust(8) + str(len(payload)).encode().ljust(10) + b'`\n'
        # Use actual length rather than a hard-coded fixture size.
        header = ('#1/' + str(len(name))).encode().ljust(16) + header[16:]
        self.archive = archive.AR_MAGIC + header + payload + (b'\n' if len(payload) & 1 else b'')
        self.map = self.map_bytes(member=name.decode())
        self.reject('availability-check')

    def test_map_comment_not_member_selection(self):
        self.map = self.map_bytes().replace((self.arpath + '(probe.o)').encode(), b'/build/unrelated.o')
        self.map += ('# ' + self.arpath + '(probe.o)\n').encode()
        self.reject('no compiler-rt member')

    def test_duplicate_member_and_file_ordinal_rejected(self):
        for row in (f'[  3] {self.arpath}(probe.o)\n', '[  2] /other.o\n'):
            self.map = self.map_bytes(extra=row)
            self.reject('duplicate')

    def test_incomplete_and_reordered_map_rejected(self):
        for raw in (b'libcompiler_rt.a(probe.o)', self.map_bytes().split(b'# Symbols:')[0],
                    self.map_bytes().replace(b'# Sections:', b'# Object files:')):
            self.map = raw
            self.reject('incomplete|before|required|duplicate')

    def test_failed_and_noninteger_execution_records_rejected(self):
        for code in (1, -9, False, 0.0, '0'):
            self.capture['exit_code'] = code
            self.reject('did not succeed')
        self.capture['exit_code'] = 0
        for value in (True, 0, 'false'):
            self.capture['timed_out'] = value
            self.reject('did not succeed')

    def test_captured_digest_cannot_be_rebound_to_another_output(self):
        raw = self.inputs()
        raw['output'] = b'different bytes'
        with self.assertRaisesRegex(check.LinkEvidenceError, 'captured output digest mismatch'):
            self.runcheck(raw)

    def test_archive_inventory_reparsed_not_trusted_from_receipt(self):
        raw = self.inputs()
        receipt = json.loads(raw['inspection'])
        receipt['members'][0]['sha256'] = 'f' * 64
        raw['inspection'] = archive.encoded(receipt)
        with self.assertRaisesRegex(check.LinkEvidenceError, 'freshly parsed'):
            self.runcheck(raw)

    def test_false_qualification_and_boolean_count_rejected(self):
        for key, value in [('build_provenance_verified', True), ('member_count', True),
                           ('schema_version', True), ('schema_version', 1)]:
            raw = self.inputs()
            receipt = json.loads(raw['inspection']); receipt[key] = value
            raw['inspection'] = archive.encoded(receipt)
            with self.subTest(key=key, value=value), self.assertRaises(check.LinkEvidenceError):
                self.runcheck(raw)

    def test_duplicate_json_and_nonobject_capture_rejected(self):
        for raw_capture in (b'{"schema_version":2,"schema_version":2}', b'[]'):
            raw = self.inputs(); raw['capture'] = raw_capture
            with self.assertRaises(check.LinkEvidenceError):
                self.runcheck(raw)

    def test_external_map_digest_mismatch(self):
        raw = self.inputs(); hashes = {k: archive.digest(v) for k, v in raw.items()}; hashes['map'] = '0' * 64
        with self.assertRaisesRegex(check.LinkEvidenceError, 'map evidence digest mismatch'):
            check.verify(raw['capture'], raw['map'], raw['archive'], raw['inspection'], raw['output'], hashes)

    def cli_args(self):
        raw = self.inputs()
        flags = {'capture': ('argv-json', 'argv-sha256'), 'map': ('map', 'map-sha256'),
                 'archive': ('archive', 'archive-sha256'), 'inspection': ('archive-inspection', 'archive-inspection-sha256'),
                 'output': ('linked-output', 'linked-output-sha256')}
        argv = []
        for label, data in raw.items():
            path = self.root / label; path.write_bytes(data)
            pathflag, hashflag = flags[label]
            argv += ['--' + pathflag, str(path), '--' + hashflag, archive.digest(data)]
        return argv + ['--output', str(self.root / 'result.json')]

    def test_cli_retains_success_and_refuses_overwrite(self):
        args = self.cli_args()
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main(args), 0)
        out = self.root / 'result.json'; before = out.read_bytes()
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertEqual(check.main(args), 1)
        self.assertEqual(json.loads(stream.getvalue())['status'], 'fail')
        self.assertEqual(out.read_bytes(), before)

    def test_failed_publication_never_reports_pass(self):
        args = self.cli_args()
        with mock.patch.object(check, 'publish', side_effect=OSError('disk failure')), contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertEqual(check.main(args), 1)
        self.assertEqual(json.loads(stream.getvalue())['status'], 'fail')
        self.assertFalse((self.root / 'result.json').exists())

    def test_map_mutation_after_verification_rejected(self):
        args = self.cli_args(); original = check.verify
        def mutate(*a, **kw):
            result = original(*a, **kw)
            (self.root / 'map').write_bytes(b'changed')
            return result
        with mock.patch.object(check, 'verify', side_effect=mutate), contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertEqual(check.main(args), 1)
        self.assertIn('changed after verification', json.loads(stream.getvalue())['error'])
        self.assertFalse((self.root / 'result.json').exists())


if __name__ == '__main__':
    unittest.main()
