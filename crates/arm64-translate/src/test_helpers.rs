//! Test helpers for arm64-translate unit tests.
//!
//! Provides a test interpreter builder and ARM64 instruction encoding helpers
//! so tests can construct instruction sequences and run them through the full
//! fetch → decode → execute pipeline.

use crate::cpu::CpuState;
use crate::ffi::FfiBridge;
use crate::interp::{ExitReason, Interpreter};
use crate::mem::Memory;

/// Sentinel LR value — when RET pops this, the interpreter returns cleanly.
const SENTINEL_LR: u64 = 0xDEAD_CAFE_FEED_F00D;

/// A test interpreter with code and data regions.
pub struct TestInterp {
    pub interp: Interpreter,
    /// Base address of the code region (first instruction).
    pub code_base: u64,
    /// Base address of a 4 KB writable data region for load/store tests.
    pub data_base: u64,
}

/// Create a test interpreter preloaded with the given instruction stream.
///
/// Layout: 4 KB code (instructions + trailing RET) followed by 4 KB data.
/// PC starts at `code_base`, SP at the mmap'd stack top, LR = sentinel.
///
/// # Example
/// ```ignore
/// let mut t = test_interp(&[enc_movz(true, 0, 42, 0)]);
/// let cpu = t.run_ok();
/// assert_eq!(cpu.x[0], 42);
/// ```
pub fn test_interp(instructions: &[u32]) -> TestInterp {
    let mut buf = vec![0u8; 8192]; // 4K code + 4K data
    for (i, &insn) in instructions.iter().enumerate() {
        let off = i * 4;
        buf[off..off + 4].copy_from_slice(&insn.to_le_bytes());
    }
    // Append RET x30 as safety net after the last instruction.
    let ret_off = instructions.len() * 4;
    if ret_off + 4 <= 4096 {
        buf[ret_off..ret_off + 4].copy_from_slice(&enc_ret(30).to_le_bytes());
    }

    let ptr = buf.as_mut_ptr();
    let base = ptr as u64;
    std::mem::forget(buf); // leak — acceptable for tests

    let segments = [(base, ptr, 8192usize, true)];
    let mem = unsafe { Memory::from_segments(&segments) };

    let mut cpu = CpuState::new();
    cpu.pc = base;
    cpu.sp = mem.initial_sp();
    cpu.x[30] = SENTINEL_LR;

    let ffi = FfiBridge::new();
    let interp = Interpreter::new(cpu, mem, ffi);

    TestInterp {
        interp,
        code_base: base,
        data_base: base + 4096,
    }
}

impl TestInterp {
    /// Run the interpreter and assert it returned cleanly.
    pub fn run_ok(&mut self) -> &CpuState {
        let result = self.interp.run();
        assert!(
            matches!(result, ExitReason::Returned { .. }),
            "Expected clean return, got: {result:?}"
        );
        &self.interp.cpu
    }

    /// Run and return the ExitReason (for tests that expect non-clean exits).
    pub fn run(&mut self) -> ExitReason {
        self.interp.run()
    }

    // ── Data region helpers ─────────────────────────────────────

    pub fn write_data_u8(&mut self, offset: u64, val: u8) {
        assert!(self.interp.mem.write_u8(self.data_base + offset, val));
    }

    pub fn write_data_u16(&mut self, offset: u64, val: u16) {
        assert!(self.interp.mem.write_u16(self.data_base + offset, val));
    }

    pub fn write_data_u32(&mut self, offset: u64, val: u32) {
        assert!(self.interp.mem.write_u32(self.data_base + offset, val));
    }

    pub fn write_data_u64(&mut self, offset: u64, val: u64) {
        assert!(self.interp.mem.write_u64(self.data_base + offset, val));
    }

    pub fn write_data_u128(&mut self, offset: u64, val: u128) {
        assert!(self.interp.mem.write_u128(self.data_base + offset, val));
    }

    pub fn read_data_u8(&self, offset: u64) -> u8 {
        self.interp.mem.read_u8(self.data_base + offset).unwrap()
    }

