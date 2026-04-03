# Apple Stack Comparison

How burdettieOS maps to Apple's real system stack, layer by layer.

```
 Apple macOS                              burdettieOS
 ─────────────────────────────────────    ─────────────────────────────────────
 ┌───────────────────────────────────┐    ┌───────────────────────────────────┐
 │         Applications              │    │  apps/hello-darwin                │
 │   Xcode CLI, Foundation tools     │    │  apps/foundation-explorer         │
 │                                   │    │  apps/control-gallery             │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │         AppKit / UIKit            │    │  app-kit-lite                     │
 │   NSApplication, NSWindow,        │    │    app, alert, event, menu,       │
 │   NSView, NSEvent, NSMenu,        │    │    responder, window_delegate     │
 │   NSResponder, Auto Layout        │    │  ui-view  (frame/bounds/subviews) │
 │                                   │    │  ui-controls  (7 built-in)        │
 │                                   │    │  cf-lite AppKit stubs (42 classes)│
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │  Core Graphics / Metal / Quartz   │    │  graphics-core                    │
 │   CGContext, CGPath, CTFont,      │    │    software renderer (scanline)   │
 │   GPU compositing, text layout    │    │    GPU renderer (wgpu + WGSL)     │
 │                                   │    │    text_layout (ab_glyph/TTF)     │
 │                                   │    │    BezierPath, Gradient, Shadow   │
 │                                   │    │  window-backend  (trait)          │
 │                                   │    │  window-backend-darwin  (macOS)   │
 │                                   │    │  window-backend-linux   (winit)   │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │         Foundation                │    │  cf-lite  (122 synthetic classes)  │
 │   NSString, NSArray, NSDictionary,│    │    nsarray, nsdictionary, nsdata  │
 │   NSData, NSURL, NSDate,          │    │    nsurl, nsdate, nsbundle        │
 │   NSBundle, NSFileManager,        │    │    nsfilemanager, nsprocessinfo   │
 │   NSProcessInfo, NSError,         │    │    nserror, nsthread, nsrunloop   │
 │   NSThread, NSRunLoop, NSTimer,   │    │    nstimer, nsnotificationcenter  │
 │   NSNotificationCenter, NSRegex,  │    │    nsregularexpression, nslocale  │
 │   NSLocale, NSCalendar,           │    │    nscalendar, nsdateformatter    │
 │   NSDateFormatter, NSCoder,       │    │    nskeyedarchiver, nspredicate   │
 │   NSKeyedArchiver, NSPredicate,   │    │    nsurlsession, nsxmlparser      │
 │   NSURLSession, NSXMLParser       │    │    + 58 more NS*/XC* modules      │
 │                                   │    │  runloop-core  (CFRunLoop model)  │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │      CoreFoundation               │    │  core-foundation-lite             │
 │   CFString, CFArray, CFDictionary,│    │    CGFloat, CGPoint, CGSize,      │
 │   CFRunLoop, CFBundle,            │    │    CGRect, NSRange, NSEdgeInsets  │
 │   CGPoint, CGRect, CGSize         │    │  cf-lite/cf_core  (CFRetain,      │
 │                                   │    │    CFRelease, CFEqual, CFHash)    │
 │                                   │    │  cf-lite/cfstring  (CFString)     │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │      ObjC Runtime                 │    │  objc-runtime                     │
 │   libobjc.A.dylib                 │    │    class/protocol registration    │
 │   objc_msgSend, class tables,     │    │    objc_msgSend trampoline        │
 │   protocol conformance,           │    │      (ARM64 + x86_64 assembly)   │
 │   method swizzling, ARC,          │    │    tagged pointer ISA             │
 │   associated objects,             │    │    reference counting + ARP       │
 │   exception handling              │    │    KVO, KVC, swizzling            │
 │                                   │    │    132 runtime shims              │
 │                                   │    │    backend.rs (libobjc2 ready)    │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │      libSystem.B.dylib            │    │  libsystem-shims  (985 shims)    │
 │   ┌─────────────────────────────┐ │    │    process, pthread, dispatch     │
 │   │ libpthread                  │ │    │    mach, lifecycle, tls, socket   │
 │   │ libsystem_kernel (mach)     │ │    │    ptrauth, darwin_compat         │
 │   │ libsystem_malloc            │ │    │    darwin_stubs/ (28 modules)     │
 │   │ libsystem_c (libc)          │ │    │      os_log, security, xpc,      │
 │   │ libdyld                     │ │    │      cfnetwork, common_crypto,    │
 │   │ libsystem_info              │ │    │      malloc_zone, sysinfo,        │
 │   │ libsystem_darwin            │ │    │      blocks_runtime, sys_config   │
 │   │ libsystem_blocks            │ │    │    cross_platform/ (3 modules)    │
 │   │ CommonCrypto                │ │    │      posix_extras, icu (stubs)    │
 │   └─────────────────────────────┘ │    │                                   │
 │                                   │    │  icu-shims  (ICU4X-backed)        │
 │   libicucore.A.dylib (ICU)       │    │    real collation, locale,         │
 │   ucol_*, uloc_*, ubrk_*         │    │    segmentation, datetime, decimal│
 │                                   │    │    formatting via icu4x           │
 │                                   │    │                                   │
 │   libdispatch (GCD)              │    │  libdispatch-sys                   │
 │   dispatch_async, dispatch_sync,  │    │    raw FFI to system libdispatch   │
 │   dispatch_queue, dispatch_group, │    │    60+ function bindings          │
 │   dispatch_semaphore, sources     │    │    pkg-config / Swift toolchain   │
 │                                   │    │                                   │
 │   CFNetwork.framework             │    │  darwin_stubs/cfnetwork           │
 │   CFHost, CFHTTPMessage,          │    │    ~45 stubs (CFHost, CFHTTP,     │
 │   CFNetService, CFProxy           │    │    Bonjour, proxy, sockets)       │
 │                                   │    │                                   │
 │   Security.framework              │    │  darwin_stubs/security            │
 │   SecItem*, SecCertificate,       │    │    ~90 stubs (keychain, certs,    │
 │   SecKey, SecTrust, SecPolicy     │    │    keys, trust, policy, random)   │
 │                                   │    │  trust-eval  (rustls-webpki)      │
 │                                   │    │    X.509 chain validation          │
 │                                   │    │    Mozilla root CA store           │
 │                                   │    │                                   │
 │   XPC                             │    │  darwin_stubs/xpc                 │
 │   xpc_connection, xpc_dictionary, │    │    ~70 real shims (object store,  │
 │   xpc_activity, xpc_pipe          │    │    transport, activity, pipe)     │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │   Platform Abstraction            │    │  platform-core  (10 PAL traits)   │
 │   (implicit in Apple's stack —    │    │    alloc, clock, fs, hash,        │
 │    macOS only)                    │    │    memory, process, rng,          │
 │                                   │    │    symbols, threading, unwind     │
 │                                   │    │  platform-darwin-host  (macOS)    │
 │                                   │    │  platform-linux        (Linux)    │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │         dyld                      │    │  macho-parse      (Mach-O parser)│
 │   /usr/lib/dyld                   │    │  macho-map        (segment mmap) │
 │   ProcessConfig, ImageLoader,     │    │  macho-resolve    (symbol res.)  │
 │   shared cache, fixup engine,     │    │  macho-exec       (fixup engine) │
 │   chained fixups, classic relocs  │    │    classic + chained (5 formats) │
 │                                   │    │  macho-run-plan   (load plan)    │
 │                                   │    │  macho-objc       (ObjC metadata)│
 │                                   │    │  macho-types      (Mach-O types) │
 │                                   │    │  macho-shared-cache (cache parse) │
 ├───────────────────────────────────┤    ├───────────────────────────────────┤
 │     Kernel (XNU)                  │    │  trust-policy  (8 security gates)│
 │   kern_exec, mach_loader, AMFI   │    │    real AMFI via csops(2)         │
 │                                   │    │  macho-runtime (orchestration)    │
 │   Rosetta 2  (x86→ARM64)         │    │  arm64-translate  (CPU emulator) │
 │                                   │    │    164 insn variants, JIT + cache │
 │                                   │    │    11.4K LOC, 427 tests           │
 └───────────────────────────────────┘    └───────────────────────────────────┘
```

## Layer-by-Layer Comparison

### Kernel & Loader

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| XNU `kern_exec.c` — validates Mach-O, sets up address space | `macho-runtime` — `Runtime::load_and_run()` orchestrates full pipeline | ✅ Functional equivalent |
| `mach_loader.c` — maps segments into VM | `macho-map` — mmap with segment/section awareness | ✅ Functional equivalent |
| AMFI (code signing, entitlements) | `trust-policy` — 8 configurable security gates, real AMFI via `csops(2)` | ✅ Real codesign verification on macOS |
| Rosetta 2 (x86→ARM64 translation) | `arm64-translate` — 164 instruction variants, basic block cache, JIT compiler (macOS MAP_JIT + W^X) | ✅ Interpreter + JIT |

### Dynamic Linker (dyld)

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| Mach-O parsing (load commands, segments) | `macho-types` + `macho-parse` | ✅ Complete |
| Symbol resolution (lazy/non-lazy, weak, re-exports) | `macho-resolve` | ✅ All binding types |
| Fixup engine (classic relocs, chained fixups) | `macho-exec` — 5 chained fixup formats + classic | ✅ Complete |
| ObjC metadata registration (classlist, catlist, selrefs) | `macho-objc` | ✅ Parse + register |
| Shared cache (dyld_shared_cache) | `macho-shared-cache` + `macho-runtime/shared_cache` | ✅ Parser + runtime integration (mmap) |
| `dyld_stub_binder` (lazy symbol resolution) | Shim in `libsystem-shims` | ✅ Stub |

