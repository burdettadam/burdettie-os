# Darwin Loader Lab — Core Capabilities

> Current state as of 2026-03-28. See [FEATURES.md](FEATURES.md) for the full
> feature index, [CRATE_MAP.md](CRATE_MAP.md) for workspace structure, and
> [ROADMAP.md](ROADMAP.md) for what comes next.

---

## Mach-O Loader

The loader is the foundation of the project. It implements the full Mach-O load
pipeline from binary-on-disk to executing code, aligned with Apple's dyld
architecture and the System V ABI.

| Capability | Detail |
|---|---|
| Header & load command parsing | All standard load commands: LC_SEGMENT_64, LC_DYLD_INFO_ONLY, LC_DYLD_CHAINED_FIXUPS, LC_CODE_SIGNATURE, LC_ENCRYPTION_INFO, LC_UNIXTHREAD, LC_MAIN, LC_LOAD_DYLIB, LC_RPATH, etc. |
| Segment mapping | Contiguous mmap with per-segment protection flags, PAGEZERO enforcement, ASLR slide computation |
| Fat / universal binaries | Slice selection by target architecture (ARM64, x86_64) with fallback ordering |
| Dependency resolution | @rpath, @loader_path, @executable_path path semantics; transitive BFS with cycle detection; init ordering respects topological sort |
| Classic fixups | Full rebase / bind / lazy-bind / export trie implementation per LC_DYLD_INFO_ONLY |
| Chained fixups | All 5 pointer chain formats (DYLD_CHAINED_PTR_ARM64E, DYLD_CHAINED_PTR_64, etc.) per LC_DYLD_CHAINED_FIXUPS |
| Re-export chain following | Follows re-exported symbol chains across dylib boundaries |
| Weak symbol support | Null-fill for unresolved weak binds |
| ObjC metadata parsing | `__objc_classlist`, `__objc_catlist`, `__objc_protolist`, `__objc_selrefs`, `__objc_imageinfo` — full class/category/protocol/selector extraction |
| TLV (thread-local variables) | TLV descriptor mechanism, `tlv_get_addr` shim, per-image TLS allocation |
| Code signature parsing | LC_CODE_SIGNATURE → SuperBlob → CodeDirectory → CDHash, page hash verification, entitlement plist extraction |
| Constructor invocation | ObjC `+load` methods, C/C++ `__attribute__((constructor))` functions — correct bottom-up ordering |
| In-process execution | `StubResolver` with 86+ allowed host symbols via `dlsym`, security-gated symbol allowlist |
| Binary comparison | `macho-diff` tool for field-by-field comparison of parsed binaries against host behavior |

---

## ObjC Runtime

A self-hosted ObjC runtime that requires no host ObjC framework. All dispatch,
exception handling, and runtime introspection go through our implementation.

| Capability | Detail |
|---|---|
| Class registration | Parse `__objc_classlist` → class table; metaclass chain; superclass linking |
| Selector interning | Intern all selectors from `__objc_selrefs`; fixup references post-mapping |
| `objc_msgSend` dispatch | ARM64 and x86_64 assembly trampolines (7 entry points each); nil-check fast path; vtable cache lookup → IMP call |
| Category merge | Same-image and cross-image category method merge (two-phase); correct ordering |
| Protocol support | `__objc_protolist` parsing; conformance tracking; `class_conformsToProtocol:` walks class + superclass chain |
| Dynamic class creation | `objc_allocateClassPair` / `objc_registerClassPair` / `objc_disposeClassPair`; isa-swizzling for KVO |
| Tagged pointer dispatch | NSNumber (tag 2), NSString (tag 3), NSDate (tag 6); tagged pointer ISA table |
| `+load` dispatch | Classes before categories, depth-first topological order; app-binary-only enforcement in interpreter path |
| Exception handling | Self-hosted via C++ ABI: `__cxa_throw` / `__cxa_begin_catch` / `__cxa_end_catch`; compact unwind → `.eh_frame` converter |
| KVO | Automatic isa-swizzling; `NSObject` KVO methods; change notification delivery |
| KVC | Key-path resolution with collection operators (`@count`, `@sum`, `@avg`, `@min`, `@max`) |
| Method swizzling | `method_exchangeImplementations`, `method_setImplementation` |
| NSInvocation | `NSMethodSignature`, `NSInvocation` invocation and return value capture |
| Blocks | `_Block_copy`, `_Block_release`, invoke-through-function-pointer; block literal ABI |
| Struct return ABI | `objc_msgSend_stret` / `objc_msgSendSuper_stret` (ARM64: x8 save/restore); `objc_msgSend_fp2ret` alias |
| Super dispatch | `objc_msgSendSuper` / `objc_msgSendSuper2` (both architectures) |
| PAC pointer handling | Pointer Authentication Code stripping in ISA / method IMP extraction |
| CF toll-free bridging | `CFStringRef` ↔ `NSString`, `CFArrayRef` ↔ `NSArray`, `CFDictionaryRef` ↔ `NSDictionary`, `CFDataRef` ↔ `NSData`, `CFURLRef` ↔ `NSURL` |
| `doesNotRecognizeSelector:` | Raises `NSException` with selector name instead of crashing |
| Runtime introspection | `class_getName`, `class_getSuperclass`, `class_copyMethodList`, `class_copyIvarList`, `class_copyPropertyList`, `object_getClassName`, `objc_getClass`, `objc_getProtocol` |
| Forwarding | `forwardInvocation:`, `methodSignatureForSelector:` (basic forwarding_basics fixture) |

