# Darwin Loader Lab — Strategic Roadmap

> **As of 2026-03-28.** This document tracks where the project is headed.
> For a complete reference of what works today, see [FEATURES.md](FEATURES.md).

---

## Publication Readiness: Font Dependency Maintenance

**Checked 2026-11-12.** Dependency updates and the YAML parser migration have
removed the reported vulnerability findings from `cargo audit`. The full
dependency policy gate still fails: `cargo deny check` rejects the unmaintained
`ttf-parser` dependency under
[RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html).
The advisory has no patched version and recommends the maintained `skrifa`
parser. The advisory is not suppressed; CI should remain blocked on this policy
until the affected font dependencies are replaced.

The dependency reaches rendering through both `ab_glyph` / `owned_ttf_parser`
and the `cosmic-text` / `fontdb` stack. Replacing it requires a focused rendering
migration, beyond dependency version updates. The existing font APIs supply
glyph metrics, outlines, and rasterization to the CPU renderer and GPU atlas.

- [ ] Select maintained font parsing and rasterization dependencies, evaluating
  `skrifa` and the Fontations ecosystem.
- [ ] Migrate both dependency paths without suppressing the advisory or vendoring
  an unchanged abandoned parser.
- [ ] Verify glyph metrics, shaping, text layout, and CPU/GPU rendering against
  the bundled fonts and representative Unicode text.
- [ ] Pass `cargo audit` and the complete `cargo deny check` policy gate, then
  update the documented CI status.

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
- [x] `NSURLSession` — `sharedSession`, `dataTaskWithURL:completionHandler:`, `dataTaskWithRequest:completionHandler:`
- [x] `NSURLSessionDataTask` — `resume` (real synchronous HTTP/HTTPS via ureq), `cancel`, `state`, `response`, `error`
- [x] `NSURLSessionConfiguration` — `defaultSessionConfiguration`, `ephemeralSessionConfiguration`
- [x] Synchronous HTTP/HTTPS path via ureq (rustls-backed, no async runtime); completionHandler called with real NSData + NSHTTPURLResponse
- [x] HTTP request body / POST data forwarding through completionHandler (`setHTTPBody:` + `HTTPBody` on NSMutableURLRequest; body bytes passed to ureq `.send()`)
- [x] `NSURLCredential`, `NSURLCredentialStorage` (singleton stub), `NSHTTPCookie`, `NSHTTPCookieStorage` (singleton stub with empty cookies/credentials)

---

### 3. NSKeyedArchiver / NSKeyedUnarchiver

**What it is**: Binary object graph serialization. Used by Xcode build caches,
user preferences, and any tool that persists complex state.

**Scope**:
- [x] `NSKeyedArchiver` — `archivedDataWithRootObject:`, `archiveRootObject:toFile:`
- [x] `NSKeyedUnarchiver` — `unarchiveObjectWithData:`, `unarchiveObjectWithFile:`, `unarchivedObjectOfClass:fromData:error:`
- [x] `NSCoding` protocol support — `encodeWithCoder:`, `initWithCoder:` dispatch
- [x] Basic type encoding: NSString, NSNumber, NSArray, NSDictionary, NSData, NSURL
- [x] `NSCoder` abstract class implementation (allowsKeyedCoding, description)

---

### 4. Remaining Foundation Gaps

Close the six open API gaps from Phase C9 in priority order:

| API | Work | Status |
|---|---|---|
| NSStream / NSInputStream / NSOutputStream | Memory-backed in-process streams; `read:maxLength:` needs proper read-cursor tracking | 🟡 Partially done — cursor bug fixed |
| NSRunLoop selectors (`run`, `runUntilDate:`, `runMode:beforeDate:`) | Wire into `runloop-core`; fire pending timers on each iteration | 🟡 Partially done — now fires due timers |
| NSXMLParser | Pure-Rust SAX tokenizer; delegate callbacks now dispatch via `objc_msg_lookup_imp` | ✅ Done |
| NSURLSession + completionHandler variants | `dataTaskWithURL:completionHandler:` / `dataTaskWithRequest:completionHandler:` + real HTTP via ureq | ✅ Done |
| NSKeyedArchiver `encodeObject:forKey:` / `decodeObjectForKey:` | Key-value encode/decode table backed by global store | ✅ Done |
| NSCoder (protocol) | Foundation archiving base — required for NSKeyedArchiver hierarchy | ✅ Done |

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

### Xcode Command Line Tools Compatibility

**What it is**: Execute the tools that ship with `xcode-select --install` —
`xcrun`, `xcodebuild`, `clang`, `actool`, and related developer utilities —
unmodified under the runtime.

**Why it matters**: Xcode command line tools are a dense concentration of
real-world Foundation and CoreFoundation usage. They exercise `NSTask`/`NSProcess`
(subprocess launching), `NSPropertyList` (xcconfig/plist parsing), `NSXMLParser`
(project files), `NSFileManager` (SDK path traversal), and `NSBundle` (framework
introspection) simultaneously. Success here validates the runtime against a
widely-used, Apple-maintained binary corpus.

**Prerequisites**: T48 (first unmodified binary), NSKeyedArchiver,
NSURLSession, NSXMLParser, NSStream.

**Key API surface required**:
- [x] `NSTask` / `NSProcess` — process launch, argument passing, environment
  inheritance, `terminationStatus`, `waitUntilExit`; `setStandardOutput/Input/Error:`,
  `launchAndReturnError:`, `launchedTaskWithLaunchPath:arguments:`, `processIdentifier`,
  `isRunning`; pipe write-end closed in parent after spawn (EOF fix)