### libSystem

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| libpthread (threads, mutexes, rwlocks, TLS) | `pthread` + `tls` modules in libsystem-shims | ✅ 40+ shims |
| libsystem_kernel (mach ports, vm, host info) | `mach` module in libsystem-shims | ✅ Real `vm_allocate`/`vm_deallocate` (mmap), `host_info` (CPU/mem), `host_page_size` |
| libsystem_c (libc wrappers) | `cross_platform/posix_extras` (18 shims) | ✅ Key functions |
| libdispatch (GCD) | `libdispatch-sys` (FFI) + `dispatch` module (6 shims) | ✅ Real GCD on macOS; shim fallback on Linux |
| GCD groups + semaphores | `darwin_stubs/dispatch_concurrency` (9 shims) | ✅ Real Mutex+Condvar concurrency |
| GCD dispatch_data + dispatch_source | `darwin_stubs/dispatch_data_source` (22 shims) | ✅ Data buffers + timer sources |
| os_log / os_signpost / os_activity | `darwin_stubs/os_log` (22 stubs) | ✅ Routes to `log` crate |
| libsystem_malloc (zone allocator) | `darwin_stubs/malloc_zone` (15 shims) | ✅ Real malloc delegation |
| libsystem_info (user/group lookup) | `darwin_stubs/sysinfo` (16 shims) | ✅ Real libc delegation |
| libsystem_blocks (_Block_copy/release) | `darwin_stubs/blocks_runtime` (6 shims) | ✅ Heap copy + refcount |
| CommonCrypto (SHA/MD5/HMAC) | `darwin_stubs/common_crypto` (8 shims) | ✅ Real FIPS 180-4 + RFC 1321 |
| SystemConfiguration (reachability) | `darwin_stubs/system_configuration` (21 stubs) | ✅ Always-reachable |
| Disk Arbitration, Launch Services, ASL | `darwin_stubs/` (11 modules, ~141 stubs) | ✅ Stub coverage |
| Sandbox (sandbox_init, extensions) | `darwin_stubs/sandbox` (11 shims) | ✅ Permits all, synthetic tokens |
| copyfile / removefile | `darwin_stubs/copyfile` (9 shims) | ✅ Real POSIX file copy + remove |
| dyld introspection (image enumeration) | `darwin_stubs/dyld_introspection` (7 shims) | ✅ Image registry + add-image callbacks |
| Compression framework (libcompression) | `darwin_stubs/compression` (7 shims) | ✅ Real ZLIB + LZ4 encode/decode |
| _keymgr (C++ EH key manager) | `darwin_stubs/keymgr` (6 shims) | ✅ Process-wide + per-thread stores |
| CoreText (font metrics, layout) | `darwin_stubs/core_text` (21 shims) | ✅ Font registry + proportional metrics |
| audit/BSM token introspection | `darwin_stubs/audit` (11 shims) | ✅ Real libc uid/gid/pid delegation |
| POSIX sockets (localhost policy) | `socket` module (12 shims) | ✅ Localhost/Unix passthrough, external denied |

### ObjC Runtime

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| `objc_msgSend` (ARM64/x86_64 fast path) | Assembly trampolines (7 entry points per arch) | ✅ Both architectures |
| Class/metaclass/ISA tables | `ObjcRuntime` with `HashMap` class table | ✅ Full class hierarchy |
| Protocol conformance | `ProtocolInfo` + `class_conforms_to_protocol` | ✅ Recursive check |
| Method swizzling (`method_exchangeImplementations`) | Supported via `class_replace_method` | ✅ |
| Reference counting (ARC) | `RefCountTable` + `AutoreleasePoolStack` | ✅ Per-thread ARP |
| Exception handling (`@try`/`@catch`) | C++ personality + DWARF unwind + legacy setjmp API | ✅ Full throw/catch/rethrow + legacy tryEnter/tryExit/extract/match |
| Runtime backend abstraction | `backend.rs` — `RuntimeBackend` trait | ✅ libobjc2 feature-gated |

### CoreFoundation

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| `CFString`, `CFStringRef` | `cfstring` module + C-API shims (`CFStringGetCString/CreateCopy/Compare/CreateWithCharacters/GetCharacterAtIndex/Find`) | ✅ Constant + dynamic strings |
| `CFRetain`/`CFRelease`/`CFEqual`/`CFHash` | `cf_core` module | ✅ |
| `CGPoint`, `CGSize`, `CGRect` | `core-foundation-lite` crate | ✅ Exact `#[repr(C)]` layout |
| `CFRunLoop` | `runloop-core` crate + C-API shims (`CFRunLoopGetCurrent/Main/Run/RunInMode/Stop/AddTimer/AddSource`) | ✅ Mode/timer model |
| `CFNumber` | C-API shims (`CFNumberCreate/GetValue/GetType/Compare/IsFloatType`) | ✅ All CFNumberType variants |
| `CFAllocator` | C-API shims (`CFAllocatorGetDefault/Allocate/Deallocate`) | ✅ |
| `CFDictionary`, `CFArray` | Toll-free bridged via NS* equivalents | ✅ |
| `CFBundle`, `CFPreferences` | `nsbundle` + `nsuserdefaults` | ✅ Via Foundation |

### Foundation

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| Core collections (NSArray, NSDictionary, NSSet) | ✅ + mutable variants + NSOrderedSet, NSCountedSet | ✅ 10 collection classes |
| NSString / NSMutableString / NSAttributedString | ✅ Full UTF-8/UTF-16 + attributed string | ✅ |
| NSData / NSMutableData | ✅ | ✅ |
| NSURL / NSURLComponents / NSURLRequest / NSURLSession | ✅ Including HTTP via `ureq` | ✅ |
| NSFileManager / NSFileHandle / NSPipe / NSStream | ✅ Real POSIX I/O | ✅ |
| NSDate / NSDateFormatter / NSCalendar / NSTimeZone | ✅ | ✅ Real ICU4X date formatting via `icu_datetime` |
| NSNumber / NSValue / NSNumberFormatter / NSDecimalNumber | ✅ | ✅ Real ICU4X number formatting; NSDecimalNumber arithmetic |
| NSLocale / NSCharacterSet / NSRegularExpression | ✅ | ✅ NSLocale has display name tables; NSRegularExpression uses `regex` crate |
| NSNotificationCenter / KVO / KVC | ✅ Full observer model | ✅ |
| NSBundle / NSProcessInfo / NSUserDefaults | ✅ | ✅ |
| NSThread / NSLock / NSOperation | ✅ Real POSIX threads | ✅ |
| NSTimer / NSRunLoop | ✅ Integrated with runloop-core | ✅ |
| NSPredicate / NSSortDescriptor | ✅ | ✅ AND, OR, NOT, comparison, LIKE, MATCHES, string ops |
| NSCoder / NSKeyedArchiver / NSKeyedUnarchiver | ✅ | ✅ Plist serialization via `plist` crate; `containsValueForKey:`, `encodeBytes:length:forKey:`, `decodeObjectOfClass:forKey:` |
| NSXMLParser / NSJSONSerialization | ✅ JSON via serde_json | ✅ JSON + XML via `quick-xml` |
| ICU integration (i18n backing) | `libicu` linked by Foundation | `icu-shims` (ICU4X real implementations) | ✅ Real collation, locale, segmentation, datetime, decimal, calendar |
| NSInvocation / NSMethodSignature | ✅ | ✅ `numberOfArguments`, `methodReturnLength`, `invokeWithTarget:` |
| NSURLCredential / NSHTTPCookie | ✅ | ✅ Credential storage + cookie jar singletons |

### AppKit

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| NSApplication lifecycle | `app-kit-lite/app.rs` + `cf-lite/nsapplication` | ✅ |
| NSWindow / NSPanel | `app-kit-lite` + `cf-lite` stubs | ✅ |
| NSView / NSViewController | `ui-view` crate + cf-lite stubs | ✅ Frame/bounds/subviews/firstResponder |
| NSResponder chain | `app-kit-lite/responder.rs` | ✅ Full event forwarding chain |
| NSEvent | `app-kit-lite/event.rs` + `cf-lite/nsevent` | ✅ |
| NSMenu / NSMenuItem | `app-kit-lite/menu/` + `cf-lite/nsmenu_shim` | ✅ Item list, actions, key equivalents |
| Controls (NSButton, NSTextField, NSSlider, ...) | `ui-controls` (13 controls) + cf-lite property shims (10 classes) | ✅ Full property read/write, target/action, enabled/state |
| NSColor / NSFont / NSImage | `cf-lite` shims | ✅ NSColor: RGBA+HSB, 19 system colors, `getRed:green:blue:alpha:` |
| NSAlert | `app-kit-lite/alert.rs` | ✅ |
| NSScreen / NSWorkspace | cf-lite shims | ✅ NSScreen: real display dims; NSWorkspace: `openURL:`/`openFile:` via xdg-open |
| sendAction:to:from: / targetForAction: | `cf-lite/nsapplication` | ✅ Guest responder-chain resolution |
| orderedWindows / keyWindow / mainWindow | `cf-lite/nsapplication` | ✅ Guest-managed z-order + focus state |
| currentEvent / pushPopEvent (nested dispatch) | `cf-lite/nsapplication` | ✅ Nested event context with push/restore |

### Rendering

| Apple | burdettieOS | Status |
|-------|-------------------|--------|
| Core Graphics (CGContext, CGPath) | `graphics-core` — `BezierPath`, `Gradient`, `Shadow` | ✅ |
| Metal / GPU compositing | `graphics-core/gpu/` — wgpu + WGSL shaders | ✅ Cross-platform GPU |
| Core Text / font rendering | `graphics-core/text_layout/` — ab_glyph + FontManager | ✅ TrueType |
| Quartz Compositor / WindowServer | `window-backend` (trait) + `window-backend-darwin` + `window-backend-linux` (winit) | ✅ Cross-platform (macOS native + Linux/X11/Wayland via winit) |

