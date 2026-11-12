# Changelog

All notable changes to burdettieOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Align Foundation class storage and restrict Objective-C superclass repair to
  registered mapped class records, avoiding reads through synthetic handles.
- Distinguish pointer-backed CFStrings from legacy inline ASCII payloads so
  printable bytes in a heap address cannot be returned as string contents.
- Use the host page size and correctly sized residency buffers for memory checks;
  reject overflowing ranges and C-string address arithmetic.
- Correct ARM64 atomic decoding, widths, signed comparisons, alignment checks,
  acquire/release flags, and exclusive-monitor handling.
- Bound malformed Mach-O export-trie traversal and integer/range parsing, while
  retaining valid shared trie paths and architecture-specific thread entry points.
- Preserve attributed-text styles through normalization and distinguish font
  faces and fractional sizes in glyph caches; make atlas placement transactional.
- Close both DNS service pipe descriptors when a service is released and invoke
  callbacks outside the registry lock so callbacks can query or release services.
- Release timer-queue and Objective-C runtime locks before invoking timer
  callbacks, allowing callbacks to enqueue timers or register runtime metadata;
  preserve timer order and reset/re-enqueue behavior.
- Return NUL-terminated launchd error strings and make raw-pointer and variadic
  Rust API contracts explicit.
- Use C-compatible geometry, range, and transform return types for the affected
  Objective-C methods, including aggregate Core Animation transform arguments.
- Repair invalid Foundation test objects, UI test imports, and process/global
  state isolation; resolve compiler and Clippy diagnostics without weakening CI
  policy.

### Changed

- Document the public source/artifact boundary, source-built quick start, and
  revision-specific validation results in the portfolio README and publication guide.

## [0.1.0] - 2026-03-31

### Added

- 33-crate workspace for loading, shimming, and executing ARM64 Mach-O binaries
- `macho-parse` — zero-copy Mach-O parser (load commands, segments, sections,
  fixup chains, ObjC metadata, code signature)
- `macho-exec` — memory mapper, fixup engine, TLV support, unwind registration
- `macho-runtime` — typestate loader pipeline with 7 phases (Parsed → Initialized)
- `objc-runtime` — synthetic ObjC runtime: class/protocol/method registration,
  `objc_msgSend` trampoline, dynamic class creation, KVO ISA swizzle
- `cf-lite` — 71 Foundation + CoreFoundation classes shimmed in Rust
  (NSString, NSArray, NSDictionary, NSDate, NSURLSession, NSTask, …)
- `libsystem-shims` — controlled shims for libc, dispatch, and Mach system calls
- `arm64-translate` — ARM64 interpreter (~122 instruction variants) with
  auto-FFI fallback for unmapped branch targets
- `platform-core` — 10 platform abstraction traits (MemoryMapper,
  HostSymbolResolver, Threading, Clock, …)
- `platform-darwin-host` — macOS implementation of platform traits
- `platform-linux` — Linux/ARM64 cross-compile stub
- `graphics-core` — 2D drawing primitives, SoftwareRenderer, BezierPath, GPU
  renderer (wgpu, behind `gpu` feature)
- `window-backend` / `window-backend-darwin` — windowing abstraction + winit backend
- `app-kit-lite`, `ui-view`, `ui-controls` — GUI widget framework
- `runtime-diagnostics` — DiagRing circular event buffer with per-category counters
- `trust-policy` — 8-gate security model for binary execution
- `darwin-runtime-ffi` — C-ABI `extern "C"` wrapper for embedding
- 63 ARM64 fixture binaries + 39 corpus tools (100% pass rate)
- Docker images for dev (`Dockerfile.dev`) and demo (`Dockerfile.demo`)
- CI pipeline: build, test, clippy, fmt, cargo-audit, cargo-deny, FFI interop
- VM image scripts for macOS Virtualization.framework
- Homebrew formula (`Formula/darwin-runtime.rb`)

[Unreleased]: https://github.com/burdettadam/burdettie-os/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/burdettadam/burdettie-os/releases/tag/v0.1.0
