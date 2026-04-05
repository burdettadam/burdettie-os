# Appendix A: The Stack Map

This appendix is a reference. The chapters told a story — of parsing headers and mapping segments and building an ObjC runtime from scratch. This appendix lays out everything we built, measures how close it comes to the real thing, and marks, with precision, the boundaries where our implementation ends and the frontier begins.

Use it as a map. If you want to know which crate handles a specific responsibility, Scene A.1 has the answer. If you want to know how many selectors Foundation registers, Scene A.2 has the number. If you want to understand *why* Mach-O parsing aligns almost perfectly with Apple's implementation while Security.framework diverges almost completely, Scenes A.3 and A.4 have the reasoning.

The numbers in this appendix are current as of the final chapter's completion: 41 crates, ~197,000 lines of Rust, 84/84 compatibility fixtures passing, 51+ Xcode development tools running on Linux, 39 corpus programs verified end-to-end.

---

## Scene A.1: Full Stack Diagram

Every macOS binary that runs through our loader traverses a stack of translation layers, each one built on the layer beneath it. The data flows from raw bytes on disk through parsing, memory mapping, dynamic linking, runtime setup, and finally execution — either natively on macOS or through interpretation on foreign architectures. Here is the complete stack, from bottom to top:

```
┌──────────────────────────────────────────────────────────┐
│                    Guest Application                      │
│            (macOS Mach-O ARM64/arm64e binary)             │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Layer 8: GUI Bridge             [Ch 10]            │ │
│  │  app-kit-lite · window-backend-darwin                │ │
│  │  NSApplication → winit event loop · softbuffer       │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 7: Foundation Shims       [Ch 5]             │ │
│  │  cf-lite (206 .rs files)                            │ │
│  │  193 ObjC classes · 1,380 selectors · 2,001 shims   │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 6: ObjC Runtime           [Ch 4]             │ │
│  │  objc-runtime                                       │ │
│  │  objc_msgSend trampoline · class tables · protocols │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 5: System Bridge          [Ch 6, 8]          │ │
│  │  libsystem-shims (57 .rs files)                     │ │
│  │  darwin-runtime-ffi                                  │ │
│  │  POSIX · Mach · GCD · XPC · Security stubs          │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 4: ARM64 Interpreter      [Ch 7]             │ │
│  │  arm64-translate                                    │ │
│  │  122 instruction variants · auto-FFI on BLR escape  │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 3: Dynamic Linker         [Ch 3]             │ │
│  │  macho-resolve · macho-exec (fixup/)                │ │
│  │  Chained fixups · bind/rebase · GOT/stub patching   │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 2: Segment Mapper         [Ch 2]             │ │
│  │  macho-exec (mapped.rs · region.rs)                 │ │
│  │  mmap · page-aligned segments · protection bits     │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 1: Binary Parser          [Ch 1–2]           │ │
│  │  macho-parse · macho-types                          │ │
│  │  Header · load commands · segments · sections       │ │
│  └────────────────────────┬────────────────────────────┘ │
│  ┌────────────────────────▼────────────────────────────┐ │
│  │  Layer 0: Orchestration                             │ │
│  │  macho-runtime (typestate) · compat-runner (CLI)    │ │
│  │  7-phase pipeline: Parse→Check→Map→Fix→Reg→Prot→Init│ │
│  └─────────────────────────────────────────────────────┘ │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                      Host OS                              │
│     (macOS: native syscalls · Linux: translated ABI)      │
└──────────────────────────────────────────────────────────┘
```

### Layer 0: Orchestration

Two crates own the top-level execution flow. `macho-runtime` implements a typestate loader — a compile-time state machine where each loading phase is a distinct Rust type: `Loader<Parsed>`, `Loader<Checked>`, `Loader<Mapped>`, `Loader<FixedUp>`, `Loader<Registered>`, `Loader<Protected>`, `Loader<Initialized>`. You can't call `apply_fixups()` on a `Loader<Parsed>` because it doesn't exist on that type. The compiler enforces the loading sequence. Mis-ordering phases is a type error, not a runtime crash.

The seven phases are:

| Phase | Typestate | Responsibility |
|-------|-----------|----------------|
| 1. Parse | `Parsed` | Read Mach-O header and load commands |
| 2. Check | `Checked` | Validate architecture, minimum OS, code signature |
| 3. Map | `Mapped` | Allocate memory, map segments, create GOT/stubs |
| 4. Fix | `FixedUp` | Apply chained fixups, bind symbols, rebase pointers |
| 5. Register | `Registered` | Initialize ObjC runtime, register classes/selectors/protocols |
| 6. Protect | `Protected` | Set final memory protections (R/W/X) |
| 7. Initialize | `Initialized` | Run `mod_init_func` constructors, call `+load` methods, invoke `main()` |

`compat-runner` is the CLI wrapper: `darwin-compat scan corpus/` runs all 39 corpus tools and reports pass/fail, `darwin-compat run <binary>` loads a single Mach-O. `macho-inspect` is the diagnostic variant with detailed phase-by-phase telemetry.

The BFS dependency loader in `macho-runtime` also handles recursive image loading. When a binary links `Foundation.framework`, the loader doesn't load Apple's Foundation — it registers our synthetic Foundation classes (Layer 7). When it links a host library that *can* be forwarded — `libz`, `libbz2`, `libssl` — it `dlopen`s the Linux-native library and binds symbols directly.

### Layer 1: Binary Parser

`macho-parse` reads raw bytes into structured Rust types. Its entry point, `ParsedMachO::parse()`, returns a complete representation of the binary: header fields (CPU type, file type, flags), load commands (each variant parsed to its concrete type), and section metadata (names, offsets, sizes, alignments). It handles both LC_SEGMENT_64 and the older LC_SEGMENT. It parses chained fixup import tables, dylib dependencies, code signature super-blobs, and UUID load commands.

`macho-types` is the type-only companion crate — pure struct definitions with no parsing logic. This separation exists so downstream crates that only need type information (for display, comparison, or serialization) don't pay the parsing cost.

The key data flowing out of this layer: a `ParsedMachO` struct containing all the information needed to map the binary into memory and resolve its symbols. The `parse_cache` module caches parsed binaries per-process, so repeated loads of the same path (common in dependency resolution) cost a HashMap lookup instead of a fresh parse.

### Layer 2: Segment Mapper

