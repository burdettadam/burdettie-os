#!/usr/bin/env python3
"""Collect a static Burdettie M1 inventory; never launch the selected application.

Python 3.9+, standard library only. JSON goes to stdout. Hardware collection is
available on macOS; Mach-O/Info.plist inspection also works on other hosts.
This is an inventory tool, not an executable verifier or a sandbox.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import stat
import struct
import subprocess
import sys
from xml.parsers.expat import ExpatError


COLLECTOR_VERSION = "1.0.0"
MAX_PLIST_BYTES = 4 * 1024 * 1024
MAX_LOAD_COMMAND_BYTES = 16 * 1024 * 1024
THIN_MAGICS = {
    b"\xce\xfa\xed\xfe": ("<", False),
    b"\xcf\xfa\xed\xfe": ("<", True),
    b"\xfe\xed\xfa\xce": (">", False),
    b"\xfe\xed\xfa\xcf": (">", True),
}
FAT_MAGICS = {
    b"\xca\xfe\xba\xbe": (">", False),
    b"\xbe\xba\xfe\xca": ("<", False),
    b"\xca\xfe\xba\xbf": (">", True),
    b"\xbf\xba\xfe\xca": ("<", True),
}
PROFILE_TYPES = (
    "SPHardwareDataType", "SPDisplaysDataType", "SPNVMeDataType",
    "SPSerialATADataType", "SPUSBDataType", "SPEthernetDataType",
    "SPAirPortDataType",
)
# Explicit allowlists are deliberate. Never emit raw system_profiler JSON:
# it can contain serials, UUIDs, MAC addresses, SSIDs and personal device names.
PROFILE_KEYS = {
    "SPHardwareDataType": {
        "machine_model", "chip_type", "boot_rom_version", "os_loader_version",
    },
    "SPDisplaysDataType": {
        "sppci_model", "spdisplays_vendor", "spdisplays_device-id",
        "spdisplays_revision-id", "spdisplays_bus", "spdisplays_vram",
        "spdisplays_vram_shared", "spdisplays_resolution", "spdisplays_metal",
    },
    "SPNVMeDataType": {
        "device_model", "spnvme_model", "link_speed", "link_width",
        "spsata_medium_type", "size_in_bytes", "physical_interconnect",
    },
    "SPSerialATADataType": {
        "device_model", "spsata_model", "spsata_medium_type",
        "spsata_linkspeed", "spsata_negotiatedlinkspeed", "size_in_bytes",
        "physical_interconnect", "spsata_ahci_version", "spsata_vendor",
    },
    "SPUSBDataType": {
        "vendor_id", "product_id", "bcd_device", "bus_power",
        "device_speed", "host_controller", "pci_device", "pci_vendor",
    },
    "SPEthernetDataType": {
        "spethernet_chipset", "spethernet_device_type", "spethernet_driver",
        "spethernet_vendor_id", "spethernet_device_id",
    },
    "SPAirPortDataType": {
        "spairport_wireless_card_type", "spairport_card_type",
        "spairport_firmware_version", "spairport_supported_phymodes",
    },
}


class InventoryError(ValueError):
    """A safe, fixed diagnostic suitable for the public inventory."""


def clean_text(value, limit=512):
    if not isinstance(value, str):
        return None
    return "".join(c for c in value if c.isprintable())[:limit]


def run_tool(argv, timeout=30):
    """Invoke only OS tools supplied by this module; never use a shell."""
    env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LC_ALL": "C", "LANG": "C"}
    try:
        result = subprocess.run(
            argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, timeout=timeout, check=False,
            env=env,
        )
        return {"status": "ok" if result.returncode == 0 else "failed",
                "return_code": result.returncode,
                "stdout": result.stdout, "stderr": result.stderr}
    except FileNotFoundError:
        return {"status": "unavailable"}
    except subprocess.TimeoutExpired:
        return {"status": "timeout"}
    except (OSError, UnicodeError):
        return {"status": "failed"}


def tool_scalar(argv):
    result = run_tool(argv)
    return clean_text(result.get("stdout", "").strip()) if result["status"] == "ok" else None


def filtered_profile(value, allowed):
    """Flatten only allowlisted scalar fields, omitting names and raw subtrees."""
    rows = []
    if isinstance(value, list):
        for item in value:
            rows.extend(filtered_profile(item, allowed))
    elif isinstance(value, dict):
        row = {}
        for key in sorted(allowed):
            val = value.get(key)
            if isinstance(val, str):
                row[key] = clean_text(val)
            elif isinstance(val, (int, float, bool)):
                row[key] = val
        if row:
            rows.append(row)
        for val in value.values():
            if isinstance(val, (list, dict)):
                rows.extend(filtered_profile(val, allowed))
    return rows


def hardware_inventory():
    if platform.system() != "Darwin":
        return {"status": "not_macos", "hardware_qualified": False}
    values = {}
    for name in ("hw.model", "hw.machine", "hw.optional.arm64", "hw.memsize",
                 "hw.physicalcpu", "hw.logicalcpu", "machdep.cpu.brand_string"):
        values[name] = tool_scalar(["/usr/sbin/sysctl", "-n", name])
    arm64 = values["hw.optional.arm64"]
    architecture = "arm64" if arm64 == "1" else values["hw.machine"]
    profile = run_tool(["/usr/sbin/system_profiler", "-detailLevel", "mini",
                        "-json", *PROFILE_TYPES], timeout=45)
    try:
        parsed = json.loads(profile.get("stdout", "{}")) if profile["status"] == "ok" else {}
        if not isinstance(parsed, dict):
            raise ValueError()
    except (ValueError, TypeError):
        parsed = {}
        profile["status"] = "invalid_output"
    devices = {kind: filtered_profile(parsed.get(kind, []), PROFILE_KEYS[kind])
               for kind in PROFILE_TYPES}
    return {
        "status": "collected_with_unmeasured_fields",
        "hardware_qualified": False,
        "model_identifier": values["hw.model"],
        "hardware_architecture": architecture,
        "collector_process_architecture": platform.machine(),
        "cpu_brand": values["machdep.cpu.brand_string"],
        "physical_cpu_count": values["hw.physicalcpu"],
        "logical_cpu_count": values["hw.logicalcpu"],
        "memory_bytes": values["hw.memsize"],
        "macos_version": tool_scalar(["/usr/bin/sw_vers", "-productVersion"]),
        "macos_build": tool_scalar(["/usr/bin/sw_vers", "-buildVersion"]),
        "device_observations": devices,
        "device_tool_status": profile["status"],
        "unmeasured": [
            "boot method and alternate OS boot support",
            "Burdettie storage, display, input and network driver operation",
            "unreported device models and controller details",
        ],
    }


def read_exact(stream, offset, size, total):
    if offset < 0 or size < 0 or offset + size > total:
        raise InventoryError("Mach-O range exceeds file or slice bounds")
    stream.seek(offset)
    data = stream.read(size)
    if len(data) != size:
        raise InventoryError("Mach-O file changed or is truncated")
    return data


def architecture_name(cpu, subtype):
    subtype &= 0x00FFFFFF
    if cpu == 0x0100000C:
        return "arm64e" if subtype == 2 else "arm64"
    if cpu == 0x01000007:
        return "x86_64h" if subtype == 8 else "x86_64"
    return {7: "i386", 12: "arm", 18: "ppc", 0x01000012: "ppc64"}.get(cpu, "unknown")


def packed_version(number):
    return f"{number >> 16}.{(number >> 8) & 255}.{number & 255}"


def install_name(value):
    text = clean_text(value)
    if text.startswith("/") and not text.startswith(("/System/Library/", "/usr/lib/", "/Library/Frameworks/")):
        return {"name": "<redacted-absolute-path>", "classification": "external_absolute"}
    if "/PrivateFrameworks/" in text:
        classification = "private_framework"
    elif text.startswith(("/System/Library/Frameworks/", "/usr/lib/")):
        classification = "system_public_path_candidate"
    elif text.startswith(("@rpath/", "@executable_path/", "@loader_path/")):
        classification = "relative_unresolved"
    else:
        classification = "other_unresolved"
    return {"name": text, "classification": classification,
            "text_normalized_or_truncated": text != value}


def command_string(command, offset, minimum):
    if offset < minimum or offset >= len(command):
        raise InventoryError("Invalid Mach-O string offset")
    end = command.find(b"\0", offset)
    if end < 0:
        raise InventoryError("Unterminated Mach-O load-command string")
    return command[offset:end].decode("utf-8", errors="replace")


def parse_slice(stream, start, size, total):
    magic = read_exact(stream, start, 4, total)
    if magic not in THIN_MAGICS:
        raise InventoryError("Unsupported or invalid Mach-O slice magic")
    endian, is64 = THIN_MAGICS[magic]
    header_size = 32 if is64 else 28
    if size < header_size:
        raise InventoryError("Truncated Mach-O header")
    header = read_exact(stream, start, header_size, total)
    cpu, subtype, file_type, count, command_bytes, _flags = struct.unpack_from(endian + "6I", header, 4)
    if count > 65536 or command_bytes > MAX_LOAD_COMMAND_BYTES or header_size + command_bytes > size:
        raise InventoryError("Invalid or excessive Mach-O load-command region")
    commands = read_exact(stream, start + header_size, command_bytes, total)
    info = {"architecture": architecture_name(cpu, subtype), "cpu_type": cpu,
            "cpu_subtype": subtype, "file_type": file_type, "offset": start,
            "size_bytes": size, "dependencies": [], "rpaths": [],
            "platform_versions": [], "code_signature_command_present": False,
            "encrypted": False}
    cursor = 0
    for _ in range(count):
        if cursor + 8 > len(commands):
            raise InventoryError("Truncated Mach-O load command")
        cmd, cmdsize = struct.unpack_from(endian + "II", commands, cursor)
        if cmdsize < 8 or cursor + cmdsize > len(commands):
            raise InventoryError("Invalid Mach-O load-command size")
        command = commands[cursor:cursor + cmdsize]
        kind = cmd & 0x7FFFFFFF
        if kind in (0xC, 0x18, 0x1F, 0x20, 0x23):
            if cmdsize < 24:
                raise InventoryError("Truncated Mach-O dylib load command")
            name_offset, _timestamp, current, compatibility = struct.unpack_from(endian + "4I", command, 8)
            dependency = install_name(command_string(command, name_offset, 24))
            dependency.update({"load_kind": {0xC: "required", 0x18: "weak", 0x1F: "reexport",
                                             0x20: "lazy", 0x23: "upward"}[kind],
                               "current_version": packed_version(current),
                               "compatibility_version": packed_version(compatibility)})
            info["dependencies"].append(dependency)
        elif kind == 0x1C:
            if cmdsize < 12:
                raise InventoryError("Truncated Mach-O rpath command")
            offset = struct.unpack_from(endian + "I", command, 8)[0]
            info["rpaths"].append(install_name(command_string(command, offset, 12)))
        elif kind == 0x32:
            if cmdsize < 24:
                raise InventoryError("Truncated Mach-O build-version command")
            platform_id, minimum, sdk, tool_count = struct.unpack_from(endian + "4I", command, 8)
            if 24 + tool_count * 8 > cmdsize:
                raise InventoryError("Truncated Mach-O build-tool records")
            info["platform_versions"].append({"command": "LC_BUILD_VERSION", "platform_id": platform_id,
                                              "minimum_os": packed_version(minimum), "sdk": packed_version(sdk)})
        elif kind in (0x24, 0x25, 0x2F, 0x30):
            if cmdsize < 16:
                raise InventoryError("Truncated Mach-O minimum-version command")
            minimum, sdk = struct.unpack_from(endian + "2I", command, 8)
            info["platform_versions"].append({"command": {0x24: "LC_VERSION_MIN_MACOSX",
                0x25: "LC_VERSION_MIN_IPHONEOS", 0x2F: "LC_VERSION_MIN_TVOS", 0x30: "LC_VERSION_MIN_WATCHOS"}[kind],
                "minimum_os": packed_version(minimum), "sdk": packed_version(sdk)})
        elif kind == 0x1D:
            if cmdsize < 16:
                raise InventoryError("Truncated Mach-O code-signature command")
            data_offset, data_size = struct.unpack_from(endian + "2I", command, 8)
            if data_offset + data_size > size:
                raise InventoryError("Code-signature data exceeds Mach-O slice")
            info["code_signature_command_present"] = True
        elif kind in (0x21, 0x2C):
            if cmdsize < (24 if kind == 0x2C else 20):
                raise InventoryError("Truncated Mach-O encryption command")
            info["encrypted"] = struct.unpack_from(endian + "I", command, 16)[0] != 0
        cursor += cmdsize
    if cursor != command_bytes:
        raise InventoryError("Mach-O command count does not match command region")
    return info


def parse_macho(stream, total):
    magic = read_exact(stream, 0, 4, total)
    if magic in THIN_MAGICS:
        return {"format": "thin", "slices": [parse_slice(stream, 0, total, total)]}
    if magic not in FAT_MAGICS:
        raise InventoryError("Main executable is not a supported Mach-O file")
    endian, is64 = FAT_MAGICS[magic]
    count = struct.unpack(endian + "I", read_exact(stream, 4, 4, total))[0]
    if not 1 <= count <= 64:
        raise InventoryError("Invalid or excessive Mach-O fat slice count")
    entry_size = 32 if is64 else 20
    entries = read_exact(stream, 8, count * entry_size, total)
    slices, ranges = [], []
    for index in range(count):
        entry = entries[index * entry_size:(index + 1) * entry_size]
        if is64:
            cpu, subtype, start, size, _alignment, _reserved = struct.unpack(endian + "IIQQII", entry)
        else:
            cpu, subtype, start, size, _alignment = struct.unpack(endian + "IIIII", entry)
        if start < 8 + count * entry_size or size < 28 or start + size > total:
            raise InventoryError("Invalid Mach-O fat slice range")
        if any(start < end and begin < start + size for begin, end in ranges):
            raise InventoryError("Overlapping Mach-O fat slices")
        ranges.append((start, start + size))
        parsed = parse_slice(stream, start, size, total)
        if parsed["cpu_type"] != cpu or parsed["cpu_subtype"] != subtype:
            raise InventoryError("Mach-O fat table and slice architecture disagree")
        slices.append(parsed)
    return {"format": "universal", "slices": slices}


def contained_file(bundle, relative):
    try:
        resolved = (bundle / relative).resolve(strict=True)
        resolved.relative_to(bundle)
        if not resolved.is_file():
            raise ValueError()
    except (OSError, ValueError, RuntimeError):
        raise InventoryError("Required bundle file is absent or resolves outside the selected bundle") from None
    return resolved


def signature_metadata(bundle):
    if platform.system() != "Darwin":
        return {"status": "not_macos", "verification_performed": False, "trust_assessed": False}
    result = run_tool(["/usr/bin/codesign", "--display", "--verbose=4", str(bundle)])
    raw = result.get("stdout", "") + "\n" + result.get("stderr", "")
    fields = {}
    # Certificate subject names, team IDs and Executable= paths are deliberately
    # omitted. A display command never establishes authenticity or trust.
    for key in ("Format", "CodeDirectory", "Signature", "CDHash"):
        delimiter = r"[ =]" if key == "CodeDirectory" else "="
        match = re.search(r"^" + key + delimiter + r"(.*)$", raw, re.MULTILINE)
        if match:
            fields[key] = clean_text(match.group(1))
    return {"status": result["status"], "return_code": result.get("return_code"),
            "tool": "codesign --display --verbose=4", "display_fields": fields,
            "verification_performed": False, "trust_assessed": False}


def app_inventory(app):
    try:
        bundle = Path(app).expanduser().resolve(strict=True)
        if bundle.suffix.lower() != ".app" or not bundle.is_dir():
            raise ValueError()
    except (OSError, ValueError, RuntimeError):
        raise InventoryError("Select an existing .app directory") from None
    plist_path = contained_file(bundle, "Contents/Info.plist")
    try:
        with plist_path.open("rb") as stream:
            plist_bytes = stream.read(MAX_PLIST_BYTES + 1)
        if len(plist_bytes) > MAX_PLIST_BYTES:
            raise InventoryError("Info.plist exceeds inventory size limit")
        info = plistlib.loads(plist_bytes)
        if not isinstance(info, dict):
            raise ValueError()
    except (OSError, ValueError, plistlib.InvalidFileException, ExpatError, OverflowError, RecursionError):
        raise InventoryError("Bundle Info.plist is unreadable or invalid") from None
    name = info.get("CFBundleExecutable")
    if (not isinstance(name, str) or not name or name in (".", "..")
            or "/" in name or "\\" in name or "\0" in name):
        raise InventoryError("CFBundleExecutable must name one file inside Contents/MacOS")
    executable = contained_file(bundle, Path("Contents/MacOS") / name)
    try:
        with executable.open("rb") as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode):
                raise InventoryError("Main executable is not a regular file")
            digest = hashlib.sha256()
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
            macho = parse_macho(stream, before.st_size)
            after = os.fstat(stream.fileno())
            if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
                raise InventoryError("Main executable changed during inventory")
    except OSError:
        raise InventoryError("Main executable could not be read") from None
    metadata = {key: clean_text(info.get(key)) for key in (
        "CFBundleName", "CFBundleDisplayName", "CFBundleIdentifier",
        "CFBundleShortVersionString", "CFBundleVersion", "LSMinimumSystemVersion",
        "NSMainNibFile", "NSMainStoryboardFile",
    ) if isinstance(info.get(key), str)}
    return {
        "status": "inventoried", "bundle_name": clean_text(bundle.name),
        "source_path": "<selected-app>", "metadata": metadata,
        "original_source_os": "not_established_by_local_inspection",
        "info_plist_sha256": hashlib.sha256(plist_bytes).hexdigest(),
        "main_executable": {"relative_path": "Contents/MacOS/" + clean_text(name),
                            "sha256": digest.hexdigest(), **macho},
        "signature_metadata": signature_metadata(bundle),
        "coverage": {
            "main_executable_and_info_plist": True,
            "transitive_dependency_graph": False,
            "embedded_code_and_resource_manifest": False,
            "runtime_loaded_libraries": False,
            "bundle_integrity_verified": False,
            "application_launched": False,
        },
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", help="One local .app to inspect statically; never executed")
    parser.add_argument("--host-role", choices=("unspecified", "physical-target", "source-oracle", "development"),
                        default="unspecified", help="Distinguish target hardware from an app-source or development Mac")
    parser.add_argument("--no-hardware", action="store_true", help="Skip macOS hardware commands")
    args = parser.parse_args(argv)
    report = {"schema_version": 1, "collector_version": COLLECTOR_VERSION,
              "host_role": args.host_role,
              "collected_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "purpose": "Burdettie OS M1 target inventory; no support or trust assertion",
              "hardware": {"status": "not_requested"} if args.no_hardware else hardware_inventory(),
              "application": {"status": "not_requested"}}
    exit_code = 0
    if args.app:
        try:
            report["application"] = app_inventory(args.app)
        except InventoryError as error:
            report["application"] = {"status": "failed", "reason": str(error)}
            exit_code = 2
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
