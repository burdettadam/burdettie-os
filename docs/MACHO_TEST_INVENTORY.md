# Mach-O Test Suite Inventory

> **Purpose**: Comprehensive catalog of publicly available test suites and test cases for Mach-O binary parsing and dynamic loading, mapped to `burdettie-os` crates for porting to Rust.
>
> **Scope**: Research only — no code produced. Each entry identifies the source project, file path/URL, what the test validates, expected behavior, fixture requirements, and target crate.

---

## Table of Contents

1. [Source Summary](#1-source-summary)
2. [LLVM lld MachO Tests](#2-llvm-lld-macho-tests)
3. [LLVM llvm-objdump MachO Tests](#3-llvm-llvm-objdump-macho-tests)
4. [Goblin Crate Tests](#4-goblin-crate-tests)
5. [Apple dyld Source-Derived Test Cases](#5-apple-dyld-source-derived-test-cases)
6. [XNU mach_loader Validation Cases](#6-xnu-mach_loader-validation-cases)
7. [Apple cctools Tests](#7-apple-cctools-tests)
8. [Darling Project Tests](#8-darling-project-tests)
9. [LLVM BinaryFormat Unit Tests](#9-llvm-binaryformat-unit-tests)
10. [Spec/RFC Conformance](#10-specrfc-conformance)
11. [Crate Mapping Summary](#11-crate-mapping-summary)
12. [Fixture Strategy](#12-fixture-strategy)
13. [Priority Tiers](#13-priority-tiers)

---

## 1. Source Summary

| Source | Language | Test Count | Test Style | Richness |
|--------|----------|-----------|------------|----------|
| LLVM lld `lld/test/MachO/` | Assembly + FileCheck | ~200+ files | Integration: assemble → link → verify | ★★★★★ |
| LLVM llvm-objdump `llvm/test/tools/llvm-objdump/MachO/` | YAML + FileCheck | ~60+ files | Integration: dump/inspect → verify | ★★★★☆ |
| Goblin `tests/macho.rs` | Rust | ~10 tests | Unit: parse fixture bytes → assert fields | ★★★☆☆ |
| Apple dyld source | C++ (behavioral) | ~30 extractable cases | No public test suite; behaviors extracted from source | ★★★★☆ |
| XNU `bsd/kern/mach_loader.c` | C (kernel) | ~15 extractable cases | Validation logic in kernel loader | ★★★☆☆ |
| Apple cctools `tests/`, `libstuff_test/` | C/shell | Unknown (directory exists) | Build-system tests for otool/lipo/etc | ★★☆☆☆ |
| Darling `src/startup/mldr/` | C | ~5 relevant cases | Loader implementation + runtime tests | ★★☆☆☆ |
| LLVM BinaryFormat unit tests | C++ (gtest) | ~2 tests | Unit: validate struct layout, unaligned access | ★★☆☆☆ |

---

## 2. LLVM lld MachO Tests

**Source**: `https://github.com/llvm/llvm-project/tree/main/lld/test/MachO/`
**Methodology**: Assemble with `llvm-mc -filetype=obj`, link with `%lld`, verify output with `FileCheck` + `llvm-objdump --macho`.

### 2.1 Header & Load Commands

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `load-commands.s` | Correct LC_MAIN, LC_LOAD_DYLINKER presence in executables; filetype EXECUTE vs DYLIB vs BUNDLE; magic MH_MAGIC_64 | `macho-types`, `macho-parse` | P0 |
| `load-command-sequence.s` | Load command ordering in output binary | `macho-parse` | P1 |
| `section-headers.s` | Section header fields: sectname, segname, addr, size, offset, align, flags | `macho-types`, `macho-parse` | P0 |
| `segments.s` | Segment layout: vmaddr, vmsize, fileoff, filesize, maxprot, initprot | `macho-types`, `macho-map` | P0 |

**Fixture Requirements**: Generated via `llvm-mc` + `ld64.lld`. For Rust tests, need pre-built `.o` and linked binaries, or construct bytes programmatically.

### 2.2 Segments & Memory Mapping

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `pagezero.s` | `__PAGEZERO` segment presence, vmaddr=0, vmsize=0x100000000 for 64-bit | `macho-map` | P0 |
| `segment-order.s` | Segment ordering: `__PAGEZERO` → `__TEXT` → `__DATA_CONST` → `__DATA` → `__LINKEDIT` | `macho-map` | P0 |
| `segment-boundary.s` | Page-aligned segment boundaries | `macho-map` | P1 |
| `arm64-32-stubs.s` | Segment layout for arm64_32 (4GB address space) | `macho-map` | P2 |

### 2.3 Symbol Resolution & Binding

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `symbol-resolution.s` | Two-level namespace symbol resolution | `macho-resolve` | P0 |
| `symbol-resolution-error.ll` | Error on undefined symbol in two-level namespace | `macho-resolve` | P1 |
| `flat-namespace.s` | Flat namespace binding: all symbols searched in load order | `macho-resolve` | P1 |
| `flat-namespace-interposable.s` | Flat namespace symbols are interposable; generates weak bind table entries | `macho-resolve` | P1 |
| `bind-opcodes.s` | BIND_OPCODE_* encoding/decoding: SET_DYLIB_ORDINAL, SET_SYMBOL, SET_TYPE, SET_ADDEND, DO_BIND | `macho-resolve` | P0 |
| `weak-binding.s` | Weak symbol binding: first definition wins | `macho-resolve` | P1 |
| `weak-definition.s` | `N_WEAK_DEF` flag handling | `macho-resolve` | P1 |
| `weak-reference.s` | `N_WEAK_REF` flag: symbol can be NULL at runtime | `macho-resolve` | P1 |

### 2.4 Export Trie

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `export-trie.s` | Export trie encoding: addresses relative to image base with `__PAGEZERO` present | `macho-resolve` | P0 |
| `export-trie-order.s` | Deterministic ordering of exports in trie | `macho-resolve` | P1 |
| `re-export-chain.s` | Re-export chains through multiple dylibs | `macho-resolve` | P1 |

### 2.5 Dylib Dependencies & Path Resolution

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `dylib.s` | LC_LOAD_DYLIB generation, install_name, current_version, compat_version | `macho-resolve` | P0 |
| `link-search-at-rpath.s` / `rpath.s` | `@rpath` resolution via LC_RPATH entries | `macho-resolve` | P0 |
| `link-search-at-loader-path.s` | `@loader_path` expansion relative to loading binary | `macho-resolve` | P0 |
| `link-search-at-executable-path.s` | `@executable_path` expansion relative to main executable | `macho-resolve` | P0 |
| `reexport-stub.s` / `reexport-dylib.s` | LC_REEXPORT_DYLIB handling | `macho-resolve` | P1 |
| `weak-dysym.s` | LC_LOAD_WEAK_DYLIB: dylib may be absent at runtime | `macho-resolve` | P1 |
| `upward-dylib.s` | LC_LOAD_UPWARD_DYLIB handling for circular dependencies | `macho-resolve` | P2 |
| `dylib-compatibility-version.s` | Compatibility version checking | `macho-resolve` | P2 |

### 2.6 Chained Fixups

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `chained-fixups.s` | LC_DYLD_CHAINED_FIXUPS basic encoding | `macho-resolve` | P1 |
| `chained-fixups-empty.s` | LC_DYLD_CHAINED_FIXUPS with no fixups (48-byte minimum header) | `macho-resolve` | P1 |
| `chained-fixups-header.s` | Chained fixups header validation | `macho-resolve` | P1 |

### 2.7 Dead Stripping & Optimization

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `dead-strip.s` | Dead stripping removes unused symbols; `__PAGEZERO` preserved; symbol table updated | `macho-run-plan` | P2 |
| `dead-strip-dylibs.s` | Unused dylib load commands removed | `macho-run-plan` | P2 |

### 2.8 Other Notable Tests

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `fat-binary.s` | Universal/fat binary with multiple architectures | `macho-parse` | P1 |
| `lc-build-version.s` | LC_BUILD_VERSION with platform/minos/sdk | `macho-types` | P1 |
| `objc-methname.s` | `__objc_methname` section handling | `macho-objc` | P1 |
| `objc-classrefs.s` | `__objc_classrefs` section | `macho-objc` | P1 |
| `compact-unwind.s` | Compact unwind info encoding | `macho-types` | P2 |
| `stabs.s` | STAB debug symbol entries | `macho-types` | P2 |
| `relocations-*.s` | Various relocation types (ARM64, X86_64) | `macho-types` | P1 |
| `tlv.s` | Thread-local variable sections | `macho-types` | P2 |
| `subsections-via-symbols.s` | `MH_SUBSECTIONS_VIA_SYMBOLS` flag behavior | `macho-parse` | P2 |
| `entry.s` | Entry point via LC_MAIN vs LC_UNIXTHREAD | `macho-run-plan` | P0 |
| `dyld-env.s` | LC_DYLD_ENVIRONMENT load command | `macho-resolve` | P2 |
| `uuid.s` | LC_UUID presence and format | `macho-parse` | P1 |
| `code-signature.s` | LC_CODE_SIGNATURE handling | `macho-parse` | P2 |
| `function-starts.s` | LC_FUNCTION_STARTS data | `macho-parse` | P2 |
| `data-in-code.s` | LC_DATA_IN_CODE handling | `macho-parse` | P2 |

---

## 3. LLVM llvm-objdump MachO Tests

**Source**: `https://github.com/llvm/llvm-project/tree/main/llvm/test/tools/llvm-objdump/MachO/`
**Methodology**: Use pre-built binary fixtures (in `Inputs/`) or YAML-constructed Mach-O files, dump with `llvm-objdump --macho`, verify with FileCheck.

### 3.1 Malformed Input Handling

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `malformed-machos.test` | Graceful handling of various malformed Mach-O binaries | `macho-parse` | P0 |
| `malformed.test` | Additional malformed binary tests | `macho-parse` | P0 |
| `bad-bind.test` | Invalid bind opcode data | `macho-resolve` | P1 |
| `bad-dysymtab.test` | Malformed LC_DYSYMTAB | `macho-parse` | P1 |
| `bad-ordinal.test` | Invalid library ordinal in bind info | `macho-resolve` | P1 |
| `bad-trie.test` | Malformed export trie data | `macho-resolve` | P1 |
| `truncated-section.test` | Section data extends past end of file | `macho-parse` | P0 |

### 3.2 Binding & Fixup Inspection

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `bind.test` / `bind2.test` | Bind info dumping: segment, section, address, type, addend, dylib, symbol | `macho-resolve` | P0 |
| `lazy-bind.test` | Lazy binding info format | `macho-resolve` | P1 |
| `weak-bind.test` | Weak binding table format | `macho-resolve` | P1 |
| `rebase.test` | Rebase info: segment, section, address, type | `macho-resolve` | P1 |
| `chained-fixups.test` | Chained fixups dumping | `macho-resolve` | P1 |
| `dyld-info.test` | dyld_info output format (rebase/bind/lazy_bind/export) | `macho-resolve` | P1 |

### 3.3 Export & Symbol Inspection

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `exports-trie.test` | Export trie traversal and dumping | `macho-resolve` | P0 |
| `exports-trie-lc.test` | Export trie from LC_DYLD_EXPORTS_TRIE vs LC_DYLD_INFO | `macho-resolve` | P1 |
| `export-trie-boundary.test` | Boundary conditions in export trie node reading | `macho-resolve` | P1 |
| `symbol-table.test` | Symbol table (LC_SYMTAB) dumping: n_type, n_sect, n_desc, n_value | `macho-parse` | P0 |
| `indirect-symbols.test` | Indirect symbol table entries | `macho-resolve` | P1 |
| `function-starts.test` | LC_FUNCTION_STARTS data decoding | `macho-parse` | P2 |

### 3.4 Structure Inspection

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `section-headers.test` | Section header field dumping | `macho-parse` | P0 |
| `section-contents.test` | Section data reading with segment names | `macho-parse` | P1 |
| `sections.test` | Multiple sections across segments | `macho-parse` | P1 |
| `private-header.test` / `private-headers.test` | Mach-O header + all load commands dump | `macho-parse` | P0 |
| `dylib.test` | Dylib load commands display | `macho-resolve` | P1 |
| `rpaths.test` | LC_RPATH entries listing | `macho-resolve` | P0 |
| `build-version.yaml` | LC_BUILD_VERSION YAML round-trip | `macho-types` | P1 |
| `archive-headers.test` | Archive (`.a` file) header parsing | `macho-parse` | P2 |

### 3.5 Universal Binary

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `universal-arm64.test` | Universal binary with arm64 slice | `macho-parse` | P1 |
| `universal-x86_64.i386.test` | Universal binary with x86_64 and i386 slices | `macho-parse` | P1 |
| `otool-arch-flag.test` | `-arch` flag for selecting universal binary slices | `macho-parse` | P2 |

### 3.6 ObjC Metadata

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `objc-meta-data.test` | ObjC class/method metadata section dumping | `macho-objc` | P1 |
| `macho-objc-meta-data.test` | Extended ObjC metadata inspection | `macho-objc` | P1 |

### 3.7 Unwind & Debug

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `compact-unwind-x86_64.test` | Compact unwind info for x86_64 | `macho-types` | P2 |
| `compact-unwind-i386.test` | Compact unwind info for i386 | `macho-types` | P2 |
| `unwind-info-x86_64.test` | `__unwind_info` section parsing | `macho-types` | P2 |
| `unwind-info-arm64.test` | `__unwind_info` for arm64 | `macho-types` | P2 |
| `malformed-unwind-x86_64.test` | Malformed unwind info handling | `macho-types` | P2 |
| `eh_frame-arm64.test` | `.eh_frame` PC offset calculation | `macho-types` | P2 |
| `stabs.test` | STAB entries in symbol table | `macho-parse` | P2 |

---

## 4. Goblin Crate Tests

**Source**: `https://github.com/m4b/goblin` (`tests/macho.rs` + inline tests in `src/mach/`)
**Methodology**: Parse embedded byte arrays or `include_bytes!` fixtures, assert parsed field values.

### 4.1 Core Parsing Tests

| Test Function | What It Validates | Target Crate | Priority |
|---------------|-------------------|--------------|----------|
| `parse_fat_header` (line 4) | Fat/universal binary header parsing: magic, nfat_arch, per-arch cputype/cpusubtype/offset/size/align | `macho-parse` | P0 |
| `parse_sections` (line 510) | Section enumeration from DEADBEEF_MACH_64 fixture: sectname, segname, addr, size, flags | `macho-parse` | P0 |
| `iter_symbols` (line 526) | Symbol table iteration: nlist entries with name, type, section, value | `macho-parse` | P0 |
| `relocations` (line 549) | Relocation entry parsing from object files | `macho-types` | P1 |

### 4.2 Robustness / Fuzzing Tests

| Test Function | What It Validates | Target Crate | Priority |
|---------------|-------------------|--------------|----------|
| `fat_mach_overflow` (line 582) | Fat header with absurd nfat_arch doesn't cause allocation overflow | `macho-parse` | P0 |
| `imports_overflow` (line 610) | Malformed import data doesn't cause unbounded allocation | `macho-parse` | P0 |
| `panic_load_cmd` (line 618) | Malformed load command with bad cmdsize doesn't panic | `macho-parse` | P0 |
| `invalid_sym_offset` | Invalid symoff/stroff values handled gracefully | `macho-parse` | P0 |
| `fuzzed_memory_growth` (line 644) | Fuzzed inputs don't cause unbounded memory growth | `macho-parse` | P0 |

### 4.3 Inline Unit Tests (in `src/mach/`)

| Test | Location | What It Validates | Target Crate | Priority |
|------|----------|-------------------|--------------|----------|
| `test_parse_armv7_header` | `src/mach/header.rs` | Parse 32-bit ARM Mach-O header | `macho-types` | P1 |
| `sizeof_header32` | `src/mach/header.rs` | `mach_header` struct is 28 bytes | `macho-types` | P0 |
| `sizeof_header64` | `src/mach/header.rs` | `mach_header_64` struct is 32 bytes | `macho-types` | P0 |
| `parse_multi_arch_of_macho_binaries` | `src/mach/mod.rs` | Multi-arch parsing via `include_bytes!` fixtures | `macho-parse` | P1 |
| `parse_multi_arch_of_archives` | `src/mach/mod.rs` | Archive file containing Mach-O objects | `macho-parse` | P2 |

### 4.4 Fixture Details

- **DEADBEEF_MACH_64**: 8496-byte inline constant in `tests/macho.rs` — a complete 64-bit Mach-O binary defined as a `&[u8]` array. Contains header, segments (__PAGEZERO, __TEXT, __DATA, __LINKEDIT), sections (__text, __stubs, __stub_helper, __cstring, __unwind_info, __got, __la_symbol_ptr, __data), symbol table, string table.
- **External fixtures**: Various `.o` and binary files in `tests/bins/mach/` (not enumerated here but referenced by `include_bytes!`).

---

## 5. Apple dyld Source-Derived Test Cases

**Source**: `https://github.com/apple-oss-distributions/dyld` (dyld-1340, latest tag)
**Note**: No public test suite directory exists. The `testing/` path returns 404. These test cases are **extracted from source code behavior** documented in the implementation.

### 5.1 Path Resolution (→ `macho-resolve`)

| Behavior | Source File:Line | Expected Behavior | Priority |
|----------|-----------------|-------------------|----------|
| `@executable_path` expansion | `Loader.cpp:1279` (`expandAtExecutablePath`) | Replace `@executable_path` with directory of main executable | P0 |
| `@loader_path` expansion | `Loader.cpp:1208` (`expandAtLoaderPath`) | Replace `@loader_path` with directory of binary containing the reference | P0 |
| `@rpath` expansion | `Loader.cpp:1300` (`forEachResolvedAtPathVar`) | Try each LC_RPATH from the rpath stack, expanding recursively | P0 |
| DYLD_LIBRARY_PATH override | `DyldProcessConfig.cpp:2360` (`forEachPathVariant`) | Paths in DYLD_LIBRARY_PATH searched before default locations for leaf names | P1 |
| DYLD_FALLBACK_LIBRARY_PATH | `DyldProcessConfig.cpp:2360` | Fallback paths tried after default locations | P1 |
| DYLD_FRAMEWORK_PATH | `DyldProcessConfig.cpp:2360` | Framework path override for `.framework` bundles | P2 |
| Leaf name extraction | `Loader.cpp:715` (`forEachPath`) | Non-absolute, non-@ paths treated as leaf names for DYLD_FALLBACK search | P1 |
| Path normalization | `Loader.cpp:1242` (`expandAndNormalizeAtExecutablePath`) | Resolve symlinks in `@executable_path` expansion | P2 |

### 5.2 Symbol Resolution (→ `macho-resolve`)

| Behavior | Source File:Line | Expected Behavior | Priority |
|----------|-----------------|-------------------|----------|
| Two-level namespace lookup | `Loader.cpp:2790` (`resolveSymbol`) | Symbol looked up only in the specific dylib identified by ordinal | P0 |
| `BIND_SPECIAL_DYLIB_SELF` | `Loader.cpp:2790` | Ordinal -1: look up symbol in the binary itself | P0 |
| `BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE` | `Loader.cpp:2790` | Ordinal -2: look up symbol in main executable | P0 |
| `BIND_SPECIAL_DYLIB_FLAT_LOOKUP` | `Loader.cpp:2790` | Ordinal -3: search all loaded images in load order | P1 |
| `BIND_SPECIAL_DYLIB_WEAK_LOOKUP` | `Loader.cpp:2790` | Ordinal -4: coalesce weak definitions across all images | P1 |
| Flat namespace search order | `DyldAPIs.cpp:1825` (`dlsym` with `RTLD_DEFAULT`) | Search all images in load order | P1 |
| Re-export forwarding | `MachOFile.cpp:577` (`forEachDependentDylib` with `LC_REEXPORT_DYLIB`) | Follow re-export chains to find actual defining dylib | P1 |

### 5.3 dlopen/dlsym Semantics (→ `macho-resolve`, `macho-run-plan`)

| Behavior | Source File:Line | Expected Behavior | Priority |
|----------|-----------------|-------------------|----------|
| `dlopen()` basic | `DyldAPIs.cpp:1312` (`dlopen_from`) | Load dylib, run initializers, return handle | P0 |
| `RTLD_NOLOAD` | `DyldAPIs.cpp:1312` | Return handle if already loaded, NULL otherwise; don't actually load | P1 |
| `RTLD_NODELETE` | `DyldAPIs.cpp:1312` | Mark image as non-unloadable | P2 |
| `RTLD_LOCAL` | `DyldAPIs.cpp:1312` | Symbols not available for flat namespace lookup from other dlopen'd images | P2 |
| `dlsym(RTLD_DEFAULT)` | `DyldAPIs.cpp:1825` | Search all images in load order | P1 |
| Search order for `dlopen` | `doc/man/man3/dlopen.3` | DYLD_FRAMEWORK_PATH → DYLD_LIBRARY_PATH → install_name dir → DYLD_FALLBACK paths | P1 |

### 5.4 Dependency Enumeration (→ `macho-resolve`)

| Behavior | Source File:Line | Expected Behavior | Priority |
|----------|-----------------|-------------------|----------|
| `forEachDependentDylib` | `MachOFile.cpp:577` | Iterate LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB, LC_LOAD_UPWARD_DYLIB | P0 |
| Dependency kind classification | `MachOFile.cpp:577` | Distinguish required vs weak vs reexport vs upward dependencies | P0 |
| LoadOptions struct | `Loader.h` | `rtldNoDelete`, `rtldNoLoad`, `insertedDylib`, `canBeDylib`, `canBeBundle`, `rpathStack` | P1 |

### 5.5 Initializer Ordering (→ `macho-run-plan`)

| Behavior | Source File:Line | Expected Behavior | Priority |
|----------|-----------------|-------------------|----------|
| Bottom-up initializer execution | `Loader.cpp:2565` (`runInitializersBottomUp`) | Dependencies initialized before dependents; leaf libraries first | P0 |
| Constructor priority | Inferred from ordering | `__mod_init_func` pointers called in order within each image | P1 |

---

## 6. XNU mach_loader Validation Cases

**Source**: `https://github.com/apple-oss-distributions/xnu` (`bsd/kern/mach_loader.c`, `libkern/kxld/`)
**Note**: These represent kernel-level Mach-O validation. They define what constitutes a valid Mach-O for execution.

### 6.1 Header Validation (→ `macho-parse`)

| Validation | Source Location | Expected Behavior | Priority |
|------------|----------------|-------------------|----------|
| Magic number check | `mach_loader.c:1075` | Accept MH_MAGIC / MH_MAGIC_64 / MH_CIGAM / MH_CIGAM_64 | P0 |
| `sizeofcmds` overflow | `mach_loader.c:1182` | `header_size + sizeofcmds` must not overflow, must fit within macho_size | P0 |
| Load command minimum size | `mach_loader.c:1334` | Each LC must be ≥ `sizeof(struct load_command)` (8 bytes) | P0 |
| Load command offset overflow | `mach_loader.c:1349` | `offset + cmdsize` must not overflow or exceed `cmds_size` | P0 |
| Header segment requirement | `mach_loader.c:1318` | Some segment must map offset 0 (the header) | P0 |
| ABI mismatch | `mach_loader.c:1430` | LC_SEGMENT in 64-bit binary → LOAD_BADMACHO | P0 |

### 6.2 Segment Validation (→ `macho-map`)

| Validation | Source Location | Expected Behavior | Priority |
|------------|----------------|-------------------|----------|
| Segment cmdsize minimum | `mach_loader.c:1375`, `MachOObjectFile.cpp:290` | `cmdsize >= sizeof(segment_command[_64])` | P0 |
| Section count bounds | `MachOObjectFile.cpp:290` | `nsects * sizeof(section) <= cmdsize - sizeof(segment_command)` | P0 |
| Header segment protection | `mach_loader.c:1444,1497` | Segment at fileoff=0 must have R+X initprot | P0 |
| Single header segment | `mach_loader.c:1444` | Only one segment may map offset zero | P0 |
| Zerofill handling | `kxld_util.c:376` | Segments with vmsize > 0 but filesize = 0 are zerofill | P1 |
| VM overflow check | `mach_loader.c:1398` | `vmaddr + vmsize + slide` must not overflow | P0 |

### 6.3 Entry Point Validation (→ `macho-run-plan`)

| Validation | Source Location | Expected Behavior | Priority |
|------------|----------------|-------------------|----------|
| LC_MAIN cmdsize | `mach_loader.c:2770` | `cmdsize >= sizeof(entry_point_command)` | P0 |
| Entry in executable segment | `mach_loader.c:2645` | Entry point must fall within a segment with R+X protection | P0 |
| LC_UNIXTHREAD vs LC_MAIN | `mach_loader.c:1497,1530` | Mutually exclusive; only one allowed per depth level | P0 |
| Driver binary (PLATFORM_DRIVERKIT) | `mach_loader.c:1296` | Driverkit binaries have no user-space entry point | P2 |

### 6.4 Multi-Pass Parsing (→ `macho-parse`, `macho-run-plan`)

| Pass | Source Location | What It Processes | Priority |
|------|----------------|-------------------|----------|
| Pass 0 | `mach_loader.c:1198` | TEXT/DATA boundary alignment check, platform version (LC_BUILD_VERSION, LC_VERSION_MIN_*) | P1 |
| Pass 1 | `mach_loader.c:1198` | Thread state (LC_UNIXTHREAD, LC_MAIN), UUID (LC_UUID), code signature (LC_CODE_SIGNATURE) | P1 |
| Pass 2 | `mach_loader.c:1198` | Segment mapping (LC_SEGMENT, LC_SEGMENT_64) | P0 |
| Pass 3 | `mach_loader.c:1198` | Dylinker (LC_LOAD_DYLINKER), encryption (LC_ENCRYPTION_INFO), entry point check | P1 |

### 6.5 ASLR & Slide Computation (→ `macho-map`)

| Validation | Source Location | Expected Behavior | Priority |
|------------|----------------|-------------------|----------|
| PIE + ASLR | `mach_loader.c:1162` | `MH_PIE` flag + dyld → apply ASLR slide | P1 |
| Slide applied to segments | `mach_loader.c:2254` | `vmaddr + slide` for all segment addresses | P1 |
| Rosetta slide | `mach_loader.c:3416` | Rosetta translation layer: custom slide below shared cache | P2 |

### 6.6 Specific Test File

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `tests/macho_size_63133398.c` | `load_machfile` improper macho_size validation (rdar://63133398) — tests that fat offset + minimal Mach-O with crafted size doesn't bypass validation | `macho-parse` | P0 |

### 6.7 kxld Validation (→ `macho-parse`)

| Validation | Source File | Expected Behavior | Priority |
|------------|------------|-------------------|----------|
| `validate_and_swap_macho_32` | `kxld_util.c:310` | Validate 32-bit Mach-O: check size fits header, iterate + validate each LC_SEGMENT's sections, check relocs and symtab bounds | P0 |
| `validate_and_swap_macho_64` | `kxld_util.c:460` | Same for 64-bit: segment vmsize > 0, fileoff + filesize <= file size, relocs within bounds | P0 |
| Zero vmsize segments | `kxld_object.c:1006` | Segments with vmsize == 0 are ignored (not mapped) | P1 |
| LINKEDIT ignored | `kxld_object.c:1006` | `__LINKEDIT` segment not processed as mappable | P1 |

---

## 7. Apple cctools Tests

**Source**: `https://github.com/apple-oss-distributions/cctools`
**Status**: The repository has `tests/` and `libstuff_test/` directories. Content not fully enumerable via GitHub search API, but directories exist at `cctools-1010.6` (tests) and `cctools-1030.6.3` (libstuff_test).

### 7.1 Known Structure

| Directory | Description | Target Crate | Priority |
|-----------|-------------|--------------|----------|
| `tests/` | Build-system-level tests for cctools binaries (otool, lipo, nm, etc.) | `macho-parse` | P2 |
| `libstuff_test/` | Unit tests for `libstuff` — internal library for Mach-O manipulation | `macho-parse`, `macho-types` | P2 |
| `otool/` | The original `otool` implementation — valuable for understanding expected output formats | Reference | P3 |
| `libmacho/` | Apple's libmacho: `getsectiondata`, `getsegbyname`, architecture helpers | Reference | P3 |

### 7.2 Potential Test Areas (Inferred)

- `lipo` tests: fat binary creation/extraction, architecture validation
- `otool` tests: load command display format, section dumping, disassembly output
- `nm` tests: symbol table display, sorting, filtering
- `libstuff` tests: byte swapping, Mach-O structure validation utilities

---

## 8. Darling Project Tests

**Source**: `https://github.com/darlinghq/darling`
**Note**: Darling is a macOS compatibility layer for Linux. Its Mach-O loading code is in `src/startup/mldr/`.

### 8.1 mldr Loader (`src/startup/mldr/`)

| File | What It Validates | Target Crate | Priority |
|------|-------------------|--------------|----------|
| `loader.c` | Mach-O segment loading: reads mach_header, validates filetype (MH_EXECUTE vs MH_DYLINKER), iterates load commands, maps segments with `mmap`, handles zerofill (vmsize > filesize) | `macho-parse`, `macho-map` | P1 |
| `mldr.c:271` (`load()`) | Fat binary detection: magic → MH_MAGIC_64/MH_MAGIC/FAT_MAGIC dispatch | `macho-parse` | P1 |

### 8.2 Runtime Tests (`tests/src/`)

| Test File | What It Validates | Target Crate | Priority |
|-----------|-------------------|--------------|----------|
| `objc_load.m` | `+[MyClass load]` called during image loading | `macho-objc`, `macho-run-plan` | P2 |
| `objc_super.m` | ObjC super message dispatch (runtime, not parsing) | N/A (runtime) | P3 |
| `objc_protocol.m` | `conformsToProtocol:` (runtime, not parsing) | N/A (runtime) | P3 |

### 8.3 Test Runner

- `tests/src/bin/runtests.py`: Harness that tests across architectures (i386, x86-64) and platform versions (10.2, 10.6, 10.8). Compares stdout against expected output files. Useful pattern for `test-harness` crate.

### 8.4 Build Tools

| File | What It Validates | Target Crate | Priority |
|------|-------------------|--------------|----------|
| `src/buildtools/getuuid.c` | Reads UUID from Mach-O/fat binary: handles MH_MAGIC, MH_MAGIC_64, FAT_MAGIC, FAT_CIGAM | `macho-parse` | P2 |
| `src/libelfloader/wrapgen/print_wrapped_elf.cpp` | Parses Mach-O to extract symbol names from segments; handles 32/64-bit and fat binaries | `macho-parse` | P2 |

---

## 9. LLVM BinaryFormat Unit Tests

**Source**: `https://github.com/llvm/llvm-project/tree/main/llvm/unittests/BinaryFormat/MachOTest.cpp`

| Test | What It Validates | Target Crate | Priority |
|------|-------------------|--------------|----------|
| `MAYBE_UnalignedLC` (line 43) | Load command access from unaligned 32-bit Mach-O: constructs valid minimal Mach-O in memory, iterates load commands verifying `cmd == LC_SEGMENT` with proper byte swapping | `macho-parse` | P1 |

---

## 10. Spec/RFC Conformance

**Finding**: There is no formal RFC or IETF-style specification for the Mach-O binary format. The de facto specification is:

1. **`EXTERNAL_HEADERS/mach-o/loader.h`** in XNU (https://github.com/apple-oss-distributions/xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h)
   - Defines all struct layouts, load command constants, segment flags, section types
   - ~1590 lines of header with extensive comments
   - Constants: LC_REQ_DYLD, MH_* flags, SG_* flags, S_* section types, PLATFORM_* values, CPU_TYPE_*

2. **Apple's `man dyld`** and **`man dlopen`** pages documented in the dyld repo under `doc/man/`

3. **LLVM's `BinaryFormat/MachO.h`** offers a parallel, independent definition

**Recommendation**: Create a conformance test category in `macho-types` that validates struct sizes, field offsets, and constant values against the `loader.h` definitions.

### Conformance Tests to Create

| Test | What It Validates | Target Crate | Priority |
|------|-------------------|--------------|----------|
| Header struct sizes | `mach_header` == 28 bytes, `mach_header_64` == 32 bytes | `macho-types` | P0 |
| Segment struct sizes | `segment_command` == 56 bytes, `segment_command_64` == 72 bytes | `macho-types` | P0 |
| Section struct sizes | `section` == 68 bytes, `section_64` == 80 bytes | `macho-types` | P0 |
| Load command constants | LC_SEGMENT == 0x1, LC_SEGMENT_64 == 0x19, LC_LOAD_DYLIB == 0xC, etc. | `macho-types` | P0 |
| LC_REQ_DYLD bit | `0x80000000` — unknown LCs with this bit → reject binary | `macho-parse` | P0 |
| CPU type constants | CPU_TYPE_X86_64 == 0x01000007, CPU_TYPE_ARM64 == 0x0100000C | `macho-types` | P0 |
| File type constants | MH_EXECUTE == 0x2, MH_DYLIB == 0x6, MH_BUNDLE == 0x8, MH_DYLINKER == 0x7 | `macho-types` | P0 |
| Flag constants | MH_PIE == 0x200000, MH_TWOLEVEL == 0x80, MH_NOUNDEFS == 0x1 | `macho-types` | P0 |
| `cmdsize` alignment | 32-bit: multiple of 4; 64-bit: multiple of 8 | `macho-parse` | P0 |

---

## 11. Crate Mapping Summary

### `macho-types` (Data Structures)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Struct size conformance | 6+ | All P0 |
| Constant value conformance | 10+ | All P0 |
| Relocation types | 3+ | P1 |
| Build version / platform | 2 | P1 |
| Unwind info structs | 4 | P2 |

### `macho-parse` (Bytes → ParsedMachO)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Header parsing (32/64-bit) | 5+ | P0 |
| Load command iteration | 5+ | P0 |
| Segment/section parsing | 5+ | P0 |
| Fat/universal parsing | 4+ | P1 |
| Malformed input rejection | 10+ | P0 |
| Symbol table parsing | 3+ | P0 |

### `macho-map` (VM Mapping Planner)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| PAGEZERO handling | 2 | P0 |
| Segment ordering | 2 | P0 |
| Page alignment | 2 | P1 |
| Zerofill computation | 2 | P1 |
| ASLR slide | 2 | P1 |

### `macho-resolve` (Dependency & Symbol Resolution)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| @rpath / @loader_path / @executable_path | 6 | P0 |
| Two-level namespace resolution | 3 | P0 |
| Export trie encoding/decoding | 5+ | P0 |
| Bind opcodes | 5+ | P0/P1 |
| Dependency enumeration (LC_LOAD_*) | 4 | P0 |
| Weak binding | 3 | P1 |
| Flat namespace | 3 | P1 |
| Chained fixups | 3 | P1 |
| Re-export chains | 2 | P1 |
| DYLD_LIBRARY_PATH / fallback | 3 | P1/P2 |

### `macho-objc` (ObjC Metadata Detection)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Section detection (__objc_classlist, etc.) | 3 | P1 |
| ObjC metadata dumping | 2 | P1 |
| +load method detection | 1 | P2 |

### `macho-run-plan` (Execution Plan)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Entry point (LC_MAIN vs LC_UNIXTHREAD) | 2 | P0 |
| Initializer ordering (bottom-up) | 2 | P0 |
| Dead stripping | 2 | P2 |

### `macho-diff` (Plan vs Observation)

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Plan-vs-observation comparison | Uses output from all above | P2 |

### `macho-fixtures` / `test-harness`

| Category | Test Count | Priority Mix |
|----------|-----------|--------------|
| Fixture generation methodology | Adopt llvm-mc + lld pattern | P1 |
| Cross-architecture validation | Adopt Darling's runtests.py pattern | P2 |

---

## 12. Fixture Strategy

### 12.1 Inline Byte Arrays (from Goblin)

Best for: `macho-types` struct tests, `macho-parse` header tests.

**Pattern**: Define `const FIXTURE: &[u8] = &[0xCF, 0xFA, ...]` directly in test files. Goblin's `DEADBEEF_MACH_64` is a proven example (8496 bytes).

**Advantages**: No external files, fully self-contained, easy to modify for edge cases.

### 12.2 Pre-built Binary Fixtures (from lld/llvm-objdump)

Best for: `macho-map`, `macho-resolve`, `macho-run-plan`.

**Pattern**: Build fixtures using:
```bash
# Simple executable
echo 'int main() { return 0; }' | clang -o fixture -
# Dylib with install_name
clang -dynamiclib -o libfoo.dylib foo.c -install_name @rpath/libfoo.dylib
# With rpath
clang -o main main.c -L. -lfoo -Wl,-rpath,@executable_path/lib
```

Store in `fixtures/` directory with TOML metadata (already exists in `macho-fixtures`).

### 12.3 YAML-Constructed Binaries (from llvm-objdump)

Best for: Malformed input testing, edge cases.

**Pattern**: Use `yaml2obj` (LLVM tool) to construct precise Mach-O binaries from YAML descriptions. Can create intentionally malformed binaries.

### 12.4 Fixture Categories Needed

| Fixture Name | Description | Used By |
|--------------|-------------|---------|
| `minimal_x86_64` | Smallest valid MH_EXECUTE for x86_64 | `macho-parse`, `macho-types` |
| `minimal_arm64` | Smallest valid MH_EXECUTE for arm64 | `macho-parse`, `macho-types` |
| `simple_dylib` | Basic `.dylib` with LC_ID_DYLIB | `macho-resolve` |
| `dylib_chain` | A → B → C dependency chain | `macho-resolve` |
| `rpath_chain` | Binary with @rpath dependencies + LC_RPATH entries | `macho-resolve` |
| `weak_dylib` | LC_LOAD_WEAK_DYLIB with absent library | `macho-resolve` |
| `reexport` | LC_REEXPORT_DYLIB chain | `macho-resolve` |
| `fat_binary` | Universal binary with arm64 + x86_64 | `macho-parse` |
| `constructor_test` | Binary with `__mod_init_func` | `macho-run-plan` |
| `objc_metadata` | Binary with `__objc_classlist`, `__objc_catlist` | `macho-objc` |
| `malformed_header` | Bad magic, truncated header | `macho-parse` |
| `malformed_lcmd` | cmdsize = 0, cmdsize > file, nsects overflow | `macho-parse` |
| `malformed_segment` | fileoff + filesize > file, vmsize overflow | `macho-parse` |
| `two_level_ns` | Binary compiled with two-level namespace (default) | `macho-resolve` |
| `flat_ns` | Binary compiled with `-flat_namespace` | `macho-resolve` |
| `export_trie` | Binary with non-trivial export trie | `macho-resolve` |
| `chained_fixups` | Binary using chained fixups (macOS 13+) | `macho-resolve` |
| `framework_style` | .framework bundle layout | `macho-resolve` |

---

## 13. Priority Tiers

### P0 — Must Have (Core correctness)

These tests validate fundamental parsing correctness. Without them, no higher-level feature works.

1. Header parsing: magic, cputype, filetype, ncmds, sizeofcmds
2. Load command iteration with bounds checking
3. Segment/section struct parsing
4. Malformed input rejection (no panics, no UB)
5. Struct size conformance (matches loader.h)
6. Symbol table parsing
7. Dependency enumeration (LC_LOAD_DYLIB family)
8. Export trie decoding
9. @rpath / @loader_path / @executable_path resolution
10. Entry point extraction (LC_MAIN)
11. Initializer ordering

**Estimated test count**: ~50-60 tests
**Primary sources**: Goblin + lld + XNU mach_loader + loader.h conformance

### P1 — Should Have (Feature completeness)

These tests cover important features needed for realistic binary analysis.

1. Fat/universal binary parsing
2. Bind opcodes (BIND_OPCODE_*)
3. Chained fixups
4. Weak binding / weak references
5. Flat namespace
6. Re-export chains
7. UUID, build version
8. Relocation entries
9. ObjC section detection
10. DYLD_LIBRARY_PATH / DYLD_FALLBACK_LIBRARY_PATH
11. ASLR slide computation

**Estimated test count**: ~40-50 tests
**Primary sources**: lld tests + dyld source

### P2 — Nice to Have (Completeness)

1. Compact unwind info
2. Stab entries
3. Thread-local variables
4. Dead stripping
5. Code signature
6. Data-in-code
7. Function starts
8. Multi-architecture archive files
9. Upward dependencies
10. Driver binaries (PLATFORM_DRIVERKIT)

**Estimated test count**: ~30 tests
**Primary sources**: llvm-objdump tests + dyld source

---

## Appendix A: Key URLs

| Source | URL |
|--------|-----|
| LLVM lld MachO tests | https://github.com/llvm/llvm-project/tree/main/lld/test/MachO/ |
| LLVM objdump MachO tests | https://github.com/llvm/llvm-project/tree/main/llvm/test/tools/llvm-objdump/MachO/ |
| Goblin tests/macho.rs | https://github.com/m4b/goblin/blob/master/tests/macho.rs |
| Apple dyld source | https://github.com/apple-oss-distributions/dyld |
| XNU mach_loader.c | https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/mach_loader.c |
| XNU loader.h | https://github.com/apple-oss-distributions/xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h |
| Apple cctools | https://github.com/apple-oss-distributions/cctools |
| Darling mldr loader | https://github.com/darlinghq/darling/tree/main/src/startup/mldr |
| LLVM MachOTest.cpp | https://github.com/llvm/llvm-project/blob/main/llvm/unittests/BinaryFormat/MachOTest.cpp |

## Appendix B: Test Methodology Reference

### lld/llvm-objdump Pattern (FileCheck)
```
# RUN: llvm-mc -filetype=obj -triple=x86_64-apple-macos %s -o %t.o
# RUN: %lld -o %t %t.o
# RUN: llvm-objdump --macho --all-headers %t | FileCheck %s
# CHECK: cmd LC_SEGMENT_64
# CHECK: segname __TEXT
```

### Goblin Pattern (Rust inline fixtures)
```rust
const FIXTURE: &[u8] = &[0xCF, 0xFA, 0xED, 0xFE, /* ... */];
#[test]
fn parse_header() {
    let mach = MachO::parse(FIXTURE, 0).unwrap();
    assert_eq!(mach.header.cputype, CPU_TYPE_X86_64);
}
```

### XNU Validation Pattern (kernel returns)
```c
if (lcp->cmdsize < sizeof(struct load_command)) {
    return LOAD_BADMACHO;
}
```
→ Rust equivalent: `parse()` returns `Err(ParseError::InvalidLoadCommand)`.

### Darling Test Runner Pattern (stdout comparison)
```python
expected = open(arch + "/" + name + ".stdout").read()
actual = subprocess.check_output([dyld_command, path + ".bin"])
assert actual == expected
```
→ Rust equivalent: `test-harness` crate's `run_fixture()` + golden file comparison.