`macho-exec`'s `MappedImage` takes a `ParsedMachO` and a `MemoryMapper` trait implementation, then maps each segment into the process address space. On macOS, this calls `mmap` with the segment's file offset and size. On Linux, it does the same — the raw bytes are platform-independent at this point.

Each segment gets mapped at its declared virtual address offset from a slide base. The `__PAGEZERO` segment is skipped (it exists to catch null dereferences in the original macOS process). `__TEXT` is mapped read-execute. `__DATA` and `__DATA_CONST` are mapped read-write (temporarily — Layer 6 will tighten protections). `__LINKEDIT` provides the raw material for fixup resolution.

The GOT (Global Offset Table) and lazy stub structures are allocated here. The mapper creates synthetic stub entries for every external symbol the binary references — function pointers that initially point to trap addresses. Layer 3 fills them in.

### Layer 3: Dynamic Linker

This is where the binary becomes a living process. `macho-resolve` resolves symbol names to addresses. `macho-exec`'s fixup module walks the chained fixup data and writes resolved addresses into the GOT and stub slots.

Three fixup formats are supported:

| Format | Description | Used By |
|--------|-------------|---------|
| Chained (DYLD_CHAINED_FIXUPS) | Modern format with import table | Most arm64e binaries |
| Classic bind/rebase (DYLD_INFO_ONLY) | Opcode stream | Older arm64 binaries |
| Legacy (DYSYMTAB) | Indirect symbol tables | Very old binaries |

For arm64e (pointer-authenticated) binaries, the fixup phase also PAC-signs resolved addresses. Auth-bind entries specify a signing key (IA, IB, DA, DB), a diversity value, and whether to use address diversification. The `ptrauth.rs` module generates PACIA/PACDA/PACIB/PACDB instructions to produce correctly signed pointers — the binary's GOT entries contain the full PAC signature that arm64e hardware will verify on use.

The `StubResolver` maintains a safe-list of 600+ symbols that may be resolved from host libraries. Symbols not on the safe-list — `fork`, `exec`, `posix_spawn`, `system` — return trap addresses that abort with a diagnostic message if called. Symbols on the safe-list are resolved via `dlsym`, either from the host libc or from pre-loaded Linux-native libraries (libbz2, zlib, liblzma, libarchive, libssl, libcrypto, libacl).

### Layer 4: ARM64 Interpreter

On non-ARM64 hosts, `arm64-translate` provides a software CPU. It decodes and executes 122 ARM64 instruction variants: 86 integer operations (arithmetic, logic, shifts, branches, loads/stores, compare-and-branch, conditional select, bitfield operations) and 36 floating-point/SIMD operations (FADD through FCSEL, load/store pairs, register-indexed addressing).

The interpreter maintains 31 general-purpose registers (X0–X30), SP, PC, NZCV condition flags, and 32 SIMD registers (V0–V31, accessed as Q/D/S/H/B slices). Memory is modeled as a flat address space — the interpreter reads and writes the same mapped memory that the native path uses.

The critical feature is auto-FFI: when the interpreter encounters a BLR (Branch with Link to Register) to an address outside the mapped guest image — a host function pointer, typically from the GOT — it packages the current register state into an `FfiCallArgs` struct and performs a native function call. X0–X7 become the first eight arguments. D0–D7 pass floating-point parameters. X8 carries the indirect result pointer for large struct returns. The host function returns, its results land back in the interpreter's registers, and execution continues at the link return address. This bridge is what makes the interpreter able to call into our shim layer, into host libc, into anything with a C calling convention.

### Layer 5: System Bridge

`libsystem-shims` is the biggest translation layer by concept count. It maps macOS's `libSystem.B.dylib` API surface onto host primitives.

Three categories of shims:

**Implement**: Pure Rust replacements. `arc4random` wraps the host's random source. `getprogname` returns a cached static string. `strlen` includes defensive checks for invalid pointers (small integers, tagged pointers, kernel-range addresses) because guest binaries can pass corrupt data.

**Delegate**: Forward to host with translation. `stat` calls the host `stat` but translates macOS's 144-byte struct layout into Linux's 128-byte layout (or vice versa). `readdir` translates macOS's 1048-byte `dirent` (with `d_name` at offset 21) into Linux's 280-byte layout (with `d_name` at offset 19). `setlocale` translates macOS's `LC_ALL=0` to Linux's `LC_ALL=6`. The variadic shims are the most architecturally complex: Apple's ARM64 ABI puts *all* variadic arguments on the stack, while Linux's AAPCS64 puts the first arguments in registers. Naked assembly shims repackage Apple-style stack arguments into a Linux `va_list` struct and tail-call the `v`-variant (`fprintf` → `vfprintf`, etc.).

**Deny**: Return errors or abort. `fork` is denied because forking a process that contains our synthetic runtime state would create an inconsistent child. `ptrace` is denied because we don't emulate the Mach debug interface.

`darwin-runtime-ffi` handles the C FFI boundary — `#[no_mangle] extern "C"` exports that the guest binary's GOT entries point to after fixup resolution.

The shim install order matters. Some shims depend on others being registered first:

```
Process → TLS → Dispatch → Pthread → Mach → Ptrauth →
Lifecycle → Random → Socket → DarwinCompat →
cross_platform → darwin_stubs
```

Process shims install first because `getprogname()` is called by error-reporting shims installed later. TLS installs before Pthread because `pthread_key_create` needs the TLS infrastructure. This order is encoded in `install_default_shims()` — getting it wrong causes shims to call uninstalled shims, producing null function pointer calls instead of useful behavior.

### Layer 6: ObjC Runtime

`objc-runtime` is a from-scratch Objective-C runtime. Not a binding to Apple's runtime. Not a wrapper around `libobjc`. A complete implementation: class tables, selector interning, method lookup, protocol conformance, dynamic class creation, category merging, super dispatch, and the assembly `objc_msgSend` trampoline.

The trampoline lives in `trampoline.s` — hand-written ARM64 assembly that saves all argument registers (x0–x7, d0–d7, x8 for struct returns), calls `objc_msg_lookup_imp` to resolve the selector on the receiver's class, and tail-calls the resolved IMP. If the IMP is a host function pointer, the call crosses into native code. If it's a shim, it crosses into our Foundation layer. If the binary is running under the interpreter, BLR-to-IMP triggers auto-FFI, which crosses back into interpreted guest code after the shim returns.

