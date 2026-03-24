# Expansion Roadmap — Post-Phase 38

> **Two tracks**: Deepen the runtime's ObjC/Foundation fidelity, and make the
> stack consumable from other languages and package managers.

---

## Current state (post-Phase 38)

| Metric | Value |
|---|---|
| Workspace members | 30 (27 crates + 3 apps) |
| Foundation classes | 40 |
| Shim functions | 59 (libsystem-shims) |
| Fixtures | 62 executable |
| Tests | 900+ passing, 0 failing |
| Compatibility | 62/62 (100%) |
| GUI apps | 3 |
| Phases completed | 1–38 (all) |

---

## Track A — Package Manager Interoperability

**Goal**: Make the runtime's core functionality consumable from Swift, C, and
other languages via standard package managers. A Swift app should be able to
`import CDarwinRuntime` and call the loader API.

### Phase A1 — C-Compatible Dynamic Libraries ✅

Expose key crates as `cdylib` targets with stable C ABI.

#### Tasks

- [x] Create `crates/darwin-runtime-ffi/` — thin C-ABI wrapper crate (`cdylib` + `rlib`)
- [x] Define `extern "C"` entry points: `darwin_runtime_run(path)`, `darwin_runtime_run_secure(path)`, `darwin_runtime_version()`
- [x] Define `extern "C"` entry points for ObjC runtime: `darwin_objc_get_class(name)`, `darwin_objc_class_count()`, `darwin_objc_selector_count()`, `darwin_objc_list_classes()`, `darwin_objc_list_methods()`
- [x] Define `extern "C"` entry points for Foundation: `darwin_foundation_class_count()`
- [x] Add `#[no_mangle]` and `extern "C"` to all 13 public FFI functions
- [x] Thread-local error handling: `darwin_runtime_last_error()`
- [x] Memory management helpers: `darwin_string_free()`, `darwin_class_list_free()`, `darwin_method_list_free()`
- [x] Create `build.rs` that runs `cbindgen` to generate `include/darwin_runtime.h`
- [x] Write 17 unit tests for all FFI entry points (null safety, error paths, fixture execution, runtime introspection)
- [x] Verify `libdarwin_runtime_ffi.dylib` exports all 13 expected symbols via `nm -gU`

#### Acceptance

- [x] `cargo build -p darwin-runtime-ffi` produces `libdarwin_runtime_ffi.dylib`
- [x] `cbindgen` generates valid `darwin_runtime.h` header with C++ guards
- [x] All FFI functions are memory-safe (no UB across FFI boundary)
- [x] 17/17 tests pass, clippy clean, fmt clean

---

### Phase A2 — pkg-config & System Integration ✅

Make the library discoverable by standard build tools.

#### Tasks

- [x] Create `packaging/darwin-runtime.pc.in` template (prefix, libdir, includedir)
- [x] Add FFI library, header, and pkg-config to `packaging/bundle-runtime.sh`
- [x] Create `packaging/install.sh` — installs dylib + headers + binaries + pkg-config to prefix
- [x] Uninstall support (`--uninstall` flag removes all installed files)
- [x] Dry-run mode (`--dry-run` shows what would be installed)
- [x] Auto-build on install if dylib not found
- [x] Create `examples/c-client/test_darwin_runtime.c` — C test program using the FFI API
- [x] Test: `pkg-config --cflags --libs darwin-runtime` outputs correct flags
- [x] Test: C program builds using `$(pkg-config --cflags --libs darwin-runtime)` and runs (11/11 checks)

#### Acceptance

- [x] `pkg-config darwin-runtime --exists` returns 0
- [x] `cc -o test test.c $(pkg-config --cflags --libs darwin-runtime)` compiles and links
- [x] C test client passes 6/6 basic tests (version, error handling) and 11/11 with fixture (runtime execution + introspection)
- [x] Install and uninstall scripts work cleanly (8 files installed/removed)

---

### Phase A3 — Swift Package Manager Wrapper ✅

Create an SPM-consumable package that wraps the C library.

#### Tasks

