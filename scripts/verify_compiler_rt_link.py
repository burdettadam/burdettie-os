#!/usr/bin/env python3
"""Cross-check retained compiler-rt archive, direct ld64 argv, map, and output.

This tool executes nothing. A v2 capture must come from a separately trusted
producer; hashing a capture is not authentication of its execution claims.
Copies of recovered artifacts are allowed, but map paths must match the original
capture, not another archive with the same basename. No library is qualified.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import struct

import inspect_puredarwin_compiler_rt as archive_inspector
from m1_evidence import (EvidenceError, digest, encoded, exact_keys, hash_value,
                         load_json, publish, read_regular, require)

SOURCE_REVISION = archive_inspector.SOURCE_REVISION
MAX = 16 * 1024 * 1024
MAX_ROWS = 100_000
MAX_OUTPUT = 256 * 1024 * 1024
LinkEvidenceError = EvidenceError
LIMITATIONS = {
    "build_provenance_verified": False, "capture_execution_authenticated": False,
    "libsystem_output_qualified": False, "symbol_resolution_verified": False,
    "guest_execution": "not_run", "security_enforcement": "not_run",
}

# This is an explicit subset of *expanded direct linker argv*. Driver syntax,
# file lists, response files and unknown options are rejected, never guessed.
ARITY = {
    "-o": 1, "-map": 1, "-arch": 1, "-platform_version": 3,
    "-install_name": 1, "-dylib_install_name": 1,
    "-compatibility_version": 1, "-current_version": 1,
    "-force_load": 1, "-reexport_library": 1, "-weak_library": 1,
    "-needed_library": 1, "-load_hidden": 1,
    "-L": 1, "-F": 1, "-syslibroot": 1, "-dylib_file": 1,
    "-u": 1, "-U": 1, "-undefined": 1,
    "-order_file": 1, "-exported_symbols_list": 1, "-unexported_symbols_list": 1,
    "-alias": 2, "-rpath": 1, "-e": 1,
}
FLAGS = {
    "-dylib", "-dynamic", "-Z", "-dead_strip", "-fixup_chains", "-data_const",
    "-no_uuid", "-no_adhoc_codesign", "-adhoc_codesign", "-twolevel_namespace",
    "-noall_load", "-no_deduplicate", "--dead-strip-duplicates", "-demangle",
    "-fatal_warnings", "-no_warn_inits", "-no_compact_unwind", "-warn_compact_unwind",
}
FORBIDDEN = {"-all_load", "--whole-archive", "-whole_archive", "-ObjC", "-filelist",
             "-Xlinker", "-Xclang", "-flat_namespace", "-dylinker"}
ALTERNATE_LOADS = {"-force_load", "-reexport_library", "-weak_library", "-needed_library", "-load_hidden"}


def canonical_path(value: object) -> str:
    require(isinstance(value, str) and value.startswith("/") and not value.startswith("//")
            and len(value) <= 8192 and not any(ord(c) < 32 or c == '\x7f' for c in value)
            and posixpath.normpath(value) == value, "capture path must be an absolute normalized path")
    return value


def argument_path(value: str, cwd: str) -> str:
    return posixpath.normpath(value if value.startswith("/") else posixpath.join(cwd, value))


def scan_argv(argv: list, cwd: str, archive_path: str, map_path: str, output_path: str) -> dict:
    require(isinstance(argv, list) and 1 < len(argv) <= 65536
            and all(isinstance(a, str) and 0 < len(a) <= 8192
                    and not any(ord(c) < 32 or c == '\x7f' for c in a) for a in argv),
            "invalid direct linker argument vector")
    canonical_path(argv[0])
    ordinary, options, other_force_loads = [], {}, 0
    i = 1
    while i < len(argv):
        arg = argv[i]
        require(not arg.startswith("@") and not arg.startswith("-Wl,"),
                "response files and compiler-driver argv are unsupported; capture expanded direct linker argv")
        require(arg not in FORBIDDEN, "unsupported hidden, global force-load or namespace option: " + arg)
        if arg in ARITY:
            n = ARITY[arg]
            require(i + n < len(argv), "missing linker option operand: " + arg)
            values = argv[i + 1:i + 1 + n]
            require(all(not v.startswith('@') for v in values), "response-file operand unsupported")
            options.setdefault(arg, []).append(values)
            if arg in ALTERNATE_LOADS:
                operand = argument_path(values[0], cwd)
                require(operand != archive_path, "compiler-rt cannot be force-loaded or used as an option operand")
                require(PurePosixPath(operand).name != PurePosixPath(archive_path).name,
                        "alternate compiler-rt archive input is ambiguous")
                if arg == "-force_load":
                    other_force_loads += 1
            if arg == "-undefined":
                require(values == ["error"], "blanket unresolved-symbol suppression is forbidden")
            i += n + 1
        elif arg in FLAGS:
            options.setdefault(arg, []).append([])
            i += 1
        elif arg.startswith(("-L", "-F", "-l")) and len(arg) > 2:
            # Search paths and library names are not ordinary archive arguments.
            require(not arg.startswith("-lcompiler_rt"), "compiler-rt must use its explicit ordinary archive path")
            i += 1
        elif arg.startswith("-"):
            raise LinkEvidenceError("unsupported direct linker option: " + arg)
        else:
            operand = argument_path(arg, cwd)
            if PurePosixPath(operand).name == PurePosixPath(archive_path).name:
                require(operand == archive_path, "alternate compiler-rt archive path in argv")
            if operand == archive_path:
                ordinary.append(i)
            i += 1
    require(len(ordinary) == 1, "compiler-rt archive must appear exactly once as a normal argv item")
    for opt, expected in (("-map", map_path), ("-o", output_path)):
        require(len(options.get(opt, [])) == 1 and argument_path(options[opt][0][0], cwd) == expected,
                "captured " + opt + " path is missing, duplicated or mismatched")
    require(options.get("-arch") == [["x86_64"]], "expected exactly one x86_64 linker architecture")
    require(options.get("-dylib") == [[]], "expected one dylib link, not a dyld or executable link")
    platform = options.get("-platform_version", [])
    require(len(platform) == 1 and platform[0][0] == "macos"
            and platform[0][1] in ("26.5", "26.5.0")
            and re.fullmatch(r"[0-9]+(?:\.[0-9]+){0,2}", platform[0][2]),
            "expected explicit selected macOS 26.5 platform with recorded SDK version")
    return {"ordinary_archive_argument_count": 1, "other_archives_force_loaded": other_force_loads,
            "platform_version_argv": platform[0]}


def parse_map(raw: bytes, *, cwd: str, archive_path: str, output_path: str) -> dict:
    require(0 < len(raw) <= MAX, "link map exceeds input bounds")
    text = raw.decode("utf-8", "strict")
    require('\0' not in text and not any(ord(c) < 32 and c not in '\n\r\t' for c in text),
            "link map contains control bytes")
    lines = text.splitlines()
    require(len(lines) <= MAX_ROWS and all(len(line) <= 16384 for line in lines), "link map exceeds line bounds")
    stage, files, selected, live, dead = 0, {}, {}, [], []
    for line in lines:
        if not line:
            continue
        if line.startswith("# Path: "):
            require(stage == 0 and argument_path(line[8:], cwd) == output_path,
                    "map output identity is wrong or duplicated")
            stage = 1
        elif line.startswith("# Arch: "):
            require(stage == 1 and line[8:] == "x86_64", "map architecture is wrong or duplicated")
            stage = 2
        elif line == "# Object files:":
            require(stage == 2, "duplicate or reordered object table")
            stage = 3
        elif line == "# Sections:":
            require(stage == 3, "missing, duplicate or reordered sections table")
            stage = 4
        elif line == "# Symbols:":
            require(stage == 4, "missing, duplicate or reordered symbols table")
            stage = 5
        elif line == "# Dead Stripped Symbols:":
            require(stage == 5, "duplicate or reordered dead-symbol table")
            stage = 6
        elif line.startswith("#"):
            # Only comments are ignored; archive names in them are never selections.
            require(stage >= 4, "unexpected map header")
        elif stage == 3:
            match = re.fullmatch(r"\[\s*([0-9]+)\]\s+(.+)", line)
            require(match, "malformed object-table row")
            index, name = int(match[1]), match[2]
            require(index <= MAX_ROWS and index not in files, "duplicate or oversized map file ordinal")
            files[index] = name
            if index == 0:
                require(name == "linker synthesized", "unexpected synthesized-object identity")
                continue
            member = re.fullmatch(r"(.+)\(([^()]+)\)", name)
            if member:
                path, item = argument_path(member[1], cwd), member[2]
                if PurePosixPath(path).name == PurePosixPath(archive_path).name:
                    require(path == archive_path, "map selected a different compiler-rt archive path")
                if path == archive_path:
                    archive_inspector.member_name(item)
                    require(item not in selected.values(), "duplicate compiler-rt member entries in linker map")
                    selected[index] = item
            elif PurePosixPath(argument_path(name, cwd)).name == PurePosixPath(archive_path).name:
                raise LinkEvidenceError("compiler-rt row lacks an unambiguous archive member")
        elif stage == 4:
            require(re.fullmatch(r"0x[0-9A-Fa-f]+\s+0x[0-9A-Fa-f]+\s+\S+\s+\S+", line),
                    "malformed section-map row")
        elif stage in (5, 6):
            pattern = (r"0x[0-9A-Fa-f]+\s+0x[0-9A-Fa-f]+\s+\[\s*([0-9]+)\]\s+(.+)"
                       if stage == 5 else r"<<dead>>\s+0x[0-9A-Fa-f]+\s+\[\s*([0-9]+)\]\s+(.+)")
            match = re.fullmatch(pattern, line)
            require(match and int(match[1]) in files, "malformed symbol row or unknown object ordinal")
            if int(match[1]) in selected:
                (live if stage == 5 else dead).append({"member": selected[int(match[1])], "name": match[2]})
        else:
            raise LinkEvidenceError("map data before required headers")
    require(stage in (5, 6) and files.get(0) == "linker synthesized", "incomplete ld64 map")
    require(selected, "linker map contains no compiler-rt member selection")
    return {"selected_members": list(selected.values()), "live_map_symbols": live,
            "dead_map_symbols": dead, "selection_basis": "object_table_not_final_symbol_resolution"}


def verify(capture_raw: bytes, map_raw: bytes, archive_raw: bytes, inspection_raw: bytes,
           output_raw: bytes, expected: dict) -> dict:
    inputs = {"capture": capture_raw, "map": map_raw, "archive": archive_raw,
              "inspection": inspection_raw, "output": output_raw}
    exact_keys(expected, set(inputs), "externally selected evidence hashes")
    for label, raw in inputs.items():
        require(digest(raw) == hash_value(expected[label]), label + " evidence digest mismatch")
    capture = load_json(capture_raw, MAX)
    exact_keys(capture, {"schema_version", "scope", "source_revision", "input_origin", "argv_kind", "argv",
                         "cwd", "exit_code", "timed_out", "archive", "link_map", "output"}, "link capture")
    require(type(capture["schema_version"]) is int and capture["schema_version"] == 2
            and capture["scope"] == "m1_compiler_rt_link_capture"
            and capture["source_revision"] == SOURCE_REVISION and capture["argv_kind"] == "ld64_direct"
            and capture["input_origin"] in ("owned_fixture", "selected_source_build"), "invalid link capture contract")
    require(type(capture["exit_code"]) is int and capture["exit_code"] == 0
            and capture["timed_out"] is False, "captured linker execution did not succeed")
    cwd = canonical_path(capture["cwd"])
    for label, source in (("archive", "archive"), ("link_map", "map"), ("output", "output")):
        item = capture[label]
        exact_keys(item, {"path", "sha256"}, "captured " + label)
        canonical_path(item["path"])
        require(hash_value(item["sha256"]) == expected[source], "captured " + label + " digest mismatch")
    arpath, mappath, outpath = (capture[label]["path"] for label in ("archive", "link_map", "output"))
    require(len({arpath, mappath, outpath}) == 3, "archive, map and output paths must be distinct")
    command = scan_argv(capture["argv"], cwd, arpath, mappath, outpath)
    inspection = load_json(inspection_raw, archive_inspector.MAX_RECEIPT)
    require(type(inspection.get("schema_version")) is int and inspection["schema_version"] == 2
            and inspection.get("scope") == "m1_compiler_rt_archive_static_inspection"
            and inspection.get("status") == "pass" and inspection.get("archive_sha256") == expected["archive"]
            and inspection.get("source_revision") == SOURCE_REVISION
            and inspection.get("target_triple") == "x86_64-apple-darwin20.4"
            and inspection.get("member_architecture") == "x86_64", "archive inspection receipt identity mismatch")
    fresh = archive_inspector.parse_ar(archive_raw)
    require(type(inspection.get("archive_size")) is int and inspection["archive_size"] == len(archive_raw)
            and type(inspection.get("member_count")) is int and inspection["member_count"] == len(fresh),
            "archive receipt size/count mismatch")
    require(encoded({"members": inspection.get("members")}) == encoded({"members": fresh}),
            "archive receipt inventory differs from freshly parsed archive bytes")
    for key, value in archive_inspector.LIMITATIONS.items():
        require(encoded({"value": inspection.get(key)}) == encoded({"value": value}),
                "archive receipt limitation mismatch: " + key)
    selection = parse_map(map_raw, cwd=cwd, archive_path=arpath, output_path=outpath)
    by_name = {m["name"]: m for m in fresh}
    for name in selection["selected_members"]:
        require(name in by_name, "linker map selects a member absent from the inspected archive inventory")
        require(not archive_inspector.availability_member(by_name[name]),
                "availability-check implementation was selected: " + name)
    return {"schema_version": 2, "scope": "m1_compiler_rt_link_selection_evidence", "status": "pass",
            "source_revision": SOURCE_REVISION, "input_origin": capture["input_origin"],
            "evidence_sha256": expected, "captured_exit_code": 0, "captured_timed_out": False,
            "archive_inventory_reparsed": True, "private_paths_in_capture_not_copied": True,
            "compiler_rt_force_loaded": False, "availability_checker_selected": False,
            **command, **selection, **LIMITATIONS}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ("argv-json", "map", "archive", "archive-inspection", "linked-output", "output"):
        parser.add_argument("--" + option, type=Path, required=True)
    for option in ("argv-sha256", "map-sha256", "archive-sha256", "archive-inspection-sha256", "linked-output-sha256"):
        parser.add_argument("--" + option, required=True)
    args = parser.parse_args(argv)
    result = {"schema_version": 2, "scope": "m1_compiler_rt_link_selection_evidence", "status": "fail", **LIMITATIONS}
    paths = {"capture": (args.argv_json, MAX), "map": (args.map, MAX),
             "archive": (args.archive, archive_inspector.MAX_ARCHIVE),
             "inspection": (args.archive_inspection, archive_inspector.MAX_RECEIPT),
             "output": (args.linked_output, MAX_OUTPUT)}
    expected = {"capture": args.argv_sha256, "map": args.map_sha256, "archive": args.archive_sha256,
                "inspection": args.archive_inspection_sha256, "output": args.linked_output_sha256}
    try:
        require(not os.path.lexists(args.output), "output must be a new path")
        raw = {label: read_regular(path, bound) for label, (path, bound) in paths.items()}
        checked = verify(raw["capture"], raw["map"], raw["archive"], raw["inspection"], raw["output"], expected)
        for label, (path, bound) in paths.items():
            require(digest(read_regular(path, bound)) == expected[label], "evidence changed after verification: " + label)
        publish(args.output, encoded(checked))
        result = checked
    except (OSError, ValueError, KeyError, TypeError, struct.error) as error:
        result.update(status="fail", error=str(error))
        print(encoded(result).decode(), end="")
        return 1
    print(encoded(result).decode(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
