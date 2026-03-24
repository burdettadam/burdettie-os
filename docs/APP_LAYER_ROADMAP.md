# Application Layer & GUI Roadmap — Phases 30–38

> **Status: ✅ ALL PHASES COMPLETE** — All 9 phases (30–38) completed successfully.
> See [EXPANSION_ROADMAP.md](EXPANSION_ROADMAP.md) for future directions.

> **One-line plan**: Build a portable macOS-compatible userspace core, package it
> with Docker for reproducible builds, and ship a bootable Darwin-hosted VM demo
> that proves CLI, Foundation, and first-stage GUI interoperability.

---

## Current baseline (final state)

| Metric | Value |
|---|---|
| Crates | 30 (27 crates + 3 apps) |
| Foundation classes | 40 |
| Shim functions | 59 (libsystem-shims) |
| Fixtures | 63 (62 executable) |
| Tests | 900+ passing, 0 failing |
| Clippy warnings | 0 |
| TODOs in source | 0 |
| Compatibility | 62/62 (100%) |
| GUI apps | 3 (hello-darwin, control-gallery, foundation-explorer) |

---

## Host coupling audit — what must be abstracted

Every direct host dependency, grouped by abstraction surface:

### Memory mapping
| Call | Files | Notes |
|---|---|---|
| `libc::mmap` | `macho-exec/src/mapped.rs` | Single contiguous anonymous region per image |
| `libc::mprotect` | `macho-exec/src/mapped.rs` | Per-segment protection after fixups |
| `libc::munmap` | `macho-exec/src/mapped.rs` | Cleanup on `MappedImage::drop` |
| `libc::sysconf(_SC_PAGESIZE)` | `macho-exec/src/mapped.rs` | Page-aligned allocation |

### Symbol resolution & dynamic loading
| Call | Files | Notes |
|---|---|---|
| `libc::dlsym(RTLD_DEFAULT, ...)` | `macho-exec/src/fixup/resolver.rs`, `objc-runtime/src/shims/dispatch.rs`, `objc-runtime/src/shims/lifecycle.rs`, `cf-lite/src/nssortdescriptor.rs`, `cf-lite/src/foundation/mod.rs` | Pervasive — 20+ callsites |
| `libc::dlopen(Foundation)` | `cf-lite/src/foundation/mod.rs` | Loads host Foundation for NSException EHTYPE |

### Unwind registration
| Call | Files | Notes |
|---|---|---|
| `__unw_add_find_dynamic_unwind_sections` | `macho-exec/src/unwind.rs` | macOS libunwind-specific |

### Assembly trampolines (arch-locked to ARM64)
| File | Purpose |
|---|---|
| `objc-runtime/src/trampoline.s` | `objc_msgSend`, `objc_msgSendSuper`, `objc_msgSendSuper2` |
| `cf-lite/src/nslog_trampoline.s` | Variadic `NSLog` argument shuffling |
| `cf-lite/src/format_trampoline.s` | `+[NSString stringWithFormat:]` variadic |

### System services
| Call | Files | What |
|---|---|---|
| `libc::mach_task_self` | `libsystem-shims/src/mach.rs` | Mach task port |
| `libc::mach_absolute_time` | `libsystem-shims/src/mach.rs` | Monotonic clock |
| `libc::gettimeofday` | `cf-lite/src/nsdate.rs`, `cf-lite/src/nstimezone.rs` | Wall clock |
| `libc::localtime_r` | `cf-lite/src/nstimezone.rs`, `cf-lite/src/nscalendar.rs` | Timezone |
| `libc::readlink("/etc/localtime")` | `cf-lite/src/nstimezone.rs` | IANA tz name |
| `libc::arc4random_buf` | `cf-lite/src/nsuuid.rs` | CSPRNG |
| `libc::getpid` | `cf-lite/src/nsprocessinfo.rs` | Process identity |
| `CC_SHA256` | `macho-parse/src/codesig.rs` | Code signature hashing |

### Heap allocation
| Pattern | Scope |
|---|---|
| `libc::calloc` / `libc::malloc` / `libc::free` | All 35 cf-lite classes + objc-runtime lifecycle |

### Filesystem
| Call | Files |
|---|---|
| `std::fs::read` | `macho-resolve`, `macho-inspect`, `macho-runtime` (dylib loading) |

---

## Crate evolution plan

### Current → Target mapping

