# Architecture Plan — darwin-loader-lab

> A Rust-based Mach-O loader laboratory aligned with Apple's dyld architecture,
> POSIX dynamic linking standards, and Darwin security model.

---

## 1. Project purpose

Build a clean-room Mach-O loader implementation that:

- Faithfully reproduces Apple `dyld`'s loading behavior
- Can be verified against real macOS behavior at every stage
- Has well-defined trait boundaries for every security and trust gate
- Supports drop-in interoperability with Apple's equivalent tools and APIs
- Follows the textbook loader lifecycle from System V ABI and Apple's published specs

This is a **loader laboratory**, not a replacement for macOS. We build understanding
first, compatibility second, and production capability only when both are proven.

---

## 2. Reference standards

| Standard | Relevance |
|---|---|
| Apple Mach-O File Format Reference | Binary format: headers, load commands, segments |
| Apple dyld source (opensource.apple.com) | Loader behavior: startup, resolution, fixups |
| System V ABI (generic + processor supplements) | Baseline dynamic linking model (ELF comparison) |
| POSIX.1-2017 §2.12 (Dynamic Linking) | `dlopen`/`dlsym`/`dlclose` API contract |
| Apple Code Signing Guide | Code signature structure, CDHash, entitlements |
| Apple Hardened Runtime documentation | Library validation, DYLD_* restrictions |
| XNU source (`kern_exec.c`, `mach_loader.c`) | Kernel-side binary loading |

Our design is informed by these standards. Where Apple diverges from POSIX/SysV
(two-level namespaces, chained fixups, @rpath), we follow Apple's model and note
the deviation.

---

## 3. Loader lifecycle — reference model

This is the canonical sequence from `execve()` to `main()`. Every stage maps to
a crate in our workspace. Stages marked with 🔒 have security gates.

