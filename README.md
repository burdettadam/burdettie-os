# burdettie-os

A Rust-based macOS runtime that parses, loads, and executes macOS Mach-O
binaries — on macOS and on Linux/ARM64.

## What this is

A **macOS runtime laboratory** — a self-contained reimplementation of Apple's
loader (`dyld`), ObjC runtime, and Foundation framework that can run real
Objective-C/Foundation command-line tools, including Xcode CLI tools, without
needing macOS.

Current capabilities:

- Parse any Mach-O binary (thin or fat/universal, arm64 and arm64e with PAC)
- Map, fixup, and execute binaries in-process with full chained-fixup and PAC pointer signing
- ObjC runtime: 95 synthetic classes, selectors, `objc_msgSend` (ARM64 assembly trampoline), categories, blocks, protocols, super dispatch, dynamic class creation
- Foundation-lite: 95 classes (NSString, NSArray, NSDictionary, NSNumber, NSData, NSDate, NSURL, NSTask, NSURLSession, NSRegularExpression, CoreAnimation, and many more)
- 430+ shims across Foundation, libsystem, CoreGraphics, CoreAnimation, and variadic ABI bridging
- 84 fixture binaries execute end-to-end (100% compat gate)
- 39 corpus tools pass (100%)
- 67/72 Xcode CLI tools run on Linux/ARM64 via Docker (93% pass rate)
- ARM64 interpreter with ~122 instruction variants and automatic host FFI fallback
- GUI application framework with software-rendered windowing (macOS)
- Interactive runtime shell with compatibility scorecard
- Performance: typical loader overhead < 100µs per tool (3× faster than v0.1.0)

## Workspace crates

41 crates + 5 demo applications across a 46-member workspace.

### Loader core

| Crate | Purpose |
|---|---|
| `macho-types` | Pure data structures for Mach-O concepts |
| `macho-parse` | Binary parser — bytes in, `ParsedMachO` out |
| `macho-map` | Virtual memory mapping planner |
| `macho-resolve` | Dependency and symbol resolution planner |
| `macho-objc` | ObjC metadata parsing + initializer extraction |
| `macho-exec` | In-process mmap, fixup application, PAC signing, TLV, entry/constructor calls |
| `trust-policy` | 8-gate security evaluation (quarantine, codesign, AMFI, SIP, etc.) |
| `trust-eval` | X.509 certificate chain validation (rustls-webpki + Mozilla roots) |
| `macho-run-plan` | Top-level execution plan generator |
| `macho-diff` | Compare computed plans against host observations |
| `macho-shared-cache` | dyld shared cache parser (extract dylibs from system cache) |
| `macho-fixtures` | Fixture metadata and expected outputs |
| `macho-inspect` | CLI tool for Mach-O inspection and execution |
| `test-harness` | End-to-end orchestration |

### Runtime layer

| Crate | Purpose |
|---|---|
| `macho-runtime` | Orchestration hub — typestate loader pipeline, Foundation template caching, perf-optimized |
| `libsystem-shims` | Process, TLS, dispatch, pthread, mach, socket, random, and cross-platform translation (stat, dirent, variadic ABI, kqueue) |
| `objc-runtime` | ObjC runtime — class table, selector interning, `objc_msgSend` (ARM64 asm), categories, blocks, protocols, dynamic classes, +load |
| `cf-lite` | Foundation-lite — 95 classes including NSString, NSArray, NSDictionary, NSURLSession, NSRegularExpression, NSTask, CoreAnimation (CALayer, CATransaction), and more |
| `core-foundation-lite` | `#[repr(C)]` CF/CG geometry types (CGPoint, CGRect, NSRange, NSEdgeInsets) |
| `cg-shims` | ~80 CoreGraphics C API stubs |
| `icu-shims` | ICU4X-backed i18n shims (collation, locale, segmentation) |
| `libdispatch-sys` | Raw FFI bindings to Apple's libdispatch/GCD |
| `platform-core` | Platform abstraction layer (10 traits: MemoryMapper, HostSymbolResolver, Clock, etc.) |
| `platform-darwin-host` | macOS host platform implementation |
| `platform-linux` | Linux platform implementation |
| `runtime-diagnostics` | DiagRing circular event buffer with per-category counters |
| `runtime-demo` | Demo fixture metadata and discovery |
| `runtime-shell` | Interactive demo shell — CLI + GUI launcher |
| `compat-runner` | `darwin-compat scan/run` CLI for structured compatibility reports |
| `perf-bench` | Performance benchmarking vs Apple native (JSON output, warmup, filter) |

### Tooling

| Crate | Purpose |
|---|---|
| `arm64-translate` | ARM64 interpreter — ~122 instruction variants (integer + FP/SIMD), auto-FFI fallback |
| `swift-stub-gen` | Generate Rust shim stubs for Swift-mangled symbols in Mach-O frameworks |
| `darwin-runtime-ffi` | C-ABI dynamic library for embedding from C, Swift, or Python |

### GUI layer

| Crate | Purpose |
|---|---|
| `graphics-core` | 2D drawing primitives, software renderer, bitmap font, gradients, bezier paths |
| `window-backend` | Platform-agnostic windowing abstraction |
| `window-backend-darwin` | macOS backend (winit + softbuffer) |
| `window-backend-linux` | Linux windowing backend |
| `runloop-core` | Timer-based run loop |
| `app-kit-lite` | Application lifecycle, responder chain, event dispatch, pasteboard |
| `ui-view` | View tree, hit testing, constraint-based layout |
| `ui-controls` | Label, Button, TextField, ScrollView, StackView, TableView, SplitView |