The runtime registers classes from two sources: the guest binary's `__objc_classlist` section (classes the binary itself defines) and our synthetic Foundation template (classes we provide as substitutes for Apple's frameworks). A `OnceLock<ObjcRuntime>` caches the Foundation template — 193 pre-built classes with method tables — and each loader run clones it via Arc reference counting rather than deep-copying 193 class records and 1,380 selector entries.

### Layer 7: Foundation Shims

`cf-lite` is the largest crate in the project by every measure: 206 source files, 193 registered ObjC classes (148 NS* Foundation classes, 40+ AppKit stubs, plus CoreAnimation and other framework classes), 1,380 registered selectors, and 2,001 individual shim function implementations. It provides the vocabulary that the ObjC runtime dispatches to.

Foundation's scope is enormous. `NSString` alone has 100+ methods. `NSArray`, `NSDictionary`, `NSData`, `NSDate`, `NSURL`, `NSError`, `NSFileManager`, `NSProcessInfo`, `NSBundle`, `NSRegularExpression`, `NSJSONSerialization`, `NSPropertyListSerialization`, `NSKeyedArchiver`, `NSKeyedUnarchiver`, `NSURLSession`, `NSTask`, `NSThread`, `NSRunLoop`, `NSNotificationCenter`, `NSUserDefaults` — each one a class with dozens of methods, each method a full implementation backed by Rust code.

The toll-free bridging model from CoreFoundation is preserved: `CFStringRef` and `NSString *` are the same object. `CFArrayRef` and `NSArray *` share a layout. `CFDictionaryRef` and `NSDictionary *`. This isn't cosmetic — guest binaries cast freely between CF and NS types, and our shim layer must handle both C-function-style CF calls (`CFStringGetLength`) and ObjC-message-style NS calls (`[string length]`) on the same underlying object.

### Layer 8: GUI Bridge

`app-kit-lite` and `window-backend-darwin` provide a structural AppKit implementation — enough to open an `NSWindow`, drive an `NSApplication` run loop, and dispatch events through the responder chain. The actual rendering uses `winit` for window management and `softbuffer` for pixel-buffer output. A `GuiBridge` trait decouples the Foundation-level AppKit shims from the platform-specific windowing backend.

This layer is the thinnest. Most CLI tools never touch it. It exists because some developer tools — Instruments, Xcode itself, Interface Builder — require at minimum an `NSApplication` instance and basic window creation. The structural stubs ensure those tools can initialize without crashing, even if the windows aren't fully functional.

### The Platform Abstraction Layer

Cutting across all layers is the PAL — a set of traits defined in `platform-core` that abstract host-specific capabilities:

| PAL Trait | Callsites | Purpose |
|-----------|-----------|---------|
| `MemoryMapper` | 4 | Virtual memory mapping for segments |
| `HostSymbolResolver` | 15 | `dlsym`-based host function resolution |
| `UnwindRegistry` | 3 | Exception unwind table registration |
| `HostObjCBridge` | 5 trampolines | ObjC message dispatch to host |
| `Clock` | 6 | `mach_absolute_time`, `gettimeofday` |
| `Rng` | 1 | `arc4random` |
| `FilesystemProvider` | 12 | File I/O abstraction |
| `ProcessInfo` | 5 | `getpid`, environment, hostname |
| `ThreadPrimitives` | 50+ | pthread, mutex, TLS |

`platform-darwin-host` implements these traits for macOS using libc and CommonCrypto. `platform-linux` implements them for Linux using libc and equivalent POSIX interfaces. The PAL is what makes the same loader binary run on both platforms — the shim code calls `platform().clock().monotonic_time()` instead of `mach_absolute_time()`, and the trait dispatch routes to the right implementation.

### Complete Crate Reference

| Crate | Role | Layer | Chapter |
|-------|------|-------|---------|
| `macho-types` | Mach-O struct definitions | 1 | Ch 1 |
| `macho-parse` | Binary parsing | 1 | Ch 1–2 |
| `macho-exec` | Mapping, fixups, TLV, unwind | 2–3 | Ch 2–3 |
| `macho-resolve` | Symbol resolution | 3 | Ch 3 |
| `macho-objc` | ObjC metadata extraction | 3 | Ch 3 |
| `objc-runtime` | Full ObjC runtime | 6 | Ch 4 |
| `cf-lite` | Foundation + AppKit shims | 7 | Ch 5 |
| `libsystem-shims` | libSystem translation | 5 | Ch 6, 8 |
| `darwin-runtime-ffi` | C FFI boundary | 5 | Ch 6, 8 |
| `arm64-translate` | Software ARM64 CPU | 4 | Ch 7 |
| `macho-runtime` | Typestate loader pipeline | 0 | Ch 2–9 |
| `compat-runner` | CLI: scan, run, benchmark | 0 | Ch 9 |
| `macho-inspect` | Diagnostic loader CLI | 0 | Ch 9 |
| `platform-core` | PAL traits (10 traits) | — | Ch 6 |
| `platform-darwin-host` | macOS PAL implementation | — | Ch 6 |
| `platform-linux` | Linux PAL implementation | — | Ch 8 |
| `app-kit-lite` | NSApplication, events, menus | 8 | Ch 10 |
| `window-backend` | Window trait + headless | 8 | Ch 10 |
| `window-backend-darwin` | winit + softbuffer backend | 8 | Ch 10 |
| `window-backend-linux` | Linux windowing backend | 8 | Ch 10 |
| `ui-view` | View tree, hit testing, layout | 8 | Ch 10 |
| `ui-controls` | Label, Button, TextField, etc. | 8 | Ch 10 |
| `graphics-core` | 2D rendering, bezier, gradient | 8 | Ch 10 |
| `runloop-core` | Run loop, timers, deferred blocks | 6 | Ch 5 |
| `cg-shims` | CoreGraphics type stubs | 7 | Ch 5 |
| `icu-shims` | ICU Unicode shims | 5 | Ch 8 |
| `libdispatch-sys` | GCD FFI bindings | 5 | Ch 6 |
| `macho-shared-cache` | dyld shared cache parsing | 1 | Ch 2 |
| `macho-map` | Memory map visualization | — | — |
| `macho-diff` | Binary comparison tool | — | — |
| `macho-fixtures` | Test binary management | — | — |
| `macho-run-plan` | Execution plan builder | 0 | — |
| `swift-stub-gen` | Swift metadata stub generation | — | — |
| `trust-eval` | Code signature evaluation | 2 | Ch 2 |
| `trust-policy` | Signing policy definitions | 2 | Ch 2 |
| `runtime-diagnostics` | DiagRing telemetry | 0 | Ch 9 |
| `runtime-demo` | Demo binaries | — | — |
| `runtime-shell` | REPL for loader exploration | — | — |
| `perf-bench` | Benchmarking harness | — | Ch 9 |
| `test-harness` | Integration test framework | — | — |

