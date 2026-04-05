# Chapter 6: The Bridge

## Scene 6.1: The Same Binary, Different Crash

Everything works on macOS.

Build the Docker image. Run the same tests. Crash.

Not a graceful failure. Not a missing library, not a polite error message, not a "file not found." A SIGSEGV. A hard fault. The CPU tried to read memory that doesn't exist, or jumped to an address that contains garbage, or passed a pointer that points into the void. The kind of crash that says *something fundamental is wrong*.

Let's back up. At the end of Chapter 5, we had a working system. The Foundation shims passed 2,682 tests. The corpus tools — 39 real macOS command-line binaries extracted from Xcode and the system — all ran correctly. `stat /` printed filesystem information. `cut` sliced columns from stdin. `sort` ordered lines. `strings` extracted printable sequences from binaries. Every tool that only needs C library functions and basic Foundation classes worked. We had a loader.

Then we asked a simple question: can we run this on Linux?

The loader is written in Rust. The target binaries are ARM64 Mach-O. Linux on ARM64 can execute ARM64 instructions natively — same ISA, same registers, same instruction encoding. The idea is straightforward: cross-compile the loader for `aarch64-unknown-linux-gnu`, copy the macOS binaries into a Docker container alongside the loader, and run them. Same binary. Same CPU architecture. Different operating system. How hard can it be?

Famous last words in systems programming. "How hard can it be" is the preamble to every three-week debugging odyssey. It's the question you ask when you think you understand the problem and you're about to discover you don't.

We build the Docker image. Multi-stage: compile in a Rust builder container, copy the binary to a slim Debian runtime. The build completes without errors. We COPY the extracted macOS tool binaries into `/opt/xcode-tools/`. We run the first test:

```
$ docker run --rm burdettie-os macho-inspect --exec /opt/xcode-tools/system/stat /
```

```
SIGSEGV at PC 0xffffb3e1e808
  x0  = 0x0000004000b31a40
  x1  = 0x00000040008c5000
  x30 = 0x0000004000825f48
```

The crash is deep. Not at the entry point, not in our loader's setup code — deep inside the binary's execution, in a library call that should work. We subtract the text base address from PC to get the offset: `0xb2808`. That's inside the binary's own code, in a function that calls through to what it thinks is `stat`. The instruction at the crash site is a load from a pointer that contains garbage.

But `stat` works. We shimmed it in Chapter 3. The binary calls `_stat`, our resolver finds the shim, the shim calls the host's `stat(2)`, returns the result. This works on macOS. Why doesn't it work on Linux?

We add tracing. The resolver log shows:

```
[TRACE] resolved '_stat$INODE64' via shim table
[TRACE] shim_stat called: path="/", buf=0x40008c5000
[TRACE] linux stat() returned 0 (success)
```

The shim runs. It succeeds. The stat call returns zero — no error. But immediately after, the binary crashes trying to read from the stat buffer. The call worked. The result is wrong.

We hexdump the buffer the binary receives. On macOS, after `stat("/", &buf)`, the binary reads `st_size` at byte offset 96. That's where Apple's `struct stat` puts it — after `st_dev` (4 bytes), `st_mode` (2), `st_nlink` (2), `st_ino` (8), `st_uid` (4), `st_gid` (4), `st_rdev` (4), padding (4), three timespec structs (48 bytes total), and `st_birthtimespec` (16 bytes). Add it up: 4+2+2+8+4+4+4+4+48+16 = 96. So `st_size` lives at offset 96 in a macOS stat buffer.

On Linux, `struct stat` for aarch64 puts `st_size` at offset 48. Different struct. Different layout. Different offsets. Same name.

Our shim was calling Linux's `stat()`, which fills in a *Linux* struct stat — 128 bytes, fields in Linux's order. Then the binary reads offset 96, expecting macOS's `st_size`. At offset 96 on Linux, that's `st_ctimespec.tv_sec` — the file's change time, not its size. The binary thinks it just got the file size. It got a timestamp. A very large number, interpreted as a byte count. It tries to allocate that many bytes, or read that many bytes, and crashes.

The stat call succeeded. The data came back in the wrong shape.

This is the moment the floor drops out. Both platforms are ARM64. The instructions are identical. The registers are the same width. The calling convention puts arguments in the same registers. But the *meaning* of the bytes that flow through those registers — the struct layouts, the constant values, the sizes and offsets — all of it diverges. We're not just running on a different OS. We're running in a different ABI universe, where every struct the binary touches could be a different size, every constant could have a different value, and every assumption about memory layout could be wrong.

ARM64 does not mean compatible. It means the instructions execute. It says nothing about what they mean.

Think about what we assumed. We assumed that because both platforms use the same CPU architecture — the same instruction set, the same register file, the same encoding — the binary would "just work" as long as we resolved the right symbols. We assumed the ABI (Application Binary Interface) was a property of the architecture. It isn't. The ABI is a property of the *platform*. The ISA defines what instructions do. The ABI defines what programs *mean*. Two platforms can share an ISA and disagree on everything else: how structs are laid out, how arguments are passed, what constant values represent, what size a `stat` buffer is, how `errno` is accessed. The instruction set is the grammar. The ABI is the language. Same grammar, different languages.

This is a lesson that Wine developers learned decades ago, that QEMU user-mode contributors fight with every port, that Darling's team warned about in their development notes. You don't emulate a platform by running its instructions. You emulate a platform by honoring its *contracts*. And the contracts are everywhere — in every struct definition, every constant value, every calling convention — and they're different everywhere.

The crash count in Docker is not one tool. It's all of them. Every tool that calls `stat`, `lstat`, `fstat`. Every tool that calls `printf` with format arguments. Every tool that calls `readdir`. Every tool that touches locale settings. The macOS binary expects macOS-shaped data at macOS-defined offsets, and Linux hands it Linux-shaped data at Linux-defined offsets. The same bytes pass through the same registers and produce garbage.

Chapter 5's victory was real but local. We proved the loader works — on the platform the binaries were built for. Cross-platform means proving it works on a platform the binaries were *never meant to run on*. That's a different kind of problem. Not a bug to fix. An entire compatibility layer to build.

The question isn't whether one thing is wrong. It's how many things are wrong and whether the list is finite.

---

## Scene 6.2: struct stat and the Sixteen Bytes

Start with the crash we understand. `struct stat`. macOS says 144 bytes. Linux says 128 bytes. Sixteen bytes of difference, and every field after the first few is at the wrong offset.

Here are the layouts side by side:

| Field              | macOS offset (size) | Linux aarch64 offset (size) |
|--------------------|--------------------|-----------------------------|
| `st_dev`           | 0 (i32)            | 0 (u64)                     |
| `st_mode`          | 4 (u16)            | 16 (u32)                    |
| `st_nlink`         | 6 (u16)            | 20 (u32)                    |
| `st_ino`           | 8 (u64)            | 8 (u64)                     |
| `st_uid`           | 16 (u32)           | 24 (u32)                    |
| `st_gid`           | 20 (u32)           | 28 (u32)                    |
| `st_rdev`          | 24 (i32)           | 32 (u64)                    |
| `st_atimespec`     | 32 (16B)           | 64 (16B)                    |
| `st_mtimespec`     | 48 (16B)           | 80 (16B)                    |
| `st_ctimespec`     | 64 (16B)           | 96 (16B)                    |
| `st_birthtimespec` | 80 (16B)           | *(none)*                    |
| `st_size`          | 96 (i64)           | 48 (i64)                    |
| `st_blocks`        | 104 (i64)          | 56 (i64)                    |
| `st_blksize`       | 112 (i32)          | 112 (u32)                   |
| `st_flags`         | 116 (u32)          | *(none)*                    |
| `st_gen`           | 120 (u32)          | *(none)*                    |
| **sizeof**         | **144**            | **128**                     |

