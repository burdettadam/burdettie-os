# World Building — Technical Landscape

**Phase:** 03 - World Building
**Purpose:** Define the technical world the book inhabits
**Status:** Outline

---

## The Two Worlds

### World 1: macOS on Apple Silicon

The binary's home. Defined by:

- **Mach-O binary format** — fat binaries, load commands, segments, __LINKEDIT
- **dyld** — Apple's dynamic linker. Shared caches. Chained fixups.
- **Objective-C runtime** — libobjc. objc_msgSend. ISA pointers. Tagged pointers.
- **Foundation/CoreFoundation** — NSString, NSDictionary, NSFileManager. Toll-free bridging.
- **libSystem** — Apple's libc + libdispatch + Mach APIs
- **ARM64 + PAC** — arm64e, pointer authentication, PACIA/PACIB/PACDA/PACDB
- **Code signing** — Required for execution. Page hashes. Entitlements.
- **Shared cache** — Prelinked dylibs at known addresses. ~2GB image.

### World 2: Linux on ARM64

The target environment. Defined by:

- **ELF binary format** — completely different from Mach-O
- **ld-linux** — GNU dynamic linker. GOT/PLT style.
- **glibc** — Standard C library, AAPCS64 calling convention
- **No Objective-C** — No runtime, no message dispatch, no Foundation
- **AAPCS64 ABI** — Variadic args in registers (not stack). Different struct stat layout.
- **No PAC** — Pointer authentication not used (or different implementation)
- **No shared cache** — Libraries loaded individually
- **Docker on Apple Silicon** — QEMU user-mode or native ARM64 host

### The Gap Between Worlds

| Feature | macOS | Linux | Gap |
|---------|-------|-------|-----|
| Binary format | Mach-O | ELF | Complete rewrite |
| Dynamic linker | dyld | ld-linux | Custom reimplementation |
| ObjC runtime | libobjc | (none) | Build from scratch |
| Foundation | Foundation.framework | (none) | Shim 130+ classes |
| Variadic ABI | Stack-only | Registers first | Assembly shims |
| Locale constants | LC_ALL=0 | LC_ALL=6 | Translation |
| stat struct | 144 bytes | 128 bytes | Layout conversion |
| PAC | PACIA/PACIB | (none) | Sign/strip stubs |
| Thread-local vars | __thread_vars section | __tls_get_addr | Custom TLV thunks |

---

## The Stack: macOS vs. burdettieOS — Layer-by-Layer Parity Map

This is the central reference diagram of the book. Each layer of Apple's
stack has a corresponding implementation in burdettieOS. The parity status
tells the reader where we're faithful, where we diverge, and where gaps remain.