    pub fn read_data_u16(&self, offset: u64) -> u16 {
        self.interp.mem.read_u16(self.data_base + offset).unwrap()
    }

    pub fn read_data_u32(&self, offset: u64) -> u32 {
        self.interp.mem.read_u32(self.data_base + offset).unwrap()
    }

    pub fn read_data_u64(&self, offset: u64) -> u64 {
        self.interp.mem.read_u64(self.data_base + offset).unwrap()
    }

    pub fn read_data_u128(&self, offset: u64) -> u128 {
        self.interp.mem.read_u128(self.data_base + offset).unwrap()
    }
}

// ═══════════════════════════════════════════════════════════════════
// ARM64 instruction encoding helpers
// ═══════════════════════════════════════════════════════════════════

// ── Data processing (immediate) ──────────────────────────────────

/// ADD Xd|SP, Xn|SP, #imm12{, LSL #12}
pub fn enc_add_imm(sf: bool, rd: u8, rn: u8, imm12: u32, shift12: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b100010 << 23)
        | (u32::from(shift12) << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ADDS Xd, Xn|SP, #imm12 (sets flags; CMP when rd=31)
pub fn enc_adds_imm(sf: bool, rd: u8, rn: u8, imm12: u32, shift12: bool) -> u32 {
    enc_add_imm(sf, rd, rn, imm12, shift12) | (1 << 29)
}

/// SUB Xd|SP, Xn|SP, #imm12
pub fn enc_sub_imm(sf: bool, rd: u8, rn: u8, imm12: u32, shift12: bool) -> u32 {
    enc_add_imm(sf, rd, rn, imm12, shift12) | (1 << 30)
}

/// SUBS Xd, Xn|SP, #imm12 (sets flags; CMP when rd=31)
pub fn enc_subs_imm(sf: bool, rd: u8, rn: u8, imm12: u32, shift12: bool) -> u32 {
    enc_add_imm(sf, rd, rn, imm12, shift12) | (1 << 30) | (1 << 29)
}

/// MOVZ Xd, #imm16{, LSL #shift}  (hw = shift/16: 0,1,2,3)
pub fn enc_movz(sf: bool, rd: u8, imm16: u16, hw: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b10 << 29)
        | (0b100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

/// MOVK Xd, #imm16{, LSL #shift}
pub fn enc_movk(sf: bool, rd: u8, imm16: u16, hw: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b11 << 29)
        | (0b100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

/// MOVN Xd, #imm16{, LSL #shift}
pub fn enc_movn(sf: bool, rd: u8, imm16: u16, hw: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b00 << 29)
        | (0b100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

// ── Data processing (register) ───────────────────────────────────

/// ADD Xd, Xn, Xm{, shift #amount}
pub fn enc_add_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b01011 << 24)
        | ((shift_type as u32 & 0x3) << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ADDS Xd, Xn, Xm (flag-setting)
pub fn enc_adds_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    enc_add_reg(sf, rd, rn, rm, shift_type, amount) | (1 << 29)
}

/// SUB Xd, Xn, Xm
pub fn enc_sub_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    enc_add_reg(sf, rd, rn, rm, shift_type, amount) | (1 << 30)
}

/// SUBS Xd, Xn, Xm (flag-setting)
pub fn enc_subs_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    enc_add_reg(sf, rd, rn, rm, shift_type, amount) | (1 << 30) | (1 << 29)
}

/// MADD Xd, Xn, Xm, Xa  (Xd = Xa + Xn*Xm; MUL when Xa=XZR)
pub fn enc_madd(sf: bool, rd: u8, rn: u8, rm: u8, ra: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011011000 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0 << 15)  // o0=0 for MADD
        | ((ra as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// MSUB Xd, Xn, Xm, Xa  (Xd = Xa - Xn*Xm)
pub fn enc_msub(sf: bool, rd: u8, rn: u8, rm: u8, ra: u8) -> u32 {
    enc_madd(sf, rd, rn, rm, ra) | (1 << 15)
}

/// UDIV Xd, Xn, Xm
pub fn enc_udiv(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010110 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b000010 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SDIV Xd, Xn, Xm
pub fn enc_sdiv(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010110 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b000011 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ORR Xd, XZR, Xm → MOV Xd, Xm (register move)
pub fn enc_mov_reg(sf: bool, rd: u8, rm: u8) -> u32 {
    enc_orr_reg(sf, rd, 31, rm, 0, 0)
}

/// ORR (shifted register)
pub fn enc_orr_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b01 << 29)
        | (0b01010 << 24)
        | ((shift_type as u32 & 0x3) << 22)
        | (0 << 21) // N=0
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// AND (shifted register)
pub fn enc_and_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift_type: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b00 << 29)
        | (0b01010 << 24)
        | ((shift_type as u32 & 0x3) << 22)
        | (0 << 21) // N=0
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSEL Xd, Xn, Xm, cond
pub fn enc_csel(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | ((cond as u32 & 0xF) << 12)
        | (0b00 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSINC Xd, Xn, Xm, cond
pub fn enc_csinc(sf: bool, rd: u8, rn: u8, rm: u8, cond: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | ((cond as u32 & 0xF) << 12)
        | (0b01 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

// ── Branch ───────────────────────────────────────────────────────

/// B (unconditional, PC-relative). `offset` is already the absolute target
/// stored by the decoder — but the *encoding* uses a relative offset.
/// Pass the byte-offset from the instruction position.
pub fn enc_b(offset_bytes: i32) -> u32 {
    let imm26 = ((offset_bytes >> 2) as u32) & 0x03FF_FFFF;
    (0b000101 << 26) | imm26
}

/// BL (branch with link)
pub fn enc_bl(offset_bytes: i32) -> u32 {
    let imm26 = ((offset_bytes >> 2) as u32) & 0x03FF_FFFF;
    (0b100101 << 26) | imm26
}

/// BR Xn
pub fn enc_br(rn: u8) -> u32 {
    0xD61F_0000 | ((rn as u32 & 0x1F) << 5)
}

/// BLR Xn
pub fn enc_blr(rn: u8) -> u32 {
    0xD63F_0000 | ((rn as u32 & 0x1F) << 5)
}

/// RET {Xn}
pub fn enc_ret(rn: u8) -> u32 {
    0xD65F_0000 | ((rn as u32 & 0x1F) << 5)
}

/// CBZ Xt, offset
pub fn enc_cbz(sf: bool, rt: u8, offset_bytes: i32) -> u32 {
    let imm19 = ((offset_bytes >> 2) as u32) & 0x7_FFFF;
    (u32::from(sf) << 31) | (0b011010 << 25) | (0 << 24) | (imm19 << 5) | (rt as u32 & 0x1F)
}

/// CBNZ Xt, offset
pub fn enc_cbnz(sf: bool, rt: u8, offset_bytes: i32) -> u32 {
    enc_cbz(sf, rt, offset_bytes) | (1 << 24)
}

/// B.cond offset  (cond: 0=EQ, 1=NE, 2=CS, 3=CC, ... 14=AL)
pub fn enc_bcond(cond: u8, offset_bytes: i32) -> u32 {
    let imm19 = ((offset_bytes >> 2) as u32) & 0x7_FFFF;
    (0b01010100 << 24) | (imm19 << 5) | (cond as u32 & 0xF)
}

/// TBZ Xt, #bit, offset
pub fn enc_tbz(rt: u8, bit: u8, offset_bytes: i32) -> u32 {
    let b5 = (bit >> 5) & 1;
    let b40 = bit & 0x1F;
    let imm14 = ((offset_bytes >> 2) as u32) & 0x3FFF;
    ((b5 as u32) << 31)
        | (0b011011 << 25)
        | (0 << 24)
        | ((b40 as u32) << 19)
        | (imm14 << 5)
        | (rt as u32 & 0x1F)
}

/// TBNZ Xt, #bit, offset
pub fn enc_tbnz(rt: u8, bit: u8, offset_bytes: i32) -> u32 {
    enc_tbz(rt, bit, offset_bytes) | (1 << 24)
}

// ── Load/Store (unsigned offset) ─────────────────────────────────

/// LDR Xt, [Xn, #imm] (64-bit, unsigned byte offset, must be 8-aligned)
pub fn enc_ldr_imm64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 8;
    (0b11 << 30)
        | (0b111001 << 24)
        | (0b01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDR Wt, [Xn, #imm] (32-bit, unsigned byte offset, must be 4-aligned)
pub fn enc_ldr_imm32(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 4;
    (0b10 << 30)
        | (0b111001 << 24)
        | (0b01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STR Xt, [Xn, #imm] (64-bit)
pub fn enc_str_imm64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 8;
    (0b11 << 30)
        | (0b111001 << 24)
        | (0b00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STR Wt, [Xn, #imm] (32-bit)
pub fn enc_str_imm32(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 4;
    (0b10 << 30)
        | (0b111001 << 24)
        | (0b00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDRB Wt, [Xn, #imm] (byte load)
pub fn enc_ldrb_imm(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    (0b00 << 30)
        | (0b111001 << 24)
        | (0b01 << 22)
        | ((byte_offset & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STRB Wt, [Xn, #imm] (byte store)
pub fn enc_strb_imm(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    (0b00 << 30)
        | (0b111001 << 24)
        | (0b00 << 22)
        | ((byte_offset & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDRH Wt, [Xn, #imm] (halfword load, byte_offset must be 2-aligned)
pub fn enc_ldrh_imm(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 2;
    (0b01 << 30)
        | (0b111001 << 24)
        | (0b01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STRH Wt, [Xn, #imm]
pub fn enc_strh_imm(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 2;
    (0b01 << 30)
        | (0b111001 << 24)
        | (0b00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STP Xt1, Xt2, [Xn, #imm] (64-bit pair, signed offset, imm must be 8-aligned)
pub fn enc_stp64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
    let imm7 = ((byte_offset / 8) as u32) & 0x7F;
    (0b10 << 30)
        | (0b101 << 27)
        | (0b0 << 26)
        | (0b010 << 23)
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDP Xt1, Xt2, [Xn, #imm] (64-bit pair)
pub fn enc_ldp64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
    enc_stp64(rt, rt2, rn, byte_offset) | (1 << 22)
}

/// STP Wt1, Wt2, [Xn, #imm] (32-bit pair, imm must be 4-aligned)
pub fn enc_stp32(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
    let imm7 = ((byte_offset / 4) as u32) & 0x7F;
    (0b00 << 30)
        | (0b101 << 27)
        | (0b0 << 26)
        | (0b010 << 23)
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDP Wt1, Wt2, [Xn, #imm] (32-bit pair)
pub fn enc_ldp32(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
    enc_stp32(rt, rt2, rn, byte_offset) | (1 << 22)
}

// ── System / Misc ────────────────────────────────────────────────

/// NOP
pub fn enc_nop() -> u32 {
    0xD503_201F
}

/// SVC #imm16
pub fn enc_svc(imm: u16) -> u32 {
    0xD400_0001 | ((imm as u32) << 5)
}

// ── Bitfield ─────────────────────────────────────────────────────

/// UBFM Xd, Xn, #immr, #imms  (opc=10)
/// Aliases: UXTB (immr=0,imms=7), UXTH (immr=0,imms=15), LSR (imms=63/31), LSL, UBFX
pub fn enc_ubfm(sf: bool, rd: u8, rn: u8, immr: u8, imms: u8) -> u32 {
    let n = u32::from(sf); // N must equal sf for valid encoding
    (u32::from(sf) << 31)
        | (0b10 << 29)
        | (0b100110 << 23)
        | (n << 22)
        | ((immr as u32 & 0x3F) << 16)
        | ((imms as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SBFM Xd, Xn, #immr, #imms  (opc=00)
/// Aliases: SXTW (immr=0,imms=31), SXTB (immr=0,imms=7), SXTH, ASR, SBFX
pub fn enc_sbfm(sf: bool, rd: u8, rn: u8, immr: u8, imms: u8) -> u32 {
    let n = u32::from(sf);
    (u32::from(sf) << 31)
        | (0b00 << 29)
        | (0b100110 << 23)
        | (n << 22)
        | ((immr as u32 & 0x3F) << 16)
        | ((imms as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

// ── FP / SIMD ────────────────────────────────────────────────────

/// FMOV Dd, Dn (sf=true → double, sf=false → single)
pub fn enc_fmov_reg(sf: bool, rd: u8, rn: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b10000_00_10000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FADD Dd, Dn, Dm / FADD Ss, Sn, Sm
pub fn enc_fadd(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b001010 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FSUB Dd, Dn, Dm
pub fn enc_fsub(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b001110 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FMUL Dd, Dn, Dm
pub fn enc_fmul(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b000010 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FDIV Dd, Dn, Dm
pub fn enc_fdiv(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b000110 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FABD Dd, Dn, Dm / FABD Sd, Sn, Sm (Advanced SIMD scalar absolute difference)
pub fn enc_fabd(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    ((if sf { 0x7E } else { 0x5E }) << 24)
        | (1 << 23)
        | (u32::from(sf) << 22)
        | (1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b110101 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FCMP Dn, Dm / FCMP Sn, Sm  (sets NZCV)
pub fn enc_fcmp(sf: bool, rn: u8, rm: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b1 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b001000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (0b00000)
}

/// FNEG Dd, Dn (opcode=000010)
pub fn enc_fneg(sf: bool, rd: u8, rn: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b10000_10_10000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FABS Dd, Dn (opcode=000001)
pub fn enc_fabs(sf: bool, rd: u8, rn: u8) -> u32 {
    let ftype = if sf { 0b01u32 } else { 0b00 };
    (0b00011110 << 24)
        | (ftype << 22)
        | (0b10000_01_10000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SCVTF Dd, Xn (signed int to double)  sf_fp=true→double, sf_int=true→64-bit int
pub fn enc_scvtf(sf_fp: bool, sf_int: bool, rd: u8, rn: u8) -> u32 {
    let ftype = if sf_fp { 0b01u32 } else { 0b00 };
    (u32::from(sf_int) << 31)
        | (0b0011110 << 24)
        | (ftype << 22)
        | (0b100010 << 16)
        | (0b000000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FCVTZS Xd, Dn (double to signed int, round toward zero)
pub fn enc_fcvtzs(sf_int: bool, sf_fp: bool, rd: u8, rn: u8) -> u32 {
    let ftype = if sf_fp { 0b01u32 } else { 0b00 };
    (u32::from(sf_int) << 31)
        | (0b0011110 << 24)
        | (ftype << 22)
        | (0b111000 << 16)
        | (0b000000 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// FMOV Xd, Dn (FP to GPR, double)
pub fn enc_fmov_to_gpr64(rd: u8, rn: u8) -> u32 {
    0x9E660000 | ((rn as u32 & 0x1F) << 5) | (rd as u32 & 0x1F)
}

/// FMOV Dn, Xn (GPR to FP, double)
pub fn enc_fmov_to_fp64(rd: u8, rn: u8) -> u32 {
    0x9E670000 | ((rn as u32 & 0x1F) << 5) | (rd as u32 & 0x1F)
}

// ── FP load/store ────────────────────────────────────────────────

/// LDR Dt, [Xn, #imm] (64-bit FP unsigned offset, imm must be 8-aligned)
pub fn enc_fp_ldr_imm64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 8;
    (0b11 << 30)
        | (0b111101 << 24)
        | (0b01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STR Dt, [Xn, #imm] (64-bit FP unsigned offset)
pub fn enc_fp_str_imm64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 8;
    (0b11 << 30)
        | (0b111101 << 24)
        | (0b00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDR St, [Xn, #imm] (32-bit FP unsigned offset, imm must be 4-aligned)
pub fn enc_fp_ldr_imm32(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 4;
    (0b10 << 30)
        | (0b111101 << 24)
        | (0b01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STR St, [Xn, #imm] (32-bit FP unsigned offset)
pub fn enc_fp_str_imm32(rt: u8, rn: u8, byte_offset: u32) -> u32 {
    let imm12 = byte_offset / 4;
    (0b10 << 30)
        | (0b111101 << 24)
        | (0b00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}
