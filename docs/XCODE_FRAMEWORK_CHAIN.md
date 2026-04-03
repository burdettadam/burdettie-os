# Xcode IDE Framework Chain: Linux-First Architecture

## Design Principle

**Linux in Docker is the primary target.**  The loader must resolve every
symbol without access to macOS system frameworks.  macOS host dlsym is a
development-time shortcut, not an architectural dependency.

Every symbol that a guest Mach-O binary imports must be satisfiable by
one of these Rust-native resolution tiers:

1. **ShimTable** — Rust `extern "C"` functions registered at startup
2. **DataSymbolStore** — Rust-owned static data for constants, type
   tokens, IVAR offsets
3. **ObjcRuntime** — Synthesized ObjC classes with method tables
4. **CrossPlatformLibs** — Thin wrappers delegating to system libxml2,
   libz, ICU, libcurl (available on Linux via apt/apk)
5. **Bind trap** (returns 0) — last resort; logs warning

macOS `dlopen`/`dlsym` is **only** used as a validation oracle during
development: run the same tool on macOS with host resolution enabled,
compare outputs with the Rust-native path, and thereby verify that stubs
produce equivalent behavior.

---

## Problem Statement

Heavy Xcode tools (momc, xcindex-test, xcsigningtool, etc.) load deep
chains of private frameworks—**80+ images** from Xcode.app and
/Library/Developer when dependencies are transitively resolved.  These
frameworks import symbols that our resolution pipeline cannot satisfy.

**574 unique unresolved symbols** for a representative tool (momc),
producing 38 crashing tools out of 90 tested.

The previous architecture relied on `dlopen`/`dlsym` to preload macOS
system libraries and resolve symbols at runtime.  This made the loader
macOS-only.  This revision eliminates that dependency.

---

## Symbol Categorization (momc, 574 total)

### By type

| Category | Count | % |
|---|---|---|
| Swift mangled (`_$s`) | 196 | 34.1% |
| Constants (`_k*` data symbols) | 72 | 12.5% |
| OBJC_CLASS (private framework classes) | 16 | 2.8% |
| OBJC_IVAR (instance variable offsets) | 6 | 1.0% |
| Data symbols (`___`) | 6 | 1.0% |
| C/system function symbols | 278 | 48.4% |

### Swift modules

| Module | Symbols | Notes |
|---|---|---|
| SwiftUI | 132 | Protocol witnesses, conformances — not callable |
| RegexBuilder | 25 | Type metadata, init functions |
| Feedback | 18 | Xcode-private diagnostics |
| _StringProcessing | 4 | Swift stdlib supplement |
| Body/Coordinator/NSViewType | 5 | SwiftUI associated types |

### C symbols by framework origin (Linux resolution strategy)

| Framework | Symbols | Linux Strategy |
|---|---|---|
| dispatch (extras) | 22 | Rust shims in libsystem-shims (GCD subset) |
| XPC (type/error constants) | 16 | DataSymbolStore — static type tokens |
| ObjC runtime (extras) | 10 | Rust shims in objc-runtime crate |
| CoreText | 6 | Rust stubs (return sensible defaults) |
| DiskArbitration | 15 | Rust stubs (return NULL — DA is macOS-only) |
| libxml2 | 17 | CrossPlatformLibs — delegate to system libxml2 |
| libz (gz*) | 5 | CrossPlatformLibs — delegate to system libz |
| ICU (uregex/uregion) | 14 | CrossPlatformLibs — delegate to system libicu |
| NStatManager + key constants | 28 | Rust stubs + DataSymbolStore for constants |
| LaunchServices | 18 | Rust stubs (return not-found/error codes) |
| MobileGestalt | 9 | Rust stubs (synthetic device info) |
| container API | 9 | Rust stubs (no-op container management) |
| sysmon | 8 | Rust stubs (no-op monitoring) |
| Carbon HI* | 8 | Rust stubs (return 0 — UI APIs unused in CLI tools) |
| Mach/mig | 7 | Rust shims in libsystem-shims (Mach port subset) |
| asl | 6 | Rust stubs (log to stderr) |
| DataDetectors | 5 | Rust stubs (return no-match) |
| posix_spawn (extras) | 4 | CrossPlatformLibs — libc on Linux |
| rosetta | 3 | Rust stubs (translation unavailable) |
| ServiceManagement | 3 | Rust stubs (return not-authorized) |
| notify | 3 | Rust stubs (no-op notification) |
| sandbox | 2 | Rust stubs (no-op sandbox) |
| CoreAnimation | 2 | Rust stubs (return 0) |
| AudioToolbox | 1 | Rust stub (return 0) |
| UserNotifications | 2 | Rust stubs (return 0) |

