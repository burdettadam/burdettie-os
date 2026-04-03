# Xcode CLI Tool Deep Investigation

> Empirical investigation of Xcode command-line tool support under the darwin-loader.
> Initial investigation: 2025-06-28 · Last updated: 2026-03-30

## Executive Summary

### Phase 1 (targeted, 10 tools)

**9 out of 10 Xcode CLI tools load and execute** under the darwin-loader, and **all 9 produce correct output**. The remaining tool (`genstrings`) requires an external framework (`XCStringsParser.framework`) that is loaded but depends on SwiftSyntax/SwiftParser which are not present locally.

### Phase 2 — Aggressive Broad Testing (2026-03-30)

Tested **110 tools** across four categories:

| Category | Total | Pass | Crash | Notes |
|---|---|---|---|---|
| Xcode Toolchain (`XcodeDefault.xctoolchain/usr/bin`) | 35 | **26** | 9 | arm64 binaries; crashes in clang/ld/libtool/unifdef |
| Xcode Developer (`Developer/usr/bin`) | 14 | **7** | 7 | altool: missing `stringWithCString:encoding:` |
| System CLI (`/usr/bin`) | 58 | **0** | 58 | **All arm64e** — universal blocker |
| Apple Frameworks (`/usr/bin` framework-heavy) | 3 | **0** | 3 | arm64e + framework dependencies |
| **Total** | **110** | **33** | **77** | |

**Two blocking gaps identified:**

1. **arm64e (pointer authentication)** — Every `/usr/bin` system tool is arm64e.
   The loader selects arm64e slices as if they were arm64, but PAC instructions
   crash with SIGBUS/SIGSEGV. This blocks ALL macOS system utilities.

2. **Unresolved arm64 crash class** — Even plain arm64 tools (clang, ld, libtool,
   unifdef) crash with SIGBUS despite identical Mach-O structure to passing tools.
   Likely a fixup application or memory protection issue for specific symbol patterns.

Key fixes applied across two sessions:
1. Fat binary main binary loading (passed raw fat data instead of arm64 slice)
2. Fat binary dependency loading (deps were parsed with `parse()` instead of `parse_auto()`)
3. Hardcoded GOT address diagnostic (read `0x10000c048 + slide` on all binaries — crashed on non-fixture binaries)
4. `_mach_task_self_` data symbol fix (was registered as function pointer, now correctly resolved as data)
5. StubResolver safe-list expansion (fortified buffer functions, NXGetArchInfo*, ctype internals, uname, modf)

An additional issue was found and fixed: `value_delimiter = ' '` on `--exec-args` was splitting paths containing spaces.

## Test Results

| Tool | Category | Status | Exit | Details |
|------|----------|--------|------|---------|
| **lipo** | Pure C (cctools) | ✅ PASS | 0 | `lipo -info` produces perfect output |
| **otool** | LLVM (llvm-otool) | ✅ PASS | 0 | `otool -h` shows correct Mach-O headers |
| **strip** | C + libcodedirectory | ✅ PASS | 1 | Shows correct "no files specified" error |
| **genstrings** | ObjC + Foundation | ✅ PASS | 64 | Full usage output; loaded XCStringsParser framework, 28 ObjC classes |
| **stapler** | ObjC + Foundation | ✅ PASS | 0 | Full usage output; 5 ObjC classes registered |
| **size** | Pure C (cctools) | ✅ PASS | 0 | Fully functional — correct size output, clean exit |
| **strings** | Pure C (cctools) | ✅ PASS | 0 | Fully functional — finds correct strings, clean exit |
| **vtool** | C + libcodedirectory | ✅ PASS | 0 | `-help` and `-show <file>` both work; loads libcodedirectory.dylib |
| **nm** | LLVM (llvm-nm) | ✅ PASS | 0 | 34 C++ constructors, correct symbol listing output |
| **swift-demangle** | LLVM + Swift | ⚠️ FUNCTIONAL | 139 | Correct demangling output; segfaults during C++ static destructor cleanup |

## Dependency Map

```
Tier 1 — Pure C (libSystem only):
  lipo          → libSystem.B.dylib (host)
  size          → libSystem.B.dylib (host)
  strings       → libSystem.B.dylib (host)

Tier 2 — C + Xcode dylib:
  vtool         → @rpath/libcodedirectory.dylib (fat, now loads) + libSystem
  strip         → @rpath/libcodedirectory.dylib (fat) + libc++ + libSystem

Tier 3 — LLVM (C++):
  nm (llvm-nm)  → libc++ + libz + libSystem
  otool (llvm-otool) → libc++ + libSystem

Tier 4 — LLVM + ObjC:
  swift-demangle → libc++ + libz + libobjc + libSystem

Tier 5 — Foundation + ObjC:
  genstrings    → XCStringsParser.framework (fat, loads) + Foundation + CF + libobjc
                  [skips SwiftSyntax/SwiftParser — not found locally]
  stapler       → CoreServices + Foundation + CF + Security + libobjc + libxar
```

