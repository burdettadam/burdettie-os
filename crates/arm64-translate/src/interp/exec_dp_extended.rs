//! Data processing: conditional, shift, bitfield instructions.

use super::{Interpreter, StepResult};
use crate::decode::Instruction;

impl Interpreter {
    pub(super) fn exec_dp_extended(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── Conditional select ─────────────────────────────────
            Instruction::Csel {
                sf,
                rd,
                rn,
                rm,
                cond,
            } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    self.cpu.read_reg_zr(rm)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csinc {
                sf,
                rd,
                rn,
                rm,
                cond,
            } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    self.cpu.read_reg_zr(rm).wrapping_add(1)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csinv {
                sf,
                rd,
                rn,
                rm,
                cond,
            } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    !self.cpu.read_reg_zr(rm)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csneg {
                sf,
                rd,
                rn,
                rm,
                cond,
            } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    self.cpu.read_reg_zr(rm).wrapping_neg()
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }

            // ── Conditional compare ────────────────────────────────
            Instruction::Ccmp {
                sf,
                rn,
                imm5_or_rm,
                nzcv,
                cond,
                is_imm,
            } => {
                if cond.eval(self.cpu.nzcv) {
                    let a = self.cpu.read_reg(rn);
                    let b = if is_imm {
                        imm5_or_rm as u64
                    } else {
                        self.cpu.read_reg(imm5_or_rm)
                    };
                    if sf {
                        let (result, borrow) = a.overflowing_sub(b);
                        let carry = !borrow;
                        let overflow = ((a ^ b) & (a ^ result)) >> 63 != 0;
                        self.cpu.update_nzcv64(result, carry, overflow);
                    } else {
                        let a32 = a as u32;
                        let b32 = b as u32;
                        let (result, borrow) = a32.overflowing_sub(b32);
                        let carry = !borrow;
                        let overflow = ((a32 ^ b32) & (a32 ^ result)) >> 31 != 0;
                        self.cpu.update_nzcv32(result, carry, overflow);
                    }
                } else {
                    self.cpu.nzcv = crate::cpu::Nzcv {
                        n: nzcv & 0b1000 != 0,
                        z: nzcv & 0b0100 != 0,
                        c: nzcv & 0b0010 != 0,
                        v: nzcv & 0b0001 != 0,
                    };
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ccmn {
                sf,
                rn,
                imm5_or_rm,
                nzcv,
                cond,
                is_imm,
            } => {
                if cond.eval(self.cpu.nzcv) {
                    let a = self.cpu.read_reg(rn);
                    let b = if is_imm {
                        imm5_or_rm as u64
                    } else {
                        self.cpu.read_reg(imm5_or_rm)
                    };
                    if sf {
                        let (result, carry) = a.overflowing_add(b);
                        let overflow = ((a ^ !b) & (a ^ result)) >> 63 != 0;
                        self.cpu.update_nzcv64(result, carry, overflow);
                    } else {
                        let a32 = a as u32;
                        let b32 = b as u32;
                        let (result, carry) = a32.overflowing_add(b32);
                        let overflow = ((a32 ^ !b32) & (a32 ^ result)) >> 31 != 0;
                        self.cpu.update_nzcv32(result, carry, overflow);
                    }
                } else {
                    self.cpu.nzcv = crate::cpu::Nzcv {
                        n: nzcv & 0b1000 != 0,
                        z: nzcv & 0b0100 != 0,
                        c: nzcv & 0b0010 != 0,
                        v: nzcv & 0b0001 != 0,
                    };
                }
                self.cpu.pc = pc + 4;
            }

            // ── Variable shift ─────────────────────────────────────
            Instruction::ShiftVar {
                sf,
                rd,
                rn,
                rm,
                shift_type,
            } => {
                let val = self.cpu.read_reg(rn);
                let shift_amount = self.cpu.read_reg(rm);
                let result = if sf {
                    let amt = (shift_amount & 63) as u32;
                    crate::decode::apply_shift(val, shift_type, amt as u8, true)
                } else {
                    let amt = (shift_amount & 31) as u32;
                    let r =
                        crate::decode::apply_shift(val & 0xFFFF_FFFF, shift_type, amt as u8, false);
                    r & 0xFFFF_FFFF
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Bitfield (SBFM/BFM/UBFM) ──────────────────────────
            Instruction::Bitfield {
                sf,
                opc,
                rd,
                rn,
                immr,
                imms,
            } => {
                let src = self.cpu.read_reg(rn);
                let reg_size: u32 = if sf { 64 } else { 32 };
                let r = immr as u32;
                let s = imms as u32;
                let result = match opc {
                    0b00 => {
                        // SBFM: signed bitfield move
                        // Extract bitfield [immr..imms], sign-extend from bit (imms - immr)
                        let bot = if s >= r {
                            // bits = s - r + 1, extracted from source starting at bit r
                            let width = s - r + 1;
                            (src >> r) & ((1u64 << width) - 1)
                        } else {
                            // wrap case: rotate right by r, then mask bottom (s+1) bits
                            let rot = src.rotate_right(r) & ((1u64 << (s + 1)) - 1);
                            rot
                        };
                        // Sign extend from bit position s (the MSB of the extracted field)
                        let sign_bit = if s >= r { s - r } else { s };
                        let shift = 64 - sign_bit - 1;
                        let extended = ((bot as i64) << shift) >> shift;
                        if sf {
                            extended as u64
                        } else {
                            extended as u64 & 0xFFFF_FFFF
                        }
                    }
                    0b01 => {
                        // BFM: bitfield move (insert)
                        let dst = self.cpu.read_reg(rd);
                        let width = if s >= r { s - r + 1 } else { s + 1 };
                        let mask = (1u64 << width) - 1;
                        let bits = if s >= r {
                            (src >> r) & mask
                        } else {
                            src.rotate_right(r) & mask
                        };
                        let insert_pos = if s >= r { 0u32 } else { reg_size - r };
                        let result_mask = mask << insert_pos;
                        (dst & !result_mask) | ((bits << insert_pos) & result_mask)
                    }
                    0b10 => {
                        // UBFM: unsigned bitfield move
                        // Extract bitfield, zero-extend
                        let bot = if s >= r {
                            let width = s - r + 1;
                            (src >> r) & ((1u64 << width) - 1)
                        } else {
                            src.rotate_right(r) & ((1u64 << (s + 1)) - 1)
                        };
                        if sf {
                            bot
                        } else {
                            bot & 0xFFFF_FFFF
                        }
                    }
                    _ => 0, // reserved, won't reach (decoder filters)
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Data processing (misc) ─────────────────────────────
            Instruction::Rev { sf, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let result = if sf {
                    val.swap_bytes()
                } else {
                    (val as u32).swap_bytes() as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Rev16 { sf, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let result = if sf {
                    // Swap bytes within each 16-bit halfword
                    let mut r = 0u64;
                    for i in 0..4 {
                        let hw = (val >> (i * 16)) & 0xFFFF;
                        r |= ((hw >> 8) | ((hw & 0xFF) << 8)) << (i * 16);
                    }
                    r
                } else {
                    let v32 = val as u32;
                    let mut r = 0u32;
                    for i in 0..2 {
                        let hw = (v32 >> (i * 16)) & 0xFFFF;
                        r |= ((hw >> 8) | ((hw & 0xFF) << 8)) << (i * 16);
                    }
                    r as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Rev32 { rd, rn } => {
                let val = self.cpu.read_reg(rn);
                // Swap bytes within each 32-bit word of a 64-bit value
                let lo = (val as u32).swap_bytes() as u64;
                let hi = ((val >> 32) as u32).swap_bytes() as u64;
                self.cpu.write_reg(rd, (hi << 32) | lo);
                self.cpu.pc = pc + 4;
            }
            Instruction::Clz { sf, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let result = if sf {
                    val.leading_zeros() as u64
                } else {
                    (val as u32).leading_zeros() as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Cls { sf, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let result = if sf {
                    // Count leading sign bits = leading zeros of (val ^ (val << 1)), minus nothing
                    // Actually CLS = number of bits following the sign bit that are equal to the sign bit
                    if val == 0 || val == u64::MAX {
                        63u64
                    } else {
                        (val as i64).leading_zeros() as u64 - 1
                    }
                } else {
                    let v = val as i32;
                    if v == 0 || v == -1 {
                        31u64
                    } else {
                        v.leading_zeros() as u64 - 1
                    }
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Rbit { sf, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let result = if sf {
                    val.reverse_bits()
                } else {
                    (val as u32).reverse_bits() as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Extr {
                sf,
                rd,
                rn,
                rm,
                lsb,
            } => {
                if sf {
                    let hi = self.cpu.read_reg(rn);
                    let lo = self.cpu.read_reg(rm);
                    let result = if lsb == 0 {
                        lo
                    } else {
                        (hi << (64 - lsb as u32)) | (lo >> lsb as u32)
                    };
                    self.cpu.write_reg(rd, result);
                } else {
                    let hi = self.cpu.read_reg(rn) as u32;
                    let lo = self.cpu.read_reg(rm) as u32;
                    let result = if lsb == 0 {
                        lo
                    } else {
                        (hi << (32 - lsb as u32)) | (lo >> lsb as u32)
                    };
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Crc32 {
                sf: _,
                rd,
                rn,
                rm,
                sz,
                c,
            } => {
                // CRC32 using the standard polynomial or the CRC32C (Castagnoli) polynomial.
                let acc = self.cpu.read_reg(rn) as u32;
                let val = self.cpu.read_reg(rm);
                let nbytes = 1u32 << sz;
                let result = if c {
                    crc32_castagnoli(acc, val, nbytes)
                } else {
                    crc32_iso(acc, val, nbytes)
                };
                self.cpu.write_reg(rd, result as u64);
                self.cpu.pc = pc + 4;
            }

            _ => unreachable!("exec_dp_extended called with non-DP instruction"),
        }
        StepResult::Continue
    }
}

/// Software CRC32 with ISO 3309 polynomial (0xEDB88320 reflected).
fn crc32_iso(mut crc: u32, val: u64, nbytes: u32) -> u32 {
    crc = !crc;
    for i in 0..nbytes {
        let byte = ((val >> (i * 8)) & 0xFF) as u8;
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Software CRC32C with Castagnoli polynomial (0x82F63B78 reflected).
fn crc32_castagnoli(mut crc: u32, val: u64, nbytes: u32) -> u32 {
    crc = !crc;
    for i in 0..nbytes {
        let byte = ((val >> (i * 8)) & 0xFF) as u8;
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x82F63B78
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::Nzcv;
    use crate::test_helpers::*;

    // ── Encoding helpers not in test_helpers ────────────────────────

    /// CSINV: sf 1 0 11010100 Rm cond 00 Rn Rd
    fn enc_csinv(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
        (u32::from(sf) << 31)
            | (1 << 30)
            | (0b0011010100 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (0b00 << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    /// CSNEG: sf 1 0 11010100 Rm cond 01 Rn Rd
    fn enc_csneg(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
        (u32::from(sf) << 31)
            | (1 << 30)
            | (0b0011010100 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (0b01 << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    /// Variable shift: sf 00 11010110 Rm 0010 op2 Rn Rd
    /// op2: 00=LSLV, 01=LSRV, 10=ASRV, 11=RORV
    fn enc_shift_var(sf: bool, rd: u8, rn: u8, rm: u8, op2: u8) -> u32 {
        (u32::from(sf) << 31)
            | (0b0011010110 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | (0b0010 << 12)
            | ((op2 as u32 & 0x3) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    /// CCMP immediate: sf 1 1 11010010 imm5 cond 1 0 0 Rn 0 nzcv
    fn enc_ccmp_imm(sf: bool, imm5: u8, cond: u8, rn: u8, nzcv: u8) -> u32 {
        (u32::from(sf) << 31)
            | (1 << 30) // op=1 → CCMP
            | (0b111010010 << 21)
            | ((imm5 as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (1 << 11) // immediate form
            | (0b0 << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (nzcv as u32 & 0xF)
    }

    /// CCMN immediate: sf 0 1 11010010 imm5 cond 1 0 0 Rn 0 nzcv
    fn enc_ccmn_imm(sf: bool, imm5: u8, cond: u8, rn: u8, nzcv: u8) -> u32 {
        (u32::from(sf) << 31)
            | (0 << 30) // op=0 → CCMN
            | (0b111010010 << 21)
            | ((imm5 as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (1 << 11)
            | (0b0 << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (nzcv as u32 & 0xF)
    }

    // ════════════════════════════════════════════════════════════════
    // Conditional select
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_exec_csel_cond_true() {
        // CSEL X0, X1, X2, EQ  — with Z flag set (EQ true), picks X1
        let mut t = test_interp(&[enc_csel(true, 0, 1, 2, 0)]); // cond=0 → EQ
        t.interp.cpu.x[1] = 42;
        t.interp.cpu.x[2] = 99;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 42);
    }

    #[test]
    fn test_exec_csel_cond_false() {
        // CSEL X0, X1, X2, EQ  — with Z flag clear (EQ false), picks X2
        let mut t = test_interp(&[enc_csel(true, 0, 1, 2, 0)]);
        t.interp.cpu.x[1] = 42;
        t.interp.cpu.x[2] = 99;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 99);
    }

    #[test]
    fn test_exec_csinc_cond_false() {
        // CSINC X0, X1, X2, EQ  — EQ false → X2 + 1
        let mut t = test_interp(&[enc_csinc(true, 0, 1, 2, 0)]);
        t.interp.cpu.x[1] = 10;
        t.interp.cpu.x[2] = 99;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 100); // 99 + 1
    }

    #[test]
    fn test_exec_csinv_cond_false() {
        // CSINV X0, X1, X2, EQ  — EQ false → NOT X2
        let mut t = test_interp(&[enc_csinv(true, 0, 1, 2, 0)]);
        t.interp.cpu.x[1] = 10;
        t.interp.cpu.x[2] = 0;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], !0u64); // NOT 0 == all ones
    }

    #[test]
    fn test_exec_csneg_cond_false() {
        // CSNEG X0, X1, X2, EQ  — EQ false → NEG X2
        let mut t = test_interp(&[enc_csneg(true, 0, 1, 2, 0)]);
        t.interp.cpu.x[1] = 10;
        t.interp.cpu.x[2] = 5;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: false,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 5u64.wrapping_neg()); // -5 in two's complement
    }

    // ════════════════════════════════════════════════════════════════
    // Conditional compare
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_exec_ccmp_cond_true() {
        // CCMP X1, #5, #0, EQ  — EQ true → performs X1 - 5, sets flags
        // X1 = 5, so 5 - 5 = 0 → Z flag set
        let mut t = test_interp(&[enc_ccmp_imm(true, 5, 0, 1, 0b0000)]);
        t.interp.cpu.x[1] = 5;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        }; // EQ true
        let cpu = t.run_ok();
        assert!(cpu.nzcv.z); // result is zero
        assert!(cpu.nzcv.c); // no borrow → carry set
        assert!(!cpu.nzcv.n);
        assert!(!cpu.nzcv.v);
    }

    #[test]
    fn test_exec_ccmp_cond_false() {
        // CCMP X1, #5, #0b1010, NE  — NE is false (Z=1) → sets nzcv to 0b1010 (N=1,C=1)
        let mut t = test_interp(&[enc_ccmp_imm(true, 5, 1, 1, 0b1010)]);
        t.interp.cpu.x[1] = 100;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        }; // NE false
        let cpu = t.run_ok();
        assert!(cpu.nzcv.n); // bit 3 of 0b1010
        assert!(!cpu.nzcv.z); // bit 2
        assert!(cpu.nzcv.c); // bit 1
        assert!(!cpu.nzcv.v); // bit 0
    }

    #[test]
    fn test_exec_ccmn_cond_true() {
        // CCMN X1, #3, #0, EQ  — EQ true → performs X1 + 3, sets flags
        // X1 = 0xFFFF_FFFF_FFFF_FFFD (-3), so -3 + 3 = 0 → Z=1, C=1 (carry on wrap)
        let mut t = test_interp(&[enc_ccmn_imm(true, 3, 0, 1, 0b0000)]);
        t.interp.cpu.x[1] = 0xFFFF_FFFF_FFFF_FFFDu64;
        t.interp.cpu.nzcv = Nzcv {
            n: false,
            z: true,
            c: false,
            v: false,
        };
        let cpu = t.run_ok();
        assert!(cpu.nzcv.z); // result is zero
        assert!(cpu.nzcv.c); // unsigned overflow (carry)
        assert!(!cpu.nzcv.n);
    }

    // ════════════════════════════════════════════════════════════════
    // Variable shift
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_exec_lslv_64() {
        // LSLV X0, X1, X2  — X1 << (X2 & 63)
        let mut t = test_interp(&[enc_shift_var(true, 0, 1, 2, 0b00)]);
        t.interp.cpu.x[1] = 1;
        t.interp.cpu.x[2] = 4;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 16); // 1 << 4
    }

    #[test]
    fn test_exec_lsrv_64() {
        // LSRV X0, X1, X2  — X1 >> (X2 & 63) logical
        let mut t = test_interp(&[enc_shift_var(true, 0, 1, 2, 0b01)]);
        t.interp.cpu.x[1] = 0x100;
        t.interp.cpu.x[2] = 4;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x10); // 0x100 >> 4
    }

    #[test]
    fn test_exec_asrv_64_negative() {
        // ASRV X0, X1, X2  — arithmetic shift right of negative value
        let mut t = test_interp(&[enc_shift_var(true, 0, 1, 2, 0b10)]);
        t.interp.cpu.x[1] = 0xFFFF_FFFF_FFFF_FF00u64; // -256
        t.interp.cpu.x[2] = 4;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF_FFFF_FFF0u64); // -256 >> 4 = -16
    }

    #[test]
    fn test_exec_rorv_64() {
        // RORV X0, X1, X2  — rotate right
        let mut t = test_interp(&[enc_shift_var(true, 0, 1, 2, 0b11)]);
        t.interp.cpu.x[1] = 1; // bit 0 set
        t.interp.cpu.x[2] = 1; // rotate right by 1
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 1u64 << 63); // bit 0 wraps to bit 63
    }

    // ════════════════════════════════════════════════════════════════
    // Bitfield
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_exec_sbfm_sxtw() {
        // SBFM X0, X1, #0, #31  (SXTW alias: sign-extend 32-bit to 64-bit)
        let mut t = test_interp(&[enc_sbfm(true, 0, 1, 0, 31)]);
        t.interp.cpu.x[1] = 0x0000_0000_7FFF_FFFFu64; // positive 32-bit
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x7FFF_FFFF); // positive stays positive
    }

    #[test]
    fn test_exec_sbfm_sxtb_negative() {
        // SBFM X0, X1, #0, #7  (SXTB alias: sign-extend byte)
        // value 0x80 → bit 7 set → sign extend to -128
        let mut t = test_interp(&[enc_sbfm(true, 0, 1, 0, 7)]);
        t.interp.cpu.x[1] = 0x80;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF_FFFF_FF80u64); // -128 sign-extended
    }

    #[test]
    fn test_exec_ubfm_uxtb() {
        // UBFM X0, X1, #0, #7  (UXTB: zero-extend byte)
        let mut t = test_interp(&[enc_ubfm(true, 0, 1, 0, 7)]);
        t.interp.cpu.x[1] = 0xDEAD_BEEF_CAFE_00FFu64;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFF); // only low byte
    }

    #[test]
    fn test_exec_ubfm_lsr_4() {
        // UBFM X0, X1, #4, #63  (LSR #4: logical shift right by 4)
        let mut t = test_interp(&[enc_ubfm(true, 0, 1, 4, 63)]);
        t.interp.cpu.x[1] = 0xF0;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0F); // 0xF0 >> 4
    }

    #[test]
    fn test_exec_ubfm_lsl_4() {
        // UBFM X0, X1, #60, #59  (LSL #4 for 64-bit: immr=64-4=60, imms=63-4=59)
        let mut t = test_interp(&[enc_ubfm(true, 0, 1, 60, 59)]);
        t.interp.cpu.x[1] = 0x0F;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xF0); // 0x0F << 4
    }
}
