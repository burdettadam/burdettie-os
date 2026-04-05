# Chapter 3: The Links

## Scene 3.1: Why Dynamic Linking

The binary is loaded. Chapters 1 and 2 got us here — the Mach-O header is parsed, load commands decoded, segments mapped with correct VM protections. The `__TEXT` segment is executable, `__DATA` is writable, `__DATA_CONST` is read-only post-fixup, and every page sits at the virtual address the binary expects. By every structural measure, this program is ready to run.

So let's run it.

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/strings
[1]    54321 segmentation fault  macho-inspect --exec strings
```

SIGSEGV. Immediately. Not deep in some library call, not in some edge case — on literally the first function the binary tries to call. Let's look at the crash:

```
SIGSEGV at PC 0x0000000000000000
  x0  = 0x0000000100004000
  x1  = 0x0000000000000003
  x30 = 0x0000000100003f48   (return address in __TEXT)
```

PC is zero. The binary jumped to address zero. That's not a wild pointer or a use-after-free — it's *no pointer at all*. The CPU tried to execute code at address zero because the function pointer it dereferenced was null. The return address in x30 points to our binary's `__TEXT` segment — that's the call site. The binary ran exactly one instruction into its first function call and crashed.

Let's look at what it tried to call through. On ARM64, indirect calls go through the Global Offset Table — the GOT. It's a table of function pointers in the `__DATA_CONST` segment. Let's dump it:

```
__DATA_CONST,__got (0x100008000):
  +0x000: 0x0000000000000000  ; slot 0 — _printf
  +0x008: 0x0000000000000000  ; slot 1 — _strlen
  +0x010: 0x0000000000000000  ; slot 2 — _malloc
  +0x018: 0x0000000000000000  ; slot 3 — _free
  +0x020: 0x0000000000000000  ; slot 4 — _open
  +0x028: 0x0000000000000000  ; slot 5 — _close
  +0x030: 0x0000000000000000  ; slot 6 — _read
  +0x038: 0x0000000000000000  ; slot 7 — _write
  +0x040: 0x0000000000000000  ; slot 8 — _fopen
  +0x048: 0x0000000000000000  ; slot 9 — _fclose
  ...
```

Every entry is zero. Every function pointer this binary depends on — `printf`, `strlen`, `malloc`, `open`, `close`, `write`, `fopen`, `fclose` — all null. The binary was compiled to call functions *through* this table. The `__TEXT` code contains instructions like:

```asm
adrp    x8, _printf@GOTPAGE
ldr     x8, [x8, _printf@GOTPAGEOFF]
blr     x8
```

Load the GOT entry for `printf`. Branch to it. If the GOT entry is zero, that branch goes to PC 0x0. SIGSEGV.

On macOS, filling in the GOT is `dyld`'s job. Apple's dynamic linker — the *d*ynamic *l*ink *ed*itor, `/usr/lib/dyld` — runs before `main()` and fills in every GOT entry with the real address of each function. It's the first code that runs in any new process, before your program, before any framework initializer, before `__attribute__((constructor))` functions. When you set a breakpoint on `main()` in lldb and hit it, dyld has already executed tens of thousands of instructions: mapping libraries, resolving symbols, writing addresses, running initializers. By the time `main()` runs, the GOT is fully populated and every function call just works.

We mapped the binary. We didn't link it. We did `dyld`'s loading — segments are mapped, pages have correct permissions — but we skipped `dyld`'s *linking*: the part that turns symbol references into addresses.

The binary is loaded but not alive. Like a nervous system with every neuron in place but no synapses connected. The structure is perfect. The function is absent.

There's a difference between loading a binary and making it work. Chapter 2 handled loading — the mechanical act of placing bytes at the right addresses with the right permissions. This chapter handles linking — the semantic act of connecting symbol references to their definitions, of turning names into addresses, of making the binary's GOT reflect reality rather than zeros.

There's a difference between loading a binary and making it work. Chapter 2 handled loading — the mechanical act of placing bytes at the right addresses with the right protections. This chapter handles linking — the semantic act of connecting symbol references to their definitions, of turning names into addresses, of making the binary's GOT reflect reality.

### Static vs. Dynamic

To understand why the GOT exists, consider the alternative. A *statically linked* binary contains everything it needs. Every function it calls — `printf`, `malloc`, `strlen` — is compiled directly into the executable. The machine code for those functions lives in the binary's `__TEXT` segment alongside the program's own code. One file, zero external dependencies.

This sounds ideal. Go produces static binaries by default. Rust can. Some embedded systems require it. But macOS has never worked this way for system libraries, and the reasons run deep.

**Memory sharing.** If every binary compiled in its own copy of libc, every process on the system would carry its own `printf` implementation, its own `malloc` arena initialization, its own string comparison routines. The machine code alone for libc is several megabytes. Multiply by the hundreds of processes running on a modern macOS system. With dynamic linking, one copy of libc's code lives in physical memory, mapped read-only into every process that needs it. The kernel's copy-on-write page sharing handles this transparently.

The savings are concrete. `libsystem_c.dylib` alone contains ~3,500 exported functions spanning about 4.5 MB of compiled code. Without sharing, 200 running processes would consume 900 MB just for libc. With dynamic linking, it's 4.5 MB. The shared cache (Scene 3.7) takes this further by coalescing *all* system libraries into a single contiguous mapping, but even the basic shared-library model drops memory consumption by orders of magnitude.

**Version independence.** On March 7, 2023, Apple patched CVE-2023-23530 in Foundation — a deserialization vulnerability that allowed arbitrary code execution. Every dynamically linked binary on the system picked up the fix immediately after the OS update. No recompilation. Every statically linked binary containing Foundation would have needed to be rebuilt and redeployed — every app, every tool, every daemon. Consider the scale: a macOS system has thousands of binaries that link Foundation, from Finder to Mail to Xcode to every Swift app in `/Applications`. A single `softwareupdate` command patches them all. Static linking would require rebuilding every one.

**Extensibility.** macOS's entire plug-in and framework architecture is built on dynamic loading. An application can load a framework at runtime that didn't exist when the app was compiled. This is the foundation of Xcode plugins, Safari extensions, Quick Look generators, Spotlight importers, and the entire bundle system. None of this works with static linking. The `NSBundle` class — which loads resources, code, and interface files from disk bundles — is the cornerstone of macOS modularity. The App Store review process relies on `dlopen`-time code signature verification. Notarization works by validating cryptographic seals on dynamic libraries.

**Shared state.** Some system functions maintain global state across the process. `malloc` manages a single heap arena. `dispatch_queue_create` feeds into the global thread pool. `objc_getClass` queries the single ObjC runtime metadata table. If each library compiled in its own copy of these subsystems, each copy would have its own separate heap, its own independent thread pool, its own class registry that doesn't know about classes registered by other copies. Pointers from one heap couldn't be freed by another. The process would have multiple contradictory views of reality.

The cost is complexity. When you dynamically link, somebody has to find those libraries at runtime, load them into the process, and wire up all those function pointers before the binary can do anything useful. On macOS, that's `dyld`.

We don't have `dyld`. So that's us.

### Install Names and @rpath

Every dynamic library on macOS has an *install name* — a path baked into the library itself that says "this is where I'm supposed to live." When `ld64` (Apple's static linker) links your binary against `libfoo.dylib`, it doesn't record the path *you* passed on the command line. It reads `libfoo.dylib`'s LC_ID_DYLIB load command to discover the library's install name, and it records *that* path. This means the library author controls how consumers reference it — a subtle but important design decision.

Look at a binary's `LC_LOAD_DYLIB` load commands and you'll see these names:

```
LC_LOAD_DYLIB  /usr/lib/libSystem.B.dylib          (compat 1.0.0, current 1345.0.0)
LC_LOAD_DYLIB  /usr/lib/libc++.1.dylib              (compat 1.0.0, current 1700.255.4)
LC_LOAD_DYLIB  @rpath/DVTFoundation.framework/Versions/A/DVTFoundation
```

The first two are absolute paths — they point to system libraries at fixed, well-known locations (or rather, at locations that *used* to be fixed before the shared cache consumed them). The third uses `@rpath`, which is a search path variable. The binary also contains `LC_RPATH` commands:

```
LC_RPATH  @executable_path/../Frameworks
LC_RPATH  @executable_path/../SharedFrameworks
LC_RPATH  @loader_path/../Frameworks
```

Three special path variables govern library search:

- **`@executable_path`** — the directory containing the main executable. For `/Applications/Xcode.app/Contents/MacOS/Xcode`, this is the `MacOS` directory. Most App Store apps use `@executable_path/../Frameworks` as their sole rpath, placing all embedded dylibs in a `Frameworks` directory alongside the executable.

- **`@loader_path`** — the directory containing the binary that declares this dependency. This is *not* always the main executable. When framework A loads framework B, `@loader_path` is A's directory. This distinction matters for nested framework bundles. Xcode's IDEFoundation framework uses `@loader_path/../Frameworks` to find its own sub-dependencies like DVTServices without knowing where the main Xcode binary lives.

- **`@rpath`** — a multi-valued search variable. `dyld` tries each `LC_RPATH` entry in order, substituting `@rpath` with each path until it finds a file that exists. The order matters — if a framework appears at both `@executable_path/../Frameworks/DVTFoundation` and `@executable_path/../SharedFrameworks/DVTFoundation`, the first matching rpath wins.

Each `LC_LOAD_DYLIB` also records two version numbers: a *compatibility version* (the minimum version your binary requires) and a *current version* (the version that was available when you linked). At load time, `dyld` checks that the loaded library's current version is at least as large as your compatibility version. This is how Apple deprecates old APIs without breaking existing binaries — the compatibility version only advances when the ABI changes, not when new APIs are added.

The system is elegant and complicated. A single command-line tool from Xcode can use all three mechanisms. `momc` (the Core Data model compiler) has five `LC_RPATH` entries and eight `LC_LOAD_DYLIB` commands using `@rpath`. Resolving its dependencies requires searching multiple directories in a specific order. Get the order wrong and you load the wrong version of a framework — a bug that manifests as subtle symbol mismatches rather than clean load failures.

### The Scope Question

At this point, a reasonable engineer might ask: do we really need to build an *entire* dynamic linker? Can't we find a shortcut?

We explored three, and the failures are instructive.

**Shortcut 1: Invoke `dyld` directly.** We tried calling `_dyld_register_func_for_add_image` and other private `dyld` APIs to inform the system linker about our mapped binary. This doesn't work. `dyld` only manages images it loaded itself through the kernel's `execve` → `dyld_start` path. It maintains internal data structures — an image list indexed by load address, a notification callback chain keyed by Mach-O header pointer, and a symbol resolution cache indexed by (image, ordinal) pairs — that assume it performed the mapping. You can't graft a manually mapped image into this state without corrupting it. We crashed trying, in `dyld::addImage` where it dereferences an `ImageLoader*` that doesn't exist in its allocated pool.

**Shortcut 2: Use `dlopen` on the binary.** Calling `dlopen` on the macOS binary we want to run would let `dyld` handle all the linking. But `dlopen` will parse the Mach-O header and refuse to load it on several grounds. On macOS, the binary's `LC_BUILD_VERSION` says platform 1 (macOS), and `dlopen` checks that this matches the host process — but that only matters if the binary is a dylib, not an executable. The real problem is `dlopen` will try to map the binary's segments at its preferred load address, conflicting with our own mapping. More fundamentally: our entire goal is to run macOS binaries on Linux. Linux's `dlopen` handles ELF files. It has no concept of load commands, segments named `__TEXT`, or two-level namespaces.

**Shortcut 3: Pre-resolve everything with `otool`.** We could use macOS tools to extract the symbol table, resolve everything ahead of time, and patch the binary before mapping. This is actually feasible for a single binary — dump the imports with `otool -bind`, resolve against the system, write the addresses into a sidecar file, apply at load time. But it creates a maintenance nightmare: every binary needs pre-processing, the sidecar file must match the exact binary version, libraries loaded transitively at runtime can't be pre-resolved, and the whole approach collapses when we try to run on Linux where `dyld` doesn't exist to resolve against.

There is no shortcut. We're building a loader. The linker is part of the loader.

The answer changes how you think about the project. We're not building a tool that calls into macOS's infrastructure with some clever API usage. We're building a *replacement* for part of that infrastructure. We're writing code that does what `dyld` does: parse format-specific fixup metadata, walk import tables, resolve symbol names to addresses, sign pointers, load dependency graphs, and write the final addresses into the binary's data pages. We're doing it from scratch, in Rust, on both macOS and Linux.

The next six scenes build that linker piece by piece: first the two symbol reference formats Apple uses (old and new), then PAC pointer authentication, then the symbol resolver that turns names into addresses, then the dependency graph walker, and finally the shared cache — a single 2GB file that swallowed every system library on macOS.

By the end, C-only tools run. `strings`, `nm`, `size` — tools that call `printf` and `fopen` and nothing more exotic. Simple. But sufficient to prove the linker works.

---

## Scene 3.2: Import Binds

A Mach-O binary describes its dynamic symbol references in one of two formats. Older binaries (and some newer ones compiled for compatibility) use *bind opcodes* — a compact bytecode language stored in the `LC_DYLD_INFO_ONLY` load command. Newer binaries use *chained fixups* via `LC_DYLD_CHAINED_FIXUPS`. We'll build both decoders, starting with the older format. Understanding bind opcodes makes the modern format's cleverness (and its traps) clearer.

Both formats solve the same problem: the binary contains references to symbols defined in other libraries, and those references need to be resolved to concrete addresses before the code can execute. The difference is *where* the metadata lives: bind opcodes store a separate instruction stream, while chained fixups embed metadata directly in the data pages.

### LC_DYLD_INFO_ONLY

The `LC_DYLD_INFO_ONLY` load command was introduced alongside Mach-O's compressed LINKEDIT format around 2009, replacing the older `LC_DYSYMTAB`-based approach that used full-size relocation entries (8 bytes each, with no compression). The command points to five compressed data regions in the `__LINKEDIT` segment:

```
cmd           LC_DYLD_INFO_ONLY
cmdsize       48
rebase_off    0x20000    rebase_size    96
bind_off      0x20060    bind_size      248
weak_bind_off 0x00000    weak_bind_size 0
lazy_bind_off 0x20158    lazy_bind_size 480
export_off    0x20338    export_size    312
```

Each region serves a different purpose: the *rebase* stream adjusts internal pointers when ASLR slides the binary (covered in Chapter 2). The *bind* stream resolves external symbol references — the ones that need real addresses from other libraries. The *lazy bind* stream also resolves external symbols, but defers resolution until the first call. The *weak bind* stream handles symbols that may or may not exist (the linker's version of optional dependencies — used for things like `_objc_opt_class` which only exists on newer runtimes). The *export trie* advertises symbols this binary provides to others, encoded as a DAWG (Directed Acyclic Word Graph) — the same data structure behind fast dictionary completion.

We care most about the bind and lazy bind streams. They contain the instructions that tell us which GOT entries need filling and what goes in them.

### The Opcode Virtual Machine

The bind opcode stream is, conceptually, a program for a tiny virtual machine. Each byte in the stream is an instruction. The upper 4 bits are the opcode. The lower 4 bits are an immediate operand:

```rust
// File: crates/macho-parse/src/fixups/mod.rs