```
┌─────────────────────────────────────────────────────────────────┐
│  KERNEL (XNU)                                                   │
│                                                                 │
│  1. execve() → kern_exec.c                                      │
│  2. Recognize Mach-O magic                                       │
│  3. 🔒 AMFI: verify code signature (page hashes, CDHash)        │
│  4. 🔒 SIP: check CS_RESTRICT, strip DYLD_* env for protected   │
│  5. Map executable segments into new address space               │
│  6. Map dyld (/usr/lib/dyld) into the process                   │
│  7. Set up thread state, transfer to dyld's entry point          │
└────────────────────────────────┬────────────────────────────────┘
                                 │
┌────────────────────────────────▼────────────────────────────────┐
│  DYLD (user-space loader)                                       │
│                                                                 │
│  8. ProcessConfig: read executable, env, platform info           │
│  9. 🔒 Check hardened runtime flags → restrict DYLD_* usage     │
│  10. Parse main executable load commands                         │
│  11. For each LC_LOAD_DYLIB:                                     │
│      a. Resolve path (@rpath, @loader_path, @executable_path)   │
│      b. 🔒 Library validation (if hardened/platform binary)      │
│      c. Check dyld shared cache first                            │
│      d. Map from disk if not cached                              │
│      e. Recurse for transitive dependencies                      │
│  12. Process fixups:                                             │
│      - Modern: LC_DYLD_CHAINED_FIXUPS (pointer chains + PAC)    │
│      - Classic: LC_DYLD_INFO_ONLY (rebase/bind/lazy/export)     │
│  13. Apply ASLR slide to all rebases                             │
│  14. Bind symbols (two-level namespace lookup)                   │
│  15. Register ObjC metadata (classes, categories, selectors)     │
│  16. Run initializers bottom-up:                                 │
│      a. ObjC +load methods                                       │
│      b. C/C++ __attribute__((constructor)) functions             │
│      c. C++ static initializers                                  │
│  17. Call libSystem_initializer (special first-init)             │
│  18. Transfer to LC_MAIN entry point (or LC_UNIXTHREAD)          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Crate architecture — aligned with lifecycle

### Layer diagram

```
┌──────────────────────────────────────────────────────────────┐
│  CLI / Harness Layer                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ macho-inspect │  │ test-harness │  │ macho-fixtures   │    │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────┘    │
├─────────┼──────────────────┼─────────────────────────────────┤
│  Plan Layer                │                                  │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌──────────────────┐    │
│  │macho-run-plan│──│  macho-diff  │  │                  │    │
│  └──┬───┬───┬───┘  └──────────────┘  │                  │    │
├─────┼───┼───┼────────────────────────│  Security Layer   │───┤
│  Model Layer       │                 │  (trait-based)    │    │
│  ┌──▼─┐ │ ┌▼──────────┐             │                  │    │
│  │map │ │ │  resolve   │             │ ┌──────────────┐ │    │
│  └────┘ │ └────────────┘             │ │ trust-policy │ │    │
│   ┌─────▼──┐  ┌──────────┐          │ │ code-sign    │ │    │
│   │  objc  │  │  fixups  │ (future)  │ │ sandbox      │ │    │
│   └────────┘  └──────────┘          │ │ amfi-mock    │ │    │
├──────────────────────────────────────│ └──────────────┘ │───┤
│  Parse Layer                         │                  │    │
│  ┌──────────────┐                    └──────────────────┘    │
│  │ macho-parse  │                                            │
│  └──────┬───────┘                                            │
├─────────┼────────────────────────────────────────────────────┤
│  Types Layer                                                  │
│  ┌──────▼───────┐                                            │
│  │ macho-types  │                                            │
│  └──────────────┘                                            │
└──────────────────────────────────────────────────────────────┘
```

### Crate-to-lifecycle mapping

| Lifecycle stage | Crate | Standard |
|---|---|---|
| Binary format structures | `macho-types` | Apple Mach-O format ref |
| Parse header + load commands | `macho-parse` | Apple Mach-O format ref |
| Segment mapping plan | `macho-map` | SysV ABI §5 (segment loading) |
| Path resolution + dep graph | `macho-resolve` | Apple dyld path semantics |
| Fixup processing | `macho-fixups` (future) | Apple chained fixups + classic |
| ObjC metadata registration | `macho-objc` | Apple ObjC runtime source |
| Execution plan generation | `macho-run-plan` | Composite |
| Host comparison | `macho-diff` | Our testing framework |
| Security/trust gates | `trust-policy` (new) | Apple code signing + AMFI |
| CLI inspection | `macho-inspect` | — |
| E2E test orchestration | `test-harness` | — |
| Fixture management | `macho-fixtures` | — |

---

## 5. Security and trust model — mock architecture

Every security gate in Apple's stack is represented as a **Rust trait**. Initial
implementations are permissive mocks that log decisions. This lets us:

- Know exactly where real security enforcement belongs
- Test loader behavior without being blocked by signature requirements
- Drop in real implementations incrementally
- Maintain the same API surface as Apple's actual checks

### 5.1 Security gate inventory

```
Binary on disk
  │
  ▼
┌─────────────────────────┐
│ Gate 1: QUARANTINE       │  Gatekeeper first-launch check
│ Checks: com.apple.quarantine xattr, notarization ticket
│ Trait: QuarantinePolicy
│ Mock: always passes
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 2: CODE SIGNATURE   │  Verify CMS signature & CDHash
│ Checks: LC_CODE_SIGNATURE → SuperBlob → CodeDirectory
│         Page hashes, team ID, signing identity
│ Trait: CodeSignatureValidator
│ Mock: accepts any signature (or unsigned)
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 3: ENTITLEMENTS     │  Parse & evaluate entitlements plist
│ Checks: embedded entitlements in code signature
│ Trait: EntitlementEvaluator
│ Mock: grants all requested entitlements
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 4: AMFI             │  Kernel code-integrity enforcement
│ Checks: CDHash against trust cache, page hash verification
│ Trait: AmfiPolicy
│ Mock: always allows execution
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 5: SIP RESTRICTION  │  System Integrity Protection
│ Checks: CS_RESTRICT flag, protected paths (/usr, /System)
│ Effect: strips DYLD_* env vars, blocks injection
│ Trait: SipPolicy
│ Mock: no paths restricted, no env stripping
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 6: HARDENED RUNTIME │  Library validation, JIT, debugging
│ Checks: CS_RUNTIME flag in code signature
│ Effect: restricts DYLD_INSERT_LIBRARIES, unsigned dylib loads
│ Trait: HardenedRuntimePolicy
│ Mock: no hardened restrictions applied
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 7: LIBRARY VALID.   │  Per-dylib load-time check
│ Checks: dylib must be signed by same team or Apple
│ Triggered: for each dylib after path resolution
│ Trait: LibraryValidator
│ Mock: all libraries pass validation
└────────────┬────────────┘
  │
  ▼
