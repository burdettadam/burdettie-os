//! Branch and call instruction execution.

use super::{ExitReason, Interpreter, StepResult, SENTINEL_LR};
use crate::decode::Instruction;

// ── PAC bit positions ───────────────────────────────────────────────────────
// In userspace (EL0, TTBR0), virtual addresses use bits[47:0] and bit 55
// indicates the top-byte-ignore region. PAC bits occupy [54:48] (7 bits).
const PAC_MASK: u64  = 0x007F_0000_0000_0000; // bits 54:48
const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // bits 47:0

/// Compute a PAC value using a simplified SipHash-like mixing of pointer,
/// context and key. Returns the original pointer with PAC bits inserted.
fn pac_sign(ptr: u64, context: u64, key: u128) -> u64 {
    let addr = ptr & ADDR_MASK;
    let k_lo = key as u64;
    let k_hi = (key >> 64) as u64;
    // Mix pointer, context, and key through a series of rounds.
    let mut h = addr ^ context ^ k_lo;
    h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15); // golden-ratio constant
    h ^= h >> 28;
    h ^= k_hi;
    h = h.wrapping_mul(0x517C_C1B7_2722_0A95);
    h ^= h >> 32;
    let pac = (h << 48) & PAC_MASK;
    // Preserve original address bits — insert PAC into upper bits.
    (ptr & !PAC_MASK) | pac
}

/// Authenticate a signed pointer: recompute PAC and verify it matches.
/// If valid, strip PAC bits to return the original pointer.
/// If invalid, set error bit (bit 54) to poison the pointer.
fn pac_auth(signed_ptr: u64, context: u64, key: u128) -> u64 {
    let addr = signed_ptr & ADDR_MASK;
    let expected = pac_sign(addr, context, key);
    if (signed_ptr & PAC_MASK) == (expected & PAC_MASK) {
        // Valid — return stripped pointer.
        addr
    } else {
        // Invalid — set error bit (bit 54) to poison the pointer.
        addr | (1u64 << 54)
    }
}

/// Strip PAC bits, returning the bare virtual address.
fn pac_strip(ptr: u64) -> u64 {
    ptr & ADDR_MASK
}

// ── System register encodings (bits[20:5] of MRS/MSR instruction) ───────────
// Formula: (o0 << 15) | (1 << 14) | (op1 << 11) | (CRn << 7) | (CRm << 3) | op2
// where o0=1 for op0=3, bit14=1 is always set in register access form.
/// TPIDR_EL0 (Thread Pointer ID, EL0): op0=3, op1=3, CRn=13, CRm=0, op2=2
const SYSREG_TPIDR_EL0:  u16 = (1 << 15) | (1 << 14) | (3 << 11) | (13 << 7) | (0 << 3) | 2;
/// CTR_EL0 (Cache Type Register): op0=3, op1=3, CRn=0, CRm=0, op2=1
const SYSREG_CTR_EL0:    u16 = (1 << 15) | (1 << 14) | (3 << 11) | (0 << 7)  | (0 << 3) | 1;
/// CNTVCT_EL0 (Virtual Timer Counter): op0=3, op1=3, CRn=14, CRm=0, op2=2
const SYSREG_CNTVCT_EL0: u16 = (1 << 15) | (1 << 14) | (3 << 11) | (14 << 7) | (0 << 3) | 2;
/// DAIF (Debug/Async/IRQ/FIQ mask): op0=3, op1=3, CRn=4, CRm=2, op2=1
const SYSREG_DAIF:       u16 = (1 << 15) | (1 << 14) | (3 << 11) | (4 << 7)  | (2 << 3) | 1;
/// NZCV (Condition flags): op0=3, op1=3, CRn=4, CRm=2, op2=0
const SYSREG_NZCV:       u16 = (1 << 15) | (1 << 14) | (3 << 11) | (4 << 7)  | (2 << 3) | 0;
/// FPCR (FP Control Register): op0=3, op1=3, CRn=4, CRm=4, op2=0
const SYSREG_FPCR:       u16 = (1 << 15) | (1 << 14) | (3 << 11) | (4 << 7)  | (4 << 3) | 0;
/// FPSR (FP Status Register): op0=3, op1=3, CRn=4, CRm=4, op2=1
const SYSREG_FPSR:       u16 = (1 << 15) | (1 << 14) | (3 << 11) | (4 << 7)  | (4 << 3) | 1;

