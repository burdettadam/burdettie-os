# Darwin-Compatible OS Stack — Next-Generation Roadmap

> **Vision**: A full Rust OS stack that mirrors Apple's userspace and is binary-
> compatible with real macOS applications—first on macOS hosts, then on Linux
> and bare-metal, ultimately running unmodified Apple binaries anywhere ARM64
> hardware exists.

---

## Current State (Post-Phase C4)

| Metric | Value |
|---|---|
| Workspace members | 34 (31 crates + 3 GUI apps) |
| Rust LOC | ~63,000 + 281 lines ARM64 assembly |
| Foundation classes | 47 (substantive implementations) |
| ObjC runtime shims | 102 functions |
| Foundation method shims | 141 |
| libsystem shims | 65 (+dispatch, +arc4random, +random) |
| Fixture binaries | 82 directories, 63 compat-scored |
| Corpus programs | 12/12 passing |
| Tests | 1,100+ passing, 0 failing |
| Total installed shims | 308 (was 279 pre-C4) |
| GUI apps | 3 (hello-darwin, control-gallery, foundation-explorer) |
| PAL traits | 10 defined, macOS impl complete, Linux stub crate created |

### Completed Phases

- **C1** — ObjC Protocol Support ✅
- **C2** — Dynamic Class Creation ✅
- **D1** — Eliminate Host ObjC Fallback ✅ (self-hosted exceptions, tagged pointers, NSConstant classes)
- **D2** — Pure Rust Crypto ✅ (CommonCrypto → sha2 crate)
- **D3** — Pure Rust Unwind Registration ✅ (compact→eh_frame converter, platform-linux crate)
- **D4** — Self-Contained libsystem ✅ (portable mach/pthread/dispatch/arc4random shims)
- **C3** — Struct Return & Extended ABI ✅ (x8 save/restore, stret/fp2ret aliases, CG types, NSValue)
- **C4** — CoreFoundation Toll-Free Bridging ✅ (CFRetain/Release/Equal/Hash, CFArray/Dict/Data/URL/Bundle)
- **C5** — AppKit Bootstrap ✅ (NSApplication, NSRunLoop, NSResponder, NSWindow, NSView, NSEvent, NSScreen, NSWorkspace, NSColor, NSFont, NSImage)
### What Works Today

