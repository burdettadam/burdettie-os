# Chapter 8: The System

## Scene 8.1: The libSystem Shim Layer

Threading, exceptions, and signals. Three missing paradigms. Chapter 7 ended with 27 passing ObjC tests and a clear admission: the interpreter models a single thread of execution, knows nothing about stack unwinding, and can't distinguish a guest SIGSEGV from a host one. Every one of those problems lives in the same place — the boundary between the macOS binary and the operating system it thinks it's running on.

That boundary has a name. On macOS, every userspace binary links exactly one mandatory library: `libSystem.B.dylib`. Not Foundation. Not CoreFoundation. Not the ObjC runtime. Those are optional — you can build a C program that links none of them. But every Mach-O executable, no matter how minimal, depends on libSystem. It is the root. It's where `malloc` lives. Where `pthread_create` lives. Where `mach_task_self` lives. Where `dyld_stub_binder` lives. Where `exit` lives. It is not "just libc." It is the *entire platform substrate* — the interface between user code and the Darwin kernel. Every system call, every threading primitive, every memory allocator, every signal handler, every file descriptor operation, every lock, every IPC mechanism, every sandbox query, every power management hint routes through libSystem.

And we have to fake the whole thing.

On macOS, when `dyld` loads a binary, it maps libSystem into the process and resolves several hundred symbols — function pointers and data addresses — that the binary's `__got` and `__la_symbol_ptr` sections reference. The binary never calls syscalls directly. It calls through libSystem wrappers: `open()` calls `__open`, which calls `syscall(SYS_open, ...)`, which traps to the Mach/BSD kernel. On Linux, we don't have the Darwin kernel. We don't have libSystem. We have our own Rust loader, our own shim table, and `libc.so` on the host. The shim layer is the translator between what the macOS binary expects and what the Linux host provides.

The crate that implements this lives at `crates/libsystem-shims/`. Count the source files:

```
libsystem-shims/src/
├── lib.rs                          # Top-level: install_default_shims()
├── table.rs                        # ShimTable: name → address registry
├── darwin_compat.rs                # errno, stdio, locale, ctype shims
├── dispatch.rs                     # GCD: dispatch_queue, dispatch_once
├── lifecycle.rs                    # atexit, exit, abort, _NSGetProgname
├── mach.rs                         # mach_task_self, mach_host_self
├── process.rs                      # getpid, getenv, isatty, getcwd
├── pthread/                        # Thread lifecycle, mutexes, TLS
│   ├── mod.rs
│   ├── shims.rs
│   └── tests.rs
├── ptrauth.rs                      # PAC sign/strip stubs
├── random.rs                       # arc4random, SecRandomCopyBytes
├── socket.rs                       # Socket option translation
├── tls.rs                          # Thread-local storage keys
├── cross_platform/                 # Struct layout translation
│   ├── dirent.rs                   # macOS → Linux dirent
│   ├── icu.rs                      # ICU Unicode shims
│   ├── stat.rs                     # macOS → Linux struct stat
│   ├── posix_extras.rs             # confstr, sysconf translations
│   └── variadic_shims.rs           # Apple → Linux variadic ABI
└── darwin_stubs/                   # No-op stubs for macOS-only APIs
    ├── xpc.rs, xpc_activity.rs     # XPC framework
    ├── security.rs                 # Security.framework
    ├── sandbox.rs                  # App Sandbox
    ├── iokit.rs                    # IOKit
    ├── dispatch_concurrency.rs     # GCD advanced
    ├── core_text.rs                # CoreText
    ├── swift_type_metadata.rs      # Swift runtime
    ├── rosetta.rs                  # Rosetta hooks
    ├── cfnetwork.rs                # CFNetwork
    └── ... (37 stub modules)
```

Fifty-seven `.rs` files. Three architectural strategies, stated plainly in the crate documentation:

1. **Implement** — pure Rust implementation: `strlen`, `getprogname`, `arc4random`
2. **Delegate** — forward to host via `libc`: `malloc`, `pthread_create`, `stat`
3. **Deny** — abort or return error: `fork`, `exec`, `posix_spawn`

The delegation strategy is the interesting one. "Forward to host via `libc`" sounds simple until you remember Chapter 6's lesson: the ABI is different. A macOS binary calling `stat()` expects a 144-byte buffer in Apple's layout. Linux's `stat()` fills a 128-byte buffer in a different layout. The delegate shim can't just forward the call — it must translate the struct:

```rust
pub struct DarwinCompatShims;

impl DarwinCompatShims {
    pub fn install(table: &mut ShimTable) {
        #[cfg(not(target_os = "macos"))]
        {
            register_shims!(table,
                ("___error", shim_error as *const () as u64, "__error → errno location"),
            );
            register_shims!(table,
                ("___maskrune", shim_maskrune as *const () as u64, "__maskrune compat"),
                ("___tolower", shim_tolower as *const () as u64, "__tolower compat"),
            );
            register_shims!(table,
                ("___stderrp", stdio_ptr_addr(StdioStream::Stderr) as u64, "__stderrp data"),
                ("___stdoutp", stdio_ptr_addr(StdioStream::Stdout) as u64, "__stdoutp data"),
            );
        }
    }
}
```

The `#[cfg(not(target_os = "macos"))]` guard is the key. On macOS, these symbols exist natively — there's nothing to shim. The binary calls `___error` and gets the real `__error` from libSystem. On Linux, `___error` doesn't exist. The errno location is accessed via `__errno_location()`, not `__error()`. The shim bridges the gap: when the macOS binary calls `___error`, our shim calls Linux's `__errno_location()` and returns the result. Same semantics. Different symbol name. Different calling convention for the location. One thin wrapper makes it invisible.

