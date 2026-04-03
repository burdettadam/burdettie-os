// x86_64 NSLog variadic trampoline for burdettie-os.
//
// On x86_64 System V ABI, variadic args are passed in registers —
// the same registers as non-variadic calls:
//   rdi = format (named parameter)
//   rsi = first variadic arg
//   rdx = second variadic arg
//   rcx = third variadic arg
//
// The Rust inner function:
//   extern "C" fn shim_nslog_inner(format: u64, a1: u64, a2: u64, a3: u64)
//
// expects: rdi=format, rsi=a1, rdx=a2, rcx=a3  — which is exactly
// what the caller set up. So we just tail-call directly.

#ifdef __x86_64__

.global _shim_nslog_x86_64_asm
.p2align 4

_shim_nslog_x86_64_asm:
    // Args already in the right registers:
    //   rdi = format, rsi = arg1, rdx = arg2, rcx = arg3
    jmp     _shim_nslog_inner

#endif /* __x86_64__ */
