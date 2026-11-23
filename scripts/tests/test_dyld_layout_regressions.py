"""Owned byte regressions for the dyld inspector's mapping gates."""
import contextlib
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
from test_inspect_puredarwin_dyld import fixture, cmd, mod


def commands(blob):
    offset = 32
    for _ in range(struct.unpack_from('<I', blob, 16)[0]):
        kind, size = struct.unpack_from('<II', blob, offset)
        yield kind, offset, size
        offset += size


def mutate_segment(**changes):
    blob = bytearray(fixture())
    offset = next(offset for kind, offset, _ in commands(blob) if kind == 0x19)
    fields = {'vmaddr': 24, 'vmsize': 32, 'fileoff': 40, 'filesize': 48}
    for name, value in changes.items():
        struct.pack_into('<Q', blob, offset + fields[name], value)
    return bytes(blob)


class DyldLayoutTests(unittest.TestCase):
    expected = {'minimum_os': '26.5.0', 'sdk_version': '26.5.0'}

    def test_entry_cannot_point_into_header_or_load_commands(self):
        end = 32 + struct.unpack_from('<I', fixture(), 20)[0]
        for displacement in (0, 1, 31, 32, end - 1):
            with self.subTest(offset=displacement), self.assertRaisesRegex(mod.InspectionError, 'header/load commands'):
                mod.parse(fixture(entry=0x100000000 + displacement), self.expected)

    def test_first_byte_after_commands_remains_supported(self):
        end = 32 + struct.unpack_from('<I', fixture(), 20)[0]
        result = mod.parse(fixture(entry=0x100000000 + end), self.expected)
        self.assertEqual(result['entry_file_offset'], end)

    def test_file_mapping_cannot_exceed_vm_mapping(self):
        with self.assertRaisesRegex(mod.InspectionError, 'larger than VM'):
            mod.parse(mutate_segment(vmsize=0x1800), self.expected)

    def test_last_file_byte_is_supported_but_zero_fill_tail_is_not(self):
        result = mod.parse(fixture(entry=0x100001FFF), self.expected)
        self.assertEqual(result['entry_file_offset'], 0x1FFF)
        blob = bytearray(fixture(entry=0x100002000))
        segment = next(p for kind, p, _ in commands(blob) if kind == 0x19)
        struct.pack_into('<Q', blob, segment + 32, 0x3000)
        with self.assertRaisesRegex(mod.InspectionError, 'not backed by file bytes'):
            mod.parse(bytes(blob), self.expected)

    def test_input_size_limit_applies_to_direct_parser_calls(self):
        from unittest import mock
        with mock.patch.object(mod, 'MAX_INPUT', 64):
            with self.assertRaisesRegex(mod.InspectionError, 'size bound'):
                mod.parse(fixture(), self.expected)

    def test_symbol_table_cannot_alias_load_commands(self):
        blob = bytearray(fixture())
        end = 32 + struct.unpack_from('<I', blob, 20)[0]
        command = struct.pack('<6I', 2, 24, 32, 1, 0x1800, 8)
        blob[end:end+len(command)] = command
        struct.pack_into('<I', blob, 16, struct.unpack_from('<I', blob, 16)[0] + 1)
        struct.pack_into('<I', blob, 20, end - 32 + len(command))
        with self.assertRaisesRegex(mod.InspectionError, 'symbol/string table overlaps'):
            mod.parse(bytes(blob), self.expected)

    def test_duplicate_symbol_tables_are_rejected(self):
        blob = bytearray(fixture())
        end = 32 + struct.unpack_from('<I', blob, 20)[0]
        command = struct.pack('<6I', 2, 24, 0x1800, 1, 0x1900, 8)
        blob[end:end+48] = command * 2
        struct.pack_into('<I', blob, 16, struct.unpack_from('<I', blob, 16)[0] + 2)
        struct.pack_into('<I', blob, 20, end - 32 + 48)
        with self.assertRaisesRegex(mod.InspectionError, 'duplicate singleton'):
            mod.parse(bytes(blob), self.expected)


