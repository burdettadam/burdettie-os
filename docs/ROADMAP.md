# Darwin Loader Lab — Strategic Roadmap

> **As of 2025-07-17.** This document tracks where the project is headed.
> For a complete reference of what works today, see [FEATURES.md](FEATURES.md).

---

## Vision

A full Rust implementation of the macOS userspace runtime — Mach-O loader, ObjC
runtime, Foundation, libSystem — that can execute unmodified ARM64 macOS binaries
on any platform where ARM64 hardware exists, beginning with macOS and Linux ARM64
and extending to x86_64 via binary translation.

The project is not a macOS clone. It is a runtime laboratory: every design
decision is made observable, every security gate is represented as a typed Rust
trait, and correctness is verified against real Apple tooling at every layer.

---

## Near-Term Milestones *(3–6 months)*

### 1. arm64e / Pointer Authentication Support (P0)

**What it is**: Support for ARM64E binaries that use pointer authentication
codes (PAC). On macOS, every `/usr/bin` system tool and most system frameworks
are compiled as arm64e.

**Why it matters**: This is the **single largest compatibility gap** in the
project. Broad testing (Phase 2, 2026-03-30) showed that all 58 tested
`/usr/bin` tools crash because they are arm64e. Without this, the loader
cannot run `grep`, `sed`, `awk`, `find`, `tar`, `defaults`, `plutil`,
`security`, `codesign`, or any other standard macOS CLI utility.

**Current state**: The loader recognizes `CpuSubtype::Arm64E` and has PAC
shim stubs (`ptrauth_strip`, `ptrauth_sign_unauthenticated`) but does not
handle PAC bits in fixups or authenticate/strip pointers during binding.

**Work remaining**:
- [ ] Detect arm64e slices during fat binary selection and log a clear diagnostic
- [ ] Strip PAC bits (mask `& 0x0000_FFFF_FFFF_FFFF`) from pointers during chained fixup application
- [ ] Handle `BIND_OPCODE_THREADED` / authenticated bind opcodes in dyld chained fixups
- [ ] Handle `DYLD_CHAINED_PTR_ARM64E*` pointer formats in import resolution
- [ ] Test against `/usr/bin/env`, `/usr/bin/uname` as simplest arm64e smoke tests
- [ ] Regression test: all 26 currently-passing Xcode toolchain tools still pass

---

### 2. Unresolved arm64 SIGBUS Crashes (P1)

**What it is**: Fix crashes in plain arm64 Xcode toolchain tools that should
work but crash with SIGBUS (unifdef, clang, ld, libtool, clangd, sourcekit-lsp).

**Why it matters**: These are the next 9 tools to unlock. `unifdef` is a
pure libSystem-only binary — the simplest possible crash case — making it
the ideal Rosetta Stone for diagnosing this class of bug.

**Current state**: 26/35 Xcode toolchain tools pass but 9 crash. The
crashing and passing tools have identical Mach-O segment layouts (`__PAGEZERO`,
`__TEXT`, `__DATA_CONST`, `__DATA`, `__LINKEDIT`). TLS and exception tables
don't correlate with crashes. The crash occurs immediately after "Calling
entry..." suggesting a fixup application or memory mapping issue.

**Work remaining**:
- [ ] Debug `unifdef` as minimal repro: compare its chained fixup imports to a passing tool (e.g., `indent`)
- [ ] Check for unresolved GOT entries (zero pointers) that could cause SIGBUS on first call
- [ ] Verify `__DATA_CONST` page protections are writable during fixup, read-only after
- [ ] Check if `libresolv.9.dylib` resolution works (needed by clang/clangd)
- [ ] Check `@rpath/libLTO.dylib` resolution for libtool

---

### 5. Linux End-to-End CI (E4 completion)

**What it is**: Wire the existing Linux ARM64 Docker test environment into
continuous integration so that every pull request verifies the full fixture suite
on Linux, not just macOS.

**Current state**: The Docker image (`docker/Dockerfile.linux-e2e`) exists and
the E4a–E4c interpreter fixes are complete. 22 fixtures pass on a macOS ARM64
host via interpreter. The Linux CI matrix job is not yet wired.

**Work remaining**:
- [ ] Connect `docker/Dockerfile.linux-e2e` to CI matrix (GitHub Actions / equivalent)
- [ ] Identify and fix any nightly-only or macOS-only compile guards that block cross-compilation
- [ ] Confirm fixture suite pass rate on real Linux ARM64 hardware matches macOS host interpreter results
- [ ] Green CI badge for `aarch64-unknown-linux-gnu`

---

### 4. NSURLSession

**What it is**: The Foundation HTTP client. Used by any CLI tool or app that
makes HTTP requests, downloads files, or calls REST APIs.