---

## Resolution Architecture

### Tier 1: ShimTable Expansion (~310 symbols — C functions + ObjC runtime)

The ShimTable pattern (Rust `extern "C"` functions registered in a
`HashMap<&str, u64>`) is already the proven approach for Foundation,
AppKit, and libsystem.  This tier extends it to cover all 278 C function
symbols plus the 10 ObjC runtime extras.

Symbols fall into three sub-categories:

**1a. Cross-platform library delegates (~41 symbols, LOW EFFORT)**

These symbols have identical semantics on Linux.  The shim is a thin
`extern "C"` wrapper that calls the real Linux library function:

```rust
// crates/libsystem-shims/src/xml2.rs
extern "C" {
    fn xmlParseMemory(buffer: *const u8, size: i32) -> *mut c_void;
}

pub extern "C" fn shim_xmlParseMemory(buffer: *const u8, size: i32) -> *mut c_void {
    unsafe { xmlParseMemory(buffer, size) }
}
```

Libraries available on Linux via system packages:
- `libxml2-dev` — 17 symbols (xml*)
- `zlib1g-dev` — 5 symbols (gz*)
- `libicu-dev` — 14 symbols (uregex_*, uregion_*)
- `libc` — 4 symbols (posix_spawn extras, waitid)
- `libcurl4-openssl-dev` — already linked for URLSession

Cargo build dependencies use `pkg-config` to find these at build time.
Docker images install the `-dev` packages.

**1b. POSIX/libc functions with Linux equivalents (~55 symbols, LOW EFFORT)**

Functions like `mkostemp`, `openpty`, `waitid`, `flsl`, `timespec_get`,
`backtrace_async` exist in Linux libc (glibc/musl) or have trivial
polyfills:

```rust
// crates/libsystem-shims/src/posix_extras.rs
pub extern "C" fn shim_flsl(value: i64) -> i32 {
    if value == 0 { 0 } else { 64 - value.leading_zeros() as i32 }
}

pub extern "C" fn shim_mkostemp(template: *mut c_char, flags: c_int) -> c_int {
    unsafe { libc::mkostemp(template, flags) }
}
```

**1c. macOS-specific API stubs (~182 symbols, MEDIUM EFFORT)**

Functions from DiskArbitration, MobileGestalt, CoreText, LaunchServices,
etc. that have no Linux equivalent.  Each gets a Rust stub returning a
safe default:

```rust
// crates/libsystem-shims/src/darwin_stubs/disk_arbitration.rs
pub extern "C" fn shim_DADiskCopyDescription(_disk: u64) -> u64 { 0 } // NULL
pub extern "C" fn shim_DASessionCreate(_alloc: u64) -> u64 { 0 }      // NULL

// crates/libsystem-shims/src/darwin_stubs/mobile_gestalt.rs
pub extern "C" fn shim_MGCopyAnswer(question: u64) -> u64 {
    // Return a synthetic CFString for known questions, NULL otherwise
    0
}

// crates/libsystem-shims/src/darwin_stubs/asl.rs
pub extern "C" fn shim_asl_open(
    ident: *const c_char, facility: *const c_char, opts: u32
) -> *mut c_void {
    // Allocate a minimal handle; asl_send will write to stderr
    Box::into_raw(Box::new(AslHandle { opts })) as *mut c_void
}
```

**Organization:** New submodule tree under `crates/libsystem-shims/src/`:

```
libsystem-shims/src/
├── table.rs              (existing — ShimTable)
├── process.rs            (existing)
├── tls.rs                (existing)
├── lifecycle.rs          (existing)
├── cross_platform/       (NEW — thin wrappers over system libs)
│   ├── mod.rs
│   ├── xml2.rs
│   ├── zlib.rs
│   ├── icu.rs
│   └── posix_extras.rs
└── darwin_stubs/         (NEW — macOS-only API stubs)
    ├── mod.rs
    ├── disk_arbitration.rs
    ├── mobile_gestalt.rs
    ├── core_text.rs
    ├── launch_services.rs
    ├── asl.rs
    ├── mach_extras.rs
    ├── dispatch_extras.rs
    ├── notify.rs
    ├── sandbox.rs
    ├── rosetta.rs
    └── misc.rs           (Carbon, sysmon, DataDetectors, etc.)
```

Each module registers its shims via `install_*_shims(table: &mut ShimTable)`.

### Tier 2: DataSymbolStore (~100 symbols — constants + data exports)

Data symbols resolve to an *address*, not a function pointer.  The guest
reads data from that address.  Returning 0 (bind trap) causes immediate
NULL-dereference.  These need statically allocated Rust-side data.

```rust
// crates/macho-exec/src/fixup/data_symbols.rs

/// Provides stable non-NULL addresses for data symbols.
/// All allocations are 'static — they live for the process lifetime.
pub struct DataSymbolStore {
    symbols: HashMap<&'static str, u64>,
}

impl DataSymbolStore {
    pub fn new() -> Self {
        let mut store = Self { symbols: HashMap::new() };
        store.register_xpc_types();
        store.register_dispatch_source_types();
        store.register_string_constants();
        store.register_ivar_offsets();
        store.register_ndr_record();
        store
    }

    pub fn resolve(&self, name: &str) -> Option<u64> {
        self.symbols.get(name).copied()
    }
}
```

**Sub-categories:**

