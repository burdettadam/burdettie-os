# Appendix B: The War Stories

*A catalog of undocumented behaviors discovered while building a macOS binary loader on Linux.*

---

There is a sentence that recurs throughout this book like a motif: *documentation describes intent; binaries describe reality*. Nowhere is that sentence more painfully true than in this appendix. What follows is a collection of secrets — behaviors that we discovered not by reading specifications, but by watching things crash.

Each entry tells the same story structure. First, a symptom: a crash, a wrong output, a silent corruption. Then an investigation: hours of disassembly, printf debugging, strace output, hexdump comparisons. Then a revelation: the moment when the actual behavior becomes clear, and the reason it was never documented becomes obvious. Finally, a fix: the code that makes the loader handle reality instead of the specification's abstract ideal.

These are the things we learned the hard way. Some of them will save you weeks. Some of them will make you question whether documentation has any relationship to implementation at all. All of them are true, verified by running code, as of macOS 15 / Xcode 26.

---

## Scene B.1: ABI Secrets

*Callbacks to Chapter 6: The Variadic Crisis.*

### Secret 1: Apple ARM64 Variadic ABI — The Silent Divergence

The crash report was terse: `SIGSEGV` in `strlen`, address `0x2`. The backtrace showed `fprintf` calling `vfprintf`, which passed a `%s` format argument to the output machinery, which called `strlen` on what it thought was a string pointer. The pointer was the integer 2.

We were running `wc` — a tool that counts lines, words, and characters. It had been compiled by Apple's Clang for macOS ARM64, and we were executing it on Linux ARM64 through the loader. The binary was valid. The fixups were correct. The symbol resolution was correct. `fprintf` resolved to glibc's `fprintf`. The format string was valid. The `FILE*` pointer was valid. The string argument — the filename — was a valid heap-allocated C string. And yet `fprintf` was reading garbage from register `x2` instead of the filename from the stack.

The investigation took two hours and one disassembly comparison. We compiled a trivial `fprintf` caller on macOS:

```c
fprintf(stderr, "file: %s size: %d\n", filename, size);
```

And compared the generated assembly on macOS versus Linux. On Linux AAPCS64:

```asm
; Linux: variadic args go in registers first
mov x0, x19        ; stderr
adr x1, .Lformat   ; format string
mov x2, x20        ; filename (variadic arg 1 — in register)
mov w3, w21        ; size     (variadic arg 2 — in register)
bl  fprintf
```

On macOS:

```asm
; Apple: ALL variadic args go on the stack
mov x0, x19         ; stderr
adr x1, .Lformat    ; format string
str x20, [sp]       ; filename (variadic arg 1 — ON STACK)
str w21, [sp, #8]   ; size     (variadic arg 2 — ON STACK)
bl  fprintf
```

There it was. Apple's ARM64 ABI places *all* variadic arguments on the stack, starting at `[sp]`. The Linux AAPCS64 standard places the first variadic arguments in registers (`x2`-`x7` for general-purpose, `q0`-`q7` for floating-point), only spilling to the stack when registers are exhausted.

This divergence is documented nowhere in Apple's public ABI documentation. Apple's "ARM64 Function Calling Conventions" document describes the non-variadic calling convention in detail. It mentions that variadic functions exist. It does not say "we put everything on the stack." We found this by reading assembly. The specification IS the implementation.

The consequence is catastrophic for a cross-platform loader. When a macOS binary calls `fprintf`, it puts the format arguments on the stack. When Linux's `fprintf` executes, its `va_arg` macro reads registers — `x2`, `x3`, `x4` — which contain whatever stale values happened to be there from the previous function call. For `%d`, this produces wrong numbers. For `%s`, it produces a crash, because `strlen` dereferences whatever garbage value is in the register as a pointer.

The fix requires 33 naked assembly shims. For each variadic C function (`fprintf`, `printf`, `snprintf`, `sprintf`, `dprintf`, `asprintf`, `sscanf`, `fscanf`, `scanf`, `syslog`, and others), we install a shim that constructs a Linux `va_list` struct — a 32-byte structure — and redirects to the corresponding `v`-variant function (`vfprintf`, `vprintf`, etc.).

The trick is in the `va_list` struct construction:

```rust
/// `int fprintf(FILE *stream, const char *fmt, ...)`
///   → `vfprintf(stream, fmt, &va_list)`
/// Named args: x0=stream, x1=fmt  ⇒ va_list in x2
#[unsafe(naked)]
pub extern "C" fn shim_fprintf() -> libc::c_int {
    core::arch::naked_asm!(
        "stp  x29, x30, [sp, #-64]!",
        "mov  x29, sp",
        "add  x8, x29, #64",          // x8 = original SP (Apple args)
        "str  x8, [sp, #16]",          // __stack = x8
        "stp  xzr, xzr, [sp, #24]",   // __gr_top = 0, __vr_top = 0
        "str  xzr, [sp, #40]",         // __gr_offs = 0, __vr_offs = 0
        "add  x2, sp, #16",            // x2 = &va_list
        "bl   {v_func}",
        "ldp  x29, x30, [sp], #64",
        "ret",
        v_func = sym vfprintf,
    )
}
```

The layout of the Linux aarch64 `va_list` struct is:

| Offset | Field       | Size | Value we set              |
|--------|-------------|------|---------------------------|
| 0      | `__stack`   | 8    | original SP (Apple args)  |
| 8      | `__gr_top`  | 8    | 0 (unused)                |
| 16     | `__vr_top`  | 8    | 0 (unused)                |
| 24     | `__gr_offs` | 4    | 0 (no GP regs left)       |
| 28     | `__vr_offs` | 4    | 0 (no FP regs left)       |

By setting `__gr_offs` and `__vr_offs` to zero, we tell Linux's `va_arg` implementation: "there are no register-saved values — go straight to the stack." And `__stack` points to the original stack pointer before we pushed our frame — exactly where Apple placed the variadic arguments. The shim creates a Linux-shaped window into Apple's stack layout.