- [x] Create `swift/Package.swift` with CDarwinRuntime (C module) + DarwinRuntime (Swift wrapper) targets
- [x] Create `swift/Sources/CDarwinRuntime/include/module.modulemap` linking `libdarwin_runtime_ffi`
- [x] Create `swift/Sources/CDarwinRuntime/include/shim.h` (umbrella header importing `darwin_runtime.h`)
- [x] Symlink `darwin_runtime.h` from cbindgen output into SPM include directory
- [x] Create `swift/Sources/DarwinRuntime/DarwinRuntime.swift` — idiomatic Swift wrapper
- [x] Implement `DarwinRuntime` enum: `version`, `run(path:)`, `runSecure(path:)`, `lastError`
- [x] Implement `ObjCRuntime` enum: `classCount`, `selectorCount`, `foundationClassCount`, `getClass(name:)`, `listClasses()`, `listMethods(className:)`
- [x] Swift types: `Version`, `RunResult`, `ClassInfo`, `MethodInfo`, `RuntimeError` — all `Sendable`
- [x] Create `swift/Tests/DarwinRuntimeTests/DarwinRuntimeTests.swift` — 15 tests in 4 suites
- [x] Test: `swift build` in `swift/` directory succeeds
- [x] Test: `swift test` passes all 15 Swift tests (version, error handling, pre-run introspection, fixture execution + ObjC introspection)

#### Acceptance

- [x] `swift build` compiles the package (0.68s)
- [x] Swift code can `import DarwinRuntime` and call `DarwinRuntime.version`
- [x] Swift test suite validates FFI roundtrip: run fixture → introspect 40+ classes, 254+ selectors, NSString methods

---

### Phase A4 — Homebrew Formula

Distribute via Homebrew for easy installation.

#### Tasks

- [ ] Create `Formula/darwin-runtime.rb`
- [ ] Formula downloads release tarball or builds from source via `cargo build --release`
- [ ] Formula installs: dylib → `lib/`, headers → `include/`, pkg-config → `lib/pkgconfig/`
- [ ] Formula installs binaries: `runtime-shell`, `macho-inspect` → `bin/`
- [ ] Formula installs demo apps: `hello-darwin`, `control-gallery`, `foundation-explorer` → `bin/`
- [ ] Add `test` block: verifies `darwin_runtime_version()` returns non-null
- [ ] Test: `brew install --build-from-source ./Formula/darwin-runtime.rb` succeeds
- [ ] Test: `brew test darwin-runtime` passes

#### Acceptance

- [ ] `brew install darwin-runtime` (from local tap) works
- [ ] `runtime-shell` launches after install
- [ ] `pkg-config darwin-runtime --exists` works after install
- [ ] `brew uninstall darwin-runtime` removes all installed files

---

### Phase A5 — Documentation & Example Projects

#### Tasks

- [ ] Create `docs/FFI_GUIDE.md` — how to use the C API from C/Swift/Python
- [ ] Create `examples/c-client/` — minimal C program using libdarwin_runtime
- [ ] Create `examples/swift-client/` — minimal Swift app using DarwinRuntime package
- [ ] Update `README.md` with "Use as a library" section
- [ ] Add CI job: build FFI crate + run C client + run Swift client

#### Acceptance

- [ ] Both example projects build and run
- [ ] FFI_GUIDE.md covers all public API functions
- [ ] CI validates the interop story on every push

---

## Track B — Runtime Depth

**Goal**: Extend the ObjC/Foundation runtime to handle more advanced patterns
found in real macOS applications. Each phase unlocks a new class of binaries.

### Phase B1 — Key-Value Observing (KVO)

Foundation's observer pattern. Required for bindings-heavy apps.

#### Tasks

- [ ] Implement `addObserver:forKeyPath:options:context:` on NSObject
- [ ] Implement `removeObserver:forKeyPath:` on NSObject
- [ ] Implement `willChangeValueForKey:` / `didChangeValueForKey:` callback dispatch
- [ ] Implement `observeValueForKeyPath:ofObject:change:context:` callback protocol
- [ ] Dynamic KVO subclass generation (prefix `NSKVONotifying_`)
- [ ] KVO change dictionary (`NSKeyValueChangeKindKey`, `NSKeyValueChangeNewKey`, `NSKeyValueChangeOldKey`)
- [ ] Options support: `NSKeyValueObservingOptionNew`, `NSKeyValueObservingOptionOld`, `NSKeyValueObservingOptionInitial`
- [ ] Create `kvo_basics` fixture — observe property changes, verify callbacks
- [ ] Create `kvo_multiple_observers` fixture — multiple observers on same key path
- [ ] Write unit tests for KVO registration, notification, and removal