```
CURRENT (16 crates)              TARGET (26+ crates)
═══════════════════              ════════════════════

macho-types ──────────────────── macho-types            (stable, unchanged)
macho-parse ──────────────────── macho-parse            (stable, swap CC_SHA256 → pal)
macho-map ────────────────────── macho-map              (stable, unchanged)
macho-resolve ────────────────── macho-resolve           (swap fs::read → pal)
macho-objc ───────────────────── macho-objc             (stable, unchanged)
macho-run-plan ───────────────── macho-run-plan          (stable, unchanged)
macho-diff ───────────────────── macho-diff             (stable, unchanged)
macho-fixtures ───────────────── macho-fixtures          (stable, unchanged)
trust-policy ─────────────────── trust-policy           (stable, unchanged)
macho-exec ───────────────────── macho-exec             (inject pal traits for mmap/unwind/dlsym)
libsystem-shims ──────────────── libsystem-shims        (inject pal traits for mach/pthread)
objc-runtime ─────────────────── objc-runtime           (inject pal trait for host ObjC bridge)
cf-lite ──────────────────────── cf-lite                (inject pal traits for time/rng/fs)
macho-runtime ────────────────── macho-runtime           (wires pal impls into pipeline)
macho-inspect ────────────────── macho-inspect           (unchanged, macOS-only CLI)
test-harness ─────────────────── test-harness           (unchanged)

                                 NEW CRATES:
                                 ──────────
                                 platform-core          (trait definitions only)
                                 platform-darwin-host   (macOS impl of all traits)
                                 runloop-core           (event loop, timer, dispatch)
                                 app-kit-lite           (NSApp, responder chain, menus)
                                 ui-view                (view tree, layout, hit testing)
                                 ui-controls            (label, button, text field)
                                 window-backend         (trait: open/close/resize/present)
                                 window-backend-darwin  (macOS CALayer/Metal or host NSWindow)
                                 graphics-core          (2D drawing primitives)
                                 runtime-shell          (boot banner, demo runner)
                                 runtime-demo           (demo app collection)
```

### Dependency flow (target state)

```
                    platform-core (traits only)
                         │
              ┌──────────┼──────────┐
              ▼                     ▼
    platform-darwin-host    (future: platform-linux)
              │
    ┌─────────┼──────────────────────────────────┐
    ▼         ▼              ▼                   ▼
macho-exec  libsystem     objc-runtime        cf-lite
    │        shims            │                  │
    │         │               │                  │
    └─────────┴───────┬───────┘                  │
                      ▼                          │
               macho-runtime ◄───────────────────┘
                      │
         ┌────────────┼─────────────┐
         ▼            ▼             ▼
   runloop-core   app-kit-lite   graphics-core
         │            │             │
         ▼            ▼             ▼
      ui-view ◄── ui-controls   window-backend
         │                          │
         │            ┌─────────────┘
         ▼            ▼
   window-backend-darwin
         │
         ▼
   runtime-shell / runtime-demo
```

---

## Phase 30 — Freeze Runtime Core

**Goal**: Declare the current 16-crate runtime stable. No more feature additions
to the core loader/runtime; only bug fixes from here.

### Tasks

- [x] Create `docs/STABLE_API.md` documenting public API surface of each crate
- [x] Add `#[doc(hidden)]` or visibility restrictions to internal implementation details
- [x] Create `docs/HOST_DEPENDENCY_REGISTER.md` from the audit above — every host coupling with its file, line, and abstraction category
- [x] Tag version `0.1.0` semantically on all 16 crates (already in `Cargo.toml`)
- [x] Add integration test: `compat_score` must remain 62/62 across all future changes (regression gate)
- [x] Define versioned schema for `RuntimeTracer` JSON export
- [x] Freeze fixture set: document the 62 fixtures as the v1 compatibility baseline

### Acceptance

- [x] `HOST_DEPENDENCY_REGISTER.md` lists every host call with its target abstraction trait
- [x] `cargo test` still 824+ tests, 0 failures
- [x] `cargo clippy --all-targets` still 0 warnings
- [x] Compatibility score still 62/62

---

## Phase 31 — Platform Abstraction Layer (PAL)

**Goal**: Introduce a trait-based platform abstraction so the runtime can
eventually run on non-hosted stacks without source changes.

### New crates

#### `platform-core` — trait definitions only, no implementations