**Why it matters**: It is the highest-impact remaining Foundation API gap. Every
networking tool — from package managers to API clients — depends on it. Its
absence blocks the largest uncovered category of real-world macOS software.

**Scope**:
- [ ] `NSURLSession` — `sharedSession`, `dataTaskWithURL:completionHandler:`, `dataTaskWithRequest:completionHandler:`
- [ ] `NSURLSessionDataTask` — `resume`, `cancel`, `state`, `response`, `error`
- [ ] `NSURLSessionConfiguration` — `defaultSessionConfiguration`, `ephemeralSessionConfiguration`
- [ ] Synchronous HTTP path first (blocking on a POSIX socket); async task delivery later
- [ ] Wire into `libsystem-shims` credential / cookie storage stubs

---

### 3. Foundation Selector Expansion (P2)

**What it is**: Implement commonly-used Foundation selectors that real-world
tools depend on but are not yet shimmed.

**Why it matters**: Broad testing showed tools like `altool` crashing on
missing `stringWithCString:encoding:`. As more tools reach Foundation code
paths (especially after arm64e and SIGBUS fixes), missing selectors will be
the next category of failures.

**Known missing selectors** (discovered in Phase 2 testing):
- [ ] `NSString stringWithCString:encoding:` — C string to NSString (altool)
- [ ] Additional selectors from `xcodebuild` NSException (needs triage)
- [ ] Selectors needed by `ibtool`, `actool` startup paths

---

### 6. NSKeyedArchiver / NSKeyedUnarchiver ✅

**What it is**: Binary object graph serialization. Used by Xcode build caches,
user preferences, and any tool that persists complex state.

**Status**: Complete. Full encode/decode cycle works including NSCoding protocol
dispatch (`encodeWithCoder:` / `initWithCoder:`) to guest-defined classes.
The `protocol_nscoding` fixture passes end-to-end.

**Scope** (all done):
- [x] `NSKeyedArchiver` — `archivedDataWithRootObject:`, `archiveRootObject:toFile:`
- [x] `NSKeyedUnarchiver` — `unarchiveObjectWithData:`, `unarchiveObjectWithFile:`
- [x] `NSCoding` protocol support — `encodeWithCoder:`, `initWithCoder:` dispatch
- [x] Basic type encoding: NSString, NSNumber, NSArray, NSDictionary, NSData, NSURL
- [x] `NSCoder` abstract class implementation (required by NSKeyedArchiver)

---

### 7. Remaining Foundation Gaps ✅

All Phase C9 gaps are now implemented with tests.

| API | Status |
|---|---|
| NSStream / NSInputStream / NSOutputStream | ✅ POSIX fd-backed stream implementation |
| NSRunLoop selectors (`run`, `runUntilDate:`, `runMode:beforeDate:`) | ✅ Wired into `runloop-core` |
| NSXMLParser | ✅ Pure-Rust SAX-style XML parser |

---

## Medium-Term Goals *(6–18 months)*

### T48: Run an Unmodified Real-World ARM64 Binary

**What it is**: Execute a small, unmodified ARM64 macOS CLI binary from the wild
— not a source-compiled fixture — successfully through the runtime.

**Why it matters**: The entire project has been validated against fixtures we
compiled ourselves. T48 is the first test against unknown external code, which
will surface every remaining gap in symbol coverage, shim correctness, and
loader accuracy.

**Prerequisites**: NSURLSession (networking tools need HTTP support).
The NSKeyedArchiver and Foundation gaps above are now complete.

**Candidate binaries**: Simple open-source CLI tools compiled without
Xcode-specific features. Note: Phase 2 testing (2026-03-30) effectively
demonstrated T48 partially — 33 unmodified Apple CLI tools already run
successfully through the loader. The remaining gap is running tools that
produce meaningful output with real arguments, not just `--help`.

---

### Swift Runtime Interoperability

**What it is**: Load and resolve Swift standard library dylibs
(`libswiftCore.dylib` and dependencies) so that Swift-native CLI tools can
start up.

**Why it matters**: Modern Xcode tools (`sourcekit-lsp`, `swift-api-digester`,
`swift-symbolgraph-extract`) link against the Swift runtime. As Apple
increasingly ships Swift-first tools, this becomes a prerequisite for broad
Xcode compatibility.

**Work**:
- [ ] Load `libswiftCore.dylib` from `/usr/lib/swift/` and apply fixups
- [ ] Handle Swift metadata sections (`__swift5_types`, `__swift5_protos`, `__swift5_proto`)
- [ ] Resolve `@rpath/lib_CompilerSwift*.dylib` for Xcode compiler tools
- [ ] Shim critical Swift runtime entry points (`swift_allocObject`, `swift_release`, etc.)

