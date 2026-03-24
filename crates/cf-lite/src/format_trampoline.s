// ARM64 stringWithFormat: variadic trampoline for darwin-loader-lab.
//
// When the ObjC runtime dispatches +[NSString stringWithFormat:], the
// IMP is called with the standard ARM64 ObjC calling convention:
//
//   x0 = cls (NSString class)
//   x1 = SEL (_cmd)
//   x2 = format (CFString* — named parameter, in register)
//   variadic args on STACK at [sp+0], [sp+8], [sp+16], ...
//
// The Rust inner function expects:
//   extern "C" fn(format: u64, a1: u64, a2: u64, a3: u64) -> u64
//
// This trampoline shuffles the arguments accordingly.

.global _shim_nsstring_format_asm
.p2align 4

_shim_nsstring_format_asm:
    // x2 = format CFString (named parameter — keep)
    mov     x0, x2              // arg0 = format
    ldr     x1, [sp, #0]       // arg1 = first variadic arg
    ldr     x2, [sp, #8]       // arg2 = second variadic arg
    ldr     x3, [sp, #16]      // arg3 = third variadic arg

    // Tail-call the Rust implementation.
    // Returns the new CFString pointer in x0.
    b       _shim_nsstring_string_with_format_inner
