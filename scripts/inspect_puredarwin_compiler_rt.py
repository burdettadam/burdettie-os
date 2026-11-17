#!/usr/bin/env python3
"""Inspect compiler-rt archive snapshots without extraction or execution.

A v2 receipt binds the reviewed archive, complete ordinary member inventory and
external symbol classification. It is not source/Nix/link/runtime provenance.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import struct

from m1_evidence import (EvidenceError, digest, encoded, exact_keys, hash_value,
                         load_json, publish, read_regular, require, unique_object)

SOURCE_REVISION = "0163b2db225b85e3bf96fc25646ea290105ac298"
AR_MAGIC = b"!<arch>\n"
THIN_MAGIC = b"!<thin>\n"
MAX_ARCHIVE = 512 * 1024 * 1024
MAX_MEMBERS = 100_000
MAX_MEMBER = 64 * 1024 * 1024
MAX_JSON = 512 * 1024
MAX_RECEIPT = 32 * 1024 * 1024
CPU_X86_64 = 0x01000007
InspectionError = EvidenceError
INDEX_NAMES = {"__.SYMDEF", "__.SYMDEF SORTED", "__.SYMDEF_64", "__.SYMDEF_64 SORTED"}
LIMITATIONS = {
    "build_provenance_verified": False, "link_member_selection_verified": False,
    "source_archive_build_provenance": "not_verified_by_static_inspector",
    "target_triple_basis": "reviewed_expectation_not_architecture_proof",
    "symbol_inventory": "external_names_classified_not_link_resolution",
    "archive_index_validation": "not_performed",
    "relocations_verified": False, "linker_selected_members": "not_verified",
    "libsystem_output_qualification": "not_established", "guest_execution": "not_run",
    "security_enforcement": "not_run",
}


def member_name(name: str) -> None:
    require(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_+.-]{1,255}", name)
            and name not in (".", ".."), "unsupported or unsafe ordinary archive member name")


def parse_ar(data: bytes) -> list[dict]:
    require(len(data) <= MAX_ARCHIVE, "archive exceeds size limit")
    require(not data.startswith(THIN_MAGIC), "thin archives are not accepted")
    require(data.startswith(AR_MAGIC), "invalid ar archive magic")
    pos, count = len(AR_MAGIC), 0
    members, names_blob, seen, indices = [], None, set(), set()
    while pos < len(data):
        count += 1
        require(count <= MAX_MEMBERS, "archive header count exceeds limit")
        require(pos + 60 <= len(data), "truncated ar member header")
        header = data[pos:pos + 60]
        require(header[58:60] == b"`\n", "invalid ar header trailer")
        def number(start, end, label, blank=True):
            text = header[start:end].decode("ascii", "strict").strip()
            require((not text and blank) or re.fullmatch(r"[0-9]+", text), "invalid ar " + label)
            return int(text or "0")
        number(16, 28, "timestamp")
        number(28, 34, "uid")
        number(34, 40, "gid")
        mode = header[40:48].decode("ascii", "strict").strip()
        require(not mode or re.fullmatch(r"[0-7]+", mode), "invalid ar mode")
        numeric_mode = int(mode or "0", 8)
        require(numeric_mode & ~0o100777 == 0, "unsupported ar mode or special permission bits")
        size = number(48, 58, "member size", False)
        require(size <= MAX_MEMBER, "archive member exceeds size limit")
        start, end = pos + 60, pos + 60 + size
        require(end <= len(data), "ar member extends beyond archive")
        if size & 1:
            require(end < len(data) and data[end:end + 1] == b"\n", "invalid ar odd-byte padding")
        raw_name = header[:16].decode("ascii", "strict").strip()
        payload, name = data[start:end], raw_name.removesuffix("/")
        payload_offset = start
        if raw_name == "//":
            require(names_blob is None, "duplicate GNU archive name table")
            names_blob = payload
            pos = end + (size & 1)
            continue
        if raw_name.startswith("#1/"):
            length = raw_name[3:]
            require(re.fullmatch(r"[0-9]+", length), "invalid BSD extended member name length")
            length = int(length)
            require(0 < length <= min(4096, len(payload)), "BSD extended member name exceeds bounds")
            raw = payload[:length]
            name_bytes, _, padding = raw.partition(b"\0")
            require(not any(padding), "nonzero BSD member-name padding")
            name = name_bytes.decode("utf-8", "strict")
            payload = payload[length:]
            payload_offset += length
        elif re.fullmatch(r"/[0-9]+", raw_name):
            index = int(raw_name[1:])
            require(names_blob is not None and index < len(names_blob), "GNU long-name offset out of bounds")
            require(index == 0 or names_blob[index - 2:index] == b"/\n",
                    "GNU long-name offset is not an entry boundary")
            stop = names_blob.find(b"/\n", index)
            require(stop >= index, "unterminated GNU long member name")
            name = names_blob[index:stop].decode("utf-8", "strict")
        special = raw_name in ("/", "/SYM64/") or name in INDEX_NAMES
        if special:
            key = raw_name if raw_name in ("/", "/SYM64/") else name
            require(key not in indices, "duplicate archive symbol index")
            indices.add(key)
        else:
            member_name(name)
            require(name not in seen, f"duplicate archive member name: {name}")
            seen.add(name)
            record = inspect_member(name, payload, mode)
            record.update(archive_header_offset=pos, archive_payload_offset=payload_offset, stored_member_size=size)
            members.append(record)
        pos = end + (size & 1)
    require(pos == len(data) and members, "archive has no object members or invalid padding")
    return members


def inspect_member(name: str, data: bytes, mode: str) -> dict:
    require(len(data) >= 4, f"member too short: {name}")
    if data[:4] in (b"BC\xc0\xde", b"\xde\xc0\x17\x0b"):
        raise InspectionError(f"bitcode-only member unsupported for this qualification: {name}")
    require(data[:4] in (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf"),
            f"unrecognized/non-Mach-O archive member: {name}")
    require(len(data) >= 32, f"truncated Mach-O member: {name}")
    require(data[:4] == b"\xcf\xfa\xed\xfe", f"non-little-endian x86_64 Mach-O member: {name}")
    _, cpu, subtype, kind, ncmds, command_bytes, flags, reserved = struct.unpack_from("<8I", data)
    require(cpu == CPU_X86_64 and subtype in (3, 0x80000003), "unsupported x86_64 architecture/subtype")
    require(kind == 1, "archive member is not MH_OBJECT")
    end_commands = 32 + command_bytes
    require(reserved == 0 and ncmds <= 4096 and ncmds <= command_bytes // 8
            and command_bytes <= 4 * 1024 * 1024 and end_commands <= len(data),
            "invalid Mach-O header bounds")
    def span(offset, length, label):
        require(0 <= offset <= len(data) and 0 <= length <= len(data) - offset,
                label + " exceeds member bounds")
        require(not length or offset >= end_commands, label + " overlaps header/load commands")
    offset, symtab, sections, platforms, uninterpreted = 32, None, [], [], []
    for _ in range(ncmds):
        require(offset + 8 <= end_commands, "truncated Mach-O load command")
        cmd, size = struct.unpack_from("<II", data, offset)
        require(size >= 8 and size % 8 == 0 and offset + size <= end_commands,
                "invalid Mach-O load command bounds")
        if cmd == 2:
            require(size == 24 and symtab is None, "invalid duplicate LC_SYMTAB")
            symtab = struct.unpack_from("<4I", data, offset + 8)
        elif cmd == 0x19:
            require(size >= 72, "truncated LC_SEGMENT_64")
            vmaddr, vmsize, fileoff, filesize, _, _, nsects, _ = struct.unpack_from("<4Q4I", data, offset + 24)
            span(fileoff, filesize, "segment file range")
            require(vmaddr + vmsize <= 1 << 64 and size == 72 + 80 * nsects
                    and len(sections) + nsects <= 255, "invalid object section count or VM range")
            for i in range(nsects):
                section = offset + 72 + 80 * i
                addr, length, loc, alignment, rel, nrel, attributes = struct.unpack_from("<QQ5I", data, section + 32)
                require(alignment <= 63 and vmaddr <= addr <= vmaddr + vmsize
                        and length <= vmaddr + vmsize - addr, "section VM range/alignment invalid")
                if attributes & 255 not in (1, 12, 18):
                    span(loc, length, "section contents")
                    require(fileoff <= loc <= fileoff + filesize and length <= fileoff + filesize - loc,
                            "section contents outside segment")
                span(rel, nrel * 8, "section relocations")
                sections.append((addr, length))
        elif cmd == 0x32:
            require(size >= 24, "truncated LC_BUILD_VERSION")
            platform, minimum, sdk, tools = struct.unpack_from("<4I", data, offset + 8)
            require(tools <= 64 and size == 24 + 8 * tools, "invalid build-version tool records")
            platforms.append({"platform_id": platform, "minimum_os": version(minimum), "sdk": version(sdk)})
        else:
            uninterpreted.append(cmd)
        offset += size
    require(offset == end_commands, "load command size mismatch")
    defined, undefined, common, indirect, private = [], [], [], [], []
    if symtab is not None:
        symoff, count, stroff, strsize = symtab
        require(count <= 262144 and strsize <= 8 * 1024 * 1024, "LC_SYMTAB exceeds inspection bounds")
        span(symoff, count * 16, "symbol table")
        span(stroff, strsize, "string table")
        require(not count or symoff + count * 16 <= stroff or stroff + strsize <= symoff,
                "symbol/string tables overlap")
        strings = data[stroff:stroff + strsize]
        def symbol_string(index):
            if index == 0:
                return ""
            require(index < len(strings), "symbol string index out of bounds")
            end = strings.find(b"\0", index, min(len(strings), index + 8193))
            require(end >= index, "unterminated/oversized symbol name")
            return strings[index:end].decode("utf-8", "strict")
        for i in range(count):
            strx, ntype, section, desc, value = struct.unpack_from("<IBBHQ", data, symoff + i * 16)
            symbol = symbol_string(strx)
            if ntype & 0xE0:
                continue
            category = ntype & 0x0E
            require(category in (0, 2, 10, 14), "unsupported object symbol type")
            if category == 14:
                require(1 <= section <= len(sections), "symbol section ordinal invalid")
                address, length = sections[section - 1]
                require(address <= value <= address + length, "symbol value outside its section")
            else:
                require(section == 0, "non-section symbol has section ordinal")
            target = symbol_string(value) if category == 10 else None
            if not ntype & 1 or not symbol:
                continue
            if ntype & 0x10:
                private.append(symbol)
            if category == 0:
                (common if value else undefined).append(symbol)
            elif category == 10:
                require(target, "indirect symbol has no target name")
                indirect.append({"name": symbol, "target": target})
            else:
                defined.append(symbol)
    return {"name": name, "size": len(data), "mode": mode, "sha256": digest(data),
            "kind": "macho_object", "architecture": "x86_64", "cpu_subtype": subtype,
            "symbol_table_present": symtab is not None,
            "defined_symbols": sorted(set(defined)), "undefined_symbols": sorted(set(undefined)),
            "common_symbols": sorted(set(common)), "indirect_symbols": indirect,
            "private_external_symbols": sorted(set(private)), "platform_versions": platforms,
            "uninterpreted_load_commands": uninterpreted}


def version(value: int) -> str:
    return f"{value >> 16}.{(value >> 8) & 255}.{value & 255}"


def inspect_archive(raw: bytes, expectation_raw: bytes, expectation_hash: str) -> dict:
    require(digest(expectation_raw) == hash_value(expectation_hash), "expectation digest mismatch")
    exp = load_json(expectation_raw, MAX_JSON)
    exact_keys(exp, {"schema_version", "scope", "source_revision", "target_triple",
                     "archive_sha256", "member_names"}, "archive expectations")
    require(type(exp["schema_version"]) is int and exp["schema_version"] == 1
            and exp["scope"] == "m1_compiler_rt_expectations"
            and exp["source_revision"] == SOURCE_REVISION
            and exp["target_triple"] == "x86_64-apple-darwin20.4", "expectation scope/source/target mismatch")
    require(digest(raw) == hash_value(exp["archive_sha256"]), "archive digest differs from reviewed expectation")
    members = parse_ar(raw)
    names = exp["member_names"]
    require(isinstance(names, list) and all(isinstance(n, str) for n in names)
            and len(names) == len(set(names)) and sorted(names) == sorted(m["name"] for m in members),
            "member set differs from independently reviewed expectation")
    return {"schema_version": 2, "scope": "m1_compiler_rt_archive_static_inspection",
            "source_revision": SOURCE_REVISION, "status": "pass", **LIMITATIONS,
            "archive_sha256": digest(raw), "archive_size": len(raw), "expectations_sha256": digest(expectation_raw),
            "member_count": len(members), "members": members, "target_triple": exp["target_triple"],
            "member_architecture": "x86_64",
            "availability_related_members": [m["name"] for m in members if availability_member(m)],
            "availability_member_policy": "identified_only; selected link requires separate checking"}


def availability_member(member: dict) -> bool:
    needles = {"isPlatformVersionAtLeast", "isOSVersionAtLeast", "availability_version_check"}
    return bool(re.search(r"os_version_check|availability_version_check", member["name"], re.I)
                or any(s.lstrip("_") in needles for key in ("defined_symbols", "undefined_symbols")
                       for s in member[key]))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--expectations", type=Path, required=True)
    parser.add_argument("--expectations-sha256", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)
    result = {"schema_version": 2, "scope": "m1_compiler_rt_archive_static_inspection",
              "source_revision": SOURCE_REVISION, "status": "blocked", **LIMITATIONS}
    output = None
    try:
        require(not os.path.lexists(args.output_dir) and args.output_dir.parent.is_dir(),
                "output directory must be new with existing parent")
        require(args.output_dir.parent.resolve(strict=True) == args.output_dir.parent.absolute(),
                "output parent must be canonical")
        require(not args.output_dir.absolute().is_relative_to('/nix/store'), "evidence must remain outside the Nix store")
        args.output_dir.mkdir(mode=0o700)
        output = args.output_dir
        result["status"] = "fail"
        exp_raw = read_regular(args.expectations, MAX_JSON)
        raw = read_regular(args.archive, MAX_ARCHIVE)
        checked = inspect_archive(raw, exp_raw, args.expectations_sha256)
        require(digest(read_regular(args.archive, MAX_ARCHIVE)) == digest(raw)
                and read_regular(args.expectations, MAX_JSON) == exp_raw, "inputs changed during inspection")
        publish(output / "expectations.json", exp_raw)
        publish(output / "result.json", encoded(checked))
        result = checked
    except (OSError, ValueError, KeyError, TypeError, struct.error) as error:
        result.update(status="fail", error=str(error))
        if output is not None and not os.path.lexists(output / "result.json"):
            try:
                publish(output / "failure.json", encoded(result))
            except (OSError, ValueError):
                pass  # stdout/exit remain authoritative when storage itself fails.
        print(encoded(result).decode(), end="")
        return 1
    print(encoded(result).decode(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