The `___stderrp` case is even subtler. On macOS, `stderr` is accessed through a *data symbol* — a global pointer at a fixed address. The binary doesn't call a function; it loads 8 bytes from the address of `___stderrp` and gets a `FILE*`. On Linux, `stderr` is a macro that expands to something platform-specific. Our shim allocates a static pointer-to-pointer, fills it with the address of the host's `stderr`, and registers that static's address as the data symbol. The binary loads 8 bytes, gets a valid `FILE*`, and never knows the difference.

The variadic shims are the most complex translation layer. On ARM64, Apple and Linux use different calling conventions for variadic arguments. Apple places all variadic arguments on the stack. Linux AAPCS64 places the first several in registers, then overflows to the stack. When a macOS binary calls `printf(fmt, arg1, arg2)`, it places `arg1` and `arg2` on the stack. Linux's `printf` implementation looks for them in registers `x1` and `x2`. It finds whatever garbage was there before. The binary's output is wrong, or it crashes.

The fix is a set of naked assembly shims that capture the original stack pointer, construct a Linux `va_list` struct that tells `va_arg` to skip the registers and read everything from the stack, then tail-call the `v`-variant (`vprintf`, `vfprintf`, `vsnprintf`). Thirty-two bytes of hand-written struct layout, matching Linux's `va_list` format exactly:

| Offset | Field | Size | Value |
|:---|:---|:---:|:---|
| 0 | `__stack` | 8 | original SP (where Apple placed args) |
| 8 | `__gr_top` | 8 | 0 (unused) |
| 16 | `__vr_top` | 8 | 0 (unused) |
| 24 | `__gr_offs` | 4 | 0 (no GP regs left — read stack) |
| 28 | `__vr_offs` | 4 | 0 (no FP regs left — read stack) |

Twenty-five variadic functions shimmed this way: `printf`, `fprintf`, `sprintf`, `snprintf`, `dprintf`, `asprintf`, `sscanf`, `fscanf`, `scanf`, `err`, `errx`, `errc`, `warn`, `warnx`, `syslog`, and all their `v`-variant counterparts.

Then there are the stubs. The `darwin_stubs/` directory contains 37 modules providing no-op or safe-default implementations of APIs that have no Linux equivalent: XPC (interprocess communication), Security.framework (keychain, trust evaluation), App Sandbox (`sandbox_check`, `sandbox_extension_consume`), IOKit (device matching, power management), CoreText (font enumeration), Launch Services (URL opening, app registration), Disk Arbitration, System Configuration (network reachability), Spotlight (metadata queries), ASL (Apple System Logger), Mobile Gestalt (hardware queries), Rosetta (translation hooks), CommonCrypto, and the Swift runtime's reflection and type metadata APIs.

Each stub follows the deny-gracefully pattern: return a null pointer, return zero, return an error code that the caller can handle without crashing. An XPC call returns `NULL` — the binary checks for failure and moves on. A sandbox check returns 0 — the binary thinks the operation is allowed. A CoreText font enumeration returns an empty array. The binary doesn't get the font catalog. It doesn't crash.

Roughly 92% of the libSystem symbols that our 39 corpus binaries reference are covered by these three strategies. The remaining 8% are hard edge cases: Mach IPC (`mach_msg` with complex descriptors), kernel-level task introspection (`task_info`, `thread_get_state`), and a handful of obscure POSIX extensions (`posix_spawn_file_actions_addinherit_np`, `_NSGetExecutablePath` with edge-case buffer sizes). These are the long tail — individually rare, collectively a constant source of new crash reports.

The lowest layer of the system is the hardest to see and the most important to get right. When `stat` returns the wrong struct layout, the binary doesn't get a helpful error message. It gets a garbage file size and a crash three function calls later. When `printf` reads variadic arguments from the wrong location, the output is corrupted in ways that look like a completely unrelated bug. When errno is at the wrong address, every error-checking path in the binary sees stale or garbage error codes. The platform substrate is opaque. Its failures are invisible until they cascade.

That's what makes libSystem the root. Not because it's architecturally interesting — it's the opposite of interesting, it's plumbing. But plumbing that leaks invisibly is the worst kind of bug to find. Getting the shim layer right means getting 57 files of careful ABI translation right, one struct field offset at a time, one #[cfg] guard at a time, one assembly thunk at a time. And if we get it right, the layers above — Foundation, ObjC, the interpreter — never have to think about it.

---

## Scene 8.2: Thread-Local Variables

Every C compiler understands `__thread`. Every C++ compiler understands `thread_local`. The concept is simple: a variable that has a distinct copy for each thread. Thread A writes 42 to its copy. Thread B reads its own copy and gets whatever it last wrote — not 42. No locks required. No atomics. Each thread has its own private instance of the variable, allocated somewhere in thread-private storage, accessed transparently through what looks like an ordinary pointer dereference.

Except it isn't an ordinary pointer dereference. It can't be. A global variable has one address for the entire process. A thread-local variable has a *different* address in every thread. The address can't be known at compile time — it depends on which thread is running. It can't even be known at link time — threads are created at runtime. Someone has to compute the address on the fly, every time the variable is accessed.

On ELF platforms (Linux, FreeBSD, etc.), the answer is `__tls_get_addr`. The compiler emits a call to this function, passing a module ID and an offset, and the function returns the address of the TLS variable for the current thread. The linker generates a GOT entry. The runtime resolves it. The TLS access model is documented in the ELF TLS specification, which is its own rabbit hole of initial-exec, local-exec, general-dynamic, and local-dynamic models.