EXPECTED = {'minimum_os': '26.5.0', 'sdk_version': '26.5.0'}
# Mach-O field positions and byte widths, including the 64-bit module entry.
TABLES = ((6, 8), (8, 56), (10, 4), (12, 4), (14, 8), (16, 8))


def image(*commands):
    data = bytearray(fixture())
    end = 32 + struct.unpack_from('<I', data, 20)[0]
    extra = b''.join(commands)
    assert end + len(extra) < 0x1000
    data[end:end + len(extra)] = extra
    struct.pack_into('<I', data, 16, struct.unpack_from('<I', data, 16)[0] + len(commands))
    struct.pack_into('<I', data, 20, end - 32 + len(extra))
    return bytes(data)


def symtab(offset=0x1800, count=6, strings=0x1900, size=32):
    return cmd(mod.LC_SYMTAB, struct.pack('<4I', offset, count, strings, size))


def dynamic(**changes):
    values = [0, 2, 2, 2, 4, 2] + [0] * 12
    for key, value in changes.items():
        values[int(key.removeprefix('f'))] = value
    return cmd(mod.LC_DYSYMTAB, struct.pack('<18I', *values))


class MetadataIntegrityTests(unittest.TestCase):
    def check(self, data):
        return mod.parse(data, EXPECTED)

    def test_disjoint_tables_and_groups_remain_supported(self):
        values = {}
        offset = 0x1A00
        for field, width in TABLES:
            values['f' + str(field)] = offset
            values['f' + str(field + 1)] = 1
            offset += width + 8
        self.assertEqual(self.check(image(symtab(), dynamic(**values)))['file_type'], 'MH_DYLINKER')

    def test_dynamic_before_original_symbol_command_is_supported(self):
        self.assertEqual(self.check(image(dynamic(), symtab()))['file_type'], 'MH_DYLINKER')

    def test_original_symbol_command_is_required_by_dynamic_command(self):
        with self.assertRaisesRegex(mod.InspectionError, 'requires LC_SYMTAB'):
            self.check(image(dynamic()))

    def test_even_empty_dynamic_command_requires_symbol_command(self):
        with self.assertRaisesRegex(mod.InspectionError, 'requires LC_SYMTAB'):
            self.check(image(cmd(mod.LC_DYSYMTAB, b'\0' * 72)))

    def test_every_dynamic_payload_rejects_header_alias(self):
        for field, _ in TABLES:
            with self.subTest(field=field), self.assertRaisesRegex(mod.InspectionError, 'header/load commands'):
                self.check(image(symtab(), dynamic(**{f'f{field}': 32, f'f{field+1}': 1})))

    def test_every_dynamic_payload_rejects_declared_command_alias(self):
        end = 32 + struct.unpack_from('<I', fixture(), 20)[0] + 24 + 80
        for field, _ in TABLES:
            with self.subTest(field=field), self.assertRaisesRegex(mod.InspectionError, 'header/load commands'):
                self.check(image(symtab(), dynamic(**{f'f{field}': end - 1, f'f{field+1}': 1})))

    def test_every_dynamic_payload_rejects_end_overrun(self):
        for field, width in TABLES:
            with self.subTest(field=field), self.assertRaisesRegex(mod.InspectionError, 'exceeds file'):
                self.check(image(symtab(), dynamic(**{f'f{field}': 0x2000 - width + 1, f'f{field+1}': 1})))

    def test_every_dynamic_payload_accepts_exact_end_boundary(self):
        for field, width in TABLES:
            with self.subTest(field=field):
                self.check(image(symtab(), dynamic(**{f'f{field}': 0x2000 - width, f'f{field+1}': 1})))

    def test_symbol_and_string_ranges_cannot_overlap(self):
        for strings in (0x1800, 0x1801, 0x185F):
            with self.subTest(strings=strings), self.assertRaisesRegex(mod.InspectionError, 'metadata payload ranges overlap'):
                self.check(image(symtab(strings=strings)))

    def test_dynamic_payload_cannot_alias_symbols_or_strings(self):
        for start in (0x1800, 0x1900):
            with self.subTest(offset=start), self.assertRaisesRegex(mod.InspectionError, 'metadata payload ranges overlap'):
                self.check(image(symtab(), dynamic(f12=start, f13=1)))

    def test_dynamic_payloads_cannot_alias_each_other(self):
        with self.assertRaisesRegex(mod.InspectionError, 'metadata payload ranges overlap'):
            self.check(image(symtab(), dynamic(f12=0x1A00, f13=2, f14=0x1A04, f15=1)))

    def test_symbols_cannot_alias_other_linkedit_payloads(self):
        extra = cmd(mod.LC_FUNCTION_STARTS, struct.pack('<2I', 0x1800, 8))
        with self.assertRaisesRegex(mod.InspectionError, 'metadata payload ranges overlap'):
            self.check(image(symtab(), extra))

    def test_dynamic_payload_cannot_alias_other_linkedit_payloads(self):
        extra = cmd(mod.LC_DATA_IN_CODE, struct.pack('<2I', 0x1A00, 8))
        with self.assertRaisesRegex(mod.InspectionError, 'metadata payload ranges overlap'):
            self.check(image(symtab(), dynamic(f12=0x1A00, f13=2), extra))

    def test_each_symbol_group_rejects_index_or_count_overrun(self):
        for field in (0, 2, 4):
            for start, count in ((6, 1), (5, 2), (0xFFFFFFFF, 1), (0, 0xFFFFFFFF)):
                with self.subTest(field=field, start=start, count=count), self.assertRaisesRegex(mod.InspectionError, 'symbol group exceeds'):
                    self.check(image(symtab(), dynamic(**{f'f{field}': start, f'f{field+1}': count})))

    def test_symbol_groups_cannot_overlap(self):
        for changes in ({'f2': 1}, {'f4': 3}, {'f0': 2}):
            with self.subTest(changes=changes), self.assertRaisesRegex(mod.InspectionError, 'symbol groups overlap'):
                self.check(image(symtab(), dynamic(**changes)))

    def test_adjacent_symbol_groups_and_payloads_are_supported(self):
        self.check(image(symtab(strings=0x1860), dynamic(f12=0x1880, f13=2, f14=0x1888, f15=1)))

    def test_empty_groups_do_not_create_false_overlap(self):
        self.check(image(symtab(), dynamic(f0=6, f1=0, f2=6, f3=0, f4=6, f5=0)))

    def test_empty_payload_does_not_overlap_nonempty_range(self):
        self.check(image(symtab(), dynamic(f12=0x1800, f13=0)))

    def test_empty_original_tables_remain_supported(self):
        self.check(image(symtab(offset=0, count=0, strings=0, size=0),
                         dynamic(f0=0, f1=0, f2=0, f3=0, f4=0, f5=0)))

    def test_cli_rejects_bad_metadata_without_publishing_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = image(symtab(), dynamic(f12=32, f13=1))
            source = root / 'owned-dyld'; source.write_bytes(data)
            expectations = dict(schema_version=1, scope='m1_dyld_executable_expectations',
                                source_revision=mod.SOURCE_REVISION, install_name=mod.INSTALL_NAME,
                                **EXPECTED)
            path = root / 'expectations.json'; path.write_text(json.dumps(expectations))
            result = root / 'result.json'
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                status = mod.main(['--dyld', str(source), '--dyld-sha256', mod.sha256(data),
                                   '--expectations', str(path), '--expectations-sha256', mod.sha256(path.read_bytes()),
                                   '--output', str(result)])
            self.assertEqual(status, 1)
            self.assertEqual(json.loads(stdout.getvalue())['status'], 'fail')
            self.assertFalse(result.exists())
            self.assertFalse(result.with_name('result.json.partial').exists())


if __name__ == '__main__':
    unittest.main()
