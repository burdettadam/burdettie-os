//! ARM64 (A64) instruction decoder.
//!
//! All A64 instructions are fixed 32-bit. The top bits determine the
//! instruction class. We decode the subset needed for typical ObjC binaries:
//!
//! - Data processing (immediate): ADD, SUB, MOV (wide), MOVK, ADRP, ADR
//! - Data processing (register): ADD, SUB, ORR, AND, ORN, MADD, shifts
//! - Load/Store: LDR, STR, LDUR, STUR, LDP, STP (imm offset, pre/post-index)
//! - Branch: B, BL, BR, BLR, RET, B.cond, CBZ, CBNZ, TBZ, TBNZ
//! - System: NOP, DMB, DSB, ISB (treated as NOP for interpreter)
//! - Comparison: CMP (alias of SUBS), CMN (alias of ADDS), TST

use crate::cpu::Condition;

/// A decoded ARM64 instruction.
#[derive(Debug, Clone)]
pub enum Instruction {
    // ── Data processing (immediate) ───────────────────────────────

    /// ADD Xd|SP, Xn|SP, #imm{, shift}
    AddImm { sf: bool, rd: u8, rn: u8, imm: u32, shift: u8 },
    /// ADDS Xd, Xn|SP, #imm{, shift}  (sets flags; CMP when Rd=XZR)
    AddsImm { sf: bool, rd: u8, rn: u8, imm: u32, shift: u8 },
    /// SUB Xd|SP, Xn|SP, #imm{, shift}
    SubImm { sf: bool, rd: u8, rn: u8, imm: u32, shift: u8 },
    /// SUBS Xd, Xn|SP, #imm{, shift}  (sets flags; CMP when Rd=XZR)
    SubsImm { sf: bool, rd: u8, rn: u8, imm: u32, shift: u8 },

    /// MOVZ Xd, #imm16{, LSL #shift}  (move wide with zero)
    Movz { sf: bool, rd: u8, imm16: u16, shift: u8 },
    /// MOVK Xd, #imm16{, LSL #shift}  (move wide with keep)
    Movk { sf: bool, rd: u8, imm16: u16, shift: u8 },
    /// MOVN Xd, #imm16{, LSL #shift}  (move wide with NOT)
    Movn { sf: bool, rd: u8, imm16: u16, shift: u8 },

    /// ADRP Xd, label  (page-relative address)
    Adrp { rd: u8, imm: i64 },
    /// ADR Xd, label  (PC-relative address)
    Adr { rd: u8, imm: i64 },

    /// Logical immediate: AND, ORR, EOR, ANDS
    LogicalImm { sf: bool, opc: u8, rd: u8, rn: u8, imm: u64 },

    // ── Data processing (register) ────────────────────────────────

    /// ADD Xd, Xn, Xm{, shift #amount}
    AddReg { sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, shift_amount: u8 },
    /// ADDS Xd, Xn, Xm{, shift #amount}
    AddsReg { sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, shift_amount: u8 },
    /// SUB Xd, Xn, Xm{, shift #amount}
    SubReg { sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, shift_amount: u8 },
    /// SUBS Xd, Xn, Xm{, shift #amount}
    SubsReg { sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, shift_amount: u8 },

    /// Logical (shifted register): AND, BIC, ORR, ORN, EOR, EON, ANDS, BICS
    LogicalReg { sf: bool, opc: u8, n: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, shift_amount: u8 },

    /// MADD Xd, Xn, Xm, Xa  (Xd = Xa + Xn * Xm)  — MUL is MADD with Xa=XZR
    Madd { sf: bool, rd: u8, rn: u8, rm: u8, ra: u8 },
    /// MSUB Xd, Xn, Xm, Xa  (Xd = Xa - Xn * Xm)
    Msub { sf: bool, rd: u8, rn: u8, rm: u8, ra: u8 },

    /// SDIV / UDIV
    Div { sf: bool, rd: u8, rn: u8, rm: u8, is_signed: bool },

    /// CSEL Xd, Xn, Xm, cond
    Csel { sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition },
    /// CSINC Xd, Xn, Xm, cond  (CSET is alias with Xn=Xm=XZR)
    Csinc { sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition },
    /// CSINV Xd, Xn, Xm, cond  (CSETM is alias)
    Csinv { sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition },
    /// CSNEG Xd, Xn, Xm, cond  (CNEG is alias)
    Csneg { sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition },

    /// CCMP Xn, #imm5, #nzcv, cond
    Ccmp { sf: bool, rn: u8, imm5_or_rm: u8, nzcv: u8, cond: Condition, is_imm: bool },
    /// CCMN Xn, #imm5, #nzcv, cond
    Ccmn { sf: bool, rn: u8, imm5_or_rm: u8, nzcv: u8, cond: Condition, is_imm: bool },

    /// Variable shift: LSLV, LSRV, ASRV, RORV
    ShiftVar { sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8 },

    /// Bitfield: SBFM, BFM, UBFM (encodesaliases SXTW, SXTB, SXTH, UXTB, UXTH, LSL, LSR, ASR, UBFX, SBFX)
    Bitfield { sf: bool, opc: u8, rd: u8, rn: u8, immr: u8, imms: u8 },

    // ── Load/Store ────────────────────────────────────────────────

    /// LDR Xt, [Xn, #imm]  (unsigned offset)
    LdrImm { sf: bool, rt: u8, rn: u8, imm: u32 },
    /// STR Xt, [Xn, #imm]  (unsigned offset)
    StrImm { sf: bool, rt: u8, rn: u8, imm: u32 },

