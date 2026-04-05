# Chapter 1: The Binary

## Scene 1.1: The Black Box

Every macOS binary begins with a greeting.

Open `/usr/bin/true` in a hex editor. At offset zero, four bytes:

```
CF FA ED FE
```

Little-endian for `0xFEEDFACF`. The Mach-O 64-bit magic number.

That's the file introducing itself to the operating system's loader. It has been doing this since 1997, when NeXT's technology became Apple's foundation. Every binary you've ever run on a Mac starts with this handshake.

Most developers never see it. They write Swift or Objective-C, press ⌘R, and the binary appears in `/Build/Products/Debug/`. A black box that enters the system and does its work unseen.

We're going to open the black box.

Run `hexdump -C /usr/bin/true | head` in your terminal. Here's what you'll see:

```
00000000  cf fa ed fe 0c 00 00 01  00 00 00 00 02 00 00 00  |................|
00000010  10 00 00 00 d0 04 00 00  85 00 20 00 00 00 00 00  |.......... .....|
00000020  19 00 00 00 48 00 00 00  5f 5f 50 41 47 45 5a 45  |....H...__PAGEZE|
00000030  52 4f 00 00 00 00 00 00  00 00 00 00 00 00 00 00  |RO..............|
00000040  00 00 00 00 01 00 00 00  00 00 00 00 00 00 00 00  |................|
00000050  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00  |................|
00000060  00 00 00 00 00 00 00 00  19 00 00 00 28 01 00 00  |............(...|
00000070  5f 5f 54 45 58 54 00 00  00 00 00 00 00 00 00 00  |__TEXT..........|
00000080  00 00 00 00 01 00 00 00  00 40 00 00 00 00 00 00  |.........@......|
00000090  00 00 00 00 00 00 00 00  00 40 00 00 00 00 00 00  |.........@......|
```

That's it. The entirety of the simplest program on your Mac — `true`, a binary whose only job is to exit with status zero — starts with these bytes. And right there in the first four, a pattern that has been stable for over twenty-five years.

`CF FA ED FE`. Read it backward: `FE ED FA CF`. Feed face. With a `CF` suffix that marks this as a 64-bit binary. Apple's engineers had a sense of humor, or at least a fondness for memorable hex constants.

The magic number is a format identifier. It says: "I am a Mach-O file. Parse me accordingly." The kernel reads these four bytes before doing anything else. If it doesn't recognize them, you get `Exec format error`. If it does, the loading process begins.

There are actually four magic values you might encounter:

| Magic | Hex | Meaning |
|---|---|---|
| `0xFEEDFACE` | `CE FA ED FE` on disk | Mach-O 32-bit |
| `0xFEEDFACF` | `CF FA ED FE` on disk | Mach-O 64-bit |
| `0xCAFEBABE` | `BE BA FE CA` on disk | Fat/Universal binary |
| `0xBEBAFECA` | `CA FE BA BE` on disk | Fat (big-endian) |

Notice the on-disk bytes look backwards. That's little-endian byte ordering — ARM64 and x86_64 both store the least significant byte first. The integer value `0xFEEDFACF` gets written to memory as `CF FA ED FE`. You'll get used to reading hex backwards. Or you'll write a parser and let the machine do it. We'll do both.

The distinction between big-endian and little-endian magic matters in practice. If you ever open a Mach-O file and see `FE ED FA CF` (no byte reversal — big-endian order), that's `MH_CIGAM_64` — a byte-swapped binary. It was built for a different byte order than the machine reading it. Our parser checks for this:

```rust
pub const MH_MAGIC_64: u32 = 0xFEED_FACF;
pub const MH_CIGAM_64: u32 = 0xCFFAEDFE;
```

`CIGAM` is `MAGIC` spelled backwards. Someone at Apple enjoyed that.

If you see `CA FE BA BE` at the start of a file, that's a fat binary — a container holding multiple architectures. Apple used these during the PowerPC-to-Intel transition and again during the Intel-to-ARM transition. Crack open a fat binary and inside you'll find two or more thin Mach-O slices, each starting with its own `FEEDFACF` or `FEEDFACE`. Our parser can extract the right slice for a given CPU type:

```rust
pub fn fat_slice_for_cpu(data: &[u8], cpu_type: CpuType) -> Result<&[u8], ParseError>;
```

Feed it a fat binary and an architecture, get back the matching slice. Turtles all the way down.

The name "Mach-O" comes from the Mach kernel, which came from Carnegie Mellon University in the 1980s. Avie Tevanian and Rick Rashid developed the Mach microkernel as a research project. Steve Jobs's company NeXT adopted it for NeXTSTEP, their operating system, along with a binary format to match. When Apple acquired NeXT in 1997, they got the Mach kernel, the Objective-C runtime, and the Mach-O binary format. All three are still at the heart of macOS today. The binary format you're staring at has survived four CPU architectures (m68k, PowerPC, x86, ARM64), two company acquisitions, and countless operating system rewrites.

This longevity isn't an accident. The Mach-O format is extensible by design — new load commands get added without changing the header, new sections get added without changing the segment structure. Apple has been extending it for decades without breaking backward compatibility. That same extensibility is what makes it possible for us to parse a modern macOS binary: the structure hasn't fundamentally changed since NeXTSTEP.

It's worth noting that `0xFEEDFACF` isn't just a cute hex constant. It's a sentinel that has real engineering value. When you're debugging a loader, when you're staring at a core dump, when you're scanning memory for mapped images — that magic number is unmistakable. You won't confuse it with anything else. It leaps out of a hex dump like a neon sign. Try finding the letters "ELF" in a random memory dump — you'll get false positives. Try finding `FEEDFACF` — you won't. That's not an accident either.

Throughout this book, `0xFEEDFACF` will recur. It's the first thing our parser checks. It's the first thing the kernel checks. It's the greeting at the beginning of every binary, and by the end of this book, seeing it in a hex dump will feel as natural as reading a street sign. Right now, it's mysterious. By Chapter 10, it'll be an old friend.

There's a difference between using a system and understanding it. You can drive a car for thirty years without knowing how an engine works. You can build apps for macOS without ever looking at a hex dump. But the moment you decide to make macOS software run on Linux — to build a loader from scratch — you need to understand every byte.

Let's look back at that hex dump. After the four magic bytes, the rest of the first sixteen bytes are:

```
0c 00 00 01  00 00 00 00  02 00 00 00
```

Those aren't random. `0c 00 00 01` is the CPU type — `0x0100000C`, which is ARM64. `00 00 00 00` is the CPU subtype — ARM64 ALL, meaning any ARM64 processor. `02 00 00 00` is the file type — `2`, which means `MH_EXECUTE`, an executable binary.

We're four fields in and already know three critical things about this file: it's 64-bit, it's ARM64, and it's an executable. Not a dynamic library. Not an object file. An executable. The kind of file the kernel can load and jump into.

Compare that to what you'd see on Linux:

```
$ hexdump -C /usr/bin/true | head -1
00000000  7f 45 4c 46 02 01 01 00  00 00 00 00 00 00 00 00  |.ELF............|
```

`7F 45 4C 46` — `\x7fELF`. Every Linux binary starts with this. Same idea, different tribe. ELF encodes the architecture, bitness, and endianness in the next few bytes, just like Mach-O. Two formats that evolved independently to solve the same problem: telling the operating system "here's what I am, here's how to load me."

The hex dump is an archaeological site. Every byte has a purpose. Every offset tells a story. The first sixteen bytes alone are a complete introduction: "I am a 64-bit ARM64 executable in Mach-O format." No ambiguity. No negotiation.

Now let's read the rest of the header.

---

## Scene 1.2: The Header

The first 32 bytes of a 64-bit Mach-O file form the `mach_header_64` structure. Apple publishes this in `<mach-o/loader.h>`, and you can find it on macOS in Apple's open-source releases. But we're building our own parser, so let's define it in Rust.

The correspondence between Apple's C struct and our Rust struct is deliberate — we want anyone who knows the C definition to recognize ours immediately. Here's the struct from our `macho-types` crate:

```rust
pub struct MachHeader {
    pub magic: u32,
    pub cpu_type: CpuType,
    pub cpu_subtype: u32,
    pub file_type: FileType,
    pub num_commands: u32,
    pub size_of_commands: u32,
    pub flags: MachFlags,
    pub reserved: Option<u32>,
}
```

Eight fields. Thirty-two bytes on disk (the `reserved` field exists only in 64-bit Mach-O, adding four bytes to the 28-byte 32-bit header). Let's walk through each one using the hex dump of `/usr/bin/true`:

```
Offset  Bytes           Field           Value
0x00    CF FA ED FE     magic           0xFEEDFACF (Mach-O 64-bit)
0x04    0C 00 00 01     cpu_type        0x0100000C (ARM64)
0x08    00 00 00 00     cpu_subtype     0x00000000 (ARM64_ALL)
0x0C    02 00 00 00     file_type       0x00000002 (MH_EXECUTE)
0x10    10 00 00 00     num_commands    16
0x14    D0 04 00 00     size_of_commands 1232 bytes
0x18    85 00 20 00     flags           0x00200085
0x1C    00 00 00 00     reserved        0x00000000
```

**`magic`** — we covered this. The format identifier. Our parser checks this first and returns `ParseError::BadMagic` if it doesn't match.