## Coverage Summary

| Layer | Apple Symbol Count (est.) | Covered | Coverage |
|-------|--------------------------|---------|----------|
| libSystem | ~800 | 985 shim fns, 36 darwin_stubs modules (285 tests) | ~92% |
| ObjC Runtime | ~120 | 132 shims (msgSend, ARC, KVO/KVC, swizzle, exceptions, introspection) | ~98% |
| CoreFoundation | ~300 | ~173 functions + CG shims (~100) | ~91% |
| Foundation | ~2000 methods | 122 synthetic classes, ~1958 shim fns (2487 tests) | ~78% |
| AppKit | ~5000 methods | 90 stub classes (responder chain, window/view depth, pasteboard, controls) | ~8% |
| ICU (i18n) | ~200 | 50 shim fns (real via icu4x: collation, locale, seg., datetime, decimal, calendar) | ~48% |
| CFNetwork | ~60 | ~45 stubs | ~75% |
| Security | ~150 | ~90 stubs + real X.509 chain validation | ~62% |
| XPC | ~70 | ~70 real shims (object store + transport + activity + pipe + Unix sockets) | ~100% |
| IOKit | ~200 | 27 stubs (most return kIOReturnSuccess/plausible values) | ~14% |
| ARM64 translator | n/a | 427 tests, 164 insn variants, JIT + block cache — MRS/MSR, LSE, LDAR/STLR, LDXR/STXR, FP/SIMD | ✅ |
| dyld (Mach-O loading) | N/A (internal) | Full pipeline + shared cache parser + 5 chained fixup formats | ✅ |
| Trust/Security gates | N/A (internal) | 8 gates + real AMFI via csops(2), 169 tests | ✅ |
| Graphics/Rendering | N/A | Software + GPU (wgpu) renderer, cross-platform window backends | ✅ |
| Fixtures (compat gate) | 39 corpus programs | 23 compat-runner test cases | ✅ |
| **Total** | | **41 crates, 5504 tests passing, 13 ignored** | |

## Resolved Gaps

1. **dyld shared cache** — ✅ `macho-shared-cache` crate parses `dyld_shared_cache_arm64e` / `x86_64` headers, extracts mapping info and image paths, and can extract individual dylib Mach-O data for further loading.

2. **ICU / i18n** — ✅ `icu-shims` crate provides real Unicode collation (`ucol_strcoll`), locale identification (`uloc_*`), word segmentation (`ubrk_*`), date formatting (`udat_format`) and parsing (`udat_parse`), and number formatting (`unum_format`) and parsing (`unum_parse`/`unum_parseDouble`/`unum_parseToNumber`) via ICU4X (pure-Rust, no system libicu dependency).

3. **Core Graphics path** — ✅ `cg-shims` crate provides ~80 CoreGraphics C API shims (CGContext, CGColorSpace, CGImage, CGPath, CGAffineTransform, CGDataProvider). Context creation, color, path building, and drawing calls are bridged to `graphics-core` (real `Surface`/`SoftwareRenderer`).

4. **Security / Keychain** — ✅ `darwin_stubs/security.rs` provides ~90 Security.framework stubs (SecItem CRUD, SecCertificate, SecKey, SecTrust, SecPolicy, Keychain, SecRandom). `trust-eval` crate provides real X.509 chain validation via `rustls-webpki` + Mozilla root store.

5. **Network stack** — ✅ `darwin_stubs/cfnetwork.rs` provides ~45 CFNetwork stubs (CFHost, CFHTTPMessage, CFNetService/Bonjour, CFProxy, CFSocketStream). `NSURLSession` still delegates to `ureq` for real HTTP.

6. **XPC** — ✅ `darwin_stubs/xpc.rs` provides ~70 XPC function stubs (xpc_connection, xpc_dictionary, xpc_array, xpc_string, xpc_bool, xpc_int64, xpc_data, xpc_endpoint).

7. **Shared cache → runtime integration** — ✅ `macho-runtime/shared_cache.rs` provides `SharedCacheLoader::open()` which memory-maps cache files via `libc::mmap` (zero-copy), parses image metadata via `macho-shared-cache`, and resolves dylib install names to Mach-O data slices.

8. **ICU date/number formatting** — ✅ `icu-shims/datetime.rs` provides real `udat_open`/`udat_format` backed by ICU4X `DateTimeFormatter` + `icu_datetime::fieldsets::YMDT`. `icu-shims/decimal.rs` provides real `unum_open`/`unum_formatInt64`/`unum_formatDouble` backed by ICU4X `DecimalFormatter`.

9. **NSXMLParser** — ✅ Upgraded from hand-rolled tokenizer to `quick-xml` crate for SAX parsing. Handles elements, CDATA, entity decoding, attribute parsing, comments, processing instructions.

10. **Window backend for Linux** — ✅ `window-backend-linux` crate implements `WindowBackend` trait using `winit` (event loop + window creation) and `softbuffer` (surface presentation). Works on Linux (X11/Wayland), macOS, and Windows.

11. **NSPredicate evaluation** — ✅ Enhanced parser supports `AND`, `OR`, `NOT` compound predicates, `LIKE`/`LIKE[c]` pattern matching, and `MATCHES` regex via `regex` crate.

12. **CG-shims → graphics-core bridge** — ✅ `cg-shims/bridge.rs` provides handle tables mapping CGContextRef/CGPathRef/CGColorRef to real `Surface`/`BezierPath`/`Color` objects. `CGBitmapContextCreate` allocates a pixel buffer, drawing calls (`FillRect`, `StrokeRect`, `FillEllipse`, `StrokePath`, `FillPath`) operate via `SoftwareRenderer`, path building builds real `BezierPath`, colors are stored with RGBA values.

13. **NSLocale display names** — ✅ `displayNameForKey:value:` returns human-readable names from built-in tables (32 languages, 38 countries, locale composites). Supports `NSLocaleIdentifier`, `NSLocaleLanguageCode`, `NSLocaleCountryCode` keys.

14. **X.509 cert validation** — ✅ `trust-eval` crate validates DER certificate chains using `rustls-webpki` against Mozilla's root CA store via `webpki-roots`.

15. **NSColor system colors + HSB** — ✅ Added 19 semantic/system colors (controlColor, windowBackgroundColor, labelColor, etc.), HSB ↔ RGB conversion (`colorWithHue:saturation:brightness:alpha:`, `hueComponent`, `saturationComponent`, `brightnessComponent`), `colorWithWhite:alpha:`, `getRed:green:blue:alpha:`, `colorUsingColorSpace:`.

16. **Mach VM/host stubs** — ✅ `vm_allocate` (real mmap), `vm_deallocate` (real munmap), `host_page_size` (real sysconf), `host_info` (real CPU count + memory), `mach_host_self`, `mach_port_allocate`/`deallocate`/`mod_refs`, `task_info`, `host_statistics` — 11 new Mach stubs.

17. **NSScreen real display dimensions** — ✅ Queries CoreGraphics on macOS (via dlsym) or reads `DISPLAY_WIDTH`/`DISPLAY_HEIGHT` env vars (default 1920×1080). Added `backingScaleFactor`, `deviceDescription`.

18. **NSWorkspace openURL/openFile** — ✅ `openURL:` and `openFile:` delegate to `open` (macOS) / `xdg-open` (Linux). Added `runningApplications`, `frontmostApplication`, `iconForFile:`.

19. **ICU date parsing** — ✅ `udat_parse` now parses ISO 8601 (`YYYY-MM-DDTHH:MM:SS`), simple dates (`YYYY-MM-DD`), ICU medium format (`Jan 15, 2024, 2:30:00 PM`), and numeric timestamps. Returns UDate (ms since epoch). Round-trips correctly with `udat_format`.

20. **ICU number parsing** — ✅ `unum_parseToNumber`, `unum_parse`, `unum_parseDouble` now parse numeric text with group separators (`12,345`), decimal (`3.14`), percent (`50%`→0.5), negative numbers (`-42`).

21. **ObjC exception legacy API** — ✅ Added `objc_exception_tryEnter`/`tryExit`/`extract`/`match` (setjmp-based `@try`/`@catch` path), `NSSetUncaughtExceptionHandler`/`NSGetUncaughtExceptionHandler`. Complements existing C++ personality path.

22. **CoreFoundation C-API expansion** — ✅ Added 23 new CF C-level shims: CFNumber C-API (5: Create/GetValue/GetType/Compare/IsFloatType), CFString expanded C-API (6: GetCString/CreateCopy/Compare/CreateWithCharacters/GetCharacterAtIndex/Find), CFRunLoop C-API (9: GetCurrent/GetMain/Run/RunInMode/Stop/AddTimer/AddSource + kCFRunLoopDefaultMode/kCFRunLoopCommonModes), CFAllocator (3: GetDefault/Allocate/Deallocate). Coverage now ~51%.

23. **libsystem-shims socket errno fix** — ✅ Fixed infinite recursion in macOS `errno_ptr()` (was calling itself instead of `__error()`). All 98 libsystem-shims tests now pass.

24. **NSCoder keyed coding expansion** — ✅ Added `containsValueForKey:`, `encodeBytes:length:forKey:`, `decodeBytesForKey:returnedLength:`, `decodeObjectOfClass:forKey:` to NSKeyedArchiver/Unarchiver registrations. NSCoder base class now has `containsValueForKey:` for polymorphic calls.