┌─────────────────────────┐
│ Gate 8: SANDBOX          │  File access + IPC restrictions
│ Checks: sandbox profile, file read access for dylib paths
│ Trait: SandboxPolicy
│ Mock: all file access allowed
└────────────┬────────────┘
  │
  ▼
  Code executes in process
```

### 5.2 Trait design

```rust
/// Every security gate implements this core trait.
pub trait SecurityGate: Send + Sync {
    /// Human-readable name for logging.
    fn name(&self) -> &str;

    /// Whether this gate is currently enforcing (true) or permissive (false).
    fn is_enforcing(&self) -> bool;
}

/// Gate 2: Code signature validation.
pub trait CodeSignatureValidator: SecurityGate {
    fn validate(&self, binary_path: &Path, parsed: &ParsedMachO) -> SecurityVerdict;
}

/// Gate 5: SIP environment filtering.
pub trait SipPolicy: SecurityGate {
    fn should_strip_dyld_env(&self, binary_path: &Path, flags: MachFlags) -> bool;
    fn is_path_protected(&self, path: &Path) -> bool;
}

/// Gate 7: Per-dylib validation at load time.
pub trait LibraryValidator: SecurityGate {
    fn validate_dylib(&self, loader_path: &Path, dylib_path: &Path) -> SecurityVerdict;
}

/// Verdict from any security gate.
pub enum SecurityVerdict {
    Allow,
    Deny { reason: String },
    /// Allow but log a warning (for mock/dev mode).
    AllowWithWarning { warning: String },
}
```

### 5.3 Policy composition

```rust
/// All security gates composed into a single policy object.
/// Injected into the loader pipeline at construction time.
pub struct TrustPolicy {
    pub quarantine: Box<dyn QuarantinePolicy>,
    pub code_signature: Box<dyn CodeSignatureValidator>,
    pub entitlements: Box<dyn EntitlementEvaluator>,
    pub amfi: Box<dyn AmfiPolicy>,
    pub sip: Box<dyn SipPolicy>,
    pub hardened_runtime: Box<dyn HardenedRuntimePolicy>,
    pub library_validator: Box<dyn LibraryValidator>,
    pub sandbox: Box<dyn SandboxPolicy>,
}

impl TrustPolicy {
    /// Returns a fully permissive policy (all mocks, nothing enforced).
    pub fn permissive() -> Self { /* ... */ }

