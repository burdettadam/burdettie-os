#!/usr/bin/env python3
"""Inspect two selected-source dylib snapshots, never realize or execute them.

This is a static identity/dependency check, not build/NAR provenance, symbol
resolution, dyld-executable inspection, fixture linking or guest qualification.
All input hashes and dependency versions must come from independent review.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import struct
import sys

REVISION = "0163b2db225b85e3bf96fc25646ea290105ac298"
SCOPE = "m1_libsystem_dylib_static_inspection"
EXPECTATION_SCOPE = "m1_libsystem_dylib_expectations"
MINIMUM_OS = "26.5.0"
CURRENT_VERSION = "1292.100.5"
COMPATIBILITY_VERSION = "1.0.0"
IDS = {"libsystem": "/usr/lib/libSystem.B.dylib", "libdyld": "/usr/lib/system/libdyld.dylib"}
EDGES = {"libsystem": (IDS["libdyld"], "reexport"), "libdyld": (IDS["libsystem"], "required")}
MAX_FILE = 128 * 1024 * 1024
MAX_COMMAND_BYTES = 4 * 1024 * 1024
MAX_COMMANDS = 4096
MAX_SYMBOLS = 262144
MAX_JSON = 512 * 1024
LOAD_KINDS = {0xC: "required", 0x80000018: "weak", 0x8000001F: "reexport",
              0x20: "lazy", 0x80000023: "upward"}
LINKEDIT_COMMANDS = {0x1D, 0x1E, 0x26, 0x29, 0x2B, 0x2E, 0x80000033, 0x80000034}
# Supported ordinary-dylib flags from LLVM 21.1.8 MachO.h. Historical
# prebinding, cache/simulator images, alternate mapping/namespace modes and
# executable-only flags need separate interpretation and are not accepted.
HEADER_FLAGS = {"MH_NOUNDEFS": 0x1, "MH_DYLDLINK": 0x4, "MH_BINDATLOAD": 0x8,
                "MH_TWOLEVEL": 0x80, "MH_NOMULTIDEFS": 0x200,
                "MH_SUBSECTIONS_VIA_SYMBOLS": 0x2000, "MH_WEAK_DEFINES": 0x8000,
                "MH_BINDS_TO_WEAK": 0x10000, "MH_ROOT_SAFE": 0x40000,
                "MH_SETUID_SAFE": 0x80000, "MH_NO_REEXPORTED_DYLIBS": 0x100000,
                "MH_DEAD_STRIPPABLE_DYLIB": 0x400000, "MH_HAS_TLV_DESCRIPTORS": 0x800000,
                "MH_NO_HEAP_EXECUTION": 0x1000000, "MH_APP_EXTENSION_SAFE": 0x2000000}
SEGMENT_FLAGS = 0x4 | 0x10  # SG_NORELOC, SG_READ_ONLY; ordinary low-VM mapping.
SECTION_ATTRIBUTES = 0xFE000700  # Known S_ATTR_* user/system bits; not the type byte.
LAST_SECTION_TYPE = 0x16  # S_INIT_FUNC_OFFSETS; known types are contiguous from zero.
ZERO_FILL_TYPES = (1, 12, 18)
LIMITATIONS = {"source_identity_verified": False, "sdk_provenance_verified": False, "build_provenance_verified": False,
               "registered_output_verified": False, "symbol_binding_checked": False,
               "fixup_semantics_checked": False, "signature_verified": False,
               "selected_libsystem_qualified": False, "dyld_executable_inspection": "not_run",
               "fixture_linking": "not_run", "guest_execution": "not_run",
               "burdettie_runtime_execution": "not_run", "security_enforcement": "not_run"}


class InspectionError(ValueError):
    """An unmet static contract; never a successful inspection."""


def require(ok, message):
    if not ok:
        raise InspectionError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def hash_value(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value), "Invalid expected SHA-256")
    return value


def span(data, offset, size, label):
    require(0 <= offset <= len(data) and 0 <= size <= len(data) - offset, "Out-of-bounds " + label)
    return data[offset:offset + size]


def version(value):
    return f"{value >> 16}.{(value >> 8) & 255}.{value & 255}"


def version_tuple(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value),
            "Version must be an explicit major.minor.patch string")
    parts = tuple(map(int, value.split(".")))
    require(parts[0] <= 65535 and parts[1] <= 255 and parts[2] <= 255
            and value == ".".join(map(str, parts)), "Invalid version")
    return parts


def exact_keys(value, keys, label):
    require(isinstance(value, dict) and set(value) == set(keys), "Unexpected fields in " + label)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "Duplicate JSON key: " + key)
        result[key] = value
    return result


def read_regular(path, limit):
    # O_NONBLOCK makes even a substituted FIFO fail promptly at fstat.
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit,
                "Input must be a bounded regular file, not a symlink or special file")
        data = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    current = path.lstat()
    require(len(data) == before.st_size and all(getattr(before, f) == getattr(after, f)
            == getattr(current, f) for f in fields), "Input changed during the bounded read")
    return data


def read_pinned_json(path, expected_hash):
    hash_value(expected_hash)
    raw = read_regular(path, MAX_JSON)
    require(digest(raw) == expected_hash, "JSON checksum differs from the reviewed input")
    return json.loads(raw, object_pairs_hook=unique_object), raw


def validate_expectations(contract):
    exact_keys(contract, ("schema_version", "scope", "source_revision", "minimum_os", "sdk_version", "files"),
               "expectations")
    require(type(contract["schema_version"]) is int and contract["schema_version"] == 1
            and contract["scope"] == EXPECTATION_SCOPE, "Wrong expectation schema or scope")
    require(contract["source_revision"] == REVISION and contract["minimum_os"] == MINIMUM_OS,
            "Expectations differ from the selected source/target")
    version_tuple(contract["sdk_version"])
    exact_keys(contract["files"], IDS, "expected files")
    for name, record in contract["files"].items():
        exact_keys(record, ("sha256", "dependencies"), name)
        hash_value(record["sha256"])
        edges = record["dependencies"]
        require(isinstance(edges, list) and len(edges) == 1, "Expected exactly one source-selected dependency")
        edge = edges[0]
        exact_keys(edge, ("name", "load_kind", "current_version", "compatibility_version"), "dependency")
        require((edge["name"], edge["load_kind"]) == EDGES[name], "Dependency expectations change the selected graph")
        current, compatibility = version_tuple(edge["current_version"]), version_tuple(edge["compatibility_version"])
        require(compatibility <= current and compatibility <= version_tuple(CURRENT_VERSION),
                "Dependency version requirement cannot be satisfied by the selected peer")
    return contract


def command_string(payload, minimum):
    offset = struct.unpack_from("<I", payload, 8)[0]
    require(minimum <= offset < len(payload), "Invalid load-command string offset")
    end = payload.find(b"\0", offset)
    require(end > offset and end - offset <= 1024, "Unterminated, empty or oversized load-command string")
    require(not any(payload[minimum:offset]) and not any(payload[end + 1:]),
            "Load-command string gap and trailing padding must be zero")
    raw = payload[offset:end]
    require(all(32 <= value < 127 for value in raw), "Load-command path must be exact printable ASCII")
    return raw.decode("ascii")


def named_bytes(raw):
    value, _, padding = raw.partition(b"\0")
    require(value and all(32 <= c < 127 for c in value) and not any(padding), "Invalid segment/section name")
    return value.decode("ascii")


def file_table(data, offset, size, command_end, label):
    span(data, offset, size, label)
    require(not size or offset >= command_end, label + " overlaps load commands")


def inspect_dylib(data, name, contract):
    require(name in IDS and 0 < len(data) <= MAX_FILE, "Unexpected dylib role or size")
    header = struct.unpack("<8I", span(data, 0, 32, "Mach-O header"))
    magic, cpu, subtype, kind, count, command_bytes, flags, reserved = header
    require((magic, cpu, subtype, kind, reserved) == (0xFEEDFACF, 0x01000007, 3, 6, 0),
            "Expected thin little-endian baseline x86_64 MH_DYLIB")
    require(not flags & ~sum(HEADER_FLAGS.values()), "Unsupported Mach-O header flags")
    require(0 < count <= MAX_COMMANDS and command_bytes <= MAX_COMMAND_BYTES
            and count <= command_bytes // 8, "Invalid or excessive load-command region")
    commands = span(data, 32, command_bytes, "load commands")
    command_end = 32 + command_bytes
    cursor, identifiers, dependencies, deployments, segments, sections = 0, [], [], [], [], []
    symtab, dysymtab, signature = None, None, False
    seen = set()
    for _ in range(count):
        cmd, size = struct.unpack("<II", span(commands, cursor, 8, "load-command header"))
        require(size >= 8 and size % 8 == 0, "Invalid load-command size or alignment")
        payload = span(commands, cursor, size, "load command")
        if cmd == 0xD or cmd in LOAD_KINDS:
            require(size >= 24, "Truncated dylib command")
            _name_offset, _timestamp, current, compatibility = struct.unpack_from("<4I", payload, 8)
            record = {"name": command_string(payload, 24), "current_version": version(current),
                      "compatibility_version": version(compatibility)}
            if cmd == 0xD:
                identifiers.append(record)
            else:
                record["load_kind"] = LOAD_KINDS[cmd]
                dependencies.append(record)
        elif cmd == 0x32:
            require(size >= 24, "Truncated build-version command")
            platform, minimum, sdk, tools = struct.unpack_from("<4I", payload, 8)
            require(tools <= 64 and size == 24 + 8 * tools, "Invalid build-version tool records")
            deployments.append({"command": "LC_BUILD_VERSION", "platform_id": platform,
                                "minimum_os": version(minimum), "sdk": version(sdk)})
        elif cmd == 0x19:
            require(size >= 72, "Truncated segment command")
            segment_name = named_bytes(payload[8:24])
            vmaddr, vmsize, fileoff, filesize, maximum, initial, nsects, segment_flags = struct.unpack_from("<4Q4I", payload, 24)
            require(not segment_flags & ~SEGMENT_FLAGS,
                    "Unsupported segment flags: protected, high-VM and fixed-VM segments are not inspected")
            span(data, fileoff, filesize, "segment file range")
            require(vmaddr + vmsize <= 1 << 64 and filesize <= vmsize and maximum <= 7
                    and initial & maximum == initial, "Invalid segment address or protection range")
            require(size == 72 + 80 * nsects and len(sections) + nsects <= 255,
                    "Invalid section count or segment size")
            for index in range(nsects):
                section = payload[72 + index * 80:152 + index * 80]
                named_bytes(section[:16])
                require(named_bytes(section[16:32]) == segment_name, "Section belongs to a different segment")
                address, length, offset, align, reloc, nreloc, attributes = struct.unpack_from("<QQ5I", section, 32)
                section_type = attributes & 255
                require(section_type <= LAST_SECTION_TYPE and not attributes & ~(SECTION_ATTRIBUTES | 255),
                        "Unsupported section type or attribute flags")
                require(align <= 63 and vmaddr <= address <= vmaddr + vmsize
                        and length <= vmaddr + vmsize - address, "Invalid section address/alignment")
                if section_type not in ZERO_FILL_TYPES:
                    file_table(data, offset, length, command_end, "section file range")
                    require(fileoff <= offset <= fileoff + filesize and length <= fileoff + filesize - offset,
                            "Section lies outside its segment file range")
                    require(address - vmaddr == offset - fileoff,
                            "Section VM address and file offset disagree with the segment mapping")
                else:
                    require(offset == 0 and address >= vmaddr + filesize,
                            "Zero-fill section must occupy the ordinary segment's zero-filled tail")
                file_table(data, reloc, nreloc * 8, command_end, "section relocations")
                sections.append((address, length))
            segments.append({"name": segment_name, "vm_address": vmaddr, "vm_size": vmsize,
                             "file_offset": fileoff, "file_size": filesize, "maximum_protection": maximum,
                             "initial_protection": initial, "section_count": nsects, "flags": segment_flags})
        elif cmd == 2:
            require(size == 24 and symtab is None, "Invalid or duplicate symbol table")
            symtab = struct.unpack_from("<4I", payload, 8)
        elif cmd == 0xB:
            require(size == 80 and dysymtab is None, "Invalid or duplicate dynamic symbol table")
            dysymtab = struct.unpack_from("<18I", payload, 8)
        elif cmd in LINKEDIT_COMMANDS:
            require(size == 16 and cmd not in seen, "Invalid or duplicate link-edit command")
            offset, length = struct.unpack_from("<II", payload, 8)
            file_table(data, offset, length, command_end, "link-edit data")
            signature = signature or cmd == 0x1D
        elif cmd in (0x22, 0x80000022):
            require(size == 48 and not ({0x22, 0x80000022} & seen), "Invalid or duplicate dyld-info command")
            pairs = struct.unpack_from("<10I", payload, 8)
            for index in range(0, 10, 2):
                file_table(data, pairs[index], pairs[index + 1], command_end, "dyld-info data")
        elif cmd in (0x1B, 0x2A):
            require(size == (24 if cmd == 0x1B else 16) and cmd not in seen,
                    "Invalid or duplicate UUID/source-version command")
        else:
            # Includes rpaths, loader/environment injection, encryption, legacy
            # or foreign platform versions and unrecognized required commands.
            raise InspectionError(f"Unsupported load command 0x{cmd:x}")
        seen.add(cmd)
        cursor += size
    require(cursor == command_bytes, "Load-command count does not match command region")
    require(identifiers == [{"name": IDS[name], "current_version": CURRENT_VERSION,
                             "compatibility_version": COMPATIBILITY_VERSION}],
            "Wrong, missing or duplicate LC_ID_DYLIB identity/version")
    require(deployments == [{"command": "LC_BUILD_VERSION", "platform_id": 1,
                            "minimum_os": MINIMUM_OS, "sdk": contract["sdk_version"]}],
            "Wrong, absent or duplicate platform/deployment/SDK metadata")
    require(dependencies == contract["files"][name]["dependencies"],
            "Dylib dependencies differ from the reviewed selected-source graph")
    require(not flags & HEADER_FLAGS["MH_NO_REEXPORTED_DYLIBS"]
            or all(edge["load_kind"] != "reexport" for edge in dependencies),
            "Header denies reexports but a reexport dependency is present")
    require(segments and len({s["name"] for s in segments}) == len(segments), "Missing or duplicate segments")
    text = [s for s in segments if s["name"] == "__TEXT"]
    require(len(text) == 1 and text[0]["file_offset"] == 0 and text[0]["file_size"] >= command_end
            and text[0]["initial_protection"] & 5 == 5, "Missing mapped executable __TEXT header region")
    for start_key, size_key in (("file_offset", "file_size"), ("vm_address", "vm_size")):
        ranges = sorted((s[start_key], s[start_key] + s[size_key]) for s in segments if s[size_key])
        require(all(a[1] <= b[0] for a, b in zip(ranges, ranges[1:])), "Overlapping segment ranges")
    symbols = 0
    if symtab is not None:
        symoff, symbols, stroff, strsize = symtab
        require(symbols <= MAX_SYMBOLS, "Symbol count exceeds inspection bound")
        file_table(data, symoff, symbols * 16, command_end, "symbol table")
        file_table(data, stroff, strsize, command_end, "string table")
        require(not symbols or symoff + symbols * 16 <= stroff or stroff + strsize <= symoff,
                "Symbol and string tables overlap")
        strings = span(data, stroff, strsize, "string table")
        for index in range(symbols):
            string_offset, typ, section, _desc, value = struct.unpack_from("<IBBHQ", data, symoff + index * 16)
            require(string_offset < len(strings), "Symbol string offset is invalid")
            end = strings.find(b"\0", string_offset, min(len(strings), string_offset + 8193))
            require(end >= 0, "Unterminated or oversized symbol name")
            if not typ & 0xE0 and typ & 0x0E == 0x0E:
                require(1 <= section <= len(sections), "Invalid symbol section index")
                address, length = sections[section - 1]
                require(address <= value <= address + length, "Symbol lies outside its section")
    if dysymtab is not None:
        require(symtab is not None, "Dynamic symbol table has no symbol table")
        for index in (0, 2, 4):
            require(dysymtab[index] <= symbols and dysymtab[index + 1] <= symbols - dysymtab[index],
                    "Dynamic symbol range is invalid")
        for index, entry_size in ((6, 8), (8, 56), (10, 4), (12, 4), (14, 8), (16, 8)):
            file_table(data, dysymtab[index], dysymtab[index + 1] * entry_size, command_end, "dynamic symbol data")
    return {"architecture": "x86_64", "cpu_subtype": subtype, "file_type": "MH_DYLIB", "flags": flags,
            "install_identity": identifiers[0], "deployment": deployments[0], "dependencies": dependencies,
            "segments": segments, "section_count": len(sections), "symbol_count": symbols,
            "load_command_count": count, "load_command_bytes": command_bytes,
            "code_signature_command_present": signature, "rpaths": [], "encrypted": False}


def inspect_pair(blobs, contract):
    validate_expectations(contract)
    exact_keys(blobs, IDS, "dylib snapshots")
    files = {}
    for name, data in blobs.items():
        require(digest(data) == contract["files"][name]["sha256"], name + " checksum differs from the selected input")
        files[name] = {"sha256": digest(data), "size_bytes": len(data), "macho": inspect_dylib(data, name, contract)}
    # All permitted edges identify the other inspected bytes; no path search or
    # host library fallback is used. This does not resolve imported symbols.
    closure = [{"from": name, "install_name": EDGES[name][0],
                "to": next(peer for peer in IDS if IDS[peer] == EDGES[name][0])} for name in IDS]
    return {"files": files, "dependency_path_closure": closure}


def retain_bytes(path, raw):
    with path.open("xb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def publish_result(directory, payload):
    # A partial write/flush/close failure must not leave a final success receipt.
    partial = directory / "result.json.partial"
    retain_bytes(partial, payload.encode("utf-8"))
    os.link(partial, directory / "result.json")  # Exclusive atomic publication.


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--libsystem", required=True, type=Path)
    parser.add_argument("--libdyld", required=True, type=Path)
    parser.add_argument("--expectations", required=True, type=Path)
    parser.add_argument("--expectations-sha256", required=True)
    parser.add_argument("--previous-inspection", type=Path)
    parser.add_argument("--previous-inspection-sha256")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args(argv)
    if bool(args.previous_inspection) != bool(args.previous_inspection_sha256):
        parser.error("Previous inspection path and checksum must be supplied together")
    report = {"schema_version": 1, "scope": SCOPE, "status": "blocked", **LIMITATIONS}
    directory = None
    try:
        output = args.output_dir.absolute()
        require(output.parent.is_dir(), "Evidence directory parent must already exist")
        require(not output.parent.resolve().is_relative_to("/nix/store"), "Evidence must remain outside the Nix store")
        output.mkdir(mode=0o700, exist_ok=False)
        directory = output
        report["status"] = "fail"
        helper_hash = digest(read_regular(Path(__file__), MAX_FILE))
        contract, raw = read_pinned_json(args.expectations, args.expectations_sha256)
        validate_expectations(contract)
        retain_bytes(directory / "expectations.json", raw)
        report.update(source_revision=REVISION, expectations_sha256=digest(raw), inspector_sha256=helper_hash)
        paths = {name: getattr(args, name) for name in IDS}
        report["input_paths"] = {name: str(path.absolute()) for name, path in paths.items()}
        report["inspection"] = inspect_pair({name: read_regular(path, MAX_FILE) for name, path in paths.items()}, contract)
        if args.previous_inspection:
            previous, old_raw = read_pinned_json(args.previous_inspection, args.previous_inspection_sha256)
            retain_bytes(directory / "previous-inspection.json", old_raw)
            require(isinstance(previous, dict) and type(previous.get("schema_version")) is int
                    and previous.get("schema_version") == 1 and previous.get("scope") == SCOPE
                    and previous.get("status") == "pass", "Previous receipt is not a successful static inspection")
            for key in ("source_revision", "expectations_sha256", "inspector_sha256", "inspection", *LIMITATIONS):
                # JSON's distinct boolean/number types must not compare equal
                # merely because Python considers False == 0 or True == 1.
                require(json.dumps(previous.get(key), sort_keys=True) == json.dumps(report[key], sort_keys=True),
                        "Previous inspection differs: " + key)
            report["previous_inspection_sha256"] = digest(old_raw)
        for name, path in paths.items():
            require(digest(read_regular(path, MAX_FILE)) == report["inspection"]["files"][name]["sha256"],
                    "Dylib changed during inspection: " + name)
        require(digest(read_regular(args.expectations, MAX_JSON)) == report["expectations_sha256"],
                "Expectations changed during inspection")
        require(digest(read_regular(Path(__file__), MAX_FILE)) == helper_hash, "Inspector changed during inspection")
        report["status"] = "pass"
    except (OSError, ValueError, TypeError, KeyError, struct.error) as error:
        report["error"] = str(error)
    payload = json.dumps(report, indent=2) + "\n"
    if directory is not None:
        try:
            publish_result(directory, payload)
        except OSError as error:
            report.update(status="fail", error="Could not retain terminal result: " + str(error))
            payload = json.dumps(report, indent=2) + "\n"
    print(payload, end="")
    return {"pass": 0, "fail": 1, "blocked": 2}[report["status"]]


if __name__ == "__main__":
    sys.exit(main())