```
     Apple macOS                           burdettieOS (darwin-loader-lab)
     ══════════                            ════════════════════════════════

  ┌─────────────────────────────┐       ┌─────────────────────────────┐
  │       Applications          │       │  apps/hello-darwin           │
  │  Xcode CLI tools, .app      │       │  apps/foundation-explorer    │
  │  bundles, Swift programs    │       │  apps/control-gallery        │
  │                             │       │  51+ Xcode tools (Docker)    │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │       AppKit / UIKit        │       │  app-kit-lite                │
  │  NSApplication, NSWindow,   │       │  ui-view, ui-controls (13)   │
  │  NSView, NSEvent, NSMenu,   │       │  cf-lite AppKit stubs (40)   │
  │  Auto Layout, responder     │       │  Cassowary constraint solver │
  │  chain, target/action       │       │  Full responder chain        │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │   Core Graphics / Metal     │       │  graphics-core               │
  │  CGContext, CGPath, fonts,  │       │  SoftwareRenderer (scanline) │
  │  GPU compositing            │       │  GpuRenderer (wgpu + WGSL)   │
  │                             │       │  text_layout (ab_glyph/TTF)  │
  │                             │       │  window-backend-{darwin,linux}│
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │       Foundation            │       │  cf-lite                     │
  │  ~2000 methods across       │       │  193 synthetic classes        │
  │  NSString, NSArray, NSDict, │       │  867 registered selectors    │
  │  NSData, NSURL, NSDate,     │       │  2,001 shim functions        │
  │  NSBundle, NSFileManager,   │       │  2,682 unit tests            │
  │  NSTask, NSURLSession,      │       │  Real HTTP (ureq), real I/O  │
  │  NSRegularExpression, ...   │       │  Real ICU4X date/number fmt  │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │     CoreFoundation          │       │  core-foundation-lite        │
  │  CFString, CFArray, CFDict, │       │  cf-lite/cf_core             │
  │  CFRunLoop, CFBundle,       │       │  runloop-core (full model)   │
  │  toll-free bridging to NS*  │       │  Toll-free: CF* == NS*       │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │     ObjC Runtime            │       │  objc-runtime                │
  │  libobjc.A.dylib            │       │  131 runtime shims           │
  │  objc_msgSend, class tables,│       │  ARM64 + x86_64 trampolines  │
  │  ARC, protocols, KVO/KVC,   │       │  Tagged pointers, ARC, KVO   │
  │  exceptions, swizzling      │       │  Full exception handling      │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │     libSystem.B.dylib       │       │  libsystem-shims             │
  │  libpthread, libc, libdyld, │       │  443 shim functions           │
  │  libsystem_kernel (Mach),   │       │  37 darwin_stubs modules      │
  │  libdispatch (GCD),         │       │  Real GCD (macOS) / shim (Lx)│
  │  CommonCrypto, os_log,      │       │  Real SHA/MD5/HMAC/AES       │
  │  Security.framework         │       │  Real X.509 (rustls-webpki)  │
  │  XPC, IOKit, Sandbox        │       │  ~70 XPC shims, 27 IOKit     │
  │  ICU (libicucore)           │       │  icu-shims (real ICU4X)      │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │      dyld                   │       │  macho-parse + macho-exec    │
  │  Mach-O loading, fixups,    │       │  5 chained fixup formats     │
  │  shared cache, lazy bind,   │       │  Shared cache parser         │
  │  DYLD_INSERT_LIBRARIES      │       │  Interpose support           │
  ├─────────────────────────────┤       ├─────────────────────────────┤
  │    XNU Kernel               │       │  trust-policy (8 gates)      │
  │  kern_exec, AMFI,           │       │  Real AMFI via csops(2)      │
  │  code signing               │       │  macho-runtime orchestration │
  │                             │       │                              │
  │    Rosetta 2                │       │  arm64-translate             │
  │  x86→ARM64 translation      │       │  164 insn variants, JIT      │
  └─────────────────────────────┘       └─────────────────────────────┘
```

### Feature Parity Scoreboard

| Layer | Apple Surface | burdettieOS Coverage | Parity |
|-------|--------------|---------------------|--------|
| dyld (Mach-O loading) | Internal | Full pipeline + 5 fixup formats + shared cache | ✅ ~100% |
| ObjC Runtime | ~120 symbols | 131 shims (msgSend, ARC, KVO, exceptions, introspection) | ✅ ~98% |
| libSystem | ~800 symbols | 443 shim fns + 37 stub modules | ✅ ~92% |
| CoreFoundation | ~300 functions | ~173 functions + CG shims | ✅ ~91% |
| XPC | ~70 symbols | ~70 real shims (object store + transport + pipe) | ✅ ~100% |
| ICU (i18n) | ~200 symbols | 50 shim fns (real via ICU4X) | ⚠️ ~48% |
| Foundation | ~2000 methods | 867 selectors, 2001 shim fns, 193 classes | ⚠️ ~43% selectors |
| Security | ~150 symbols | ~90 stubs + real SecItem + X.509 | ⚠️ ~65% |
| AppKit | ~5000 methods | 40 classes, 513 selectors, constraint solver | 🔨 ~10% |
| ARM64 interpreter | N/A | 164 instruction variants, JIT, 435 tests | ✅ Complete |
| **Total** | | **~198K LOC, 41 crates + 5 apps** | |

### Where We Align Strongly

These are the layers where burdettieOS is architecturally faithful to Apple:

1. **Same stack ordering.** Kernel → dyld → libSystem → CF → Foundation → AppKit. We mirror this exactly.
2. **Same fixup engine.** All 5 chained fixup pointer formats plus classic relocs. Real PAC signing on auth binds.
3. **Same dispatch model.** Assembly `objc_msgSend` trampoline with ISA → cache → method list lookup.
4. **Same bridging.** CFStringRef and NSString* point to the same object. All toll-free bridges work.
5. **Same load order.** BFS dependency walk, bottom-up constructor invocation, class-before-category `+load`.

### Where We Intentionally Diverge

| Apple Does | We Do Instead | Why |
|-----------|--------------|-----|
| Shared cache (2GB prelinked blob) | Individual dylib loading + caching | We don't have Apple's prelinker |
| Mach IPC (full port messaging) | Stub mach_msg returning error codes | No XNU kernel available |
| Persistent Keychain (encrypted on disk) | In-memory HashMap keychain | No Security.framework daemon |
| Real Metal/GPU compositing | wgpu abstraction layer | Cross-platform portability |
| Closed-source Swift runtime | swift-stub-gen (auto-generated shims) | No access to Swift internals |