```rust
// platform-core/src/lib.rs

pub mod memory;    // MemoryMapper trait
pub mod symbols;   // HostSymbolResolver trait
pub mod unwind;    // UnwindRegistry trait
pub mod clock;     // Clock trait
pub mod rng;       // Rng trait
pub mod process;   // ProcessInfo trait
pub mod fs;        // FilesystemProvider trait
pub mod hash;      // Hasher trait
pub mod objc;      // HostObjCBridge trait
pub mod thread;    // ThreadPrimitives trait (future)
pub mod window;    // WindowBackend trait (Phase 35)
```

Trait sketches:

```rust
// memory.rs
pub trait MemoryMapper: Send + Sync {
    fn page_size(&self) -> usize;
    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError>;
    fn protect(&self, addr: *mut u8, size: usize, prot: Protection) -> Result<(), PalError>;
    fn unmap(&self, addr: *mut u8, size: usize) -> Result<(), PalError>;
}

// symbols.rs
pub trait HostSymbolResolver: Send + Sync {
    fn resolve(&self, name: &str) -> Option<*const ()>;
    fn resolve_in(&self, handle: *mut (), name: &str) -> Option<*const ()>;
    fn open_library(&self, path: &str) -> Option<*mut ()>;
}

// clock.rs
pub trait Clock: Send + Sync {
    fn wall_time_usec(&self) -> (i64, i64); // (sec, usec)
    fn monotonic_ns(&self) -> u64;
    fn utc_offset_sec(&self) -> i32;
    fn timezone_name(&self) -> String;
}

// rng.rs
pub trait Rng: Send + Sync {
    fn fill_bytes(&self, buf: &mut [u8]);
}

// hash.rs
pub trait Hasher: Send + Sync {
    fn sha256(&self, data: &[u8]) -> [u8; 32];
}

// fs.rs
pub trait FilesystemProvider: Send + Sync {
    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>>;
    fn exists(&self, path: &std::path::Path) -> bool;
    fn read_link(&self, path: &std::path::Path) -> std::io::Result<std::path::PathBuf>;
}

// objc.rs
pub trait HostObjCBridge: Send + Sync {
    fn msg_send_addr(&self) -> u64;
    fn msg_send_super_addr(&self) -> u64;
    fn sel_register_name(&self, name: &str) -> u64;
    fn exception_throw(&self, exc: u64) -> !;
    fn begin_catch(&self, exc: *mut ()) -> *mut ();
    fn end_catch(&self);
    fn get_class(&self, name: &str) -> u64;
    fn available(&self) -> bool;
}
```

#### `platform-darwin-host` — macOS implementation

Implements all `platform-core` traits using the existing `libc::` calls.
This is a mechanical extraction: move the actual `mmap`, `dlsym`, `gettimeofday`,
`CC_SHA256`, etc. calls into this crate.

```rust
// platform-darwin-host/src/lib.rs
pub struct DarwinHostPlatform;

impl platform_core::memory::MemoryMapper for DarwinHostPlatform { ... }
impl platform_core::symbols::HostSymbolResolver for DarwinHostPlatform { ... }
impl platform_core::clock::Clock for DarwinHostPlatform { ... }
// ... etc
```

### Integration approach

**Do not** restructure everything at once. Use **dependency injection** at the
`macho-runtime` orchestration level:

1. `platform-core` defines traits
2. `platform-darwin-host` implements them
3. `macho-runtime` constructs a `DarwinHostPlatform` and passes it to sub-crates
4. Sub-crates accept `&dyn MemoryMapper`, `&dyn Clock`, etc. in their APIs
5. Existing code moves behind the trait impls — **no behavior changes**

### Tasks

- [x] Create `crates/platform-core/` with all trait definitions
- [x] Create `crates/platform-darwin-host/` with macOS implementations
- [x] Add `platform-core` to workspace `Cargo.toml`
- [x] Add `platform-darwin-host` to workspace `Cargo.toml`
- [x] Extract `mmap/mprotect/munmap/sysconf` from `macho-exec/src/mapped.rs` → `MemoryMapper`
- [x] Extract `dlsym/dlopen` from `macho-exec/src/fixup/resolver.rs` → `HostSymbolResolver`
- [x] Extract `__unw_add/remove...` from `macho-exec/src/unwind.rs` → `UnwindRegistry`
- [x] Extract `CC_SHA256` from `macho-parse/src/codesig.rs` → `Hasher` (or replace with `sha2` crate)
- [x] Extract host ObjC bridge calls from `objc-runtime/src/shims/dispatch.rs` → `HostObjCBridge`
- [x] Extract host ObjC bridge calls from `objc-runtime/src/shims/lifecycle.rs` → `HostObjCBridge`
- [x] Extract time calls from `cf-lite/src/nsdate.rs`, `nstimezone.rs`, `nscalendar.rs` → `Clock`
- [x] Extract `arc4random_buf` from `cf-lite/src/nsuuid.rs` → `Rng`
- [x] Extract `getpid` from `cf-lite/src/nsprocessinfo.rs` → `ProcessInfo`
- [x] Extract `std::fs::read` from `macho-resolve/` → `FilesystemProvider`
- [x] Wire `RuntimeConfig` to carry a `Box<dyn PlatformProvider>` (trait object bundle)
- [x] `macho-runtime::load_and_run` passes platform provider to all sub-crates
- [x] All existing tests still pass (behavior-preserving refactor)

