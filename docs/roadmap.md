# Darwin Loader Lab — Open Core Roadmap

> **Strategic goal**: Transform the project from a completed prototype into
> a full macOS-compatible runtime platform capable of running real software
> and eventually supporting its own host environment.

> **Next-generation roadmap**: See [NEXT_GENERATION_ROADMAP.md](NEXT_GENERATION_ROADMAP.md)
> for Tracks C–F: running real apps, host independence, cross-platform Linux
> support, and a native GUI stack.

Every subsystem may evolve:

* loader core
* ObjC runtime
* Foundation layer
* GUI framework
* platform abstraction
* host independence

Progress is driven by **running real binaries and expanding capabilities where required**.

---

## Status

- **900+ tests, 0 failures, 0 clippy warnings**
- **62 fixture binaries** executing end-to-end
- **30 workspace members** (27 crates + 3 GUI apps)
- **40 Foundation classes** registered
- **59 shim functions** (libsystem-shims) + ObjC/Foundation method shims
- **Compatibility: 62/62 (100%)**
- **3 GUI apps** (hello-darwin, control-gallery, foundation-explorer)
- **App layer phases 30–38 complete** — see [APP_LAYER_ROADMAP.md](APP_LAYER_ROADMAP.md)

---

## Completed Phases

<details>
<summary>Phase 1–17 — Loader Core (click to expand)</summary>

## Phase 1 — Parse & Inspect

- [x] `macho-types`: core data structures
- [x] `macho-parse`: binary parser
- [x] `macho-inspect`: CLI tool
- [x] Parse real binaries (simple_cli fixture)
- [x] Validate against otool output
- [x] Fixup parsing (classic rebase/bind opcodes + chained fixups)
- [x] Export trie parser (two-level namespace symbol lookup)

## Phase 2 — Planning

- [x] `macho-map`: mapping plan from segments
- [x] `macho-run-plan`: execution plan generator
- [x] Test mapping plans against vmmap output
- [x] Handle PAGEZERO, LINKEDIT correctly

## Phase 3 — Dependency Resolution

- [x] `macho-resolve`: path resolution + dependency graph
- [x] @rpath resolution with real fixture
- [x] Transitive dependency walking (BFS with cycle detection)
- [x] Init order computation matching dyld (post-order DFS)
- [x] Symbol binding: resolve undefined symbols against export tries

## Phase 4 — Host Comparison

- [x] `macho-diff`: comparison engine (dylibs + segments + entry point)
- [x] `scripts/collect_host_info.sh` (otool -L, -l, -h + DYLD_PRINT_*)
- [x] Automated comparison pipeline (`run_comparison.sh`)
- [x] JSON diff reports
- [x] CI integration (GitHub Actions: build, test, clippy, fmt)

## Phase 5 — ObjC & Constructors

- [x] `macho-objc`: metadata detection
- [x] Parse __objc_classlist (ObjcClassRef, ObjcMetadata)
- [x] Model constructor ordering (__mod_init_func + __init_offsets)
- [x] Compare init order with DYLD_PRINT_INITIALIZERS

## Phase 6 — Security Gate Modeling

- [x] `trust-policy`: trait-based security gate framework
- [x] 8 security gates (Quarantine → Sandbox) as traits
- [x] Permissive mock implementations for all gates
- [x] TrustPolicy composition object
- [x] Integration with ExecutionPlan (SecurityReport)
- [x] Restrictive (Apple-default) policy implementations

## Phase 6.5 — Framework & Environment Resolution

- [x] Expand MachFlags (WEAK_DEFINES, BINDS_TO_WEAK, ALLOW_STACK_EXECUTION, etc.)
- [x] Framework fixture (MyFramework.framework bundle)
- [x] SearchEnv: DYLD_LIBRARY_PATH, DYLD_FRAMEWORK_PATH, fallback paths
- [x] Framework bundle resolution (.framework/ marker detection)
- [x] Standard dylib/framework fallback paths (/usr/lib, /System/Library/Frameworks)
- [x] Fix BIND_SPECIAL_DYLIB_SELF (ordinal 0 → own export trie)
- [x] `--resolve` CLI flag (dependency tree view)
- [x] `--plan` CLI flag (full execution plan)

## Phase 7 — Controlled Execution

- [x] `macho-exec` crate: in-process segment mapping via mmap
- [x] MappedImage: map segments, skip __PAGEZERO, proper permissions
- [x] Chained fixup chain walker (DYLD_CHAINED_PTR_64_OFFSET)
- [x] Imports table parsing (symbol name resolution from ordinals)
- [x] StubResolver: safe-list dlsym for known libc functions
- [x] Classic fixup application (rebases + binds)
- [x] `return42` fixture: trivial binary for controlled entry testing
- [x] Controlled entry: call LC_MAIN entry point in mapped code
- [x] Stack safety verification after controlled entry
- [x] Contiguous segment mapping (preserves PC-relative stub references)
- [x] In-process execution of simple_cli (printf via stub/GOT pipeline)

## Phase 8 — Security Hardening

- [x] `macho-types::codesig`: SuperBlob, CodeDirectory, CsFlags, CsHashType types
- [x] Code signature parsing (SuperBlob → CodeDirectory → page hashes + CDHash)
- [x] `CodeSignature` field on `ParsedMachO` (auto-parsed from LC_CODE_SIGNATURE)
- [x] CDHash computation (SHA-256 via CommonCrypto)
- [x] CS flag detection: ADHOC, LINKER_SIGNED, RUNTIME, RESTRICT, REQUIRE_LV
- [x] Entitlement plist extraction (CSSLOT_ENTITLEMENTS → raw XML)
- [x] `CodeSignatureInfo` summary type in trust-policy
- [x] `evaluate_binary_parsed()`: enriched evaluation with real CS flags
- [x] CS_RESTRICT → strip_dyld_env override
- [x] CS_RUNTIME → hardened_runtime override
- [x] `SecurityDenied` error variant in macho-exec
- [x] `checked_entry()`: security-gated execution (policy evaluation before entry)
- [x] Expanded StubResolver safe-list (+16 symbols: stack protector, IO, strings)
- [x] 15 new tests (11 codesig parsing + 4 security-gated execution)

## Phase 9 — Multi-Image Loading & Constructor Execution

- [x] `SymbolResolver` trait: abstract interface for symbol resolution
- [x] `StubResolver` implements `SymbolResolver` (system dlsym)
- [x] `apply_chained_fixups` / `walk_chain` accept `&dyn SymbolResolver`
- [x] `imports_format` support: DYLD_CHAINED_IMPORT (4B), ADDEND (8B), ADDEND64 (16B)
- [x] `__init_offsets` parsing fix: offsets from image base, not position-relative
- [x] `initializer.rs`: `call_constructors()` — slide-adjusted extern "C" fn() calls
- [x] `ImageRegistry`: multi-image load, parse, map, extract exports
- [x] `RegistryResolver`: composite resolver (export tries → system fallback)
- [x] `apply_all_fixups()`: cross-image chained fixup resolution
- [x] `controlled_entry_constructor_test`: 2 constructors + main in-process
- [x] `multi_image_dylib_chain`: libchainb → libchaina → app end-to-end execution
- [x] CLI `--exec` flag: in-process map → fixup → constructors → entry
- [x] CLI `--codesig` flag: SuperBlob, CodeDirectory, CDHash display
- [x] 2 new integration tests (320 total)

## Phase 10 — Fixup Precision & CLI Multi-Image

- [x] ARM64e pointer format dispatch (formats 1, 12, 13): bit 62 = bind, bit 63 = auth
- [x] ARM64e rebase: auth 32-bit target vs non-auth 43-bit target + high8
- [x] ARM64E_USERLAND24 (format 13): 24-bit bind ordinal, 11-bit next
- [x] Weak import null-fill: unresolvable weak imports → write 0 (dyld semantics)
- [x] Out-of-range ordinal warning (was silently skipped)
- [x] Format-6 (PTR_64_OFFSET) rebase: RVA + segment vmaddr + slide (was missing vmaddr)
- [x] `LC_LOAD_WEAK_DYLIB` (0x80000018): added to types + parser → `parsed.dylibs`
- [x] `weak_dylib` fixture: `main.c` weak-links `liboptional.dylib`, tests absent + present cases
- [x] `weak_import_null_fill_absent_dylib` test: absent dylib → null-fill → else branch taken
- [x] `weak_import_resolved_when_dylib_present` test: ImageRegistry loads dylib → function resolved
- [x] CLI `--exec` multi-image: recursive BFS dependency loading via ImageRegistry
- [x] `@rpath` / `@executable_path` / `@loader_path` resolution in CLI loader
- [x] Transitive dependency loading (dylib_chain: app→libchaina→libchainb all auto-loaded)
- [x] 2 new integration tests (322 total)

## Phase 11 — Fat/Universal Binary Support & Page Hash Verification

- [x] `FatHeader`, `FatArch`, `FatBinary` types in macho-types
- [x] `fat.rs` module: `parse_fat()`, `is_fat()`, `fat_slice_for_cpu()`, `parse_auto()`
- [x] Big-endian / byte-swapped (`FAT_CIGAM`) header parsing
- [x] `fat_binary` fixture: universal binary with arm64 + x86_64 slices
- [x] 9 fat binary tests: header parse, slice extraction, auto-detect, system binary
- [x] CLI `--cpu` flag: select architecture from fat binary (arm64, x86_64, arm, x86)
- [x] Fat binary auto-detection with architecture listing in CLI output
- [x] `verify_page_hashes()`: SHA-256 page hash verification against CodeDirectory
- [x] `PageHashResult` / `PageHashMismatch` types for programmatic inspection
- [x] 4 page hash tests: clean binary, tampered binary (XOR at 0x100), constructor_test, dylib_chain
- [x] CLI `--codesig` now shows page hash verification status (N/N verified OK or MISMATCH)
- [x] Fat binary page hash verification uses architecture slice (not full fat data)
- [x] 13 new tests (335 total)

## Phase 12 — Fixup Correctness, Re-Export Chains & Runtime Completeness

- [x] **Fix**: `apply_all_fixups()` now dispatches on `FixupModel` — `Classic` → `apply_rebases` + resolver-based binds, `Chained` → `apply_chained_fixups`, `None` → no-op (was always calling chained path)
- [x] **Fix**: `RegistryResolver::resolve()` follows re-export chains — when an export has `REEXPORT` flag, looks up `reexport_name` in the dylib at `reexport_ordinal` (recursive)
- [x] `apply_classic_binds_with_resolver()`: resolver-based classic bind application for `ImageRegistry` execution
- [x] `SectionType` named enum (23 variants): `Regular`, `ZeroFill`, `CstringLiterals`, `ModInitFuncPointers`, `ThreadLocalVariables`, `InitOffsets`, etc.
- [x] `Section::section_type_name()` returns `SectionType` enum variant
- [x] `__objc_imageinfo` parsing: `ObjcImageInfo { version, flags }` with `supports_objc2()`, `requires_gc()`, `is_simulated()`, `swift_version()`
- [x] `ObjcMetadata.image_info: Option<ObjcImageInfo>` field
- [x] 8 new `LoadCommandType` variants: `UnixThread`, `VersionMinMacOS`, `VersionMinIOS`, `DyldEnvironment`, `LinkerOption`, `VersionMinTvOS`, `VersionMinWatchOS`, `Note`
- [x] `LC_UNIXTHREAD` parser: extracts entry address from x86_64/arm64/i386 thread state
- [x] `VersionMin { version, sdk }` and `DyldEnvironment { name }` payload variants
- [x] `LC_FUNCTION_STARTS` decoder: ULEB128-delta-compressed function entry point parsing
- [x] `parse_function_starts()` returns `Vec<u64>` of function start virtual addresses
- [x] CLI `--funcs` flag: display decoded function start addresses
- [x] `StubResolver` allowlist expanded: +30 symbols (pthread, ObjC runtime, Blocks, GCD, math, mach)
- [x] 25 new tests (360 total)

