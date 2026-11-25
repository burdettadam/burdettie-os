"""Exercise staging, the pinned patch/hook paths, and real CMake installation."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import stage_libtapi_source as staging


ARCHITECTURE = "src/llvm/include/llvm/TextAPI/MachO/Architecture.def"
CONFIG_GUESS = "src/llvm/cmake/config.guess"
HEADERS = ("APIVersion.h", "Defines.h", "LinkerInterfaceFile.h", "PackedVersion32.h",
           "Symbol.h", "tapi.h", "Version.h")
# The exact three-line hunk context and additions from pinned PureDarwin's
# nix/pkgs/toolchain/libtapi-riscv.patch, against a small synthetic input.
PATCH = """--- a/src/llvm/include/llvm/TextAPI/MachO/Architecture.def
+++ b/src/llvm/include/llvm/TextAPI/MachO/Architecture.def
@@ -1,3 +1,7 @@
 ///
 ARCHINFO(arm64, MachO::CPU_TYPE_ARM64, MachO::CPU_SUBTYPE_ARM64_ALL, 64)
 ARCHINFO(arm64e, MachO::CPU_TYPE_ARM64, MachO::CPU_SUBTYPE_ARM64E, 64)
+
+// riscv, cpu type 24 as ld64 defines it
+ARCHINFO(riscv32, 24, 0, 32)
+ARCHINFO(riscv64, 24 | MachO::CPU_ARCH_ABI64, 0, 64)
"""


def make_source(root):
    source = root / "immutable input"
    files = {
        "build.sh": "#!/usr/bin/env bash\n: build\n",
        "install.sh": "#!/usr/bin/env bash\n: install\n",
        "tools/tools.sh": ": setup\n",
        "tools/cpucount/get_cpu_count.sh": "#!/usr/bin/env bash\necho 1\n",
        CONFIG_GUESS: "#!/bin/sh\necho original\n",
        ARCHITECTURE: "///\nARCHINFO(arm64, MachO::CPU_TYPE_ARM64, MachO::CPU_SUBTYPE_ARM64_ALL, 64)\n"
                      "ARCHINFO(arm64e, MachO::CPU_TYPE_ARM64, MachO::CPU_SUBTYPE_ARM64E, 64)\n",
        ".metadata": "complete source includes hidden files\n",
        "src/clang/nested/ordinary file.cpp": "unmodified source\n",
        "CMakeLists.txt": "cmake_minimum_required(VERSION 3.13)\nproject(Staging NONE)\n"
                          "file(GLOB_RECURSE sources RELATIVE \"${CMAKE_CURRENT_SOURCE_DIR}\" \"src/*.cpp\")\n"
                          "file(WRITE \"${CMAKE_CURRENT_BINARY_DIR}/discovered.txt\" \"${sources}\")\n"
                          "file(WRITE \"${CMAKE_CURRENT_BINARY_DIR}/source-dir.txt\" \"${CMAKE_CURRENT_SOURCE_DIR}\")\n"
                          "add_subdirectory(src/tapi/include)\n",
        "src/tapi/include/CMakeLists.txt": "set(HEADERS\n" + "".join(
            '  "${CMAKE_CURRENT_SOURCE_DIR}/tapi/' + name + '"\n' for name in HEADERS)
            + ")\nfile(WRITE \"${CMAKE_CURRENT_BINARY_DIR}/Version.inc\" \"generated version\\n\")\n"
              "install(FILES ${HEADERS} \"${CMAKE_CURRENT_BINARY_DIR}/Version.inc\" DESTINATION include/tapi)\n",
    }
    files.update({"src/tapi/include/tapi/" + name: "header " + name + "\n" for name in HEADERS})
    for name, content in files.items():
        path = source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        path.chmod(0o555 if name.endswith(".sh") or name == CONFIG_GUESS else 0o444)
    (source / "empty").mkdir()
    (source / "build_tapi_tools.sh").symlink_to("build.sh")
    return source


class SourceStagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = make_source(self.root)
        self.destination = self.root / "stage"
        self.manifest = self.root / "stage-manifest.json"

    def stage(self):
        return staging.stage(self.source, self.destination, self.manifest)

    def test_complete_inventory_preserves_relative_symlink_and_private_copies(self):
        before = staging.inspect_source(self.source)
        receipt = self.stage()
        after = staging.inspect_source(self.source)
        self.assertEqual(before, after)
        manifest = json.loads(self.manifest.read_text())
        self.assertEqual(manifest["entries"], before["entries"])
        self.assertEqual(receipt["manifest_sha256"], hashlib.sha256(self.manifest.read_bytes()).hexdigest())
        self.assertEqual(receipt["source_inventory_sha256"], before["source_inventory_sha256"])
        self.assertGreater(receipt["staged_allocated_bytes"], 0)
        self.assertFalse(receipt["summary"]["peak_space_qualified"])
        for entry in before["entries"]:
            original, staged = self.source / entry["path"], self.destination / entry["path"]
            if entry["kind"] == "directory":
                self.assertTrue(staged.is_dir())
                self.assertFalse(staged.is_symlink())
                self.assertEqual(staged.stat().st_mode & 0o700, 0o700)
            elif entry["kind"] == "original_symlink":
                self.assertEqual(os.readlink(staged), os.readlink(original))
            elif entry["staging"] == "copy":
                self.assertFalse(staged.is_symlink())
                self.assertNotEqual(staged.stat().st_ino, original.stat().st_ino)
                self.assertTrue(staged.stat().st_mode & 0o200)
                self.assertEqual(staged.read_bytes(), original.read_bytes())
            else:
                self.assertTrue(staged.is_symlink())
                self.assertEqual(os.readlink(staged), str(original))
        self.assertEqual((self.destination / "build_tapi_tools.sh").resolve(), self.destination / "build.sh")

    def test_patch_shebang_and_gnu_config_hook_paths_do_not_modify_source(self):
        before = staging.inspect_source(self.source)
        self.stage()
        subprocess.run(["patch", "--batch", "--forward", "-p1"], input=PATCH,
                       cwd=self.destination, text=True, capture_output=True, check=True)
        # The pinned patchShebangs hook uses find -type f and rewrites in place.
        found = subprocess.check_output(["find", "build.sh", "install.sh", "tools",
                                         "-type", "f", "-perm", "-0100"],
                                        cwd=self.destination, text=True).splitlines()
        self.assertIn("build.sh", found)
        self.assertIn("install.sh", found)
        self.assertIn("tools/cpucount/get_cpu_count.sh", found)
        for name in found:
            path = self.destination / name
            if path.read_text().startswith("#!"):
                with path.open("r+") as stream:
                    contents = stream.read().splitlines(keepends=True)
                    stream.seek(0)
                    stream.write("#!/pinned/bash\n" + "".join(contents[1:]))
                    stream.truncate()
        # Match the pinned preConfigure hook's find/cp -f behavior: a file
        # symlink would be silently omitted, changing the original build.
        found = subprocess.check_output(["find", ".", "-type", "f", "-name", "config.guess"],
                                        cwd=self.destination, text=True).splitlines()
        self.assertEqual(found, ["./" + CONFIG_GUESS])
        replacement = self.root / "new-config.guess"
        replacement.write_text("#!/bin/sh\necho updated\n")
        subprocess.run(["cp", "-f", str(replacement), found[0]], cwd=self.destination, check=True)
        self.assertEqual((self.destination / CONFIG_GUESS).read_bytes(), replacement.read_bytes())
        # Namespaced includes resolve the staged private patch file.
        probe = '#define ARCHINFO(name, cpu, sub, bits) M1_##name\n#include "llvm/TextAPI/MachO/Architecture.def"\n'
        preprocessed = subprocess.check_output(["cc", "-E", "-P", "-x", "c", "-I",
                                                str(self.destination / "src/llvm/include"), "-"],
                                               input=probe, text=True)
        self.assertIn("M1_riscv32", preprocessed)
        self.assertIn("M1_riscv64", preprocessed)
        self.assertEqual(before, staging.inspect_source(self.source))

    def test_real_cmake_discovers_sources_and_installs_regular_headers(self):
        cmake = os.environ.get("CMAKE_BIN") or shutil.which("cmake")
        self.assertIsNotNone(cmake, "Set CMAKE_BIN or install CMake; this check must not be skipped")
        self.stage()
        build, install = self.root / "configured", self.root / "installed"
        subprocess.run([cmake, "-S", str(self.destination), "-B", str(build),
                        "-DCMAKE_INSTALL_PREFIX=" + str(install)], capture_output=True, text=True, check=True)
        self.assertEqual((build / "source-dir.txt").read_text(), str(self.destination))
        self.assertIn("src/clang/nested/ordinary file.cpp", (build / "discovered.txt").read_text())
        subprocess.run([cmake, "--install", str(build)], capture_output=True, text=True, check=True)
        for name in (*HEADERS, "Version.inc"):
            path = install / "include/tapi" / name
            self.assertTrue(path.is_file())
            self.assertFalse(path.is_symlink())
        # Negative control: CMake really would preserve an uncorrected source
        # file symlink during this same installation.
        header = self.destination / "src/tapi/include/tapi/tapi.h"
        header.unlink()
        header.symlink_to(self.source / "src/tapi/include/tapi/tapi.h")
        wrong_install = self.root / "symlink-install"
        subprocess.run([cmake, "--install", str(build), "--prefix", str(wrong_install)],
                       capture_output=True, text=True, check=True)
        self.assertTrue((wrong_install / "include/tapi/tapi.h").is_symlink())

    def test_existing_or_source_internal_outputs_are_refused_without_overwrite(self):
        self.stage()
        retained = self.manifest.read_bytes()
        with self.assertRaisesRegex(ValueError, "existing output"):
            self.stage()
        self.assertEqual(self.manifest.read_bytes(), retained)
        with self.assertRaisesRegex(ValueError, "outside the immutable source"):
            staging.stage(self.source, self.source / "bad-stage", self.root / "other.json")
        self.assertFalse((self.source / "bad-stage").exists())
        (self.root / "broken-link").symlink_to("missing")
        with self.assertRaisesRegex(ValueError, "existing output"):
            staging.stage(self.source, self.root / "broken-link", self.root / "other.json")

    def test_required_writable_inputs_and_unexpected_hook_paths_fail_closed(self):
        (self.source / CONFIG_GUESS).unlink()
        with self.assertRaises(FileNotFoundError):
            self.stage()
        self.assertFalse(self.destination.exists())
        (self.source / CONFIG_GUESS).write_text("original")
        (self.source / "config.sub").write_text("unexpected hook input")
        with self.assertRaisesRegex(ValueError, "GNU config hook inputs"):
            self.stage()
        self.assertFalse(self.destination.exists())

    def test_unsupported_entry_type_is_rejected_without_staging(self):
        os.mkfifo(self.source / "unexpected-fifo")
        with self.assertRaisesRegex(ValueError, "Unsupported source entry"):
            self.stage()
        self.assertFalse(self.destination.exists())

    def test_source_change_during_staging_is_not_reported_as_success(self):
        copystat = staging.shutil.copystat
        changed = False

        def external_change(*args, **kwargs):
            nonlocal changed
            copystat(*args, **kwargs)
            if not changed:
                metadata = self.source / ".metadata"
                metadata.chmod(0o644)
                metadata.write_text("simulated external source change\n")
                changed = True

        with mock.patch.object(staging.shutil, "copystat", side_effect=external_change):
            with self.assertRaisesRegex(ValueError, "Immutable source changed"):
                self.stage()
        self.assertTrue(self.destination.is_dir())
        self.assertFalse(self.manifest.exists())


if __name__ == "__main__":
    unittest.main()