### Acceptance

- [x] Zero direct `libc::mmap`, `libc::dlsym`, `libc::gettimeofday`, `CC_SHA256` calls outside `platform-darwin-host`
- [x] `cargo test` still passes completely
- [x] `compat_score` still 62/62
- [x] One could **in theory** implement `platform-linux` and have the core compile

---

## Phase 32 — Reproducible Build & Package Infrastructure

**Goal**: Make the project buildable, testable, and packageable in a single command,
independent of the developer's local macOS setup.

### New artifacts

#### `docker/Dockerfile.dev`

```dockerfile
# Build/test image — uses macOS cross-compilation or native builder
FROM rust:latest
WORKDIR /workspace
COPY . .
RUN cargo build --release --all-targets
RUN cargo test --release
RUN cargo clippy --all-targets
```

> **Note**: Full runtime execution tests require macOS (ARM64 assembly, host
> forwarding). Docker image handles build + unit tests + clippy. Integration
> tests remain macOS-native.

#### `docker/Dockerfile.demo`

Packages the built runtime + fixtures + demo launcher into a distributable image.

#### `packaging/` scripts

```
packaging/
  build.sh         — full build + test + package pipeline
  bundle-runtime.sh — collects runtime binaries + fixtures into a rootfs
  make-vm-image.sh — creates bootable VM image from rootfs
```

### Tasks

- [x] Create `docker/Dockerfile.dev`
- [x] Create `docker/Dockerfile.demo`
- [x] Create `packaging/build.sh`
- [x] Create `packaging/bundle-runtime.sh`
- [x] Create `packaging/make-vm-image.sh`
- [x] Add CI configuration (GitHub Actions) for build + unit test
- [x] Document build workflow in `docs/BUILD.md`
- [x] Test that `docker build -f docker/Dockerfile.dev .` succeeds

### Acceptance

- [x] `./packaging/build.sh` produces a clean build from scratch
- [x] Docker dev image builds and runs unit tests
- [x] Docker demo image contains runtime + all 62 fixtures

---

## Phase 33 — Bootable Demo Substrate

**Goal**: Create a minimal bootable VM image that launches the runtime shell
and runs demo binaries.

### Approach: Path A (fast) → macOS guest via Virtualization.framework

Since the runtime **requires** macOS ARM64 (assembly trampolines, host forwarding),
the demo VM should be a lightweight macOS guest:

1. Use Apple's Virtualization framework (`VZVirtualMachine`)
2. Boot a minimal macOS install image
3. Auto-launch the runtime shell on login
4. Bundle fixtures + runtime binaries into a shared directory

### Alternative: Path B (lighter but limited)

Minimal Linux guest with the runtime running in cross-compilation mode.
Only works once PAL is complete and assembly trampolines have x86_64 equivalents.

### New crate

#### `vm-image/` (Swift package, not Rust)

A small Swift CLI that uses `Virtualization.framework` to:
- Create/restore a VM configuration
- Mount a shared directory with runtime binaries
- Boot the VM
- Forward stdout/stderr

### Tasks

- [x] Create `vm-image/` Swift package with `VZVirtualMachine` wrapper
- [x] Write `vm-image/Sources/main.swift` — minimal VM launcher
- [x] Create `packaging/prepare-vm-rootfs.sh` — assembles runtime + fixtures
- [x] Create shared directory layout: `/darwin-runtime/{bin,fixtures,config}`
- [x] Write auto-launch script for the VM guest (launchd plist or login item)
- [x] Document VM setup in `docs/VM_DEMO.md`
- [x] Test: `./vm-image/launch.sh` boots VM and prints runtime banner

### Acceptance

