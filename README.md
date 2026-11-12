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
- GUI application framework with software-rendered windowing (macOS + Linux display backends)
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

### Run in Docker

```bash
# Build the Docker image for the host CPU by default
docker build -f docker/Dockerfile.burdettie -t burdettie-os .

# Force the reference ARM64 lane when you want it explicitly
docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .

# Run a tool (e.g., sw_vers)
docker run --rm burdettie-os \
  macho-inspect --exec /opt/xcode-tools/system/sw_vers

# Run the full Xcode tool compatibility suite (67/72 pass)
docker run --rm burdettie-os test-xcode-tools

# Start the Burdettie home screen with a browser viewer on http://127.0.0.1:6080
./scripts/docker-gui.sh

# Drop into a GUI-capable shell instead
./scripts/docker-gui.sh --shell

# Run a guest Cocoa app against the Docker display stack instead of the home screen
RUST_LOG=info BURDETTIE_GUI_MODE=guest BURDETTIE_GUEST_APP=/workspace/fixtures/cocoa_draw/app ./scripts/docker-gui.sh

# Force the GUI helper onto the ARM64 reference lane when needed
BURDETTIE_DOCKER_PLATFORM=linux/arm64 ./scripts/docker-gui.sh

# Compose example: defaults to the Burdettie home screen
docker compose -f docker/compose.burdettie-gui.example.yaml up

# Switch the Compose service into guest-app mode
BURDETTIE_GUI_MODE=guest \
  BURDETTIE_GUEST_APP=/opt/imported-artifacts/apps/Stickies.app/Contents/MacOS/Stickies \
  docker compose -f docker/compose.burdettie-gui.example.yaml up

# Launch an imported open-source terminal bundle
BURDETTIE_GUI_MODE=guest \
  BURDETTIE_GUEST_APP=/opt/imported-artifacts/apps/WezTerm.app/Contents/MacOS/wezterm-gui \
  docker compose -f docker/compose.burdettie-gui.example.yaml up
```

Fast dev loop (~3s incremental):

```bash
./scripts/linux-test.sh  # zigbuild + volume-mount into running container
```

The GUI Docker path uses Xvfb plus optional VNC/noVNC inside `docker/Dockerfile.burdettie`.
The default session now opens the native Launchpad home screen so the viewer is not left on an empty
desktop after a short-lived smoke app exits. Short-lived fixtures like `fixtures/cocoa_draw/app` are still
useful in `BURDETTIE_GUI_MODE=guest`, and imported app bundles can use that same guest mode once staged.
When `/opt/imported-artifacts/manifest.json` is mounted, or when local dev stages bundles under `imports/apps`,
Launchpad now auto-discovers those imported GUI apps and lists them on the desktop rail.

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

**Docker runtime image**:
- Docker build/run support for your host CPU architecture
- Optional: `--platform linux/arm64` or `BURDETTIE_DOCKER_PLATFORM=linux/arm64` for the ARM64 reference lane

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
