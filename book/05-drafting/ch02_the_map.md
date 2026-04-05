# Chapter 2: The Map

We can read the binary. Every header field, every load command, every segment boundary — Chapter 1 gave us the anatomy. We can print it, inspect it, diff it against `otool -l`. But reading isn't running.

The parsed Mach-O sits on the heap as a nice `ParsedMachO` struct: vectors of segments, a header, some load commands. It's a description of a program. It's a recipe, not a meal. If you tried to jump to address `0x100003f40` — the entry point declared by `LC_MAIN` — you'd land in empty space or someone else's memory and immediately crash. The binary's code lives in a file on disk. For it to execute, it needs to live in memory, at real addresses, with the right permissions on every page.

This chapter is about that transition. We're going to map a Mach-O binary into the current process's address space — not as a bag of bytes on the heap, but as an executable image at real virtual addresses with real page protections. By the end, we'll have a `MappedImage` struct that owns a contiguous memory region where `__TEXT` is executable, `__DATA` is writable, and `__LINKEDIT` is read-only. The binary will be alive in memory.

It still won't work. Every function pointer in the GOT will be zero. Every external symbol will point at nothing. But that's Chapter 3's problem. First, we need to give the binary a place to live.

---

## Scene 2.1: Virtual Memory Primer

Before we write a single line of mapping code, we need to understand the canvas we're painting on. Virtual memory is that canvas, and if you've only ever used it through `malloc()` and `free()`, there's a layer underneath that matters enormously for binary loading.

### Pages: The Unit of Everything

The operating system doesn't manage memory byte by byte. It manages memory in *pages* — fixed-size chunks that are the atomic unit of allocation, protection, and mapping. On Intel x86-64, pages are 4,096 bytes (4 KiB). On Apple Silicon, they're 16,384 bytes (16 KiB). This isn't a suggestion. It's a hardware constraint. The page table entries that map virtual addresses to physical frames operate at page granularity. You can't make half a page executable and the other half writable. The entire page gets one set of permissions.

This matters for us immediately: when we map a Mach-O segment into memory, the segment's size must be a multiple of the page size. If `__TEXT` is 12,288 bytes, that's exactly 3 pages on Apple Silicon (3 × 4,096 on Intel). If the segment were 12,289 bytes, the kernel would round up to the next page boundary — 16,384 bytes on Apple Silicon — and the extra bytes would be zero-filled. The Mach-O format knows this. Every segment's `vmsize` field is already page-aligned by the linker. We just need to respect it.

```
Page size by architecture:
  x86-64:        4,096 bytes  (4 KiB)
  Apple Silicon: 16,384 bytes (16 KiB)
  Linux/ARM64:   4,096 bytes  (4 KiB, usually)
```

That Linux line will matter later. When we run macOS binaries on Linux in Docker, the host kernel uses 4K pages but the binary was linked for 16K. This mismatch is a source of real bugs — segments that should be on separate pages end up sharing a page, and the protection bits collide. We'll deal with that in Chapter 6. For now, just know that "page size" is platform-specific and our code needs to ask the kernel, not assume.

### Permissions: Read, Write, Execute

Every page in a process's address space has three permission bits:

- **Read (R):** The process can read bytes from this page.
- **Write (W):** The process can write bytes to this page.
- **Execute (X):** The CPU can fetch and execute instructions from this page.

These map directly to the hardware's permission flags in the page table. Violate them and you get a signal — `SIGSEGV` for reads and writes, `SIGBUS` sometimes on macOS, and the specific flavor depends on the OS. The crash is immediate and non-negotiable. There is no exception handler. There is no second chance. The kernel delivers the signal and your process is done.

For a Mach-O binary, the permission mapping is intuitive:

| Segment | Permissions | Why |
|---------|------------|-----|
| `__TEXT` | `r-x` | Code must be readable (for PC-relative data access) and executable. Never writable — that would let attackers modify code. |
| `__DATA` | `rw-` | Global variables, the GOT, Objective-C metadata. Must be writable for fixups. Never executable — W+X together is a security nightmare. |
| `__LINKEDIT` | `r--` | Symbol tables, string tables, code signatures. Read-only reference data. |
| `__DATA_CONST` | `r--` (final) | ObjC class pointers and vtables. Starts `rw-` during loading for fixups, then tightened to `r--` after initialization. |

The `__DATA_CONST` case is interesting. The segment *declares* `r--` as its intended protection, but the loader needs to write fixup values into it during startup. So the real sequence is: map as `rw-`, apply fixups, then `mprotect()` to `r--`. This is a pattern we'll see in our code — initial permissions are permissive, final permissions are tight.

### mmap(): Placing Data at Addresses

The syscall that makes all of this work is `mmap()`. If you've used it for memory-mapped files or large allocations, you already know the basics. But for binary loading, we use it in a specific way.

```c
void *mmap(
    void *addr,    // Desired address (or NULL for kernel's choice)
    size_t length, // Size of mapping (page-aligned)
    int prot,      // PROT_READ | PROT_WRITE | PROT_EXEC
    int flags,     // MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED
    int fd,        // File descriptor (-1 for anonymous)
    off_t offset   // Offset in file (0 for anonymous)
);
```

