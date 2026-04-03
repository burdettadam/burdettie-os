// x86_64 objc_msgSend trampoline for burdettie-os.
//
// Preserves the entire argument register file (rdi-r9, xmm0-7) across
// the class-table lookup, then tail-calls the resolved IMP. This is
// the x86_64 equivalent of the ARM64 trampoline.
//
// System V AMD64 ABI:
//   Integer args:  rdi, rsi, rdx, rcx, r8, r9  (6 registers)
//   Float args:    xmm0–xmm7                   (8 registers, 128-bit each)
//   Return:        rax (int), xmm0 (float)
//   Scratch:       rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11
//   Callee-saved:  rbx, rbp, r12, r13, r14, r15
//
// Flow:
//   1. Save rdi-r9, xmm0-7 to stack (176 bytes)
//   2. Call _objc_msg_lookup_imp(rdi=self, rsi=sel) → rax = IMP
//   3. Move IMP to r11 (scratch register)
//   4. Restore rdi-r9, xmm0-7
//   5. jmp *r11 (tail call — IMP sees original args)
//
// On x86_64, objc_msgSend_stret is identical to objc_msgSend under
// the System V ABI because struct return uses rdi (hidden first param),
// shifting self→rsi, SEL→rdx. The trampoline saves and restores all
// of those registers regardless, so it works for both conventions.
// objc_msgSend_fp2ret returns long double in x87 st(0) — the IMP
// handles that, our trampoline just dispatches.

#ifdef __x86_64__

.global _shim_objc_msg_send_x86_64_asm
.global _shim_objc_msg_send_stret_x86_64_asm
.global _shim_objc_msg_send_fp2ret_x86_64_asm
.p2align 4

_shim_objc_msg_send_x86_64_asm:
_shim_objc_msg_send_stret_x86_64_asm:
_shim_objc_msg_send_fp2ret_x86_64_asm:
    // ── nil check ──────────────────────────────────────────
    testq   %rdi, %rdi
    jz      .Lnil_return_x86

    // ── save state ─────────────────────────────────────────
    // Layout (176 bytes, 16-byte aligned):
    //   rsp+0:    rdi  (self)        8
    //   rsp+8:    rsi  (SEL)         8
    //   rsp+16:   rdx                8
    //   rsp+24:   rcx                8
    //   rsp+32:   r8                 8
    //   rsp+40:   r9                 8
    //   rsp+48:   xmm0              16
    //   rsp+64:   xmm1              16
    //   rsp+80:   xmm2              16
    //   rsp+96:   xmm3              16
    //   rsp+112:  xmm4              16
    //   rsp+128:  xmm5              16
    //   rsp+144:  xmm6              16
    //   rsp+160:  xmm7              16
    //   Total = 176 bytes
    //
    // After call instruction, rsp is 8-byte aligned (return address pushed).
    // 176 + 8 = 184, which is NOT 16-byte aligned.
    // We need 176 + 8 (padding) = 184 → sub 184 to get 16-byte alignment
    // with the return address: 184 + 8(retaddr) = 192 = 16*12 ✓
    subq    $184, %rsp

    // Save integer argument registers
    movq    %rdi,   (%rsp)
    movq    %rsi,  8(%rsp)
    movq    %rdx, 16(%rsp)
    movq    %rcx, 24(%rsp)
    movq    %r8,  32(%rsp)
    movq    %r9,  40(%rsp)

    // Save floating-point argument registers (128-bit SSE)
    movaps  %xmm0,  48(%rsp)
    movaps  %xmm1,  64(%rsp)
    movaps  %xmm2,  80(%rsp)
    movaps  %xmm3,  96(%rsp)
    movaps  %xmm4, 112(%rsp)
    movaps  %xmm5, 128(%rsp)
    movaps  %xmm6, 144(%rsp)
    movaps  %xmm7, 160(%rsp)

    // ── lookup ─────────────────────────────────────────────
    // rdi = self (already in rdi)
    // rsi = SEL  (already in rsi)
    // Call Rust lookup: extern "C" fn(self: u64, sel: u64) -> u64
    //   Returns: rax = IMP
    callq   _objc_msg_lookup_imp

    // IMP is now in rax — move to r11 (scratch, not an arg register)
    movq    %rax, %r11

    // ── restore state ──────────────────────────────────────
    // Restore integer argument registers
    movq      (%rsp), %rdi
    movq     8(%rsp), %rsi
    movq    16(%rsp), %rdx
    movq    24(%rsp), %rcx
    movq    32(%rsp), %r8
    movq    40(%rsp), %r9

    // Restore floating-point argument registers
    movaps   48(%rsp), %xmm0
    movaps   64(%rsp), %xmm1
    movaps   80(%rsp), %xmm2
    movaps   96(%rsp), %xmm3
    movaps  112(%rsp), %xmm4
    movaps  128(%rsp), %xmm5
    movaps  144(%rsp), %xmm6
    movaps  160(%rsp), %xmm7

    addq    $184, %rsp

    // ── tail call ──────────────────────────────────────────
    // Jump to IMP. The IMP sees the original caller's register
    // state exactly as if objc_msgSend were never called.
    jmpq    *%r11

.Lnil_return_x86:
    // ObjC nil-messaging semantics: return 0
    xorq    %rax, %rax
    xorps   %xmm0, %xmm0
    ret


// ── objc_msgSendSuper trampoline (x86_64) ──────────────────────────
//
// rdi = pointer to struct objc_super { id receiver; Class super_class; }
// rsi = SEL
//
// Flow:
//   1. Read receiver and super_class from the struct.
//   2. Save argument registers.
//   3. Call _objc_msg_super_lookup_imp(receiver, super_class, sel) → IMP.
//   4. Restore argument registers, but replace rdi with receiver.
//   5. Tail-call the IMP.

