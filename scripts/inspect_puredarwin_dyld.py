#!/usr/bin/env python3
"""Bounded static inspection of the selected PureDarwin x86_64 dyld executable.

This tool never executes the input or invokes a compiler, linker, loader, Nix,
or a subprocess. A pass is structural/static evidence only.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import sys
import re
from m1_evidence import EvidenceError, read_regular, publish, load_json, exact_keys, require

SOURCE_REVISION = "0163b2db225b85e3bf96fc25646ea290105ac298"
INSTALL_NAME = "/usr/lib/dyld"
MAX_INPUT = 128 * 1024 * 1024
MAX_JSON = 512 * 1024
MH_DYLINKER = 7
CPU_TYPE_X86_64 = 0x01000007
CPU_SUBTYPE_X86_64_ALL = 3
LC_UNIXTHREAD = 0x5
LC_ID_DYLINKER = 0xF
LC_BUILD_VERSION = 0x32
LC_RPATH = 0x8000001C
LC_LOAD_DYLINKER = 0xE
LC_MAIN = 0x80000028
LC_LOAD_DYLIB = 0xC
LC_REEXPORT_DYLIB = 0x8000001F
LC_LOAD_WEAK_DYLIB = 0x80000018
LC_LOAD_UPWARD_DYLIB = 0x80000023
LC_LAZY_LOAD_DYLIB = 0x20
LC_CODE_SIGNATURE = 0x1D
LC_UUID = 0x1B
LC_SYMTAB = 0x2
LC_DYSYMTAB = 0xB
LC_FUNCTION_STARTS = 0x26
LC_DATA_IN_CODE = 0x29
LC_SOURCE_VERSION = 0x2A
LC_DYLD_INFO_ONLY = 0x80000022
LC_DYLD_CHAINED_FIXUPS = 0x80000034
LC_DYLD_EXPORTS_TRIE = 0x80000033
LC_NOTE = 0x31
PLATFORM_MACOS = 1
VM_PROT_READ, VM_PROT_WRITE, VM_PROT_EXECUTE = 1, 2, 4

InspectionError = EvidenceError

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def cstring_region(data: bytes, offset: int, minimum: int) -> str:
    if offset < minimum or offset >= len(data):
        raise InspectionError("load-command string offset is out of bounds")
    end = data.find(b"\0", offset)
    if end < 0 or end - offset > 1024 or any(data[minimum:offset]) or any(data[end + 1:]):
        raise InspectionError("load-command string is unterminated or has nonzero trailing padding")
    raw = data[offset:end]
    if not raw or any(b < 0x20 or b > 0x7e for b in raw):
        raise InspectionError("load-command string is empty or contains unsupported bytes")
    return raw.decode("ascii")

def parse(data: bytes, expected: dict) -> dict:
    if len(data) > MAX_INPUT:
        raise InspectionError("dyld input exceeds size bound")
    if len(data) < 32:
        raise InspectionError("truncated mach_header_64")
    magic, cputype, cpusubtype, filetype, ncmds, sizeofcmds, flags, reserved = struct.unpack_from("<IIIIIIII", data)
    if magic != 0xFEEDFACF:
        raise InspectionError("expected little-endian 64-bit Mach-O")
    if cputype != CPU_TYPE_X86_64 or (cpusubtype & 0xffffffff) not in (CPU_SUBTYPE_X86_64_ALL, 0x80000003):
        raise InspectionError("unexpected CPU type or subtype")
    if filetype != MH_DYLINKER:
        raise InspectionError("expected MH_DYLINKER, not an executable or dylib")
    if reserved != 0 or ncmds == 0 or ncmds > 4096 or sizeofcmds > 4 * 1024 * 1024:
        raise InspectionError("invalid header reserved field or load-command bounds")
    end_commands = 32 + sizeofcmds
    if end_commands > len(data):
        raise InspectionError("load-command region exceeds file")
    offset = 32
    ids, builds, threads, segments = [], [], [], []
    forbidden = []
    seen_singletons = set()
    linkedit_ranges = []
    for _ in range(ncmds):
        if offset + 8 > end_commands:
            raise InspectionError("truncated load command")
        cmd, cmdsize = struct.unpack_from("<II", data, offset)
        if cmdsize < 8 or cmdsize % 8 or offset + cmdsize > end_commands:
            raise InspectionError("invalid load-command size/alignment")
        raw = data[offset:offset + cmdsize]
        if cmd in (LC_SYMTAB, LC_DYSYMTAB, LC_DYLD_INFO_ONLY, LC_CODE_SIGNATURE,
                   LC_FUNCTION_STARTS, LC_DATA_IN_CODE, LC_DYLD_CHAINED_FIXUPS,
                   LC_DYLD_EXPORTS_TRIE):
            if cmd in seen_singletons:
                raise InspectionError("duplicate singleton metadata command")
            seen_singletons.add(cmd)
        if cmd == LC_ID_DYLINKER:
            if cmdsize < 12: raise InspectionError("truncated LC_ID_DYLINKER")
            nameoff = struct.unpack_from("<I", raw, 8)[0]
            ids.append(cstring_region(raw, nameoff, 12))
        elif cmd == LC_BUILD_VERSION:
            if cmdsize < 24: raise InspectionError("truncated LC_BUILD_VERSION")
            platform_id, minos, sdk, ntools = struct.unpack_from("<IIII", raw, 8)
            if 24 + ntools * 8 != cmdsize: raise InspectionError("invalid build-tool array bounds")
            tools = []
            for i in range(ntools): tools.append(struct.unpack_from("<II", raw, 24 + i * 8))
            builds.append({"platform_id": platform_id, "minimum_os": version(minos), "sdk": version(sdk), "tools": tools})
        elif cmd == LC_UNIXTHREAD:
            if cmdsize < 16: raise InspectionError("truncated LC_UNIXTHREAD")
            flavor, count = struct.unpack_from("<II", raw, 8)
            if flavor != 4 or count != 42 or 16 + count * 4 != cmdsize:
                raise InspectionError("unsupported x86_64 thread-state layout")
            rip = struct.unpack_from("<Q", raw, 16 + 16 * 8)[0]
            threads.append(rip)
        elif cmd in (LC_LOAD_DYLINKER, LC_RPATH, LC_MAIN, LC_LOAD_DYLIB,
                     LC_REEXPORT_DYLIB, LC_LOAD_WEAK_DYLIB, LC_LOAD_UPWARD_DYLIB,
                     LC_LAZY_LOAD_DYLIB):
            forbidden.append(cmd)
        elif cmd in (LC_UUID, LC_SOURCE_VERSION):
            required = 24 if cmd == LC_UUID else 16
            if cmdsize != required: raise InspectionError("invalid fixed-size metadata command")
            if cmd in seen_singletons: raise InspectionError("duplicate singleton metadata command")
            seen_singletons.add(cmd)
        elif cmd == LC_SYMTAB:
            if cmdsize != 24: raise InspectionError("invalid LC_SYMTAB size")
            symoff, nsyms, stroff, strsize = struct.unpack_from("<IIII", raw, 8)
            if (nsyms and symoff < end_commands) or (strsize and stroff < end_commands):
                raise InspectionError("symbol/string table overlaps header/load commands")
            if symoff + nsyms * 16 > len(data) or stroff + strsize > len(data):
                raise InspectionError("symbol/string table range exceeds file")
        elif cmd == LC_DYSYMTAB:
            if cmdsize != 80: raise InspectionError("invalid LC_DYSYMTAB size")
            fields = struct.unpack_from("<18I", raw, 8)
            # Validate the six table offset/count pairs; indirect-symbol entries
            # are 4 bytes and relocation entries are 8 bytes.
            for index, width in ((6, 8), (8, 56), (10, 4), (12, 4), (14, 8), (16, 8)):
                start, count = fields[index], fields[index + 1]
                if start + count * width > len(data): raise InspectionError("dynamic symbol table range exceeds file")
        elif cmd in (LC_CODE_SIGNATURE, LC_FUNCTION_STARTS, LC_DATA_IN_CODE, LC_DYLD_CHAINED_FIXUPS, LC_DYLD_EXPORTS_TRIE):
            if cmdsize != 16: raise InspectionError("invalid linkedit-data command size")
            dataoff, datasize = struct.unpack_from("<II", raw, 8)
            if datasize and (dataoff < end_commands or dataoff + datasize > len(data)):
                raise InspectionError("linkedit-data range exceeds file")
            linkedit_ranges.append((dataoff, dataoff + datasize, cmd))
        elif cmd == LC_DYLD_INFO_ONLY:
            if cmdsize != 48: raise InspectionError("invalid LC_DYLD_INFO_ONLY size")
            pairs = struct.unpack_from("<10I", raw, 8)
            for i in range(0, 10, 2):
                start, size = pairs[i], pairs[i + 1]
                if size and (start < end_commands or start + size > len(data)):
                    raise InspectionError("dyld-info payload range exceeds file")
                if size: linkedit_ranges.append((start, start + size, cmd))
        elif cmd == LC_NOTE:
            if cmdsize != 40: raise InspectionError("invalid LC_NOTE size")
            noteoff, notesize = struct.unpack_from("<QQ", raw, 24)
            if notesize and (noteoff < end_commands or noteoff + notesize > len(data)):
                raise InspectionError("LC_NOTE payload range exceeds file")
        elif cmd not in (LC_ID_DYLINKER, LC_BUILD_VERSION, LC_UNIXTHREAD, 0x19):
            raise InspectionError(f"unsupported load command 0x{cmd:08x}")
        elif cmd == 0x19:  # LC_SEGMENT_64
            if cmdsize < 72: raise InspectionError("truncated LC_SEGMENT_64")
            segname = raw[8:24].split(b"\0", 1)[0].decode("ascii", "strict")
            vmaddr, vmsize, fileoff, filesize = struct.unpack_from("<QQQQ", raw, 24)
            maxprot, initprot, nsects, segflags = struct.unpack_from("<IIII", raw, 56)
            if 72 + nsects * 80 != cmdsize: raise InspectionError("invalid section table bounds")
            if filesize > vmsize:
                raise InspectionError("segment file mapping is larger than VM mapping")
            if fileoff + filesize > len(data) or vmaddr + vmsize > 0xFFFFFFFFFFFFFFFF:
                raise InspectionError("segment range overflows input or address space")
            if initprot & ~(VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE) or maxprot & ~(VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE) or initprot & ~maxprot:
                raise InspectionError("unsupported segment protection flags")
            if segflags != 0:
                raise InspectionError("unsupported segment flags")
            for si in range(nsects):
                so = 72 + si * 80
                sectname = raw[so:so+16].split(b"\0",1)[0]
                section_seg = raw[so+16:so+32].split(b"\0",1)[0]
                addr, size, fileoff_s, align, reloff, nreloc, secflags, reserved1, reserved2, reserved3 = struct.unpack_from("<QQIIIIIIII", raw, so+32)
                if reserved1 or reserved2 or reserved3: raise InspectionError("nonzero reserved section fields")
                if section_seg.decode("ascii", "strict") != segname:
                    raise InspectionError("section segment name disagrees with parent")
                if align > 31 or addr < vmaddr or addr + size > vmaddr + vmsize:
                    raise InspectionError("section VM range/alignment invalid")
                stype = secflags & 0xff
                zerofill = stype in (1, 0xC, 0x12)
                if zerofill:
                    if fileoff_s != 0: raise InspectionError("zero-fill section has a file offset")
                elif size and (fileoff_s < fileoff or fileoff_s + size > fileoff + filesize or addr - vmaddr != fileoff_s -fileoff):
                    raise InspectionError("file-backed section mapping is inconsistent")
                if nreloc and (reloff < 32 or reloff + nreloc * 8 > len(data)):
                    raise InspectionError("section relocation range is invalid")
            segments.append({"name": segname, "vmaddr": vmaddr, "vmsize": vmsize, "fileoff": fileoff, "filesize": filesize, "initprot": initprot, "nsects": nsects})
        offset += cmdsize
    if offset != end_commands:
        raise InspectionError("load-command count does not consume declared region")
    if ids != [INSTALL_NAME]: raise InspectionError("expected exactly one /usr/lib/dyld LC_ID_DYLINKER")
    if len(builds) != 1: raise InspectionError("expected exactly one LC_BUILD_VERSION")
    build = builds[0]
    if build["platform_id"] != PLATFORM_MACOS or build["minimum_os"] != expected["minimum_os"] or build["sdk"] != expected["sdk_version"]:
        raise InspectionError("platform, minimum OS or SDK differs from reviewed expectation")
    if forbidden: raise InspectionError("dyld executable contains an unsupported loader/rpath/entry command")
    if len(threads) != 1 or threads[0] == 0: raise InspectionError("expected exactly one nonzero x86_64 LC_UNIXTHREAD entry point")
    entry = threads[0]
    executable = [s for s in segments if s["initprot"] & VM_PROT_EXECUTE and s["vmaddr"] <= entry < s["vmaddr"] + s["vmsize"]]
    if len(executable) != 1: raise InspectionError("thread entry point is not in exactly one executable segment")
    entry_segment = executable[0]
    if entry_segment["filesize"] == 0 or entry >= entry_segment["vmaddr"] + entry_segment["filesize"]:
        raise InspectionError("thread entry point is not backed by file bytes")
    entry_file_offset = entry_segment["fileoff"] + entry - entry_segment["vmaddr"]
    if entry_file_offset < end_commands:
        raise InspectionError("thread entry point overlaps header/load commands")
    for i, left in enumerate(segments):
        for right in segments[i + 1:]:
            if left["filesize"] and right["filesize"] and max(left["fileoff"], right["fileoff"]) < min(left["fileoff"] + left["filesize"], right["fileoff"] + right["filesize"]):
                raise InspectionError("file-backed segments overlap")
            if left["vmsize"] and right["vmsize"] and max(left["vmaddr"], right["vmaddr"]) < min(left["vmaddr"] + left["vmsize"], right["vmaddr"] + right["vmsize"]):
                raise InspectionError("VM segments overlap")
    for i, left in enumerate(linkedit_ranges):
        for right in linkedit_ranges[i + 1:]:
            if max(left[0], right[0]) < min(left[1], right[1]):
                raise InspectionError("linkedit payload ranges overlap")
    if not any(s["name"] == "__TEXT" and s["fileoff"] == 0 and s["filesize"] >= end_commands and s["initprot"] & VM_PROT_READ and s["initprot"] & VM_PROT_EXECUTE for s in segments):
        raise InspectionError("missing file-backed executable __TEXT segment")
    return {"file_type": "MH_DYLINKER", "architecture": "x86_64", "install_name": ids[0], "build_version": build, "entry_point": entry, "entry_file_offset": entry_file_offset, "entry_segment": entry_segment, "segments": segments}

def version(value: int) -> str:
    return f"{value >> 16}.{(value >> 8) & 0xff}.{value & 0xff}"

def load_expectations(path: Path, digest: str) -> dict:
    raw = read_regular(path, MAX_JSON)
    if sha256(raw) != digest: raise InspectionError("expectation SHA-256 mismatch")
    value = load_json(raw, MAX_JSON)
    exact_keys(value, {"schema_version", "scope", "source_revision", "install_name", "minimum_os", "sdk_version"}, "dyld expectations")
    if type(value.get("schema_version")) is not int or value["schema_version"] != 1 or value.get("scope") != "m1_dyld_executable_expectations" or value.get("source_revision") != SOURCE_REVISION:
        raise InspectionError("expectation schema, scope or source revision mismatch")
    if value.get("minimum_os") != "26.5.0": raise InspectionError("minimum OS differs from selected target")
    if value.get("install_name") != INSTALL_NAME: raise InspectionError("unexpected dyld install name")
    for key in ("minimum_os", "sdk_version"):
        if not isinstance(value.get(key), str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value[key]): raise InspectionError("missing reviewed " + key)
    return value

def inspect(input_path: Path, expected_hash: str, expectation_path: Path, expectation_hash: str) -> dict:
    expectations = load_expectations(expectation_path, expectation_hash)
    raw = read_regular(input_path, MAX_INPUT)
    actual = sha256(raw)
    if actual != expected_hash: raise InspectionError("dyld executable SHA-256 mismatch")
    metadata = parse(raw, expectations)
    if sha256(read_regular(expectation_path, MAX_JSON)) != expectation_hash: raise InspectionError("expectations changed after inspection")
    if sha256(read_regular(input_path, MAX_INPUT)) != actual: raise InspectionError("dyld executable changed after inspection")
    return {"schema_version": 1, "scope": "m1_dyld_executable_static_inspection", "status": "pass", "source_revision": SOURCE_REVISION, "input_sha256": actual, "expectations_sha256": expectation_hash, "metadata": metadata, "source_identity_verified": False, "build_provenance_verified": False, "signature_verified": False, "dyld_runtime_execution": "not_run", "guest_boot": "not_run", "burdettie_runtime_execution": "not_run", "security_enforcement": "not_run", "limitations": ["static Mach-O structure only", "no source, SDK, Nix derivation, store registration or producer provenance", "no code-signature trust, chained-fixup or symbol-binding validation", "no loader execution, guest boot, application workflow or security qualification"]}

def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--dyld", type=Path, required=True)
    p.add_argument("--dyld-sha256", required=True)
    p.add_argument("--expectations", type=Path, required=True)
    p.add_argument("--expectations-sha256", required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args(argv)
    result = {"schema_version": 1, "scope": "m1_dyld_executable_static_inspection", "status": "blocked", "dyld_runtime_execution": "not_run", "guest_boot": "not_run", "security_enforcement": "not_run"}
    try:
        if a.output.exists() or a.output.is_symlink(): raise InspectionError("output must be a new path")
        if not a.output.parent.is_dir(): raise InspectionError("output parent must already exist")
        result = inspect(a.dyld, a.dyld_sha256, a.expectations, a.expectations_sha256)
        encoded = json.dumps(result, sort_keys=True, indent=2).encode() + b"\n"
        publish(a.output, encoded)
    except (InspectionError, OSError, ValueError, KeyError, TypeError) as exc:
        result = {"schema_version": 1, "scope": "m1_dyld_executable_static_inspection", "status": "fail", "error": str(exc), "dyld_runtime_execution": "not_run", "guest_boot": "not_run", "security_enforcement": "not_run"}
        print(json.dumps(result, sort_keys=True, indent=2)); return 1
    print(json.dumps(result, sort_keys=True, indent=2)); return 0

if __name__ == "__main__": raise SystemExit(main())
