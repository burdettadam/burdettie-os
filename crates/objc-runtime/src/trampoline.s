// ARM64 objc_msgSend trampoline for burdettie-os.
//
// Cross-platform: builds on macOS (Mach-O) and Linux (ELF).
// Mach-O uses _underscore C symbol prefix and @PAGE/@PAGEOFF relocations.
// ELF uses bare names and GOT-relative addressing for external symbols
// (required for PIC shared libraries on Linux).

#ifdef __APPLE__
#define CSYM(name) _##name
// macOS: direct page-relative addressing
#define LOAD_GLOBAL(reg, sym) \
    adrp reg, sym@PAGE ; \
    ldr  reg, [reg, sym@PAGEOFF]
#else
#define CSYM(name) name
// Linux ELF: GOT-relative addressing (external symbols in PIC)
#define LOAD_GLOBAL(reg, sym) \
    adrp reg, :got:sym ; \
    ldr  reg, [reg, :got_lo12:sym] ; \
    ldr  reg, [reg]
#endif

// Preserves the entire argument register file (x0-x7, x8, d0-d7) across
// the class-table lookup, then tail-calls the resolved IMP. This is
// required for correct calling-convention behavior — a Rust
// `extern "C"` function cannot preserve floating-point registers or
// guarantee the stack frame matches what the caller constructed.
//
// x8 is the "indirect result location register" in the ARM64 ABI.
// Methods returning large structs (>16 bytes, e.g. CGRect) receive
// the caller's return-buffer pointer in x8. We must preserve it
// across the lookup so the IMP writes to the correct address.
//
// Flow:
//   1. Save x0-x7, x8, d0-d7, lr to stack (176 bytes)
//   2. Call _objc_msg_lookup_imp(x0=self, x1=sel) → returns IMP in x0
//   3. Move IMP to x16 (scratch register per ARM64 ABI)
//   4. Restore x0-x7, x8, d0-d7, lr
//   5. Branch to x16 (tail call — IMP sees original args)
//
// On ARM64, objc_msgSend_stret is identical to objc_msgSend because
// x8 is preserved through the trampoline. Apple defines it as an alias.
// Similarly, objc_msgSend_fp2ret is identical (fp2ret only matters on x86).

.global CSYM(shim_objc_msg_send_asm)
.global CSYM(shim_objc_msg_send_stret_asm)
.global CSYM(shim_objc_msg_send_fp2ret_asm)
.p2align 4

