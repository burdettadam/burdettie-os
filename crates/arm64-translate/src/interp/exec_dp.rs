//! Integer data-processing instruction execution.

use super::{shift_imm12, ExitReason, Interpreter, StepResult};
use crate::decode::{self, Instruction};

impl Interpreter {
    pub(super) fn exec_dp(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── NOP / System ───────────────────────────────────────
            Instruction::Nop => {
                self.cpu.pc = pc + 4;
            }
            Instruction::Svc { imm } => {
                log::debug!("SVC #{imm} at {pc:#x} — treating as exit(x0)");
                return StepResult::Exit(ExitReason::Exit {
                    code: self.cpu.x[0] as i64,
                });
            }

            // ── ADD/SUB immediate ──────────────────────────────────
            Instruction::AddImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift);
                let base = self.cpu.read_reg(rn);
                let result = if sf {
                    base.wrapping_add(op2 as u64)
                } else {
                    (base as u32).wrapping_add(op2) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AddsImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift) as u64;
                let base = self.cpu.read_reg(rn);
                if sf {
                    let (result, carry) = base.overflowing_add(op2);
                    let overflow = ((base ^ !op2) & (base ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let a = base as u32;
                    let b = op2 as u32;
                    let (result, carry) = a.overflowing_add(b);
                    let overflow = ((a ^ !b) & (a ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::SubImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift);
                let base = self.cpu.read_reg(rn);
                let result = if sf {
                    base.wrapping_sub(op2 as u64)
                } else {
                    (base as u32).wrapping_sub(op2) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::SubsImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift) as u64;
                let base = self.cpu.read_reg(rn);
                if sf {
                    let (result, borrow) = base.overflowing_sub(op2);
                    let carry = !borrow;
                    let overflow = ((base ^ op2) & (base ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let a = base as u32;
                    let b = op2 as u32;
                    let (result, borrow) = a.overflowing_sub(b);
                    let carry = !borrow;
                    let overflow = ((a ^ b) & (a ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }

            // ── MOV wide ───────────────────────────────────────────
            Instruction::Movz { sf, rd, imm16, shift } => {
                let val = (imm16 as u64) << shift;
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Movk { sf, rd, imm16, shift } => {
                let mask = 0xFFFFu64 << shift;
                let old = self.cpu.read_reg(rd);
                let val = (old & !mask) | ((imm16 as u64) << shift);
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Movn { sf, rd, imm16, shift } => {
                let val = !((imm16 as u64) << shift);
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }

            // ── PC-relative addressing ─────────────────────────────
            Instruction::Adrp { rd, imm } => {
                // imm is already the full computed address (page(PC) + offset)
                self.cpu.write_reg(rd, imm as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::Adr { rd, imm } => {
                self.cpu.write_reg(rd, imm as u64);
                self.cpu.pc = pc + 4;
            }

            // ── Logical immediate ──────────────────────────────────
            Instruction::LogicalImm { sf, opc, rd, rn, imm } => {
                let src = self.cpu.read_reg_zr(rn);
                let result = match opc {
                    0b00 => src & imm,        // AND
                    0b01 => src | imm,        // ORR
                    0b10 => src ^ imm,        // EOR
                    0b11 => {                 // ANDS
                        let r = src & imm;
                        if sf {
                            self.cpu.update_nzcv64(r, false, false);
                        } else {
                            self.cpu.update_nzcv32(r as u32, false, false);
                        }
                        r
                    }
                    _ => unreachable!(),
                };
                let result = if sf { result } else { result & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── ADD/SUB register ───────────────────────────────────
            Instruction::AddReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                let result = if sf { a.wrapping_add(b) } else { (a as u32).wrapping_add(b as u32) as u64 };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AddsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                if sf {
                    let (result, carry) = a.overflowing_add(b);
                    let overflow = ((a ^ !b) & (a ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let (result, carry) = (a as u32).overflowing_add(b as u32);
                    let overflow = (((a as u32) ^ !(b as u32)) & ((a as u32) ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::SubReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                let result = if sf { a.wrapping_sub(b) } else { (a as u32).wrapping_sub(b as u32) as u64 };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::SubsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                if sf {
                    let (result, borrow) = a.overflowing_sub(b);
                    let carry = !borrow;
                    let overflow = ((a ^ b) & (a ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let (result, borrow) = (a as u32).overflowing_sub(b as u32);
                    let carry = !borrow;
                    let overflow = (((a as u32) ^ (b as u32)) & ((a as u32) ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }

            // ── Logical register ───────────────────────────────────
            Instruction::LogicalReg { sf, opc, n, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg_zr(rn);
                let mut b = decode::apply_shift(self.cpu.read_reg_zr(rm), shift_type, shift_amount, sf);
                if n { b = !b; } // N bit inverts Rm
                let result = match opc {
                    0b00 => a & b,  // AND / BIC
                    0b01 => a | b,  // ORR / ORN
                    0b10 => a ^ b,  // EOR / EON
                    0b11 => {       // ANDS / BICS
                        let r = a & b;
                        if sf {
                            self.cpu.update_nzcv64(r, false, false);
                        } else {
                            self.cpu.update_nzcv32(r as u32, false, false);
                        }
                        r
                    }
                    _ => unreachable!(),
                };
                let result = if sf { result } else { result & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Multiply-add / Multiply-sub ────────────────────────
            Instruction::Madd { sf, rd, rn, rm, ra } => {
                let a = self.cpu.read_reg_zr(ra);
                let n = self.cpu.read_reg(rn);
                let m = self.cpu.read_reg(rm);
                let result = if sf {
                    a.wrapping_add(n.wrapping_mul(m))
                } else {
                    (a as u32).wrapping_add((n as u32).wrapping_mul(m as u32)) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Msub { sf, rd, rn, rm, ra } => {
                let a = self.cpu.read_reg_zr(ra);
                let n = self.cpu.read_reg(rn);
                let m = self.cpu.read_reg(rm);
                let result = if sf {
                    a.wrapping_sub(n.wrapping_mul(m))
                } else {
                    (a as u32).wrapping_sub((n as u32).wrapping_mul(m as u32)) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Division ───────────────────────────────────────────
            Instruction::Div { sf, rd, rn, rm, is_signed } => {
                let a = self.cpu.read_reg(rn);
                let b = self.cpu.read_reg(rm);
                let result = if sf {
                    if b == 0 {
                        0
                    } else if is_signed {
                        ((a as i64).wrapping_div(b as i64)) as u64
                    } else {
                        a / b
                    }
                } else {
                    let a32 = a as u32;
                    let b32 = b as u32;
                    if b32 == 0 {
                        0
                    } else if is_signed {
                        ((a32 as i32).wrapping_div(b32 as i32)) as u32 as u64
                    } else {
                        (a32 / b32) as u64
                    }
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            // ── Delegate conditional/shift/bitfield variants ──────────────────────
            insn => return self.exec_dp_extended(insn, pc),
        }
        StepResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    // ── Local encoding helpers (not in test_helpers) ─────────────

    fn enc_logical_imm(sf: bool, opc: u8, n: u32, immr: u32, imms: u32, rn: u8, rd: u8) -> u32 {
        (u32::from(sf) << 31)
            | ((opc as u32 & 0x3) << 29)
            | (0b100100 << 23)
            | ((n & 1) << 22)
            | ((immr & 0x3F) << 16)
            | ((imms & 0x3F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    fn enc_adr(rd: u8, offset: i32) -> u32 {
        let imm = offset as u32;
        let immlo = imm & 0x3;
        let immhi = (imm >> 2) & 0x7FFFF;
        (immlo << 29) | (0b10000 << 24) | (immhi << 5) | (rd as u32 & 0x1F)
    }

    fn enc_adrp(rd: u8, imm21: i32) -> u32 {
        let imm = imm21 as u32;
        let immlo = imm & 0x3;
        let immhi = (imm >> 2) & 0x7FFFF;
        (1u32 << 31) | (immlo << 29) | (0b10000 << 24) | (immhi << 5) | (rd as u32 & 0x1F)
    }

    // ── NOP ──────────────────────────────────────────────────────

    #[test]
    fn test_nop() {
        let mut t = test_interp(&[enc_nop(), enc_movz(true, 0, 42, 0)]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    // ── AddImm ───────────────────────────────────────────────────

    #[test]
    fn test_add_imm_64() {
        let mut t = test_interp(&[enc_add_imm(true, 0, 1, 0x10, false)]);
        t.interp.cpu.x[1] = 100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 116);
    }

    #[test]
    fn test_add_imm_32() {
        let mut t = test_interp(&[enc_add_imm(false, 0, 1, 5, false)]);
        t.interp.cpu.x[1] = 0x1_0000_000A;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xF); // (0xA + 5) truncated to 32 bits
    }

    #[test]
    fn test_add_imm_lsl12() {
        let mut t = test_interp(&[enc_add_imm(true, 0, 1, 1, true)]);
        t.interp.cpu.x[1] = 0;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x1000); // 1 << 12
    }

    // ── AddsImm (flag-setting) ───────────────────────────────────

    #[test]
    fn test_adds_imm_zero_flag() {
        let mut t = test_interp(&[enc_adds_imm(true, 0, 1, 0, false)]);
        t.interp.cpu.x[1] = 0;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0);
        assert!(cpu.nzcv.z);
        assert!(!cpu.nzcv.n);
    }

    #[test]
    fn test_adds_imm_carry() {
        // X1=MAX, add #1 → wraps to 0, C=1
        let mut t = test_interp(&[enc_adds_imm(true, 0, 1, 1, false)]);
        t.interp.cpu.x[1] = u64::MAX;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
    }

    // ── SubImm ───────────────────────────────────────────────────

    #[test]
    fn test_sub_imm_64() {
        let mut t = test_interp(&[enc_sub_imm(true, 0, 1, 10, false)]);
        t.interp.cpu.x[1] = 100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 90);
    }

    #[test]
    fn test_sub_imm_32() {
        let mut t = test_interp(&[enc_sub_imm(false, 0, 1, 1, false)]);
        t.interp.cpu.x[1] = 0x1_0000_0005;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 4); // (5 - 1) truncated to 32 bits
    }

    // ── SubsImm / CMP ───────────────────────────────────────────

    #[test]
    fn test_subs_imm_cmp_equal() {
        // CMP X1, #42 → SUBS XZR, X1, #42; X1=42 → Z=1, C=1
        let mut t = test_interp(&[enc_subs_imm(true, 31, 1, 42, false)]);
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.n);
    }

    #[test]
    fn test_subs_imm_cmp_less() {
        // CMP X1, #50 with X1=10 → N=1, C=0 (borrow)
        let mut t = test_interp(&[enc_subs_imm(true, 31, 1, 50, false)]);
        t.interp.cpu.x[1] = 10;
        let cpu = t.run_ok();
        assert!(!cpu.nzcv.z);
        assert!(cpu.nzcv.n);
        assert!(!cpu.nzcv.c);
    }

    // ── Movz ─────────────────────────────────────────────────────

    #[test]
    fn test_movz_64() {
        let mut t = test_interp(&[enc_movz(true, 0, 0xBEEF, 0)]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xBEEF);
    }

    #[test]
    fn test_movz_hw_shift() {
        // MOVZ X0, #1, LSL #16
        let mut t = test_interp(&[enc_movz(true, 0, 1, 1)]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 1 << 16);
    }

    // ── Movk ─────────────────────────────────────────────────────

    #[test]
    fn test_movk_keeps_bits() {
        // MOVZ X0, #0xAAAA; MOVK X0, #0xBBBB, LSL #16
        let mut t = test_interp(&[
            enc_movz(true, 0, 0xAAAA, 0),
            enc_movk(true, 0, 0xBBBB, 1),
        ]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xBBBB_AAAA);
    }

    // ── Movn ─────────────────────────────────────────────────────

    #[test]
    fn test_movn_64() {
        // MOVN X0, #0 → NOT(0) = 0xFFFF_FFFF_FFFF_FFFF
        let mut t = test_interp(&[enc_movn(true, 0, 0, 0)]);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], u64::MAX);
    }

    // ── Adrp ─────────────────────────────────────────────────────

    #[test]
    fn test_adrp() {
        // ADRP X0, #0 → X0 = page(PC)
        let mut t = test_interp(&[enc_adrp(0, 0)]);
        let code_base = t.code_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], code_base & !0xFFF);
    }

    // ── Adr ──────────────────────────────────────────────────────

    #[test]
    fn test_adr() {
        // ADR X0, #0 → X0 = PC of this instruction
        let mut t = test_interp(&[enc_adr(0, 0)]);
        let code_base = t.code_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], code_base);
    }

    // ── LogicalImm ───────────────────────────────────────────────

    #[test]
    fn test_and_imm() {
        // AND X0, X1, #0xFF (N=1, immr=0, imms=7)
        let mut t = test_interp(&[enc_logical_imm(true, 0b00, 1, 0, 7, 1, 0)]);
        t.interp.cpu.x[1] = 0x12345678_ABCDEF01;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x01);
    }

    #[test]
    fn test_orr_imm() {
        // ORR X0, X1, #0xFF (N=1, immr=0, imms=7)
        let mut t = test_interp(&[enc_logical_imm(true, 0b01, 1, 0, 7, 1, 0)]);
        t.interp.cpu.x[1] = 0x100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x1FF);
    }

    #[test]
    fn test_eor_imm() {
        // EOR X0, X1, #0xFF (N=1, immr=0, imms=7)
        let mut t = test_interp(&[enc_logical_imm(true, 0b10, 1, 0, 7, 1, 0)]);
        t.interp.cpu.x[1] = 0xFF;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x00);
    }

    #[test]
    fn test_ands_imm_sets_zero_flag() {
        // ANDS X0, X1, #0xFF; X1=0x100 → result=0, Z=1
        let mut t = test_interp(&[enc_logical_imm(true, 0b11, 1, 0, 7, 1, 0)]);
        t.interp.cpu.x[1] = 0x100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0);
        assert!(cpu.nzcv.z);
    }

    // ── AddReg ───────────────────────────────────────────────────

    #[test]
    fn test_add_reg_64() {
        let mut t = test_interp(&[enc_add_reg(true, 0, 1, 2, 0, 0)]);
        t.interp.cpu.x[1] = 30;
        t.interp.cpu.x[2] = 12;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    #[test]
    fn test_add_reg_lsl() {
        // ADD X0, X1, X2, LSL #2
        let mut t = test_interp(&[enc_add_reg(true, 0, 1, 2, 0, 2)]);
        t.interp.cpu.x[1] = 10;
        t.interp.cpu.x[2] = 5;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 30); // 10 + (5 << 2)
    }

    // ── SubsReg ──────────────────────────────────────────────────

    #[test]
    fn test_subs_reg_equal() {
        let mut t = test_interp(&[enc_subs_reg(true, 0, 1, 2, 0, 0)]);
        t.interp.cpu.x[1] = 100;
        t.interp.cpu.x[2] = 100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
    }

    // ── LogicalReg (MOV alias) ───────────────────────────────────

    #[test]
    fn test_mov_reg() {
        // MOV X0, X1 → ORR X0, XZR, X1
        let mut t = test_interp(&[enc_mov_reg(true, 0, 1)]);
        t.interp.cpu.x[1] = 0xDEAD_BEEF;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xDEAD_BEEF);
    }

    // ── Madd ─────────────────────────────────────────────────────

    #[test]
    fn test_madd_mul() {
        // MUL X0, X1, X2 → MADD X0, X1, X2, XZR
        let mut t = test_interp(&[enc_madd(true, 0, 1, 2, 31)]);
        t.interp.cpu.x[1] = 7;
        t.interp.cpu.x[2] = 6;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    #[test]
    fn test_madd_accumulate() {
        // MADD X0, X1, X2, X3 → X3 + X1*X2
        let mut t = test_interp(&[enc_madd(true, 0, 1, 2, 3)]);
        t.interp.cpu.x[1] = 5;
        t.interp.cpu.x[2] = 10;
        t.interp.cpu.x[3] = 100;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 150);
    }

    // ── Div ──────────────────────────────────────────────────────

    #[test]
    fn test_udiv() {
        let mut t = test_interp(&[enc_udiv(true, 0, 1, 2)]);
        t.interp.cpu.x[1] = 100;
        t.interp.cpu.x[2] = 7;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 14);
    }

    #[test]
    fn test_sdiv() {
        let mut t = test_interp(&[enc_sdiv(true, 0, 1, 2)]);
        t.interp.cpu.x[1] = (-100i64) as u64;
        t.interp.cpu.x[2] = 10;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0] as i64, -10);
    }

    #[test]
    fn test_udiv_by_zero() {
        let mut t = test_interp(&[enc_udiv(true, 0, 1, 2)]);
        t.interp.cpu.x[0] = 999;
        t.interp.cpu.x[1] = 42;
        t.interp.cpu.x[2] = 0;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0);
    }
}