Forty-one crates. ~197,000 lines of Rust. Zero unsafe blocks outside of assembly trampolines and FFI boundary code.

---

## Scene A.2: Parity Scoreboard

How close are we to the real thing? The answer depends on the framework. Some subsystems are nearly complete. Others are deliberate stubs — structural enough to prevent crashes, empty enough that we don't pretend capability we don't have.

This section presents the data. Every number comes from the actual codebase: `grep -r "register_shims!" | wc -l`, selector counts from registration files, test counts from `cargo test` output.

### libSystem Coverage

libSystem is the platform substrate — the layer between user code and the kernel. Our coverage here is the highest because it's the most critical and the most straightforward to implement: much of libSystem is standard POSIX.

| Category | Files | Shimmed Functions | Coverage | Notes |
|----------|-------|-------------------|----------|-------|
| **POSIX I/O** | `process.rs`, cross_platform/ | open, close, read, write, stat, fstat, lstat, fstatat, readdir, opendir, closedir, mkdir, rmdir, unlink, rename, access, chmod, chown, link, symlink, readlink, realpath, getcwd, chdir, dup, dup2, pipe, fcntl, ftruncate, fsync, flock, mmap, munmap, mprotect | ~92% | Struct translations for stat (144→128B), dirent (1048→280B), statfs (2168→120B) on Linux |
| **Process** | `process.rs`, `lifecycle.rs` | getpid, getppid, getuid, geteuid, getgid, getegid, getprogname, setprogname, isatty, getenv, setenv, unsetenv, putenv, exit, _exit, abort, atexit, __cxa_atexit | ~90% | `fork`/`exec` denied; `_NSGetArgv`/`_NSGetArgc`/`_NSGetProgname`/`_NSGetEnviron` shimmed |
| **Threading** | `pthread/`, `tls.rs` | pthread_create, pthread_join, pthread_detach, pthread_self, pthread_mutex_init/lock/unlock/destroy, pthread_rwlock_*, pthread_cond_*, pthread_key_create/setspecific/getspecific, pthread_once, pthread_attr_* | ~85% | 50+ callsites through ThreadPrimitives PAL trait |
| **Mach** | `mach.rs` | mach_task_self, mach_host_self, mach_absolute_time, mach_timebase_info, host_page_size, vm_allocate, vm_deallocate | ~40% | `mach_task_self_` is a data symbol (not function). Mach IPC not implemented |
| **GCD** | `dispatch.rs`, `dispatch_concurrency.rs` | dispatch_once, dispatch_queue_create, dispatch_async, dispatch_sync, dispatch_get_main_queue, dispatch_get_global_queue, dispatch_group_*, dispatch_semaphore_*, dispatch_source_* | ~60% | Synchronous execution model; async dispatches run synchronously |
| **XPC** | `xpc.rs`, `xpc_objects.rs`, `xpc_pipe.rs` | xpc_connection_create, xpc_connection_set_event_handler, xpc_dictionary_create, xpc_dictionary_get_string, xpc_string_create, xpc_pipe_routine | ~15% | Structural stubs only — no launchd IPC, no XPC service hosting |
| **Security** | `security.rs`, `common_crypto.rs` | CC_SHA256, CC_MD5, CC_SHA1, SecRandomCopyBytes, SecItem* stubs | ~10% | Crypto hash delegation to host; keychain/TLS/codesign are stubs |
| **Variadic ABI** | `variadic_shims.rs` | 33 shims: fprintf→vfprintf, printf→vprintf, snprintf→vsnprintf, sprintf→vsprintf, dprintf→vdprintf, asprintf→vasprintf, sscanf→vsscanf, fscanf→vfscanf, scanf→vscanf, syslog→vsyslog, err/errx/warn/warnx, plus `_chk` variants | 100% | Naked assembly; translates Apple stack-only variadic ABI to Linux register-first ABI |
| **Socket** | `socket.rs` | socket, connect, bind, listen, accept, send, recv, sendto, recvfrom, setsockopt, getsockopt, getaddrinfo, freeaddrinfo | ~70% | Socket option constant translation between macOS and Linux |
| **Random** | `random.rs` | arc4random, arc4random_buf, arc4random_uniform, SecRandomCopyBytes | ~95% | Delegates to host `/dev/urandom` or `getrandom` |
| **PAC/Ptrauth** | `ptrauth.rs` | ptrauth_sign_unauthenticated, ptrauth_strip, ptrauth_blend_discriminator | 100% | Real PACIA/PACDA on ARM64; no-op on non-PAC hosts |
| **Locale/ICU** | `darwin_compat.rs`, `cross_platform/icu.rs` | setlocale (category translation), __mb_cur_max (data symbol), nl_langinfo, ___maskrune, __tolower, __toupper | ~50% | LC_* constant mapping (macOS LC_ALL=0, Linux LC_ALL=6) |

**Aggregate libSystem coverage**: ~92% of symbols referenced by the 39 corpus tools and 51+ Xcode tools resolve successfully. The gaps are concentrated in Mach IPC (which no corpus tool exercises), XPC services (stub-only), and advanced Security.framework operations.

### Foundation Coverage

Foundation is the largest implementation surface. The numbers tell the story:

| Metric | Count |
|--------|-------|
| Registered ObjC classes | 193 |
| — NS* Foundation classes | 148 |
| — AppKit classes | 40+ |
| — CoreAnimation classes | 4 |
| — Other framework stubs | ~5 |
| Registered selectors | 1,380 |
| — Foundation selectors | 867 |
| — AppKit selectors | 513 |
| Shim function implementations | 2,001 |
| Source modules | 206 .rs files |
| Unit tests | 2,682 |

Coverage varies dramatically by class. Some classes are nearly complete. Others are structural shells.