/// Synthetic CTR_EL0 value:
/// IcacheLineSize=4 (16 words = 64 bytes), DcacheLineSize=4, IDC=1, DIC=1, L1Ip=3 (VIPT)
const CTR_EL0_VALUE: u64 = 0x8201_4004;


impl Interpreter {
    pub(super) fn exec_branch(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── Branches ───────────────────────────────────────────
            Instruction::B { offset } => {
                self.cpu.pc = offset as u64;
            }
            Instruction::Bl { offset } => {
                self.cpu.x[30] = pc + 4; // LR = return address
                let target = offset as u64;
                // Check if this is an FFI call (registered or unmapped).
                if self.ffi.is_ffi_target(target) || !self.mem.is_valid(target) {
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Br { rn } => {
                let target = self.cpu.read_reg(rn);
                if !self.mem.is_valid(target) {
                    // Target is outside mapped memory — treat as FFI call.
                    // BR is a tail-call (no LR update), so we return to caller's LR.
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Blr { rn } => {
                let target = self.cpu.read_reg(rn);
                self.cpu.x[30] = pc + 4;
                if self.ffi.is_ffi_target(target) || !self.mem.is_valid(target) {
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Ret { rn } => {
                let target = self.cpu.read_reg(rn);
                if target == SENTINEL_LR {
                    log::trace!("RET to sentinel: x0={:#x} sp={:#x}", self.cpu.x[0], self.cpu.sp);
                    return StepResult::Exit(ExitReason::Returned {
                        code: self.cpu.x[0] as i64,
                    });
                }
                if !self.mem.is_valid(target) {
                    return StepResult::Exit(ExitReason::ReturnedToHost { lr: target });
                }
                self.cpu.pc = target;
            }
            Instruction::Bcond { cond, offset } => {
                if cond.eval(self.cpu.nzcv) {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Cbz { sf, rt, offset } => {
                let val = if sf {
                    self.cpu.read_reg(rt)
                } else {
                    self.cpu.read_reg(rt) & 0xFFFF_FFFF
                };
                if val == 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Cbnz { sf, rt, offset } => {
                let val = if sf {
                    self.cpu.read_reg(rt)
                } else {
                    self.cpu.read_reg(rt) & 0xFFFF_FFFF
                };
                if val != 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Tbz { rt, bit, offset } => {
                let val = self.cpu.read_reg(rt);
                if (val >> bit) & 1 == 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Tbnz { rt, bit, offset } => {
                let val = self.cpu.read_reg(rt);
                if (val >> bit) & 1 != 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }

            Instruction::Nop => {
                self.cpu.pc = pc + 4;
            }
            Instruction::Svc { .. } => {
                // SVC is treated as NOP in the interpreter (no kernel syscall support).
                self.cpu.pc = pc + 4;
            }
            Instruction::Mrs { rt, sysreg } => {
                // Known system registers: return plausible stub values.
                let val = match sysreg {
                    SYSREG_TPIDR_EL0   => self.cpu.tpidr_el0,
                    SYSREG_CTR_EL0     => CTR_EL0_VALUE,
                    SYSREG_CNTVCT_EL0  => {
                        // Virtual timer counter: approximate using monotonic clock.
                        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
                        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
                        (ts.tv_sec as u64).wrapping_mul(1_000_000_000)
                            .wrapping_add(ts.tv_nsec as u64)
                    }
                    SYSREG_NZCV => {
                        let n = self.cpu.nzcv.n as u64;
                        let z = self.cpu.nzcv.z as u64;
                        let c = self.cpu.nzcv.c as u64;
                        let v = self.cpu.nzcv.v as u64;
                        (n << 31) | (z << 30) | (c << 29) | (v << 28)
                    }
                    SYSREG_DAIF => 0x3C0, // D,A,I,F all masked (interrupts disabled)
                    SYSREG_FPCR => self.cpu.fpcr,
                    SYSREG_FPSR => self.cpu.fpsr,
                    _ => {
                        log::debug!("MRS: unknown sysreg {sysreg:#06x} at {pc:#x}, returning 0");
                        0
                    }
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Msr { rt, sysreg } => {
                let val = self.cpu.read_reg_zr(rt);
                match sysreg {
                    SYSREG_TPIDR_EL0 => self.cpu.tpidr_el0 = val,
                    SYSREG_NZCV => {
                        self.cpu.nzcv.n = (val >> 31) & 1 == 1;
                        self.cpu.nzcv.z = (val >> 30) & 1 == 1;
                        self.cpu.nzcv.c = (val >> 29) & 1 == 1;
                        self.cpu.nzcv.v = (val >> 28) & 1 == 1;
                    }
                    SYSREG_FPCR => self.cpu.fpcr = val,
                    SYSREG_FPSR => self.cpu.fpsr = val,
                    SYSREG_DAIF => {} // ignore DAI flag writes
                    _ => {
                        log::debug!("MSR: unknown sysreg {sysreg:#06x} ← {val:#x} at {pc:#x}");
                    }
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Brk { imm } => {
                log::debug!("BRK #{imm} at {pc:#x}");
                return StepResult::Exit(ExitReason::Unimplemented { pc, raw: 0xD420_0000 | ((imm as u32) << 5) });
            }

            // ── Pointer Authentication (ARMv8.3 arm64e) ──────────
            Instruction::PacIA { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let signed = pac_sign(ptr, ctx, self.cpu.pac_ia_key);
                self.cpu.write_reg_zr(rd, signed);
                self.cpu.pc = pc + 4;
            }
            Instruction::PacIB { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let signed = pac_sign(ptr, ctx, self.cpu.pac_ib_key);
                self.cpu.write_reg_zr(rd, signed);
                self.cpu.pc = pc + 4;
            }
            Instruction::PacDA { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let signed = pac_sign(ptr, ctx, self.cpu.pac_da_key);
                self.cpu.write_reg_zr(rd, signed);
                self.cpu.pc = pc + 4;
            }
            Instruction::PacDB { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let signed = pac_sign(ptr, ctx, self.cpu.pac_db_key);
                self.cpu.write_reg_zr(rd, signed);
                self.cpu.pc = pc + 4;
            }
            Instruction::AutIA { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let result = pac_auth(ptr, ctx, self.cpu.pac_ia_key);
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AutIB { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let result = pac_auth(ptr, ctx, self.cpu.pac_ib_key);
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AutDA { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let result = pac_auth(ptr, ctx, self.cpu.pac_da_key);
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AutDB { rd, rn } => {
                let ptr = self.cpu.read_reg(rd);
                let ctx = self.cpu.read_reg_zr(rn);
                let result = pac_auth(ptr, ctx, self.cpu.pac_db_key);
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Xpaci { rd } => {
                let ptr = self.cpu.read_reg(rd);
                self.cpu.write_reg_zr(rd, pac_strip(ptr));
                self.cpu.pc = pc + 4;
            }
            Instruction::Xpacd { rd } => {
                let ptr = self.cpu.read_reg(rd);
                self.cpu.write_reg_zr(rd, pac_strip(ptr));
                self.cpu.pc = pc + 4;
            }

            _ => unreachable!("exec_branch called with non-branch instruction"),
        }
        StepResult::Continue
    }

    /// Handle an FFI call: extract args from registers, call the host function,
    /// and process the result (return, exit, or tail-call).
    fn do_ffi_call(&mut self, target: u64, _call_pc: u64) -> StepResult {
        // The interpreter stack is mmap'd at a real virtual address, so
        // guest pointers (stack, segment) ARE valid host pointers.
        // No address translation is needed at the FFI boundary.
        //
        // Enforce 16-byte SP alignment (ARM64 ABI requirement).  Misaligned
        // SP can cause SIGSEGV in host NEON/SIMD code.
        let aligned_sp = self.cpu.sp & !0xF;
        let args = crate::ffi::FfiCallArgs {
            x: [
                self.cpu.x[0], self.cpu.x[1], self.cpu.x[2], self.cpu.x[3],
                self.cpu.x[4], self.cpu.x[5], self.cpu.x[6], self.cpu.x[7],
            ],
            d: [
                self.cpu.read_d(0), self.cpu.read_d(1), self.cpu.read_d(2), self.cpu.read_d(3),
                self.cpu.read_d(4), self.cpu.read_d(5), self.cpu.read_d(6), self.cpu.read_d(7),
            ],
            x8: self.cpu.x[8],
            host_x8: self.cpu.x[8],
            guest_sp: aligned_sp,
        };
        let name = self.ffi.name_of(target);
        let result = self.ffi.call_or_stub(target, &args);
        match result {
            crate::ffi::FfiResult::Return(val) => {
                log::trace!("FFI {target:#x} ({}) returned {val:#x}", name.unwrap_or("?"));
                self.cpu.x[0] = val;
                self.cpu.pc = self.cpu.x[30];
                StepResult::Continue
            }
            crate::ffi::FfiResult::ReturnFull { x0, x1, d0, d1 } => {
                self.cpu.x[0] = x0;
                self.cpu.x[1] = x1;
                self.cpu.write_d(0, d0);
                self.cpu.write_d(1, d1);
                self.cpu.pc = self.cpu.x[30];
                StepResult::Continue
            }
            crate::ffi::FfiResult::Exit(code) => {
                StepResult::Exit(ExitReason::Exit { code: code as i64 })
            }
            crate::ffi::FfiResult::TailCall(addr) => {
                self.cpu.pc = addr;
                StepResult::Continue
            }
            crate::ffi::FfiResult::TailCallWithX0(addr, x0) => {
                self.cpu.x[0] = x0;
                self.cpu.pc = addr;
                StepResult::Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    // ── B (unconditional) ──────────────────────────────────────

    #[test]
    fn test_b_forward_skip() {
        // pos 0: B +8 → jump to pos 2
        // pos 1: MOVZ X0, #99 (skipped)
        // pos 2: MOVZ X0, #42
        let mut t = test_interp(&[
            enc_b(8),
            enc_movz(true, 0, 99, 0),
            enc_movz(true, 0, 42, 0),
        ]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    #[test]
    fn test_bcond_ne_backward_loop() {
        // 2-iteration loop: increment x0 until it equals 2
        // pos 0: MOVZ X0, #0
        // pos 1: ADD X0, X0, #1
        // pos 2: SUBS XZR, X0, #2  (CMP X0, #2)
        // pos 3: B.NE -8           (back to pos 1)
        let mut t = test_interp(&[
            enc_movz(true, 0, 0, 0),
            enc_add_imm(true, 0, 0, 1, false),
            enc_subs_imm(true, 31, 0, 2, false),
            enc_bcond(1, -8), // NE = cond 1
        ]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 2);
    }

    // ── B.cond ─────────────────────────────────────────────────

    #[test]
    fn test_bcond_eq_taken() {
        // CMP X1, #0 with x1=0 → Z=1, B.EQ taken → skip MOVZ
        // pos 0: SUBS XZR, X1, #0
        // pos 1: B.EQ +8 → jump to pos 3 (implicit RET)
        // pos 2: MOVZ X0, #99
        let mut t = test_interp(&[
            enc_subs_imm(true, 31, 1, 0, false),
            enc_bcond(0, 8), // EQ = cond 0
            enc_movz(true, 0, 99, 0),
        ]);
        t.interp.cpu.x[0] = 42;
        t.interp.cpu.x[1] = 0;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42); // MOVZ skipped
    }

    #[test]
    fn test_bcond_eq_not_taken() {
        // CMP X1, #0 with x1=5 → Z=0, B.EQ not taken → MOVZ executes
        let mut t = test_interp(&[
            enc_subs_imm(true, 31, 1, 0, false),
            enc_bcond(0, 8),
            enc_movz(true, 0, 99, 0),
        ]);
        t.interp.cpu.x[0] = 42;
        t.interp.cpu.x[1] = 5;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 99); // MOVZ executed
    }

    // ── CBZ ────────────────────────────────────────────────────

    #[test]
    fn test_cbz_taken() {
        // x0=0 → CBZ taken, skip MOVZ
        let mut t = test_interp(&[
            enc_cbz(true, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 0;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 42); // MOVZ skipped
    }

    #[test]
    fn test_cbz_not_taken() {
        // x0=1 → CBZ not taken, MOVZ executes
        let mut t = test_interp(&[
            enc_cbz(true, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 1;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 99); // MOVZ executed
    }

    #[test]
    fn test_cbz_32bit_upper_bits_ignored() {
        // 32-bit CBZ: upper 32 bits ignored. x0=0x1_0000_0000 → W0=0 → taken
        let mut t = test_interp(&[
            enc_cbz(false, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 0x1_0000_0000;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 42); // CBZ taken (lower 32 = 0)
    }

    // ── CBNZ ───────────────────────────────────────────────────

    #[test]
    fn test_cbnz_taken() {
        // x0=5 → CBNZ taken, skip MOVZ
        let mut t = test_interp(&[
            enc_cbnz(true, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 5;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 42); // MOVZ skipped
    }

    #[test]
    fn test_cbnz_not_taken() {
        // x0=0 → CBNZ not taken, MOVZ executes
        let mut t = test_interp(&[
            enc_cbnz(true, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 0;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 99); // MOVZ executed
    }

    // ── BL ─────────────────────────────────────────────────────

    #[test]
    fn test_bl_sets_lr() {
        // Verify BL sets LR = pc+4 and jumps to target.
        // pos 0: MOV X2, X30      — save sentinel LR
        // pos 1: BL +16           — x30=code_base+8, jump to pos 5
        // pos 2: MOV X1, X30      — (returned here) save BL's LR
        // pos 3: MOV X30, X2      — restore sentinel
        // pos 4: RET X30           — clean exit
        // pos 5: MOVZ X0, #42     — callee body
        // pos 6: RET X30           — return to caller (code_base+8)
        let mut t = test_interp(&[
            enc_mov_reg(true, 2, 30),
            enc_bl(16),
            enc_mov_reg(true, 1, 30),
            enc_mov_reg(true, 30, 2),
            enc_ret(30),
            enc_movz(true, 0, 42, 0),
            enc_ret(30),
        ]);
        let code_base = t.code_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
        // BL at pos 1 (pc=code_base+4) sets LR = code_base+8
        assert_eq!(cpu.x[1], code_base + 8);
    }

    // ── RET ────────────────────────────────────────────────────

    #[test]
    fn test_ret_clean() {
        // RET with sentinel LR exits cleanly
        let mut t = test_interp(&[
            enc_movz(true, 0, 42, 0),
            enc_ret(30),
        ]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    // ── TBZ ────────────────────────────────────────────────────

    #[test]
    fn test_tbz_taken() {
        // x0=2 (0b10) → bit 0 is 0 → TBZ taken, skip MOVZ
        let mut t = test_interp(&[
            enc_tbz(0, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 2;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 42); // MOVZ skipped
    }

    #[test]
    fn test_tbz_not_taken() {
        // x0=3 (0b11) → bit 0 is 1 → TBZ not taken, MOVZ executes
        let mut t = test_interp(&[
            enc_tbz(0, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 3;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 99); // MOVZ executed
    }

    // ── TBNZ ───────────────────────────────────────────────────

    #[test]
    fn test_tbnz_taken() {
        // x0=1 → bit 0 is 1 → TBNZ taken, skip MOVZ
        let mut t = test_interp(&[
            enc_tbnz(0, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 1;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 42); // MOVZ skipped
    }

    #[test]
    fn test_tbnz_not_taken() {
        // x0=2 (0b10) → bit 0 is 0 → TBNZ not taken, MOVZ executes
        let mut t = test_interp(&[
            enc_tbnz(0, 0, 8),
            enc_movz(true, 1, 99, 0),
        ]);
        t.interp.cpu.x[0] = 2;
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 99); // MOVZ executed
    }

    // ── PAC (Pointer Authentication) ───────────────────────────

    /// Encode a PAC instruction: base 0xDAC10000 | opcode<<10 | rn<<5 | rd
    fn enc_pac_insn(opcode: u32, rd: u32, rn: u32) -> u32 {
        0xDAC10000 | (opcode << 10) | (rn << 5) | rd
    }

    #[test]
    fn test_pacia_sign_and_autia_verify() {
        // PACIA X0, X1 → sign pointer in x0 with context x1
        // AUTIA X0, X1 → verify+strip → should restore original pointer
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 1), // PACIA X0, X1
            enc_pac_insn(0b001000, 0, 1), // AUTIA X0, X1
        ]);
        t.interp.cpu.x[0] = 0x0000_1234_5678_9ABC; // pointer to sign
        t.interp.cpu.x[1] = 0x42;                    // context
        let cpu = t.run_ok();
        // After sign+verify, original pointer should be restored
        assert_eq!(cpu.x[0], 0x0000_1234_5678_9ABC);
    }

    #[test]
    fn test_pacia_inserts_pac_bits() {
        // PACIA X0, X1 → should modify upper bits of pointer
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 1), // PACIA X0, X1
        ]);
        let original = 0x0000_1234_5678_9ABC_u64;
        t.interp.cpu.x[0] = original;
        t.interp.cpu.x[1] = 0x42;
        let cpu = t.run_ok();
        // Lower 48 bits should be preserved
        assert_eq!(cpu.x[0] & ADDR_MASK, original & ADDR_MASK);
        // PAC bits should be non-zero (signed pointer differs from original)
        assert_ne!(cpu.x[0], original);
    }

    #[test]
    fn test_autia_wrong_context_poisons() {
        // PACIA X0, X1 (context=0x42) then AUTIA X0, X1 (context=0x99) → mismatch
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 1), // PACIA X0, X1
            enc_movz(true, 1, 0x99, 0),   // change context
            enc_pac_insn(0b001000, 0, 1), // AUTIA X0, X1 — wrong context!
        ]);
        t.interp.cpu.x[0] = 0x0000_1234_5678_9ABC;
        t.interp.cpu.x[1] = 0x42;
        let cpu = t.run_ok();
        // Should have error bit set (bit 54)
        assert_ne!(cpu.x[0] & (1 << 54), 0, "error bit should be set on auth failure");
    }

    #[test]
    fn test_xpaci_strips_pac_bits() {
        // PACIA X0, X1 → signed. XPACI X0 → strips PAC bits
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 1),  // PACIA X0, X1
            enc_pac_insn(0b100000, 0, 31), // XPACI X0
        ]);
        let original = 0x0000_ABCD_EF01_2345_u64;
        t.interp.cpu.x[0] = original;
        t.interp.cpu.x[1] = 0xBEEF;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], original & ADDR_MASK);
    }

    #[test]
    fn test_xpacd_strips_pac_bits() {
        // PACDA X0, X1 → signed. XPACD X0 → strips PAC bits
        let mut t = test_interp(&[
            enc_pac_insn(0b000100, 0, 1),  // PACDA X0, X1
            enc_pac_insn(0b100010, 0, 31), // XPACD X0
        ]);
        let original = 0x0000_FEED_FACE_CAFE_u64;
        t.interp.cpu.x[0] = original;
        t.interp.cpu.x[1] = 0xCAFE;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], original & ADDR_MASK);
    }

    #[test]
    fn test_pacib_autib_roundtrip() {
        // PACIB + AUTIB with same context → clean roundtrip
        let mut t = test_interp(&[
            enc_pac_insn(0b000010, 0, 1), // PACIB X0, X1
            enc_pac_insn(0b001010, 0, 1), // AUTIB X0, X1
        ]);
        t.interp.cpu.x[0] = 0x0000_DEAD_BEEF_0000;
        t.interp.cpu.x[1] = 0x100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0000_DEAD_BEEF_0000);
    }

    #[test]
    fn test_pacda_autda_roundtrip() {
        let mut t = test_interp(&[
            enc_pac_insn(0b000100, 0, 1), // PACDA X0, X1
            enc_pac_insn(0b001100, 0, 1), // AUTDA X0, X1
        ]);
        t.interp.cpu.x[0] = 0x0000_CAFE_BABE_0000;
        t.interp.cpu.x[1] = 0x200;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0000_CAFE_BABE_0000);
    }

    #[test]
    fn test_pacdb_autdb_roundtrip() {
        let mut t = test_interp(&[
            enc_pac_insn(0b000110, 0, 1), // PACDB X0, X1
            enc_pac_insn(0b001110, 0, 1), // AUTDB X0, X1
        ]);
        t.interp.cpu.x[0] = 0x0000_F00D_BEAD_0000;
        t.interp.cpu.x[1] = 0x300;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0000_F00D_BEAD_0000);
    }

    #[test]
    fn test_pac_different_keys_different_signatures() {
        // PACIA and PACIB on the same pointer+context should produce different PACs
        // because they use different keys.
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 2), // PACIA X0, X2
            enc_pac_insn(0b000010, 1, 2), // PACIB X1, X2
        ]);
        let ptr = 0x0000_1111_2222_3333_u64;
        t.interp.cpu.x[0] = ptr;
        t.interp.cpu.x[1] = ptr;
        t.interp.cpu.x[2] = 0x42; // same context
        let cpu = t.run_ok();
        // Both should have PAC bits set but they should differ
        let pac_a = cpu.x[0] & PAC_MASK;
        let pac_b = cpu.x[1] & PAC_MASK;
        assert_ne!(pac_a, 0, "PACIA should set PAC bits");
        assert_ne!(pac_b, 0, "PACIB should set PAC bits");
        assert_ne!(pac_a, pac_b, "different keys should produce different PACs");
    }

    #[test]
    fn test_pac_zero_context() {
        // PACIZA (rn=31 → context=0) should still produce a valid PAC
        let mut t = test_interp(&[
            enc_pac_insn(0b000000, 0, 31), // PACIZA X0 (context=XZR=0)
            enc_pac_insn(0b001000, 0, 31), // AUTIZA X0
        ]);
        t.interp.cpu.x[0] = 0x0000_ABAB_CDCD_0000;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0000_ABAB_CDCD_0000);
    }
}