For our loader, the key combination is `MAP_PRIVATE | MAP_ANONYMOUS`. This gives us a fresh region of zero-filled memory at a kernel-chosen address. We're not memory-mapping the Mach-O file directly (that would be simpler but gives us less control over the layout). Instead, we allocate anonymous memory and then *copy* the segment data into it. This lets us control the exact layout: all segments contiguous, at their correct relative offsets.

Why contiguous? Because ARM64 instructions use PC-relative addressing. An `adrp` instruction in `__TEXT` might reference a literal pool 4 megabytes away in `__DATA`. If those segments aren't at their correct relative offsets, the PC-relative calculation lands on garbage. Intel's x86-64 has the same issue with RIP-relative addressing. The binary's internal references assume a specific memory layout. We must reproduce it.

Here's the mental model: `mmap()` with `MAP_ANONYMOUS` gives us a blank canvas — a range of virtual addresses backed by zero-filled pages, with whatever permissions we request. We then paint the binary's segments onto this canvas using `memcpy()` (or in Rust, `ptr::copy_nonoverlapping`). After painting, we adjust the permissions with `mprotect()` so that `__TEXT` becomes executable and `__DATA` loses execute permission.

```
Process address space (simplified):

0x000000000000  ┌─────────────────────┐
                │ (unmapped / NULL)   │
                ├─────────────────────┤
  0x100000000   │ __TEXT  (r-x)       │  ← code lives here
                ├─────────────────────┤
  0x100008000   │ __DATA  (rw-)       │  ← globals, GOT, ObjC metadata
                ├─────────────────────┤
  0x10000C000   │ __LINKEDIT (r--)    │  ← symbols, signatures
                ├─────────────────────┤
                │ ...                 │
                │ heap, stack, libs   │
                └─────────────────────┘
```

This is the address space we're building. The binary describes where each segment should live (the `vmaddr` field in each segment command) and how big it is (`vmsize`). Our job is to honour those relationships. The actual base address might differ — that's ASLR, and we'll get to it — but the relative layout must be exact.

### The Canvas Metaphor

Think of the process address space as a vast, empty canvas — roughly 48 bits of addressable space on ARM64, or 256 terabytes of virtual addresses. Most of it is unmapped. Touch an unmapped page and you crash. The kernel uses the page table to give meaning to specific ranges: this range is your stack, that range is your heap, this other range is a shared library.

When we load a Mach-O, we're claiming a new region of this canvas. We ask the kernel for a contiguous chunk big enough to hold all segments, and then we fill it in. The binary goes from being data-on-the-heap — a `Vec<u8>` that we parsed in Chapter 1 — to being *live-in-memory* at real addresses. The `__TEXT` segment's instructions are at addresses the CPU can actually jump to. The `__DATA` segment's global variables are at addresses that `adrp` / `ldr` sequences can actually reach.

This is the difference between analysis and execution. The parsed Mach-O tells us *about* the program. The mapped image *is* the program — or at least, the program's body. It still needs a nervous system (fixups) and a brain (the runtime), but the body is real.

Let's build it.

---

## Scene 2.2: Segments to Memory

With the virtual memory model clear, let's map an actual binary. We'll take the `MappingPlan` from `macho-map` — which we built from the parsed Mach-O in Chapter 1 — and turn it into a live memory image.

### The Mapping Plan

The `MappingPlan` is a simple intermediate representation. It strips out everything we don't need for mapping and keeps just the geometry:

```rust
pub struct MapRegion {
    pub segment_name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub protection: VmProt,
    pub max_protection: VmProt,
    pub zerofill_size: u64,
}

pub struct MappingPlan {
    pub regions: Vec<MapRegion>,
}
```

Each `MapRegion` is one segment. The `vmaddr` is where the linker intended it to go. The `vmsize` is how much virtual address space it occupies. The `fileoff` and `filesize` tell us where the segment's data lives in the file. The `zerofill_size` is the difference — `vmsize - filesize` — which is the BSS region (uninitialized globals that the loader must zero-fill).

For a typical small binary, the mapping plan looks like:

```
$ macho-inspect --plan /usr/bin/true

Segment __PAGEZERO
  vmaddr: 0x00000000
  vmsize: 0x100000000
  fileoff: 0x0
  filesize: 0x0
  prot: ---

Segment __TEXT
  vmaddr: 0x100000000
  vmsize: 0x4000
  fileoff: 0x0
  filesize: 0x4000
  prot: r-x

Segment __DATA_CONST
  vmaddr: 0x100004000
  vmsize: 0x4000
  fileoff: 0x4000
  filesize: 0x38
  prot: r--

Segment __LINKEDIT
  vmaddr: 0x100008000
  vmsize: 0x4000
  fileoff: 0x8000
  filesize: 0x118
  prot: r--
```

Notice `__PAGEZERO`. It's a special segment with no permissions — its entire purpose is to make addresses below `0x100000000` crash on access, catching null pointer dereferences. We skip it during mapping. It occupies 4 GiB of virtual address space in the binary's layout, but we never allocate physical memory for it. The kernel's default unmapped-page behaviour gives us the same crash-on-access semantics for free.