---

### Eliminate Host ObjC Fallback (Complete Self-Hosting)

**Current state**: The runtime is already self-hosted for all ObjC dispatch and
Foundation — but the `host_fallback` flag in `RuntimeConfig` still exists and
can be enabled. Some edge-case tests rely on it.

**Goal**: Remove the flag entirely from the default path. All resolution goes
through our shim table → registry → deny (no host `dlsym` escape hatch).

**Work**:
- [ ] Audit all remaining `host_fallback = true` usages in the test suite
- [ ] Implement missing shims that currently rely on host fallback
- [ ] Remove `host_fallback` option from `RuntimeConfig` public API
- [ ] Verify all 84 compat-scored fixtures and 30 corpus tools still pass

---

### E3d: JIT / AOT Translation

**What it is**: A translation layer that compiles ARM64 basic blocks to host
machine code (x86_64 initially) at runtime — converting the interpreter from an
instruction-at-a-time fetch-decode-execute loop into a code generator.

**Why it matters**: The pure interpreter runs at ~12–50 microseconds per fixture for small
programs. For longer-running programs and loops, interpretation overhead becomes
prohibitive. JIT/AOT is required for practical performance on x86_64 hosts.

**Planned architecture**:
- [ ] Basic block detection (BR/BL/B.cond as block terminators)
- [ ] ARM64 → x86_64 instruction lowering for the decoded instruction set
- [ ] Code cache: translated blocks keyed by source address; invalidation on `mprotect`
- [ ] FFI bridge integration: JIT blocks call through `call_host_capturing`
- [ ] Benchmark target: <=10x overhead vs. native ARM64 execution for loop-heavy workloads

---

### Foundation Depth: NSRunLoop and Concurrency

Real apps require an event loop that can process timers, I/O, and
dispatch queue callbacks concurrently. The current single-threaded stub is
insufficient for most GUI apps.

**Work**:
- [ ] `NSRunLoop` with real timer firing via `kevent` / `epoll`
- [ ] `dispatch_async` / `dispatch_after` with a real thread pool (or at minimum a serial queue)
- [ ] `NSOperationQueue` actual multi-operation scheduling
- [ ] Thread safety audit for the ObjC runtime class table and shim table

---

## Long-Term Vision

### Full macOS Binary Compatibility

The project's north star: executing any ARM64 macOS binary that doesn't rely on
GPU compositing (CoreAnimation/Metal) or hardware-specific entitlements.

This requires:
- NIB / XIB loading (XML + binary format parsing into view instantiation)
- CoreGraphics basic 2D rendering API compatibility layer (over `wgpu`)
- NSBundle plugin/framework loading
- Mach IPC stubs for XPC-based services
- A large-scale Foundation API completion push (500+ selectors remaining in
  system frameworks beyond what is already shimmed)

---

### Bare-Metal and Embedded Deployment

Run macOS ARM64 binaries on ARM64 hardware without any OS at all — or on
Linux-based ARM SBCs — using the PAL's `MemoryMapper` and the interpreter's
guest memory model as the complete execution environment.

Requirements:
- `no_std` compatible core (loader + interpreter + ObjC runtime)
- PAL implementation targeting a bare-metal or RTOS environment
- Bootloader that sets up the initial memory map for the Mach-O binary

---

### Package Ecosystem Expansion

Make the runtime easily consumable from the broader ecosystem:

- `darwin-runtime-ffi` Python bindings (via PyO3 or `ctypes`)
- CocoaPods-style spec for embedding the C dylib in iOS/macOS projects
- Nix flake for reproducible builds
- Published crates on crates.io (the stable loader core is already freeze-stable)

---

## Milestone Summary

| Horizon | Milestone | Key Deliverable |
|---|---|---|
| Near-term | Linux CI | Green `aarch64-unknown-linux-gnu` CI job |
| Near-term | NSURLSession | HTTP client — closes the networking gap |
| Near-term | NSKeyedArchiver | Binary serialization — closes the archiving gap |
| Near-term | NSStream + NSXMLParser | Closes all 6 remaining C9 gaps |
| Medium-term | T48 | First unmodified real-world binary passes |
| Medium-term | Eliminate host fallback | Fully self-contained runtime |
| Medium-term | E3d JIT | Practical performance on x86_64 |
| Medium-term | NSRunLoop + concurrency | Real event loop for GUI apps |
| Long-term | Full binary compat | Any macOS ARM64 CLI binary |
| Long-term | Bare-metal | Runs without any OS |
| Long-term | Package ecosystem | Python, Nix, crates.io |
