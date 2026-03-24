# Runtime Architecture — darwin-loader-lab

> *Build the smallest usable macOS runtime layer above an already-strong
> loader, and let real binaries decide what comes next.*

## Table of contents

1. [Where we are](#where-we-are)
2. [Architecture overview](#architecture-overview)
3. [Crate map](#crate-map)
4. [Target-program matrix](#target-program-matrix)
5. [Layer contracts](#layer-contracts)
6. [Phase plan](#phase-plan)
7. [Design principles](#design-principles)
8. [Anti-goals](#anti-goals)

---

## Where we are

After 33+ phases the **loader core** and **runtime layer** are fully operational:

| Area | Status |
|---|---|
| Mach-O parsing | Complete — headers, segments, load commands, symbols, relocations, fat binaries, code signature, entitlements, data-in-code, cstrings |
| Dependency resolution | Complete — @rpath/@loader_path/@executable_path, frameworks, search envs, transitive BFS, cycle detection, init ordering |
| Fixup application | Complete — classic rebase/bind, chained fixups (all 5 pointer formats), re-export chain following, weak null-fill |
| In-process execution | Complete — contiguous mmap, protection finalization, `StubResolver` with 86+ allowed symbols via `dlsym`, constructor invocation |
| Security modeling | Complete — 8-gate evaluation, trust reports, entitlement keys, encrypted binary rejection |
| Runtime shims | 158 shims across 8 categories (process, TLS, dispatch, pthread, mach, lifecycle, ObjC, Foundation) |
| ObjC runtime | ARM64 assembly `objc_msgSend` / `objc_msgSendSuper` / `objc_msgSendSuper2` trampolines, class registration with superclass chain traversal, metaclass dispatch, selector interning + `__objc_selrefs` uniquing, `__objc_stubs` support, category merge (instance + class methods), +load, retain/release/autorelease, blocks runtime (`_Block_copy`/`_Block_release`), two-phase cross-image registration, post-fixup superclass repair, ObjC introspection (5 helpers), host msgSend fallback with selector translation, tagged pointer forwarding |
| Foundation-lite | 35 Foundation classes: NSObject, NSString/__NSCFConstantString (30 methods), NSArray (9), NSMutableArray (16), NSDictionary (8), NSMutableDictionary (12), NSNumber (6+3), NSMutableString (13), NSData (4+2), NSProcessInfo (7+1), NSError (6+1), NSBundle (4+1), NSFileManager (4+1), NSException (6+1), NSEnumerator (2), NSDate (8+3), NSUserDefaults (8+1), NSSet (10+3), NSMutableSet (16+2), NSURL (8+2), NSNull (1+1), NSDateFormatter (8), NSCharacterSet (1+10), NSNotificationCenter (5+1), NSValue (4+3), NSUUID (4+1), NSLocale (6+1), NSTimeZone (4+5), NSCalendar (6+2), NSNumberFormatter (8), NSIndexSet (6+3), NSMutableIndexSet (5+1), NSSortDescriptor (5+1), NSTimer (5+3); NSLog with ARM64 variadic trampoline + capture |
| T-matrix coverage | T0–T45 all passing — 35 known fixtures, 136 runtime tests + 348 lib/unit tests, 0 warnings |
| Exception handling | Compact unwind registration via `__unw_add_find_dynamic_unwind_sections`, cf-lite→host NSException translation, `@try`/`@catch`/`@finally` through mapped code |
| Code quality | Workspace-wide clippy lint config, silenced error audit, 0 warnings |

---

## Architecture overview

```text
┌────────────────────────────────────────────────────┐
│                   macho-inspect (CLI)               │
│                   test-harness                      │
└────────────────────┬───────────────────────────────┘
                     │
┌────────────────────▼───────────────────────────────┐
│              macho-runtime (orchestration)          │
│  Owns: load → fixup → shim-install → init → entry  │
│  Provides: RuntimeConfig, RuntimeContext            │
│  Coordinates all layers below                       │
└──┬──────┬──────┬──────┬──────┬─────────────────────┘
   │      │      │      │      │
   ▼      ▼      ▼      ▼      ▼
┌──────┐┌────┐┌──────┐
│libsys││objc││ cf-  │
│-shims││-rt ││ lite │
└──────┘└────┘└──────┘
   │      │      │
   ▼      ▼      ▼
┌────────────────────────────────────────────────────┐
│        Loader core (unchanged, stable API)          │
│  macho-types · macho-parse · macho-map              │
│  macho-resolve · macho-objc · macho-exec            │
│  trust-policy · macho-run-plan                      │
└────────────────────────────────────────────────────┘
```

Key principle: the **loader core never gains runtime knowledge**. It parses,
resolves, maps, and fixups. The runtime layer sits atop it and provides the
execution environment that real programs expect.

---

## Crate map

### Existing (stable — loader core)

| Crate | Role | Changes needed |
|---|---|---|
| `macho-types` | Data structures | None |
| `macho-parse` | Binary parser | None |
| `macho-map` | Mapping planner | None |
| `macho-resolve` | Dependency resolution | None |
| `macho-objc` | ObjC metadata parsing (read-only) | None |
| `macho-exec` | mmap + fixup + entry | Minor: expose `MappedImage` internals needed by `macho-runtime` |
| `trust-policy` | Security evaluation | Pluggable, already trait-based |
| `macho-run-plan` | Execution plan display | None |
| `macho-diff` | Binary comparison | None |
| `macho-fixtures` | Fixture metadata | Add new fixture descriptors |
| `test-harness` | Integration tests | Add runtime-level test scenarios |
| `macho-inspect` | CLI | Add `--runtime` flag family |

### New (runtime layer)

#### `macho-runtime` — Orchestration hub

**Purpose**: Single entry point for "load and run a binary to
completion". Replaces the ad-hoc sequence of calls that tests and
macho-inspect currently assemble manually.

**Responsibilities**:
- `RuntimeConfig` — knobs: security policy, allowed symbols, shim selection, env overrides
- `RuntimeContext` — execution state: loaded images, installed shims, TLS state, process metadata
- `Runtime::load_and_run(path, config) -> RuntimeResult` — full pipeline
- Coordinates: parse → resolve deps → map all images → install shims → apply fixups → run initializers → transfer to entry
- Owns the `SymbolResolver` composition: shims → registry exports → host fallback

**Key types**:
```rust
pub struct RuntimeConfig {
    pub security_policy: TrustPolicy,
    pub host_fallback: bool,        // allow dlsym to host process
    pub shim_set: ShimSet,          // which shims to install
    pub env_overrides: Vec<(String, String)>,
    pub process_name: String,
    pub process_path: PathBuf,
    pub verbose: bool,
}

pub struct RuntimeContext {
    pub registry: ImageRegistry,
    pub shim_table: ShimTable,
    pub tls_state: TlsState,
    pub process_info: ProcessInfo,
}

pub struct RuntimeResult {
    pub exit_code: i32,
    pub security_report: Option<SecurityReport>,
    pub images_loaded: usize,
    pub symbols_resolved: usize,
    pub shims_installed: usize,
    pub initializers_run: usize,
}
```

**Does not own**: parsing, mapping, fixup application — delegates to loader core.

#### `libsystem-shims` — libc/libpthread/libdispatch stubs

**Purpose**: Provide controlled implementations of system library
functions that real binaries call, without delegating to the host's
actual libSystem (unless explicitly allowed).

**Design**: Each shim is a Rust function with `extern "C"` ABI that
gets registered in the `ShimTable` and made available to the symbol
resolver. The shim can:
1. Implement the real behavior (e.g., `strlen` → just compute it)
2. Delegate to host (e.g., `malloc` → host `malloc`)
3. Record the call (for testing/tracing)
4. Deny the call (for sandboxing)

**Shim categories** (ordered by priority):

| Category | Functions | Strategy |
|---|---|---|
| **string/memory** | `strlen`, `strcmp`, `strcpy`, `strdup`, `strlcpy`, `memcpy`, `memset`, `memmove`, `bzero` | Implement or delegate to host — these are pure functions |
| **I/O** | `printf`, `fprintf`, `snprintf`, `puts`, `fwrite`, `fflush`, `fopen`, `fclose`, `fread`, `fgets` | Delegate to host — side effects go to real stderr/stdout |
| **allocation** | `malloc`, `free`, `calloc`, `realloc` | Delegate to host — memory must be compatible |
| **process** | `exit`, `_exit`, `abort`, `atexit`, `getenv`, `setenv`, `getpid`, `getppid` | Implement: `exit` → controlled teardown, `getenv` → from RuntimeConfig env map |
| **process identity** | `getprogname`, `_NSGetExecutablePath`, `_NSGetArgc`, `_NSGetArgv`, `_NSGetEnviron` | Implement with values from RuntimeConfig |
| **pthread basics** | `pthread_self`, `pthread_create`, `pthread_mutex_init/lock/unlock/destroy`, `pthread_once` | Phase 1: single-threaded stubs. Phase N: real thread support |
| **TLS** | `_tlv_atexit`, `tlv_get_addr`, `_tlv_bootstrap` | Implement: per-image TLS allocation from parsed TLV descriptors |
| **dispatch** | `dispatch_once`, `dispatch_once_f` | Implement: single-threaded fast path (just call the block) |
| **mach** | `mach_task_self_`, `mach_absolute_time`, `mach_timebase_info` | Implement with real values |
| **stack guard** | `__stack_chk_fail`, `__stack_chk_guard` | Delegate to host (same process) |
| **math** | `sin`, `cos`, `sqrt`, `pow`, `floor`, `ceil`, `log`, `exp`, `fabs` | Delegate to host libm |

**Explicit deny list** (return error / abort):
`system`, `popen`, `fork`, `exec*`, `posix_spawn`, `dlopen`, `dlsym`
(except through controlled runtime path)

**Key types**:
```rust
pub type ShimFn = extern "C" fn() -> i64;  // type-erased

pub struct ShimTable {
    shims: HashMap<String, *const ()>,
}

impl ShimTable {
    pub fn register(&mut self, name: &str, addr: *const ());
    pub fn resolve(&self, name: &str) -> Option<*const ()>;
}

impl SymbolResolver for ShimTable { ... }
```

**Testing strategy**: Each shim gets a unit test. Integration tests
use fixtures that call the shimmed function and verify behavior.

#### `objc-runtime` — Minimal ObjC runtime

**Purpose**: Enough ObjC runtime to register classes from loaded
images and deliver `objc_msgSend` calls to the correct IMP.

**Scope** (ordered by what unblocks real binaries):

| Feature | Description | Needed for |
|---|---|---|
| Class registration | Register classes from `__objc_classlist` into a class table | Any ObjC binary |
| Selector uniquing | Intern selector strings, fix up `__objc_selrefs` | Any ObjC binary |
| `objc_msgSend` dispatch | vtable/cache lookup → IMP call | Any ObjC binary |
| Category registration | Apply category methods to existing classes | Most ObjC binaries |
| Protocol registration | Record protocol conformances | ObjC binaries with protocol checks |
| `+load` dispatch | Call `+load` on registered classes (before main) | ObjC binaries with `+load` |
| `NSObject` basics | `alloc`/`init`/`dealloc`/`retain`/`release`/`autorelease` | Foundation-using binaries |
| Autorelease pool | `objc_autoreleasePoolPush/Pop` | Foundation-using binaries |
| Associated objects | `objc_setAssociatedObject/getAssociatedObject` | Advanced ObjC |
| Blocks | `_Block_copy`, `_Block_release` | Any binary using blocks |

**Design**:
```rust
pub struct ObjcRuntime {
    classes: HashMap<String, ClassInfo>,
    selectors: HashMap<String, Selector>,
    protocols: HashMap<String, ProtocolInfo>,
}

pub struct ClassInfo {
    pub name: String,
    pub superclass: Option<String>,
    pub methods: Vec<MethodInfo>,         // name → IMP address
    pub class_methods: Vec<MethodInfo>,
    pub ivars: Vec<IvarInfo>,
    pub protocols: Vec<String>,
    pub metaclass: Box<ClassInfo>,        // isa chain
}

pub struct MethodInfo {
    pub selector: String,
    pub imp_address: u64,                 // in mapped image
    pub type_encoding: String,
}
```

**Not in scope**: KVO, KVC, method swizzling, NSInvocation,
runtime type creation (`class_addMethod` at runtime). Those are
Foundation-level concerns.

**Implementation order**:
1. Class table + selector interning (read-only, from parsed metadata)
2. `objc_msgSend` trampoline (lookup in class table → call IMP)
3. Category merge
4. `+load` ordering (classes → categories, depth-first)
5. Autorelease pool (stack-based scope)
6. `NSObject` base implementation (alloc/init/dealloc via malloc)

#### `cf-lite` — CoreFoundation object model

**Purpose**: Minimal CFRuntime to support Foundation types. Only
built if the target-program matrix requires it.

**Scope**:
- `CFTypeRef` / `CFRetain` / `CFRelease` / `CFGetTypeID`
- `CFStringRef` — backed by Rust `String` (no CFStringEncoding complexity)
- `CFArrayRef` — backed by `Vec<CFTypeRef>`
- `CFDictionaryRef` — backed by `HashMap`
- `CFBooleanRef`, `CFNumberRef` — trivial wrappers
- `CFBundleRef` — path lookups from `ProcessInfo`
- `CFRunLoopRef` — stub (single run, no actual event loop)

**Not in scope**: CFSocket, CFStream, CFPreferences (push to
`process-services` if needed), CFNotification, CFXMLParser.

#### `process-services` — Process metadata and conventions

**Purpose**: Everything a macOS process expects to know about itself
and its environment. Process identity shims (`getprogname`,
`_NSGetExecutablePath`, etc.) are implemented directly in `libsystem-shims`.

---

## Target-program matrix

Development is driven by making each row pass, top to bottom.

### Completed

| # | Target | What it exercises | Fixture | Status |
|---|---|---|---|---|
| **T0** | `return42` (nostdlib) | Pure computation, no deps | `return42` | ✅ |
| **T1** | `simple_cli` (printf) | GOT/stub, `dlsym` delegation | `simple_cli` | ✅ |
| **T2** | `constructor_test` | Initializer ordering | `constructor_test` | ✅ |
| **T3** | `dylib_chain` | Multi-image, transitive init | `dylib_chain` | ✅ |
| **T4** | TLS-using CLI | TLV runtime allocation | `tlv_basic` | ✅ |
| **T5** | `getenv`/`getprogname` CLI | Process identity | `process_info` | ✅ |
| **T6** | `dispatch_once`-using CLI | GCD single-thread path | `dispatch_test` | ✅ |
| **T7** | Simple ObjC CLI (no Foundation) | Class registration, messaging | `objc_minimal` | ✅ |
| **T8** | ObjC CLI with `NSLog` | Foundation bridge | `objc_foundation` | ✅ |
| **T9** | ObjC CLI with `NSString` ops | Foundation string | `nsstring_ops` | ✅ |
| **T10** | Multi-dylib ObjC app | Cross-image class + category | `objc_dylib_chain` | ✅ |
| **T11** | Block-using binary | Blocks ABI, copy/release | `block_basics` | ✅ |
| **T12** | ObjC inheritance | Superclass chain, __objc_stubs | `objc_inherit` | ✅ |
| **T13** | NSObject methods | Metaclass dispatch, respondsToSelector: | `nsobject_methods` | ✅ |
| **T14** | Super dispatch | objc_msgSendSuper / Super2 | `objc_super` | ✅ |
| **T15** | Dynamic NSString | initWithUTF8String:, ObjC introspection | `nsstring_dynamic` | ✅ |
| **T16** | NSArray | Array creation, element access, containsObject: | `nsarray_basics` | ✅ |
| **T17** | NSString formatting | stringWithFormat: + mixed specifiers | `nsstring_format` | ✅ |
| **T18** | Collection-heavy Foundation | NSDictionary + NSMutableArray + NSMutableDict + NSNumber | multiple | ✅ |

| **T19** | Property introspection | @property, ivar access, getter/setter | `property_introspection` | ✅ |
| **T20** | Runtime introspection | class_getName, object_getClassName, protocols | `runtime_introspection` | ✅ |
| **T21** | NSLog format specifiers | %d, %f, %@, %lu, %p mixed | `nslog_format_specifiers` | ✅ |
| **T22** | NSString path ops | pathExtension, lastPathComponent, path manipulations | `nsstring_path` | ✅ |
| **T23** | NSString mutable | NSMutableString append, insert, delete, replace | `nsstring_mutable` | ✅ |
| **T24** | NSString manipulation | Additional NSString ops (replace, components, range) | `nsstring_manip` | ✅ |
| **T25** | NSData basics | dataWithBytes:length:, bytes, length, isEqualToData: | `nsdata_basics` | ✅ |
| **T26** | NSProcessInfo basics | processName, processIdentifier, environment | `nsprocessinfo_basics` | ✅ |
| **T27** | ObjC categories | Category method merge, cross-image | `objc_category` | ✅ |
| **T28** | ObjC classes | Class creation, ivar layout, method dispatch | `objc_classes` | ✅ |
| **T29** | Fat binary | Universal binary, arm64 slice selection | `fat_binary` | ✅ |
| **T30** | Weak dylib | Weak-linked dylib symbol null fill | `weak_dylib` | ✅ |
| **T31** | Real CLI app | 26 checks across 13 Foundation classes | `real_cli` | ✅ |
| **T32** | Collection enumeration | objectEnumerator, keyEnumerator, NSEnumerator | `collection_enum` | ✅ |
| **T33** | NSDate basics | date creation, comparison, intervals | `nsdate_basics` | ✅ |
| **T34** | NSUserDefaults basics | standardUserDefaults, get/set by key | `nsuserdefaults_basics` | ✅ |
| **T35** | NSSet basics | set creation, dedup, containsObject, set algebra | `nsset_basics` | ✅ |
| **T36** | NSURL basics | fileURLWithPath, URLWithString, path components | `nsurl_basics` | ✅ |
| **T37** | NSNull basics | singleton, identity, in collections | `nsnull_basics` | ✅ |
| **T38** | NSDateFormatter | custom format strings, style-based formatting | `nsdateformatter_basics` | ✅ |
| **T39** | NSCharacterSet | predefined sets, membership, string trim/split | `nscharacterset_basics` | ✅ |
| **T40** | NSNotificationCenter | defaultCenter singleton, post/add/remove no-ops | `nsnotification_basics` | ✅ |
| **T41** | NSValue / NSUUID | Value wrapping, UUID generation | `nsvalue_basics`, `nsuuid_basics` | ✅ |
| **T42** | NSLocale / NSTimeZone | Locale properties, time zone math | `nslocale_basics`, `nstimezone_basics` | ✅ |
| **T43** | NSCalendar | Date components, calendrical calculations | `nscalendar_basics` | ✅ |
| **T44** | NSNumberFormatter | Locale-aware number formatting | `nsnumberformatter_basics` | ✅ |
| **T45** | NSTimer | Scheduled timer fire (single-threaded) | `nstimer_basics` | ✅ |
| **T46** | NSIndexSet / NSMutableIndexSet | Sorted integer collections, range ops | `nsindexset_basics` | ✅ |
| **T47** | NSSortDescriptor | Key-path sorting, compare, reversed | `nssortdescriptor_basics` | ✅ |

### Forward

| # | Target | What it exercises | Key blockers |
|---|---|---|---|
| **T48** | Small real-world CLI Mach-O | External binary not from fixture set | Unknown — drives next roadmap cycle |

Each target gets a fixture binary, a build script entry, and an
integration test that asserts correct output/exit code.

---

## Layer contracts

### Loader core → Runtime

The runtime calls into the loader core through these stable APIs:

```rust
// Parse
macho_parse::parse(data) -> ParsedMachO
macho_parse::extract_fixups(parsed, data) -> Fixups
macho_parse::extract_exports(parsed, data) -> Vec<ExportedSymbol>

// Resolve
macho_resolve::ResolutionPlan::from_parsed_transitive(parsed, path)
macho_resolve::bind_symbols(data, resolution) -> BindingReport

// Map + Execute
macho_exec::MappedImage::map(parsed, data) -> MappedImage
macho_exec::apply_rebases(image, fixups)
macho_exec::apply_binds(image, fixups, bindings)
macho_exec::apply_chained_fixups(image, parsed, data, resolver)
macho_exec::call_constructors(image, initializer_info)
macho_exec::call_entry(image, entry) -> EntryResult

// Multi-image
macho_exec::ImageRegistry::load(name, data) -> usize
macho_exec::ImageRegistry::apply_all_fixups(resolver)

// ObjC metadata (read-only)
macho_objc::parse_objc_metadata(parsed, data) -> ObjcMetadata
macho_objc::extract_initializers(parsed, data) -> InitializerInfo
```

### Runtime → Shims

```rust
// Shim registration
trait ShimProvider {
    fn name(&self) -> &str;
    fn install(&self, table: &mut ShimTable);
}

// Symbol resolution chain
impl SymbolResolver for RuntimeResolver {
    fn resolve(&self, name: &str) -> Option<u64> {
        self.shim_table.resolve(name)
            .or_else(|| self.registry_resolver.resolve(name))
            .or_else(|| if self.config.host_fallback {
                self.host_resolver.resolve(name)
            } else {
                None
            })
    }
}
```

### Runtime → ObjC runtime

```rust
// After images are mapped but before entry:
objc_runtime.register_classes_from_image(parsed, mapped_image)
objc_runtime.register_categories_from_image(parsed, mapped_image)
objc_runtime.fixup_selrefs(parsed, mapped_image)
objc_runtime.run_load_methods()  // +load in correct order
```

---

## Phase plan

### Phase 18 — Runtime scaffolding + TLS

**Goal**: Create `macho-runtime` and `libsystem-shims`. Run T4 (TLS CLI).

Deliverables:
- `macho-runtime` crate with `RuntimeConfig`, `RuntimeContext`, `Runtime::load_and_run()`
- `libsystem-shims` crate with `ShimTable`, process identity shims, TLS shims
- `tlv_runtime` fixture — verifies TLS values survive across function calls
- `RuntimeResolver` composing shims → registry → host
- Integration test proving T4

### Phase 19 — Process services + dispatch

**Goal**: Run T5 (process identity) and T6 (dispatch_once).

Deliverables:
- `process-services` crate with `ProcessInfo`
- `getprogname`/`_NSGetExecutablePath`/`_NSGetArgv` shims
- `dispatch_once` single-threaded shim
- Two new fixtures + tests

### Phase 20 — ObjC runtime (class registration + messaging)

**Goal**: Run T7 (simple ObjC CLI without Foundation).

Deliverables:
- `objc-runtime` crate with class table, selector interning, `objc_msgSend` trampoline
- Class registration from mapped image metadata
- New fixture: pure-ObjC CLI defining a class, instantiating, calling a method
- Integration test proving method dispatch works

### Phase 21 — ObjC runtime (categories + load + autorelease) ✅

**Goal**: Categories, +load dispatch, autorelease pool stubs.

Deliverables (completed):
- `ObjcCategoryRef` struct + `parse_catlist()` in `macho-objc` — parses `__objc_catlist` section (category_t: name, class, instance methods, class methods)
- Metaclass method extraction (`extract_metaclass_methods()`) — parses ISA → metaclass → class_ro_t → method_list for class methods (+load, +initialize)
- `class_methods: Vec<MethodEntry>` field on `ClassInfo` — stores class methods from metaclass
- Category merge in `register_image()` — appends category instance methods to target class via `classes_by_addr` lookup
- `collect_load_methods()` on `ObjcRuntime` — scans class_methods for "load" selector, returns (name, IMP) pairs
- +load dispatch in runtime pipeline (step 8.5) — after mprotect, before constructors, with lock dropped before calling IMPs
- 5 new shims: `objc_autoreleasePoolPush` (sentinel token), `objc_autoreleasePoolPop` (noop), `objc_retain` (pass-through), `objc_release` (noop), `objc_storeStrong` (direct store) → 11 ObjC shims total, 32 shims overall
- `__stderrp`/`__stdoutp`/`__stdinp` + `objc_storeStrong` added to security allowlist
- New fixture: `objc_category` (3-file: greeter.m + extras.m + main.m, compiled with `-no_objc_category_merging`) — tests class + category + +load + objc_msgSend dispatch
- `RUNTIME_LOCK` mutex in runtime integration tests for parallel-safe execution
- 610 tests passing, 0 failures

### Phase 22 — CF-lite + Foundation bridge ✅

**Goal**: Run T8 (ObjC CLI with NSLog) and support Foundation-level fixtures.

Completed:
- `cf-lite` crate (16th workspace crate): Foundation/CF-lite shim layer
  - `nslog` module: `shim_nslog` — extracts C-string from CFConstantString (offset 16), `expand_format` handles `%@`, `%d`, `%s`, `%%`, etc.
  - `nsobj` module: fake NSObject class/metaclass 40-byte structs with ISA cross-references via `UnsafeCell` + `std::sync::Once`
  - `foundation` module: `FoundationShims::install()` registers 4 symbols: `_NSLog`, `___CFConstantStringClassReference`, `_OBJC_CLASS_$_NSObject`, `_OBJC_METACLASS_$_NSObject`
- 3 new ObjC shims: `objc_alloc`, `objc_alloc_init`, `objc_opt_self` (14 total ObjC shims, 39 shims overall)
- `ShimSet.foundation` field — 8th shim category, wired into `RuntimeContext::new()`
- **Critical bug fix**: PTR_64_OFFSET (format 6) chained rebases were using segment vmaddr instead of image base vmaddr, causing all internal pointers (selrefs, classlist, class data) to point to wrong addresses. Fixed by computing `preferred_base` from first non-PAGEZERO segment.
- T8 fixtures verified: `objc_foundation/app` (NSLog with @"..." literal), `objc_classes/app` (NSObject subclass, alloc/init, msgSend dispatch, NSLog)
- 4 new integration tests: `runtime_objc_foundation_exit_zero`, `runtime_objc_foundation_nslog_shim_resolved`, `runtime_objc_classes_with_foundation_exit_zero`, `runtime_objc_classes_foundation_classes_registered`
- 13 cf-lite unit tests, 1 new exec diagnostic test
- 628 tests passing, 0 failures

### Phase 23 — Multi-image ObjC + integration ✅

**Goal**: Run T10 (multi-dylib ObjC app).

Completed:
- `objc_dylib_chain` fixture: `libgreeter.dylib` (defines Greeter class with +load) + `libextras.dylib` (category adding `farewell` method, imports `_OBJC_CLASS_$_Greeter`) + `app` (uses objc_getClass, class_createInstance, objc_msgSend)
- **Two-phase ObjC registration** in runtime pipeline:
  - Phase A: Register classes from on-disk data + merge same-image categories (existing `register_image()`)
  - Phase B: Merge cross-image categories by reading resolved `cls` pointer from mapped memory post-fixup (`merge_categories_post_fixup()`)
- `ObjcCategoryRef.cat_struct_vmaddr` field: records vmaddr of category_t struct for post-fixup memory reads
- Cross-image class symbols (`_OBJC_CLASS_$_Greeter`) resolved via `RegistryResolver` export trie lookup
- Cross-image category merge: `farewell` from libextras merged into Greeter from libgreeter
- +load dispatched from libgreeter before main
- 4 new integration tests: exit_zero, cross_image_class, cross_image_category, load_method
- 632 tests passing, 0 failures

### Phase 24 — Hardening & coverage gaps ✅

**Goal**: Close the last TODO, fill coverage gaps, harden error paths.

Completed:
- **Category class-method merge**: Categories adding class methods (`+foo`) are now merged into the target class's `class_methods` vec, in both `register_image()` (same-image) and `merge_categories_post_fixup()` (cross-image). Previously only instance methods were merged; the single TODO in the codebase is resolved.
- **Category class methods dispatchable via `collect_load_methods()`**: A category adding `+load` now works correctly.
- 2 new unit tests: `category_class_methods_merged_into_class`, `category_class_methods_collected_as_load`
- **`framework_style` runtime test**: The only loadable fixture without runtime integration coverage now has a test (`runtime_framework_style`) verifying exit code 0 and multi-image loading (app + MyFramework.framework).
- **Malformed binary rejection tests**: `runtime_malformed_bad_magic_rejected` and `runtime_malformed_truncated_header_rejected` confirm parse errors bubble cleanly through `Runtime::load_and_run()`.
- **Doc refresh**: Updated "Where we are" section (was 7 phases stale), fixed crate map (removed never-created `process-services` and `host-abstraction` references), updated README with all 16 crates.
- 637 tests passing, 0 failures

### Phase 25 — ARM64 trampoline, CFString/NSString, T9 ✅

**Goal**: Move from "load-and-enter" to sustaining real Objective-C/Foundation execution.
Target T9 (`nsstring_ops`) as the forcing function: CFString creation, retain/release,
toll-free bridging, and ObjC message dispatch through `objc_msgSend`.

Completed:
- **ARM64 `objc_msgSend` assembly trampoline** (`crates/objc-runtime/src/trampoline.s`):
  Hand-written AArch64 stub saves x0-x7 + d0-d7, calls Rust `objc_msg_lookup_imp` for
  class-table lookup, restores all registers, tail-calls the resolved IMP via `br x10`.
  Built via `cc` crate in `build.rs`.
- **Selector reference uniquing** (`__objc_selrefs` fixup): `ObjcMetadata` now captures
  `selref_addrs` (vmaddrs of selector reference slots). New `fixup_selrefs()` method on
  `ObjcRuntime` walks each slot, reads the C string, and overwrites it with the interned
  selector pointer so runtime dispatch matches by pointer equality.
- **Retain/release/autorelease** (`crates/objc-runtime/src/refcount.rs`): `RefCountTable`
  backed by `HashMap<u64, usize>` behind a `Mutex`. `objc_retain`, `objc_release`,
  `objc_autorelease` shims wired up (15 ObjC shims total).
- **CFString / NSString** (`crates/cf-lite/src/cfstring.rs`): Heap-allocated 32-byte
  struct matching Apple's `__CFConstantString` layout. Shim functions:
  `CFStringCreateWithCString`, `CFStringGetLength`, `CFStringGetCStringPtr`, `CFRelease`,
  `kCFAllocatorDefault`. NSString methods (`length`, `UTF8String`, `description`) dispatch
  through the ObjC class table.
- **Synthetic class registration**: `register_synthetic_class()` on `ObjcRuntime` lets
  Foundation provide classes not parsed from Mach-O metadata. `register_nsstring_class()`
  creates `__NSCFConstantString` with 3 method entries at the constant-string class address.
- **4-phase ObjC registration pipeline**: Phase A (classes) → Phase B (cross-image
  categories) → Phase C (selref fixup) → Phase D (synthetic Foundation classes) → global
  store → mprotect → +load → constructors → entry.
- **T9 fixture** (`fixtures/nsstring_ops`): 7 verification checks — CFString creation,
  length, CStringPtr, toll-free bridge, `[nsStr length]` via explicit `objc_msgSend`,
  `[nsStr UTF8String]`, `[nsStr description]`, CFRelease. All pass.
- **Flaky atexit test fix**: Added `ATEXIT_TEST_LOCK` mutex in `libsystem-shims` to
  serialise tests that share the atexit handler table.
- 46 shims total: 21 libSystem + 15 ObjC + 10 Foundation
- 648 tests passing, 0 failures

### Phase 26 — Blocks runtime, T11 ✅

**Goal**: Implement the Objective-C blocks ABI (`_Block_copy`, `_Block_release`,
class reference data symbols) so binaries using `^{}` block literals can execute.

Completed:
- **Blocks module** (`crates/objc-runtime/src/blocks.rs`): Full block ABI support:
  - `_NSConcreteGlobalBlock` and `_NSConcreteStackBlock` data symbols — stable
    addresses matching what clang writes into the `isa` field of block literals.
  - `_NSConcreteMallocBlock` internal class for heap-copied blocks.
  - `_Block_copy`: global → return as-is; stack → `malloc` + bitwise copy + set isa
    to MallocBlock + call copy helper if `BLOCK_HAS_COPY_DISPOSE`; malloc → bump
    refcount via `RefCountTable`.
  - `_Block_release`: global/stack → no-op; malloc → decrement refcount, call
    dispose helper + `free` at 0.
  - `objc_retainBlock`: ARC block retention (delegates to `_Block_copy`).
  - 6 unit tests: class address stability, null safety, global copy identity,
    stack→heap copy, retain/release cycle.
- **20 ObjC shims** (up from 15): added `__NSConcreteGlobalBlock`,
  `__NSConcreteStackBlock`, `__Block_copy`, `__Block_release`, `_objc_retainBlock`.
- **T11 fixture** (`fixtures/block_basics`): 6 verification checks — global block
  invocation, stack block with capture, `Block_copy` to heap, invoke copied block,
  double `Block_copy` returns same pointer, `Block_release` lifecycle, global block
  copy returns identity.
- 51 shims total: 21 libSystem + 20 ObjC + 10 Foundation
- 656 tests passing, 0 failures

### Phase 27 — Superclass chain + `__objc_stubs`, T12 ✅

**Goal**: Support ObjC inheritance and modern clang `__objc_stubs` so real
subclass hierarchies and bracket syntax (`[obj method]`) work correctly.

Completed:
- **Superclass parsing** (`crates/macho-objc`): New `superclass_addr` field on
  `ObjcClassRef` — reads `objc_class_t+8` to get the superclass vmaddr via
  `extract_superclass_addr()`. Flows through to `ClassInfo::superclass_addr:
  Option<u64>` (slide-adjusted).
- **Superclass chain traversal** in `msg_lookup()`: When a selector isn't found on
  the direct class, walks `superclass_addr` up the chain (depth-limited to 64) until
  the method is found or the chain ends. Eliminates the "root classes only" limitation.
- **`__objc_stubs` support**: Modern Xcode clang generates selector-specific stubs
  (`_objc_msgSend$length`) in `__objc_stubs` that load from `__objc_selrefs` and
  call through the GOT-resolved `_objc_msgSend`. Our existing fixup pipeline
  (chained bind → shim table) and Phase C selref uniquing already handle this
  correctly — no new code needed, just verified with a fixture binary.
- **T12 fixture** (`fixtures/objc_inherit`): Three-class hierarchy
  (Shape → Circle → FilledCircle) testing:
  - Direct method dispatch on root class
  - Overridden method on subclass
  - Subclass-specific method
  - Inherited method from parent (one level)
  - Inherited method from grandparent (two levels)
  - Bracket syntax routed through `__objc_stubs` (Xcode clang binary)
  - 12 verification checks, all passing
- 51 shims total: 21 libSystem + 20 ObjC + 10 Foundation
- 658 tests passing, 0 failures

### Phase 28 — NSObject method stubs + metaclass dispatch, T13 ✅

- **Metaclass dispatch**: `msg_lookup()` now checks whether the ISA address is
  a metaclass (tracked in `metaclass_addrs: HashSet<u64>`). If so, searches
  `class_methods` instead of `methods`, enabling `[MyClass classMethod]` via
  `objc_msgSend(widgetClass, sel)` where ISA = metaclass.
- **Metaclass parsing** (`crates/macho-objc`): New `metaclass_addr` field on
  `ObjcClassRef` — reads `objc_class_t.isa` (offset 0) to find the metaclass
  vmaddr. `register_image()` inserts both class and metaclass addresses into
  `classes_by_addr` and tracks metaclass addrs in a `HashSet`.
- **NSObject method shims** (`crates/cf-lite/src/nsobj.rs`): 7 shim functions:
  - Instance: `init` (return self), `self` (return self), `class` (return isa),
    `respondsToSelector:` (calls `msg_lookup`), `isKindOfClass:` (walks chain)
  - Class: `alloc` (calloc + set isa), `new` (delegates to alloc)
- **Synthetic NSObject registration**: `register_nsobject_class()` creates an
  NSObject entry in the ObjC runtime with 5 instance + 2 class methods, plus
  metaclass addr mapping. Registered in Phase D before NSString.
- **Post-fixup superclass repair** (Phase E): `repair_superclass_addrs()` re-reads
  `objc_class_t+8` from mapped memory (where bind fixups have resolved
  `_OBJC_CLASS_$_NSObject` to `nsobject_class_addr()`). Fixes the garbage
  values that `register_image` parsed from on-disk bind encodings.
- **T13 fixture** (`fixtures/nsobject_methods`): NSObject-based hierarchy
  (Widget : BaseObj : NSObject) testing:
  - `alloc`/`init` via `objc_msgSend` on class object (metaclass dispatch)
  - Instance methods: widgetId (own), baseValue (inherited)
  - Class method: `[Widget version]` (metaclass dispatch)
  - `respondsToSelector:` YES/NO
  - `isKindOfClass:` YES (direct, superclass) / NO (unrelated)
  - `self` returns self, `class` returns class
  - 10 verification checks, all passing
- 51 shims total: 21 libSystem + 20 ObjC + 10 Foundation (+ 7 NSObject method shims)
- 660 tests passing, 0 failures

### Phase 29 — `objc_msgSendSuper` / `objc_msgSendSuper2`, T14 ✅

- **ARM64 assembly trampolines** (`trampoline.s`): Two new entry points:
  - `_shim_objc_msg_send_super_asm`: reads `objc_super` struct from x0
    (receiver + super_class), saves all argument registers, calls
    `_objc_msg_super_lookup_imp(receiver, super_class, sel)`, restores regs
    with x0 = receiver (not struct pointer), tail-calls IMP.
  - `_shim_objc_msg_send_super2_asm`: same struct but `super_class` field
    holds the *current* class. The Rust callback follows
    `current_class.superclass_addr` to find the actual parent before lookup.
- **Rust lookup callbacks** (`shims.rs`):
  - `objc_msg_super_lookup_imp`: dispatches `msg_lookup(super_class, sel)`
  - `objc_msg_super2_lookup_imp`: reads `superclass_addr` from class table,
    then dispatches `msg_lookup(parent, sel)`
- **Shim registration**: `_objc_msgSendSuper` + `_objc_msgSendSuper2` added to
  ObjC shims (20 → 22). Non-aarch64 Rust fallbacks also provided.
- **T14 fixture** (`fixtures/objc_super`): Three-level NSObject hierarchy
  (Animal → Dog → Puppy) where every `init` calls `[super init]` and
  methods use `[super method]` for value chaining:
  - `[super init]` chains through Dog → Animal → NSObject
  - `[super legs]` / `[super sound]` resolve to correct parent method
  - `Puppy.chain` = `[super sound]*100 + [super legs]` = 104 (verifies
    multi-level super dispatch)
  - 12 verification checks, all passing
- 53 shims total: 21 libSystem + 22 ObjC + 10 Foundation (+ 7 NSObject method shims)
- 662 tests passing, 0 failures

### Phase 30 — Real NSString + ObjC Introspection, T15 ✅

- **NSString as a proper synthetic class**: `_OBJC_CLASS_$_NSString` and
  `_OBJC_METACLASS_$_NSString` data symbols exported. NSString registered with
  40-byte static class/metaclass data (same `SyncArray` pattern as NSObject).
  Instance methods: `length`, `UTF8String`, `description`, `initWithUTF8String:`,
  `isEqualToString:`. Class method: `stringWithUTF8String:`.
- **Three-level class hierarchy**: NSObject → NSString → `__NSCFConstantString`.
  Literal strings (`@"..."`) are `__NSCFConstantString` instances that inherit
  all NSString methods via the superclass chain.
- **Superclass data fix**: `register_nsstring_class` now writes `nsstring_class_addr()`
  into offset 8 of the `CFCONSTANT_STRING_CLASS` static so that `repair_superclass_addrs()`
  (Phase E) reads the correct value instead of zero.
- **Dynamic NSString creation**:
  - `initWithUTF8String:`: fills pre-allocated 32-byte struct (ISA already set by alloc)
    with flags, c_string_ptr, length. Uses `CString::new` + `into_raw` for the string data.
  - `stringWithUTF8String:`: class factory — calloc(1,32), sets ISA=class, fills string data.
  - `isEqualToString:`: reads c_string_ptr/length from both objects, compares byte-by-byte.
- **ObjC introspection shims** (5 new):
  - `object_getClass`: reads ISA from obj[0]
  - `objc_lookUpClass`: delegates to `objc_getClass`
  - `class_getSuperclass`: reads `superclass_addr` from ClassInfo
  - `class_respondsToSelector`: calls `msg_lookup(cls, sel_str)`
  - `sel_isEqual`: compares CStr contents
- **T15 fixture** (`fixtures/nsstring_dynamic`): 13 tests covering dynamic
  `[[NSString alloc] initWithUTF8String:]`, `[NSString stringWithUTF8String:]`,
  `length`, `UTF8String`, `isEqualToString:` (same/different/literal-vs-dynamic),
  `object_getClass`, `class_getSuperclass`, `class_respondsToSelector`, `sel_isEqual`.
- 60 shims total: 21 libSystem + 27 ObjC + 12 Foundation (+ 7 NSObject method shims + 6 NSString method shims)
- 664 tests passing, 0 failures

### Phase 31 — NSLog Variadic Fix + %@ Formatting ✅

- **Root cause**: ARM64 macOS variadic calling convention places variadic arguments
  on the **stack** at `[sp+0]`, `[sp+8]`, ... — NOT in registers `x1-x7`. The Rust
  `shim_nslog(format, a1, ...)` received `a1` from `x1`, which held stale data,
  causing T8 to print `(null)` and T15 to SIGSEGV.
- **ARM64 assembly trampoline** (`cf-lite/src/nslog_trampoline.s`):
  `_shim_nslog_asm` reads `x0` (format, named param from register), loads
  `[sp+0/8/16]` into `x1-x3`, then tail-calls `_shim_nslog_inner` with normal
  C ABI. Compiled via `cc` build.rs in cf-lite. Previously lived in `objc-runtime`
  but moved to cf-lite to avoid cross-crate linker dependency on `_shim_nslog_inner`.
- **NSLog capture infrastructure**: `static NSLOG_CAPTURE: Mutex<Vec<String>>`
  in `cf-lite/src/nslog.rs`. `shim_nslog_inner` pushes each expanded NSLog string
  to the capture buffer. `drain_captured_nslog()` reads and clears captured lines
  for test verification without parsing stderr.
- **T15 NSLog re-enabled**: `NSLog(@"T15 str1=%@", str1)` was disabled in Phase 30
  due to the SIGSEGV. Now enabled and working — fixture rebuilt.
- **Verification tests** (2 new):
  - `runtime_nslog_format_at_sign_outputs_string`: runs T8, verifies capture
    contains "hello from Foundation" (previously printed "(null)")
  - `runtime_nslog_dynamic_nsstring`: runs T15, verifies capture contains
    "T15 str1=hello" (previously SIGSEGV'd)
- 60 shims (unchanged), 666 tests, 0 failures

### Phase 32 — static-mut Elimination + Warning Cleanup ✅

- **Eliminated all `static mut` UB** in two crates:
  - `objc-runtime/src/blocks.rs`: Replaced hand-rolled `SyncAddr(u64, Once)` pattern
    with `OnceLock<u64>` for block class addresses (`GLOBAL_BLOCK_CLASS`,
    `STACK_BLOCK_CLASS`, `MALLOC_BLOCK_CLASS`). Removed unused `ISA_OFF` and
    `INVOKE_OFF` constants.
  - `libsystem-shims/src/lifecycle.rs`: Replaced `static mut ORDER: Vec<i32>` with
    `Mutex<Vec<i32>>` and `static mut CALLED: bool` with `AtomicBool`.
  - `libsystem-shims/src/pthread.rs`: Replaced `static mut COUNTER: i32` with
    `AtomicI32`.
- **Motivation**: The `static mut` patterns triggered 13 compiler warnings
  ("creating a shared/mutable reference to mutable static") — these are genuine
  undefined behavior and will become hard errors in Rust 2024 edition.
- **Result**: 0 compiler warnings across entire workspace.
- 60 shims (unchanged), 666 tests, 0 failures

### Phase 33 — NSArray + T16 ✅

- **NSArray as a synthetic Foundation class**: `_OBJC_CLASS_$_NSArray` and
  `_OBJC_METACLASS_$_NSArray` data symbols exported. NSArray registered with
  40-byte static class/metaclass data (same `SyncArray` pattern as NSString).
  Instance methods: `count`, `objectAtIndex:`, `firstObject`, `lastObject`,
  `containsObject:`, `description`. Class method: `arrayWithObjects:count:`.
- **NSArray object layout**: 24 bytes — isa(8) + count(8) + elements_ptr(8).
  Elements stored in a separate `malloc`'d `[u64; count]` buffer.
- **Content-aware `containsObject:`**: Compares both by pointer equality and by
  string content (via `extract_cfstring_cstr`) for NSString elements.
- **Refactored `extract_cfstring_cstr`**: Moved from `nslog.rs` (private) to
  `cfstring.rs` (public) so both `nslog.rs` and `nsarray.rs` can share it.
- **T16 fixture** (`fixtures/nsarray_basics`): 18 checks covering
  `arrayWithObjects:count:`, `count`, `objectAtIndex:` (first/last/middle),
  `firstObject`, `lastObject`, `containsObject:` (pointer equality, content
  equality, negative case), empty array (count=0, firstObject=nil, lastObject=nil),
  NSLog `%@` on array element.
- 62 shims total: 21 libSystem + 27 ObjC + 14 Foundation (+ 7 NSObject method
  shims + 6 NSString method shims + 7 NSArray method shims)
- 674 tests, 0 failures, 0 warnings

---

## Design principles

### 1. Binary-driven development

Never implement a runtime feature because "macOS has it". Implement
it because a target on the matrix needs it to run. Evidence: a
fixture binary that fails without the feature and passes with it.

### 2. Shim-first, implement-later

When a binary needs `NSLog`, the first pass is a shim that calls
`fprintf(stderr, ...)`. The real implementation comes later only if
the shim proves insufficient for the next target.

### 3. Resolution chain, not monolith

Symbol resolution is a chain: shim table → loaded image exports →
host fallback. Each layer can be independently tested and swapped.

### 4. Single-threaded until proven otherwise

All runtime structures are `!Send + !Sync` initially. Threading
support (real `pthread_create`) is added only when a target binary
requires it, and only behind a `RuntimeConfig` flag.

### 5. Loader core is frozen

The 12 existing crates are stable. No new features or refactors to
the loader core unless a runtime feature *cannot* be implemented
without it. If the runtime needs a loader API that doesn't exist,
the API is added minimally (e.g., exposing a field, not redesigning
a module).

### 6. Test at every layer

- Unit tests for each shim function
- Integration tests for each target-program row
- Regression tests: every previously-passing fixture continues to pass

---

## Observability / Runtime Tracing

The `RuntimeTracer` module (`macho-runtime/src/tracer.rs`) provides structured
event collection throughout the load-and-run pipeline.

### Usage

```rust
let tracer = RuntimeTracer::new();
let mut config = RuntimeConfig::permissive(&binary);
config.tracer = Some(tracer);

let result = Runtime::load_and_run(&binary, &config)?;
let tracer = result.tracer.expect("tracer enabled");

// Query collected events
println!("{} total events", tracer.count());
for ev in tracer.events_of(EventKind::ImageLoad) {
    println!("  loaded: {}", ev.message);
}
```

### Event Kinds

| Kind | Source | Description |
|---|---|---|
| `Pipeline` | `lib.rs` | High-level stage milestones (parse, mprotect, +load, entry) |
| `ImageLoad` | `lib.rs` | Each image mapped (name, base address, segment count) |
| `ClassRegistration` | `lib.rs` | Each ObjC class registered (name, source image) |
| `CategoryMerge` | `lib.rs` | Each category merged (name, method counts) |
| `SelectorRegistration` | `lib.rs` | Selector fixup batches (count per image) |
| `MethodLookup` | `dispatch.rs` | Per-message dispatch via `log::trace!` |
| `BlockLifecycle` | lifecycle shims | Block copy/release via `log::debug!` |
| `RetainRelease` | ARC shims | Retain/release operations via `log::debug!` |
| `FoundationCreate` | `lib.rs` | Synthetic Foundation class registration |

### Design

- **Thread-safe**: `Arc<Mutex<Vec<RuntimeEvent>>>` + `Arc<AtomicBool>` enable flag
- **Zero-cost when disabled**: `RuntimeTracer::disabled()` short-circuits all `record()` calls
- **Clone-friendly**: All clones share the same underlying event buffer
- **Filtering**: `events_of(kind)`, `count_of(kind)` for targeted queries

---

## Anti-goals

| Do not | Reason |
|---|---|
| Build a full ObjC runtime | Apple's is ~50K LOC. We need class registration + msgSend. |
| Implement CoreAnimation / AppKit / UIKit | GUI frameworks are out of scope for this milestone |
| Support 32-bit binaries | All Apple Silicon is 64-bit. 32-bit adds complexity with no target. |
| Build a dyld shared cache parser | Cache parsing is interesting but doesn't unblock any target. |
| Implement real thread scheduling | `pthread_create` shim returns `ENOSYS` until T-matrix needs it. |
| Replace `dlsym` host fallback | For math, memory, I/O — the host implementation is correct. Replacing it adds bugs. |
| Deep code signing verification | CMS signature validation is Apple's job. We check presence/flags. |
| PAC/pointer authentication emulation | ARM64e binary support is interesting but no target needs it yet. |