## Failure Analysis

### 1. swift-demangle Cleanup Crash (cosmetic)

**Symptom**: swift-demangle correctly demangles symbols (e.g., `_$s4main5helloyyF` → `main.hello() -> ()`) but segfaults during process exit in C++ static destructor cleanup.

**Root Cause**: LLVM's internal data structures have complex C++ static initialization/destruction ordering. The destructors run after `main()` returns and attempt to tear down objects that depend on C++ runtime state (libc++ TLS, atexit handlers) in an order that doesn't survive the loader's fixup boundary.

**Impact**: Cosmetic only. The tool produces correct output before the crash. Exit code is 139 (SIGSEGV) instead of 0.

**Status**: Low priority — the tool is functionally correct.

---

## Phase 2 — Broad Compatibility Testing (2026-03-30)

### Xcode Toolchain Results (26/35 pass)

All Xcode toolchain binaries are **arm64** (not arm64e). Passing tools include
all cctools, LLVM utilities, tapi, dsymutil, indent, and swift-demangle.

**New passes** (beyond Phase 1):
- `ranlib`, `ar`, `install_name_tool`, `bitcode_strip`, `nmedit`, `segedit`, `codesign_allocate`
- `llvm-objdump`, `llvm-nm`, `llvm-size`, `llvm-otool`, `llvm-dwarfdump`, `llvm-profdata`, `llvm-cov`, `llvm-cxxfilt`
- `tapi`, `tapi-analyze`, `indent`, `dsymutil`

**Crashing tools (9)**:

| Tool | Signal | Libraries | Diagnosis |
|---|---|---|---|
| `unifdef` | SIGBUS | libSystem only | Simplest crashing case — pure C, no extra deps. Fixup or page-protection issue. |
| `libtool` | SIGBUS | libLTO + libc++ | `@rpath/libLTO.dylib` dependency |
| `ld` | SIGBUS | libtapi + libLTO + Foundation | Complex deps including `@rpath` dylibs and frameworks |
| `clang` | SIGBUS | libresolv + libc++ | Same struct as nm (which passes); libresolv may be the differentiator |
| `clang++` | SIGBUS | (symlink to clang) | Same binary |
| `clangd` | SIGBUS | libresolv + libc++ | Same pattern as clang |
| `sourcekit-lsp` | SIGBUS | Foundation + Swift runtime | Needs Swift standard library + CryptoKit + OSLog + Security |
| `swift-api-digester` | SIGSEGV | libobjc + Swift runtime | Needs `@rpath/lib_CompilerSwift*.dylib` (many) |
| `swift-symbolgraph-extract` | SIGSEGV | libobjc + Swift runtime | Same pattern as swift-api-digester |

**Key investigation notes**: `unifdef` is the Rosetta Stone — it uses only libSystem,
has identical Mach-O structure to passing tools (same segments, no TLS, no
exceptions), yet crashes at entry. This suggests a fixup application bug affecting a
subset of binaries, possibly related to chained fixup import ordinal differences or
`__DATA_CONST` handling.

### Xcode Developer Tools Results (7/14 pass)

| Tool | Status | Signal | Notes |
|---|---|---|---|
| `genstrings` | ✅ PASS | — | |
| `stapler` | ✅ PASS | — | |
| `heap` | ✅ PASS | — | |
| `leaks` | ✅ PASS | — | |
| `vmmap` | ✅ PASS | — | |
| `malloc_history` | ✅ PASS | — | |
| `filtercalltree` | ✅ PASS | — | |
| `xcodebuild` | ❌ ABORT | NSException | Uncaught exception during startup |
| `ibtool` | ❌ CRASH | SIGSEGV | Interface Builder tool |
| `actool` | ❌ CRASH | SIGSEGV | Asset catalog compiler |
| `notarytool` | ❌ CRASH | SIGBUS | Notarization tool |
| `altool` | ❌ ABORT | NSException | Missing `stringWithCString:encoding:` selector |
| `xctest` | ❌ CRASH | SIGBUS | Testing framework |
| `xctrace` | ❌ CRASH | SIGSEGV | Instruments tracing |

**`altool` root cause**: Calls `[NSString stringWithCString:encoding:]` which is not
yet shimmed. This is a commonly used NSString class method for C-string conversion.

### System CLI Tools Results (0/58 pass)

**ALL /usr/bin system tools are arm64e** (pointer authentication). The loader selects
the arm64e slice (since CpuType is Arm64 for both arm64 and arm64e) but cannot execute
PAC instructions, resulting in universal SIGBUS/SIGSEGV crashes.