- Full Mach-O load → execute pipeline (parse, resolve, map, fixup, security, constructors, entry)
- Multi-image loading with transitive dependency resolution
- ARM64 `objc_msgSend` dispatch (assembly trampoline)
- 47 Foundation classes with real implementations
- ObjC protocols (parsing, conformance, runtime creation)
- Dynamic class creation (allocate/register/dispose class pairs, isa-swizzling)
- Self-hosted exception handling (C++ ABI, no host ObjC runtime needed)
- Tagged pointer dispatch (NSNumber, NSString, NSDate)
- KVO, KVC (with collection operators), method swizzling, NSInvocation
- C/Swift FFI (dylib, SPM package, Homebrew formula, pkg-config)
- Software-rendered GUI with 7 controls (label, button, text field, scroll, stack, table, split)
- Platform abstraction layer with 10 traits (macOS impl complete)
- Portable libsystem shims (#[cfg(target_os)] gated for Linux)

### What Doesn't Work Yet

- No real macOS `.app` bundles can run (missing AppKit, CoreGraphics, CoreFoundation depth)
- No Linux or x86_64 support (ARM64 assembly, PAL Linux stubs but no end-to-end)
- GUI is software-rendered (no GPU, no text layout engine, no images)

---

## Strategic Tracks

```
Track C — Run Real Apps (on macOS host)         "make it useful"
Track D — Self-Contained Runtime (no host)       "cut the cord"
Track E — Cross-Platform (Linux ARM64/x86_64)    "run anywhere"
Track F — Native GUI Stack                       "own the pixels"
```

Tracks are sequenced: **C feeds into D**, D enables E, F can be worked in
parallel with C/D. Each phase is sized for ~1–3 sessions of focused work.

---

## Track C — Run Real Apps on macOS Host

**Goal**: Execute progressively more complex real-world macOS binaries through
the runtime, discovering and filling gaps as they appear. Each phase targets
a concrete app or class of binaries.

### Phase C1 — ObjC Protocol Support ✅

Protocols are used everywhere in real apps (`NSCoding`, `NSCopying`,
`UITableViewDelegate`, etc.). Without them, most binaries crash on launch.

#### Tasks

- [x] Parse `__objc_protolist` section → `ObjcProtocolRef` structs (name, instance methods, class methods, optional methods)
- [x] Parse `__objc_protorefs` section → protocol reference resolution
- [x] Parse `protocol_t` on-disk layout: name, protocols (adopted), instance methods, class methods, optional instance methods, optional class methods, instance properties
- [x] Parse `class_ro_t + 40` (baseProtocols) → class protocol conformance
- [x] Register protocols in ObjC runtime: `ProtocolInfo` struct, protocol table keyed by name
- [x] Implement `objc_getProtocol(name)` → returns registered protocol
- [x] Implement `protocol_getName`, `protocol_isEqual`
- [x] Implement `protocol_conformsToProtocol` (recursive)
- [x] Implement `class_conformsToProtocol` — walk class's protocol list + superclass chain
- [x] Implement `class_copyProtocolList` — adopted protocols array
- [x] Implement `protocol_copyProtocolList` (adopted protocols)
- [x] Wire protocol metadata into `ObjcMetadata` during image loading
- [x] Implement `NSObject` method: `conformsToProtocol:`
- [x] Create `protocol_basics` fixture — class conforming to custom protocol, protocol introspection
- [x] Write unit tests for protocol registration, conformance, and metadata parsing
- [x] Implement `objc_allocateProtocol(name)` / `objc_registerProtocol(proto)`
- [x] Implement `class_addProtocol` — add conformance at runtime
- [x] Implement `protocol_copyMethodDescriptionList`, `protocol_getMethodDescription`
- [x] Implement `protocol_copyPropertyList`, `protocol_getProperty`
- [x] Create `protocol_nscoding` fixture — class adopting `NSCoding`, encode/decode cycle

#### Acceptance

- [x] Protocol metadata is parsed from real macOS binaries
- [x] `class_conformsToProtocol` traverses class + superclass chain
- [x] Protocol-related fixture passes through runtime pipeline

---

### Phase C2 — Dynamic Class Creation ✅

Many frameworks (including parts of KVO, CoreData, SwiftUI bridging) create
classes at runtime. Required for any non-trivial app.

#### Tasks

- [x] Implement `objc_allocateClassPair(superclass, name, extraBytes)` — allocate new class + metaclass
- [x] Implement `objc_registerClassPair(cls)` — finalize and register in class table
- [x] Implement `objc_disposeClassPair(cls)` — remove if no instances exist
- [x] Implement `objc_duplicateClass(original, name, extraBytes)` — for KVO isa-swizzle
- [x] Support `class_addMethod` on unregistered class pairs (already existed)
- [x] Support `class_addIvar` on unregistered class pairs (before registration)
- [x] Support `class_addProperty` on unregistered class pairs
- [x] Support `class_addProtocol` on unregistered class pairs (already existed from C1)
- [x] Implement `object_setClass` for isa-swizzling (already existed)
- [x] Wire dynamic class creation into the runtime class table with proper superclass chain
- [x] Create `dynamic_class` fixture — create class at runtime, add methods, instantiate, call methods
- [x] Create `kvo_isa_swizzle` fixture — KVO with automatic isa-swizzling
- [x] Write unit tests

#### Acceptance

- [x] Dynamically created classes are visible to `objc_getClass`
- [x] Instances of dynamic classes dispatch methods correctly
- [x] Dynamic classes support ivar layout and property metadata

---

### Phase C3 — Struct Return & Extended ABI ✅

Real binaries use `objc_msgSend_stret` (struct returns) and other ABI
variants constantly—`CGRect`, `NSRange`, `NSPoint`, `NSSize`, etc.

#### Tasks

- [x] Implement ARM64 `objc_msgSend_stret` trampoline — x8 now saved/restored in all trampolines; stret is an alias for msgSend on ARM64
- [x] Implement `objc_msgSendSuper_stret` trampoline — alias, x8 preserved
- [x] Implement `objc_msgSend_fp2ret` trampoline — alias on ARM64 (fp2ret only matters on x86)
- [x] Implement `objc_msgSendSuper2_stret` trampoline — alias, x8 preserved
- [x] Define `CGPoint`, `CGSize`, `CGRect`, `NSRange`, `NSEdgeInsets` as `#[repr(C)]` structs in `cf-lite/cg_types.rs`
- [x] Implement `NSValue` struct wrapping for CG types: `valueWithPoint:`, `valueWithSize:`, `valueWithRect:`
- [x] Implement `NSStringFromRect`, `NSStringFromPoint`, `NSStringFromSize`, `NSStringFromRange` as Foundation shims
- [x] Create `struct_return` fixture — ObjC class returning CGRect/CGPoint/CGSize/NSRange, plus NSStringFrom* calls
- [x] Write ABI conformance tests (objc_shims_registered updated to 102, runtime exec tests for struct_return)

#### Acceptance

- [x] Methods returning structs > 16 bytes work correctly (CGRect via x8 indirect)
- [x] `CGRect`-returning methods produce correct values (verified in fixture: 10,20,300,400)
- [x] Struct return fixture passes through runtime pipeline (exit code 0)
- [x] Shim count: 271 → 279 (+4 msgSend ABI variants, +4 NSStringFrom* functions)

---

### Phase C4 — CoreFoundation Toll-Free Bridging ✅

Many real apps mix `CF*` and `NS*` types interchangeably. CoreFoundation
types like `CFString`, `CFArray`, `CFDictionary` must bridge to their
Foundation counterparts.

#### Tasks

- [x] Implement toll-free bridging infrastructure: CF types share ISA with NS counterparts
- [x] Implement `CFStringRef` ↔ `NSString` bridging (same object, different type)
- [x] Implement `CFArrayRef` ↔ `NSArray` bridging
- [x] Implement `CFDictionaryRef` ↔ `NSDictionary` bridging
- [x] Implement `CFDataRef` ↔ `NSData` bridging
- [x] Implement `CFURLRef` ↔ `NSURL` bridging
- [x] Implement core CF functions: `CFRetain`, `CFRelease`, `CFGetTypeID`, `CFEqual`, `CFHash`, `CFCopyDescription`
- [x] Implement `CFStringCreateWithCString`, `CFStringGetCStringPtr`, `CFStringGetLength`
- [x] Implement `CFBundleGetMainBundle`, `CFBundleCopyBundleURL`
- [x] Add CF function shims to foundation shim_core.rs (29 new shim registrations)
- [x] Create `cf_bridging` fixture — create CF objects, pass to NS APIs, verify identity
- [x] Write unit tests (11 cf_core tests + 2 runtime exec tests)

#### Acceptance

- [x] `(__bridge NSString *)cfString` produces a valid NSString
- [x] CF retain/release interoperates with ObjC retain/release
- [x] CF fixture passes through runtime pipeline
- [x] Shim count: 279 → 308 (+29 CF shims)

---

### Phase C5 — AppKit Bootstrap (NSApplication Launch) ✅

The minimum viable path to launching a real `.app` bundle. Get `[NSApplication
sharedApplication]` → `[NSApp run]` to not crash.

#### Tasks

- [x] Implement `NSApplication` class in cf-lite: `sharedApplication`, `run`, `terminate:`, `mainMenu`, `setMainMenu:`
- [x] Implement `NSRunLoop` class: `currentRunLoop`, `mainRunLoop`, `runUntilDate:`, `addTimer:forMode:`
- [x] Implement `NSRunLoop` mode constants: `NSDefaultRunLoopMode`, `NSRunLoopCommonModes`
- [x] Implement `NSWorkspace` stub: `sharedWorkspace`
- [x] Implement `NSScreen` stub: `mainScreen`, `frame`, `visibleFrame`
- [x] Implement `NSEvent` class: `type`, `locationInWindow`, `modifierFlags`, `characters`, `keyCode`
- [x] Implement `NSResponder` chain: `nextResponder`, `keyDown:`, `mouseDown:`, etc.
- [x] Implement `NSWindow` stub: `initWithContentRect:styleMask:backing:defer:`, `makeKeyAndOrderFront:`, `contentView`
- [x] Implement `NSView` stub: `initWithFrame:`, `addSubview:`, `setNeedsDisplay:`, `frame`, `bounds`
- [x] Implement `NSColor` stub: `whiteColor`, `blackColor`, `redColor`, etc.
- [x] Implement `NSFont` stub: `systemFontOfSize:`, `fontName`, `pointSize`
- [x] Implement `NSImage` stub: `imageNamed:` (return nil gracefully)
- [x] Implement `NSBundle` dynamic loading: `+[NSBundle loadNibNamed:owner:]` stub (no-op, log warning)
- [x] Create `appkit_bootstrap` fixture — minimal NSApplication launch + quit after 1 runloop iteration
- [x] Write unit tests

#### Acceptance

- [x] `[NSApplication sharedApplication]` returns a valid object
- [x] `[NSApp run]` enters and exits the run loop without crashing
- [x] App delegate receives `applicationDidFinishLaunching:`
- [x] Bootstrap fixture exits cleanly with exit code 0

---

### Phase C6 — Real App Attempt: Command-Line Tools

Test against real-world command-line tools compiled for macOS ARM64.
These are the simplest "real" binaries—no GUI, no NIBs, just Foundation +
argument parsing.

#### Tasks

- [ ] Identify 5 open-source macOS CLI tools that use Foundation (e.g., `xcrun cloc` style tools, `trash`, `mas`, `jq`-like tools with ObjC bridges)
- [ ] Run each through the runtime, capture missing shims/classes
- [ ] Implement missing Foundation classes as needed (gap-driven development)
- [ ] Implement missing `libSystem` shims as needed
- [ ] Create a `real_app_compat` test that tracks pass/fail for each real binary
- [ ] Document gaps and workarounds

#### Acceptance

- [ ] At least 3 real-world CLI tools run to completion through the runtime
- [ ] Gap analysis document identifies the top 20 missing APIs
- [ ] Missing APIs are prioritized for Track C7+

---

### Phase C7 — Real App Attempt: Simple GUI App

Run a minimal real Cocoa app (source-compiled fixture with AppKit, not
a full Xcode project—something like a single-window app that shows a label
and a button).

#### Tasks

- [ ] Create a simple Cocoa `.app` bundle fixture: Info.plist, MainMenu.nib (or programmatic), one window with a label
- [ ] Implement `.app` bundle loading: traverse bundle structure, find executable, load Info.plist
- [ ] Implement `NSUserNotificationCenter` stub (many apps touch this on launch)
- [ ] Implement `NSAppearance` stub (dark mode queries)
- [ ] Implement `NSNib` / `NSCoder` basics for nib loading (or prove programmatic approach works without nibs)
- [ ] Identify and fill the top 10 symbol gaps that appear during launch
- [ ] Route AppKit calls through our app-kit-lite / ui-view / ui-controls stack where possible
- [ ] Connect real NSWindow creation to our window-backend rendering

#### Acceptance

- [ ] A source-compiled Cocoa single-window app launches through the runtime
- [ ] The window appears on screen (even if rendered via our software renderer)
- [ ] The app responds to at least one user event (click or key press)
- [ ] The app quits cleanly

---

## Track D — Self-Contained Runtime (Cut the Host Cord)

**Goal**: Eliminate all dependencies on the host macOS ObjC runtime,
Foundation framework, and macOS-specific system calls so the runtime
can operate as a fully self-contained entity.

### Phase D1 — Eliminate Host ObjC Fallback ✅

Today, unresolved selectors fall back to `dlsym(RTLD_DEFAULT, "objc_msgSend")`
on the host. On Linux there is no host ObjC. Remove this dependency.

#### Tasks

- [x] Audit all `dlsym`-based host ObjC calls in objc-runtime and cf-lite
- [x] Implement `doesNotRecognizeSelector:` properly — raise NSException instead of host fallback
- [x] Remove host `objc_msgSend` fallback from dispatch.rs — all dispatch must go through our runtime
- [x] Handle tagged pointers internally (NSNumber, NSString tagged pointers) instead of forwarding to host
- [x] Implement tagged pointer ISA table for small NSNumber/NSString values
- [x] Remove `dlopen("Foundation.framework")` — implement `_OBJC_EHTYPE_$_NSException` internally
- [x] Self-host `NSConstantArray` and `NSConstantDictionary` (compiler-generated constant collections)
- [x] Add a `hostfallback on|off` toggle in runtime-shell
- [x] Self-host exception throw/catch/rethrow via C++ ABI (`__cxa_throw`/`__cxa_begin_catch`/etc.)
- [x] Remove `host_compare()` NSSortDescriptor fallback to host `objc_msgSend`

#### Implementation Notes

- Exception throwing uses `__cxa_throw` directly; `@catch` in mapped binaries
  requires full unwind-info registration (future work — `try_catch_test` ignored).
- `objc_ehtype_vtable` data symbol still resolved from host libobjc (needed by
  `__objc_personality_v0` for ObjC exception type matching).
- Tagged pointer classes registered: NSNumber (tag 2), NSString (tag 3), NSDate (tag 6).
- 267 shims installed (up from 247), 261 tests passing, 1 ignored.

#### Acceptance

- [x] All fixture tests pass (except try_catch_test which needs unwind-info registration)
- [x] Zero host ObjC function calls remain in shim code (only data symbols)
- [x] Tagged pointer dispatch is handled internally

---

### Phase D2 — Replace CommonCrypto with Pure Rust ✅

#### Tasks

- [x] Add `sha2` crate (pure Rust, `no_std` compatible) as dependency
- [x] Replace `CC_SHA256` in `platform-darwin-host` hasher with `sha2::Sha256`
- [x] Remove `CC_SHA256` fallback in `macho-parse/src/codesig.rs`
- [x] Remove CommonCrypto link dependency
- [x] Verify code signature hashing produces identical CDHash values
- [x] Run page hash verification tests

#### Implementation Notes

- `sha2 v0.10` added as workspace dependency; used by `platform-darwin-host` and
  `macho-parse` (fallback path).
- CDHash values verified identical via `simple_cli_cd_hash_matches_codesign` test
  which compares against `codesign` tool output.
- All 15 codesig tests pass (page hash verification, tamper detection, etc.).
- Zero `extern "C" { CC_SHA256(...) }` calls remain in the codebase.

#### Acceptance

- [x] Code signature verification still works identically
- [x] No CommonCrypto dependency in `cargo tree`
- [x] All codesig tests pass

---

### Phase D3 — Pure Rust Unwind Registration ✅

Replace macOS-specific `__unw_add_find_dynamic_unwind_sections` with a
portable approach.

#### Tasks

- [x] Implement `.eh_frame` generation from compact unwind info
- [x] On macOS: continue using `__unw_add_find_dynamic_unwind_sections` (it works)
- [x] On Linux: register via `__register_frame` / `__deregister_frame` (libgcc/LLVM)
- [x] Abstract behind `UnwindRegistry` PAL trait (already defined)
- [x] Create `platform-linux` stub for `UnwindRegistry`
- [x] Test exception unwinding through mapped code on macOS (existing tests)

#### Implementation Notes

- `macho-exec::compact_to_ehframe` module: parses `__unwind_info` section
  (version 1 format with regular + compressed second-level pages) into
  per-function entries, then generates DWARF `.eh_frame` (CIE + FDEs).
- ARM64 FRAME mode: CFA=FP+16, FP/LR + callee-saved register pairs (X19-X28,
  D8-D15) with correct slot ordering.
- ARM64 FRAMELESS mode: CFA=SP+stack_size from compact encoding.
- `platform-linux` crate: `LinuxUnwindRegistry` stores generated `.eh_frame`
  buffers, calls `__register_frame`/`__deregister_frame` on Linux, no-ops
  on other platforms (compiles on macOS for development).
- `UnwindRegistry` PAL trait was already in place from Phase 43; macOS path
  (`DarwinUnwindRegistry`) unchanged.
- 11 unit tests for compact parsing + eh_frame generation + LEB128 encoding.
- 3 integration tests on real fixture binaries (simple_cli, exception_basics,
  autorelease).

#### Acceptance

- [x] Unwind registration is fully behind PAL trait
- [x] macOS path unchanged, Linux path compiles
- [x] Exception handling tests still pass

---

### Phase D4 — Self-Contained libsystem

Replace Mach-specific APIs with portable equivalents hidden behind PAL traits.

#### Tasks

- [x] Replace `mach_task_self()` — return a synthetic task port ID (0x103 on non-macOS)
- [x] Replace `mach_absolute_time()` — use `clock_gettime(CLOCK_MONOTONIC)` on Linux, existing on macOS
- [x] Replace `mach_timebase_info()` — return {1, 1} on Linux (nanoseconds already)
- [x] Implement `dispatch_once_f` without host GCD (atomic + mutex) — already self-hosted
- [x] Implement `dispatch_async` / `dispatch_sync` stubs — already inline
- [x] Implement `dispatch_get_main_queue` / `dispatch_get_global_queue` stubs
- [x] Replace `pthread_main_np()` with `gettid() == getpid()` on Linux
- [x] Replace `arc4random_buf` with `getrandom(2)` on Linux — added arc4random, arc4random_buf, arc4random_uniform
- [x] All replacements gated behind `#[cfg(target_os)]` or go through PAL
- [x] Added `random` shim category to `ShimSet` and `RuntimeContext`

#### Acceptance

- [x] `libsystem-shims` compiles on both macOS and Linux
- [x] All shim functions produce correct results on macOS (47 tests pass)
- [x] Linux implementations compile (gated behind `#[cfg(not(target_os = "macos"))]`)
- [x] Runtime tests pass with 271 total shims (267 → 271: +dispatch_get_global_queue, +arc4random/buf/uniform)

---

## Track E — Cross-Platform (Linux ARM64 → x86_64)

**Goal**: Run macOS ARM64 Mach-O binaries on Linux. This requires solving
the hardest problem: the binaries contain ARM64 machine code that must
execute on the host, either natively (ARM64 Linux) or via translation
(x86_64 Linux).

### Phase E1 — Linux PAL Implementation (ARM64)

#### Tasks

- [ ] Create `crates/platform-linux/` — implements all 10 PAL traits
- [ ] `LinuxAllocator` — `libc::malloc/calloc/realloc/free`
- [ ] `LinuxMemoryMapper` — `mmap/mprotect/munmap` (same API, different headers)
- [ ] `LinuxHostSymbolResolver` — `dlsym/dlopen` (for host libc linkage only)
- [ ] `LinuxUnwindRegistry` — `__register_frame/__deregister_frame`
- [ ] `LinuxClock` — `clock_gettime(CLOCK_REALTIME/MONOTONIC)`, `localtime_r`, timezone from `/etc/timezone`
- [ ] `LinuxRng` — `getrandom(2)` syscall
- [ ] `LinuxProcessInfo` — `getpid()`, `gettid() == getpid()` for main thread check
- [ ] `LinuxFilesystem` — `std::fs::read`, `std::fs::metadata`, `readlink`
- [ ] `LinuxHasher` — `sha2::Sha256` (pure Rust)
- [ ] `LinuxThreading` — `pthread_mutex_*`, `pthread_cond_*` (POSIX, same API)
- [ ] Feature-gate platform selection: `--features platform-linux` vs default macOS
- [ ] CI: add Linux ARM64 test job (GitHub Actions, or cross-compile check)

#### Acceptance

- [ ] `cargo check --target aarch64-unknown-linux-gnu` succeeds
- [ ] PAL trait implementations compile and pass basic unit tests
- [ ] CI validates Linux compilation

---

### Phase E2 — x86_64 Assembly Trampolines

ARM64 assembly can't run on x86_64. Need equivalent trampolines.

#### Tasks

- [ ] Write x86_64 `objc_msgSend` trampoline (save rdi/rsi/rdx/rcx/r8/r9 + xmm0-7, call lookup, restore, jmp)
- [ ] Write x86_64 `objc_msgSendSuper` trampoline
- [ ] Write x86_64 `objc_msgSendSuper2` trampoline
- [ ] Write x86_64 `NSLog` variadic trampoline (or use libffi)
- [ ] Write x86_64 `stringWithFormat:` variadic trampoline (or use libffi)
- [ ] Consider libffi-based generic trampoline as alternative to hand-written assembly
- [ ] Feature-gate: `#[cfg(target_arch = "aarch64")]` vs `#[cfg(target_arch = "x86_64")]`
- [ ] Run dispatch tests on x86_64 macOS (validates trampoline correctness)

#### Acceptance

- [ ] `objc_msgSend` dispatch works on x86_64
- [ ] Method calls with integer, pointer, and floating-point arguments work
- [ ] All dispatch-related tests pass on x86_64

---

### Phase E3 — ARM64 Binary Translation (x86_64 Host)

The crown jewel: executing ARM64 Mach-O machine code on an x86_64 host.
This is a JIT/AOT binary translation challenge.

#### Tasks

- [ ] Evaluate translation strategies: interpretive (slow, simple), JIT (fast, complex), or delegate to existing translators (QEMU user-mode, Rosetta-compatible, FEX-Emu)
- [ ] **Option A — FEX-Emu integration**: Link FEX-Emu as a library for ARM64→x86_64 translation; map segments, translate, execute
- [ ] **Option B — QEMU user-mode**: Fork `qemu-aarch64` with custom syscall handlers that route through our runtime
- [ ] **Option C — Custom JIT**: Basic block → x86_64 translation for the subset of ARM64 instructions in typical ObjC binaries (ADRP, LDR, STR, BL, BLR, RET, CMP, B.cond, STP, LDP, MOV, ADD, SUB)
- [ ] Implement instruction decoder for the ARM64 subset (A64 encoding)
- [ ] Implement code cache (translated blocks keyed by source address)
- [ ] Handle self-modifying code / JIT code (unlikely in ObjC binaries, but possible in blocks)
- [ ] Map ARM64 registers to x86_64 registers (or use a register file in memory)
- [ ] Handle ARM64 → x86_64 calling convention translation at FFI boundary
- [ ] Test with `return42` fixture (simplest possible: loads constant, returns)
- [ ] Test with `simple_cli` fixture (printf call via GOT stub)
- [ ] Test with Foundation fixtures (full ObjC dispatch)

#### Acceptance

- [ ] At least `return42` and `simple_cli` fixtures execute correctly on x86_64
- [ ] Foundation fixtures produce correct output via translation
- [ ] Translation overhead documented (slowdown factor)

---

### Phase E4 — Linux End-to-End Integration

Wire everything together: Mach-O loader + ObjC runtime + Foundation +
PAL + translation (if x86_64) running on Linux.

#### Tasks

- [ ] Build integration test: Linux ARM64 loads and runs `simple_cli` fixture
- [ ] Handle Mach-O → Linux memory mapping differences (no __PAGEZERO enforcement)
- [ ] Route `printf` / `puts` / `malloc` / `free` to Linux libc
- [ ] Route filesystem calls to Linux paths (no `/System/Library/Frameworks`)
- [ ] Handle library resolution: fixture dylibs must be found without macOS search paths
- [ ] Run Foundation fixture suite on Linux ARM64
- [ ] Docker image for Linux ARM64 testing
- [ ] CI: Linux ARM64 test matrix

#### Acceptance

- [ ] `simple_cli` fixture prints correct output on Linux ARM64
- [ ] At least 30 Foundation fixtures pass on Linux ARM64
- [ ] Linux CI job is green

---

## Track F — Native GUI Stack

**Goal**: Replace the software renderer with a real graphics pipeline that
can render AppKit-compatible UIs at native quality on any platform.

### Phase F1 — Text Layout Engine

#### Tasks

- [x] Integrate `ab_glyph` for TrueType/OpenType font loading
- [x] Implement `CTFont` equivalent: font loading, glyph lookup, metrics (ascent, descent, leading) — `FontFace`, `FontMetrics`, `FontManager`
- [x] Implement `CTLine` equivalent: single-line text layout with glyph positions — `layout_single_line()`
- [x] Implement `CTFrame` equivalent: multi-line text layout with line breaking — `layout_wrapped()` with `WrapMode::Word` / `Character`
- [ ] Implement attributed string rendering (bold, italic, color, size per range)
- [x] Replace 5×7 bitmap font with system font rendering in `SoftwareRenderer` — TrueType rendering with alpha blending, bitmap fallback preserved
- [x] Unicode support: NFC normalization, combining characters, zero-width char filtering, emoji (DejaVu monochrome), missing glyph placeholder boxes
- [x] Font fallback chain (primary → system → last resort) — `FontManager::get()` falls back to system font

#### Acceptance

- [x] Text renders with proper kerning and metrics
- [x] Multi-line text wraps correctly
- [x] GUI apps display readable proportional text

---

### Phase F2 — GPU-Accelerated Rendering

#### Tasks

- [ ] Integrate `wgpu` for cross-platform GPU rendering
- [ ] Implement `GpuRenderer` implementing the `Renderer` trait
- [ ] Render filled/stroked rects via GPU quad batching
- [ ] Render text via glyph atlas texture
- [ ] Implement compositing (view layers with opacity, clipping)
- [ ] Dual path: `GpuRenderer` when GPU available, fallback to `SoftwareRenderer`
- [ ] Benchmark: measure FPS for control-gallery with GPU vs software

#### Acceptance

- [ ] GUI apps render via GPU at 60fps
- [ ] Visual output matches software renderer
- [ ] Fallback to software renderer works on headless systems

---

### Phase F3 — Image & Drawing Primitives

#### Tasks

- [ ] Image loading: PNG, JPEG via `image` crate
- [ ] `NSImage` implementation: `imageNamed:`, `initWithContentsOfFile:`, `size`, `drawInRect:`
- [ ] `NSBezierPath` equivalent: move/line/curve/close, fill, stroke
- [ ] Rounded rectangles, circles, arcs
- [ ] Gradient fills (linear, radial)
- [ ] Shadow rendering
- [ ] `CGContext`-style drawing API in graphics-core

#### Acceptance

- [ ] Images display in GUI apps
- [ ] Bezier paths render correctly
- [ ] Gradients and shadows work

---

### Phase F4 — Layout Engine (Auto Layout Lite)

#### Tasks

- [ ] Implement constraint-based layout solver (Cassowary algorithm via `cassowary` crate or custom)
- [ ] `NSLayoutConstraint` equivalent: item1.attr = multiplier * item2.attr + constant
- [ ] Intrinsic content size for labels, buttons, text fields
- [ ] Content hugging and compression resistance priorities
- [ ] `NSStackView` backed by constraints
- [ ] Safe area / margin guides
- [ ] Layout debugging: constraint conflict detection and logging

#### Acceptance

- [ ] Constraints solve correctly for complex layouts
- [ ] Window resizing re-layouts correctly
- [ ] No constraint conflicts in demo apps

---

## Priority & Sequencing

```
             C1 ─ C2 ─ C3 ─ C4 ─ C5 ─ C6 ─ C7
             │                   ╱
             ▼                  ╱
        D1 ─ D2 ─ D3 ─ D4 ──╱
                          │
                          ▼
                     E1 ─ E2 ─ E3 ─ E4
                         ╱
        F1 ─ F2 ─ F3 ─ F4
        (parallel with C/D)
```

### Recommended execution order

| Order | Phase | Why |
|---|---|---|
| 1 | **C1** — Protocols | ✅ Done |
| 2 | **C2** — Dynamic classes | ✅ Done |
| 3 | **C3** — Struct return ABI | ✅ Done |
| 4 | **D1** — Kill host fallback | ✅ Done |
| 5 | **D2** — Pure Rust crypto | ✅ Done |
| 6 | **C4** — CF bridging | ✅ Done |
| 7 | **C5** — AppKit bootstrap | ✅ Done |
| 8 | **F1** — Text layout | GUI needs real fonts to be useful |
| 9 | **C6** — Real CLI tools | First "real app works!" moment |
| 10 | **D3** — Portable unwind | ✅ Done |
| 11 | **D4** — Self-contained libsystem | ✅ Done |
| 12 | **E1** — Linux PAL | Linux compilation gate |
| 13 | **E2** — x86_64 trampolines | x86_64 dispatch |
| 14 | **C7** — Real GUI app | The showcase milestone |
| 15 | **F2** — GPU rendering | Real graphics quality |
| 16 | **E3** — Binary translation | The moonshot |
| 17 | **E4** — Linux E2E | Full cross-platform |
| 18 | **F3** — Images & drawing | Complete graphics |
| 19 | **F4** — Auto Layout | Complete UI framework |

---

## Milestone Map

### Milestone 1: "It understands Objective-C" (C1–C3)
Protocol metadata, dynamic class creation, struct returns. After this,
the ObjC runtime is feature-complete for the vast majority of real binaries.

### Milestone 2: "Cut the cord" (D1–D4)
No more host ObjC runtime dependency. The runtime is fully self-contained.
This is the prerequisite for running on Linux.

### Milestone 3: "A real app works" (C4–C6)
CoreFoundation bridging, AppKit bootstrap, and at least 3 real-world CLI
tools execute through the runtime. The first time someone runs a program
they didn't compile specifically for this project.

### Milestone 4: "It runs on Linux" (E1–E2, E4)
ARM64 Mach-O binaries execute on Linux ARM64. Foundation classes,
ObjC dispatch, exception handling—all working without macOS.

### Milestone 5: "A window appears" (C7, F1–F2)
A real Cocoa application launches, shows a window with real text rendering,
and responds to user input—running through our runtime, rendered by our
graphics stack.

### Milestone 6: "Run anywhere" (E3)
ARM64 binaries execute on x86_64 hosts via binary translation. The ultimate
proof that Apple's userspace is just software.

---

## Anti-Goals

| Avoid | Reason |
|---|---|
| Full AppKit reimplementation | Focus on the subset real apps actually use |
| iOS/watchOS/tvOS support | macOS ARM64 first; iOS later if at all |
| App Store DRM / FairPlay | Research project, not piracy tool |
| Bit-exact Apple silicon behavior | Functional compatibility, not hardware emulation |
| Production security parity | We strip PAC, skip AMFI; this is a research runtime |
| Swift runtime reimplementation | ObjC interop layer is sufficient; pure-Swift binaries are a separate challenge |
| Kernel / XNU reimplementation | Userspace only; rely on host kernel |

---

## Success Criteria

This roadmap is complete when:

1. A macOS CLI tool using Foundation runs through the runtime on macOS **and** Linux ARM64
2. A simple Cocoa GUI app launches, displays a window, and responds to user input
3. The runtime has zero dependencies on host ObjC/Foundation frameworks
4. ARM64 Mach-O fixtures execute on x86_64 via binary translation (at least `simple_cli` level)