**High-coverage Foundation classes** (>70% of commonly-used selectors):

| Class | Methods Shimmed | Key Capabilities |
|-------|----------------|------------------|
| NSString | 100+ | Creation, formatting, comparison, searching, encoding (UTF-8/UTF-16), path operations, substring, range-based ops, case conversion |
| NSMutableString | 30+ | appendString, appendFormat, insertString, replaceCharacters, deleteCharacters |
| NSArray / NSMutableArray | 50+ | Creation, indexing, enumeration, sorting, filtering, mapping, subscript operators |
| NSDictionary / NSMutableDictionary | 40+ | Creation, key lookup, enumeration, allKeys/allValues, merge, subscript |
| NSData / NSMutableData | 25+ | Creation, bytes/length, base64, file I/O, append, subdataWithRange |
| NSNumber | 20+ | Integer/float/bool creation, boxing, unboxing, comparison, NSCoding |
| NSURL | 25+ | Creation, components (scheme, host, path, query), file URLs, relative URLs |
| NSError | 15+ | Creation, domain/code/userInfo, localizedDescription |
| NSDate | 20+ | Creation, timeIntervalSince*, comparison, date arithmetic |
| NSFileManager | 20+ | fileExists, createDirectory, contentsOfDirectory, moveItem, removeItem, attributesOfItem |
| NSProcessInfo | 15+ | processName, arguments, environment, hostName, OS version, processorCount |
| NSRegularExpression | 12+ | Pattern creation, firstMatch, matchesInString, enumerateMatches, fast-path for no-capture patterns |
| NSJSONSerialization | 6 | dataWithJSONObject, JSONObjectWithData, isValidJSON, read from file |
| NSPropertyListSerialization | 6 | XML and binary plist parsing, plist writing |
| NSThread | 10+ | currentThread, isMainThread, threadDictionary, sleep, detachNewThreadSelector |

**Medium-coverage Foundation classes** (30–70%):

| Class | Methods Shimmed | Status |
|-------|----------------|--------|
| NSBundle | 12 | mainBundle, bundleWithPath, infoDictionary, pathForResource, loadNibNamed stub |
| NSURLSession | 10 | Synchronous HTTP via ureq; dataTaskWithURL, dataTaskWithRequest with completion handler |
| NSTask | 15 | Launch via posix_spawn, stdout/stderr/stdin pipes, environment, currentDirectoryPath |
| NSKeyedArchiver/Unarchiver | 8 | archiveRootObject:toFile:, unarchiveObjectWithFile:, basic encode/decode |
| NSCalendar | 12 | currentCalendar, components, dateFromComponents, dateByAdding |
| NSNotificationCenter | 10 | addObserver, postNotification, removeObserver, block-based observers |
| NSUserDefaults | 10 | standardUserDefaults, objectForKey, setObject, synchronize, registerDefaults |

**Structural stubs** (~30% or less — crash prevention, not full behavior):

| Class | Purpose | Stub Strategy |
|-------|---------|---------------|
| NSURLCredential | Auth credential storage | Returns empty credential objects |
| NSHTTPCookieStorage | Cookie jar management | Empty singleton, no persistence |
| NSXMLParser | SAX XML parsing | Struct allocation, no parsing |
| NSPredicate | Query language | String storage, no evaluation engine |
| NSOperation / NSOperationQueue | Task scheduling | Synchronous execution |
| NSCache | Memory-aware storage | NSDictionary wrapper, no eviction policy |
| NSProgress | Progress reporting | Key-value storage, no cross-process |
| NSUndoManager | Undo/redo stack | Stub allocation |

### AppKit Coverage

AppKit coverage is deliberately structural. The goal isn't to render macOS windows — it's to prevent developer tools from crashing during initialization.

| Class | Instance Size | Methods | Purpose |
|-------|--------------|---------|---------|
| NSApplication | 56 bytes | sharedApplication, run, terminate, sendAction, windows, keyWindow, mainWindow, currentEvent, orderedWindows, targetForAction | Event loop, window management |
| NSWindow | 80 bytes | initWithContentRect, makeKeyAndOrderFront, setTitle, setFrame, close, contentView, firstResponder, makeFirstResponder, isVisible | Window lifecycle |
| NSView | 72 bytes | initWithFrame, addSubview, removeFromSuperview, subviews, setFrame, bounds, frame, window, superview, wantsLayer, layer | View hierarchy |
| NSButton | 24 bytes | initWithFrame, setTitle, title, setAction, action, setTarget, state | Control |
| NSTextField | 32 bytes | initWithFrame, stringValue, setStringValue, isEditable | Text input |
| NSMenu / NSMenuItem | 24/56 bytes | addItem, addItemWithTitle, numberOfItems, itemAtIndex, separatorItem, action, keyEquivalent | Menu structure |
| NSEvent | 64 bytes | type, locationInWindow, modifierFlags, characters, keyCode, window, clickCount, pressure, timestamp | Event representation |
| NSResponder | 16 bytes | nextResponder, keyDown, keyUp, mouseDown, mouseUp | Responder chain |
| NSColor | 8 bytes | Color factory methods (redColor, blueColor, etc.) | Color constants |
| NSFont | 8 bytes | systemFontOfSize, fontWithName | Font references |
| NSImage | 8 bytes | imageNamed, initWithSize | Image placeholders |
| NSAlert | 8 bytes | runModal (returns 0) | Dialog placeholder |
| NSAppearance | 8 bytes | currentAppearance, name | Theme detection |

Plus CoreAnimation: CALayer (48 bytes, 34 methods), CATransaction (8 class methods), CADisplayLink (24 bytes), CAMediaTimingFunction.

### Test and Compatibility Metrics

| Metric | Count | Description |
|--------|-------|-------------|
| `cf-lite` unit tests | 2,682 | Per-class functional tests |
| `objc-runtime` unit tests | 196 | Runtime mechanics: dispatch, lookup, protocol, dynamic class |
| `macho-exec` unit tests | 50+ | Mapping, fixups, TLV, unwind |
| `arm64-translate` unit tests | 417 + 11 ignored | Per-instruction decode/execute verification |
| Compatibility fixtures | 84/84 passing | End-to-end binary loading and execution |
| Corpus programs | 39/39 passing | Real-world Foundation/system usage patterns |
| Xcode tools (Docker/Linux) | 52 passing | Out of 72 tested; 19 crash, 1 fail |
| Xcode tools (macOS native) | 45/46 passing | Only notarytool crashes (deep Swift-ObjC bridging) |