## Phase 13 — Parser Completeness, ObjC Class Names & Robustness Hardening

- [x] `vmaddr_to_file_offset()` utility: converts virtual address to file offset by scanning segments
- [x] **ObjC class name extraction**: `extract_class_name()` follows on-disk pointer chain: `__objc_classlist[i]` → `objc_class_t.data` (offset 32) → `objc_class_ro_t.name` (offset 24) → C-string
- [x] `resolve_on_disk_pointer()`: strips chained fixup metadata (low 40 bits), tries `vmaddr_to_file_offset` then direct file offset fallback
- [x] `ObjcClassRef.name: Option<String>` field: class names populated during `parse_objc_metadata()`
- [x] `objc_classes` fixture: `Greeter` and `Calculator` classes compiled to arm64 binary
- [x] `LoadCommandPayload::SourceVersion { version: u64 }` + `decode_source_version()` decoder (5 fields: a.b.c.d.e)
- [x] `LoadCommandPayload::Dysymtab { ilocalsym, nlocalsym, iextdefsym, nextdefsym, iundefsym, nundefsym, indirectsymoff, nindirectsyms }`
- [x] `decode_version()`: packed u32 → "major.minor.patch" string
- [x] `platform_name()`: 12 platforms (macOS, iOS, tvOS, watchOS, bridgeOS, macCatalyst, simulators, driverKit, visionOS)
- [x] CLI ObjC metadata section: classes with names, categories, protocols, image_info
- [x] CLI Build Version section: platform name, min OS, SDK version
- [x] CLI Source Version section
- [x] CLI `--symbols` flag: full symbol table listing (address, type, section, name)
- [x] **Malformed input hardening**: empty input, single byte, bad magic, truncated header, absurd fat nfat_arch, load command count overflow, truncated classlist, vmaddr with no segments
- [x] Version decode tests: `decode_version()`, `platform_name()`, `decode_source_version()` (12 tests)
- [x] Parser tests: vmaddr_to_file_offset, SourceVersion, Dysymtab, BuildVersion, ObjC class names (10 tests)
- [x] 32 new tests (392 total)

## Phase 14 — Indirect Symbols, Weak Binds & Symbol Metadata

- [x] **Bug fix (P0)**: `LoadCommandPayload::DyldInfo` was missing `weak_bind_off`/`weak_bind_size` fields — classic `dyld_info_command` has 10 data fields (5 pairs), parser only read 8, causing lazy_bind and export offsets to be silently wrong
- [x] `DyldInfo` now has all 10 fields: rebase, bind, **weak_bind**, lazy_bind, export (off+size each)
- [x] `Fixups.weak_binds: Vec<BindEntry>`: classic weak bind opcode stream parsed and stored separately
- [x] `Section.reserved1: u32` and `Section.reserved2: u32`: indirect symbol table index and stub size, previously discarded with `_reserved1`/`_reserved2`
- [x] `indirect.rs` module: `read_indirect_symbol_table()` reads raw u32 indices; `resolve_indirect_symbols()` cross-references `__stubs`, `__got`, `__la_symbol_ptr`, `__nl_symbol_ptr` slots to symbol names
- [x] `IndirectSymbolEntry` struct with section, slot_index, slot_addr, symbol_name, raw_index
- [x] `INDIRECT_SYMBOL_LOCAL` (0x8000_0000) and `INDIRECT_SYMBOL_ABS` (0x4000_0000) sentinel handling
- [x] `Symbol` accessor methods: `library_ordinal()`, `is_weak_ref()`, `is_weak_def()`, `is_no_dead_strip()`, `is_referenced_dynamically()`
- [x] `is_stab()` function: detect stab debugging symbols from raw n_type byte
- [x] ObjC selector string extraction: `parse_selrefs()` follows `__objc_selrefs` pointer array → `__objc_methnames` C-strings
- [x] `ObjcMetadata.selectors: Vec<String>`: selector names populated during `parse_objc_metadata()`
- [x] CLI `--indirect` flag: displays indirect symbol table grouped by section (stubs, GOT, lazy symbol ptrs)
- [x] CLI `--selrefs` flag: displays ObjC selector strings
- [x] CLI `--symbols` enhanced: shows `[weak_ref]`, `[weak_def]`, `[no_dead_strip]`, `[ref_dyn]` annotations
- [x] CLI `print_text()` ObjC section now shows selector count
- [x] CLI `print_text()` fixups now shows `weak_binds` count and entries
- [x] 31 new tests (423 total)

## Phase 15 — Environment Simulation, TLV, Entitlements & JSON Completeness

- [x] **Bug fix (P0)**: ordinal -3 (`BIND_SPECIAL_DYLIB_WEAK_LOOKUP`) unhandled — now calls `flat_lookup()`, callers treat `None` as null (weak semantics)
- [x] `SearchEnv::from_env()`: reads `DYLD_LIBRARY_PATH`, `DYLD_FRAMEWORK_PATH`, `DYLD_FALLBACK_LIBRARY_PATH`, `DYLD_FALLBACK_FRAMEWORK_PATH` from process environment (colon-split)
- [x] `SearchEnv::merge_dyld_env_entries()`: parses `KEY=val` entries from `LC_DYLD_ENVIRONMENT` load commands, appends to corresponding `SearchEnv` fields
- [x] `LC_DYLD_ENVIRONMENT` wired into `resolve_inner()`: extracts `DyldEnvironment` payloads from parsed commands, merges into `SearchEnv`, passes to `PathResolver::new_full()`
- [x] `TlvDescriptor` struct in `macho-types::segment`: address, thunk, key, offset fields (3-pointer layout, 24 bytes on 64-bit)
- [x] `parse_tlv_descriptors()` in `macho-parse::tlv`: scans `__thread_vars` sections (`S_THREAD_LOCAL_VARIABLES` / 0x13), extracts descriptors with 64-bit and 32-bit support
- [x] `tlv_basic` fixture: C program with `_Thread_local int` variables, compiled for arm64, produces 2 TLV descriptors + `__thread_data` + `__thread_bss` sections
- [x] `CodeSignature::entitlement_keys()`: lightweight XML scanner extracts `<key>` tags from entitlements plist, resolves `<true/>` / `<false/>` values, non-boolean defaults to `true` (present)
- [x] `JsonReport` struct in `macho-inspect`: comprehensive JSON output including `ParsedMachO`, fixups, exports, ObjC metadata, function starts, TLV descriptors, indirect symbols, entitlement keys
- [x] CLI `--json-full` flag: full JSON dump via `JsonReport` serialization
- [x] CLI `--tlv` flag: displays TLV descriptors (address, thunk, key, offset)
- [x] CLI `--codesig` enhanced: shows structured entitlement keys with boolean values
- [x] 32 new tests (455 total)

## Phase 16 — Relocations, Correctness Fixes & Infrastructure Hardening

- [x] **Bug fix (P0)**: `RelocationType::From<u8>` missing arm64 codes 5–13 — added `Branch24`, `Arm64Branch26`, `Arm64Page21`, `Arm64Pageoff12`, `Arm64GotLoadPage21`, `Arm64GotLoadPageoff12`, `Arm64Subtractor`, `Arm64Addend`, `Arm64Pointer`
- [x] **Bug fix (P0)**: `is_zerofill()` missing `S_THREAD_LOCAL_ZEROFILL` (0x12) — now checks `0x01 || 0x0C || 0x12`
- [x] **Bug fix (P1)**: `ParsedMachO` only derived `Serialize`, not `Deserialize` — blocked JSON roundtripping and golden-file testing
- [x] **Bug fix (P1)**: `RegistryResolver` not exported from `macho-exec` — inaccessible to downstream crates
- [x] Relocation entry parser: `parse_relocations()` / `SectionRelocations` struct / `parse_relocation_entries()` — decodes 8-byte `relocation_info` entries (symbolnum:24, pcrel:1, length:2, extern:1, type:4)
- [x] `EntitlementEvaluator` trait enriched with `evaluate_with_keys()` default method — takes `binary_path` + `keys: &[(String, bool)]`, backward-compatible
- [x] `AppleEntitlements::evaluate_with_keys()` — detects dangerous entitlements: `disable-library-validation` → AllowWithWarning, `allow-dyld-environment-variables` → AllowWithWarning
- [x] `TrustPolicy::evaluate_binary_parsed_with_keys()` — re-evaluates entitlements gate with parsed keys; `evaluate_binary_parsed()` now delegates to this
- [x] CLI `--relocs` flag: displays relocations grouped by section (address, symbol, type, length, extern, pcrel)
- [x] `JsonReport` extended with `relocations: Vec<SectionRelocations>`
- [x] `known_fixtures()` expanded from 6 → 11 entries (added return42, fat_binary, objc_classes, weak_dylib, tlv_basic)
- [x] `build_fixtures.sh` updated with build steps for tlv_basic, objc_classes, fat_binary, weak_dylib
- [x] 41 new tests (496 total)

## Phase 17 — Ordinal Correctness, Execution Integrity & Type Completeness

- [x] **Bug fix (P0)**: `LC_REEXPORT_DYLIB` (0x8000_001F) fell to `Unknown`, causing ordinal holes — added `ReexportDylib` variant, parsed as dylib-loading command, contributes to ordinal table
- [x] **Bug fix (P0)**: `LC_LAZY_LOAD_DYLIB` (0x20) fell to `Unknown`, same ordinal corruption — added `LazyLoadDylib` variant with dylib parsing
- [x] **Bug fix (P0)**: `LC_LOAD_UPWARD_DYLIB` (0x8000_0023) fell to `Unknown` — added `LoadUpwardDylib` variant for circular dependency handling
- [x] **Bug fix (P0)**: Export trie `STUB_AND_RESOLVER` (flags 0x10) only read stub_offset, dropped resolver_offset — now reads both ULEBs and returns the resolver (real function address)
- [x] **Bug fix (P1)**: `call_entry()` never called `image.protect()` before execution — segments were RW during code execution. Now calls `protect()` to finalize permissions (RX for __TEXT, R for __DATA_CONST) before entry
- [x] **Bug fix (P1)**: `ExecutionPlan::from_parsed_secured` used `evaluate_binary()` (flag-only check) — now uses `evaluate_binary_parsed_with_keys()` with `CodeSignatureInfo` and entitlement keys
- [x] `MapRegion.max_protection: VmProt` — stores maxprot from segment commands (previously discarded). Displayed in `MappingPlan::display()`, `print_text()`, `ExecutionPlan::display()`
- [x] `MachFlags`: added `INCRLINK` (0x02), `FORCE_FLAT` (0x100), `SUBSECTIONS_VIA_SYMBOLS` (0x2000), `CANONICAL` (0x4000), `ROOT_SAFE` (0x0040_0000) — all appear in real macOS binaries
- [x] `CpuSubtype` enum: `Arm64All`, `Arm64E`, `ArmV7`, `ArmV7S`, `X86_64All`, `X86_64Haswell`, `Unknown(u32)` — with `from_pair(CpuType, u32)`, masks `CPU_SUBTYPE_LIB64` (0x8000_0000)
- [x] `fat_slice_for_cpu_subtype()`: prefers exact (type + subtype) match, falls back to type-only. `fat_slice_for_cpu()` delegates to it with `None`
- [x] `LC_ENCRYPTION_INFO` (0x21) / `LC_ENCRYPTION_INFO_64` (0x2C): parsed as `EncryptionInfo { crypt_off, crypt_size, crypt_id }`. `checked_entry()` rejects binaries with `crypt_id != 0` via `ExecError::EncryptedBinary`
- [x] `DataInCodeEntry` type + `DataInCodeKind` enum: `Data`, `JumpTable8/16/32`, `AbsJumpTable32`
- [x] `parse_data_in_code()`: decodes 8-byte entries from `LC_DATA_IN_CODE` linked-edit blob
- [x] CLI `--data-in-code` flag: displays data-in-code entries (offset, length, kind)
- [x] `extract_cstrings()` + `SectionStrings`: extracts null-terminated strings from `S_CSTRING_LITERALS` sections and known section names (`__cstring`, `__oslogstring`, `__objc_methnames`, `__objc_classname`, `__objc_methtype`)
- [x] CLI `--strings` flag: displays C-strings grouped by section
- [x] CLI `--rebase` flag: verbose rebase entry listing (segment index, offset, type)
- [x] `JsonReport` extended with `data_in_code: Vec<DataInCodeEntry>`
- [x] `LoadCommandPayload` extended: `ReexportDylib(DylibRef)`, `LazyLoadDylib(DylibRef)`, `LoadUpwardDylib(DylibRef)`, `EncryptionInfo { crypt_off, crypt_size, crypt_id }`
- [x] 44 new tests (540 total)