25. **os_log format string extraction** — ✅ `__os_log_impl` and `__os_log_error_impl` now read the format string argument (a C string pointer) and emit the actual format template via the `log` crate, surfacing real diagnostic text from guest binaries.

26. **ICU calendar API (ucal_*)** — ✅ New `calendar.rs` module in icu-shims with 8 functions: `ucal_open`, `ucal_close`, `ucal_get`, `ucal_set`, `ucal_getMillis`, `ucal_setMillis`, `ucal_add`, `ucal_getTimeZoneDisplayName`. Gregorian date decomposition with full field arithmetic (year/month/day/hour/min/sec/ms). 7 tests.

27. **NSDecimalNumber** — ✅ New Foundation class (53rd) with 20 methods: arithmetic (`add`, `subtract`, `multiply`, `divide`, `raisingToPower`), factory methods (`decimalNumberWithString:`, `decimalNumberWithMantissa:exponent:isNegative:`, `zero`, `one`, `notANumber`), comparison, string conversion. 11 tests. Backed by f64.

28. **ObjC runtime introspection** — ✅ Added `objc_getMetaClass` (returns metaclass for named class) and `object_getClassName` (returns class name for object). ObjC runtime now at 132 shims.

29. **notify(3) expansion** — ✅ Added `notify_post`, `notify_set_state`, `notify_register_check` to Darwin notification stubs (6 total, up from 3).

30. **XPC transport** — ✅ Replaced ~70 sentinel-returning XPC stubs with a real object store and connection transport. New `xpc_objects.rs` provides handle-based XPC value system (Dictionary, Array, String, Int64, Uint64, Bool, Data, Double, Date, UUID, Fd, Null, Error, Connection, Endpoint) with full dictionary set/get round-trips and array operations. New `xpc_transport.rs` provides connection state machine (Created→Resumed→Suspended→Cancelled), event handler registration, loopback message delivery, pending message queueing, and a service listener registry. All ~70 shim registrations now delegate to real implementations.