**Coverage methodology**: A shim counts as "functional" when it passes its unit tests *and* the binaries that exercise it pass the compatibility gate. A stub that returns zero and prevents a crash doesn't count as coverage for the parity scoreboard — it counts as "structural" (crash prevention). The numbers above reflect functional implementations.

---

## Scene A.3: Where We Align

Some parts of the macOS platform are straightforward to replicate. Not easy — nothing in this project was easy — but *tractable*. Clearly specified, well-documented, stable across OS versions. When we align well with Apple's implementation, it's almost always for the same set of reasons.

### Mach-O Parsing: The Stable Format

Mach-O is one of the best-documented binary formats in existence. Apple publishes the header definitions in `<mach-o/loader.h>`. Every struct is `#[repr(C)]` with explicit field sizes. The magic number hasn't changed since the 64-bit transition. Load commands are tagged with a type field and a size — you can skip commands you don't understand and still parse the ones you do.

Our parser handles every load command that matters for loading (LC_SEGMENT_64, LC_DYLD_INFO_ONLY, LC_DYLD_CHAINED_FIXUPS, LC_SYMTAB, LC_DYSYMTAB, LC_LOAD_DYLIB, LC_MAIN, LC_UUID, LC_CODE_SIGNATURE, LC_BUILD_VERSION), and the format is stable enough that binaries from macOS 10.15 and macOS 15 parse identically.

The stability also extends to section naming conventions. `__objc_classlist`, `__objc_selrefs`, `__objc_methnames`, `__objc_protolist` — these section names haven't changed in a decade. The struct layouts for `class_t`, `method_t`, `protocol_t` are documented in Apple's open-source `objc4` repository. You can read the header file and write a parser that works on every macOS binary ever shipped.