Mach-O doesn't use any of that. It has its own mechanism, wholly incompatible with ELF's.

In a Mach-O binary, thread-local variables appear in a special section: `__DATA,__thread_vars`. Each variable gets a *TLV descriptor* — a 3-pointer struct, 24 bytes on ARM64:

```rust
#[repr(C)]
struct TlvDescriptor {
    thunk: unsafe extern "C" fn(*mut TlvDescriptor) -> *mut u8,
    key: usize,
    offset: usize,
}
```

Three fields. The `thunk` is a function pointer — the address of a function that, when called with this descriptor as its argument, returns a pointer to the current thread's copy of the variable. The `key` identifies which image (loaded Mach-O binary) this variable belongs to. The `offset` is the byte offset within that image's TLV block where this particular variable lives.

When the compiler encounters a thread-local access — `int x = tls_var;` — it doesn't generate a simple load. It generates a call through the descriptor:

```asm
adrp  x8, __thread_vars@PAGE
add   x8, x8, __thread_vars@PAGEOFF  ; x8 → TlvDescriptor for tls_var
ldr   x9, [x8]                        ; x9 → thunk function pointer
blr   x9                              ; call thunk(x8), returns &tls_var
ldr   w0, [x0]                        ; load the actual value
```

Four instructions. Load the descriptor address. Load the thunk pointer from the descriptor's first 8 bytes. Call the thunk, passing the descriptor itself. The thunk returns a pointer. Load from that pointer. The compiler trusts the thunk to do the right thing — to figure out which thread is executing, find that thread's TLV block, look up the offset, and return the address.

On macOS, `dyld` provides the thunk. It's called `_tlv_bootstrap` — an internal function that `dyld` sets as the initial thunk for every TLV descriptor. On first call, `_tlv_bootstrap` replaces itself with a faster `_tlv_get_addr` function and allocates the per-thread TLV block. On subsequent calls, `_tlv_get_addr` is a fast lookup: pthread-specific key → block pointer → add offset → done.

We don't have `dyld`. We don't have `_tlv_bootstrap`. And after fixups in the loader, the thunk field in each TLV descriptor has been rebased to point at... whatever address `_tlv_bootstrap` would have had in the real dyld. On Linux, that address is garbage. If the binary accesses a thread-local variable, it calls through the thunk, jumps to garbage, and crashes.

The fix is module `crates/macho-exec/src/tlv.rs`. After fixups are applied but before `protect()` makes `__DATA` read-only, we walk every TLV descriptor in the mapped image and patch the thunk to point at our own implementation:

```rust
unsafe extern "C" fn tlv_get_addr(desc: *mut TlvDescriptor) -> *mut u8 {
    let d = unsafe { &*desc };
    let key_val = d.key;
    let offset = d.offset;
    // Look up per-image info, allocate per-thread block if needed,
    // return pointer to this thread's copy at the requested offset.
}
```

The full implementation uses `pthread_key_create` for per-thread storage. Each loaded image that contains TLV sections gets a unique key. The first time a thread accesses a TLV from that image, `tlv_get_addr` allocates a block large enough to hold all of the image's thread-local variables, copies in the initial values from `__thread_data` (or zeros the block if all TLVs are BSS), and stores the block pointer via `pthread_setspecific`. On subsequent accesses from the same thread, the block is already there — `pthread_getspecific` returns it, we add the offset, done.

The per-image TLV metadata lives in a global registry:

| Field | Purpose |
|:---|:---|
| `pthread_key` | Per-thread storage key for this image's TLV block |
| `total_size` | `__thread_data` + `__thread_bss` sizes |
| `template` | Initial values from `__thread_data` (copied per thread) |

When a thread exits, the destructor registered with `pthread_key_create` frees the block. No leaks. When the process exits, the global registry is dropped. Clean.

The patching logic walks the `__thread_vars` section in mapped memory:

```rust
for i in 0..desc_count {
    let desc_offset = section_offset_in_seg + i * desc_size;
    let desc_ptr = unsafe { seg.base.add(desc_offset) as *mut TlvDescriptor };
    unsafe {
        (*desc_ptr).thunk = tlv_get_addr;
        (*desc_ptr).key = key_val;
        // offset stays as-is (byte offset within the TLV block)
    }
}
```

Each descriptor gets our thunk and a key that maps back to the image's metadata. The offset field is left untouched — it was set by the linker and correctly encodes the variable's position within the TLV block. Only the thunk and key need patching.

There's a subtle ordering constraint. TLV patching must happen after fixups (because fixups rebase the descriptor addresses to their mapped locations) but before `protect()` (because `protect()` makes `__DATA` segments read-only). The typestate pipeline (Scene 8.5) ensures this ordering at compile time — you literally cannot call `protect()` before `apply_fixups()`, and the TLV patching runs inside `apply_fixups()`:

```rust
// Patch TLV thunks before protect() makes __DATA read-only
for loaded in registry.images() {
    macho_exec::patch_tlv_thunks(&loaded.image, &loaded.parsed, &loaded.data);
}
```

One line in the pipeline, but it represents an entire mechanism: finding sections by type, computing block sizes, allocating pthread keys, registering metadata, and patching function pointers in mapped memory. All invisible to the binary. The compiled code still does `ldr x9, [x8]; blr x9` — it still calls through the descriptor — but now the thunk points to our function instead of dyld's. Same ABI. Same calling convention. Same three-pointer descriptor layout. Different implementation behind the curtain.