### The Contiguous Allocation Strategy

The naive approach would be to `mmap()` each segment independently — three calls for three segments. This is wrong. ARM64's `adrp` instruction computes addresses relative to the program counter, and it works over a ±4 GiB range. If `__TEXT` lands at `0x7fff0000` and `__DATA` lands at `0x3000_0000` because the kernel chose independently, the PC-relative reference will compute the wrong address.

The correct approach: allocate **one** contiguous region that spans all segments, then place each segment at its correct offset within that region:

```rust
// Compute the virtual address range
let min_vmaddr = regions.iter().map(|r| r.vmaddr).min().unwrap();
let max_vm_end = regions.iter().map(|r| r.vmaddr + r.vmsize).max().unwrap();
let total_vm_span = (max_vm_end - min_vmaddr) as usize;

// Round up to page boundary
let alloc_size = (total_vm_span + page_size - 1) & !(page_size - 1);

// One big anonymous mapping
let alloc_base = mapper.map_image_pages(alloc_size)?;
```

The `& !(page_size - 1)` trick is a standard page-alignment idiom. If `page_size` is `0x4000` (16,384), then `page_size - 1` is `0x3FFF`, and `!(page_size - 1)` is `0xFFFF_FFFF_FFFF_C000`. ANDing with this mask zeros the bottom 14 bits, rounding down to the nearest 16K boundary. Adding `page_size - 1` first ensures we round *up* instead of down.

The `map_image_pages()` call is a method on our `MemoryMapper` trait. On macOS, it's a simple `mmap()` with anonymous flags. On Linux with SELinux in enforcing mode, it uses `memfd_create()` to get a file-descriptor-backed mapping that can later be transitioned from writable to executable without violating the W^X policy. We'll cover that in Chapter 6. For now, it gives us a pointer to `alloc_size` bytes of zero-filled, read-write memory.

### Painting the Segments

With the contiguous region allocated, we place each segment at its correct offset:

```rust
let slide = alloc_base as i64 - min_vmaddr as i64;

for region in &regions {
    let offset_in_alloc = (region.vmaddr - min_vmaddr) as usize;
    let seg_base = unsafe { alloc_base.add(offset_in_alloc) };
    let seg_size = (region.vmsize as usize + page_size - 1) & !(page_size - 1);

    // Copy file-backed data
    if region.filesize > 0 {
        let src = &binary_data[region.fileoff as usize
            ..(region.fileoff + region.filesize) as usize];
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), seg_base, src.len());
        }
    }
    // Remainder is already zero-filled by MAP_ANON

    segments.push(MappedSegment {
        name: region.segment_name.clone(),
        base: seg_base,
        size: seg_size,
        vmaddr: region.vmaddr,
        protection: region.protection,
        fileoff: region.fileoff,
        filesize: region.filesize,
    });
}
```

The critical line is `offset_in_alloc = (region.vmaddr - min_vmaddr) as usize`. This preserves the relative layout. If `__TEXT` has `vmaddr = 0x100000000` and `__DATA` has `vmaddr = 0x100008000`, then `__DATA` will be placed 32,768 bytes after `__TEXT` in our allocation — exactly where the instructions expect it.

The `ptr::copy_nonoverlapping` is Rust's equivalent of `memcpy()`. We're copying raw bytes from the binary file into the mapped region. For `__TEXT`, this copies the executable code. For `__DATA`, it copies initial values of global variables. For `__LINKEDIT`, it copies symbol tables and code signature data. Whatever the binary's `filesize` declares, that's how many bytes we copy. The rest of the `vmsize` — the zerofill region — stays zero, which is correct for BSS data (uninitialized globals default to zero in C).

### Why file-backed data can be smaller than the segment

Look at `__DATA_CONST` from our example: `vmsize = 0x4000` but `filesize = 0x38`. The segment spans one full 16K page in memory, but only 56 bytes of that come from the file. The remaining 16,328 bytes are zero-filled. This is normal. The linker pads segments to page boundaries in virtual memory, but it doesn't waste disk space storing those zeros. Our anonymous mapping already starts zero-filled, so we just copy the 56 file-backed bytes and everything works.

### Protection: A Two-Phase Dance

We don't set protections during mapping. The entire region starts as read-write (`PROT_READ | PROT_WRITE`). This is deliberate — we need to write fixup values into `__DATA` and even into `__TEXT` in some cases (stub patching). Only after all fixups are applied do we tighten the permissions:

```rust
pub fn protect(&self) -> Result<(), ExecError> {
    for seg in &self.segments {
        let prot = vmprot_to_protection(seg.protection);
        self.mapper.protect(seg.base, seg.size, prot)?;
    }
    Ok(())
}

fn vmprot_to_protection(prot: VmProt) -> Protection {
    Protection {
        read: prot.contains(VmProt::READ),
        write: prot.contains(VmProt::WRITE),
        execute: prot.contains(VmProt::EXECUTE),
    }
}
```

