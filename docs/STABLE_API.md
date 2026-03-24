# Stable API Reference — v0.1.0

> **Freeze point**: Phase 30 of the darwin-loader-lab roadmap.
> This document records the public API surface of all 16 crates at the
> point the runtime core was declared stable.  From this point forward,
> only bug fixes are permitted in core crates; new features go in new
> crates (Phase 31+).

---

## Version

All crates inherit `version = "0.1.0"` from the workspace root `Cargo.toml`.
Edition: 2021.  MSRV: 1.82.

---

## Crate Catalogue

| # | Crate | Type | Purpose |
|---|-------|------|---------|
| 1 | `macho-types` | lib | Pure data structures for Mach-O binary format concepts |
| 2 | `macho-parse` | lib | Mach-O binary parser — bytes in, `ParsedMachO` out |
| 3 | `macho-map` | lib | Virtual memory mapping planner for Mach-O segments |
| 4 | `macho-objc` | lib | Objective-C metadata detection and inspection |
| 5 | `macho-resolve` | lib | Dependency and symbol resolution planner |
| 6 | `macho-run-plan` | lib | Top-level execution plan generator |
| 7 | `macho-diff` | lib | Compare computed loading plans against host observations |
| 8 | `trust-policy` | lib | Security gate traits and mock policies |
| 9 | `macho-exec` | lib | In-process segment mapping, fixup application, controlled entry |
| 10 | `libsystem-shims` | lib | Controlled system library shims |
| 11 | `objc-runtime` | lib | ObjC runtime — class registration, selector interning, dispatch, allocation |
| 12 | `cf-lite` | lib | Foundation/CoreFoundation class implementations |
| 13 | `macho-runtime` | lib | Runtime orchestration — load, shim, and run Mach-O binaries |
| 14 | `macho-fixtures` | lib | Fixture metadata and expected outputs for test binaries |
| 15 | `test-harness` | lib | End-to-end test orchestration |
| 16 | `macho-inspect` | bin | CLI tool for human-readable Mach-O inspection |

---

## 1. `macho-types`

Pure data types — no host dependencies, no unsafe code.

### Public modules

`header`, `load_command`, `segment`, `symbol`, `relocation`, `dylib`, `entry`, `codesig`, `data_in_code`

### Key types

| Type | Kind | Description |
|------|------|-------------|
| `MachHeader` | struct | Mach-O header (magic, cpu_type, cpu_subtype, file_type, ncmds, sizeofcmds, flags) |
| `CpuType` | enum | `Arm64`, `X86_64`, `Arm`, `X86`, … |
| `CpuSubtype` | enum | CPU subtype variants |
| `FileType` | enum | Mach-O file types |
| `FatHeader`, `FatArch`, `FatBinary` | structs | Universal binary support |
| `LoadCommand` | struct | Load command representation |
| `Segment`, `Section`, `SectionType` | structs/enum | Segment and section metadata |
| `TlvDescriptor` | struct | Thread-local variable descriptor |
| `VmProt` | bitflags | Memory protection flags |
| `SymbolTable`, `Symbol`, `SymbolType` | structs/enum | Symbol table representation |
| `Relocation`, `RelocationType` | struct/enum | Relocation entries |
| `DylibRef` | struct | Dynamic library reference |
| `EntryPoint` | struct | Entry point specification |
| `CodeSignature`, `CodeDirectory`, `SuperBlob` | structs | Code signature structures |
| `CsFlags`, `CsHashType`, `CsSlotType` | enum/bitflags | Code signature enumerations |
| `DataInCodeEntry`, `DataInCodeKind` | struct/enum | Data-in-code markers |

### Constants

`MH_MAGIC_64`, `MH_CIGAM_64`, `MH_MAGIC`, `MH_CIGAM`, `FAT_MAGIC`, `FAT_CIGAM`

---

## 2. `macho-parse`

### Public modules

`fixups`, `exports`, `codesig`, `fat`, `func_starts`, `indirect`, `tlv`, `relocations`, `data_in_code`, `strings`

### Key types and functions

| Item | Kind | Signature |
|------|------|-----------|
| `ParsedMachO` | struct | Fields: header, commands, segments, sections, dylibs, rpaths, entry, symbols, uuid, code_signature |
| `parse` | fn | `fn parse(data: &[u8]) -> Result<ParsedMachO, ParseError>` |
| `ParseError` | enum | Parsing error variants |
| `extract_fixups` | fn | `fn extract_fixups(parsed, data) -> Fixups` |
| `Fixups`, `FixupModel` | struct/enum | Fixup representations |
| `RebaseEntry`, `BindEntry`, `ChainedFixup` | structs | Fixup entry types |
| `extract_exports`, `ExportedSymbol` | fn/struct | Export trie parsing |
| `parse_code_signature` | fn | Code signature parsing |
| `verify_page_hashes` | fn | Page hash verification |
| `parse_fat`, `is_fat`, `fat_slice_for_cpu` | fns | Universal binary support |
| `parse_auto` | fn | Auto-detects fat vs thin and parses |
| `parse_function_starts` | fn | Function starts section parsing |
| `resolve_indirect_symbols` | fn | Indirect symbol table resolution |
| `parse_tlv_descriptors` | fn | Thread-local variable parsing |
| `parse_relocations` | fn | Relocation entry parsing |
| `parse_data_in_code` | fn | Data-in-code entry parsing |
| `extract_cstrings` | fn | C string extraction from sections |
| `vmaddr_to_file_offset` | fn | VM address to file offset conversion |