- [x] One command creates and boots a VM
- [x] VM auto-launches runtime shell
- [x] All CLI fixtures can be run inside the VM
- [x] VM can be stopped cleanly

---

## Phase 34 — Runtime Shell & Demo Launcher

**Goal**: Make the stack feel like a product. A person can interact with it
without reading source code.

### New crates

#### `runtime-shell` — interactive TUI/CLI session

```rust
// Capabilities:
// - Boot banner with version, platform, test count
// - List runnable demos with descriptions
// - Run a demo and display output + exit code
// - Show compatibility scorecard
// - Toggle trace/log verbosity
// - Toggle security policy (permissive vs Apple-like)
// - `exit` / `quit`
```

#### `runtime-demo` — demo app collection

```rust
// Pre-packaged demo scenarios:
// - "hello" — simple_cli
// - "return42" — exit code demo
// - "objc" — ObjC class creation + messaging
// - "foundation" — Foundation class exercise
// - "exceptions" — @try/@catch/@finally
// - "real-cli" — full 26-check integration
// - "scorecard" — run all 62 fixtures, print compatibility table
```

### Tasks

- [x] Create `crates/runtime-shell/` with `[[bin]]` target
- [x] Implement boot banner (version, build date, platform, test stats)
- [x] Implement demo listing (`list` command)
- [x] Implement demo runner (`run <name>` command)
- [x] Implement scorecard display (`score` command)
- [x] Implement trace toggle (`trace on/off`)
- [x] Implement policy toggle (`policy permissive/secure`)
- [x] Create `crates/runtime-demo/` with demo descriptors
- [x] Add help text and error handling
- [x] Test all shell commands

### Acceptance

- [x] `cargo run -p runtime-shell` launches interactive session
- [x] `list` shows all available demos
- [x] `run hello` executes simple_cli and shows output
- [x] `score` displays the 62/62 compatibility table
- [x] `exit` cleanly terminates

---

## Phase 35 — Application Framework Skeleton

**Goal**: Build the semantic foundation of a GUI framework — run loop, app object,
responder chain, event queue — without any visual rendering yet.

### New crates

#### `runloop-core` — event loop and timer infrastructure

```rust
pub struct RunLoop { ... }

impl RunLoop {
    pub fn main() -> &'static RunLoop;
    pub fn run(&self);                           // blocks until stopped
    pub fn stop(&self);
    pub fn perform(&self, block: Box<dyn FnOnce()>);
    pub fn schedule_timer(&self, interval: Duration, repeats: bool, f: Box<dyn FnMut()>) -> TimerToken;
    pub fn cancel_timer(&self, token: TimerToken);
}
```

#### `app-kit-lite` — application object and responder chain

```rust
pub struct NSApplication { ... }

impl NSApplication {
    pub fn shared() -> &'static NSApplication;
    pub fn run(&self);                           // enters main run loop
    pub fn terminate(&self, exit_code: i32);
    pub fn set_delegate(&self, delegate: Box<dyn AppDelegate>);
}

pub trait AppDelegate {
    fn application_did_finish_launching(&self) {}
    fn application_will_terminate(&self) {}
    fn application_should_terminate(&self) -> bool { true }
}

pub trait Responder {
    fn next_responder(&self) -> Option<&dyn Responder> { None }
    fn accepts_first_responder(&self) -> bool { false }
    fn key_down(&self, event: &Event) {}
    fn key_up(&self, event: &Event) {}
    fn mouse_down(&self, event: &Event) {}
    fn mouse_up(&self, event: &Event) {}
}
```

#### `ui-view` — view tree, layout, hit testing

```rust
pub struct View {
    pub frame: Rect,
    pub bounds: Rect,
    pub subviews: Vec<Box<dyn ViewProtocol>>,
    pub background_color: Color,
    pub hidden: bool,
    pub tag: usize,
}

pub trait ViewProtocol: Responder {
    fn frame(&self) -> Rect;
    fn set_frame(&mut self, frame: Rect);
    fn add_subview(&mut self, view: Box<dyn ViewProtocol>);
    fn remove_from_superview(&mut self);
    fn set_needs_display(&self);
    fn draw(&self, rect: Rect);              // called by display system
    fn hit_test(&self, point: Point) -> Option<&dyn ViewProtocol>;
    fn layout_subviews(&mut self) {}
}
```

#### `ui-controls` — concrete controls

