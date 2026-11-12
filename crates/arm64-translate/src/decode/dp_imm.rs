//! Data processing — immediate instruction decoding.

use super::util::{decode_bitmask_imm, sign_extend};
use super::Instruction;

pub(super) fn decode_dp_imm(raw: u32, pc: u64) -> Instruction {
    let op0 = (raw >> 23) & 0x7;

    match op0 {
        // 000 — PC-rel. addressing (ADR, ADRP)
        0b000 | 0b001 => decode_pc_rel(raw, pc),
        // 010 — Add/subtract (immediate)
        0b010 | 0b011 => decode_add_sub_imm(raw),
        // 100 — Logical (immediate)
        0b100 => decode_logical_imm(raw),
        // 101 — Move wide (immediate)
        0b101 => decode_move_wide(raw),
        // 110 — Bitfield (SBFM, BFM, UBFM → SXTW, SXTB, SXTH, UXTB, UXTH, LSL, LSR, ASR, UBFX, SBFX)
        0b110 => decode_bitfield(raw),
        // 111 — Extract
        _ => Instruction::Unknown { raw },
    }
}

fn decode_pc_rel(raw: u32, pc: u64) -> Instruction {
    let sf = (raw >> 31) & 1; // 0 = ADR, 1 = ADRP
    let rd = (raw & 0x1F) as u8;
    let immhi = ((raw >> 5) & 0x7FFFF) as i64;
    let immlo = ((raw >> 29) & 0x3) as i64;
    let imm21 = (immhi << 2) | immlo;

    if sf == 1 {
        // ADRP: sign-extend 21-bit, shift left 12, add to page(PC)
        let imm = sign_extend(imm21 as u64, 21) << 12;
        let page_pc = pc & !0xFFF;
        Instruction::Adrp {
            rd,
            imm: page_pc as i64 + imm,
        }
    } else {
        // ADR: sign-extend 21-bit, add to PC
        let imm = sign_extend(imm21 as u64, 21);
        Instruction::Adr {
            rd,
            imm: pc as i64 + imm,
        }
    }
}

