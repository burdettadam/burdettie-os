#!/usr/bin/env python3
"""Inspect an already-realized pinned cross-toolchain; never run its wrappers.

The caller must first validate the selected source, cross plan, successful
native-linker/SDK receipts and helper hashes. This is installed-output evidence,
not a build driver or proof of compiler, target ABI, guest or security behavior.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat

import inspect_puredarwin_native as inspection


native = inspection.native
base = inspection.base
NIX = inspection.NIX
STORE = Path("/nix/store")
INSPECTION_SCOPE = "puredarwin_cross_toolchain_installed_wrappers"
TARGET = "x86_64-apple-darwin20.4"
TOOLS = tuple(TARGET + "-" + name for name in (
    "clang", "clang++", "ar", "ranlib", "strip", "nm", "objdump", "otool",
    "dsymutil", "install_name_tool", "lipo", "ld")) + ("xcrun", "dsymutil")
NATIVE_TOOLS = frozenset((TOOLS[0], TOOLS[1], TOOLS[11]))
MAX_JSON = 16 * 1024 * 1024
MAX_SCRIPT = 65536
MAX_DERIVATIONS = 128
MAX_CLOSURE = 2048
IDENTITY_KEYS = ("path", "deriver", "nar_hash", "nar_size")


def store_path(value, *, derivation=False):
    native.require(isinstance(value, str) and Path(value).parent == STORE
                   and re.fullmatch(r"[0-9abcdfghijklmnpqrsvwxyz]{32}-[A-Za-z0-9+._?=-]+", Path(value).name)
                   and str(Path(value)) == value and value.endswith(".drv") is derivation,
                   "Expected a canonical local store " + ("derivation" if derivation else "output") + " path")
    return value


def json_object(raw):
    native.require(len(raw) <= MAX_JSON, "Nix metadata exceeds the inspection size bound")

    def unique(pairs):
        result = {}
        for key, value in pairs:
            native.require(key not in result, "Duplicate field in Nix metadata")
            result[key] = value
        return result

    result = json.loads(raw.decode("utf-8"), object_pairs_hook=unique)
    native.require(isinstance(result, dict), "Nix metadata must be an object")
    return result


def canonical_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                     ensure_ascii=False).encode("utf-8")).hexdigest()


def inputs(record):
    """Read the pinned Nix 2.28 derivation shape, without dynamic outputs."""
    values = record.get("inputDrvs")
    native.require(isinstance(values, dict) and len(values) <= MAX_DERIVATIONS,
                   "Missing or oversized derivation input map")
    result = {}
    for drv, selected in values.items():
        store_path(drv, derivation=True)
        native.require(isinstance(selected, dict) and set(selected) == {"outputs", "dynamicOutputs"}
                       and selected["dynamicOutputs"] == {}, "Unsupported dynamic or unknown derivation inputs")
        outputs = selected["outputs"]
        native.require(isinstance(outputs, list) and 1 <= len(outputs) <= 32
                       and all(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_+-]+", name) for name in outputs)
                       and len(outputs) == len(set(outputs)), "Invalid selected derivation outputs")
        result[drv] = sorted(outputs)
    sources = record.get("inputSrcs")
    native.require(isinstance(sources, list) and len(sources) <= MAX_DERIVATIONS
                   and len(sources) == len(set(sources)), "Invalid derivation source inputs")
    for path in sources:
        store_path(path)
    return result


def declared_output(record, name):
    outputs = record.get("outputs")
    native.require(isinstance(outputs, dict) and name in outputs
                   and isinstance(outputs[name], dict) and set(outputs[name]) == {"path"},
                   "Expected an input-addressed declared output")
    return store_path(outputs[name]["path"])


def read_derivations(paths, name, directory, report):
    paths = sorted(set(paths))
    native.require(0 < len(paths) <= MAX_DERIVATIONS, "Derivation query exceeds the bounded selection")
    for path in paths:
        store_path(path, derivation=True)
    raw, _ = inspection.checked(NIX + ["derivation", "show", "--offline"] + paths, name, directory, report)
    values = json_object(raw)
    native.require(set(values) == set(paths), "Derivation query returned a different selection")
    for value in values.values():
        native.require(isinstance(value, dict), "Invalid derivation record")
        inputs(value)
    return values


def wrapper_specs(candidate_record, direct, candidate, native_registered):
    native.require(set(candidate_record.get("outputs", {})) == {"out"}
                   and declared_output(candidate_record, "out") == candidate["out"],
                   "Cross derivation does not produce the exact planned output")
    env = candidate_record.get("env", {})
    native.require(candidate_record.get("system") == "x86_64-linux"
                   and env.get("__structuredAttrs", "") == "" and isinstance(env.get("paths"), str),
                   "Unsupported cross derivation environment")
    paths = env["paths"].split(" ")
    native.require(len(paths) == len(TOOLS) and len(set(paths)) == len(paths),
                   "Expected the fourteen declared wrapper outputs in source order")
    selected = inputs(candidate_record)
    result = {}
    for name, path in zip(TOOLS, paths):
        store_path(path)
        matches = [(drv, value) for drv, value in direct.items()
                   if "out" in selected[drv] and declared_output(value, "out") == path]
        native.require(len(matches) == 1, "Wrapper output has no unique declared producer: " + name)
        drv, value = matches[0]
        wrapper_env = value.get("env", {})
        native.require(set(value["outputs"]) == {"out"} and value.get("system") == "x86_64-linux"
                       and wrapper_env.get("__structuredAttrs", "") == ""
                       and wrapper_env.get("destination") == "/bin/" + name
                       and wrapper_env.get("executable") == "1", "Unexpected wrapper destination or executable contract: " + name)
        text = wrapper_env.get("text")
        native.require(isinstance(text, str) and "\0" not in text, "Missing exact wrapper derivation text: " + name)
        data = text.encode("utf-8")
        native.require(0 < len(data) <= MAX_SCRIPT, "Wrapper script exceeds the bounded byte contract")
        wrapper_inputs = inputs(value)
        bound = "out" in wrapper_inputs.get(native_registered["deriver"], [])
        native.require(bound is (name in NATIVE_TOOLS), "Wrapper has the wrong selected native-linker input: " + name)
        result[name] = {"drv": drv, "out": path, "script": data,
                        "input_drvs": wrapper_inputs, "input_srcs": value["inputSrcs"],
                        "derivation_sha256": canonical_hash(value)}
    return result


def closure_records(metadata, roots):
    native.require(0 < len(metadata) <= MAX_CLOSURE and set(roots) <= set(metadata),
                   "Incomplete or oversized registered closure")
    result = {}
    for path, entry in metadata.items():
        store_path(path)
        native.require(isinstance(entry, dict), "Invalid registered closure entry")
        # Copied source paths in a runtime closure can legitimately lack a
        # deriver. They receive NAR verification, not a producer assertion.
        deriver = entry.get("deriver")
        if deriver is not None:
            store_path(deriver, derivation=True)
        identity = native.registered_identity(dict(entry, deriver=deriver), path)
        refs = entry.get("references")
        native.require(isinstance(refs, list) and all(isinstance(ref, str) for ref in refs)
                       and len(refs) == len(set(refs)) and set(refs) <= set(metadata),
                       "Closure has duplicate, missing or unregistered references")
        result[path] = {**identity, "references": sorted(refs)}
    reached, pending = set(), list(roots)
    while pending:
        path = pending.pop()
        if path not in reached:
            reached.add(path)
            pending.extend(result[path]["references"])
    native.require(reached == set(result), "Path-info includes paths outside the selected closure")
    return result


def root_layout(root, names):
    """The pinned join and writeShellScriptBin outputs contain only bin/."""
    native.require(root.is_dir() and not root.is_symlink() and root.resolve(strict=True) == root,
                   "Output root must be a real canonical directory")
    native.require({path.name for path in root.iterdir()} == {"bin"}, "Unexpected output root entries")
    binary_dir = root / "bin"
    native.require(binary_dir.is_dir() and not binary_dir.is_symlink()
                   and {path.name for path in binary_dir.iterdir()} == set(names),
                   "Output is missing a tool or contains extra entries")
    return {".": stat.S_IMODE(root.stat().st_mode), "bin": stat.S_IMODE(binary_dir.stat().st_mode)}


def installed_inventory(candidate, specs):
    output = Path(candidate["out"])
    directories = root_layout(output, TOOLS)
    links, wrappers = {}, {}
    for name, spec in specs.items():
        path = output / "bin" / name
        native.require(path.is_symlink(), "Joined tool must be the declared symlink: " + name)
        target = Path(spec["out"]) / "bin" / name
        native.require(os.readlink(path) == str(target), "Joined tool link does not name its declared wrapper: " + name)
        wrapper_dirs = root_layout(Path(spec["out"]), (name,))
        mode = target.lstat().st_mode
        native.require(stat.S_ISREG(mode) and mode & 0o111 and not mode & 0o7000,
                       "Wrapper must be a regular executable without special mode bits: " + name)
        # Read only the declared regular file, never an arbitrary symlink chain.
        with target.open("rb") as stream:
            data = stream.read(MAX_SCRIPT + 1)
        native.require(data == spec["script"], "Installed wrapper bytes differ from exact derivation text: " + name)
        links["bin/" + name] = {"target": os.readlink(path), "resolved": str(target)}
        wrappers[name] = {"directories": wrapper_dirs, "file": {"path": "bin/" + name,
                          "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                          "mode": stat.S_IMODE(mode)}}
    return {"directories": directories, "symlinks": links, "wrappers": wrappers}


def inspect_cross_toolchain_output(candidate, dependencies, directory, report, *, expected=None):
    """Inspect exact installed wrappers and their currently registered closure.

    dependencies is {native_linker: <registered identity>, sdk: <identity>} from
    validated successful receipts. directory must be new; every error propagates
    and leaves the caller's status as fail, with partial evidence retained.
    expected optionally binds reinspection to a prior successful inspection.
    """
    report["cross_toolchain_output_inspection_status"] = "fail"
    native.require(set(candidate) == {"drv", "out"} and set(dependencies) == {"native_linker", "sdk"},
                   "Exact candidate and successful native-linker/SDK identities are required")
    store_path(candidate["drv"], derivation=True)
    store_path(candidate["out"])
    for prior in dependencies.values():
        store_path(prior["path"])
        store_path(prior["deriver"], derivation=True)
    directory = Path(directory).absolute()
    native.require(not directory.is_relative_to(STORE) and directory.parent.is_dir()
                   and directory.parent.resolve(strict=True) == directory.parent,
                   "Evidence must be outside the store under a real existing parent")
    directory.mkdir(mode=0o700)  # Never reuse or erase an existing evidence path.
    raw, _ = inspection.checked(NIX + ["path-info", "--offline", "--json", candidate["out"]],
                                 "cross-path-info", directory, report)
    with (directory / "output-path-info.json").open("xb") as stream:
        stream.write(raw)
    selected_metadata = json_object(raw)
    native.require(set(selected_metadata) == {candidate["out"]}, "Output path-info selection changed")
    registered = native.registered_identity(selected_metadata[candidate["out"]], candidate["out"])
    native.require(registered["deriver"] == candidate["drv"], "Cross output has the wrong registered deriver")
    registered["path_info_sha256"] = hashlib.sha256(raw).hexdigest()
    derivations = read_derivations([candidate["drv"]], "cross-derivation", directory, report)
    candidate_record = derivations[candidate["drv"]]
    direct = read_derivations(inputs(candidate_record), "cross-input-derivations", directory, report)
    derivations.update(direct)
    specs = wrapper_specs(candidate_record, direct, candidate, dependencies["native_linker"])
    roots = [candidate["out"]] + [dependencies[name]["path"] for name in ("native_linker", "sdk")]
    raw, _ = inspection.checked(NIX + ["path-info", "--offline", "--recursive", "--json"] + roots,
                                 "cross-closure-path-info", directory, report)
    closure = closure_records(json_object(raw), roots)
    native.require(all(closure[candidate["out"]][key] == registered[key] for key in IDENTITY_KEYS),
                   "Cross registration changed between reads")
    for name, prior in dependencies.items():
        native.require(all(closure[prior["path"]][key] == prior[key] for key in IDENTITY_KEYS),
                       "Successful " + name + " registration changed")
    native.require(set(closure[candidate["out"]]["references"]) == {value["out"] for value in specs.values()},
                   "Cross output must reference exactly its declared wrapper outputs")
    reference_producers = set()
    for name, spec in specs.items():
        entry = closure[spec["out"]]
        native.require(entry["deriver"] == spec["drv"], "Wrapper has the wrong registered producer: " + name)
        if name in NATIVE_TOOLS:
            native.require(dependencies["native_linker"]["path"] in entry["references"],
                           "Wrapper lacks its selected native-linker reference: " + name)
        for ref in entry["references"]:
            if ref not in spec["input_srcs"]:
                producer = closure[ref]["deriver"]
                native.require(producer in spec["input_drvs"], "Wrapper reference is not a declared input: " + name)
                reference_producers.add(producer)
    missing = reference_producers - set(derivations)
    if missing:
        derivations.update(read_derivations(missing, "cross-reference-derivations", directory, report))
    declared_references = {}
    for name, spec in specs.items():
        declared_references[name] = {}
        for ref in closure[spec["out"]]["references"]:
            if ref in spec["input_srcs"]:
                declared_references[name][ref] = {"kind": "source_path"}
                continue
            drv = closure[ref]["deriver"]
            matches = [out for out in spec["input_drvs"][drv] if declared_output(derivations[drv], out) == ref]
            native.require(len(matches) == 1, "Wrapper reference has the wrong selected producer output: " + name)
            declared_references[name][ref] = {"kind": "derivation_output", "drv": drv, "output": matches[0]}
    contents = installed_inventory(candidate, specs)
    _, drv_check = inspection.checked(NIX + ["store", "verify", "--offline", "--derivation"] + sorted(derivations),
                                      "cross-derivation-store-verify", directory, report, timeout=90)
    _, closure_check = inspection.checked(NIX + ["store", "verify", "--offline", "--recursive"] + roots,
                                          "cross-closure-store-verify", directory, report, timeout=90)
    native.require(installed_inventory(candidate, specs) == contents, "Installed wrappers changed during verification")
    raw, _ = inspection.checked(NIX + ["path-info", "--offline", "--recursive", "--json"] + roots,
                                 "cross-final-closure-path-info", directory, report)
    native.require(closure_records(json_object(raw), roots) == closure,
                   "Registered closure changed during verification")
    stable = {"candidate": candidate, "registered_identity": {key: registered[key] for key in IDENTITY_KEYS},
              "dependencies": {name: {key: value[key] for key in IDENTITY_KEYS} for name, value in dependencies.items()},
              "derivations": {drv: canonical_hash(record) for drv, record in sorted(derivations.items())},
              "wrapper_producers": {name: {key: value[key] for key in ("drv", "out", "derivation_sha256")}
                                    for name, value in specs.items()},
              "declared_references": declared_references, "registered_closure": closure, **contents}
    if expected is not None:
        native.require(type(expected.get("schema_version")) is int and expected["schema_version"] == 1
                       and expected.get("scope") == INSPECTION_SCOPE and expected.get("status") == "pass"
                       and expected.get("observed") == stable, "Cross output differs from the successful inspection")
    result = {"schema_version": 1, "scope": INSPECTION_SCOPE, "status": "pass", "observed": stable,
              "registered_output": registered, "derivation_store_verification": drv_check,
              "closure_store_verification": closure_check,
              "native_dependency_declared_in_clang_clangxx_ld": True,
              "wrapper_execution": "not_run", "compiler_target_qualification": "not_run",
              "sdk_use": "requires_explicit_DARWIN_SDK_ROOT_per_invocation",
              "dependency_realization": "not_run", "fixture_linking": "not_run",
              "guest_execution": "not_run", "security_enforcement": "not_run"}
    with (directory / "inspection.json").open("xb") as stream:
        stream.write((json.dumps(result, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    report["cross_toolchain_output_inspection_status"] = "pass"
    return result