---

## Apple's Undocumented Behaviors — Secrets the Loader Revealed

Building a compatibility layer against Apple's platform forces you to discover
behaviors that Apple never documents. These are the hidden rules, implicit
contracts, and surprising implementation details uncovered during the project.
Each one was found by hitting a crash and working backward to the cause.

### ABI Secrets

**1. Apple ARM64 variadic ABI is unique.**
Apple's ARM64 ABI puts ALL variadic arguments on the stack, always. This
contradicts the standard AAPCS64 calling convention, where the first 8
arguments go in registers. The difference is invisible on macOS (everything
uses the same convention) but catastrophic on Linux: a macOS binary calling
`printf("%s", str)` expects `str` on the stack at `[sp+8]`, while Linux's
`vfprintf` reads it from register `x1`. The fix requires naked assembly
shims that construct a Linux `va_list` struct pointing at the Apple-style
stack args.

*Crate: `libsystem-shims/src/cross_platform/variadic_shims.rs`*

**2. Locale category numbers differ between macOS and Linux.**
macOS defines `LC_ALL = 0` and `LC_CTYPE = 1`. Linux defines `LC_CTYPE = 0`
and `LC_ALL = 6`. A macOS binary calling `setlocale(0, "")` (meaning LC_ALL
on macOS) calls `setlocale(LC_CTYPE, "")` on Linux. Seven category values
must be translated between the two numbering schemes.

*Crate: `libsystem-shims/src/darwin_compat.rs`*

**3. struct stat is a different size and layout.**
macOS `struct stat` is 144 bytes. Linux `struct stat` is 128 bytes. Field
offsets differ for `st_size`, `st_mtime`, `st_mode`, and others. A macOS
binary reading stat results from a Linux kernel reads garbage past the end
of the buffer unless the layout is translated field by field.

*Crate: `libsystem-shims/src/cross_platform/stat.rs`*

### Binary Format Secrets

**4. Chained fixup `next` field is always bits 51–61.**
Apple's documentation implies auth and non-auth chained fixup entries use
different bit layouts for the `next` field. In practice, both use bits 51–61.
The auth entry documentation appears to describe bits 32–42, but this is
wrong—using those bits produces garbage offsets and corrupted pointer chains.

*Crate: `macho-exec/src/fixup/chained/walk.rs`*

**5. Format 12 (USERLAND) rebases are offsets, not absolute VMAddrs.**
Chained fixup format DYLD_CHAINED_PTR_ARM64E_USERLAND stores rebase targets
as offsets from the preferred base address, not absolute virtual addresses.
The distinction is critical when ASLR is applied—if you treat them as
absolute, every rebase points to the wrong address.

*Crate: `macho-exec/src/fixup/chained/walk.rs`*

**6. Auth fixup key/diversity bits are not where Apple's header says.**
In `DYLD_CHAINED_PTR_ARM64E` auth entries, the PAC key is at bits 49–50
(not 43–44 as header comments suggest), `addrDiv` is at bit 48 (not 45),
and diversity is at bits 32–47. Getting any of these wrong produces
invalid PAC signatures that trigger SIGBUS on auth-checking hardware.

*Crate: `macho-exec/src/ptrauth.rs`*

### Runtime Secrets

**7. Tagged pointers have architecture-specific tag positions.**
On ARM64, the tag is in the high bits (bit 63 marks a tagged pointer,
bits 60–62 encode the class index). On x86_64, the tag is in the low
bits. NSNumber uses tag 2, NSString tag 3, NSDate tag 6. The value
encoding differs per tag class—NSNumber packs the numeric value inline,
NSString packs up to 7 ASCII bytes.

*Crate: `objc-runtime/src/shims/dispatch.rs`*

**8. Non-pointer ISAs use a mask, not raw dereference.**
On ARM64, heap-allocated objects use "non-pointer ISAs" where bit 0 is
set and the actual class pointer is extracted via
`raw_isa & 0x0000000ffffffff8`. Code that directly dereferences `*(obj)`
as a class pointer will get garbage if it doesn't check bit 0 first
and apply the mask.

*Crate: `objc-runtime/src/shims/dispatch/host_forward.rs`*

**9. `__cxa_throw` must NOT be declared `-> !` in Rust.**
If the Rust extern "C" declaration of `__cxa_throw` uses `-> !`
(never-return), the compiler moves the function epilogue before the call.
This corrupts x29/x30, and the C++ unwinder cannot find the correct
return address. The fix is to declare it as returning `()` and let
the unwinder handle control flow.