const BIND_OPCODE_MASK: u8     = 0xF0;
const BIND_IMMEDIATE_MASK: u8  = 0x0F;

const BIND_OPCODE_DONE: u8                              = 0x00;
const BIND_OPCODE_SET_DYLIB_ORDINAL_IMM: u8             = 0x10;
const BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB: u8            = 0x20;
const BIND_OPCODE_SET_DYLIB_SPECIAL_IMM: u8             = 0x30;
const BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM: u8     = 0x40;
const BIND_OPCODE_SET_TYPE_IMM: u8                      = 0x50;
const BIND_OPCODE_SET_ADDEND_SLEB: u8                   = 0x60;
const BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB: u8       = 0x70;
const BIND_OPCODE_ADD_ADDR_ULEB: u8                     = 0x80;
const BIND_OPCODE_DO_BIND: u8                           = 0x90;
const BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB: u8             = 0xA0;
const BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED: u8       = 0xB0;
const BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB: u8  = 0xC0;
```

Thirteen opcodes. The VM maintains seven pieces of state: a segment index, an offset within that segment, a library ordinal, a symbol name, a bind type, an addend, and a weak-import flag. The `SET_*` opcodes update this state. The `DO_BIND*` opcodes perform the actual bind — recording that the resolved address should be written to the current segment+offset — then advance the offset.

Think of it like a cursor moving through the GOT. `SET_SEGMENT_AND_OFFSET_ULEB` positions the cursor. `SET_SYMBOL_TRAILING_FLAGS_IMM` sets what we're looking for. `DO_BIND` writes the result and advances.

The numeric arguments use ULEB128 and SLEB128 encoding — *unsigned* and *signed Little Endian Base 128*. This is the same variable-length integer encoding used in DWARF debug info and WebAssembly. Each byte's lower 7 bits carry data; the high bit indicates whether more bytes follow. The value `0x00` encodes zero in one byte. The value `0x80 0x02` encodes 256 (0x02 shifted left 7 bits, plus the lower 7 bits of 0x80 which are zero). In practice, most offsets in bind streams fit in one or two ULEB128 bytes.

The design is reminiscent of how you'd hand-compress a list of (segment, offset, symbol) triples. Instead of storing each triple independently (which would be ~40 bytes per bind for a 64-bit segment offset plus a symbol string), the opcode format exploits the fact that consecutive binds typically target the same segment, the same library, and offsets that advance by the pointer size. So the common case — "bind the next GOT slot from the same library" — costs 2 bytes: `SET_SYMBOL` (opcode + name) + `DO_BIND` (1 byte).

The three `SET_DYLIB_ORDINAL` variants handle different ordinal sizes. `SET_DYLIB_ORDINAL_IMM` packs ordinals 1-15 into the immediate — one byte total. `SET_DYLIB_ORDINAL_ULEB` handles larger ordinals via ULEB128 — needed for binaries that link against more than 15 libraries. `SET_DYLIB_SPECIAL_IMM` encodes three special ordinals: 0 (self — the binary imports from itself, used for re-exports), -1 (main executable — a dylib importing from its host), and -2 (flat lookup — search all loaded images, the pre-two-level-namespace behavior).

### Walking a Real Opcode Stream

Let's trace through real bytes. Here's a hex dump from a simple tool's bind section:

```
Offset  Hex                                 Meaning
------  ---                                 -------
0x000:  11                                  SET_DYLIB_ORDINAL_IMM(1)
0x001:  40 5F 70 72 69 6E 74 66 00          SET_SYMBOL("_printf")
0x00A:  72 00                               SET_SEGMENT(2), offset=ULEB(0)
0x00C:  90                                  DO_BIND → write _printf to seg2+0x000
0x00D:  40 5F 73 74 72 6C 65 6E 00          SET_SYMBOL("_strlen")
0x016:  90                                  DO_BIND → write _strlen to seg2+0x008
0x017:  40 5F 6D 61 6C 6C 6F 63 00          SET_SYMBOL("_malloc")
0x020:  90                                  DO_BIND → write _malloc to seg2+0x010
0x021:  40 5F 66 72 65 65 00                SET_SYMBOL("_free")
0x028:  90                                  DO_BIND → write _free to seg2+0x018
0x029:  00                                  DONE
```

Let's decode each instruction:

**Byte 0x00: `0x11`** — Opcode `0x10` is `SET_DYLIB_ORDINAL_IMM`, immediate `0x01`. This sets the library ordinal to 1. Library ordinals are 1-indexed and correspond to the `LC_LOAD_DYLIB` commands in order. Ordinal 1 is typically `/usr/lib/libSystem.B.dylib` — the uber-library that re-exports libc, libpthread, and most other system libraries.

**Bytes 0x01–0x09: `0x40 5F 70 72 69 6E 74 66 00`** — Opcode `0x40` is `SET_SYMBOL_TRAILING_FLAGS_IMM`, immediate `0x00` (no special flags). The symbol name follows as a null-terminated C string. `5F 70 72 69 6E 74 66` is `_printf` in ASCII. The leading underscore is the Mach-O convention for C symbols.

**Bytes 0x0A–0x0B: `0x72 0x00`** — Opcode `0x70` is `SET_SEGMENT_AND_OFFSET_ULEB`, immediate `0x02`. Set the segment index to 2 (typically `__DATA_CONST`, where the GOT lives). The next byte is a ULEB128-encoded offset: `0x00` → offset 0. We're targeting the first GOT entry.

**Byte 0x0C: `0x90`** — Opcode `0x90` is `DO_BIND`, immediate `0x00`. Execute the bind: resolve `_printf` from library ordinal 1, write the resolved address to segment 2 at offset 0. Then advance the offset by the pointer size (8 bytes on 64-bit). Now the cursor is at offset 0x008.

**Bytes 0x0D–0x15: `0x40 5F 73 74 72 6C 65 6E 00`** — Set symbol to `_strlen`. The library ordinal is still 1 (it persists until changed).

**Byte 0x16: `0x90`** — DO_BIND. Write `_strlen`'s address to segment 2, offset 0x008. Advance to 0x010.

And so on for `_malloc` and `_free`. Four symbols, 42 bytes of opcodes. The format is remarkably compact. A binary with 200 import symbols might use only 2KB of opcode data. Apple designed this format to minimize binary size — every byte matters on the App Store and every byte matters for code-signing (the entire `__LINKEDIT` segment is included in the code signature hash).

Consider the alternative: a flat table of (segment, offset, library, symbol) tuples. Each entry would need: 1 byte for segment index, 4 bytes for offset, 1 byte for library ordinal, and a null-terminated symbol name averaging 15 bytes — roughly 21 bytes per entry. Our four-symbol example would be 84 bytes. The opcode stream is 42 bytes. For larger binaries, the compression ratio improves further because the VM state carries over between opcodes — the segment and library only need setting once.

The more interesting opcodes are the bulk variants. `DO_BIND_ADD_ADDR_IMM_SCALED` binds and then advances the offset by the immediate value times the pointer size — useful for binding every Nth slot in a table. If you have a block of data with a pointer at offset 0, non-pointer data at offsets 8 and 16, and another pointer at offset 24, the `ADD_ADDR_IMM_SCALED` with immediate 2 will skip 16 bytes (2 × 8) after binding, landing exactly on the next pointer slot.

`DO_BIND_ULEB_TIMES_SKIPPING_ULEB` is the maximally compressed opcode — it binds N consecutive entries separated by a fixed skip. This encoding shines for vtable initialization: a C++ vtable is a dense array of function pointers, and binding 50 vtable entries with one opcode (count=50, skip=0) costs just 3-5 bytes depending on ULEB128 size. Without this opcode, 50 entries would need 50 separate `DO_BIND` opcodes (50 bytes). The compression ratio for vtable-heavy binaries (Objective-C++ frameworks like WebKit) is substantial.

### Our Decoder

The decoder in `crates/macho-parse/src/fixups/decode.rs` implements the VM as a simple loop:

```rust
// File: crates/macho-parse/src/fixups/decode.rs (simplified)