---

## 3. `macho-map`

| Type | Kind | Description |
|------|------|-------------|
| `MapRegion` | struct | Fields: segment_name, vmaddr, vmsize, fileoff, filesize, protection, max_protection, zerofill_size |
| `MappingPlan` | struct | `from_parsed(&ParsedMachO) -> Self`, `display() -> String`.  Field: `regions: Vec<MapRegion>` |

---

## 4. `macho-objc`

### Re-exported types (via `pub use types::*`)

| Type | Kind | Description |
|------|------|-------------|
| `ObjcPresence` | struct | Fields: has_objc, sections_found |
| `ObjcMethodInfo` | struct | Method metadata |
| `ObjcIvarInfo` | struct | Instance variable metadata |
| `ObjcPropertyInfo` | struct | Property metadata |
| `ObjcClassRef` | struct | Class reference |
| `ObjcCategoryRef` | struct | Category reference |
| `ObjcMetadata` | struct | Fields: presence, class_refs, categories, category_count, protocol_count, selectors, selref_addrs, image_info |
| `ObjcImageInfo` | struct | Image-level ObjC information |
| `ConstructorEntry` | struct | Fields: address, source |
| `ConstructorSource` | enum | `ModInitFunc`, `InitOffsets` |
| `InitializerInfo` | struct | Fields: constructors, init_steps |

### Public functions

| Function | Signature |
|----------|-----------|
| `detect_objc` | `fn detect_objc(parsed: &ParsedMachO) -> ObjcPresence` |
| `parse_objc_metadata` | `fn parse_objc_metadata(parsed: &ParsedMachO, data: &[u8]) -> ObjcMetadata` |
| `extract_initializers` | `fn extract_initializers(parsed: &ParsedMachO, data: &[u8]) -> InitializerInfo` |

---

## 5. `macho-resolve`

### Public modules

`bind`

### Key types

| Item | Kind | Description |
|------|------|-------------|
| `DependencyGraph` | struct | Fields: nodes, edges |
| `DependencyEdge` | struct | Dependency edge |
| `ImageNode` | struct | Dependency graph node |
| `ResolutionPlan` | struct | Methods: `from_parsed`, `from_parsed_transitive`, `init_order` |
| `PathResolver` | struct | Platform-aware library path resolution |
| `SearchEnv` | struct | Search environment configuration |
| `bind_symbols` | fn | Symbol binding |
| `BindingReport`, `SymbolBinding` | structs | Binding results |

---

## 6. `macho-run-plan`

| Type | Kind | Description |
|------|------|-------------|
| `ExecutionPlan` | struct | `from_parsed`, `from_parsed_transitive`, `from_parsed_full`, `from_parsed_secured`.  Fields: binary_path, mapping, dependencies, objc, objc_metadata, initializers, symbol_bindings, security, init_order, entry |

---

## 7. `macho-diff`

| Type | Kind | Description |
|------|------|-------------|
| `HostObservation` | struct | Fields: loaded_dylibs, segments, entry_offset |
| `ObservedSegment` | struct | Fields: name, vmaddr, vmsize |
| `Mismatch` | struct | Fields: category, expected, observed |
| `DiffReport` | struct | `compare_dylibs`, `compare_segments`, `is_clean`.  Fields: matches, mismatches, warnings |

---

## 8. `trust-policy`

### Traits

`SecurityGate`, `QuarantinePolicy`, `CodeSignatureValidator`, `EntitlementEvaluator`, `AmfiPolicy`, `SipPolicy`, `HardenedRuntimePolicy`, `LibraryValidator`, `SandboxPolicy`

### Apple implementations

`AppleQuarantine`, `AppleCodeSignature`, `AppleEntitlements`, `AppleAmfi`, `AppleSip`, `AppleHardenedRuntime`, `AppleLibraryValidator`, `AppleSandbox`

### Mock implementations

`MockQuarantine`, `MockCodeSignature`, `MockEntitlements`, `MockAmfi`, `MockSip`, `MockHardenedRuntime`, `MockLibraryValidator`, `MockSandbox`

### Core types