| Category | Count | Strategy |
|---|---|---|
| XPC type constants (`__xpc_type_*`) | 8 | 8-byte unique tokens (identity comparison only) |
| XPC error constants (`__xpc_error_*`) | 8 | Synthetic xpc_object_t pointers |
| dispatch source types (`__dispatch_source_type_*`) | 6 | 8-byte unique tokens |
| String constants (`_k*`) | 72 | `CFString`-compatible structs with the constant name as value |
| CF system version keys | 4 | Synthetic version strings ("15.0", "26A100") |
| `_NDR_record` | 1 | 16-byte struct: `{ 0, 1, 1, 0, 1, 0, 0, 0 }` (little-endian, 2's-complement) |
| `_sys_signame` | 1 | `&[*const c_char; 32]` — standard signal name table |
| OBJC_IVAR offsets | 6 | `u64` values — **only for verified offsets** |

All allocations use `Box::leak()` or `static` items to guarantee lifetime.

### Tier 3: ObjC Class Synthesis (~16 symbols)

The 16 unresolved `OBJC_CLASS_$_*` symbols reference classes from
private frameworks.  When these resolve to NULL, any
`[[ClassName alloc] init]` crashes.

Extend the existing class synthesis in `cf-lite::foundation::register`:

```rust
// crates/cf-lite/src/foundation/register/xcode_stubs.rs
pub fn register_xcode_stub_classes(rt: &mut ObjcRuntime) {
    let stub_classes = [
        "ACSURLSession",
        "AKAppleIDAuthenticationInAppContext",
        "CPProvisioningManager",
        "DVTDarwinupSession",
        // ... 12 more
    ];
    for name in stub_classes {
        register_stub_class(rt, name);
    }
}

fn register_stub_class(rt: &mut ObjcRuntime, name: &str) {
    // Creates a class with NSObject as superclass.
    // Inherits alloc/init/class/respondsToSelector: from NSObject.
    // Unknown selectors return 0 via unrecognized_selector_stub.
    let info = ClassInfo::new(name)
        .superclass("NSObject");
    rt.register_class(info);
}
```

The resolver checks `OBJC_CLASS_$_<name>` and `OBJC_METACLASS_$_<name>`
patterns and returns the synthetic class/metaclass address from the
ObjcRuntime before falling through to other tiers.

### Tier 4: Swift Metadata Stubs (~196 symbols, HIGH EFFORT)

Swift mangled symbols are the largest category (34.1%).  These are NOT
ordinary functions — they are:

- **Type metadata descriptors** (`_$s...MN`) — runtime type identity
- **Protocol witness tables** (`_$s...WP`) — vtable-like dispatch records
- **Protocol conformance descriptors** (`_$s...MC`) — conformance records
- **Type metadata access functions** (`_$s...Ma`) — metadata singletons

Simple return-0 stubs crash the Swift runtime because it reads specific
fields from these structures.

**Strategy: Generate minimal valid metadata structures**

Build a `darwin-stub-gen` tool that reads a macOS framework binary and
emits Rust source declaring static metadata with valid layouts:

```rust
// Generated by darwin-stub-gen from SwiftUI.framework
// crates/swift-stubs/src/swiftui.rs

/// Type metadata for SwiftUI.AnyView
#[repr(C)]
#[allow(non_upper_case_globals)]
pub static _s7SwiftUI7AnyViewVMn: SwiftTypeDescriptor = SwiftTypeDescriptor {
    flags: 0x0000_0050,           // struct kind
    parent: core::ptr::null(),     // top-level
    name: b"AnyView\0".as_ptr(),
    access_function: 0,
    field_descriptor: core::ptr::null(),
};
```

**Phased approach:**

1. **Analyze** — Use `swift-demangle` + export trie parsing to
   categorize all 196 symbols by their metadata kind
2. **Emit type descriptors** — Minimal `SwiftTypeDescriptor` structs
   (read-only, no vtable needed for non-called types)
3. **Emit witness tables** — For SwiftUI types that tools actually
   invoke (likely few — CLI tools rarely render UI)
4. **Register** — Each stub symbol registered in ShimTable at its
   static address

The generated stubs compile on both macOS and Linux — no host dependency.

### Tier 5: Bind Trap (Last Resort)

Any symbol not resolved by Tiers 1–4 gets the bind trap: a function at
a known address that returns 0.  The diagnostic system logs which symbols
hit this path, enabling prioritized stub development.

---

## Open-Source Precedent Analysis

### Darling (macOS-on-Linux)

**Strategy:** Rebuild Apple's open-source components from source; use
`darling-stub-gen` for private frameworks.

- `darling-stub-gen` reads a Mach-O binary, extracts ObjC class/method
  signatures via `class-dump`, generates C source with empty function
  bodies.
- *Limitation:* No constant symbols, no Swift metadata.
- *Our takeaway:* Validates the stub generation pattern.  We extend it
  to cover data symbols and Swift metadata.

### Wine (Windows-on-Linux)

**Strategy:** Reimplement DLLs as Unix shared libraries.  Uses `.spec`
files to declare exports and `winebuild` to generate PE import tables.

- Reimplements 1000+ DLLs with a "real where possible, stubs where not"
  strategy.
- *Our takeaway:* Proves that hybrid real/stub works at scale.  Their
  prioritization approach (implement what's called, stub what's not)
  maps directly to our per-tool symbol analysis.

### swift-corelibs-foundation (Apple official)

**Strategy:** Cross-platform Foundation in Swift + C.  Does not require
ObjC.  Uses libcurl for networking, libxml2 for XML, ICU for i18n.

- *Our takeaway:* Confirms that Foundation can work without macOS.  Our
  Rust Foundation (cf-lite: 93 classes, 1129 selectors) follows the same
  principle but in Rust.

---

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                        Guest Mach-O Binary                       │
│                     (e.g., momc, xcindex-test)                   │
└─────────────────────────────┬────────────────────────────────────┘
                              │ imports symbols
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│              Symbol Resolution Pipeline (Linux-native)            │
│                                                                  │
│  ┌────────────────┐  ┌─────────────────┐  ┌──────────────────┐  │
│  │   ShimTable     │  │  DataSymbolStore │  │  ObjcRuntime     │  │
│  │  Rust-native    │  │  Rust-native     │  │  Rust-native     │  │
│  │                 │  │                  │  │                  │  │
│  │  Foundation     │  │  XPC types       │  │  93 Foundation   │  │
│  │  AppKit         │  │  dispatch srcs   │  │  16 Xcode stubs  │  │
│  │  libsystem      │  │  k* constants    │  │                  │  │
│  │  cross-platform │  │  NDR_record      │  │                  │  │
│  │   (xml2,z,icu)  │  │  IVAR offsets    │  │                  │  │
│  │  darwin stubs   │  │                  │  │                  │  │
│  │   (DA,MG,CT,..) │  │                  │  │                  │  │
│  └───────┬─────────┘  └───────┬──────────┘  └───────┬──────────┘  │
│          │ miss                │ miss                 │ miss       │
│          ▼                    ▼                      ▼            │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                    Swift Metadata Stubs                      │ │
│  │  (generated from framework analysis, compiled into Rust)     │ │
│  │                                                              │ │
│  │  Type descriptors, protocol witness tables,                  │ │
│  │  conformance records — static data, no host dependency       │ │
│  └───────────────────────────┬──────────────────────────────────┘ │
│                              │ miss                               │
│                              ▼                                    │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │              macOS Shortcut (development only)               │ │
│  │  #[cfg(target_os = "macos")] + DARWIN_HOST_RESOLVE=1         │ │
│  │                                                              │ │
│  │  dlsym(RTLD_DEFAULT) with is_safe_symbol() gate             │ │
│  │  Used to validate Rust stubs match host behavior             │ │
│  └───────────────────────────┬──────────────────────────────────┘ │
│                              │ miss                               │
│                              ▼                                    │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                     Bind Trap (returns 0)                    │ │
│  │  Last resort — logs warning, returns 0                       │ │
│  └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## StubResolver Refactoring

The current `StubResolver` in `resolver.rs` is a thin wrapper around
`dlsym`.  It needs to become a multi-strategy resolver:

```rust
pub struct StubResolver {
    /// Rust-native symbol implementations (works on all platforms)
    shim_table: Arc<ShimTable>,
    data_store: Arc<DataSymbolStore>,
    objc_runtime: Arc<ObjcRuntime>,

    /// macOS-only: host dlsym for development validation
    #[cfg(target_os = "macos")]
    host: Option<Arc<dyn HostSymbolResolver>>,
}

impl SymbolResolver for StubResolver {
    fn resolve(&self, name: &str) -> Option<u64> {
        // 1. ShimTable (Rust functions)
        if let Some(addr) = self.shim_table.resolve(name) {
            return Some(addr);
        }

        // 2. DataSymbolStore (Rust-owned data)
        if let Some(addr) = self.data_store.resolve(name) {
            return Some(addr);
        }

        // 3. ObjcRuntime (synthesized classes)
        if let Some(addr) = self.resolve_objc_symbol(name) {
            return Some(addr);
        }

        // 4. macOS shortcut (opt-in, development only)
        #[cfg(target_os = "macos")]
        if std::env::var("DARWIN_HOST_RESOLVE").is_ok() {
            if let Some(host) = &self.host {
                if is_safe_symbol(name) {
                    if let Some(addr) = host.resolve_in(self.handle, name) {
                        return Some(addr);
                    }
                }
            }
        }

        // 5. Miss — caller installs bind trap
        None
    }
}
```

The `PRELOAD_LIBRARIES` list and `is_safe_symbol()` remain but are
guarded behind `#[cfg(target_os = "macos")]` and the
`DARWIN_HOST_RESOLVE` environment variable.  They become a validation
tool, not a production dependency.

---

## Implementation Priority

| Phase | Effort | Symbols | Coverage | Linux-Ready |
|---|---|---|---|---|
| 1. Cross-platform lib shims | 1 session | ~41 | 7.1% | Yes |
| 2. POSIX/libc shims | 1 session | ~55 | 9.6% | Yes |
| 3. DataSymbolStore | 1 session | ~100 | 17.4% | Yes |
| 4. ObjC class synthesis | 1 session | ~16 | 2.8% | Yes |
| 5. macOS-specific stubs | 2–3 sessions | ~182 | 31.7% | Yes |
| 6. Swift metadata stubs | 3–5 sessions | ~196 | 34.1% | Yes |

**Phases 1–4** resolve ~212/574 symbols (36.9%) and are all
Linux-native from day one.  Each phase is independently testable.

**Phase 5** is the bulk of the C function work but each framework's
stubs are independent and can be parallelized.

**Phase 6** is the most complex but also the most impactful single
category.  It requires the `darwin-stub-gen` tooling.

**Combined Phases 1–6** cover all 574 symbols with zero macOS host
dependency.

---

## Docker Build Integration

The Docker image needs system libraries for cross-platform delegates:

```dockerfile
# In Dockerfile.linux-e2e
RUN apt-get update && apt-get install -y \
    libxml2-dev \
    zlib1g-dev \
    libicu-dev \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*
```

Cargo.toml links these conditionally:

```toml
# crates/libsystem-shims/Cargo.toml
[target.'cfg(target_os = "linux")'.build-dependencies]
pkg-config = "0.3"

[target.'cfg(target_os = "linux")'.dependencies]
xml2-sys = { version = "0.1", optional = true }
```

Build script uses `pkg-config` to locate libraries at compile time,
ensuring the same binary works in Docker and on bare-metal Linux.

---

## macOS Development Workflow

When developing on macOS, the host dlsym path provides a way to validate
stubs:

1. **Run with host resolution:** `DARWIN_HOST_RESOLVE=1 cargo run -- momc ...`
   → all symbols resolve via dlsym, tool runs correctly
2. **Run without host resolution:** `cargo run -- momc ...`
   → only Rust-native stubs used, failures reveal missing stubs
3. **Diff outputs:** Compare stdout/stderr/exit-code between modes
   → verifies that stubs produce equivalent behavior
4. **Merge:** Once a stub passes diff validation, it's production-ready
   for Linux

This workflow replaces the previous model where macOS dlsym was the
primary resolution path.

---

## Stability Safeguards

1. **Baseline regression gate:** Every change must pass the existing
   tool baseline before expanding coverage.

2. **Incremental shim addition:** Add stubs in small batches, test each
   batch against specific tools.  Never bulk-add without validation.

3. **Cross-platform CI:** Every PR runs on both macOS and Linux.  Linux
   is the gate — macOS is advisory.

4. **Data symbol safety:** OBJC_IVAR stubs are dangerous — wrong offsets
   corrupt object memory.  Only stub IVARs with verified offsets.  All
   other data symbols are safe (constants are read-only).

5. **Generated stub versioning:** Swift metadata stubs are generated per
   Xcode version.  The generator records the source Xcode version.
   Version mismatches produce a warning, not an error.

6. **Host validation mode:** On macOS, `DARWIN_HOST_RESOLVE=1` enables
   side-by-side comparison against real frameworks to catch stub
   behavioral divergence.

---

## Security Considerations

- **No host function execution from stubs:** Stub functions return
  safe defaults without executing host code.  This prevents arbitrary
  code execution through crafted Mach-O binaries.

- **Symbol safe-list remains mandatory:** On macOS with host resolution
  enabled, every symbol must pass `is_safe_symbol()`.  We never resolve
  arbitrary symbols via dlsym.

- **Data symbol addresses are read-only:** Stub data symbols point at
  Rust-owned read-only allocations.  Guest code cannot corrupt host state.

- **Cross-platform library delegation is bounded:** Only specific
  well-known libraries (libxml2, libz, libicu, libcurl) are called
  through.  The shim layer validates arguments before forwarding.

- **No macOS framework loading on Linux:** The loader never attempts to
  dlopen macOS frameworks.  All resolution is through compiled-in Rust
  code.

---

## Files to Modify

| File | Changes |
|---|---|
| `crates/macho-exec/src/fixup/resolver.rs` | Refactor StubResolver to multi-strategy; guard dlsym behind cfg+env |
| `crates/libsystem-shims/src/cross_platform/` | New: xml2, zlib, icu, posix_extras shim modules |
| `crates/libsystem-shims/src/darwin_stubs/` | New: DA, MG, CT, LS, asl, mach, dispatch, etc. stub modules |
| `crates/macho-exec/src/fixup/data_symbols.rs` | New: DataSymbolStore for constants, type tokens, IVAR offsets |
| `crates/macho-exec/src/fixup/chained/mod.rs` | Integrate DataSymbolStore fallback before bind trap |
| `crates/cf-lite/src/foundation/register/` | New xcode_stubs.rs for ObjC class synthesis |
| `crates/cf-lite/src/foundation/mod.rs` | Register xcode stub classes |
| (new) `crates/swift-stubs/` | Generated Swift metadata stubs |
| (new) `tools/darwin-stub-gen/` | Stub generator tool |
| `docker/Dockerfile.linux-e2e` | Add libxml2-dev, zlib1g-dev, libicu-dev |
