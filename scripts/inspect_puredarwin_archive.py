#!/usr/bin/env python3
"""Inspect static archive members, not build provenance or executable behavior.

No subprocess, extraction, network access or dependency realization. Receipts
contain private input paths. Pair them with the selected build/NAR evidence
before linking; this helper does not qualify a compiler-runtime build by itself.
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

MAX_ARCHIVE = 64 * 1024 * 1024
MAX_MEMBER = 16 * 1024 * 1024
MAX_MEMBERS = 8192
MAX_SYMBOLS = 65536
INDEX_NAMES = {"/", "/SYM64/", "__.SYMDEF", "__.SYMDEF SORTED",
               "__.SYMDEF_64", "__.SYMDEF_64 SORTED"}


class InspectionError(ValueError):
    """Unsupported or inconsistent inspected bytes; never a successful check."""


def require(ok, message):
    if not ok:
        raise InspectionError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def span(data, offset, size, label):
    require(0 <= offset <= len(data) and 0 <= size <= len(data) - offset,
            "Out-of-bounds " + label)
    return data[offset:offset + size]


def version(value):
    return f"{value >> 16}.{(value >> 8) & 255}.{value & 255}"


def version_arg(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value),
            "Expected minimum OS must be major.minor.patch")
    parts = tuple(map(int, value.split(".")))
    require(parts[0] <= 65535 and parts[1] <= 255 and parts[2] <= 255
            and value == ".".join(map(str, parts)), "Invalid minimum OS version")
    return value


def decimal(raw):
    field = raw.rstrip(b" ")
    require(bool(field) and field.isdigit(), "Invalid archive decimal field")
    return int(field)


def archive_members(data):
    """Read each occurrence by offset, including duplicate object names."""
    require(len(data) <= MAX_ARCHIVE and data[:8] == b"!<arch>\n",
            "Expected bounded regular ar archive; thin/fat archives are unsupported")
    offset, count, names = 8, 0, None
    while offset < len(data):
        count += 1
        require(count <= MAX_MEMBERS, "Archive member count exceeds bound")
        header_offset = offset
        header = span(data, offset, 60, "archive header")
        require(header[58:] == b"`\n", "Invalid archive header terminator")
        size = decimal(header[48:58])
        require(size <= MAX_MEMBER, "Archive member exceeds size bound")
        offset += 60
        body = span(data, offset, size, "archive member")
        name = header[:16].rstrip(b" ")
        if name.startswith(b"#1/"):
            length = decimal(name[3:])
            require(0 < length <= min(size, 1024), "Invalid BSD extended name length")
            name = body[:length].rstrip(b"\0")
            body = body[length:]
        elif name.startswith(b"/") and name[1:].isdigit():
            start = decimal(name[1:])
            require(names is not None and start < len(names)
                    and (start == 0 or names[start - 1:start] == b"\n"),
                    "Invalid GNU long-name table reference")
            end = names.find(b"/\n", start)
            require(end >= start, "Unterminated GNU long name")
            name = names[start:end]
        elif name not in (b"/", b"//", b"/SYM64/"):
            name = name.removesuffix(b"/")
        require(0 < len(name) <= 1024 and all(32 <= c < 127 for c in name),
                "Invalid archive member name")
        text = name.decode("ascii")
        if text == "//":
            require(names is None, "Duplicate GNU long-name table")
            names = body
        elif text not in INDEX_NAMES:
            require("/" not in text and "\\" not in text and text not in (".", ".."),
                    "Unsupported path-like archive member name")
        yield text, header_offset, body
        offset += size
        if size % 2:
            require(span(data, offset, 1, "archive padding") == b"\n", "Invalid archive padding")
            offset += 1


def inspect_object(data, symbol_limit=MAX_SYMBOLS):
    require(len(data) <= MAX_MEMBER, "Object exceeds size bound")
    header = struct.unpack("<8I", span(data, 0, 32, "Mach-O header"))
    magic, cpu, subtype, kind, ncmds, cmdbytes, flags, reserved = header
    require(magic == 0xFEEDFACF and cpu == 0x01000007 and subtype == 3 and kind == 1
            and reserved == 0, "Expected thin little-endian baseline x86_64 MH_OBJECT")
    commands = span(data, 32, cmdbytes, "load commands")
    require(ncmds <= 4096 and ncmds <= cmdbytes // 8, "Invalid load-command count")
    cursor, sections, versions, symbols = 0, [], [], None
    for _ in range(ncmds):
        cmd, size = struct.unpack("<II", span(commands, cursor, 8, "load command header"))
        require(size >= 8 and size % 8 == 0, "Invalid load-command size or alignment")
        payload = span(commands, cursor, size, "load command")
        if cmd == 0x19:  # LC_SEGMENT_64
            require(size >= 72, "Truncated segment command")
            fileoff, filesize = struct.unpack_from("<QQ", payload, 40)
            span(data, fileoff, filesize, "segment file range")
            nsects = struct.unpack_from("<I", payload, 64)[0]
            require(size == 72 + 80 * nsects and len(sections) + nsects <= 255,
                    "Invalid section count or segment size")
            for index in range(nsects):
                section = payload[72 + index * 80:152 + index * 80]
                addr, length, start, align, reloc, nreloc, attributes = struct.unpack_from("<QQ5I", section, 32)
                require(align <= 63, "Invalid section alignment")
                if attributes & 255 not in (1, 12, 18):  # zero-fill variants have no file payload
                    span(data, start, length, "section file range")
                span(data, reloc, nreloc * 8, "section relocations")
                sections.append((addr, length))
        elif cmd == 2:  # LC_SYMTAB
            require(size == 24 and symbols is None, "Invalid or duplicate symbol-table command")
            symbols = struct.unpack_from("<4I", payload, 8)
        elif cmd in (0x32, 0x24):
            if cmd == 0x32:
                require(size >= 24, "Truncated build-version command")
                platform, minimum, sdk, tools = struct.unpack_from("<4I", payload, 8)
                require(platform == 1 and tools <= 64 and size == 24 + tools * 8,
                        "Expected macOS platform and bounded build-version tools")
            else:
                require(size == 16, "Invalid macOS minimum-version command")
                minimum, sdk = struct.unpack_from("<II", payload, 8)
            versions.append({"command": "LC_BUILD_VERSION" if cmd == 0x32 else "LC_VERSION_MIN_MACOSX",
                             "minimum_os": version(minimum), "sdk": version(sdk)})
        elif cmd in (0x25, 0x2F, 0x30):
            raise InspectionError("Non-macOS minimum-version command")
        cursor += size
    require(cursor == cmdbytes and len(versions) <= 1, "Inconsistent commands or duplicate deployment metadata")
    definitions, hidden, undefined, count = [], [], [], 0
    if symbols is not None:
        symoff, count, stroff, strsize = symbols
        require(count <= symbol_limit, "Symbol count exceeds archive-wide bound")
        require((not count or symoff >= 32 + cmdbytes) and (not strsize or stroff >= 32 + cmdbytes),
                "Symbol data overlaps load commands")
        table = span(data, symoff, count * 16, "symbol table")
        strings = span(data, stroff, strsize, "string table")
        require(not count or (symoff + count * 16 <= stroff or stroff + strsize <= symoff),
                "Symbol and string tables overlap")
        for index in range(count):
            name_offset, typ, sect, desc, value = struct.unpack_from("<IBBHQ", table, index * 16)
            require(name_offset < len(strings), "Symbol name is outside the string table")
            end = strings.find(b"\0", name_offset)
            require(end >= 0 and end - name_offset <= 1024, "Unterminated or oversized symbol name")
            name = strings[name_offset:end].decode("utf-8", errors="strict")
            if typ & 0xE0:  # debug symbols are never builtin definitions
                continue
            if typ & 0x0E == 0x0E:
                require(1 <= sect <= len(sections), "Symbol section index is invalid")
                addr, length = sections[sect - 1]
                require(addr <= value <= addr + length, "Symbol value lies outside its section")
                if typ & 1:
                    require(bool(name), "External definition has an empty name")
                    record = {"name": name, "weak": bool(desc & 0x80)}
                    (hidden if typ & 0x10 else definitions).append(record)
            elif typ & 0x0E == 0 and typ & 1:
                undefined.append({"name": name, "common": value != 0})
    return {"architecture": "x86_64", "cpu_subtype": subtype, "file_type": "MH_OBJECT",
            "flags": flags, "deployment": versions[0] if versions else None,
            "symbol_table_present": symbols is not None, "symbol_count": count, "definitions": definitions,
            "private_external_definitions": hidden, "undefined_symbols": undefined}


def inspect_archive(data, required_symbols, expected_minimum_os):
    expected_minimum_os = version_arg(expected_minimum_os)
    require(isinstance(required_symbols, (list, tuple)) and 0 < len(required_symbols) <= 1024
            and all(isinstance(name, str) and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_.$]*", name)
                    and len(name) <= 256 for name in required_symbols)
            and len(set(required_symbols)) == len(required_symbols), "Required symbols must be explicit and unique")
    objects, metadata, found, absent, mismatched = [], [], set(), [], []
    remaining_symbols = MAX_SYMBOLS
    for name, offset, body in archive_members(data):
        record = {"name": name, "archive_offset": offset, "bytes": len(body), "sha256": digest(body)}
        if name in INDEX_NAMES or name == "//":
            metadata.append(record)
            continue
        try:
            record.update(inspect_object(body, remaining_symbols))
            remaining_symbols -= record["symbol_count"]
        except (InspectionError, UnicodeError) as error:
            raise InspectionError(f"Member at {offset} ({name}): {error}") from error
        objects.append(record)
        found.update(item["name"] for item in record["definitions"])
        if record["deployment"] is None:
            absent.append(offset)
        elif record["deployment"]["minimum_os"] != expected_minimum_os:
            mismatched.append(offset)
    require(bool(objects), "Archive contains no Mach-O objects")
    missing = sorted(set(required_symbols) - found)
    status = "fail" if missing or mismatched else ("blocked" if absent else "pass")
    return {"schema_version": 1, "scope": "m1_static_archive_member_inspection", "status": status,
            "archive_sha256": digest(data), "archive_bytes": len(data), "object_count": len(objects),
            "expected_minimum_os": expected_minimum_os, "required_symbols": list(required_symbols),
            "missing_symbols": missing, "members_without_version": absent, "members_with_wrong_version": mismatched,
            "objects": objects, "archive_metadata": metadata, "archive_index_semantics_checked": False,
            "build_provenance_verified": False, "selected_compiler_rt_qualified": False,
            "fixture_linking": "not_run", "guest_execution": "not_run", "security_enforcement": "not_run"}


def read_archive(path):
    # O_NONBLOCK prevents a substituted FIFO from hanging before fstat.
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= MAX_ARCHIVE,
                "Archive must be a bounded regular file, not a link or special file")
        raw = stream.read(MAX_ARCHIVE + 1)
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    require(len(raw) == before.st_size and all(getattr(before, f) == getattr(after, f) for f in fields),
            "Archive changed during the bounded read")
    return raw


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--expected-minimum-os", required=True)
    parser.add_argument("--required-symbol", required=True, action="append")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args(argv)
    report = {"schema_version": 1, "scope": "m1_static_archive_member_inspection", "status": "blocked",
              "fixture_linking": "not_run", "guest_execution": "not_run", "security_enforcement": "not_run"}
    output = None
    try:
        require(re.fullmatch(r"[0-9a-f]{64}", args.archive_sha256), "Expected archive SHA-256 is invalid")
        version_arg(args.expected_minimum_os)
        directory = args.output_dir.absolute()
        require(directory.parent.is_dir(), "Evidence directory parent must already exist")
        directory.mkdir(mode=0o700, exist_ok=False)
        output = directory
        report["status"] = "fail"
        data = read_archive(args.archive)
        require(digest(data) == args.archive_sha256, "Archive checksum differs from the selected input")
        report.update(inspect_archive(data, args.required_symbol, args.expected_minimum_os))
        report["archive_path"] = str(args.archive.absolute())
        report["inspector_sha256"] = digest(Path(__file__).read_bytes())
    except (OSError, ValueError, TypeError, struct.error) as error:
        report["error"] = str(error)
    payload = json.dumps(report, indent=2) + "\n"
    if output is not None:
        try:
            with (output / "result.json").open("x", encoding="utf-8") as stream:
                stream.write(payload)
        except OSError as error:
            report.update(status="fail", error="Could not retain result: " + str(error))
            payload = json.dumps(report, indent=2) + "\n"
    print(payload, end="")
    return {"pass": 0, "fail": 1, "blocked": 2}[report["status"]]


if __name__ == "__main__":
    sys.exit(main())