---

## Foundation Layer

A Rust implementation of the Foundation framework's class library. Every class
listed below has a real implementation — not a stub — with the selectors that
real CLI applications exercise.

### Counting Methodology

Two numbers appear in project documents:

- **52 substantive implementations**: Classes with full logic (NSArray, NSDate, NSFileManager, etc.). This is what `RUNTIME_ARCHITECTURE.md` tracks.
- **71 fully shimmed classes**: The 52 above plus AppKit stubs (NSApplication, NSWindow, NSView, etc.) and synthetic/constant variants (`__NSCFConstantString`, `NSConstantIntegerNumber`, `NSConstantDictionary`, `NSConstantArray`). This is what the archived `GAP_ANALYSIS.md` tracks.

The list below uses the 71-class count, organized by domain.

### Core Object Model
`NSObject`, `NSNull`, `NSValue`, `NSNumber`, `NSConstantIntegerNumber`

### Strings
`NSString`, `NSMutableString`, `__NSCFConstantString`

### Collections
`NSArray`, `NSMutableArray`, `NSConstantArray`,
`NSDictionary`, `NSConstantDictionary`, `NSMutableDictionary`,
`NSSet`, `NSMutableSet`,
`NSOrderedSet`, `NSMutableOrderedSet`,
`NSCountedSet`,
`NSIndexSet`, `NSMutableIndexSet`,
`NSMapTable`, `NSHashTable`,
`NSSortDescriptor`, `NSEnumerator`

### Data & Bytes
`NSData`, `NSMutableData`, `NSURL`

### Text
`NSCharacterSet`, `NSMutableCharacterSet`,
`NSRegularExpression`, `NSTextCheckingResult`,
`NSAttributedString`, `NSMutableAttributedString`,
`NSScanner`

### Date & Time
`NSDate`, `NSDateFormatter`, `NSISO8601DateFormatter`,
`NSCalendar`, `NSTimeZone`, `NSLocale`, `NSTimer`

### Filesystem & Process
`NSFileManager`, `NSFileHandle`, `NSBundle`,
`NSProcessInfo`, `NSUserDefaults`,
`NSTask`, `NSPipe`

### Serialization
`NSJSONSerialization`, `NSPropertyListSerialization`

### Formatting
`NSNumberFormatter`, `NSByteCountFormatter`

### Error Handling
`NSError`, `NSException`

### Networking
`NSURL`, `NSURLComponents`, `NSURLQueryItem`,
`NSURLRequest`, `NSMutableURLRequest`,
`NSURLResponse`, `NSHTTPURLResponse`,
`NSHost`

### Identity & Progress
`NSUUID`, `NSProgress`

### Concurrency
`NSThread`, `NSLock`, `NSRecursiveLock`, `NSCondition`, `NSConditionLock`,
`NSOperation`, `NSBlockOperation`, `NSOperationQueue`