#### Acceptance

- [ ] Property changes trigger observer callbacks
- [ ] Multiple observers on same key path all notified
- [ ] Removing an observer stops notifications
- [ ] KVO fixtures pass through runtime pipeline

---

### Phase B2 — Key-Value Coding (KVC)

Programmatic property access by name string. Foundation of KVO, bindings, and Core Data.

#### Tasks

- [ ] Implement `valueForKey:` — accessor method lookup → ivar fallback
- [ ] Implement `setValue:forKey:` — setter method lookup → ivar fallback
- [ ] Implement `valueForKeyPath:` — dot-separated recursive resolution
- [ ] Implement `setValue:forKeyPath:` — dot-separated recursive setter
- [ ] Implement `valueForUndefinedKey:` / `setValue:forUndefinedKey:` — NSUndefinedKeyException
- [ ] KVC accessor pattern detection: `<key>`, `is<Key>`, `_<key>`, `_is<Key>`
- [ ] KVC setter pattern detection: `set<Key>:`, `_set<Key>:`
- [ ] Auto-boxing: wrap/unwrap NSNumber for scalar properties
- [ ] Collection operators: `@count`, `@sum`, `@avg`, `@min`, `@max`
- [ ] Create `kvc_basics` fixture — get/set by key, key path, undefined key
- [ ] Write unit tests for accessor pattern matching and auto-boxing

#### Acceptance

- [ ] `valueForKey:` finds accessor methods and ivars
- [ ] `setValue:forKey:` finds setter methods and ivars
- [ ] Key path traversal works across object graphs
- [ ] Undefined key raises exception
- [ ] KVC fixture passes through runtime pipeline

---

### Phase B3 — Method Swizzling

Runtime method replacement. Used by testing frameworks, analytics SDKs, and
some Foundation internals.

#### Tasks

- [ ] Implement `method_exchangeImplementations` — swap IMP pointers between two methods
- [ ] Implement `method_setImplementation` — replace a method's IMP, return old IMP
- [ ] Implement `class_replaceMethod` — replace or add a method on a class
- [ ] Thread-safety: method table modifications under lock
- [ ] Verify swizzling works across superclass chain (dispatch finds new IMP)
- [ ] Create `swizzle_basics` fixture — swizzle NSString method, verify new behavior
- [ ] Create `swizzle_restore` fixture — swizzle and restore original, verify roundtrip
- [ ] Write unit tests for exchange, set, and replace operations

#### Acceptance

- [ ] `method_exchangeImplementations` swaps behavior bidirectionally
- [ ] `method_setImplementation` returns old IMP and installs new
- [ ] Swizzled methods are found by `objc_msgSend` dispatch
- [ ] Both fixtures pass through runtime pipeline

---

### Phase B4 — NSInvocation & Dynamic Method Resolution

Full method invocation objects and forwarding. Required for undo managers,
distributed objects, and proxy patterns.

#### Tasks

- [ ] Implement `NSInvocation` class — encapsulates target, selector, arguments, return value
- [ ] Implement `NSMethodSignature` class — type encoding parser for argument/return types
- [ ] Implement `+signatureWithObjCTypes:` — create signature from type encoding string
- [ ] Implement `-invocationWithMethodSignature:` — factory method
- [ ] Implement `-setArgument:atIndex:` / `-getArgument:atIndex:` — raw buffer argument access
- [ ] Implement `-invoke` — call target with stored selector and arguments
- [ ] Implement `-getReturnValue:` — extract return value after invoke
- [ ] Implement `forwardInvocation:` on NSObject — forwarding machinery
- [ ] Implement `methodSignatureForSelector:` on NSObject — signature lookup
- [ ] Implement `+resolveInstanceMethod:` / `+resolveClassMethod:` — dynamic method resolution
- [ ] Create `nsinvocation_basics` fixture — create invocation, set args, invoke, get result
- [ ] Create `forwarding_basics` fixture — forwardInvocation: proxy pattern
- [ ] Write unit tests for type encoding parser and argument buffer management

#### Acceptance

- [ ] NSInvocation can invoke arbitrary methods with correct argument passing
- [ ] forwardInvocation: chain works for unrecognized selectors
- [ ] Dynamic method resolution can lazily add methods
- [ ] Both fixtures pass through runtime pipeline