After `protect()`, `__TEXT` becomes `r-x` — readable and executable, but no longer writable. `__DATA` becomes `rw-` — readable and writable, but not executable. If anything tries to execute code from `__DATA`, the hardware triggers a fault. If anything tries to write to `__TEXT`, same. This is W^X (write XOR execute) enforcement, and it's a fundamental security boundary.

The actual implementation is more nuanced. We handle page sharing — when the host's page size is larger than the Mach-O's segment alignment, multiple segments can share a physical page. In that case, the shared page gets the union of all overlapping segments' permissions. We build a per-page protection map using a `BTreeMap`, coalesce contiguous pages with identical permissions into single `mprotect()` calls, and apply them efficiently.

But the core idea is simple: map everything writable, do your fixup work, then lock it down.

### Validation Before Mapping

Before we copy any bytes, we validate bounds. Every region's `fileoff + filesize` must fit within the actual binary data. A malformed binary that claims `fileoff = 0` and `filesize = 0xFFFFFFFF` would cause an out-of-bounds read without this check:

```rust
for region in &regions {
    if region.filesize > 0 {
        let end = region.fileoff + region.filesize;
        if end > binary_data.len() as u64 {
            return Err(ExecError::SegmentOutOfBounds {
                segment: region.segment_name.clone(),
                fileoff: region.fileoff,
                filesize: region.filesize,
                binary_len: binary_data.len() as u64,
            });
        }
    }
}
```

This runs before any allocation happens. If the binary is corrupt or truncated, we fail cleanly before touching kernel resources.

When all segments are placed successfully, we have a `MappedImage`. The binary's code is at real addresses. Its data sections are initialized. The memory is live. But every address inside the binary still assumes the linker's preferred base address — and that's not where we loaded it.

---

## Scene 2.3: ASLR and the Slide

Run a binary twice. Check its base address both times. They're different.

```
$ # First run
$ lldb /usr/bin/true -o 'image list' -o 'quit' 2>/dev/null | grep true
[  0] /usr/bin/true  0x0000000100b90000

$ # Second run
$ lldb /usr/bin/true -o 'image list' -o 'quit' 2>/dev/null | grep true
[  0] /usr/bin/true  0x0000000102a14000
```

The binary declared `vmaddr = 0x100000000` for `__TEXT`. The kernel loaded it at `0x100b90000` the first time and `0x102a14000` the second. This is Address Space Layout Randomization — ASLR.

### Why ASLR Exists

Before ASLR, every binary loaded at the same address every time. `/usr/bin/true` always started at `0x100000000`. This made exploitation trivial. An attacker who found a buffer overflow knew exactly where every function was. Return-to-libc attacks, ROP chains, code injection — all of them relied on predictable addresses.

ASLR adds a random offset to the base address at load time. Now the attacker doesn't know where `system()` lives, doesn't know where the GOT is, doesn't know where to land a ROP gadget. It's not a complete defense — information leaks can reveal the slide — but it raises the bar enormously. macOS has enforced ASLR since 10.7 Lion (2011), and modern binaries are always compiled as PIE (Position Independent Executable) to support it.

### The Slide

The "slide" is the difference between where the binary wanted to load and where it actually landed:

```
slide = actual_base_address - preferred_base_address
```

If the binary preferred `0x100000000` and we loaded it at `0x100b90000`, the slide is `0xb90000`. Every internal pointer in the binary — function addresses, global variable references, vtable entries, Objective-C class pointers — was encoded assuming the preferred base. They all need to be adjusted by the slide.

In our code, we calculate this trivially:

```rust
let slide = alloc_base as i64 - min_vmaddr as i64;
```

It's `i64` because the slide can be negative — the kernel might assign a base address *below* the preferred base. In practice, on 64-bit systems with a large address space, the slide is usually positive and small relative to the address range. But the type needs to handle both directions.

### What Needs Adjusting

Consider a binary with this code:

```asm
; In __TEXT, a function loads a global variable:
adrp    x8, _my_global@PAGE        ; x8 = PC-aligned address of the page
ldr     x8, [x8, _my_global@PAGEOFF] ; x8 = value of _my_global
```

The `adrp` instruction computes addresses relative to the current PC. These work *without* adjustment — the relative offset between `__TEXT` and `__DATA` is preserved by our contiguous allocation. This is why PIE works: most references are PC-relative and don't care about the absolute base.

But some things store absolute addresses:

- **Rebase entries** in the fixup chain: the linker records places in `__DATA` that contain absolute pointers to `__TEXT` or other segments. These must be adjusted by the slide.
- **Objective-C metadata:** class pointers, method lists, selector references — many of these are absolute addresses baked into `__DATA`.
- **C++ vtables:** virtual function pointers stored as absolute addresses.
- **The GOT (Global Offset Table):** function pointers resolved by the dynamic linker. These start as zeros (for lazy binding) or preferred addresses (for non-lazy binding).

The pattern is always the same: the binary contains an address like `0x100003F40`. The linker computed this assuming the preferred base. The actual address is `0x100003F40 + slide`. The rebase fixups — which we'll implement in Chapter 3 — walk through every such pointer and add the slide.

