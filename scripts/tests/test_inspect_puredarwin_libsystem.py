"""Owned byte fixtures test static checks; no selected library or guest is run."""
import contextlib
import copy
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
import inspect_puredarwin_libsystem as check


def packed(value):
    major, minor, patch = map(int, value.split("."))
    return major << 16 | minor << 8 | patch


def dylib_command(kind, name, current=check.CURRENT_VERSION, compatibility=check.COMPATIBILITY_VERSION, gap=0):
    text = name.encode("ascii") + b"\0"
    size = (24 + gap + len(text) + 7) & ~7
    return (struct.pack("<6I", kind, size, 24 + gap, 0, packed(current), packed(compatibility))
            + b"\0" * gap + text).ljust(size, b"\0")


def make_dylib(name="libsystem", *, cpu=0x01000007, subtype=3, file_type=6,
               identity=None, current=check.CURRENT_VERSION, compatibility=check.COMPATIBILITY_VERSION,
               platform=1, minimum=check.MINIMUM_OS, sdk="26.5.0", edge=None,
               version_command=True, extra=(), symbols=False, identity_gap=0):
    dependency, load_kind = edge or check.EDGES[name]
    commands = [dylib_command(0xD, identity or check.IDS[name], current, compatibility, gap=identity_gap),
                dylib_command(0x8000001F if load_kind == "reexport" else 0xC, dependency)]
    if version_command:
        commands.append(struct.pack("<6I", 0x32, 24, platform, packed(minimum), packed(sdk), 0))
    commands.extend(extra)
    command_size = 152 + sum(map(len, commands)) + (24 if symbols else 0)
    data_start = 32 + command_size
    code = b"\xc3\x90\x90\x90\x90\x90\x90\x90"
    tail = b""
    if symbols:
        strings = b"\0_owned\0"
        tail = struct.pack("<IBBHQ", 1, 0x0F, 1, 0, data_start) + strings
        commands.append(struct.pack("<6I", 2, 24, data_start + len(code), 1,
                                    data_start + len(code) + 16, len(strings)))
    total = data_start + len(code) + len(tail)
    segment = struct.pack("<II16s4Q4I", 0x19, 152, b"__TEXT", 0, total, 0, total, 5, 5, 1, 0)
    section = struct.pack("<16s16sQQ8I", b"__text", b"__TEXT", data_start, len(code),
                          data_start, 0, 0, 0, 0x80000400, 0, 0, 0)
    all_commands = segment + section + b"".join(commands)
    return struct.pack("<8I", 0xFEEDFACF, cpu, subtype, file_type, 1 + len(commands), len(all_commands), 0x80, 0) + all_commands + code + tail


def command_offset(data, kind):
    offset = 32
    for _ in range(struct.unpack_from("<I", data, 16)[0]):
        cmd, size = struct.unpack_from("<II", data, offset)
        if cmd == kind:
            return offset
        offset += size
    raise AssertionError("Fixture command absent")


def replace(data, offset, fmt, *values):
    blob = bytearray(data)
    struct.pack_into(fmt, blob, offset, *values)
    return bytes(blob)


def expectations(blobs):
    files = {}
    for name, data in blobs.items():
        target, kind = check.EDGES[name]
        files[name] = {"sha256": check.digest(data), "dependencies": [{"name": target, "load_kind": kind,
                      "current_version": check.CURRENT_VERSION, "compatibility_version": check.COMPATIBILITY_VERSION}]}
    return {"schema_version": 1, "scope": check.EXPECTATION_SCOPE, "source_revision": check.REVISION,
            "minimum_os": check.MINIMUM_OS, "sdk_version": "26.5.0", "files": files}