```rust
pub struct Label { ... }    // text display
pub struct Button { ... }   // click target with action
pub struct TextField { ... } // text input

impl Button {
    pub fn set_title(&mut self, title: &str);
    pub fn set_action(&mut self, action: Box<dyn Fn()>);
    pub fn set_enabled(&mut self, enabled: bool);
}
```

#### `window-backend` — trait for windowing abstraction

```rust
pub trait WindowBackend: Send + Sync {
    fn create_window(&self, config: WindowConfig) -> WindowId;
    fn destroy_window(&self, id: WindowId);
    fn set_title(&self, id: WindowId, title: &str);
    fn set_size(&self, id: WindowId, width: f64, height: f64);
    fn present_surface(&self, id: WindowId, surface: &Surface);
    fn poll_events(&self) -> Vec<WindowEvent>;
    fn request_redraw(&self, id: WindowId);
}

pub enum WindowEvent {
    CloseRequested(WindowId),
    Resized(WindowId, f64, f64),
    KeyDown(WindowId, KeyCode),
    KeyUp(WindowId, KeyCode),
    MouseDown(WindowId, f64, f64, MouseButton),
    MouseUp(WindowId, f64, f64, MouseButton),
    MouseMoved(WindowId, f64, f64),
    RedrawRequested(WindowId),
}
```

#### `graphics-core` — 2D drawing primitives

```rust
pub struct Surface { ... }  // pixel buffer or GPU texture handle

pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64);
    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color);
    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64);
}

pub struct Color { pub r: f64, pub g: f64, pub b: f64, pub a: f64 }
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
pub struct Point { pub x: f64, pub y: f64 }
pub struct Font { pub name: String, pub size: f64 }
```

### Tasks

- [x] Create `crates/runloop-core/` with `RunLoop`, `TimerToken`
- [x] Create `crates/app-kit-lite/` with `NSApplication`, `AppDelegate`, `Responder`
- [x] Create `crates/ui-view/` with `View`, `ViewProtocol`
- [x] Create `crates/ui-controls/` with `Label`, `Button`, `TextField`
- [x] Create `crates/window-backend/` with `WindowBackend` trait
- [x] Create `crates/graphics-core/` with `Renderer`, `Surface`, geometry types
- [x] Implement run loop with timer support (no I/O sources yet)
- [x] Implement `NSApplication` lifecycle (singleton, `run()`, `terminate()`)
- [x] Implement responder chain walking (view → superview → window → app)
- [x] Implement view tree with `add_subview`, `remove_from_superview`, `hit_test`
- [x] Implement `Label` (stores text + font, calls `Renderer::draw_text` in `draw()`)
- [x] Implement `Button` (label + action closure + enabled state)
- [x] Write unit tests for: run loop timer dispatch, responder chain, hit testing, view layout
- [x] Write headless integration test: create app → add window → add views → simulate event → verify action fires

### Acceptance

- [x] `RunLoop` can schedule and fire timers
- [x] `NSApplication::run()` enters the run loop and processes events
- [x] Responder chain correctly walks view → window → app
- [x] Hit testing returns correct view for point
- [x] All framework semantics work **without any window backend** (headless/test mode)

---

## Phase 36 — Host-Backed GUI Backend

**Goal**: Make GUI demos work today on macOS without locking the framework to
the host forever.

### New crate

#### `window-backend-darwin` — macOS window backend

Implementation options (choose one):

**Option A — Raw Core Graphics + Quartz (lightweight)**
- Use `objc2` or raw FFI to call `NSWindow`, `NSView`, `CALayer`
- Software-rendered `CGBitmapContext` surfaces
- Input from `NSEvent`

**Option B — `winit` + `softbuffer` (portable, fast to implement)**
- `winit` handles window creation + events
- `softbuffer` provides CPU-rendered pixel buffers
- Our `Renderer` draws into the buffer
- Cross-platform from day 1

**Option C — `winit` + `wgpu` (GPU-accelerated)**
- Same as B but with GPU rendering
- More complex, better performance
- Future path to custom compositor

**Recommendation**: Start with **Option B** (`winit` + `softbuffer`). It gets
windows on screen in days not weeks, is cross-platform, and the Renderer
abstraction keeps the door open for Metal/Vulkan later.

### Tasks

