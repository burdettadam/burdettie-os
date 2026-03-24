# Mach-O Loader Technical Reference

> Comprehensive research for darwin-loader-lab architecture planning.
> Sources: Apple open-source dyld/XNU, System V ABI, POSIX.1-2017, Apple developer documentation.

---

## Table of Contents

1. [Mach-O Loader Standards (Apple dyld)](#1-mach-o-loader-standards-apple-dyld)
2. [Comparison with ELF/POSIX Standards](#2-comparison-with-elfposix-standards)
3. [Apple Security and Trust Model — Full Layer Map](#3-apple-security-and-trust-model--full-layer-map)
4. [Interoperability With Apple's Stack](#4-interoperability-with-apples-stack)
5. [Industry Best Practices for Loader/OS Projects](#5-industry-best-practices-for-loaderos-projects)
6. [Key Specifications and References](#6-key-specifications-and-references)

---

## 1. Mach-O Loader Standards (Apple dyld)

### 1.1 Startup Sequence: Kernel exec to main()

The path from `execve()` to `main()` involves three distinct subsystems: XNU kernel, dyld, and libSystem.

#### Phase 1: XNU Kernel (`kern_exec.c`)

1. **System call entry**: `execve()` → `__mac_execve()` or `posix_spawn()` → `exec_activate_image()`
2. **Image activator chain**: XNU iterates an activator table in fixed order:
   ```c
   const struct execsw execsw[] = {
       { exec_mach_imgact,    "Mach-o Binary" },
       { exec_fat_imgact,     "Fat Binary" },
       { exec_shell_imgact,   "Interpreter Script" },
       { NULL, NULL }
   };
   ```
3. **`exec_mach_imgact()`** validates:
   - Magic number (`MH_MAGIC` / `MH_MAGIC_64`; rejects `MH_CIGAM` reverse-endian)
   - `filetype == MH_EXECUTE`
   - CPU type/subtype grading (architecture compatibility)
   - arm64e pointer authentication ABI version checks
4. **`load_machfile()`**: Parses load commands, creates a new VM map, maps segments, locates dyld
5. **ASLR slide**: Kernel picks a random slide value; all vmaddrs are offset by this amount
6. **Process setup**: The kernel populates the `apple[]` string vector on the stack with:
   - `executable_path=` — path to the main executable
   - `pfz=` — commpage text location
   - `stack_guard=` — stack canary entropy (first value contains embedded NUL)
   - `malloc_entropy=` — entropy for system malloc
   - `ptr_munge=` — pointer obfuscation entropy
   - `main_stack=` — stack address and size
   - `executable_file=` — fsid and fileid
   - `dyld_file=` — dyld's fsid and fileid
   - `executable_cdhash=` — code directory hash
   - `executable_boothash=` — hash of cdhash + boot session UUID
   - `th_port=` — main thread Mach port name
   - `dyld_flags=` — (DEVELOPMENT/DEBUG only)
   - `arm64e_abi=` — "all" or "os" (arm64e only)
   - `ptrauth_disabled=1` — if arm64e running as arm64
   - `security_config=` — task security configuration bitmask
7. **Stack layout** (grows downward):
   ```
   +-------------+ <- p->user_stack
   |     16b     |
   +-------------+
   | STRING AREA |  (argv strings, env strings, apple strings)
   +-------------+
   |  PATH AREA  |
   +-------------+
   |      0      |
   +-------------+
   | applev[n]   |
   |     ...     |
   | applev[0]   |  (exec_path)
   +-------------+
   |      0      |
   +-------------+
   |   env[n]    |
   |     ...     |
   |   env[0]    |
   +-------------+
   |      0      |
   +-------------+
   |  arg[n-1]   |
   |     ...     |
   |   arg[0]    |
   +-------------+
   |    argc     |
   +-------------+ <- sp (entry point)
   ```
8. **Entry transfer**: `thread_setentrypoint(thread, result->entry_point)` sets PC to dyld's entry point. The address of the main executable's `mach_header` is pushed onto the stack.

#### Phase 2: dyld (Dynamic Link Editor)

dyld4 (current, macOS 12+) is organized around `ProcessConfig`, a singleton capturing all fixed initial process state:

```
ProcessConfig
├── Process       — kernel args, main executable info, platform, archs
├── Security      — AMFI-derived permissions (allowAtPaths, allowEnvVarsPrint, etc.)
├── Logging       — DYLD_PRINT_* flags
├── DyldCache     — shared cache address, slide, ObjC/Swift optimization tables
└── PathOverrides — DYLD_LIBRARY_PATH, DYLD_FRAMEWORK_PATH, fallback paths, etc.
```

dyld startup sequence:

1. **Self-bootstrap**: dyld reads its own load commands, initializes the allocator
2. **Process configuration**: Constructs `ProcessConfig` from `KernelArgs`:
   - Parses `apple[]` strings from the stack
   - Queries AMFI for security policy bits via `getAMFI()`
   - Processes environment variables (with security filtering)
   - Maps or attaches shared cache
3. **Load main executable**: Reads load commands from the main executable `mach_header`
4. **Dependency loading**: For each `LC_LOAD_DYLIB` / `LC_LOAD_WEAK_DYLIB` / `LC_REEXPORT_DYLIB`:
   - Resolve the install name using path search order (see §1.5)
   - Check if already loaded or in shared cache
   - Map the dylib into the process
   - Recursively load its dependencies
5. **Inserted dylibs**: Process `DYLD_INSERT_LIBRARIES` (if security policy allows)
6. **Fixups (rebasing + binding)**:
   - Apply ASLR slide adjustments to internal pointers
   - Resolve external symbol references
   - Process interposing (`__DATA,__interpose` section)
7. **Weak symbol coalescing**: Resolve weak definitions across all loaded images
8. **Initializer execution**: Run in bottom-up dependency order (see §1.7)
9. **Entry point**: Jump to `main()` via the offset from `LC_MAIN` (or the address from `LC_UNIXTHREAD` for older binaries)

#### Phase 3: libSystem

libSystem is always the first non-dyld initializer to run. Its `libSystem_initializer()`:

1. Initializes libdispatch (`_dispatch_init()`)
2. Initializes the Objective-C runtime (`_objc_init()`)
3. Registers dyld callbacks for future image loads
4. Initializes CoreFoundation bridges
5. Initializes pthread subsystem
6. Sets up TSD (Thread-Specific Data)

### 1.2 Load Command Types and Processing Order

Load commands appear sequentially in the Mach-O file after the header. They are processed in the order they appear, but different commands are consumed at different stages.

#### Structural / Mapping Commands

| Load Command | Value | Purpose |
|---|---|---|
| `LC_SEGMENT_64` | `0x19` | Defines a 64-bit memory-mapped region (segment + sections) |
| `LC_SEGMENT` | `0x01` | Defines a 32-bit memory-mapped region |
| `LC_SYMTAB` | `0x02` | Symbol table location and size |
| `LC_DYSYMTAB` | `0x0B` | Dynamic symbol table info (indirect symbols, etc.) |
| `LC_UUID` | `0x1B` | 128-bit unique build identifier |
| `LC_BUILD_VERSION` | `0x32` | Platform, minos, SDK version, ntools |
| `LC_SOURCE_VERSION` | `0x2A` | Source version (A.B.C.D.E packed) |

#### Entry Point Commands

| Load Command | Value | Purpose |
|---|---|---|
| `LC_MAIN` | `0x80000028` | Entry point as offset from `__TEXT` segment start. Preferred for modern binaries. |
| `LC_UNIXTHREAD` | `0x05` | Entry point as absolute virtual address in thread state. Legacy. |

dyld prefers `LC_MAIN` over `LC_UNIXTHREAD` if both are present. Only the first of each is honored.

#### Dependency Commands

| Load Command | Value | Purpose |
|---|---|---|
| `LC_LOAD_DYLIB` | `0x0C` | Required dylib dependency |
| `LC_LOAD_WEAK_DYLIB` | `0x80000018` | Optional dylib (missing symbols → NULL) |
| `LC_REEXPORT_DYLIB` | `0x8000001F` | Re-exported dylib (symbols forwarded) |
| `LC_LAZY_LOAD_DYLIB` | `0x20` | Loaded on first symbol use |
| `LC_LOAD_UPWARD_DYLIB` | `0x80000023` | Upward dependency (breaks cycles) |
| `LC_ID_DYLIB` | `0x0D` | Identifies this file as a dylib (install name) |
| `LC_RPATH` | `0x8000001C` | Adds a runtime search path for `@rpath` resolution |
| `LC_DYLD_ENVIRONMENT` | `0x27` | Injects environment variable (main exec only) |

#### Fixup / Linking Commands

| Load Command | Value | Purpose |
|---|---|---|
| `LC_DYLD_INFO_ONLY` | `0x80000022` | Classic fixup data: rebase, bind, weak_bind, lazy_bind, export opcodes |
| `LC_DYLD_CHAINED_FIXUPS` | `0x80000034` | Modern chained fixup data (macOS 12+, arm64e) |
| `LC_DYLD_EXPORTS_TRIE` | `0x80000033` | Modern export trie (separated from `LC_DYLD_INFO`) |
| `LC_FUNCTION_STARTS` | `0x26` | Table of function start addresses |
| `LC_DATA_IN_CODE` | `0x29` | Regions of non-code in `__TEXT` |

#### Code Signing

| Load Command | Value | Purpose |
|---|---|---|
| `LC_CODE_SIGNATURE` | `0x1D` | Offset/size of embedded code signature |
| `LC_ENCRYPTION_INFO_64` | `0x2C` | FairPlay encryption info (App Store binaries) |

#### Other Notable Commands

| Load Command | Value | Purpose |
|---|---|---|
| `LC_THREAD` | `0x04` | Thread state (kernel threads) |
| `LC_PREBOUND_DYLIB` | `0x10` | Prebinding info (obsolete) |
| `LC_ROUTINES_64` | `0x1A` | Dynamic shared library init routine address |
| `LC_SUB_FRAMEWORK` | `0x12` | Umbrella framework declaration |
| `LC_SUB_CLIENT` | `0x14` | Allowable client of sub-framework |
| `LC_SUB_LIBRARY` | `0x15` | Allowable sub-library |
| `LC_LINKER_OPTION` | `0x2D` | Linker option strings |
| `LC_NOTE` | `0x31` | Arbitrary note (owner + offset + size) |
| `LC_FILESET_ENTRY` | `0x80000035` | Kernel collection fileset entry |

### 1.3 The Two Fixup Models

#### Classic Model: `LC_DYLD_INFO_ONLY`

Used by older binaries (pre-macOS 12, x86_64). The load command points to four streams of opcodes in `__LINKEDIT`:

**Rebase opcodes** (`rebase_off` / `rebase_size`):
- Fix internal pointers due to ASLR slide
- Opcode-based bytecode: `REBASE_OPCODE_SET_TYPE_IMM`, `REBASE_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB`, `REBASE_OPCODE_ADD_ADDR_ULEB`, `REBASE_OPCODE_DO_REBASE_*`
- Types: `REBASE_TYPE_POINTER` (data), `REBASE_TYPE_TEXT_ABSOLUTE32`, `REBASE_TYPE_TEXT_PCREL32`

**Bind opcodes** (`bind_off` / `bind_size`):
- Resolve external (non-lazy) symbol references at load time
- Opcodes specify: symbol name, library ordinal, segment/offset, addend, type
- Key opcodes: `BIND_OPCODE_SET_DYLIB_ORDINAL_*`, `BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM`, `BIND_OPCODE_SET_TYPE_IMM`, `BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB`, `BIND_OPCODE_DO_BIND`
- Two-level namespace: ordinal identifies which dylib provides the symbol

**Lazy bind opcodes** (`lazy_bind_off` / `lazy_bind_size`):
- Deferred symbol resolution — stub jumps to `dyld_stub_binder` on first call
- Each lazy binding entry is its own self-contained opcode stream at a known offset
- The stub helper passes the lazy bind info offset to `dyld_stub_binder`

**Export trie** (`export_off` / `export_size`):
- Trie data structure encoding all exported symbols with their:
  - Address (as offset from image base)
  - Flags: `EXPORT_SYMBOL_FLAGS_KIND_REGULAR`, `_THREAD_LOCAL`, `_ABSOLUTE`
  - `EXPORT_SYMBOL_FLAGS_REEXPORT` — forwarded to another dylib
  - `EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER` — lazy-evaluated symbol
  - `EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION`

#### Modern Model: `LC_DYLD_CHAINED_FIXUPS` + `LC_DYLD_EXPORTS_TRIE`

Introduced for arm64e (macOS 12+), now used for all new arm64 binaries:

**Chained fixups** (`LC_DYLD_CHAINED_FIXUPS`):
- All fixup locations are encoded as an in-place linked list within the `__DATA` segments
- The `__LINKEDIT` payload contains:
  - `dyld_chained_fixups_header` — overall structure
  - `dyld_chained_starts_in_image` — per-segment start info
  - `dyld_chained_starts_in_segment` — page-level start offsets
  - Import table with symbol info
- Each fixup location in `__DATA` encodes a pointer-sized value that is either:
  - **Rebase**: Contains the target vmaddr + authentication info (for arm64e)
  - **Bind**: Contains import table index + addend
- The chain: each fixup value includes a `next` delta field pointing to the next fixup in the same page
- Pointer formats: `DYLD_CHAINED_PTR_64`, `DYLD_CHAINED_PTR_ARM64E`, `DYLD_CHAINED_PTR_ARM64E_USERLAND`, `DYLD_CHAINED_PTR_ARM64E_USERLAND24`, etc.
- arm64e fixups include PAC (Pointer Authentication Code) diversity and key information

**Advantages of chained fixups**:
- Smaller `__LINKEDIT` (no opcode streams)
- Faster to process (walk chains, not decode opcodes)
- Single pass: rebases and binds are interleaved in the chain
- Better support for pointer authentication metadata

**Export trie** (`LC_DYLD_EXPORTS_TRIE`):
- Same trie format as the classic model's export info
- Separated into its own load command for clarity

### 1.4 Mach-O Header and Key Data Structures

```c
struct mach_header_64 {
    uint32_t    magic;          // MH_MAGIC_64 = 0xFEEDFACF
    cpu_type_t  cputype;        // e.g., CPU_TYPE_ARM64
    cpu_subtype_t cpusubtype;   // e.g., CPU_SUBTYPE_ARM64E
    uint32_t    filetype;       // MH_EXECUTE, MH_DYLIB, MH_BUNDLE, etc.
    uint32_t    ncmds;          // number of load commands
    uint32_t    sizeofcmds;     // total size of load commands
    uint32_t    flags;          // MH_PIE, MH_TWOLEVEL, MH_DYLDLINK, etc.
    uint32_t    reserved;       // 64-bit padding
};
```

Key `MH_` flags:
- `MH_PIE` (0x200000): Position-independent executable (ASLR compatible)
- `MH_TWOLEVEL` (0x80): Two-level namespace (symbol lookups scoped to specific dylib)
- `MH_DYLDLINK` (0x04): Input for the dynamic linker
- `MH_NOUNDEFS` (0x01): No undefined references
- `MH_WEAK_DEFINES` (0x8000): Has weak-defined symbols
- `MH_BINDS_TO_WEAK` (0x10000): Binds to weak symbols
- `MH_HAS_TLV_DESCRIPTORS` (0x800000): Has thread-local variable sections
- `MH_NO_REEXPORTED_DYLIBS` (0x100000): No re-exported dylibs
- `MH_DYLIB_IN_CACHE` (0x80000000): This dylib is in the shared cache

File types:
- `MH_EXECUTE` (2): Main executable
- `MH_DYLIB` (6): Dynamic shared library
- `MH_BUNDLE` (8): Loadable bundle (plugin)
- `MH_DYLINKER` (7): Dynamic linker (dyld itself)
- `MH_OBJECT` (1): Relocatable object file
- `MH_DSYM` (10): Debug symbol companion file
- `MH_KEXT_BUNDLE` (11): Kernel extension
- `MH_FILESET` (12): Kernel collection

Version encoding in `LC_LOAD_DYLIB`: `(major << 16) | (minor << 8) | patch`

### 1.5 Path Resolution Semantics

#### Path Variables

- **`@executable_path`**: Resolves to the directory containing the main executable. Set once at process start.
- **`@loader_path`**: Resolves to the directory containing the Mach-O file that has the `LC_LOAD_DYLIB` or `LC_RPATH` command. Allows dylibs to find their peers relative to themselves.
- **`@rpath`**: Searched against the ordered list of `LC_RPATH` entries from the loading image and all images in its dependency chain up to and including the main executable.

#### Search Order (via `PathOverrides::forEachPathVariant`)

For a given requested path, dyld tries variants in this order:

1. **`DYLD_LIBRARY_PATH` / `DYLD_FRAMEWORK_PATH` overrides** — prepended directories searched first (leaf name extracted)
2. **Versioned overrides** — `DYLD_VERSIONED_LIBRARY_PATH` / `DYLD_VERSIONED_FRAMEWORK_PATH` (pick higher version)
3. **Image suffix** — if `DYLD_IMAGE_SUFFIX` is set (e.g., `_debug`), try suffixed variant
4. **Catalyst prefix** — for Mac Catalyst apps, try iOS-compatible paths
5. **Overlay prefix** — cryptex/overlay paths
6. **Simulator prefix** — for simulator builds
7. **Raw path on disk** — the path as specified in the load command
8. **Raw path in cache** — check the dyld shared cache
9. **`@rpath` expansion** — expand `@rpath` using `LC_RPATH` entries
10. **`@loader_path` expansion**
11. **`@executable_path` expansion**
12. **Implicit rpath** if the dylib has no explicit path and the binary was linked with `-rpath`
13. **Custom fallback paths** — `DYLD_FALLBACK_LIBRARY_PATH` / `DYLD_FALLBACK_FRAMEWORK_PATH`
14. **Standard fallback paths** — default fallback paths are:
    - `/usr/local/lib` (for dylibs)
    - `/usr/lib`
    - `/usr/local/lib` (again, for frameworks: `/Library/Frameworks`, `/System/Library/Frameworks`)

Fallback mode:
```c
enum class FallbackPathMode { classic, restricted, none };
```
- `classic`: Use `DYLD_FALLBACK_*` paths if set, otherwise standard fallbacks
- `restricted`: Only `/usr/lib` and system framework paths
- `none`: No fallbacks (for hardened/restricted binaries)

#### Framework Resolution

Frameworks are special directory bundles. For path `/System/Library/Frameworks/Foo.framework/Versions/A/Foo`:
- `getFrameworkPartialPath()` extracts `Foo.framework/Versions/A/Foo`
- Framework search inserts this partial path after each `DYLD_FRAMEWORK_PATH` entry

### 1.6 dyld Shared Cache

#### What It Is

The dyld shared cache (`dyld_shared_cache_arm64e`, etc.) is a single large file that pre-links most system dylibs into a contiguous mapped region:

- Location: `/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/`
- Typically 1–2 GB, memory-mapped read-only into every process
- Contains: all system dylibs (`/usr/lib/`, `/System/Library/Frameworks/`), pre-computed ObjC metadata, Swift metadata, export tries, pre-bound stubs

#### How It Affects Resolution

- Dylibs in the cache are **removed from disk** (macOS 11+). `stat()` will fail for them.
- `_dyld_shared_cache_contains_path(path)` returns true if the path is in the cache
- dyld checks the cache before looking on disk for system libraries
- The `DyldCache` configuration in `ProcessConfig` tracks:
  - `addr`: Mapped base address of the cache
  - `slide`: ASLR slide applied to the cache
  - `dylibCount`: Number of dylibs in the cache
  - `development`: Whether this is the development cache (supports roots/overrides)
  - `rootsAreSupported`: Whether on-disk overrides ("roots") can replace cached dylibs
  - ObjC selector/class/protocol hash tables for rapid lookup
  - Swift cache info

#### Closures (dyld3 mechanism, largely superseded by dyld4)

In dyld3, "closures" were pre-computed data structures cached to disk that described:
- All dylibs to load
- All bindings to perform
- All initializer ordering

dyld4 replaced closures with `JustInTimeLoader` and `PrebuiltLoader` objects:
- `PrebuiltLoader`: Embedded in the shared cache for cached dylibs. Pre-computed, very fast.
- `JustInTimeLoader`: Built at runtime for non-cached images. Does all the parsing/resolution on the fly.
- `PrebuiltLoaderSet`: A set of pre-built loaders for apps, can be stored in the shared cache or on disk.

### 1.7 Initializer Ordering Rules

Initializers run in a strictly defined order:

1. **Bottom-up dependency order**: A dylib's initializers run only after ALL of its dependencies' initializers have run
2. **libSystem is always first** among non-dyld initializers (it's the root of the dependency graph)
3. **Within a single image**, the order is:
   a. **Objective-C `+load` methods** — run before any C++ constructors in the same image
   b. **C/C++ constructors** (`__mod_init_func` section entries) — run in the order they appear in the section
   c. **`LC_ROUTINES_64`** initializer (rare, for dylibs that need special init)

4. **`DYLD_INSERT_LIBRARIES`** images: their initializers run after the main executable's direct dependencies but before the main executable's own initializers

5. **`dlopen()`'d images**: initializers run immediately upon successful load, after all dependencies of the dlopen'd image are initialized

The initializer function signature:
```c
typedef void (*Initializer)(int argc, const char* const argv[],
    const char* const envp[], const char* const apple[],
    const ProgramVars* vars);
```

### 1.8 DYLD_* Environment Variables

From `ProcessConfig::Security` and `ProcessConfig::Logging`:

#### Diagnostic / Print Variables

| Variable | Effect |
|---|---|
| `DYLD_PRINT_LIBRARIES` | Log each dylib as it is loaded |
| `DYLD_PRINT_SEGMENTS` | Log segment mapping details |
| `DYLD_PRINT_INITIALIZERS` | Log each initializer as it runs |
| `DYLD_PRINT_BINDINGS` (fixups) | Log symbol binding operations |
| `DYLD_PRINT_APIS` | Log dyld API calls (`dlopen`, `dlsym`, etc.) |
| `DYLD_PRINT_NOTIFICATIONS` | Log notification callbacks |
| `DYLD_PRINT_INTERPOSING` | Log interposed symbols |
| `DYLD_PRINT_LOADERS` | Log loader construction |
| `DYLD_PRINT_SEARCHING` | Log path searching during resolution |
| `DYLD_PRINT_ENV` | Log environment variable processing |
| `DYLD_PRINT_WARNINGS` | Print non-fatal warnings |

#### Path Override Variables

| Variable | Effect |
|---|---|
| `DYLD_LIBRARY_PATH` | Colon-separated directories to search BEFORE default paths |
| `DYLD_FRAMEWORK_PATH` | Colon-separated framework directories to search BEFORE defaults |
| `DYLD_FALLBACK_LIBRARY_PATH` | Override default library fallback paths |
| `DYLD_FALLBACK_FRAMEWORK_PATH` | Override default framework fallback paths |
| `DYLD_VERSIONED_LIBRARY_PATH` | Directory of versioned dylib overrides (higher version wins) |
| `DYLD_VERSIONED_FRAMEWORK_PATH` | Directory of versioned framework overrides |
| `DYLD_INSERT_LIBRARIES` | Colon-separated list of dylibs to force-load into every process |
| `DYLD_IMAGE_SUFFIX` | Suffix to append when searching (e.g., `_debug`) |

#### Behavior Variables

| Variable | Effect |
|---|---|
| `DYLD_SHARED_REGION` | `use`, `private`, or `avoid` the shared cache |
| `DYLD_SHARED_CACHE_DIR` | Override shared cache directory |
| `DYLD_ROOT_PATH` | Prefix all paths with this root (simulator use) |

#### Security Restrictions on DYLD_* Variables

`ProcessConfig::Security` determines which variables are honored. The AMFI kernel policy returns a bitmask; `pruneEnvVars()` strips disallowed variables:

- `allowAtPaths`: Whether `@loader_path`, `@executable_path`, `@rpath` are processed
- `allowEnvVarsPrint`: Whether `DYLD_PRINT_*` variables are honored
- `allowEnvVarsPath`: Whether `DYLD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, etc. are honored
- `allowEnvVarsSharedCache`: Whether `DYLD_SHARED_REGION` etc. are honored
- `allowInsertFailures`: Whether failure to load an inserted dylib is non-fatal
- `allowInterposing`: Whether `DYLD_INSERT_LIBRARIES` and `__DATA,__interpose` are processed

Variables are **stripped** (ignored) for:
- **Restricted binaries**: `CS_RESTRICT` flag set, setuid/setgid binaries
- **Platform binaries**: System-signed Apple binaries
- **Hardened runtime**: Binaries with hardened runtime enabled (unless specific entitlements override)
- **SIP-protected**: Binaries in SIP-protected paths

### 1.9 dyld Notifications API

```c
// Called for every already-loaded image and each future image load
extern void _dyld_register_func_for_add_image(
    void (*func)(const struct mach_header* mh, intptr_t vmaddr_slide));

// Called when an image is about to be removed (after terminators, before unmap)
extern void _dyld_register_func_for_remove_image(
    void (*func)(const struct mach_header* mh, intptr_t vmaddr_slide));
```

Additional image enumeration APIs:
```c
extern uint32_t              _dyld_image_count(void);
extern const struct mach_header* _dyld_get_image_header(uint32_t image_index);
extern intptr_t              _dyld_get_image_vmaddr_slide(uint32_t image_index);
extern const char*           _dyld_get_image_name(uint32_t image_index);
```

Shared cache query:
```c
extern bool _dyld_shared_cache_contains_path(const char* path);
```

Thread-local variable support:
```c
extern void _tlv_atexit(void (*termFunc)(void* objAddr), void* objAddr);
extern void _tlv_bootstrap(void);  // Never called directly; marker for uninitialized TLVs
```

Path query:
```c
extern int _NSGetExecutablePath(char* buf, uint32_t* bufsize);
```

All legacy `NS*` APIs (NSAddImage, NSLookupSymbolInImage, NSLinkModule, etc.) are deprecated since macOS 10.5 in favor of `dlopen`/`dlsym`/`dlclose`.

---

## 2. Comparison with ELF/POSIX Standards

### 2.1 ELF Loading Model vs Mach-O

| Aspect | ELF | Mach-O |
|---|---|---|
| **File header** | `Elf64_Ehdr` (magic `\x7fELF`) | `mach_header_64` (magic `0xFEEDFACF`) |
| **Segment headers** | `PT_LOAD` program headers (`Elf64_Phdr`) | `LC_SEGMENT_64` load commands |
| **Dynamic linker** | `PT_INTERP` specifies path (e.g., `/lib64/ld-linux-x86-64.so.2`) | Hard-coded to `/usr/lib/dyld` (mapped by kernel) |
| **Dependencies** | `DT_NEEDED` entries in `.dynamic` section | `LC_LOAD_DYLIB` load commands |
| **Symbol table** | `.dynsym` + `.dynstr` (separate from debug `.symtab`) | `LC_SYMTAB` + `LC_DYSYMTAB` (single table with dynamic/static markers) |
| **Relocations** | `DT_REL` / `DT_RELA` / `DT_JMPREL` (explicit relocation entries) | Opcode-encoded rebase/bind or chained fixups |
| **Lazy binding** | PLT/GOT with `DT_JMPREL` | `__stubs` / `__stub_helper` / `__la_symbol_ptr` with lazy bind opcodes |
| **Export symbols** | In `.dynsym` with binding `STB_GLOBAL` / `STB_WEAK` | Export trie (compact prefix tree) |
| **Init functions** | `DT_INIT` + `DT_INIT_ARRAY` / `DT_INIT_ARRAYSZ` | `__mod_init_func` section + `LC_ROUTINES_64` |
| **Fini functions** | `DT_FINI` + `DT_FINI_ARRAY` | `__mod_term_func` section |
| **Position independence** | `ET_DYN` type with `-fPIE` | `MH_PIE` flag |
| **Symbol namespacing** | Flat namespace (first match wins, unless `RTLD_DEEPBIND`) | Two-level namespace (symbol scoped to specific dylib ordinal) |

### 2.2 glibc ld.so vs musl ld.so vs dyld

| Feature | glibc ld.so | musl ld.so | dyld |
|---|---|---|---|
| **Complexity** | Very large (~100K+ lines), many extensions | Minimal (~3K lines), POSIX-focused | Large (~100K+ lines), Apple-specific |
| **Symbol versioning** | Full support (`DT_VERSYM`, `DT_VERDEF`, `DT_VERNEED`) | Partial support | Not applicable (uses two-level namespaces) |
| **Audit interface** | `LD_AUDIT` / `la_objopen` etc. | Not supported | No equivalent (use `DYLD_INSERT_LIBRARIES`) |
| **Prelink/preload** | `LD_PRELOAD` | `LD_PRELOAD` | `DYLD_INSERT_LIBRARIES` |
| **Lazy binding** | Default; disable with `LD_BIND_NOW` | Always bind now (no lazy) | Default; dyld3 closures removed lazy |
| **Shared object cache** | `ldconfig` / `ld.so.cache` (path lookup cache only) | None | Full pre-linked shared cache (merged binaries) |
| **Namespace isolation** | `dlmopen()` with `LM_ID_NEWLM` | Not supported | Not supported (flat process space) |

### 2.3 POSIX dlopen/dlsym/dlclose API Surface

Both Mach-O and ELF implement the POSIX.1-2017 `<dlfcn.h>` API:

```c
void *dlopen(const char *filename, int flags);
void *dlsym(void *handle, const char *symbol);
int   dlclose(void *handle);
char *dlerror(void);
int   dladdr(const void *addr, Dl_info *info);
```

**`dlopen()` flags**:
- `RTLD_LAZY` (1): Defer symbol resolution. On Mach-O, this is the default behavior with stub-based lazy binding.
- `RTLD_NOW` (2): Resolve all symbols immediately.
- `RTLD_LOCAL` (4): Symbols not available for subsequently loaded objects. Default on Mach-O.
- `RTLD_GLOBAL` (8): Symbols available to subsequent `dlsym(RTLD_DEFAULT, ...)`.
- `RTLD_NOLOAD` (0x10): Check if already loaded without loading.
- `RTLD_NODELETE` (0x80): Don't unload on `dlclose()`.
- `RTLD_FIRST` (0x100): macOS-specific: search only this image (not its dependencies).

**`dlsym()` pseudo-handles**:
- `RTLD_DEFAULT` ((void*)(-2)): Search all loaded images in load order.
- `RTLD_NEXT` ((void*)(-1)): Search images loaded after the calling image.
- `RTLD_SELF` ((void*)(-3)): macOS-specific: search this image and its dependencies.
- `RTLD_MAIN_ONLY` ((void*)(-5)): macOS-specific: search only the main executable.

**Mach-O-specific behavior differences**:
- `dlclose()` on macOS does NOT unload code in modern dyld. The image stays mapped. This is because Objective-C class registration is not reversible.
- `dlsym()` with two-level namespaces: by default, `dlsym(handle, sym)` searches the specified image and its re-exported dependencies. With `RTLD_FIRST`, only the specified image.

### 2.4 TLS: ELF Model vs Mach-O TLV Descriptors

#### ELF TLS Model

ELF defines four TLS access models (System V ABI TLS supplement):
- **Local Exec**: Direct offset from TP (thread pointer). Fastest, only for main executable.
- **Initial Exec**: GOT-indirect offset from TP. For dylibs known at load time.
- **Local Dynamic**: `__tls_get_addr()` call with module index. For multiple TLS vars in same module.
- **General Dynamic**: `__tls_get_addr()` call with module index + offset. Most general, for any dylib.

The linker can relax from General Dynamic → Initial Exec → Local Exec at link time.

Storage: `PT_TLS` program header defines the TLS template (initialization data + BSS size).

#### Mach-O TLV (Thread-Local Variable) Descriptors

Mach-O uses a completely different mechanism:
- Section type `S_THREAD_LOCAL_VARIABLES` (`__thread_vars`): array of TLV descriptors
- Section type `S_THREAD_LOCAL_REGULAR` (`__thread_data`): initialization template
- Section type `S_THREAD_LOCAL_ZEROFILL` (`__thread_bss`): zero-filled TLV area

Each TLV descriptor is a struct:
```c
struct TLVDescriptor {
    void* (*thunk)(TLVDescriptor*);  // Initially _tlv_bootstrap, then tlv_get_addr
    unsigned long key;                // pthread_key_t for this TLV
    unsigned long offset;             // Offset into per-thread TLV block
};
```

Access pattern:
1. Code loads the descriptor from `__thread_vars`
2. Calls the `thunk` function pointer
3. On first access, `_tlv_bootstrap` allocates the per-thread block and patches the thunk to `tlv_get_addr`
4. Subsequent accesses go through `tlv_get_addr` directly

Key difference: ELF TLS is integrated into the ABI with designated registers (FS/GS on x86, TPIDR on ARM). Mach-O TLV uses function call indirection, making it more flexible but slower for hot paths.

### 2.5 Symbol Versioning vs Two-Level Namespaces

#### ELF Symbol Versioning

ELF supports per-symbol versioning via `DT_VERSYM`, `DT_VERDEF`, `DT_VERNEED`:
- A library can export multiple versions of the same symbol (e.g., `GLIBC_2.5`, `GLIBC_2.17`)
- Binaries link against a specific version; the loader resolves to the matching version
- Enables ABI evolution without soname bumps
- The `@` and `@@` syntax: `symbol@VERSION` (reference), `symbol@@VERSION` (default definition)

#### Mach-O Two-Level Namespaces

Mach-O achieves ABI stability through a different mechanism:
- Each undefined symbol reference records which dylib ordinal it expects to find the symbol in
- Set by `MH_TWOLEVEL` flag (default since macOS 10.1)
- Benefits: faster symbol lookup (no scanning all loaded images), eliminates symbol collisions
- Compatibility versions in `LC_ID_DYLIB` / `LC_LOAD_DYLIB` serve a similar role to sonames:
  - `compatibility_version`: minimum version the binary is compatible with
  - `current_version`: version of the dylib providing the symbol
  - dyld checks `current_version >= compatibility_version` at load time

The two-level namespace approach means Mach-O doesn't need per-symbol versioning — the dylib itself is the versioning unit.

### 2.6 Key Specifications for a Loader Lab

| Specification | Relevance |
|---|---|
| **System V ABI AMD64 Supplement** | Defines ELF loading conventions, relocation types, TLS model, calling conventions for x86_64 |
| **System V ABI ARM64 Supplement (AAPCS64)** | Same for ARM64 ELF |
| **Apple Mach-O File Format Reference** | Canonical header definitions in `<mach-o/loader.h>`, `<mach-o/nlist.h>`, `<mach-o/fat.h>` |
| **Apple dyld source code** | `apple-oss-distributions/dyld` on GitHub — the authoritative loader behavior |
| **Apple XNU source code** | `apple-oss-distributions/xnu` — kernel side of exec and loading |
| **POSIX.1-2017 (IEEE Std 1003.1-2017)** | §2.12 "Dynamic Linking" defines `dlopen`/`dlsym`/`dlclose` semantics |
| **ELF Specification (gABI)** | Tool Interface Standard, Executable and Linking Format |
| **LSB (Linux Standard Base)** | Practical ELF loading requirements beyond the base spec |

---

## 3. Apple Security and Trust Model — Full Layer Map

### 3.1 Code Signing

#### CMS Signature Format in `LC_CODE_SIGNATURE`

The `LC_CODE_SIGNATURE` load command points to a `SuperBlob` structure in `__LINKEDIT`:
```
SuperBlob (CSMAGIC_EMBEDDED_SIGNATURE = 0xFADE0CC0)
├── CodeDirectory (CSMAGIC_CODEDIRECTORY = 0xFADE0C02)
│   ├── CDHash (SHA-1 and/or SHA-256)
│   ├── Page hashes (hash of each 4KB page of code)
│   ├── Special slot hashes (Info.plist, entitlements, requirements, resources)
│   ├── Team ID
│   ├── Platform, flags, version
│   └── Exec segment info (base, limit, flags)
├── Requirements blob (CSMAGIC_REQUIREMENTS = 0xFADE0C01)
│   └── Designated requirement expression
├── Entitlements blob (CSMAGIC_EMBEDDED_ENTITLEMENTS = 0xFADE7171)
│   └── XML plist
├── DER Entitlements blob (CSMAGIC_EMBEDDED_DER_ENTITLEMENTS = 0xFADE7172)
│   └── DER-encoded entitlements
└── CMS Signature blob (CSMAGIC_BLOBWRAPPER = 0xFADE0B01)
    └── PKCS#7/CMS detached signature over the CodeDirectory
```

#### Code Directory Hash (CDHash)

The CDHash is the canonical identifier for a signed binary:
- It is the hash of the `CodeDirectory` blob itself
- Typically SHA-256 (SHA-1 is legacy)
- 20 bytes (SHA-1) or 32 bytes (SHA-256), often represented as 40/64 hex characters
- Used by: AMFI, Gatekeeper, notarization tickets, trust caches, launch constraints

The kernel computes and caches the CDHash during `process_signature()` in `kern_exec.c`.

#### Designated Requirements

A code requirement is a compiled expression language that specifies what code is allowed:
- Example: `identifier "com.example.app" and anchor apple generic and certificate leaf[subject.CN] = "Developer ID Application: ..."`
- The designated requirement (DR) defines the identity of "this program"
- Can be verified by `SecStaticCodeCheckValidity()` or `codesign -v`

#### Entitlements

Entitlements are key-value pairs embedded in the code signature:
- Stored as both XML plist and DER-encoded blobs
- Checked by AMFI at exec time and by various subsystems at runtime
- Key loader-relevant entitlements:
  - `com.apple.security.cs.disable-library-validation` — allows loading dylibs not signed by the same team
  - `com.apple.security.cs.allow-unsigned-executable-memory` — allows `MAP_JIT` and `W^X` exceptions
  - `com.apple.security.cs.allow-jit` — allows JIT compilation
  - `com.apple.security.cs.allow-dyld-environment-variables` — allows `DYLD_*` env vars even with hardened runtime
  - `com.apple.security.cs.debugger` — allows attaching as a debugger
  - `com.apple.security.get-task-allow` — allows task port access (dev-signed only)
  - `com.apple.security.cs.disable-executable-page-protection` — disables code signing enforcement for pages

#### Signing Identity Types

| Identity | Usage | Distribution |
|---|---|---|
| **Developer ID Application** | Stand-alone distribution outside App Store | Notarization required |
| **Developer ID Installer** | Installer packages | Notarization required |
| **Apple Distribution** | App Store distribution | App Store review |
| **Apple Development** | Local testing / device deployment | Development only |
| **Ad-hoc** | No identity, self-signed with no certificate | Local only, no Gatekeeper |
| **Platform binary** | Apple-signed, part of the OS | Highest trust level |

### 3.2 Gatekeeper

#### What It Checks and When

Gatekeeper is a user-facing policy enforcement layer. It checks at **first launch** of quarantined items:

1. **Quarantine check**: Is the file quarantined? (`com.apple.quarantine` extended attribute)
2. **Signature validation**: Is the code signed with a valid Developer ID?
3. **Notarization check**: Does Apple have a notarization ticket for this CDHash?
4. **Malware scan**: XProtect checks for known malware signatures
5. **Assessment**: `Security.framework` `SecAssessmentCreate()` produces allow/deny with explanation

#### Quarantine Attributes

The `com.apple.quarantine` xattr is set by:
- Safari, Mail, Messages, AirDrop
- Any app using `LSQuarantineProperties` or `NSURLQuarantineProperties`

Format: `QQ;timestamp;source_app_bundle_id;UUID`
- Example: `0083;5f1b3e44;Safari;5A5C5D23-3F25-4E5B-9ABC-6B3D2F5E5F5E`

Flag bits:
- `0x0040` — user has approved this item
- `0x0020` — item has been notarized
- No quarantine attribute = not subject to Gatekeeper (e.g., binaries built locally)

#### Notarization Ticket Stapling

- The notary service produces a ticket (signed blob) for each submitted CDHash
- `stapler staple App.app` embeds the ticket in the bundle
- For DMGs: ticket is stapled to the DMG file itself
- On first launch, if no stapled ticket, macOS checks Apple's servers (OCSP-like)
- Offline machines fall back to previously cached tickets

### 3.3 AMFI (AppleMobileFileIntegrity)

AMFI is a kernel extension (`com.apple.driver.AppleMobileFileIntegrity`) that enforces code signing policy. From the dyld perspective, the AMFI policy is queried during exec via the `getAMFI()` call in `ProcessConfig::Security`.

#### Kernel-Level Code Signature Validation

- The kernel validates code signatures at `exec` time in `process_signature()` and `load_machfile()`
- Code signing flags are set on the process: `CS_VALID`, `CS_SIGNED`, `CS_HARD`, `CS_KILL`, `CS_RESTRICT`, `CS_ENFORCEMENT`, `CS_RUNTIME`, `CS_REQUIRE_LV`, etc.
- `CS_HARD` — don't load invalid pages
- `CS_KILL` — kill the process if a code signing violation occurs
- `CS_RESTRICT` — restricted binary (SIP-protected, `DYLD_*` vars stripped)

#### Page-Level Hash Verification

- Each 4KB page of `__TEXT` has a hash in the CodeDirectory
- On page fault, the kernel computes the hash of the faulted page and compares it to the stored hash
- Mismatch → `SIGKILL` (if `CS_KILL` is set) or the page is marked invalid
- This provides tamper detection at runtime, not just at load time

#### Entitlement Enforcement

AMFI validates entitlements at exec time:
- `IOVnodeHasEntitlement()` / `IOTaskHasEntitlement()` — kernel APIs to check entitlements
- Certain entitlements are restricted to Apple-signed or platform binaries
- `CS_ENTITLEMENT_FLAGS` — bitmask of common entitlement states cached on the process

#### Hardened Runtime Implications

When `CS_RUNTIME` is set (hardened runtime):
- **Library validation** (`CS_REQUIRE_LV`): Only dylibs signed by the same Team ID or Apple can be loaded. Overridden by `com.apple.security.cs.disable-library-validation`.
- **DYLD_* restrictions**: Environment variables that modify loading behavior are stripped. Overridden by `com.apple.security.cs.allow-dyld-environment-variables`.
- **Debugging restrictions**: `task_for_pid()` is denied. Overridden by `com.apple.security.cs.debugger` or `get-task-allow`.
- **Executable memory restrictions**: `MAP_JIT` required for W+X pages. Overridden by `com.apple.security.cs.allow-unsigned-executable-memory`.

### 3.4 SIP (System Integrity Protection)

#### Protected Paths

SIP protects these filesystem locations from modification, even by root:
- `/System/` (entire system volume, read-only APFS snapshot since Catalina)
- `/usr/` (except `/usr/local/`)
- `/bin/`, `/sbin/`
- `/Applications/` system apps
- `/Library/` (partially)

The system volume is mounted as a cryptographically sealed snapshot (SSV — Signed System Volume).

#### CS_RESTRICT Flag

Binaries in SIP-protected paths are marked with `CS_RESTRICT`:
- All `DYLD_*` environment variables are stripped
- `task_for_pid()` is denied
- Library validation is enforced
- The binary cannot be modified or replaced (even by root with SIP enabled)

#### DYLD Environment Variable Stripping

From `ProcessConfig::Security::pruneEnvVars()`:
- For restricted processes, ALL `DYLD_*` environment variables are removed from the process environment
- This happens before dyld processes any load commands
- The function checks `allowEnvVarsPath`, `allowEnvVarsPrint`, `allowEnvVarsSharedCache` flags
- These flags are derived from the AMFI policy query

#### Library Validation for Platform Binaries

For Apple platform binaries:
- Only Apple-signed dylibs can be loaded
- No `DYLD_INSERT_LIBRARIES`
- No interposing
- `DyldCache::isOverridablePath()` controls which cached dylibs can be overridden:
  - `isAlwaysOverridablePath()` — a few specific paths that are always overridable
  - `isProtectedLibSystemPath()` — libSystem paths that can never be overridden

### 3.5 Sandbox (App Sandbox / Seatbelt)

#### How Sandbox Profiles Affect dylib Loading

The macOS sandbox (kernel extension `com.apple.security.sandbox`) applies MACF (Mandatory Access Control Framework) policies:

- **File access**: Sandbox profiles can deny `open()` / `mmap()` on paths outside the sandbox container, which prevents dylib loading from unauthorized locations
- **`mmap` with `MAP_JIT`**: Requires explicit sandbox permission
- **Process exec**: Sandboxed processes may be restricted in what they can exec

#### Loader-Relevant Sandbox Effects

- A sandboxed app loading a dylib from a path outside its sandbox will get `EPERM` at `open()` time
- The sandbox profile can deny `file-read-data` on specific paths
- `dyld` itself runs within the process's sandbox — it cannot bypass sandbox restrictions
- Network access restrictions may affect notarization ticket online checks

#### IPC / Mach Port Restrictions

- Sandboxed processes have restricted `mach_port` access
- `task_for_pid()` is denied for sandboxed processes
- This indirectly affects loader behavior: debugger attachment is restricted, interposition may be denied

### 3.6 TCC (Transparency, Consent, and Control)

TCC is not directly loader-related but affects runtime behavior:

- TCC protects access to: Contacts, Calendar, Photos, Camera, Microphone, Location, etc.
- Access is gated by entitlements AND user consent (TCC database)
- Key interaction with loading: an app must include the correct usage strings (`NS*UsageDescription`) in Info.plist and have the correct entitlements to access TCC-protected resources
- TCC checks the calling process's code signature identity (CDHash/Team ID)
- `com.apple.private.tcc.allow` — special entitlement for TCC bypass (Apple-only)
- TCC decisions are cached per-Team-ID, so re-signing with a different identity resets approvals

### 3.7 Notarization

#### Server-Side Scanning

Apple's notary service:
1. Accepts submission of `.app`, `.dmg`, `.pkg`, or `.zip` archives
2. Scans all embedded executables and dylibs for:
   - Valid Developer ID code signature
   - Hardened runtime enabled
   - No `get-task-allow` entitlement
   - Linked against macOS 10.9+ SDK
   - Properly formatted entitlements
   - Known malware signatures
3. Returns success/failure with a log URL detailing any issues

#### Ticket Stapling

- On success, the notary service generates a ticket keyed to each CDHash in the submission
- `xcrun stapler staple App.app` downloads and embeds the ticket
- For DMGs: `xcrun stapler staple disk-image.dmg`
- Tickets are also published to Apple's CDN for online lookup

#### Online/Offline Ticket Checking

- **Online**: On first launch, macOS contacts `api.apple-cloudkit.com` to check for a notarization ticket
- **Offline**: If a stapled ticket is present, it's validated locally against the CDHash
- **Revocation**: Apple can revoke tickets, causing previously-approved apps to be blocked
- **Grace period**: Some older unnotarized software may still run with a warning dialog

### 3.8 Security Gate Summary: Binary on Disk → Code Running in Process

```
Binary on Disk
    │
    ├─ [1] Gatekeeper Assessment (first launch only, if quarantined)
    │       ├─ Check quarantine xattr
    │       ├─ Validate Developer ID signature
    │       ├─ Check notarization ticket (online or stapled)
    │       └─ XProtect malware scan
    │
    ├─ [2] Kernel exec (posix_spawn/execve)
    │       ├─ exec_check_permissions() — POSIX file permissions
    │       ├─ exec_mach_imgact() — Mach-O format validation
    │       ├─ MAC framework checks (mac_vnode_check_exec)
    │       ├─ load_machfile() — map segments, locate dyld
    │       ├─ Code signature validation (in-kernel)
    │       │     ├─ Verify CMS signature chain
    │       │     ├─ Validate CodeDirectory
    │       │     ├─ Check CDHash against trust caches
    │       │     └─ Set CS_* flags on process
    │       ├─ AMFI policy query (getAMFI)
    │       │     ├─ Determine allowed DYLD_* variables
    │       │     ├─ Determine library validation requirements
    │       │     └─ Set hardened runtime restrictions
    │       ├─ SIP checks (for protected paths)
    │       ├─ Entitlement validation
    │       ├─ Launch constraint checks (mac_proc_check_launch_constraints)
    │       ├─ Sandbox profile application
    │       └─ process_signature() — final signature processing
    │
    ├─ [3] dyld initialization
    │       ├─ Security config from AMFI → prune unauthorized env vars
    │       ├─ Library validation for each loaded dylib
    │       │     ├─ Team ID must match (if library validation enabled)
    │       │     └─ Or dylib must be Apple-signed
    │       ├─ Code signature validation for each mapped dylib
    │       │     └─ Kernel validates page hashes on fault
    │       └─ Initializer execution
    │
    └─ [4] Runtime
            ├─ Page-level hash verification on every page fault
            ├─ TCC checks on protected resource access
            ├─ Sandbox enforcement on all syscalls
            └─ AMFI enforcement on mach port operations
```

---

## 4. Interoperability With Apple's Stack

### 4.1 Binary Format Compatibility

**Can we produce/consume the same Mach-O files?**

Yes, with requirements:
- Must correctly produce the `mach_header_64` with proper magic, cputype, and flags
- Must emit valid `LC_SEGMENT_64` commands with correct vmaddr, vmsize, fileoff, filesize, maxprot, initprot
- Must produce valid `LC_CODE_SIGNATURE` if the binary is to pass codesign validation
- For arm64e: chained fixups with pointer authentication metadata are mandatory
- The `__LINKEDIT` segment must be the last segment and contain all metadata (symtab, strtab, code sig, etc.)
- Fat (universal) binaries require `fat_header` + `fat_arch` entries per slice

### 4.2 dyld API Compatibility

**Can code call dyld functions and get correct answers?**

The public dyld API surface (from `<mach-o/dyld.h>`):
- `_dyld_image_count()`, `_dyld_get_image_header()`, `_dyld_get_image_name()`, `_dyld_get_image_vmaddr_slide()` — image enumeration
- `_dyld_register_func_for_add_image()` / `_dyld_register_func_for_remove_image()` — notification callbacks
- `_NSGetExecutablePath()` — path to main executable
- `_dyld_shared_cache_contains_path()` — shared cache queries

For a loader lab producing outputs that model dyld's behavior:
- These APIs are implemented inside dyld itself (which lives at `/usr/lib/dyld`)
- They access internal dyld data structures; there is no stable binary interface
- Compatibility means producing the same observable behavior (same images loaded, same addresses, same init order)
- The `dyld_all_image_infos` structure is the kernel↔dyld↔debugger interface defined in `<mach-o/dyld_images.h>`

### 4.3 dlopen/dlsym Compatibility (POSIX API Surface)

The POSIX API (`<dlfcn.h>`) is implemented by `libdyld.dylib` (part of libSystem), which trampolines into dyld:

```c
dlopen()  → dyld4::APIs::dlopen_from()
dlsym()   → dyld4::APIs::dlsym()
dlclose() → dyld4::APIs::dlclose()
dlerror() → dyld4::APIs::dlerror()
dladdr()  → dyld4::APIs::dladdr()
```

For interoperability:
- A loader lab must resolve symbols through the same search algorithm dyld uses
- Two-level namespace aware: `dlsym(handle, sym)` searches only the specified image tree
- `RTLD_DEFAULT` searches all images in load order
- Thread-safety: all dlopen/dlsym operations in dyld are serialized with a recursive lock

### 4.4 Objective-C Runtime Compatibility

The Objective-C runtime (`libobjc.A.dylib`) integrates tightly with dyld:

1. **Class registration**: When dyld maps an image containing ObjC metadata, the runtime's `_objc_init()` callback processes:
   - `__objc_classlist` section → registers classes
   - `__objc_catlist` section → attaches categories
   - `__objc_protolist` section → registers protocols
   - `__objc_selrefs` section → uniquifies selector strings
   - `__objc_classrefs` / `__objc_superrefs` → resolves class references

2. **`+load` methods**: Called during image initialization, before C++ constructors. Order: superclass `+load` before subclass `+load`, categories `+load` in link order.

3. **Message dispatch**: Uses the ObjC method cache and vtable stubs. The shared cache contains pre-computed ObjC optimization data (`objcSelectorHashTable`, `objcClassHashTable`, `objcProtocolHashTable`).

4. **Non-fragile ABI**: Instance variable offsets are resolved at runtime, allowing superclass layout changes without recompilation.

For a loader lab: ObjC metadata detection is critical for understanding init dependencies. The `macho-objc` crate should parse `__objc_classlist` to identify `+load` methods and model their ordering.

### 4.5 Swift ABI Stability

Swift achieved ABI stability in Swift 5.0 (macOS 10.14.4):
- Swift runtime (`libswiftCore.dylib`) is part of the OS
- The compiler emits stable calling conventions, type metadata layout, and value witness tables
- Type metadata accessors follow a stable pattern
- Protocol conformance records are in `__swift5_proto` section
- Swift type descriptors are in `__swift5_types` section

The shared cache contains `SwiftOptimizationHeader` with pre-computed Swift metadata.

For interoperability: a loader lab needs to recognize Swift metadata sections but doesn't need to interpret them deeply unless modeling Swift runtime initialization.

### 4.6 Framework Resolution

#### System Frameworks (`/System/Library/Frameworks/`)

- In the shared cache: the framework's dylib is embedded in the cache; the on-disk path doesn't exist
- Resolution: dyld checks the cache first, then falls back to disk
- Umbrella frameworks: `LC_SUB_FRAMEWORK` declares the umbrella; sub-frameworks can only be linked through the umbrella

#### Private Frameworks (`/System/Library/PrivateFrameworks/`)

- Not part of the public API; may change without notice
- Same resolution mechanism as public frameworks
- Usage restricted by:
  - App Store review (will reject references to private frameworks)
  - Library validation (third-party binaries can't load them if signed with hardened runtime)

#### Embedded Frameworks

- Located at `Foo.app/Contents/Frameworks/Bar.framework/`
- Resolved via `@rpath` — the app's `LC_RPATH` typically includes `@executable_path/../Frameworks`
- Must be code-signed with the same Team ID or with library validation disabled

### 4.7 dyld Shared Cache Interaction

**Can a loader lab use the shared cache?**

- The shared cache is mapped by the kernel before dyld runs (or dyld maps it during init)
- It's a complex binary format with its own header (`dyld_cache_header`) and internal tables
- Reading it is feasible: Apple provides `dyld_shared_cache_extract_dylibs` and `dsc_extractor` tools
- A loader lab can:
  - Parse the cache format to enumerate contained dylibs
  - Extract individual dylib images from the cache
  - Use `_dyld_shared_cache_contains_path()` to check membership
  - But cannot easily "use" the pre-bound cache, as the binding decisions are specific to Apple's ABI

**Must a loader lab use it?**

No, but without it, system dylibs are unavailable on macOS 11+. Options:
1. Extract dylibs from the cache for analysis
2. Model the cache's contents as "pre-loaded images"
3. Use the cache as a black box (acknowledge it exists, run fixtures that don't depend on system dylibs)

### 4.8 Interposing

#### `DYLD_INSERT_LIBRARIES`

- Dylibs listed in `DYLD_INSERT_LIBRARIES` are loaded before all other dylibs
- They can provide symbol definitions that override (interpose) symbols in other dylibs
- Requires `allowInterposing` security permission

#### `__DATA,__interpose` Section

A dylib can declare interpositions without `DYLD_INSERT_LIBRARIES`:
```c
typedef struct {
    const void* replacement;
    const void* replacee;
} interpose_tuple;

__attribute__((used, section("__DATA,__interpose")))
static const interpose_tuple interposers[] = {
    { (const void*)my_malloc, (const void*)malloc },
};
```

dyld processes `__interpose` sections during fixup:
- All references to the `replacee` symbol are redirected to `replacement`
- Applies to all images loaded after the interposing image
- Does NOT affect direct calls within the same image (already bound at link time)

---

## 5. Industry Best Practices for Loader/OS Projects

### 5.1 Darling

**Project**: Linux-based macOS compatibility layer (darlinghq.org)

**Architecture**:
- Translates Mach-O binaries to run on Linux
- Implements a compatibility kernel module (`darling-mach`) providing Mach IPC, BSD syscall layer
- Has its own dyld implementation that runs inside a "prefix" (like Wine's wineprefix)
- Loads Mach-O segments via `mmap()` on Linux
- Implements key macOS frameworks (CoreFoundation, Foundation, AppKit stubs)

**Loader approach**:
- Runs the real Apple `dyld` inside the translated environment where possible
- Provides a `mldr` (Mach-O Loader) that bootstraps the process:
  1. Parses the Mach-O header
  2. Maps segments with appropriate protections
  3. Sets up the stack with `apple[]` strings
  4. Transfers control to dyld's entry point
- Does NOT reimplement dyld's full symbol resolution — delegates to the real dyld

**Relevant lessons**:
- Separating "format parsing" from "loading" from "execution" is essential
- The stack layout (argc, argv, envp, apple[]) must be exact
- dyld is very sensitive to the `ProcessConfig` it receives from the kernel

### 5.2 Wine

**Project**: Windows compatibility layer for POSIX systems

**Loader architecture**:
- `wine-preloader`: Maps the PE binary and `ntdll.dll` into the address space
- `ntdll.dll` equivalent: Implements the Windows loader (`LdrpLoadDll`, `LdrpInitializeProcess`)
- Separation of concerns:
  - **Format parser** (PE): understands the binary format
  - **Mapper**: creates virtual memory mappings
  - **Resolver**: processes imports/relocations
  - **Init runner**: calls `DllMain()` in dependency order

**Relevant patterns for darwin-loader-lab**:
- Use a "preloader" that only does minimal mapping, then hands off to a more complex loader
- Maintain a clean separation between the format parser and the loading logic
- Test against known-good behavior (Wine has extensive test suites)

### 5.3 QEMU User-Mode

**Project**: Binary translation and foreign-architecture execution

**Loader approach** (`linux-user/elfload.c`):
- Parses ELF headers and loads segments
- Sets up the auxiliary vector (equivalent of Mach-O's apple[] strings)
- Maps the interpreter (`ld.so`) and transfers control
- Handles format-specific details (ELF, with stubs for other formats)

**Relevant patterns**:
- Modular loader design: per-format loader modules
- Clean ABI boundary between format loading and execution
- Auxiliary data structures parallel Apple's `apple[]` string vector

### 5.4 musl libc's ld.so

**Project**: Minimal, standards-conforming C library with clean-room dynamic linker

**Architecture** (`ldso/dynlink.c`, ~3000 lines):
- Single-file implementation of the entire dynamic linker
- Clear, readable code with minimal dependencies
- No symbol versioning complexity (minimal support)
- No `LD_AUDIT` or `dlmopen()` complexity

**Design principles applicable to darwin-loader-lab**:
1. **Single-pass loading**: Load all dependencies, then relocate all at once, then run initializers
2. **Minimal state**: Very few global variables; most state is per-loaded-object
3. **Dependency graph as simple linked list**: Each loaded DSO has a pointer to its next
4. **Bottom-up init**: Walk the dependency graph depth-first, init leaves before roots
5. **No lazy binding**: Simplifies the loader significantly (all binds are eager)
6. **Clean error handling**: Each failure point has a clear error message

**Specific patterns to adopt**:
- musl's `struct dso` is the equivalent of darwin-loader-lab's per-image data structure
- musl's `load_library()` → `map_library()` → `relocate()` → `do_init_fini()` pipeline maps cleanly to the darwin-loader-lab crate architecture

### 5.5 FreeBSD Mach-O Support

FreeBSD does not have a native Mach-O compatibility layer. The `binmisc` kernel module allows registering foreign binary format handlers (similar to Linux's `binfmt_misc`), but no Mach-O implementation exists in the base system.

The closest is FreeBSD's Linux binary compatibility layer (Linuxulator), which handles ELF but not Mach-O.

### 5.6 Rust Ecosystem: goblin Crate

**Crate**: `goblin` (crates.io/crates/goblin)

**Architecture**:
```rust
pub struct MachO<'a> {
    pub header: Header,
    pub load_commands: Vec<LoadCommand>,
    pub segments: Segments<'a>,
    pub symbols: Option<Symbols<'a>>,
    pub libs: Vec<&'a str>,          // dylib dependencies
    pub rpaths: Vec<&'a str>,        // LC_RPATH entries
    pub entry: u64,                  // entry point VM address
    pub old_style_entry: bool,       // LC_UNIXTHREAD vs LC_MAIN
    pub name: Option<&'a str>,       // LC_ID_DYLIB name
    pub is_64: bool,
    pub little_endian: bool,
    export_trie: Option<ExportTrie<'a>>,
    bind_interpreter: Option<BindInterpreter<'a>>,
}
```

**Key design decisions**:
- Zero-copy parsing: `MachO<'a>` borrows from the input `&[u8]`
- Lazy evaluation: exports and imports are parsed on demand
- Multi-arch support: `Mach::Fat(MultiArch)` vs `Mach::Binary(MachO)`
- Cross-format: same crate handles ELF, PE, Mach-O, archives
- Endian-aware: uses `scroll` crate for endian-safe parsing

**Modules relevant to darwin-loader-lab**:
- `goblin::mach::load_command` — comprehensive load command parsing
- `goblin::mach::exports` — export trie walking
- `goblin::mach::imports` — bind opcode interpretation
- `goblin::mach::bind_opcodes` — raw opcode definitions
- `goblin::mach::segment` — segment and section iteration

**Comparison with darwin-loader-lab's `macho-parse`**:
- `goblin` is a general-purpose parser; `macho-parse` can be more focused
- `goblin` proves that zero-copy Mach-O parsing in Rust is practical and performant
- darwin-loader-lab should consider using `goblin` as a reference implementation or even a dependency for validation

### 5.7 Published Research and Formal Work

#### Academic Papers

1. **"Formal Verification of a Dynamic Linker"** — Work on formally verifying ELF loading semantics. Demonstrates that loader behavior can be specified precisely enough for formal methods.

2. **"An Evil Copy: How the Loader Betrays You"** (NDSS 2017) — Shows how loader behaviors can be exploited; relevant for understanding security boundaries.

3. **"How the ELF Ruined Christmas"** (USENIX Security 2015) — Detailed analysis of ELF loading attack surface; analogous concerns apply to Mach-O.

4. **"Mach-O Internals"** by Jonathan Levin (*OS Internals books) — The most comprehensive non-Apple reference on Mach-O format and loading. Key resource for data structure details.

#### Key Open-Source References

| Project | Language | What It Demonstrates |
|---|---|---|
| `apple-oss-distributions/dyld` | C++ | Authoritative loader behavior, `ProcessConfig`, `JustInTimeLoader` |
| `apple-oss-distributions/xnu` | C | Kernel exec path, AMFI, code signing |
| `m4b/goblin` | Rust | Zero-copy Mach-O/ELF/PE parsing |
| `nicknisi/darling` / `darlinghq` | C/C++ | Mach-O loading on Linux |
| `ArtSabintsev/Lipo` | Swift | Fat binary manipulation |
| `nickhutchinson/libdyld` | C | Standalone dyld API stub library |
| `keith/dyld-shared-cache-extractor` | Rust | Shared cache extraction |
| `blacktop/ipsw` | Go | iOS/macOS binary analysis toolkit |
| `thefloweringash/sigtool` | Rust | Code signature parsing and manipulation |

### 5.8 Architectural Patterns to Adopt

Based on the survey of these projects:

1. **Layered architecture** (musl, Wine, darwin-loader-lab's current design):
   ```
   Parse → Plan → Resolve → Bind → Execute
   ```
   Each layer is independently testable.

2. **Fixture-driven testing** (Wine, darwin-loader-lab's current approach):
   Known-good binaries with expected outputs, compared against host behavior.

3. **JSON-serializable intermediate representations** (darwin-loader-lab's invariant):
   Every stage emits JSON. This enables diff-based testing and CI integration.

4. **Separation of "can parse" from "will execute"** (goblin, darwin-loader-lab):
   The parser should never crash on malformed input; it should return structured errors.

5. **Security as a separate concern** (dyld's `ProcessConfig::Security`):
   Security policy decisions should be centralized and checked before any loading occurs.

6. **Incremental capability** (darwin-loader-lab's phased roadmap):
   Start with inspection, progress to planning, then execution. Don't try to build a full loader from day one.

7. **Reference validation** (darwin-loader-lab's `macho-diff`):
   Always compare computed results against ground truth from the real OS.

---

## 6. Key Specifications and References

### Primary Sources

| Document | URL / Location |
|---|---|
| Apple Mach-O header definitions | `<mach-o/loader.h>`, `<mach-o/nlist.h>`, `<mach-o/fat.h>`, `<mach-o/reloc.h>` |
| Apple dyld source code | `github.com/apple-oss-distributions/dyld` |
| Apple XNU kernel source | `github.com/apple-oss-distributions/xnu` |
| Apple Developer: Code Signing Guide | `developer.apple.com/documentation/security/code-signing` |
| Apple Developer: Hardened Runtime | `developer.apple.com/documentation/security/hardened-runtime` |
| Apple Developer: Notarization | `developer.apple.com/documentation/security/notarizing-macos-software-before-distribution` |
| Apple Technical Note TN3126: dyld | `developer.apple.com/documentation/technotes/tn3126-inside-code-signing-hashes` |

### ELF and POSIX

| Document | Description |
|---|---|
| System V ABI (gABI) | Generic ELF specification: `refspecs.linuxfoundation.org/elf/gabi4+/` |
| System V ABI AMD64 Supplement | x86_64-specific ELF conventions |
| System V ABI ARM64 (AAPCS64) | ARM64-specific ELF conventions |
| POSIX.1-2017 (IEEE Std 1003.1) | §2.12 Dynamic Linking: `dlopen`, `dlsym`, `dlclose`, `dlerror` |
| ELF TLS ABI | `akkadia.org/drepper/tls.pdf` (Ulrich Drepper's TLS document) |
| Linux Standard Base (LSB) | Practical ELF loading requirements |

### Books

| Title | Author | Relevance |
|---|---|---|
| *\*OS Internals Volume I: User Mode* | Jonathan Levin | Most detailed Mach-O and dyld reference |
| *Linkers and Loaders* | John Levine | Foundational concepts applicable to all binary formats |
| *Mac OS X Internals* | Amit Singh | XNU architecture, Mach IPC, process lifecycle |
| *Advanced Apple Debugging & Reverse Engineering* | raywenderlich.com | Practical dyld debugging techniques |