fn decode_add_sub_imm(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let op = (raw >> 30) & 1; // 0=ADD, 1=SUB
    let s = (raw >> 29) & 1; // set flags
    let shift = ((raw >> 22) & 0x3) as u8; // 0=none, 1=LSL#12
    let imm12 = ((raw >> 10) & 0xFFF) as u32;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match (op, s) {
        (0, 0) => Instruction::AddImm {
            sf,
            rd,
            rn,
            imm: imm12,
            shift,
        },
        (0, 1) => Instruction::AddsImm {
            sf,
            rd,
            rn,
            imm: imm12,
            shift,
        },
        (1, 0) => Instruction::SubImm {
            sf,
            rd,
            rn,
            imm: imm12,
            shift,
        },
        (1, _) => Instruction::SubsImm {
            sf,
            rd,
            rn,
            imm: imm12,
            shift,
        },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_logical_imm(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opc = ((raw >> 29) & 0x3) as u8;
    let n = (raw >> 22) & 1;
    let immr = ((raw >> 16) & 0x3F) as u32;
    let imms = ((raw >> 10) & 0x3F) as u32;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    let imm = decode_bitmask_imm(n, imms, immr, sf);
    Instruction::LogicalImm {
        sf,
        opc,
        rd,
        rn,
        imm,
    }
}

fn decode_move_wide(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opc = (raw >> 29) & 0x3;
    let hw = ((raw >> 21) & 0x3) as u8;
    let imm16 = ((raw >> 5) & 0xFFFF) as u16;
    let rd = (raw & 0x1F) as u8;
    let shift = hw * 16;

    match opc {
        0b00 => Instruction::Movn {
            sf,
            rd,
            imm16,
            shift,
        },
        0b10 => Instruction::Movz {
            sf,
            rd,
            imm16,
            shift,
        },
        0b11 => Instruction::Movk {
            sf,
            rd,
            imm16,
            shift,
        },
        _ => Instruction::Unknown { raw },
    }
}

/// Decode bitfield instructions: SBFM, BFM, UBFM.
///
/// Encoding: sf opc 100110 N immr imms Rn Rd
///   opc: 00=SBFM, 01=BFM, 10=UBFM
///
/// Common aliases:
///   SXTW = SBFM Xd, Xn, #0, #31      (opc=00, immr=0, imms=31)
///   SXTB = SBFM Xd, Xn, #0, #7
///   SXTH = SBFM Xd, Xn, #0, #15
///   UXTB = UBFM Wd, Wn, #0, #7       (opc=10, immr=0, imms=7)
///   UXTH = UBFM Wd, Wn, #0, #15
///   LSL  = UBFM Xd, Xn, #(-shift MOD 64), #(63-shift)
///   LSR  = UBFM Xd, Xn, #shift, #63
///   ASR  = SBFM Xd, Xn, #shift, #63
///   UBFX = UBFM Xd, Xn, #lsb, #(lsb+width-1)
///   SBFX = SBFM Xd, Xn, #lsb, #(lsb+width-1)
fn decode_bitfield(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opc = ((raw >> 29) & 0x3) as u8;
    let immr = ((raw >> 16) & 0x3F) as u8;
    let imms = ((raw >> 10) & 0x3F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    if opc == 0b11 {
        return Instruction::Unknown { raw }; // reserved
    }

    Instruction::Bitfield {
        sf,
        opc,
        rd,
        rn,
        immr,
        imms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::{decode, Instruction};
    use crate::test_helpers::*;

    // ── Helper: encode ADR (sf=0) ──────────────────────────────────
    fn enc_adr(rd: u8, imm21: i32) -> u32 {
        let imm = imm21 as u32 & 0x1F_FFFF; // 21-bit mask
        let immlo = imm & 0x3;
        let immhi = (imm >> 2) & 0x7FFFF;
        (0b0 << 31) | (immlo << 29) | (0b10000 << 24) | (immhi << 5) | (rd as u32 & 0x1F)
    }

    // ── Helper: encode ADRP (sf=1) ─────────────────────────────────
    fn enc_adrp(rd: u8, imm21: i32) -> u32 {
        enc_adr(rd, imm21) | (1 << 31)
    }

    // ── Helper: encode Logical immediate ────────────────────────────
    // Format: sf opc 100100 N immr imms Rn Rd
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

    // ── Helper: encode BFM (opc=01, no helper in test_helpers) ──────
    fn enc_bfm(sf: bool, rd: u8, rn: u8, immr: u8, imms: u8) -> u32 {
        let n = u32::from(sf);
        (u32::from(sf) << 31)
            | (0b01 << 29)
            | (0b100110 << 23)
            | (n << 22)
            | ((immr as u32 & 0x3F) << 16)
            | ((imms as u32 & 0x3F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_pc_rel — ADR
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_adr_positive_offset() {
        // ADR X5, #+0x100 (imm21 = 0x100)
        let insn = decode(enc_adr(5, 0x100), 0x2000);
        // Result: pc + offset = 0x2000 + 0x100 = 0x2100
        assert!(matches!(insn, Instruction::Adr { rd: 5, imm: 0x2100 }));
    }

    #[test]
    fn test_decode_adr_negative_offset() {
        // ADR X3, #-4 (imm21 = -4, sign-extended)
        let insn = decode(enc_adr(3, -4), 0x4000);
        // Result: 0x4000 + (-4) = 0x3FFC
        assert!(matches!(insn, Instruction::Adr { rd: 3, imm: 0x3FFC }));
    }

    #[test]
    fn test_decode_adr_zero_offset() {
        let insn = decode(enc_adr(0, 0), 0x1000);
        assert!(matches!(insn, Instruction::Adr { rd: 0, imm: 0x1000 }));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_pc_rel — ADRP
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_adrp_positive_offset() {
        // ADRP X1, #+1 page (imm21 = 1 → offset = 1 << 12 = 0x1000)
        let insn = decode(enc_adrp(1, 1), 0x5ABC);
        // page(PC) = 0x5000, + 0x1000 = 0x6000
        assert!(matches!(insn, Instruction::Adrp { rd: 1, imm: 0x6000 }));
    }

    #[test]
    fn test_decode_adrp_negative_offset() {
        // ADRP X2, #-1 page (imm21 = -1 → offset = -1 << 12 = -0x1000)
        let insn = decode(enc_adrp(2, -1), 0x5ABC);
        // page(PC) = 0x5000, + (-0x1000) = 0x4000
        assert!(matches!(insn, Instruction::Adrp { rd: 2, imm: 0x4000 }));
    }

    #[test]
    fn test_decode_adrp_zero_offset() {
        let insn = decode(enc_adrp(0, 0), 0x8FFF);
        // page(PC) = 0x8000, + 0 = 0x8000
        assert!(matches!(insn, Instruction::Adrp { rd: 0, imm: 0x8000 }));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_add_sub_imm — AddImm
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_add_imm_64() {
        let insn = decode(enc_add_imm(true, 1, 2, 0x10, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: true,
                rd: 1,
                rn: 2,
                imm: 0x10,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_add_imm_32() {
        let insn = decode(enc_add_imm(false, 5, 6, 0xFF, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: false,
                rd: 5,
                rn: 6,
                imm: 0xFF,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_add_imm_64_lsl12() {
        let insn = decode(enc_add_imm(true, 0, 1, 0x100, true), 0);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: true,
                rd: 0,
                rn: 1,
                imm: 0x100,
                shift: 1
            }
        ));
    }

    #[test]
    fn test_decode_add_imm_max_imm12() {
        let insn = decode(enc_add_imm(true, 3, 4, 0xFFF, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: true,
                rd: 3,
                rn: 4,
                imm: 0xFFF,
                shift: 0
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_add_sub_imm — AddsImm
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_adds_imm_64() {
        let insn = decode(enc_adds_imm(true, 3, 4, 0x20, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddsImm {
                sf: true,
                rd: 3,
                rn: 4,
                imm: 0x20,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_adds_imm_32() {
        let insn = decode(enc_adds_imm(false, 7, 8, 1, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddsImm {
                sf: false,
                rd: 7,
                rn: 8,
                imm: 1,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_adds_imm_lsl12() {
        let insn = decode(enc_adds_imm(true, 0, 1, 0x1, true), 0);
        assert!(matches!(
            insn,
            Instruction::AddsImm {
                sf: true,
                rd: 0,
                rn: 1,
                imm: 1,
                shift: 1
            }
        ));
    }

    // CMP is SUBS with rd=31 (XZR)
    #[test]
    fn test_decode_cmp_imm_alias() {
        let insn = decode(enc_subs_imm(true, 31, 5, 0x10, false), 0);
        assert!(matches!(
            insn,
            Instruction::SubsImm {
                sf: true,
                rd: 31,
                rn: 5,
                imm: 0x10,
                shift: 0
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_add_sub_imm — SubImm
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_sub_imm_64() {
        let insn = decode(enc_sub_imm(true, 10, 11, 0x42, false), 0);
        assert!(matches!(
            insn,
            Instruction::SubImm {
                sf: true,
                rd: 10,
                rn: 11,
                imm: 0x42,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_sub_imm_32() {
        let insn = decode(enc_sub_imm(false, 2, 3, 0x80, false), 0);
        assert!(matches!(
            insn,
            Instruction::SubImm {
                sf: false,
                rd: 2,
                rn: 3,
                imm: 0x80,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_sub_imm_64_lsl12() {
        let insn = decode(enc_sub_imm(true, 1, 2, 0x1, true), 0);
        assert!(matches!(
            insn,
            Instruction::SubImm {
                sf: true,
                rd: 1,
                rn: 2,
                imm: 1,
                shift: 1
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_add_sub_imm — SubsImm
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_subs_imm_64() {
        let insn = decode(enc_subs_imm(true, 0, 1, 0x100, false), 0);
        assert!(matches!(
            insn,
            Instruction::SubsImm {
                sf: true,
                rd: 0,
                rn: 1,
                imm: 0x100,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_subs_imm_32() {
        let insn = decode(enc_subs_imm(false, 4, 5, 0x50, false), 0);
        assert!(matches!(
            insn,
            Instruction::SubsImm {
                sf: false,
                rd: 4,
                rn: 5,
                imm: 0x50,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_subs_imm_lsl12() {
        let insn = decode(enc_subs_imm(true, 2, 3, 0xAB, true), 0);
        assert!(matches!(
            insn,
            Instruction::SubsImm {
                sf: true,
                rd: 2,
                rn: 3,
                imm: 0xAB,
                shift: 1
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_add_sub_imm — SP/zero register edge cases
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_add_imm_sp_as_dest() {
        // ADD SP, SP, #0x20 — rd=31, rn=31
        let insn = decode(enc_add_imm(true, 31, 31, 0x20, false), 0);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: true,
                rd: 31,
                rn: 31,
                imm: 0x20,
                shift: 0
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_logical_imm — AND (opc=0)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_and_imm_64() {
        // AND X1, X2, #0xFF (N=1, immr=0, imms=7 → 8 ones = 0xFF replicated)
        let raw = enc_logical_imm(true, 0b00, 1, 0, 7, 2, 1);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 0,
                rd: 1,
                rn: 2,
                imm: 0xFF
            }
        ));
    }

    #[test]
    fn test_decode_and_imm_32() {
        // AND W1, W2, #0xFF (N=0, immr=0, imms=7)
        let raw = enc_logical_imm(false, 0b00, 0, 0, 7, 2, 1);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: false,
                opc: 0,
                rd: 1,
                rn: 2,
                imm: 0xFF
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_logical_imm — ORR (opc=1)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_orr_imm_64() {
        // ORR X5, X6, #0xFFFF (N=1, immr=0, imms=15 → 16 ones)
        let raw = enc_logical_imm(true, 0b01, 1, 0, 15, 6, 5);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 1,
                rd: 5,
                rn: 6,
                imm: 0xFFFF
            }
        ));
    }

    #[test]
    fn test_decode_orr_imm_32() {
        // ORR W3, W4, #0xFFFF (N=0, immr=0, imms=15)
        let raw = enc_logical_imm(false, 0b01, 0, 0, 15, 4, 3);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: false,
                opc: 1,
                rd: 3,
                rn: 4,
                imm: 0xFFFF
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_logical_imm — EOR (opc=2)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_eor_imm_64() {
        // EOR X0, X1, #0xFFFFFFFF (N=1, immr=0, imms=31 → 32 ones)
        let raw = enc_logical_imm(true, 0b10, 1, 0, 31, 1, 0);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 2,
                rd: 0,
                rn: 1,
                imm: 0xFFFF_FFFF
            }
        ));
    }

    #[test]
    fn test_decode_eor_imm_32() {
        // EOR W7, W8, #0xFF (N=0, immr=0, imms=7 → 8 ones)
        let raw = enc_logical_imm(false, 0b10, 0, 0, 7, 8, 7);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: false,
                opc: 2,
                rd: 7,
                rn: 8,
                imm: 0xFF
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_logical_imm — ANDS (opc=3)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_ands_imm_64() {
        // ANDS X2, X3, #0xFF
        let raw = enc_logical_imm(true, 0b11, 1, 0, 7, 3, 2);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 3,
                rd: 2,
                rn: 3,
                imm: 0xFF
            }
        ));
    }

    #[test]
    fn test_decode_ands_imm_32() {
        // ANDS W10, W11, #0xFFFF
        let raw = enc_logical_imm(false, 0b11, 0, 0, 15, 11, 10);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: false,
                opc: 3,
                rd: 10,
                rn: 11,
                imm: 0xFFFF
            }
        ));
    }

    // TST is ANDS with rd=XZR(31)
    #[test]
    fn test_decode_tst_imm_alias() {
        let raw = enc_logical_imm(true, 0b11, 1, 0, 7, 5, 31);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 3,
                rd: 31,
                rn: 5,
                imm: 0xFF
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_logical_imm — rotated pattern
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_and_imm_rotated() {
        // AND X0, X1, with N=1, immr=2, imms=7 → 8 ones rotated right by 2
        // Pattern: 0xFF rotated right by 2 in 64-bit = 0xC000_0000_0000_003F
        let raw = enc_logical_imm(true, 0b00, 1, 2, 7, 1, 0);
        let insn = decode(raw, 0);
        assert!(matches!(
            insn,
            Instruction::LogicalImm {
                sf: true,
                opc: 0,
                rd: 0,
                rn: 1,
                ..
            }
        ));
        // Verify the decoded imm value is the rotated bitmask
        if let Instruction::LogicalImm { imm, .. } = insn {
            assert_eq!(imm, 0xC000_0000_0000_003F);
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_move_wide — Movz
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_movz_64_hw0() {
        let insn = decode(enc_movz(true, 5, 0x1234, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: true,
                rd: 5,
                imm16: 0x1234,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movz_32_hw0() {
        let insn = decode(enc_movz(false, 8, 42, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: false,
                rd: 8,
                imm16: 42,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movz_64_hw1() {
        let insn = decode(enc_movz(true, 0, 0xABCD, 1), 0);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: true,
                rd: 0,
                imm16: 0xABCD,
                shift: 16
            }
        ));
    }

    #[test]
    fn test_decode_movz_64_hw2() {
        let insn = decode(enc_movz(true, 1, 0xFFFF, 2), 0);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: true,
                rd: 1,
                imm16: 0xFFFF,
                shift: 32
            }
        ));
    }

    #[test]
    fn test_decode_movz_64_hw3() {
        let insn = decode(enc_movz(true, 2, 1, 3), 0);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: true,
                rd: 2,
                imm16: 1,
                shift: 48
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_move_wide — Movk
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_movk_64_hw0() {
        let insn = decode(enc_movk(true, 3, 0xBEEF, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movk {
                sf: true,
                rd: 3,
                imm16: 0xBEEF,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movk_32_hw0() {
        let insn = decode(enc_movk(false, 4, 0x1, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movk {
                sf: false,
                rd: 4,
                imm16: 1,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movk_64_hw2() {
        let insn = decode(enc_movk(true, 7, 0xDEAD, 2), 0);
        assert!(matches!(
            insn,
            Instruction::Movk {
                sf: true,
                rd: 7,
                imm16: 0xDEAD,
                shift: 32
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_move_wide — Movn
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_movn_64_hw0() {
        let insn = decode(enc_movn(true, 0, 0, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movn {
                sf: true,
                rd: 0,
                imm16: 0,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movn_32_hw0() {
        let insn = decode(enc_movn(false, 9, 0xFF, 0), 0);
        assert!(matches!(
            insn,
            Instruction::Movn {
                sf: false,
                rd: 9,
                imm16: 0xFF,
                shift: 0
            }
        ));
    }

    #[test]
    fn test_decode_movn_64_hw1() {
        let insn = decode(enc_movn(true, 6, 0x1234, 1), 0);
        assert!(matches!(
            insn,
            Instruction::Movn {
                sf: true,
                rd: 6,
                imm16: 0x1234,
                shift: 16
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_bitfield — SBFM (opc=0)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_sbfm_64_sxtw() {
        // SXTW = SBFM Xd, Xn, #0, #31
        let insn = decode(enc_sbfm(true, 0, 1, 0, 31), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 0,
                rd: 0,
                rn: 1,
                immr: 0,
                imms: 31
            }
        ));
    }

    #[test]
    fn test_decode_sbfm_64_sxtb() {
        // SXTB = SBFM Xd, Xn, #0, #7
        let insn = decode(enc_sbfm(true, 2, 3, 0, 7), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 0,
                rd: 2,
                rn: 3,
                immr: 0,
                imms: 7
            }
        ));
    }

    #[test]
    fn test_decode_sbfm_64_sxth() {
        // SXTH = SBFM Xd, Xn, #0, #15
        let insn = decode(enc_sbfm(true, 4, 5, 0, 15), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 0,
                rd: 4,
                rn: 5,
                immr: 0,
                imms: 15
            }
        ));
    }

    #[test]
    fn test_decode_sbfm_64_asr() {
        // ASR X0, X1, #4 = SBFM X0, X1, #4, #63
        let insn = decode(enc_sbfm(true, 0, 1, 4, 63), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 0,
                rd: 0,
                rn: 1,
                immr: 4,
                imms: 63
            }
        ));
    }

    #[test]
    fn test_decode_sbfm_32() {
        // SBFM W0, W1, #0, #7 (SXTB 32-bit)
        let insn = decode(enc_sbfm(false, 0, 1, 0, 7), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: false,
                opc: 0,
                rd: 0,
                rn: 1,
                immr: 0,
                imms: 7
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_bitfield — BFM (opc=1)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_bfm_64() {
        let insn = decode(enc_bfm(true, 0, 1, 8, 15), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 1,
                rd: 0,
                rn: 1,
                immr: 8,
                imms: 15
            }
        ));
    }

    #[test]
    fn test_decode_bfm_32() {
        let insn = decode(enc_bfm(false, 5, 6, 0, 7), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: false,
                opc: 1,
                rd: 5,
                rn: 6,
                immr: 0,
                imms: 7
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_bitfield — UBFM (opc=2)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_ubfm_64_lsr() {
        // LSR X0, X1, #4 = UBFM X0, X1, #4, #63
        let insn = decode(enc_ubfm(true, 0, 1, 4, 63), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 2,
                rd: 0,
                rn: 1,
                immr: 4,
                imms: 63
            }
        ));
    }

    #[test]
    fn test_decode_ubfm_64_uxtb() {
        // UXTB = UBFM Xd, Xn, #0, #7
        let insn = decode(enc_ubfm(true, 3, 4, 0, 7), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 2,
                rd: 3,
                rn: 4,
                immr: 0,
                imms: 7
            }
        ));
    }

    #[test]
    fn test_decode_ubfm_32_uxth() {
        // UXTH = UBFM Wd, Wn, #0, #15
        let insn = decode(enc_ubfm(false, 2, 3, 0, 15), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: false,
                opc: 2,
                rd: 2,
                rn: 3,
                immr: 0,
                imms: 15
            }
        ));
    }

    #[test]
    fn test_decode_ubfm_64_lsl() {
        // LSL X0, X1, #10 = UBFM X0, X1, #(-10 MOD 64)=54, #(63-10)=53
        let insn = decode(enc_ubfm(true, 0, 1, 54, 53), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: true,
                opc: 2,
                rd: 0,
                rn: 1,
                immr: 54,
                imms: 53
            }
        ));
    }

    #[test]
    fn test_decode_ubfm_32() {
        // UBFM W7, W8, #2, #9 (UBFX W7, W8, #2, #8)
        let insn = decode(enc_ubfm(false, 7, 8, 2, 9), 0);
        assert!(matches!(
            insn,
            Instruction::Bitfield {
                sf: false,
                opc: 2,
                rd: 7,
                rn: 8,
                immr: 2,
                imms: 9
            }
        ));
    }

    // ════════════════════════════════════════════════════════════════
    //  decode_bitfield — reserved opc=3
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_bitfield_opc3_unknown() {
        // opc=11 is reserved → Unknown
        let raw = (1u32 << 31)   // sf=1
            | (0b11 << 29)       // opc=11
            | (0b100110 << 23)   // bitfield fixed bits
            | (1 << 22)          // N=1
            | (0 << 16)          // immr=0
            | (7 << 10)          // imms=7
            | (1 << 5)           // rn=1
            | 0; // rd=0
        let insn = decode(raw, 0);
        assert!(matches!(insn, Instruction::Unknown { .. }));
    }
}
