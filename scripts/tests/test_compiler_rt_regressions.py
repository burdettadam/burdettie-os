"""Archive interoperability and symbol-classification regressions."""
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import inspect_puredarwin_compiler_rt as check
from test_inspect_puredarwin_compiler_rt import ar_member, macho


def raw_member(name, payload):
    h = name.encode().ljust(16) + b'0'.ljust(12) + b'0'.ljust(6) + b'0'.ljust(6)
    h += b'100644'.ljust(8) + str(len(payload)).encode().ljust(10) + b'`\n'
    return h + payload + (b'\n' if len(payload) & 1 else b'')


def symbol_object(symbols):
    """Each input: (name, type, section, value-or-alias-name)."""
    strings = bytearray(b'\0'); entries = []
    for name, typ, section, value in symbols:
        index = len(strings); strings += name.encode() + b'\0'
        if isinstance(value, str):
            target = len(strings); strings += value.encode() + b'\0'; value = target
        entries.append(struct.pack('<IBBHQ', index, typ, section, 0, value))
    entries = b''.join(entries)
    return (struct.pack('<8I', 0xFEEDFACF, check.CPU_X86_64, 3, 1, 1, 24, 0, 0)
            + struct.pack('<6I', 2, 24, 56, len(symbols), 56 + len(entries), len(strings))
            + entries + strings)


class ArchiveRegressions(unittest.TestCase):
    def test_gnu_symbol_index_is_not_rejected_as_an_empty_member(self):
        raw = check.AR_MAGIC + raw_member('/', b'\0' * 4) + ar_member('probe.o', macho())
        self.assertEqual(check.parse_ar(raw)[0]['name'], 'probe.o')

    def test_gnu_long_member_names_and_entry_boundaries(self):
        table = b'first_really_long_name.o/\nsecond_long_name.o/\n'
        raw = check.AR_MAGIC + raw_member('//', table) + raw_member('/0', macho())
        self.assertEqual(check.parse_ar(raw)[0]['name'], 'first_really_long_name.o')
        bad = check.AR_MAGIC + raw_member('//', table) + raw_member('/2', macho())
        with self.assertRaisesRegex(check.InspectionError, 'entry boundary'): check.parse_ar(bad)

    def test_duplicate_gnu_name_table_rejected(self):
        raw = check.AR_MAGIC + raw_member('//', b'first.o/\n') * 2 + ar_member('probe.o', macho())
        with self.assertRaisesRegex(check.InspectionError, 'duplicate GNU'): check.parse_ar(raw)

    def test_duplicate_index_and_header_count_bounds(self):
        raw = check.AR_MAGIC + raw_member('/', b'\0' * 4) * 2 + ar_member('probe.o', macho())
        with self.assertRaisesRegex(check.InspectionError, 'duplicate archive symbol'): check.parse_ar(raw)
        raw = check.AR_MAGIC + raw_member('/', b'\0' * 4) + ar_member('probe.o', macho())
        with mock.patch.object(check, 'MAX_MEMBERS', 1), self.assertRaisesRegex(check.InspectionError, 'header count'):
            check.parse_ar(raw)

    def test_special_prefix_does_not_hide_an_unknown_object(self):
        name = b'__.SYMDEF_unknown'; payload = name + b'not an object'
        raw = check.AR_MAGIC + raw_member('#1/' + str(len(name)), payload) + ar_member('good.o', macho())
        with self.assertRaisesRegex(check.InspectionError, 'unrecognized'): check.parse_ar(raw)

    def test_unsafe_member_paths_bsd_padding_and_special_mode_rejected(self):
        for name in ('../bad.o', '/absolute.o', 'bad\\name.o'):
            payload = name.encode() + macho()
            with self.subTest(name=name), self.assertRaisesRegex(check.InspectionError, 'unsafe'):
                check.parse_ar(check.AR_MAGIC + raw_member('#1/' + str(len(name)), payload))
        with self.assertRaisesRegex(check.InspectionError, 'BSD member-name padding'):
            check.parse_ar(check.AR_MAGIC + raw_member('#1/12', b'name.o\0xxxxx' + macho()))
        with self.assertRaisesRegex(check.InspectionError, 'permission'):
            check.parse_ar(check.AR_MAGIC + ar_member('bad.o', macho(), b'104644'))

    def test_common_indirect_and_undefined_not_concrete_definitions(self):
        obj = symbol_object([('_absolute', 3, 0, 2), ('_unresolved', 1, 0, 0),
                             ('_tentative', 1, 0, 8), ('_alias', 11, 0, '_target')])
        result = check.inspect_member('owned.o', obj, '100644')
        self.assertEqual(result['defined_symbols'], ['_absolute'])
        self.assertEqual(result['undefined_symbols'], ['_unresolved'])
        self.assertEqual(result['common_symbols'], ['_tentative'])
        self.assertEqual(result['indirect_symbols'], [{'name': '_alias', 'target': '_target'}])

    def test_defined_section_ordinal_validated(self):
        obj = symbol_object([('_pretend', 15, 1, 0)])
        with self.assertRaisesRegex(check.InspectionError, 'section ordinal'): check.inspect_member('bad.o', obj, '100644')

    def test_symbol_tables_cannot_overlap_header_or_each_other(self):
        obj = symbol_object([('_absolute', 3, 0, 0)])
        for offset, value in ((40, 0), (48, 56)):
            bad = bytearray(obj); struct.pack_into('<I', bad, offset, value)
            with self.subTest(offset=offset), self.assertRaisesRegex(check.InspectionError, 'overlap'):
                check.inspect_member('bad.o', bytes(bad), '100644')

    def test_renamed_availability_object_detected_by_symbols(self):
        obj = symbol_object([('___isPlatformVersionAtLeast', 3, 0, 0)])
        record = check.inspect_member('renamed.o', obj, '100644')
        self.assertTrue(check.availability_member(record))

    def test_missing_symbol_table_is_not_an_empty_symbol_claim(self):
        result = check.inspect_member('no_table.o', macho(), '100644')
        self.assertFalse(result['symbol_table_present'])

    @unittest.skipUnless(shutil.which('clang') and shutil.which('llvm-ar'), 'clang/llvm-ar unavailable')
    def test_real_darwin_and_gnu_archives_with_common_symbol(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); source = root / 'owned.c'; obj = root / 'long_owned_archive_member.o'
            source.write_text('int tentative; extern int external_func(void); int owned_func(void) { return external_func() + tentative; }\n')
            subprocess.run(['clang', '-target', 'x86_64-apple-macosx26.5', '-ffreestanding', '-fcommon',
                            '-c', str(source), '-o', str(obj)], check=True, capture_output=True, timeout=15)
            for format in ('darwin', 'gnu'):
                path = root / (format + '.a')
                subprocess.run(['llvm-ar', '--format=' + format, 'rcs', str(path), str(obj)],
                               check=True, capture_output=True, timeout=15)
                records = check.parse_ar(path.read_bytes())
                self.assertEqual(len(records), 1)
                self.assertEqual(records[0]['name'], obj.name)
                self.assertIn('_owned_func', records[0]['defined_symbols'])
                self.assertIn('_external_func', records[0]['undefined_symbols'])
                self.assertIn('_tentative', records[0]['common_symbols'])
                self.assertNotIn('_tentative', records[0]['defined_symbols'])


if __name__ == '__main__': unittest.main()
