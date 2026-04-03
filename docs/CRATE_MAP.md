# Darwin Loader Lab — Crate Map

> Current state as of 2026-04-03. See [FEATURES.md](FEATURES.md) for the full
> feature index and [roadmap.md](roadmap.md) for what comes next.

44 workspace members (41 crates + 3 GUI apps), organized by layer.

---

## Layer 1 — Loader Core (stable, frozen API)

| Crate | Role |
|---|---|
| `macho-types` | All Mach-O data structures: headers, load commands, segments, sections, reloc types |
| `macho-parse` | Binary parser: headers, segments, load commands, symbols, ObjC metadata, code signature |
| `macho-map` | Segment mapping planner: computes mmap calls, protection flags, ASLR slide |
| `macho-resolve` | Dependency resolution: @rpath/@loader_path/@executable_path, transitive BFS, cycle detection |
| `macho-objc` | ObjC metadata extraction (read-only): classlist, catlist, protolist, selrefs, imageinfo |
| `macho-exec` | mmap execution: segment mapping, fixup application (classic + chained), constructor dispatch, entry call |
| `trust-policy` | Security gate evaluation: 8-gate policy pipeline, `TrustPolicy` composition, `SecurityGate` trait |
| `macho-run-plan` | Execution plan generation and display (for CLI inspection and planning) |
| `macho-diff` | Field-by-field binary comparison against host behavior |
| `macho-fixtures` | Fixture metadata: expected exit codes, shim assertions, compat scoring |
| `test-harness` | Integration test orchestration: runtime-level test scenarios |
| `macho-inspect` | CLI: inspect, run, diff, plan subcommands |

## Layer 2 — Runtime Layer

| Crate | Role |
|---|---|
| `macho-runtime` | Orchestration hub: `RuntimeConfig`, `RuntimeContext`, `Runtime::load_and_run()`, full pipeline coordination |
| `objc-runtime` | ObjC runtime: class table, selector interning, msgSend trampolines (ARM64 + x86_64), category merge, +load, exceptions, KVO/KVC/swizzling |
| `cf-lite` | CoreFoundation / Foundation shim layer: NSObject, NSString, NSArray, all Foundation classes, CF toll-free bridging, NSLog |
| `libsystem-shims` | libSystem / libpthread / libdispatch / libmach shims: 65 shimmed functions, ShimTable |

## Layer 3 — Platform Abstraction

| Crate | Role |
|---|---|
| `platform-core` | All 10 PAL trait definitions and `PlatformProvider` composition struct |
| `platform-darwin-host` | macOS implementation of all PAL traits (`DarwinHostPlatform`) |
| `platform-linux` | Linux implementation of all PAL traits (`LinuxHostPlatform`) |

## Layer 4 — Cross-Platform Translation

| Crate | Role |
|---|---|
| `arm64-translate` | Pure-Rust ARM64 interpreter: decoder (55+ instructions), CPU state, memory model, FFI bridge, fetch-decode-execute loop |
| `compat-runner` | Compatibility test runner: runs fixture suite, captures exit code + output, computes compat score |

## Layer 5 — GUI Framework

| Crate | Role |
|---|---|
| `graphics-core` | Text layout (ab_glyph), software renderer, GPU renderer (wgpu, optional), drawing primitives (BezierPath, Gradient, ImageBuffer) |
| `runloop-core` | Run loop abstraction: event dispatch, timer management, mode constants |
| `app-kit-lite` | Auto Layout Lite: constraint solver, layout pass, intrinsic sizes, conflict detection |
| `ui-view` | View tree: frame/bounds, subview management, display scheduling |
| `ui-controls` | 7 built-in controls: Label, Button, TextField, ScrollView, StackView, TableView, SplitView |
| `window-backend` | Platform-agnostic window backend trait |
| `window-backend-darwin` | macOS window backend implementation |

## Layer 6 — Tooling & Development

| Crate | Role |
|---|---|
| `runtime-shell` | Interactive runtime REPL for debugging: `load`, `run`, `hostfallback`, `shims` commands |
| `runtime-demo` | Demo binary: demonstrates runtime pipeline with example fixtures |
| `runtime-diagnostics` | Lock-free diagnostic ring buffer: resource-request logging, cycle detection for hangs |
| `darwin-runtime-ffi` | C ABI wrapper (cbindgen): exposes runtime as `libdarwin_runtime` for C/Swift consumers |

## GUI Applications

| App | Description |
|---|---|
| `apps/hello-darwin` | Minimal "Hello World" GUI app using the native GUI framework |
| `apps/control-gallery` | Gallery app showcasing all 7 built-in UI controls |
| `apps/foundation-explorer` | Interactive Foundation class browser |