</details>

<details>
<summary>Phase 18–20 — Runtime Layer Foundation (click to expand)</summary>

## Phase 18 — Runtime Layer Foundation

> Architectural pivot: from loader completeness to runtime compatibility.
> Goal: "build the smallest usable macOS runtime layer above the already-strong loader."

### Architecture & Design
- [x] `docs/RUNTIME_ARCHITECTURE.md`: comprehensive architecture document (350+ lines) — architecture diagram, crate map, target-program matrix (T0–T10), layer contracts, phase plan (18–23), design principles, anti-goals
- [x] Binary-driven development model: target programs (not Mach-O features) drive what to build
- [x] Loader core frozen — no new features unless runtime cannot work without them

### `libsystem-shims` crate — controlled system library shims
- [x] `ShimTable`: symbol → address mapping with register/resolve/resolve_stripped/iter/symbol_names
- [x] `ShimEntry`: name + address + description for debugging
- [x] `ShimConfig`: process_name, executable_path, argc, argv, env (with Default)
- [x] `ProcessShims`: getprogname, _NSGetExecutablePath, _NSGetArgc, getenv — all `extern "C"` ABI
- [x] `TlsShims`: _tlv_bootstrap (allocates per-descriptor TLS storage), _tlv_atexit (no-op in single-threaded lab)
- [x] `TlvDescriptor` repr(C) struct matching dyld/libpthread layout
- [x] Global `TlvState` with key allocation and per-key heap allocations
- [x] `DispatchShims`: dispatch_once (Block_literal invoke extraction at offset 16), dispatch_once_f (function pointer variant), dispatch_async (inline execution)
- [x] Single-threaded fast-path for all dispatch shims (if not done, call, mark done)
- [x] `install_default_shims(table, config)`: installs all 9 shims in one call

### `macho-runtime` crate — orchestration hub
- [x] `RuntimeConfig`: security_policy (Optional<TrustPolicy>), host_fallback, shim_set, env_overrides, process_name, process_path, verbose
- [x] `RuntimeConfig::permissive()`: no security, host fallback enabled, all shims
- [x] `RuntimeConfig::secure()`: Apple-default TrustPolicy, host fallback enabled
- [x] `ShimSet`: per-category shim enable flags (process, tls, dispatch)
- [x] `RuntimeContext`: builds and holds ShimTable with selected shim categories
- [x] `CompositeResolver`: layered resolution chain (shim table → host StubResolver), implements `SymbolResolver`
- [x] `RuntimeError`: Io, Parse, Exec, SecurityDenied, EncryptedBinary, NoArm64Slice, NoEntryPoint
- [x] `RuntimeResult`: exit_code, security_report, images_loaded, shims_installed, initializers_run
- [x] `Runtime::load_and_run(path, config)`: full pipeline — read → parse_auto (fat-aware) → security → map → shims → composite resolver → fixups (classic vs chained dispatch) → constructors → entry
- [x] Security evaluation: encrypted binary rejection, CodeSignatureInfo extraction, policy evaluation with deny-on-failure

### Tests
- [x] 18 unit tests in libsystem-shims: ShimTable CRUD, ProcessShims registration, TlsShims bootstrap/reuse/null, DispatchShims once semantics
- [x] 3 unit tests in macho-runtime: CompositeResolver priority/fallback/none
- [x] 12 integration tests: config construction, context shim installation, resolver composition, full runtime execution of return42 (exit 42), simple_cli (exit 0), constructor_test (constructors run), nonexistent file error, no-host-fallback mode
- [x] All 3 fixture binaries execute end-to-end through the runtime layer
- [x] 33 new tests (573 total including all crates)

## Phase 19 — Multi-Image Runtime, Pipeline Hardening & Shim Expansion

> Goal: Run T3 (dylib_chain) through T6 (dispatch_test) through the runtime layer.
> Multi-image loading, cross-image fixup resolution, and shim coverage for real-world primitives.

### Multi-Image Runtime Loading
- [x] `Runtime::load_and_run()` rewritten: single-image → multi-image pipeline via `ImageRegistry`
- [x] BFS dependency loading (`load_dependencies`): walks LC_LOAD_DYLIB/LC_LOAD_WEAK_DYLIB, skips system libraries
- [x] `is_system_library()`: detects `/usr/lib/`, `/System/Library/`, `/Library/` prefixes
- [x] `resolve_dylib_path()`: resolves `@rpath`, `@executable_path`, `@loader_path` relative to main binary
- [x] `registry.apply_all_fixups(&composite)`: unified fixup application for both chained and classic models including classic binds
- [x] `protect()` called on all images after fixup application (was missing in Phase 18)
- [x] Constructors run on all images in load order, not just main binary
- [x] `images_loaded` reflects actual `registry.images().len()` count

### New Shim Modules (12 new shims, 21 total)
- [x] `ProcessShims`: added `_NSGetArgv` (5th process shim) — returns `*const *const c_char` pointer array
- [x] `ProcessState.argv_ptrs: Vec<usize>` — stores argv pointers as usize for Send+Sync compatibility with OnceLock
- [x] `PthreadShims` (6 shims): `pthread_self` (fixed 0x1000), `pthread_mutex_init/lock/unlock/destroy` (no-op stubs), `pthread_once` (single-threaded fast path)
- [x] `MachShims` (3 shims): `mach_task_self_`, `mach_absolute_time`, `mach_timebase_info` — all delegate to real libc
- [x] `LifecycleShims` (2 shims): `atexit` (registers handlers in global Vec), `__cxa_atexit` (C++ ABI variant)
- [x] `run_atexit_handlers()`: controlled LIFO atexit handler execution
- [x] `ShimSet` extended: 6 categories (process, tls, dispatch, pthread, mach, lifecycle) — all default-enabled
- [x] `install_default_shims()` updated to install all 6 categories

### New Fixtures
- [x] `process_info` fixture: calls `getprogname()`, `getenv("HOME")`, prints `argc` — validates process identity shims
- [x] `dispatch_test` fixture: calls `dispatch_once_f` 3 times, verifies `init_count==1` — validates dispatch shim semantics
- [x] `known_fixtures()` expanded from 11 → 13 entries

### Integration Tests (20 tests, up from 12)
- [x] `runtime_dylib_chain`: multi-image loading, exit 0, images_loaded > 1
- [x] `runtime_weak_dylib`: weak import resolution through runtime
- [x] `runtime_fat_binary`: fat binary execution through runtime pipeline
- [x] `runtime_tlv_basic` (T4): TLS runtime with _tlv_bootstrap shim
- [x] `runtime_process_info` (T5): process identity shims end-to-end
- [x] `runtime_dispatch_test` (T6): dispatch_once_f semantics end-to-end
- [x] `runtime_dylib_chain_images_counted`: verifies images_loaded >= 2
- [x] `runtime_shim_count_with_all_categories`: verifies 21 installed shims
- [x] 21 new tests (594 total including all crates)

## Phase 20 — ObjC Runtime (Class Registration + Messaging)

> Goal: Run T7 (simple ObjC CLI without Foundation) through the runtime layer.
> New `objc-runtime` crate provides class registration, selector interning, and `objc_msgSend` dispatch.

### Enhanced ObjC Metadata Parsing (`macho-objc`)
- [x] `ObjcMethodInfo` struct: selector name, type encoding, IMP virtual address
- [x] `ObjcClassRef` extended: `methods: Vec<ObjcMethodInfo>`, `instance_size: u32`, `ro_flags: u32`
- [x] `parse_method_list()`: parses `class_ro_t.baseMethods` → `method_list_t` → `method_t` entries
- [x] Relative method list support (modern arm64 ABI, bit 31 set): 3 × int32 per entry (12 bytes)
- [x] Selector indirect resolution: relative ptr → `__objc_selrefs` → pointer → `__objc_methnames` string
- [x] Absolute method list support (legacy): 3 × pointer per entry (24 bytes)
- [x] `file_offset_to_vmaddr()`: inverse of `vmaddr_to_file_offset` for relative offset computation
- [x] `resolve_to_vmaddr()`: strips chained fixup metadata, reconstructs full virtual address
- [x] `class_data_addr` now stores resolved vmaddr (not raw on-disk pointer with fixup bits)

### New `objc-runtime` Crate (15th workspace member)
- [x] `ObjcRuntime`: class table (`HashMap<String, ClassInfo>`) + address index (`HashMap<u64, String>`) + selector interning
- [x] `ClassInfo`: name, mapped_addr, methods, instance_size, is_root, is_meta
- [x] `MethodEntry`: selector, IMP address (slide-adjusted), type encoding
- [x] `register_image(metadata, slide)`: populates class table from parsed ObjC metadata
- [x] `msg_lookup(isa, selector)`: ISA → class lookup → method search → IMP address
- [x] `get_class(name)` / `get_class_by_addr(addr)`: class table lookups
- [x] `intern_selector(name)`: stable C-string pointer allocation for selector uniquing
- [x] Global `OBJC_RUNTIME: Mutex<Option<ObjcRuntime>>` — reset-safe for multi-run test support

### ObjC Shims (6 new shims, 27 total)
- [x] `objc_msgSend` — 8-register forwarding pattern: captures x0-x7, looks up IMP via class table, tail-calls
- [x] `objc_getClass` — name-based class lookup, returns mapped class address
- [x] `class_createInstance` — allocates `calloc(1, instanceSize)`, sets ISA pointer
- [x] `sel_registerName` — returns interned selector pointer (falls back to input pointer)
- [x] `class_getName` — returns class name C-string from interned selector table
- [x] `__objc_empty_cache` — 16-byte zero-filled data symbol for class cache references
- [x] `ShimSet.objc: bool` — 7th shim category, default enabled
- [x] All shim functions panic-safe (`extern "C"` boundary: uses `eprintln!` + `abort()` instead of `panic!`)
- [x] Mutex poison recovery via `lock().unwrap_or_else(|e| e.into_inner())`