This is the pattern that repeats through the entire system layer: invisible infrastructure. Thread-local variables work perfectly on macOS because `dyld` handles everything silently, before `main()` runs, before any constructor executes, before the binary even knows it has TLVs. You never think about them. You just write `__thread int x;` and it works. Implementing them yourself forces you to see every piece of the mechanism: the section types, the descriptor layout, the thunk calling convention, the per-thread allocation, the template copying, the cleanup on thread exit. Thread-locals work until you implement them yourself. Then you realize they're an entire subsystem hiding in plain sight.

---

## Scene 8.3: Exception Handling

A C++ `throw` looks simple. A source-level keyword. The compiler inserts a call to `__cxa_throw`. The runtime unwinds the stack. A `catch` block somewhere up the call chain receives the exception object. Between the throw and the catch, destructors run. Stack frames are popped. Registers are restored. Control transfers cleanly across function boundaries, past arbitrary call depths, landing precisely in the right handler.

The mechanism that makes this work is astonishingly complex. It involves three separate binaries — the compiler, the linker, and the runtime — all agreeing on a data format that none of them fully owns.

It starts with the compiler. When `clang` compiles a function that might throw or catch exceptions, it generates metadata describing the function's frame layout: which registers are saved where, how large the frame is, and where to find the exception handling personality function. This metadata goes into the `__unwind_info` section (compact form) and the `__eh_frame` section (DWARF form). Both encode the same conceptual information — how to restore the caller's state from any instruction within the function — but in different formats optimized for different use cases.

Compact unwind info is the fast path. Each entry is a single 32-bit integer:

| Bits | Field | Meaning |
|:---:|:---|:---|
| [31:28] | Encoding type | Frame-based, frameless, dwarf-reference |
| [27:24] | Common encoding index | Which pre-built encoding to select |
| [23:0] | Encoding payload | Register saves, frame offset, etc. |

A frame-based ARM64 function — the common case, where `x29` (FP) and `x30` (LR) are saved at the start — gets a compact encoding that says: "This function uses x29 as frame pointer. To unwind, restore x29 and x30 from [sp], and set PC to x30." That's one 32-bit entry per function. Compare this to DWARF's `__eh_frame`, which encodes the same information as a variable-length bytecode program that describes how to restore each register at every instruction within the function. DWARF is more expressive but vastly larger. Compact unwind covers 95% of functions in a fraction of the space.

The `__unwind_info` section is organized as a two-level page table. The first-level index maps PC ranges to second-level pages. Each second-level page contains either regular entries (one per function, sorted by function start address) or compressed entries (delta-encoded for space efficiency). To unwind from a PC, the runtime binary-searches the first level, then searches within the second-level page, and extracts the compact encoding.

But compact unwind only tells you how to restore registers and pop the frame. It doesn't tell you about exception handlers — `catch` blocks, `finally` blocks, cleanup actions. That information lives in the LSDA: the Language Specific Data Area. Each function that participates in exception handling has an LSDA, pointed to by its compact unwind entry. The LSDA contains a call-site table that maps PC ranges to landing pad addresses and action records. "If an exception occurs while executing instructions between PC=0x100 and PC=0x120, jump to the landing pad at PC=0x180. The action record says: catch `std::exception` and resume."

The personality function ties it all together. For C++, it's `__gxx_personality_v0`. For Objective-C, it's `__objc_personality_v0`. When the unwinder pops a frame, it calls that frame's personality function with the exception object and the LSDA data. The personality function reads the call-site table, checks whether the current PC falls within a range that has a handler for this exception type, and tells the unwinder what to do: continue unwinding, transfer control to a landing pad, or invoke a cleanup action.

Consider what happens when ObjC code uses `@try`/`@catch`:

```objc
@try {
    [obj riskyMethod];
} @catch (NSException *e) {
    NSLog(@"Caught: %@", e);
}
```

The compiler transforms this into calls to `objc_begin_catch` and `objc_end_catch`, with the catch block installed as a landing pad. The personality function `__objc_personality_v0` understands ObjC exception types. When `[obj riskyMethod]` throws, the runtime unwinds the stack using the compact unwind info, calls the personality function at each frame, and the personality function says "yes, there's a catch block at this landing pad for `NSException`." Control transfers to the landing pad. The exception is caught.

Now consider what our loader needs to do with all of this.

On macOS, the system unwinder (`libunwind`) reads `__unwind_info` and `__eh_frame` from the binary's mapped segments. It works because `dyld` registered those segments with the unwinder during load. The compact unwind entries contain function addresses that are valid in the process's address space. The personality function pointers have been resolved. The LSDA pointers are correct.

In our loader, the binary's segments are mapped at different addresses than they would be on macOS. The fixup phase rebases the internal pointers, but the compact unwind entries encode function offsets, not pointers — they're relative to the image's load address. Our `mprotect` calls make `__TEXT` executable. The system unwinder on Linux (`libgcc_s` or `libunwind`) could theoretically walk these sections — if it knew they existed.

The critical insight is that for our use case — running macOS command-line tools that primarily use C and ObjC, not heavily exception-dependent C++ — we don't need a full unwinder. We need a *minimal* exception infrastructure that prevents crashes when exceptions are thrown in common scenarios: `@try`/`@catch` around file operations, `NSException` handling in Foundation internals, and the rare `std::exception` from C++ code.

Our approach has three parts. First, we register the mapped image's unwind sections with the host's unwinder via `__register_frame`. This is a GCC extension that both `libgcc_s` and LLVM's `libunwind` support on Linux. It takes a pointer to `__eh_frame` data and adds it to the unwinder's search list. When an exception occurs, the host unwinder can now find our binary's frame descriptions.