### Our Loader's Slide

In a real kernel loader (dyld), the OS kernel picks the random slide. In our userspace loader, the story is slightly different. We call `mmap()` with `MAP_ANONYMOUS` and let the kernel pick the address. The kernel returns whatever address it likes — this gives us a random-ish base that's effectively ASLR from the loaded binary's perspective.

```rust
// After mmap:
let alloc_base = mapper.map_image_pages(alloc_size)?;
let slide = alloc_base as i64 - min_vmaddr as i64;

log::info!(
    "Mapped {} bytes at {:#x}, slide = {:#x}",
    alloc_size,
    alloc_base as u64,
    slide
);
```

A typical run on Linux/arm64 might show:

> Mapped 0x10000 bytes at 0x7ffec4200000, slide = 0x7ffec4200000

That's a huge slide — but it's just arithmetic. Every rebase fixup adds `0x7ffec4200000` to the pointer value and it all works. The slide being large doesn't cost anything; it's the same single addition per fixup whether the slide is `0x1000` or `0x7ffec4200000`.

### PIE vs Non-PIE

Modern macOS binaries are universally PIE. The `MH_PIE` flag in the header signals this. But what about non-PIE binaries? They expect to be loaded at their exact preferred address. If you try to load a non-PIE binary at a different address, the code will jump to wrong locations because it uses absolute addresses everywhere — not just in data, but in code.

For our loader, this is a concern we can sidestep. The Apple toolchain has required PIE for years. Every binary in the Xcode command-line tools, every framework library, every dylib we'll encounter — they're all PIE. We check the flag and refuse non-PIE binaries:

```rust
if !parsed.header.flags.contains(MachFlags::PIE) {
    log::warn!("Binary is not PIE — ASLR slide may cause crashes");
}
```

In practice, we've never hit a non-PIE binary in our corpus. The warning is informational, not a hard error — some special-purpose binaries might work at any address despite not declaring PIE.

### The Slide as a Thread Through the Book

The slide concept shows up again and again. In Chapter 3, rebase fixups apply the slide to data pointers. In Chapter 4, Objective-C class registration adjusts metadata pointers by the slide. In Chapter 7, the interpreter maintains a virtual slide for the guest binary's address space. Every chapter builds on this one arithmetic operation: `actual = preferred + slide`.

It's simple math. But when you first encounter a SIGSEGV at some wild address and subtract the text base to find the crash offset, and then realize the offset doesn't match any instruction boundary because the slide is wrong — that's when you *really* understand ASLR. Not because the concept is hard, but because debugging it without understanding it is nearly impossible.

---

## Scene 2.4: The MappedImage

We've covered the theory: pages, permissions, `mmap()`, ASLR. We've seen the code fragments. Now let's pull it all together into the abstraction that the rest of the loader will depend on: `MappedImage`.

### The Crate: macho-exec

The `macho-exec` crate is where analysis crosses into execution. The `macho-parse` crate from Chapter 1 was pure read-only analysis — it never touched the process's address space. `macho-exec` changes the rules. It allocates memory, copies bytes into executable regions, and eventually jumps into mapped code. Here's what the crate exposes:

```rust
// macho-exec/src/lib.rs
pub use mapped::{MappedImage, MappedSegment};
pub use entry::{call_entry, call_entry_with_args, EntryArgs, EntryResult};
pub use fixup::{apply_binds, apply_chained_fixups, apply_rebases};
```

The dependency chain is clean: `macho-parse` produces a `ParsedMachO`. `macho-map` extracts a `MappingPlan` from it. `macho-exec` takes the `MappingPlan` and the raw binary data, and produces a `MappedImage`. Each crate does one thing. Each output feeds the next input.

### The MappedSegment

A `MappedSegment` is a bookkeeping record for one mapped segment:

```rust
pub struct MappedSegment {
    /// Segment name (e.g., "__TEXT").
    pub name: String,
    /// Base address of this segment within the contiguous region.
    pub base: *mut u8,
    /// Size of this segment (page-aligned).
    pub size: usize,
    /// Original vmaddr from the Mach-O.
    pub vmaddr: u64,
    /// Declared protection from the segment command.
    pub protection: VmProt,
    /// File offset in the original binary.
    pub fileoff: u64,
    /// File size copied into the mapping.
    pub filesize: u64,
}
```

The raw pointer `base` is the actual virtual address where this segment lives in our process. After mapping `__TEXT`, `seg.base` points to the first byte of executable code. You can read the `0xFEEDFACF` magic right from that pointer — the Mach-O header is the first thing in `__TEXT`:

```rust
let text = image.segment("__TEXT").unwrap();
let magic = unsafe { *(text.base as *const u32) };
assert_eq!(magic, 0xFEEDFACF); // We're looking at live mapped code
```

The `vmaddr` field is the *preferred* address from the binary. The `base` field is the *actual* address. The difference is the slide.

### The MappedImage

The `MappedImage` struct owns the entire mapped region:

```rust
pub struct MappedImage {
    /// Mapped segments (in order, excluding PAGEZERO).
    pub segments: Vec<MappedSegment>,
    /// ASLR slide: actual base minus the binary's lowest vmaddr.
    pub slide: i64,
    /// Base of the contiguous allocation (for unmap on drop).
    alloc_base: *mut u8,
    /// Total size of the contiguous allocation (for unmap on drop).
    alloc_size: usize,
    /// Memory mapper for cleanup on drop.
    mapper: Arc<dyn MemoryMapper>,
    /// Unwind registry for cleanup on drop.
    unwind: Arc<dyn UnwindRegistry>,
}
```

The private fields — `alloc_base`, `alloc_size`, `mapper` — handle cleanup. When the `MappedImage` is dropped, it calls `munmap()` through the mapper trait:

```rust
impl Drop for MappedImage {
    fn drop(&mut self) {
        if !self.alloc_base.is_null() && self.alloc_size > 0 {
            if let Err(e) = self.mapper.unmap(self.alloc_base, self.alloc_size) {
                log::warn!("MappedImage Drop: failed to unmap: {}", e);
            }
        }
    }
}
```

This is RAII for memory mappings. The `MappedImage` *is* the mapping. When it goes out of scope, the kernel reclaims the pages. No leaks. No dangling mapped regions. The `Arc<dyn MemoryMapper>` ensures the mapper outlives the image — Rust's borrow checker won't let you create a `MappedImage` with a mapper that might be freed first.

### The MemoryMapper Trait

The mapper trait is a seam for platform abstraction:

```rust
pub trait MemoryMapper: Send + Sync {
    fn page_size(&self) -> usize;
    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError>;
    fn map_image_pages(&self, size: usize) -> Result<*mut u8, PalError>;
    fn protect(&self, addr: *mut u8, size: usize, prot: Protection)
        -> Result<(), PalError>;
    fn unmap(&self, addr: *mut u8, size: usize) -> Result<(), PalError>;
}
```

On macOS, `map_anonymous()` calls `mmap()` with `MAP_ANON | MAP_PRIVATE`. On Linux, `map_image_pages()` might use `memfd_create()` for W^X-safe mappings. In tests, we can inject a mock mapper that tracks allocations without touching the kernel. The trait-based design means `MappedImage` doesn't know what OS it's running on. It just asks the mapper for pages and trusts the answers.

### The Lifecycle: Parse → Plan → Map → Fix → Protect → Execute

Here's the full pipeline crystallized:

```rust
// 1. Parse the binary (Chapter 1)
let parsed = ParsedMachO::parse(&binary_data)?;

// 2. Build the mapping plan
let plan = MappingPlan::from_parsed(&parsed);

// 3. Map into memory (this chapter)
let image = MappedImage::map(
    &plan,
    &binary_data,
    mapper.clone(),
    unwind.clone(),
)?;

// 4. Apply fixups (Chapter 3)
apply_chained_fixups(&image, &parsed, &resolver)?;

// 5. Tighten permissions
image.protect()?;

// 6. Call entry point
let result = call_entry(&image, &parsed.entry_point)?;
println!("Exit code: {}", result.exit_code);
```

Steps 1 through 3 are what we've built so far. Step 4 — fixups — is the critical next step. Step 5 is the `protect()` call we discussed. Step 6 is the payoff: we compute the entry address from `text_base + entry_offset` and call it as a function pointer.

But between steps 3 and 4, the `MappedImage` is in a curious state. The code is at real addresses. The data is initialized with values from the file. But every pointer in the GOT — the table that routes calls to external functions like `printf`, `malloc`, `exit` — is zero. Zero or the preferred-base value from the linker, which is equally wrong since we loaded at a different address.

Let's look at the GOT right after mapping:

```rust
if let Some(data) = image.segment("__DATA") {
    // Dump the first 128 bytes of __DATA (includes GOT entries)
    let got_bytes = unsafe {
        std::slice::from_raw_parts(data.base, 128.min(data.size))
    };
    for (i, chunk) in got_bytes.chunks(8).enumerate() {
        let val = u64::from_le_bytes(chunk.try_into().unwrap());
        println!("GOT[{i}]: {val:#018x}");
    }
}
```

> GOT[0]: 0x0000000000000000
> GOT[1]: 0x0000000000000000
> GOT[2]: 0x0000000000000000
> GOT[3]: 0x0000000000000000

All zeros. Every external function call will jump to address zero and crash. The binary is mapped, but it's inert — a body without a nervous system. The fixups in Chapter 3 will fill in these zeros with actual function addresses, connecting the binary's stub calls to real implementations.

### Error Handling

The `MappedImage::map()` function returns `Result<MappedImage, ExecError>`. The error variants are specific and actionable:

```rust
pub enum ExecError {
    MmapFailed { segment: String, source: std::io::Error },
    MprotectFailed { segment: String, source: std::io::Error },
    SegmentOutOfBounds { segment: String, fileoff: u64,
                         filesize: u64, binary_len: u64 },
    NoTextSegment,
    NoEntryPoint,
    SecurityDenied { reason: String },
    EncryptedBinary { crypt_id: u32 },
    // ...
}
```

