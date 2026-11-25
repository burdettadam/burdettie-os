# Test fixture sources

These small C and Objective-C programs exercise Mach-O parsing, linking,
Darwin runtime behavior and GUI shims. The source files belong in the
repository; their compiled executables, dylibs and framework bundles are
local build outputs.

Build on macOS with the Xcode command-line tools:

```sh
./scripts/build_fixtures.sh
```

The script builds ARM64 fixtures, an ARM64e fixture, and the explicit ARM64 /
x86_64 universal-binary case. It discovers ordinary `main.c` and `main.m`
programs and keeps dedicated recipes for dylib chains, categories and the
framework fixture. Compiler or linker errors stop the build. GUI fixtures
are compiled without launching their windows.

The multi-source `m1_compiler_rt_probe` also has an explicit recipe. It generates
the deterministic arithmetic vectors beside fresh copies of its sources, then
builds and verifies a universal ARM64 + x86_64 `app`. The ARM64 slice keeps it in
the existing native runtime compatibility gate; the x86_64 slice is a macOS SDK
reference build. Compiler, generator and slice-verification failures stop the
build. Failed temporary work remains available for diagnosis. This reference
does not qualify the selected PureDarwin libraries, guest or runtime ABI.

Most outputs are named `app`. The historical `return42`, `process_info`,
`dispatch_test`, `objc_minimal`, and `objc_category` executable names remain
available, with `app` symlinks for consumers that expect a common name.

The four-byte `malformed/bad_magic` and sixteen-byte
`malformed/truncated_header` files are intentional parser inputs. They are
kept in version control, and the build script preserves their exact bytes.

For Linux interpreter tests, build these inputs on macOS and transfer the
generated fixtures while preserving executable modes and framework
symlinks. CI performs that transfer as a tar archive. Apple SDKs and system
framework binaries are not part of the fixture archive.
