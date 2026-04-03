//! Load/store instruction execution.

use super::{ExitReason, Interpreter, StepResult};
use crate::decode::Instruction;
use crate::{mem_read_u64, mem_read_u32, mem_read_u16, mem_read_u8};

impl Interpreter {
    pub(super) fn exec_ls(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── Load immediate (unsigned offset) ───────────────────
            Instruction::LdrImm { sf, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrImm { sf, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.write_u64(addr, val)
                } else {
                    self.mem.write_u32(addr, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Load/Store unscaled ────────────────────────────────
            Instruction::Ldur { sf, rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stur { sf, rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.write_u64(addr, val)
                } else {
                    self.mem.write_u32(addr, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Load/Store pre-index ───────────────────────────────
            Instruction::LdrPre { sf, rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base); // writeback first
                let val = if sf {
                    mem_read_u64!(self, base, pc)
                } else {
                    mem_read_u32!(self, base, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrPre { sf, rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.write_u64(base, val)
                } else {
                    self.mem.write_u32(base, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Load/Store post-index ──────────────────────────────
            Instruction::LdrPost { sf, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let val = if sf {
                    mem_read_u64!(self, base, pc)
                } else {
                    mem_read_u32!(self, base, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrPost { sf, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.write_u64(base, val)
                } else {
                    self.mem.write_u32(base, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }

            // ── Byte/Halfword/Signed pre-index ─────────────────────
            Instruction::LdrbPre { rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let val = mem_read_u8!(self, base, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrbPre { rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                if !self.mem.write_u8(base, self.cpu.read_reg_zr(rt) as u8) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrhPre { rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let val = mem_read_u16!(self, base, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrhPre { rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                if !self.mem.write_u16(base, self.cpu.read_reg_zr(rt) as u16) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrsbPre { sf, rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let byte = mem_read_u8!(self, base, pc);
                let val = if sf { (byte as i8) as i64 as u64 } else { (byte as i8) as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrshPre { sf, rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let hw = mem_read_u16!(self, base, pc);
                let val = if sf { (hw as i16) as i64 as u64 } else { (hw as i16) as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrswPre { rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let word = mem_read_u32!(self, base, pc);
                self.cpu.write_reg_zr(rt, (word as i32) as i64 as u64);
                self.cpu.pc = pc + 4;
            }

            // ── Byte/Halfword/Signed post-index ────────────────────
            Instruction::LdrbPost { rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let val = mem_read_u8!(self, base, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrbPost { rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.mem.write_u8(base, self.cpu.read_reg_zr(rt) as u8) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrhPost { rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let val = mem_read_u16!(self, base, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrhPost { rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.mem.write_u16(base, self.cpu.read_reg_zr(rt) as u16) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrsbPost { sf, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let byte = mem_read_u8!(self, base, pc);
                let val = if sf { (byte as i8) as i64 as u64 } else { (byte as i8) as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrshPost { sf, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let hw = mem_read_u16!(self, base, pc);
                let val = if sf { (hw as i16) as i64 as u64 } else { (hw as i16) as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrswPost { rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let word = mem_read_u32!(self, base, pc);
                self.cpu.write_reg_zr(rt, (word as i32) as i64 as u64);
                self.cpu.write_reg(rn, (base as i64).wrapping_add(imm as i64) as u64);
                self.cpu.pc = pc + 4;
            }

            // ── PRFM (prefetch memory) — NOP ─────────────────────
            Instruction::Prfm => {
                self.cpu.pc = pc + 4;
            }

            // ── Delegate extended load/store variants ─────────────────────────────
            insn => return self.exec_ls_extended(insn, pc),
        }
        StepResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    // 1. STR/LDR 64-bit round-trip
    #[test]
    fn test_exec_str_ldr_64() {
        let mut t = test_interp(&[
            enc_str_imm64(0, 2, 0),
            enc_ldr_imm64(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xDEAD_BEEF_CAFE_BABE;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xDEAD_BEEF_CAFE_BABE);
    }

    // 2. STR/LDR 32-bit round-trip (zero-extends on load)
    #[test]
    fn test_exec_str_ldr_32() {
        let mut t = test_interp(&[
            enc_str_imm32(0, 2, 0),
            enc_ldr_imm32(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xFFFF_FFFF_ABCD_1234;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        // 32-bit store truncates to low 32 bits, 32-bit load zero-extends
        assert_eq!(cpu.x[1], 0xABCD_1234);
    }

    // 3. STRB/LDRB byte store/load
    #[test]
    fn test_exec_strb_ldrb() {
        let mut t = test_interp(&[
            enc_strb_imm(0, 2, 0),
            enc_ldrb_imm(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xFF42;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0x42); // only low byte stored/loaded
    }

    // 4. STRH/LDRH halfword store/load
    #[test]
    fn test_exec_strh_ldrh() {
        let mut t = test_interp(&[
            enc_strh_imm(0, 2, 0),
            enc_ldrh_imm(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xDEAD_BEEF;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xBEEF); // only low halfword
    }

    // 5. STP/LDP 64-bit pair store/load
    #[test]
    fn test_exec_stp_ldp_64() {
        let mut t = test_interp(&[
            enc_stp64(0, 1, 4, 0),
            enc_ldp64(2, 3, 4, 0),
        ]);
        t.interp.cpu.x[0] = 0x1111_2222_3333_4444;
        t.interp.cpu.x[1] = 0xAAAA_BBBB_CCCC_DDDD;
        t.interp.cpu.x[4] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[2], 0x1111_2222_3333_4444);
        assert_eq!(cpu.x[3], 0xAAAA_BBBB_CCCC_DDDD);
    }

    // 6a. STUR 64-bit (unscaled offset)
    #[test]
    fn test_exec_stur_ldur_64() {
        // STUR X0, [X2, #0]:  0xF8000040 with rt=0, rn=2, imm9=0
        // LDUR X1, [X2, #0]:  0xF8400041 with rt=1, rn=2, imm9=0
        let stur = 0xF800_0040u32; // STUR X0, [X2, #0]
        let ldur = 0xF840_0041u32; // LDUR X1, [X2, #0]
        let mut t = test_interp(&[stur, ldur]);
        t.interp.cpu.x[0] = 0xCAFE_BABE_DEAD_BEEF;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xCAFE_BABE_DEAD_BEEF);
    }

    // 6b. LDUR with negative offset
    #[test]
    fn test_exec_ldur_negative_offset() {
        // Store value at data_base, then load via [data_base+16, #-16]
        // LDUR X1, [X2, #-16]: imm9 = -16 = 0x1F0, encoding = 0xF8400000 | (0x1F0 << 12) | (2 << 5) | 1
        let ldur_neg16 = 0xF840_0000u32 | (0x1F0 << 12) | (2 << 5) | 1;
        let mut t = test_interp(&[ldur_neg16]);
        t.write_data_u64(0, 0xBEEF_F00D);
        t.interp.cpu.x[2] = t.data_base + 16; // base + 16 - 16 = data_base
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xBEEF_F00D);
    }

    // 7. LDR with non-zero offset
    #[test]
    fn test_exec_ldr_with_offset() {
        let mut t = test_interp(&[enc_ldr_imm64(0, 2, 16)]);
        t.write_data_u64(16, 0x1234_5678_9ABC_DEF0);
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x1234_5678_9ABC_DEF0);
    }

    // 8. LDR 32-bit zero-extends (not sign-extends)
    #[test]
    fn test_exec_ldr32_zero_extends() {
        let mut t = test_interp(&[enc_ldr_imm32(0, 2, 0)]);
        t.write_data_u32(0, 0xFFFF_FFFF);
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF); // zero-extended, not 0xFFFFFFFF_FFFFFFFF
    }

    // 9. Memory fault on unmapped address
    #[test]
    fn test_exec_ldr_memory_fault() {
        let mut t = test_interp(&[enc_ldr_imm64(0, 2, 0)]);
        t.interp.cpu.x[2] = 0xDEAD_0000; // unmapped address
        let exit = t.run();
        assert!(
            matches!(exit, ExitReason::MemoryFault { is_write: false, .. }),
            "Expected MemoryFault, got: {exit:?}"
        );
    }

    // 10. STR 64-bit then verify via data helper
    #[test]
    fn test_exec_str64_verify_memory() {
        let mut t = test_interp(&[enc_str_imm64(0, 2, 0)]);
        t.interp.cpu.x[0] = 0xFEED_FACE;
        t.interp.cpu.x[2] = t.data_base;
        t.run_ok();
        assert_eq!(t.read_data_u64(0), 0xFEED_FACE);
    }

    // 11. STR 32-bit truncates upper bits
    #[test]
    fn test_exec_str32_truncates() {
        let mut t = test_interp(&[enc_str_imm32(0, 2, 0)]);
        t.interp.cpu.x[0] = 0xAAAA_BBBB_CCCC_DDDD;
        t.interp.cpu.x[2] = t.data_base;
        t.run_ok();
        assert_eq!(t.read_data_u32(0), 0xCCCC_DDDD);
    }

    // 12. STRB only stores low byte
    #[test]
    fn test_exec_strb_stores_low_byte() {
        let mut t = test_interp(&[enc_strb_imm(0, 2, 0)]);
        t.interp.cpu.x[0] = 0xABCD;
        t.interp.cpu.x[2] = t.data_base;
        t.run_ok();
        assert_eq!(t.read_data_u8(0), 0xCD);
    }

    // 13. STRH only stores low halfword
    #[test]
    fn test_exec_strh_stores_low_halfword() {
        let mut t = test_interp(&[enc_strh_imm(0, 2, 0)]);
        t.interp.cpu.x[0] = 0x1234_5678;
        t.interp.cpu.x[2] = t.data_base;
        t.run_ok();
        assert_eq!(t.read_data_u16(0), 0x5678);
    }

    // 14. LDR 64-bit from pre-populated memory
    #[test]
    fn test_exec_ldr64_from_prewritten() {
        let mut t = test_interp(&[enc_ldr_imm64(0, 2, 0)]);
        t.write_data_u64(0, 0x0123_4567_89AB_CDEF);
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x0123_4567_89AB_CDEF);
    }

    // 15. LDRB zero-extends byte
    #[test]
    fn test_exec_ldrb_zero_extends() {
        let mut t = test_interp(&[enc_ldrb_imm(0, 2, 0)]);
        t.write_data_u8(0, 0xFF);
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFF); // zero-extended, not sign-extended
    }

    // 16. LDRH zero-extends halfword
    #[test]
    fn test_exec_ldrh_zero_extends() {
        let mut t = test_interp(&[enc_ldrh_imm(0, 2, 0)]);
        t.write_data_u16(0, 0xFFFF);
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF);
    }

    // 17. STP/LDP 64-bit with offset
    #[test]
    fn test_exec_stp_ldp_64_with_offset() {
        let mut t = test_interp(&[
            enc_stp64(0, 1, 4, 16),
            enc_ldp64(2, 3, 4, 16),
        ]);
        t.interp.cpu.x[0] = 0xAAAA;
        t.interp.cpu.x[1] = 0xBBBB;
        t.interp.cpu.x[4] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[2], 0xAAAA);
        assert_eq!(cpu.x[3], 0xBBBB);
    }

    // 18. Memory fault on store to unmapped
    #[test]
    fn test_exec_str_memory_fault() {
        let mut t = test_interp(&[enc_str_imm64(0, 2, 0)]);
        t.interp.cpu.x[0] = 42;
        t.interp.cpu.x[2] = 0xDEAD_0000;
        let exit = t.run();
        assert!(
            matches!(exit, ExitReason::MemoryFault { is_write: true, .. }),
            "Expected write MemoryFault, got: {exit:?}"
        );
    }

    // 19. Multiple STR/LDR at different offsets
    #[test]
    fn test_exec_str_ldr_multiple_offsets() {
        let mut t = test_interp(&[
            enc_str_imm64(0, 4, 0),
            enc_str_imm64(1, 4, 8),
            enc_ldr_imm64(2, 4, 0),
            enc_ldr_imm64(3, 4, 8),
        ]);
        t.interp.cpu.x[0] = 0x1111;
        t.interp.cpu.x[1] = 0x2222;
        t.interp.cpu.x[4] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[2], 0x1111);
        assert_eq!(cpu.x[3], 0x2222);
    }

    // 20. STRB at byte offset, LDRB at same offset
    #[test]
    fn test_exec_strb_ldrb_with_offset() {
        let mut t = test_interp(&[
            enc_strb_imm(0, 2, 5),
            enc_ldrb_imm(1, 2, 5),
        ]);
        t.interp.cpu.x[0] = 0x7F;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0x7F);
    }
}

