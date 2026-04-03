// ARM64 NSLog variadic trampoline for burdettie-os.
//
// ARM64 macOS variadic calling convention:
//   x0 = format (named parameter, in register)
//   variadic args are passed on the STACK at [sp+0], [sp+8], ...
//
// The Rust shim expects all args in registers:
//   extern "C" fn(format: u64, a1: u64, a2: u64, a3: u64)
//
// This trampoline loads the first 3 stack-passed variadic args into
// x1-x3 and tail-calls the Rust implementation.
//
// _shim_nslog_inner is defined in nslog.rs (#[no_mangle]) within
// the same crate, so the symbol is always available at link time.

.global _shim_nslog_asm
.p2align 4

_shim_nslog_asm:
    // x0 = format (keep as-is, named parameter)
    // Read variadic args from caller's stack frame
    ldr     x1, [sp, #0]        // first variadic arg
    ldr     x2, [sp, #8]        // second variadic arg
    ldr     x3, [sp, #16]       // third variadic arg

    // Tail-call the Rust NSLog implementation.
    // shim_nslog_inner(format, a1, a2, a3) is a regular C function —
    // all 4 args are in registers now.
    b       _shim_nslog_inner
