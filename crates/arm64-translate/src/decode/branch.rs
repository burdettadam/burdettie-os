//! Branch instruction decoding.

use super::util::sign_extend;
use super::Instruction;
use crate::cpu::Condition;

pub(super) fn decode_branch(raw: u32, pc: u64) -> Instruction {
    // Unconditional branch (immediate): 0 00101 imm26
    if (raw >> 26) & 0x3F == 0b000101 {
        let imm26 = (raw & 0x03FFFFFF) as u64;
        let offset = sign_extend(imm26, 26) << 2;
        return Instruction::B {
            offset: pc as i64 + offset,
        };
    }
    // BL: 1 00101 imm26
    if (raw >> 26) & 0x3F == 0b100101 {
        let imm26 = (raw & 0x03FFFFFF) as u64;
        let offset = sign_extend(imm26, 26) << 2;
        return Instruction::Bl {
            offset: pc as i64 + offset,
        };
    }

    // Conditional branch: 0101010 0 imm19 0 cond
    if (raw >> 25) & 0x7F == 0b0101010 {
        let cond = Condition::from_bits(raw & 0xF);
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        return Instruction::Bcond {
            cond,
            offset: pc as i64 + offset,
        };
    }

    // CBZ/CBNZ: sf 011010 op imm19 Rt
    if (raw >> 25) & 0x3F == 0b011010 {
        let sf = (raw >> 31) & 1 == 1;
        let op = (raw >> 24) & 1;
        let rt = (raw & 0x1F) as u8;
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        return if op == 0 {
            Instruction::Cbz {
                sf,
                rt,
                offset: pc as i64 + offset,
            }
        } else {
            Instruction::Cbnz {
                sf,
                rt,
                offset: pc as i64 + offset,
            }
        };
    }

    // TBZ/TBNZ: b5 011011 op b40 imm14 Rt
    if (raw >> 25) & 0x3F == 0b011011 {
        let b5 = ((raw >> 31) & 1) as u8;
        let op = (raw >> 24) & 1;
        let b40 = ((raw >> 19) & 0x1F) as u8;
        let bit = (b5 << 5) | b40;
        let rt = (raw & 0x1F) as u8;
        let imm14 = ((raw >> 5) & 0x3FFF) as u64;
        let offset = sign_extend(imm14, 14) << 2;
        return if op == 0 {
            Instruction::Tbz {
                rt,
                bit,
                offset: pc as i64 + offset,
            }
        } else {
            Instruction::Tbnz {
                rt,
                bit,
                offset: pc as i64 + offset,
            }
        };
    }

    // Unconditional branch (register): 1101011 opc 11111 000000 Rn 00000
    if (raw >> 25) & 0x7F == 0b1101011 {
        let opc = (raw >> 21) & 0xF;
        let rn = ((raw >> 5) & 0x1F) as u8;
        return match opc {
            0b0000 => Instruction::Br { rn },
            0b0001 => Instruction::Blr { rn },
            0b0010 => {
                // Check for RETAA/RETAB: bits[12:10] distinguish authenticated returns.
                // RET: bits[12:10]=000, RETAA: bits[12:10]=010, RETAB: bits[12:10]=011
                let auth = (raw >> 10) & 0x7;
                if auth == 0b010 || auth == 0b011 {
                    // RETAA/RETAB — always return via LR (x30)
                    Instruction::Ret { rn: 30 }
                } else {
                    Instruction::Ret { rn }
                }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Exception generation: SVC, HVC, SMC, BRK
    // Encoding: 11010100 opc imm16 op2 LL (bits[31:24] = 0xD4)
    if (raw >> 24) & 0xFF == 0b11010100 {
        let opc = (raw >> 21) & 0x7;
        let ll = raw & 0x3;
        if opc == 0b000 && ll == 0b01 {
            let imm = ((raw >> 5) & 0xFFFF) as u16;
            return Instruction::Svc { imm };
        }
        // BRK #imm16: opc=001, LL=00
        if opc == 0b001 && ll == 0b00 {
            let imm = ((raw >> 5) & 0xFFFF) as u16;
            return Instruction::Brk { imm };
        }
        return Instruction::Unknown { raw };
    }

    // System: NOP, DMB, DSB, ISB, MRS, MSR
    // All system instructions have bits[31:22] = 0b1101010100
    if (raw >> 22) & 0x3FF == 0b1101010100 {
        let l = (raw >> 21) & 1; // L=1: MRS (read from sysreg), L=0: MSR (write to sysreg)

        // Check for system register access (MRS/MSR register form):
        // bit[20] = o0, which is 1 for op0=3 system regs (most common)
        // or 0 for op0=2. We handle both.
        // Bit[19] is always 1 in the register access form.
        // Discard: bit[19]=0 means it's an MSR immediate (DAIF set/clear etc.)
        if (raw >> 19) & 1 == 1 {
            let rt = (raw & 0x1F) as u8;
            // sysreg encoding is bits[20:5] (16 bits)
            let sysreg = ((raw >> 5) & 0xFFFF) as u16;
            return if l == 1 {
                Instruction::Mrs { rt, sysreg }
            } else {
                Instruction::Msr { rt, sysreg }
            };
        }

        // NOP and other hints (D503201F = NOP, etc.)
        // CLREX = 0xD503305F — in the barrier group, bit[19]=0
        if raw & 0xFFFFF0FF == 0xD503305F {
            return Instruction::Clrex;
        }
        return Instruction::Nop;
    }
    // NOP encoding: D503201F
    if raw == 0xD503201F {
        return Instruction::Nop;
    }
    // DSB, DMB, ISB — treat as NOP
    if (raw & 0xFFFFF0FF) == 0xD503309F || (raw & 0xFFFFF0FF) == 0xD50330BF || raw == 0xD50330DF {
        return Instruction::Nop;
    }

    Instruction::Unknown { raw }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::Condition;
    use crate::decode::{decode, Instruction};
    use crate::test_helpers::*;

    // ── B (unconditional branch) ─────────────────────────────

    #[test]
    fn test_decode_b_positive_offset() {
        let insn = decode(enc_b(8), 0x1000);
        assert!(matches!(insn, Instruction::B { offset } if offset == 0x1008));
    }

    #[test]
    fn test_decode_b_negative_offset() {
        let insn = decode(enc_b(-8), 0x1000);
        assert!(matches!(insn, Instruction::B { offset } if offset == 0x0FF8));
    }

    #[test]
    fn test_decode_b_zero_offset() {
        let insn = decode(enc_b(0), 0x1000);
        assert!(matches!(insn, Instruction::B { offset } if offset == 0x1000));
    }

    #[test]
    fn test_decode_b_max_positive_offset() {
        // Max forward B offset: +0x7FF_FFFC (26-bit signed imm << 2)
        let insn = decode(enc_b(0x7FF_FFFC), 0x1000);
        assert!(matches!(insn, Instruction::B { offset } if offset == 0x1000 + 0x7FF_FFFC));
    }

    #[test]
    fn test_decode_b_max_negative_offset() {
        // Max backward B offset: -0x800_0000
        let insn = decode(enc_b(-0x800_0000), 0x800_0000);
        assert!(matches!(insn, Instruction::B { offset } if offset == 0));
    }

    // ── BL (branch with link) ────────────────────────────────

    #[test]
    fn test_decode_bl_positive_offset() {
        let insn = decode(enc_bl(0x100), 0x1000);
        assert!(matches!(insn, Instruction::Bl { offset } if offset == 0x1100));
    }

    #[test]
    fn test_decode_bl_negative_offset() {
        let insn = decode(enc_bl(-0x100), 0x1000);
        assert!(matches!(insn, Instruction::Bl { offset } if offset == 0x0F00));
    }

    // ── BR (branch to register) ──────────────────────────────

    #[test]
    fn test_decode_br_x0() {
        let insn = decode(enc_br(0), 0x1000);
        assert!(matches!(insn, Instruction::Br { rn: 0 }));
    }

    #[test]
    fn test_decode_br_x15() {
        let insn = decode(enc_br(15), 0x1000);
        assert!(matches!(insn, Instruction::Br { rn: 15 }));
    }

    #[test]
    fn test_decode_br_x30() {
        let insn = decode(enc_br(30), 0x1000);
        assert!(matches!(insn, Instruction::Br { rn: 30 }));
    }

    // ── BLR (branch with link to register) ───────────────────

    #[test]
    fn test_decode_blr_x0() {
        let insn = decode(enc_blr(0), 0x1000);
        assert!(matches!(insn, Instruction::Blr { rn: 0 }));
    }

    #[test]
    fn test_decode_blr_x10() {
        let insn = decode(enc_blr(10), 0x1000);
        assert!(matches!(insn, Instruction::Blr { rn: 10 }));
    }

    // ── RET ──────────────────────────────────────────────────

    #[test]
    fn test_decode_ret_default_x30() {
        let insn = decode(enc_ret(30), 0x1000);
        assert!(matches!(insn, Instruction::Ret { rn: 30 }));
    }

    #[test]
    fn test_decode_ret_x0() {
        let insn = decode(enc_ret(0), 0x1000);
        assert!(matches!(insn, Instruction::Ret { rn: 0 }));
    }

    #[test]
    fn test_decode_ret_x15() {
        let insn = decode(enc_ret(15), 0x1000);
        assert!(matches!(insn, Instruction::Ret { rn: 15 }));
    }

    // ── B.cond (conditional branch) ──────────────────────────

    #[test]
    fn test_decode_bcond_eq() {
        let insn = decode(enc_bcond(0, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Eq, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_ne() {
        let insn = decode(enc_bcond(1, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Ne, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_cs() {
        let insn = decode(enc_bcond(2, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Cs, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_cc() {
        let insn = decode(enc_bcond(3, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Cc, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_mi() {
        let insn = decode(enc_bcond(4, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Mi, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_pl() {
        let insn = decode(enc_bcond(5, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Pl, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_vs() {
        let insn = decode(enc_bcond(6, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Vs, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_vc() {
        let insn = decode(enc_bcond(7, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Vc, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_hi() {
        let insn = decode(enc_bcond(8, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Hi, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_ls() {
        let insn = decode(enc_bcond(9, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Ls, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_ge() {
        let insn = decode(enc_bcond(10, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Ge, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_lt() {
        let insn = decode(enc_bcond(11, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Lt, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_gt() {
        let insn = decode(enc_bcond(12, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Gt, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_le() {
        let insn = decode(enc_bcond(13, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Le, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_al() {
        let insn = decode(enc_bcond(14, 0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Al, offset } if offset == 0x1020)
        );
    }

    #[test]
    fn test_decode_bcond_negative_offset() {
        let insn = decode(enc_bcond(0, -0x20), 0x1000);
        assert!(
            matches!(insn, Instruction::Bcond { cond: Condition::Eq, offset } if offset == 0x0FE0)
        );
    }

    // ── CBZ (compare and branch if zero) ─────────────────────

    #[test]
    fn test_decode_cbz_64() {
        let insn = decode(enc_cbz(true, 5, 0x40), 0x1000);
        assert!(matches!(insn, Instruction::Cbz { sf: true, rt: 5, offset } if offset == 0x1040));
    }

    #[test]
    fn test_decode_cbz_32() {
        let insn = decode(enc_cbz(false, 3, 0x40), 0x1000);
        assert!(matches!(insn, Instruction::Cbz { sf: false, rt: 3, offset } if offset == 0x1040));
    }

    #[test]
    fn test_decode_cbz_negative_offset() {
        let insn = decode(enc_cbz(true, 0, -0x40), 0x1000);
        assert!(matches!(insn, Instruction::Cbz { sf: true, rt: 0, offset } if offset == 0x0FC0));
    }

    // ── CBNZ (compare and branch if not zero) ────────────────

    #[test]
    fn test_decode_cbnz_64() {
        let insn = decode(enc_cbnz(true, 7, 0x80), 0x1000);
        assert!(matches!(insn, Instruction::Cbnz { sf: true, rt: 7, offset } if offset == 0x1080));
    }

    #[test]
    fn test_decode_cbnz_32() {
        let insn = decode(enc_cbnz(false, 2, 0x80), 0x1000);
        assert!(matches!(insn, Instruction::Cbnz { sf: false, rt: 2, offset } if offset == 0x1080));
    }

    // ── TBZ (test bit and branch if zero) ────────────────────

    #[test]
    fn test_decode_tbz_bit0() {
        let insn = decode(enc_tbz(5, 0, 0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbz { rt: 5, bit: 0, offset } if offset == 0x1020));
    }

    #[test]
    fn test_decode_tbz_bit31() {
        let insn = decode(enc_tbz(5, 31, 0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbz { rt: 5, bit: 31, offset } if offset == 0x1020));
    }

    #[test]
    fn test_decode_tbz_bit63() {
        // b5=1, b40=31 → bit=63
        let insn = decode(enc_tbz(5, 63, 0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbz { rt: 5, bit: 63, offset } if offset == 0x1020));
    }

    #[test]
    fn test_decode_tbz_negative_offset() {
        let insn = decode(enc_tbz(0, 0, -0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbz { rt: 0, bit: 0, offset } if offset == 0x0FE0));
    }

    // ── TBNZ (test bit and branch if not zero) ───────────────

    #[test]
    fn test_decode_tbnz_bit0() {
        let insn = decode(enc_tbnz(3, 0, 0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbnz { rt: 3, bit: 0, offset } if offset == 0x1020));
    }

    #[test]
    fn test_decode_tbnz_bit31() {
        let insn = decode(enc_tbnz(3, 31, 0x20), 0x1000);
        assert!(matches!(insn, Instruction::Tbnz { rt: 3, bit: 31, offset } if offset == 0x1020));
    }

    // ── NOP ──────────────────────────────────────────────────

    #[test]
    fn test_decode_nop() {
        let insn = decode(enc_nop(), 0x1000);
        assert!(matches!(insn, Instruction::Nop));
    }

    // ── SVC (supervisor call) ────────────────────────────────

    #[test]
    fn test_decode_svc() {
        let insn = decode(enc_svc(0x42), 0x1000);
        assert!(matches!(insn, Instruction::Svc { imm: 0x42 }));
    }

    // ── PAC instructions (ARMv8.3) ───────────────────────────

    /// Encode a PAC data-processing-1-source instruction.
    /// opcode occupies bits[15:10], rn at bits[9:5], rd at bits[4:0].
    fn enc_pac(opcode: u32, rd: u32, rn: u32) -> u32 {
        0xDAC10000 | (opcode << 10) | (rn << 5) | rd
    }

    #[test]
    fn test_decode_pacia() {
        let insn = decode(enc_pac(0b000000, 3, 5), 0x1000);
        assert!(matches!(insn, Instruction::PacIA { rd: 3, rn: 5 }));
    }

    #[test]
    fn test_decode_pacib() {
        let insn = decode(enc_pac(0b000010, 0, 1), 0x1000);
        assert!(matches!(insn, Instruction::PacIB { rd: 0, rn: 1 }));
    }

    #[test]
    fn test_decode_pacda() {
        let insn = decode(enc_pac(0b000100, 2, 3), 0x1000);
        assert!(matches!(insn, Instruction::PacDA { rd: 2, rn: 3 }));
    }

    #[test]
    fn test_decode_pacdb() {
        let insn = decode(enc_pac(0b000110, 4, 5), 0x1000);
        assert!(matches!(insn, Instruction::PacDB { rd: 4, rn: 5 }));
    }

    #[test]
    fn test_decode_autia() {
        let insn = decode(enc_pac(0b001000, 7, 8), 0x1000);
        assert!(matches!(insn, Instruction::AutIA { rd: 7, rn: 8 }));
    }

    #[test]
    fn test_decode_autib() {
        let insn = decode(enc_pac(0b001010, 1, 2), 0x1000);
        assert!(matches!(insn, Instruction::AutIB { rd: 1, rn: 2 }));
    }

    #[test]
    fn test_decode_autda() {
        let insn = decode(enc_pac(0b001100, 9, 10), 0x1000);
        assert!(matches!(insn, Instruction::AutDA { rd: 9, rn: 10 }));
    }

    #[test]
    fn test_decode_autdb() {
        let insn = decode(enc_pac(0b001110, 11, 12), 0x1000);
        assert!(matches!(insn, Instruction::AutDB { rd: 11, rn: 12 }));
    }

    #[test]
    fn test_decode_xpaci() {
        // XPACI: opcode=0b100000, Rn=0b11111 (ZR/SP)
        let insn = decode(enc_pac(0b100000, 6, 31), 0x1000);
        assert!(matches!(insn, Instruction::Xpaci { rd: 6 }));
    }

    #[test]
    fn test_decode_xpacd() {
        let insn = decode(enc_pac(0b100010, 15, 31), 0x1000);
        assert!(matches!(insn, Instruction::Xpacd { rd: 15 }));
    }

    #[test]
    fn test_decode_paciza() {
        // PACIZA Xd = PACIA Xd, XZR — rn=31 means zero context
        let insn = decode(enc_pac(0b000000, 2, 31), 0x1000);
        assert!(matches!(insn, Instruction::PacIA { rd: 2, rn: 31 }));
    }

    #[test]
    fn test_decode_retaa() {
        // RETAA = 0xD65F0BFF (authenticated return via LR, key A)
        let insn = decode(0xD65F0BFF, 0x1000);
        assert!(matches!(insn, Instruction::Ret { rn: 30 }));
    }

    #[test]
    fn test_decode_retab() {
        // RETAB = 0xD65F0FFF (authenticated return via LR, key B)
        let insn = decode(0xD65F0FFF, 0x1000);
        assert!(matches!(insn, Instruction::Ret { rn: 30 }));
    }
}