**`cpu_type`** — an encoded CPU family. The value `0x0100000C` means ARM64. The high byte `0x01` is the `CPU_ARCH_ABI64` flag that marks 64-bit architectures. The low portion, `0x0C` (decimal 12), is `CPU_TYPE_ARM`. Put them together: 64-bit ARM.

```rust
pub enum CpuType {
    X86 = 7,
    X86_64 = 0x0100_0007,
    Arm = 12,
    Arm64 = 0x0100_000C,
    Unknown(u32),
}
```

This is the same encoding Apple has used since the NeXT days, extended over time. When Apple added ARM64, they didn't redesign the field — they just added `CPU_ARCH_ABI64 | CPU_TYPE_ARM`. When they add the next architecture (if they ever do), it'll slot in the same way.

**`cpu_subtype`** — narrows the CPU target. For ARM64, `0` or `0xFF` both mean "ALL" — any ARM64 chip. The interesting case is `2`, which means ARM64E — ARMv8.3 with pointer authentication. Here's our decoding:

```rust
pub enum CpuSubtype {
    Arm64All,
    Arm64E,
    ArmV7,
    ArmV7S,
    X86_64All,
    X86_64Haswell,
    Unknown(u32),
}

impl CpuSubtype {
    pub fn from_pair(cpu_type: CpuType, raw: u32) -> Self {
        let sub = raw & !0x8000_0000; // mask off CPU_SUBTYPE_LIB64
        match cpu_type {
            CpuType::Arm64 => match sub {
                0 | 0xFF => Self::Arm64All,
                2 => Self::Arm64E,
                _ => Self::Unknown(raw),
            },
            // ...
        }
    }
}
```

The high bit (`0x80000000`) is `CPU_SUBTYPE_LIB64` — a capability flag that we mask off for matching. ARM64E — pointer-authenticated code — matters enormously for us. Binaries compiled with `arm64e` use `PACIA`/`AUTIA` instructions to sign and verify function pointers, making ROP (return-oriented programming) attacks much harder. We'll encounter pointer authentication in Chapter 4 when we build the Objective-C runtime shim (method pointers in iOS binaries are PAC-signed), and it matters deeply in Chapter 7 when we interpret ARM64 instructions. For now, just note that the subtype tells the loader whether the binary uses hardware security features that affect how function pointers work.

**`file_type`** — what kind of Mach-O this is:

```rust
pub enum FileType {
    Object = 1,      // .o — relocatable object
    Execute = 2,     // executable binary
    Dylib = 6,       // .dylib — dynamic library
    Bundle = 8,      // .bundle — loadable bundle
    DylibStub = 9,   // stub dylib
    Dsym = 10,       // dSYM debug symbols
    Unknown(u32),
}
```

Executables are type 2. Dylibs are type 6. Object files (`.o`) are type 1. Each type has different loading semantics. We're focused on executables for most of this book, but we'll need to parse dylibs starting in Chapter 3 when we resolve symbols.

**`num_commands`** — the number of load commands that follow the header. Our binary has 16. Each load command is an instruction to the loader: map this segment, load this library, use this entry point. We'll dissect them in the next scene.

**`size_of_commands`** — the total byte size of all load commands combined. Here it's 1,232 bytes. If you add the 32-byte header to 1,232, you get 1,264 — the offset where load commands end and the first segment's data begins. This size field is a safety check: it tells the parser exactly how much data to expect before the load commands are done.

**`flags`** — a bitfield that encodes various loader behaviors. The value `0x00200085` decomposes like this:

```rust
bitflags! {
    pub struct MachFlags: u32 {
        const NO_UNDEFS    = 0x0000_0001;  // no undefined symbols
        const DYLDLINK     = 0x0000_0004;  // input for the dynamic linker
        const TWO_LEVEL    = 0x0000_0080;  // two-level namespace
        const PIE          = 0x0020_0000;  // position-independent executable
        // ... many more
    }
}
```

`0x00200085` has bits set for `NO_UNDEFS`, `DYLDLINK`, `TWO_LEVEL`, and `PIE`. Let's break these down:

- **`NO_UNDEFS`** (`0x1`) — no undefined symbols remain unresolved. The linker verified at build time that every symbol reference has a corresponding definition in some dylib. This doesn't mean the symbols are *resolved* — they're not, not until load time — it means the linker checked that they *can be* resolved.

- **`DYLDLINK`** (`0x4`) — this binary is an input to the dynamic linker. It has external references that need to be bound at load time. Almost every binary has this set. The rare exception is a statically linked binary, which would have `NO_UNDEFS` but not `DYLDLINK`.

- **`TWO_LEVEL`** (`0x80`) — two-level namespaces. Each undefined symbol specifies which library provides it, not just that *someone* provides it. Without this flag, symbol resolution degrades to flat namespace — search every loaded library in order, take the first match. Two-level is safer and faster.

- **`PIE`** (`0x200000`) — position-independent executable. The binary can be loaded at any base address. The kernel will randomize where it lands in memory (ASLR — Address Space Layout Randomization), and the binary's internal references use relative offsets, not absolute addresses. Every modern macOS executable has this set. If you tried to build one without it, the linker would warn you. Position independence is assumed by the security model.

There are other flags we care about too:

```rust
const HAS_TLV_DESCRIPTORS = 0x0080_0000;  // thread-local variables
const DYLIB_IN_CACHE      = 0x8000_0000;  // lives in dyld shared cache
```

`HAS_TLV_DESCRIPTORS` tells us the binary uses thread-local storage — `__thread` variables in C or `thread_local` in Swift. We'll need to handle TLV initialization in Chapter 8 when we build the full runtime pipeline. `DYLIB_IN_CACHE` indicates a system dylib that lives in the dyld shared cache rather than as a standalone file — important when we need to extract libraries for parsing.

**`reserved`** — padding to align the load commands on a nice boundary. Always zero.

That's the full header. Thirty-two bytes, fully decoded. Let's take a moment to appreciate what just happened: we went from opaque hex — `cf fa ed fe 0c 00 00 01...` — to a structured description. ARM64 executable, 16 load commands, position-independent, two-level namespace. The bytes were always saying this. We just learned to listen.

This is the process that will repeat throughout the book. Raw bytes become structured data. Structured data reveals behavior. Behavior reveals the assumptions the binary makes about its environment. And those assumptions, one by one, become the specification for our loader.

Now let's try something dangerous.

### The First Crash

Take the binary somewhere it doesn't belong. Copy `/usr/bin/true` to a Docker container running Ubuntu on ARM64:

```
$ docker run --rm --platform linux/arm64 ubuntu:22.04 ./true
Exec format error
```

Linux doesn't recognize `0xFEEDFACF`. It's looking for `0x7F454C46` — the ELF magic number, `\x7fELF`. The kernel reads the first four bytes, checks them against its list of registered binary formats (binfmt), finds no match, and refuses to execute. Fair enough. The kernel's job is to check the magic number and dispatch to the right binary handler. Linux has a handler for ELF. It does not have one for Mach-O. End of story — at least, the kernel's story.

Our story is different. We're going to register ourselves as the binary handler.

Instead of asking Linux to execute the Mach-O binary, we write a program — an ELF program, one that *Linux can run* — that reads the Mach-O file, maps its segments into memory with the right permissions, resolves its symbols, and jumps to the entry point. A loader. A user-space program that does what `dyld` does on macOS. The kernel doesn't need to know about Mach-O. We handle it.

This is the entire project in a nutshell. And the first crash from our loader won't be `Exec format error`. It'll be something more informative and much more interesting.

```
$ ./our-loader /usr/bin/true
[SIGSEGV] signal 11 at PC 0x0000000100003f9c
```

Signal 11. Segmentation fault. The binary loaded — we got past the `Exec format error` because *we* read the header, not the kernel. The code started to execute — the PC (program counter) is at `0x100003f9c`, which is inside the binary's `__TEXT` segment. But then it touched memory it shouldn't have. A function pointer was null. A library wasn't loaded. The binary expected macOS underneath it. It got Linux. The ground it stood on was wrong.

The SIGSEGV is at PC `0x100003f9c`. We subtract the text base address (`0x100000000`) and get offset `0x3f9c`. That's eight bytes past `_main` — the binary set up its stack frame, then tried to call `_setlocale` through a stub. The stub loaded a GOT entry. The GOT entry was zero — we never filled it in. The CPU jumped to address zero. Crash.

That crash is information, not failure. It tells us exactly what the binary expected and didn't find. The header told us *what* the binary is. The crash tells us *what it needs*. And what it needs, it turns out, is chapters 2 through 9 of this book.

Every crash is a clue. That's the philosophy. When the loader segfaults, we don't panic — we examine. What address? What instruction? What was it trying to access? What register held the pointer that was wrong? The binary tells the truth, even when it dies. *Especially* when it dies.

There's something almost poetic about it. The binary has been silently cooperating with macOS for years — loaded, relocated, linked, initialized, executed, torn down — thousands of times a day, across millions of machines — and nobody noticed. The handshake was invisible. Now we try to load it ourselves, on the wrong operating system, and it objects violently. Every objection is a lesson. Every SIGSEGV is a specification. The crash is the binary saying "I need *this* and you haven't provided it."

We will provide it. All of it. Every library, every symbol, every runtime structure. By the time you finish this book, that same binary will run on Linux inside a Docker container. And you'll understand every byte, every shim, and every fixup that makes it possible.

