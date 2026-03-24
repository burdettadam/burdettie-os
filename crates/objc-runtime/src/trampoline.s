// ARM64 objc_msgSend trampoline for burdetty-os.
//
// Preserves the entire argument register file (x0-x7, d0-d7) across
// the class-table lookup, then tail-calls the resolved IMP. This is
// required for correct calling-convention behavior — a Rust
// `extern "C"` function cannot preserve floating-point registers or
// guarantee the stack frame matches what the caller constructed.
//
// Flow:
//   1. Save x0-x7, d0-d7, lr to stack (192 bytes)
//   2. Call _objc_msg_lookup_imp(x0=self, x1=sel) → returns IMP in x0
//   3. Move IMP to x16 (scratch register per ARM64 ABI)
//   4. Restore x0-x7, d0-d7, lr
//   5. Branch to x16 (tail call — IMP sees original args)

.global _shim_objc_msg_send_asm
.p2align 4

_shim_objc_msg_send_asm:
    // ── nil check ──────────────────────────────────────────
    cbz     x0, .Lnil_return

    // ── save state ─────────────────────────────────────────
    // 192 bytes: 8 x-regs (64) + 8 d-regs (64) + lr + pad (16) = 144
    // But we need 16-byte alignment, so we use a 176-byte frame:
    //   sp+0..63:    x0-x7 (64 bytes)
    //   sp+64..127:  d0-d7 (64 bytes)
    //   sp+128..143: x30(lr) + x16(pad) (16 bytes)
    // Total = 144 bytes, round up to 160 for 16-byte alignment
    sub     sp, sp, #160

    // Save integer argument registers
    stp     x0, x1, [sp, #0]
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]

    // Save floating-point argument registers
    stp     d0, d1, [sp, #64]
    stp     d2, d3, [sp, #80]
    stp     d4, d5, [sp, #96]
    stp     d6, d7, [sp, #112]

    // Save link register
    stp     x30, xzr, [sp, #128]

    // ── lookup ─────────────────────────────────────────────
    // x0 = self (already in x0)
    // x1 = SEL  (already in x1)
    // Call Rust lookup: extern "C" fn(self: u64, sel: u64) -> (u64, u64)
    //   Returns: x0 = IMP, x1 = translated selector (0 if no translation)
    bl      _objc_msg_lookup_imp

    // IMP is now in x0 — move to x16 (intra-procedure-call scratch)
    mov     x16, x0

    // If x1 != 0, the lookup translated the selector for host fallback.
    // Overwrite the saved SEL on the stack so the restore picks it up.
    cbz     x1, .Lno_sel_xlat
    str     x1, [sp, #8]
.Lno_sel_xlat:

    // ── restore state ──────────────────────────────────────
    // Restore link register
    ldp     x30, xzr, [sp, #128]

    // Restore floating-point argument registers
    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]

    // Restore integer argument registers
    ldp     x0, x1, [sp, #0]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #160

    // ── tail call ──────────────────────────────────────────
    // Branch to IMP. The IMP sees the original caller's register
    // state exactly as if objc_msgSend were never called.
    br      x16

.Lnil_return:
    // ObjC nil-messaging semantics: return 0
    mov     x0, #0
    // Also zero d0 for floating-point returns from nil
    fmov    d0, xzr
    ret

// ── objc_msgSendSuper trampoline ───────────────────────────────────
//
// x0 = pointer to struct objc_super { id receiver; Class super_class; }
// x1 = SEL
//
// Flow:
//   1. Read receiver and super_class from the struct.
//   2. Save argument registers.
//   3. Call _objc_msg_super_lookup_imp(receiver, super_class, sel) → IMP.
//   4. Restore argument registers, but replace x0 with receiver (not the
//      struct pointer — the IMP's self parameter must be the object).
//   5. Tail-call the IMP.

.global _shim_objc_msg_send_super_asm
.p2align 4

_shim_objc_msg_send_super_asm:
    // Read objc_super struct *before* saving registers.
    // x9 = receiver, x10 = super_class
    ldr     x9,  [x0, #0]      // receiver = objc_super->receiver
    ldr     x10, [x0, #8]      // super_class = objc_super->super_class

    // ── save state ─────────────────────────────────────────
    sub     sp, sp, #176

    // Save original x0-x7 (caller's args)
    stp     x0, x1, [sp, #0]
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]

    // Save floating-point registers
    stp     d0, d1, [sp, #64]
    stp     d2, d3, [sp, #80]
    stp     d4, d5, [sp, #96]
    stp     d6, d7, [sp, #112]

    // Save lr + the receiver (x9) for restore
    stp     x30, x9, [sp, #128]
    // Save x10 (super_class) too
    stp     x10, xzr, [sp, #144]

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver: u64, super_class: u64, sel: u64) -> u64
    mov     x0, x9              // arg0 = receiver
    mov     x1, x10             // arg1 = super_class
    // x1 had SEL, but we saved it. Load SEL from the saved x1:
    ldr     x2, [sp, #8]        // arg2 = saved x1 = SEL
    bl      _objc_msg_super_lookup_imp

    // IMP → x16
    mov     x16, x0

    // ── restore state ──────────────────────────────────────
    ldp     x30, x9, [sp, #128]

    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]

    // Restore x1-x7 (skip x0, will set to receiver)
    ldr     x1, [sp, #8]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #176

    // x0 = receiver (the IMP expects self, not the struct pointer)
    mov     x0, x9

    // ── tail call ──────────────────────────────────────────
    br      x16

// ── objc_msgSendSuper2 trampoline ──────────────────────────────────
//
// Same as objc_msgSendSuper, but super_class is the *current* class
// (not the parent). The Rust lookup function will follow
// current_class.superclass_addr to find the actual super.
//
// x0 = pointer to struct objc_super { id receiver; Class current_class; }
// x1 = SEL

.global _shim_objc_msg_send_super2_asm
.p2align 4

_shim_objc_msg_send_super2_asm:
    // Read objc_super struct
    ldr     x9,  [x0, #0]      // receiver
    ldr     x10, [x0, #8]      // current_class (NOT the super)

    // ── save state ─────────────────────────────────────────
    sub     sp, sp, #176

    stp     x0, x1, [sp, #0]
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]

    stp     d0, d1, [sp, #64]
    stp     d2, d3, [sp, #80]
    stp     d4, d5, [sp, #96]
    stp     d6, d7, [sp, #112]

    stp     x30, x9, [sp, #128]
    stp     x10, xzr, [sp, #144]

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver, current_class, sel) -> IMP
    mov     x0, x9
    mov     x1, x10
    ldr     x2, [sp, #8]        // SEL from saved x1
    bl      _objc_msg_super2_lookup_imp

    mov     x16, x0

    // ── restore state ──────────────────────────────────────
    ldp     x30, x9, [sp, #128]

    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]

    ldr     x1, [sp, #8]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #176

    mov     x0, x9

    // ── tail call ──────────────────────────────────────────
    br      x16


