# Changelog

All notable changes to burdettieOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