- [x] Create `crates/window-backend-darwin/`
- [x] Add `winit` and `softbuffer` (or chosen backend) as dependencies
- [x] Implement `WindowBackend` trait for the chosen backend
- [x] Implement basic `Renderer` using CPU-rendered pixel buffer
- [x] Wire `graphics-core::Renderer` to draw views into the surface
- [x] Wire `window-backend-darwin` into `app-kit-lite` app lifecycle
- [x] Implement window creation, resize, close
- [x] Implement keyboard event forwarding to responder chain
- [x] Implement mouse event forwarding with hit testing
- [x] Implement redraw cycle: `set_needs_display` → `draw()` → `present_surface`
- [x] Create demo app: one window, one label, one button, button click changes label
- [x] Test on macOS ARM64

### Acceptance

- [x] A window opens on macOS
- [x] Label renders readable text
- [x] Button click fires action and updates label
- [x] Window resizes and redraws correctly
- [x] Close button terminates the app
- [x] Demo app can be run with `cargo run -p window-backend-darwin --example demo`

---

## Phase 37 — First Interoperable GUI App Target

**Goal**: Prove the stack can run apps that look and feel like simple Mac apps.

### Demo apps

#### App 1: "Hello Darwin" — minimal lifecycle
- Window with title "Hello Darwin"
- Label: "Running on burdetty-os runtime v0.1"
- Button: "Run Scorecard" → runs compat_score, shows result in label
- Button: "Quit" → `NSApplication::terminate(0)`

#### App 2: "Control Gallery" — widget exercise
- Multiple controls: Label, Button, TextField
- Text field + button that echoes input to a label
- Toggle button that changes background color

#### App 3: "Foundation Explorer" — runtime introspection
- List of all 35 Foundation classes
- Tap a class → shows registered methods
- Shows ObjC runtime stats (classes, selectors, categories)

### Tasks

- [x] Create `apps/hello-darwin/` with `main.rs`
- [x] Create `apps/control-gallery/` with `main.rs`
- [x] Create `apps/foundation-explorer/` with `main.rs`
- [x] Implement text input handling in `TextField`
- [x] Implement basic list/table view for Foundation Explorer
- [x] Add menu bar support to `app-kit-lite` (File → Quit at minimum)
- [x] Test all three apps on macOS
- [x] Screenshot each app for documentation

### Acceptance

- [x] All three apps launch, render, and respond to input
- [x] Apps can be run as `cargo run -p hello-darwin`
- [x] Apps look like simple native Mac apps (basic but functional)
- [x] Foundation Explorer correctly shows runtime class/method data

---

## Phase 38 — Bootable GUI Demo Image

**Goal**: The public demo artifact — a bootable VM that showcases the full stack.

### Demo image contents

```
/darwin-runtime/
  bin/
    runtime-shell         — interactive CLI
    hello-darwin          — GUI demo app
    control-gallery       — GUI demo app
    foundation-explorer   — GUI demo app
  fixtures/               — all 62 test binaries
  config/
    banner.txt            — boot message
    demos.json            — demo app descriptors
  logs/
    trace.log             — runtime trace output
```

### Boot sequence

1. VM boots → auto-login → desktop appears
2. `runtime-shell` launches in a terminal window
3. Boot banner displays:
   ```
   ╔═══════════════════════════════════════════════════╗
   ║  burdetty-os Runtime Environment v0.1       ║
   ║  Compatibility: 62/62 (100%)                      ║
   ║  Foundation classes: 35 | Shim functions: 344     ║
   ║  Type 'help' for commands, 'gui' for GUI demos    ║
   ╚═══════════════════════════════════════════════════╝
   ```
4. User can run CLI demos, view scorecard, or launch GUI apps

### Tasks

- [x] Update `packaging/make-vm-image.sh` to include GUI apps
- [x] Create boot banner and auto-launch configuration
- [x] Create `demos.json` descriptor for all demos (CLI + GUI)
- [x] Add `gui` command to `runtime-shell` that launches GUI apps
- [x] Test full boot-to-GUI flow
- [x] Record demo video (2-3 minutes showing CLI + GUI)
- [x] Write `docs/DEMO.md` with setup instructions
- [x] Create `README.md` section: "Try it yourself"

### Acceptance

- [x] One command creates and boots the demo VM
- [x] Boot banner appears automatically
- [x] `help` lists all available commands
- [x] `run return42` shows exit code 42
- [x] `score` shows 62/62 compatibility table
- [x] `gui hello-darwin` opens a GUI window
- [x] Demo video exists and tells the story

---

## Package manager interoperability

> **Note**: This section has been expanded into a full roadmap.
> See [EXPANSION_ROADMAP.md](EXPANSION_ROADMAP.md) Track A for the detailed plan.