### Runtime Pipeline Integration
- [x] ObjC class registration injected between fixup application (step 7) and protect (step 8)
- [x] All loaded images scanned for ObjC metadata: `parse_objc_metadata()` per image
- [x] Classes registered with mapped addresses (vmaddr + ASLR slide)
- [x] Root class instance_size 0 → 8 (minimum for ISA pointer)

### New Fixture: `objc_minimal` (T7)
- [x] Pure ObjC, no Foundation — `__attribute__((objc_root_class))`
- [x] Defines `Answer` class with `-value` method returning 42
- [x] Uses `objc_getClass`, `class_createInstance`, `sel_registerName`, `objc_msgSend`
- [x] Compiled: `xcrun clang -arch arm64 -fno-objc-arc -Wno-objc-root-class -lobjc`
- [x] `known_fixtures()` expanded from 13 → 14 entries

### Tests
- [x] 8 unit tests in `objc-runtime`: class registration, addr lookup, msg_lookup, selector interning, root class sizing, shim registration
- [x] `runtime_objc_minimal` (T7): end-to-end ObjC method dispatch through runtime pipeline, exit 0
- [x] `runtime_objc_minimal_class_registration`: verifies Answer class registered with 'value' method
- [x] 10 new tests (604 total including all crates)

</details>

<details>
<summary>Phase 21–33 — ObjC Runtime, Foundation, Hardening (click to expand)</summary>

### Phase 21 — ObjC Categories, +load, Autorelease ✅
- [x] Category parsing + merge (instance + class methods)
- [x] +load dispatch from metaclass class_methods
- [x] Autorelease pool shims (push/pop)
- [x] objc_retain / objc_release / objc_storeStrong shims
- [x] `objc_category` fixture (3-file ObjC with category + +load)
- [x] 610 tests

### Phase 22 — CF-lite + Foundation Bridge ✅
- [x] `cf-lite` crate: NSLog, CFConstantString, NSObject stubs
- [x] objc_alloc / objc_alloc_init / objc_opt_self shims
- [x] Critical PTR_64_OFFSET chained rebase bug fix
- [x] `objc_foundation` + `objc_classes` fixtures working through runtime
- [x] 628 tests

### Phase 23 — Multi-Image ObjC ✅
- [x] `objc_dylib_chain` fixture (cross-image class + category merge)
- [x] Two-phase ObjC registration (same-image → cross-image)
- [x] Cross-image category merge via post-fixup memory reads
- [x] 632 tests

### Phase 24 — Hardening & Coverage Gaps ✅
- [x] Category class-method merge (both registration phases)
- [x] framework_style runtime test
- [x] Malformed binary rejection tests
- [x] 637 tests

### Phase 25 — ARM64 Trampoline, CFString/NSString, T9 ✅
- [x] ARM64 `objc_msgSend` assembly trampoline (saves x0-x7, d0-d7, tail-calls IMP)
- [x] Selector reference uniquing (`__objc_selrefs` fixup)
- [x] Retain/release/autorelease via RefCountTable
- [x] CFString/NSString with heap-allocated 32-byte struct
- [x] Synthetic class registration (NSObject → NSString → __NSCFConstantString)
- [x] `nsstring_ops` fixture (T9): 7 CFString/NSString checks
- [x] 648 tests

### Phase 26 — Blocks Runtime, T11 ✅
- [x] _NSConcreteGlobalBlock / _NSConcreteStackBlock / _NSConcreteMallocBlock
- [x] _Block_copy (global→identity, stack→heap copy, malloc→refcount bump)
- [x] _Block_release (malloc→decrement+free, others→no-op)
- [x] `block_basics` fixture (T11): 6 block lifecycle checks
- [x] 656 tests

### Phase 27 — Superclass Chain + __objc_stubs, T12 ✅
- [x] Superclass parsing (objc_class_t+8)
- [x] msg_lookup superclass chain traversal (depth-limited to 64)
- [x] __objc_stubs verified working (no new code needed)
- [x] `objc_inherit` fixture (T12): 3-class hierarchy, 12 checks
- [x] 658 tests

### Phase 28 — NSObject Methods + Metaclass Dispatch, T13 ✅
- [x] Metaclass dispatch in msg_lookup (class_methods vs methods)
- [x] NSObject method shims (init, self, class, respondsToSelector:, isKindOfClass:, alloc, new)
- [x] Post-fixup superclass repair
- [x] `nsobject_methods` fixture (T13): NSObject hierarchy, 10 checks
- [x] 660 tests

### Phase 29 — objc_msgSendSuper / objc_msgSendSuper2, T14 ✅
- [x] ARM64 assembly trampolines for super dispatch
- [x] Rust lookup callbacks (super_class direct vs current_class.superclass_addr)
- [x] `objc_super` fixture (T14): 3-level hierarchy with [super method], 12 checks
- [x] 662 tests

### Phase 30 — Real NSString + ObjC Introspection, T15 ✅
- [x] NSString as proper synthetic class with data symbols
- [x] Dynamic NSString (initWithUTF8String:, stringWithUTF8String:, isEqualToString:)
- [x] ObjC introspection (object_getClass, objc_lookUpClass, class_getSuperclass, class_respondsToSelector, sel_isEqual)
- [x] `nsstring_dynamic` fixture (T15): 13 checks
- [x] 664 tests

### Phase 31 — NSLog Variadic Fix + %@ Formatting ✅
- [x] ARM64 variadic calling convention fix (stack-based args, not registers)
- [x] NSLog assembly trampoline
- [x] NSLog capture infrastructure for test verification
- [x] 666 tests

### Phase 32 — static-mut Elimination + Warning Cleanup ✅
- [x] Replaced all `static mut` with OnceLock/Mutex/AtomicBool
- [x] 0 compiler warnings across workspace
- [x] 666 tests

### Phase 33 — NSArray + T16 ✅
- [x] NSArray synthetic class (count, objectAtIndex:, firstObject, lastObject, containsObject:, description, arrayWithObjects:count:)
- [x] Content-aware containsObject: (pointer + string comparison)
- [x] `nsarray_basics` fixture (T16): 18 checks
- [x] 674 tests

### Unnumbered — NSDictionary, NSMutableArray, NSMutableDictionary, NSNumber, NSString Extras, NSLog Format Specifiers, Code Quality ✅
- [x] NSDictionary (dictionaryWithObjects:forKeys:count:, objectForKey:, count, allKeys, description)
- [x] NSMutableArray (mutableArray factory, addObject:, removeObjectAtIndex:, insertObject:atIndex:, replaceObjectAtIndex:withObject:, removeLastObject, removeAllObjects, count)
- [x] NSMutableDictionary (dictionary factory, setObject:forKey:, removeObjectForKey:, removeAllObjects, setObject:forKeyedSubscript:)
- [x] NSNumber (numberWithInt:, numberWithDouble:, numberWithBool:, intValue, doubleValue, boolValue, isEqualToNumber:, description)
- [x] NSString extras (stringWithFormat:, hasPrefix:, hasSuffix:, stringByAppendingString:, substringFromIndex:, substringToIndex:, copy, compare:, lowercaseString, uppercaseString, hash)
- [x] NSLog format specifiers (%d, %ld, %lld, %llu, %s, %f, %lf, %e, %x, %X, %o, %p, %zu, %zd, %%)
- [x] 14 fixture binaries for Foundation types
- [x] Workspace lint infrastructure (clippy, unused_must_use deny, manual_let_else)
- [x] Silenced error audit (6 fixes: bind.rs, resolve/mod.rs, parser/mod.rs, codesig.rs, decode.rs)
- [x] Export trie STUB_AND_RESOLVER bug fix
- [x] File decomposition (9 large files split into modules)
- [x] 741 tests, 0 failures, 0 clippy warnings

### Unnumbered — NSMutableString, Autorelease Model, Runtime Introspection ✅
- [x] NSMutableString class (appendString:, insertString:atIndex:, deleteCharactersInRange:, replaceCharactersInRange:withString:, setString:)
- [x] NSString completeness (6 new methods: rangeOfString:, componentsSeparatedByString:, stringByReplacingOccurrencesOfString:withString:, integerValue, doubleValue, mutableCopy)
- [x] T21 nsstring_mutable fixture
- [x] ARC return-value shims (autoreleaseReturnValue, retainAutoreleasedReturnValue, unsafeClaimAutoreleasedReturnValue, retainAutorelease)
- [x] NSObject memory methods (retain, release, autorelease, retainCount, dealloc)
- [x] CFRelease fixed to free memory on zero refcount
- [x] T22 autorelease fixture
- [x] 11 runtime introspection shims (class_addMethod, class_getInstanceMethod, class_copyMethodList, method_getName, method_getTypeEncoding, method_getImplementation, method_getNumberOfArguments, sel_getName, class_getMethodImplementation, class_getInstanceSize, class_isMetaClass)
- [x] lookup_method_entry() — full MethodEntry lookup with superclass chain walk
- [x] ObjcRuntime mutation APIs (get_class_mut, get_class_by_addr_mut, add_method_to_class)
- [x] T23 runtime_introspection fixture — 11 introspection APIs exercised end-to-end
- [x] 756 tests, 0 failures, 0 clippy warnings

### Unnumbered — Property & Ivar Introspection ✅
- [x] Property metadata parsing from class_ro_t offset 64 (baseProperties) — ObjcPropertyInfo { name, attributes }
- [x] Ivar metadata parsing from class_ro_t offset 48 (ivars) — ObjcIvarInfo { name, type_encoding, offset, size, alignment }
- [x] IvarEntry + PropertyEntry added to ClassInfo, populated during register_image
- [x] 11 new shims: class_copyIvarList, class_getInstanceVariable, ivar_getName, ivar_getOffset, ivar_getTypeEncoding, class_copyPropertyList, class_getProperty, property_getName, property_getAttributes, object_getIvar, object_setIvar
- [x] Fake Ivar struct (32 bytes: name_ptr, type_encoding_ptr, offset, size) and Property struct (16 bytes: name_ptr, attributes_ptr)
- [x] T19 property_introspection fixture — @property, ivar access, getter/setter dispatch, 3 ivars, 2 properties
- [x] 759 tests, 0 failures, 0 clippy warnings

### Unnumbered — NSData, NSProcessInfo, NSString Path Utilities ✅
- [x] NSData class (dataWithBytes:length:, data, length, bytes, description, isEqualToData:) — 24-byte layout (isa/length/bytes_ptr)
- [x] NSProcessInfo class (processInfo singleton, processName, processIdentifier, operatingSystemVersionString, arguments, environment, hostName, description)
- [x] NSString path utility methods (pathExtension, lastPathComponent, stringByDeletingLastPathComponent, stringByDeletingPathExtension, stringByAppendingPathComponent:, isAbsolutePath)
- [x] `alloc_nsstring_from_cstr()` convenience helper in cfstring module
- [x] `process_name()` public accessor in libsystem-shims
- [x] 2 new synthetic class pairs (NSData, NSProcessInfo) with full registration pipeline
- [x] 4 new Foundation data symbols (class + metaclass for NSData and NSProcessInfo)
- [x] T24 nsdata_basics fixture — 7 NSData checks
- [x] T25 nsstring_path fixture — 8 path utility checks
- [x] T26 nsprocessinfo_basics fixture — 6 NSProcessInfo checks
- [x] 765 tests, 0 failures, 0 clippy warnings