pub fn parse_binds(data: &[u8], bind_off: u32, bind_size: u32, is_lazy: bool) -> Vec<BindEntry> {
    let stream = &data[start..end];
    let mut pos = 0usize;
    let mut entries = Vec::new();

    // VM state
    let mut seg_index: usize = 0;
    let mut seg_offset: u64 = 0;
    let mut dylib_ordinal: i64 = 0;
    let mut symbol_name = String::new();
    let mut addend: i64 = 0;
    let mut is_weak = false;
    let pointer_size: u64 = 8;

    while pos < stream.len() {
        let byte = stream[pos];
        pos += 1;
        let opcode = byte & BIND_OPCODE_MASK;
        let immediate = byte & BIND_IMMEDIATE_MASK;

        match opcode {
            BIND_OPCODE_DONE => {
                if !is_lazy { break; }
                // In lazy streams, DONE marks the end of one binding,
                // not the entire stream.
            }
            BIND_OPCODE_SET_DYLIB_ORDINAL_IMM => {
                dylib_ordinal = immediate as i64;
            }
            BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB => {
                dylib_ordinal = read_uleb128(stream, &mut pos) as i64;
            }
            BIND_OPCODE_SET_DYLIB_SPECIAL_IMM => {
                // Negative ordinals: 0 = self, -1 = main exe, -2 = flat
                dylib_ordinal = if immediate == 0 { 0 }
                    else { (BIND_OPCODE_MASK | immediate) as i8 as i64 };
            }
            BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM => {
                is_weak = (immediate & 0x01) != 0;
                symbol_name = read_cstring(stream, &mut pos);
            }
            BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB => {
                seg_index = immediate as usize;
                seg_offset = read_uleb128(stream, &mut pos);
            }
            BIND_OPCODE_SET_ADDEND_SLEB => {
                addend = read_sleb128(stream, &mut pos);
            }
            BIND_OPCODE_ADD_ADDR_ULEB => {
                seg_offset = seg_offset.wrapping_add(read_uleb128(stream, &mut pos));
            }
            BIND_OPCODE_DO_BIND => {
                entries.push(BindEntry {
                    segment_index: seg_index,
                    segment_offset: seg_offset,
                    symbol_name: symbol_name.clone(),
                    dylib_ordinal,
                    addend,
                    is_weak,
                });
                seg_offset += pointer_size;
            }
            BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB => {
                entries.push(BindEntry { /* same fields */ });
                seg_offset += pointer_size + read_uleb128(stream, &mut pos);
            }
            BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED => {
                entries.push(BindEntry { /* same fields */ });
                seg_offset += pointer_size + (immediate as u64 * pointer_size);
            }
            BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB => {
                let count = read_uleb128(stream, &mut pos);
                let skip = read_uleb128(stream, &mut pos);
                for _ in 0..count {
                    entries.push(BindEntry { /* same fields */ });
                    seg_offset += pointer_size + skip;
                }
            }
            _ => {}
        }
    }
    entries
}
```

Each `BindEntry` tells us: write the address of `symbol_name` to `segment_index` at byte offset `segment_offset`. That's everything the fixup applicator needs.

### Lazy Binding — and Why We Skip It

The lazy bind stream contains symbols resolved on first use. On a normal macOS system, this is the default for most function calls. The mechanism is elegant: each lazy GOT entry (in `__DATA,__la_symbol_ptr`) initially points to a small stub in `__TEXT`:

```asm
; __TEXT,__stubs entry for _printf:
  adrp    x16, __la_symbol_ptr@PAGE
  ldr     x16, [x16, __la_symbol_ptr@PAGEOFF]
  br      x16                          ; → initially targets stub_helper

; __TEXT,__stub_helper entry for _printf:
  ldr     w16, =lazy_bind_offset       ; offset into lazy bind info
  b       __stub_helper_common         ; → calls dyld_stub_binder
```

The first call lands on the stub, which loads a byte offset into the lazy bind info stream and branches to `__stub_helper_common`. That stub calls `dyld_stub_binder` — a function exported by `dyld` itself (via `libdyld.dylib`). The binder reads the lazy bind opcode at that offset, resolves the symbol, patches the `__la_symbol_ptr` slot with the real address, and branches directly to the resolved function. The caller never knows anything happened. Subsequent calls go through `__la_symbol_ptr`, which now contains the real function address — no stub overhead.

Lazy binding amortizes startup cost. A binary with 200 external function calls might only use 30 in a typical run. Resolving all 200 at launch wastes work. Lazy binding resolves each function at first use — converting O(N) startup cost to O(1) per function, paid on demand.

But `dyld_stub_binder` is part of `dyld`. We don't have `dyld`. And `dyld_stub_binder` is more complex than it appears — it must:

1. Save all argument registers (`x0-x7`, `q0-q7` for floating-point/SIMD) to the stack, because the stub runs *inside* the caller's function call sequence and must not clobber arguments destined for the target function.
2. Read the lazy bind offset from `w16` (loaded by `ldr w16, =offset`).
3. Walk the lazy bind opcode stream starting at that offset from to extract the symbol name and ordinal.
4. Resolve the symbol via `dyld`'s internal resolver.
5. Write the resolved address into the `__la_symbol_ptr` slot.
6. Restore all saved registers.
7. Branch to the resolved address with registers exactly as the caller set them.

Thread safety adds another layer. Two threads can call the same lazy-bound function simultaneously. `dyld_stub_binder` handles this with lock-free atomic stores to the GOT slot, but correctness requires careful ordering — the store must be visible to all cores before returning.

Implementing all of this — register save/restore trampoline, lazy opcode stream navigation, thread-safe GOT patching — is significant complexity for an optimization that saves microseconds at startup.

Our strategy: bind everything eagerly. Parse both bind and lazy_bind streams as if they were the same stream. Resolve all symbols upfront. Write every address into the GOT and the lazy symbol pointer table before execution begins. The startup cost is real but tiny — resolving 200 symbols takes less than a millisecond. We trade lazy-load elegance for implementation simplicity.

```rust
// File: crates/macho-exec/src/fixup/mod.rs

pub fn apply_binds(
    image: &MappedImage,
    fixups: &Fixups,
    bindings: &BindingReport,
) -> Result<usize, ExecError> {
    let mut count = 0;
    for bind in fixups.binds.iter()
        .chain(fixups.lazy_binds.iter())    // Lazy binds resolved eagerly
        .chain(fixups.weak_binds.iter())
    {
        let seg = &image.segments[bind.segment_index];
        if let Some(resolved_addr) = resolve_bind_address(&bind.symbol_name, bindings) {
            let final_addr = (resolved_addr as i64 + bind.addend) as u64;
            unsafe {
                let ptr = seg.base.add(bind.segment_offset as usize) as *mut u64;
                ptr.write_unaligned(final_addr);
            }
            count += 1;
        }
    }
    Ok(count)
}
```

The `addend` field handles offsets: when a bind target is `_some_struct + 16`, the addend is 16. The resolved address of `_some_struct` gets the offset added before writing. This is common for C struct field references — the binary imports the struct's base address and adjusts by the field offset.

The weak bind stream needs special treatment. A weak bind with `is_weak = true` means "resolve this if you can, but don't fail if it's missing." Missing weak symbols get zero — a null function pointer that the binary should check before calling. ObjC runtime uses weak imports extensively for API availability: `_objc_opt_class` exists only on newer runtime versions, and older code weak-imports it to fall back gracefully.

The classic bind format works. It's reliable, well-understood, and easy to debug — you can `xxd` the opcode stream and follow along byte by byte. But Apple has moved on. The binaries we care most about — modern Xcode tools built with the latest linker — use a different system entirely. One that's more compact, faster to process, and embeds its fixup metadata directly into the data pages themselves.

---

## Scene 3.3: Chained Fixups

Open any Mach-O binary built with Xcode 13 or later and search for `LC_DYLD_INFO_ONLY`. You won't find it. Instead:

```
Load command 7
     cmd  LC_DYLD_CHAINED_FIXUPS
 cmdsize  16
data_off   0x34000
data_size  1256
```

This is the modern format, introduced in macOS 12 and iOS 15, and it works on a fundamentally different principle. Instead of a separate opcode stream in `__LINKEDIT`, the fixup information is encoded *inside the data pages themselves*. Those 64-bit values sitting in the GOT — the ones that were all zeros from our mapping — aren't zeros in the raw file. They're packed metadata words, each describing what that slot needs and where to find the next slot.

The motivation is performance. Bind opcodes require sequential processing: read an opcode, decode it, update VM state, repeat. The processing is inherently serial — each opcode modifies state that the next opcode reads. Chained fixups trade complexity of the metadata format for parallelism of processing. Each page's chain can be walked independently. `dyld` on modern hardware processes multiple pages concurrently using Grand Central Dispatch. Apple measured the difference at 30-40% faster fixup application on M1 at launch time for large applications.

There's a subtler advantage. With bind opcodes, the linker must produce the opcode stream as a separate artifact and store it in `__LINKEDIT`. With chained fixups, the metadata is embedded in the data pages that will hold the final resolved values. After fixups are applied, those embedded metadata words are *overwritten* with real addresses. The metadata doesn't permanently occupy space — it reuses the space that the final pointer will live in. This reduces `__LINKEDIT` size, which matters for code-signing performance (smaller signed content = faster verification).

### The Header

The `LC_DYLD_CHAINED_FIXUPS` payload starts with a `dyld_chained_fixups_header`:

```rust
// File: crates/macho-exec/src/fixup/chained/mod.rs

let buf = &binary_data[base..end];

// dyld_chained_fixups_header (first 24 bytes)
let fixups_version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
let starts_offset  = u32::from_le_bytes([buf[4],  buf[5],  buf[6],  buf[7]]) as usize;
let imports_offset = u32::from_le_bytes([buf[8],  buf[9],  buf[10], buf[11]]) as usize;
let symbols_offset = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]) as usize;
let imports_count  = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
let imports_format = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
```

The header references three data regions:

The **starts table** is a per-segment, per-page index. For each data segment (`__DATA`, `__DATA_CONST`, `__AUTH_CONST`) and each 4KB page within that segment, it records the byte offset where the first fixup chain entry lives. A sentinel value of `0xFFFF` means "no chain on this page" — the page contains no fixup entries. The starts table also records the pointer format for each segment — this is critical because different segments in the same binary can use different chain formats.

The starts table is organized as a `dyld_chained_starts_in_image` structure containing an array of segment offsets, each pointing to a `dyld_chained_starts_in_segment` structure. The per-segment structure contains:

```
segment_offset: u64    — offset of this segment in the binary
max_valid_pointer: u32 — max valid pointer value (for validation)
page_size: u16         — 0x1000 (4KB) or 0x4000 (16KB)
pointer_format: u16    — which chain encoding (1, 2, 6, 12, or 13)
page_count: u16        — number of pages described
page_start: [u16]      — per-page start offset of the first chain entry
```

The `pointer_format` field is the single most important value in the entire fixup system. It determines how every 64-bit word on this segment's pages is decoded. Get it wrong and you'll extract the wrong bits for next-offset, ordinal, key, diversity — everything downstream will be garbage.

The **imports table** is an array of compact entries, each naming an imported symbol:

```rust
// File: crates/macho-exec/src/fixup/chained/imports.rs

