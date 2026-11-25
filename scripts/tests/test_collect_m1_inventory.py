#!/usr/bin/env python3
"""Synthetic parser, passive-inspection and privacy regression tests."""

import importlib.util
import io
import json
from pathlib import Path
import plistlib
import struct
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("inventory", Path(__file__).resolve().parents[1] / "collect_m1_inventory.py")
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


def dylib(name, kind=0xC, endian="<"):
    string = name.encode() + b"\0"
    size = (24 + len(string) + 7) & ~7
    return struct.pack(endian + "6I", kind, size, 24, 0, 0x10002, 0x10000) + string + b"\0" * (size - 24 - len(string))


def thin(cpu=0x01000007, subtype=3, commands=(), endian="<"):
    data = b"".join(commands)
    magic = b"\xcf\xfa\xed\xfe" if endian == "<" else b"\xfe\xed\xfa\xcf"
    return magic + struct.pack(endian + "7I", cpu, subtype, 2, len(commands), len(data), 0, 0) + data


def universal(slices, fat64=False, endian=">"):
    entry_size = 32 if fat64 else 20
    offset = 8 + len(slices) * entry_size
    entries, bodies = [], []
    for cpu, subtype, body in slices:
        fields = (cpu, subtype, offset, len(body), 0, 0) if fat64 else (cpu, subtype, offset, len(body), 0)
        entries.append(struct.pack(endian + ("IIQQII" if fat64 else "IIIII"), *fields))
        bodies.append(body)
        offset += len(body)
    magic = struct.pack(endian + "I", 0xCAFEBABF if fat64 else 0xCAFEBABE)
    return magic + struct.pack(endian + "I", len(slices)) + b"".join(entries + bodies)


class MachOTests(unittest.TestCase):
    def parse(self, data):
        return inventory.parse_macho(io.BytesIO(data), len(data))

    def test_direct_dependencies_and_version_are_observed(self):
        version = struct.pack("<6I", 0x32, 24, 1, 0x0D0000, 0x0F0100, 0)
        result = self.parse(thin(commands=[dylib("/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"), version]))
        item = result["slices"][0]
        self.assertEqual(item["architecture"], "x86_64")
        self.assertEqual(item["dependencies"][0]["classification"], "system_public_path_candidate")
        self.assertEqual(item["platform_versions"][0]["minimum_os"], "13.0.0")
        self.assertEqual(item["platform_versions"][0]["sdk"], "15.1.0")
        self.assertFalse(item["code_signature_command_present"])

    def test_all_fat_byte_orders_and_widths(self):
        slices = [(0x01000007, 3, thin()), (0x0100000C, 2, thin(0x0100000C, 2))]
        for endian in ("<", ">"):
            for fat64 in (False, True):
                with self.subTest(endian=endian, fat64=fat64):
                    result = self.parse(universal(slices, fat64, endian))
                    self.assertEqual([s["architecture"] for s in result["slices"]], ["x86_64", "arm64e"])

    def test_big_endian_thin(self):
        self.assertEqual(self.parse(thin(endian=">"))["slices"][0]["architecture"], "x86_64")

    def test_weak_and_private_dependencies(self):
        result = self.parse(thin(commands=[dylib("/System/Library/PrivateFrameworks/Example.framework/Example", 0x80000018)]))
        dep = result["slices"][0]["dependencies"][0]
        self.assertEqual(dep["load_kind"], "weak")
        self.assertEqual(dep["classification"], "private_framework")

    def test_absolute_developer_paths_are_redacted(self):
        result = self.parse(thin(commands=[dylib("/Users/alice/private-build/libsecret.dylib")]))
        self.assertNotIn("alice", json.dumps(result))
        self.assertEqual(result["slices"][0]["dependencies"][0]["classification"], "external_absolute")

    def test_truncated_and_unknown_formats(self):
        for data in (b"", b"hello", thin()[:20], thin(commands=[dylib("@rpath/libone.dylib")])[:-1]):
            with self.subTest(size=len(data)), self.assertRaises(inventory.InventoryError):
                self.parse(data)

    def test_bad_command_region(self):
        data = bytearray(thin(commands=[dylib("@rpath/libone.dylib")]))
        struct.pack_into("<I", data, 36, 4)
        with self.assertRaises(inventory.InventoryError):
            self.parse(data)

    def test_bad_dylib_string_offset(self):
        data = bytearray(thin(commands=[dylib("@rpath/libone.dylib")]))
        struct.pack_into("<I", data, 40, 8)
        with self.assertRaises(inventory.InventoryError):
            self.parse(data)

    def test_architecture_mismatch(self):
        data = universal([(0x0100000C, 0, thin())])
        with self.assertRaises(inventory.InventoryError):
            self.parse(data)

    def test_overlapping_fat_slices(self):
        data = bytearray(universal([(0x01000007, 3, thin()), (0x01000007, 3, thin())]))
        struct.pack_into(">I", data, 8 + 20 + 8, 48)
        with self.assertRaises(inventory.InventoryError):
            self.parse(data)