But first, we need to understand the instructions the binary carries for its own loading: the load commands.

---

## Scene 1.3: The Load Commands

The header is a table of contents, but it doesn't tell the loader *what to do*. That's the job of the load commands — an array of variable-length structures that immediately follow the 32-byte header. Each one is an instruction: "Map this chunk of the file into memory." "Load this dynamic library." "Start execution at this offset." "Here's my UUID." "Here's my code signature."

If the header says *what I am*, the load commands say *what I need*.

The header told us there are 16 load commands totaling 1,232 bytes. Let's walk them.

Every load command starts with two fields:

```
Offset    Bytes          Field
0x20      19 00 00 00    cmd       = 0x19 (LC_SEGMENT_64)
0x24      48 00 00 00    cmdsize   = 72 bytes
```

That's the universal prefix. `cmd` identifies the command type. `cmdsize` tells you how many bytes this command occupies, including the prefix. To iterate to the next command, advance by `cmdsize` bytes. Repeat until you've consumed all `num_commands` entries.

In Rust:

```rust
pub struct LoadCommand {
    pub cmd: LoadCommandType,
    pub cmd_size: u32,
    pub payload: LoadCommandPayload,
}
```

The `cmd_size` field makes the array self-describing. A parser that doesn't understand a particular command type can skip it by jumping `cmd_size` bytes forward. This is how Apple adds new load commands without breaking old tools — and they add them regularly. `LC_DYLD_CHAINED_FIXUPS` (command `0x80000034`) didn't exist before macOS 12. Our parser handles it. Older parsers skip it. Everyone moves on.

There's a subtlety in the command numbering you'll notice in our enum definition:

```rust
pub enum LoadCommandType {
    Segment = 0x1,
    Symtab = 0x2,
    UnixThread = 0x5,
    Dysymtab = 0xB,
    LoadDylib = 0xC,
    Segment64 = 0x19,
    Uuid = 0x1B,
    CodeSignature = 0x1D,
    BuildVersion = 0x32,
    // Notice the high bit:
    LoadWeakDylib = 0x8000_0018,
    Main = 0x8000_0028,
    DyldChainedFixups = 0x8000_0034,
    Unknown(u32),
}
```

Commands with the high bit set (`0x80000000`) indicate that the loader *must* understand them or refuse to load the binary. Commands without the high bit are optional — if you don't recognize them, skip them. `LC_MAIN` has the high bit because if you don't know where execution starts, you can't run the binary. `LC_UUID` does not because identification is informational — the binary runs fine without it.

This backwards-compatible versioning scheme has kept the format functional across three decades of evolution. The same basic iterator works on a NeXTSTEP binary from 1993 and a macOS binary from 2025. Some commands are new, but the iteration mechanism is unchanged.

Let's look at the taxonomy. Of the ~40 defined load command types, three categories matter most for a loader:

### Segment Commands: `LC_SEGMENT_64`

The most important load command. `LC_SEGMENT_64` (command `0x19`) describes a contiguous region of the binary that needs to be mapped into memory. A typical executable has four or five segments:

```
LC_SEGMENT_64  __PAGEZERO   vmaddr=0x0         vmsize=0x100000000   filesize=0
LC_SEGMENT_64  __TEXT        vmaddr=0x100000000 vmsize=0x4000        filesize=0x4000
LC_SEGMENT_64  __DATA_CONST  vmaddr=0x100004000 vmsize=0x4000        filesize=0x4000
LC_SEGMENT_64  __DATA        vmaddr=0x100008000 vmsize=0x4000        filesize=0x4000
LC_SEGMENT_64  __LINKEDIT   vmaddr=0x10000c000 vmsize=0x4000        filesize=0x2a48
```

`__PAGEZERO` is a curiosity: it has a massive virtual size (4 GB for arm64) but zero file size. It's a guard region — unmapped memory at the bottom of the address space that catches null pointer dereferences. Any access to an address below `0x100000000` on arm64 will fault immediately. The kernel maps this as a hole, not actual memory. If someone writes `*(int *)0 = 42;`, the CPU attempts to access address `0x0`, which falls inside `__PAGEZERO`, which has no permissions, which triggers a SIGSEGV. Null pointer protection is built into the binary format.

Why 4 GB? Because Apple wanted to ensure that all 32-bit memory addresses — the entire range `0x0` through `0xFFFFFFFF` — are off limits. No valid pointer in a 64-bit ARM64 binary should ever look like a 32-bit address. This catches an entire class of pointer truncation bugs where a 64-bit pointer is accidentally stored in a 32-bit variable.

`__TEXT` contains the executable code and read-only data. It gets mapped with read+execute permissions (`r-x`). This is where your compiled functions live, along with string constants, jump stubs, and unwind information. The permissions matter — `__TEXT` is never writable. On macOS, if you try to write to a page mapped from `__TEXT`, you get a SIGBUS. This is deliberate: code immutability is a security invariant. Self-modifying code went out of fashion decades ago, and the kernel enforces that.

`__DATA_CONST` and `__DATA` hold writable data — global variables, Objective-C metadata, GOT (Global Offset Table) entries, thread-local storage descriptors. `__DATA_CONST` is a relatively modern addition (macOS 10.14 era): it gets mapped as writable during loading so the fixup engine can rewrite pointers, then re-protected to read-only before execution begins. `__DATA` stays writable for the program's lifetime — it holds mutable global state that the binary reads and writes during normal execution.

`__LINKEDIT` is special. It doesn't contain code or data that the program directly uses. Instead, it holds metadata the loader needs: the symbol table, the string table, code signature data, fixup information. It's the backstage area of the binary.

Each segment command includes the fields needed to map it:

```rust
pub struct Segment {
    pub name: String,
    pub vmaddr: u64,       // desired virtual address
    pub vmsize: u64,       // size in memory
    pub fileoff: u64,      // offset in the file
    pub filesize: u64,     // size in the file
    pub maxprot: VmProt,   // maximum memory protection
    pub initprot: VmProt,  // initial memory protection
    pub nsects: u32,       // number of sections
    pub flags: u32,
    pub sections: Vec<Section>,
}
```

The `vmaddr` and `vmsize` tell you where to place this segment in virtual memory and how large the mapping should be. The `fileoff` and `filesize` tell you where to read the data from the file. If `vmsize` exceeds `filesize`, the extra bytes are zero-filled — this handles BSS (uninitialized data) and zero-fill sections without wasting file space. The `initprot` tells you what permissions to set: read, write, execute. `maxprot` is the maximum protection the segment can be changed to at runtime — a segment with `maxprot=r-x` can never become writable, even if you call `mprotect`. In Chapter 2, we'll use exactly these fields to call `mmap` — one segment at a time, building the binary's memory image from nothing.

### Dependency Commands: `LC_LOAD_DYLIB`

Every dynamic library the binary needs is declared with an `LC_LOAD_DYLIB` command. Let's see what `/usr/bin/true` depends on:

```
LC_LOAD_DYLIB  /usr/lib/libSystem.B.dylib  (compat 1.0.0, current 1319.0.0)
```

Just one library: `libSystem.B.dylib`, Apple's foundational system library. It's the macOS equivalent of `libc` on Linux, and more — it bundles libdispatch, libc, libm, and dozens of other subsystems into a single dylib.

A more complex binary — say, a Cocoa application — might list a dozen or more:

```
LC_LOAD_DYLIB  /usr/lib/libSystem.B.dylib
LC_LOAD_DYLIB  /System/Library/Frameworks/Foundation.framework/Foundation
LC_LOAD_DYLIB  /System/Library/Frameworks/AppKit.framework/AppKit
LC_LOAD_DYLIB  /System/Library/Frameworks/CoreFoundation.framework/CoreFoundation
```

Each `LC_LOAD_DYLIB` carries the library's install path, compatibility version, and current version:

```rust
pub struct DylibRef {
    pub name: String,
    pub compatibility_version: u32,
    pub current_version: u32,
    pub timestamp: u32,
}
```

The install path is how the dynamic linker (`dyld`) finds the library at runtime. Paths starting with `/usr/lib/` or `/System/Library/` point into the dyld shared cache — a pre-linked mega-file containing all system frameworks. Paths containing `@rpath` or `@loader_path` are relative to the binary's location, which lets app bundles carry their own libraries.

