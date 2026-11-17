"""Object-only workflow gates. Synthetic Mach-O inputs are not target execution."""
import contextlib
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
import compile_puredarwin_objects as probe


def macho_object(cpu=0x01000007, file_type=1):
    return struct.pack("<8I", 0xFEEDFACF, cpu, 3, file_type, 0, 0, 0, 0)


class ObjectInspectionTests(unittest.TestCase):
    def test_only_x86_64_objects_are_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "input.o"
            path.write_bytes(macho_object())
            report = probe.inspect_object(path)
            self.assertEqual(report["macho"]["slices"][0]["file_type"], 1)
            self.assertEqual(report["symbol_inspection"], "not_run")
            self.assertIsNone(report["undefined_symbols"])
            for data in (macho_object(cpu=0x0100000C), macho_object(file_type=2), macho_object()[:-1]):
                path.write_bytes(data)
                with self.assertRaises((probe.base.BuildError, probe.base.InventoryError)):
                    probe.inspect_object(path)

    def test_resolution_is_offline_sdk_only_and_refuses_missing_outputs(self):
        records = {name: {"out": "/unrealized/" + name, "drv": name + ".drv"}
                   for name in ("sdk", "compiler")}
        with mock.patch.object(probe.base.shutil, "which", return_value="nix"), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": ""}), \
                mock.patch.object(probe.base, "command", side_effect=[
                    "nix (Nix) 2.28.3", json.dumps(records)]) as command:
            with self.assertRaisesRegex(probe.base.BuildError, "not realized"):
                probe.resolve_inputs(Path("/source"))
        argv = command.call_args.args[0]
        for flag in ("--offline", "--no-write-lock-file", "--no-update-lock-file", "allow-import-from-derivation"):
            self.assertIn(flag, argv)
        self.assertIn(probe.base.source_flake_url(Path("/source")) + "#packages.x86_64-linux.apple-sdk", argv)
        self.assertNotIn("build", argv)
        self.assertNotIn("libsystem", " ".join(argv))

    def test_missing_nix_and_remote_store_do_not_run_tools(self):
        with mock.patch.object(probe.base.shutil, "which", return_value=None), \
                mock.patch.object(probe.base, "command") as command:
            with self.assertRaisesRegex(probe.base.BuildError, "unavailable"):
                probe.resolve_inputs(Path("/source"))
            command.assert_not_called()
        with mock.patch.object(probe.base.shutil, "which", return_value="nix"), \
                mock.patch.dict(os.environ, {"NIX_REMOTE": "ssh://other-store"}), \
                mock.patch.object(probe.base, "command") as command:
            with self.assertRaisesRegex(probe.base.BuildError, "remote"):
                probe.resolve_inputs(Path("/source"))
            command.assert_not_called()


class ObjectWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.records = {name: {"out": str(self.root / name), "drv": name + ".drv"}
                        for name in ("sdk", "compiler")}
        self.compiler = self.root / "compiler/bin/clang"
        self.compiler.parent.mkdir(parents=True)
        self.compiler.write_text("mock compiler")
        self.sdk = self.root / "sdk/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
        for name in ("usr/include/stdio.h", "usr/include/pthread.h", "usr/lib/libSystem.tbd"):
            path = self.sdk / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(name)
        self.calls = []
        self.failure = False
        self.nm_failure = False
        self.compiler_version = "clang version 21.1.8"

    def command(self, argv, **kwargs):
        self.calls.append((argv, kwargs))
        if argv[0] == "git":
            return "source-revision"
        if argv[-1] == "--version":
            return "LLVM version 21.1.8" if argv[0].endswith("llvm-nm") else self.compiler_version
        if "--undefined-only" in argv:
            if self.nm_failure:
                raise probe.base.BuildError("nm failed")
            return "                 U _printf\n"
        if self.failure:
            raise probe.base.BuildError("compiler failed")
        self.assertIn("-c", argv)
        Path(argv[argv.index("-o") + 1]).write_bytes(macho_object())
        return ""

    def run_probe(self, extra=()):
        with mock.patch.object(probe.base.platform, "system", return_value="Linux"), \
                mock.patch.object(probe.base.platform, "machine", return_value="x86_64"), \
                mock.patch.object(probe.base, "source_identity", return_value={"revision": "pinned"}), \
                mock.patch.object(probe, "resolve_inputs", return_value=("nix 2.28.3", self.records)), \
                mock.patch.object(probe.base, "command", side_effect=self.command), \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            code = probe.main(["--puredarwin-source", str(self.root), *extra])
        return code, json.loads(stdout.getvalue())

    def test_default_preflight_never_compiles_or_creates_output(self):
        output = self.root / "unused"
        code, report = self.run_probe(["--output-dir", str(output)])
        self.assertEqual((code, report["status"]), (0, "ready_to_compile"))
        self.assertFalse(output.exists())
        self.assertFalse(any("-c" in argv for argv, _ in self.calls))

    def test_compile_freezes_sources_and_never_claims_link_or_execution(self):
        output = self.root / "new"
        with mock.patch.dict(os.environ, {"CPATH": "/foreign", "SDKROOT": "/foreign"}):
            code, report = self.run_probe(["--compile", "--output-dir", str(output)])
        self.assertEqual((code, report["status"]), (0, "objects_compiled_and_inspected"))
        self.assertIsNone(report["llvm_nm"])
        for key in ("linking", "guest_execution", "burdettie_runtime_execution", "security_enforcement"):
            self.assertEqual(report[key], "not_run")
        for name, relative in probe.base.FIXTURES.items():
            self.assertEqual((output / (name + ".c")).read_bytes(), (probe.base.ROOT / relative).read_bytes())
            self.assertEqual(report["artifacts"][name]["source_sha256"], probe.base.sha256(output / (name + ".c")))
            self.assertEqual(report["artifacts"][name]["symbol_inspection"], "not_run")
        for argv, kwargs in self.calls:
            if "-c" in argv:
                self.assertIn("--target=x86_64-apple-macosx26.5", argv)
                self.assertEqual(argv[argv.index("-isysroot") + 1], str(self.sdk))
                self.assertNotIn("-lSystem", argv)
                self.assertNotIn("CPATH", kwargs["env"])
                self.assertNotIn("SDKROOT", kwargs["env"])

    def test_existing_output_and_evidence_are_preserved(self):
        output = self.root / "retained"
        output.mkdir()
        (output / "result.json").write_text("original")
        code, report = self.run_probe(["--compile", "--output-dir", str(output)])
        self.assertEqual((code, report["status"]), (2, "blocked"))
        self.assertEqual((output / "result.json").read_text(), "original")
        self.assertFalse(any("-c" in argv for argv, _ in self.calls))

    def test_compiler_failure_is_retained_as_failure(self):
        output = self.root / "failed"
        self.failure = True
        code, report = self.run_probe(["--compile", "--output-dir", str(output)])
        self.assertEqual((code, report["status"]), (1, "fail"))
        self.assertEqual(json.loads((output / "result.json").read_text())["error"], "compiler failed")

    def test_unexpected_compiler_blocks_before_compilation(self):
        self.compiler_version = "clang version 22.0.0"
        code, report = self.run_probe()
        self.assertEqual((code, report["status"]), (2, "blocked"))
        self.assertFalse(any("-c" in argv for argv, _ in self.calls))

    def test_optional_nm_records_imports_and_propagates_failure(self):
        nm = self.root / "llvm-nm"
        nm.write_text("mock LLVM nm")
        code, report = self.run_probe(["--compile", "--output-dir", str(self.root / "with-nm"), "--llvm-nm", str(nm)])
        self.assertEqual(code, 0)
        self.assertEqual(report["artifacts"]["simple_cli"]["undefined_symbols"], ["                 U _printf"])
        self.nm_failure = True
        code, report = self.run_probe(["--compile", "--output-dir", str(self.root / "failed-nm"), "--llvm-nm", str(nm)])
        self.assertEqual((code, report["status"], report["error"]), (1, "fail", "nm failed"))


if __name__ == "__main__":
    unittest.main()