.global _shim_objc_msg_send_super_x86_64_asm
.global _shim_objc_msg_send_super_stret_x86_64_asm
.p2align 4

_shim_objc_msg_send_super_x86_64_asm:
_shim_objc_msg_send_super_stret_x86_64_asm:
    // Read objc_super struct before saving registers.
    // r10 = receiver, r11 = super_class
    movq    (%rdi),  %r10       // receiver = objc_super->receiver
    movq   8(%rdi),  %r11       // super_class = objc_super->super_class

    // ── save state ─────────────────────────────────────────
    subq    $200, %rsp          // 184 (args) + 16 (receiver + super_class)

    // Save original rdi-r9 (caller's args)
    movq    %rdi,   (%rsp)
    movq    %rsi,  8(%rsp)
    movq    %rdx, 16(%rsp)
    movq    %rcx, 24(%rsp)
    movq    %r8,  32(%rsp)
    movq    %r9,  40(%rsp)

    // Save floating-point registers
    movaps  %xmm0,  48(%rsp)
    movaps  %xmm1,  64(%rsp)
    movaps  %xmm2,  80(%rsp)
    movaps  %xmm3,  96(%rsp)
    movaps  %xmm4, 112(%rsp)
    movaps  %xmm5, 128(%rsp)
    movaps  %xmm6, 144(%rsp)
    movaps  %xmm7, 160(%rsp)

    // Save receiver and super_class for restore
    movq    %r10, 176(%rsp)     // receiver
    movq    %r11, 184(%rsp)     // super_class

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver: u64, super_class: u64, sel: u64) -> u64
    movq    %r10, %rdi          // arg0 = receiver
    movq    %r11, %rsi          // arg1 = super_class
    movq   8(%rsp), %rdx        // arg2 = saved rsi = SEL
    callq   _objc_msg_super_lookup_imp

    // IMP → r11
    movq    %rax, %r11

    // ── restore state ──────────────────────────────────────
    movq    176(%rsp), %r10     // receiver

    // Restore floating-point registers
    movaps   48(%rsp), %xmm0
    movaps   64(%rsp), %xmm1
    movaps   80(%rsp), %xmm2
    movaps   96(%rsp), %xmm3
    movaps  112(%rsp), %xmm4
    movaps  128(%rsp), %xmm5
    movaps  144(%rsp), %xmm6
    movaps  160(%rsp), %xmm7

    // Restore rsi-r9 (skip rdi — will set to receiver)
    movq     8(%rsp), %rsi
    movq    16(%rsp), %rdx
    movq    24(%rsp), %rcx
    movq    32(%rsp), %r8
    movq    40(%rsp), %r9

    addq    $200, %rsp

    // rdi = receiver (the IMP expects self, not the struct pointer)
    movq    %r10, %rdi

    // ── tail call ──────────────────────────────────────────
    jmpq    *%r11


// ── objc_msgSendSuper2 trampoline (x86_64) ─────────────────────────
//
// Same as Super, but super_class is the current class.
// The Rust lookup follows current_class.superclass.
//
// rdi = pointer to struct objc_super { id receiver; Class current_class; }
// rsi = SEL

.global _shim_objc_msg_send_super2_x86_64_asm
.global _shim_objc_msg_send_super2_stret_x86_64_asm
.p2align 4

_shim_objc_msg_send_super2_x86_64_asm:
_shim_objc_msg_send_super2_stret_x86_64_asm:
    // Read objc_super struct
    movq    (%rdi),  %r10       // receiver
    movq   8(%rdi),  %r11       // current_class (NOT the super)

    // ── save state ─────────────────────────────────────────
    subq    $200, %rsp

    movq    %rdi,   (%rsp)
    movq    %rsi,  8(%rsp)
    movq    %rdx, 16(%rsp)
    movq    %rcx, 24(%rsp)
    movq    %r8,  32(%rsp)
    movq    %r9,  40(%rsp)

    movaps  %xmm0,  48(%rsp)
    movaps  %xmm1,  64(%rsp)
    movaps  %xmm2,  80(%rsp)
    movaps  %xmm3,  96(%rsp)
    movaps  %xmm4, 112(%rsp)
    movaps  %xmm5, 128(%rsp)
    movaps  %xmm6, 144(%rsp)
    movaps  %xmm7, 160(%rsp)

    movq    %r10, 176(%rsp)     // receiver
    movq    %r11, 184(%rsp)     // current_class

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver, current_class, sel) -> IMP
    movq    %r10, %rdi
    movq    %r11, %rsi
    movq   8(%rsp), %rdx        // SEL from saved rsi
    callq   _objc_msg_super2_lookup_imp

    movq    %rax, %r11

    // ── restore state ──────────────────────────────────────
    movq    176(%rsp), %r10     // receiver

    movaps   48(%rsp), %xmm0
    movaps   64(%rsp), %xmm1
    movaps   80(%rsp), %xmm2
    movaps   96(%rsp), %xmm3
    movaps  112(%rsp), %xmm4
    movaps  128(%rsp), %xmm5
    movaps  144(%rsp), %xmm6
    movaps  160(%rsp), %xmm7

    movq     8(%rsp), %rsi
    movq    16(%rsp), %rdx
    movq    24(%rsp), %rcx
    movq    32(%rsp), %r8
    movq    40(%rsp), %r9

    addq    $200, %rsp

    movq    %r10, %rdi

    // ── tail call ──────────────────────────────────────────
    jmpq    *%r11

#endif /* __x86_64__ */