let raw = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]);
let lib_ordinal = (raw & 0xFF) as u8;
let weak = ((raw >> 8) & 1) != 0;
let name_offset = (raw >> 9) as usize;
let name = read_cstring_at(buf, symbols_offset + name_offset);
```

Four bytes per import. An imports table with 50 entries uses only 200 bytes. Compare this to the bind opcode format where each symbol name appears inline in the opcode stream — in chained fixups, names are pooled and referenced by offset.

The **symbols pool** is a blob of null-terminated strings — the actual symbol names referenced by the imports table. Because names are referenced by byte offset rather than index, duplicated suffixes can be shared. `_CFStringCreateWithCString` and `_CFStringCreateWithFormat` could theoretically share their `CreateWith` prefix region, though in practice the linker doesn't perform this optimization.

### The Chain Concept

Here's the key insight: in the raw binary file, each location that needs a fixup contains a 64-bit metadata word. That word encodes (a) what to do with this slot and (b) where to find the next slot that also needs fixing. The entries form a singly-linked list — a *chain* — that threads through each page.

```
Page layout (conceptual):

  +0x000: [fixup metadata] → next=+0x020
  +0x008: [non-fixup data]    (skip)
  +0x010: [non-fixup data]    (skip)
  +0x018: [non-fixup data]    (skip)
  +0x020: [fixup metadata] → next=+0x038
  +0x028: [non-fixup data]    (skip)
  +0x030: [non-fixup data]    (skip)
  +0x038: [fixup metadata] → next=0 (end)
```

The starts table tells us where each chain begins. We read the first entry, process it (rebase or bind), follow the `next` pointer to the second entry, process that, and so on until `next` is zero.

The `next` field isn't a byte offset — it's measured in *stride* units. The stride depends on the pointer format: ARM64e formats use a stride of 8 (the natural pointer alignment), while PTR_64 formats use a stride of 4. This means the `next` offset is `next_value * stride` bytes from the current position. An 11-bit `next` field with stride 4 can reach 8,188 bytes ahead — more than enough to span any page's entries. With stride 8, it reaches 16,376 bytes.

Why not just use byte offsets? Because the stride encoding squeezes more range out of fewer bits. Every fixup location is naturally aligned — pointers live at pointer-aligned addresses. Encoding the offset in pointer-sized (or half-pointer-sized) units loses no information while doubling the addressable range.

### Rebase vs. Bind in One Word

Each 64-bit fixup word can be either a *rebase* (internal pointer needing ASLR slide) or a *bind* (external symbol needing resolution). Bit 62 discriminates:

```
Bit 63: auth flag (arm64e only — Scene 3.4)
Bit 62: bind flag (1 = bind, 0 = rebase)
Bits 51-61: next (offset to next entry, in stride units)
Bits 0-50: payload
```

For **binds**: the low bits contain an ordinal — an index into the imports table. Look up the symbol name, resolve it, write the address.

For **rebases**: the low bits contain a target virtual address (split across non-contiguous bit fields). Add the ASLR slide, write the relocated address.

Let's walk a real chain. Here's a `__DATA_CONST` page from a format-6 (`PTR_64_OFFSET`) binary:

```
Page start offset from starts table: 0x000

+0x000: 0x8000000300000003
        bit 63 = 1 → bind
        ordinal = raw & 0x00FFFFFF = 3 → imports[3] = "_strlen"
        next = (raw >> 51) & 0xFFF = 4 → next at +0x000 + 4*4 = +0x010
        addend = (raw >> 24) & 0xFF = 0

+0x010: 0x8000000400000007
        bind, ordinal 7 → "_printf", next=4 → +0x020

+0x020: 0x0000000000000001
        bit 63 = 0 → rebase
        target = raw & 0x0F_FFFF_FFFF = 1
        high8 = (raw >> 36) & 0xFF = 0
        rebased = preferred_base + target + slide
        next = (raw >> 51) & 0xFFF = 0 → end of chain
```

After walking, the page contains real addresses:

```
+0x000: 0x00000001a4d35580  ; _strlen
+0x010: 0x00000001a4c2e000  ; _printf
+0x020: 0x00000001000040a0  ; rebased internal pointer
```

The rebase encoding deserves attention. For format 12 (`DYLD_CHAINED_PTR_ARM64E_USERLAND`), the 64-bit target address is split across non-contiguous bit fields:

```rust
let target43 = raw & 0x7FF_FFFF_FFFF;  // bits 0-42: lower 43 bits
let high8    = (raw >> 43) & 0xFF;      // bits 43-50: upper 8 bits
let raw_target = base_for_rebase + ((high8 << 56) | target43);
let rebased = (raw_target as i64 + slide) as u64;
```

Bits 0-42 hold the lower 43 bits of the address. Bits 43-50 hold what will become bits 56-63 of the final address. Bits 43-55 of the address are zero-extended (the 48-bit VA space means these bits are determined by sign extension). It's compact and correct — but the first time you see a split 64-bit address encoding, it's surprising. We expected contiguous storage and wrote a bug that took an hour to find.

The split exists because the `high8` bits encode what Apple calls "pointer tagging" — the upper byte of a 64-bit pointer, which ARM64's Top Byte Ignore (TBI) feature makes available for metadata. ObjC uses these bits for tagged pointers (small objects encoded directly in the pointer value) and for non-pointer isa optimization (encoding class information in the isa pointer's upper bits). By storing the high8 separately, the chained fixup format preserves pointer tags through the rebase process — the slide is applied to the lower 56 bits while the tag in the upper 8 bits passes through unchanged.

For format 2 (`PTR_64`) and format 6 (`PTR_64_OFFSET`), the difference is in the base address. Format 2 encodes absolute target addresses — the raw value in the low bits is the final VM address before slide. Format 6 encodes preferred-base-relative offsets — the raw value is an offset from the binary's preferred load address (`__TEXT` segment's VM address from the Mach-O header). You must add `preferred_base` before applying the slide:

```rust
// Format 2 (PTR_64): absolute — target is the raw VM address
let rebased = ((high8 << 56) | target) as i64 + slide;

// Format 6 (PTR_64_OFFSET): base-relative — target is offset from preferred base
let rebased = (preferred_base + ((high8 << 56) | target)) as i64 + slide;
```

Using the wrong interpretation produces addresses that are either impossibly small (format 2 base math on a format 6 raw offset) or impossibly large (format 6 base addition on a format 2 absolute address). Both crash, but the crash looks different — a clue for debugging.

The split exists because the `high8` bits encode what Apple calls "pointer tagging" — the upper byte of a 64-bit pointer, which ARM64's Top Byte Ignore (TBI) feature makes available for metadata. ObjC uses these bits for tagged pointers (small objects encoded directly in the pointer value) and for non-pointer isa optimization (encoding class information in the isa pointer's upper bits). By storing the high8 separately, the chained fixup format preserves pointer tags through the rebase process — the slide is applied to the lower 56 bits while the tag in the upper 8 bits passes through unchanged.

For format 2 (`PTR_64`) and format 6 (`PTR_64_OFFSET`), the difference is in the base. Format 2 uses absolute target addresses — the raw target value in the low bits is the final VM address before slide. Format 6 uses preferred-base-relative targets — the raw value is an offset from the binary's preferred load address (`__TEXT` segment's VM address from the Mach-O header). We add `preferred_base` before sliding:

```rust
// Format 2 (PTR_64): absolute
let rebased = ((high8 << 56) | target) as i64 + slide;

// Format 6 (PTR_64_OFFSET): base-relative
let rebased = (preferred_base + (high8 << 56) | target) as i64 + slide;
```

The difference matters when parsing: format 2 targets are already valid addresses and typically in the `0x100000000` range. Format 6 targets are small offsets (often under `0x100000`) that need the base added. Using the wrong interpretation produces addresses that are either impossibly small (format 2 interpreted as format 6) or impossibly large (format 6 interpreted as format 2). Both crash, but the crash looks different — one produces a SIGSEGV at a low address, the other at a very high address.

### The Moment the Documentation Lies

And here's where we hit the wall that defines this chapter.

Apple's `<mach-o/loader.h>` header describes the `DYLD_CHAINED_PTR_ARM64E` format (pointer format 1). For authenticated entries (bit 63 = 1), the header comments say the bit layout is:

```c
// From Apple's <mach-o/loader.h> (paraphrased):
//   Bits 0-15:  ordinal (for bind) or target (for rebase)
//   Bits 16-31: (unused/zero)
//   Bits 32-42: next
//   Bits 43-44: key
//   Bit 45:     addrDiv
//   Bits 46-61: diversity
//   Bit 62:     bind
//   Bit 63:     auth
```

We implemented this. We extracted the PAC key from bits 43-44 and the diversity from bits 46-61. Our code compiled. Our walker ran without panics. The values we extracted were in the valid ranges — keys are 2-bit values (0-3), diversity is 16-bit (0-65535). Everything looked correct.

The first symptom was PAC auth faults. Not on every entry — only on some. A function call through one GOT slot worked fine. The next slot crashed with `PC 0x004000...`, the characteristic PAC failure address. We checked our PAC signing assembly. We checked our discriminator blending. We added logging that printed every key and diversity value we extracted. The signing code looked correct.

The debugging session that followed was among the most frustrating in the entire project. PAC failures are inherently opaque: the CPU tells you *that* authentication failed but not *why*. Was the key wrong? Was the diversity wrong? Was the address wrong? Was the pointer unsigned when it should have been signed? The CPU doesn't say. It corrupts the pointer and SIGSEGVs.

Our first hypothesis was a signing bug — perhaps our inline assembly didn't handle all four key variants correctly. We wrote a standalone test that signed and verified with each key. All passed. The signing assembly was correct.

Our second hypothesis was an address diversity issue — perhaps we were computing the slot address incorrectly for the blend. We hex-dumped the GOT entries, computed expected addresses, and compared. The addresses matched.

Then we compared the key and diversity values we extracted against what Apple's open-source dyld code extracts from the same binary. We pulled up `MachOLoaded.cpp` from the dyld4 source code, found the `ChainedFixupPointerOnDisk` union type, and traced its bit extraction for `AuthRebase64`. The values were different.

We hex-dumped an authentic fixup entry — a raw 64-bit word from a real arm64e binary:

```
0x 80 1A 00 00 00 8D 03 80
```

Reversed for little-endian: `0x80038D000000001A80`

In binary, the upper 32 bits are:
```
1000 0000 0000 0011 1000 1101 0000 0000
│ │         ││  │└── bits 32-47: diversity = 0x008D
│ │         ││  └─── bit 48: addrDiv = 0
│ │         │└────── bits 49-50: key = 0b11 = DA
│ │         └─────── bits 51-61: next = 0x001 = 1
│ └───────────────── bit 62: bind = 0 (rebase)
└─────────────────── bit 63: auth = 1
```

Using Apple's header layout (key at bits 43-44), we'd extract key = 0b01 (IB). Using dyld's actual code (key at bits 49-50), we extract key = 0b11 (DA). Both are valid PacKey values. Both produce well-formed PAC signatures. Only one is correct.

The real layout, as implemented in dyld's source:

```
Bits 32-47: diversity (16 bits)
Bit 48:     addrDiv (address diversity)
Bits 49-50: key (2 bits)
Bits 51-61: next (11 bits)
Bit 62:     bind
Bit 63:     auth
```

The key is at bits 49-50, not 43-44. The diversity is at bits 32-47, not 46-61. The `next` field is at bits 51-61 — the *same position* as in non-authenticated entries. That last point is the tell that the header's layout is wrong: the chain-walking code needs to find `next` regardless of whether an entry is authenticated. Putting `next` at different positions for auth vs. non-auth entries would require branching on bit 63 before you could even find the next entry. dyld's implementation extracts `next` identically for both — because it's always at bits 51-61.

Once you see it, the correct layout is obviously right. Auth and non-auth entries share the same `next` field at the same position. The binding/rebase payload occupies the low 32 bits. The authentication metadata (diversity, address-diversity, key) fills the gap between the payload and the shared `next` field. It's clean. It's consistent. It's just not what the header documents.

Our corrected implementation:

```rust
// File: crates/macho-exec/src/ptrauth.rs

