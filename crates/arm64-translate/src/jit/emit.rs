//! Native aarch64 code emitter for JIT-compiled blocks.
//!
//! Each block is compiled to a function with signature:
//!   `unsafe fn(cpu: *mut CpuState, mem: *mut Memory) -> u64`
//!
//! Calling convention:
//! - x0 = pointer to CpuState
//! - x1 = pointer to Memory
//! - Return value in x0 = number of instructions executed
//!
//! Strategy: map guest registers to host registers where possible,
//! emit native arithmetic/logic directly, and bail out for complex
//! instructions (load/store, branches, system).
//!
//! Registers used:
//! - x19 = CpuState pointer (callee-saved)
//! - x20 = instruction counter (callee-saved)
//! - x21 = Memory pointer (callee-saved)
//! - x0-x15 = scratch / guest register cache
//!
//! For simplicity, this initial JIT implementation uses a "load-exec-store"
//! approach: for each instruction, load guest regs from CpuState, perform the
//! operation with native instructions, store results back. This avoids complex
//! register allocation while still being 5-10x faster than interpretation.

use super::CodeBuffer;
use crate::decode::Instruction;

/// Offset of `x[n]` (n=0..30) in CpuState.
/// CpuState layout: x: [u64; 31], sp: u64, pc: u64, nzcv: Nzcv(4 bools), ...
/// x[n] is at offset n*8.
const fn gpr_offset(n: u8) -> u32 {
    (n as u32) * 8
}

/// Offset of `sp` in CpuState (after x[31]).
const _SP_OFFSET: u32 = 31 * 8;

/// Offset of `pc` in CpuState.
const _PC_OFFSET: u32 = 32 * 8;