Second, we provide personality function shims. The binary references `___gxx_personality_v0` and `___objc_personality_v0` as imported symbols. Our resolver maps these to the host's personality functions — on Linux, `__gxx_personality_v0` lives in `libstdc++` or `libc++abi`, and we resolve it via `dlsym`. For `__objc_personality_v0`, we provide our own implementation that understands ObjC exception semantics and delegates to the underlying C++ personality function for the actual DWARF interpretation.

Third, we keep `setjmp`/`longjmp` working. Many ObjC exception patterns — especially in Apple's Foundation internals — use `setjmp`/`longjmp` as a fast path instead of true C++ exceptions. Our shim table registers `_setjmp`, `_longjmp`, and `__sigsetjmp` to the host's implementations. As long as the jump buffer is in host-accessible memory (which it is, since we map guest `__DATA` with `mmap`), the host's `longjmp` can restore the saved registers and transfer control back to the `setjmp` site.

The result is not a complete exception handling system. We don't support C++ exception chaining, nested `try`/`catch` blocks more than two levels deep, or exceptions thrown from one loaded image that need to be caught in a different loaded image. What we support is the common case: Foundation code that uses `@try`/`@catch` for error recovery, C library code that uses `setjmp`/`longjmp` for signal handling, and C++ code that throws in a single function scope after our runtime detects a recoverable error.

Exception handling bridges three worlds: compiler metadata (`__unwind_info`, `__eh_frame`), linker output (LSDA pointers, personality function references), and runtime infrastructure (the unwinder, personality functions, catch/cleanup dispatch). No single component owns the whole mechanism. The compiler generates data it never executes. The linker lays out sections it doesn't understand. The runtime reads formats it didn't create. And all three must agree precisely — one wrong offset, one misaligned LSDA, one personality function that misinterprets the action table, and the process crashes *during* the exception handling that was supposed to prevent crashes.

The elegance is in the contract, not the implementation. Compact unwind info is a beautiful space-time tradeoff: 4 bytes per function versus dozens for DWARF. Personality functions are a beautiful abstraction boundary: the unwinder doesn't know ObjC from C++ from Rust — it just calls the personality function and follows orders. And the two-level page table layout of `__unwind_info` is a genuinely clever piece of data structure design, optimized for the overwhelmingly common case (binary search finds the right function in two or three cache-line reads) while handling the uncommon case (DWARF fallback for functions the compact format can't encode).

This is infrastructure that works so well you forget it exists. Every ObjC `@throw` in every iOS and macOS app hits these code paths. Every C++ `throw` in every system library hits these code paths. Billions of exceptions per day, globally, unwinding through compact unwind entries and personality functions. And it's a system that Appendix B unpacks in full detail — the binary format secrets that make it all possible.

---

## Scene 8.4: Signal Handling

Signal 11. SIGSEGV. The segmentation violation. We've met it before. Chapter 1 opened with it — the very first time we tried to run a Mach-O binary on Linux, the process died with SIGSEGV. Chapter 6 opened with it — the bridge to Linux crashing because struct layouts were wrong. Every chapter has had at least one. It's the leitmotif of this book, the crash that means "you got something wrong at the lowest level."

Now it's time to handle it instead of dying to it.

The problem is deceptively simple to state. When the loader runs a macOS binary, that binary's code executes in the same process as the loader itself. There is no VM boundary, no hypervisor, no separate address space. Guest code and host code share one process, one set of signal handlers, one stack. If the guest binary dereferences a null pointer, the kernel delivers SIGSEGV to the process. If the loader itself has a bug, the kernel delivers the same signal. The signal handler receives a `siginfo_t` with a fault address and a context structure containing the register state at the moment of the fault. But whose registers? Whose fault?

If the fault address is in a known guest memory region — a mapped `__DATA` segment, a mapped `__TEXT` segment, a guest stack allocation — the crash is the guest's problem. A null pointer in guest code. A bad offset in a guest array. An unresolved symbol that resolved to zero. These are bugs in the guest binary, or gaps in our shim layer, or missing Foundation methods. The right response is: log the fault address, dump the guest register state, identify the crashing instruction, and exit cleanly with a diagnostic.

If the fault address is in host memory — the loader's heap, a Rust stack frame, a shim function's local variables — the crash is our problem. A bug in the loader. A shim that returned a bad pointer. A memory corruption from a botched fixup. The right response is: crash normally, produce a core dump, let the developer debug it.

Distinguishing the two requires `sigaction()` with `SA_SIGINFO`:

```c
struct sigaction sa = {
    .sa_sigaction = signal_handler,
    .sa_flags = SA_SIGINFO | SA_ONSTACK,
};
sigaction(SIGSEGV, &sa, NULL);
sigaction(SIGBUS, &sa, NULL);
```

The `SA_SIGINFO` flag tells the kernel to pass three arguments to the handler: the signal number, a `siginfo_t` with the fault address (`si_addr`), and a `ucontext_t` with the full register state. The `SA_ONSTACK` flag means the signal handler runs on an alternate stack — critical, because if the guest stack is corrupted, we don't want the signal handler running on it.

Inside the handler, the decision logic is:

```
let fault_addr = siginfo.si_addr as u64;
if is_guest_region(fault_addr) || is_guest_pc(context.pc) {
    // Guest fault → log diagnostics and exit gracefully
    log_guest_crash(signal, siginfo, context);
    exit(128 + signal);
} else {
    // Host fault → restore default handler and re-raise
    signal(sig, SIG_DFL);
    raise(sig);
}
```

The `is_guest_region` check walks the image registry: for each mapped image, is the fault address between `vmaddr` and `vmaddr + vmsize`? If yes, it's a guest address. The `is_guest_pc` check is similar: is the program counter (from `ucontext_t.uc_mcontext.pc`) within a guest `__TEXT` segment?

The diagnostic output when a guest crash is caught:

```
[CRASH] SIGSEGV at guest PC 0x0000004000825f48
  Fault address: 0x0000000000000000 (NULL dereference)
  Image: /opt/xcode-tools/system/stat
  Section: __TEXT,__text + 0x1f48
  x0  = 0x0000004000b31a40  x1  = 0x0000000000000000
  x29 = 0x0000fffff7ffab70  x30 = 0x0000004000825f44
  sp  = 0x0000fffff7ffab60
```

That's the crash from Chapter 1 — but now it's caught, diagnosed, and reported instead of killing the process with an opaque `Segmentation fault (core dumped)` message. The fault address is 0x0 — a null dereference. The PC is in the guest binary's text section, offset 0x1f48. The register dump shows exactly what state the guest was in when it crashed. With this information, you can trace back: what instruction is at that offset? What symbol reference resolved to null? Which shim is missing?

A crash is just a signal you didn't handle.

That sentence captures the philosophical shift of this chapter. In Chapter 1, SIGSEGV was an enemy — a wall that stopped progress, an opaque failure with no explanation. In Chapter 6, SIGSEGV was a clue — each crash pointed to a missing struct translation or a broken ABI assumption. Now, in Chapter 8, SIGSEGV is a *tool*. We install a handler. We intercept the signal. We classify the fault. We extract diagnostics. We report the problem in human-readable terms. The signal hasn't changed — it's the same signal 11, the same kernel mechanism, the same hardware fault. What changed is our relationship to it. We went from victims of crashes to handlers of crashes to diagnosticians of crashes.

There's a deeper point about signals and the loader model. Because we run guest code in the same process as host code — no VM, no separate address space — we inherit both the power and the danger of shared-process execution. The power: guest code can call host functions directly via function pointers, zero overhead, no context switch. The danger: guest bugs corrupt host state. A wild pointer from guest code can overwrite the shim table. A stack overflow in guest code can blow past the guard page and into loader heap. Signals are our one defense mechanism — the kernel's notification that something went wrong at the hardware level.

On macOS, the real `dyld` and the binaries it loads share a process too. A crashing app crashes the whole process — there's no isolation. But `dyld` doesn't need to *diagnose* crashes, it just needs to not cause them. Our position is different. We're a compatibility layer. Crashes are *expected* — every missing shim, every unimplemented Foundation method, every incorrect ABI translation will eventually surface as a crash. The signal handler is how we turn those crashes from opaque failures into actionable diagnostics.

SIGBUS deserves a mention. It's delivered for misaligned memory accesses on architectures that don't support unaligned loads (some ARM configurations), for accessing memory-mapped files that have been truncated, and for certain types of invalid physical-memory access. The handler treats it identically to SIGSEGV: classify the fault address, diagnose if guest, re-raise if host. In practice, SIGBUS is rare for our use case — ARM64 supports unaligned access for most instructions — but it appears when a mapped image's backing file is modified while the loader is running. Defense in depth.

The signal handler is the last piece of the safety net. Below it: hardware faults. Above it: the entire loader stack — shims, Foundation, ObjC runtime, the interpreter. The signal handler sits at the boundary between "the operating system detected a problem" and "we can explain what went wrong." Every crash in chapters 1 through 7 that we diagnosed by reading register dumps and computing offsets? The signal handler automates that diagnostic process. It's the crash-as-teacher made mechanical.

---

## Scene 8.5: The Typestate Loader

Here is the complete Mach-O loader pipeline in seven lines of Rust:

```rust
let result = Loader::parse(path, config).unwrap()
    .check_security().unwrap()
    .map_images().unwrap()
    .apply_fixups().unwrap()
    .register_objc().unwrap()
    .protect().unwrap()
    .initialize().unwrap()
    .run().unwrap();
```

Parse. Check. Map. Fix. Register. Protect. Initialize. Run. Eight method calls, chained fluently, each one consuming the previous state and returning a new one. It reads like a recipe. It executes like a state machine. And the Rust type system guarantees — at compile time, not at runtime — that these calls happen in this order and no other.

This is the typestate pattern. The `Loader` struct is generic over a phase parameter:

```rust
pub struct Parsed;
pub struct Checked;
pub struct Mapped;
pub struct FixedUp;
pub struct Registered;
pub struct Protected;
pub struct Initialized;

pub struct Loader<S: sealed::Phase> {
    inner: LoaderInner,
    _phase: PhantomData<S>,
}
```

Seven empty structs. One generic `Loader`. The `PhantomData<S>` field carries the phase type without using any runtime memory — it's a zero-size marker that exists only for the type checker. `Loader<Parsed>` and `Loader<Mapped>` are different types. You can't pass one where the other is expected. You can't assign one to a variable typed as the other. They are as distinct as `String` and `Vec<u8>`.

The transition methods are where the magic happens. `check_security()` is defined only on `Loader<Parsed>`:

```rust
impl Loader<Parsed> {
    pub fn check_security(self) -> Result<Loader<Checked>, RuntimeError> {
        // ... evaluate trust policy, produce SecurityReport ...
        Ok(Loader {
            inner: self.inner,
            _phase: PhantomData,
        })
    }
}
```

It takes `self` by value — consuming the `Loader<Parsed>`. After this call, the parsed loader no longer exists. You can't call `check_security()` twice. You can't use the `Loader<Parsed>` for anything else. It's gone. The method returns a `Loader<Checked>` — a new type, with `map_images()` available but `check_security()` no longer callable.

Similarly, `map_images()` is defined only on `Loader<Checked>`. `apply_fixups()` only on `Loader<Mapped>`. `register_objc()` only on `Loader<FixedUp>`. Each phase transition is a one-way door: you pass through it, the previous type is consumed, and the next type offers exactly the operations that are valid in the new state.