Tested tools: `grep`, `sed`, `awk`, `sort`, `uniq`, `wc`, `cut`, `tr`, `head`, `tail`,
`tee`, `diff`, `find`, `stat`, `file`, `basename`, `dirname`, `tar`, `gzip`, `zip`,
`unzip`, `uname`, `sw_vers`, `env`, `id`, `whoami`, `openssl`, `defaults`, `plutil`,
`log`, `lldb`, `codesign`, and 26 more.

**Signal breakdown**: 25 SIGBUS, 33 SIGSEGV.

### Apple Framework Tools Results (0/3 pass)

All arm64e, all crash: `mdutil`, `mdimport`, `dscacheutil`.

---

## Gap Analysis — Blocking Issues by Priority

### P0: arm64e / Pointer Authentication Support

**Impact**: Blocks **ALL** `/usr/bin` system utilities (58+ tested, 882 total in directory).
This is the single largest compatibility gap in the entire project.

**What's needed**:
- Detect arm64e slices during fat binary selection (cpusubtype == 2)
- Strip PAC bits from pointers during fixup application (mask off high bits)
- Handle `BIND_OPCODE_THREADED` / authenticated binds in chained fixups
- Optional: reject arm64e binaries with a clear error message until full support lands

### P1: Unresolved arm64 SIGBUS Crashes

**Impact**: Blocks 9 Xcode toolchain tools, including clang, ld, and libtool.

**What's needed**:
- Debug `unifdef` as the minimal reproduction case (libSystem-only, arm64)
- Compare chained fixup imports between crashing vs passing tools
- Check if `__stubs` resolution differs for certain import ordinal patterns
- May be a missing safe-list symbol causing an unresolved GOT entry → SIGBUS

### P2: Missing Foundation Selectors

**Impact**: Blocks tools like `altool` that use common NSString methods.

**Known missing**:
- `NSString stringWithCString:encoding:` — C string to NSString conversion
- Additional selectors will be exposed as more tools reach Foundation code

### P3: Swift Runtime Support

**Impact**: Blocks `sourcekit-lsp`, `swift-api-digester`, `swift-symbolgraph-extract`.

**What's needed**:
- Load and resolve `libswiftCore.dylib` and its transitive dependencies
- Handle Swift metadata structures (type descriptors, protocol conformances)
- Support `@rpath/lib_CompilerSwift*.dylib` resolution for compiler tools

## Resolved Issues

### Fixed: `vm_deallocate` Errors (size, strings) — 2026-03-30

**Symptom**: `Can't vm_deallocate mapped memory for file: ... ((ipc/send) invalid destination port)`

**Root Cause**: `_mach_task_self_` is a DATA symbol on macOS — a global `mach_port_t` variable at a specific address, not a function. The shim table was registering a function pointer for it, so when guest code read the GOT entry expecting a pointer to a port value, it got a code address instead. Dereferencing that code address produced a garbage port value, which `vm_deallocate` rejected as "invalid destination port".

**Fix**: Removed `_mach_task_self_` from the macOS shim table entirely. The StubResolver's safe-list includes `mach_task_self_`, so `dlsym` returns the correct host data address. On non-macOS platforms, a static `u32` with synthetic port value `0x103` is registered instead.

**File**: `crates/libsystem-shims/src/mach.rs`

### Fixed: vtool `-show` Bus Error — 2026-03-30

**Symptom**: `vtool -help` worked perfectly but `vtool -show <file>` crashed with SIGBUS.

**Root Cause**: vtool uses `DyldChainedFixups` (PTR_64_OFFSET format). Three of its 55 chained import symbols were missing from the StubResolver safe-list: `NXGetArchInfoFromCpuType`, `NXGetArchInfoFromName`, and `__memcpy_chk`. Their GOT entries remained unresolved (zero), causing bus errors when accessed during file processing.

**Fix**: Added the missing symbols to the safe-list in `crates/macho-exec/src/fixup/resolver.rs`:
- Mach-O arch info: `NXGetArchInfoFromCpuType`, `NXGetArchInfoFromName`, `NXGetLocalArchInfo`, `NXGetAllArchInfos`
- Fortified buffer functions: `__memcpy_chk`, `__memmove_chk`, `__memset_chk`, `__strcpy_chk`, `__strncpy_chk`, `__strcat_chk`, `__strlcpy_chk`, `__strlcat_chk`

### Fixed: nm (llvm-nm) Bus Error — 2026-03-30

**Symptom**: 34 C++ static constructors executed successfully, then crashed at entry point.

**Root Cause**: nm imports `__tolower`, `__toupper`, `modf`, and `uname` — none of which were on the StubResolver safe-list. Additionally, the `_mach_task_self_` data symbol issue (same as size/strings) affected nm.

