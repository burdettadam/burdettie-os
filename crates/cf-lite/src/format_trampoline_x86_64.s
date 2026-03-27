// x86_64 stringWithFormat: variadic trampoline for burdetty-os.
//
// When the ObjC runtime dispatches +[NSString stringWithFormat:], the
// IMP is called with the x86_64 ObjC calling convention:
//
//   rdi = cls  (NSString class)
//   rsi = SEL  (_cmd)
//   rdx = format (CFString* — named parameter)
//   rcx = first variadic arg
//   r8  = second variadic arg
//   r9  = third variadic arg
//
// The Rust inner function expects:
//   extern "C" fn(format: u64, a1: u64, a2: u64, a3: u64) -> u64
//
// Register mapping:
//   rdi ← rdx  (format)
//   rsi ← rcx  (arg1)
//   rdx ← r8   (arg2)
//   rcx ← r9   (arg3)

#ifdef __x86_64__

.global _shim_nsstring_format_x86_64_asm
.p2align 4

_shim_nsstring_format_x86_64_asm:
    // Shuffle registers: discard cls (rdi) and SEL (rsi),
    // move format → rdi, variadics → rsi/rdx/rcx.
    // Order matters to avoid clobbering:
    //   rdi ← rdx    (format)     — clobbers rdi (cls, no longer needed)
    //   rsi ← rcx    (arg1)       — clobbers rsi (SEL, no longer needed)
    //   rdx ← r8     (arg2)       — clobbers rdx (format, already saved to rdi)
    //   rcx ← r9     (arg3)       — clobbers rcx (arg1, already saved to rsi)
    movq    %rdx, %rdi
    movq    %rcx, %rsi
    movq    %r8,  %rdx
    movq    %r9,  %rcx

    // Tail-call the Rust implementation.
    // Returns the new CFString pointer in rax.
    jmp     _shim_nsstring_string_with_format_inner

#endif /* __x86_64__ */
