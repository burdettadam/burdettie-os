# Darwin Loader Lab — Feature Reference

> **Current state as of 2026-03-28.** This is the feature index.
> See [ROADMAP.md](ROADMAP.md) for what comes next.

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
| [CAPABILITIES.md](CAPABILITIES.md) | All 8 subsystem capability references: Mach-O Loader, ObjC Runtime, Foundation Layer (71 classes), libsystem Shims, ARM64 Interpreter, GUI Framework, PAL, FFI & Packaging |
| [CRATE_MAP.md](CRATE_MAP.md) | All 35 workspace members organized by layer (Loader Core, Runtime, PAL, Translation, GUI, Tooling) |
| [METRICS.md](METRICS.md) | Compatibility & coverage metrics: 63/63 fixtures, 30/30 corpus tools, 1090+ tests, full corpus breakdown |
| [SECURITY.md](SECURITY.md) | 8-gate security model, `TrustPolicy` composition, platform support matrix, known limitations |
| [ROADMAP.md](ROADMAP.md) | Near-term milestones, medium-term goals, long-term vision |

---

## Quick Reference

| Metric | Value |
|---|---|
| Workspace members | 35 (32 crates + 3 apps) |
| Tests passing | 1,090+ (0 failing) |
| Compat-scored fixtures | 63/63 |
| Corpus CLI tools | 30/30 (100%) |
| Foundation classes | 71 shimmed (52 substantive) |
| Shim registrations | 340+ |
| Foundation selectors | 523+ |
| Interpreted fixtures | 22/22 (macOS ARM64 host) |
| PAL implementations | macOS + Linux (both complete) |
| GUI controls | 7 |
| GUI apps | 3 |
| Rust LOC | ~70,000 + 620 asm |