impl AuthMetadata {
    pub fn from_auth_entry(raw: u64) -> Self {
        let diversity = ((raw >> 32) & 0xFFFF) as u16;  // bits 32-47
        let addr_diversity = (raw >> 48) & 1 == 1;       // bit 48
        let key = PacKey::from_bits((raw >> 49) & 0x3);   // bits 49-50
        AuthMetadata { key, addr_diversity, diversity }
    }
}
```

The binary tells the truth. The header lies.

This isn't malice. Apple's headers are generally excellent — `<mach-o/loader.h>` is one of the best-documented system headers in the Unix world. This discrepancy exists because the format evolved — the bit layout changed between dyld versions, the comments weren't updated to match, and the only canonical source is the dyld source code and the binaries themselves. If you're building a Mach-O tool and you trust the header comments more than the actual bytes in the file, you'll get wrong results in ways that are almost impossible to diagnose. The values are in valid ranges. The code doesn't crash at extraction time. It just produces wrong PAC signatures that fail silently on hardware — or worse, work on most entries and fail on a few, depending on which actual key bits happen to overlap with the documented key position.

How many other Mach-O tools implement the wrong bit layout? We checked three open-source implementations at the time. Two used the header's layout. Both had open issues for PAC-related crashes on arm64e binaries.

We catalog every case where documentation contradicts binary reality in Appendix B. This was the first. This was the moment that established the project's epistemology: *read the binary*. Not the header. Not the documentation. Not the WWDC slide deck. The binary.

### Our Chain Walker

The complete chain walker handles five pointer formats across three architecture generations. Each format uses a different bit layout for the same conceptual fields (bind/rebase, ordinal/target, next, auth metadata), and the stride — the multiplier for the `next` field — differs between formats.

Here's the format landscape:

| ID | Name | Stride | Ordinal Bits | Next Bits | Auth? | When Used |
|----|------|--------|------|------|------|-----------|
| 1 | `PTR_ARM64E` | 8 | 16 | 11 | Yes | macOS 12 kernel cache, pre-userland |
| 2 | `PTR_64` | 4 | 24 | 12 | No | Universal x86_64 / arm64 non-e |
| 6 | `PTR_64_OFFSET` | 4 | 24 | 12 | No | arm64 with preferred-base-relative targets |
| 12 | `PTR_ARM64E_USERLAND` | 8 | 16 | 11 | Yes | arm64e apps and frameworks (most common) |
| 13 | `PTR_ARM64E_USERLAND24` | 8 | 24 | 11 | Yes | arm64e with >65K imports (Swift-heavy) |

Format 12 is what you'll encounter in essentially every modern arm64e binary — it's the default output of Xcode's linker since Xcode 14. Format 6 appears in arm64 (non-e) slices and in Universal binaries targeting both Intel and Apple Silicon. Format 2 is the oldest chained fixup format and appears in Intel-only arm64 legacy binaries. Format 1 is interesting historically — it was the first ARM64e chained format and differs from format 12 in how rebase targets are encoded (absolute addresses vs. preferred-base-relative). Format 13 was added when Swift generics and large Xcode frameworks started generating enough symbol references to overflow 16-bit ordinals.

```rust
// File: crates/macho-exec/src/fixup/chained/walk.rs

pub(super) fn walk_chain(
    seg: &MappedSegment,
    start_offset: usize,
    pointer_format: u16,
    slide: i64,
    preferred_base: u64,
    imports: &[ChainedImport],
    resolver: &dyn SymbolResolver,
) -> (usize, usize) {
    let mut offset = start_offset;
    let stride = chain_stride(pointer_format);
    let mut rebases = 0;
    let mut binds = 0;

    loop {
        if offset + 8 > seg.size { break; }
        let raw = unsafe {
            let ptr = seg.base.add(offset) as *const u64;
            ptr.read_unaligned()
        };

        // Extract next-offset (always at bits 51-61 for auth formats,
        // varies for non-auth)
        let next = extract_next(raw, pointer_format);

        match pointer_format {
            1 | 12 | 13 => {
                // ARM64e: check auth bit (63), then bind bit (62)
                let is_auth = (raw >> 63) & 1 == 1;
                let is_bind = (raw >> 62) & 1 == 1;

                if is_auth {
                    let meta = AuthMetadata::from_auth_entry(raw);
                    if is_bind {
                        let ordinal = extract_ordinal(raw, pointer_format);
                        let import = &imports[ordinal];
                        // Resolve and PAC-sign the result
                        handle_auth_bind(seg, offset, import, &meta, resolver);
                        binds += 1;
                    } else {
                        // Auth rebase: target in low bits, PAC-sign with metadata
                        handle_auth_rebase(seg, offset, raw, &meta, slide, preferred_base);
                        rebases += 1;
                    }
                } else {
                    if is_bind {
                        let ordinal = extract_ordinal(raw, pointer_format);
                        handle_plain_bind(seg, offset, &imports[ordinal], resolver);
                        binds += 1;
                    } else {
                        handle_plain_rebase(seg, offset, raw, slide, preferred_base, pointer_format);
                        rebases += 1;
                    }
                }
            }
            6 => {
                // PTR_64_OFFSET: preferred-base-relative targets
                let is_bind = (raw >> 62) & 1 == 1;
                if is_bind {
                    let ordinal = (raw & 0x00FFFFFF) as usize;
                    handle_plain_bind(seg, offset, &imports[ordinal], resolver);
                    binds += 1;
                } else {
                    let target = (raw & 0x0F_FFFF_FFFF) as u64;
                    let high8 = ((raw >> 36) & 0xFF) as u64;
                    let value = preferred_base + ((high8 << 56) | target);
                    let rebased = (value as i64 + slide) as u64;
                    write_u64(seg, offset, rebased);
                    rebases += 1;
                }
            }
            _ => {
                // PTR_64: absolute target addresses
                let is_bind = (raw >> 62) & 1 == 1;
                if is_bind {
                    let ordinal = (raw & 0x00FFFFFF) as usize;
                    handle_plain_bind(seg, offset, &imports[ordinal], resolver);
                    binds += 1;
                } else {
                    let target = raw & 0x0F_FFFF_FFFF;
                    let high8 = (raw >> 36) & 0xFF;
                    let rebased = ((high8 << 56) | target) as i64 + slide;
                    write_u64(seg, offset, rebased as u64);
                    rebases += 1;
                }
            }
        }

        if next == 0 { break; }
        offset += next * stride;
    }
    (rebases, binds)
}

pub(super) fn chain_stride(pointer_format: u16) -> usize {
    match pointer_format {
        1  => 8,  // DYLD_CHAINED_PTR_ARM64E
        2  => 4,  // DYLD_CHAINED_PTR_64
        6  => 4,  // DYLD_CHAINED_PTR_64_OFFSET
        12 => 8,  // DYLD_CHAINED_PTR_ARM64E_USERLAND
        13 => 8,  // DYLD_CHAINED_PTR_ARM64E_USERLAND24
        _  => 4,
    }
}
```

The four-way branch inside the loop — auth bind, auth rebase, plain bind, plain rebase — is the heart of the fixup engine. Each path extracts different bit fields and performs different operations, but they all share the same chain-walking structure: read the 64-bit word, process it, extract `next`, advance.

A real-world binary might have 50 data pages with chains averaging 20 entries each — 1,000 fixup operations total. The walker processes all of them in a single pass per segment, typically completing in under 100 microseconds. The bulk of the time is spent in symbol resolution, not chain walking.

Chained fixups are what we encounter in every modern arm64e binary. They're the format that matters. And those auth bits at the top — bit 63, the key, the diversity — they're not optional decoration. They're Pointer Authentication Codes.

---

## Scene 3.4: Auth Fixups and PAC

ARM64e is Apple Silicon's extended instruction set. The "e" stands for "enhanced," and the critical enhancement for binary loading is PAC — Pointer Authentication Codes. Every function pointer in an arm64e binary can carry a cryptographic signature in its upper bits. When the CPU branches to a signed pointer, it verifies the signature first. If verification fails, the CPU corrupts the pointer's upper bits to `0x0040` and faults. The resulting crash lands at a distinctive unmapped address.

PAC exists because of a fundamental tension in systems programming. C and C++ give you raw pointer arithmetic — the flexibility that makes operating systems possible. But that flexibility is also the attack surface for every return-oriented programming (ROP) and jump-oriented programming (JOP) exploit ever written. An attacker who can overwrite a function pointer controls the next branch target. Overwrite a GOT entry and you redirect every call through that slot. Overwrite a return address on the stack and you hijack the function's return path.

Apple's answer is hardware-enforced: make every pointer carry a cryptographic signature that the CPU verifies before use. An attacker who overwrites a signed pointer produces an invalid signature. The CPU catches it, corrupts the pointer, and crashes rather than jumping to attacker-controlled code. The signature computation uses a secret key embedded in the CPU that software cannot read.

We just learned how to fill in the GOT with resolved addresses. On arm64e, raw unsigned addresses aren't enough:

```
SIGSEGV at PC 0x004000ffffb3e808
  x16 = 0x004000ffffb3e808    ← PAC-corrupted pointer
  x17 = 0x000060a10001c730    ← discriminator used for auth check
```

The `0x0040` prefix in bits 45-62 is the unmistakable signal: PAC verification failed. The original target was somewhere in libc (`0xffffb3e808` after stripping the corrupted bits). The authenticated branch `BRAA x16, x17` checked x16's signature against key IA with modifier x17, found no valid signature, and corrupted the pointer.

Reading a PAC crash from a register dump is a learnable skill. The key elements:

1. **PC contains `0x0040...`** — The `0x0040` pattern in the upper bits is the CPU's corruption pattern. Real addresses never have these bits set (they'd be in kernel space). This pattern says "PAC auth failed."
2. **x16 holds the corrupted pointer** — On ARM64, `BRAA x16, x17` is the standard authenticated indirect call. `x16` contains the target pointer, `x17` contains the discriminator.
3. **Strip the corruption to find the intended target** — Zero out bits 47-62. The remaining bits are the original function address: `0x00000001ffffb3e808`. We can look this up in the shared cache exports to identify the function.
4. **x17 tells you the modifier** — The discriminator used for auth. Compare it against what you computed to see if your blend is wrong.

### The Four Keys

PAC uses four 128-bit hardware keys stored in system registers inaccessible to user code:

```rust
pub enum PacKey {
    IA = 0,  // Instruction Address A — function pointers, vtables
    IB = 1,  // Instruction Address B — return addresses (PACIBSP/RETAB)
    DA = 2,  // Data Address A — ObjC class pointers, data references
    DB = 3,  // Data Address B — method lists, protocol references
}
```

The split between A and B keys provides domain separation. Return addresses (IB) use a different key than function pointers (IA), so an attacker who somehow forges an IA signature cannot reuse it on a return address. The split between instruction (I) and data (D) keys prevents confusing code pointers with data pointers.

Most GOT entries use key IA (code pointers). ObjC class references and Swift metadata pointers use key DA. Return addresses (protected by `PACIBSP` on function entry and `RETAB` on return) use key IB — managed entirely by the CPU's function call/return mechanism, not by our linker. Method lists in the ObjC runtime use key DB.

### Discriminators

Signing with just a key would be too coarse — all function pointers signed with key IA would be interchangeable. A signed pointer to `printf` could be substituted for a signed pointer to `system`. PAC adds a 64-bit *discriminator* (Apple calls it "modifier") that makes each signature context-specific:

```rust
pub fn blend_discriminator(addr: u64, diversity: u16) -> u64 {
    ((addr & 0xFFFF) << 48) | (diversity as u64)
}
```

Two independent sources combine to produce the discriminator:

**The diversity value** is a 16-bit constant baked into the fixup entry (bits 32-47 of the auth metadata). The linker assigns this value — different call sites can have different diversities even for the same target function. A GOT entry for `printf` called from function A might have diversity `0x4578`, while the same `printf` GOT entry used from function B might have `0x12AB`. This ensures that stealing a signed pointer from one call site and grafting it into another produces an invalid signature.

**Address diversity** (bit 48) optionally mixes in the GOT slot's own address. When enabled, the discriminator incorporates the low 16 bits of the slot's virtual address via the `blend_discriminator` function. This binds the signed pointer to its physical location in memory. Copy a signed pointer to a different memory location and the signature becomes invalid — the verification at the new location will compute a different discriminator (because the new location has a different address) and reject the copied pointer.

### Signing in Our Loader

We run in-process on the same ARM64 hardware. We can use the PAC instructions directly via Rust's inline assembly:

```rust
pub unsafe fn pac_sign_pointer(
    value: u64, key: PacKey, slot_addr: u64,
    diversity: u16, addr_div: bool,
) -> u64 {
    if value == 0 { return 0; }
    let modifier = if addr_div {
        if diversity == 0 { slot_addr }
        else { blend_discriminator(slot_addr, diversity) }
    } else {
        diversity as u64
    };
    pac_sign_raw(value, key, modifier)
}

