#!/usr/bin/env python3
"""Small owned filesystem fixtures; every Nix result is synthetic, never built."""
import base64
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import inspect_puredarwin_cross_toolchain as cross


class CrossOutputInspectionTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="burdettie-cross-inspection-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.store = self.root / "store"
        self.store.mkdir()
        patcher = mock.patch.object(cross, "STORE", self.store)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.serial = 0
        self.metadata, self.derivations, self.calls = {}, {}, []
        self.failure_name = self.timeout_name = self.mutation = None
        self.evidence_serial = 0
        self.report = {}
        self.native = self.package("native-linker")
        self.sdk = self.package("sdk")
        self.shell = self.package("shell")
        self.tool = self.package("host-tools")
        self.dependencies = {"native_linker": self.identity(self.native["out"]), "sdk": self.identity(self.sdk["out"])}
        self.candidate = self.package("cross")
        out = Path(self.candidate["out"])
        (out / "bin").mkdir()
        self.wrappers = {}
        for name in cross.TOOLS:
            spec = self.package(name)
            wrapper = Path(spec["out"]) / "bin" / name
            wrapper.parent.mkdir()
            refs = [self.shell["out"], self.tool["out"]]
            if name in cross.NATIVE_TOOLS:
                refs.append(self.native["out"])
            # The script is deliberately not a real compiler. Its exact bytes
            # and declared producers, not an exec/shebang heuristic, are tested.
            text = "# owned synthetic wrapper " + name + "\n" + "\n".join(refs) + "\n"
            wrapper.write_text(text)
            wrapper.chmod(0o755)
            (out / "bin" / name).symlink_to(wrapper)
            record = self.derivations[spec["drv"]]
            record["env"] = {"text": text, "destination": "/bin/" + name, "executable": "1", "__structuredAttrs": ""}
            record["inputDrvs"] = {self.metadata[ref]["deriver"]: self.selected() for ref in refs}
            self.metadata[spec["out"]]["references"] = refs
            self.wrappers[name] = spec
        joined = self.derivations[self.candidate["drv"]]
        joined["env"] = {"paths": " ".join(spec["out"] for spec in self.wrappers.values()), "__structuredAttrs": ""}
        joined["inputDrvs"] = {spec["drv"]: self.selected() for spec in self.wrappers.values()}
        self.metadata[self.candidate["out"]]["references"] = [spec["out"] for spec in self.wrappers.values()]
        patcher = mock.patch.object(cross.inspection.subprocess, "run", side_effect=self.nix)
        patcher.start()
        self.addCleanup(patcher.stop)

    @staticmethod
    def selected(*names):
        return {"outputs": list(names or ("out",)), "dynamicOutputs": {}}

    def package(self, name):
        self.serial += 1
        out = str(self.store / (str(self.serial).zfill(32) + "-" + name))
        drv = out + ".drv"
        Path(out).mkdir()
        self.metadata[out] = {"deriver": drv, "narHash": "sha256-" + base64.b64encode(hashlib.sha256(out.encode()).digest()).decode(),
                              "narSize": 1234, "references": []}
        self.derivations[drv] = {"system": "x86_64-linux", "env": {}, "inputDrvs": {}, "inputSrcs": [],
                                 "outputs": {"out": {"path": out}}}
        return {"drv": drv, "out": out}

    def identity(self, path):
        return cross.native.registered_identity(self.metadata[path], path)

    def nix(self, argv, **kwargs):
        self.calls.append(argv)
        self.assertEqual(argv[:len(cross.NIX)], cross.NIX)
        self.assertIn("--store", argv)
        self.assertEqual(argv[argv.index("--store") + 1], "local")
        self.assertIn("--offline", argv)
        for flag in ("build", "--repair", "--no-trust", "--no-contents", "--no-check-sigs", "--all", "--impure"):
            self.assertNotIn(flag, argv)
        command = argv[len(cross.NIX):]
        if command[:2] == ["derivation", "show"]:
            key = "derivation"
            content = {path: self.derivations[path] for path in command[3:]}
        elif command[0] == "path-info":
            key = "closure" if "--recursive" in command else "path-info"
            roots = command[command.index("--json") + 1:]
            paths, pending = set(), list(roots)
            while pending:
                path = pending.pop()
                if path not in paths:
                    paths.add(path)
                    if "--recursive" in command:
                        pending.extend(self.metadata[path]["references"])
            content = {path: self.metadata[path] for path in sorted(paths)}
        elif command[:2] == ["store", "verify"]:
            key = "drv-verify" if "--derivation" in command else "closure-verify"
            if self.mutation and key == "closure-verify":
                self.mutation()
            content = None
        else:
            self.fail("Unexpected subprocess; wrappers and realization must never execute: " + repr(argv))
        if key == self.timeout_name:
            raise subprocess.TimeoutExpired(argv, kwargs["timeout"], output=b"partial read\n", stderr=b"deadline\n")
        if key == self.failure_name:
            return subprocess.CompletedProcess(argv, 7, b"actual failure\n", b"owned negative control\n")
        return subprocess.CompletedProcess(argv, 0, b"" if content is None else json.dumps(content).encode(), b"")

    def inspect(self, **kwargs):
        self.evidence_serial += 1
        self.directory = self.root / ("evidence-" + str(self.evidence_serial))
        return cross.inspect_cross_toolchain_output(self.candidate, self.dependencies, self.directory, self.report, **kwargs)

    def rejected(self, pattern=None):
        with self.assertRaisesRegex(cross.base.BuildError, pattern or ".+"):
            self.inspect()
        self.assertEqual(self.report["cross_toolchain_output_inspection_status"], "fail")
        self.assertFalse((self.directory / "inspection.json").exists())

    def test_accepts_declared_external_join_links_and_retains_full_evidence(self):
        result = self.inspect()
        self.assertEqual(result["status"], "pass")
        self.assertEqual(len(result["observed"]["wrappers"]), 14)
        self.assertEqual(json.loads((self.directory / "inspection.json").read_bytes()), result)
        self.assertEqual(result["registered_output"]["path_info_sha256"],
                         hashlib.sha256((self.directory / "output-path-info.json").read_bytes()).hexdigest())
        self.assertEqual(result["observed"]["dependencies"], self.dependencies)
        self.assertEqual(result["wrapper_execution"], "not_run")
        self.assertEqual(result["guest_execution"], "not_run")
        self.assertEqual(result["security_enforcement"], "not_run")
        for check in self.report["read_checks"].values():
            self.assertIs(type(check["exit_code"]), int)
            self.assertEqual(check["exit_code"], 0)
            self.assertIs(check["timed_out"], False)
        self.assertTrue(any("--derivation" in argv and "verify" in argv for argv in self.calls))
        self.assertTrue(any("--recursive" in argv and "verify" in argv for argv in self.calls))

    def test_reinspection_binds_stable_identity_and_inventory(self):
        first = self.inspect()
        self.assertEqual(self.inspect(expected=first)["observed"], first["observed"])
        changed = copy.deepcopy(first)
        changed["observed"]["wrappers"][cross.TOOLS[0]]["file"]["sha256"] = "0" * 64
        with self.assertRaisesRegex(cross.base.BuildError, "differs from the successful"):
            self.inspect(expected=changed)

    def test_existing_evidence_and_prior_failure_are_preserved(self):
        self.failure_name = "closure-verify"
        self.rejected("failed; inspect retained")
        first = self.directory
        original = {p.name: p.read_bytes() for p in first.iterdir()}
        self.failure_name = None
        self.inspect()
        self.assertEqual(original, {p.name: p.read_bytes() for p in first.iterdir()})
        with self.assertRaises(FileExistsError):
            cross.inspect_cross_toolchain_output(self.candidate, self.dependencies, first, {})
        self.assertEqual(original, {p.name: p.read_bytes() for p in first.iterdir()})

    def test_unregistered_or_wrong_output_deriver_rejected(self):
        self.metadata[self.candidate["out"]]["deriver"] = self.tool["drv"]
        self.rejected("wrong registered deriver")

    def test_wrong_wrapper_deriver_rejected_even_when_link_and_bytes_match(self):
        self.metadata[self.wrappers[cross.TOOLS[0]]["out"]]["deriver"] = self.tool["drv"]
        self.rejected("wrong registered producer")

    def test_nar_hash_and_positive_integer_size_required(self):
        entry = self.metadata[self.candidate["out"]]
        initial = copy.deepcopy(entry)
        for field, bad in (("narHash", "sha256-YQ=="), ("narSize", True), ("narSize", 0), ("narSize", 2.0)):
            with self.subTest(field=field, bad=bad):
                entry.update(initial)
                entry[field] = bad
                self.rejected()

    def test_sdk_or_native_identity_drift_rejected(self):
        for prior in self.dependencies.values():
            with self.subTest(path=prior["path"]):
                self.metadata[prior["path"]]["narSize"] += 1
                self.rejected("registration changed")
                self.metadata[prior["path"]]["narSize"] -= 1

    def test_matching_filename_and_shell_signature_do_not_authorize_changed_bytes(self):
        path = Path(self.wrappers[cross.TOOLS[0]]["out"]) / "bin" / cross.TOOLS[0]
        path.write_bytes(b'#!/bin/sh\nexec /bin/true "$@"\n')
        self.rejected("differ from exact derivation text")

    def test_join_cannot_point_to_a_different_registered_wrapper(self):
        path = Path(self.candidate["out"]) / "bin" / cross.TOOLS[0]
        path.unlink()
        path.symlink_to(Path(self.wrappers[cross.TOOLS[1]]["out"]) / "bin" / cross.TOOLS[1])
        self.rejected("does not name its declared wrapper")

    def test_unregistered_escape_dangling_and_cycle_rejected_without_following(self):
        path = Path(self.candidate["out"]) / "bin" / cross.TOOLS[0]
        for target in (self.root / "private", self.store / "missing", path):
            with self.subTest(target=str(target)):
                path.unlink()
                path.symlink_to(target)
                self.rejected("does not name its declared wrapper")

    def test_wrapper_target_must_be_regular_not_another_link(self):
        path = Path(self.wrappers[cross.TOOLS[0]]["out"]) / "bin" / cross.TOOLS[0]
        path.unlink()
        path.symlink_to(path)
        self.rejected("regular executable")

    def test_missing_or_extra_join_tools_fail(self):
        binary_dir = Path(self.candidate["out"]) / "bin"
        extra = binary_dir / "unreviewed"
        extra.write_text("x")
        self.rejected("missing a tool or contains extra")
        extra.unlink()
        (binary_dir / cross.TOOLS[-1]).unlink()
        self.rejected("missing a tool or contains extra")

    def test_extra_wrapper_files_and_directory_symlinks_fail(self):
        root = Path(self.wrappers[cross.TOOLS[0]]["out"])
        (root / "unexpected").write_text("x")
        self.rejected("Unexpected output root")
        (root / "unexpected").unlink()
        (root / "bin").rename(root / "saved")
        (root / "bin").symlink_to(root / "saved")
        self.rejected("Unexpected output root")

    def test_old_native_dependency_is_rejected(self):
        record = self.derivations[self.wrappers[cross.TOOLS[0]]["drv"]]
        record["inputDrvs"].pop(self.native["drv"])
        record["inputDrvs"][self.sdk["drv"]] = self.selected()
        self.rejected("wrong selected native-linker")

    def test_declared_native_dependency_without_registered_reference_fails(self):
        self.metadata[self.wrappers[cross.TOOLS[0]]["out"]]["references"].remove(self.native["out"])
        self.rejected("lacks its selected native-linker reference")

    def test_same_deriver_does_not_authorize_unselected_output(self):
        self.derivations[self.tool["drv"]]["outputs"] = {"out": {"path": self.sdk["out"]}}
        self.rejected("wrong selected producer output")

    def test_undeclared_registered_reference_is_rejected(self):
        self.metadata[self.wrappers[cross.TOOLS[-1]]["out"]]["references"].append(self.sdk["out"])
        self.rejected("not a declared input")

    def test_registered_copied_source_without_deriver_is_explicit(self):
        source = self.package("owned-script-source")
        self.metadata[source["out"]].pop("deriver")
        name = cross.TOOLS[-1]
        spec = self.wrappers[name]
        self.derivations[spec["drv"]]["inputSrcs"].append(source["out"])
        self.metadata[spec["out"]]["references"].append(source["out"])
        result = self.inspect()
        self.assertEqual(result["observed"]["declared_references"][name][source["out"]], {"kind": "source_path"})
        self.assertIsNone(result["observed"]["registered_closure"][source["out"]]["deriver"])

    def test_unknown_metadata_duplicate_paths_and_dynamic_inputs_fail(self):
        record = self.derivations[self.candidate["drv"]]
        original = copy.deepcopy(record)
        mutations = (
            lambda: record["env"].pop("paths"),
            lambda: record["env"].update(paths=record["env"]["paths"] + " " + self.tool["out"]),
            lambda: record["env"].update(paths=" ".join([next(iter(self.wrappers.values()))["out"]] * len(cross.TOOLS))),
            lambda: record["inputDrvs"][next(iter(record["inputDrvs"]))].update(dynamicOutputs={"unexpected": {}}),
            lambda: record["env"].update(__structuredAttrs="1"),
        )
        for mutation in mutations:
            record.clear()
            record.update(copy.deepcopy(original))
            mutation()
            self.rejected()

    def test_declared_destination_text_and_size_are_required(self):
        record = self.derivations[self.wrappers[cross.TOOLS[0]]["drv"]]
        initial = copy.deepcopy(record["env"])
        for key, value in (("destination", "/bin/not-the-tool"), ("text", None),
                           ("text", "x" * (cross.MAX_SCRIPT + 1)), ("text", "x\0"), ("executable", "")):
            record["env"] = dict(initial, **{key: value})
            self.rejected()

    def test_non_store_candidate_fails_before_any_command(self):
        self.candidate["out"] = str(self.root)
        with self.assertRaisesRegex(cross.base.BuildError, "canonical local store"):
            self.inspect()
        self.assertEqual(self.calls, [])

    def test_verify_failures_and_timeouts_keep_actual_nonzero_observation(self):
        for command in ("drv-verify", "closure-verify"):
            self.failure_name = command
            self.rejected("failed; inspect retained")
            records = list(self.report["read_checks"].values())
            self.assertEqual(records[-1]["exit_code"], 7)
            self.assertTrue(any(p.read_bytes() == b"owned negative control\n" for p in self.directory.glob("*.stderr")))
        self.failure_name = None
        self.timeout_name = "closure"
        self.rejected("failed; inspect retained")
        record = self.report["read_checks"]["cross-closure-path-info"]
        self.assertIsNone(record["exit_code"])
        self.assertIs(record["timed_out"], True)
        self.assertEqual((self.directory / "cross-closure-path-info.stdout").read_bytes(), b"partial read\n")

    def test_file_and_registration_changes_during_verify_fail(self):
        target = Path(self.wrappers[cross.TOOLS[0]]["out"]) / "bin" / cross.TOOLS[0]
        original = target.read_bytes()
        self.mutation = lambda: target.write_bytes(b"changed\n")
        self.rejected("differ from exact derivation text")
        target.write_bytes(original)
        self.mutation = lambda: self.metadata[self.tool["out"]].update(narSize=4321)
        self.rejected("closure changed during verification")

    def test_final_evidence_write_failure_cannot_report_pass(self):
        self.mutation = lambda: (self.directory / "inspection.json").write_bytes(b"preserve prior bytes")
        with self.assertRaises(FileExistsError):
            self.inspect()
        self.assertEqual((self.directory / "inspection.json").read_bytes(), b"preserve prior bytes")
        self.assertEqual(self.report["cross_toolchain_output_inspection_status"], "fail")


class MetadataBoundsTests(unittest.TestCase):
    def test_duplicate_keys_invalid_encoding_and_json_limit(self):
        for raw in (b'{"a": 1, "a": 2}', b'\xff', b'[]', b'{}' + b' ' * cross.MAX_JSON):
            with self.subTest(size=len(raw)):
                with self.assertRaises((cross.base.BuildError, UnicodeDecodeError)):
                    cross.json_object(raw)

    def test_truncated_and_extra_closure_records_fail(self):
        one, two = ("/nix/store/" + str(index).zfill(32) + "-owned" for index in (1, 2))
        entry = {"narHash": "sha256-" + base64.b64encode(b"a" * 32).decode(), "narSize": 12, "references": []}
        bad_records = ({}, {one: dict(entry, references=[two])}, {one: entry, two: entry},
                       {one: dict(entry, references=[one, one])})
        for records in bad_records:
            with self.subTest(records=records):
                with self.assertRaises(cross.base.BuildError):
                    cross.closure_records(records, [one])


if __name__ == "__main__":
    unittest.main()