    /// Returns a policy matching Apple's default macOS behavior.
    pub fn apple_default() -> Self { /* future */ }
}
```

### 5.4 Where gates are checked

| Gate | Called by | When |
|---|---|---|
| Quarantine | `test-harness` | Before any loading starts |
| Code signature | `macho-parse` or `macho-run-plan` | After parsing, before mapping |
| Entitlements | `macho-resolve` | During env var processing |
| AMFI | `macho-map` | Before mapping segments |
| SIP | `macho-resolve` | Before resolving paths, stripping env |
| Hardened runtime | `macho-resolve` | Before processing DYLD_INSERT_LIBRARIES |
| Library validator | `macho-resolve` | For each resolved dylib |
| Sandbox | `macho-resolve` | For each file access during resolution |

---

## 6. Interoperability strategy

### 6.1 Binary format compatibility

**Goal:** Produce and consume the exact same Mach-O files as Apple's tools.

- Our `macho-types` matches Apple's `<mach-o/loader.h>` structs exactly
- Byte-level roundtrip: `parse → serialize → compare with original = identical`
- Support both fixup models (chained + classic) for full macOS version coverage
- Validate against `otool -l`, `dwarfdump`, `codesign -dv` output

### 6.2 API surface compatibility

**Goal:** Expose the same public API as `dyld` and `libdl`.

| Apple API | Our equivalent | Status |
|---|---|---|
| `dlopen(path, mode)` | `Loader::dlopen()` | Future |
| `dlsym(handle, symbol)` | `Loader::dlsym()` | Future |
| `dlclose(handle)` | `Loader::dlclose()` | Future |
| `dladdr(addr, info)` | `Loader::dladdr()` | Future |
| `_dyld_image_count()` | `ImageList::count()` | Future |
| `_dyld_get_image_name(i)` | `ImageList::name(i)` | Future |
| `_dyld_get_image_header(i)` | `ImageList::header(i)` | Future |
| `_dyld_register_func_for_add_image` | `ImageList::on_add()` | Future |

POSIX compatibility: Our `dlopen`/`dlsym` should honor the same flags as Apple's:
`RTLD_LAZY`, `RTLD_NOW`, `RTLD_GLOBAL`, `RTLD_LOCAL`, `RTLD_FIRST` (Apple extension),
`RTLD_MAIN_ONLY` (Apple extension).

### 6.3 Framework resolution compatibility

Follow Apple's exact search order:

```
1. DYLD_FRAMEWORK_PATH/<name>.framework/<name>     (if not SIP-restricted)
2. /Library/Frameworks/<name>.framework/<name>
3. /System/Library/Frameworks/<name>.framework/<name>
4. DYLD_FALLBACK_FRAMEWORK_PATH/<name>.framework/<name>
```

For dylibs:

```
1. DYLD_LIBRARY_PATH/<name>                        (if not SIP-restricted)
2. @rpath entries (from LC_RPATH in loader + executable)
3. /usr/lib/<name>
4. /usr/local/lib/<name>
5. DYLD_FALLBACK_LIBRARY_PATH/<name>
```

### 6.4 Shared cache awareness

The dyld shared cache (`/System/Volumes/Preboot/.../dyld_shared_cache_arm64e`)
contains most system dylibs pre-linked. Our strategy:

- **Phase 1-4:** Ignore shared cache; resolve from disk paths
- **Phase 5+:** Read shared cache header and image list for comparison
- **Future:** Use shared cache data for accurate symbol resolution

### 6.5 ObjC runtime interop

For binaries with ObjC metadata:

- Read `__objc_classlist`, `__objc_catlist`, `__objc_selrefs`
- Model what `_objc_init()` would register
- Compare our class/category list against `objc_getClassList()` on host
- Do NOT call the real ObjC runtime — model only

---

## 7. Two-level namespace and symbol resolution model

Apple uses two-level namespaces by default (since macOS 10.1). This is a
fundamental difference from ELF's flat namespace.

```
ELF:   symbol "foo" → search all loaded libraries in order
Mach-O: symbol "foo" from "libBar" → search only libBar's exports
```

Our resolver must:

1. Parse the ordinal from each bind entry
2. Map ordinal to the specific dylib (from LC_LOAD_DYLIB order)
3. Look up the symbol in that dylib's export trie only
4. Handle special ordinals: SELF, MAIN_EXECUTABLE, FLAT_LOOKUP, DYNAMIC_LOOKUP

---

## 8. Fixup models — both must be supported

### Classic (macOS ≤ 11, older binaries)

```
LC_DYLD_INFO_ONLY
├── rebase_off/size  → ASLR slide adjustments (opcode stream)
├── bind_off/size    → symbol binds (opcode stream)
├── lazy_bind_off/size → lazy symbol binds (opcode stream)
└── export_off/size  → export trie (compressed prefix tree)
```

### Modern (macOS 12+, arm64e)

```
LC_DYLD_CHAINED_FIXUPS   → pointer chains in __DATA segments
LC_DYLD_EXPORTS_TRIE     → same trie format, separate load command
```

Chained fixups embed fixup metadata directly in the data pages. Each pointer is
encoded with next-pointer offset, bind-or-rebase flag, and on arm64e, PAC key info.

**Crate mapping:**
- Future `macho-fixups` crate handles both models
- `macho-types` defines the data structures for both
- `macho-run-plan` orchestrates fixup processing in the execution plan

---

## 9. Architectural invariants

1. **Parse ≠ interpret.** `macho-parse` reads bytes only. Higher crates assign meaning.

2. **Every stage emits JSON.** All plan/result types derive `Serialize` for
   inspection, diffing, and regression testing.

3. **Security gates are traits.** Every trust decision flows through a trait
   boundary. Mocks and real implementations share the same interface.

4. **Fixtures over random binaries.** Known-good test binaries with declared
   expected outputs are the primary test targets.

5. **Model before execute.** No `mmap()` or code execution until computed plans
   match host observations.

6. **Host macOS is the oracle.** dyld's actual behavior is the reference
   implementation. All correctness is measured against it.

7. **Layered dependencies only.** Crates depend downward only. No cycles, no
   upward imports.

8. **Apple-compatible output.** Our inspect/plan output should be comparable to
   `otool`, `dyldinfo`, and `dyld_info` output for the same binary.

---

## 10. Crate dependency graph

```
test-harness
├── macho-run-plan
│   ├── macho-map         ← macho-parse, macho-types
│   ├── macho-resolve     ← macho-parse, macho-types, trust-policy
│   ├── macho-objc        ← macho-parse, macho-types
│   └── macho-fixups      ← macho-parse, macho-types        (future)
├── macho-diff            ← macho-run-plan
├── macho-fixtures
├── macho-parse           ← macho-types
└── trust-policy          ← macho-types                      (mock first)