class InventoryTests(unittest.TestCase):
    def make_bundle(self, directory, executable="Example"):
        bundle = Path(directory) / "Example.app"
        (bundle / "Contents/MacOS").mkdir(parents=True)
        (bundle / "Contents/Info.plist").write_bytes(plistlib.dumps({
            "CFBundleExecutable": executable, "CFBundleIdentifier": "org.example.fixture",
            "CFBundleShortVersionString": "1.2", "CFBundleVersion": "12",
        }))
        (bundle / "Contents/MacOS/Example").write_bytes(thin())
        return bundle

    def test_bundle_only_static_inspection(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(directory)
            with patch.object(inventory.platform, "system", return_value="Linux"), patch.object(inventory, "run_tool") as tool:
                report = inventory.app_inventory(bundle)
            tool.assert_not_called()
            self.assertEqual(report["metadata"]["CFBundleVersion"], "12")
            self.assertEqual(len(report["main_executable"]["sha256"]), 64)
            self.assertFalse(report["coverage"]["application_launched"])
            self.assertFalse(report["coverage"]["transitive_dependency_graph"])
            self.assertNotIn(directory, json.dumps(report))

    def test_symlink_escape_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(directory)
            outside = Path(directory) / "outside"
            outside.write_bytes(thin())
            path = bundle / "Contents/MacOS/Example"
            path.unlink()
            path.symlink_to(outside)
            with self.assertRaises(inventory.InventoryError):
                inventory.app_inventory(bundle)

    def test_executable_path_traversal_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(directory, "../../../outside")
            with self.assertRaises(inventory.InventoryError):
                inventory.app_inventory(bundle)

    def test_malformed_xml_plist_has_safe_diagnostic(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = self.make_bundle(directory)
            (bundle / "Contents/Info.plist").write_bytes(b'<?xml version="1.0"?><plist><dict>')
            with self.assertRaisesRegex(inventory.InventoryError, "Info.plist is unreadable or invalid"):
                inventory.app_inventory(bundle)

    def test_raw_hardware_identity_fields_are_excluded(self):
        data = [{"_name": "Alice's keyboard", "serial_num": "SECRET-SERIAL",
                 "platform_UUID": "SECRET-UUID", "vendor_id": "0x1234",
                 "_items": [{"product_id": "0x5678", "_name": "Alice's mouse",
                              "mac_address": "11:22:33:44:55:66", "SSID": "home-address"}]}]
        output = inventory.filtered_profile(data, inventory.PROFILE_KEYS["SPUSBDataType"])
        self.assertEqual(output, [{"vendor_id": "0x1234"}, {"product_id": "0x5678"}])

    def test_signature_display_does_not_claim_trust_or_emit_paths(self):
        raw = "Executable=/Users/alice/Example.app/Contents/MacOS/Example\nAuthority=Alice Secret\nTeamIdentifier=SECRETTEAM\nFormat=app bundle with Mach-O thin (x86_64)\nSignature=adhoc\nCDHash=1234\n"
        with patch.object(inventory.platform, "system", return_value="Darwin"), patch.object(inventory, "run_tool", return_value={"status": "ok", "return_code": 0, "stderr": raw}) as tool:
            report = inventory.signature_metadata(Path("/selected/Example.app"))
        tool.assert_called_once_with(["/usr/bin/codesign", "--display", "--verbose=4", "/selected/Example.app"])
        self.assertFalse(report["verification_performed"])
        self.assertFalse(report["trust_assessed"])
        self.assertEqual(report["display_fields"]["Signature"], "adhoc")
        self.assertNotIn("alice", json.dumps(report))
        self.assertNotIn("SECRET", json.dumps(report))

    def test_non_macos_does_not_probe_hardware(self):
        with patch.object(inventory.platform, "system", return_value="Linux"), patch.object(inventory, "run_tool") as tool:
            self.assertEqual(inventory.hardware_inventory()["status"], "not_macos")
        tool.assert_not_called()

    def test_invalid_app_cli_produces_safe_json_and_nonzero(self):
        with patch.object(inventory.sys, "stdout", new_callable=io.StringIO) as output:
            code = inventory.main(["--no-hardware", "--app", "/Users/alice/no-such-app.app"])
        report = json.loads(output.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(report["application"]["status"], "failed")
        self.assertNotIn("alice", output.getvalue())


if __name__ == "__main__":
    unittest.main()