There are also `LC_LOAD_WEAK_DYLIB` (the library is optional — don't crash if it's missing), `LC_REEXPORT_DYLIB` (this library's symbols are re-exported through me), and `LC_LAZY_LOAD_DYLIB` (defer loading until first use). Same structure, different semantics. Our codebase handles all of them with a unified `DylibRef`:

```rust
pub enum LoadCommandPayload {
    LoadDylib(DylibRef),
    LoadWeakDylib(DylibRef),
    ReexportDylib(DylibRef),
    LazyLoadDylib(DylibRef),
    LoadUpwardDylib(DylibRef),
    IdDylib(DylibRef),
    // ...
}
```

The ordinal matters. The first `LC_LOAD_DYLIB` in the binary is ordinal 1. The second is ordinal 2. When a symbol table entry says "resolve `_exit` from ordinal 1," it means "find `_exit` in the first dylib listed." This is the two-level namespace at work — the binary specifies not just *what* it needs, but *where to find it*.

Here's the key insight: the binary knows what it needs. It carries a manifest of every library dependency, versioned and ordered. When we build our loader, we won't need to guess — we'll read the manifest and resolve each one. Chapter 3 is entirely about this process.

### Entry Point: `LC_MAIN`

Where does execution start? The `LC_MAIN` command:

```
LC_MAIN  entry_offset=0x3f94  stack_size=0
```

```rust
// From LoadCommandPayload
Main {
    entry_offset: u64,
    stack_size: u64,
},
```

The `entry_offset` is relative to the start of the `__TEXT` segment. Add it to `__TEXT`'s `vmaddr` and you have the entry point address. For our binary: `0x100000000 + 0x3f94 = 0x100003f94`. That's where `main()` lives (or rather, where the CRT startup code calls `main` after setting up the process environment).

The `stack_size` of zero means "use the system default" — typically 8 MB on macOS. If a binary needs more stack (a deeply recursive parser, a game engine with heavy stack allocation), the developer sets this at link time, and it shows up here.

There's an older mechanism too: `LC_UNIXTHREAD`, which carries a raw CPU thread state — all the register values to set before execution begins. The key register is `pc` (the program counter on ARM64) or `rip` (on x86_64). Older macOS binaries use this instead of `LC_MAIN`. The difference is semantic: `LC_MAIN` specifies an offset that the C runtime (`_start`) uses, while `LC_UNIXTHREAD` specifies an absolute address where execution begins directly. Our parser normalizes both:

```rust
pub enum EntryPoint {
    Main { entry_offset: u64, stack_size: u64 },
    UnixThread { entry_address: u64 },
}
```

Why does this matter for us? Because when our loader finishes setting up memory, resolving symbols, and initializing the runtime, it needs to jump to the right address. `LC_MAIN` says "add this offset to `__TEXT`'s vmaddr." `LC_UNIXTHREAD` says "jump to exactly this address." Two mechanisms, one goal: start running code.

### Other Load Commands

The remaining load commands provide supplementary information. Each one adds another facet to the binary's self-portrait:

- **`LC_UUID`** — a 16-byte UUID uniquely identifying this build of the binary. Two builds of the same source code at different times get different UUIDs. Crash reporters use this to match core dumps to debug symbols.
- **`LC_BUILD_VERSION`** — the platform (macOS, iOS, tvOS, etc.) and minimum OS version. Apple uses this to prevent running an iOS binary on macOS or a macOS 14 binary on macOS 13. Our parser decodes the platform integer to a human-readable name:

```rust
pub fn platform_name(platform: u32) -> &'static str {
    match platform {
        1 => "macOS",
        2 => "iOS",
        3 => "tvOS",
        4 => "watchOS",
        6 => "macCatalyst",
        // ...
    }
}
```

- **`LC_SOURCE_VERSION`** — the source version from the build system, as a packed 64-bit integer.
- **`LC_SYMTAB`** — offsets to the symbol table and string table in `__LINKEDIT`. We'll dive into this in Scene 1.5.
- **`LC_DYSYMTAB`** — dynamic symbol table partitioning info. Tells the linker where local, external, and undefined symbols start and how large the indirect symbol table is.
- **`LC_CODE_SIGNATURE`** — offset and size of the code signature in `__LINKEDIT`. On macOS, every binary is required to be signed (even if only ad-hoc). The code signature covers the entire binary — modify a single byte and the signature check fails.
- **`LC_DYLD_CHAINED_FIXUPS`** — the modern fixup format introduced in macOS 12. This replaced the older `LC_DYLD_INFO_ONLY` command with a more compact encoding. We'll implement it in Chapter 3.
- **`LC_DYLD_EXPORTS_TRIE`** — a compressed trie structure for symbol export lookup. Instead of linear-scanning the symbol table, `dyld` walks this trie in O(n) time relative to symbol name length.
- **`LC_LOAD_DYLINKER`** — path to the dynamic linker. Almost always `/usr/lib/dyld`. This is Apple's dynamic linker, the native macOS loader. We're building its equivalent.

That last one is worth pausing on. The binary carries the path to its own loader. It's not just data — it's a protocol specification. "Load me with `/usr/lib/dyld`. I expect these libraries, this entry point, these fixups, this runtime." When we build our own loader, we're substituting ourselves for `dyld`. The binary doesn't care who loads it, as long as the protocol is honored.

Each load command is a field in the binary's self-description. The binary doesn't just contain code — it contains a conversation between itself and whatever loads it. "Here's my code. Here are my dependencies. Here's where to start. Here's how to verify I haven't been tampered with. Here's the runtime I expect."

When we build our loader, we're implementing our side of that conversation.

---

## Scene 1.4: The Sections

Segments are coarse containers. Sections are the fine-grained structure inside them. If a segment is a chapter, a section is a paragraph. A segment like `__TEXT` might contain five or six sections, each holding a different kind of content — executable code, jump stubs, unwinding tables, C string literals. Sections are where the real content lives, and understanding them is understanding *what the binary actually contains*.

The segment-section hierarchy is one of Mach-O's distinguishing features compared to ELF. In ELF, segments and sections are two independent views of the same file: the segment view is for the loader, the section view is for the linker and debugger. They can disagree with each other. In Mach-O, sections are subordinate to segments — each section lives inside exactly one segment, defined as part of the segment's load command. There's no ambiguity. The hierarchy is enforced by the format.

Here's what `/usr/bin/true`'s `__TEXT` segment contains:

```
Segment __TEXT (vmaddr=0x100000000, vmsize=0x4000, prot=r-x)
  __text         addr=0x100003f94  size=0x2c    offset=0x3f94  (S_REGULAR)
  __stubs        addr=0x100003fc0  size=0x24    offset=0x3fc0  (S_SYMBOL_STUBS)
  __stub_helper  addr=0x100003fe4  size=0x3c    offset=0x3fe4  (S_REGULAR)
  __unwind_info  addr=0x100004020  size=0x48    offset=0x4020  (S_REGULAR)
```

Each section is defined by this structure:

```rust
pub struct Section {
    pub sectname: String,    // e.g., "__text"
    pub segname: String,     // e.g., "__TEXT"
    pub addr: u64,           // virtual address
    pub size: u64,           // size in bytes
    pub offset: u32,         // file offset
    pub align: u32,          // alignment (power of 2)
    pub reloff: u32,         // relocation entries offset
    pub nreloc: u32,         // number of relocation entries
    pub flags: u32,          // type and attributes
    pub reserved1: u32,      // indirect symbol table index
    pub reserved2: u32,      // stub size (for __stubs)
}
```

Let's look at the most important sections and what they contain.

### `__TEXT/__text` — The Code

This is where the compiled machine code lives. For `/usr/bin/true`, it's 44 bytes (0x2c). The shortest meaningful program on your Mac is only 44 bytes of actual machine code. Let's look at it:

```
00003f94  ff 43 00 d1  f4 4f 01 a9  fd 7b 02 a9  fd 83 00 91  |.C...O...{......|
00003fa4  e0 03 1f 2a  97 ff ff 97  00 00 80 52  94 ff ff 97  |...*......R.....|
00003fb4  e0 03 1f 2a  00 00 00 14                            |...*....|
```

Those are ARM64 instructions. Each instruction is exactly four bytes (the beauty of a fixed-width ISA). `ff 43 00 d1` is `sub sp, sp, #16` — setting up a stack frame. `f4 4f 01 a9` is `stp x20, x19, [sp, #16]` — saving callee-saved registers. `00 00 80 52` is `mov w0, #0` — loading the return value zero.

The function literally does nothing useful, and even that takes a stack frame setup, register saves, a call to `_setlocale`, a zero load, and a call to `_exit`. The simplest possible behavior still requires ceremony. Every C program goes through a startup sequence, and this is what it looks like at the instruction level.

Let's decode a few of these instructions to see the pattern:

```
00003f94: ff 43 00 d1    sub   sp, sp, #0x10      ; allocate 16 bytes on stack
00003f98: f4 4f 01 a9    stp   x20, x19, [sp,#16] ; save callee-saved registers
00003f9c: fd 7b 02 a9    stp   x29, x30, [sp,#32] ; save frame pointer and link register
00003fa0: fd 83 00 91    add   x29, sp, #0x20      ; set up frame pointer
00003fa4: e0 03 1f 2a    mov   w0, wzr             ; w0 = 0 (first argument)
00003fa8: 97 ff ff 97    bl    _setlocale_stub     ; call _setlocale(0, ...)
00003fac: 00 00 80 52    mov   w0, #0              ; w0 = 0 (exit code)
00003fb0: 94 ff ff 97    bl    _exit_stub          ; call _exit(0)
```

That `bl` instruction at offset `0x3fa8` branches to a stub in `__stubs` — the stub loads a GOT pointer and jumps through it. If the GOT entry is zero (not resolved yet), the binary crashes. That's exactly the SIGSEGV we saw in Scene 1.2 — the binary called `_setlocale` through a stub, the stub dereferenced a null GOT entry, and the CPU faulted.

We can't fully interpret these instructions in our loader yet (that's Chapter 7, where we build an ARM64 interpreter), but we can see them. The binary's code isn't hidden. It's sitting at a known offset, at a known address, with a known size. The section header tells us exactly where to find it.

### `__TEXT/__stubs` and `__TEXT/__stub_helper`