CSYM(shim_objc_msg_send_asm):
CSYM(shim_objc_msg_send_stret_asm):
CSYM(shim_objc_msg_send_fp2ret_asm):
    // ── nil check ──────────────────────────────────────────
    cbz     x0, .Lnil_return

    // ── save state ─────────────────────────────────────────
    // 176 bytes (16-byte aligned):
    //   sp+0..63:    x0-x7    (64 bytes, 4 stp)
    //   sp+64..127:  d0-d7    (64 bytes, 4 stp)
    //   sp+128..143: x30(lr) + x8(struct return ptr) (16 bytes)
    //   sp+144..159: padding  (16 bytes)
    // Total = 176 bytes
    sub     sp, sp, #176

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

    // Save link register and x8 (indirect result / struct return pointer)
    stp     x30, x8, [sp, #128]

    // ── lookup ─────────────────────────────────────────────
    // x0 = self (already in x0)
    // x1 = SEL  (already in x1)
    // Call Rust lookup: extern "C" fn(self: u64, sel: u64) -> u64
    //   Returns: x0 = IMP
    bl      CSYM(objc_msg_lookup_imp)

    // IMP is now in x0 — move to x16 (intra-procedure-call scratch)
    mov     x16, x0

    // ── restore state ──────────────────────────────────────
    // Restore link register and x8 (struct return pointer)
    ldp     x30, x8, [sp, #128]

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

    add     sp, sp, #176

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
//   2. Save argument registers (including x8 for struct returns).
//   3. Call _objc_msg_super_lookup_imp(receiver, super_class, sel) → IMP.
//   4. Restore argument registers, but replace x0 with receiver (not the
//      struct pointer — the IMP's self parameter must be the object).
//   5. Tail-call the IMP.

.global CSYM(shim_objc_msg_send_super_asm)
.global CSYM(shim_objc_msg_send_super_stret_asm)
.p2align 4

CSYM(shim_objc_msg_send_super_asm):
CSYM(shim_objc_msg_send_super_stret_asm):
    // Read objc_super struct *before* saving registers.
    // x9 = receiver, x10 = super_class
    ldr     x9,  [x0, #0]      // receiver = objc_super->receiver
    ldr     x10, [x0, #8]      // super_class = objc_super->super_class

    // ── save state ─────────────────────────────────────────
    sub     sp, sp, #192

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
    // Save x10 (super_class) + x8 (struct return pointer)
    stp     x10, x8, [sp, #144]

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver: u64, super_class: u64, sel: u64) -> u64
    mov     x0, x9              // arg0 = receiver
    mov     x1, x10             // arg1 = super_class
    // x1 had SEL, but we saved it. Load SEL from the saved x1:
    ldr     x2, [sp, #8]        // arg2 = saved x1 = SEL
    bl      CSYM(objc_msg_super_lookup_imp)

    // IMP → x16
    mov     x16, x0

    // ── restore state ──────────────────────────────────────
    ldp     x30, x9, [sp, #128]
    ldp     x10, x8, [sp, #144]

    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]

    // Restore x1-x7 (skip x0, will set to receiver)
    ldr     x1, [sp, #8]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #192

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

.global CSYM(shim_objc_msg_send_super2_asm)
.global CSYM(shim_objc_msg_send_super2_stret_asm)
.p2align 4

CSYM(shim_objc_msg_send_super2_asm):
CSYM(shim_objc_msg_send_super2_stret_asm):
    // Read objc_super struct
    ldr     x9,  [x0, #0]      // receiver
    ldr     x10, [x0, #8]      // current_class (NOT the super)

    // ── save state ─────────────────────────────────────────
    sub     sp, sp, #192

    stp     x0, x1, [sp, #0]
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]

    stp     d0, d1, [sp, #64]
    stp     d2, d3, [sp, #80]
    stp     d4, d5, [sp, #96]
    stp     d6, d7, [sp, #112]

    stp     x30, x9, [sp, #128]
    stp     x10, x8, [sp, #144]

    // ── lookup ─────────────────────────────────────────────
    // Call: extern "C" fn(receiver, current_class, sel) -> IMP
    mov     x0, x9
    mov     x1, x10
    ldr     x2, [sp, #8]        // SEL from saved x1
    bl      CSYM(objc_msg_super2_lookup_imp)

    mov     x16, x0

    // ── restore state ──────────────────────────────────────
    ldp     x30, x9, [sp, #128]
    ldp     x10, x8, [sp, #144]

    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]

    ldr     x1, [sp, #8]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #192

    mov     x0, x9

    // ── tail call ──────────────────────────────────────────
    br      x16


// ── Host-object forwarding trampoline ──────────────────────────────
//
// Called as an IMP when the receiver's ISA is not registered in the
// guest ObjC runtime (i.e. it's a real host Foundation object returned
// by a host API like CFPreferencesCopyValue).
//
// Re-interns the guest selector through host sel_registerName, then
// tail-calls host objc_msgSend with the correct host SEL.
//
// Preserves all argument registers (x0-x7, x8, d0-d7) so methods
// with float args or large-struct returns work correctly.
//
// Reads function pointers from Rust globals:
//   HOST_SEL_REGISTER_NAME — host sel_registerName()
//   HOST_OBJC_MSG_SEND     — host objc_msgSend()

.global CSYM(host_objc_forward_asm)
.p2align 4

CSYM(host_objc_forward_asm):
    // x0 = obj, x1 = guest SEL (C string), x2-x7 = args
    // x8 = struct return ptr, d0-d7 = float args

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
    stp     x30, x8, [sp, #128]

    // ── re-intern selector ─────────────────────────────────
    // Load HOST_SEL_REGISTER_NAME function pointer
    LOAD_GLOBAL(x17, CSYM(HOST_SEL_REGISTER_NAME))
    cbz     x17, .Lhost_fwd_nil

    // Call sel_registerName(guest_SEL)
    mov     x0, x1              // x0 = guest SEL string
    blr     x17                 // x0 = host SEL
    str     x0, [sp, #8]       // overwrite saved x1 with host SEL

    // ── load host objc_msgSend ─────────────────────────────
    LOAD_GLOBAL(x17, CSYM(HOST_OBJC_MSG_SEND))
    cbz     x17, .Lhost_fwd_nil

    // ── restore state ──────────────────────────────────────
    ldp     x30, x8, [sp, #128]
    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]
    ldp     x0, x1, [sp, #0]   // x1 = host SEL (overwritten above)
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #176

    // ── tail-call host objc_msgSend ────────────────────────
    br      x17

.Lhost_fwd_nil:
    // Host functions not available — return nil
    ldp     x30, x8, [sp, #128]
    add     sp, sp, #176
    mov     x0, #0
    fmov    d0, xzr
    ret