| Type | Kind | Description |
|------|------|-------------|
| `SecurityVerdict` | enum | `Allow`, `AllowWithWarning`, `Deny` |
| `GateDecision` | struct | Fields: gate, verdict |
| `CodeSignatureInfo` | struct | `from_parsed(cs)` |
| `TrustPolicy` | struct | `permissive()`, `apple_default()`, `evaluate_binary`, `validate_dylib` |
| `SecurityReport` | struct | Fields: decisions, allowed, denied_reasons, strip_dyld_env, hardened_runtime, cs_info |

---

## 9. `macho-exec`

| Item | Kind | Description |
|------|------|-------------|
| `MappedSegment` | struct | Fields: name, base, size, protection |
| `MappedImage` | struct | `map(plan, data)`, `segment(name)`, `segment_ptr_mut(name, offset)`, `protect()`, `text_base()`, `total_mapped_size()` |
| `apply_rebases` | fn | Apply rebase fixups to mapped image |
| `apply_binds` | fn | Apply bind fixups with symbol resolver |
| `apply_chained_fixups` | fn | Apply chained fixup format |
| `SymbolResolver` | trait | `fn resolve(&self, name: &str) -> Option<u64>` |
| `StubResolver` | struct | Host `dlsym` fallback resolver |
| `call_entry` | fn | Call binary entry point |
| `EntryResult` | struct | Fields: exit_code |
| `checked_entry` | fn | Entry with security checks |
| `call_constructors` | fn | Run C++ constructors / ObjC +load |
| `ImageRegistry` | struct | Multi-image loader.  `new()`, `load(name, data)`, `get(name)`, `images()`, `apply_all_fixups(resolver)`, `resolver()` |
| `LoadedImage` | struct | Fields: install_name, image, parsed, fixups, data |
| `RegistryResolver` | struct | Symbol resolver across registry images |
| `register_unwind_info` | fn | Register unwind info with libunwind |
| `unregister_unwind_info` | fn | Deregister unwind info |
| `ExecError` | enum | `MmapFailed`, `MprotectFailed`, `SegmentOutOfBounds`, `NoTextSegment`, `NoEntryPoint`, `FixupSegmentOutOfRange`, `FixupOffsetOutOfBounds`, `SecurityDenied`, `EncryptedBinary` |

---

## 10. `libsystem-shims`

### Public modules

`table`, `process`, `tls`, `dispatch`, `pthread`, `mach`, `lifecycle`

### Key types

| Item | Kind | Description |
|------|------|-------------|
| `ShimTable` | struct | `new()`, `register(name, addr, desc)`, `resolve(name)`, `resolve_stripped(name)`, `len()`, `is_empty()`, `iter()`, `symbol_names()` |
| `ShimEntry` | struct | Fields: name, address, description |
| `ProcessShims` | struct | `install(table, config)` |
| `TlsShims` | struct | `install(table)` |
| `DispatchShims` | struct | `install(table)` |
| `PthreadShims` | struct | `install(table)` |
| `MachShims` | struct | `install(table)` |
| `LifecycleShims` | struct | `install(table)` |
| `ShimConfig` | struct | Fields: process_name, executable_path, argc, argv, env |
| `install_default_shims` | fn | Install all shim categories into a table |

---

## 11. `objc-runtime`

### Public modules

`blocks`, `refcount`

### Key types

| Item | Kind | Description |
|------|------|-------------|
| `ClassInfo` | struct | ObjC class metadata (name, super_class, methods, ivars, properties, …) |
| `MethodEntry` | struct | Method metadata |
| `IvarEntry` | struct | Instance variable metadata |
| `PropertyEntry` | struct | Property metadata |
| `ObjcRuntime` | struct | The runtime state object |
| `OBJC_RUNTIME` | static | `Mutex<Option<ObjcRuntime>>` — global runtime instance |
| `ObjcShims` | struct | `install(table)` — registers 60 ObjC runtime shims |

### Re-exported shim functions

`shim_nsobject_retain`, `shim_nsobject_release`, `shim_nsobject_autorelease`, `shim_nsobject_retain_count`, `shim_nsobject_dealloc`, `shim_objc_exception_throw`

### `blocks` module

`global_block_class_addr()`, `stack_block_class_addr()`, `shim_block_copy()`, `shim_block_release()`, `shim_objc_retain_block()`

### `refcount` module

`REFCOUNTS` (static), `RefCountTable`, `AutoreleasePoolStack`

---

## 12. `cf-lite`

### Public modules (40 Foundation class modules)

