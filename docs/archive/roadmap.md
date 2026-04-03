<--- ARCHIVED 2026-03-28 — superseded by FEATURES.md and ROADMAP.md -->

# Darwin Loader Lab — Strategic Roadmap

> **As of 2026-03-28.** This document tracks where the project is headed.
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

### 1. Linux End-to-End CI (E4 completion)

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

### 2. NSURLSession

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

### 3. NSKeyedArchiver / NSKeyedUnarchiver

**What it is**: Binary object graph serialization. Used by Xcode build caches,
user preferences, and any tool that persists complex state.

**Scope**:
- [ ] `NSKeyedArchiver` — `archivedDataWithRootObject:`, `archiveRootObject:toFile:`
- [ ] `NSKeyedUnarchiver` — `unarchiveObjectWithData:`, `unarchiveObjectWithFile:`
- [ ] `NSCoding` protocol support — `encodeWithCoder:`, `initWithCoder:` dispatch
- [ ] Basic type encoding: NSString, NSNumber, NSArray, NSDictionary, NSData, NSURL
- [ ] `NSCoder` abstract class implementation (required by NSKeyedArchiver)

---

### 4. Remaining Foundation Gaps

Close the six open API gaps from Phase C9 in priority order:

| API | Work |
|---|---|
| NSStream / NSInputStream / NSOutputStream | POSIX fd-backed stream implementation |
| NSRunLoop selectors (`run`, `runUntilDate:`, `runMode:beforeDate:`) | Wire into `runloop-core`; single-threaded event loop execution |
| NSXMLParser | Expat-backed or pure-Rust SAX-style XML parser |

---

## Medium-Term Goals *(6–18 months)*

### T48: Run an Unmodified Real-World ARM64 Binary

**What it is**: Execute a small, unmodified ARM64 macOS CLI binary from the wild
— not a source-compiled fixture — successfully through the runtime.

**Why it matters**: The entire project has been validated against fixtures we
compiled ourselves. T48 is the first test against unknown external code, which
will surface every remaining gap in symbol coverage, shim correctness, and
loader accuracy.

**Prerequisites**: NSURLSession, NSKeyedArchiver, and the remaining Foundation
gaps above (tools with external dependencies need networking and serialization).

**Candidate binaries**: Simple open-source CLI tools (e.g., `jq` ObjC bridges,
`mas`, `xcinfo`, small scripting utilities) compiled without Xcode-specific
features.

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
- [ ] Verify all 63 compat-scored fixtures and 30 corpus tools still pass

---

### E3d: JIT / AOT Translation

**What it is**: A translation layer that compiles ARM64 basic blocks to host
machine code (x86_64 initially) at runtime — converting the interpreter from an
instruction-at-a-time fetch-decode-execute loop into a code generator.

**Why it matters**: The pure interpreter runs at ~12–50µs per fixture for small
programs. For longer-running programs and loops, interpretation overhead becomes
prohibitive. JIT/AOT is required for practical performance on x86_64 hosts.

**Planned architecture** (basic block translation, no self-modifying code
support initially):
- [ ] Basic block detection (BR/BL/B.cond as block terminators)
- [ ] ARM64 → x86_64 instruction lowering for the same decoded instruction set as the interpreter
- [ ] Code cache: translated blocks keyed by source address; invalidation on `mprotect`
- [ ] FFI bridge integration: JIT blocks call through the same `call_host_capturing` path as the interpreter
- [ ] Benchmark: target ≤10× overhead vs. native ARM64 execution for loop-heavy workloads

---

### Foundation Depth: NSRunLoop & Concurrency

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
- NIB / XIB loading (XML + binary format parsing → view instantiation)
- CoreGraphics basic 2D rendering API compatibility layer (over `wgpu`)
- NSBundle plugin/framework loading
- Mach IPC stubs for XPC-based services
- A large-scale Foundation API completion push (500+ selectors remaining in
  system frameworks beyond what's already shimmed)

---

### Bare-Metal & Embedded Deployment

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