    /// LDUR Xt, [Xn, #simm9]  (unscaled)
    Ldur { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STUR Xt, [Xn, #simm9]  (unscaled)
    Stur { sf: bool, rt: u8, rn: u8, imm: i16 },

    /// LDR Xt, [Xn, #simm]!  (pre-index)
    LdrPre { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STR Xt, [Xn, #simm]!  (pre-index)
    StrPre { sf: bool, rt: u8, rn: u8, imm: i16 },

    /// LDR Xt, [Xn], #simm  (post-index)
    LdrPost { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STR Xt, [Xn], #simm  (post-index)
    StrPost { sf: bool, rt: u8, rn: u8, imm: i16 },

    /// LDP Xt1, Xt2, [Xn, #imm]
    Ldp { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP Xt1, Xt2, [Xn, #imm]
    Stp { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDP Xt1, Xt2, [Xn, #imm]!  (pre-index)
    LdpPre { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP Xt1, Xt2, [Xn, #imm]!  (pre-index)
    StpPre { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDP Xt1, Xt2, [Xn], #imm  (post-index)
    LdpPost { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP Xt1, Xt2, [Xn], #imm  (post-index)
    StpPost { sf: bool, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDRB Wt, [Xn, #imm]  (byte load, zero-extend)
    Ldrb { rt: u8, rn: u8, imm: u32 },
    /// STRB Wt, [Xn, #imm]  (byte store)
    Strb { rt: u8, rn: u8, imm: u32 },
    /// LDRH Wt, [Xn, #imm]  (halfword load, zero-extend)
    Ldrh { rt: u8, rn: u8, imm: u32 },
    /// STRH Wt, [Xn, #imm]  (halfword store)
    Strh { rt: u8, rn: u8, imm: u32 },

    /// LDRSW Xt, [Xn, #imm]  (signed word load, sign-extend to 64)
    Ldrsw { rt: u8, rn: u8, imm: u32 },

    /// LDRSB Xt/Wt, [Xn, #imm]  (signed byte load)
    Ldrsb { rt: u8, rn: u8, imm: u32, sf: bool },
    /// LDRSH Xt/Wt, [Xn, #imm]  (signed halfword load)
    Ldrsh { rt: u8, rn: u8, imm: u32, sf: bool },

    /// LDR/STR Xt, [Xn, Xm{, extend/shift}]  (register-indexed)
    LdrReg { sf: bool, rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    StrReg { sf: bool, rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    /// LDRB/STRB Wt, [Xn, Xm{, extend}]  (register-indexed byte)
    LdrbReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    StrbReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    /// LDRH/STRH Wt, [Xn, Xm{, extend}]  (register-indexed halfword)
    LdrhReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    StrhReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    /// LDRSB Xt/Wt, [Xn, Xm{, extend}]  (register-indexed signed byte)
    LdrsbReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool, sf: bool },
    /// LDRSH Xt/Wt, [Xn, Xm{, extend}]  (register-indexed signed half)
    LdrshReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool, sf: bool },
    /// LDRSW Xt, [Xn, Xm{, extend}]  (register-indexed signed word)
    LdrswReg { rt: u8, rn: u8, rm: u8, extend: u8, s: bool },

    /// LDR Xt, label  (PC-relative literal load)
    LdrLiteral { sf: bool, rt: u8, imm: i64 },

    // ── Branch ────────────────────────────────────────────────────

    /// B label  (unconditional branch)
    B { offset: i64 },
    /// BL label  (branch with link = call)
    Bl { offset: i64 },
    /// BR Xn  (branch to register)
    Br { rn: u8 },
    /// BLR Xn  (branch with link to register = indirect call)
    Blr { rn: u8 },
    /// RET {Xn}  (return, default Xn=x30)
    Ret { rn: u8 },
    /// B.cond label  (conditional branch)
    Bcond { cond: Condition, offset: i64 },
    /// CBZ Xt, label
    Cbz { sf: bool, rt: u8, offset: i64 },
    /// CBNZ Xt, label
    Cbnz { sf: bool, rt: u8, offset: i64 },
    /// TBZ Xt, #bit, label
    Tbz { rt: u8, bit: u8, offset: i64 },
    /// TBNZ Xt, #bit, label
    Tbnz { rt: u8, bit: u8, offset: i64 },

    // ── System ────────────────────────────────────────────────────

    /// NOP (and other system instructions we treat as NOP)
    Nop,
    /// SVC #imm  (supervisor call — for syscall trapping)
    Svc { imm: u16 },

    // ── SIMD/FP load/store ────────────────────────────────────────

    /// LDR Bt/Ht/St/Dt/Qt, [Xn, #imm]  (FP unsigned offset)
    FpLdrImm { size: u8, rt: u8, rn: u8, imm: u32 },
    /// STR Bt/Ht/St/Dt/Qt, [Xn, #imm]  (FP unsigned offset)
    FpStrImm { size: u8, rt: u8, rn: u8, imm: u32 },

    /// LDUR Bt/Ht/St/Dt/Qt, [Xn, #simm]  (FP unscaled)
    FpLdur { size: u8, rt: u8, rn: u8, imm: i16 },
    /// STUR Bt/Ht/St/Dt/Qt, [Xn, #simm]  (FP unscaled)
    FpStur { size: u8, rt: u8, rn: u8, imm: i16 },

    /// LDR Bt/St/Dt/Qt, [Xn, #simm]!  (FP pre-index)
    FpLdrPre { size: u8, rt: u8, rn: u8, imm: i16 },
    /// STR Bt/St/Dt/Qt, [Xn, #simm]!  (FP pre-index)
    FpStrPre { size: u8, rt: u8, rn: u8, imm: i16 },

    /// LDR Bt/St/Dt/Qt, [Xn], #simm  (FP post-index)
    FpLdrPost { size: u8, rt: u8, rn: u8, imm: i16 },
    /// STR Bt/St/Dt/Qt, [Xn], #simm  (FP post-index)
    FpStrPost { size: u8, rt: u8, rn: u8, imm: i16 },

    /// LDP St1, St2, [Xn, #imm] / LDP Dt1, Dt2, [Xn, #imm] / LDP Qt1, Qt2, [Xn, #imm]
    FpLdp { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP St1, St2, [Xn, #imm] / STP Dt1, Dt2, [Xn, #imm]
    FpStp { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDP with pre-index
    FpLdpPre { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP with pre-index
    FpStpPre { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDP with post-index
    FpLdpPost { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },
    /// STP with post-index
    FpStpPost { size: u8, rt: u8, rt2: u8, rn: u8, imm: i16 },

    /// LDR St/Dt/Qt, [Xn, Xm{, extend/shift}]  (FP register-indexed)
    FpLdrReg { size: u8, rt: u8, rn: u8, rm: u8, extend: u8, s: bool },
    /// STR St/Dt/Qt, [Xn, Xm{, extend/shift}]  (FP register-indexed)
    FpStrReg { size: u8, rt: u8, rn: u8, rm: u8, extend: u8, s: bool },

    /// LDR St/Dt/Qt, label  (FP PC-relative literal)
    FpLdrLiteral { size: u8, rt: u8, imm: i64 },

    // ── SIMD/FP data processing ───────────────────────────────────

    /// FMOV Dd, Dn / FMOV Ss, Sn (FP register move)
    FMovReg { sf: bool, rd: u8, rn: u8 },
    /// FMOV Dd, Xn (GPR to FP)
    FMovToFp { sf: bool, rd: u8, rn: u8 },
    /// FMOV Xn, Dd (FP to GPR)
    FMovToGpr { sf: bool, rd: u8, rn: u8 },
    /// FMOV Dd, #imm (FP immediate)
    FMovImm { sf: bool, rd: u8, imm: u64 },

    /// SCVTF Dd/Sd, Xn/Wn (signed int to float)
    Scvtf { sf_fp: bool, sf_int: bool, rd: u8, rn: u8 },
    /// UCVTF Dd/Sd, Xn/Wn (unsigned int to float)
    Ucvtf { sf_fp: bool, sf_int: bool, rd: u8, rn: u8 },
    /// FCVTZS Xd/Wd, Dn/Sn (float to signed int, round toward zero)
    Fcvtzs { sf_int: bool, sf_fp: bool, rd: u8, rn: u8 },
    /// FCVTZU Xd/Wd, Dn/Sn (float to unsigned int, round toward zero)
    Fcvtzu { sf_int: bool, sf_fp: bool, rd: u8, rn: u8 },

    /// FCVT Sd, Dn / FCVT Dd, Sn (float precision conversion)
    Fcvt { rd: u8, rn: u8, to_double: bool },

    /// FCMP Dn, Dm / FCMP Sn, Sm (sets NZCV)
    Fcmp { sf: bool, rn: u8, rm: u8 },
    /// FCMP Dn, #0.0 / FCMP Sn, #0.0
    FcmpZero { sf: bool, rn: u8 },

    /// FADD Dd, Dn, Dm / FADD Ss, Sn, Sm
    Fadd { sf: bool, rd: u8, rn: u8, rm: u8 },
    /// FSUB Dd, Dn, Dm
    Fsub { sf: bool, rd: u8, rn: u8, rm: u8 },
    /// FMUL Dd, Dn, Dm
    Fmul { sf: bool, rd: u8, rn: u8, rm: u8 },
    /// FDIV Dd, Dn, Dm
    Fdiv { sf: bool, rd: u8, rn: u8, rm: u8 },
    /// FNEG Dd, Dn
    Fneg { sf: bool, rd: u8, rn: u8 },
    /// FABS Dd, Dn
    Fabs { sf: bool, rd: u8, rn: u8 },
    /// FSQRT Dd, Dn
    Fsqrt { sf: bool, rd: u8, rn: u8 },

    /// FCSEL Dd, Dn, Dm, cond
    Fcsel { sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition },

    // ── Fallback ──────────────────────────────────────────────────

    /// Unknown/unimplemented instruction.
    Unknown { raw: u32 },
}

/// Decode a single 32-bit ARM64 instruction.
pub fn decode(raw: u32, pc: u64) -> Instruction {
    // Top-level dispatch by bits [28:25] (the "op0" field).
    let op0 = (raw >> 25) & 0xF;

    match op0 {
        // 100x — Data processing (immediate)
        0b1000 | 0b1001 => decode_dp_imm(raw, pc),
        // x101 — Data processing (register)
        0b0101 | 0b1101 => decode_dp_reg(raw),
        // x1x0 — Loads and stores
        0b0100 | 0b0110 | 0b1100 | 0b1110 => decode_load_store(raw, pc),
        // x111 — Floating-point / SIMD data processing
        0b0111 | 0b1111 => decode_fp_dp(raw),
        // 101x — Branch, exception, system
        0b1010 | 0b1011 => decode_branch(raw, pc),
        // 0101 — data processing (register)  [already covered]
        // Remaining patterns
        _ => {
            // Check for branch patterns more precisely
            // Bits [31:26] = 00010x → B.cond / SVC / etc.
            let top6 = raw >> 26;
            if top6 == 0b000101 || top6 == 0b100101 {
                // B / BL
                return decode_branch(raw, pc);
            }
            Instruction::Unknown { raw }
        }
    }
}

// ── Data processing (immediate) ────────────────────────────────────

fn decode_dp_imm(raw: u32, pc: u64) -> Instruction {
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
        Instruction::Adrp { rd, imm: page_pc as i64 + imm }
    } else {
        // ADR: sign-extend 21-bit, add to PC
        let imm = sign_extend(imm21 as u64, 21);
        Instruction::Adr { rd, imm: pc as i64 + imm }
    }
}

fn decode_add_sub_imm(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let op = (raw >> 30) & 1;  // 0=ADD, 1=SUB
    let s = (raw >> 29) & 1;   // set flags
    let shift = ((raw >> 22) & 0x3) as u8; // 0=none, 1=LSL#12
    let imm12 = ((raw >> 10) & 0xFFF) as u32;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rd = (raw & 0x1F) as u8;

    match (op, s) {
        (0, 0) => Instruction::AddImm { sf, rd, rn, imm: imm12, shift },
        (0, 1) => Instruction::AddsImm { sf, rd, rn, imm: imm12, shift },
        (1, 0) => Instruction::SubImm { sf, rd, rn, imm: imm12, shift },
        (1, _) => Instruction::SubsImm { sf, rd, rn, imm: imm12, shift },
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
    Instruction::LogicalImm { sf, opc, rd, rn, imm }
}

fn decode_move_wide(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let opc = (raw >> 29) & 0x3;
    let hw = ((raw >> 21) & 0x3) as u8;
    let imm16 = ((raw >> 5) & 0xFFFF) as u16;
    let rd = (raw & 0x1F) as u8;
    let shift = hw * 16;

    match opc {
        0b00 => Instruction::Movn { sf, rd, imm16, shift },
        0b10 => Instruction::Movz { sf, rd, imm16, shift },
        0b11 => Instruction::Movk { sf, rd, imm16, shift },
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

    Instruction::Bitfield { sf, opc, rd, rn, immr, imms }
}

// ── Data processing (register) ─────────────────────────────────────

fn decode_dp_reg(raw: u32) -> Instruction {
    // Bits [28:24] further subdivide
    let _op1 = (raw >> 21) & 0xF;
    let _op0_top = (raw >> 28) & 1;

    // Bits [28:21]
    let op_class = (raw >> 21) & 0xFF;

    // Data processing (2 source): sf 0 D 11010110 Rm opcode Rn Rd
    if (op_class & 0b11111110) == 0b00011010 || (op_class & 0b11111110) == 0b10011010 {
        return decode_dp_2source(raw);
    }

    // Data processing (3 source): sf op54 11011 op31 Rm o0 Ra Rn Rd
    if (raw >> 24) & 0x1F == 0b11011 {
        return decode_dp_3source(raw);
    }

    // Conditional compare: sf op 1 11010010 Rm/imm5 cond 0/1 0 o3 Rn nzcv
    // bits[28:21] == x1011010 0 (CCMP/CCMN)
    if (raw >> 24) & 0x1F == 0b11010 && ((raw >> 21) & 0x1) == 0b0 && ((raw >> 10) & 0x1) == 0b0 {
        // This is a conditional-compare, not conditional-select
        // bit[4]==0 → register, bit[4]==1 → would be something else, but bit[11] distinguishes
        return decode_ccmp(raw);
    }

    // Conditional select: sf op S 11010100 Rm cond op2 Rn Rd
    if (raw >> 21) & 0x7FF == 0b00011010100
        || (raw >> 21) & 0x7FF == 0b10011010100
    {
        return decode_cond_select(raw);
    }
    // More flexible check for conditional select
    if (raw >> 24) & 0xFF == 0b11010100 || (raw >> 21) & 0x3FF == 0b1101010100 {
        return decode_cond_select(raw);
    }

    // Logical (shifted register): sf opc 01010 shift N Rm imm6 Rn Rd
    let op_bits = (raw >> 24) & 0x1F;
    if op_bits == 0b01010 {
        return decode_logical_reg(raw);
    }

    // Add/subtract (shifted register): sf op S 01011 shift 0 Rm imm6 Rn Rd
    if op_bits == 0b01011 && ((raw >> 21) & 1) == 0 {
        return decode_add_sub_reg(raw);
    }

    // Add/subtract (extended register): sf op S 01011 opt 1 Rm option imm3 Rn Rd
    if op_bits == 0b01011 && ((raw >> 21) & 1) == 1 {
        // Extended register add/sub — simplified: treat extend as zero
        return decode_add_sub_ext_reg(raw);
    }

    // Conditional select: more patterns
    if (raw >> 24) & 0x1F == 0b11010 && ((raw >> 21) & 0x7) == 0b100 {
        return decode_cond_select(raw);
    }

    Instruction::Unknown { raw }
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
        _ => Instruction::Unknown { raw },
    }
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

// ── Load/Store ─────────────────────────────────────────────────────

fn decode_load_store(raw: u32, pc: u64) -> Instruction {
    let _op0 = (raw >> 28) & 0xF;
    let _op1 = (raw >> 26) & 1;

    // LDR (literal): opc 01 1 V 00 imm19 Rt
    if (raw >> 24) & 0x3F == 0b011000 || (raw >> 24) & 0x3F == 0b011001 {
        let top2 = (raw >> 30) & 0x3;
        let v = (raw >> 26) & 1;
        let rt = (raw & 0x1F) as u8;
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        if v == 1 {
            // FP/SIMD literal: opc 01 1 1 00 imm19 Rt
            // opc: 00=S(32), 01=D(64), 10=Q(128)
            let size = match top2 { 0b00 => 4u8, 0b01 => 8, 0b10 => 16, _ => return Instruction::Unknown { raw } };
            return Instruction::FpLdrLiteral { size, rt, imm: pc as i64 + offset };
        }
        if (raw >> 24) & 0xFF == 0b00011000 || (raw >> 24) & 0xFF == 0b01011000 {
            let sf = top2 == 0b01;
            return Instruction::LdrLiteral { sf, rt, imm: pc as i64 + offset };
        }
    }

    // Load/store pair: opc 101 V 0xx L ... → bits[29:27] == 0b101
    let is_pair = (raw >> 27) & 0x7 == 0b101;
    if is_pair {
        let v = (raw >> 26) & 1;
        if v == 1 {
            return decode_fp_load_store_pair(raw);
        }
        return decode_load_store_pair(raw);
    }

    // Load/store register (unsigned immediate, unscaled, pre/post-index)
    decode_load_store_reg(raw)
}

fn decode_load_store_pair(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let _opc = (raw >> 22) & 0x3; // 01=LDP, 00=STP (with variants)
    let l = (raw >> 22) & 1;     // 1=load, 0=store
    let idx = (raw >> 23) & 0x3; // 01=post, 11=pre, 10=signed offset
    let imm7 = ((raw >> 15) & 0x7F) as i16;
    let rt2 = ((raw >> 10) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    // Sign-extend 7-bit immediate, scale by register size
    let scale = if sf { 8 } else { 4 };
    let offset = sign_extend7(imm7) * scale;

    match (l, idx) {
        (1, 0b01) => Instruction::LdpPost { sf, rt, rt2, rn, imm: offset },
        (1, 0b11) => Instruction::LdpPre { sf, rt, rt2, rn, imm: offset },
        (1, 0b10) => Instruction::Ldp { sf, rt, rt2, rn, imm: offset },
        (0, 0b01) => Instruction::StpPost { sf, rt, rt2, rn, imm: offset },
        (0, 0b11) => Instruction::StpPre { sf, rt, rt2, rn, imm: offset },
        (0, 0b10) => Instruction::Stp { sf, rt, rt2, rn, imm: offset },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_fp_load_store_pair(raw: u32) -> Instruction {
    let opc = (raw >> 30) & 0x3; // 00=S(32-bit), 01=D(64-bit), 10=Q(128-bit)
    let l = (raw >> 22) & 1;     // 1=load, 0=store
    let idx = (raw >> 23) & 0x3; // 01=post, 11=pre, 10=signed offset
    let imm7 = ((raw >> 15) & 0x7F) as i16;
    let rt2 = ((raw >> 10) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    let (size, scale): (u8, i16) = match opc {
        0b00 => (4, 4),   // S registers
        0b01 => (8, 8),   // D registers
        0b10 => (16, 16), // Q registers
        _ => return Instruction::Unknown { raw },
    };
    let offset = sign_extend7(imm7) * scale;

    match (l, idx) {
        (1, 0b01) => Instruction::FpLdpPost { size, rt, rt2, rn, imm: offset },
        (1, 0b11) => Instruction::FpLdpPre { size, rt, rt2, rn, imm: offset },
        (1, 0b10) => Instruction::FpLdp { size, rt, rt2, rn, imm: offset },
        (0, 0b01) => Instruction::FpStpPost { size, rt, rt2, rn, imm: offset },
        (0, 0b11) => Instruction::FpStpPre { size, rt, rt2, rn, imm: offset },
        (0, 0b10) => Instruction::FpStp { size, rt, rt2, rn, imm: offset },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_load_store_reg(raw: u32) -> Instruction {
    let size = (raw >> 30) & 0x3;
    let v = (raw >> 26) & 1;     // SIMD/FP
    let opc = (raw >> 22) & 0x3;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    if v == 1 {
        return decode_fp_load_store_reg(raw, size, opc, rn, rt);
    }

    // Unsigned offset: size 11 1 0 01 opc imm12 Rn Rt
    if (raw >> 24) & 0x3F == 0b111001 || (raw >> 24) & 0x3F == 0b101001
        || (raw >> 24) & 0x3F == 0b011001 || (raw >> 24) & 0x3F == 0b001001
    {
        let imm12 = ((raw >> 10) & 0xFFF) as u32;
        let scale = 1u32 << size;
        let offset = imm12 * scale;

        return match (size, opc) {
            (0b11, 0b01) => Instruction::LdrImm { sf: true, rt, rn, imm: offset },
            (0b11, 0b00) => Instruction::StrImm { sf: true, rt, rn, imm: offset },
            (0b10, 0b01) => Instruction::LdrImm { sf: false, rt, rn, imm: offset },
            (0b10, 0b00) => Instruction::StrImm { sf: false, rt, rn, imm: offset },
            (0b10, 0b10) => Instruction::Ldrsw { rt, rn, imm: offset },
            (0b00, 0b01) => Instruction::Ldrb { rt, rn, imm: offset },
            (0b00, 0b00) => Instruction::Strb { rt, rn, imm: offset },
            (0b01, 0b01) => Instruction::Ldrh { rt, rn, imm: offset },
            (0b01, 0b00) => Instruction::Strh { rt, rn, imm: offset },
            // Signed byte/halfword loads
            (0b00, 0b10) => Instruction::Ldrsb { rt, rn, imm: offset, sf: true },   // LDRSB Xt
            (0b00, 0b11) => Instruction::Ldrsb { rt, rn, imm: offset, sf: false },  // LDRSB Wt
            (0b01, 0b10) => Instruction::Ldrsh { rt, rn, imm: offset, sf: true },   // LDRSH Xt
            (0b01, 0b11) => Instruction::Ldrsh { rt, rn, imm: offset, sf: false },  // LDRSH Wt
            _ => Instruction::Unknown { raw },
        };
    }

    // Unscaled / pre-index / post-index: size 11 1 0 00 opc 0 imm9 idx Rn Rt
    if (raw >> 24) & 0x3F == 0b111000 || (raw >> 24) & 0x3F == 0b101000
        || (raw >> 24) & 0x3F == 0b011000 || (raw >> 24) & 0x3F == 0b001000
    {
        let imm9 = ((raw >> 12) & 0x1FF) as i16;
        let imm = sign_extend9(imm9);
        let idx = (raw >> 10) & 0x3;
        let sf = size == 0b11;

        let is_load = opc == 0b01;
        let is_sw = size == 0b10 && opc == 0b10; // LDRSW

        return match idx {
            0b00 => {
                // Unscaled
                if is_sw {
                    Instruction::Ldur { sf: true, rt, rn, imm }
                } else if is_load || opc == 0b01 {
                    if size == 0b00 { Instruction::Ldrb { rt, rn, imm: imm as u32 } }
                    else { Instruction::Ldur { sf, rt, rn, imm } }
                } else {
                    if size == 0b00 { Instruction::Strb { rt, rn, imm: imm as u32 } }
                    else { Instruction::Stur { sf, rt, rn, imm } }
                }
            }
            0b01 => {
                // Post-index
                if is_load { Instruction::LdrPost { sf, rt, rn, imm } }
                else { Instruction::StrPost { sf, rt, rn, imm } }
            }
            0b11 => {
                // Pre-index
                if is_load { Instruction::LdrPre { sf, rt, rn, imm } }
                else { Instruction::StrPre { sf, rt, rn, imm } }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Register offset: size 11 1 0 00 opc 1 Rm option S 10 Rn Rt
    // bits[11:10] == 0b10 (register offset), bit[21] == 1
    if ((raw >> 21) & 1) == 1 && ((raw >> 10) & 0x3) == 0b10 {
        let rm = ((raw >> 16) & 0x1F) as u8;
        let extend = ((raw >> 13) & 0x7) as u8; // option field
        let s = ((raw >> 12) & 1) == 1;         // shift flag
        return match (size, opc) {
            (0b11, 0b01) => Instruction::LdrReg { sf: true, rt, rn, rm, extend, s },
            (0b11, 0b00) => Instruction::StrReg { sf: true, rt, rn, rm, extend, s },
            (0b10, 0b01) => Instruction::LdrReg { sf: false, rt, rn, rm, extend, s },
            (0b10, 0b00) => Instruction::StrReg { sf: false, rt, rn, rm, extend, s },
            (0b10, 0b10) => Instruction::LdrswReg { rt, rn, rm, extend, s },
            (0b00, 0b01) => Instruction::LdrbReg { rt, rn, rm, extend, s },
            (0b00, 0b00) => Instruction::StrbReg { rt, rn, rm, extend, s },
            (0b01, 0b01) => Instruction::LdrhReg { rt, rn, rm, extend, s },
            (0b01, 0b00) => Instruction::StrhReg { rt, rn, rm, extend, s },
            (0b00, 0b10) => Instruction::LdrsbReg { rt, rn, rm, extend, s, sf: true },
            (0b00, 0b11) => Instruction::LdrsbReg { rt, rn, rm, extend, s, sf: false },
            (0b01, 0b10) => Instruction::LdrshReg { rt, rn, rm, extend, s, sf: true },
            (0b01, 0b11) => Instruction::LdrshReg { rt, rn, rm, extend, s, sf: false },
            _ => Instruction::Unknown { raw },
        };
    }

    Instruction::Unknown { raw }
}

/// Decode SIMD/FP scalar load/store (v=1 path from decode_load_store_reg).
///
/// size:opc determines the register width:
///   size=00, opc=0x → B (8-bit)
///   size=01, opc=0x → H (16-bit)
///   size=10, opc=0x → S (32-bit)
///   size=11, opc=0x → D (64-bit)
///   size=00, opc=1x → Q (128-bit)
fn decode_fp_load_store_reg(raw: u32, size: u32, opc: u32, rn: u8, rt: u8) -> Instruction {
    // Compute access size in bytes
    let access_size: u8 = if opc & 0b10 != 0 {
        // opc bit[1]=1 → 128-bit (Q)
        16
    } else {
        1u8 << size // 1,2,4,8
    };
    let is_load = opc & 1 == 1;
    let log2_sz = match access_size { 1 => 0u32, 2 => 1, 4 => 2, 8 => 3, 16 => 4, _ => 0 };

    // Unsigned offset: size 11 1 1 01 opc imm12 Rn Rt  (bit[24]=1, bit[23:22]=opc)
    if (raw >> 24) & 0x3F == 0b111101 || (raw >> 24) & 0x3F == 0b101101
        || (raw >> 24) & 0x3F == 0b011101 || (raw >> 24) & 0x3F == 0b001101
    {
        let imm12 = ((raw >> 10) & 0xFFF) as u32;
        let offset = imm12 << log2_sz;
        return if is_load {
            Instruction::FpLdrImm { size: access_size, rt, rn, imm: offset }
        } else {
            Instruction::FpStrImm { size: access_size, rt, rn, imm: offset }
        };
    }

    // Unscaled/pre/post-index: size 11 1 1 00 opc 0 imm9 idx Rn Rt
    if (raw >> 24) & 0x3F == 0b111100 || (raw >> 24) & 0x3F == 0b101100
        || (raw >> 24) & 0x3F == 0b011100 || (raw >> 24) & 0x3F == 0b001100
    {
        let imm9 = ((raw >> 12) & 0x1FF) as i16;
        let imm = sign_extend9(imm9);
        let idx = (raw >> 10) & 0x3;

        return match idx {
            0b00 => {
                // Unscaled
                if is_load { Instruction::FpLdur { size: access_size, rt, rn, imm } }
                else { Instruction::FpStur { size: access_size, rt, rn, imm } }
            }
            0b01 => {
                if is_load { Instruction::FpLdrPost { size: access_size, rt, rn, imm } }
                else { Instruction::FpStrPost { size: access_size, rt, rn, imm } }
            }
            0b11 => {
                if is_load { Instruction::FpLdrPre { size: access_size, rt, rn, imm } }
                else { Instruction::FpStrPre { size: access_size, rt, rn, imm } }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Register offset: size 11 1 1 00 opc 1 Rm option S 10 Rn Rt
    if ((raw >> 21) & 1) == 1 && ((raw >> 10) & 0x3) == 0b10 {
        let rm = ((raw >> 16) & 0x1F) as u8;
        let extend = ((raw >> 13) & 0x7) as u8;
        let s = ((raw >> 12) & 1) == 1;
        return if is_load {
            Instruction::FpLdrReg { size: access_size, rt, rn, rm, extend, s }
        } else {
            Instruction::FpStrReg { size: access_size, rt, rn, rm, extend, s }
        };
    }

    Instruction::Unknown { raw }
}

// ── Branch ─────────────────────────────────────────────────────────

fn decode_branch(raw: u32, pc: u64) -> Instruction {
    // Unconditional branch (immediate): 0 00101 imm26
    if (raw >> 26) & 0x3F == 0b000101 {
        let imm26 = (raw & 0x03FFFFFF) as u64;
        let offset = sign_extend(imm26, 26) << 2;
        return Instruction::B { offset: pc as i64 + offset };
    }
    // BL: 1 00101 imm26
    if (raw >> 26) & 0x3F == 0b100101 {
        let imm26 = (raw & 0x03FFFFFF) as u64;
        let offset = sign_extend(imm26, 26) << 2;
        return Instruction::Bl { offset: pc as i64 + offset };
    }

    // Conditional branch: 0101010 0 imm19 0 cond
    if (raw >> 25) & 0x7F == 0b0101010 {
        let cond = Condition::from_bits(raw & 0xF);
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        return Instruction::Bcond { cond, offset: pc as i64 + offset };
    }

    // CBZ/CBNZ: sf 011010 op imm19 Rt
    if (raw >> 25) & 0x3F == 0b011010 {
        let sf = (raw >> 31) & 1 == 1;
        let op = (raw >> 24) & 1;
        let rt = (raw & 0x1F) as u8;
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        return if op == 0 {
            Instruction::Cbz { sf, rt, offset: pc as i64 + offset }
        } else {
            Instruction::Cbnz { sf, rt, offset: pc as i64 + offset }
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
            Instruction::Tbz { rt, bit, offset: pc as i64 + offset }
        } else {
            Instruction::Tbnz { rt, bit, offset: pc as i64 + offset }
        };
    }

    // Unconditional branch (register): 1101011 opc 11111 000000 Rn 00000
    if (raw >> 25) & 0x7F == 0b1101011 {
        let opc = (raw >> 21) & 0xF;
        let rn = ((raw >> 5) & 0x1F) as u8;
        return match opc {
            0b0000 => Instruction::Br { rn },
            0b0001 => Instruction::Blr { rn },
            0b0010 => Instruction::Ret { rn },
            _ => Instruction::Unknown { raw },
        };
    }

    // System: NOP, DMB, DSB, ISB, SVC
    if (raw >> 22) & 0x3FF == 0b1101010100 {
        // SVC #imm16
        if (raw >> 21) & 0x7 == 0b000 && (raw >> 5) & 1 == 1 {
            let imm = ((raw >> 5) & 0xFFFF) as u16;
            return Instruction::Svc { imm };
        }
        // NOP and other hints
        return Instruction::Nop;
    }
    // NOP encoding: D503201F
    if raw == 0xD503201F {
        return Instruction::Nop;
    }
    // DSB, DMB, ISB — treat as NOP
    if (raw & 0xFFFFF0FF) == 0xD503309F
        || (raw & 0xFFFFF0FF) == 0xD50330BF
        || raw == 0xD50330DF
    {
        return Instruction::Nop;
    }

    Instruction::Unknown { raw }
}

// ── Floating-Point / SIMD Data Processing ──────────────────────────

/// Decode floating-point and SIMD data processing instructions.
///
/// Covers: FMOV, FADD, FSUB, FMUL, FDIV, FNEG, FABS, FSQRT, FCMP,
/// SCVTF, UCVTF, FCVTZS, FCVTZU, FCVT, FCSEL, FMOV imm.
fn decode_fp_dp(raw: u32) -> Instruction {
    // Floating-point conditional select: type 11110 ftype 1 Rm cond 11 Rn Rd
    // bits[23:21] = 100, bits[11:10] = 11
    if (raw >> 21) & 0x7 == 0b100 && (raw >> 10) & 0x3 == 0b11
        && (raw >> 24) & 0xFF == 0b00011110
    {
        let ftype = (raw >> 22) & 0x3;
        let sf = ftype == 0b01; // 00=S, 01=D
        let rm = ((raw >> 16) & 0x1F) as u8;
        let cond = Condition::from_bits((raw >> 12) & 0xF);
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return Instruction::Fcsel { sf, rd, rn, rm, cond };
    }

    // Floating-point compare: x 0011110 ftype 1 Rm 001000 Rn opcode2
    // bits[15:10] = 001000 (op=00, fixed=1000)
    if (raw >> 24) & 0xFF == 0b00011110 && (raw >> 21) & 1 == 1
        && (raw >> 10) & 0x3F == 0b001000
    {
        let ftype = (raw >> 22) & 0x3;
        let sf = ftype == 0b01;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let opc = raw & 0x1F;
        if opc & 0b01000 != 0 {
            // Compare with zero
            return Instruction::FcmpZero { sf, rn };
        }
        return Instruction::Fcmp { sf, rn, rm };
    }

    // Floating-point data-processing (2 source): 0 0011110 ftype 1 Rm opcode 10 Rn Rd
    if (raw >> 24) & 0xFF == 0b00011110 && (raw >> 10) & 0x3 == 0b10
        && (raw >> 21) & 0x1 == 1
    {
        let ftype = (raw >> 22) & 0x3;
        let sf = ftype == 0b01;
        let rm = ((raw >> 16) & 0x1F) as u8;
        let opcode = (raw >> 12) & 0xF;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return match opcode {
            0b0000 => Instruction::Fmul { sf, rd, rn, rm },
            0b0001 => Instruction::Fdiv { sf, rd, rn, rm },
            0b0010 => Instruction::Fadd { sf, rd, rn, rm },
            0b0011 => Instruction::Fsub { sf, rd, rn, rm },
            _ => Instruction::Unknown { raw },
        };
    }

    // Floating-point data-processing (1 source): 0 0011110 ftype 1 opcode 10000 Rn Rd
    if (raw >> 24) & 0xFF == 0b00011110 && (raw >> 10) & 0x3F == 0b010000
        && (raw >> 21) & 0x1 == 1
    {
        let ftype = (raw >> 22) & 0x3;
        let sf = ftype == 0b01;
        let opcode = (raw >> 15) & 0x3F;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;
        return match opcode {
            0b000000 => Instruction::FMovReg { sf, rd, rn },
            0b000001 => Instruction::Fabs { sf, rd, rn },
            0b000010 => Instruction::Fneg { sf, rd, rn },
            0b000011 => Instruction::Fsqrt { sf, rd, rn },
            // FCVT between single and double
            0b000100 => {
                // ftype=00 (S→D) or ftype=01 (D→S)
                if ftype == 0b00 {
                    Instruction::Fcvt { rd, rn, to_double: true }
                } else {
                    Instruction::Fcvt { rd, rn, to_double: false }
                }
            }
            0b000101 => {
                // ftype=01 (D→S) or ftype=00 (S→D) — double→single
                if ftype == 0b01 {
                    Instruction::Fcvt { rd, rn, to_double: false }
                } else {
                    Instruction::Fcvt { rd, rn, to_double: true }
                }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Floating-point<->integer conversion: sf 0011110 ftype 1 rmode opcode 000000 Rn Rd
    if (raw >> 24) & 0x7F == 0b0011110 && (raw >> 10) & 0x3F == 0b000000 {
        let sf_int = (raw >> 31) & 1 == 1; // 0=W, 1=X
        let ftype = (raw >> 22) & 0x3;     // 00=S, 01=D
        let sf_fp = ftype == 0b01;
        let rmode = (raw >> 19) & 0x3;
        let opcode = (raw >> 16) & 0x7;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rd = (raw & 0x1F) as u8;

        return match (rmode, opcode) {
            // FCVTZS (FP→signed int, round toward zero)
            (0b11, 0b000) => Instruction::Fcvtzs { sf_int, sf_fp, rd, rn },
            // FCVTZU (FP→unsigned int, round toward zero)
            (0b11, 0b001) => Instruction::Fcvtzu { sf_int, sf_fp, rd, rn },
            // SCVTF (signed int→FP)
            (0b00, 0b010) => Instruction::Scvtf { sf_fp, sf_int, rd, rn },
            // UCVTF (unsigned int→FP)
            (0b00, 0b011) => Instruction::Ucvtf { sf_fp, sf_int, rd, rn },
            // FMOV Xd, Dn (FP to GPR)
            (0b00, 0b110) => Instruction::FMovToGpr { sf: sf_fp, rd, rn },
            // FMOV Dn, Xd (GPR to FP)
            (0b00, 0b111) => Instruction::FMovToFp { sf: sf_fp, rd, rn },
            _ => Instruction::Unknown { raw },
        };
    }

    // Floating-point immediate: 0 0011110 ftype 1 imm8 100 imm5=00000 Rd
    if (raw >> 24) & 0xFF == 0b00011110 && (raw >> 5) & 0x1F == 0 && (raw >> 10) & 0x7 == 0b100
        && (raw >> 21) & 0x1 == 1
    {
        let ftype = (raw >> 22) & 0x3;
        let sf = ftype == 0b01;
        let imm8 = ((raw >> 13) & 0xFF) as u8;
        let rd = (raw & 0x1F) as u8;
        let imm = decode_fp_imm8(imm8, sf);
        return Instruction::FMovImm { sf, rd, imm };
    }

    Instruction::Unknown { raw }
}

/// Decode an 8-bit floating-point immediate to its bit pattern.
///
/// The 8-bit immediate encodes: sign(1) | exp_hi(1) | exp_lo(2) | frac(4)
/// For double (64-bit): sign | NOT(b) repeated 8 times | b:c:d | frac:zeros(48)
/// For single (32-bit): sign | NOT(b) repeated 5 times | b:c:d | frac:zeros(19)
fn decode_fp_imm8(imm8: u8, is_double: bool) -> u64 {
    let sign = ((imm8 >> 7) & 1) as u64;
    let exp3 = ((imm8 >> 4) & 0x7) as u64;
    let frac4 = (imm8 & 0xF) as u64;

    if is_double {
        let top_bit = (exp3 >> 2) & 1;
        let exp_hi = if top_bit != 0 { 0b0_u64 } else { 0b1 };
        let exp_rep = if top_bit != 0 { 0x00_u64 } else { 0xFF };
        let full_exp = (exp_hi << 10) | (exp_rep << 2) | (exp3 & 0x3);
        (sign << 63) | (full_exp << 52) | (frac4 << 48)
    } else {
        let top_bit = (exp3 >> 2) & 1;
        let exp_hi = if top_bit != 0 { 0b0_u64 } else { 0b1 };
        let exp_rep = if top_bit != 0 { 0x00_u64 } else { 0x1F };
        let full_exp = (exp_hi << 7) | (exp_rep << 2) | (exp3 & 0x3);
        ((sign << 31) | (full_exp << 23) | (frac4 << 19)) & 0xFFFF_FFFF
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Sign-extend a `bits`-wide value to i64.
fn sign_extend(val: u64, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((val as i64) << shift) >> shift
}

/// Sign-extend a 7-bit value.
fn sign_extend7(val: i16) -> i16 {
    if val & 0x40 != 0 {
        val | !0x7F
    } else {
        val & 0x7F
    }
}

/// Sign-extend a 9-bit value.
fn sign_extend9(val: i16) -> i16 {
    if val & 0x100 != 0 {
        val | !0x1FF
    } else {
        val & 0x1FF
    }
}

/// Decode an A64 bitmask immediate from N:imms:immr fields.
/// This is one of the most complex parts of the ARM64 encoding.
fn decode_bitmask_imm(n: u32, imms: u32, immr: u32, sf: bool) -> u64 {
    let len = if n != 0 {
        6
    } else {
        // highest bit set in ~imms (6-bit field)
        let not_imms = (!imms) & 0x3F;
        if not_imms == 0 { return 0; }
        (31 - not_imms.leading_zeros()) as u32
    };

    if len == 0 { return 0; }

    let size = 1u32 << len;
    let levels = size - 1;
    let s = imms & levels;
    let r = immr & levels;

    if s == levels { return 0; } // reserved

    let welem: u64 = if s + 1 >= 64 { u64::MAX } else { (1u64 << (s + 1)) - 1 };
    // Rotate right by r within 'size' bits
    let rotated = if r == 0 {
        welem
    } else {
        let mask = if size >= 64 { u64::MAX } else { (1u64 << size) - 1 };
        ((welem >> r) | (welem << (size - r))) & mask
    };

    // Replicate the pattern across 64 bits
    let mut result = 0u64;
    let mut pos = 0;
    while pos < 64 {
        result |= rotated << pos;
        pos += size;
    }

    if !sf {
        result &= 0xFFFF_FFFF;
    }
    result
}

/// Shift a value by (shift_type, amount).
/// shift_type: 0=LSL, 1=LSR, 2=ASR, 3=ROR
pub fn apply_shift(val: u64, shift_type: u8, amount: u8, is_64: bool) -> u64 {
    if amount == 0 {
        return val;
    }
    let amt = amount as u32;
    if is_64 {
        match shift_type {
            0 => val << amt,
            1 => val >> amt,
            2 => ((val as i64) >> amt) as u64,
            3 => val.rotate_right(amt),
            _ => val,
        }
    } else {
        let v32 = val as u32;
        let r = match shift_type {
            0 => v32.wrapping_shl(amt),
            1 => v32.wrapping_shr(amt),
            2 => ((v32 as i32) >> amt) as u32,
            3 => v32.rotate_right(amt),
            _ => v32,
        };
        r as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_return42() {
        // From the return42 fixture disassembly:
        // 0000000100000328  sub sp, sp, #0x20
        let insn = decode(0xd10083ff, 0x100000328);
        assert!(matches!(insn, Instruction::SubImm { sf: true, rd: 31, rn: 31, imm: 0x20, shift: 0 }));

        // 000000010000032c  str wzr, [sp, #0x1c]
        let insn = decode(0xb9001fff, 0x10000032c);
        assert!(matches!(insn, Instruction::StrImm { sf: false, .. }));

        // 0000000100000338  mov w8, #0x28
        let insn = decode(0x52800508, 0x100000338);
        assert!(matches!(insn, Instruction::Movz { sf: false, rd: 8, imm16: 0x28, shift: 0 }));

        // 0000000100000344  add w8, w8, #0x2
        let insn = decode(0x11000908, 0x100000344);
        assert!(matches!(insn, Instruction::AddImm { sf: false, rd: 8, rn: 8, imm: 2, shift: 0 }));

        // 0000000100000354  ret
        let insn = decode(0xd65f03c0, 0x100000354);
        assert!(matches!(insn, Instruction::Ret { rn: 30 }));
    }

    #[test]
    fn test_decode_simple_cli() {
        // stp x29, x30, [sp, #0x10]
        let insn = decode(0xa9017bfd, 0x100000464);
        assert!(matches!(insn, Instruction::Stp { sf: true, rt: 29, rt2: 30, .. }));

        // adrp x0, 0
        let insn = decode(0x90000000, 0x100000478);
        assert!(matches!(insn, Instruction::Adrp { rd: 0, .. }));

        // bl 0x100000494
        let insn = decode(0x94000005, 0x100000480);
        assert!(matches!(insn, Instruction::Bl { .. }));

        // ldp x29, x30, [sp, #0x10]
        let insn = decode(0xa9417bfd, 0x100000488);
        assert!(matches!(insn, Instruction::Ldp { sf: true, rt: 29, rt2: 30, .. }));
    }

    #[test]
    fn test_sign_extend() {
        // 0x3FFFF = 19 bits all set except bit 18 → positive (max positive 19-bit)
        assert_eq!(sign_extend(0x3FFFF, 19), 0x3FFFF);
        // 0x40000 = bit 18 set → sign bit → negative
        assert_eq!(sign_extend(0x40000, 19), -0x40000_i64);
        // 0x7FFFF = all 19 bits set → -1
        assert_eq!(sign_extend(0x7FFFF, 19), -1);
    }
}