`cfstring`, `nsarray`, `nsdata`, `nsdictionary`, `nslog`, `nsmutablearray`, `nsmutabledict`, `nsmutablestring`, `nsnumber`, `nsobj`, `nsprocessinfo`, `nserror`, `nsbundle`, `nsfilemanager`, `nsexception`, `nsenumerator`, `nsdate`, `nsuserdefaults`, `nsset`, `nsmutableset`, `nsurl`, `nsnull`, `nsdateformatter`, `nscharacterset`, `nsnotificationcenter`, `nsvalue`, `nsuuid`, `nslocale`, `nstimezone`, `nscalendar`, `nsnumberformatter`, `nsindexset`, `nssortdescriptor`, `nstimer`, `nsthread`, `nslock`, `foundation`

### Key type

| Item | Kind | Description |
|------|------|-------------|
| `FoundationShims` | struct | `install(table)` — registers 77 Foundation/CF shims |

---

## 13. `macho-runtime`

### Public modules

`config`, `context`, `error`, `resolver`, `tracer`

### Key types

| Item | Kind | Description |
|------|------|-------------|
| `Runtime` | struct | `load_and_run(binary_path, config) -> Result<RuntimeResult, RuntimeError>` |
| `RuntimeConfig` | struct | Fields: security_policy, host_fallback, shim_set, env_overrides, process_name, process_path, tracer |
| `ShimSet` | struct | Per-category shim toggle (8 booleans) |
| `RuntimeContext` | struct | Holds built shim_table and ObjC/Foundation state |
| `RuntimeError` | enum | `Io`, `Parse`, `Exec`, `SecurityDenied`, `EncryptedBinary`, `NoArm64Slice`, `NoEntryPoint` |
| `CompositeResolver` | struct | Chains shim table + optional host StubResolver |
| `RuntimeResult` | struct | Fields: exit_code, security_report, images_loaded, shims_installed, initializers_run, tracer |
| `RuntimeTracer` | struct | `new()`, `disabled()`, `record(kind, msg)`, `events()`, `events_of(kind)`, `count()`, `count_of(kind)`, `is_enabled()`, `clear()` |
| `RuntimeEvent` | struct | Fields: kind, message |
| `EventKind` | enum | `ImageLoad`, `SelectorRegistration`, `ClassRegistration`, `CategoryMerge`, `MethodLookup`, `BlockLifecycle`, `RetainRelease`, `FoundationCreate`, `Pipeline` |

---

## 14. `macho-fixtures`

| Item | Kind | Description |
|------|------|-------------|
| `FixtureDescriptor` | struct | Fields: name, binary_path, description, expected_dylibs, expected_entry_type, expected_segments, tags |
| `fixtures_dir` | fn | Returns the path to the fixtures directory |
| `load_descriptor` | fn | Load a fixture descriptor from a JSON file |
| `known_fixtures` | fn | Returns list of known fixture names |

---

## 15. `test-harness`

| Item | Kind | Description |
|------|------|-------------|
| `run_fixture` | fn | Run a fixture through the full pipeline |
| `run_fixture_transitive` | fn | Run with transitive dependency resolution |
| `run_fixture_full` | fn | Run with full analysis including init ordering |
| `collect_host_observation` | fn | Collect host dyld observation for comparison |
| `collect_init_order` | fn | Observe host initializer order |
| `ObservedInitOrder` | struct | Fields: init_sequence, raw_output |

---

## 16. `macho-inspect` (binary)

CLI tool — no public library API.

### Subcommands / flags

`--json`, `--json-full`, `--security`, `--resolve`, `--plan`, `--exec`, `--codesig`, `--funcs`, `--symbols`, `--indirect`, `--selrefs`, `--tlv`, `--relocs`, `--data-in-code`, `--strings`, `--rebase`, `--cpu <arch>`

---

## Shim Counts at Freeze Point

| Category | Count | Crate |
|----------|-------|-------|
| Process | 5 | libsystem-shims |
| TLS | 2 | libsystem-shims |
| Dispatch | 5 | libsystem-shims |
| Pthread | 42 | libsystem-shims |
| Mach | 3 | libsystem-shims |
| Lifecycle | 2 | libsystem-shims |
| ObjC Runtime | 60 | objc-runtime |
| Foundation/CF | 77 | cf-lite |
| **Total** | **196** | |

## Foundation Classes (40)

NSObject, NSString, NSMutableString, NSArray, NSMutableArray, NSDictionary,
NSMutableDictionary, NSNumber, NSData, NSProcessInfo, NSLog, NSError, NSBundle,
NSFileManager, NSException, NSEnumerator, NSDate, NSUserDefaults, NSSet,
NSMutableSet, NSURL, NSNull, NSDateFormatter, NSCharacterSet,
NSNotificationCenter, NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar,
NSNumberFormatter, NSIndexSet, NSSortDescriptor, NSTimer, NSThread, NSLock,
NSRecursiveLock, NSCondition, NSConditionLock, CFConstantString

---

## Test Counts at Freeze Point

| Metric | Value |
|--------|-------|
| Total tests | 831+ |
| Failures | 0 |
| Clippy warnings | 0 |
| Compatibility score | 63/63 (100%) |