*Crate: `objc-runtime/src/shims/lifecycle.rs`*

**10. `_OBJC_EHTYPE_$_NSException` must point to the class, not the host EHTYPE.**
The GOT slot for `_OBJC_EHTYPE_$_NSException` must contain the address
of the synthetic NSException class, not the host's EHTYPE struct. The
C++ personality routine uses this address during exception matching
(`shim_objc_exception_match`) to walk the ISA chain. If it points to
the host struct, ISA chain comparison fails and `@catch (NSException *)`
never catches.

*Crate: `cf-lite/src/foundation/cfconstant_string.rs`*

**11. `mach_task_self_` is a data symbol, not a function.**
macOS exports `_mach_task_self_` as a global `mach_port_t` variable
(a 4-byte integer). If the StubResolver registers a function pointer
address for it, guest code dereferences the function's instruction
bytes as a port number → garbage value → every `vm_*` call fails with
"invalid destination port."

*Crate: `libsystem-shims/src/mach.rs`*

**12. Darwin variant suffixes must be stripped before dlsym.**
Symbols like `_fstat$INODE64`, `_opendir$DARWIN_EXTSN`, and
`_strlcpy$UNIX2003` carry suffix annotations that encode OS version
variants. On Linux, `dlsym("fstat$INODE64")` returns NULL. The loader
must strip everything from `$` onward before resolving against the host.

*Crate: `macho-exec/src/fixup/resolver.rs`*

### Framework Secrets

**13. Foundation's `+load` must fire before any message dispatch.**
The Objective-C runtime guarantees that `+load` runs before any message
is sent to a class. But Foundation's initialization uses `+load` to set
up internal infrastructure (e.g., `NSThread`'s main thread, notification
center defaults). If synthetic Foundation classes don't fire `+load` in
the correct bottom-up order, later selectors silently fail.

*Crate: `macho-runtime/src/objc_setup.rs`*

**14. Binary plist is the default on modern macOS.**
All significant plist files (`Info.plist`, preferences, etc.) are binary
plists, not XML. A plist parser that only handles XML will fail on real
Xcode bundles—`xcodebuild -version` can't report its version without
reading the binary `Info.plist` from `Xcode.app/Contents/Info.plist`.

*Crate: `cf-lite/src/nspropertylistserialization.rs`*

**15. err/errx/warn/warnx use `program_invocation_short_name` on Linux.**
Linux's libc `err()` family reads `program_invocation_short_name`, which
resolves to `"macho-inspect"` (the loader), not the guest tool name.
macOS tools calling `err(1, "file not found")` display the wrong program
name. The fix is to reimplement them using `getprogname()` + `vfprintf`.

*Crate: `libsystem-shims/src/cross_platform/variadic_shims.rs`*

**16. FreeBSD's libxo structured output is pervasive in system tools.**
macOS system tools (wc, sort, df, etc.) are derived from FreeBSD and
use `libxo` for structured output (`xo_emit`, `xo_open_container`,
etc.). Linux has no libxo. Without 29 stub functions, these tools crash
before producing any output.

*Crate: `libsystem-shims/src/darwin_stubs/`*

**17. The shared cache means most dylibs don't exist as files.**
On modern macOS, `/usr/lib/libSystem.B.dylib` and most system frameworks
don't exist as individual files on disk. They're packed into
`dyld_shared_cache_arm64e` (~2GB). Tools that resolve paths to dylibs
must understand this—the file doesn't exist, but the code is available
via the shared cache extraction pipeline.

*Crate: `macho-shared-cache/`*

---

## The Technology Stack (Sensory Palette)

How does the reader "see" this world? Through:

- **Hex dumps** — Raw bytes with annotations. The archaeological view.
- **Register dumps** — PC, LR, x0-x28, SP. The crash scene.
- **Rust source** — Annotated code listings. The construction view.
- **Terminal output** — Tool execution results. The validation view.
- **Architecture diagrams** — Memory maps, dispatch chains, data flow.
- **Diff views** — Before/after of binary sections, struct layouts, test results.

---

## Rules of the World

1. **Every binary encodes its platform's assumptions.** You can read the platform from the binary.
2. **Crashes are information.** A SIGSEGV at a specific PC tells you exactly which assumption was violated.
3. **Compatibility is fidelity.** The shim doesn't need to be correct in general—it needs to be correct for the specific binaries we're loading.
4. **Layers depend on layers.** You can't shim Foundation without a runtime. You can't register classes without a parser. Order matters.
5. **The shared cache changes everything.** Most macOS libraries don't exist as individual files on modern systems.