These sections implement *lazy binding* — a mechanism where function pointers to external libraries aren't resolved until the function is actually called. The idea is simple: most binaries link against dozens of functions but only call a handful during any given execution. Why resolve all of them eagerly when you can defer the work?

Each entry in `__stubs` is a small trampoline: jump through a pointer in the GOT (Global Offset Table). On ARM64, each stub is just three instructions:

```asm
adrp    x16, GOT_page        ; load the page address of the GOT entry
ldr     x16, [x16, GOT_off]  ; load the function pointer from the GOT
br      x16                   ; branch to whatever the GOT points to
```

If the pointer has been resolved, `x16` holds the real function address and execution continues there. If it hasn't been resolved yet, the GOT entry points to the corresponding `__stub_helper` entry, which pushes a lazy-bind offset, calls `dyld_stub_binder`, and lets the dynamic linker resolve the symbol, patch the GOT entry with the real address, and jump to the function. The next call through the same stub goes straight to the target — the GOT entry has been patched in place.

For `/usr/bin/true`:

```
__stubs (6 entries × 12 bytes each = 72 bytes):
  _exit
  _fprintf
  _fwrite
  _perror
  _setlocale
  _strerror
```

Six external functions, including `_exit`. Even the simplest binary has dependencies. When we build our loader, these stubs are the mechanism we need to intercept — either by pre-resolving all symbols (which is what we do: we fill in the GOT before the binary starts executing, making all stubs resolve immediately) or by implementing our own lazy binding.

The `reserved1` field of a `__stubs` section stores the starting index into the indirect symbol table. The `reserved2` field stores the stub size (typically 12 bytes per stub on ARM64 — three 4-byte instructions). These "reserved" fields aren't reserved at all — they're load-time metadata with specific meanings that depend on the section type. Apple named them `reserved` back when the format was first designed, and the name stuck even after every byte was spoken for. It's a minor lesson in the archaeology of specifications: field names outlive their accuracy.

### `__TEXT/__unwind_info` — Exception Handling Metadata

There's one more section in our `__TEXT` segment worth mentioning: `__unwind_info`. This compact section encodes the unwinding rules for every function — how to restore registers and walk the stack during exception handling or crash reporting. On macOS, both C++ exceptions and Objective-C exceptions use this information. The DWARF-based `.eh_frame` equivalent exists on Linux; macOS uses its own compressed format.

We don't need to *execute* unwind info in our loader (unless the binary throws exceptions), but we do need to map it correctly. If this section is corrupted or missing, crash reporters can't produce useful backtraces, and exception handling may fail silently.

The `reserved1` field of a `__stubs` section stores the starting index into the indirect symbol table, and `reserved2` stores the stub size (typically 12 bytes on ARM64). These "reserved" fields aren't reserved at all — they're load-time metadata with specific meanings that depend on the section type.

### `__DATA_CONST/__got` — The Global Offset Table

```
Segment __DATA_CONST (vmaddr=0x100004000, vmsize=0x4000, prot=rw-)
  __got          addr=0x100004000  size=0x30    offset=0x4000  (S_NON_LAZY_SYMBOL_POINTERS)
```

The segment is called `__DATA_CONST` — it starts out writable (so the loader can fill in function pointers), but after fixups are applied, the kernel can rprotect it to read-only. This is a security hardening measure: once the GOT is filled in, there's no reason for it to remain writable. Any attempt to overwrite a GOT entry after initialization (as a code injection attack might try) would trigger a protection fault.

The GOT holds pointers to external symbols. Before the dynamic linker runs, each entry is zero (or contains a fixup instruction encoded in the pointer itself — more on that in Chapter 3). After linking, each entry holds the actual address of the target function. When `__stubs` code does `ldr x16, [got_entry]; br x16`, it's reading a resolved pointer from the GOT.

Let's visualize this with actual hex. Here's what the GOT looks like before our loader touches it, and after:

```
Before (raw file data):
100004000: 00 00 00 00 00 00 00 00   ← _exit: unresolved (zero)
100004008: 00 00 00 00 00 00 00 00   ← _fprintf: unresolved
100004010: 00 00 00 00 00 00 00 00   ← _fwrite: unresolved

After (loader resolves symbols):
100004000: c0 e5 3b b7 ff ff 00 00   ← _exit → 0xffffb73be5c0
100004008: 40 12 2c b7 ff ff 00 00   ← _fprintf → 0xffffb72c1240
100004010: 20 11 2c b7 ff ff 00 00   ← _fwrite → 0xffffb72c1120
```

Each 8-byte slot transforms from empty to a live function pointer. That transformation is the heart of Chapter 3. But the slots themselves — their addresses, their order, which symbols they correspond to — all of that is specified right here in the section headers and the indirect symbol table.

The GOT is where Chapters 2 and 3 converge: Chapter 2 maps the segment into memory at the right address. Chapter 3 fills in the pointers with resolved function addresses. Without both, the binary crashes — which is exactly what happens in our first real loading attempt, and why we go through the work of understanding both.

### `__DATA/__objc_classlist` and Friends

Larger binaries — anything that uses Objective-C or Swift — have a constellation of metadata sections in `__DATA`:

```
__DATA/__objc_classlist     — pointers to ObjC class descriptors
__DATA/__objc_catlist       — category metadata
__DATA/__objc_protolist     — protocol metadata
__DATA/__objc_selrefs       — selector name references (C strings)
__DATA/__objc_classrefs     — class reference pointers
__DATA/__objc_imageinfo     — ObjC version and flags
```

These sections don't contain code. They contain *metadata* — arrays of pointers and structures that the Objective-C runtime reads during process initialization. When `dyld` finishes mapping and fixing up a binary, it calls `_objc_init` in the ObjC runtime. The runtime then walks these sections to register every class, category, and protocol. This is the `map_images` phase — a callback that receives pointers to all loaded Mach-O images and processes their ObjC metadata.

In Chapter 4, we'll build a shim Objective-C runtime that does the same thing. Our `map_images` reads `__objc_classlist`, creates synthetic class structures in our runtime, and registers method implementations. Every selector in `__objc_selrefs` gets uniqued. Every class in `__objc_classlist` gets registered. It's a substantial piece of work, and it all starts from these section pointers.

For `/usr/bin/true`, these sections don't exist. It's a pure C program with no Objective-C involvement. But try a binary that uses Foundation:

```
$ macho-inspect /usr/bin/plutil
Segment __DATA
  __objc_classlist      24 bytes (3 class pointers)
  __objc_selrefs        128 bytes (16 selector references)
  __objc_methnames      432 bytes
```

Three classes, sixteen selectors. The binary carries its entire Objective-C class hierarchy in these sections, waiting for a runtime to bring it to life. A full Cocoa application might have hundreds of classes and thousands of selectors — all encoded as section data, all discoverable by any tool that can read Mach-O.

### `__LINKEDIT` — The Backstage

The `__LINKEDIT` segment is unlike the others. It isn't mapped for direct program use — it's consumed by the loader. It contains:

- The symbol table (`nlist_64` entries)
- The string table (null-terminated C strings for symbol names)
- The code signature (hash pages and certificates)
- Chained fixup data (binding and rebase information)
- The export trie (compressed symbol export table)
- Function start addresses (offsets for each function entry point)
- Data-in-code markers (for constant data embedded in code sections)

Its contents aren't addressed by section structures in the traditional way. Instead, various load commands (`LC_SYMTAB`, `LC_CODE_SIGNATURE`, `LC_DYLD_CHAINED_FIXUPS`, `LC_DYLD_EXPORTS_TRIE`) point into `__LINKEDIT` using file offsets and sizes. It's a shared data pool, and the load commands are its index.

You can think of `__LINKEDIT` as the binary's appendix — supplementary material that supports the main content. Or think of the binary as a building. `__TEXT` is the public area — the offices and conference rooms where work happens. `__DATA` is the storage room — mutable state, configuration, metadata that the program reads and writes. `__LINKEDIT` is the mechanical floor — wiring, plumbing, HVAC infrastructure. You don't see it during normal operation, but without it, nothing works. The building inspector (`codesign`) checks it. The contractors (the loader) use it to connect everything. The tenants (the program) never touch it directly.

### The Section Type System

One last detail. The `flags` field of each section encodes a *type* in its low eight bits. We parse this in our `SectionType` enum:

```rust
pub enum SectionType {
    Regular,                      // S_REGULAR — normal section
    ZeroFill,                     // S_ZEROFILL — zero-filled on demand
    CstringLiterals,              // S_CSTRING_LITERALS — C string constants
    NonLazySymbolPointers,        // S_NON_LAZY_SYMBOL_POINTERS — GOT
    LazySymbolPointers,           // S_LAZY_SYMBOL_POINTERS — lazy GOT
    SymbolStubs,                  // S_SYMBOL_STUBS — jump stubs
    ModInitFuncPointers,          // S_MOD_INIT_FUNC_POINTERS — initializers
    ThreadLocalRegular,           // S_THREAD_LOCAL_REGULAR — TLS data
    ThreadLocalVariables,         // S_THREAD_LOCAL_VARIABLES — TLS descriptors
    // ... and more
}
```

The type determines how the loader treats the section. `S_ZEROFILL` sections have no file backing — the loader allocates zeroed memory. `S_NON_LAZY_SYMBOL_POINTERS` sections are GOT entries that must be resolved before execution starts. `S_MOD_INIT_FUNC_POINTERS` sections contain function pointers that the loader calls during initialization — these are the `__attribute__((constructor))` functions in C, and they run before `main()`.

