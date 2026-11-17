"""Owned byte regressions for the dyld inspector's mapping gates."""
import struct
import unittest
from test_inspect_puredarwin_dyld import fixture, mod


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


if __name__ == '__main__':
    unittest.main()