macho-inspect (CLI)       ← macho-parse, macho-types, macho-run-plan
```

---

## 11. Development phases (updated)

### Phase 1 — Parse & Inspect ✅
- `macho-types`, `macho-parse`, `macho-inspect`
- Parse real binaries, validate against otool

### Phase 2 — Planning
- `macho-map`, `macho-run-plan`
- Mapping plans, execution plans
- `trust-policy` crate with all-permissive mocks

### Phase 3 — Resolution
- `macho-resolve` with full path semantics
- @rpath, @loader_path, @executable_path
- Dependency graph with correct search order
- Security gates wired in (mocked)

### Phase 4 — Host Comparison
- `macho-diff` with structured comparison
- Automated collection + comparison pipeline
- Regression test suite

### Phase 5 — Fixups & Symbols
- `macho-fixups` crate (classic + chained)
- Export trie parsing
- Two-level namespace symbol resolution
- Rebase/bind plan generation

### Phase 6 — ObjC & Initializers
- `macho-objc` class/category/selector enumeration
- Initializer ordering model
- Compare against DYLD_PRINT_INITIALIZERS

### Phase 7 — Controlled Execution
- In-process segment mapping for trivial fixtures
- Stub symbol resolution
- Controlled entry for test binaries

### Phase 8 — Security Hardening
- Replace mock gates with real implementations
- Code signature parsing (SuperBlob, CodeDirectory)
- Entitlement extraction
- CS_RESTRICT detection and env var stripping

---

## 12. Key design decisions

### Why trait-based security from the start?

The alternative is to bolt security on later, which always results in gaps.
By defining every gate as a trait now:
- The loader pipeline always calls through security checks
- Tests exercise the security API surface even with mocks
- Real implementations slot in without refactoring the loader

### Why both fixup models?

Any binary compiled before macOS 12 uses classic fixups. Any arm64e binary or
macOS 13+ default toolchain binary uses chained fixups. A useful loader lab
must handle both to parse the real-world corpus.

### Why not use goblin/object crate?

We could, but the purpose of the lab is to understand the format. Parsing is
the learning; delegating it to a library defeats the educational goal. We may
cross-validate against goblin's output for correctness.

### Why mock rather than skip security?

Skipping security means the API surface doesn't exist, so there's nothing to
drop a real implementation into. Mocking means the wiring and control flow are
correct from day one — only the policy logic changes.