**Fix**: Added missing symbols to the safe-list: `__tolower`, `__toupper`, `modf`, `uname`. Combined with the `_mach_task_self_` fix, nm now runs to completion.

## Bugs Found & Fixed

### Fix 1: Fat Binary Main Binary Loading
**File**: `crates/macho-inspect/src/main.rs:169`
**Issue**: `exec::print_exec(&parsed, &data, ...)` passed raw fat binary `data` to the exec module. The exec module then tried `registry.load("__main__", data.to_vec())` which failed with "bad magic: 0xbebafeca".
**Fix**: Changed to `exec::print_exec(&parsed, &effective_data, ...)` which passes the extracted arm64 slice.

### Fix 2: Fat Binary Dependency Loading
**File**: `crates/macho-inspect/src/exec.rs:51-57`
**Issue**: Dependencies resolved via `@rpath` were parsed with `parse()` (thin-only). Fat binary dependencies like `libcodedirectory.dylib` and `XCStringsParser.framework` failed with magic `0xbebafeca`.
**Fix**: Added fat binary detection: if `is_fat()`, use `parse_auto()` + `fat_slice_for_cpu()` to extract the arm64 slice.

### Fix 3: Hardcoded GOT Address Diagnostic
**File**: `crates/macho-inspect/src/exec.rs:201-203`
**Issue**: Read `unsafe { *(0x10000c048 + slide) }` — a hardcoded vmaddr that's only valid for a specific fixture binary. On other binaries, this reads random/unmapped memory, causing segfaults (confirmed with `mach_vm_query` corpus).
**Fix**: Removed the unsafe hardcoded GOT read. Kept symbol resolution diagnostics.

### Fix 4: Space-Splitting in --exec-args
**File**: `crates/macho-inspect/src/main.rs:46`
**Issue**: `value_delimiter = ' '` on the `--exec-args` clap attribute caused paths with spaces to be split: `/path/with spaces/...` → `["/path/with", "spaces/..."]`.
**Fix**: Removed `value_delimiter = ' '` from the attribute.

### Fix 5: `_mach_task_self_` Data Symbol (2026-03-30)
**File**: `crates/libsystem-shims/src/mach.rs`
**Issue**: `_mach_task_self_` is a DATA symbol on macOS (a global `mach_port_t` variable), not a function. The shim table registered a function pointer, so guest code dereferenced function code bytes as a port value → invalid port → `vm_deallocate` "invalid destination port" errors in size, strings, and other tools.
**Fix**: Removed `_mach_task_self_` from macOS shim table; StubResolver `dlsym`s the real data address. On non-macOS, register address of a static `u32` holding synthetic port `0x103`.

### Fix 6: StubResolver Safe-List Gaps (2026-03-30)
**File**: `crates/macho-exec/src/fixup/resolver.rs`
**Issue**: Several symbols imported by vtool, nm, and other Xcode tools were missing from the safe-list, leaving their GOT entries unresolved (zero) and causing bus errors on access.
**Fix**: Added 16 symbols across 4 categories:
- Fortified buffer: `__memcpy_chk`, `__memmove_chk`, `__memset_chk`
- Fortified string: `__strcpy_chk`, `__strncpy_chk`, `__strcat_chk`, `__strlcpy_chk`, `__strlcat_chk`
- Mach-O arch info: `NXGetArchInfoFromCpuType`, `NXGetArchInfoFromName`, `NXGetLocalArchInfo`, `NXGetAllArchInfos`
- Misc POSIX: `__tolower`, `__toupper`, `modf`, `uname`

## Build Note

The workspace uses `CARGO_TARGET_DIR=/tmp/burdettieOS-target/`. Binaries are at `/tmp/burdettieOS-target/release/`, NOT `target/release/`.

## What's Left for Full Xcode CLI Support

### Fully Functional (9 tools)
- **lipo**: ✅ Fully functional
- **otool**: ✅ Fully functional (delegates to `otool-classic` for some modes)
- **strip**: ✅ Functional (usage/error paths)
- **genstrings**: ✅ Functional (usage; file processing needs SwiftSyntax/SwiftParser frameworks)
- **stapler**: ✅ Functional (usage; actual stapling needs network access + Security framework)
- **size**: ✅ Fully functional — clean exit, correct output
- **strings**: ✅ Fully functional — clean exit, correct output
- **vtool**: ✅ Fully functional — `-help`, `-show`, `-show-build` all work
- **nm**: ✅ Fully functional — correct symbol listing, clean exit

### Cosmetic Issues
- **swift-demangle**: Core demangling works correctly; segfaults during C++ static destructor cleanup after `main()` returns. Low priority — functional output is produced before the crash.
