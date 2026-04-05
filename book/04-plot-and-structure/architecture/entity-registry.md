# Entity Registry

**Phase:** 04 - Plot and Structure
**Purpose:** Canonical registry of all entities referenced across scenes
**Status:** Filled

---

## Crate Entities (Code Artifacts)

These are the "characters" of the book—the crates and modules the reader
builds understanding of chapter by chapter.

| Entity ID | Name | Introduced | Role |
|-----------|------|-----------|------|
| `crate:macho-parse` | macho-parse | Ch 1 | Mach-O binary parser |
| `crate:macho-types` | macho-types | Ch 1 | Shared type definitions |
| `crate:macho-exec` | macho-exec | Ch 2 | Memory mapping, fixups, execution |
| `crate:macho-resolve` | macho-resolve | Ch 3 | Symbol resolution |
| `crate:macho-shared-cache` | macho-shared-cache | Ch 3 | dyld shared cache reader |
| `crate:macho-objc` | macho-objc | Ch 4 | ObjC metadata parser |
| `crate:objc-runtime` | objc-runtime | Ch 4 | Synthetic Objective-C runtime |
| `crate:cf-lite` | cf-lite | Ch 5 | Foundation framework shims |
| `crate:core-foundation-lite` | core-foundation-lite | Ch 5 | CoreFoundation shims |
| `crate:libsystem-shims` | libsystem-shims | Ch 6, 8 | System library compatibility |
| `crate:arm64-translate` | arm64-translate | Ch 7 | ARM64 interpreter |
| `crate:platform-core` | platform-core | Ch 8 | Platform abstraction traits |
| `crate:platform-darwin-host` | platform-darwin-host | Ch 8 | macOS PAL implementation |
| `crate:platform-linux` | platform-linux | Ch 6 | Linux PAL implementation |
| `crate:macho-runtime` | macho-runtime | Ch 8 | Typestate loader pipeline |
| `crate:macho-inspect` | macho-inspect | Ch 9 | The executable: loads and runs binaries |
| `crate:app-kit-lite` | app-kit-lite | Ch 10 | AppKit shims |
| `crate:test-harness` | test-harness | Ch 9 | Corpus test framework |

---

## Concept Entities (Technical Concepts)

| Entity ID | Name | Introduced | Recurs In |
|-----------|------|-----------|-----------|
| `concept:mach-o` | Mach-O format | Ch 1 | Ch 2, 3, 9 |
| `concept:load-command` | Load commands | Ch 1 | Ch 2, 3 |
| `concept:virtual-memory` | Virtual memory | Ch 2 | Ch 3, 7, 8 |
| `concept:aslr` | ASLR / slide | Ch 2 | Ch 3, 6 |
| `concept:dynamic-linking` | Dynamic linking | Ch 3 | Ch 6, 9 |
| `concept:chained-fixups` | Chained fixup format | Ch 3 | Ch 6 |
| `concept:pac` | Pointer Authentication | Ch 3 | Ch 6, 7 |
| `concept:isa-pointer` | ISA pointer | Ch 4 | Ch 5, 6 |
| `concept:objc-msgsend` | objc_msgSend dispatch | Ch 4 | Ch 5, 9 |
| `concept:tagged-pointer` | Tagged pointers | Ch 4 | Ch 5 |
| `concept:toll-free-bridge` | Toll-free bridging | Ch 5 | Ch 9 |
| `concept:variadic-abi` | Variadic ABI mismatch | Ch 6 | Ch 9 |
| `concept:arm64-isa` | ARM64 instruction set | Ch 7 | — |
| `concept:ffi-boundary` | FFI boundary | Ch 7 | Ch 8 |
| `concept:typestate` | Typestate pattern | Ch 8 | — |
| `concept:compact-unwind` | Compact unwind info | Ch 8 | — |
| `concept:responder-chain` | Responder chain | Ch 10 | — |
| `concept:parity-map` | Stack parity map | App A | App B |
| `concept:undocumented` | Undocumented behaviors | App B | Ch 6, 9 |

---

## Tool Entities (Real Binaries)

| Entity ID | Name | Category | Appears In |
|-----------|------|----------|-----------|
| `tool:lipo` | lipo | Xcode toolchain | Ch 9 (easy win) |
| `tool:nm` | nm | Xcode toolchain | Ch 1, 9 |
| `tool:strings` | strings | Xcode toolchain | Ch 9 |
| `tool:xcodebuild` | xcodebuild | Xcode dev | Ch 9 (hard case) |
| `tool:swift-demangle` | swift-demangle | Xcode toolchain | Ch 9 |
| `tool:notarytool` | notarytool | Xcode dev | Ch 9 (unsolved) |
| `tool:cut` | cut | System | Ch 6, 9 |
| `tool:sort` | sort | System | Ch 6, 9 |

---

## Setting Entities (Environments)

| Entity ID | Name | Description |
|-----------|------|-------------|
| `setting:hex-editor` | Hex editor view | Binary inspection, raw bytes |
| `setting:terminal` | Terminal / shell | Running tools, observing output |
| `setting:debugger` | Crash/register dump | SIGSEGV diagnosis, PC/LR/x16 |
| `setting:ide` | VS Code / editor | Reading and writing Rust source |
| `setting:docker` | Docker container | Linux ARM64 environment |
| `setting:source` | Source code listing | Annotated Rust code walkthrough |

---

## File Cross-Reference

| Entity | Primary Source File(s) |
|--------|----------------------|
| `crate:macho-parse` | crates/macho-parse/src/lib.rs |
| `crate:macho-exec` | crates/macho-exec/src/mapped.rs, fixup/ |
| `crate:objc-runtime` | crates/objc-runtime/src/class/, shims/ |
| `crate:cf-lite` | crates/cf-lite/src/ (130+ files) |
| `crate:arm64-translate` | crates/arm64-translate/src/ |
| `crate:libsystem-shims` | crates/libsystem-shims/src/ |
| `crate:macho-runtime` | crates/macho-runtime/src/typestate/ |
| `crate:macho-inspect` | crates/macho-inspect/src/exec.rs |