### Demo applications

| App | Purpose |
|---|---|
| `hello-darwin` | Lifecycle demo — runs fixture scorecard |
| `control-gallery` | Widget exercise — text input, buttons, labels |
| `foundation-explorer` | Browse 95 Foundation classes and their methods |
| `launchpad` | Finder-style grid launcher for fixtures, corpus tools, and GUI apps |
| `file-explorer` | Two-pane file browser (sidebar + directory listing) |

## Quick start

```bash
# Build everything
cargo build --workspace

# Build fixture binaries
./scripts/build_fixtures.sh

# Inspect a Mach-O binary
cargo run -p macho-inspect -- fixtures/simple_cli/app

# Execute a binary
cargo run -p macho-inspect -- --exec fixtures/simple_cli/app

# JSON output
cargo run -p macho-inspect -- --json fixtures/simple_cli/app
```

## Try it yourself

```bash
# Build and run the interactive runtime shell
cargo run -p runtime-shell

# Inside the shell:
darwin> score             # compatibility scorecard (84/84)
darwin> run hello         # run a fixture
darwin> gui               # list GUI apps
darwin> gui hello-darwin  # launch a GUI app
```

### Run on Linux/ARM64 via Docker

```bash
# Cross-compile for Linux (requires: brew install zig, cargo install cargo-zigbuild)
cargo zigbuild --release --target aarch64-unknown-linux-gnu.2.36 -p macho-inspect
cp target/aarch64-unknown-linux-gnu/release/macho-inspect docker/macho-inspect-linux-arm64

# Build the Docker image
docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .

# Run a tool (e.g., sw_vers)
docker run --rm --platform linux/arm64 burdettie-os \
  macho-inspect --exec /opt/xcode-tools/system/sw_vers

# Run the full Xcode tool compatibility suite (67/72 pass)
docker run --rm --platform linux/arm64 burdettie-os test-xcode-tools
```

Fast dev loop (~3s incremental):

```bash
./scripts/linux-test.sh  # zigbuild + volume-mount into running container
```

### Package as a demo image

```bash
./packaging/build.sh --release
./packaging/bundle-runtime.sh --release
./packaging/make-vm-image.sh          # creates burdettie-os.dmg
```

See [docs/DEMO.md](docs/DEMO.md) for the full demo guide.

## Development phases

1. **Parse** — `macho-types` + `macho-parse` + `macho-inspect` ✅
2. **Plan** — `macho-map` + `macho-run-plan` ✅
3. **Resolve** — `macho-resolve` + fixture dependency tests ✅
4. **Compare** — host observation capture + `macho-diff` ✅
5. **ObjC** — metadata detection + init modeling ✅
6. **Execute** — controlled in-process mapping for tiny fixtures ✅
7. **Runtime** — ObjC runtime, Foundation-lite, shims, 84 fixtures ✅
8. **Application Layer** — GUI framework, demo apps, interactive shell ✅
9. **Demo Image** — Bootable VM with CLI + GUI showcase ✅
10. **Cross-Platform** — Linux/ARM64 Docker image, 67/72 Xcode tools ✅

## Milestone

> **Self-contained macOS runtime for Objective-C/Foundation CLI software,
> running on both macOS and Linux/ARM64**

Key numbers:

| Metric | Value |
|---|---|
| Fixture binaries (compat gate) | 84/84 (100%) |
| Corpus tools | 39/39 (100%) |
| Xcode CLI tools (Docker/Linux) | 67/72 (93%) |
| Synthetic Foundation classes | 95 |
| Total shims | 430+ |
| ARM64 interpreter instructions | ~122 variants |
| Unit tests | 2,600+ (cf-lite) + 400+ (arm64-translate) + 196 (objc-runtime) + more |

See [docs/FEATURES.md](docs/FEATURES.md) for the full feature reference and
[docs/ROADMAP.md](docs/ROADMAP.md) for the strategic roadmap and target matrix.

## Prerequisites

**macOS (native)**:
- macOS 13+ (Ventura or later)
- Rust stable (see `rust-toolchain.toml`)
- Xcode Command Line Tools (`xcode-select --install`)

**Linux/ARM64 (Docker)**:
- Docker with `--platform linux/arm64` support
- For cross-compilation: `zig`, `cargo-zigbuild`, `rustup target add aarch64-unknown-linux-gnu`

## Use as a library

The runtime exposes a C-ABI dynamic library (`libdarwin_runtime_ffi.dylib`) that
can be called from C, Swift, Python, or any language with C FFI support.

### Install

```bash
# From source
./packaging/install.sh

# Or via Homebrew
brew install --build-from-source burdettie-os/tap/darwin-runtime
```

### C

```c
#include <darwin_runtime.h>

DarwinVersion v = darwin_runtime_version();
printf("darwin-runtime %u.%u.%u\n", v.major, v.minor, v.patch);

DarwinRuntimeResult r = darwin_runtime_run("/path/to/binary");
printf("exit code: %d, images: %zu\n", r.exit_code, r.images_loaded);
```

Build with `cc -o app app.c $(pkg-config --cflags --libs darwin-runtime)`.

### Swift

```swift
import DarwinRuntime

let v = DarwinRuntime.version
let result = try DarwinRuntime.run(path: "/path/to/binary")
print("Classes: \(ObjCRuntime.classCount)")
```

Add the `swift/` directory as an SPM dependency.

See [docs/FFI_GUIDE.md](docs/FFI_GUIDE.md) for the full API reference, Python
examples, memory management rules, and thread safety notes.