31. **XPC Activity scheduler** — ✅ New `xpc_activity.rs` module with real activity registration, state machine (CHECK_IN→RUN→DONE), criteria dictionary tracking, and initial check-in callback. `xpc_activity_register` fires the handler once on registration (matching Apple's launchd check-in behavior), `xpc_activity_set_criteria` updates the criteria, `xpc_activity_get_state` reads real state, `xpc_activity_unregister` removes activities.

32. **XPC Pipe routing** — ✅ New `xpc_pipe.rs` module with named pipe handler registry. `xpc_pipe_create` creates pipe objects with service name tracking. `xpc_pipe_routine` looks up registered handlers by pipe name and routes messages to them, returning replies. Default launchd stub handlers installed for `com.apple.xpc.launchd` and `com.apple.xpc.launchd.domain-port` (return error replies indicating no launchd).

33. **Cross-process XPC (Unix domain sockets)** — ✅ `xpc_transport.rs` enhanced with `socket_transport` module providing Unix domain socket IPC at `/tmp/darwin-xpc/<service>.sock`. Binary message serialization/deserialization for dictionaries (int64, uint64, bool, string, data, double, null). `send_message` and `send_message_with_reply` auto-detect socket listeners and route through them before falling back to loopback. `start_listener` spawns a background thread accepting connections. Path sanitization prevents traversal attacks. 147 tests total in libsystem-shims.

34. **XPC Activity periodic re-fire** — ✅ `xpc_activity_register` now reads `_interval_secs` from the criteria dictionary and spawns a background timer thread that re-fires the handler at the specified interval. Cancellation via `Arc<AtomicBool>` — re-registering the same activity name or calling `xpc_activity_unregister` stops the previous timer. Each re-fire transitions the activity back to RUN state and increments `run_count`.

35. **XPC nested object serialization** — ✅ Replaced flat wire format with recursive `serialize_value`/`deserialize_value` supporting nested dictionaries (tag `D`), arrays (tag `A`), and all XPC value types. Round-trips arbitrary nesting depth (dict→dict→array→dict verified). 150 tests total in libsystem-shims.

36. **libsystem_malloc zone APIs** — ✅ New `malloc_zone.rs` module with 15 shims: `malloc_default_zone`, `malloc_create_zone`, `malloc_destroy_zone`, `malloc_zone_malloc/calloc/realloc/free/memalign`, `malloc_size`, `malloc_good_size`, `malloc_zone_statistics`, `malloc_zone_batch_malloc/batch_free`, `malloc_set_zone_name`, `malloc_zone_from_ptr`. Single-zone model wrapping standard libc allocator. 7 tests.

37. **CommonCrypto** — ✅ New `common_crypto.rs` module with 8 shims: `CC_SHA256` (one-shot + streaming Init/Update/Final), `CC_SHA1`, `CC_MD5`, `CCRandomGenerateBytes`, `CCHmac` (HMAC-SHA256). All hash algorithms implemented in pure Rust per FIPS 180-4 (SHA-256/SHA-1) and RFC 1321 (MD5). Verified against standard test vectors. 9 tests.

38. **libsystem_info (user/group lookup)** — ✅ New `sysinfo.rs` module with 16 shims: `getpwnam/getpwuid/getpwnam_r/getpwuid_r/getpwent/setpwent/endpwent` (user), `getgrnam/getgrgid/getgrnam_r/getgrgid_r/getgrent/setgrent/endgrent` (group), `gethostname/sethostname`. All delegate to real libc functions. 6 tests.

39. **libsystem_blocks runtime** — ✅ New `blocks_runtime.rs` module with 6 shims: `_Block_copy` (null/global/heap with refcount), `_Block_release` (refcount decrement + free), `_Block_object_assign/dispose` (no-op stubs), `_NSConcreteStackBlock/GlobalBlock` data symbols. Follows Apple Blocks ABI layout. 6 tests.

40. **SystemConfiguration (SCNetworkReachability)** — ✅ New `system_configuration.rs` module with 21 shims: SCNetworkReachability (7: Create, GetFlags→always reachable, SetCallback, Schedule/Unschedule, SetDispatchQueue), SCDynamicStore (6: Create, CopyValue, CopyMultiple, SetValue, SetNotificationKeys, SetDispatchQueue), SCPreferences (5: Create, Get/SetValue, Commit/ApplyChanges), SCNetworkConfiguration (2: InterfaceCopyAll, ServiceCopyAll), kSCNetworkReachabilityFlagsReachable data symbol. 6 tests. 184 tests total in libsystem-shims.

41. **GCD dispatch_group + dispatch_semaphore** — ✅ New `dispatch_concurrency.rs` module with 9 shims: `dispatch_group_create/enter/leave/wait/notify/async` (real counter-based group coordination with Condvar wait + notify block execution), `dispatch_semaphore_create/wait/signal` (real counting semaphore with Condvar). Supports `DISPATCH_TIME_FOREVER` and timeout-based waits. 11 tests including 50-thread concurrency stress tests.

42. **GCD dispatch_data + dispatch_source** — ✅ New `dispatch_data_source.rs` module with 22 shims: `dispatch_data_create/get_size/create_map/apply/create_concat/create_subrange` (immutable scatter-gather buffers backed by `Arc<Vec<u8>>`), `dispatch_source_create/set_event_handler/set_cancel_handler/set_timer/cancel` (event monitoring with timer support), `dispatch_resume/suspend/release/retain/set_target_queue`, `_dispatch_data_empty` data symbol, 5 source type sentinel data symbols (timer/read/write/signal/proc). 10 tests.

43. **dyld introspection APIs** — ✅ New `dyld_introspection.rs` module with 7 shims: `_dyld_image_count/get_image_header/get_image_name/get_image_vmaddr_slide` (backed by global image registry), `_dyld_register_func_for_add_image` (fires callbacks on image registration), `dyld_image_path_containing_address`, `_NSGetExecutablePath`. Public `register_image()` API for the loader to populate. 10 tests.

44. **copyfile / removefile** — ✅ New `copyfile.rs` module with 9 shims: `copyfile` (real file copy with COPYFILE_DATA/STAT/EXCL flags via `std::fs`), `fcopyfile` (fd-to-fd copy via `libc::read/write` loop), `copyfile_state_alloc/free/get/set` (state tracking with bytes-copied counter), `removefile` (with REMOVEFILE_RECURSIVE via `std::fs::remove_dir_all`), `removefile_state_alloc/free`. 7 tests.

45. **Sandbox expansion** — ✅ Expanded `sandbox.rs` from 2 to 11 shims: added `sandbox_init/init_with_parameters` (accept but don't enforce profiles), `sandbox_free_error`, `sandbox_extension_issue_file/mach/generic` (return synthetic tokens), `sandbox_extension_release`, `sandbox_check_by_audit_token`, `sandbox_apply`. All report "not sandboxed" / success. 6 tests. 228 tests total in libsystem-shims.

46. **Compression framework (libcompression)** — ✅ New `compression.rs` module with 7 shims: `compression_encode_buffer`/`decode_buffer` (one-shot API), `compression_stream_init`/`process`/`destroy` (streaming API), `compression_encode/decode_scratch_buffer_size`. Real ZLIB encode/decode (stored blocks + Adler-32) and real LZ4 block format encode/decode implemented in pure Rust. 12 tests.

47. **_keymgr C++ EH key manager** — ✅ New `keymgr.rs` module with 6 shims: `_keymgr_get_and_lock_processwide_ptr`/`set_and_unlock_processwide_ptr`/`get_processwide_ptr` (process-wide `Mutex<HashMap<u32, u64>>`), `_keymgr_get_per_thread_data`/`set_per_thread_data` (per-thread `thread_local!` store), `_keymgr_global` data symbol. Supports KEYMGR_GCC3_DW2_OBJ_LIST (302) for C++ unwinder EH frame registration. 7 tests.

48. **CoreText expansion** — ✅ Expanded `core_text.rs` from 6 to 21 shims. New handle-based CTFont registry with proportional metrics (ascent=size*0.8, descent=size*0.2, etc.). Added `CTFontGetAscent/Descent/Leading/XHeight/CapHeight/UnitsPerEm/Size/BoundingBox` (8 metric shims), `CTFontDescriptorCreateWithAttributes/CreateMatchingFontDescriptor/CopyAttribute` (3), `CTParagraphStyleCreate/GetValueForSpecifier` (2), `CTRunGetGlyphCount/GetPositions` (2). 10 tests.

49. **audit/BSM token utilities** — ✅ New `audit.rs` module with 11 shims: `audit_token_to_pid/euid/egid/ruid/rgid/auid/asid/pidversion` (8 field extractors reading from audit_token_t struct, falling back to current process identity via libc), `audit_token_to_au32` (unpacks all 8 fields), `audit_session_self`, `audit_session_join`. 7 tests.

50. **Socket localhost passthrough** — ✅ Upgraded all 12 socket shims from blanket EOPNOTSUPP deny to policy-checked passthrough. `socket()` allows AF_UNIX/AF_INET/AF_INET6 creation. `connect()`/`bind()` allow AF_UNIX always, AF_INET to 127.0.0.1 only, AF_INET6 to ::1 only; deny all external addresses. `send`/`recv`/`listen`/`accept`/`getsockopt`/`setsockopt`/`shutdown` passthrough to libc. 10 tests including Unix socketpair send/recv roundtrip. 261 tests total in libsystem-shims.

51. **Mach thread/task APIs** — ✅ Added 12 shims to `mach_extras.rs`: `thread_get_state/set_state`, `thread_create/terminate`, `task_threads`, `thread_info`, `thread_suspend/resume/abort/abort_safely`, `task_policy_set/get`, `vm_protect`, `host_statistics64`. 8 unit tests. libsystem-shims total: 280 tests.

52. **IOKit stubs** — ✅ New `darwin_stubs/iokit.rs` with 25 IOKit stubs all returning `kIOReturnUnsupported = 0xe00002c7` or NULL. Covers: `IOServiceGetMatchingService/Services`, `IOServiceMatching/Open/Close`, `IOObjectRelease/GetClass`, `IOIteratorNext/Reset`, `IORegistryEntryCreate*`, `IOConnectCallMethod/StructMethod/ScalarMethod`, `IOPMAssertionCreateWithName/Release/SetProperty`, `IOHIDManagerCreate/Open/Close`, `IOHIDDeviceCreate/Open/Close`. 7 tests.

53. **posix_spawn full API** — ✅ Added 17 shims to `cross_platform/posix_extras.rs`: `posix_spawn`, `posix_spawnp` (core functions, thin wrappers to libc), `posix_spawn_file_actions_init/destroy/addclose/adddup2/addopen` (libc wrappers), `posix_spawn_file_actions_addchdir_np`/`addfchdir_np` (stub → ENOSYS), `posix_spawnattr_init/destroy/getsigmask/setsigmask/getsigdefault/setsigdefault/getpgroup/setpgroup` (libc wrappers). 7 tests.

54. **ARM64 MRS/MSR system registers** — ✅ Added `tpidr_el0`, `fpcr`, `fpsr` fields to `CpuState`. Decode: MRS/MSR distinguished from system instructions by bit[19]=1. Execute: TPIDR_EL0 (TLS pointer), CTR_EL0 (cache info), CNTVCT_EL0 (wall clock via `clock_gettime`), DAIF, NZCV, FPCR, FPSR. CLREX decoded and executed as no-op.

55. **ARM64 LDAR/STLR/Exclusive/LSE atomics** — ✅ Full acquire-release memory model: `LDAR/LDAPR/STLR` (64/32/byte/half), `LDXR/LDAXR/STXR/STLXR` (exclusive load/store), `LDADD/LDCLR/LDEOR/LDSET/LDSMAX/LDSMIN/LDUMAX/LDUMIN` (LSE atomic operations), `SWP` (atomic swap), `CAS/CASAL/CASA/CASL` (compare-and-swap). In the single-threaded interpreter all are modeled as non-atomic but semantically correct. ARM64 tests total: 422 passing.

56. **CF mutable collections, UUID, byteorder, propertylist** — ✅ New `cf_core/cf_extras.rs` with 50 CF C-API functions registered in the shim table: `CFArrayCreateMutable` + 6 mutation ops, `CFDictionaryCreateMutable` + 5 mutation ops, `CFSetCreate/CreateMutable` + 9 ops, `CFUUIDCreate/CreateString/CreateFromString/CreateFromUUIDBytes`, 15 `CFByteOrder/CFSwapInt*` functions, `CFPropertyListCreateWithData/CreateData/IsValid` (delegates to NSPropertyListSerialization).

57. **NSPasteboard** — ✅ New `nspasteboard.rs` with full clipboard API: `generalPasteboard`, `pasteboardWithName:`, `pasteboardWithUniqueName`, `declareTypes:owner:`, `clearContents`, `setString/Data/PropertyList:forType:`, `writeObjects:`, `stringForType:`, `dataForType:`, `propertyListForType:`, `readObjectsForClasses:options:`, `types`, `availableTypeFromArray:`, `changeCount`, `name`. 11 `NSPasteboardType` constants (`NSPasteboardTypeString`, `NSTiffPboardType`, etc.). 7 tests. cf-lite tests total: 2479.

58. **NSUnit/NSMeasurement** — ✅ New `nsmeasurement.rs` with complete NSUnit + NSDimension + NSMeasurement class hierarchy: `NSUnit` (symbol, initWithSymbol), `NSDimension` (baseUnit stub), `NSMeasurement` (doubleValue, unit, initWithDoubleValue:unit:, measurementByConvertingToUnit: stub, isEqualToMeasurement:, description). 9 concrete unit subclasses: `NSUnitLength` (10 factory methods: meters/kilometers/centimeters/inches/feet/yards/miles/nauticalMiles/lightYears), `NSUnitMass` (7), `NSUnitTemperature` (3), `NSUnitVolume` (7), `NSUnitAngle` (4), `NSUnitSpeed` (4), `NSUnitDuration` (7). All registered as synthetic ObjC classes + OBJC_CLASS data symbols. 5 tests.

59. **AppKit responder-chain depth** — ✅ `sendAction:to:from:` walks the guest responder chain (firstResponder → nextResponder / superview / window → delegate → app). `targetForAction:` and `targetForAction:to:from:` use the same walk. `currentEvent` stored on shared NSApp and updated during event dispatch. Nested event dispatch: `pushCurrentEvent`/`popCurrentEvent` restores prior currentEvent on unwind. `NSEvent.window` auto-derives from receiver when dispatch begins. 27 tests added across `nsapplication`, `nsevent`, `nsresponder`.

60. **NSWindow / NSView depth** — ✅ Guest-managed window focus state: `makeKeyAndOrderFront:`, `makeKeyWindow`, `makeMainWindow`, `orderOut:`, `isKeyWindow`, `isMainWindow`. Guest-managed first responder: `firstResponder`, `makeFirstResponder:`. View-tree safety: `addSubview:` rejects cycles, propagates window pointer recursively through descendants, detaches from previous superview. `setFrame:`, `setFrameSize:`, `setFrameOrigin:`, `removeFromSuperview`, `subviews`. `setContentView:` detaches stale focused views and rehomes first-responder. NSWindow at 80 bytes, NSView at 72 bytes.

61. **NSApplication window registry** — ✅ `NSApp.windows` returns all registered windows. `NSApp.orderedWindows` returns visible windows front-to-back by guest z-order. `NSApp.keyWindow` / `NSApp.mainWindow` track focus state derived from the guest `makeKeyAndOrderFront:`/`makeKeyWindow`/`orderOut:` sequence. `NSApp.currentEvent` clears on runtime reuse. NSApplication at 56 bytes.

62. **NSMenu / NSMenuItem item tracking** — ✅ `NSMenu` tracks a real guest-side item list (24 bytes). `addItem:`, `addItemWithTitle:action:keyEquivalent:`, `numberOfItems`, `itemAtIndex:`, `itemArray`, `removeAllItems`, non-nil `separatorItem`. NSMenuItem exposes `action`, key-equivalent accessors. 42 tests.

63. **Binary plist parsing** — ✅ `cf-lite::nspropertylistserialization::parse_plist_file()` now delegates to the `plist` crate for binary plist files in addition to the existing XML handler. Fixes `xcodebuild -version` reading `DVTSystemPrerequisites` from `/Applications/Xcode.app/Contents/Info.plist`.

64. **Host metaclass aliasing + ObjC exception path** — ✅ `objc_getClass()` via dlsym discovers all host ObjC classes and registers their metaclass ISAs as aliases. Exception handling now throws real host `NSException` on macOS via `objc_exception_throw`/`objc_begin_catch`/`objc_end_catch` while preserving the guest object in `CURRENT_EXCEPTION`. Guest `@catch` blocks still receive the guest exception object. _Compat score reached 84/84 (since partially regressed — see Remaining Gaps #1)._

65. **NSInvocation + NSMethodSignature** — ✅ `nsinvocation.rs`: `invocationWithMethodSignature:`, `setArgument:atIndex:`, `getReturnValue:`, `invokeWithTarget:`, `methodSignature`, `selector`, `target`. `nsmethodsignature.rs`: `signatureWithObjCTypes:`, `numberOfArguments`, `methodReturnLength`, `argumentsRetained`, `frameLength`. 10 tests.

66. **NSURLSession POST body + credential / cookie classes** — ✅ `NSURLRequest` layout expanded to 40 bytes; `HTTPBody`/`setHTTPBody:` shims. HTTP POST passes body bytes via `ureq` `.send(body)`. New classes: `NSURLCredential` (32 bytes), `NSURLCredentialStorage` singleton, `NSHTTPCookie` (24 bytes), `NSHTTPCookieStorage` singleton. All in `nscredential.rs`.

67. **Foundation startup optimization** — ✅ `ObjcRuntime` Foundation template cloned once per process via `OnceLock`; clones use `Arc<ClassInfo>` / `Arc<ProtocolInfo>` / `Arc<str>` selector keys, reducing 160-class overlay from ~178 µs to ~28 µs (−84%). Binary parse results + ShimTables cached per process. `overlay_from()` extends HashMap directly without temporary allocations. Compat fixtures unaffected. Startup overhead for typical tools: ~65 µs loader overhead, execution-dominated.

68. **Guest-only Foundation paths** — ✅ Removed all host ObjC runtime fallbacks: `nsprocessinfo`, `nsfilehandle`, `%@ tagged-string formatting`, `nsurl` CFURL introspection, `cf_core::CFPreferencesCopyValue`, `CFString` host-ISA upgrade. Stack is now fully self-contained (runs on Linux/Docker without macOS host). 45/46 Xcode tools pass; `notarytool` startup crash fixed by bridge interception (see Resolved Gap #81 / Remaining Gaps #2).

69. **Xcode tool compat expansion** — ✅ 45/46 Xcode dev tools pass: lipo, strip, stapler, size, strings, vtool, nm, swift-demangle, libtool, ld, xcodebuild (`Xcode 26.4 / Build version 17E192`), altool, pngcrush, SetFile, placeholderutil, xarsigner, TextureAtlas, ictool, and 27 more. Arm64e PAC pointer signing + USERLAND rebase fixed (3 bit-field bugs). GCD dispatch_group/semaphore/data/source real implementations. xpc cross-process Unix socket transport.

70. **NSRunLoop / NSTimer depth** — ✅ `performSelector:target:argument:order:modes:` dispatches through guest ObjC runtime. `acceptInputForMode:beforeDate:` drains due timers. `reset_runtime_state_for_reuse()` clears pending timers between in-process runs; NSApplication and NSRunLoop cleanup coordinated for safe multi-run perf benchmarking.

71. **NSUnit / NSDimension real linear converters** — ✅ `measurementByConvertingToUnit:` applies `base = value × coeff + const` / `converted = (base − const) / coeff`. All 7 unit dimensions carry per-unit coefficients: NSUnitLength (m/km/cm/mm/in/ft/yd/mi/NM/ly), NSUnitMass (kg/g/mg/lb/oz/t/tn), NSUnitTemperature (K/°C/°F with correct 273.15 K offset), NSUnitVolume (L/mL/m³/gal/fl oz/cup/pt), NSUnitAngle (°/rad/arcmin/arcsec), NSUnitSpeed (m/s/km⋅h⁻¹/mph/kn), NSUnitDuration (s/min/hr/ms/µs/ns/ps). New `unit_singleton_linear!` macro.

72. **NSMeasurementFormatter** — ✅ New 32-byte synthetic class. `stringFromMeasurement:` formats as `"{value} {symbol}"` with trailing-zero stripping. `alloc`/`init`/`new`, `unitStyle`/`setUnitStyle:`, `numberFormatter`/`setNumberFormatter:` accessors. Registered with `_OBJC_CLASS_$_NSMeasurementFormatter` data symbol.

73. **NSRegularExpression full options** — ✅ `build_regex_pattern()` helper maps all 5 NSRegularExpressionOptions bits: CaseInsensitive→`(?i)`, AllowCommentsAndWhitespace→`(?x)`, IgnoreMetacharacters→`regex::escape()`, DotMatchesLineSeparators→`(?s)`, AnchorsMatchLines→`(?m)`. Both `regularExpressionWithPattern:options:error:` and `initWithPattern:options:error:` use the shared helper.

74. **NSMeasurementFormatter + CommonCrypto AES + mach_msg + IOKit** — ✅ See items 4,5,6,8 in Remaining Gaps (resolved). CommonCrypto: CCCryptorCreate/Update/Final/Release/CCCrypt + PKCS7 + RSA stubs. mach_msg: MACH_SEND_INVALID_DEST stub. IOKit: open/close/release → kIOReturnSuccess.

75. **CoreData stub classes** — ✅ 6 new synthetic classes: `NSManagedObjectContext`, `NSPersistentStoreCoordinator`, `NSFetchRequest`, `NSManagedObject`, `NSEntityDescription`, `NSManagedObjectModel`. `save:` returns YES, `executeFetchRequest:error:` returns empty array, `addPersistentStoreWithType:…error:` returns nil+no-error. Apps that need CoreData to not crash can now instantiate these classes.

76. **AVFoundation stub classes** — ✅ 6 new synthetic classes: `AVAudioPlayer`, `AVAudioRecorder`, `AVAudioEngine`, `AVCaptureDevice`, `AVCaptureSession`, `AVPlayer`. AVCaptureDevice returns not-determined authorization and nil device for all media types. AVPlayer/AVAudioPlayer return playing=NO, duration=0. Apps using AVFoundation for capability checks no longer crash on unrecognized selectors.

77. **CoreLocation + CoreBluetooth stub classes** — ✅ 5 new synthetic classes: `CLLocationManager` (authorizationStatus=NotDetermined, locationServicesEnabled=NO, delegate/desiredAccuracy/distanceFilter, all start/stop methods no-op), `CLLocation` (initWithLatitude:longitude:, coordinate/altitude/accuracy accessors), `CLGeocoder` (geocodeAddressString: no-op), `CBCentralManager` (state=Unknown, scan/connect no-ops, +authorization=NotDetermined), `CBPeripheralManager` (state=Unknown, advertising no-ops). Prevents crashes in apps with CoreLocation/Bluetooth permission checks.

78. **Linux clipboard (arboard)** — ✅ `arboard` v3.6.1 added as optional dep in `crates/cf-lite` (feature `arboard`). `shim_nspasteboard_set_string_for_type` writes to the system clipboard via `arboard::Clipboard::set_text()`. `shim_nspasteboard_string_for_type` attempts arboard read-back first when the in-memory store is empty. `change_count` correctly incremented on every write. Headless/wayland-less environments fall back gracefully to in-memory singleton (arboard returns `Err` and the shim continues). Compat score unaffected: 84/84.

79. **ARM64 basic block cache** — ✅ `BlockCache` struct (`HashMap<u64, Arc<[Instruction]>>`) added to `arm64-translate/interp`. `Instruction` promoted to `Copy` (all 140 variants have only primitive/`Copy` fields). `BlockCache::build(start_pc, &mem)` is a static method — taking `&Memory` only — so it can be called while the HashMap is unborrowed. After the build, `Arc::clone` releases the map borrow before `&mut self.execute()` is called, keeping the implementation safe-Rust throughout. Blocks end at the first branch/SVC/Unknown instruction or after `MAX_BLOCK_SIZE=64` instructions. Re-entry after a taken conditional branch hits the same block on its second visit. Hit/miss counters are public for diagnostics. Two new unit tests verify: (a) a 100-iteration counter loop achieves >90 % hit rate; (b) a straight-line program produces zero hits. 84/84 compat score unaffected; 424 arm64-translate tests pass.

80. **Swift metadata stub generator** — ✅ New `crates/swift-stub-gen/` binary crate: a Rust-native offline code generator that reads any Mach-O framework binary and emits compilable shim modules for Swift-mangled symbols. `extractor.rs` reads arm64/arm64e slices from fat binaries via `macho-parse` export trie (`LC_DYLD_EXPORTS_TRIE`/`LC_DYLD_INFO`) plus nlist fallback. `demangler.rs` detects new-style (`_$s`/`_$S`) and old-style (`__TF`, `__TM`, `__TW`…) Swift name mangling without false-positives on `__TEXT`/`__THREAD`. `codegen.rs` emits a self-contained `.rs` module (single `swift_null_stub` fn + `SWIFT_NULL_DATA` static, wired into `ShimTable`). A data-symbol heuristic (`Ma`/`Mo`/`Mn`/`WP`/`WL`/`Wl` suffixes) separates type-metadata and protocol-witness stubs from function stubs. First applied: `swift_reflection.rs` (44 stubs for `_swift_reflection_*` from `libswiftRemoteMirror.dylib`) added to `darwin_stubs`. 19 unit tests. 84/84 compat score unaffected.

81. **Swift-ObjC bridge interception** — ✅ `crates/libsystem-shims/src/darwin_stubs/swift_foundation_bridge.rs` intercepts 13 Swift-Foundation bridge entry points before they reach host `libswiftFoundation`, preventing the `notarytool` startup crash. **Root cause**: Swift-mangled bridge symbols matched the `$s*` safe-list rule in `StubResolver::is_safe_symbol`, routing them to the host. Host `_unconditionallyBridgeFromObjectiveC` calls `objc_msgSend` on cf-lite synthetic object pointers; host `lookUpImpOrForward` cannot find the ISA (not registered in host class table) → SIGBUS at ISA+0x10. **Fix**: Symbols registered in `ShimTable` shadow the safe-list lookup in `CompositeResolver` (ShimTable is checked first). **ARM64 calling convention**: Swift caller allocates an indirect-return buffer and passes its address in **x8** (AAPCS64 `x8` = indirect result register). All 7 `_unconditionallyBridgeFromObjectiveC` shims call a `#[unsafe(naked)]` function (`write_zero_to_x8_ret_buf`) that reads x8 directly and zeros 32 bytes with `cbz x8, 2f` / `stp xzr, xzr, [x8]` / `stp xzr, xzr, [x8, #16]` — verified via `rustc --emit=asm` to produce no compiler-generated prologue before the asm. The 6 `_forceBridgeFromObjectiveC` shims zero the inout result pointer passed in x1 (`write_bytes(*x1, 0, 32)`). Covered types: `Dictionary←NSDictionary`, `Array←NSArray`, `String/Substring←NSString`, `Set←NSSet`, `Data←NSData`, `IndexSet←NSIndexSet`. 4 unit tests (null-safety, write verification, 10 symbol registration checks, fn-ptr non-null). 84/84 compat score unaffected.

82. **Xcode framework chain — partial resolution** — ✅ Four-part attack on the 574 unresolved symbols for the momc/Xcode IDE tool chain: **(a) DataSymbolStore wired into CompositeResolver as Tier 2** — previously only tested, never consulted in production. Now resolves ~100+ constant data symbols on Linux too (before they silently fall through to dlsym). **(b) 100+ new symbols in DataSymbolStore**: `___kCFBooleanTrue/False` (CFBoolean sentinels), `___CFConstantStringClassReference` (ISA for string literals), `___NSConcreteGlobalBlock/StackBlock/MallocBlock/AutoBlock` (Block runtime ISAs), `_kCFRunLoopDefaultMode/CommonModes` + 5 NSRunLoop mode strings, 15 `_kDADiskDescription*` DiskArbitration keys, 12 `_kLS*` LaunchServices keys, 19 `_NS*Key/Domain/Notification` NSError/Foundation constants (`_NSLocalizedDescriptionKey`, `_NSFilePathErrorKey`, `_NSURLErrorKey`, KVO change keys, notification names, `_DVTEnvironmentSnapshotBool`), 14 `_kNStat*` NStatManager keys. **(c) 7 more system framework preloads** added to `StubResolver::PRELOAD_LIBRARIES`: CoreText, DiskArbitration, QuartzCore, AudioToolbox, UserNotifications, libxml2, ServiceManagement → ~221 more C function symbols available via dlsym on macOS. **(d) SwiftUI + Swift StringProcessing preloads**: `SwiftUI.framework/SwiftUI` and `libswift_RegexParser.dylib` / `libswift_StringProcessing.dylib` dlopened at startup — the 132 SwiftUI protocol-witness symbols used by momc resolve to real host implementations via dlsym. 5 new unit tests in `data_symbols.rs`. 84/84 compat score unaffected.

## Remaining Gaps

The following gaps are identified in priority order. "Remaining Limitations" has been replaced with actionable items.

### P0 — Active Regressions

1. **`minimal_throw` + `try_catch_test`** — ✅ Fixed. Three-part fix: (a) `__cxa_throw` must not be declared `-> !` or Rust moves the epilogue before the call, corrupting x29/x30; (b) `shim_objc_exception_throw/rethrow` must use `extern "C-unwind"` to allow foreign exceptions to propagate; (c) `_OBJC_EHTYPE_$_NSException` GOT slot must point to the synthetic class address (not the host EHTYPE struct) so `shim_objc_exception_match` can walk the ISA chain. Compat score restored to 84/84.

### P1 — High Value

2. **notarytool crash (Swift-ObjC bridging)** — ⚠️ Startup crash intercepted: 13 Swift-Foundation bridge entry points now registered in `ShimTable` (see Resolved Gap #81), shadowing the `$s*` safe-list rule that previously routed them to the host. The `_unconditionallyBridgeFromObjectiveC` × 7 shims zero the x8 indirect-return buffer; `_forceBridgeFromObjectiveC` × 6 shims write `Optional.none` to the inout result pointer. `notarytool` can now proceed past the bridge phase. Remaining risk: if `notarytool` downstream logic dereferences data it expected from a bridged NSDictionary/NSArray (business logic, not startup), it will see empty collections and may silently fail or take a different error path rather than crashing.

3. **Xcode framework chain — 574 unresolved symbols** — ⚠️ Representative tool `momc` loads 80 Xcode private frameworks (IDEFoundation, DVTFoundation, CoreSimulator, etc.) with 574 unique unresolved symbols: ~196 Swift-mangled (34%, mostly SwiftUI/132), ~278 C functions (48%), ~72 constants (13%), ~22 ObjC class/ivar. Partial resolution applied this session — see Resolved Gap #82.

4. **CommonCrypto AES/RSA** — ✅ Implemented. `common_crypto.rs` now covers full AES-128/192/256 CBC and ECB modes via `CCCryptorCreate`/`CCCryptorUpdate`/`CCCryptorFinal`/`CCCryptorRelease`/`CCCrypt` (one-shot). PKCS7 padding in `CCCryptorFinal`. RSA/EC stubs return `kCCUnimplemented` (-4305). 19 shims total.

5. **mach_msg shimmed** — ✅ Added `_mach_msg` and `_mach_msg_trap` stubs returning `MACH_SEND_INVALID_DEST` (0x10000003). Tools that make synchronous Mach IPC calls no longer fall through to unmapped symbols on Linux.

### P2 — Medium Value

6. **IOKit coverage** — ✅ Expanded. `IOServiceOpen`, `IOServiceClose`, `IOObjectRelease`, `IOPMAssertionCreateWithName`, `IOPMAssertionRelease`, `IOPMAssertionSetProperty`, `IOHIDManagerOpen/Close`, `IOHIDDeviceOpen/Close`, `IOServiceGetMatchingServices` all return `kIOReturnSuccess` with plausible out-values. `IOPMCopyBatteryInfo` returns `kIOReturnUnsupported`. 27 stubs total.

7. **NSUnitConverter — real math** — ✅ Implemented. `measurementByConvertingToUnit:` applies a linear converter formula (`base = value × coeff + const`, `result = (base − const) / coeff`). All 9 concrete unit classes have per-unit coefficients: NSUnitLength (m/km/cm/mm/in/ft/yd/mi/NM/ly), NSUnitMass (kg/g/mg/lb/oz/t/tn), NSUnitTemperature (K/°C/°F with correct offsets), NSUnitVolume (L/mL/m³/gal/fl oz/cup/pt), NSUnitAngle (°/rad/arcmin/arcsec), NSUnitSpeed (m/s/km⋅h⁻¹/mph/kn), NSUnitDuration (s/min/hr/ms/µs/ns/ps).

8. **NSMeasurementFormatter** — ✅ Implemented. `crates/cf-lite/src/nsmeasurement.rs` + registration in `measurement.rs`. 32-byte layout with `unitStyle` and `numberFormatter` fields. `stringFromMeasurement:` formats as `"{value} {symbol}"` (trailing-zero stripped). `alloc`/`init`/`new`, `unitStyle`/`setUnitStyle:`, `numberFormatter`/`setNumberFormatter:` accessors.

15. **NSRegularExpression extended options** — ✅ Implemented. `build_regex_pattern()` helper now maps all NSRegularExpressionOptions bits correctly: bit 0 → `(?i)`, bit 1 → `(?x)` (AllowCommentsAndWhitespace), bit 2 → `regex::escape()` (IgnoreMetacharacters), bit 3 → `(?s)` (DotMatchesLineSeparators), bit 4 → `(?m)` (AnchorsMatchLines).

9. **Real Linux clipboard** — ✅ `arboard` v3.6.1 integrated as an optional dependency (`features = ["arboard"]`) in `crates/cf-lite`. `shim_nspasteboard_set_string_for_type` now writes to the system clipboard via `arboard::Clipboard::set_text()` when the feature is enabled. `shim_nspasteboard_string_for_type` tries arboard read-back first when the in-memory store is empty, falling back to the singleton. `change_count` is incremented on every write. Graceful fallback on headless systems (arboard returns `Err`, shim falls back to in-memory). Compat score unaffected: 84/84.

10. **ARM64 interpreter performance** — ✅ Basic block cache added to `arm64-translate`. `BlockCache` (`HashMap<u64, Arc<[Instruction]>>`) maps each block-start PC to a pre-decoded instruction slice. `Instruction` is now `Copy` (all fields are primitives), so cached slices iterate with zero-copy `*insn` dereference. Block terminators: B/Bl/Br/Blr/Ret/Bcond/Cbz/Cbnz/Tbz/Tbnz/Svc/Unknown (Mrs/Msr are sequential). `MAX_BLOCK_SIZE=64` prevents pathological straight-line sequences from bloating the cache. `Arc::clone` releases the HashMap borrow before each `execute()` call without unsafe code. On hot loops (~100 iterations) cache hit rate ≈ 97 %. 84/84 compat score unaffected; 424 arm64-translate tests pass.

### P3 — Completeness / Future Work

11. **CoreData** — ✅ Minimal stub layer. `NSManagedObjectContext` (`initWithConcurrencyType:`, `performBlock:`, `save:` → YES, `executeFetchRequest:error:` → empty array), `NSPersistentStoreCoordinator` (`initWithManagedObjectModel:`, `addPersistentStoreWithType:…error:` → nil/success), `NSFetchRequest` (full `predicate`/`sortDescriptors`/`fetchLimit` accessors), `NSManagedObject` (`initWithEntity:insertIntoManagedObjectContext:`, `valueForKey:`/`setValue:forKey:`), `NSEntityDescription` (`entityForName:inManagedObjectContext:` → nil), `NSManagedObjectModel` (`mergedModelFromBundles:` → nil). All 6 classes registered as synthetic ObjC classes with `_OBJC_CLASS_$_` data symbols. Apps that use CoreData but degrade gracefully on empty results no longer crash on unrecognized selectors.

12. **AVFoundation / Media** — ✅ Stub classes. `AVAudioPlayer` (full init/play/pause/stop/duration/currentTime/volume/numberOfLoops), `AVAudioRecorder` (init/record/stop/isRecording), `AVAudioEngine` (init/startAndReturnError:/stop/isRunning/nodes), `AVCaptureDevice` (+defaultDeviceWithMediaType:→nil, +devicesWithMediaType:→empty, +authorizationStatusForMediaType:→NotDetermined), `AVCaptureSession` (full addInput/Output/startRunning/stopRunning), `AVPlayer` (playerWithURL:/playerWithPlayerItem:, play/pause/rate/status/volume). All 6 classes registered with `_OBJC_CLASS_$_` data symbols.

13. **CoreLocation / CoreBluetooth** — ✅ Stub classes. `CLLocationManager` (+authorizationStatus→NotDetermined, +locationServicesEnabled→NO, delegate/desiredAccuracy/distanceFilter accessors, start/stop updating location/heading, requestWhenInUseAuthorization/requestAlwaysAuthorization), `CLLocation` (initWithLatitude:longitude:, coordinate/altitude/horizontalAccuracy/speed/course), `CLGeocoder` (geocodeAddressString:completionHandler: no-op). `CBCentralManager` (initWithDelegate:queue:options:, state→Unknown, scanForPeripherals/stopScan/connect/cancel, +authorization→NotDetermined), `CBPeripheralManager` (state→Unknown, startAdvertising/stopAdvertising/addService). All 5 classes registered with `_OBJC_CLASS_$_` data symbols.

14. **Swift metadata stubs** — ✅ New `crates/swift-stub-gen/` binary crate implements a Rust-native stub generator for Swift-mangled symbols in any Mach-O framework binary. Reads arm64/arm64e slices from fat binaries via `macho-parse` export trie (`LC_DYLD_EXPORTS_TRIE` / `LC_DYLD_INFO`) + nlist fallback. `demangler.rs` detects new-style (`_$s`/`_$S`) and old-style (`__TF`, `__TM`, `__TW`, …) Swift name mangling, classifies symbols into categories (function, type-metadata, protocol-witness, initialiser, property-accessor, runtime-helper, thunk), and optionally invokes `swift-demangle` for human-readable comments. `codegen.rs` emits a self-contained `.rs` shim module with a single `swift_null_stub` (returns 0) shared by all function stubs and a `SWIFT_NULL_DATA` static for data symbols, both integrated into the `ShimTable` via `table.register()`. `extractor.rs` uses a data-segment heuristic (suffix `Ma`/`Mo`/`Mn`, patterns `WP`/`Wl`/`WL`) to distinguish code from data exports. Generated output is ready to drop into `crates/libsystem-shims/src/darwin_stubs/`. First generated module: `swift_reflection.rs` (44 function stubs for `_swift_reflection_*` from `libswiftRemoteMirror.dylib`). 19 unit tests. 84/84 compat score unaffected; 443 arm64-translate tests pass.

15. **NSRegularExpression extended options** — ✅ Fixed (see P2 item 15 above).

## Architecture Alignment Analysis

How burdettieOS's architecture maps to Apple's real system and where it diverges.

### Where We Align Well

| Apple Architecture Pattern | burdettieOS Equivalent | Alignment |
|---------------------------|----------------------|-----------|
| Layered stack (kernel → dyld → libSystem → CF → Foundation → AppKit) | Same layering: macho-runtime → macho-* → libsystem-shims → core-foundation-lite → cf-lite → app-kit-lite | ✅ Strong |
| Mach-O as the universal binary format | macho-parse/macho-types handle all Mach-O formats including fat binaries | ✅ Strong |
| dyld as a lazy symbol resolver with chained fixups | macho-resolve + macho-exec with 5 chained fixup formats + classic relocs | ✅ Strong |
| ObjC runtime as message dispatch + class registration layer | objc-runtime with assembly trampolines, class tables, ARC, KVO/KVC | ✅ Strong |
| CoreFoundation toll-free bridging with Foundation | cf-lite handles both CF and NS APIs with shared backing stores | ✅ Strong |
| Platform Abstraction Layer (implicit in XNU) | platform-core with 10 PAL traits + platform-darwin-host + platform-linux | ✅ Strong |
| Security gating (AMFI, code signing, entitlements) | trust-policy with 8 gates + real AMFI via csops(2) + X.509 via rustls-webpki | ✅ Strong |
| GCD (libdispatch) as the concurrency primitive | libdispatch-sys (real FFI on macOS) + real group/semaphore/data/source shims | ✅ Strong |

### Where We Diverge

| Apple Architecture | burdettieOS Current State | Gap | Priority |
|-------------------|--------------------------|-----|----------|
| **JIT compilation** (Rosetta 2 produces native code) | Interpreter with basic block cache + nascent JIT (MAP_JIT, W^X); most execution still interpreted | JIT code generation for hot paths | P1 |
| **Shared cache preloading** (dyld prelinks all system libraries into one contiguous mapping) | Cache parsed and mmap'd, but each dylib still resolved on-demand | Eager shared cache resolution for startup perf | P2 |
| **NSObject** is a real class hierarchy with ivar layout | Synthetic 8-byte ISA-only objects; no real ivar offset tables | Proper ivar layout descriptors for binary-compatible objects | P2 |
| **Blocks ABI** (stack → heap promotion with copy/dispose helpers) | Copy/release implemented, but copy/dispose helpers not invoked | Invoke block copy/dispose helper function pointers | P2 |
| **RunLoop source/observer model** (CFRunLoopSource, CFRunLoopObserver, input sources) | Timer-only runloop; no input source or observer callbacks | Full CFRunLoopSource + CFRunLoopObserver | P1 |
| **Auto Layout** (constraint solver in AppKit) | ui-view has LayoutConstraint types but no Cassowary solver | Integrate a constraint solver (e.g., `cassowary-rs`) | P3 |
| **Keychain Services** (persistent encrypted storage) | All SecItem* calls return stub responses | Real keychain-like storage (SQLite + keyring) | P3 |
| **Thread-safe ARC** (atomic reference counting) | Non-atomic refcount (single-threaded interpreter assumption) | Atomic refcount when multi-threaded execution is supported | P2 |
| **dyld interposing** (DYLD_INSERT_LIBRARIES) | Not implemented | Library interposing for debugging hooks | P3 |
| **Mach IPC** (port-based messaging) | mach_msg returns MACH_SEND_INVALID_DEST | Functional mach_msg for XPC bootstrap | P2 |

### Future Parity Roadmap

#### Phase 1: Execution Fidelity (P1)

1. **JIT hot path compilation** — Extend `arm64-translate/jit/` to emit native code for hot basic blocks. The block cache already identifies hot paths (>N executions). Emit native ARM64 on macOS (MAP_JIT is already set up), x86_64 on Linux. Target: 10-50x speedup for computation-heavy loops.

2. **RunLoop input sources** — Implement `CFRunLoopSource` (version 0 + version 1) and `CFRunLoopObserver`. This is required for any GUI event loop and for `NSURLSession` async callbacks. The `runloop-core` crate already has the mode/timer model; extend with source/observer dispatch.

3. **Multi-threaded execution** — The ARM64 interpreter is single-threaded. To run real concurrent ObjC programs, support multiple interpreter threads sharing the same address space with proper memory ordering. `LDAR/STLR` and LSE atomics are already decoded; they need real atomic semantics.

#### Phase 2: Binary Compatibility (P2)

4. **NSObject ivar layout** — Replace the 8-byte ISA-only stub objects with proper ivar-layout-aware allocation. Read `__objc_ivar` section from Mach-O to determine field offsets. This enables binary-compatible access to Foundation object fields.

5. **Shared cache eager loading** — Pre-resolve the top ~20 system dylibs from the shared cache at startup (libSystem, libobjc, CoreFoundation, Foundation). Eliminates per-symbol cache lookup overhead during execution.

6. **Mach IPC bootstrap** — Implement enough of `mach_msg` to support XPC bootstrap port lookups. This enables tools that use `xpc_connection_create_mach_service()` to connect to launchd services.

7. **Atomic ARC** — Switch `RefCountTable` from `HashMap<u64, usize>` to `HashMap<u64, AtomicUsize>` (or use DashMap) to support concurrent retain/release from multiple interpreter threads.

#### Phase 3: Ecosystem Depth (P3)

8. **AppKit beyond stubs** — Current AppKit coverage is ~8%. The responder chain, window/view hierarchy, and event model are functional, but Auto Layout, drawing, and most controls are stubs. Prioritize Auto Layout (Cassowary solver) and NSTableView/NSOutlineView for data-driven apps.

9. **Swift interop maturity** — `swift-stub-gen` can generate null stubs for any Swift framework. Next step: type metadata stubs that return plausible `TypeMetadata` structs so Swift generic specializations don't crash. Long-term: Swift calling convention support in the ARM64 interpreter (self/error registers, witness tables).

10. **CoreData with SQLite** — Replace the no-op stub with a real `NSManagedObjectContext` backed by SQLite (via `rusqlite`). Implement `NSFetchRequest` execution, `NSManagedObject` fault/fire, and schema migration. This unlocks a large class of data-driven macOS/iOS apps.

11. **Keychain + credential storage** — Replace Security.framework stubs with a real encrypted key-value store. Use the system keyring on macOS (`security` CLI or Security.framework FFI) and Linux (`libsecret` or file-based with `ring` encryption).

12. **Network stack depth** — `NSURLSession` uses `ureq` for synchronous HTTP. Add async HTTP/2 support (via `hyper` or `reqwest`) with delegate callbacks, authentication challenges, and download/upload tasks.
