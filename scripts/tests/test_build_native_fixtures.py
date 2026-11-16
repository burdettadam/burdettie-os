"""Exercise the native builder recipe with real, source-owned Linux arithmetic.

Only uname/xcrun and unrelated fixture compilation are simulated. Each runtime
slice is compiled and linked by the installed host C compiler. The lipo shim
copies one owned ELF slice for execution; it provides no Mach-O, SDK, universal
binary or selected-runtime qualification.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import m1_compiler_rt_vectors as vectors
import probe_m1_compiler_rt_abi as probe


class NativeFixtureBuilderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="burdettie native fixtures ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.project = self.root / "project"
        self.scripts = self.project / "scripts"
        self.scripts.mkdir(parents=True)
        for name in ("build_fixtures.sh", "m1_compiler_rt_vectors.py"):
            shutil.copyfile(ROOT / "scripts" / name, self.scripts / name)
        self.fixtures = self.project / "fixtures"
        self.fixtures.mkdir()
        # Exercise the two relevant generic-loop routes. Unrelated explicit
        # recipes still run through the compiler shim, which creates parents.
        for name in ("m1_compiler_rt_probe", "simple_cli"):
            shutil.copytree(ROOT / "fixtures" / name, self.fixtures / name)
        self.fixture = self.fixtures / "m1_compiler_rt_probe"
        self.output = self.fixture / "app"
        self.assertFalse(self.output.exists(), "source fixture must not contain a built app")
        self.tools = self.root / "tools"
        self.tools.mkdir()
        self.log = self.root / "compiler-arguments.jsonl"
        self.xcrun_log = self.root / "xcrun-arguments.jsonl"
        self.sdk = self.root / "SDK with spaces"
        self.sdk.mkdir()
        self.work = self.root / "native build work"
        self.work.mkdir()
        self.cc = shutil.which("cc")
        self.assertIsNotNone(self.cc, "native recipe regression requires a real host C compiler")
        self.write_tool("uname", "print('Darwin')\n")
        self.write_tool("xcrun", '''import json
import os
from pathlib import Path
import shutil
import sys
args = sys.argv[1:]
with open(os.environ["NATIVE_TEST_XCRUN_LOG"], "a") as stream:
    stream.write(json.dumps(args) + "\\n")
if args == ["--sdk", "macosx", "--find", "clang"]:
    print(os.environ["NATIVE_TEST_COMPILER"])
elif args == ["--sdk", "macosx", "--show-sdk-path"]:
    print(os.environ["NATIVE_TEST_SDK"])
elif args and args[0] == "lipo" and "-create" in args:
    inputs = args[args.index("-create") + 1:args.index("-output")]
    if not inputs or not all(Path(name).is_file() for name in inputs):
        raise SystemExit("lipo shim requires actual input files")
    shutil.copy2(inputs[0], args[args.index("-output") + 1])
elif args and args[0] == "lipo" and "-verify_arch" in args:
    # Apple lipo consumes every operand after -verify_arch as an architecture.
    if args.index("-verify_arch") != 2:
        raise SystemExit("lipo verification requires one input before -verify_arch")
    architectures = args[3:]
    if not architectures or any(arch not in ("arm64", "x86_64") for arch in architectures):
        raise SystemExit("lipo verification requires only architectures after -verify_arch")
    if not Path(args[1]).is_file():
        raise SystemExit("lipo verification shim requires an existing output")
    if os.environ.get("NATIVE_TEST_FAIL_VERIFY") == "1":
        print("injected lipo verification failure", file=sys.stderr)
        raise SystemExit(69)
else:
    raise SystemExit("unexpected xcrun invocation: " + repr(args))
''')
        self.compiler = self.write_tool("compiler", '''import json
import os
from pathlib import Path
import subprocess
import sys
args = sys.argv[1:]
is_probe = any(Path(arg).suffix == ".c" and Path(arg).is_file()
               and (Path(arg).parent / "probe.h").is_file()
               and b"m1_compiler_rt_probe" in Path(arg).read_bytes() for arg in args)
with open(os.environ["NATIVE_TEST_LOG"], "a") as stream:
    stream.write(json.dumps({"argv": args, "runtime_fixture": is_probe}) + "\\n")
if is_probe:
    # Keep source inputs and every non-platform compiler flag intact.
    host_args = []
    index = 0
    while index < len(args):
        if args[index] in ("-isysroot", "-arch"):
            index += 2
        else:
            host_args.append(args[index])
            index += 1
    result = subprocess.run([os.environ["NATIVE_TEST_HOST_CC"], *host_args, "-lm"])
    raise SystemExit(result.returncode)
output = Path(args[args.index("-o") + 1])
output.parent.mkdir(parents=True, exist_ok=True)
output.write_bytes(b"stub unrelated fixture\\n")
''')
        self.env = dict(os.environ)
        self.env.update(PATH=str(self.tools) + os.pathsep + self.env["PATH"],
                        TMPDIR=str(self.work),
                        NATIVE_TEST_COMPILER=str(self.compiler),
                        NATIVE_TEST_HOST_CC=self.cc, NATIVE_TEST_LOG=str(self.log),
                        NATIVE_TEST_XCRUN_LOG=str(self.xcrun_log),
                        NATIVE_TEST_SDK=str(self.sdk))

    def write_tool(self, name, source):
        path = self.tools / name
        path.write_text("#!" + sys.executable + "\n" + source)
        path.chmod(0o700)
        return path

    def build(self):
        return subprocess.run(["bash", str(self.scripts / "build_fixtures.sh")],
                              cwd=self.project, env=self.env, capture_output=True,
                              timeout=30)

    def invocations(self, fixture):
        if not self.log.exists():
            return []
        records = map(json.loads, self.log.read_text().splitlines())
        if fixture == "m1_compiler_rt_probe":
            return [record["argv"] for record in records if record["runtime_fixture"]]
        return [record["argv"] for record in records
                if str(self.fixtures / fixture / "main.c") in record["argv"]]

    def test_real_runtime_fixture_is_complete_and_ordinary_architecture_is_preserved(self):
        stale_header = self.fixture / "vectors.h"
        poison = b"#error stale source vectors header must never be used\n"
        stale_header.write_bytes(poison)
        process = self.build()
        self.assertEqual(process.returncode, 0,
                         (process.stdout + process.stderr).decode(errors="replace"))
        invocations = self.invocations("m1_compiler_rt_probe")
        self.assertEqual(len(invocations), 2)
        self.assertEqual(sorted(args[args.index("-arch") + 1] for args in invocations),
                         ["arm64", "x86_64"])
        for args in invocations:
            sources = [Path(arg) for arg in args if Path(arg).suffix == ".c"]
            self.assertEqual(sorted(source.name for source in sources), ["main.c", "probe.c"])
            self.assertEqual(sources[0].parent, sources[1].parent)
            self.assertNotEqual(sources[0].parent, self.fixture)
        self.assertEqual(stale_header.read_bytes(), poison)
        slice_outputs = {args[args.index("-o") + 1] for args in invocations}
        xcrun = list(map(json.loads, self.xcrun_log.read_text().splitlines()))
        creates = [args for args in xcrun if "-create" in args and slice_outputs <= set(args)]
        self.assertEqual(len(creates), 1)
        created = creates[0][creates[0].index("-output") + 1]
        verifies = [args for args in xcrun if "-verify_arch" in args
                    and args[1:3] == [created, "-verify_arch"]]
        self.assertEqual(len(verifies), 1)
        self.assertCountEqual(verifies[0][3:], ["arm64", "x86_64"])
        ordinary = self.invocations("simple_cli")
        self.assertEqual(len(ordinary), 1)
        self.assertEqual(ordinary[0][ordinary[0].index("-arch") + 1], "arm64")
        executed = subprocess.run([str(self.output)], capture_output=True, timeout=5)
        self.assertEqual(executed.returncode, 0, executed.stderr.decode())
        self.assertEqual(executed.stderr, b"")
        result = probe.validate_execution(executed.stdout, executed.returncode,
                                          vectors.counts(vectors.corpus()))
        self.assertEqual(sum(case["checked"] for case in result["cases"].values()), 7008)

    def test_vector_generator_failure_stops_before_compilation_or_success_app(self):
        (self.scripts / "m1_compiler_rt_vectors.py").write_text(
            "raise RuntimeError('injected native vector generation failure')\n")
        process = self.build()
        self.assertNotEqual(process.returncode, 0)
        self.assertIn(b"injected native vector generation failure", process.stderr)
        self.assertEqual(self.invocations("m1_compiler_rt_probe"), [])
        self.assertFalse(self.output.exists())
        self.assertNotIn(b"All fixture binaries rebuilt from source.", process.stdout)

    def test_linker_and_verification_failures_do_not_publish_success_app(self):
        main = self.fixture / "main.c"
        original = main.read_text()
        source = original.replace(
            "int main(void) {",
            "extern void m1_missing_native_dependency(void);\n"
            "int main(void) {\n    m1_missing_native_dependency();", 1)
        main.write_text(source)
        with self.subTest(operation="real host linker failure"):
            process = self.build()
            self.assertEqual(process.returncode, 1)
            self.assertIn(b"m1_missing_native_dependency", process.stderr)
            self.assertEqual(len(self.invocations("m1_compiler_rt_probe")), 1)
            self.assertFalse(self.output.exists())
            self.assertNotIn(b"All fixture binaries rebuilt from source.", process.stdout)
        main.write_text(original)
        self.log.unlink()
        self.xcrun_log.unlink()
        sentinel = b"preserve the previously published app\n"
        self.output.write_bytes(sentinel)
        bad_verification_arguments = (
            (["lipo", "-verify_arch", "arm64", "x86_64", str(self.output)],
             b"one input before -verify_arch"),
            (["lipo", str(self.output), "-verify_arch", "arm64", "x86_64", str(self.output)],
             b"only architectures after -verify_arch"),
        )
        for arguments, diagnostic in bad_verification_arguments:
            with self.subTest(operation="reject misplaced lipo input", arguments=arguments):
                process = subprocess.run([str(self.tools / "xcrun"), *arguments],
                                         env=self.env, capture_output=True, timeout=5)
                self.assertEqual(process.returncode, 1)
                self.assertIn(diagnostic, process.stderr)
                self.assertEqual(self.output.read_bytes(), sentinel)
        self.env["NATIVE_TEST_FAIL_VERIFY"] = "1"
        with self.subTest(operation="simulated lipo verification failure"):
            process = self.build()
            self.assertEqual(process.returncode, 69)
            self.assertIn(b"injected lipo verification failure", process.stderr)
            self.assertEqual(len(self.invocations("m1_compiler_rt_probe")), 2)
            self.assertEqual(self.output.read_bytes(), sentinel)
            self.assertNotIn(b"All fixture binaries rebuilt from source.", process.stdout)
        self.env.pop("NATIVE_TEST_FAIL_VERIFY")
        self.output.unlink()
        for symlink in (False, True):
            destination = self.root / "existing app directory" if symlink else self.output
            destination.mkdir()
            (destination / "keep").write_bytes(sentinel)
            if symlink:
                self.output.symlink_to(destination, target_is_directory=True)
            self.log.unlink()
            self.xcrun_log.unlink()
            with self.subTest(operation="directory destination", symlink=symlink):
                process = self.build()
                self.assertEqual(process.returncode, 1)
                self.assertIn(b"app destination is a directory", process.stderr)
                self.assertEqual(self.invocations("m1_compiler_rt_probe"), [])
                self.assertEqual((destination / "keep").read_bytes(), sentinel)
                self.assertEqual(sorted(path.name for path in destination.iterdir()), ["keep"])
                self.assertEqual(self.output.is_symlink(), symlink)
                self.assertNotIn(b"All fixture binaries rebuilt from source.", process.stdout)
            if symlink:
                self.output.unlink()
            else:
                (destination / "keep").unlink()
                destination.rmdir()


if __name__ == "__main__":
    unittest.main()
