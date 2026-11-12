# Darwin Loader Lab — Feature Reference

> **Current state as of 2026-04-03.** This is the feature index.
> See [roadmap.md](roadmap.md) for what comes next.

---

## Project Overview

Darwin Loader Lab is a clean-room Mach-O loader and macOS userspace runtime
written in Rust. It faithfully reproduces the loading behavior of Apple's `dyld`
and implements enough of the macOS runtime environment (ObjC, Foundation,
libSystem, CoreFoundation) to execute real ARM64 macOS binaries without
requiring a macOS host.

**What it solves**: Running macOS ARM64 binaries on non-Apple hardware — Linux
ARM64 natively, and x86_64 via an interpreter — while remaining independently
verifiable against Apple tooling at every stage.

**What it is not**: A replacement for macOS, a full AppKit implementation, or a
production runtime for arbitrary macOS app bundles. It is a runtime laboratory
that prioritizes correctness, observability, and portability.

---

## Documents

| Document | Contents |
|---|---|
| [CAPABILITIES.md](CAPABILITIES.md) | All 8 subsystem capability references: Mach-O Loader, ObjC Runtime, Foundation Layer (193 classes), libsystem Shims, ARM64 Interpreter, GUI Framework, PAL, FFI & Packaging |
| [CRATE_MAP.md](CRATE_MAP.md) | All 44 workspace members organized by layer (Loader Core, Runtime, PAL, Translation, GUI, Tooling) |
| [METRICS.md](METRICS.md) | Compatibility & coverage metrics: 63/63 fixtures, 30/30 corpus tools, 5,599 tests, full corpus breakdown |
| [SECURITY.md](SECURITY.md) | 8-gate security model, `TrustPolicy` composition, platform support matrix, known limitations |
| [APPLE_STACK_COMPARISON.md](APPLE_STACK_COMPARISON.md) | Layer-by-layer Apple stack comparison, coverage summary, parity roadmap |
| [roadmap.md](roadmap.md) | Near-term milestones, medium-term goals, long-term vision |
| [SEMANTIC_MEMORY_BUILD_BRIEF.md](SEMANTIC_MEMORY_BUILD_BRIEF.md) | Repo-scoped semantic memory subsystem plan: placement, crates, APIs, phases, acceptance criteria |

---

## Quick Reference

| Metric | Value |
|---|---|
| Workspace members | 44 (41 crates + 3 apps) |
| Tests passing | 5,599 (0 failing, 11 ignored) |
| Compat-scored fixtures | 63/63 |
| Corpus CLI tools | 30/30 (100%) |
| Registered ObjC classes | 193 (148 NS* + 40 AppKit + misc) |
| Registered selectors | 1,380 (867 Foundation + 513 AppKit) |
| cf-lite shim functions | 2,001 (2,538 tests) |
| Interpreted fixtures | 22/22 (macOS ARM64 host) |
| PAL implementations | macOS + Linux (both complete) |
| GUI controls | 13 (244 tests) |
| GUI apps | 3 |
| Rust LOC | ~186,000 + 620 asm |