`MmapFailed` means the kernel refused our allocation — usually because we asked for too much memory or don't have the right privileges. `SegmentOutOfBounds` means the binary is truncated or corrupt. `NoTextSegment` means we can't find `__TEXT`, which shouldn't happen in a valid Mach-O but defensive coding requires handling it.

The error type uses `thiserror` for derive-based `Display` implementations. Each variant formats into a human-readable message that includes the segment name and the specific failure. When a mapping fails at 2 AM in a Docker build log, you want the error message to tell you exactly which segment and which bounds check failed. Generic "mapping failed" messages are worthless.

---

## Scene 2.5: Code Signing and Verification

The binary is mapped. The segments are placed. The slide is calculated. Before we move to fixups, there's one more layer to understand — a layer that's invisible in normal use but fundamental to how macOS trusts (or refuses to trust) a binary.

### LC_CODE_SIGNATURE

Every signed macOS binary has an `LC_CODE_SIGNATURE` load command. It points to a region at the very end of `__LINKEDIT` containing a nested structure called the **SuperBlob** — a container that holds multiple signature-related blobs:

```
LC_CODE_SIGNATURE:
  dataoff:  0x8120     (offset in file)
  datasize: 0x4E0      (size of signature data)
```

The data at that offset starts with a magic number: `0xFADE0CC0`. If `0xFEEDFACF` is the binary's opening handshake, `0xFADE0CC0` is its notarized seal. Every `FADE` magic in the code signature subsystem follows the same pattern — `FADE0C02` for the CodeDirectory, `FADE7171` for entitlements, `FADE0B01` for CMS signature wrappers. Apple likes their hex wordplay.

### The SuperBlob Structure

The SuperBlob is an indexed container. It consists of a header with a count of sub-blobs, followed by an index table that maps slot types to offsets:

```rust
pub struct SuperBlob {
    pub magic: u32,      // 0xFADE0CC0
    pub length: u32,     // total size
    pub count: u32,      // number of blobs
    pub index: Vec<BlobIndex>,
}

pub struct BlobIndex {
    pub slot_type: CsSlotType,
    pub offset: u32,
}
```

The slot types identify what each blob contains:

| Slot | Type | Content |
|------|------|---------|
| 0 | `CodeDirectory` | Page hashes, identity, flags |
| 2 | `Requirements` | Code requirement expressions |
| 5 | `Entitlements` | XML plist of capabilities |
| 0x10000 | `CmsSignature` | PKCS#7 cryptographic signature |

Note the big-endian byte order. Unlike the rest of the Mach-O format (which matches the host architecture — little-endian on ARM64 and x86-64), the code signature structures are always big-endian. This is a legacy from the PowerPC era. When you're parsing these by hand, endianness flip bugs are a reliable source of confusion.

### The CodeDirectory

The CodeDirectory is the heart of the code signature. It contains:

```rust
pub struct CodeDirectory {
    pub version: u32,
    pub flags: CsFlags,
    pub hash_offset: u32,
    pub ident_offset: u32,
    pub n_special_slots: u32,
    pub n_code_slots: u32,
    pub code_limit: u32,
    pub hash_type: CsHashType,  // Usually SHA-256
    pub hash_size: u8,           // 32 for SHA-256
    pub page_size_log2: u8,      // 12 = 4K pages
    pub identity: String,
    pub team_id: String,
    pub cd_hash: Vec<u8>,
    pub page_hashes: Vec<Vec<u8>>,
}
```

The `page_hashes` field is the critical part. The CodeDirectory divides the binary into pages (typically 4K, regardless of the runtime page size) and stores a SHA-256 hash of each page. When the kernel loads the binary, it can verify every page individually: hash the page, compare against the stored hash. If they don't match, the page has been tampered with. The binary is rejected.

The `page_size_log2` field controls the code-signing page size. A value of `12` means `2^12 = 4096` bytes per page. This is independent of the runtime page size — even on Apple Silicon with 16K runtime pages, code signing hashes 4K chunks. Four code-signature hashes cover one runtime page.

### CsFlags: Security Properties

The `flags` field in the CodeDirectory tells the kernel what security properties the binary claims:

```rust
bitflags! {
    pub struct CsFlags: u32 {
        const ADHOC           = 0x0000_0002;  // Linker-signed, no identity
        const GET_TASK_ALLOW  = 0x0000_0004;  // Debuggable
        const RESTRICT        = 0x0000_0800;  // Strip DYLD_* env vars
        const RUNTIME         = 0x0001_0000;  // Hardened runtime
        const LINKER_SIGNED   = 0x0002_0000;  // Signed by ld, not codesign
        const REQUIRE_LV      = 0x0000_2000;  // Library validation
    }
}
```

`CS_RUNTIME` (hardened runtime) is the most impactful for us. A hardened-runtime binary gets additional protections: it can't load unsigned libraries, can't use `DYLD_INSERT_LIBRARIES` for injection, and can't map `RWX` memory. When we're debugging why a tool fails inside our loader, the hardened runtime flag is often the first thing to check.