### Unnumbered — Blocks + Dispatch Integration (T20) ✅
- [x] `dispatch_sync` shim (single-threaded inline block execution)
- [x] `dispatch_get_main_queue` data symbol (`DISPATCH_MAIN_Q` — fake 8-byte queue address)
- [x] `enumerateObjectsUsingBlock:` on NSArray — iterates elements, calls block with (obj, idx, stop), supports early stop
- [x] `sortedArrayUsingComparator:` on NSArray — copies elements, sorts with comparator block, returns new immutable array
- [x] `sortUsingComparator:` on NSMutableArray — in-place sort with comparator block
- [x] `enumerateKeysAndObjectsUsingBlock:` on NSDictionary — iterates key-value pairs, calls block with (key, obj, stop)
- [x] All block methods registered on mutable subclasses (NSMutableArray, NSMutableDictionary) via register.rs
- [x] `_Unwind_Resume` + `__objc_personality_v0` added to StubResolver safe passthrough list
- [x] T20 `blocks_foundation` fixture — 15 checks covering all new methods + dispatch_sync
- [x] 767 tests, 0 failures, 0 clippy warnings

### Unnumbered — NSError, NSBundle, NSFileManager ✅
- [x] NSError class (errorWithDomain:code:userInfo:, initWithDomain:code:userInfo:, domain, code, userInfo, localizedDescription, description)
- [x] NSBundle class (mainBundle singleton, bundlePath, resourcePath, executablePath, bundleIdentifier)
- [x] NSFileManager class (defaultManager singleton, fileExistsAtPath:, fileExistsAtPath:isDirectory:, currentDirectoryPath, contentsOfDirectoryAtPath:error:)
- [x] 3 new synthetic class pairs (NSError, NSBundle, NSFileManager) with full registration pipeline
- [x] 6 new Foundation data symbols (class + metaclass for NSError, NSBundle, NSFileManager)
- [x] `process_path()` public accessor in libsystem-shims
- [x] T27 nserror_basics fixture — 6 NSError checks
- [x] T28 nsbundle_basics fixture — 5 NSBundle checks
- [x] T29 nsfilemanager_basics fixture — 6 NSFileManager checks
- [x] 773 tests, 0 failures, 0 clippy warnings

### Unnumbered — NSException + ObjC Exception Shims ✅
- [x] NSException class (exceptionWithName:reason:userInfo:, initWithName:reason:userInfo:, name, reason, userInfo, description, raise)
- [x] 32-byte layout (isa=0, name=8, reason=16, userInfo=24)
- [x] ObjC exception shims: objc_exception_throw, objc_begin_catch, objc_end_catch, objc_exception_rethrow (all delegate to host libobjc via dlsym)
- [x] objc_terminate shim (abort with message)
- [x] _OBJC_EHTYPE_$_NSException resolved from host Foundation via dlopen+dlsym (for @catch type matching)
- [x] NSException class pair (class + metaclass data symbols) registered in Foundation shims
- [x] Full registration pipeline: cf-lite → objc-runtime → macho-runtime Phase D
- [x] T30 exception_basics fixture — 6 NSException class method checks (creation, property access, description)
- [x] Note: @throw/@catch through mapped code requires unwind info registration (future work)
- [x] 775 tests, 0 failures, 0 clippy warnings

### Unnumbered — Real CLI Integration (T31) ✅
- [x] `real_cli` fixture: realistic "directory reporter" CLI exercising 13 Foundation classes together
- [x] Cohesive workflow: process identity → file system scan → config dictionary → mutable file list → sorted output → string manipulation → enumeration with blocks → error handling → path utilities → statistics → report building
- [x] NSString (format, path utils, case, compare, split), NSMutableString (report building), NSArray (creation, enumeration, sorting blocks), NSMutableArray (addObject, sorted), NSDictionary (creation, lookup), NSMutableDictionary (set/remove), NSNumber (int, double, bool roundtrip), NSData (bytes, length), NSProcessInfo (name, PID), NSFileManager (exists, cwd, contentsOfDirectory, error propagation), NSBundle (mainBundle, bundlePath), NSError (factory, code, localizedDescription), NSException (factory, name, reason)
- [x] Compatibility scoring: all 7 dimensions green — Parse ✅ Load ✅ Fixups ✅ Enter main ✅ Complete ✅ Output ✅ Coverage ✅
- [x] All 26 checks pass through loader on first attempt — zero gaps
- [x] 777 tests, 0 failures, 0 clippy warnings

### Unnumbered — Collection Enumeration + Advanced Foundation ✅
- [x] NSEnumerator class (nextObject, allObjects) — 24-byte layout (isa/source/index), iterates NSArray elements
- [x] objectEnumerator on NSArray + NSMutableArray, keyEnumerator on NSDictionary + NSMutableDictionary
- [x] NSDate class — 16-byte layout (isa + f64 CF absolute time), 8 instance methods + 3 class methods
- [x] Core Foundation epoch offset (978307200s between Unix 1970 and CF 2001 reference dates)
- [x] Civil calendar date formatting via days_to_ymd algorithm for description output
- [x] NSUserDefaults class — singleton with in-memory NSMutableDictionary backing store
- [x] Typed accessors (stringForKey:, integerForKey:, boolForKey:, doubleForKey:) with ISA-based type detection
- [x] T32 collection_enum, T33 nsdate_basics, T34 nsuserdefaults_basics fixtures — 25 checks total
- [x] 783 tests, 0 failures, 0 clippy warnings

### Unnumbered — NSSet, NSMutableSet, NSURL, NSNull ✅
- [x] NSSet class (setWithObjects:count:, set, setWithArray:, count, containsObject:, member:, allObjects, anyObject, isSubsetOfSet:, intersectsSet:, isEqualToSet:, description, enumerateObjectsUsingBlock:) — 24-byte layout (isa/count/elements_ptr), deduplication on insert
- [x] NSMutableSet class (set, setWithCapacity:, addObject:, removeObject:, removeAllObjects, unionSet:, intersectSet:, minusSet:) — inherits NSSet read methods, mutable backing array
- [x] NSURL class (fileURLWithPath:, URLWithString:, absoluteString, path, lastPathComponent, pathExtension, URLByAppendingPathComponent:, isFileURL, scheme, description) — 24-byte layout (isa/string_ptr/is_file_url)
- [x] NSNull class (null singleton, description) — 8-byte singleton via OnceLock, returns "<null>"
- [x] 4 new synthetic class pairs with full registration pipeline
- [x] 8 new Foundation data symbols (class + metaclass for each)
- [x] T35 nsset_basics fixture — 17 checks (set creation, dedup, containsObject, member, allObjects, anyObject, isSubsetOfSet, intersectsSet, isEqualToSet, mutable add/remove/union/removeAll, empty, setWithArray)
- [x] T36 nsurl_basics fixture — 10 checks (fileURLWithPath, isFileURL, path, lastPathComponent, pathExtension, URLByAppendingPathComponent, URLWithString, scheme, absoluteString)
- [x] T37 nsnull_basics fixture — 6 checks (singleton, identity, description, NSNull in array, NSNull in dictionary)
- [x] 789 tests, 0 failures, 0 clippy warnings

</details>

---

## Architecture

### Runtime Stack

```
loader-core        = macho-types + macho-parse + macho-map + macho-resolve
                     + macho-objc + macho-exec (evolves as real binaries
                     demand new Mach-O / dyld features)

objc-runtime       = classes / selectors / dispatch / super / categories
                     / blocks / retain-release / introspection

foundation-lite    = cf-lite (NSString, NSArray, NSDictionary, NSNumber,
                     NSMutableArray, NSMutableDictionary, NSMutableString,
                     NSData, NSProcessInfo, NSError, NSBundle,
                     NSFileManager, NSException, NSEnumerator, NSDate,
                     NSUserDefaults, NSSet, NSMutableSet, NSURL, NSNull,
                     NSObject)

libsystem-shims    = pthread / dispatch / process / TLS / lifecycle / mach

policy-layer       = trust-policy (optional — permissive is the default
                     development path; Apple-like mode for comparison only)

runtime-orchestrator = macho-runtime (end-to-end load_and_run pipeline)

gui-framework      = graphics-core + window-backend + window-backend-darwin
                     + runloop-core + app-kit-lite + ui-view + ui-controls

demo-apps          = hello-darwin, control-gallery, foundation-explorer
                     (3 GUI apps in apps/ directory)
```

### Development Rules

1. **Any layer may evolve** when real software requires it — loader, runtime, Foundation, GUI
2. **Every new feature** must unlock or stabilize a real fixture/app/binary pattern
3. **Target-driven**: a real binary or fixture that fails without the feature and passes with it
4. **Permissive mode stays the default** — trust-policy is not the main product surface
5. **Host dependency shrinks over time** — replace host fallbacks with native implementations

---

## Milestone

> **A complete macOS-compatible runtime platform capable of running real
> software independently of Apple's OS.**

Progress is measured by:
* real programs running
* host dependency shrinking
* platform capabilities expanding

---

## Target Matrix (Forward)

| # | Target | What it exercises | Status |
|---|---|---|---|
| **T0–T16** | (see completed phases) | Loader + runtime fundamentals | ✅ done |
| **T17** | `nsstring_format` | `stringWithFormat:` + mixed specifiers | ✅ done |
| **T18** | Collection-heavy Foundation CLI | NSArray + NSDictionary + NSMutableArray + NSMutableDict interop | ✅ done |
| **T19** | Property-backed ObjC class | `@property`, ivar access, setter/getter dispatch | ✅ done |
| **T20** | Block-using ObjC/Foundation binary | Blocks inside Foundation workflows (e.g. enumerateObjectsUsingBlock:) | ✅ done |
| **T21** | File/path/process-info Foundation binary | NSBundle, NSProcessInfo, NSFileManager basics | ✅ done |
| **T22** | Exception/error-handling binary | NSError, NSException class methods, exception shims | ✅ done (class methods; @throw/@catch needs unwind registration) |
| **T23** | Real Foundation CLI app | 13 Foundation classes, blocks, file I/O, sorting, enumeration, errors, paths | ✅ done |
| **T24** | Collection enumeration + NSDate + NSUserDefaults | NSEnumerator, NSDate arithmetic/comparison, NSUserDefaults in-memory store | ✅ done |
| **T25** | NSSet, NSURL, NSNull | Set collections, URL handling, null placeholder | ✅ done |

---

## Next Task Order

### 1. NSMutableString + NSString completeness
- [x] `NSMutableString` class (appendString:, insertString:atIndex:, deleteCharactersInRange:, replaceCharactersInRange:withString:, setString:)
- [x] NSString `stringByReplacingOccurrencesOfString:withString:`
- [x] NSString `componentsSeparatedByString:`
- [x] NSString `rangeOfString:` (NSRange return)
- [x] NSString `integerValue`, `doubleValue`, `mutableCopy`
- [x] Fixture: T21 nsstring_mutable — dynamic string manipulation heavy CLI

### 2. Autorelease + ownership model
- [x] ARC return-value optimization shims (autoreleaseReturnValue, retainAutoreleasedReturnValue, unsafeClaimAutoreleasedReturnValue, retainAutorelease)
- [x] NSObject memory methods (retain, release, autorelease, retainCount, dealloc)
- [x] CFRelease fixed to free memory on zero refcount
- [x] Object lifetime tests across autoreleasepool, nested pools, retain-across-pool-drain
- [x] Fixture: T22 autorelease — MRC retain/release/autorelease + nested pool exercise