#[cfg(target_arch = "aarch64")]
unsafe fn pac_sign_asm(value: u64, key: PacKey, modifier: u64) -> u64 {
    let result: u64;
    match key {
        PacKey::IA => core::arch::asm!(
            ".arch_extension pauth",
            "pacia {ptr}, {mod}",
            ptr = inout(reg) value => result,
            mod = in(reg) modifier,
        ),
        PacKey::IB => core::arch::asm!(
            ".arch_extension pauth",
            "pacib {ptr}, {mod}",
            ptr = inout(reg) value => result,
            mod = in(reg) modifier,
        ),
        PacKey::DA => core::arch::asm!(
            ".arch_extension pauth",
            "pacda {ptr}, {mod}",
            ptr = inout(reg) value => result,
            mod = in(reg) modifier,
        ),
        PacKey::DB => core::arch::asm!(
            ".arch_extension pauth",
            "pacdb {ptr}, {mod}",
            ptr = inout(reg) value => result,
            mod = in(reg) modifier,
        ),
    }
    result
}
```

The `.arch_extension pauth` directive enables PAC instructions in the assembler — needed on Linux aarch64 where the default target doesn't assume PAC. Without it, `pacia` is an "unknown instruction" error. Each PAC instruction takes a pointer value and a modifier, computes a cryptographic hash using the hardware key, and writes the hash into the pointer's upper bits (bits 47-62 on Apple's implementation, where the virtual address space is 47 bits).

For unresolved symbols — functions the binary imports but our resolver can't find — we PAC-sign a trap address instead:

```rust
None => {
    let slot_addr = seg.base as u64 + offset as u64;
    unsafe {
        pac_sign_pointer(trap_addr(), meta.key, slot_addr,
                         meta.diversity, meta.addr_diversity)
    }
}
```

This is subtle but important. If we left unresolved auth-GOT entries as zero, the binary's authenticated branch would try to verify PAC on address zero, fail, corrupt the zero to `0x0040...`, and SIGSEGV at an address that reveals nothing useful about which symbol was missing. By signing our trap address with the correct key and discriminator, the authenticated branch *succeeds*, lands on our diagnostic trap function, and we print a clear message: "unresolved symbol: _NSCreateObjectFileImageFromMemory". Much better.

### Cross-Platform Detection

On Linux aarch64 (Docker on Apple Silicon), PAC availability depends on the kernel and the CPU:

```rust
pub fn has_hardware_pac() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        { return true; }  // All Apple Silicon has PAC
        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            const HWCAP_PACA: libc::c_ulong = 1 << 30;
            return unsafe { libc::getauxval(libc::AT_HWCAP) } & HWCAP_PACA != 0;
        }
        false  // x86_64, other architectures
    })
}
```

Docker Desktop on Apple Silicon with the default `linux/arm64` platform passes through PAC support — the Linux kernel in the VM recognizes the Apple Silicon PAC capability via `AT_HWCAP`. QEMU user-mode emulation does not emulate PAC — it ignores the signing instructions, treating them as no-ops, which means auth verification also no-ops, which means arm64e binaries *appear* to work but with no actual pointer protection. The Rosetta 2 Linux translation layer does support PAC.

Without hardware PAC, we leave pointers unsigned. The binary's `BRAA` instructions will fault on real arm64e hardware, but if we're running on non-PAC hardware (ARM Cortex-A76 in a Raspberry Pi, for instance), those instructions don't exist and the binary can't run regardless.

---

## Scene 3.5: The Stub Resolver

We can decode fixup entries and sign pointers. Now we need to actually resolve symbol names to addresses. Given the string `"_printf"`, produce the number `0x00000001a4c2e000` — the address where `printf`'s machine code lives in this process.

The simplest approach — `dlsym(RTLD_DEFAULT, name)` for everything — would make the implementation trivial. It would also be a security hole wide enough to drive a truck through. `fork()`, `execve()`, `system()`, `popen()`, `ptrace()`, `dlopen()` would all resolve happily. We're building a loader that runs arbitrary macOS binaries. Giving those binaries unrestricted access to every function on the host is not acceptable.

### The Four-Tier Cascade

Our `CompositeResolver` layers four resolution sources in priority order:

```rust
// File: crates/macho-runtime/src/resolver.rs

impl SymbolResolver for CompositeResolver<'_> {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        // Tier 1: Shim table — our reimplementations
        if let Some(addr) = self.shim_table.resolve(symbol_name) {
            return Some(addr);
        }
        // Tier 2: Data symbols — constants and global variables
        if let Some(addr) = self.data_symbols.resolve(symbol_name) {
            return Some(addr);
        }
        // Tier 3: Eager cache — pre-built HashMap from shared cache
        if let Some(cache) = self.eager_cache {
            if let Some(addr) = cache.resolve(symbol_name) {
                return Some(addr);
            }
        }
        // Tier 4: StubResolver — safe-list gated dlsym
        if let Some(stub) = self.stub_resolver {
            return stub.resolve(symbol_name);
        }
        None
    }
}
```

Priority order matters. When a binary imports `_stat`, the shim table intercepts it (tier 1) and provides our reimplementation that translates between macOS and Linux `stat` struct layouts. If we let tier 4 handle it, `dlsym` would return the Linux `stat` — which takes `struct stat*` with different field sizes, offsets, and padding — and the binary would corrupt memory when it tried to read the results.

### Tier 1: Shims

**Shims** intercept functions we need to reimplement. These fall into several categories:

**ABI mismatches.** `stat()` on macOS uses a 144-byte `struct stat` (with `st_flags`, `st_gen`, and 64-bit timespec nanoseconds). Linux uses 128 bytes. `setlocale()` has different category constants (macOS: `LC_MESSAGES=6`, Linux: `LC_MESSAGES=5`). `strerror_r()` on macOS returns `int` (XSI-compliant) while Linux's default returns `char*` (GNU extension). Each of these needs a translation shim.

**Variadic functions.** On ARM64, macOS and Linux differ in how variadic arguments are passed. Apple's ABI passes the first 8 arguments in registers regardless of whether they're variadic; Linux's ABI switches to the stack after the declared (non-variadic) parameters. A `printf(fmt, ...)` call compiled for macOS will place the first variadic argument in a register where Linux `printf` expects it on the stack. Our `printf` shim extracts arguments from the macOS register layout and calls the Linux `printf` with correct positioning.

**Runtime functions.** `objc_msgSend` (the ObjC message dispatch function) needs our own trampoline that redirects to shim method implementations. `dispatch_async` needs a wrapper that adapts between GCD block calling conventions. These are the most complex shims and grow in later chapters.

### Tier 2: Data Symbols

**Data symbols** provide addresses for global variables that can't be resolved via `dlsym` because they're either macOS-specific or need specific values:

- `_mach_task_self_` — a `mach_port_t` representing the current task. On macOS, this is available from `<mach/mach.h>`. On Linux, we provide a dummy value.
- `__mb_cur_max` — an `int` tracking the current locale's maximum multibyte character length.
- `_OBJC_CLASS_$_NSObject`, `_OBJC_CLASS_$_NSString` — ObjC class symbols whose addresses must point to valid class metadata structures.
- `_kCFBooleanTrue`, `_kCFAllocatorDefault` — CoreFoundation constants that must be real objects, not just addresses.

### Tier 3: Eager Cache

The eager cache is a pre-computed `HashMap<String, u64>` built from the shared cache's export tries. On macOS, we build it by walking every image in `dyld_shared_cache_arm64e`, parsing each export trie, and collecting every exported symbol with its resolved address. On Linux, we parse a copy of the cache transferred from macOS.

The eager cache provides ~280,000 symbol resolutions with O(1) lookup. It handles CoreFoundation functions, Security framework symbols, and thousands of other system APIs that aren't in our safe list because they've never been needed — but if they are needed, the cache produces the answer.

### Tier 4: The Safe List

The StubResolver — the final tier — gates `dlsym` access. Not all symbols go through. The resolver checks each symbol against a policy before calling `dlsym`:

```rust
fn is_safe_symbol(name: &str) -> bool {
    // Prefix rules — broad categories
    if name.starts_with("_Z") || name.starts_with("__Z") { return true; }  // C++ mangled
    if name.starts_with("OBJC_CLASS_$_") { return true; }                   // ObjC classes
    if name.starts_with("kCF") { return true; }                             // CF constants
    if name.starts_with("CF") && name.as_bytes().get(2)
        .is_some_and(|b| b.is_ascii_uppercase()) { return true; }           // CF functions
    if name.starts_with("swift_") || name.starts_with("$s") { return true; }// Swift runtime
    if name.starts_with("_dispatch_") { return true; }                       // GCD
    if name.starts_with("_objc_") { return true; }                          // ObjC runtime
    if name.starts_with("_os_log") { return true; }                         // os_log
    if name.starts_with("_xpc_") { return true; }                           // XPC

    // Individual symbols — ~600 C/POSIX functions
    SAFE_SYMBOLS.contains(&name)
}
```

The current safe list contains 600+ individual symbols and 50+ prefix patterns. It covers:

- **POSIX I/O**: `open`, `close`, `read`, `write`, `lseek`, `mmap`, `munmap`, `poll`, `select`
- **String/memory**: `strlen`, `strcmp`, `memcpy`, `memmove`, `memset`, `strdup`, `strndup`
- **Memory management**: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`
- **Stdio**: `fopen`, `fclose`, `fread`, `fwrite`, `fprintf`, `fgets`, `fputc`
- **Process**: `getpid`, `getppid`, `getuid`, `geteuid`, `isatty`, `getcwd`
- **Time**: `gettimeofday`, `clock_gettime`, `time`, `localtime`, `strftime`
- **Networking**: `socket`, `connect`, `bind`, `listen`, `accept`, `send`, `recv`

Explicitly blocked: `fork`, `execve`, `system`, `popen`, `setuid`, `chroot`, `ptrace`, `kill` (with signal other than SIGKILL to self), `dlopen` (to prevent loading arbitrary native code).