`CS_ADHOC` means the binary was signed by the linker at build time with no identity or certificate. All modern Apple toolchain outputs are at minimum ad-hoc signed. An unsigned binary — one with no `LC_CODE_SIGNATURE` at all — won't run on macOS Ventura or later without overriding Gatekeeper.

### The CDHash

The CDHash is a hash of the entire CodeDirectory blob itself. It's a compact fingerprint — 20 bytes for SHA-1, 32 bytes for SHA-256 — that uniquely identifies a specific version of a specific binary. Apple's AMFI (AppleMobileFileIntegrity) subsystem checks the CDHash against trust caches and notarization tickets. If the CDHash isn't in any trust cache, the binary must have a valid CMS signature chain back to an Apple signing certificate.

In our loader, we don't validate this chain (we're not AMFI), but we extract the CDHash for logging and diagnostics:

```rust
pub struct CodeSignatureInfo {
    pub present: bool,
    pub adhoc: bool,
    pub runtime: bool,
    pub restrict: bool,
    pub library_validation: bool,
    pub identity: String,
    pub team_id: String,
    pub cd_hash: String,
    pub has_cms_signature: bool,
    pub has_entitlements: bool,
}
```

When we run `macho-inspect` on a binary and the security report shows `adhoc=true, runtime=true, cms=false`, that's a linker-signed hardened binary. Standard output from modern Xcode. When it shows `adhoc=false, runtime=true, cms=true`, that's a developer-signed or Apple-signed binary with a full certificate chain. The distinction matters when debugging why `DYLD_INSERT_LIBRARIES` is being ignored or why library loading fails.

### Security Policy in Our Loader

Our loader implements a composable trust policy system. Each security gate — code signature verification, entitlement evaluation, AMFI emulation, sandbox checks — is a trait:

```rust
pub fn checked_entry(
    image: &MappedImage,
    entry: &EntryPoint,
    parsed: &ParsedMachO,
    binary_path: &Path,
    policy: &TrustPolicy,
) -> Result<(EntryResult, SecurityReport), ExecError> {
    // Reject encrypted binaries
    for cmd in &parsed.commands {
        if let LoadCommandPayload::EncryptionInfo { crypt_id, .. } = &cmd.payload {
            if *crypt_id != 0 {
                return Err(ExecError::EncryptedBinary { crypt_id: *crypt_id });
            }
        }
    }

    // Build CodeSignatureInfo from parsed code signature
    let cs_info = parsed.code_signature.as_ref()
        .map(CodeSignatureInfo::from_parsed)
        .unwrap_or_default();

    // Evaluate all security gates
    let report = policy.evaluate_binary_parsed_with_keys(
        binary_path,
        parsed.header.flags,
        &cs_info,
        &entitlement_keys,
    );

    if !report.allowed {
        return Err(ExecError::SecurityDenied {
            reason: report.denied_reasons.join("; "),
        });
    }

    let result = call_entry(image, entry)?;
    Ok((result, report))
}
```

The design uses mock implementations by default — our loader is permissive, since we're not trying to enforce Apple's security model, just understand it. But the gate traits are there, and swapping in a strict implementation that actually verifies the CMS signature chain with a root certificate is a matter of implementing the trait. The architecture anticipates the need without paying for it upfront.

### Why This Matters for Loading

You might wonder whether code signing is strictly necessary for mapping a binary. It isn't — we can (and do) map binaries without verifying their signature. The kernel on macOS verifies signatures before our userspace loader ever touches the binary, and on Linux there's no signature verification at all.

But understanding code signing serves three purposes:

First, it explains why certain load commands exist. `LC_CODE_SIGNATURE` points to data in `__LINKEDIT` that we need to skip past when parsing other `__LINKEDIT` structures. If you don't know the signature data is there, you'll try to parse it as symbol tables and get garbage.

Second, it explains platform behavior. When our Docker-hosted loader runs a binary and the trace shows `hardened_runtime=true`, we know to expect strict library validation. When it shows `restrict=true`, we know `DYLD_*` environment variables will be stripped. These flags affect how the loader pipeline behaves, and the code signature is where they live.

Third, it reveals a deeper truth about macOS. The platform doesn't just *load* binaries — it *guards* them. Every page has a hash. Every binary has an identity. Every identity has entitlements. The security model is woven into the binary format itself. `0xFADE0CC0` isn't just a magic number — it's a statement: this binary carries proof of its integrity, and the system will check it.

### Looking Forward

The binary is mapped. We understand the memory layout — contiguous segments at their correct relative offsets, with ASLR slide applied. We understand the security envelope — code signatures, page hashes, trust policies. The `MappedImage` struct wraps all of this in a clean Rust type that owns its memory and drops it safely.

But look at the GOT. Print those first few entries. Every single one is zero.

```
GOT[0]: 0x0000000000000000
GOT[1]: 0x0000000000000000  
GOT[2]: 0x0000000000000000
```

The binary is loaded. It has code at real addresses, data in writable pages, a signature we could verify. It's a body with no nervous system. Every call to `printf` will jump to address zero. Every call to `malloc` will crash. Every Objective-C message send will fault.

The binary needs its libraries. It needs its function pointers filled in. It needs the dynamic linker's work done for it.

That's Chapter 3.
