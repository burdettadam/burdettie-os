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

mod branch;
mod dp_imm;
mod dp_reg;
mod fp_dp;
mod load_store;
pub(crate) mod util;

use crate::cpu::Condition;

pub use util::apply_shift;

/// A decoded ARM64 instruction.
///
/// All fields are primitive types or `Condition` (also `Copy`), so this is
/// `Copy` — block-cache slots can be iterated without cloning.
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    // ── Data processing (immediate) ───────────────────────────────
    /// ADD Xd|SP, Xn|SP, #imm{, shift}
    AddImm {
        sf: bool,
        rd: u8,
        rn: u8,
        imm: u32,
        shift: u8,
    },
    /// ADDS Xd, Xn|SP, #imm{, shift}  (sets flags; CMP when Rd=XZR)
    AddsImm {
        sf: bool,
        rd: u8,
        rn: u8,
        imm: u32,
        shift: u8,
    },
    /// SUB Xd|SP, Xn|SP, #imm{, shift}
    SubImm {
        sf: bool,
        rd: u8,
        rn: u8,
        imm: u32,
        shift: u8,
    },
    /// SUBS Xd, Xn|SP, #imm{, shift}  (sets flags; CMP when Rd=XZR)
    SubsImm {
        sf: bool,
        rd: u8,
        rn: u8,
        imm: u32,
        shift: u8,
    },

    /// MOVZ Xd, #imm16{, LSL #shift}  (move wide with zero)
    Movz {
        sf: bool,
        rd: u8,
        imm16: u16,
        shift: u8,
    },
    /// MOVK Xd, #imm16{, LSL #shift}  (move wide with keep)
    Movk {
        sf: bool,
        rd: u8,
        imm16: u16,
        shift: u8,
    },
    /// MOVN Xd, #imm16{, LSL #shift}  (move wide with NOT)
    Movn {
        sf: bool,
        rd: u8,
        imm16: u16,
        shift: u8,
    },

    /// ADRP Xd, label  (page-relative address)
    Adrp { rd: u8, imm: i64 },
    /// ADR Xd, label  (PC-relative address)
    Adr { rd: u8, imm: i64 },

    /// Logical immediate: AND, ORR, EOR, ANDS
    LogicalImm {
        sf: bool,
        opc: u8,
        rd: u8,
        rn: u8,
        imm: u64,
    },

    // ── Data processing (register) ────────────────────────────────
    /// ADD Xd, Xn, Xm{, shift #amount}
    AddReg {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
        shift_amount: u8,
    },
    /// ADDS Xd, Xn, Xm{, shift #amount}
    AddsReg {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
        shift_amount: u8,
    },
    /// SUB Xd, Xn, Xm{, shift #amount}
    SubReg {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
        shift_amount: u8,
    },
    /// SUBS Xd, Xn, Xm{, shift #amount}
    SubsReg {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
        shift_amount: u8,
    },

    /// Logical (shifted register): AND, BIC, ORR, ORN, EOR, EON, ANDS, BICS
    LogicalReg {
        sf: bool,
        opc: u8,
        n: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
        shift_amount: u8,
    },

    /// MADD Xd, Xn, Xm, Xa  (Xd = Xa + Xn * Xm)  — MUL is MADD with Xa=XZR
    Madd {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },
    /// MSUB Xd, Xn, Xm, Xa  (Xd = Xa - Xn * Xm)
    Msub {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },

    /// SDIV / UDIV
    Div {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        is_signed: bool,
    },

    /// CSEL Xd, Xn, Xm, cond
    Csel {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
    },
    /// CSINC Xd, Xn, Xm, cond  (CSET is alias with Xn=Xm=XZR)
    Csinc {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
    },
    /// CSINV Xd, Xn, Xm, cond  (CSETM is alias)
    Csinv {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
    },
    /// CSNEG Xd, Xn, Xm, cond  (CNEG is alias)
    Csneg {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
    },

    /// CCMP Xn, #imm5, #nzcv, cond
    Ccmp {
        sf: bool,
        rn: u8,
        imm5_or_rm: u8,
        nzcv: u8,
        cond: Condition,
        is_imm: bool,
    },
    /// CCMN Xn, #imm5, #nzcv, cond
    Ccmn {
        sf: bool,
        rn: u8,
        imm5_or_rm: u8,
        nzcv: u8,
        cond: Condition,
        is_imm: bool,
    },

    /// Variable shift: LSLV, LSRV, ASRV, RORV
    ShiftVar {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        shift_type: u8,
    },

    /// Bitfield: SBFM, BFM, UBFM (encodes aliases SXTW, SXTB, SXTH, UXTB, UXTH, LSL, LSR, ASR, UBFX, SBFX)
    Bitfield {
        sf: bool,
        opc: u8,
        rd: u8,
        rn: u8,
        immr: u8,
        imms: u8,
    },

    // ── Load/Store ────────────────────────────────────────────────
    /// LDR Xt, [Xn, #imm]  (unsigned offset)
    LdrImm { sf: bool, rt: u8, rn: u8, imm: u32 },
    /// STR Xt, [Xn, #imm]  (unsigned offset)
    StrImm { sf: bool, rt: u8, rn: u8, imm: u32 },

    /// LDUR Xt, [Xn, #simm9]  (unscaled)
    Ldur { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STUR Xt, [Xn, #simm9]  (unscaled)
    Stur { sf: bool, rt: u8, rn: u8, imm: i16 },

    /// LDR Xt, [Xn, #simm]!  (pre-index, 32/64-bit)
    LdrPre { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STR Xt, [Xn, #simm]!  (pre-index, 32/64-bit)
    StrPre { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRB Wt, [Xn, #simm]!  (pre-index byte)
    LdrbPre { rt: u8, rn: u8, imm: i16 },
    /// STRB Wt, [Xn, #simm]!  (pre-index byte)
    StrbPre { rt: u8, rn: u8, imm: i16 },
    /// LDRH Wt, [Xn, #simm]!  (pre-index halfword)
    LdrhPre { rt: u8, rn: u8, imm: i16 },
    /// STRH Wt, [Xn, #simm]!  (pre-index halfword)
    StrhPre { rt: u8, rn: u8, imm: i16 },
    /// LDRSB Xt/Wt, [Xn, #simm]!  (pre-index signed byte)
    LdrsbPre { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRSH Xt/Wt, [Xn, #simm]!  (pre-index signed halfword)
    LdrshPre { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRSW Xt, [Xn, #simm]!  (pre-index signed word)
    LdrswPre { rt: u8, rn: u8, imm: i16 },

    /// LDR Xt, [Xn], #simm  (post-index, 32/64-bit)
    LdrPost { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// STR Xt, [Xn], #simm  (post-index, 32/64-bit)
    StrPost { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRB Wt, [Xn], #simm  (post-index byte)
    LdrbPost { rt: u8, rn: u8, imm: i16 },
    /// STRB Wt, [Xn], #simm  (post-index byte)
    StrbPost { rt: u8, rn: u8, imm: i16 },
    /// LDRH Wt, [Xn], #simm  (post-index halfword)
    LdrhPost { rt: u8, rn: u8, imm: i16 },
    /// STRH Wt, [Xn], #simm  (post-index halfword)
    StrhPost { rt: u8, rn: u8, imm: i16 },
    /// LDRSB Xt/Wt, [Xn], #simm  (post-index signed byte)
    LdrsbPost { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRSH Xt/Wt, [Xn], #simm  (post-index signed halfword)
    LdrshPost { sf: bool, rt: u8, rn: u8, imm: i16 },
    /// LDRSW Xt, [Xn], #simm  (post-index signed word)
    LdrswPost { rt: u8, rn: u8, imm: i16 },

    /// LDP Xt1, Xt2, [Xn, #imm]
    Ldp {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP Xt1, Xt2, [Xn, #imm]
    Stp {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDP Xt1, Xt2, [Xn, #imm]!  (pre-index)
    LdpPre {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP Xt1, Xt2, [Xn, #imm]!  (pre-index)
    StpPre {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDP Xt1, Xt2, [Xn], #imm  (post-index)
    LdpPost {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP Xt1, Xt2, [Xn], #imm  (post-index)
    StpPost {
        sf: bool,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDRB Wt, [Xn, #imm]  (byte load, zero-extend)
    Ldrb { rt: u8, rn: u8, imm: i32 },
    /// STRB Wt, [Xn, #imm]  (byte store)
    Strb { rt: u8, rn: u8, imm: i32 },
    /// LDRH Wt, [Xn, #imm]  (halfword load, zero-extend)
    Ldrh { rt: u8, rn: u8, imm: i32 },
    /// STRH Wt, [Xn, #imm]  (halfword store)
    Strh { rt: u8, rn: u8, imm: i32 },

    /// LDRSW Xt, [Xn, #imm]  (signed word load, sign-extend to 64)
    Ldrsw { rt: u8, rn: u8, imm: i32 },

    /// LDRSB Xt/Wt, [Xn, #imm]  (signed byte load)
    Ldrsb { rt: u8, rn: u8, imm: i32, sf: bool },
    /// LDRSH Xt/Wt, [Xn, #imm]  (signed halfword load)
    Ldrsh { rt: u8, rn: u8, imm: i32, sf: bool },

    /// LDR/STR Xt, [Xn, Xm{, extend/shift}]  (register-indexed)
    LdrReg {
        sf: bool,
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    StrReg {
        sf: bool,
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    /// LDRB/STRB Wt, [Xn, Xm{, extend}]  (register-indexed byte)
    LdrbReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    StrbReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    /// LDRH/STRH Wt, [Xn, Xm{, extend}]  (register-indexed halfword)
    LdrhReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    StrhReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    /// LDRSB Xt/Wt, [Xn, Xm{, extend}]  (register-indexed signed byte)
    LdrsbReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
        sf: bool,
    },
    /// LDRSH Xt/Wt, [Xn, Xm{, extend}]  (register-indexed signed half)
    LdrshReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
        sf: bool,
    },
    /// LDRSW Xt, [Xn, Xm{, extend}]  (register-indexed signed word)
    LdrswReg {
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },

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
    /// MRS Xt, <sysreg>  (move from system register to GPR)
    Mrs { rt: u8, sysreg: u16 },
    /// MSR <sysreg>, Xt  (move from GPR to system register)
    Msr { rt: u8, sysreg: u16 },

    // ── Acquire/Release load/store ────────────────────────────────
    /// LDAR Wt/Xt, [Xn]  (load-acquire)
    Ldar { sf: bool, rt: u8, rn: u8 },
    /// LDAPR Wt/Xt, [Xn]  (load-acquire RCpc — treated as LDAR)
    Ldapr { sf: bool, rt: u8, rn: u8 },
    /// STLR Wt/Xt, [Xn]  (store-release)
    Stlr { sf: bool, rt: u8, rn: u8 },
    /// LDARB Wt, [Xn]  (load-acquire byte)
    Ldarb { rt: u8, rn: u8 },
    /// STLRB Wt, [Xn]  (store-release byte)
    Stlrb { rt: u8, rn: u8 },
    /// LDARH Wt, [Xn]  (load-acquire halfword)
    Ldarh { rt: u8, rn: u8 },
    /// STLRH Wt, [Xn]  (store-release halfword)
    Stlrh { rt: u8, rn: u8 },

    // ── Exclusive load/store ──────────────────────────────────────
    /// LDXR/LDAXR Wt/Xt, [Xn]  (load exclusive; acquire variant)
    LoadExclusive {
        sf: bool,
        acquire: bool,
        rt: u8,
        rn: u8,
    },
    /// LDXRB/LDAXRB Wt, [Xn]  (load exclusive byte)
    LoadExclusiveByte { acquire: bool, rt: u8, rn: u8 },
    /// LDXRH/LDAXRH Wt, [Xn]  (load exclusive halfword)
    LoadExclusiveHalf { acquire: bool, rt: u8, rn: u8 },
    /// STXR/STLXR Ws, Wt/Xt, [Xn]  (store exclusive; release variant)
    StoreExclusive {
        sf: bool,
        release: bool,
        rs: u8,
        rt: u8,
        rn: u8,
    },
    /// STXRB/STLXRB Ws, Wt, [Xn]  (store exclusive byte)
    StoreExclusiveByte {
        release: bool,
        rs: u8,
        rt: u8,
        rn: u8,
    },
    /// STXRH/STLXRH Ws, Wt, [Xn]  (store exclusive halfword)
    StoreExclusiveHalf {
        release: bool,
        rs: u8,
        rt: u8,
        rn: u8,
    },
    /// CLREX  (clear exclusive monitor)
    Clrex,

    // ── LSE atomics (ARMv8.1) ─────────────────────────────────────
    /// LDADD/LDCLR/LDSET/LDEOR/LDMAX/LDMIN/LDUMAX/LDUMIN (with A/AL/L variants).
    /// `op`: 0=LDADD, 1=LDCLR, 2=LDEOR, 3=LDSET, 4=LDSMAX, 5=LDSMIN, 6=LDUMAX, 7=LDUMIN
    LseLoad {
        size: u8,
        acquire: bool,
        release: bool,
        op: u8,
        rs: u8,
        rt: u8,
        rn: u8,
    },
    /// SWP/SWPA/SWPAL/SWPL  (atomic swap)
    Swp {
        size: u8,
        acquire: bool,
        release: bool,
        rs: u8,
        rt: u8,
        rn: u8,
    },
    /// CAS/CASA/CASAL/CASL  (compare-and-swap)
    Cas {
        sf: bool,
        acquire: bool,
        release: bool,
        rs: u8,
        rt: u8,
        rn: u8,
    },

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
    FpLdp {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP St1, St2, [Xn, #imm] / STP Dt1, Dt2, [Xn, #imm]
    FpStp {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDP with pre-index
    FpLdpPre {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP with pre-index
    FpStpPre {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDP with post-index
    FpLdpPost {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },
    /// STP with post-index
    FpStpPost {
        size: u8,
        rt: u8,
        rt2: u8,
        rn: u8,
        imm: i16,
    },

    /// LDR St/Dt/Qt, [Xn, Xm{, extend/shift}]  (FP register-indexed)
    FpLdrReg {
        size: u8,
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },
    /// STR St/Dt/Qt, [Xn, Xm{, extend/shift}]  (FP register-indexed)
    FpStrReg {
        size: u8,
        rt: u8,
        rn: u8,
        rm: u8,
        extend: u8,
        s: bool,
    },

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
    Scvtf {
        sf_fp: bool,
        sf_int: bool,
        rd: u8,
        rn: u8,
    },
    /// UCVTF Dd/Sd, Xn/Wn (unsigned int to float)
    Ucvtf {
        sf_fp: bool,
        sf_int: bool,
        rd: u8,
        rn: u8,
    },
    /// FCVTZS Xd/Wd, Dn/Sn (float to signed int, round toward zero)
    Fcvtzs {
        sf_int: bool,
        sf_fp: bool,
        rd: u8,
        rn: u8,
    },
    /// FCVTZU Xd/Wd, Dn/Sn (float to unsigned int, round toward zero)
    Fcvtzu {
        sf_int: bool,
        sf_fp: bool,
        rd: u8,
        rn: u8,
    },

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
    /// FABD Dd, Dn, Dm (Advanced SIMD scalar absolute difference)
    Fabd { sf: bool, rd: u8, rn: u8, rm: u8 },
    /// FNEG Dd, Dn
    Fneg { sf: bool, rd: u8, rn: u8 },
    /// FABS Dd, Dn
    Fabs { sf: bool, rd: u8, rn: u8 },
    /// FSQRT Dd, Dn
    Fsqrt { sf: bool, rd: u8, rn: u8 },

    /// FCSEL Dd, Dn, Dm, cond
    Fcsel {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        cond: Condition,
    },

    /// FMADD Dd, Dn, Dm, Da  (Dd = Da + Dn * Dm)
    Fmadd {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },
    /// FMSUB Dd, Dn, Dm, Da  (Dd = Da - Dn * Dm)
    Fmsub {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },
    /// FNMADD Dd, Dn, Dm, Da  (Dd = -(Da + Dn * Dm))
    Fnmadd {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },
    /// FNMSUB Dd, Dn, Dm, Da  (Dd = -(Da - Dn * Dm))
    Fnmsub {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        ra: u8,
    },

    /// FRINTM (round toward minus infinity), FRINTP (toward +inf),
    /// FRINTZ (toward zero), FRINTA (round away from zero / ties-away),
    /// FRINTN (round to nearest, ties-even), FRINTI (current rounding), FRINTX (exact, raise inexact)
    /// `mode`: 0=N, 1=P, 2=M, 3=Z, 4=A, 6=X, 7=I
    Frint { sf: bool, rd: u8, rn: u8, mode: u8 },

    // ── Data processing (misc) ────────────────────────────────────
    /// REV Xd, Xn / REV Wd, Wn  (byte-reverse full register)
    Rev { sf: bool, rd: u8, rn: u8 },
    /// REV16 Xd, Xn / REV16 Wd, Wn  (byte-reverse each halfword)
    Rev16 { sf: bool, rd: u8, rn: u8 },
    /// REV32 Xd, Xn  (byte-reverse each 32-bit word in 64-bit)
    Rev32 { rd: u8, rn: u8 },
    /// CLZ Xd, Xn / CLZ Wd, Wn  (count leading zeros)
    Clz { sf: bool, rd: u8, rn: u8 },
    /// CLS Xd, Xn / CLS Wd, Wn  (count leading sign bits)
    Cls { sf: bool, rd: u8, rn: u8 },
    /// RBIT Xd, Xn / RBIT Wd, Wn  (reverse bits)
    Rbit { sf: bool, rd: u8, rn: u8 },

    /// EXTR Xd, Xn, Xm, #lsb  (extract bits from pair of registers)
    Extr {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        lsb: u8,
    },

    /// PRFM (prefetch memory) — treated as NOP
    Prfm,

    /// BRK #imm16  (breakpoint)
    Brk { imm: u16 },

    // ── SIMD scalar / vector (basic) ──────────────────────────────
    /// DUP Vd.<T>, Xn  (broadcast GPR to all SIMD lanes)
    SimdDupGeneral { q: bool, size: u8, rd: u8, rn: u8 },
    /// DUP Vd.<T>, Vn.<Ts>[index]  (broadcast element to all lanes)
    SimdDupElement { q: bool, imm5: u8, rd: u8, rn: u8 },
    /// INS Vd.<Ts>[index], Xn  (insert GPR into SIMD lane)
    SimdInsGeneral { imm5: u8, rd: u8, rn: u8 },
    /// INS Vd.<Ts>[dst], Vn.<Ts>[src]  (insert element from another vector)
    SimdInsElement { imm5: u8, imm4: u8, rd: u8, rn: u8 },
    /// UMOV Xd, Vn.<Ts>[index]  (unsigned move from SIMD lane to GPR)
    SimdUmov { q: bool, imm5: u8, rd: u8, rn: u8 },
    /// SMOV Xd, Vn.<Ts>[index]  (signed move from SIMD lane to GPR)
    SimdSmov { q: bool, imm5: u8, rd: u8, rn: u8 },

    /// MOVI / MVNI (SIMD modified immediate)
    SimdMovi { q: bool, rd: u8, imm: u64 },

    /// ADD Vd.<T>, Vn.<T>, Vm.<T>  (SIMD vector add)
    SimdAdd {
        q: bool,
        size: u8,
        rd: u8,
        rn: u8,
        rm: u8,
    },
    /// SUB Vd.<T>, Vn.<T>, Vm.<T>  (SIMD vector subtract)
    SimdSub {
        q: bool,
        size: u8,
        rd: u8,
        rn: u8,
        rm: u8,
    },
    /// AND Vd.16B, Vn.16B, Vm.16B  (bitwise AND)
    SimdAnd { q: bool, rd: u8, rn: u8, rm: u8 },
    /// ORR Vd.16B, Vn.16B, Vm.16B  (bitwise OR)
    SimdOrr { q: bool, rd: u8, rn: u8, rm: u8 },
    /// EOR Vd.16B, Vn.16B, Vm.16B  (bitwise XOR)
    SimdEor { q: bool, rd: u8, rn: u8, rm: u8 },

    /// CMEQ / CMGT / CMGE / CMHI / CMHS (SIMD vector compare)
    /// `op`: encodes the comparison type
    SimdCmp {
        q: bool,
        size: u8,
        rd: u8,
        rn: u8,
        rm: u8,
        op: u8,
    },

    /// CNT Vd.8B/16B, Vn.8B/16B  (population count per byte)
    SimdCnt { q: bool, rd: u8, rn: u8 },

    /// ADDV Sd, Vn.<T>  (add across vector)
    SimdAddv { q: bool, size: u8, rd: u8, rn: u8 },

    // ── CRC32 ─────────────────────────────────────────────────────
    /// CRC32B/CRC32H/CRC32W/CRC32X / CRC32CB/CRC32CH/CRC32CW/CRC32CX
    Crc32 {
        sf: bool,
        rd: u8,
        rn: u8,
        rm: u8,
        sz: u8,
        c: bool,
    },

    // ── Pointer Authentication (ARMv8.3 arm64e) ──────────────────
    /// PACIA Xd, Xn / PACIZA Xd — sign pointer with IA key.
    PacIA { rd: u8, rn: u8 },
    /// PACIB Xd, Xn / PACIZB Xd — sign pointer with IB key.
    PacIB { rd: u8, rn: u8 },
    /// PACDA Xd, Xn / PACDZA Xd — sign data pointer with DA key.
    PacDA { rd: u8, rn: u8 },
    /// PACDB Xd, Xn / PACDZB Xd — sign data pointer with DB key.
    PacDB { rd: u8, rn: u8 },
    /// AUTIA Xd, Xn / AUTIZA Xd — authenticate pointer with IA key.
    AutIA { rd: u8, rn: u8 },
    /// AUTIB Xd, Xn / AUTIZB Xd — authenticate pointer with IB key.
    AutIB { rd: u8, rn: u8 },
    /// AUTDA Xd, Xn / AUTDZA Xd — authenticate data pointer with DA key.
    AutDA { rd: u8, rn: u8 },
    /// AUTDB Xd, Xn / AUTDZB Xd — authenticate data pointer with DB key.
    AutDB { rd: u8, rn: u8 },
    /// XPACI Xd — strip PAC bits from instruction pointer.
    Xpaci { rd: u8 },
    /// XPACD Xd — strip PAC bits from data pointer.
    Xpacd { rd: u8 },

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
        0b1000 | 0b1001 => dp_imm::decode_dp_imm(raw, pc),
        // x101 — Data processing (register)
        0b0101 | 0b1101 => dp_reg::decode_dp_reg(raw),
        // x1x0 — Loads and stores
        0b0100 | 0b0110 | 0b1100 | 0b1110 => load_store::decode_load_store(raw, pc),
        // x111 — Floating-point / SIMD data processing
        0b0111 | 0b1111 => fp_dp::decode_fp_dp(raw),
        // 101x — Branch, exception, system
        0b1010 | 0b1011 => branch::decode_branch(raw, pc),
        _ => {
            // Check for branch patterns more precisely
            // Bits [31:26] = 00010x → B.cond / SVC / etc.
            let top6 = raw >> 26;
            if top6 == 0b000101 || top6 == 0b100101 {
                // B / BL
                return branch::decode_branch(raw, pc);
            }
            Instruction::Unknown { raw }
        }
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
        assert!(matches!(
            insn,
            Instruction::SubImm {
                sf: true,
                rd: 31,
                rn: 31,
                imm: 0x20,
                shift: 0
            }
        ));

        // 000000010000032c  str wzr, [sp, #0x1c]
        let insn = decode(0xb9001fff, 0x10000032c);
        assert!(matches!(insn, Instruction::StrImm { sf: false, .. }));

        // 0000000100000338  mov w8, #0x28
        let insn = decode(0x52800508, 0x100000338);
        assert!(matches!(
            insn,
            Instruction::Movz {
                sf: false,
                rd: 8,
                imm16: 0x28,
                shift: 0
            }
        ));

        // 0000000100000344  add w8, w8, #0x2
        let insn = decode(0x11000908, 0x100000344);
        assert!(matches!(
            insn,
            Instruction::AddImm {
                sf: false,
                rd: 8,
                rn: 8,
                imm: 2,
                shift: 0
            }
        ));

        // 0000000100000354  ret
        let insn = decode(0xd65f03c0, 0x100000354);
        assert!(matches!(insn, Instruction::Ret { rn: 30 }));
    }

    #[test]
    fn test_decode_simple_cli() {
        // stp x29, x30, [sp, #0x10]
        let insn = decode(0xa9017bfd, 0x100000464);
        assert!(matches!(
            insn,
            Instruction::Stp {
                sf: true,
                rt: 29,
                rt2: 30,
                ..
            }
        ));

        // adrp x0, 0
        let insn = decode(0x90000000, 0x100000478);
        assert!(matches!(insn, Instruction::Adrp { rd: 0, .. }));

        // bl 0x100000494
        let insn = decode(0x94000005, 0x100000480);
        assert!(matches!(insn, Instruction::Bl { .. }));

        // ldp x29, x30, [sp, #0x10]
        let insn = decode(0xa9417bfd, 0x100000488);
        assert!(matches!(
            insn,
            Instruction::Ldp {
                sf: true,
                rt: 29,
                rt2: 30,
                ..
            }
        ));
    }
}