### ObjC Runtime
`NSMethodSignature`, `NSInvocation`

### Notifications
`NSNotificationCenter`, `NSNotification`

### Caching
`NSCache`

### AppKit Stubs (for `.app` bundle compatibility)
`NSApplication`, `NSRunLoop`, `NSWindow`, `NSView`, `NSEvent`, `NSScreen`,
`NSWorkspace`, `NSColor`, `NSFont`, `NSImage`, `NSResponder`,
`NSAppearance`, `NSUserNotificationCenter`, `NSMenu`, `NSMenuItem`, `NSNib`

### Shim Count

> **Note on shim counts**: Two figures appear in project documents and refer to
> different things:
>
> - **340+ total installed shims** — unique shim function-pointer registrations
>   across all 9 categories (ObjC: 103; Foundation: 172; libSystem: 65; misc).
>   This is the Registration Count.
>
> - **523+ total shims** — all Foundation method selectors individually counted
>   across 71 classes (Foundation: 355+; ObjC runtime: 103; libSystem: 65).
>   This is the Selector Count.
>
> Both are correct; they measure different things. The Registration Count (340+)
> reflects the number of `ShimTable::register()` calls at runtime startup. The
> Selector Count (523+) reflects the total number of `objc_msgSend`-dispatchable
> methods across all shimmed classes.

---

## libsystem Shims

Controlled implementations of macOS system library functions (`libSystem`,
`libpthread`, `libdispatch`, `libmach`). Each shim is an `extern "C"` Rust
function registered in the `ShimTable` and intercepted before any host call.

**65 libSystem shims across these categories:**

| Category | Key functions | Strategy |
|---|---|---|
| String / memory | `strlen`, `strcmp`, `strcpy`, `strdup`, `strlcpy`, `memcpy`, `memset`, `memmove`, `bzero` | Implement in Rust (pure functions) |
| I/O | `printf`, `fprintf`, `snprintf`, `puts`, `fwrite`, `fflush`, `fopen`, `fclose`, `fread`, `fgets` | Delegate to host (side effects on real stdout/stderr) |
| Allocation | `malloc`, `free`, `calloc`, `realloc` | Delegate to host (memory must be abi-compatible) |
| Process | `exit`, `_exit`, `abort`, `atexit`, `getenv`, `setenv`, `getpid`, `getppid` | Implement: `exit` → controlled teardown; `getenv` → from `RuntimeConfig` |
| Process identity | `getprogname`, `_NSGetExecutablePath`, `_NSGetArgc`, `_NSGetArgv`, `_NSGetEnviron` | Implement from `RuntimeConfig` |
| pthread | `pthread_self`, `pthread_create`, `pthread_mutex_*`, `pthread_once`, `pthread_main_np` | Single-threaded stubs; Linux uses `gettid() == getpid()` |
| TLS | `_tlv_atexit`, `tlv_get_addr`, `_tlv_bootstrap` | Implement: per-image TLS allocation from TLV descriptors |
| GCD / dispatch | `dispatch_once`, `dispatch_once_f`, `dispatch_async`, `dispatch_sync`, `dispatch_get_main_queue`, `dispatch_get_global_queue` | Self-hosted (no GCD dependency) |
| Mach | `mach_task_self_`, `mach_absolute_time`, `mach_timebase_info` | Real values on macOS; portable equivalents on Linux |
| Randomness | `arc4random`, `arc4random_buf`, `arc4random_uniform` | `getrandom(2)` on Linux; native on macOS |
| Stack protection | `__stack_chk_fail`, `__stack_chk_guard` | Delegate to host (same process ABI requirement) |
| Crypto | `CC_SHA256` → replaced with pure Rust `sha2` crate | No CommonCrypto dependency |

**Explicit deny list** (cannot be called through the runtime): `system`, `popen`,
`fork`, `exec*`, `posix_spawn`, `dlopen`, `dlsym` (except through the controlled
`StubResolver` path).

---

## ARM64 Interpreter & Translation

The `arm64-translate` crate implements a pure-Rust ARM64 interpreter enabling
macOS ARM64 binaries to run interpretively on any host architecture.

### Architecture

