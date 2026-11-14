"""Build gate tests plus native-host probe logic; no Darwin execution claim."""

import contextlib
import hashlib
import io
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_puredarwin_fixtures as fixture


def string_command(kind, value, header=12):
    text = value.encode() + b"\0"
    size = (header + len(text) + 7) & ~7
    data = struct.pack("<III", kind, size, header)
    data += b"\0" * (header - len(data)) + text
    return data.ljust(size, b"\0")


def executable(*, cpu=0x01000007, version=0x001A0500,
               dependency="/usr/lib/libSystem.B.dylib", dylinker="/usr/lib/dyld",
               main=True, extra=b""):
    commands = [struct.pack("<6I", 0x32, 24, 1, version, version, 0),
                string_command(0xC, dependency, 24), string_command(0xE, dylinker)]
    if main:
        commands.append(struct.pack("<IIQQ", 0x80000028, 24, 0x1000, 0))
    if extra:
        commands.append(extra)
    blob = b"".join(commands)
    return struct.pack("<8I", 0xFEEDFACF, cpu, 3, 2, len(commands), len(blob), 0, 0) + blob


class FixtureInspectionTests(unittest.TestCase):
    def inspect(self, blob):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "fixture"
            path.write_bytes(blob)
            return fixture.inspect_fixture(path)

    def test_valid_structure_never_claims_guest_execution(self):
        report = self.inspect(executable())
        self.assertEqual(report["guest_execution"], "not_run")
        self.assertEqual(report["burdettie_runtime_execution"], "not_run")
        self.assertEqual(report["dylinker"], "/usr/lib/dyld")

    def test_rejects_wrong_architecture(self):
        with self.assertRaisesRegex(fixture.BuildError, "x86_64"):
            self.inspect(executable(cpu=0x0100000C))

    def test_rejects_old_deployment_version(self):
        with self.assertRaisesRegex(fixture.BuildError, "deployment"):
            self.inspect(executable(version=0x000B0300))

    def test_rejects_host_or_proprietary_framework_dependency(self):
        with self.assertRaisesRegex(fixture.BuildError, "dependency"):
            self.inspect(executable(dependency="/System/Library/Frameworks/AppKit.framework/AppKit"))

    def test_rejects_rpath(self):
        with self.assertRaisesRegex(fixture.BuildError, "search path"):
            self.inspect(executable(extra=string_command(0x8000001C, "@loader_path")))

    def test_rejects_missing_entry_or_foreign_dylinker(self):
        for args in ({"main": False}, {"dylinker": "/tmp/other-dyld"}):
            with self.subTest(args=args), self.assertRaisesRegex(fixture.BuildError, "LC_MAIN"):
                self.inspect(executable(**args))

    def test_rejects_truncated_commands(self):
        with self.assertRaises((fixture.BuildError, fixture.InventoryError)):
            self.inspect(executable()[:-1])

    def test_rejects_duplicate_version(self):
        with self.assertRaisesRegex(fixture.BuildError, "deployment"):
            self.inspect(executable(extra=struct.pack("<6I", 0x32, 24, 1, 0x1A0500, 0x1A0500, 0)))


