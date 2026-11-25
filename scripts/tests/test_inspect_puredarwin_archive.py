"""Synthetic archive cases; actual selected-source outputs remain a separate gate."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import inspect_puredarwin_archive as subject


SYMBOL = "___burdettie_builtin"
MINIMUM = (26 << 16) | (5 << 8)


def object_bytes(*, cpu=0x01000007, subtype=3, kind=1, minimum=MINIMUM,
                 platform=1, legacy=False, typ=15, sect=1, value=0, name=SYMBOL):
    deployment = (b"" if minimum is None else
                  struct.pack("<4I", 0x24, 16, minimum, 0) if legacy else
                  struct.pack("<6I", 0x32, 24, platform, minimum, 0, 0))
    ncmds, cmdbytes = 2 + bool(deployment), 152 + 24 + len(deployment)
    fileoff = 32 + cmdbytes
    symoff = (fileoff + 1 + 7) & ~7
    strings = b"\0" + name.encode() + b"\0"
    header = struct.pack("<8I", 0xFEEDFACF, cpu, subtype, kind, ncmds, cmdbytes, 0, 0)
    segment = struct.pack("<II16sQQQQ4I", 0x19, 152, b"", 0, 1, fileoff, 1, 7, 7, 1, 0)
    section = struct.pack("<16s16sQQ8I", b"__text", b"__TEXT", 0, 1, fileoff, 0, 0, 0, 0, 0, 0, 0)
    symtab = struct.pack("<6I", 2, 24, symoff, 1, symoff + 16, len(strings))
    commands = segment + section + deployment + symtab
    return header + commands + b"\xc3" + b"\0" * (symoff - fileoff - 1) + struct.pack("<IBBHQ", 1, typ, sect, 0, value) + strings


def member(name, data, *, bsd=False):
    encoded = name.encode()
    if bsd:
        encoded += b"\0" * (-len(encoded) % 8)
        field, body = ("#1/" + str(len(encoded))).encode(), encoded + data
    else:
        field, body = encoded, data
    if len(field) > 16:
        raise ValueError("Use an extended test name")
    header = (field.ljust(16) + b"0".ljust(12) + b"0".ljust(6) + b"0".ljust(6)
              + b"100644".ljust(8) + str(len(body)).encode().ljust(10) + b"`\n")
    return header + body + (b"\n" if len(body) % 2 else b"")


def archive(*entries):
    return b"!<arch>\n" + b"".join(entries or (member("probe.o/", object_bytes()),))


def inspect(raw=None):
    return subject.inspect_archive(archive() if raw is None else raw, [SYMBOL], "26.5.0")


class ArchiveTests(unittest.TestCase):
    def test_regular_archive_passes_without_execution_claim(self):
        result = inspect()
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["objects"][0]["deployment"]["sdk"], "0.0.0")
        self.assertEqual(result["objects"][0]["symbol_count"], 1)
        self.assertFalse(result["selected_compiler_rt_qualified"])
        self.assertFalse(result["archive_index_semantics_checked"])
        self.assertEqual(result["guest_execution"], "not_run")

    def test_extended_bsd_name_and_opaque_index(self):
        raw = archive(member("__.SYMDEF SORTED", b"index-not-a-link-test", bsd=True),
                      member("long_builtin_name.c.o", object_bytes(), bsd=True))
        result = inspect(raw)
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["objects"][0]["name"], "long_builtin_name.c.o")
        self.assertEqual(len(result["archive_metadata"]), 1)

    def test_gnu_long_name(self):
        raw = archive(member("//", b"long_builtin_name.c.o/\n"), member("/0", object_bytes()))
        self.assertEqual(inspect(raw)["objects"][0]["name"], "long_builtin_name.c.o")

    def test_duplicate_names_inspected_by_offset(self):
        result = inspect(archive(member("a.o/", object_bytes()), member("a.o/", object_bytes())))
        self.assertEqual(result["object_count"], 2)
        self.assertNotEqual(*(item["archive_offset"] for item in result["objects"]))

    def test_wrong_arch_in_second_duplicate_is_not_missed(self):
        with self.assertRaisesRegex(subject.InspectionError, "x86_64"):
            inspect(archive(member("a.o/", object_bytes()), member("a.o/", object_bytes(cpu=0x0100000C))))

    def test_thin_fat_elf_bitcode_and_wrong_object_kind_rejected(self):
        for raw in (b"!<thin>\n", b"\xca\xfe\xba\xbe", b"", archive(member("a.o/", b"\x7fELF")),
                    archive(member("a.o/", b"BC\xc0\xde")), archive(member("a.o/", object_bytes(kind=2))),
                    archive(member("a.o/", object_bytes(subtype=8)))):
            with self.subTest(prefix=raw[:8]):
                with self.assertRaises((subject.InspectionError, struct.error)):
                    inspect(raw)

    def test_empty_archive_and_metadata_only_rejected(self):
        for raw in (b"!<arch>\n", archive(member("/", b"index"))):
            with self.assertRaisesRegex(subject.InspectionError, "no Mach-O objects"):
                inspect(raw)

    def test_truncation_rejected_at_every_boundary(self):
        raw = archive()
        for length in range(1, len(raw)):
            with self.subTest(length=length):
                with self.assertRaises((subject.InspectionError, struct.error)):
                    inspect(raw[:length])

    def test_invalid_header_size_and_terminator(self):
        for start, replacement in ((56, b"-1        "), (66, b"??")):
            raw = bytearray(archive())
            raw[start:start + len(replacement)] = replacement
            with self.assertRaises(subject.InspectionError):
                inspect(raw)

    def test_odd_padding_checked(self):
        raw = archive(member("/", b"x"), member("a.o/", object_bytes()))
        broken = bytearray(raw)
        broken[69] = 0
        with self.assertRaisesRegex(subject.InspectionError, "padding"):
            inspect(broken)

    def test_invalid_long_name_reference(self):
        for first, second in ((b"long_name.o/\n", "/3"), (b"long_name.o", "/0"),
                              (b"long_name.o/\n", "/99")):
            with self.subTest(second=second):
                with self.assertRaises(subject.InspectionError):
                    inspect(archive(member("//", first), member(second, object_bytes())))
        with self.assertRaises(subject.InspectionError):
            inspect(archive(member("/0", object_bytes())))

    def test_duplicate_name_table_and_path_name_rejected(self):
        for raw in (archive(member("//", b"a/\n"), member("//", b"b/\n")),
                    archive(member("../probe.o", object_bytes(), bsd=True)),
                    archive(member("a\x00b", object_bytes(), bsd=True))):
            with self.assertRaises(subject.InspectionError):
                inspect(raw)

    def test_oversized_extended_name_rejected(self):
        with self.assertRaises(subject.InspectionError):
            inspect(archive(member("#1/999999", object_bytes())))

    def test_size_member_count_and_symbol_bounds(self):
        with mock.patch.object(subject, "MAX_ARCHIVE", 8):
            with self.assertRaises(subject.InspectionError):
                inspect()
        with mock.patch.object(subject, "MAX_MEMBER", 16):
            with self.assertRaises(subject.InspectionError):
                inspect()
        with mock.patch.object(subject, "MAX_MEMBERS", 1):
            with self.assertRaises(subject.InspectionError):
                inspect(archive(member("a.o/", object_bytes()), member("b.o/", object_bytes())))
        with mock.patch.object(subject, "MAX_SYMBOLS", 1):
            with self.assertRaisesRegex(subject.InspectionError, "archive-wide"):
                inspect(archive(member("a.o/", object_bytes()), member("b.o/", object_bytes())))

    def test_wrong_and_missing_deployment_are_distinct(self):
        self.assertEqual(inspect(archive(member("a.o/", object_bytes(minimum=11 << 16))))["status"], "fail")
        result = inspect(archive(member("a.o/", object_bytes(minimum=None))))
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["members_without_version"], [8])

    def test_legacy_version_supported(self):
        result = inspect(archive(member("a.o/", object_bytes(legacy=True))))
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["objects"][0]["deployment"]["command"], "LC_VERSION_MIN_MACOSX")

    def test_foreign_platform_rejected(self):
        with self.assertRaisesRegex(subject.InspectionError, "macOS"):
            inspect(archive(member("a.o/", object_bytes(platform=2))))

    def test_symbol_definition_not_name_presence(self):
        for typ, sect, value in ((1, 0, 0), (1, 0, 4), (0x1F, 1, 0), (0xE, 1, 0),
                                  (0xEF, 1, 0), (3, 0, 0)):
            with self.subTest(typ=typ, value=value):
                result = inspect(archive(member("a.o/", object_bytes(typ=typ, sect=sect, value=value))))
                self.assertEqual(result["status"], "fail")
                self.assertEqual(result["missing_symbols"], [SYMBOL])

    def test_symbol_names_are_exact(self):
        result = inspect(archive(member("a.o/", object_bytes(name=SYMBOL + "_suffix"))))
        self.assertEqual(result["missing_symbols"], [SYMBOL])

    def test_invalid_symbol_section_and_value(self):
        for options in ({"sect": 0}, {"sect": 2}, {"value": 2}):
            with self.assertRaises(subject.InspectionError):
                inspect(archive(member("a.o/", object_bytes(**options))))

    def test_malformed_load_commands(self):
        for offset, value in ((16, 99999), (20, 0xFFFFFFFF), (36, 4), (96, 1000)):
            raw = bytearray(object_bytes())
            struct.pack_into("<I", raw, offset, value)
            with self.assertRaises(subject.InspectionError):
                inspect(archive(member("a.o/", raw)))

    def test_invalid_string_and_symbol_ranges(self):
        for field, value in ((0, 0), (1, 0xFFFFFFFF), (2, 0xFFFFFFFF), (3, 0xFFFFFFFF)):
            raw = bytearray(object_bytes())
            struct.pack_into("<I", raw, 32 + 152 + 24 + 8 + field * 4, value)
            with self.assertRaises(subject.InspectionError):
                inspect(archive(member("a.o/", raw)))
        raw = object_bytes()[:-1] + b"x"
        with self.assertRaisesRegex(subject.InspectionError, "Unterminated"):
            inspect(archive(member("a.o/", raw)))

    def test_duplicate_versions_and_symtabs(self):
        for command in (struct.pack("<6I", 0x32, 24, 1, MINIMUM, 0, 0),
                        struct.pack("<6I", 2, 24, 0, 0, 0, 0)):
            raw = struct.pack("<8I", 0xFEEDFACF, 0x01000007, 3, 1, 2, 48, 0, 0) + command * 2
            with self.assertRaises(subject.InspectionError):
                inspect(archive(member("a.o/", raw)))

    def test_explicit_requirements_and_version_validation(self):
        for names in ([], [SYMBOL, SYMBOL], ["bad\nname"], ["x" * 257], [True]):
            with self.assertRaises(subject.InspectionError):
                subject.inspect_archive(archive(), names, "26.5.0")
        for minimum in ("26.5", "026.5.0", "26.256.0", "65536.0.0", "latest"):
            with self.assertRaises(subject.InspectionError):
                subject.inspect_archive(archive(), [SYMBOL], minimum)


class CliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.input = self.root / "input.a"
        self.input.write_bytes(archive())
        self.output = self.root / "evidence"

    def run_cli(self, *extra):
        argv = ["--archive", str(self.input), "--archive-sha256", subject.digest(self.input.read_bytes()),
                "--expected-minimum-os", "26.5.0", "--required-symbol", SYMBOL,
                "--output-dir", str(self.output), *extra]
        stream = io.StringIO()
        with contextlib.redirect_stdout(stream):
            code = subject.main(argv)
        return code, json.loads(stream.getvalue())

    def test_pass_retains_exact_receipt(self):
        before = self.input.read_bytes()
        code, report = self.run_cli()
        self.assertEqual(code, 0)
        self.assertEqual(json.loads((self.output / "result.json").read_text()), report)
        self.assertEqual(self.input.read_bytes(), before)
        self.assertEqual(report["inspector_sha256"], subject.digest(Path(subject.__file__).read_bytes()))

    def test_hash_mismatch_records_failure(self):
        code, report = self.run_cli("--archive-sha256", "0" * 64)
        self.assertEqual(code, 1)
        self.assertEqual(report["status"], "fail")
        self.assertTrue((self.output / "result.json").is_file())

    def test_missing_metadata_returns_blocked_not_pass(self):
        self.input.write_bytes(archive(member("a.o/", object_bytes(minimum=None))))
        self.assertEqual(self.run_cli()[0], 2)

    def test_malformed_object_retains_failure(self):
        self.input.write_bytes(archive(member("a.o/", b"not an object")))
        code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertEqual(json.loads((self.output / "result.json").read_text()), report)

    def test_existing_output_is_never_modified(self):
        self.output.mkdir()
        marker = self.output / "result.json"
        marker.write_text("retained original")
        self.assertEqual(self.run_cli()[0], 2)
        self.assertEqual(marker.read_text(), "retained original")

    def test_symlink_output_is_never_followed(self):
        target = self.root / "other"
        target.mkdir()
        self.output.symlink_to(target)
        self.assertEqual(self.run_cli()[0], 2)
        self.assertEqual(list(target.iterdir()), [])

    def test_symlink_input_rejected(self):
        link = self.root / "alias.a"
        link.symlink_to(self.input)
        code, _ = self.run_cli("--archive", str(link))
        self.assertEqual(code, 1)

    def test_fifo_input_does_not_block(self):
        fifo = self.root / "fifo"
        os.mkfifo(fifo)
        code, _ = self.run_cli("--archive", str(fifo))
        self.assertEqual(code, 1)

    def test_failed_receipt_write_never_passes(self):
        original = Path.open
        def broken(path, *args, **kwargs):
            if path.name == "result.json":
                raise OSError("synthetic write failure")
            return original(path, *args, **kwargs)
        with mock.patch.object(Path, "open", broken):
            code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("Could not retain", report["error"])

    def test_missing_parent_is_blocked(self):
        self.output = self.root / "absent" / "evidence"
        self.assertEqual(self.run_cli()[0], 2)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