**Why this aligns**: The format is a public ABI. Changing it would break every macOS binary in existence and every third-party tool that reads Mach-O files (`otool`, `nm`, `size`, `objdump`, all of LLVM's binary utilities). Apple has no incentive to change it and strong incentive to keep it stable.

### ObjC Runtime: The Open-Source Protocol

Apple publishes the source code for `objc4` — the Objective-C runtime — at their open-source repository. Not every release, and not always promptly, but enough that we can read the algorithm for `lookUpImpOrForward`, understand the method cache structure, and verify our dispatch behavior against the reference implementation.

The message-send protocol is straightforward: read the ISA from the object (offset 0), walk the class hierarchy calling `getMethodNoSuper_nolock` on each class's method list, fall back to `resolveMethod:` and `forwardingTargetForSelector:` if nothing is found. Our trampoline in `trampoline.s` implements this same walk, just with different data structures — our method lists are Rust `HashMap<u64, u64>` (selector hash → IMP address) instead of Apple's sorted C arrays.

Our selector interning works identically to Apple's: every unique selector string gets a unique pointer value, and pointer equality substitutes for string comparison during dispatch. This is a significant performance property — `objc_msgSend` never calls `strcmp`. We preserve this by interning every selector from `__objc_selrefs` into our global table during phase 5 (Register). Guest code that compares `SEL` values by pointer will get correct results.

**Why this aligns**: The ObjC runtime's behavior is its API. Objective-C's messaging semantics — method resolution order, forwarding, `doesNotRecognizeSelector:`, `nil` messaging returning zero, tagged pointer behavior — are relied upon by every Objective-C program ever written. The runtime can't change its external behavior without breaking the world.

### Foundation Data Types: The Stable Surface

`NSString`, `NSArray`, `NSDictionary`, `NSData` — these are the core value types. Their API surface has been stable since NeXTSTEP. Apple adds new convenience methods occasionally, but `[NSString length]`, `[NSArray count]`, `[NSDictionary objectForKey:]`, `[NSData bytes]` have had identical semantics since the 1990s.

The toll-free bridging contract between CoreFoundation and Foundation is equally stable: a `CFStringRef` and an `NSString *` are the same object, period. You can pass one to any API that expects the other. This has been true since OS X 10.0, and it constrains Apple's implementation as much as ours — they can't change the memory layout of `CFString` without breaking toll-free bridging.

Our implementations of these classes are functional, not just structural. `NSString` handles UTF-8 and UTF-16 encoding, comparison with locale-aware options (`NSCaseInsensitiveSearch`, `NSNumericSearch`), range-based operations, format string processing (including `%@` for object description, `%d`/`%f`/`%s` for C types, and positional specifiers). It passes 100+ unit tests that exercise the same operations Xcode tools call in practice.

**Why this aligns**: These are the atomic building blocks. Every macOS binary communicates through these types. Apple's entire ecosystem — frameworks, tools, applications, system services — passes `NSString` values between components. The behavior of `[NSString isEqualToString:]` is as close to a universal constant as software gets.

### File I/O: POSIX Is POSIX

`open`, `read`, `write`, `close`, `stat`, `fstat`, `opendir`, `readdir`, `mkdir`, `unlink`. POSIX file I/O is a contract that every Unix system honors. On macOS, these functions are in libSystem. On Linux, they're in libc. The semantics are identical. File descriptors are integers. Error codes are `errno` values. Return conventions are the same (-1 on error).

The only translation needed is struct layouts: `struct stat` is 144 bytes on macOS (with `st_size` at offset 96) and 128 bytes on Linux (with `st_size` at offset 48). `struct dirent` has `d_name` at offset 21 on macOS and offset 19 on Linux. `struct statfs` is 2,168 bytes on macOS (with 1024-character path buffers) and 120 bytes on Linux (no path fields at all). These translations are mechanical — read the fields from one layout, write them to the other.

**Why this aligns**: POSIX is a specification, not an implementation. The specification is public, versioned, and extremely stable. Apple's compliance is tested by POSIX conformance suites. Our shim layer translates struct layouts, but the behavioral contract is identical because both systems implement the same spec.

### String Handling: Well-Specified Encodings

UTF-8 and UTF-16 are international standards (RFC 3629 and RFC 2781). Converting between them is a pure algorithm with no platform dependencies. On macOS, `NSString` stores strings internally as one of several representations (ASCII, UTF-8, UTF-16, or tagged pointer for short strings). On our side, we store the backing buffer as a Rust `String` (guaranteed UTF-8) and convert to UTF-16 on demand for selectors like `characterAtIndex:` and `getCharacters:range:`.

The specification of `NSString`'s comparison behavior (`[NSString compare:options:range:locale:]`) is documented in Apple's developer documentation with enough precision that we can replicate it using `std::cmp` and ICU collation rules. Case-insensitive comparison uses Unicode case folding, not ASCII `tolower` — and the Unicode standard documents exactly what case folding means for every code point.

**Why this aligns**: String encoding is mathematics. There is no implementation freedom in UTF-8 → UTF-16 conversion. There is no platform-specific behavior in `[string length]`. The encoding standards are not Apple's to change.

### Process Information: Thin Wrappers

`getpid()`, `getenv()`, `sysctl()`, `gethostname()`, `uname()` — these are thin wrappers over kernel state. The macOS binary calls them; we forward to the host kernel's equivalent. The PID it gets back is a real PID. The environment variables are real. The hostname is real.

`NSProcessInfo` is a slightly thicker wrapper — it adds `processName`, `arguments`, `environment` (as an NSDictionary), `hostName`, `operatingSystemVersion`, `processorCount`. Each one maps to a system call or a cached value from the process startup state. We initialize `NSProcessInfo` from the host process state, and the returned values are correct — they describe the actual runtime environment.

**Why this aligns**: Process information is *factual*. The PID is what it is. The hostname is what it is. There is no interpretation layer, no abstraction, no behavioral contract to match — just raw data forwarding from the kernel.

---

## Scene A.4: Where We Diverge

Every compatibility project has an edge. A boundary where the implementation stops working and the honest answer is "no." Understanding where that boundary is — and *why* it's there — is as valuable as understanding the parts that work.

### Security.framework: The Trust Root Problem

Security.framework is Apple's security subsystem. It handles code signing verification, keychain access, certificate validation, TLS/SSL, and cryptographic operations. We shim `CC_SHA256` and `CC_MD5` (by delegating to the host's crypto library), and we shim `SecRandomCopyBytes` (by delegating to the host's CSPRNG). Everything else is a stub.

**Why we diverge**: Security.framework is fundamentally tied to macOS's trust model. Code signing verification depends on the Secure Enclave, Apple's root certificates, the system keychain, and the notarization service. Keychain access goes through `securityd`, a daemon with its own authorization model. TLS/SSL uses Apple's custom Secure Transport implementation (now mostly replaced by Network.framework), which negotiates with Apple's own root certificate store.

None of these exist on Linux. You can't verify an Apple code signature without Apple's root certificates and the specific chain of trust they encode. You can't access the macOS keychain from Linux because there is no macOS keychain. You could build a *parallel* security infrastructure — use OpenSSL for TLS, a SQLite database for keychain storage, your own certificate store — but it wouldn't be *Apple's* security infrastructure. Code that checks `SecTrustEvaluate()` and branches on the result would get different answers. Code that reads keychain items with `SecItemCopyMatching` would find an empty store.

**Impact on target use case**: Moderate. Most CLI developer tools don't exercise deep Security.framework paths. But tools that are *signing-aware* — `codesign`, `notarytool`, `stapler` — depend on it heavily. `notarytool` is our single remaining macOS crash (45/46 tools), and the root cause is deep in the Security/Swift bridging layer.

### Swift Runtime: The Moving Target

Swift's ABI stabilized in Swift 5.0 (2019), which means the calling convention, struct layout rules, and mangling scheme are fixed. But the *runtime implementation* — type metadata accessors, witness tables, protocol conformance caches, the generic specialization engine — continues to evolve. Every Swift release changes internal data structures.

Our loader handles Swift in limited ways. `swift-demangle` works fully (it's a C++ tool that happens to be in the Swift toolchain). Swift tools that only use Foundation through ObjC bridging work when all the bridged types are covered. But tools that exercise the Swift runtime itself — generic type instantiation, protocol witness table lookup, dynamic casting — hit unimplemented territory quickly.

The specific failure mode is instructive: `swift::ClangImporter::create` calls into the Clang driver, which constructs `SmallVector` and `InputArgList` objects whose internal metadata is interpreted differently under our loader. The addresses in argument arrays contain corrupt entries — small integers, tagged pointers, unmapped ranges. We work around the worst case by stubbing `getClangCC1Arguments` to return null (making ClangImporter fail gracefully), but this means Swift tools that need Clang integration print "clang importer creation failed" and exit 1 instead of working.

**Why we diverge**: Swift's internal data structures are not a public ABI. Apple changes them between Xcode releases. The type metadata format, the witness table layout, the generic context descriptor structure — these are implementation details that Swift binaries expect the runtime to handle, not documented contracts. Implementing a compatible Swift runtime would be a project of comparable scope to this entire book.

**Impact on target use case**: Significant and growing. As Apple's toolchain increasingly moves to Swift (SwiftUI, Swift Package Manager, the new build system), more developer tools exercise Swift runtime paths. The divergence will widen over time unless a parallel effort produces a compatible Swift runtime.

### Metal and GPU APIs: The Wrong Abstraction Level

Metal is Apple's GPU programming API. It provides shader compilation, command buffer management, render pipeline state, compute pipeline state, texture sampling, and resource management. It is an enormous API surface — hundreds of classes, thousands of methods — tied to specific GPU hardware through Apple's driver stack.

We don't implement Metal at all. Not even stubs.

**Why we diverge**: Metal is a hardware abstraction layer. It doesn't just describe an API — it describes a *contract with silicon*. A program that creates an `MTLDevice` expects to talk to a GPU. A program that compiles a shader expects a specific compiler backend (AIR — Apple Intermediate Representation). A program that submits a render pass expects pixel output. There is no meaningful way to stub this. You can return fake device objects, but the moment a program tries to create an actual render pipeline, it needs a shader compiler, a resource allocator, and a framebuffer — the entire graphics stack.

Vulkan could serve as an alternative backend (MoltenVK goes the other direction: Vulkan → Metal), but mapping Metal's API surface onto Vulkan would be a graphics-engineer-years project with no guarantee of correctness for Metal-specific features like tile shading, memoryless render targets, and ray tracing.

**Impact on target use case**: Negligible for CLI tools. `clang --version` doesn't need Metal. `xcodebuild` doesn't render pixels. `lipo`, `codesign`, `swift-demangle`, `otool` — none of them touch GPU APIs. Metal matters only if you want to run graphical macOS applications, which is not this project's target.

### Private Frameworks: The Undocumented Interior

Xcode ships dozens of private frameworks: DVTFoundation, IDEFoundation, IDEKit, DVTCocoaAdditionsKit, CoreSimulator, SourceKit, XCTest, DVTSourceControl. These frameworks are internal implementation details of Xcode's IDE. They are not documented. Their APIs change between Xcode versions. Their class names, method selectors, and behavioral contracts are discovered only through reverse engineering.

We encounter private frameworks when loading Xcode developer tools. `xcodebuild` links DVTFoundation and IDEFoundation. `momc` (the Core Data model compiler) links DVTFoundation, DVTCocoaAdditionsKit, and CoreSimulator. The dependency graph is deep: a single tool can pull in 80+ framework images.

Our approach is pragmatic: if a private framework symbol can be resolved from the macOS host (on macOS) or preloaded from a copied framework (on Linux), we resolve it. If it can't, it becomes a stub that returns zero. This works for tools that test for capability and degrade gracefully. It fails for tools that assume the capability exists and dereference the result.

**Why we diverge**: There is no specification. No documentation. No open-source reference. The only way to understand `DVTFoundation`'s behavior is to run it and observe, or to disassemble it. Every Xcode update can change the internal API. Building a compatible implementation would require reverse-engineering hundreds of classes with no guarantee the behavior is correct — and the effort would be invalidated by the next Xcode release.

**Impact on target use case**: High for Xcode IDE integration. Low for command-line developer tools. `xcodebuild` needs DVTFoundation to function, but `clang`, `lipo`, `swift-demangle`, `strings`, `nm`, `size`, `otool`, `strip`, `codesign` do not.

### IOKit: The Hardware Layer

IOKit is macOS's hardware abstraction framework. It provides device enumeration, driver matching, power management, USB/Bluetooth/Thunderbolt communication, and GPU resource management. Programs that call `IOServiceGetMatchingService` or `IORegistryEntryCreateCFProperties` are talking to the kernel's I/O registry, a tree of device drivers and their properties.

We stub IOKit entirely.

**Why we diverge**: IOKit is a kernel-to-userspace interface. The I/O registry is a kernel data structure populated by kernel drivers. On Linux, the equivalent information lives in `/sys/` and `/proc/` with completely different structure and naming conventions. Building a translation layer would require mapping Apple's driver model onto Linux's — not just device enumeration, but power management semantics, hotplug notification, USB endpoint management, everything.

**Impact on target use case**: Low. Developer tools rarely access hardware directly. The IOKit calls that do appear in tool binaries are typically for system identification (`IOPlatformExpertDevice` for the hardware model string) or platform detection — easy to stub with static values.

### Core Data: The ORM Continent

Core Data is Apple's object-graph and persistence framework. It provides an object-relational mapping built on SQLite, with managed object contexts, persistent store coordinators, fetch requests, sort descriptors, and a full change-tracking and undo system. It is a framework-sized project unto itself.

We don't implement Core Data. The `momc` (managed object model compiler) tool exercises Core Data heavily, and while we can parse and begin loading it, the deep dependency chain into Core Data's type system and model validation logic is beyond our scope.

**Why we diverge**: Core Data is not a thin API wrapper. It's an application framework with complex internal state — an object graph with change tracking, relationship management, validation rules, migration policies, and query optimization. Implementing it would require a complete SQLite-backed ORM, which is orthogonal to the binary-loading problem this project solves.

### The Boundary Is the Map

Every divergence listed above shares one of two properties: either the API is tied to platform-specific hardware or services that don't exist on other operating systems (Security, IOKit, Metal), or the API is an undocumented implementation detail that changes between releases (private frameworks, Swift runtime internals).

The convergent areas share the opposite properties: stable ABIs, public specifications, open-source reference implementations, or mathematical definitions that leave no room for platform-specific interpretation.

This boundary isn't a failure. It's the *information* the project produces. A loader that claimed 100% compatibility would be lying. A loader that stopped at Mach-O parsing would be trivial. The value is in the boundary itself — in knowing exactly which macOS APIs are portable abstractions over universal concepts (POSIX file I/O, Unicode strings, HTTP networking) and which are irreducible platform-specific artifacts (code signing trust chains, GPU shader compilation, kernel device trees).

For our target use case — running macOS developer tools on Linux — the boundary falls in a useful place. The tools we care about most (compilers, linkers, binary utilities, build system drivers) live almost entirely in the converging territory: POSIX I/O, Foundation data types, ObjC messaging, process management. The tools that fall outside the boundary (notarytool, Xcode IDE features, graphical instruments) are exactly the ones you'd expect to be platform-bound.

Divergence isn't failure — it's the boundary. Understanding the boundary is as valuable as extending it.

### Divergence Summary

| Domain | Coverage | Blocking Factor | Impact on CLI Tools |
|--------|----------|-----------------|---------------------|
| Security.framework | ~10% (crypto hashes only) | Hardware trust roots, keychain daemon, notarization | Moderate — blocks signing-aware tools |
| Swift runtime | ~5% (ABI layer only) | Internal metadata changes per release, witness tables, generic specialization | Significant — growing with Swift adoption |
| Metal | 0% | GPU hardware abstraction, shader compiler, driver stack | Negligible — CLI tools don't render |
| Private frameworks | ~15% (symbol preload) | No documentation, changes every Xcode release | High for IDE tools, low for CLI |
| IOKit | ~2% (stub) | Kernel I/O registry, driver model | Low — tools rarely access hardware |
| Core Data | 0% | Full ORM implementation needed | Low — only momc exercises it |
| Network.framework | ~5% (stub) | Modern transport API, QUIC, TLS 1.3 | Low — NSURLSession covers most HTTP |
| CloudKit | 0% | Apple server infrastructure | None for CLI |
| HealthKit/HomeKit/etc. | 0% | Apple ecosystem services | None for developer tools |

The 41-crate, 197,000-line loader covers the ground that matters for its use case. The gaps are real, documented, and — critically — well-understood. The map shows both the territory we've claimed and the frontier we haven't crossed, and that is exactly what a map should do.

---

*End of Appendix A*