| Module | Role |
|---|---|
| `cpu.rs` | ARM64 register state: x0–x30, SP, PC, NZCV flags, condition evaluation |
| `decode.rs` | A64 instruction decoder — 55+ instruction variants, bitmask immediates |
| `mem.rs` | Guest → host memory model: BTreeMap regions, interpreter stack, host passthrough via `mincore()` |
| `ffi.rs` | FFI bridge: registered handlers + auto-stub for unmapped targets; full ABI return capture (x0, x1, d0, d1, x8 indirect) |
| `interp.rs` | Fetch-decode-execute loop; sentinel LR for return detection; `TailCall` variant for `objc_msgSend` dispatch |

### Implemented Instruction Groups

- Data processing: ADD/SUB/AND/ORR/EOR/CMP/TST/MOV and shift variants
- Multiply: MUL/MADD/MSUB, wide multiply (SMADDL, UMADDL, SMULH, UMULH)
- Bitfield: SBFM/UBFM/BFM and all aliases (SXTW, SXTB, SXTH, UXTB, UXTH, UBFX, SBFX, LSL/LSR imm)
- Shifts: LSLV, LSRV, ASRV, RORV
- Add/sub with carry: ADC, ADCS, SBC, SBCS
- Loads/stores: LDR/STR (all sizes), LDRSB/LDRSH/LDRSW, LDUR/STUR, LDP/STP, pre/post-index
- Register-indexed loads/stores with extend/shift options
- Branches: B, BL, BLR, BR, RET, B.cond, CBNZ/CBZ, TBNZ/TBZ
- Conditional select: CSEL, CSINC, CSINV, CSNEG and aliases (CSET, CSETM, CINC, CINV, CNEG)
- Conditional compare: CCMP, CCMN
- System: NOP, MSR/MRS (user-accessible registers), YIELD, WFI stubs

### Current Status

| Sub-phase | Status | Details |
|---|---|---|
| E3a — Interpreter MVP | ✅ Complete | `return42`, `simple_cli` pass |
| E3b — Decoder Expansion | ✅ Complete | Bitfield, signed loads, reg-indexed ld/st, variable shifts; 4 additional fixtures pass |
| E3c — ObjC/Foundation Integration | ✅ Complete | 14 ObjC/Foundation fixtures pass via interpreter; `objc_msgSend` via TailCall |
| E4a — Full ABI Return Capture | ✅ Complete | FP returns (d0/d1), struct returns (x8), variadic stack args |
| E4b — Sign-Extension & Stack Alignment | ✅ Complete | Byte/halfword addr bugs fixed; 16-byte SP alignment before FFI calls |
| E4c — ObjC Category +load | ✅ Complete | `+load` fires in interpreter; superclass → class → category ordering |
| **22 fixtures pass via interpreter** | macOS ARM64 host | return42, simple_cli, constructor_test, objc_minimal, objc_classes, objc_inherit, nsstring_ops, nslog_format_specifiers, nsurl_basics, nsindexset_basics, nsnotification_basics, nscache_basics, nsobject_methods, kvc_simple, nscalendar_basics, nscharacterset_basics, nserror_basics, nsmutablearray_basics, nsmutabledictionary_basics, objc_category |
| E3d — JIT / AOT Translation | 🔲 Future | Basic block → host code generation; self-modifying code handling |
| Linux CI | 🔲 In progress | Docker image exists; CI matrix not yet wired |

---

## Native GUI Framework

A software-rendered and GPU-accelerated GUI framework built in Rust, independent
of AppKit's C++ / Objective-C internals.

