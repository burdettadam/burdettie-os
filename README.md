# burdettie-os

A Rust-based non-GUI macOS runtime for studying, reproducing, and executing macOS Mach-O binaries.

## What this is

A **macOS runtime laboratory** — building the smallest believable non-GUI macOS
runtime that can run ordinary Objective-C/Foundation command-line software.

Current capabilities:

- Parse any Mach-O binary (thin or fat/universal)
- Model what Apple's loader (`dyld`) would do
- Map, fixup, and execute binaries in-process
- Provide ObjC runtime (classes, selectors, msgSend, categories, blocks, super dispatch)
- Provide Foundation-lite (40 classes: NSString, NSArray, NSDictionary, NSNumber, NSObject, NSData, NSDate, NSURL, and more)
- 62 fixture binaries execute end-to-end, 879+ tests, 0 failures
- GUI application framework with software-rendered windowing
- Interactive runtime shell with compatibility scorecard

## Workspace crates

### Loader core (frozen — no new features unless a target binary needs them)

| Crate | Purpose |
|---|---|
| `macho-types` | Pure data structures for Mach-O concepts |
| `macho-parse` | Binary parser — bytes in, `ParsedMachO` out |
| `macho-map` | Virtual memory mapping planner |
| `macho-resolve` | Dependency and symbol resolution planner |
| `macho-objc` | ObjC metadata parsing + initializer extraction |
| `macho-exec` | In-process mmap, fixup application, entry/constructor calls |
| `trust-policy` | 8-gate security evaluation (quarantine, codesign, AMFI, SIP, etc.) |
| `macho-run-plan` | Top-level execution plan generator |
| `macho-diff` | Compare computed plans against host observations |
| `macho-fixtures` | Fixture metadata and expected outputs |
| `macho-inspect` | CLI tool for human-readable Mach-O inspection |
| `test-harness` | End-to-end orchestration |

### Runtime layer (active development)

| Crate | Purpose |
|---|---|
| `macho-runtime` | Orchestration hub — `Runtime::load_and_run()` full pipeline |
| `libsystem-shims` | 59 shims: process, TLS, dispatch, pthread, mach, lifecycle |
| `objc-runtime` | ObjC runtime — class table, selector interning, `objc_msgSend`, category merge, blocks, +load |
| `cf-lite` | Foundation-lite — 40 classes: NSString, NSArray, NSDictionary, NSNumber, NSObject, NSData, and more |
| `platform-core` | Platform abstraction layer |
| `platform-darwin-host` | macOS host platform implementation |
| `runtime-demo` | Demo fixture metadata and discovery |
| `runtime-shell` | Interactive demo shell — CLI + GUI launcher |

### GUI layer

| Crate | Purpose |
|---|---|
| `graphics-core` | 2D drawing primitives, software renderer, bitmap font |
| `window-backend` | Platform-agnostic windowing abstraction |
| `window-backend-darwin` | macOS backend (winit + softbuffer) |
| `runloop-core` | Timer-based run loop |
| `app-kit-lite` | Application lifecycle, responder chain, event dispatch |
| `ui-view` | View tree, hit testing, layout |
| `ui-controls` | Label, Button, TextField controls |

### Demo applications

| App | Purpose |
|---|---|
| `hello-darwin` | Minimal lifecycle demo — runs fixture scorecard |
| `control-gallery` | Widget exercise — text input, buttons, labels |
| `foundation-explorer` | Browse 40 Foundation classes and their methods |

## Quick start

```bash
# Build everything
cargo build --workspace

# Build fixture binaries
./scripts/build_fixtures.sh

# Inspect a Mach-O binary
cargo run -p macho-inspect -- fixtures/simple_cli/app

# JSON output
cargo run -p macho-inspect -- --json fixtures/simple_cli/app
```

## Try it yourself

```bash
# Build and run the interactive runtime shell
cargo run -p runtime-shell

# Inside the shell:
darwin> score         # compatibility scorecard (62/62)
darwin> run hello     # run a fixture
darwin> gui           # list GUI apps
darwin> gui hello-darwin  # launch a GUI app
```

To package as a bootable demo image:

```bash
./packaging/build.sh --release
./packaging/bundle-runtime.sh --release
./packaging/make-vm-image.sh          # creates burdettie-os.dmg
```

Or build a Docker image (CLI only):

```bash
./packaging/bundle-runtime.sh --release
docker build -f docker/Dockerfile.demo -t burdettie-os-demo dist/
docker run -it --rm burdettie-os-demo
```

See [docs/DEMO.md](docs/DEMO.md) for the full demo guide.

## Development phases

1. **Parse** — `macho-types` + `macho-parse` + `macho-inspect` ✅
2. **Plan** — `macho-map` + `macho-run-plan` ✅
3. **Resolve** — `macho-resolve` + fixture dependency tests ✅
4. **Compare** — host observation capture + `macho-diff` ✅
5. **ObjC** — metadata detection + init modeling ✅
6. **Execute** — controlled in-process mapping for tiny fixtures ✅
7. **Runtime** — ObjC runtime, Foundation-lite, shims, 62 fixtures ✅
8. **Application Layer** — GUI framework, demo apps, interactive shell ✅
9. **Demo Image** — Bootable VM with CLI + GUI showcase ✅

## Milestone

> **Usable non-GUI macOS runtime for Objective-C/Foundation CLI software**

See [docs/FEATURES.md](docs/FEATURES.md) for the full feature reference and
[docs/ROADMAP.md](docs/ROADMAP.md) for the strategic roadmap and target matrix.

## Prerequisites

- macOS 13+ (Ventura or later)
- Rust stable (see `rust-toolchain.toml`)
- Xcode Command Line Tools (`xcode-select --install`)

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