### 3. Property / introspection / runtime helpers ✅
- [x] `class_addMethod` (runtime method addition)
- [x] `class_getInstanceMethod`, `class_copyMethodList`, `class_getMethodImplementation`
- [x] `method_getName`, `method_getTypeEncoding`, `method_getImplementation`, `method_getNumberOfArguments`
- [x] `sel_getName`
- [x] `class_getInstanceSize`, `class_isMetaClass`
- [x] `lookup_method_entry()` — full MethodEntry lookup with superclass chain walk
- [x] Fixture: T23 runtime_introspection — 11 introspection APIs exercised end-to-end
- [x] `@property` metadata parsing from `class_ro_t` offset 64 (baseProperties)
- [x] Ivar metadata parsing from `class_ro_t` offset 48 (ivars)
- [x] `IvarEntry` + `PropertyEntry` in ClassInfo, populated from binary + registration
- [x] `class_copyIvarList`, `class_getInstanceVariable`, `ivar_getName`, `ivar_getOffset`, `ivar_getTypeEncoding`
- [x] `class_copyPropertyList`, `class_getProperty`, `property_getName`, `property_getAttributes`
- [x] `object_getIvar`, `object_setIvar` — direct memory access via ivar offset
- [x] Fixture: T19 property_introspection — @property, ivar access, getter/setter dispatch

### 4. NSData / NSError / file-path utilities
- [x] `NSData` class (dataWithBytes:length:, data, length, bytes, description, isEqualToData:)
- [x] `NSError` class (domain, code, userInfo, localizedDescription, description, errorWithDomain:code:userInfo:, initWithDomain:code:userInfo:)
- [x] `NSBundle` (mainBundle, bundlePath, resourcePath, executablePath, bundleIdentifier)
- [x] `NSProcessInfo` (processInfo, processName, processIdentifier, operatingSystemVersionString, arguments, environment, hostName, description)
- [x] `NSFileManager` (defaultManager, fileExistsAtPath:, fileExistsAtPath:isDirectory:, currentDirectoryPath, contentsOfDirectoryAtPath:error:)
- [x] Path utilities (NSString pathExtension, lastPathComponent, stringByAppendingPathComponent:, stringByDeletingLastPathComponent, stringByDeletingPathExtension, isAbsolutePath)
- [x] Fixtures: T24 nsdata_basics, T25 nsstring_path, T26 nsprocessinfo_basics, T27 nserror_basics, T28 nsbundle_basics, T29 nsfilemanager_basics

### 5. Blocks + dispatch integration ✅
- [x] Stack-to-heap block copy correctness with captured variables
- [x] Captured variable behavior (byref vs byval)
- [x] dispatch_async / dispatch_sync basics (single-threaded execution)
- [x] Queue semantics sufficient for simple apps (dispatch_get_main_queue data symbol)
- [x] Blocks inside Foundation workflows (enumerateObjectsUsingBlock:, sortedArrayUsingComparator:, sortUsingComparator:, enumerateKeysAndObjectsUsingBlock:)
- [x] Fixture: block-using ObjC/Foundation binary (T20)

### 6. Exception / error handling ✅
- [x] NSException class (name, reason, userInfo, description, raise, exceptionWithName:reason:userInfo:)
- [x] ObjC exception shims (objc_exception_throw, objc_begin_catch, objc_end_catch, objc_exception_rethrow, objc_terminate)
- [x] _OBJC_EHTYPE_$_NSException from host Foundation (for @catch type matching)
- [x] Unhandled exception → abort with message (objc_terminate)
- [x] Fixture: T30 exception_basics — NSException class method checks
- [x] @try/@catch/@finally through mapped code (compact unwind registration via `__unw_add_find_dynamic_unwind_sections`)
- [x] Unwind section callback: registers mapped images' `__unwind_info` and `__eh_frame` with system unwinder
- [x] Host NSException translation: cf-lite exceptions converted to host NSException before throwing (fixes ISA incompatibility)
- [x] Host objc_msgSend fallback with selector translation: translates our selectors to host canonical pointers (ObjC uses pointer equality)
- [x] Tagged pointer dispatch: arm64 tagged pointers (bit 63 set) forwarded to host objc_msgSend
- [x] Unwind cleanup on MappedImage drop: prevents stale registrations from interfering with subsequent loads
- [x] Fixture: try_catch_test — basic @try/@catch, @try/@catch/@finally, nested @try/@catch with re-throw

### 7. Real external CLI target ✅
- [x] `real_cli` fixture: realistic "directory reporter" CLI tool exercising 13 Foundation classes in a cohesive workflow
- [x] Classes exercised: NSString, NSMutableString, NSArray, NSMutableArray, NSDictionary, NSMutableDictionary, NSNumber, NSData, NSProcessInfo, NSFileManager, NSBundle, NSError, NSException
- [x] Features exercised: blocks (sorting comparator, enumerate), path utilities, file existence, directory listing, error propagation, string splitting, case transform, mutable containers, @autoreleasepool
- [x] Compatibility scoring: Parse ✅ | Load ✅ | Fixups ✅ | Enter main ✅ | Complete execution ✅ | Output correctness ✅ (26/26 checks) | Runtime feature coverage ✅
- [x] All 26 checks pass through loader on first attempt — no gaps discovered
- [x] Fixture: T31 real_cli — 26-check comprehensive Foundation integration test

### 8. Collection enumeration + advanced Foundation ✅
- [x] `NSEnumerator` class (nextObject, allObjects) — 24-byte layout (isa/source/index)
- [x] `objectEnumerator` on NSArray + NSMutableArray returning NSEnumerator
- [x] `keyEnumerator` on NSDictionary + NSMutableDictionary returning NSEnumerator
- [x] `enumerateObjectsUsingBlock:` on NSArray
- [x] `enumerateKeysAndObjectsUsingBlock:` on NSDictionary
- [x] `NSDate` class (date, dateWithTimeIntervalSince1970:, dateWithTimeIntervalSinceReferenceDate:, timeIntervalSince1970, timeIntervalSinceReferenceDate, timeIntervalSinceNow, timeIntervalSinceDate:, dateByAddingTimeInterval:, isEqualToDate:, compare:, description)
- [x] Civil calendar date formatting in description (days_to_ymd algorithm)
- [x] `NSUserDefaults` class (standardUserDefaults singleton, objectForKey:, stringForKey:, integerForKey:, boolForKey:, doubleForKey:, setObject:forKey:, removeObjectForKey:, synchronize)
- [x] In-memory backing store via NSMutableDictionary
- [x] 3 new synthetic class pairs (NSEnumerator, NSDate, NSUserDefaults)
- [x] 6 new Foundation data symbols (class + metaclass for each)
- [x] T32 collection_enum fixture — 6 NSEnumerator checks (objectEnumerator, keyEnumerator, nextObject, allObjects)
- [x] T33 nsdate_basics fixture — 10 NSDate checks (creation, roundtrip, arithmetic, comparison, description)
- [x] T34 nsuserdefaults_basics fixture — 9 NSUserDefaults checks (singleton, get/set, typed access, remove, synchronize)
- [x] 783 tests, 0 failures, 0 clippy warnings

### 9. NSSet, NSMutableSet, NSURL, NSNull ✅
- [x] `NSSet` class (setWithObjects:count:, set, setWithArray:, count, containsObject:, member:, allObjects, anyObject, isSubsetOfSet:, intersectsSet:, isEqualToSet:, description, enumerateObjectsUsingBlock:) — 24-byte layout with deduplication
- [x] `NSMutableSet` class (set, setWithCapacity:, addObject:, removeObject:, removeAllObjects, unionSet:, intersectSet:, minusSet:)
- [x] `NSURL` class (fileURLWithPath:, URLWithString:, absoluteString, path, lastPathComponent, pathExtension, URLByAppendingPathComponent:, isFileURL, scheme, description) — 24-byte layout
- [x] `NSNull` class (null singleton via OnceLock, description) — 8-byte singleton
- [x] 4 new synthetic class pairs, 8 new Foundation data symbols
- [x] T35 nsset_basics (17 checks), T36 nsurl_basics (10 checks), T37 nsnull_basics (6 checks)
- [x] 789 tests, 0 failures, 0 clippy warnings

### 10. NSDateFormatter, NSCharacterSet, NSNotificationCenter ✅
- [x] `NSDateFormatter` class (init, setDateFormat:/dateFormat, setDateStyle:/dateStyle, setTimeStyle:/timeStyle, stringFromDate:) — 32-byte layout, Unicode TR35 format subset + style-based formatting, UTC decomposition
- [x] `NSCharacterSet` class (whitespaceCharacterSet, whitespaceAndNewlineCharacterSet, decimalDigitCharacterSet, letterCharacterSet, lowercaseLetterCharacterSet, uppercaseLetterCharacterSet, alphanumericCharacterSet, punctuationCharacterSet, newlineCharacterSet, characterSetWithCharactersInString:, characterIsMember:) — 32-byte layout with predefined set kinds + custom sorted codepoint arrays
- [x] `NSNotificationCenter` class (defaultCenter singleton, postNotificationName:object:, postNotificationName:object:userInfo:, addObserver:selector:name:object:, removeObserver:, removeObserver:name:object:) — no-op stubs for binary compatibility
- [x] 3 new NSString methods: stringByTrimmingCharactersInSet:, componentsSeparatedByCharactersInSet:, rangeOfCharacterFromSet:
- [x] 3 new synthetic class pairs, 6 new Foundation data symbols (57 macOS / 56 non-macOS)
- [x] T38 nsdateformatter_basics (8 checks), T39 nscharacterset_basics (10 checks), T40 nsnotification_basics (6 checks)
- [x] 795 tests, 0 failures, 0 clippy warnings

### 11. NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar
- [x] `NSValue` class — wraps non-object types (pointers, raw bytes); valueWithBytes:objCType:, valueWithRange:, valueWithPointer:, rangeValue, pointerValue, objCType, description
- [x] `NSUUID` class — UUID generation and string representation; UUID, initWithUUIDString:, UUIDString, isEqual:, description
- [x] `NSLocale` class — locale identity and properties; currentLocale singleton, localeIdentifier, languageCode, countryCode, displayNameForKey:value:, objectForKey:, description
- [x] `NSTimeZone` class — time zone representation; systemTimeZone, localTimeZone singletons, timeZoneWithName:, timeZoneWithAbbreviation:, timeZoneForSecondsFromGMT:, name, abbreviation, secondsFromGMT, description
- [x] `NSCalendar` class — calendrical calculations; currentCalendar singleton, calendarWithIdentifier:, components:fromDate:, dateFromComponents:, dateByAddingComponents:toDate:options:, component:fromDate:, calendarIdentifier, description
- [x] 5 new synthetic class pairs (30 total), 10 new Foundation data symbols (67 macOS / 66 non-macOS)
- [x] Fixtures: nsvalue_basics, nsuuid_basics, nslocale_basics, nstimezone_basics, nscalendar_basics
- [x] 805 tests, 0 failures, 0 clippy warnings, 148 shims (macOS), 30 fixtures