class DylibTests(unittest.TestCase):
    def pair(self, system=None, dyld=None):
        blobs = {"libsystem": make_dylib() if system is None else system,
                 "libdyld": make_dylib("libdyld") if dyld is None else dyld}
        return blobs, expectations(blobs)

    def reject(self, data, pattern=None):
        blobs, contract = self.pair(system=data)
        with self.assertRaisesRegex(check.InspectionError, pattern or "."):
            check.inspect_pair(blobs, contract)

    def test_selected_identity_and_closed_pair(self):
        blobs, contract = self.pair(system=make_dylib(symbols=True))
        result = check.inspect_pair(blobs, contract)
        self.assertEqual(result["files"]["libsystem"]["macho"]["install_identity"]["name"], check.IDS["libsystem"])
        self.assertEqual(result["files"]["libsystem"]["macho"]["symbol_count"], 1)
        self.assertEqual([(x["from"], x["to"]) for x in result["dependency_path_closure"]],
                         [("libsystem", "libdyld"), ("libdyld", "libsystem")])

    def test_wrong_architecture_subtype_file_type_or_magic(self):
        for changes in ({"cpu": 0x0100000C}, {"subtype": 8}, {"subtype": 0x80000003}, {"file_type": 2}):
            with self.subTest(changes=changes):
                self.reject(make_dylib(**changes), "x86_64 MH_DYLIB")
        for magic in (0xCAFEBABE, 0xCFFAEDFE, 0xFEEDFACE):
            self.reject(replace(make_dylib(), 0, "<I", magic), "x86_64 MH_DYLIB")
        self.reject(replace(make_dylib(), 28, "<I", 1), "x86_64 MH_DYLIB")

    def test_wrong_platform_versions_and_missing_metadata(self):
        for changes in ({"platform": 2}, {"minimum": "11.0.0"}, {"sdk": "0.0.0"}, {"version_command": False}):
            with self.subTest(changes=changes):
                self.reject(make_dylib(**changes), "platform/deployment/SDK")
        self.reject(make_dylib(extra=[struct.pack("<6I", 0x32, 24, 1, 0x1A0500, 0x1A0500, 0)]), "duplicate platform")

    def test_wrong_missing_or_duplicate_install_identity(self):
        for changes in ({"identity": "/usr/lib/other.dylib"}, {"current": "1292.100.6"}, {"compatibility": "2.0.0"}):
            with self.subTest(changes=changes):
                self.reject(make_dylib(**changes), "LC_ID_DYLIB")
        self.reject(make_dylib(extra=[dylib_command(0xD, check.IDS["libsystem"])]), "duplicate LC_ID_DYLIB")
        data = make_dylib()
        self.reject(replace(data, command_offset(data, 0xD), "<I", 0xC), "LC_ID_DYLIB")

    def test_dependency_path_kind_duplicate_and_version_mismatch(self):
        for edge in (("@rpath/libdyld.dylib", "reexport"), ("/tmp/libdyld.dylib", "reexport"),
                     (check.IDS["libdyld"], "required"), ("/usr/lib/libobjc.A.dylib", "reexport")):
            with self.subTest(edge=edge):
                self.reject(make_dylib(edge=edge), "dependencies differ")
        self.reject(make_dylib(extra=[dylib_command(0xC, check.IDS["libdyld"])]), "dependencies differ")
        data = make_dylib()
        offset = command_offset(data, 0x8000001F)
        self.reject(replace(data, offset + 16, "<I", packed("2.0.0")), "dependencies differ")
        self.reject(replace(data, offset, "<I", 0x80000018), "dependencies differ")

    def test_rejects_unknown_required_commands_rpaths_encryption_and_loader(self):
        for cmd in (0x8000001C, 0x2C, 0xE, 0x27, 0x24, 0x8000FFFF, 0x1F):
            with self.subTest(command=cmd):
                self.reject(make_dylib(extra=[struct.pack("<II", cmd, 8)]), "Unsupported load command")

    def test_strings_refuse_bad_offsets_unterminated_controls_and_nonascii(self):
        data = make_dylib()
        offset = command_offset(data, 0xD)
        size = struct.unpack_from("<I", data, offset + 4)[0]
        self.reject(replace(data, offset + 8, "<I", 8), "string offset")
        self.reject(data[:offset + 24] + b"A" * (size - 24) + data[offset + size:], "Unterminated")
        for byte in (10, 0xFF):
            self.reject(data[:offset + 24] + bytes([byte]) + data[offset + 25:], "printable ASCII")

    def test_load_command_string_gap_and_trailing_padding_must_be_zero(self):
        data = make_dylib()
        for kind in (0xD, 0x8000001F):
            offset = command_offset(data, kind)
            size, name_offset = struct.unpack_from("<II", data, offset + 4)
            end = data.index(b"\0", offset + name_offset)
            self.assertLess(end + 1, offset + size)
            self.reject(replace(data, end + 1, "<B", ord("X")), "padding must be zero")
        data = make_dylib(identity_gap=8)
        blobs, contract = self.pair(system=data)
        self.assertEqual(check.inspect_pair(blobs, contract)["files"]["libsystem"]["sha256"], check.digest(data))
        self.reject(replace(data, command_offset(data, 0xD) + 24, "<B", ord("X")), "gap and trailing padding")

    def test_load_command_and_build_tool_bounds(self):
        data = make_dylib()
        for offset, value in ((16, 0), (16, check.MAX_COMMANDS + 1), (20, check.MAX_COMMAND_BYTES + 1),
                              (36, 7), (36, 150), (36, len(data) + 8)):
            with self.subTest(offset=offset, value=value):
                self.reject(replace(data, offset, "<I", value))
        self.reject(data[:31], "Mach-O header")
        self.reject(data[:-1], "segment file range")
        self.reject(replace(data, command_offset(data, 0x32) + 20, "<I", 1), "tool records")
        self.reject(replace(data, 16, "<I", 3), "count does not match")

    def test_segment_file_vm_protection_and_section_bounds(self):
        data = make_dylib()
        for offset, fmt, value in ((72, "<Q", len(data) + 1), (80, "<Q", len(data) + 1),
                                   (56, "<Q", (1 << 64) - 1), (88, "<I", 8), (92, "<I", 7),
                                   (96, "<I", 2), (152, "<I", 0), (156, "<I", 64),
                                   (160, "<I", len(data) + 1), (164, "<I", 1), (144, "<Q", len(data) + 1)):
            with self.subTest(offset=offset):
                self.reject(replace(data, offset, fmt, value))
        self.reject(data[:120] + b"__OTHER".ljust(16, b"\0") + data[136:], "different segment")
        self.reject(data[:40] + b"__OTHER".ljust(16, b"\0") + data[56:])

    def test_segment_overlap_and_missing_text(self):
        overlap = struct.pack("<II16s4Q4I", 0x19, 72, b"__OTHER", 0, 32, 0, 32, 1, 1, 0, 0)
        self.reject(make_dylib(extra=[overlap]), "Overlapping segment")
        duplicate = struct.pack("<II16s4Q4I", 0x19, 72, b"__TEXT", 0, 0, 0, 0, 1, 1, 0, 0)
        self.reject(make_dylib(extra=[duplicate]), "duplicate segments")
        self.reject(replace(make_dylib(), 92, "<I", 1), "executable __TEXT")

    def test_protected_highvm_fixedvm_and_unknown_segment_flags_are_unsupported(self):
        for flags in (1, 2, 8, 0x20, 0x80000000, 8 | check.SEGMENT_FLAGS):
            with self.subTest(flags=flags):
                self.reject(replace(make_dylib(), 100, "<I", flags), "Unsupported segment flags")
        blobs, contract = self.pair(system=replace(make_dylib(), 100, "<I", check.SEGMENT_FLAGS))
        self.assertFalse(check.inspect_pair(blobs, contract)["files"]["libsystem"]["macho"]["encrypted"])

    def test_unknown_and_unsupported_header_modes_and_reexport_contradiction_fail(self):
        for flags in (0x10000000, 0x80000000, 0x10, 0x20, 0x100, 0x20000, 0x200000):
            with self.subTest(flags=flags):
                self.reject(replace(make_dylib(), 24, "<I", 0x80 | flags), "Unsupported Mach-O header flags")
        self.reject(replace(make_dylib(), 24, "<I", 0x80 | check.HEADER_FLAGS["MH_NO_REEXPORTED_DYLIBS"]),
                    "Header denies reexports")
        blobs, contract = self.pair(dyld=replace(make_dylib("libdyld"), 24, "<I", 0x100085))
        self.assertEqual(check.inspect_pair(blobs, contract)["files"]["libdyld"]["macho"]["flags"], 0x100085)

    def test_unknown_section_types_attributes_and_inconsistent_file_vm_mapping_fail(self):
        for flags in (0x17, 0xFF, 0x80000400 | 0x01000000, 0x80000400 | 0x800):
            with self.subTest(flags=flags):
                self.reject(replace(make_dylib(), 168, "<I", flags), "Unsupported section type or attribute")
        self.reject(replace(make_dylib(), 136, "<Q", 0), "VM address and file offset disagree")

    def test_ordinary_zero_fill_mapping_accepts_tail_and_rejects_file_backed_addresses(self):
        segment = struct.pack("<II16s4Q4I", 0x19, 152, b"__DATA", 0x100000, 4096, 0, 0, 3, 3, 1, 0)
        section = struct.pack("<16s16sQQ8I", b"__bss", b"__DATA", 0x100000, 4096, 0, 12, 0, 0, 1, 0, 0, 0)
        blobs, contract = self.pair(system=make_dylib(extra=[segment + section]))
        self.assertEqual(check.inspect_pair(blobs, contract)["files"]["libsystem"]["macho"]["section_count"], 2)
        for offset in (0, 1):
            data = replace(make_dylib(), 168, "<I", 1)
            data = replace(data, 152, "<I", offset)
            self.reject(data, "zero-filled tail")

    def test_symbol_tables_are_bounded_but_not_used_as_a_runtime_claim(self):
        data = make_dylib(symbols=True)
        offset = command_offset(data, 2)
        symoff = struct.unpack_from("<I", data, offset + 8)[0]
        for place, value in ((offset + 8, 0), (offset + 12, check.MAX_SYMBOLS + 1),
                             (offset + 16, len(data)), (symoff, 500)):
            with self.subTest(offset=place):
                self.reject(replace(data, place, "<I", value))
        self.reject(replace(data, symoff + 5, "<B", 2), "symbol section index")
        self.reject(data[:-1] + b"X", "Unterminated or oversized symbol")
        self.reject(replace(data, symoff + 8, "<Q", 0), "outside its section")
        self.reject(make_dylib(extra=[struct.pack("<20I", 0xB, 80, *([0] * 18))]), "has no symbol table")

    def test_dynamic_symbol_subranges_and_linkedit_bounds(self):
        dynamic = struct.pack("<20I", 0xB, 80, 0, 2, *([0] * 16))
        self.reject(make_dylib(extra=[dynamic], symbols=True), "Dynamic symbol range")
        for kind in check.LINKEDIT_COMMANDS:
            with self.subTest(kind=kind):
                self.reject(make_dylib(extra=[struct.pack("<4I", kind, 16, 1, 1)]), "overlaps load commands")
                self.reject(make_dylib(extra=[struct.pack("<4I", kind, 16, 100000, 1)]), "Out-of-bounds")
        self.reject(make_dylib(extra=[struct.pack("<12I", 0x80000022, 48, 1, 1, *([0] * 8))]), "dyld-info data")

    def test_file_hash_and_contract_schema_binding(self):
        blobs, contract = self.pair()
        cases = []
        for key, value in (("schema_version", True), ("scope", "planned"), ("source_revision", "0" * 40),
                           ("minimum_os", "11.0.0"), ("sdk_version", "026.5.0")):
            changed = copy.deepcopy(contract)
            changed[key] = value
            cases.append(changed)
        changed = copy.deepcopy(contract)
        changed["files"]["libsystem"]["sha256"] = "0" * 64
        cases.append(changed)
        changed = copy.deepcopy(contract)
        changed["files"]["libsystem"]["dependencies"][0]["name"] = "/usr/lib/other.dylib"
        cases.append(changed)
        changed = copy.deepcopy(contract)
        changed["files"]["libdyld"]["dependencies"] = []
        cases.append(changed)
        changed = copy.deepcopy(contract)
        changed["files"]["libsystem"]["dependencies"][0]["compatibility_version"] = "65535.0.0"
        cases.append(changed)
        changed = copy.deepcopy(contract)
        changed["permit_any_dependency"] = True
        cases.append(changed)
        for changed in cases:
            with self.subTest(contract=changed), self.assertRaises(check.InspectionError):
                check.inspect_pair(blobs, changed)


class CliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.blobs = {name: make_dylib(name) for name in check.IDS}
        self.paths = {name: self.root / (name + ".dylib") for name in check.IDS}
        for name, path in self.paths.items():
            path.write_bytes(self.blobs[name])
        self.contract = self.root / "reviewed.json"
        self.contract.write_text(json.dumps(expectations(self.blobs)))
        self.output = self.root / "evidence"

    def argv(self, output=None):
        return ["--libsystem", str(self.paths["libsystem"]), "--libdyld", str(self.paths["libdyld"]),
                "--expectations", str(self.contract), "--expectations-sha256", check.digest(self.contract.read_bytes()),
                "--output-dir", str(output or self.output)]

    def run_cli(self, argv=None):
        with contextlib.redirect_stdout(io.StringIO()) as stdout:
            code = check.main(argv or self.argv())
        return code, json.loads(stdout.getvalue())

    def test_retained_pass_is_only_a_static_check(self):
        code, report = self.run_cli()
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "pass")
        for key, value in check.LIMITATIONS.items():
            self.assertEqual(report[key], value)
        self.assertEqual(json.loads((self.output / "result.json").read_text()), report)
        self.assertEqual((self.output / "expectations.json").read_bytes(), self.contract.read_bytes())
        self.assertEqual(self.paths["libsystem"].read_bytes(), self.blobs["libsystem"])

    def test_wrong_dylib_or_expectation_hash_fails_with_evidence(self):
        self.paths["libsystem"].write_bytes(self.blobs["libsystem"] + b"changed")
        code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("checksum differs", report["error"])
        self.assertEqual(json.loads((self.output / "result.json").read_text())["status"], "fail")
        argv = self.argv(self.root / "other")
        argv[argv.index("--expectations-sha256") + 1] = "0" * 64
        code, report = self.run_cli(argv)
        self.assertEqual(code, 1)
        self.assertIn("JSON checksum", report["error"])

    def test_duplicate_json_keys_are_rejected(self):
        self.contract.write_text('{"schema_version":1,"schema_version":1}')
        code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("Duplicate JSON key", report["error"])

    def test_review_regressions_fail_despite_matching_reviewed_file_hashes(self):
        identity = command_offset(self.blobs["libsystem"], 0xD)
        padding = self.blobs["libsystem"].index(b"\0", identity + 24) + 1
        cases = (("protected", 100, "<I", 8), ("highvm", 100, "<I", 1),
                 ("unknown-header", 24, "<I", 0x10000000), ("unknown-section", 168, "<I", 255),
                 ("mapping", 136, "<Q", 0), ("string-padding", padding, "<B", ord("X")))
        for name, offset, fmt, value in cases:
            with self.subTest(case=name):
                changed = {**self.blobs, "libsystem": replace(self.blobs["libsystem"], offset, fmt, value)}
                self.paths["libsystem"].write_bytes(changed["libsystem"])
                self.contract.write_text(json.dumps(expectations(changed)))
                directory = self.root / name
                code, report = self.run_cli(self.argv(directory))
                self.assertEqual(code, 1)
                self.assertEqual(report["status"], "fail")
                self.assertFalse(report["selected_libsystem_qualified"])
                self.assertNotIn("checksum", report["error"])
                self.assertEqual(json.loads((directory / "result.json").read_text())["status"], "fail")

    def test_read_refuses_symlinks_fifo_empty_and_oversized_input(self):
        for name in ("symlink", "fifo", "empty", "large"):
            with self.subTest(name=name):
                target = self.root / name
                if name == "symlink":
                    target.symlink_to(self.paths["libsystem"])
                elif name == "fifo":
                    os.mkfifo(target)
                else:
                    with target.open("wb") as stream:
                        stream.truncate(0 if name == "empty" else check.MAX_FILE + 1)
                with self.assertRaises((OSError, check.InspectionError)):
                    check.read_regular(target, check.MAX_FILE)

    def test_read_detects_file_replacement_during_read(self):
        original = check.os.fstat
        calls = 0
        def changed(fd):
            nonlocal calls
            calls += 1
            result = original(fd)
            if calls == 2:
                moved = self.root / "moved"
                self.paths["libsystem"].rename(moved)
                self.paths["libsystem"].write_bytes(self.blobs["libsystem"])
            return result
        with mock.patch.object(check.os, "fstat", side_effect=changed), self.assertRaisesRegex(check.InspectionError, "changed"):
            check.read_regular(self.paths["libsystem"], check.MAX_FILE)

    def test_exclusive_directory_preserves_prior_evidence_and_symlink(self):
        self.output.mkdir()
        prior = self.output / "result.json"
        prior.write_text("preserve")
        code, report = self.run_cli()
        self.assertEqual(code, 2)
        self.assertEqual(report["status"], "blocked")
        self.assertEqual(prior.read_text(), "preserve")
        link = self.root / "output-link"
        link.symlink_to(self.output, target_is_directory=True)
        code, _ = self.run_cli(self.argv(link))
        self.assertEqual(code, 2)
        self.assertEqual(prior.read_text(), "preserve")

    def test_missing_evidence_parent_is_blocked(self):
        code, report = self.run_cli(self.argv(self.root / "missing" / "evidence"))
        self.assertEqual(code, 2)
        self.assertIn("parent", report["error"])

    def test_input_and_helper_mutation_before_success_are_rejected(self):
        original = check.inspect_pair
        def mutate(blobs, contract):
            result = original(blobs, contract)
            self.paths["libdyld"].write_bytes(self.blobs["libdyld"] + b"changed")
            return result
        with mock.patch.object(check, "inspect_pair", side_effect=mutate):
            code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("Dylib changed", report["error"])
        self.paths["libdyld"].write_bytes(self.blobs["libdyld"])
        reader = check.read_regular
        helper_reads = 0
        def changed_helper(path, limit):
            nonlocal helper_reads
            data = reader(path, limit)
            if path == Path(check.__file__):
                helper_reads += 1
                if helper_reads > 1:
                    data += b"# changed"
            return data
        with mock.patch.object(check, "read_regular", side_effect=changed_helper):
            code, report = self.run_cli(self.argv(self.root / "helper-drift"))
        self.assertEqual(code, 1)
        self.assertIn("Inspector changed", report["error"])

    def test_expectation_mutation_before_success_is_rejected(self):
        original = check.inspect_pair
        def mutate(blobs, contract):
            result = original(blobs, contract)
            self.contract.write_text(self.contract.read_text() + "\n")
            return result
        with mock.patch.object(check, "inspect_pair", side_effect=mutate):
            code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("Expectations changed", report["error"])

    def test_reinspection_binds_previous_bytes_and_substantive_result(self):
        self.assertEqual(self.run_cli()[0], 0)
        previous = self.output / "result.json"
        def recheck(directory):
            return self.argv(self.root / directory) + ["--previous-inspection", str(previous),
                "--previous-inspection-sha256", check.digest(previous.read_bytes())]
        code, report = self.run_cli(recheck("second"))
        self.assertEqual(code, 0)
        self.assertEqual(report["previous_inspection_sha256"], check.digest(previous.read_bytes()))
        for number, (key, value) in enumerate((("status", "planned"), ("selected_libsystem_qualified", True),
                ("selected_libsystem_qualified", 0), ("schema_version", True))):
            old = json.loads(previous.read_text())
            old[key] = value
            previous.write_text(json.dumps(old))
            code, report = self.run_cli(recheck("bad-" + key + str(number)))
            self.assertEqual(code, 1)
            self.assertIn("Previous", report["error"])
            old[key] = "pass" if key == "status" else (False if key == "selected_libsystem_qualified" else 1)
            previous.write_text(json.dumps(old))
        old = json.loads(previous.read_text())
        old["inspection"]["files"]["libsystem"]["macho"]["flags"] = 0
        previous.write_text(json.dumps(old))
        code, report = self.run_cli(recheck("different-metadata"))
        self.assertEqual(code, 1)
        self.assertIn("Previous inspection differs: inspection", report["error"])

    def test_previous_receipt_checksum_cannot_be_replaced(self):
        self.assertEqual(self.run_cli()[0], 0)
        previous = self.output / "result.json"
        argv = self.argv(self.root / "bad-previous") + ["--previous-inspection", str(previous),
                                                       "--previous-inspection-sha256", "0" * 64]
        code, report = self.run_cli(argv)
        self.assertEqual(code, 1)
        self.assertIn("JSON checksum", report["error"])

    def test_snapshot_write_failure_is_never_pass(self):
        writer = check.retain_bytes
        def fail_snapshot(path, data):
            if path.name == "expectations.json":
                raise OSError("snapshot write failed")
            return writer(path, data)
        with mock.patch.object(check, "retain_bytes", side_effect=fail_snapshot):
            code, report = self.run_cli()
        self.assertEqual(code, 1)
        self.assertEqual(json.loads((self.output / "result.json").read_text())["status"], "fail")
        self.assertIn("snapshot write failed", report["error"])

    def test_partial_write_flush_fsync_close_or_publication_failure_cannot_publish_success(self):
        for failure in ("write", "flush", "fsync", "close", "link"):
            with self.subTest(failure=failure):
                directory = self.root / ("failed-" + failure)
                open_file, fsync = Path.open, check.os.fsync
                target_fd = None
                class BrokenStream:
                    def __init__(self, stream):
                        self.stream = stream
                    def __enter__(self):
                        return self
                    def __exit__(self, *args):
                        self.stream.close()
                        if failure == "close":
                            raise OSError("close failed")
                    def write(self, raw):
                        if failure == "write":
                            self.stream.write(raw[:10])
                            raise OSError("partial write")
                        return self.stream.write(raw)
                    def flush(self):
                        if failure == "flush":
                            raise OSError("flush failed")
                        return self.stream.flush()
                    def fileno(self):
                        return self.stream.fileno()
                def broken_open(path, *args, **kwargs):
                    nonlocal target_fd
                    stream = open_file(path, *args, **kwargs)
                    if path.name == "result.json.partial":
                        target_fd = stream.fileno()
                        return BrokenStream(stream)
                    return stream
                def broken_fsync(fd):
                    if failure == "fsync" and fd == target_fd:
                        raise OSError("fsync failed")
                    return fsync(fd)
                with mock.patch.object(Path, "open", new=broken_open), \
                        mock.patch.object(check.os, "fsync", side_effect=broken_fsync), \
                        mock.patch.object(check.os, "link", side_effect=OSError("failed link")) if failure == "link" else contextlib.nullcontext():
                    code, report = self.run_cli(self.argv(directory))
                self.assertEqual(code, 1)
                self.assertEqual(report["status"], "fail")
                self.assertFalse((directory / "result.json").exists())


if __name__ == "__main__":
    unittest.main()