The section type system is how the binary communicates fine-grained loading requirements without custom load commands for every case. The loader reads the type, knows the protocol, and acts accordingly.

The section structure gives us a complete map. We know where every byte in the file lives, what segment it belongs to, what purpose it serves, and how the loader should handle it. A 16 KB binary becomes transparent. Complexity is manageable when you see the partitioning. Every byte has a designated place.

---

## Scene 1.5: The Symbol Table

Every Mach-O binary carries a phone book. It's called the symbol table, and it lists every function and global variable the binary defines or needs. Understanding the symbol table is understanding the binary's contract with the outside world — the complete enumeration of what it provides and what it requires.

If the header says "I am an ARM64 executable," and the load commands say "map these segments and load these libraries," the symbol table says "here are the specific functions I need from those libraries, by name." It's the most granular level of the binary's self-description.

The symbol table lives in `__LINKEDIT`, pointed to by the `LC_SYMTAB` load command:

```
LC_SYMTAB  sym_off=0xc0a0  nsyms=24  str_off=0xc260  str_size=0x128
```

Four values: the file offset and count of symbol entries, and the file offset and size of the string table. Every symbol name is stored as a null-terminated C string in the string table. Each symbol entry references its name by byte offset into that table.

A symbol entry is an `nlist_64` structure — twelve bytes on 64-bit:

```
struct nlist_64 {
    n_strx:  u32,    // string table index (byte offset)
    n_type:  u8,     // type flags
    n_sect:  u8,     // section number (1-based, 0 = NO_SECT)
    n_desc:  u16,    // description field (library ordinal, flags)
    n_value: u64,    // value (address for defined symbols)
}
```

Our Rust representation captures the semantics rather than the raw bytes:

```rust
pub struct Symbol {
    pub name: String,
    pub sym_type: SymbolType,
    pub section_index: u8,
    pub desc: u16,
    pub value: u64,
}

pub enum SymbolType {
    Undefined,          // needs resolution from a dylib
    Absolute,           // not relative to any section
    Defined,            // defined in a section of this binary
    PreboundUndefined,  // rare — pre-bound
    Indirect,           // indirect symbol
    Unknown(u8),
}
```

The `n_type` byte is packed with meaning. Bits 1-3 (the `N_TYPE` mask, `0x0E`) tell you the symbol classification. Bit 0 (`N_EXT`, `0x01`) marks the symbol as external — visible outside this binary. The high bits contain debugger information (STAB entries). A single byte encodes three different classification schemes overlaid on each other. Welcome to 1980s systems programming.

```rust
impl SymbolType {
    pub fn from_ntype(n_type: u8) -> Self {
        let masked = n_type & 0x0E;
        match masked {
            0x00 => Self::Undefined,   // N_UNDF
            0x02 => Self::Absolute,    // N_ABS
            0x0E => Self::Defined,     // N_SECT
            0x0C => Self::PreboundUndefined,
            0x0A => Self::Indirect,
            _ => Self::Unknown(masked),
        }
    }

    pub fn is_external(n_type: u8) -> bool {
        n_type & 0x01 != 0
    }
}
```

Let's classify the symbols in a real binary. Run `nm` on `/usr/bin/true`:

```
                 U _exit
                 U _fprintf
                 U _fwrite
                 U _perror
                 U _setlocale
                 U _strerror
0000000100003f94 T _main
```

`U` means undefined — these symbols exist somewhere else, and the binary needs them at runtime. `T` means defined in the `__TEXT` segment. `_main` is the only function this binary defines. Everything else it borrows.

Look at that list. Six undefined symbols. Six open questions the binary is asking: "Where is `_exit`? Where is `_fprintf`? Where is `_strerror`?" Those questions are answered during linking — either by `dyld` at load time on macOS, or by our loader on Linux.

But here's what's interesting: even this trivial binary needs `_setlocale`. Not just `_exit` — it needs locale configuration. Not just `_fprintf` — it needs `_fwrite`, `_perror`, `_strerror`. The C runtime initialization code calls `_setlocale` before `main()` runs. Error handling paths (which `true` never executes) reference `_fprintf`, `_perror`, and `_strerror`. The compiler and linker included them because they appear in the source code, regardless of whether they're reachable at runtime.

This is a pattern you'll see again and again in this book: binaries carry more dependencies than they strictly need. Dead code elimination removes unused *functions*, but not unused *references* from live functions. If a function calls `perror()` in an error path that never triggers, the symbol reference still exists, and our loader still has to resolve it.

### The String Table

Let's look at how symbol names are actually stored. The string table is a blob of null-terminated C strings concatenated together:

```
Offset      Content
0x0000      \0                    ← empty string (null symbol name)
0x0001      _main\0
0x0007      _exit\0
0x000d      _fprintf\0
0x0016      _fwrite\0
0x001e      _perror\0
0x0025      _setlocale\0
0x0030      _strerror\0
```

Each `nlist_64` entry's `n_strx` field is a byte offset into this table. If `n_strx` is 7, the name starts at byte 7 of the string table — `_exit`. The null byte marks the end of each name. This is the most space-efficient encoding possible: no length prefixes, no alignment padding, just raw strings packed together.

The leading underscore on every symbol (`_main`, `_exit`) is a C convention inherited from older systems where the compiler prepended an underscore to distinguish C symbols from assembler symbols. macOS Mach-O preserves this convention — every C function name in the symbol table has a leading underscore. Our loader needs to account for this when matching macOS symbol names to Linux function addresses. When the binary asks for `_exit`, we need to resolve it to the Linux `exit` function (no underscore). A simple `strip_prefix("_")` in our symbol resolver handles this mismatch.

### Defined vs. Undefined: The Binary's Self-Knowledge

The symbol table partitions the binary's world into things it knows and things it needs. Defined symbols (type `N_SECT`, displayed as `T` or `t` in `nm` output) are addresses within this binary's sections — functions and data it contains. Undefined symbols (type `N_UNDF`, displayed as `U`) are external references — names the binary mentions but can't resolve on its own.

This partition is the binary's way of being honest about its limits. `/usr/bin/true` defines `_main` and that's it. It acknowledges that it can't exit without `_exit`, can't format strings without `_fprintf`, can't handle errors without `_strerror`. Each undefined symbol is an admission: "I can't do this alone."

For our loader, undefined symbols become a to-do list. Every `U` in the symbol table is a function we need to provide. How we provide it — by linking to a real dylib, by shimming it with our own implementation, or by wiring it to the host Linux system's libc — depends on the symbol. `_exit` maps cleanly to Linux's `exit()`. `_setlocale` maps with some glue code. `_fprintf` passes through unchanged. But some symbols — `_objc_msgSend`, `_dispatch_async`, `_CFStringCreateWithCString` — have no Linux equivalent. For those, we build our own implementations. Chapters 4 through 6 are about exactly this.

### Library Ordinals and Two-Level Namespaces

The `n_desc` field carries additional metadata. For undefined symbols in a two-level namespace binary (the `MH_TWO_LEVEL` flag we saw in the header), bits 8-15 encode a *library ordinal* — an index into the binary's `LC_LOAD_DYLIB` list that specifies *which library* should provide this symbol:

| Ordinal | Meaning |
|---|---|
| 1 | First `LC_LOAD_DYLIB` |
| 2 | Second `LC_LOAD_DYLIB` |
| 0 | Self (symbol from this binary) |
| -1 | Main executable |
| -2 | Flat namespace (search all) |
| -3 | Dynamic lookup (weak) |

```rust
impl Symbol {
    pub fn library_ordinal(&self) -> i16 {
        ((self.desc >> 8) & 0xFF) as i8 as i16
    }
}
```

Two-level namespaces solve a real problem. Imagine two libraries each define a function called `_init_module`. Without two-level namespaces, the linker picks whichever one it finds first — fragile and order-dependent. With two-level namespaces, the binary specifies: "I want `_exit` from ordinal 1, which is `libSystem.B.dylib`." No ambiguity. No surprises. The binary knows *exactly* which library provides each symbol.

The `n_desc` field also carries weak-reference flags (`REFERENCED_DYNAMICALLY`, `N_WEAK_REF`, `N_WEAK_DEF`). A weak reference can be null at runtime — the loader resolves it if the symbol exists and leaves it zero if it doesn't. A weak definition can be overridden by a non-weak definition from another library. These flags matter in our loader when we handle optional framework symbols.

### The Indirect Symbol Table

Beyond the main symbol table, Mach-O has an **indirect symbol table** — an array of `uint32` values that map GOT and stub entries back to the main symbol table. It's pointed to by `LC_DYSYMTAB`:

```
LC_DYSYMTAB
  ilocalsym=0    nlocalsym=8
  iextdefsym=8   nextdefsym=4
  iundefsym=12   nundefsym=6
  indirectsymoff=0xc098  nindirectsyms=12
```

This tells us the main symbol table is partitioned into three ranges: local symbols first (indices 0-7), then external defined symbols (8-11), then undefined symbols (12-17). The partitioning eliminates linear scans — if you need to find undefined symbols, jump straight to index `iundefsym`.