Try to skip a step:

```rust
let loader = Loader::parse(path, config)?;
loader.map_images(); // ERROR: no method named `map_images` on `Loader<Parsed>`
```

The compiler catches it. `Loader<Parsed>` doesn't have `map_images()`. You have to call `check_security()` first to get a `Loader<Checked>`, which does have `map_images()`. The error message tells you exactly what's wrong: wrong phase, wrong type, wrong order.

Try to call a method that belongs to a later phase:

```rust
let loader = Loader::parse(path, config)?
    .check_security()?
    .map_images()?;
loader.protect(); // ERROR: no method named `protect` on `Loader<Mapped>`
```

You can't protect before fixing up. The type system says so. Not a runtime assertion, not a debug-mode check, not a comment in the code saying "don't call this before fixups." A compile-time error. The type system is the documentation. The type system is the enforcement. The type system is the architecture.

The `sealed::Phase` trait prevents external code from defining new phases:

```rust
mod sealed {
    pub trait Phase {}
}
```

The trait is in a private module. Only `Loader`'s own code can implement it. You can't add a `Confused` phase from outside the crate. You can't impl `Phase` for `String`. The type parameter is locked down to exactly seven known types.

Why does this matter for a loader? Consider what each phase requires:

| Phase | Precondition | What becomes available |
|:---|:---|:---|
| Parsed | Binary read and parsed | Segments, commands, sections in memory |
| Checked | Security policy evaluated | Green/yellow/red trust report |
| Mapped | Images mmap'd into address space | Valid mapped memory regions |
| FixedUp | Rebase + bind fixups applied | Pointers are correct, TLVs patched |
| Registered | ObjC classes/categories merged | Runtime method dispatch works |
| Protected | Segment permissions set | `__TEXT` is RX, `__DATA_CONST` is R |
| Initialized | +load and constructors run | Static state initialized |

Every phase depends on the previous one. You can't apply fixups to unmapped memory. You can't register ObjC classes before fixups make their pointers valid. You can't protect segments after initialization, because initializers might write to `__DATA`. You can't run constructors before ObjC registration, because constructors might call `objc_msgSend`.

Traditional loaders enforce this ordering with assertions or careful documentation. "Don't call `fixup()` before `mmap()`." "Make sure to register ObjC before calling `protect()`." Assertions catch violations at runtime — in debug builds. Documentation catches them when someone reads it — if they read it. Both fail silently in production.

The typestate pattern catches violations at compile time. Before the code runs. Before the tests run. Before the binary ships. The compiler sees the types, checks the method constraints, and refuses to compile code that violates the ordering invariant. Zero runtime cost — `PhantomData` is zero-size, the phase parameter is erased by monomorphization, the generated code is identical to a non-generic version with no phase tracking at all.

This is the thesis in action. The entire book is about building a Mach-O loader. The typestate pipeline is about building it *correctly*. Not "correct enough for the tests we've written." Not "correct unless someone changes the call order." Correct as proven by the type system — the same type system that checks every other invariant in every other Rust program. The loader's architecture isn't enforced by convention. It's enforced by the compiler.

The design reflects a deeper philosophy about systems programming in Rust. C and C++ give you the power to express almost anything, and the responsibility to get it right. Rust gives you the same power, but lets you encode your architectural constraints in the type system so the compiler shares the responsibility. The typestate pattern isn't unique to loaders — it appears in state machine protocols, connection lifecycle management, build pipelines, and anywhere that "do step A before step B" matters. But in a loader, where the consequences of wrong ordering are memory corruption, security bypasses, or silent data corruption, the stakes justify the pattern.

Seven phases. Seven empty structs. One `PhantomData` field. Zero runtime overhead. Complete ordering guarantee. That's what Rust's type system buys you: compile-time proof that the loader does what the architecture requires.

---

## Scene 8.6: Performance

The system layer works. The shim table resolves symbols. TLV descriptors point to our thunk. Exceptions can unwind through registered frames. Signals are caught and diagnosed. The typestate pipeline ensures correct ordering. Every piece is in place. Let's measure it.

First run of the performance benchmark — 39 corpus tool binaries, each loaded and executed 5 times:

```
Average loader total: 300 µs
```

Three hundred microseconds. For a single invocation of a command-line tool. That's fast in absolute terms — 0.3 milliseconds, invisible to the human eye. But consider the use case: a build system that invokes `xcrun`, `clang`, `ld`, `lipo`, `codesign`, and `otool` hundreds of times per project build. Three hundred microseconds per invocation, multiplied by 500 tool invocations, is 150 milliseconds of pure loader overhead. Not execution time — that's the work the tool actually does. Loader overhead is the tax we charge for the privilege of running the binary at all. A 150ms tax, repeated every build, is unacceptable.

Where does the time go? We instrument every phase of the pipeline with `Instant::now()` and compute percentages:

| Phase | Time (µs) | % of Total |
|:---|---:|---:|
| parse | 12 | 4.0% |
| security | 5 | 1.7% |
| map_images | 193 | 64.3% |
| fixups | 35 | 11.7% |
| objc_registration | 25 | 8.3% |
| protect | 8 | 2.7% |
| initialize | 22 | 7.3% |
| **Total** | **300** | **100%** |

The culprit is `map_images` at 64.3% of total runtime. That's the phase where we map the binary's segments into memory, resolve symbols, and set up the shim table. Why does it take so long?