---

### Phase B5 — PAC / ARM64e Pointer Authentication

Pointer authentication for security-hardened binaries. Required to run
binaries compiled with `-arch arm64e`.

#### Tasks

- [ ] Implement PAC stripping in pointer resolution (mask out PAC bits)
- [ ] Support `DYLD_CHAINED_PTR_ARM64E` (format 1) rebase with auth bit handling
- [ ] Support `DYLD_CHAINED_PTR_ARM64E_USERLAND` (format 12) with 24-bit ordinal
- [ ] Support `DYLD_CHAINED_PTR_ARM64E_USERLAND24` (format 13) bind + rebase
- [ ] Implement `ptrauth_sign_unauthenticated` equivalent for signing function pointers
- [ ] Implement `ptrauth_strip` for stripping auth from pointers before use
- [ ] Handle PAC key discrimination (IA/IB/DA/DB keys)
- [ ] Handle `__auth_got` section (authenticated GOT entries)
- [ ] Create `arm64e_basic` fixture — compiled with `-arch arm64e`, basic function calls
- [ ] Write unit tests for PAC bit manipulation and pointer resolution

#### Acceptance

- [ ] ARM64e binaries load and execute through the runtime
- [ ] PAC-signed pointers are correctly stripped/verified
- [ ] Auth GOT entries resolve correctly
- [ ] Fixture passes through runtime pipeline

---

### Phase B6 — Additional Foundation Classes

Expand Foundation coverage for more real-world app compatibility.

#### Tasks

- [ ] `NSRegularExpression` — regex matching via `regex` crate backend
- [ ] `NSPredicate` — predicate evaluation for filtered collections
- [ ] `NSAttributedString` — string with attribute ranges
- [ ] `NSCache` — auto-evicting in-memory cache
- [ ] `NSMapTable` / `NSHashTable` — weakly-referenced collection variants
- [ ] `NSOperationQueue` / `NSOperation` — task queue abstraction (single-threaded)
- [ ] `NSJSONSerialization` — JSON ↔ Foundation object conversion
- [ ] `NSPropertyListSerialization` — plist ↔ Foundation object conversion
- [ ] Create fixtures for each new class
- [ ] Write unit tests for each new class

#### Acceptance

- [ ] Each new class passes its fixture through the runtime
- [ ] JSON/plist roundtrip works correctly
- [ ] NSRegularExpression matches patterns correctly
- [ ] All existing tests still pass (no regressions)

---

## Priority & Sequencing

### Recommended order

```
Track A (interop)          Track B (depth)
═══════════════            ═══════════════
A1 — cdylib + FFI          B1 — KVO
A2 — pkg-config            B2 — KVC
A3 — SPM wrapper           B3 — Method swizzling
A4 — Homebrew              B4 — NSInvocation
A5 — Docs & examples       B5 — ARM64e / PAC
                            B6 — More Foundation
```

Tracks A and B are **independent** and can be worked in parallel or interleaved.
Within each track, phases are sequential.

### Track A rationale

Makes the project useful to others without them needing to understand Rust.
A Swift developer can `import DarwinRuntime` and use the loader as a library.
A `brew install` makes the demo apps accessible to anyone with a Mac.

### Track B rationale

Each phase unlocks a new category of real-world binaries. KVO/KVC alone
covers a large swath of Cocoa patterns. Method swizzling enables testing
frameworks. ARM64e is needed for Apple silicon system binaries.

---

## Success criteria

The expansion is complete when:

1. **Track A**: A Swift app can `import DarwinRuntime`, load a Mach-O binary, and
   inspect its ObjC classes — installed via `brew install darwin-runtime`
2. **Track B**: A binary using KVO, KVC, and method swizzling runs correctly
   through the runtime pipeline

---

## Anti-goals (carried forward)

| Do not | Reason |
|---|---|
| Build a full custom OS kernel | Userspace runtime only |
| Clone all of AppKit | Basic GUI is sufficient for demos |
| Support iOS/watchOS/tvOS | macOS ARM64 only |
| Make it production-ready for App Store | Research/demonstration project |
| Replace Xcode | CLI + runtime shell is the interface |
| Implement full Distributed Objects | NSInvocation local-only is sufficient |
| Match Apple's PAC implementation exactly | Functional compatibility, not security parity |