The indirect symbol table is an array of main-symbol-table indices. Entry 0 of the GOT section maps to indirect entry `reserved1 + 0` (where `reserved1` is the section's starting index in the indirect table). That indirect entry contains a symbol table index. That index points to an `nlist_64` entry. That entry's `n_strx` field points into the string table. It's an indirection chain:

```
GOT slot [N]  →  indirect_symbols[section.reserved1 + N]
              →  symbol_table[indirect_index]
              →  string_table[nlist.n_strx]
              →  "_exit"
```

Why the extra indirection? Because the GOT slots are ordered by the binary's code layout, not by symbol name. The indirect symbol table bridges the spatial layout (where things are in memory) to the logical layout (what they're called). When our loader fills in GOT entry N, it follows this chain to find the target symbol name, resolves that name against available libraries, and writes the address into the GOT slot.

We parse all of this into a clean Rust structure:

```rust
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
}
```

Twenty-four symbols in `/usr/bin/true`. Eight local (compiler-generated labels and debug info), four externally-defined, six undefined — plus twelve indirect entries for the GOT and stubs. A complete dependency manifest for a program that does almost nothing.

### The Binary's Incompleteness

The symbol table reveals something fundamental about how binaries work: they are incomplete by design. No binary is self-contained. Every one declares what it needs from the outside world — functions to call, data to access, libraries to load. `_exit`, `_fprintf`, `_strerror` — these are promises yet to be fulfilled. The symbol table is a list of unresolved questions, and the loader's job is to answer every one.

Think of it this way: the binary is one side of a handshake. It extends its hand — "I need `_exit` from `libSystem.B.dylib`" — and waits for someone to grip it. On macOS, `dyld` completes the handshake. On our Linux loader, we complete it. The protocol is the same. The implementation is ours.

In Chapter 3, we'll walk through symbol resolution in detail — chained fixups, the export trie, ordinal-based lookup, the whole machinery. For now, understand this: the symbol table is the contract between the binary and its environment. When we build a loader for Linux, we need to honor that contract — provide every symbol, at the right address, through the right mechanism. If we miss even one, the binary crashes with a SIGSEGV at the exact instruction that tried to use the missing symbol. And the crash will tell us exactly which one we missed.

What makes this manageable is that the contract is explicit. The binary doesn't have hidden dependencies. It doesn't call functions that aren't listed. Every requirement is enumerated in the symbol table, ordered in the indirect symbol table, and attributed to a specific library via the ordinal. We know, before we run a single instruction, *exactly* what the binary expects. The challenge isn't discovery — it's implementation.

And that's the difference between fear and anticipation. If the dependencies were implicit — buried in code paths, only discoverable at runtime — we'd be debugging blind. Instead, we have a manifest. A complete parts list. The symbol table is a gift. It tells us everything we need to know to make this binary run, and we haven't even executed it yet.

---

## Scene 1.6: Building the Parser

We've walked through a Mach-O binary by hand — magic number, header, load commands, segments, sections, symbols. Now let's turn that understanding into code.

The `macho-parse` crate is the foundation of our entire project. Every other crate in the system — the mapper, the linker, the runtime — depends on it. If the parser gets something wrong, everything built on top of it is wrong. So we build it carefully.

### The Design: Zero-Copy Where Possible

Our parser takes a `&[u8]` — a borrowed byte slice — and returns a `ParsedMachO` struct containing everything we've discussed:

```rust
pub struct ParsedMachO {
    pub header: MachHeader,
    pub commands: Vec<LoadCommand>,
    pub segments: Vec<Segment>,
    pub sections: Vec<Section>,
    pub dylibs: Vec<DylibRef>,
    pub rpaths: Vec<String>,
    pub entry: Option<EntryPoint>,
    pub symbols: Option<SymbolTable>,
    pub uuid: Option<[u8; 16]>,
    pub code_signature: Option<CodeSignature>,
}
```

This struct is the parsed form of everything we explored in this chapter. The header, the load commands with their payloads, the segments and their sections, the dependency list, the entry point, and the symbol table. One call to `parse()` and the entire binary is accessible as typed Rust data.

The top-level function:

```rust
pub fn parse(data: &[u8]) -> Result<ParsedMachO, ParseError> {
    let mut r = Reader::new(data);
    let header = r.read_header()?;

    let mut commands = Vec::with_capacity(header.num_commands as usize);
    let mut segments = Vec::new();
    let mut dylibs = Vec::new();
    let mut entry = None;
    let mut symbols = None;
    // ... more collectors

    for _ in 0..header.num_commands {
        let cmd_start = r.pos;
        let cmd_raw = r.read_u32()?;
        let cmd_size = r.read_u32()?;

        if cmd_size < 8 {
            return Err(ParseError::LoadCommandTooSmall {
                size: cmd_size,
                min: 8,
            });
        }

        let cmd_type = LoadCommandType::from(cmd_raw);
        let payload = match cmd_type {
            LoadCommandType::Segment64 => {
                let seg = r.read_segment_64()?;
                segments.push(seg.clone());
                LoadCommandPayload::Segment(seg)
            }
            LoadCommandType::LoadDylib => {
                let dref = r.read_dylib_ref(cmd_start)?;
                dylibs.push(dref.clone());
                LoadCommandPayload::LoadDylib(dref)
            }
            LoadCommandType::Main => {
                let entry_offset = r.read_u64()?;
                let stack_size = r.read_u64()?;
                entry = Some(EntryPoint::Main {
                    entry_offset,
                    stack_size,
                });
                LoadCommandPayload::Main { entry_offset, stack_size }
            }
            // ... 20+ more command types
            _ => {
                let remaining = cmd_size as usize - 8;
                let bytes = r.read_bytes(remaining)?;
                LoadCommandPayload::Unknown(bytes.to_vec())
            }
        };

        commands.push(LoadCommand {
            cmd: cmd_type,
            cmd_size,
            payload,
        });
    }

    Ok(ParsedMachO {
        header,
        commands,
        segments,
        // ...
    })
}
```

The `Reader` is a simple cursor over the byte slice. It tracks a position, reads little-endian integers, and returns `ParseError` when it hits unexpected data. No allocator tricks, no unsafe pointer casts, no memory-mapped I/O magic. Just sequential reads from a byte slice. Here's the core idea:

```rust
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn read_u32(&mut self) -> Result<u32, ParseError> {
        if self.pos + 4 > self.data.len() {
            return Err(ParseError::UnexpectedEof {
                offset: self.pos,
                needed: 4,
                available: self.data.len() - self.pos,
            });
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }
}
```

Read four bytes. Interpret them as a little-endian `u32`. Advance the cursor. If the slice runs short, return an error with exact context. That's the entire reading mechanism. `read_u64` works the same way with eight bytes. `read_bytes` returns a raw slice. The header is seven `read_u32` calls and one conditional `read_u32` for the 64-bit reserved field.

The real merit of this approach becomes clear when you compare it to the alternative: casting a raw pointer to a `*const mach_header_64` and reading fields through the pointer. That's what C code does. It's fast and compact. It's also undefined behavior if the data isn't properly aligned, it doesn't handle truncated files (you get a segfault instead of an error), and it can't report which field was missing. Our approach is slightly slower but vastly safer. Since we parse once and then work with the typed result, the parsing overhead is negligible.

The `Unknown` variant in the payload enum is essential. When the parser encounters a load command type it doesn't recognize, it stashes the raw bytes and moves on:

```rust
_ => {
    let remaining = cmd_size as usize - 8;
    let bytes = r.read_bytes(remaining)?;
    LoadCommandPayload::Unknown(bytes.to_vec())
}
```

This makes the parser forward-compatible. A binary from a future macOS version with new load commands will still parse; we just won't understand the new stuff. Unknown commands are preserved, not rejected. If someone adds a new load command `LC_ARM64_PAC_INFO` in 2027, our parser from 2025 will still produce a complete `ParsedMachO` — with that command stored as `Unknown` bytes. No crash. No data loss.

### Validation at the Boundary

We validate early and specifically:

```rust
pub enum ParseError {
    BadMagic { magic: u32 },

    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },

    LoadCommandTooSmall { size: u32, min: u32 },

    TooManySections { count: u32 },

    InvalidUtf8 {
        context: &'static str,
        source: std::str::Utf8Error,
    },

    Other(String),
}
```

`BadMagic` catches non-Mach-O files immediately — before wasting time parsing gibberish. Give us a JPEG, a PDF, a random file — we'll return `not a Mach-O file (bad magic: 0xffd8ffe0)` and stop. `UnexpectedEof` catches truncated files with precise context: "I was at offset 1024, needed 8 bytes, but only 4 were available." This exact error message saved us hours during development when Docker volume mounts silently truncated files. `LoadCommandTooSmall` catches corrupted command sizes before they send the parser into infinite loops — a command that claims to be 4 bytes long can't hold the 8-byte `cmd`/`cmdsize` prefix, so something is wrong. `TooManySections` catches the case where a segment claims to have millions of sections, which would exhaust memory before parsing anything useful. Every error tells you exactly what went wrong and where.

Notice what we *don't* validate: we don't check that `vmaddr` is page-aligned (some binaries from older toolchains aren't), we don't verify that section sizes fit within their segment (the kernel handles that at mapping time), and we don't validate the code signature (that's a separate operation with its own verification path). Validation happens at system boundaries — where untrusted input enters our code. After parsing, the data is typed and checked. We don't re-validate it every time we read a field.