### Darwin Variant Suffixes

macOS symbols carry variant suffixes that select ABI-specific implementations: `_stat$INODE64`, `_fopen$UNIX2003`, `_strtod$DARWIN_EXTSN`, `_readdir$INODE64`. These suffixes select behaviors that became default in later OS versions. `$INODE64` selects 64-bit inode numbers (default since macOS 10.6). `$UNIX2003` selects POSIX-2003-compliant behavior (default since 10.5). The suffixed symbols exist for backward compatibility with binaries built against older SDKs.

On Linux, these suffixed variants don't exist. `stat` is always `stat`. We strip everything after `$`:

```rust
let c_name = c_name.split('$').next().unwrap_or(c_name);
```

This one line fixes dozens of resolution failures. Without it, `dlsym("stat$INODE64")` returns null — not because the function doesn't exist, but because its canonical name on Linux is just `stat`.

### Library Preloading

`dlsym(RTLD_DEFAULT)` only searches loaded libraries. If Foundation isn't loaded, `dlsym("_NSLog")` returns null even though Foundation exists on the system. We preload 30+ system frameworks at startup:

```rust
const PRELOAD_LIBRARIES: &[&str] = &[
    "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation",
    "/System/Library/Frameworks/Foundation.framework/Foundation",
    "/System/Library/Frameworks/Security.framework/Security",
    "/System/Library/Frameworks/IOKit.framework/IOKit",
    "/usr/lib/libz.1.dylib",
    "/usr/lib/libbz2.1.0.dylib",
    "/usr/lib/libxml2.2.dylib",
    "/usr/lib/libsqlite3.dylib",
    "/usr/lib/swift/libswiftCore.dylib",
    // ... 20+ more
];
```

Preloading happens once per process via a `Once` guard. Each path goes through `dlopen` with `RTLD_GLOBAL` — making all symbols available to `RTLD_DEFAULT` lookups. On macOS, the kernel transparently resolves these paths from the shared cache. On Linux, we preload Linux equivalents: `libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libz.so.1`.

The Linux preload list is shorter (9 libraries vs 30+) because Linux doesn't have equivalents for most Apple frameworks. CoreFoundation, Foundation, Security — these are macOS-specific. On Linux, those symbols come from the eager cache or from our shim table.

---

## Scene 3.6: Dependency Loading

A binary doesn't exist in isolation. Even the simplest tool has dependencies. `strings` depends on libSystem (libc, libpthread, libm) and libc++ (the C++ standard library). But the Xcode tools we actually care about go much deeper:

```
$ otool -L .../momc
  @rpath/DVTFoundation.framework/Versions/A/DVTFoundation
  @rpath/IDEFoundation.framework/Versions/A/IDEFoundation
  @rpath/DVTCocoaAdditionsKit.framework/Versions/A/DVTCocoaAdditionsKit
  @rpath/DVTServices.framework/Versions/A/DVTServices
  @rpath/DVTPortal.framework/Versions/A/DVTPortal
  /System/Library/Frameworks/Foundation.framework/Versions/C/Foundation
  /System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation
  /usr/lib/libSystem.B.dylib
  /usr/lib/libc++.1.dylib
  ...
```

Each `@rpath` framework has its own dependencies. DVTFoundation imports DVTServices and DVTAnalytics. IDEFoundation imports DVTFoundation, DVTServices, and DVTCompilation. The transitive closure for `momc` spans 80+ images.

This isn't unique to macOS. Any dynamically linked system has this property — ELF binaries on Linux have `DT_NEEDED` entries that form similar graphs. What makes macOS more complex is the depth: Apple's framework layering means even simple tools pull in framework chains five or six layers deep.

### BFS Walk

We load dependencies breadth-first. Breadth-first is the natural choice because it matches dyld's behavior: all direct dependencies are loaded before any transitive dependency. This matters when initializer order is involved (not yet — Chapter 5), but it also produces better error messages: if a direct dependency is missing, we report it immediately rather than failing deep in some sub-framework's sub-dependency.

```rust
// File: crates/macho-runtime/src/loader.rs

pub(crate) fn load_dependencies(
    main_parsed: &ParsedMachO,
    registry: &mut ImageRegistry,
    binary_dir: &Path,
    fs: &dyn FilesystemProvider,
) -> Result<DependencyLoadStats, RuntimeError> {
    let mut queue: VecDeque<(String, Vec<String>, PathBuf)> = VecDeque::new();
    let mut stats = DependencyLoadStats::default();

    // Seed: main binary's direct deps (skip system libs)
    let rpaths = main_parsed.rpaths.clone();
    for dlib in &main_parsed.dylibs {
        if is_system_library(&dlib.name) {
            stats.system_skipped += 1;
            continue;
        }
        queue.push_back((dlib.name.clone(), rpaths.clone(), binary_dir.to_path_buf()));
    }

    while let Some((raw_name, rpaths, loader_dir)) = queue.pop_front() {
        let install_name = normalize_install_name(&raw_name);
        if registry.get(&install_name).is_some() {
            stats.deduplicated += 1;
            continue;  // Already loaded — skip
        }

        let path = resolve_dylib_path(&install_name, &rpaths,
                                        binary_dir, &loader_dir, fs)?;
        let dylib_data = fs.read(&path)?;
        let sub_parsed = parse(&dylib_data)?;

        // Enqueue sub-deps before registering (BFS order)
        let sub_rpaths = sub_parsed.rpaths.clone();
        let sub_loader_dir = path.parent().unwrap_or(binary_dir).to_path_buf();
        for sub_dlib in &sub_parsed.dylibs {
            if !is_system_library(&sub_dlib.name)
                && registry.get(&sub_dlib.name).is_none() {
                queue.push_back((
                    sub_dlib.name.clone(),
                    sub_rpaths.clone(),
                    sub_loader_dir.clone(),
                ));
            }
        }

        registry.load_preparsed(&install_name,
                                 Arc::new(sub_parsed), Arc::from(dylib_data))?;
        stats.loaded += 1;
    }

    Ok(stats)
}
```

Cycle detection is implicit in the deduplication check — `registry.get(&install_name).is_some()`. If A depends on B and B depends on A, the first pass loads A and queues B. When B's dependencies are processed, A is already in the registry, so it's skipped. No infinite loop, no explicit visited set needed.

Diamond dependencies are the common case, not the exception. Both IDEFoundation and DVTCocoaAdditionsKit depend on DVTFoundation. Without deduplication, DVTFoundation would be loaded twice — wasting memory about double fixup processing. With it, the second reference finds DVTFoundation already registered and increments `stats.deduplicated`.

### System Library Boundary

The system library check is deliberately simple:

```rust
pub(crate) fn is_system_library(name: &str) -> bool {
    name.starts_with("/usr/lib/")
        || name.starts_with("/System/")
        || name.starts_with("/Library/")
}
```

System libraries are resolved through our CompositeResolver (tiers 1-4 from Scene 3.5), not loaded from disk. We never try to parse `/usr/lib/libSystem.B.dylib` — it's a stub file since macOS 11, and its real code lives in the shared cache. Non-system libraries (Xcode's private frameworks at `@rpath` paths) are loaded via BFS, parsed, and get their own fixup processing.

This boundary is one of the most important architectural decisions in the loader. It separates the world into "things we load and manage" (application code, Xcode frameworks) and "things the host system provides" (libc, CoreFoundation, Foundation). Everything below the boundary is accessed through our resolver. Everything above is under our control.

### Path Resolution

The path resolver handles all three special prefixes and the rpath search:

```rust
fn resolve_dylib_path(name: &str, rpaths: &[String],
                       binary_dir: &Path, loader_dir: &Path,
                       fs: &dyn FilesystemProvider) -> Option<PathBuf> {
    if let Some(rest) = name.strip_prefix("@rpath/") {
        for rpath in rpaths {
            let expanded = expand_at_variables(rpath, binary_dir, loader_dir);
            let candidate = expanded.join(rest);
            if fs.exists(&candidate) { return Some(candidate); }
        }
        return None;
    }
    if let Some(rest) = name.strip_prefix("@executable_path/") {
        return Some(binary_dir.join(rest));
    }
    if let Some(rest) = name.strip_prefix("@loader_path/") {
        return Some(loader_dir.join(rest));
    }
    // Absolute path — use as-is
    Some(PathBuf::from(name))
}
```

The distinction between `@executable_path` and `@loader_path` is subtle but critical. When the main binary loads DVTFoundation, both variables point to the same directory. But when DVTFoundation loads DVTServices, `@executable_path` still points to the main binary's directory while `@loader_path` points to DVTFoundation's directory. If DVTFoundation's rpath is `@loader_path/../Frameworks`, the search starts relative to DVTFoundation — not the main binary. This allows frameworks to find their own sub-frameworks without knowing the application's bundle structure.

### Scale

For `momc`, the dependency graph statistics look like:

```
Dependency loading complete:
  Direct dependencies:    9
  Transitive loaded:     73
  System skipped:        14
  Deduplicated:          31
  Total unique images:   82
  Time: 0.847ms
```

82 unique images. Each gets its own fixup phase — chained fixups decoded, PAC pointers signed, symbols resolved through the CompositeResolver. The whole graph loads in under a millisecond because we're only parsing headers and load commands, not mapping code pages. The system libraries — where the bulk of the code lives — are handled by the host OS through our `dlopen` preloading.

The BFS walk is the foundation. Everything built on top — fixup application (this chapter), ObjC metadata loading (Chapter 4), initializer ordering (Chapter 5) — operates on the image set produced by this walk. Get the walk wrong and everything downstream fails in confusing ways: missing fixups because an image wasn't loaded, missing ObjC classes because the framework that defines them wasn't parsed, wrong initializer order because dependencies weren't discovered in the right sequence.

---

## Scene 3.7: The Shared Cache

Look in `/usr/lib/` on a modern Mac:

```
$ ls -la /usr/lib/libSystem.B.dylib
-rwxr-xr-x  1 root  wheel  140944 Mar  6 2024 /usr/lib/libSystem.B.dylib
```

The file exists. It's 140KB. But libSystem re-exports all of libc, libpthread, libdyld, libdispatch — megabytes of code. Where is it?

```
$ ls -la /System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/
total 4485760
drwxr-xr-x  4 root  wheel         128 Mar  6  2024 .
drwxr-xr-x  3 root  wheel          96 Sep 29  2023 ..
-rwxr-xr-x  1 root  wheel  2294153216 Mar  6  2024 dyld_shared_cache_arm64e
-rwxr-xr-x  1 root  wheel   547864576 Mar  6  2024 dyld_shared_cache_arm64e.01
```

`dyld_shared_cache_arm64e` is 2.2 gigabytes. It contains every system framework and library — over 2,000 images — pre-linked and optimized into a single contiguous block. The `.01` file is an overflow — the cache grew beyond the single-file limit and Apple split it. Together they represent the entirety of macOS's system code.

The 140KB file on disk is a *stub*. Since macOS 11 (Big Sur), individual `.dylib` files mostly don't exist as real binaries anymore. The code — the `__TEXT` sections with executable machine code — lives only in the cache. The stubs remain on disk for backward compatibility: tools that `stat()` or `access()` system library paths still get success. But `mmap()` the stub and you'll find a minimal Mach-O with no code segments, just enough load commands to redirect to the cache.

### Why It Exists

The shared cache is Apple's most aggressive optimization for application launch time, and its history traces a compelling evolution.