| Component | Crates | Detail |
|---|---|---|
| Text layout engine | `graphics-core` | `ab_glyph` TrueType/OpenType font loading; `FontManager` with fallback chain; single-line and multi-line layout; attributed rendering (bold, italic, color, size per range); Unicode NFC normalization, emoji support |
| Software renderer | `graphics-core` | `SoftwareRenderer`: scanline path fill (even-odd rule), nearest-neighbor image blit, bilinear gradients, shadow blur |
| GPU-accelerated renderer | `graphics-core` | `GpuRenderer` via `wgpu`: 1,024×1,024 glyph atlas, quad batching, WGSL shader; behind optional `gpu` feature flag |
| Drawing primitives | `graphics-core` | `BezierPath` (cubic/quad splines, flatten to segments); `Gradient` (linear/radial, per-pixel sampling); `Shadow`; `ImageBuffer` (PNG/JPEG via `imaging` feature); scanline fill |
| NSImage backing store | `cf-lite` | `initWithContentsOfFile:`, `initWithData:`, `size`, `setSize:`, `drawInRect:` shims with backing ImageBuffer |
| View system | `ui-view` | View tree, `frame` / `bounds`, `addSubview:`, `setNeedsDisplay:` |
| Controls | `ui-controls` | Label, Button, TextField, ScrollView, StackView, TableView, SplitView — 7 controls |
| Auto Layout Lite | `app-kit-lite` | Constraint solver (`LayoutConstraint`), intrinsic content sizes, hugging/compression resistance priorities, conflict detection |
| Run loop | `runloop-core` | Single-threaded event loop abstraction; `NSRunLoop` modes: `NSDefaultRunLoopMode`, `NSRunLoopCommonModes` |
| Window backend | `window-backend`, `window-backend-darwin` | Platform window integration; Darwin backend (`window-backend-darwin`) |
| Demo apps | `apps/hello-darwin`, `apps/control-gallery`, `apps/foundation-explorer` | 3 working GUI applications built on the framework |

---

## Platform Abstraction Layer

The PAL separates all OS-specific calls behind Rust traits in `platform-core`.
Both macOS and Linux have complete implementations.

| Trait | Purpose | macOS impl | Linux impl |
|---|---|---|---|
| `MemoryMapper` | `mmap` / `mprotect` / `munmap` | `DarwinMemoryMapper` | `LinuxMemoryMapper` |
| `HostSymbolResolver` | `dlsym` / `dlopen` for host libc | `DarwinHostSymbolResolver` | `LinuxHostSymbolResolver` |
| `UnwindRegistry` | Unwind info registration | `DarwinUnwindRegistry` (via `__unw_add_find_dynamic_unwind_sections`) | `LinuxUnwindRegistry` (via `__register_frame` / `__deregister_frame`) |
| `Clock` | Monotonic time, wall time, timezone | `DarwinClock` | `LinuxClock` (`clock_gettime`, `/etc/timezone`) |
| `Rng` | Cryptographic randomness | `DarwinRng` (`arc4random_buf`) | `LinuxRng` (`getrandom(2)`) |
| `Hasher` | SHA-256 for CDHash | `DarwinHasher` (pure Rust `sha2`) | `LinuxHasher` (pure Rust `sha2`) |
| `FilesystemProvider` | File reads, metadata, symlink resolution | `DarwinFilesystem` | `LinuxFilesystem` (`std::fs`) |
| `HostObjCBridge` | Host ObjC runtime interop (off by default) | Permissive / disabled | N/A |
| `ProcessInfo` | PID, thread identity, main-thread check | `DarwinProcessInfo` | `LinuxProcessInfo` (`gettid() == getpid()`) |
| `Threading` | Mutex, condvar, pthread wrappers | `DarwinThreading` | `LinuxThreading` (POSIX) |

**`PlatformProvider`**: A top-level struct (`DarwinHostPlatform`, `LinuxHostPlatform`)
that composes all 10 traits and is injected into `RuntimeConfig` at construction time.

---

## FFI & Package Integration

The runtime is consumable from outside of Rust via a stable C ABI and packaged
for common distribution channels.

| Integration | Crate / File | Detail |
|---|---|---|
| C dylib | `darwin-runtime-ffi` + `cbindgen` | `libdarwin_runtime.dylib` / `.so` with a C header auto-generated by `cbindgen` |
| pkg-config | `Formula/burdettieOS.pc` | Standard `.pc` file for `pkg-config --cflags --libs burdettieOS` |
| Swift Package Manager | `swift/` | Swift wrapper types over the C dylib; distributable as an SPM package |
| Homebrew formula | `Formula/burdettieOS.rb` | Tap installable via `brew install` |
| C client example | `examples/c-client/` | Demonstrates calling the C dylib from plain C |
| Swift client example | `examples/swift-client/` | Demonstrates calling the Swift wrapper from a Swift app |