/// Emit a compiled block. Returns `Some(())` on success, `None` if the block
/// contains instructions we can't JIT.
#[cfg(target_arch = "aarch64")]
pub(super) fn emit_block(code: &mut CodeBuffer, block: &[Instruction]) -> Option<()> {
    // Prologue: save callee-saved registers, set up CpuState/Memory pointers.
    //   stp x29, x30, [sp, #-48]!
    //   stp x19, x20, [sp, #16]
    //   stp x21, x22, [sp, #32]
    //   mov x19, x0          // x19 = cpu
    //   mov x20, xzr         // x20 = insn_count = 0
    //   mov x21, x1          // x21 = mem
    code.emit32(enc_stp_pre(29, 30, 31, -48));
    code.emit32(enc_stp_offset(19, 20, 31, 16));
    code.emit32(enc_stp_offset(21, 22, 31, 32));
    code.emit32(enc_mov_reg(19, 0));       // mov x19, x0
    code.emit32(enc_movz(20, 0, 0));       // mov x20, #0
    code.emit32(enc_mov_reg(21, 1));       // mov x21, x1

    let mut insn_count = 0u32;

    for insn in block {
        insn_count += 1;

        match *insn {
            Instruction::Nop => {
                // Just increment counter, no code needed.
            }

            Instruction::AddImm { sf, rd, rn, imm, shift } => {
                let effective_imm = if shift == 1 { imm << 12 } else { imm };
                // Load Rn from CpuState
                emit_load_guest_gpr(code, 0, rn, sf);
                // ADD x0/w0, x0/w0, #imm
                if effective_imm <= 0xFFF {
                    code.emit32(enc_add_imm(sf, 0, 0, effective_imm as u16, false));
                } else if effective_imm <= 0xFFF_000 && effective_imm & 0xFFF == 0 {
                    code.emit32(enc_add_imm(sf, 0, 0, (effective_imm >> 12) as u16, true));
                } else {
                    return None; // Can't encode
                }
                // Store result
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::SubImm { sf, rd, rn, imm, shift } => {
                let effective_imm = if shift == 1 { imm << 12 } else { imm };
                emit_load_guest_gpr(code, 0, rn, sf);
                if effective_imm <= 0xFFF {
                    code.emit32(enc_sub_imm(sf, 0, 0, effective_imm as u16, false));
                } else if effective_imm <= 0xFFF_000 && effective_imm & 0xFFF == 0 {
                    code.emit32(enc_sub_imm(sf, 0, 0, (effective_imm >> 12) as u16, true));
                } else {
                    return None;
                }
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::Movz { sf, rd, imm16, shift } => {
                // MOVZ directly into x0, then store
                code.emit32(enc_movz(0, imm16, shift / 16));
                if !sf {
                    // Zero-extend to 32 bits: MOV W0, W0 (implicit zero-ext)
                }
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::Movk { sf, rd, imm16, shift } => {
                emit_load_guest_gpr(code, 0, rd, sf);
                code.emit32(enc_movk(sf, 0, imm16, shift / 16));
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::Movn { sf, rd, imm16, shift } => {
                code.emit32(enc_movn(sf, 0, imm16, shift / 16));
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::AddReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                // ADD Xd, Xn, Xm, shift #amount
                code.emit32(enc_add_reg(sf, 0, 0, 1, shift_type, shift_amount));
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::SubReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                code.emit32(enc_sub_reg(sf, 0, 0, 1, shift_type, shift_amount));
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::LogicalReg { sf, opc, n, rd, rn, rm, shift_type, shift_amount } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                code.emit32(enc_logical_reg(sf, opc, n, 0, 0, 1, shift_type, shift_amount));
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::Madd { sf, rd, rn, rm, ra } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_guest_gpr(code, 2, ra, sf);
                // MADD Xd, Xn, Xm, Xa: Xd = Xa + Xn * Xm
                code.emit32(enc_madd(sf, 3, 0, 1, 2));
                emit_store_guest_gpr(code, 3, rd, sf);
            }

            Instruction::Msub { sf, rd, rn, rm, ra } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_guest_gpr(code, 2, ra, sf);
                code.emit32(enc_msub(sf, 3, 0, 1, 2));
                emit_store_guest_gpr(code, 3, rd, sf);
            }

            // Any instruction we can't JIT → bail out.
            // The caller will fall back to the interpreter for the whole block.
            _ => return None,
        }
    }

    // Update instruction count
    // ADD x20, x20, #insn_count
    if insn_count > 0 && insn_count <= 0xFFF {
        code.emit32(enc_add_imm(true, 20, 20, insn_count as u16, false));
    }

    // Epilogue: return instruction count, restore callee-saved regs.
    //   mov x0, x20          // return insn_count
    //   ldp x21, x22, [sp, #32]
    //   ldp x19, x20, [sp, #16]
    //   ldp x29, x30, [sp], #48
    //   ret
    code.emit32(enc_mov_reg(0, 20));
    code.emit32(enc_ldp_offset(21, 22, 31, 32));
    code.emit32(enc_ldp_offset(19, 20, 31, 16));
    code.emit32(enc_ldp_post(29, 30, 31, 48));
    code.emit32(0xD65F03C0); // RET

    Some(())
}

/// On non-aarch64, block emission always fails.
#[cfg(not(target_arch = "aarch64"))]
pub(super) fn emit_block(_code: &mut CodeBuffer, _block: &[Instruction]) -> Option<()> {
    None
}

// ── Helper: load a guest GPR from CpuState[x19] into host register ──────────

#[cfg(target_arch = "aarch64")]
fn emit_load_guest_gpr(code: &mut CodeBuffer, host_reg: u8, guest_reg: u8, sf: bool) {
    if guest_reg == 31 {
        // XZR / WZR — zero
        code.emit32(enc_movz(host_reg, 0, 0));
        return;
    }
    let offset = gpr_offset(guest_reg);
    // LDR Xn, [x19, #offset]
    if sf {
        code.emit32(enc_ldr_imm(true, host_reg, 19, offset / 8));
    } else {
        // LDR Wn — 32-bit load (zero-extends)
        code.emit32(enc_ldr_imm(false, host_reg, 19, offset / 4));
    }
}

#[cfg(target_arch = "aarch64")]
fn emit_store_guest_gpr(code: &mut CodeBuffer, host_reg: u8, guest_reg: u8, sf: bool) {
    if guest_reg == 31 {
        return; // Writes to XZR are discarded
    }
    let offset = gpr_offset(guest_reg);
    if sf {
        code.emit32(enc_str_imm(true, host_reg, 19, offset / 8));
    } else {
        // Store 32-bit, then zero upper 32 bits
        code.emit32(enc_str_imm(false, host_reg, 19, offset / 4));
        // Zero upper bits by storing zero to offset+4
        code.emit32(enc_str_imm(false, 31, 19, offset / 4 + 1)); // STR WZR
    }
}

// ── Native instruction encoders ─────────────────────────────────────────────

/// ADD Xd/Wd, Xn/Wn, #imm12{, LSL #12}
fn enc_add_imm(sf: bool, rd: u8, rn: u8, imm12: u16, shift12: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b00_10001 << 24)
        | (u32::from(shift12) << 22)
        | ((imm12 as u32 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SUB Xd/Wd, Xn/Wn, #imm12{, LSL #12}
fn enc_sub_imm(sf: bool, rd: u8, rn: u8, imm12: u16, shift12: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b10_10001 << 24)
        | (u32::from(shift12) << 22)
        | ((imm12 as u32 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ADD/SUB (shifted register)
fn enc_add_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0001011 << 24)
        | ((shift as u32 & 0x3) << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

fn enc_sub_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b1001011 << 24)
        | ((shift as u32 & 0x3) << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// Logical (shifted register): sf opc 01010 shift N Rm imm6 Rn Rd
fn enc_logical_reg(sf: bool, opc: u8, n: bool, rd: u8, rn: u8, rm: u8, shift: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | ((opc as u32 & 0x3) << 29)
        | (0b01010 << 24)
        | ((shift as u32 & 0x3) << 22)
        | (u32::from(n) << 21)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// MADD: sf 00 11011 000 Rm 0 Ra Rn Rd
fn enc_madd(sf: bool, rd: u8, rn: u8, rm: u8, ra: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011011000 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0 << 15) // o0=0
        | ((ra as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// MSUB: sf 00 11011 000 Rm 1 Ra Rn Rd
fn enc_msub(sf: bool, rd: u8, rn: u8, rm: u8, ra: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011011000 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (1 << 15) // o0=1
        | ((ra as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// MOVZ Xd/Wd, #imm16, LSL #(hw*16)
fn enc_movz(rd: u8, imm16: u16, hw: u8) -> u32 {
    (1u32 << 31) // sf=1 for 64-bit
        | (0b10_100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

/// MOVK: sf 11 100101 hw imm16 Rd
fn enc_movk(sf: bool, rd: u8, imm16: u16, hw: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b11_100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

/// MOVN: sf 00 100101 hw imm16 Rd
fn enc_movn(sf: bool, rd: u8, imm16: u16, hw: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b00_100101 << 23)
        | ((hw as u32 & 0x3) << 21)
        | ((imm16 as u32) << 5)
        | (rd as u32 & 0x1F)
}

/// MOV Xd, Xm (alias for ORR Xd, XZR, Xm)
fn enc_mov_reg(rd: u8, rm: u8) -> u32 {
    0xAA0003E0 | ((rm as u32 & 0x1F) << 16) | (rd as u32 & 0x1F)
}

/// LDR Xt/Wt, [Xn, #imm] (unsigned offset, scaled)
fn enc_ldr_imm(sf: bool, rt: u8, rn: u8, scaled_imm: u32) -> u32 {
    let size = if sf { 0b11u32 } else { 0b10 };
    (size << 30)
        | (0b111001_01 << 22)
        | ((scaled_imm & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STR Xt/Wt, [Xn, #imm] (unsigned offset, scaled)
fn enc_str_imm(sf: bool, rt: u8, rn: u8, scaled_imm: u32) -> u32 {
    let size = if sf { 0b11u32 } else { 0b10 };
    (size << 30)
        | (0b111001_00 << 22)
        | ((scaled_imm & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STP Xt1, Xt2, [Xn, #imm]! (pre-index)
fn enc_stp_pre(rt: u8, rt2: u8, rn: u8, imm: i32) -> u32 {
    let imm7 = ((imm / 8) as u32) & 0x7F;
    (0b10_101_0_011 << 23)
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STP Xt1, Xt2, [Xn, #imm] (signed offset)
fn enc_stp_offset(rt: u8, rt2: u8, rn: u8, imm: i32) -> u32 {
    let imm7 = ((imm / 8) as u32) & 0x7F;
    (0b10_101_0_010 << 23)
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDP Xt1, Xt2, [Xn, #imm] (signed offset)
fn enc_ldp_offset(rt: u8, rt2: u8, rn: u8, imm: i32) -> u32 {
    let imm7 = ((imm / 8) as u32) & 0x7F;
    (0b10_101_0_010 << 23)
        | (1 << 22) // L=1
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDP Xt1, Xt2, [Xn], #imm (post-index)
fn enc_ldp_post(rt: u8, rt2: u8, rn: u8, imm: i32) -> u32 {
    let imm7 = ((imm / 8) as u32) & 0x7F;
    (0b10_101_0_001 << 23)
        | (1 << 22) // L=1
        | (imm7 << 15)
        | ((rt2 as u32 & 0x1F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enc_add_imm() {
        // ADD X0, X0, #1 → 0x91000400
        let insn = enc_add_imm(true, 0, 0, 1, false);
        assert_eq!(insn & 0xFF000000, 0x91000000); // sf=1, op=0, S=0
    }

    #[test]
    fn test_enc_mov_reg() {
        // MOV X19, X0 → ORR X19, XZR, X0
        let insn = enc_mov_reg(19, 0);
        assert_eq!(insn & 0x1F, 19); // rd=19
        assert_eq!((insn >> 16) & 0x1F, 0); // rm=0
    }

    #[test]
    fn test_enc_movz() {
        // MOVZ X0, #42
        let insn = enc_movz(0, 42, 0);
        assert_eq!((insn >> 5) & 0xFFFF, 42);
        assert_eq!(insn & 0x1F, 0);
    }
}
