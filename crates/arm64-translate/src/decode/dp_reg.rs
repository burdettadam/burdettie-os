//! Data processing — register instruction decoding.

use crate::cpu::Condition;
use super::Instruction;

pub(super) fn decode_dp_reg(raw: u32) -> Instruction {
    // Primary dispatch on bits[28:21] — uniquely identifies most DP-reg classes.
    let op_class = (raw >> 21) & 0xFF; // bits[28:21]

    match op_class {
        // Data processing (1/2 source): sf X S 11010110 ...
        // bit[30]=1,bit[29]=0 → 1-source (REV, CLZ, CLS, RBIT)
        // bit[30]=0 → 2-source (UDIV, SDIV, LSLV, etc.)
        // bit[30]=0,bit[29]=1 → 2-source with S=1 (CRC32)
        0b11010110 => {
            if (raw >> 30) & 1 == 1 && (raw >> 29) & 1 == 0 {
                return decode_dp_1source(raw);
            }
            return decode_dp_2source(raw);
        }
        // Conditional select: sf op S 11010100 Rm cond op2 Rn Rd
        0b11010100 => return decode_cond_select(raw),
        // Conditional compare: sf op 1 11010010 Rm/imm5 cond 0 o2 o3 Rn nzcv
        0b11010010 => return decode_ccmp(raw),
        _ => {}
    }

    // Secondary dispatch on bits[28:24] for wider classes.
    let top5 = (raw >> 24) & 0x1F;
    match top5 {
        // Data processing (3 source): sf op54 11011 op31 Rm o0 Ra Rn Rd
        0b11011 => decode_dp_3source(raw),
        // Add/subtract (shifted or extended register): sf op S 01011 ...
        0b01011 => {
            if (raw >> 21) & 1 == 1 {
                decode_add_sub_ext_reg(raw)
            } else {
                decode_add_sub_reg(raw)
            }
        }
        // Logical (shifted register): sf opc 01010 shift N Rm imm6 Rn Rd
        0b01010 => decode_logical_reg(raw),
        // EXTR: sf 00 100111 N0 Rm imms Rn Rd  →  bits[28:23] = 100111
        _ if (raw >> 23) & 0x3F == 0b100111 => decode_extr(raw),
        _ => Instruction::Unknown { raw },
    }
}