### Swift Package Manager (SPM)

The Rust crates can expose C-compatible libraries that SPM can consume:

```swift
// Package.swift
let package = Package(
    name: "DarwinRuntime",
    targets: [
        .systemLibrary(
            name: "CDarwinRuntime",
            pkgConfig: "darwin-runtime",
            providers: [.brew(["darwin-runtime"])]
        ),
    ]
)
```

**Tasks for interop:**
- [ ] Create `cdylib` targets for key crates (`macho-runtime`, `objc-runtime`, `cf-lite`)
- [ ] Generate C headers using `cbindgen`
- [ ] Create `pkg-config` `.pc` file
- [ ] Create SPM `Package.swift` wrapper
- [ ] Test: Swift app can `import CDarwinRuntime` and call the loader

### Homebrew

- [ ] Create `Formula/darwin-runtime.rb` 
- [ ] Formula builds from source via `cargo build --release`
- [ ] Installs runtime binary, demo apps, and headers

### CocoaPods / Carthage (lower priority)

CocoaPods requires Xcode integration. If SPM works, these are additive.

---

## Milestone sequence (summary)

| # | Milestone | Key deliverable | Status |
|---|---|---|---|
| 1 | **Phase 30** — Freeze core | `HOST_DEPENDENCY_REGISTER.md`, API stability | ✅ Complete |
| 2 | **Phase 31** — PAL | `platform-core` + `platform-darwin-host` crates | ✅ Complete |
| 3 | **Phase 32** — Build infra | Docker images, packaging scripts | ✅ Complete |
| 4 | **Phase 33** — Bootable VM | One-command VM boot with CLI demos | ✅ Complete |
| 5 | **Phase 34** — Runtime shell | Interactive demo experience | ✅ Complete |
| 6 | **Phase 35** — App framework | Run loop, app object, view tree, controls | ✅ Complete |
| 7 | **Phase 36** — GUI backend | Windows on screen, events, rendering | ✅ Complete |
| 8 | **Phase 37** — GUI apps | Three demo apps proving the stack | ✅ Complete |
| 9 | **Phase 38** — Demo image | Bootable VM with GUI — the showcase artifact | ✅ Complete |

---

## Anti-goals (explicit)

| Do not | Reason |
|---|---|
| Build a full custom OS kernel | Userspace runtime only |
| Clone all of AppKit | 400K+ LOC; we need app lifecycle + basic controls |
| Support iOS/watchOS/tvOS | macOS ARM64 only for now |
| Build a general-purpose compositor | One window backend is enough |
| Make it production-ready for App Store | This is a demonstration/research project |
| Replace Xcode | CLI + runtime shell is the interface |
| Support x86_64 initially | ARM64 first; x86_64 via PAL later |

---

## Workspace layout (target state)

```
burdetty-os/
  Cargo.toml                    (workspace root — 26+ members)
  crates/
    # ── Core loader (stable, frozen) ──
    macho-types/
    macho-parse/
    macho-map/
    macho-resolve/
    macho-objc/
    macho-run-plan/
    macho-diff/
    macho-fixtures/
    macho-exec/
    macho-inspect/
    test-harness/
    trust-policy/
    # ── Runtime shim stack (stable, PAL-injected) ──
    libsystem-shims/
    objc-runtime/
    cf-lite/
    macho-runtime/
    # ── Platform abstraction ──
    platform-core/
    platform-darwin-host/
    # ── Application framework ──
    runloop-core/
    app-kit-lite/
    ui-view/
    ui-controls/
    window-backend/
    window-backend-darwin/
    graphics-core/
    # ── Demo & shell ──
    runtime-shell/
    runtime-demo/
  apps/
    hello-darwin/
    control-gallery/
    foundation-explorer/
  fixtures/                     (62 test binaries)
  docs/
    architecture.md
    roadmap.md
    APP_LAYER_ROADMAP.md        (this document)
    HOST_DEPENDENCY_REGISTER.md (Phase 30)
    STABLE_API.md               (Phase 30)
    BUILD.md                    (Phase 32)
    VM_DEMO.md                  (Phase 33)
    DEMO.md                     (Phase 38)
  docker/
    Dockerfile.dev
    Dockerfile.demo
  packaging/
    build.sh
    bundle-runtime.sh
    make-vm-image.sh
  vm-image/                     (Swift — Virtualization.framework)
  scripts/                      (existing)
```