### 12. NSNumberFormatter, NSIndexSet, NSSortDescriptor, NSTimer ✅
- [x] `NSNumberFormatter` class — locale-aware number formatting; numberFromString:, stringFromNumber:, setNumberStyle:, numberStyle, setMinimumFractionDigits:, setMaximumFractionDigits:, description
- [x] `NSIndexSet` / `NSMutableIndexSet` classes — sorted integer range sets; indexSetWithIndex:, indexSetWithIndexesInRange:, count, containsIndex:, firstIndex, lastIndex, addIndex:, addIndexesInRange:, removeIndex:, removeAllIndexes, description
- [x] `NSSortDescriptor` class — key-path-based sort criteria; sortDescriptorWithKey:ascending:, key, ascending, compareObject:toObject:, reversedSortDescriptor, description
- [x] `NSTimer` class — single-fire timer with block; scheduledTimerWithTimeInterval:repeats:block:, timerWithTimeInterval:repeats:block:, fire, invalidate, isValid, timeInterval, description
- [x] 5 new synthetic class pairs (35 total), 10 new Foundation data symbols (158 shims macOS / 157 non-macOS)
- [x] Fixtures: nsnumberformatter_basics, nsindexset_basics, nssortdescriptor_basics, nstimer_basics (34 total)
- [x] 813 tests passing, 0 clippy warnings

---

## Observability

### Runtime tracing
Structured tracing for debugging and alignment via `RuntimeTracer`:
- [x] Image load order
- [x] Selector registration
- [x] Class/category merge events
- [x] Method lookup path (hit/miss/super chain) — `log::trace!` in dispatch.rs
- [x] Block lifecycle (copy/release) — covered by existing `log::debug!` in lifecycle shims
- [x] Retain/release traffic — covered by existing `log::debug!` in ARC shims
- [x] Foundation object creation — traced via `EventKind::ClassRegistration` for 35 synthetic classes

---

## Out of Scope (current phase)

Not targeted in the immediate term but part of the forward roadmap (Phases 39–52):

| Area | Forward Phase |
|---|---|
| Full kernel / Darwin recreation | Out of scope — this is a userspace runtime |
| Full notarization / security parity | Policy layer is optional/comparison-only |
| Custom window server / graphics stack | Phase 52 — long-term host independence |

Previously out-of-scope items now part of the open core roadmap:

| Area | Phase |
|---|---|
| KVO / KVC / NSInvocation | Phase 42 — Foundation Growth |
| PAC / ARM64e pointer auth | Phase 40 — Loader Expansion |
| Method swizzling | Phase 41 — Runtime Expansion |
| GUI / AppKit expansion | Phase 45 — GUI Framework Expansion |
| Platform independence | Phase 43–44 — Platform Abstraction + Host Reduction |

---

## Forward Roadmap — Phases 39–52

### Phase 39 — Real Software Corpus ✅

**Completed.** Created `compat-runner` crate and 12-program corpus. All 12 pass (100%).

Goal: move beyond fixture tests. Create a **real application test corpus**.

| Category | Examples |
|---|---|
| CLI tools | ripgrep, jq, fd |
| ObjC utilities | small Cocoa tools |
| Foundation-heavy tools | plist processors |
| GUI apps | tiny Cocoa samples |
| system utilities | file tools, network tools |

Add crate: `compat-runner`

Features:
- runs binaries under runtime
- records missing symbols/classes/selectors
- stores reports

Output: `compatibility-report.json`

Success criteria: **10 real binaries run successfully** ✅ 12/12 pass

#### Deliverables
- `crates/compat-runner/` — library + `darwin-compat` CLI binary
- `corpus/` — 12 programs: word_counter, json_formatter, file_lister, string_processor, config_reader, date_calculator, path_resolver, collection_pipeline, error_reporter, runtime_info, todo_manager, text_analyzer
- `corpus/build_corpus.sh` — builds all corpus programs
- 8 integration tests including regression gate (≥10 must pass)
- Host class resolution for `_OBJC_CLASS_$_NSConstantArray` and `_OBJC_CLASS_$_NSConstantDictionary` (for `@[]`/`@{}` literals)

---

### Phase 40 — Loader Expansion (when required) ✅

**Completed.** Five loader improvements driven by real-binary analysis.

Goal: evolve loader as real binaries demand.

#### Mach-O features
- new load commands
- relocation formats
- dyld shared cache behaviors
- stub-and-resolver symbols
- new export trie variants
- ARM64e pointer authentication expansion

#### Loader improvements
- lazy binding correctness
- TLS improvements
- initializer edge cases
- symbol interposition

Success criteria: **real-world binaries load reliably** ✅

#### Deliverables
1. **Weak bind application** — `fixups.weak_binds` now applied in `apply_binds()` alongside regular and lazy binds (previously silently skipped → null GOT slots)
2. **NSConstantIntegerNumber host class** — `_OBJC_CLASS_$_NSConstantIntegerNumber` resolved via host `dlsym` for `@(int)` literal support (joins NSConstantArray/NSConstantDictionary)
3. **StubResolver safe-list expansion** — 40+ new symbols: `pthread_cond_*`, `pthread_rwlock_*`, `pthread_mutex_trylock`, ObjC runtime introspection (`class_*`, `sel_*`, `method_*`, `ivar_*`, `property_*`), GCD (`dispatch_async/sync/queue_create`), blocks (`_NSConcreteStackBlock/GlobalBlock`), atomics (`os_unfair_lock_*`), system (`sysctl*`, `arc4random*`, `atexit`, `_tlv_bootstrap`)
4. **Entry point argv/envp propagation** — `call_entry_with_args()` passes `argc`, `argv[]`, `envp[]` from `RuntimeConfig` (previously hardcoded `argc=1, argv=["app"]`)
5. **`__TEXT_EXEC` unwind scan** — `register_unwind_info()` now scans both `__TEXT` and `__TEXT_EXEC` segments for `__unwind_info` / `__eh_frame` sections

---

### Phase 41 — Runtime Expansion ✅

Goal: extend ObjC runtime when needed.

#### ObjC runtime gaps — DONE
- ✅ `objc_alloc_init` — now dispatches `-init` after alloc
- ✅ Associated objects (`objc_setAssociatedObject`, `objc_getAssociatedObject`, `objc_removeAssociatedObjects`) — per-object key-value store with RETAIN/COPY/ASSIGN policies
- ✅ Weak references (`objc_initWeak`, `objc_destroyWeak`, `objc_loadWeakRetained`, `objc_storeWeak`, `objc_loadWeak`, `objc_copyWeak`, `objc_moveWeak`) — weak reference table
- ✅ `object_setClass` — swaps ISA, returns old class
- ✅ `class_getClassMethod` — class method lookup returning fake Method struct
- ✅ IMP caching — `method_index`/`class_method_index` HashMaps on ClassInfo for O(1) selector→IMP lookup; dispatch, category merge, `class_addMethod`, and `class_replaceMethod` all maintain indices

#### Shim count
- ObjC shims: 66 → 78 (+12 new shims)
- Total shims: 235 → 247 (macOS)

#### Test results
- 961 tests, 0 failures
- 155 runtime tests (63 fixtures, 12 corpus) all pass
- 0 clippy errors

---

### Phase 42 — Foundation Growth ✅

Goal: extend Foundation as required by real programs.

#### NSNumber — 10 new factory methods, 10 new accessors
- ✅ Factory methods: `numberWithInteger:`, `numberWithLong:`, `numberWithLongLong:`, `numberWithFloat:`, `numberWithUnsignedInt:`, `numberWithUnsignedInteger:`, `numberWithShort:`, `numberWithChar:`, `numberWithUnsignedChar:`
- ✅ Accessors: `longLongValue`, `longValue`, `floatValue`, `unsignedIntegerValue`, `unsignedIntValue`, `shortValue`, `charValue`, `unsignedCharValue`, `stringValue`
- ✅ `isEqual:` — delegates to `isEqualToNumber:` for NSNumber objects

#### NSString — 7 new methods
- ✅ `containsString:` — substring search
- ✅ `substringWithRange:` — extract substring by NSRange
- ✅ `characterAtIndex:` — UTF-16 code unit access
- ✅ `initWithString:` — copy constructor
- ✅ `capitalizedString` — title-case transformation
- ✅ `caseInsensitiveCompare:` — case-insensitive ordering
- ✅ `isEqual:` — delegates to `isEqualToString:`

#### NSArray — 4 new methods
- ✅ `indexOfObject:` — linear scan returning index or NSNotFound
- ✅ `isEqualToArray:` — element-wise equality
- ✅ `arrayByAddingObject:` — returns new array with element appended
- ✅ `subarrayWithRange:` — extract subarray by NSRange

#### NSMutableArray — 3 new methods
- ✅ `removeObject:` — remove by value (all occurrences)
- ✅ `addObjectsFromArray:` — batch append from another array
- ✅ `exchangeObjectAtIndex:withObjectAtIndex:` — swap elements

#### NSUserDefaults — 7 new methods
- ✅ Typed setters: `setInteger:forKey:`, `setBool:forKey:`, `setDouble:forKey:`, `setFloat:forKey:`
- ✅ Typed getters: `floatForKey:`, `arrayForKey:`, `dictionaryForKey:`

#### NSDateFormatter — 1 new method
- ✅ `dateFromString:` — parse date string using format pattern (simplified TR35 parser)

#### NSMutableDictionary — 1 new method
- ✅ `addEntriesFromDictionary:` — batch-merge from another dictionary

#### Test results
- 961 tests, 0 failures
- 155 runtime tests (63 fixtures, 12 corpus) all pass
- 0 clippy errors

---

### Phase 43 — Platform Abstraction Layer ✅

Goal: remove direct host dependencies by routing through existing PAL traits.

Deliverables:
- **Unwind deduplication**: removed 150+ lines of duplicate FFI/globals/callback from `macho-exec/src/unwind.rs`; now delegates to `UnwindRegistry` trait via `platform-darwin-host`
  - `MappedImage` stores `Arc<dyn UnwindRegistry>`, Drop calls `registry.unregister()`
  - `ImageRegistry` accepts and threads `Arc<dyn UnwindRegistry>`, registers unwind in `load()`
  - `NoopUnwindRegistry` added to `platform-core` for tests
- **HostSymbolResolver injection**: replaced 7 raw `libc::dlsym` calls in `objc-runtime/src/shims/lifecycle.rs` with `resolve_host_symbol()` helper that queries the global `HostSymbolResolver`
  - `objc_runtime::set_host_symbols()` / `host_symbols()` global
  - Fallback to raw dlsym if resolver not initialized (backward compat)
- **PAL trait expansion**: added `ProcessInfo::is_main_thread()` and `Clock::sleep_secs()`
  - Implemented in `DarwinProcessInfo` (via `pthread_main_np`) and `DarwinClock` (via `nanosleep`)
  - `cf-lite/nsthread.rs` now routes through PAL instead of direct `libc::pthread_main_np()` and `libc::nanosleep()`
- **Deferred**: macho-resolve filesystem threading (low-priority, only 2 `std::fs::read` sites, would require 40+ caller updates)

Test results: 961 passed, 0 failures, 0 clippy errors

---

### Phase 43b — Allocator PAL ✅

Goal: abstract heap allocation (calloc/malloc/realloc/free) behind the Platform Abstraction Layer.