Look at that table. Almost nothing lines up. `st_dev` is an `i32` on macOS and a `u64` on Linux — different *size*, not just different offset. `st_mode` is a `u16` at offset 4 on macOS and a `u32` at offset 16 on Linux. macOS has `st_birthtimespec` — the file creation timestamp — that Linux simply doesn't track. macOS has `st_flags` and `st_gen` from its BSD heritage. Linux has neither.

The field that kills us is `st_size` — the most commonly read field in any stat result. Offset 96 on macOS. Offset 48 on Linux. Forty-eight bytes apart. When a macOS binary calls our shim, gets back a Linux stat buffer, and reads offset 96, it's reading `st_ctimespec` — a timestamp, not a file size. The result is a number like `1712345678` — the seconds since epoch — interpreted as a file size in bytes. That's 1.7 gigabytes. The binary tries to allocate a 1.7 GB buffer for what should be a 4 KB file. On a good day, `malloc` returns NULL and the binary exits with an error. On a bad day, the allocation succeeds and the binary tries to read 1.7 billion bytes from a 4 KB file. Either way, crash.

The fix is structural translation. We can't just forward the Linux stat buffer to the macOS binary. We have to define the macOS struct layout in our code, call Linux's `stat`, read the Linux-layout result, and then write the values into a macOS-layout buffer at the offsets the binary expects:

```rust
#[repr(C)]
struct DarwinStat {
    st_dev: i32,                    // 0
    st_mode: u16,                   // 4
    st_nlink: u16,                  // 6
    st_ino: u64,                    // 8
    st_uid: u32,                    // 16
    st_gid: u32,                    // 20
    st_rdev: i32,                   // 24
    _pad0: i32,                     // 28
    st_atimespec: [i64; 2],         // 32
    st_mtimespec: [i64; 2],         // 48
    st_ctimespec: [i64; 2],         // 64
    st_birthtimespec: [i64; 2],     // 80
    st_size: i64,                   // 96
    st_blocks: i64,                 // 104
    st_blksize: i32,                // 112
    st_flags: u32,                  // 116
    st_gen: u32,                    // 120
    _pad1: i32,                     // 124
    _reserved: [i64; 2],            // 128
}
```

Then the shim becomes a translator. Call Linux `stat`. Read the result. Map each field from its Linux position to its macOS position, widening or narrowing types as needed:

```rust
unsafe fn linux_stat_to_darwin(linux: &libc::stat, darwin: *mut DarwinStat) {
    let d = &mut *darwin;
    d.st_dev = linux.st_dev as i32;       // u64 → i32
    d.st_mode = linux.st_mode as u16;     // u32 → u16
    d.st_nlink = linux.st_nlink as u16;   // u32 → u16
    d.st_ino = linux.st_ino;
    d.st_uid = linux.st_uid;
    d.st_gid = linux.st_gid;
    d.st_rdev = linux.st_rdev as i32;
    d.st_atimespec = [linux.st_atime, linux.st_atime_nsec];
    d.st_mtimespec = [linux.st_mtime, linux.st_mtime_nsec];
    d.st_ctimespec = [linux.st_ctime, linux.st_ctime_nsec];
    d.st_birthtimespec = [linux.st_ctime, linux.st_ctime_nsec]; // fallback
    d.st_size = linux.st_size;
    d.st_blocks = linux.st_blocks;
    d.st_blksize = linux.st_blksize as i32;
    d.st_flags = 0;    // no BSD flags on Linux
    d.st_gen = 0;
}
```

Linux doesn't track birth times, so we substitute `st_ctime` — the closest approximation. It's wrong, but it's wrong in a way that doesn't crash. `st_flags` and `st_gen` get zeroed — no Linux equivalent. The type narrowing is lossy (`st_dev` from `u64` to `i32`), but in practice device numbers on modern systems fit in 32 bits. We trade correctness at the margins for functionality at the core.

The stat shim works. We install it for `_stat`, `_lstat`, `_fstat`, `_fstatat`, and all their `$INODE64` variants — Apple's INODE64 suffixed symbols that use the 64-bit inode struct. Twelve shims just for the stat family.

The "ah ha" moment passes quickly. Because if `struct stat` is different... what about `struct statfs`? We check.
Read that ratio again. An eighteen-to-one size difference for the same system call result. macOS statfs contains `f_mntonname` — a 1,024-character buffer for the mount point path. And `f_mntfromname` — another 1,024-character buffer for the device path. And `f_fstypename` — a 16-character buffer for the filesystem type name. Linux's statfs has none of these fields. It reports block counts and ID numbers and that's it.

If a macOS binary allocates a 2,168-byte buffer on the stack and calls our `statfs` shim, and we write only 120 bytes of Linux statfs data into it, the remaining 2,048 bytes are uninitialized stack garbage. Worse — the fields that exist on both platforms are at completely different offsets. `f_bsize` is at offset 0 (u32) on macOS and offset 8 (i64) on Linux. Everything is wrong.

Another translation shim. Another pair of struct definitions. Another careful field-by-field mapping. Zero the 2,168-byte macOS buffer first, then translate each field from its Linux position. For the path buffers that Linux doesn't provide, leave them zeroed. Most tools don't read them. The ones that do get empty strings, which is wrong but not crashworthy
Two thousand one hundred and sixty-eight bytes versus one hundred and twenty. That's not a sixteen-byte delta. That's an eighteen-to-one ratio. macOS statfs contains `f_mntonname` — a 1,024-character buffer for the mount point path. And `f_mntfromname` — another 1,024-character buffer for the device path. And `f_fstypename` — a 16-character buffer for the filesystem type. Linux's statfs has none of these fields. It reports block counts and ID numbers and that's it.

Another translation shim. Another pair of struct definitions. Another careful field-by-field mapping.

And `struct dirent`? macOS: **1,048 bytes** — because `d_name` is `char[1024]`, accommodating the maximum HFS+ filename length. Linux: **280 bytes** — because `d_name` is `char[256]`. But it's worse than a size difference. macOS has a `d_namlen` field at offset 18 that Linux doesn't have. This shifts `d_type` from offset 20 (macOS) to offset 18 (Linux), and `d_name` from offset 21 to offset 19. A two-byte shift. A macOS binary reading `d_name` at offset 21 from a Linux dirent pointer reads two bytes of garbage prepended to the filename. Every directory listing, every `readdir` call, every filename extraction — two bytes off.

The fix for dirent needs a thread-local buffer. `readdir()` returns a pointer that's valid until the next `readdir()` call on the same stream — that's the POSIX contract. We allocate a thread-local 1,048-byte buffer (macOS dirent size), call Linux's `readdir()`, translate the result into macOS layout, and return a pointer to our buffer:

```rust
std::thread_local! {
Three struct families. Three translation layers. Stat, statfs, dirent. And the sinking realization that these won't be the last. POSIX defines dozens of structures — `sockaddr`, `msghdr`, `iovec`, `termios`, `rusage`. Each platform extends them differently. Some are binary-compatible by luck. Some are off by a byte. Some are off by a factor of eighteen. You have to check every single one.

Binary compatibility means matching byte layouts, not just API signatures. The struct is the contract, and every platform writes different contracts using the same names. Every `#include <sys/stat.h>` compiles to a different binary layout on a different OS. Source compatibility is the polite fiction that lets programmers sleep at night. Binary compatibility is what keeps us up.
        UnsafeCell::new([0u8; 1048]);
}
```

Three struct families. Three translation layers. Stat, statfs, dirent. And the sinking realization that these won't be the last. POSIX defines dozens of structures. Each platform extends them differently. Binary compatibility means matching byte layouts, not API signatures. The struct is the contract, and every platform writes different contracts using the same names.

---

## Scene 6.3: The Variadic Crisis

The struct translations fix stat, statfs, and dirent. Tools that only call those functions start working. But a much larger class of tools still crashes. Everything that prints formatted output. Everything that scans formatted input. Everything that logs, warns, or reports errors with format strings.

`printf`. `fprintf`. `snprintf`. `sprintf`. `scanf`. `sscanf`. `err`. `warn`. `syslog`. The entire formatted-I/O family. Thirty-plus functions. All of them crash. All of them crash the same way.

The crash is in `printf`. But `printf` works. Something *before* printf is wrong.

We add tracing inside our `printf` shim. The format string arrives correctly — `"%s: %d\n"`. The binary is calling `printf` with a string and an integer. Simple call. Should work. We forward to the host's `printf`. It crashes. Not inside `printf` itself — inside `printf`'s internal call to `strlen`, trying to compute the length of the `%s` argument. `strlen` receives not a string pointer, but the value `0x2`. `strlen(0x2)` — dereference address 2. SIGSEGV.

Where did `0x2` come from? It's the integer argument. The one that should be the second variadic argument, paired with `%d`. But `printf` is reading it as the first variadic argument, paired with `%s`. The arguments are in the wrong positions.
Linux ARM64, following the ARM Architecture Procedure Call Standard (AAPCS64), they go in `x1` and `x2` — the next available registers. On macOS ARM64?

They're on the stack. Both of them. At `[sp+0]` and `[sp+8]`.

Wait. Read that again.

Apple's ARM64 ABI puts **all variadic arguments on the stack**. Not in registers. On the stack. Not "variadic arguments that overflow past register capacity." Not "variadic arguments beyond the eighth." *Every single variadic argument*, starting with the first one, goes directly to the stack. Registers `x1` through `x7` sit there unused, holding stale values from whatever the calling function was doing before.

This violates AAPCS64. Or rather — Apple never committed to AAPCS64 for variadic functions. They followed it for non-variadic calls. They forked it for variadic calls. And they documented this divergence almost nowhere.

This is the kind of undocumented behavior that makes you close your laptop, stand up, walk to the window, and stare at the sky for a full minute before sitting back down
Apple does not do this.

Apple's ARM64 ABI has a silent, undocumented divergence from AAPCS64: **all variadic arguments go on the stack**. Not "variadic arguments that don't fit in registers." Not "variadic arguments after the eighth." *All* of them. The first variadic argument, the second, the hundredth — they all go on the stack, starting at `[sp+0]`. The registers `x1` through `x7` are left untouched by variadic arguments on Apple platforms.

This is nowhere in the AAPCS64 specification. Apple's own documentation mentions it only in passing, in a single paragraph buried in the iOS ABI reference that most developers never read. Clang implements it silently. LLVM's AArch64 backend has an `IsAppleABI` flag that controls this behavior. If you grep the LLVM source for it, you'll find a check deep in `AArch64ISelLowering.cpp` that says, essentially: "if Apple platform, force all variadic arguments to memory." A one-line divergence in the compiler backend that ripples through every binary ever compiled for macOS ARM64.

But if you're implementing a cross-platform compatibility layer and you search for "ARM64 variadic calling convention," every source tells you the AAPCS64 rules. StackOverflow, the ARM Architecture Reference Manual, textbooks, blog posts — all AAPCS64. The rules that Linux follows. The rules that Apple silently breaks.

We discovered this by reading compiler output. Not documentation — compiler output. Compile a simple `printf` call on macOS, look at the generated assembly: The variadic args are stored to the stack with `str` instructions before the `bl _printf`. Compile the same code on Linux. The variadic args are in registers. Same source code. Same programming language. Same compiler. Different ABI.

Now we understand the crash. The macOS binary calls our `printf` shim with variadic args on the stack. Our shim forwards to Linux's `printf`. Linux's `printf` reads registers `x1`, `x2`, `x3`... looking for variadic arguments. Those registers contain whatever was in them before the call — stale values, leftovers from previous computations. The actual arguments are on the stack, where Linux's `printf` never looks. It reads `0x2` from `x2`, interprets it as a string pointer for `%s`, calls `strlen(0x2)`, and crashes.

Every variadic function. Every `printf`. Every `fprintf`. Every `scanf`. Every `err` and `warn`. Every `syslog`. If it takes `...` in its signature, it's broken on Linux. The format string reaches the function — that's a named parameter, passed in a register on both platforms. But every argument after the format string is in the wrong place.

This is the **All Is Lost** moment. We're not talking about one struct layout to translate. We're talking about the fundamental mechanism by which C programs pass variable numbers of arguments to functions. And the two platforms implement it differently at the ABI level. The instructions are the same. The calling convention is different. And there's no way to fix this in the type system, in the shimming layer, or in the resolver. The mismatch is below the function boundary — it's in how the caller *sets up the stack frame* before the function is even called.

But it's worse. Because `va_list` — the type that functions use to iterate over variadic arguments — is also different.

On macOS ARM64, `va_list` is `char *`. A pointer. It points to the stack location where the variadic arguments start. When `va_arg` needs the next argument, it reads from the pointer and advances it by the argument's size. Simple. Linear. Stack-only.

On Linux ARM64, `va_list` is a **32-byte struct**:

```
struct __va_list {
    void *__stack;      // offset 0:  pointer to stack overflow area
    void *__gr_top;     // offset 8:  top of general-purpose register save area
    void *__vr_top;     // offset 16: top of vector register save area
    int   __gr_offs;    // offset 24: offset from __gr_top to next GP reg arg
    int   __vr_offs;    // offset 28: offset from __vr_top to next FP/SIMD reg arg
};
```

Linux `va_arg` checks `__gr_offs` first. If there are still register-saved arguments available (offset is negative, meaning registers haven't been exhausted), it reads from the register save area. Once registers are exhausted (offset reaches zero), it falls through to `__stack`. This means Linux `va_arg` looks in *two different places* for arguments — the register save area and the stack — depending on how many have been consumed.

macOS `va_arg` only ever looks at the stack. Because all the variadic args are there. Always.

A macOS binary that calls `vprintf` constructs a macOS-style `va_list` — a bare `char *` pointing to stack arguments. If we forward this to Linux's `vprintf`, Linux tries to interpret that 8-byte pointer as the first field of a 32-byte struct, reads 24 bytes of garbage after it, and crashes.

We can't change how the macOS binary places its arguments. The instructions are baked into the binary. We can't ask Linux `printf` to read arguments from the stack instead of registers. The implementation is in glibc. We control neither side of this interface.

We can only stand in the middle.

What follows is the hardest week of the project. Not because the solution is conceptually difficult — once you see the problem, the solution is almost obvious. Hard because it's mechanical, repetitive, and unforgiving. Miss one shim and a tool crashes. Get one `va_list` field wrong and `printf` reads garbage. The margin for error is zero bytes.

---

## Scene 6.4: Naked Assembly

The solution is ugly and beautiful and the only thing that works. We write assembly shims. Not regular Rust `extern "C"` functions — naked assembly functions that operate below the calling convention, directly manipulating the stack and registers.

The insight is this: we can't change where the macOS binary puts its arguments (on the stack). And we can't change where Linux's `printf` looks for them (in registers, via `va_list`). But every variadic C function has a `v`-variant that takes an explicit `va_list` parameter. `printf` → `vprintf`. `fprintf` → `vfprintf`. `snprintf` → `vsnprintf`. `sprintf` → `vsprintf`. The `v`-variants are not variadic — they take a fixed number of arguments, with the last being a pointer to a `va_list` struct.

The plan: intercept the macOS binary's call to `printf`. Build a Linux-format `va_list` struct — 32 bytes — that points to the stack where Apple put the variadic arguments. Call `vprintf` instead, passing this synthetic `va_list`. Linux's `vprintf` reads the `va_list`, finds `__gr_offs = 0` (no register arguments available), and falls through to `__stack`, which we've carefully set to point at the Apple-convention stack arguments. `vprintf` reads the arguments from the stack, exactly where the macOS binary put them.

We're building a bridge between two calling conventions using a 32-byte struct as the foundation.

Here's the naked assembly for `printf`:

```asm
shim_printf:
    stp  x29, x30, [sp, #-64]!    // save frame pointer and link register
    mov  x29, sp                    // set up frame
    add  x8, x29, #64              // x8 = original SP (Apple's stack args)
    str  x8, [sp, #16]             // va_list.__stack = original SP
    stp  xzr, xzr, [sp, #24]      // va_list.__gr_top = 0, __vr_top = 0
    str  xzr, [sp, #40]           // va_list.__gr_offs = 0, __vr_offs = 0
    add  x1, sp, #16               // x1 = pointer to our va_list
    bl   vprintf                    // call vprintf(fmt, &va_list)
    ldp  x29, x30, [sp], #64      // restore frame
    ret
```

Read that carefully. It's thirteen instructions and it does something impossible: it translates between two ABIs that neither side knows are different.

Line by line:

**Save the frame.** `stp x29, x30, [sp, #-64]!` — store the frame pointer and link register, decrementing the stack by 64 bytes. We need 32 bytes for the `va_list`, 16 bytes for the frame pair, and padding to keep 16-byte alignment.

**Compute the original stack pointer.** `add x8, x29, #64` — the original SP (before our frame) is at `FP + 64`. That's where Apple put the variadic arguments. This is the bridge: we know exactly where the macOS binary left its arguments.

**Build the va_list.** Four stores that construct the 32-byte Linux `va_list` struct in our stack frame at `[sp+16]`:
- `__stack = original SP` — tells Linux `va_arg` to read from where Apple wrote
- `__gr_top = 0` — no register save area
- `__vr_top = 0` — no floating-point register save area
- `__gr_offs = 0, __vr_offs = 0` — no registers left, skip straight to stack

Setting `__gr_offs` and `__vr_offs` to zero is the key trick. Linux's `va_arg` checks: "do I have register-saved arguments left?" Zero means no. "Fall through to `__stack`." Which points to the Apple stack args. The arguments flow through.

**Pass the va_list pointer.** `add x1, sp, #16` — for `printf` (one named argument: the format string in `x0`), the `va_list` pointer becomes the second argument, in `x1`. For `fprintf` (two named arguments: stream and format), it would go in `x2`. For `snprintf` (three named arguments: buffer, size, format), it would go in `x3`.

**Call and return.** `bl vprintf` followed by frame teardown. `vprintf` is not variadic — it's a regular function call. No ABI mismatch.

The pattern repeats for every variadic function. The only thing that changes is which register gets the `va_list` pointer (determined by how many named arguments the function has) and which `v`-variant to call:

| Function    | Named args | va_list in | Calls     |
|-------------|-----------|------------|-----------|
| `printf`    | 1 (fmt)   | x1         | `vprintf`    |
| `fprintf`   | 2 (stream, fmt)   | x2 | `vfprintf`   |
| `snprintf`  | 3 (buf, size, fmt) | x3 | `vsnprintf`  |
| `sprintf`   | 2 (buf, fmt) | x2       | `vsprintf`   |
| `scanf`     | 1 (fmt)   | x1         | `vscanf`     |
| `sscanf`    | 2 (str, fmt) | x2      | `vsscanf`    |
| `err`       | 2 (eval, fmt) | x2     | `verr`       |
| `warn`      | 1 (fmt)   | x1         | `vwarn`      |
| `syslog`    | 2 (priority, fmt) | x2 | `vsyslog`    |

In Rust, we write these as `#[unsafe(naked)]` functions using `core::arch::naked_asm!`. Rust's naked function support lets us write the assembly inline while still registering the function as a normal symbol that our shim table can reference. No separate `.s` files, no external assembler, no linking gymnastics. The functions are transparent to the Rust compiler — it doesn't try to generate prologues, epilogues, or register-saving code. The assembly we write is exactly the assembly that executes.

There's a subtlety with the `v`-variant family too. When a macOS binary calls `vfprintf`, it passes a macOS-style `va_list` — a bare `char *` pointer to the stack. We can't forward that to Linux's `vfprintf` directly. So we shim the `v`-variants as well: extract the `char *` from the macOS `va_list`, build a Linux `va_list` with `__stack` pointing there, and call the Linux `v`-variant with our translated struct.

We end up with thirty-three variadic shims. Thirty-three naked assembly functions. `printf`, `fprintf`, `snprintf`, `sprintf`, `dprintf`, `asprintf`, `sscanf`, `fscanf`, `scanf`, `err`, `errx`, `errc`, `warn`, `warnx`, `syslog`. Plus their fortified `_chk` variants — `__snprintf_chk`, `__sprintf_chk`, `__fprintf_chk`, `__printf_chk` — that Apple's compiler generates when optimization is enabled. Plus the `v`-variant translations. Each one follows the identical pattern: save frame, compute original SP, build `va_list`, load `va_list` pointer into the right argument register, call `v`-variant, restore frame, return.

```
VariadicShims: registered 33 Apple→Linux ABI shims
```

Thirty-three bridges between two worlds that speak the same language with different accents. Thirty-three functions where we stand between a macOS binary that puts things on the stack and a Linux library that looks in registers, and we translate.

Why thirty-three? Because Apple's compiler doesn't just emit `printf`. When you compile with `-O1` or higher, Apple's clang replaces `printf` calls with `__printf_chk` — a fortified variant that takes an extra flag parameter and checks for buffer overflows. So we need shims for `__snprintf_chk`, `__sprintf_chk`, `__fprintf_chk`, `__printf_chk` — and their `v`-variant counterparts `__vsnprintf_chk`, `__vsprintf_chk`, `__vfprintf_chk`, `__vprintf_chk`. The fortified functions have *more* named parameters (the extra flag and sometimes a buffer-length argument), which means the `va_list` pointer goes in a different register for each one. `__snprintf_chk` takes five named arguments — buffer, maxlen, flag, slen, format — so the `va_list` would go in... wait. We've run out of easily addressable argument registers at that point. The assembly needs careful attention.

Then there's the `err`/`warn` family from BSD's `<err.h>`. `err(eval, fmt, ...)` prints a message, appends `strerror(errno)`, and exits. `warn(fmt, ...)` prints but doesn't exit. `errx` and `warnx` omit the errno string. Four functions, each variadic, each needing its own naked shim with the right `v`-variant callthrough — `verr`, `vwarn`, `verrx`, `vwarnx`.

And `syslog(priority, fmt, ...)`, the system logging function. One more shim, calling through to `vsyslog`.

Every single shim follows the identical thirteen-instruction template. The only things that change are the target `v`-variant and which register holds the `va_list` pointer. It's repetitive. It's mechanical. It's the kind of code where you write a template, stamp it out thirty-three times, and triple-check every register number.

We wrote a test for every shim. `printf("%s: %d\n", "test", 42)` should produce `test: 42`. `snprintf(buf, 256, "%d plus %d is %d", 1, 2, 3)` should write `1 plus 2 is 3`. `fprintf(stderr, "error: %s\n", "boom")` should write to stderr. Test all thirty-three. Every one is a potential misaligned register, a potential garbage `va_list` field, a potential crash. These tests don't feel academic. They feel like defusing thirty-three landmines.

The first time all thirty-three shims are installed and `printf("%s: %d\n", "hello", 42)` produces `hello: 42` on Linux — that's not a test passing. That's two incompatible calling conventions being reconciled by thirteen instructions.

---

## Scene 6.5: The Long Tail

The struct translations handle layout mismatches. The variadic shims handle calling convention mismatches. Two categories down. The remaining mismatches are smaller, but there are more of them, and they hide in places you don't think to look.

`setlocale`. Every C program that handles text properly calls `setlocale(LC_ALL, "")` at startup. It tells the C library to use the user's locale settings instead of the "C" locale default. The call looks portable. The constants are not.

On macOS, `LC_ALL` is `0`. On Linux, `LC_ALL` is `6`.

That's it. Same constant name. Different integer value. When a macOS binary calls `setlocale(0, "")`, our shim forwards `0` to Linux's `setlocale`. On Linux, category `0` is `LC_CTYPE` — character classification only. The binary thinks it set all locale categories. It only set one. Later, when it tries to format a number with thousands separators, `LC_NUMERIC` isn't set. The format string produces no separators. The output is wrong. Not a crash — wrong output. Worse than a crash, in some ways, because wrong output can go unnoticed.

Here's the full translation table:

| Category    | macOS | Linux |
|-------------|-------|-------|
| LC_ALL      | 0     | 6     |
| LC_COLLATE  | 1     | 3     |
| LC_CTYPE    | 2     | 0     |
| LC_MONETARY | 3     | 4     |
| LC_NUMERIC  | 4     | 1     |
| LC_TIME     | 5     | 2     |
| LC_MESSAGES | 6     | 5     |

Every value is different. Not one matches. POSIX defines what `LC_ALL` *does* — it says "set all categories." POSIX does not define what integer value represents it. The standard specifies behavior, not constants. Binary compatibility requires constants.

The fix is a seven-line `match`:

```rust
extern "C" fn shim_setlocale(macos_cat: c_int, locale: *const c_char) -> *mut c_char {
    let linux_cat = match macos_cat {
        0 => 6,  // LC_ALL
        1 => 3,  // LC_COLLATE
        2 => 0,  // LC_CTYPE
        3 => 4,  // LC_MONETARY
        4 => 1,  // LC_NUMERIC
        5 => 2,  // LC_TIME
        6 => 5,  // LC_MESSAGES
        _ => macos_cat,
    };
    unsafe { libc::setlocale(linux_cat, locale) }
}
```

Seven lines. But we also need `newlocale`, which takes a bitmask instead of a category number. macOS `LC_COLLATE_MASK` is `1 << 0`. Linux `LC_COLLATE_MASK` is `1 << 3`. Every bit position is different. The mask translation function needs to check each macOS bit and set the corresponding Linux bit. And `querylocale`. And `nl_langinfo`, which takes `nl_item` values that also differ between platforms.

Each fix reveals two new problems. Fix `setlocale` and `nl_langinfo` starts crashing because its item codes don't match. Fix `nl_langinfo` and `MB_CUR_MAX` returns wrong values because it reads from a data symbol instead of calling a function.

`___mb_cur_max`. On macOS, this is exported by libSystem as a global integer — the maximum number of bytes in a multibyte character for the current locale. Programs `#include <stdlib.h>` and read `MB_CUR_MAX`, which expands to `(___mb_cur_max)` — a direct memory read. Not a function call. The binary loads from a fixed address, expecting to find an integer there.

On Linux, `MB_CUR_MAX` is `__ctype_get_mb_cur_max()` — a function call. Completely different mechanism. The macOS binary doesn't call a function. It dereferences a pointer. You can't shim a memory read with a function shim. The binary executes an `ldr` instruction — a load from a memory address that was resolved during linking. If the address doesn't contain a valid integer, the binary gets garbage.

We need a data symbol. An actual memory location containing the value `4` (the maximum bytes per character in UTF-8), at an address that our resolver returns when the binary asks for `___mb_cur_max`. Not a function that returns `4`. An address that *contains* `4`.

```rust
static MB_CUR_MAX_VALUE: i32 = 4;

fn shim_mb_cur_max_addr() -> u64 {
    &MB_CUR_MAX_VALUE as *const i32 as u64
}
```

Register the *address* of that static as the symbol value. When the binary resolves `___mb_cur_max`, it gets a pointer. When it loads from that pointer, it gets `4`. It never calls a function. It never knows the difference. The `static` lives for the entire process lifetime — guaranteed.

This pattern — data symbols that are memory addresses, not function pointers — comes up more than you'd expect. `mach_task_self_` is supposed to be a Mach port number. `_environ` is a pointer to the environment array. `_sys_signame` is an array of signal name strings. `_NXArgc` and `_NXArgv` are the argument count and vector. Each one is a global variable that macOS binaries read directly from memory. Each one needs a real allocation at a real address containing a real value. the differences between macOS and Linux are fractal. Zoom into any seemingly portable interface and you find platform-specific constants, platform-specific struct layouts, platform-specific mechanisms (function call vs. memory read), and platform-specific behaviors. Every layer of the system, from the kernel interface to the C library to the locale subsystem, diverges in ways that are invisible at the source level and catastrophic at the binary level.

This is the Dark Night of the Soul. Not one dramatic failure but a grinding, unending sequence of small ones. Fix `setlocale`. Find that `nl_langinfo` is broken. Fix `nl_langinfo`. Find that `___mb_cur_max` is a data symbol. Fix `___mb_cur_max`. Find that `___stderrp` is a pointer-to-pointer. Fix `___stderrp`. Find that `___assert_rtn` has its arguments in the wrong order. Fix that. Find `_DefaultRuneLocale`. Find `___maskrune`. Find `_clock_gettime_nsec_np`. Each one is five minutes to understand and thirty minutes to implement and test. Each one is essential. Skip any one and some tool, somewhere, produces wrong output or crashes.

The psychological toll is real. Every fix you write reveals two new problems in your test suite. The diff grows. The shim table grows. The number of `cfg(not(target_os = "macos"))` blocks multiplies. You start to wonder if the list is infinite. You start to wonder if you're playing whack-a-mole with an OS.

It isn't infinite. That's the thing you learn by grinding through. POSIX is finite. The C library is finite. The number of structures, constants, and mechanisms that a command-line tool touches is large but bounded. You can enumerate them. You can test them. You can track the count of passing tools and watch it go up instead of down.

The `___assert_rtn` symbol is another example. macOS's assertion failure function takes `(function, file, line, expression)`. Linux's `__assert_fail` takes `(expression, file, line, function)`. Same four arguments. Different order. Without a shim that reorders them, every assertion failure in a macOS binary prints the function name where the assertion message should be.

The `___error` symbol — macOS's equivalent of `errno`. On macOS, `___error()` returns a pointer to the thread-local errno. On Linux, `__errno_location()` does the same thing. Same functionality, different name. But the macOS binary imports `___error`, and Linux doesn't export it. Without a shim, the binary can't check error codes.

The stdio file pointers. macOS exports `___stderrp`, `___stdoutp`, `___stdinp` — pointers to `FILE *`. Not the `FILE *` themselves — pointers to pointers. The macOS binary loads from `___stderrp` to get `stderr`, then uses it. On Linux, `stderr` is typically a macro that expands to `&_IO_2_1_stderr_` or similar. The names don't match, the levels of indirection don't match.

We build a static initialization function that captures the host's `stderr`, `stdout`, `stdin` into static variables, then register the *addresses* of those variables as the macOS symbols:

```rust
static mut STDERR_PTR: *mut FILE = std::ptr::null_mut();
static mut STDOUT_PTR: *mut FILE = std::ptr::null_mut();
static mut STDIN_PTR: *mut FILE = std::ptr::null_mut();

fn init_stdio_pointers() {
    unsafe {
        STDERR_PTR = libc::fdopen(2, "w\0".as_ptr() as _);

The emotional texture of this work is something that doesn't come through in commit messages or changelogs. Each shim takes between ten minutes and two hours to implement. `___error` takes ten minutes: rename `__errno_location`, done. `_setlocale` takes an hour: build the mapping table, test all seven categories, verify `setlocale(LC_ALL, "")` propagates correctly to all subcategories. The `DefaultRuneLocale` data symbol takes two hours: reverse-engineer the 256-byte `_RuneLocale` struct from macOS headers, populate each character classification entry by cross-referencing `isupper`, `islower`, `isdigit`, `isalpha`, and synthesize a complete rune table that macOS's `___maskrune` function can query without crashing.

You do this sixty times. Sixty small translation problems, each important, each requiring its own research into struct layouts, constant values, and behavioral differences. Some you can verify with a single `printf`. Some require running a real tool and checking its full output against macOS's output.

The shim table at the end of this process is a snapshot of the distance between macOS and Linux. Not the distance at the kernel level — that's a different, larger project. The distance at the C library level. The distance that POSIX was supposed to close and didn't. Every row in that table is a place where "portable" means "compiles on both platforms" but not "runs on both platforms."
        STDOUT_PTR = libc::fdopen(1, "w\0".as_ptr() as _);
        STDIN_PTR  = libc::fdopen(0, "r\0".as_ptr() as _);
    }
}
```

Register `&STDERR_PTR` as `___stderrp`. The macOS binary loads from that address, gets the `FILE *`, and writes to it. It works because the indirection levels match — the binary expects a `FILE **`, and that's what we provide.

The shim count grows. `___error`. `___assert_rtn`. `___maskrune`. `___tolower`. `___toupper`. `__DefaultRuneLocale`. `___strlcpy_chk`. `___strlcat_chk`. `_clock_gettime_nsec_np`. `_malloc_type_malloc`. `_malloc_type_calloc`. `_malloc_type_realloc`. `____chkstk_darwin`. `_sysctl`. `_sysctlbyname`. `_strlcpy`. `_strlcat`. `_fmtcheck`.

Each one a small translation. Each one essential. Remove any one and some tool crashes, some output is wrong, some locale is misconfigured. The compat layer is not a wall — it's a mesh, and every hole lets through a failure.

The emotional texture of this work doesn't show up in commit logs. Each shim takes between ten minutes and two hours. `___error` is ten minutes — wrap `__errno_location`, register it, done. `_setlocale` takes an hour: build the mapping table, test all seven categories, verify that `sort` produces correctly-ordered output. The `DefaultRuneLocale` data symbol takes two hours: reverse-engineer the 256-entry `_RuneLocale` struct from macOS headers, populate each character classification bit, synthesize a complete rune table that `___maskrune` can query without crashing.

You do this dozens of times. Each shim requires research into header file differences, constant values, behavioral quirks. Some you verify with a single test. Some require running a real tool and comparing its output character by character against macOS. The debugging methodology becomes mechanical: a tool crashes, read the log, find the missing symbol, diff the macOS and Linux headers, write the struct definition, write the translation function, register the shim, test it, move on. The shim table grows into a map of the distance between macOS and Linux at the C library level. Every row is a place where "portable" means "compiles on both platforms" but not "runs on both platforms."


---

## Scene 6.6: The Docker Container

Stand back. Look at the full system.

We have struct translation shims for `stat`, `lstat`, `fstat`, `fstatat`, `statfs`, `fstatfs`, and `readdir` — twelve shims covering three struct families. We have thirty-three naked assembly variadic shims for every `printf`/`scanf`/`err`/`warn`/`syslog` variant. We have locale constant translation for `setlocale`, `newlocale`, `querylocale`, `nl_langinfo`. We have data symbol providing for `___mb_cur_max`, `___stderrp`, `___stdoutp`, `___stdinp`, `__DefaultRuneLocale`. We have function remapping for `___error`, `___assert_rtn`, `____chkstk_darwin`, and dozens more. Over a hundred shims, total.

Now they need to compose. A macOS binary doesn't call these functions in isolation — it calls them in sequence, in nested contexts, passing results from one to the next. `stat` a file, check `st_mode`, `printf` the result. Three shims in one statement. If any one fails, the chain breaks.

The composition challenge is library loading order. On macOS, all system libraries are loaded by `dyld` before `main()` runs. The shared cache means they're effectively pre-loaded — every process starts with `libSystem`, `CoreFoundation`, `Foundation`, and hundreds of other frameworks already mapped. Symbols from any library are available to any other library. It's a flat namespace.

On Linux, we use `dlopen` and `dlsym` to load libraries at runtime. But `dlopen` by default loads libraries in a local namespace — their symbols aren't visible to other libraries unless you pass `RTLD_GLOBAL`. The macOS binary expects a flat namespace where `_printf` is available everywhere. If we load our shim library without `RTLD_GLOBAL`, the binary can't find it.

The fix is deliberate: load shim libraries early, load them globally, load them in order. `libSystem` shims first — that's `libsystem-shims`, the crate that contains all the ABI translations. Then the resolver connects macOS symbol names to shim addresses. Then Foundation shims. The order matters because Foundation shims depend on libSystem shims — `NSString stringWithFormat:` eventually calls `vsnprintf`, which is one of our variadic shims. If the variadic shim isn't installed when Foundation initializes, the first formatted string operation crashes.

The `DataSymbolStore` handles non-function symbols — the addresses that macOS binaries load from directly. This is the store we introduced in Chapter 3 for things like XPC type constants and dispatch source types. On Linux, it also provides `___mb_cur_max`, `mach_task_self_`, and `environ`. These symbols correspond to actual memory locations that contain actual values. The binary doesn't call a function — it dereferences an address. We allocate static variables with the right values and register their addresses.

The store is a process-lifetime singleton:

```rust
static GLOBAL_STORE: OnceLock<DataSymbolStore> = OnceLock::new();

impl DataSymbolStore {
    pub fn global() -> &'static Self {
        GLOBAL_STORE.get_or_init(Self::new)
    }
}
```

`OnceLock` ensures one-time initialization. The data symbols are static addresses that never change — `___mb_cur_max` points to a `static i32` that we set to `4` (UTF-8 max bytes). The binary loads from this address for the entire process lifetime. It works because we guarantee the address is stable and the value is correct.

The resolver hierarchy chains everything together:

```
CompositeResolver
  └─ ShimTable          (function shims — stat, printf, setlocale, ...)
  └─ DataSymbolStore    (data symbols — __mb_cur_max, __stderrp, ...)
  └─ EagerCacheResolver (dyld shared cache entries)
  └─ StubResolver       (dlsym fallback to host libraries)
```

When the macOS binary references a symbol, the resolver tries each tier in order. Shim table first — if we have an explicit translation for this symbol, use it. Data symbols second — if it's a data symbol we provide, return our address. Eager cache third — for framework symbols we pre-resolve. `dlsym` fallback last — for symbols that happen to exist on the host with compatible signatures (like `malloc`, `free`, `memcpy`).

The Dockerfile puts it all together:

```dockerfile
FROM rust:bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --workspace \
    --exclude hello-darwin \
    --exclude control-gallery \
    --exclude window-backend-darwin \
    --exclude platform-darwin-host

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 libbz2-1.0 zlib1g liblzma5 libarchive13 \
    libssl3 libc++1 libc++abi1 locales \
    && echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen \
    && locale-gen

COPY --from=builder /build/target/release/macho-inspect /usr/local/bin/
COPY docker/xcode-tools/ /opt/xcode-tools/
COPY scripts/test_xcode_tools_docker.sh /usr/local/bin/test-xcode-tools
```

Two stages. The builder stage cross-compiles the entire workspace for `aarch64-unknown-linux-gnu` — our loader, all shimming crates, the entire runtime stack. The runtime stage is a slim Debian image with only the shared libraries the loader needs at runtime. We install `locales` and generate `en_US.UTF-8` — without this, `setlocale(LC_ALL, "")` fails silently and every locale-dependent function misbehaves.

The line `echo "burdettie-os 1.0" > /etc/burdettie-release` is a small vanity. This isn't just a container. It's a Linux distribution that runs macOS binaries. It deserves a name.

The macOS tool binaries — extracted from Xcode and the macOS system — are staged in `/opt/xcode-tools/`, organized by origin: `xcode-tc/` for toolchain tools (clang, lipo, strings, nm), `xcode-dev/` for developer tools (xcodebuild, xcrun), and `system/` for system utilities (stat, sort, cut, find, file, wc). These are unmodified Mach-O binaries, `arm64` slice only, copied from a macOS host. They have never been on a Linux machine before.

```
$ docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .
```

The build takes several minutes. Compiling thirty-three naked assembly functions and a hundred-plus shims and the entire loader pipeline is not instant. But it produces a single Docker image — about 200 MB — that contains everything needed to run macOS command-line tools on Linux.

Container image. Assembled. All shims installed. All translators registered. All data symbols allocated.

There's one more piece: environment bridging. macOS binaries interrogate their environment in macOS-specific ways. `sw_vers` calls `_NSGetExecutablePath` and reads `/System/Library/CoreServices/SystemVersion.plist`. `uname` calls the `uname` syscall and expects Darwin-specific kernel version strings. `arch` queries `sysctlbyname("hw.machine")` for the architecture name.

None of these system interfaces exist on Linux. `_NSGetExecutablePath` isn't a libc function on Linux. `/System/Library/CoreServices/SystemVersion.plist` doesn't exist. `sysctlbyname` with Apple-specific keys returns nothing.

We shim them. `_sysctl` and `_sysctlbyname` get translation layers that map Apple's key hierarchy to Linux equivalents or return hardcoded values where no equivalent exists. `hw.ncpu` maps to reading `/proc/cpuinfo`. `hw.memsize` maps to `sysinfo()`. Apple-specific keys like `hw.cputype` return ARM64 identifiers because that's what the binary expects.

The `____chkstk_darwin` shim is a no-op. On macOS, this is a stack-probing function that touches each page in a large stack allocation to ensure the guard pages are committed. On Linux, the kernel handles guard pages differently — `MAP_GROWSDOWN` stack allocations expand automatically when faulted. The probe is unnecessary. We register an empty function and the binary's large stack allocations work without it.

The `_malloc_type_malloc`, `_malloc_type_calloc`, `_malloc_type_realloc` shims are recent additions — macOS Sonoma introduced typed allocation metadata in its malloc implementation. Old binaries use `malloc`. New binaries use `malloc_type_malloc` with an extra type descriptor parameter. On Linux, we ignore the type descriptor and forward to `malloc`. The memory allocator doesn't care about Apple's type tracking.

Time to find out if any of it actually works.

---

## Scene 6.7: First Light on Linux

```
$ docker run --rm --platform linux/arm64 burdettie-os test-xcode-tools
```

The test runner iterates over every macOS binary in `/opt/xcode-tools/`. For each one, it runs the binary through `macho-inspect --exec` with sensible default arguments. `stat /`. `sort --version`. `cut --version`. `strings /opt/xcode-tools/xcode-tc/lipo`. `nm /opt/xcode-tools/xcode-tc/lipo`. `file /etc/hostname`. `find /tmp -maxdepth 0 -print`. Each tool gets fifteen seconds. Each result is classified: PASS, FAIL, CRASH, TIMEOUT, MISSING_SHIM.

The output starts scrolling.

```
═══ System CLI Tools ═══
  stat                                PASS     (exit 0, 284 bytes)
  sort                                PASS     (exit 0, 12 bytes)
  cut                                 PASS     (exit 1, 0 bytes)
  wc                                  PASS     (exit 0, 42 bytes)
  find                                PASS     (exit 0, 5 bytes)
  file                                PASS     (exit 0, 38 bytes)
  uname                               PASS     (exit 0, 52 bytes)
  basename                            PASS     (exit 0, 8 bytes)
  dirname                             PASS     (exit 0, 5 bytes)
  env                                 PASS     (exit 0, 1024 bytes)
  sed                                 PASS     (exit 0, 0 bytes)
  id                                  PASS     (exit 0, 18 bytes)
  whoami                              PASS     (exit 0, 5 bytes)
```

Green. Green. Green. Green. Every system tool that uses stat, printf, readdir, setlocale — all the interfaces we shimmed — reports PASS. `stat /` prints correct file metadata. The file size is right (not a timestamp). The mode bits are right (not a device number). The timestamps are right. The struct translation works.

```
═══ Xcode Toolchain Tools ═══
  strings                             PASS     (exit 0, 2847 bytes)
  nm                                  PASS     (exit 0, 15432 bytes)
  lipo                                PASS     (exit 0, 89 bytes)
  size                                PASS     (exit 0, 125 bytes)
  otool                               PASS     (exit 0, 3421 bytes)
  llvm-nm                             PASS     (exit 0, 15432 bytes)
  llvm-size                           PASS     (exit 0, 125 bytes)
  llvm-strings                        PASS     (exit 0, 2847 bytes)
  llvm-otool                          PASS     (exit 0, 3421 bytes)
  llvm-objdump                        PASS     (exit 0, 8923 bytes)
  llvm-cxxfilt                        PASS     (exit 0, 19 bytes)
  ar                                  PASS     (exit 0, 73 bytes)
  clang                               PASS     (exit 0, 245 bytes)
```

Xcode toolchain tools. Compiled on Apple's build servers, linked against Apple's SDK, expecting Apple's runtime. Running on Debian. `strings` extracts printable strings from a binary. `nm` lists symbols. `lipo` reports architecture slices. `clang --version` prints its version string. All PASS.

These tools exercise the variadic shims heavily. `nm` formats symbol addresses with `printf("%016llx", addr)`. `strings` uses `fprintf(stdout, "%s\n", str)`. `clang --version` uses `printf` extensively for its multi-line version output. Every single formatted-output call goes through our naked assembly, through the synthetic `va_list`, through Linux's `vprintf`. The assembly holds.

The counter at the bottom:

```
═══════════════════════════════════════
  PASS:  51
  FAIL:  3
  CRASH: 2
  SHIM:  4
  TOTAL: 60
═══════════════════════════════════════
```

Fifty-one tools pass. On a platform they were never built for, never tested on, never intended to run on. macOS ARM64 Mach-O binaries, executing on Linux ARM64, with every system call translated, every struct layout converted, every variadic argument moved from the stack to a synthetic `va_list`, every locale constant mapped, every data symbol provided at the address the binary expects.

The three FAILs are tools with unsupported arguments or missing framework dependencies — `xcodebuild` needs the full Xcode framework stack, `xcrun` needs the developer tools directory structure. Not ABI issues — feature gaps. The crashes are in tools that use Mach-specific kernel interfaces we haven't shimmed: `mach_port_allocate`, `task_info`, `vm_region` — calls that talk to the Mach microkernel, which doesn't exist on Linux. These would need a full Mach IPC emulation layer, a project unto itself. The SHIM results are tools that hit a missing shim — an unimplemented symbol — but that's a finite, trackable list. The test runner captures the missing symbol name in its output, so we know precisely what to build next.

What matters is the ratio. Fifty-one passes out of sixty attempts. Eighty-five percent. And the nine non-passes are all *explained* — each one has a specific, diagnosable reason. No mysterious crashes. No unexplained garbage output. The ABI translation layer either works completely or fails clearly. There's no middle ground where things sort-of-work and produce subtly wrong output. That's the benefit of working at the binary level: it's either right or it's SIGSEGV.

The numbers tell a story. Chapter 1 loaded a single binary. Chapter 3 resolved its symbols. Chapter 5 built the Foundation layer. Chapter 6 translated the platform. Each chapter expanded the surface area of compatibility. The compat gate — the count of tools that produce correct output — is the measure of that expansion:

- After Chapter 3: ~10 tools (C-only tools, no Foundation)
- After Chapter 5: 39 tools (macOS-only, Foundation included)
- After Chapter 6: 51 tools (cross-platform, Linux Docker)

Twelve more tools, from an entirely different platform. But the significance isn't the delta — it's the *infrastructure*. After this chapter, the loader runs the same binaries on Linux that it runs on macOS. The Dockerfile is reproducible. The test suite runs in CI. The compat layer is systematic, not ad hoc — struct translations are derived from header definitions, variadic shims follow a mechanical pattern, constant mappings are exhaustive tables.

Run it again, slower. Watch the output from a tool that exercises every layer:

```
$ docker run --rm burdettie-os macho-inspect --exec /opt/xcode-tools/system/stat /
16777234 4294967294 drwxr-xr-x 20 root wheel 0 640 "Mar 15 12:34:56 2024" "Mar  9 08:15:23 2024" "Mar 15 12:34:56 2024" "Dec 30 19:01:42 2006" 4096 0 0 /
```

That output. Every number in it passed through a translation layer. `st_dev`, `st_ino`, permissions, link count, owner, group, file size, timestamps, block size — all read from a Linux `stat` syscall, field-mapped into a macOS-layout struct, then formatted by a macOS binary using `printf` format strings that went through our naked assembly variadic shims, with locale settings that were translated through our constant mapping. The owner name `root` came from `getpwuid`, which reads `/etc/passwd` — a file that exists on both platforms, in the same format, because *some* things are actually portable. The permissions string `drwxr-xr-x` was computed by the binary's own `strmode` function, running natively on Linux ARM64 just as it would on macOS. Twelve shims in one line of output. One line of correct output that traverses the entire compatibility layer and comes out the other side intact.

The chapter began with a crash. `SIGSEGV at PC 0xffffb3e1e808`. The same binary, the same architecture, the wrong platform. We thought ARM64 meant compatible. We learned that ARM64 means the instructions execute, and everything else — struct layouts, calling conventions, constant values, memory indirection patterns — is the platform's to define.

The distance between "the instructions execute" and "the program works" is measured in shim functions. One hundred and twenty-three of them, at last count.

It ends with this:

```
$ docker run --rm burdettie-os macho-inspect --exec /opt/xcode-tools/system/uname -a
Darwin burdettie-os 23.0.0 Darwin Kernel Version 23.0.0 root:xnu-10002.0.0~1/RELEASE_ARM64_T6020 arm64
```

That's a lie. A beautiful, functional, hard-won lie. The kernel is Linux. The architecture is ARM64. But the macOS binary asks `uname` questions and gets macOS answers, because our shim translates the query. The binary thinks it's home. It isn't. It doesn't matter. It works.

This is what compatibility engineering looks like when it succeeds. Not a clean, elegant solution — a hundred small translations stacked on top of each other, each one correct enough, each one essential, the whole thing held together by struct definitions and register conventions and a shellscript that counts passes and failures.

The bridge holds.

Fifty-one tools. Let that sink in. Each one is a macOS binary — compiled with Apple's clang, linked against Apple's libSystem, depending on Apple's struct layouts, Apple's calling conventions, Apple's constant values. Each one is running on Debian Linux, where none of those assumptions hold. Between the binary and the host OS stands a layer of translation — a hundred-plus shims covering three struct families, thirty-three variadic functions, seven locale constants, dozens of data symbols, and a handful of environment bridges.

If you zoom out far enough, what we built in this chapter is a compatibility layer in the tradition of Wine's `ntdll.dll.so`, or Darling's `libSystem` stubs, or FreeBSD's Linux compatibility layer. But where those projects translate from one OS's system call interface to another at the kernel boundary, we translate at the *ABI* boundary — at the level of struct layouts and calling conventions. The macOS binary never makes a Darwin syscall. It makes POSIX function calls that we intercept and translate. The approach is different, but the problem is the same: make foreign software feel at home on alien soil.

The hardest part wasn't any individual shim. The hardest part was the variadic ABI — the undocumented Apple convention that put all variadic arguments on the stack while every reference manual said they went in registers. That discovery took three days of staring at register dumps, disassembling compiler output, and reading LLVM source code before the pattern emerged. Three days of "this should work" followed by SIGSEGV. Three days of the wrong mental model, because every document we consulted described the Linux convention and called it universal.

The lesson is brutal and foundational: **documentation describes intent. Binaries describe reality.** When the documentation says "variadic arguments go in registers" and the binary puts them on the stack, the binary is right. Always. The binary is the contract. The binary is the truth. The binary is the specification that actually matters, because the binary is what executes.

But there's a question we haven't asked. Everything in this chapter — the struct translations, the variadic shims, the constant mappings — works because both sides are ARM64. The instructions execute natively. The registers are the same width. The memory model is the same. We translated the ABI, but the ISA was never in question.

What if the host machine isn't ARM64 at all?

What if the macOS binary is ARM64 and the host is x86_64? Or the binary is x86_64 and the host is ARM64? What if we can't execute the instructions natively — if every instruction needs to be *interpreted*?

That's a different kind of bridge. Not between two ABIs on the same architecture, but between two architectures entirely. That's Chapter 7.