Before the cache (macOS 10.0-10.5, circa 2001-2007), launching an application meant sequential I/O against spinning hard drives. Each framework was a separate file. A typical Cocoa app pulled in 30-50 frameworks — Foundation, AppKit, CoreFoundation, CoreGraphics, CoreText, Security, SystemConfiguration, and dozens more. Reading each file required filesystem metadata lookups, inode reads, and block allocation reads. On a 5400 RPM laptop drive, each framework load added 5-15ms of seek time. Thirty frameworks meant 150-450ms just for disk I/O — before any parsing, fixup, or initialization.

Apple's response was `update_dyld_shared_cache`, a tool that runs during OS installation and after system updates. It reads every system library, parses their Mach-O structure, pre-applies all rebases and binds between system libraries, optimizes the ObjC metadata (selector uniquing, class metadata relocation), and writes the result as a single contiguous file.

The kernel cooperates. At process creation, in the `execve` system call, the kernel maps `dyld_shared_cache_arm64e` into the new process's address space at a fixed base address (with ASLR slide). Because all systemlibraries are in one contiguous mapping, there's one `mmap` call instead of 2,000+. Page faults pull in 4KB at a time from one file instead of seeking across thousands.

In macOS 11, Apple went further: individual libraries were removed from disk entirely. The stubs exist for `stat`/`access` compatibility, but the real code is only in the cache. This saves approximately 1GB of disk space (the shared cache is smaller than the sum of its parts due to deduplication and optimization), simplifies updates, and makes reverse engineering harder.

### Parsing the Format

The cache starts with a recognizable magic string — 16 bytes that identify architecture and format version:

```rust
// File: crates/macho-shared-cache/src/header.rs

const MAGIC_ARM64E: &[u8; 16] = b"dyld_v1  arm64e\0";
const MAGIC_ARM64: &[u8; 16]  = b"dyld_v1   arm64\0";
const MAGIC_X86_64: &[u8; 16] = b"dyld_v1  x86_64\0";
```

The leading space-padding is intentional — all magic strings are exactly 16 bytes, with spaces filling the gap between `dyld_v1` and the architecture name. Parsing starts with a 16-byte read and a `memcmp`.

The header immediately following describes the cache's structure:

```rust
pub struct CacheHeader {
    pub magic: [u8; 16],
    pub mapping_count: u32,
    pub mapping_offset: u32,
    pub images_count: u32,
    pub images_offset: u32,
    pub dyld_base_address: u64,
    // ... additional fields for local symbols, code signature, slide info
}
```

The first thing to understand about the cache is its physical layout. It's organized into large contiguous *mappings* — regions that correspond to memory protection boundaries:

| Region | Purpose | Typical Size | Protection |
|--------|---------|------|------|
| `__TEXT` | Executable code for all 2,000+ libraries | ~800 MB | r-x |
| `__DATA` | Writable data (globals, writeable metadata) | ~200 MB | rw- |
| `__DATA_CONST` | Read-only-after-fixup data (GOT, const metadata) | ~100 MB | rw- → r-- |
| `__LINKEDIT` | Symbol tables, string pools, export tries | ~1.1 GB | r-- |

Each mapping is described by a `MappingInfo` structure:

```rust
pub struct MappingInfo {
    pub address: u64,       // VM address where this region is mapped
    pub size: u64,          // Size in bytes
    pub file_offset: u64,   // Offset within the cache file
    pub max_prot: u32,      // Maximum memory protection
    pub init_prot: u32,     // Initial memory protection (before fixups)
}
```

The key relationship: `address` tells you where in virtual memory this region lives. `file_offset` tells you where in the 2.2GB file the data is. To convert a virtual address (from an export trie or symbol table) to a file offset (to actually read the data), you subtract the mapping's base address and add the file offset:

```rust
fn vm_addr_to_file_offset(&self, addr: u64) -> Option<usize> {
    for mapping in &self.mappings {
        if addr >= mapping.address && addr < mapping.address + mapping.size {
            return Some((mapping.file_offset + (addr - mapping.address)) as usize);
        }
    }
    None
}
```

This function is called hundreds of thousands of times during cache analysis. Each symbol resolution from the export trie produces a VM address that needs conversion to a file offset for reading.

### The Image Catalog

The image catalog is an array of ~2,400 `ImageInfo` entries. Each maps an install name (like `/usr/lib/libSystem.B.dylib`) to a VM address where that library's Mach-O header lives within the cache's `__TEXT` mapping:

```rust
pub fn find_image(&self, path: &str) -> Option<&ImageInfo> {
    self.images.iter().find(|img| img.path() == path)
}
```

The linear search is adequate for our use — we typically look up fewer than 100 images. dyld uses a hash table in the cache header for O(1) lookup during application launch, but we don't need that speed.

Finding an image gives you a VM address. Converting that address to a file offset gives you the start of a Mach-O header. From there, you can parse the library's load commands, segments, export trie — all the same structures we parse in standalone binaries.

### Extracting Images

To get a specific library's parseable data, we read starting from the Mach-O header and extending through the load commands:

```rust
pub fn extract_image(&self, image: &ImageInfo) -> Result<&'a [u8], SharedCacheError> {
    let file_offset = self.vm_addr_to_file_offset(image.address())?;
    let remaining = &self.data[file_offset..];

    // Read mach_header_64: magic(4) + cputype(4) + cpusubtype(4) +
    //   filetype(4) + ncmds(4) + sizeofcmds(4) + flags(4) + reserved(4) = 32 bytes
    let sizeofcmds = u32::from_le_bytes([
        remaining[20], remaining[21], remaining[22], remaining[23],
    ]) as usize;

    // Header (32 bytes) + load commands
    Ok(&self.data[file_offset..file_offset + 32 + sizeofcmds])
}
```

The extracted data is a valid Mach-O parseable by `macho-parse`. We can read its export trie (to find which symbols this library exports), its dependencies (to discover its own imports), and its load commands. The key limitation: the extracted slice only includes the header and load commands, not the segment data. The actual code and data pages live at different file offsets within the cache — they're interleaved with code and data from other libraries in the same mapping region.

### Slide Info

The cache has its own ASLR. At process creation, the kernel applies a random slide to the cache's base address. Every internal pointer within the cache — every rebase target, every data reference — needs adjustment by this slide value.

The cache includes a *slide info* section that describes which `__DATA` pointers need adjustment. This is the cache-level equivalent of per-library rebase fixups, adapted for the cache's unique structure. The slide info uses a page-based bitmap: for each 4KB data page, a bitmask indicates which 8-byte-aligned slots contain pointers that need sliding.

For our use case — symbol resolution via `dlsym` on macOS, or export trie parsing for the eager cache — we generally don't need to process slide info ourselves. On macOS, the kernel handled it during process creation. On Linux, where we parse a *copy* of the cache file, we need the slide info to correctly interpret data pointers within the cache — but we only do this for building the eager export map, where we work with VM addresses from export tries that are slide-independent.

### Local Symbol Extraction

When Apple builds the cache, local symbols (functions and variables not exported to other libraries) are stripped from individual libraries' symbol tables. But they're not thrown away — they're preserved in a compressed local symbol table within the cache. This data gives us function names for addresses inside system frameworks.

We use local symbols for a specific purpose: crash diagnostics. When the loader crashes at an address inside Foundation or CoreFoundation, the local symbol table lets us identify the function — information that's invaluable for debugging but not available through `dlsym` (which only handles exported symbols) or through the on-disk stubs (which have no code at all).

The local symbol data is organized as a `nlist` array (the standard Mach-O symbol table format) with entries sorted by library. A side table maps image index → (start entry, count) into the local symbol array. To find symbols for a specific image, look up the range and binary-search within it.

### Impact on Our Loader

The shared cache means system libraries can't be loaded from disk as separate files. This affects our architecture in two places:

**On macOS**, we use `dlopen` to load system frameworks. The kernel transparently resolves these from the cache — `dlopen("/System/Library/Frameworks/Foundation.framework/Foundation", RTLD_GLOBAL)` doesn't read the on-disk stub. It finds Foundation's code already mapped from the shared cache and sets up `dlsym` resolution. Our preload strategy (Scene 3.5) leverages this.

**On Linux**, there is no shared cache and no `dlopen` for macOS dylibs. We transfer a copy of `dyld_shared_cache_arm64e` to the Linux machine, `mmap` it, parse every image's export trie, and build an eager export map:

```rust
pub struct EagerCacheResolver {
    exports: HashMap<String, u64>,
}

impl EagerCacheResolver {
    pub fn from_cache(cache: &SharedCache) -> Self {
        let mut exports = HashMap::new();
        for image in cache.images() {
            if let Ok(data) = cache.extract_image(image) {
                if let Ok(parsed) = parse(data) {
                    for export in &parsed.exports {
                        exports.insert(export.name.clone(), export.address);
                    }
                }
            }
        }
        EagerCacheResolver { exports }
    }
}

impl SymbolResolver for EagerCacheResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        self.exports.get(symbol_name).copied()
    }
}
```

Building the eager map takes ~2 seconds at startup — parsing 2,400 export tries is not free. But once built, every symbol resolve is a HashMap lookup.

### First Light

With the complete linking pipeline assembled — chained fixup decoding, PAC signing, four-tier symbol resolution, BFS dependency loading, shared cache integration — we have a functional dynamic linker. Let's test it:

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/strings test.txt
Hello, world!
This is a test file.

$ macho-inspect --exec /opt/xcode-tools/xcode-tc/nm test.o
0000000100003f40 T _main
                 U _printf

$ macho-inspect --exec /opt/xcode-tools/xcode-tc/size test.o
__TEXT    __DATA    __OBJC    others    dec       hex
4096     0         0         4096      8192      2000
```

`strings` runs. `nm` runs. `size` runs.

Each of these tools makes dozens of function calls — `printf`, `open`, `read`, `malloc`, `strlen`, `fopen`, `fclose`, `fwrite`, `strcmp`, `qsort`. Every call goes through the GOT that was empty at the start of this chapter. Our decoder parsed the chained fixup entries. Our resolver turned symbol names into addresses. Our PAC signer produced valid cryptographic signatures. Our dependency walker found every framework these tools need. And the final addresses — signed, verified, correct — sit in the GOT slots where the binary expects them.

These are C-only tools. They use `printf` to write output and `fopen` to read files. Basic. But consider what had to work:

1. The chained fixup header was parsed correctly — pointer format, starts table, imports table.
2. Every chain was walked with the correct stride — every `next` offset decoded properly.
3. Every bind ordinal was looked up in the imports table to get a symbol name.
4. Every symbol name was resolved through the four-tier cascade to get an address.
5. Every rebase target was relocated by the correct ASLR slide.
6. Every auth-GOT entry was signed with the correct PAC key and discriminator.
7. Every dependency was discovered by BFS and registered.
8. Every system library was resolved from the shared cache or via `dlsym`.

Any one of these steps failing would produce a SIGSEGV. All of them work.

Now try something more ambitious:

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/xcodebuild -version
SIGSEGV at PC 0x00000001a42f8c30
  x0  = 0x0000000100010000   (an ObjC object)
  x1  = 0x00000001a4250038   (a selector — "init")
```

`xcodebuild` isn't C-only. Its first meaningful act after entering `main()` is to call `objc_msgSend` — the Objective-C message dispatch function. `objc_msgSend` doesn't read from the GOT. It reads *class metadata*: isa pointers, method lists, selector tables, category lists, protocol conformances, associated object tables, exception handling data, autorelease pool state. An entirely separate runtime hidden inside these binaries, with its own loading requirements, its own initialization sequence, its own compatibility challenges.

C functions work. The linker is proven. But the first ObjC call — `[NSObject init]`, the simplest possible message send — crashes. Not because our linking is wrong, but because there's an entire *runtime* that needs building.

That's Chapter 4.