class BuildSafetyTests(unittest.TestCase):
    def test_nix_version_floor_for_selected_flake_syntax(self):
        for version in ("nix (Nix) 2.18.1", "nix (Nix) 2.26.3", "unidentified"):
            with self.subTest(version=version), self.assertRaisesRegex(fixture.BuildError, "2.27"):
                fixture.require_nix_version(version)
        for version in ("nix (Nix) 2.27.0", "nix (Nix) 2.31.6"):
            fixture.require_nix_version(version)

    def test_lock_follows_root_input_instead_of_first_named_node(self):
        raw = json.dumps({"root": "root", "nodes": {
            "root": {"inputs": {"nixpkgs": "nixpkgs_3"}},
            "nixpkgs": {"locked": {"rev": "wrong-indirect-input"}},
            "nixpkgs_3": {"locked": {"rev": fixture.NIXPKGS_REVISION}},
        }}).encode()
        with mock.patch.object(fixture, "LOCK_SHA256", hashlib.sha256(raw).hexdigest()):
            self.assertEqual(fixture.lock_identity(raw)["nixpkgs_revision"], fixture.NIXPKGS_REVISION)
        with self.assertRaisesRegex(fixture.BuildError, "differs"):
            fixture.lock_identity(raw)

    def test_source_wrong_revision_and_incomplete_submodule_refused(self):
        with mock.patch.object(fixture, "command", return_value="newer-revision\n"):
            with self.assertRaisesRegex(fixture.BuildError, "selected revision"):
                fixture.source_identity(Path("."))
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp)
            (source / "flake.lock").write_text("{}")
            with mock.patch.object(fixture, "lock_identity", return_value={}), \
                    mock.patch.object(fixture, "command", side_effect=[
                        fixture.PUREDARWIN_REVISION, "", "-01234 external/corecrypto\n"]):
                with self.assertRaisesRegex(fixture.BuildError, "submodules"):
                    fixture.source_identity(source)

    def test_no_nix_means_blocked_without_installing(self):
        with mock.patch.object(fixture.shutil, "which", return_value=None), \
                mock.patch.object(fixture, "command") as command:
            with self.assertRaisesRegex(fixture.BuildError, "unavailable"):
                fixture.resolve_packages(Path("/some/source"))
            command.assert_not_called()

    def test_nix_only_evaluates_offline_and_refuses_unrealized_output(self):
        packages = {name: {"out": "/missing/store/" + name, "drv": "unused"}
                    for name in fixture.PACKAGES}
        with mock.patch.object(fixture.shutil, "which", return_value="/usr/bin/nix"), \
                mock.patch.dict(fixture.os.environ, {"NIX_REMOTE": ""}), \
                mock.patch.object(fixture, "command", return_value=json.dumps(packages)) as command:
            with self.assertRaisesRegex(fixture.BuildError, "not realized"):
                fixture.resolve_packages(Path("/some/source"))
            argv = command.call_args.args[0]
            self.assertIn("--offline", argv)
            self.assertIn("--no-update-lock-file", argv)
            self.assertIn("allow-import-from-derivation", argv)
            self.assertNotIn("build", argv)

    def test_link_never_masks_undefined_symbols(self):
        paths = {name: Path("/pinned") / name for name in ("compiler", "linker", "libsystem")}
        argv = fixture.compile_command(paths, Path("probe.c"), Path("probe"))
        self.assertNotIn("-Wl,-undefined,dynamic_lookup", argv)
        self.assertIn("-lSystem", argv)
        self.assertIn("-Wl,-platform_version,macos,26.5,26.5", argv)

    def test_existing_output_is_untouched_and_failure_is_not_success(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "retained"
            output.mkdir()
            result = output / "result.json"
            result.write_text("original evidence")
            paths = {name: Path(temp) / name for name in ("compiler", "linker", "libsystem")}
            with mock.patch.object(fixture.platform, "system", return_value="Linux"), \
                    mock.patch.object(fixture.platform, "machine", return_value="x86_64"), \
                    mock.patch.object(fixture, "source_identity", return_value={}), \
                    mock.patch.object(fixture, "resolve_packages", return_value={}), \
                    mock.patch.object(fixture.shutil, "which", return_value="/usr/bin/nix"), \
                    mock.patch.object(fixture, "tool_paths", return_value=paths), \
                    mock.patch.object(fixture, "sha256", return_value="hash"), \
                    mock.patch.object(fixture, "command", return_value="nix (Nix) 2.27.1"), \
                    contextlib.redirect_stdout(io.StringIO()) as stdout:
                code = fixture.main(["--puredarwin-source", temp, "--build", "--output-dir", str(output)])
            self.assertEqual(code, 2)
            self.assertEqual(json.loads(stdout.getvalue())["status"], "blocked")
            self.assertEqual(result.read_text(), "original evidence")


@unittest.skipUnless(shutil.which("cc"), "A native C compiler is required for host-logic tests")
class ProbeHostLogicTests(unittest.TestCase):
    """These are native Linux/macOS source tests, never candidate VM evidence."""

    @classmethod
    def setUpClass(cls):
        cls.build = tempfile.TemporaryDirectory()
        cls.binary = Path(cls.build.name) / "host-probe"
        subprocess.run([shutil.which("cc"), "-std=c11", "-Wall", "-Wextra", "-Werror", "-pthread",
                        str(fixture.ROOT / "fixtures/m1_abi_probe/probe.c"), "-o", str(cls.binary)],
                       check=True, capture_output=True, timeout=30)

    @classmethod
    def tearDownClass(cls):
        cls.build.cleanup()

    def run_probe(self, mode, path, nonce="unit-test-123"):
        return subprocess.run([str(self.binary), mode, str(path), nonce],
                              capture_output=True, text=True, timeout=5)

    def test_write_then_read_and_exit_cleanup(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "probe"
            for mode in ("write", "read"):
                result = self.run_probe(mode, path)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.splitlines(), [
                    f"BURDETTIE_ABI_V1 nonce=unit-test-123 mode={mode} allocation=pass file=pass thread=pass",
                    "BURDETTIE_ABI_V1 nonce=unit-test-123 exit_cleanup=pass"])
            self.assertEqual(path.read_bytes(), "Burdettie M1 ABI v1\nnonce=unit-test-123\nunicode=café 雪\n".encode())

    def test_existing_file_is_not_replaced(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "probe"
            path.write_text("preserve")
            result = self.run_probe("write", path)
            self.assertEqual(result.returncode, 6)
            self.assertEqual(result.stdout, "")
            self.assertEqual(path.read_text(), "preserve")

    def test_exact_read_rejects_wrong_nonce_and_trailing_bytes(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "probe"
            self.assertEqual(self.run_probe("write", path).returncode, 0)
            self.assertEqual(self.run_probe("read", path, "different-nonce").returncode, 7)
            with path.open("ab") as stream:
                stream.write(b"trailing bytes")
            self.assertEqual(self.run_probe("read", path).returncode, 7)

    def test_missing_file_and_control_character_nonce_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "probe"
            self.assertEqual(self.run_probe("read", path).returncode, 7)
            self.assertEqual(self.run_probe("write", path, "fake\npass").returncode, 2)
            self.assertFalse(path.exists())


if __name__ == "__main__":
    unittest.main()