Drill deeper. The `map_images` phase includes `DataSymbolStore::new()`, which resolves ObjC and Swift runtime global symbols via `dlsym`. Each `dlsym` call searches the host's dynamic linker tables. We call `dlsym` 10 times per `DataSymbolStore::new()` — once for each runtime global we need to locate: `_objc_msgSend`, `_objc_release`, `_objc_retain`, `_swift_allocObject`, and so on. And `DataSymbolStore::new()` is called on every pipeline run. Every time we load a binary, we re-resolve the same 10 symbols to the same addresses.

The addresses never change. `_objc_msgSend` is at the same address for the entire lifetime of the process. The dynamic linker mapped `libobjc.A.dylib` once, at process startup. These are static addresses being resolved dynamically, repeatedly, for no reason.

The fix is a `OnceLock` singleton:

```rust
static GLOBAL_STORE: OnceLock<DataSymbolStore> = OnceLock::new();

impl DataSymbolStore {
    pub fn global() -> &'static Self {
        GLOBAL_STORE.get_or_init(Self::new)
    }
}
```

`OnceLock` is a Rust standard library primitive: initialize once, return a reference forever after. The first call to `DataSymbolStore::global()` calls `Self::new()`, resolves all 10 `dlsym` symbols, builds the HashMap, and stores it in the static. Every subsequent call — the second binary, the hundredth binary — returns a reference to the already-initialized store. Zero `dlsym` calls. Zero HashMap allocation. One pointer dereference.

The same pattern applies throughout the system layer. The `ShimTable` — our registry of 400+ libSystem shim function pointers — was being rebuilt for every binary load. Same shims, same addresses, same table. Now it's a `OnceLock` singleton. The ObjC runtime's class table merging was recomputing category merges for synthetic Foundation classes that never change. Now the merge results are cached.

After optimization, the benchmarks on the same 39 corpus tools, 5 iterations each:

| Phase | Before (µs) | After (µs) | Reduction |
|:---|---:|---:|:---|
| parse | 12 | 12 | — |
| security | 5 | 5 | — |
| map_images | 193 | 16 | 12x |
| fixups | 35 | 32 | 1.1x |
| objc_registration | 25 | 12 | 2x |
| protect | 8 | 8 | — |
| initialize | 22 | 15 | 1.5x |
| **Total** | **300** | **100** | **3x** |

map_images: 64.3% → 15.6% of total. Three hundred microseconds → one hundred. A 3x improvement from targeted caching — no algorithmic changes, no data structure redesigns, no parallelism. Just stop doing work you already did.

The 100µs total is dominated by different things now. Parse and fixups are roughly equal. ObjC registration and initialization contribute their share. The global bottleneck shifted from map_images to execution — the binary's own startup code (constructors, static initializers) now takes as long as the loader's overhead. When the loader overhead is smaller than the binary's own initialization, you've won.

We verified the optimization doesn't break anything. The compatibility gate: 84 out of 84 tests pass. The corps tool battery: all 39 tools produce correct output. The worst-case overhead (the `regex_search` corpus tool, which uses ICU regex patterns internally) dropped from 0.4x overhead to 0.3x overhead. No regressions.

The approach here is pragmatic engineering, not academic optimization. We didn't profile first with `perf stat` and flamegraphs and cache-miss analysis. We instrumented the phases with timestamps, found the dominant cost center, asked "is this work repeated?", answered "yes", and applied the simplest caching mechanism available: a static singleton. Measure first. Don't guess where the time goes — instrument and find out. Then fix the measured bottleneck, not the one you assumed.

The deeper lesson is that loader performance is a systems problem, not an algorithmic one. The `dlsym` calls weren't algorithmically slow — each one takes ~26µs, which is fast for a symbol lookup. The problem was *repetition*: 10 calls × every pipeline run × no caching. The HashMap allocation wasn't slow — it's O(n) in the number of entries. The problem was allocating and populating it from scratch when the contents never change. `OnceLock` is the answer to both: do the work once, share the result forever.

This is also the chapter where performance meets architecture. The typestate pipeline from Scene 8.5 isn't just a correctness mechanism — it's also a performance mechanism. Because each phase is a distinct type, and each transition consumes the previous state, the pipeline naturally flows through each phase exactly once. There's no "accidentally map images twice" because you can't call `map_images()` on a `Loader<Mapped>` — the method doesn't exist on that type. The type system prevents the performance bug that the `OnceLock` singletons address at the data level.

One hundred microseconds. That's our tax. One-tenth of a millisecond to parse a Mach-O binary, evaluate security policy, memory-map segments, apply fixups, register ObjC classes, set memory protection, run initializers, and hand control to the entry point. For the build system scenario — 500 invocations per build — that's 50 milliseconds of total loader overhead. Invisible.

The whole system layer — 57 shim files, TLV patching, exception registration, signal handling, typestate enforcement, performance caching — exists so that the binary doesn't have to know any of it. The binary calls `stat()` and gets a struct. It reads a thread-local variable and gets a value. It throws an exception and lands in a catch block. It crashes and gets a diagnostic instead of silence. From the binary's perspective, it's running on macOS. The system layer maintains that illusion.

But an illusion that works for 39 corpus tools in a controlled test environment is not a proven system. The shim layer covers the symbols those tools need. The Foundation implementations cover the classes those tools call. The struct translations handle the layouts those tools touch. What about the tools we haven't tested? What about the Foundation methods we haven't shimmed? What about the edge cases in the 8% of libSystem symbols we haven't covered?

We need to test everything. Not 39 tools, but hundreds. Not against a handful of expected outputs, but against macOS ground truth — the real output of the real binary on the real platform. Not once, but continuously, as every change to the shim layer or Foundation implementation risks breaking something that used to work.

That's Chapter 9: The Test.