- [x] `NSPipe` / `NSFileHandle` — subprocess stdio capture;
  `readDataToEndOfFile`, `writeData:`, `closeFile`, `fileDescriptor`,
  `fileHandleForReading/Writing`, `fileHandleWithStandardOutput/Input/Error`
- [x] `NSPropertyList` — `.plist` read via `NSPropertyListSerialization`
  (`propertyListWithData:options:format:error:`, `parse_plist_file`); XML + binary formats
- [x] `NSBundle` — `mainBundle`, `bundleWithPath:`, `infoDictionary`,
  `objectForInfoDictionaryKey:`, `pathForResource:ofType:`, resource lookup
- [x] `DEVELOPER_DIR` / `SDKROOT` environment variable plumbing through
  `libsystem-shims` (`getenv` falls through to host libc, `setenv`/`putenv`/`unsetenv` added)
- [x] `xcrun`-style tool-path resolution (path table or shim covering the
  common `xcrun --find <tool>` contract); `xcrun_query` corpus tool exercises
  `xcrun --show-sdk-path`, `xcrun --find clang`, `xcrun clang --version`
- [x] Fat binary tool dispatch: `parse_auto` / `fat_slice_for_cpu` selects ARM64 slice from universal binaries; `runtime_fat_binary` end-to-end test in `runtime_exec_tests.rs`

**Exit criterion**: `xcrun --show-sdk-path` and `xcrun clang --version`
produce correct output under the runtime without host ObjC fallback.

---

### Eliminate Host ObjC Fallback (Complete Self-Hosting)

**Current state**: ✅ **COMPLETE** — `host_fallback` field removed from `RuntimeConfig`.
The safe-symbol `StubResolver` (C stdlib passthrough) is always active. All ObjC
dispatch, Foundation, and system shims go through our own shim table. No escape hatch.

**Work**:
- [x] Audit all remaining `host_fallback = true` usages in the test suite
- [x] Implement missing shims that currently rely on host fallback
- [x] Remove `host_fallback` option from `RuntimeConfig` public API
- [x] Verify all 63 compat-scored fixtures and 30 corpus tools still pass

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
- Broader NIB / XIB compatibility, especially build-time XIB compilation and
  higher-fidelity runtime view instantiation
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
| Medium-term | Xcode CLI tools | `xcrun`/`clang`/`xcodebuild` run without host fallback |
| Medium-term | Eliminate host fallback | Fully self-contained runtime |
| Medium-term | E3d JIT | Practical performance on x86_64 |
| Medium-term | NSRunLoop + concurrency | Real event loop for GUI apps |
| Long-term | Full binary compat | Any macOS ARM64 CLI binary |
| Long-term | Bare-metal | Runs without any OS |
| Long-term | Package ecosystem | Python, Nix, crates.io |

---

## Known Limitations

> Updated 2026-03-29. These are the current hard boundaries of the runtime.

| # | Limitation | Severity | Remedy |
|---|---|---|---|
| L1 | **NSURLSession — no real HTTP** | Blocking | `resume` fires completionHandler with stub 200 OK response (nil data); synchronous POSIX socket GET required |
| L2 | **NSKeyedArchiver / NSCoder semantic coverage incomplete** | Medium | Apple keyed-archive decode and plist-backed archive support exist; broaden `NSCoder` protocol parity and archive semantics for wider compatibility |
| L3 | **NSCoder protocol — basic stubs only** | Medium | `NSCoder` registered as base class; `NSKeyedArchiver`/`NSKeyedUnarchiver` now inherit from it; full abstract protocol delegation unimplemented |
| L4 | **NSRunLoop — single-pass timer firing, no I/O sources** | High | `run`/`addTimer:forMode:` now fire due timers once; `kevent`/`epoll` I/O source registration unimplemented |
| L5 | **No external binary tested (T48)** | Blocking | All 63 fixtures compiled in-house; first wild binary will expose unknown selector gaps |
| L6 | **Threading completely single-threaded** | High | `pthread_create`, `NSThread`, `dispatch_async` are stubs — concurrent programs serialize silently |
| L7 | **No build-time NIB / XIB compilation** | High | Runtime loading for compiled nib archives exists; XIB compilation and wider Interface Builder compatibility remain incomplete |
| L8 | **ARM64 interpreter has no JIT** | High | Pure fetch-decode-execute loop; prohibitive for loop-heavy real-world binaries; E3d planned |
| L9 | **Security gates are all permissive mocks** | Low (lab) | All 8 `TrustPolicy` gates log but enforce nothing — intentional for laboratory use |
| L10 | ~~**Host ObjC fallback escape hatch**~~ | ✅ Fixed | `host_fallback` removed from `RuntimeConfig`; StubResolver (safe C stdlib) always active; no ObjC escape hatch |
| L11 | **POSIX socket syscalls intercepted, deny-by-default** | Low | All 12 BSD socket calls return EOPNOTSUPP via shim table; accounting in place for future PAL routing |
| L12 | **Tagged pointer / PAC boundary** | Low | `cf-lite` ISAs incompatible with ARM64 non-pointer ISA masking; cannot pass cf-lite objects to host `objc_msgSend` |
| L13 | **Linux CI not wired** | Medium | Docker image exists; CI matrix not connected; Linux x86_64 requires E3d JIT (unstarted) |