The register holding `&va_list` varies by function signature. `printf` has one named argument (`fmt` in `x0`), so `va_list` goes in `x1`. `fprintf` has two (`stream` in `x0`, `fmt` in `x1`), so `va_list` goes in `x2`. `snprintf` has three (`buf`, `size`, `fmt`), so `va_list` goes in `x3`. Each shim is hand-coded for its calling convention.

But there was a second class of incompatibility hiding inside the `v`-variant functions themselves.

On Apple ARM64, `va_list` is defined as `char *` — a simple 8-byte pointer to the stack. On Linux aarch64, `va_list` is the 32-byte struct described above. When a macOS binary calls `vfprintf` directly, it passes a `char*` in the `va_list` argument register. Linux's `vfprintf` expects a pointer to a `__va_list` struct. It reads 32 bytes from that address. It gets 8 bytes of valid pointer and 24 bytes of whatever happens to be next on the stack. Chaos.

So we need a second set of shims for the `v`-variant functions:

```rust
/// `int vfprintf(FILE *stream, const char *fmt, va_list ap)`
/// Apple: x2 = char* (Apple va_list — just a stack pointer)
/// Linux: x2 = __va_list* (pointer to 32-byte struct)
#[unsafe(naked)]
pub extern "C" fn shim_vfprintf() -> libc::c_int {
    core::arch::naked_asm!(
        "stp  x29, x30, [sp, #-64]!",
        "mov  x29, sp",
        "str  x2, [sp, #16]",          // __stack = Apple va_list
        "stp  xzr, xzr, [sp, #24]",   // __gr_top = 0, __vr_top = 0
        "str  xzr, [sp, #40]",         // __gr_offs = 0, __vr_offs = 0
        "add  x2, sp, #16",            // x2 = &linux_va_list
        "bl   {v_func}",
        "ldp  x29, x30, [sp], #64",
        "ret",
        v_func = sym vfprintf,
    )
}
```

The pattern is the same — wrap the Apple `char*` in a Linux `va_list` struct — but the source of the stack pointer is different. For the variadic-to-v shims, we capture the original SP. For the v-to-v shims, we read the Apple `va_list` value from the argument register.

In total, we install 33 shims: 15 for variadic functions (`fprintf`, `printf`, `snprintf`, `sprintf`, `dprintf`, `asprintf`, `sscanf`, `fscanf`, `scanf`, `err`, `errx`, `errc`, `warn`, `warnx`, `syslog`), 10 for v-variant functions (`vfprintf`, `vprintf`, `vsnprintf`, `vsprintf`, `vdprintf`, `vasprintf`, `vsscanf`, `vfscanf`, `vscanf`, `vsyslog`), and 8 for fortified `_chk` variants (`__snprintf_chk`, `__sprintf_chk`, `__fprintf_chk`, `__printf_chk`, and their v-variant counterparts).

After installing these shims, `wc` printed correct output. `sort` stopped crashing on locale-formatted numbers. `find` stopped producing garbled filenames. `stat` stopped printing garbage timestamps. The variadic ABI mismatch had been silently corrupting output of every tool that used `printf`-family functions — which is to say, every tool.

### Secret 2: Struct Return ABI — The Register Threshold Game

The struct return convention is simpler than the variadic mess but equally underdocumented in its exact rules. On ARM64, the rule for how a struct is returned depends on its size and composition:

- Structs 16 bytes or smaller are returned in registers: `x0`/`x1` for integer-only structs, `d0`/`d1` for float-only structs, or a mix for mixed structs.
- Structs larger than 16 bytes are returned via an *implicit pointer* in `x8`. The caller allocates space, puts the address in `x8`, the callee writes the result there, and `x8` is preserved across the call (it's not a normal argument register).

The `x8` convention is consistent between Apple and Linux — both use it. But the edge cases in what counts as "small" and how registers are assigned for composite types differ subtly. A struct with two `float` fields (8 bytes total) returns in `d0` and `d1` on both platforms. A struct with one `float` and one `int` (8 bytes total) returns in `d0` and `x0` — but only if the struct meets the Homogeneous Floating-Point Aggregate rules, which depend on the exact definition of "homogeneous." The C standard says nothing. Apple's ABI documentation says "up to four elements." The AAPCS64 says "up to four elements of the same type." In practice, both implementations agree for the common cases, because both are implemented by the same compiler (LLVM/Clang). The divergence shows up in corner cases with anonymous unions and bit-fields that we haven't encountered in real Mach-O binaries — yet.

We discovered the `x8` convention the hard way when our `objc_msgSend` trampoline failed to preserve `x8`:

```asm
; Before fix: x8 was clobbered by trampoline scratch usage
; After fix: save and restore x8 across the trampoline
stp  x8, x9, [sp, #-16]!    ; save x8 (indirect result location)
; ... dispatch logic ...
ldp  x8, x9, [sp], #16      ; restore x8 before calling IMP
```

The trampoline was using `x8` as a scratch register for method lookup. When the found method returned a large struct (like `CGRect`, 32 bytes), it wrote the result to wherever `x8` pointed — which was now somewhere in our trampoline's stack frame instead of the caller's allocated buffer. Memory corruption. Silent. Intermittent. Excruciating to debug.

The lesson: on ARM64, `x8` is not a general-purpose scratch register. It's the indirect result location register, and if you clobber it, every function that returns a struct larger than 16 bytes will corrupt memory.

We encountered this with `CGRect` (32 bytes: four `CGFloat` values for x, y, width, height) returned from methods like `-[NSValue rectValue]` and `-[NSView frame]`. The caller allocates 32 bytes on its stack, puts the address in `x8`, and calls the method. The method writes four doubles into `*x8`. If `x8` has been redirected, the four doubles land somewhere else — often on a live stack frame — and when the caller returns, the saved frame pointer or return address has been overwritten with a floating-point coordinate. The resulting crash is in a completely unrelated function, at a completely unrelated time, with a backtrace that makes no sense until you realize the memory corruption happened three calls ago.

The fix was simple once diagnosed — save `x8` on entry to the trampoline, restore it before calling the target IMP — but the debugging session that found it was one of the longest of the project. The symptom was "random crashes in Foundation code after calling any AppKit method that returns a rect." The root cause was one missing register save in assembly.

There's a further subtlety about the threshold between register and indirect return that varies by struct composition. A struct containing exactly two `double` fields (16 bytes) returns in `d0` and `d1`. A struct containing one `double` and one `long` (16 bytes) returns in `d0` and `x0` — mixed register return. A struct containing three of anything (24 bytes) goes via `x8`. A struct containing four `float` fields (16 bytes) *also* returns in registers as a Homogeneous Floating-Point Aggregate via `d0`-`d3` (each promoted to double-width). The rules are in the AAPCS64 specification, sections 5.4 and 5.5, and Apple follows them identically — this is one of the few areas where the two ABIs actually agree completely, thanks to both being implemented by LLVM's calling convention lowering. The Apple divergence is strictly in variadic argument passing, not in return value conventions.

---

## Scene B.2: Binary Format Secrets

*Callbacks to Chapter 3: The Links.*

### Secret 1: Chained Fixup Pointer Formats — Five Dialects of the Same Language

Apple's chained fixup mechanism uses `LC_DYLD_CHAINED_FIXUPS` to encode both rebases (adjusting pointers for ASLR) and binds (resolving external symbols) into linked chains embedded directly in the `__DATA` segment. The concept is elegant: each 64-bit slot in the data segment contains either the final pointer value or an encoded entry that says "this is a rebase/bind, and the next entry in the chain is N bytes away."

What the documentation doesn't adequately convey is that there are five distinct pointer format dialects, each with different bit layouts, different chain stride sizes, and different semantics:

```rust
pub const DYLD_CHAINED_PTR_ARM64E: u16 = 1;          // stride 8
pub const DYLD_CHAINED_PTR_64: u16 = 2;               // stride 4
pub const DYLD_CHAINED_PTR_64_OFFSET: u16 = 6;        // stride 4
pub const DYLD_CHAINED_PTR_ARM64E_USERLAND: u16 = 12; // stride 8
pub const DYLD_CHAINED_PTR_ARM64E_USERLAND24: u16 = 13; // stride 8
```

Format 2 (`DYLD_CHAINED_PTR_64`) was the first we implemented. The bit layout is clean: bit 63 selects bind vs. rebase, the lower bits encode the target value or ordinal, and bits 51-61 encode the `next` field — the delta to the next chain entry, in units of 4 bytes. This format works for standard (non-arm64e) binaries.

Format 6 (`DYLD_CHAINED_PTR_64_OFFSET`) is identical in structure but the rebase target is an *offset from the preferred load address* rather than an absolute virtual address. The difference is one line of code — `target + preferred_base` vs. `target` — but getting it wrong means every rebased pointer is off by the image's base address, typically `0x100000000`. The symptoms are spectacular: every method call jumps to an address 4GB away from where the code actually lives.

Formats 1, 12, and 13 are the ARM64e variants, and they introduced three bugs in our implementation that each took a full debugging session to find.

The first bug was in the `next` field extraction for authenticated entries. Apple's `<mach-o/loader.h>` header shows the bit layout for `DYLD_CHAINED_PTR_ARM64E` format (pointer format 1). For authenticated entries (bit 63 = 1), the header appears to suggest the `next` field is in bits 32-42. We implemented that. Everything crashed. The actual `next` field is in bits 51-61 — the same position as in non-authenticated entries. We found this by reading Apple's open-source `dyld` implementation. The header comments and the code disagreed. The code was right.

The second bug was in format 12 (`DYLD_CHAINED_PTR_ARM64E_USERLAND`) rebases. The rebase target in this format is an *offset from the preferred base address*, not an absolute virtual address. But we were treating it as absolute — the same behavior as format 1. The result: every rebased pointer was off by `preferred_base`, which for most binaries is `0x100000000`. Pointers into the first few pages of the address space. Immediate crash on dereference.

The third bug was in the `AuthMetadata` bit fields for PAC (Pointer Authentication Code) signing. We had the key field at bits 43-44 and the `addrDiv` flag at bit 45. The correct positions are key at bits 49-50 and `addrDiv` at bit 48. A 6-bit shift. The PAC signature was being computed with the wrong key type and the wrong discriminator blending, producing pointers that failed authentication on arm64e hardware.

```rust
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

    loop {
        if offset + 8 > seg.size { break; }
        let raw = unsafe {
            let ptr = seg.base.add(offset) as *const u64;
            ptr.read_unaligned()
        };

        match pointer_format {
            1 | 12 | 13 => {
                let is_auth = (raw >> 63) & 1 == 1;
                let is_bind = (raw >> 62) & 1 == 1;
                // ... dispatch to auth/non-auth, bind/rebase handlers
            }
            2 | 6 => {
                let is_bind = (raw >> 63) & 1 == 1;
                // ... simpler non-arm64e handling
            }
            _ => break,
        }

        // Next field is ALWAYS bits 51-61, regardless of auth/bind
        let next = ((raw >> 51) & 0x7FF) as usize;
        if next == 0 { break; }
        offset += next * stride;
    }
}
```

There's one more trap in the page-level structure: the `page_starts` array (which tells you where the first chain entry lives on each page) can contain the sentinel value `DYLD_CHAINED_PTR_START_NONE` (`0xFFFF`), meaning "this page has no fixups, skip it." If you don't check for the sentinel and try to use `0xFFFF` as an offset, you'll walk off the end of the page into whatever data follows. The sentinel is documented in the header, but it's easy to miss among the many other constants.

The overall lesson: reading `dyld` source code (which Apple publishes as open-source at `opensource.apple.com/source/dyld/`) was essential. Apple's Mach-O header documentation defines the data structures but leaves ambiguous how the walk actually works in practice — which bit fields to use for which operations, which formats use offsets vs. absolute addresses, what the sentinel values mean. The reference implementation is the specification.

We ultimately found that the critical debugging technique for chained fixups was printing the raw 64-bit value at each chain slot alongside our decoded interpretation. When the decoded interpretation produced crashes, we could compare our bit extraction against `dyld`'s source code and find the divergence. The ARM64e auth key bug, for instance, was invisible in the decoded output — the key looked like a valid key value (0, 1, 2, or 3) — but it was the *wrong* key for the pointer being authenticated. Only by comparing our PAC signature against the one that `dyld` would compute could we see that our key extraction was shifted by 6 bits. The bug was silent at static analysis level: both the wrong key and the right key produce syntactically valid pointers. Only the authentication check at runtime reveals the difference — and the authentication check produces `SIGBUS`, not a helpful error message.

### Secret 2: Binary Plist — The Undocumented Trailer

Property list files come in two formats: XML (human-readable, starts with `<?xml`) and binary (compact, starts with `bplist00`). We needed to parse both because macOS uses them everywhere — every `.app` bundle has an `Info.plist`, every settings file is a plist, and Xcode's own metadata files are plists.

The XML format is well-documented. The binary format is not.

Binary plists start with the 8-byte magic `bplist00`. The actual object data fills the middle of the file. The critical metadata — the information you need to decode anything — is in the *last 32 bytes* of the file: the trailer.

```rust
let format = if slice.starts_with(b"bplist00") {
    PLIST_BINARY_FORMAT   // 200
} else {
    100                   // XML
};
```

The trailer contains: the size of each offset table entry (1, 2, 4, or 8 bytes), the size of each object reference (1, 2, 4, or 8 bytes), the number of objects in the file, the index of the top (root) object, and the file offset of the offset table. Without the trailer, you cannot decode a single object in the file — the offset table is the index, and its entry size is variable.

This format is not in any Apple documentation that we found. We reverse-engineered it from two sources: Apple's open-source CoreFoundation (`CFBinaryPlist.c`), and the `plist` Rust crate that had done the same reverse-engineering before us. The format hasn't changed since OS X 10.2, but Apple has never published a specification.

The discovery mattered because `xcodebuild -version` was printing nothing. The loader ran `xcodebuild`, which loaded DVTFoundation, which called `DVTSystemPrerequisites`, which read `CFBundleShortVersionString` from Xcode's `Info.plist`. That `Info.plist` is binary format. Our XML-only parser returned nil. DVTSystemPrerequisites got nil. `xcodebuild` printed an empty version string. After adding binary plist support: `Xcode 26.4 / Build version 17E192`.

The binary plist format has a charming quirk that tripped us up during implementation. Object references within the plist (e.g., dictionary keys referencing string objects) use 1, 2, 4, or 8 bytes depending on the *total number of objects in the file*, and this reference size is stored in the trailer. A plist with fewer than 256 objects uses 1-byte references. A plist with 256-65535 objects uses 2-byte references. The offset table entries have a *separate* width, also in the trailer, determined by the maximum file offset. So the same data can be encoded with 1-byte object references and 4-byte offset entries, or 2-byte references and 8-byte offsets — you have to read the trailer to know which combination to expect. Hardcoding either width produces a parser that works on small plists and silently misparses large ones, or vice versa.

We also found that Apple's binary plist writer can produce files where the top-level object is not object 0 — the root object index in the trailer can point to any object in the offset table. Our initial parser assumed the root object was always at index 0 and produced incorrect results for plists where a dictionary was embedded inside a containing array and the array was the root. The fix was one line: read the root object index from the trailer instead of hardcoding 0.

### Secret 3: The Shared Dylib Cache — A Continent of Concatenated Libraries

On macOS, most system libraries do not exist as individual files on disk. They exist as a single monolithic blob: the *dyld shared cache*, located at `/System/Library/dyld/dyld_shared_cache_arm64e` (or the corresponding architecture variant). This file concatenates Foundation, libSystem, AppKit, CoreFoundation, Security, and hundreds of other system frameworks into one memory-mappable image.

The cache starts with a 16-byte magic string that encodes the architecture:

```rust
const MAGIC_ARM64E: &[u8; 16] = b"dyld_v1  arm64e\0";
const MAGIC_ARM64: &[u8; 16]  = b"dyld_v1   arm64\0";
const MAGIC_X86_64: &[u8; 16] = b"dyld_v1  x86_64\0";
```

Note the whitespace padding: `arm64e` has two spaces, `arm64` has three spaces, `x86_64` has two spaces (and there's an `x86_64h` variant with one space for Haswell-optimized builds). The architecture name is right-justified in a 7-character field after `dyld_v1`. This string format is undocumented outside the `dyld` source code.

The cache header contains mapping information — virtual address, file offset, size, memory protection flags — for several memory regions. Extracting a single dylib from the cache requires translating virtual addresses (which the dylib's Mach-O metadata uses) to file offsets (which you need to read the actual bytes) through the mapping table. The mapping is not a simple base+offset calculation, because the cache has multiple non-contiguous regions with different base addresses.

```rust
pub struct MappingInfo {
    pub address: u64,     // VM address where this region is mapped
    pub size: u64,        // region size in bytes
    pub file_offset: u64, // offset in the cache file
    pub max_prot: u32,    // maximum memory protection
    pub init_prot: u32,   // initial memory protection
}
```

The real complication is slide info — the data structure that enables ASLR (Address Space Layout Randomization) for shared cache pages. When the cache is mapped at a different address than its preferred base, every internal pointer must be adjusted. The slide info tells `dyld` which pages contain pointers and how to adjust them. This structure's format varies by cache version and is documented exclusively in `dyld` source code. We parse enough of the cache to extract individual dylib images and their metadata, but we do not reimplement the full slide info machinery — that's `dyld`'s job.

The shared cache matters for the loader because when we parse the dependency list of any Xcode tool, the install names reference dylibs that exist only inside the cache. We need to either extract them or know enough about their structure to skip them and resolve their symbols from the host system's loaded libraries instead. The cache is the reality. Individual `.dylib` files are the abstraction.

---

## Scene B.3: Runtime Secrets

*Callbacks to Chapter 4: The Runtime.*

### Secret 1: Tagged Pointers — Objects That Aren't Objects

The crash was in `objc_msgSend`. The receiver pointer was `0xE000000000000042`. We tried to read the ISA (the class pointer at byte 0 of every ObjC object). The memory at `0xE000000000000042` was not mapped. Segfault.

But the program that produced this pointer was correct. It was a Foundation method that returned an `NSNumber` wrapping the integer `66` (ASCII `'B'`). On Apple's runtime, `NSNumber` objects that hold small values are not heap-allocated at all. The "object" IS the pointer. This optimization is called *tagged pointers*.

On ARM64, a tagged pointer is identified by bit 63 being set. Bits 60-62 encode the *tag class index* — a 3-bit value that identifies which class this tagged pointer belongs to:

| Tag | Class     | Encoding |
|-----|-----------|----------|
| 0   | Reserved  | —        |
| 1   | Reserved  | —        |
| 2   | NSNumber  | Value encoded in lower bits |
| 3   | NSString  | Up to 11 bytes of string data encoded in the pointer |
| 4   | NSIndexPath | Path components encoded in the pointer |
| 5   | Reserved  | —        |
| 6   | NSDate    | Timestamp encoded in lower bits |

The remaining bits (0-59, minus the tag bits) encode the object's payload. For `NSNumber`, the payload is the numeric value. For `NSString`, the payload contains up to 11 bytes of UTF-8 string data — the string literal, the length, and the encoding indicator, all packed into a 64-bit word. No heap allocation. No reference counting. No ISA dereference.

This means `objc_msgSend` must check for tagged pointers *before* attempting to dereference the ISA. If bit 63 is set, the pointer doesn't point to valid memory — it IS the object:

```rust
pub fn tagged_class_for_pointer(&self, obj: u64) -> Option<u64> {
    // ARM64 tagged pointer: bits 60-62 = tag class index
    let tag = ((obj >> 60) & 0x7) as usize;
    if tag < self.tagged_classes.len() && self.tagged_classes[tag] != 0 {
        Some(self.tagged_classes[tag])
    } else {
        None
    }
}
```

Our `objc_msgSend` dispatch path calls `tagged_class_for_pointer` first. If it returns a class address, we dispatch to that class's method table without ever reading memory through the object pointer. If `obj` has bit 63 clear, it's a real heap object and we proceed to the normal ISA read → method lookup path.

The tagged pointer table must be populated during runtime initialization with the class addresses that Apple assigns to each tag slot. Getting these addresses wrong means every `NSNumber` comparison, every short-string operation, every `NSDate` creation will dispatch to the wrong class and either crash or produce wrong results.

The punch line is that tagged pointers are used pervasively. Apple's Foundation framework returns tagged `NSNumber` objects for all small integers and tagged `NSString` objects for any string under 12 bytes. In a typical Foundation-heavy program, the majority of temporary objects are tagged pointers. If your `objc_msgSend` doesn't handle them, you can't run Foundation code. Period.

On x86_64, the story is slightly different: tagged pointers are identified by bit 0 being set (since pointer alignment means the low bit is always zero for real heap addresses). The tag class index is in bits 1-3. Apple chose different sentinel bits for each architecture because the addressing constraints differ — ARM64 can use the high bits freely (they're ignored by hardware via Top Byte Ignore), while x86_64 can't use the high bits (they're sign-extended) but can use the low bits (which are alignment-guaranteed zero). The result is that cross-architecture tagged pointer code requires `#[cfg]` branches everywhere, and any abstraction that tries to handle both architectures must parameterize both the detection check and the tag extraction.

### Secret 2: Non-Pointer ISA — The Mask Behind the Curtain

Even when an object IS a real heap object (bit 63 is clear), the ISA pointer at byte 0 isn't a simple class pointer anymore. Modern ObjC uses a *non-pointer ISA* encoding where additional flags are packed into the ISA word's unused bits.

On a 64-bit system, not all 64 bits of a pointer are needed for addressing. ARM64 uses 48-bit virtual addresses (with the upper 16 bits reserved for kernel/PAC), and x86_64 uses 47 bits (with the upper 17 bits sign-extended). Apple's ObjC runtime exploits these unused bits to store per-object metadata inline in the ISA word:

- Bit 0: non-pointer ISA flag (1 = non-pointer, 0 = raw pointer)
- Bits 1-2: `has_assoc` (has associated objects), `has_cxx_dtor` (needs C++ destructor)
- Bottom 3 bits: alignment padding used by some runtime optimizations
- Upper bits: PAC signature (ARM64e), inline reference count data

The mask that extracts the actual class pointer differs by architecture:

```rust
// ARM64: 48-bit virtual addresses, upper byte used for PAC
let isa = raw_isa & 0x007ffffffffffff8;

// x86_64: 47-bit virtual addresses, sign extension in high bits
let isa = raw_isa & 0x00007ffffffffff8;
```

The ARM64 mask `0x007ffffffffffff8` strips: bit 63 (tagged pointer flag), bits 56-62 (PAC signature on arm64e), the bottom 3 bits (runtime flags). What remains is the 44-bit class address, which is enough to address any class in the process's 48-bit virtual address space.

We discovered this when `objc_msgSend` started crashing on objects allocated by our own runtime. We were writing raw class addresses into the ISA field. The dispatch code was reading the ISA and using it directly as a class pointer. That worked fine — until we loaded a binary that had been linked against a newer SDK that set bit 0 of the ISA to indicate "this is a non-pointer ISA." Suddenly the class pointer was off by one. Method lookup failed. Crash.

The fix was to always apply the ISA mask, even for our own synthetic objects:

```rust
fn unmask_isa(raw_isa: u64) -> u64 {
    if raw_isa & 1 != 0 {
        // Non-pointer ISA: mask to extract class address
        raw_isa & 0x007ffffffffffff8
    } else {
        // Raw pointer ISA (older binaries or synthetic objects)
        raw_isa
    }
}
```

The interaction between tagged pointers and non-pointer ISA creates a subtle priority chain. When dispatching a message to an object:

1. Check bit 63: if set, it's a tagged pointer. Look up the tag class. Do NOT dereference the pointer.
2. If not tagged, read the 8 bytes at the object's address to get the raw ISA.
3. Check bit 0 of the raw ISA: if set, apply the non-pointer ISA mask to extract the class.
4. If bit 0 is clear, use the raw ISA directly as a class pointer.
5. Look up the method on the resulting class.

Getting any step wrong — or getting the order wrong — produces crashes that are extremely difficult to diagnose, because the symptom is "method lookup failed on a valid class" and the root cause is "the class pointer has extra bits that weren't stripped."

### Secret 3: +load Order and the Shared Cache Bootstrap

ObjC classes can implement a `+load` method that is called once, automatically, at class load time — before `main()` runs, before `+initialize` is called, before any explicit message sends. The `+load` mechanism is the ObjC equivalent of C++ static constructors, and like C++ static constructors, the order matters.

Apple's runtime guarantees a specific `+load` order:

1. All classes loaded from the shared cache have their `+load` methods called first, in order: first libSystem, then CoreFoundation, then Foundation, then other frameworks in dependency order.
2. Classes from the executable's own image have their `+load` called next.
3. Within each image, superclass `+load` is called before subclass `+load`.

This ordering is not merely a convention — code depends on it. A `+load` method in a Foundation class might send a message to a CoreFoundation class, assuming that CoreFoundation's `+load` has already run and initialized the class's state. If CoreFoundation's `+load` hasn't run yet — because your loader calls `+load` methods in the wrong order — the class's internal state is uninitialized, and the message produces garbage or crashes.

The distinction between `+load` and `+initialize` compounds the problem:

- `+load` is called *exactly once*, at class load time, in the order described above. It is *not* triggered by sending a message to the class. It is called by the runtime as part of the image loading process.
- `+initialize` is called *lazily*, the first time any message is sent to a class. It is called at most once per class, and the runtime holds a lock to ensure thread safety.

If you confuse the two — calling `+load` lazily, or calling `+initialize` eagerly — you get subtle violations of initialization ordering that manifest as "works sometimes, crashes sometimes" bugs. We initially implemented `+load` as a lazily-triggered mechanism (similar to `+initialize`) and spent three days tracking down crashes in `NSKeyedArchiver` that only happened when `NSKeyedArchiver` was the first Foundation class to receive a message. The fix was to call all `+load` methods during the `map_images` phase, in dependency order, before handing control to the executable.

The shared cache adds another wrinkle. Classes in the cache have their method lists pre-optimized — selectors are pre-uniqued, method lists are pre-sorted for binary search. When we load classes from Mach-O images outside the cache, the selectors in their method lists are *not* uniqued — they're raw string pointers. `objc_msgSend`'s fast path compares selector pointers, not strings. If the selector pointer in the class's method list doesn't match the selector pointer that the caller passed (because one was uniqued and the other wasn't), the fast path misses and falls through to the slow path. The slow path does string comparison and works correctly, but the performance difference is significant for hot methods.

Our loader runs `fixup_selrefs` during the registration phase to unique selectors in loaded images, matching the runtime's behavior for shared-cache images. Without this step, every method call goes through the slow path, and the overhead is measurable — a 2x slowdown in selector-heavy code like `NSString` formatting.

There's a deeper implication of `+load` ordering that we discovered only when running complex tools. Some `+load` methods register notification observers (via `NSNotificationCenter`), set up KVO listeners, or install method swizzles that replace existing method implementations. These side effects create implicit dependencies: class A's `+load` swizzles a method on class B, which only works if class B's `+load` has already registered the original method. Apple's dyld resolves these dependencies implicitly by walking the shared cache in link order — a topological sort of the framework dependency graph. Our loader replicates this by calling `+load` in BFS order based on the `LC_LOAD_DYLIB` dependency edges, which matches the dyld behavior for the common case.

The one case where `+load` ordering still catches us is when two frameworks at the same depth in the dependency graph have an ordering dependency that isn't captured in the `LC_LOAD_DYLIB` chain. Apple's dyld resolves these by the order of images in the shared cache file (which is determined by Apple's build system and is consistent across all machines running the same macOS version). Our loader resolves them by the order we encounter the images during BFS, which may differ. In practice, this affects approximately zero tools we've tested — the cases where intra-level `+load` ordering matters are rare and mostly confined to Apple's own internal frameworks that we don't load.

---

## Scene B.4: Framework Secrets

*Callbacks to Chapter 9: The Tools.*

### Secret 1: libxo — The Library That Nobody Mentions

The crash in `wc` was perplexing. After fixing the variadic ABI issue (Scene B.1), `wc` started executing correctly — it counted lines, words, and characters. Then it tried to print the results. And it called a function we'd never seen: `xo_emit`.

```
TRACE  macho-exec::fixup::resolver: unresolved symbol: _xo_emit
```

`xo_emit` does not appear in any Apple Framework documentation. It's not in the Foundation headers. It's not in the libSystem headers. It's not in any SDK header file we could find. Yet macOS's `wc` binary imports it, and crashes if it's not available.

Investigation revealed that `xo_emit` is part of **libxo** — a structured output library originally from FreeBSD. libxo provides a unified API for producing text, JSON, XML, or HTML output from the same code. Instead of calling `printf("  %7lld", count)`, a libxo-aware program calls `xo_emit("{:lines/%7lld}", count)`, and libxo renders the output in whatever format was requested via command-line flags (`--libxo=json`, etc.).

Apple adopted libxo from FreeBSD and linked it into most system command-line tools: `wc`, `sort`, `df`, `ls`, `ps`, `w`, `netstat`, and others. This is undocumented. Apple ships libxo as a private library, and the tools that use it don't mention it in their man pages. On macOS, the library is present in the shared cache. On Linux, it doesn't exist.

For text mode (the default), most of libxo's API surface can be stubbed to no-ops:

```rust
// libxo is the FreeBSD/macOS structured-output library used by system
// tools like wc, sort, df.  In text mode (the default), we can provide
// minimal stubs that let tools produce their normal text output.

extern "C" fn shim_xo_parse_args(
    argc: libc::c_int,
    _argv: *mut *mut libc::c_char,
) -> libc::c_int {
    argc  // return argc unchanged — no --libxo options to strip
}

extern "C" fn shim_xo_open_container(
    _name: *const libc::c_char,
) -> libc::c_int {
    0  // success, do nothing
}

extern "C" fn shim_xo_close_container(
    _name: *const libc::c_char,
) -> libc::c_int {
    0
}
```

The key insight is that tools using libxo have *two* output paths: the libxo structured path (for JSON/XML output) and a fallback text path. When libxo is in text mode — which it is by default — the text output path produces the familiar `printf`-style output. Our stubs let the tools initialize libxo (in text mode), call `xo_emit` (which we no-op, since the text output goes through the regular `fprintf` path), and call the container/list/instance open/close functions (all no-ops in text mode).

We implemented 29 libxo stubs in total: `xo_parse_args`, `xo_create_to_file`, `xo_create`, `xo_set_style`, `xo_set_flags`, `xo_set_options`, `xo_emit`, `xo_emit_h`, `xo_emit_hv`, `xo_open_container`, `xo_close_container`, `xo_open_list`, `xo_close_list`, `xo_open_instance`, `xo_close_instance`, `xo_finish`, `xo_flush`, `xo_error`, `xo_warn`, `xo_warnx`, `xo_err`, `xo_errx`, and several others. All of them are trivial — return 0, return argc, return the input pointer as a fake handle. But without them, a dozen macOS command-line tools crash on first output.

The philosophical lesson: Apple's system tools are not simple POSIX utilities. They're FreeBSD tools with Apple modifications, linked against Apple-private libraries, compiled with Apple's extended calling conventions. Even `wc` — a tool so simple that its specification fits in one paragraph — carries hidden dependencies that exist nowhere in public documentation.

### Secret 2: Compact Unwind → .eh_frame Synthesis — Translating Exception Metadata

When C++ or ObjC code throws an exception, the runtime needs to unwind the call stack — walking backwards through stack frames, calling destructors, finding `catch` handlers. On most platforms, the metadata that describes each function's stack layout, saved registers, and exception handlers lives in a DWARF `.eh_frame` section.

macOS doesn't use `.eh_frame` for most binaries. It uses Apple's proprietary `__unwind_info` format — also called *compact unwind*. This format is smaller and faster to search than DWARF, because it packs each function's unwind information into a 32-bit encoding value instead of a variable-length DWARF instruction sequence.

The problem is that Linux's exception handling infrastructure (`libgcc_s` or `libunwind`) expects `.eh_frame`. When we load a macOS binary on Linux, the binary has `__unwind_info` but no `.eh_frame`. If an exception is thrown, `libunwind` can't find any unwind metadata for the function, and the exception propagates as an uncaught crash.

The solution is to synthesize a `.eh_frame` section at load time by translating the compact unwind entries:

```rust
pub fn synthesize_runtime_eh_frame(
    segments: &[Segment],
    slide: i64,
) -> Option<Vec<u8>> {
    let sections = collect_unwind_sections(segments, slide)?;
    if sections.eh_frame.is_some() {
        return None;  // already has .eh_frame, skip
    }
    let (addr, len) = sections.compact_unwind?;
    let cu_slice = unsafe {
        std::slice::from_raw_parts(addr as *const u8, len)
    };
    let entries = crate::compact_to_ehframe::parse_compact_unwind(cu_slice);
    if entries.is_empty() { return None; }
    let eh = crate::compact_to_ehframe::generate_eh_frame_runtime(
        &entries, sections.text_start,
    );
    (!eh.is_empty()).then_some(eh)
}
```

Parsing the compact unwind format revealed its own set of undocumented behaviors. The `__unwind_info` section starts with a version word (must be 1), followed by offsets to various internal tables: common encodings, personality functions, and the index. The index is a two-level structure: first-level index entries point to second-level pages, and each second-level page contains per-function entries.

```rust
pub struct CompactEntry {
    pub func_offset: u32,         // function start, relative to __TEXT base
    pub func_length: u32,         // function length in bytes
    pub encoding: u32,            // compact encoding value
    pub personality_slot_offset: Option<u32>,  // personality function slot
    pub lsda_offset: Option<u32>,             // language-specific data area
}
```

Second-level pages come in two flavors: "regular" (each function has its own 12-byte entry with explicit encoding) and "compressed" (functions share encoding values from a per-page table, reducing space). The compact encoding value itself is a 32-bit word that packs the frame size, which registers are saved, and the personality function index into a dense bit format. The encoding scheme varies by architecture — ARM64 uses different bit assignments than x86_64.

The most treacherous detail is the LSDA (Language-Specific Data Area) offset. This is the pointer to the `catch` handler table for the function. Without a valid LSDA offset in the synthesized `.eh_frame`, exceptions will propagate past catch handlers as if they don't exist. The compact unwind format stores LSDA offsets in a separate table indexed by function, and the offset is relative to a different base than the function offset. Getting the base calculation wrong means `catch` blocks silently stop catching.

We verified the synthesis by running our `try_catch_test` fixture — a binary that throws an ObjC exception inside a `@try` block and catches it. Before compact unwind synthesis: uncaught exception, crash. After: caught, clean exit. The 84-fixture compatibility gate includes this test.

The synthesis pipeline has to handle one additional complication: personality functions. A personality function is the runtime callback that decides whether a particular catch handler matches the thrown exception type. ObjC uses `__objc_personality_v0`. C++ uses `__gxx_personality_v0`. The compact unwind format stores personality functions as slots (indices into a small array of personality function pointers in the `__unwind_info` header), not as direct pointers. When we synthesize the `.eh_frame`, we need to resolve each slot to an actual function address and emit it as the CIE (Common Information Entry) personality field. Getting this wrong means the runtime calls the wrong personality function — or address zero — during exception handling. The symptom is "exceptions are thrown but never caught," which is one of the most frustrating debugging experiences possible because the throw site, the catch site, and the exception object itself are all correct. Only the metadata bridge between them is wrong.

We also discovered that some binaries have both `__unwind_info` and `__eh_frame` sections. When both exist, we skip synthesis — the existing `.eh_frame` takes precedence. This check is essential because synthesizing a second `.eh_frame` when one already exists produces duplicate unwind entries, and `libunwind` gets confused trying to unwind through a function that has two conflicting descriptions of its stack layout.

### Secret 3: Private Framework Dependencies — The Xcode Stack

The most ambitious goal for the loader was running Xcode command-line developer tools: `clang`, `lipo`, `swift-demangle`, `strings`, `nm`, `size`, `codesign`. Most of these tools are self-contained — they depend on public system libraries and nothing else. But some Xcode tools depend on private frameworks that exist only inside Xcode's app bundle.

The dependency chain goes deep. When we first tried to run `xcodebuild`, the loader's dependency resolution traced a staggering path. `xcodebuild` links `DVTFoundation.framework` — Apple's internal "Developer Tools Vocabulary" framework that provides Xcode's plugin architecture, build setting evaluation, and workspace management. DVTFoundation isn't part of the public SDK. It's not documented. It's not versioned for external use.

DVTFoundation depends on `IDEFoundation`, which depends on `DVTServices`, which depends on `DVTAnalytics`. IDEFoundation also depends on `DVTCompilation`, `DVTCocoaAdditionsKit`, and `CoreSimulator`. The transitive closure for tools like `momc` (the Core Data model compiler) spans 80+ framework images. Each one has its own class list, its own symbol table, its own assumptions about the runtime environment.

We don't have any of these frameworks on Linux. We can't provide them. And unlike public frameworks — where the behavior is specified and we can implement a compatible shim — private frameworks have no specification. Their APIs change between Xcode versions. Their class names, method selectors, and behavioral contracts are discovered only through reverse engineering.

The shim strategy for tools that depend on private frameworks is damage limitation. We register stub symbols for the most critical entry points:

- `DVTInitialize()` — the framework initialization function. If it doesn't exist, DVTFoundation's `+load` method aborts. Our stub returns immediately.
- `DVTAddFetchedAttributeStrings()` — called during localization setup. Stub returns 0.
- Class symbols (`_OBJC_CLASS_$_DVTDeveloperPaths`, etc.) — resolved to trap addresses in the synthetic data symbol store.

This is enough for some tools to get past initialization. `xcodebuild -version` actually works — it loads DVTFoundation, calls `DVTSystemPrerequisites` to read Xcode's `Info.plist` (through our binary plist parser, per Secret B.2.2), extracts the version string, and prints `Xcode 26.4 / Build version 17E192`. But `xcodebuild -list` or `xcodebuild build` immediately hit methods on classes we haven't implemented, because the tool starts actually *using* DVTFoundation's APIs rather than just initializing it.

The honest engineering conclusion: these tools aren't in scope. The loader runs command-line utilities that stay within the public SDK surface. `clang`, `lipo`, `swift-demangle`, `strings`, `nm`, `size`, `strip`, `codesign`, `vtool` — these tools depend on public libraries and work correctly. `xcodebuild`, `momc`, `notarytool` — these tools depend on private frameworks that require reimplementing Apple's entire IDE infrastructure, and that's a different project.

What the private framework investigation taught us is where the *boundary* of shimming lives. A shim layer works when the binary's expectations can be translated to the host's capabilities. `stat` works because Linux has `stat` — the struct layout differs, but the concept is identical. `malloc` works because Linux has `malloc`. Foundation works because we implemented Foundation. But DVTFoundation has no Linux equivalent. There's no translation. There's only reimplementation or absence.

The 52 tools that pass represent the translateable surface. They use public system calls, public C library functions, public Foundation APIs, and public ObjC runtime behaviors. The 20 tools that crash represent the untranslateable boundary: private frameworks, the Swift runtime bridge, code-signing infrastructure tied to Apple's security hardware. The line between the two is not granularity of effort — it's *kind* of effort. More engineering hours won't make DVTFoundation work on Linux. Only reimplementing DVTFoundation would, and then you'd have a new project, not a loader.

---

## Closing: Documentation Describes Intent; Binaries Describe Reality

These war stories share a common thread. In every case, the official documentation — Apple's ABI reference, the Mach-O header specification, the ObjC runtime documentation, the framework release notes — described an idealized, simplified model of the system. And in every case, the actual binary behavior deviated from that model in ways that mattered.

Apple's ARM64 ABI document says "variadic arguments are passed according to the calling convention." It doesn't say "a different calling convention than AAPCS64 that puts everything on the stack." The Mach-O format specification describes chained fixup headers but doesn't specify that the `next` field position differs from what the header comments suggest. The ObjC runtime documentation mentions tagged pointers but doesn't detail the tag class table or the non-pointer ISA bitmask. The framework reference lists public APIs but doesn't mention libxo, compact unwind, or the private framework dependency chain.

None of this is malicious omission. The documentation describes *intent*: what the designers wanted the system to do, at the level of abstraction they chose to present. The binaries describe *reality*: what the compiler actually generated, what the linker actually produced, what the runtime actually checks. The gap between intent and reality is where loaders live — and where this appendix was born.

The specification IS the implementation. The rest is commentary.

---

*End of Appendix B.*