Deliverables:
- **Allocator trait**: `platform_core::alloc::Allocator` with `calloc`, `malloc`, `realloc`, `free` methods returning `*mut u8`
- **DarwinAllocator**: `platform-darwin-host/src/alloc.rs` — delegates to libc
- **PlatformProvider**: added `fn alloc() -> &dyn Allocator` to the bundle trait
- **cf-lite**: replaced ~120 `libc::calloc/malloc/realloc/free` calls with `crate::pal_alloc()`. Fallback `LibcAllocator` for unit tests
- **objc-runtime**: replaced ~25 `libc::calloc/malloc/free` calls with `crate::pal_alloc()`. Fallback `LibcAllocator` for unit tests. `set_allocator()` called by `macho-runtime` during init
- **Excluded**: `libsystem-shims` TLS/pthread allocation (stays as direct libc — POSIX primitives, not ObjC objects), `lifecycle.rs` pthread_mutex_t allocation

Test results: 961 passed, 0 failures, 0 clippy errors

---

### Phase 44 — Host Dependency Reduction ✅

Goal: audit all host dependencies, document them, and reduce where practical.

#### Host Dependency Inventory

| Component | Category | Calls | Status |
|---|---|---|---|
| `platform-darwin-host` | libc (mmap/mprotect/munmap) | 4 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libc (calloc/malloc/realloc/free) | 4 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libc (clock/time) | 6 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libc (dlsym/dlopen) | 3 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libc (pthread_mutex/cond) | 15 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libc (arc4random, pthread_main_np, mach_absolute_time) | 3 | ✅ Correct — macOS PAL impl |
| `platform-darwin-host` | CommonCrypto (CC_SHA256) | 1 | ✅ Correct — PAL impl |
| `platform-darwin-host` | libunwind (__unw_add/remove) | 2 | ✅ Correct — PAL impl |
| `cf-lite` | dlsym/dlopen (Foundation symbols) | 0 | ✅ Replaced → `pal_symbols()` |
| `cf-lite` | pthread_mutex/cond (NSLock family) | 0 | ✅ Replaced → `pal_threading()` |
| `cf-lite` | libc (allocator fallback) | 4 | ⚠️ Test-only fallback |
| `cf-lite` | std::env (LANG, TZ) | 2 | 📌 Acceptable — env var reads |
| `cf-lite` | ARM64 asm trampolines | 2 files | 📌 Architecture-specific by design |
| `cf-lite` | `#[cfg(target_os = "macos")]` | 5 blocks | 📌 Host-only features (EHTYPE, ISA aliases) |
| `objc-runtime` | pthread_mutex (@synchronized) | 0 | ✅ Replaced → `pal_threading()` |
| `objc-runtime` | libc::dlsym (host fallback) | 1 | 📌 Acceptable — already behind `host_symbols()` PAL |
| `objc-runtime` | ARM64 asm trampoline | 1 file | 📌 Architecture-specific by design |
| `objc-runtime` | std::process::abort | 12 | 📌 Standard library, not host-specific |
| `libsystem-shims` | pthread/TLS/mach | 55+ | 📌 Correct — shim layer MUST use host primitives |
| `macho-parse` | Mach-O format parsing | all | 📌 Correct — format-specific by design |
| `macho-resolve` | std::fs, DYLD_* env vars | 3 | 📌 Correct — resolver needs filesystem |
| `macho-exec` | dyld chained fixups | all | 📌 Correct — execution-specific by design |
| `trust-policy` | DYLD_* / SIP policy | 5 | 📌 Correct — security policy by design |

#### Deliverables

- **Threading PAL trait**: `platform_core::threading::Threading` with mutex (init, init_recursive, lock, unlock, trylock, destroy) and condition variable (init, wait, signal, broadcast, destroy) operations
- **PosixThreading**: `platform-darwin-host/src/threading.rs` — POSIX pthread implementation
- **PlatformProvider**: added `fn threading() -> &dyn Threading` to the bundle trait (now 10 methods)
- **cf-lite dlsym/dlopen → PAL**: replaced 7 `libc::dlsym` + 2 `libc::dlopen` calls in `foundation/mod.rs` and `nssortdescriptor.rs` with `crate::pal_symbols()` (HostSymbolResolver)
- **cf-lite NSLock → PAL**: replaced 35 `libc::pthread_mutex_*/pthread_cond_*` calls in `nslock.rs` with `crate::pal_threading()`, removed 13 unnecessary `unsafe` blocks
- **objc-runtime @synchronized → PAL**: replaced `libc::calloc` + 7 `libc::pthread_*` calls in `lifecycle.rs` sync_mutex_for/sync_enter/sync_exit with `crate::pal_threading()`
- **Fallback impls**: `LibcSymbolResolver` + `PosixThreadingFallback` in both `cf-lite` and `objc-runtime` for unit tests

Test results: 961 passed, 0 failures, 0 clippy errors

---

### Phase 45 — GUI Framework Expansion ✅

Goal: evolve GUI layer into a usable framework.

Delivered:
- **Menu system** (`app-kit-lite/src/menu.rs`): `NSMenu`, `NSMenuItem` with titles, actions,
  key equivalents (keyboard shortcuts), submenus, separators, modifier flags, recursive
  key-equivalent dispatch
- **Event modifiers**: `Event::KeyDown`/`KeyUp` now carry `ModifierFlags`; `NSApplication`
  performs menu key equivalents before dispatching to responder chain
- **ScrollView** (`ui-controls/src/scrollview.rs`): content offset, content size, scroll-by,
  vertical/horizontal enable, visible rect, scroll indicators, offset clamping
- **Layout system** (`ui-controls/src/layout.rs`): `StackView` with horizontal/vertical axis,
  natural and fill-equally distribution, spacing, padding; `layout_vertical`/`layout_horizontal`
  helper functions
- **TableView** (`ui-controls/src/tableview.rs`): `TableDataSource` trait, `VecDataSource`
  in-memory data source, `TableColumn`, row selection, scrolling, alternating row colors,
  column headers, hit-test for click-to-select
- **SplitView** (`ui-controls/src/splitview.rs`): horizontal/vertical orientation, adjustable
  divider position, min-size constraints, pane frame computation, divider hit testing
- **Menu bar rendering** in `window-backend-darwin`: `GuiApp.set_main_menu()`, menu title
  rendering, modifier tracking via `ModifiersChanged`, key-equivalent interception before
  text field dispatch
- **Control Gallery updated**: File/Edit menu bar with Cmd+N/O/Q/X/C/V shortcuts,
  layout adjusted for menu bar height

Test results: **1007 passed, 0 failed** (46 new tests)

---

### Phase 46 — Window Backend Abstraction ✅

Goal: GUI independent of host.

Delivered:
- **Extended `WindowBackend` trait** (`window-backend/src/lib.rs`): added `Modifiers` struct,
  `CharacterInput`, `ModifiersChanged`, `ScrollWheel` event variants, `window_size()` and
  `scale_factor()` methods with default implementations
- **Enhanced `HeadlessBackend`**: supports all new events, configurable scale factor via
  `with_scale()`, window size tracking
- **`AppScene` kernel** (`ui-controls/src/scene.rs`): backend-agnostic rendering and interaction
  core — holds labels, buttons, textfields, menus; provides `render()` → Surface, `handle_click()`,
  `handle_key_down()`, `handle_character_input()`, `handle_backspace()`, modifier tracking
- **Refactored `GuiApp`** (window-backend-darwin): delegates all rendering/interaction logic to
  `AppScene` instead of owning it directly; winit `AppState` is now a thin presentation layer
- **Refactored `HeadlessApp`** (ui-controls): wraps `AppScene`, exposes `scene()`/`scene_mut()`
  for direct scene access, added `render()` for headless surface generation
- **Unified code path**: same `AppScene` renders identically in both native (winit+softbuffer)
  and headless (testing) modes — proving GUI framework is backend-agnostic
- **NSApplication** handles all new WindowEvent variants (CharacterInput, ModifiersChanged,
  ScrollWheel)

Test results: **1027 passed, 0 failed** (20 new tests)

---

### Phase 47 — Runtime Diagnostics

Goal: make debugging compatibility easier.

Add runtime events:
- SelectorMissing
- ClassMissing
- SymbolMissing
- HostFallback
- MethodDispatch

Add CLI tool: `darwin-inspect --why-failed`

Success criteria: **every compatibility failure is diagnosable**

---

### Phase 48 — Developer Tooling

Goal: improve usability for developers.

Tools:
- darwin-run
- darwin-trace
- darwin-compat
- darwin-inspect

Capabilities:
- class browser
- selector lookup
- dispatch trace
- method resolution trace

Success criteria: **developers can debug runtime behavior easily**

---

### Phase 49 — Demo Environment

Goal: create a polished demonstration system.

Add:
- runtime-shell
- runtime-control-panel

Capabilities:
- run apps
- inspect runtime state
- toggle security modes
- view compatibility reports

Success criteria: **interactive runtime environment**

---

### Phase 50 — Performance Work

Goal: optimize runtime.

Targets:
- method dispatch
- selector lookup caching
- symbol resolution
- ARC operations
- startup time

Add benchmarks.

Success criteria: **performance competitive with host runtime**

---

### Phase 51 — Ecosystem Integration

Goal: allow external projects to use runtime.

Improve:
- C API
- Swift bindings
- Rust library usage

Add:
- documentation
- sample apps
- tutorials

Success criteria: **third-party projects use runtime**

---

### Phase 52 — Toward Host Independence

Goal: enable runtime to operate without macOS host.

Future work may include:
- custom window server
- graphics abstraction
- filesystem layer
- network stack
- process management

This prepares the runtime to become the userspace of a future OS.

---

## Guiding Principle

Instead of freezing layers, the roadmap follows this rule:

> **Any layer may evolve when real software requires it.**

Progress is measured by:
* real programs running
* host dependency shrinking
* platform capabilities expanding

## North Star

> **A complete macOS-compatible runtime platform capable of running real software independently of Apple's OS.**

---

## Compatibility Scoring

Automated scorecard across 62 fixtures via `cargo test --test compat_score -- --nocapture`:

| Dimension | Score | Meaning |
|---|---|---|
| Parse | 62/62 | Mach-O parsed without error |
| Load | 62/62 | All images mapped + dependencies resolved |
| Fixups | 62/62 | Rebases + binds applied correctly |
| Enter main | 62/62 | Entry point called without crash |
| Complete execution | 62/62 | Program runs to exit |
| Exit correctness | 62/62 | Exit code matches expected |
| Runtime feature coverage | 62/62 | ObjC/Foundation features exercised |

**Result: 100% compatibility** — all 62 non-malformed fixtures pass all dimensions.

Fixture categories covered:
- Pure C: `return42`, `simple_cli`, `constructor_test`, `fat_binary`, `dispatch_test`, `tlv_basic`
- Dependency chains: `dylib_chain`, `weak_dylib`, `framework_style`, `objc_dylib_chain`
- ObjC core: `objc_minimal`, `objc_classes`, `objc_category`, `objc_inherit`, `objc_super`, `objc_foundation`
- Foundation NS*: 28 fixtures covering NSString, NSArray, NSData, NSDate, NSNumber, NSDictionary, NSSet, NSError, NSBundle, NSProcessInfo, NSSortDescriptor, NSTimer, NSURL, NSUserDefaults, NSUUID, NSValue, NSNotification, NSNull, NSLocale, NSCalendar, NSCharacterSet, NSIndexSet, NSNumberFormatter, NSDateFormatter, NSTimezone, NSFileManager
- Blocks: `block_basics`, `blocks_foundation`
- ARC/memory: `autorelease`
- Runtime introspection: `runtime_introspection`, `property_introspection`, `nsobject_methods`
- Exceptions: `exception_basics`, `minimal_throw`, `try_catch_test`
- Integration: `real_cli` (26-check multi-feature exercise), `collection_enum`