This philosophy — *validate at the boundary, trust the interior* — runs through the entire codebase. The parser is the boundary between raw bytes and typed data. After `parse()` returns, you have a `ParsedMachO` that is guaranteed to be internally consistent: every segment index is valid, every string is valid UTF-8 (or we returned `InvalidUtf8`), every load command is at least as large as its prefix. Downstream code never needs to check "is this a valid Mach-O?" The parser already answered that question.

### The Type System as Documentation

Look at the `LoadCommandPayload` enum:

```rust
pub enum LoadCommandPayload {
    Segment(Segment),
    Symtab { sym_off: u32, nsyms: u32, str_off: u32, str_size: u32 },
    LoadDylib(DylibRef),
    Main { entry_offset: u64, stack_size: u64 },
    Uuid { uuid: [u8; 16] },
    BuildVersion { platform: u32, minos: u32, sdk: u32, ntools: u32 },
    LinkEdit { data_off: u32, data_size: u32 },
    // ... more variants
    Unknown(Vec<u8>),
}
```

Each variant captures precisely the data that command type carries. You can't accidentally read the `entry_offset` of a `Segment` or the `vmaddr` of a `Uuid`. The Rust compiler enforces correctness through the type system. Pattern match on the payload, and you know exactly what fields are available.

This is a deliberate design choice. We could have stored everything as a bag of `u32` values with string keys — a `HashMap<String, u32>` for each load command. It would be more flexible. It would also be wrong — every consumer would need to validate field names, handle missing keys, cast types, check for typos. By encoding the structure in Rust's type system, we push all that validation into the parser, once, and every downstream consumer gets clean, typed data for free.

The compiler becomes our co-pilot. If someone writes a function that processes `LC_MAIN` entries, the function receives `entry_offset: u64` and `stack_size: u64` — not "some fields that might be there." If they accidentally try to read `vmaddr` from a `Main` payload, the compiler stops them. This is the same principle that makes Rust's `Option<T>` better than null pointers: impossible states are unrepresentable.

### Utility Functions

The parser also provides helper functions that other crates will use heavily. One of the most important converts virtual memory addresses to file offsets:

```rust
pub fn vmaddr_to_file_offset(parsed: &ParsedMachO, vmaddr: u64) -> Option<usize> {
    for seg in &parsed.segments {
        if vmaddr >= seg.vmaddr && vmaddr < seg.vmaddr + seg.vmsize {
            let offset_in_seg = vmaddr - seg.vmaddr;
            if offset_in_seg < seg.filesize {
                return Some((seg.fileoff + offset_in_seg) as usize);
            }
        }
    }
    None
}
```

When you find a pointer in the binary — say, a class reference in `__objc_classlist` — the pointer is a virtual address (e.g., `0x100008120`). But you haven't mapped the binary into memory yet; you're working with the raw file data. To read what that pointer points to, you need to convert the virtual address back to a file offset. This function walks the segment table, finds which segment contains the address, and computes the file offset. We use it everywhere — symbol resolution, ObjC metadata parsing, fixup processing.

### Running the Parser

Let's see it in action. The `macho-inspect` binary is a thin CLI wrapper around `macho-parse`. It calls `parse()`, then formats the result for human consumption. Run it on `/usr/bin/true`:

```
$ macho-inspect --info /usr/bin/true
Mach-O 64-bit Executable
  CPU:        ARM64 (ALL)
  File type:  Execute
  Commands:   16 (1232 bytes)
  Flags:      NO_UNDEFS | DYLDLINK | TWO_LEVEL | PIE
  UUID:       a1b2c3d4-e5f6-7890-abcd-ef1234567890

Segments:
  __PAGEZERO     vmaddr=0x000000000  vmsize=0x100000000  prot=---
  __TEXT          vmaddr=0x100000000  vmsize=0x4000       prot=r-x
  __DATA_CONST   vmaddr=0x100004000  vmsize=0x4000       prot=rw-
  __DATA          vmaddr=0x100008000  vmsize=0x4000       prot=rw-
  __LINKEDIT     vmaddr=0x10000c000  vmsize=0x4000       prot=r--

Dylibs:
  /usr/lib/libSystem.B.dylib (compat 1.0.0, current 1319.0.0)

Entry point: offset 0x3f94 (LC_MAIN)

Symbols: 24 total (8 local, 4 external, 6 undefined)
  Undefined:
    _exit              (libSystem.B.dylib)
    _fprintf           (libSystem.B.dylib)
    _fwrite            (libSystem.B.dylib)
    _perror            (libSystem.B.dylib)
    _setlocale         (libSystem.B.dylib)
    _strerror          (libSystem.B.dylib)
```

From an opaque black box to structured, human-readable output. One function call. Every field we discussed in this chapter is right there — the magic number resolved to "Mach-O 64-bit Executable," the CPU type expanded to "ARM64 (ALL)," the flags decoded to named constants, the segments listed with their addresses and permissions, the dylibs displayed with version strings, and the undefined symbols annotated with their library ordinals.

Look at that output and compare it to the hex dump from Scene 1.1:

```
00000000  cf fa ed fe 0c 00 00 01  00 00 00 00 02 00 00 00  |................|
```

Same information. Completely different level of understanding. The hex dump was cryptic — sixteen indistinguishable bytes. The parsed output is transparent — "ARM64 executable with 16 commands, PIE enabled, depends on libSystem." You can see why learning to parse matters. The hex dump is where we started. The parsed output is where we are now.

Run it on a more complex binary and you'll see the format scale gracefully:

```
$ macho-inspect --info /usr/bin/plutil
Mach-O 64-bit Executable
  CPU:        ARM64 (ALL)
  File type:  Execute
  Commands:   24 (2368 bytes)
  Flags:      NO_UNDEFS | DYLDLINK | TWO_LEVEL | PIE

Segments:
  __PAGEZERO     vmaddr=0x000000000  vmsize=0x100000000  prot=---
  __TEXT          vmaddr=0x100000000  vmsize=0x8000       prot=r-x
  __DATA_CONST   vmaddr=0x100008000  vmsize=0x4000       prot=rw-
  __DATA          vmaddr=0x10000c000  vmsize=0x4000       prot=rw-
  __LINKEDIT     vmaddr=0x100010000  vmsize=0x8000       prot=r--

Dylibs:
  /usr/lib/libSystem.B.dylib (compat 1.0.0, current 1319.0.0)
  /System/Library/Frameworks/Foundation.framework/Foundation (compat 300.0.0)
  /System/Library/Frameworks/CoreFoundation.framework/CoreFoundation (compat 150.0.0)

Entry point: offset 0x3e44 (LC_MAIN)

Symbols: 147 total (89 local, 12 external, 46 undefined)
```

More commands, more dylibs, more symbols — but the same structure. The same parser handles them all. A 20 KB utility and a 50 MB application go through the same code path. The format scales because it was designed to: the header tells you how many commands to expect, each command tells you how big it is, and the segments tell you where everything lives. Self-describing all the way down.

### What We Built

Let's take inventory. In this chapter, we built a mental model and a concrete tool:

The **mental model**: A Mach-O binary is a self-describing artifact. It starts with a magic number (`0xFEEDFACF`) and a header that declares its architecture and format. After the header, an array of load commands specifies how to load it — which segments to map, which libraries to link, where to start executing. Within the segments, sections organize the actual content: code in `__text`, function stubs in `__stubs`, metadata in `__objc_classlist`, linking data in `__LINKEDIT`. The symbol table declares the binary's external dependencies — every function it needs from the outside world.

The **concrete tool**: `macho-parse`, a Rust crate that reads a byte slice and produces a typed `ParsedMachO` struct. The parser handles 20+ load command types, validates at input boundaries, preserves unknown commands for forward compatibility, and produces clean errors when things go wrong. `macho-inspect` gives us a CLI interface to this parser — point it at any Mach-O file and get a complete structural summary.

That's the power of understanding the format. Once you know what the bytes mean, parsing is mechanical. The magic number leads to the header. The header leads to the load commands. The load commands lead to the segments, the sections, the symbols, the dependencies. Each layer references the next. The binary is self-describing, and our parser speaks its language.

### What Comes Next

But parsing isn't loading. We can read the binary's every wish, itemize its every dependency, map its entire structure — and it still can't run. Right now, the binary's contents are sitting on the heap in a `Vec<u8>`. The CPU can't execute code from there. The binary says it wants `__TEXT` mapped at `0x100000000` with read+execute permissions. It says it wants `__DATA` at `0x100008000` with read-write permissions. It says it wants `_exit` to be a live function pointer.

We've read the architect's blueprints. Now we need to build the house.

In Chapter 2, we take the parsed `Segment` descriptions and call `mmap` to map each one into virtual memory at the right address, with the right permissions, from the right file offset. We'll see the binary's code at its intended addresses for the first time. We'll encounter ASLR slides — the difference between where the binary *wants* to live and where we *put* it — and learn to relocate cleanly. We'll deal with page alignment, the 16 KB page size on ARM64, and the subtleties of mapping `__PAGEZERO` (spoiler: don't).

And we'll discover that even perfectly mapped code crashes instantly — because the GOT entries are all zeros, and every function call through a stub dereferences null.

The parser gave us understanding. The mapper gives us a home. The linker, in Chapter 3, gives us connections.

We can read the binary. Now we need to give it somewhere to live.