fn decode_add_sub_reg(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let op = (raw >> 30) & 1;
    let s = (raw >> 29) & 1;
    let shift_type = ((raw >> 22) & 0x3) as u8;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let imm6 = ((raw >> 10) & 0x3F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match (op, s) {
        (0, 0) => Instruction::AddReg { sf, rd, rn, rm, shift_type, shift_amount: imm6 },
        (0, 1) => Instruction::AddsReg { sf, rd, rn, rm, shift_type, shift_amount: imm6 },
        (1, 0) => Instruction::SubReg { sf, rd, rn, rm, shift_type, shift_amount: imm6 },
        (1, _) => Instruction::SubsReg { sf, rd, rn, rm, shift_type, shift_amount: imm6 },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_add_sub_ext_reg(raw: u32) -> Instruction {
    // Simplified: treat as add/sub register with no shift
    let sf = (raw >> 31) & 1 == 1;
    let op = (raw >> 30) & 1;
    let s = (raw >> 29) & 1;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match (op, s) {
        (0, 0) => Instruction::AddReg { sf, rd, rn, rm, shift_type: 0, shift_amount: 0 },
        (0, 1) => Instruction::AddsReg { sf, rd, rn, rm, shift_type: 0, shift_amount: 0 },
        (1, 0) => Instruction::SubReg { sf, rd, rn, rm, shift_type: 0, shift_amount: 0 },
        (1, _) => Instruction::SubsReg { sf, rd, rn, rm, shift_type: 0, shift_amount: 0 },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_logical_reg(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opc = ((raw >> 29) & 0x3) as u8;
    let shift_type = ((raw >> 22) & 0x3) as u8;
    let n = (raw >> 21) & 1 == 1;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let imm6 = ((raw >> 10) & 0x3F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    Instruction::LogicalReg { sf, opc, n, rd, rn, rm, shift_type, shift_amount: imm6 }
}

fn decode_dp_1source(raw: u32) -> Instruction {
    // DP 1-source: sf 1 S 11010110 opcode2(6) Rn Rd
    // opcode2 bits[15:10]:
    //   000000=RBIT, 000001=REV16, 000010=REV(32)/REV32(64), 000011=REV(64)
    //   000100=CLZ, 000101=CLS
    let sf = (raw >> 31) & 1 == 1;
    let opcode2 = (raw >> 10) & 0x3F;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;
    match opcode2 {
        0b000000 => Instruction::Rbit { sf, rd, rn },
        0b000001 => Instruction::Rev16 { sf, rd, rn },
        0b000010 if sf => Instruction::Rev32 { rd, rn },
        0b000010 => Instruction::Rev { sf: false, rd, rn },
        0b000011 => Instruction::Rev { sf: true, rd, rn },
        0b000100 => Instruction::Clz { sf, rd, rn },
        0b000101 => Instruction::Cls { sf, rd, rn },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_dp_2source(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opcode = (raw >> 10) & 0x3F;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match opcode {
        0b000010 => Instruction::Div { sf, rd, rn, rm, is_signed: false }, // UDIV
        0b000011 => Instruction::Div { sf, rd, rn, rm, is_signed: true },  // SDIV
        0b001000 => Instruction::ShiftVar { sf, rd, rn, rm, shift_type: 0 }, // LSLV
        0b001001 => Instruction::ShiftVar { sf, rd, rn, rm, shift_type: 1 }, // LSRV
        0b001010 => Instruction::ShiftVar { sf, rd, rn, rm, shift_type: 2 }, // ASRV
        0b001011 => Instruction::ShiftVar { sf, rd, rn, rm, shift_type: 3 }, // RORV
        // CRC32: sf=1(X)/0(W), opcode 010 sz C
        // 010000=CRC32B, 010001=CRC32H, 010010=CRC32W, 010011=CRC32X
        // 010100=CRC32CB, 010101=CRC32CH, 010110=CRC32CW, 010111=CRC32CX
        0b010000..=0b010111 => {
            let sz = ((opcode >> 0) & 0x3) as u8;
            let c = (opcode >> 2) & 1 == 1;
            Instruction::Crc32 { sf, rd, rn, rm, sz, c }
        }
        _ => Instruction::Unknown { raw },
    }
}

fn decode_extr(raw: u32) -> Instruction {
    // EXTR: sf 00 100111 N0 Rm imms Rn Rd
    let sf = (raw >> 31) & 1 == 1;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let imms = ((raw >> 10) & 0x3F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;
    Instruction::Extr { sf, rd, rn, rm, lsb: imms }
}

fn decode_dp_3source(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let o0 = (raw >> 15) & 1;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let ra = ((raw >> 10) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match o0 {
        0 => Instruction::Madd { sf, rd, rn, rm, ra },
        _ => Instruction::Msub { sf, rd, rn, rm, ra },
    }
}

fn decode_cond_select(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let op2 = (raw >> 10) & 0x3;
    let rm = ((raw >> 16) & 0x1F) as u8;
    let cond = Condition::from_bits((raw >> 12) & 0xF);
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    // op bit[30] distinguishes CSEL/CSINC (op=0) from CSINV/CSNEG (op=1)
    let op = (raw >> 30) & 1;
    match (op, op2) {
        (0, 0b00) => Instruction::Csel { sf, rd, rn, rm, cond },
        (0, 0b01) => Instruction::Csinc { sf, rd, rn, rm, cond },
        (1, 0b00) => Instruction::Csinv { sf, rd, rn, rm, cond },
        (1, 0b01) => Instruction::Csneg { sf, rd, rn, rm, cond },
        _ => Instruction::Unknown { raw },
    }
}

/// Decode CCMP / CCMN (conditional compare).
///
/// Encoding: sf op 1 11010010 Rm/imm5 cond 0 o2 o3 Rn nzcv
///   op=0 → CCMN (add), op=1 → CCMP (sub)
///   bit[11]=0 → register, bit[11]=1 → immediate
fn decode_ccmp(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let op = (raw >> 30) & 1; // 0=CCMN, 1=CCMP
    let is_imm = (raw >> 11) & 1 == 1;
    let rm_or_imm5 = ((raw >> 16) & 0x1F) as u8;
    let cond = Condition::from_bits((raw >> 12) & 0xF);
    let rn = ((raw >> 5) & 0x1F) as u8;
    let nzcv = (raw & 0xF) as u8;

    if op == 1 {
        Instruction::Ccmp { sf, rn, imm5_or_rm: rm_or_imm5, nzcv, cond, is_imm }
    } else {
        Instruction::Ccmn { sf, rn, imm5_or_rm: rm_or_imm5, nzcv, cond, is_imm }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use crate::decode::{decode, Instruction};
    use crate::cpu::Condition;

    // ── Helper: build a logical shifted-register encoding ────────────
    // sf opc 01010 shift N Rm imm6 Rn Rd
    fn enc_logical_reg(sf: bool, opc: u8, shift_type: u8, n: bool, rm: u8, imm6: u8, rn: u8, rd: u8) -> u32 {
        (u32::from(sf) << 31)
            | ((opc as u32 & 0x3) << 29)
            | (0b01010 << 24)
            | ((shift_type as u32 & 0x3) << 22)
            | (u32::from(n) << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((imm6 as u32 & 0x3F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    // ── Helper: build CSINV encoding ─────────────────────────────────
    // sf 1 0 11010100 Rm cond 00 Rn Rd
    fn enc_csinv(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
        (u32::from(sf) << 31)
            | (1 << 30) // op=1
            | (0b0011010100 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (0b00 << 10) // op2=00
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    // ── Helper: build CSNEG encoding ─────────────────────────────────
    // sf 1 0 11010100 Rm cond 01 Rn Rd
    fn enc_csneg(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
        (u32::from(sf) << 31)
            | (1 << 30) // op=1
            | (0b0011010100 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (0b01 << 10) // op2=01
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    // ── Helper: build variable shift encoding ─────────────────────────
    // sf 0011010110 Rm 0010 op2 Rn Rd
    fn enc_shift_var(sf: bool, rd: u8, rn: u8, rm: u8, op2: u8) -> u32 {
        (u32::from(sf) << 31)
            | (0b0011010110 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | (0b0010 << 12)
            | ((op2 as u32 & 0x3) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rd as u32 & 0x1F)
    }

    // ── Helper: build CCMP/CCMN register encoding ────────────────────
    // sf op 1 11010010 Rm cond 00 0 Rn 0 nzcv
    fn enc_ccmp_reg(sf: bool, op: u8, rm: u8, cond: u8, rn: u8, nzcv: u8) -> u32 {
        (u32::from(sf) << 31)
            | ((op as u32 & 1) << 30)
            | (0b111010010 << 21)
            | ((rm as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (0b0 << 11) // register form
            | (0b0 << 10)
            | (0b0 << 4) // o3=0
            | ((rn as u32 & 0x1F) << 5)
            | (nzcv as u32 & 0xF)
    }

    // ── Helper: build CCMP/CCMN immediate encoding ───────────────────
    // sf op 1 11010010 imm5 cond 10 0 Rn 0 nzcv
    fn enc_ccmp_imm(sf: bool, op: u8, imm5: u8, cond: u8, rn: u8, nzcv: u8) -> u32 {
        (u32::from(sf) << 31)
            | ((op as u32 & 1) << 30)
            | (0b111010010 << 21)
            | ((imm5 as u32 & 0x1F) << 16)
            | ((cond as u32 & 0xF) << 12)
            | (1 << 11) // immediate form
            | (0b0 << 10)
            | (0b0 << 4)
            | ((rn as u32 & 0x1F) << 5)
            | (nzcv as u32 & 0xF)
    }

    // ════════════════════════════════════════════════════════════════
    // 1. Add/Sub shifted register
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_add_reg_64() {
        let insn = decode(enc_add_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_add_reg_32() {
        let insn = decode(enc_add_reg(false, 3, 4, 5, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_add_reg_with_lsl_shift() {
        let insn = decode(enc_add_reg(true, 0, 1, 2, 0, 3), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 3 }));
    }

    #[test]
    fn test_decode_add_reg_with_lsr_shift() {
        let insn = decode(enc_add_reg(true, 0, 1, 2, 1, 16), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 1, shift_amount: 16 }));
    }

    #[test]
    fn test_decode_add_reg_with_asr_shift() {
        let insn = decode(enc_add_reg(true, 0, 1, 2, 2, 8), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 2, shift_amount: 8 }));
    }

    #[test]
    fn test_decode_adds_reg_64() {
        let insn = decode(enc_adds_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::AddsReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_adds_reg_32() {
        let insn = decode(enc_adds_reg(false, 5, 6, 7, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::AddsReg { sf: false, rd: 5, rn: 6, rm: 7, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_adds_reg_with_shift() {
        let insn = decode(enc_adds_reg(true, 0, 1, 2, 0, 4), 0x1000);
        assert!(matches!(insn, Instruction::AddsReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 4 }));
    }

    #[test]
    fn test_decode_sub_reg_64() {
        let insn = decode(enc_sub_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::SubReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_sub_reg_32() {
        let insn = decode(enc_sub_reg(false, 3, 4, 5, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::SubReg { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_sub_reg_with_shift() {
        let insn = decode(enc_sub_reg(true, 0, 1, 2, 1, 12), 0x1000);
        assert!(matches!(insn, Instruction::SubReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 1, shift_amount: 12 }));
    }

    #[test]
    fn test_decode_subs_reg_64() {
        let insn = decode(enc_subs_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::SubsReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_subs_reg_32() {
        let insn = decode(enc_subs_reg(false, 8, 9, 10, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::SubsReg { sf: false, rd: 8, rn: 9, rm: 10, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_subs_reg_with_shift() {
        let insn = decode(enc_subs_reg(true, 0, 1, 2, 2, 5), 0x1000);
        assert!(matches!(insn, Instruction::SubsReg { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 2, shift_amount: 5 }));
    }

    // ════════════════════════════════════════════════════════════════
    // 2. Logical shifted register
    // ════════════════════════════════════════════════════════════════

    // AND (opc=00, N=0)
    #[test]
    fn test_decode_and_reg_64() {
        let insn = decode(enc_and_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 0, n: false, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_and_reg_32() {
        let insn = decode(enc_and_reg(false, 3, 4, 5, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: false, opc: 0, n: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_and_reg_with_shift() {
        let insn = decode(enc_and_reg(true, 0, 1, 2, 1, 8), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 0, n: false, rd: 0, rn: 1, rm: 2, shift_type: 1, shift_amount: 8 }));
    }

    // BIC (opc=00, N=1)
    #[test]
    fn test_decode_bic_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b00, 0, true, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 0, n: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    // ORR (opc=01, N=0)
    #[test]
    fn test_decode_orr_reg_64() {
        let insn = decode(enc_orr_reg(true, 0, 1, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 1, n: false, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_orr_reg_32() {
        let insn = decode(enc_orr_reg(false, 3, 4, 5, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: false, opc: 1, n: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_orr_reg_with_shift() {
        let insn = decode(enc_orr_reg(true, 0, 1, 2, 2, 16), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 1, n: false, rd: 0, rn: 1, rm: 2, shift_type: 2, shift_amount: 16 }));
    }

    // ORN (opc=01, N=1)
    #[test]
    fn test_decode_orn_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b01, 0, true, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 1, n: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    // EOR (opc=10, N=0)
    #[test]
    fn test_decode_eor_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b10, 0, false, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 2, n: false, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_eor_reg_32() {
        let insn = decode(enc_logical_reg(false, 0b10, 0, false, 5, 0, 4, 3), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: false, opc: 2, n: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    // EON (opc=10, N=1)
    #[test]
    fn test_decode_eon_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b10, 0, true, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 2, n: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    // ANDS (opc=11, N=0)
    #[test]
    fn test_decode_ands_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b11, 0, false, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 3, n: false, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_ands_reg_32() {
        let insn = decode(enc_logical_reg(false, 0b11, 0, false, 5, 0, 4, 3), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: false, opc: 3, n: false, rd: 3, rn: 4, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    // BICS (opc=11, N=1)
    #[test]
    fn test_decode_bics_reg_64() {
        let insn = decode(enc_logical_reg(true, 0b11, 0, true, 2, 0, 1, 0), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 3, n: true, rd: 0, rn: 1, rm: 2, shift_type: 0, shift_amount: 0 }));
    }

    // ════════════════════════════════════════════════════════════════
    // 3. Multiply — MADD / MSUB
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_madd_64() {
        let insn = decode(enc_madd(true, 0, 1, 2, 3), 0x1000);
        assert!(matches!(insn, Instruction::Madd { sf: true, rd: 0, rn: 1, rm: 2, ra: 3 }));
    }

    #[test]
    fn test_decode_madd_32() {
        let insn = decode(enc_madd(false, 4, 5, 6, 7), 0x1000);
        assert!(matches!(insn, Instruction::Madd { sf: false, rd: 4, rn: 5, rm: 6, ra: 7 }));
    }

    #[test]
    fn test_decode_madd_mul_alias() {
        // MUL is MADD with Ra=XZR (31)
        let insn = decode(enc_madd(true, 0, 1, 2, 31), 0x1000);
        assert!(matches!(insn, Instruction::Madd { sf: true, rd: 0, rn: 1, rm: 2, ra: 31 }));
    }

    #[test]
    fn test_decode_msub_64() {
        let insn = decode(enc_msub(true, 0, 1, 2, 3), 0x1000);
        assert!(matches!(insn, Instruction::Msub { sf: true, rd: 0, rn: 1, rm: 2, ra: 3 }));
    }

    #[test]
    fn test_decode_msub_32() {
        let insn = decode(enc_msub(false, 4, 5, 6, 7), 0x1000);
        assert!(matches!(insn, Instruction::Msub { sf: false, rd: 4, rn: 5, rm: 6, ra: 7 }));
    }

    #[test]
    fn test_decode_msub_mneg_alias() {
        // MNEG is MSUB with Ra=XZR (31)
        let insn = decode(enc_msub(true, 0, 1, 2, 31), 0x1000);
        assert!(matches!(insn, Instruction::Msub { sf: true, rd: 0, rn: 1, rm: 2, ra: 31 }));
    }

    // ════════════════════════════════════════════════════════════════
    // 4. Division — UDIV / SDIV
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_udiv_64() {
        let insn = decode(enc_udiv(true, 0, 1, 2), 0x1000);
        assert!(matches!(insn, Instruction::Div { sf: true, rd: 0, rn: 1, rm: 2, is_signed: false }));
    }

    #[test]
    fn test_decode_udiv_32() {
        let insn = decode(enc_udiv(false, 3, 4, 5), 0x1000);
        assert!(matches!(insn, Instruction::Div { sf: false, rd: 3, rn: 4, rm: 5, is_signed: false }));
    }

    #[test]
    fn test_decode_sdiv_64() {
        let insn = decode(enc_sdiv(true, 0, 1, 2), 0x1000);
        assert!(matches!(insn, Instruction::Div { sf: true, rd: 0, rn: 1, rm: 2, is_signed: true }));
    }

    #[test]
    fn test_decode_sdiv_32() {
        let insn = decode(enc_sdiv(false, 3, 4, 5), 0x1000);
        assert!(matches!(insn, Instruction::Div { sf: false, rd: 3, rn: 4, rm: 5, is_signed: true }));
    }

    // ════════════════════════════════════════════════════════════════
    // 5. Conditional select — CSEL, CSINC, CSINV, CSNEG
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_csel_64_eq() {
        let insn = decode(enc_csel(true, 0, 1, 2, 0), 0x1000);
        assert!(matches!(insn, Instruction::Csel { sf: true, rd: 0, rn: 1, rm: 2, cond: Condition::Eq }));
    }

    #[test]
    fn test_decode_csel_32_ne() {
        let insn = decode(enc_csel(false, 3, 4, 5, 1), 0x1000);
        assert!(matches!(insn, Instruction::Csel { sf: false, rd: 3, rn: 4, rm: 5, cond: Condition::Ne }));
    }

    #[test]
    fn test_decode_csel_64_ge() {
        let insn = decode(enc_csel(true, 0, 1, 2, 0xA), 0x1000);
        assert!(matches!(insn, Instruction::Csel { sf: true, rd: 0, rn: 1, rm: 2, cond: Condition::Ge }));
    }

    #[test]
    fn test_decode_csinc_64_eq() {
        let insn = decode(enc_csinc(true, 0, 1, 2, 0), 0x1000);
        assert!(matches!(insn, Instruction::Csinc { sf: true, rd: 0, rn: 1, rm: 2, cond: Condition::Eq }));
    }

    #[test]
    fn test_decode_csinc_32_cs() {
        let insn = decode(enc_csinc(false, 3, 4, 5, 2), 0x1000);
        assert!(matches!(insn, Instruction::Csinc { sf: false, rd: 3, rn: 4, rm: 5, cond: Condition::Cs }));
    }

    #[test]
    fn test_decode_csinc_cset_alias() {
        // CSET Xd, cond → CSINC Xd, XZR, XZR, invert(cond)
        let insn = decode(enc_csinc(true, 0, 31, 31, 1), 0x1000);
        assert!(matches!(insn, Instruction::Csinc { sf: true, rd: 0, rn: 31, rm: 31, cond: Condition::Ne }));
    }

    #[test]
    fn test_decode_csinv_64_eq() {
        let insn = decode(enc_csinv(true, 0, 1, 2, 0), 0x1000);
        assert!(matches!(insn, Instruction::Csinv { sf: true, rd: 0, rn: 1, rm: 2, cond: Condition::Eq }));
    }

    #[test]
    fn test_decode_csinv_32_lt() {
        let insn = decode(enc_csinv(false, 3, 4, 5, 0xB), 0x1000);
        assert!(matches!(insn, Instruction::Csinv { sf: false, rd: 3, rn: 4, rm: 5, cond: Condition::Lt }));
    }

    #[test]
    fn test_decode_csinv_csetm_alias() {
        // CSETM Xd, cond → CSINV Xd, XZR, XZR, invert(cond)
        let insn = decode(enc_csinv(true, 0, 31, 31, 1), 0x1000);
        assert!(matches!(insn, Instruction::Csinv { sf: true, rd: 0, rn: 31, rm: 31, cond: Condition::Ne }));
    }

    #[test]
    fn test_decode_csneg_64_eq() {
        let insn = decode(enc_csneg(true, 0, 1, 2, 0), 0x1000);
        assert!(matches!(insn, Instruction::Csneg { sf: true, rd: 0, rn: 1, rm: 2, cond: Condition::Eq }));
    }

    #[test]
    fn test_decode_csneg_32_mi() {
        let insn = decode(enc_csneg(false, 3, 4, 5, 4), 0x1000);
        assert!(matches!(insn, Instruction::Csneg { sf: false, rd: 3, rn: 4, rm: 5, cond: Condition::Mi }));
    }

    // ════════════════════════════════════════════════════════════════
    // 6. Conditional compare — CCMP / CCMN
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_ccmp_reg_64() {
        let insn = decode(enc_ccmp_reg(true, 1, 2, 0, 3, 0b0100), 0x1000);
        assert!(matches!(insn, Instruction::Ccmp { sf: true, rn: 3, imm5_or_rm: 2, nzcv: 0b0100, cond: Condition::Eq, is_imm: false }));
    }

    #[test]
    fn test_decode_ccmp_reg_32() {
        let insn = decode(enc_ccmp_reg(false, 1, 5, 1, 6, 0b1000), 0x1000);
        assert!(matches!(insn, Instruction::Ccmp { sf: false, rn: 6, imm5_or_rm: 5, nzcv: 0b1000, cond: Condition::Ne, is_imm: false }));
    }

    #[test]
    fn test_decode_ccmp_imm_64() {
        let insn = decode(enc_ccmp_imm(true, 1, 10, 0, 3, 0b0010), 0x1000);
        assert!(matches!(insn, Instruction::Ccmp { sf: true, rn: 3, imm5_or_rm: 10, nzcv: 0b0010, cond: Condition::Eq, is_imm: true }));
    }

    #[test]
    fn test_decode_ccmp_imm_32() {
        let insn = decode(enc_ccmp_imm(false, 1, 15, 0xA, 4, 0b0001), 0x1000);
        assert!(matches!(insn, Instruction::Ccmp { sf: false, rn: 4, imm5_or_rm: 15, nzcv: 0b0001, cond: Condition::Ge, is_imm: true }));
    }

    #[test]
    fn test_decode_ccmn_reg_64() {
        let insn = decode(enc_ccmp_reg(true, 0, 2, 0, 3, 0b0100), 0x1000);
        assert!(matches!(insn, Instruction::Ccmn { sf: true, rn: 3, imm5_or_rm: 2, nzcv: 0b0100, cond: Condition::Eq, is_imm: false }));
    }

    #[test]
    fn test_decode_ccmn_reg_32() {
        let insn = decode(enc_ccmp_reg(false, 0, 7, 3, 8, 0b1100), 0x1000);
        assert!(matches!(insn, Instruction::Ccmn { sf: false, rn: 8, imm5_or_rm: 7, nzcv: 0b1100, cond: Condition::Cc, is_imm: false }));
    }

    #[test]
    fn test_decode_ccmn_imm_64() {
        let insn = decode(enc_ccmp_imm(true, 0, 5, 0xC, 1, 0b0000), 0x1000);
        assert!(matches!(insn, Instruction::Ccmn { sf: true, rn: 1, imm5_or_rm: 5, nzcv: 0b0000, cond: Condition::Gt, is_imm: true }));
    }

    #[test]
    fn test_decode_ccmn_imm_32() {
        let insn = decode(enc_ccmp_imm(false, 0, 31, 0xE, 10, 0b1111), 0x1000);
        assert!(matches!(insn, Instruction::Ccmn { sf: false, rn: 10, imm5_or_rm: 31, nzcv: 0b1111, cond: Condition::Al, is_imm: true }));
    }

    // ════════════════════════════════════════════════════════════════
    // 7. Variable shift — LSLV, LSRV, ASRV, RORV
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_lslv_64() {
        let insn = decode(enc_shift_var(true, 0, 1, 2, 0), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 0 }));
    }

    #[test]
    fn test_decode_lslv_32() {
        let insn = decode(enc_shift_var(false, 3, 4, 5, 0), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 0 }));
    }

    #[test]
    fn test_decode_lsrv_64() {
        let insn = decode(enc_shift_var(true, 0, 1, 2, 1), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 1 }));
    }

    #[test]
    fn test_decode_lsrv_32() {
        let insn = decode(enc_shift_var(false, 3, 4, 5, 1), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 1 }));
    }

    #[test]
    fn test_decode_asrv_64() {
        let insn = decode(enc_shift_var(true, 0, 1, 2, 2), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 2 }));
    }

    #[test]
    fn test_decode_asrv_32() {
        let insn = decode(enc_shift_var(false, 3, 4, 5, 2), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 2 }));
    }

    #[test]
    fn test_decode_rorv_64() {
        let insn = decode(enc_shift_var(true, 0, 1, 2, 3), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: true, rd: 0, rn: 1, rm: 2, shift_type: 3 }));
    }

    #[test]
    fn test_decode_rorv_32() {
        let insn = decode(enc_shift_var(false, 3, 4, 5, 3), 0x1000);
        assert!(matches!(insn, Instruction::ShiftVar { sf: false, rd: 3, rn: 4, rm: 5, shift_type: 3 }));
    }

    // ════════════════════════════════════════════════════════════════
    // Edge cases
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_add_reg_zero_regs() {
        // XZR as rd and rn
        let insn = decode(enc_add_reg(true, 31, 31, 2, 0, 0), 0x1000);
        assert!(matches!(insn, Instruction::AddReg { sf: true, rd: 31, rn: 31, rm: 2, .. }));
    }

    #[test]
    fn test_decode_mov_reg_alias() {
        // MOV is ORR Xd, XZR, Xm
        let insn = decode(enc_mov_reg(true, 0, 5), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 1, n: false, rd: 0, rn: 31, rm: 5, shift_type: 0, shift_amount: 0 }));
    }

    #[test]
    fn test_decode_logical_reg_ror_shift() {
        // ORR with ROR shift (shift_type=3)
        let insn = decode(enc_orr_reg(true, 0, 1, 2, 3, 10), 0x1000);
        assert!(matches!(insn, Instruction::LogicalReg { sf: true, opc: 1, n: false, rd: 0, rn: 1, rm: 2, shift_type: 3, shift_amount: 10 }));
    }

    #[test]
    fn test_decode_csel_all_conditions() {
        // Spot-check several condition codes
        for (code, expected) in [
            (0u8, Condition::Eq), (1, Condition::Ne), (2, Condition::Cs),
            (3, Condition::Cc), (4, Condition::Mi), (5, Condition::Pl),
            (8, Condition::Hi), (0xC, Condition::Gt), (0xE, Condition::Al),
        ] {
            let insn = decode(enc_csel(true, 0, 1, 2, code), 0x1000);
            assert!(matches!(insn, Instruction::Csel { cond, .. } if cond == expected),
                "failed for condition code {code}");
        }
    }
}
