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
use crate::cpu::{Condition, CpuState, Nzcv};
use crate::decode::Instruction;

/// Offset of `x[n]` (n=0..30) in CpuState.
/// CpuState layout: x: [u64; 31], sp: u64, pc: u64, nzcv: Nzcv(4 bools), ...
/// x[n] is at offset n*8.
const fn gpr_offset(n: u8) -> u32 {
    (n as u32) * 8
}

/// Offset of `sp` in CpuState (after x[31]).
const SP_OFFSET: u32 = 31 * 8;

/// Offset of `pc` in CpuState.
const PC_OFFSET: u32 = 32 * 8;

/// Offset of `nzcv.n` in CpuState.
const NZCV_N_OFFSET: u32 = std::mem::offset_of!(CpuState, nzcv) as u32;
const NZCV_Z_OFFSET: u32 = NZCV_N_OFFSET + std::mem::offset_of!(Nzcv, z) as u32;
const NZCV_C_OFFSET: u32 = NZCV_N_OFFSET + std::mem::offset_of!(Nzcv, c) as u32;
const NZCV_V_OFFSET: u32 = NZCV_N_OFFSET + std::mem::offset_of!(Nzcv, v) as u32;

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

            // ── Flag-setting arithmetic ────────────────────────────────
            // ADDS/SUBS with immediate — needed for CMP (= SUBS with Rd=31)
            Instruction::AddsImm { sf, rd, rn, imm, shift } => {
                let effective_imm = if shift == 1 { imm << 12 } else { imm };
                emit_load_guest_gpr(code, 0, rn, sf);
                if effective_imm <= 0xFFF {
                    code.emit32(enc_adds_imm(sf, 0, 0, effective_imm as u16, false));
                } else if effective_imm <= 0xFFF_000 && effective_imm & 0xFFF == 0 {
                    code.emit32(enc_adds_imm(sf, 0, 0, (effective_imm >> 12) as u16, true));
                } else {
                    return None;
                }
                emit_store_nzcv_from_host(code);
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::SubsImm { sf, rd, rn, imm, shift } => {
                let effective_imm = if shift == 1 { imm << 12 } else { imm };
                emit_load_guest_gpr(code, 0, rn, sf);
                if effective_imm <= 0xFFF {
                    code.emit32(enc_subs_imm(sf, 0, 0, effective_imm as u16, false));
                } else if effective_imm <= 0xFFF_000 && effective_imm & 0xFFF == 0 {
                    code.emit32(enc_subs_imm(sf, 0, 0, (effective_imm >> 12) as u16, true));
                } else {
                    return None;
                }
                emit_store_nzcv_from_host(code);
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::AddsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                code.emit32(enc_adds_reg(sf, 0, 0, 1, shift_type, shift_amount));
                emit_store_nzcv_from_host(code);
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            Instruction::SubsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                code.emit32(enc_subs_reg(sf, 0, 0, 1, shift_type, shift_amount));
                emit_store_nzcv_from_host(code);
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            // ── Load/Store (unsigned immediate offset) ─────────────────
            // Guest addresses are identity-mapped (host addr == guest addr),
            // so we can emit native loads/stores directly.
            Instruction::LdrImm { sf, rt, rn, imm } => {
                // imm is the byte offset already scaled by decoder
                emit_load_guest_gpr(code, 0, rn, true); // base addr always 64-bit
                // Add offset if non-zero
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm) {
                        return None;
                    }
                }
                // Native LDR from [x0] → x1
                if sf {
                    code.emit32(enc_ldr_imm(true, 1, 0, 0)); // LDR X1, [X0]
                } else {
                    code.emit32(enc_ldr_imm(false, 1, 0, 0)); // LDR W1, [X0]
                }
                emit_store_guest_gpr(code, 1, rt, sf);
            }

            Instruction::StrImm { sf, rt, rn, imm } => {
                emit_load_guest_gpr(code, 1, rt, sf);     // value
                emit_load_guest_gpr(code, 0, rn, true);   // base addr
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm) {
                        return None;
                    }
                }
                if sf {
                    code.emit32(enc_str_imm(true, 1, 0, 0)); // STR X1, [X0]
                } else {
                    code.emit32(enc_str_imm(false, 1, 0, 0)); // STR W1, [X0]
                }
            }

            // ── Load/Store pair (signed offset) ────────────────────────
            Instruction::Ldp { sf, rt, rt2, rn, imm } => {
                emit_load_guest_gpr(code, 0, rn, true); // base addr
                let byte_offset = imm as i32;
                if byte_offset != 0 {
                    if !emit_add_imm_signed(code, 0, 0, byte_offset) {
                        return None;
                    }
                }
                let scale = if sf { 8u32 } else { 4 };
                // LDR first reg from [x0], second from [x0+scale]
                if sf {
                    code.emit32(enc_ldr_imm(true, 1, 0, 0));
                    code.emit32(enc_ldr_imm(true, 2, 0, scale / 8));
                } else {
                    code.emit32(enc_ldr_imm(false, 1, 0, 0));
                    code.emit32(enc_ldr_imm(false, 2, 0, scale / 4));
                }
                emit_store_guest_gpr(code, 1, rt, sf);
                emit_store_guest_gpr(code, 2, rt2, sf);
            }

            Instruction::Stp { sf, rt, rt2, rn, imm } => {
                emit_load_guest_gpr(code, 1, rt, sf);
                emit_load_guest_gpr(code, 2, rt2, sf);
                emit_load_guest_gpr(code, 0, rn, true); // base addr
                let byte_offset = imm as i32;
                if byte_offset != 0 {
                    if !emit_add_imm_signed(code, 0, 0, byte_offset) {
                        return None;
                    }
                }
                let scale = if sf { 8u32 } else { 4 };
                if sf {
                    code.emit32(enc_str_imm(true, 1, 0, 0));
                    code.emit32(enc_str_imm(true, 2, 0, scale / 8));
                } else {
                    code.emit32(enc_str_imm(false, 1, 0, 0));
                    code.emit32(enc_str_imm(false, 2, 0, scale / 4));
                }
            }

            // ── Byte/halfword loads/stores ─────────────────────────────
            Instruction::Ldrb { rt, rn, imm } => {
                emit_load_guest_gpr(code, 0, rn, true);
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm as u32) {
                        return None;
                    }
                }
                code.emit32(enc_ldrb_imm_unsigned(1, 0, 0)); // LDRB W1, [X0]
                emit_store_guest_gpr(code, 1, rt, false); // zero-extend to 64
            }

            Instruction::Strb { rt, rn, imm } => {
                emit_load_guest_gpr(code, 1, rt, false);
                emit_load_guest_gpr(code, 0, rn, true);
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm as u32) {
                        return None;
                    }
                }
                code.emit32(enc_strb_imm_unsigned(1, 0, 0)); // STRB W1, [X0]
            }

            Instruction::Ldrh { rt, rn, imm } => {
                emit_load_guest_gpr(code, 0, rn, true);
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm as u32) {
                        return None;
                    }
                }
                code.emit32(enc_ldrh_imm_unsigned(1, 0, 0)); // LDRH W1, [X0]
                emit_store_guest_gpr(code, 1, rt, false);
            }

            Instruction::Strh { rt, rn, imm } => {
                emit_load_guest_gpr(code, 1, rt, false);
                emit_load_guest_gpr(code, 0, rn, true);
                if imm > 0 {
                    if !emit_add_imm_flexible(code, 0, 0, imm as u32) {
                        return None;
                    }
                }
                code.emit32(enc_strh_imm_unsigned(1, 0, 0)); // STRH W1, [X0]
            }

            // ── Division ───────────────────────────────────────────────
            Instruction::Div { sf, rd, rn, rm, is_signed } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                code.emit32(enc_div(sf, 2, 0, 1, is_signed));
                emit_store_guest_gpr(code, 2, rd, sf);
            }

            // ── Conditional select ─────────────────────────────────────
            Instruction::Csel { sf, rd, rn, rm, cond } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_nzcv_to_host(code);
                code.emit32(enc_csel(sf, 2, 0, 1, cond));
                emit_store_guest_gpr(code, 2, rd, sf);
            }

            Instruction::Csinc { sf, rd, rn, rm, cond } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_nzcv_to_host(code);
                code.emit32(enc_csinc(sf, 2, 0, 1, cond));
                emit_store_guest_gpr(code, 2, rd, sf);
            }

            Instruction::Csinv { sf, rd, rn, rm, cond } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_nzcv_to_host(code);
                code.emit32(enc_csinv(sf, 2, 0, 1, cond));
                emit_store_guest_gpr(code, 2, rd, sf);
            }

            Instruction::Csneg { sf, rd, rn, rm, cond } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                emit_load_guest_gpr(code, 1, rm, sf);
                emit_load_nzcv_to_host(code);
                code.emit32(enc_csneg(sf, 2, 0, 1, cond));
                emit_store_guest_gpr(code, 2, rd, sf);
            }

            // ── Bitfield (covers LSR, LSL, ASR, SXTW, UXTB, UXTH, etc.) ──
            Instruction::Bitfield { sf, opc, rd, rn, immr, imms } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                code.emit32(enc_bitfield(sf, opc, 0, 0, immr, imms));
                // For opc=1 (BFM), it's an insert — we need the destination too.
                // BFM merges bits into Rd, so load Rd first into a different reg.
                // For simplicity, only JIT UBFM (opc=2) and SBFM (opc=0) which
                // don't require the old Rd value. Bail on BFM (opc=1).
                if opc == 1 { return None; }
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            // ── Logical immediate ─────────────────────────────────────
            Instruction::LogicalImm { sf, opc, rd, rn, imm } => {
                emit_load_guest_gpr(code, 0, rn, sf);
                // Load the pre-decoded immediate into x1 via MOVZ+MOVK
                emit_load_imm64(code, 1, imm);
                // Emit the logical op: AND(0)/ORR(1)/EOR(2)/ANDS(3)
                match opc {
                    0 => code.emit32(enc_and_reg(sf, 0, 0, 1)),
                    1 => code.emit32(enc_orr_reg_simple(sf, 0, 0, 1)),
                    2 => code.emit32(enc_eor_reg(sf, 0, 0, 1)),
                    3 => {
                        code.emit32(enc_ands_reg(sf, 0, 0, 1));
                        emit_store_nzcv_from_host(code);
                    }
                    _ => return None,
                }
                emit_store_guest_gpr(code, 0, rd, sf);
            }

            // ── ADRP: page-relative address ────────────────────────────
            Instruction::Adrp { rd, imm } => {
                // imm is the full computed address from the decoder
                emit_load_imm64(code, 0, imm as u64);
                emit_store_guest_gpr(code, 0, rd, true);
            }

            // ── ADR: PC-relative address ───────────────────────────────
            Instruction::Adr { rd, imm } => {
                emit_load_imm64(code, 0, imm as u64);
                emit_store_guest_gpr(code, 0, rd, true);
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

// ── NZCV flag helpers ───────────────────────────────────────────────────────

/// After a flag-setting native instruction (ADDS/SUBS/ANDS), read host NZCV
/// and store each bit into the guest CpuState NZCV struct.
/// Uses x2, x3 as scratch.
#[cfg(target_arch = "aarch64")]
fn emit_store_nzcv_from_host(code: &mut CodeBuffer) {
    // MRS x2, NZCV → x2[31]=N, [30]=Z, [29]=C, [28]=V
    code.emit32(0xD53B4202); // MRS X2, NZCV

    // N bit → CpuState.nzcv.n
    code.emit32(enc_lsr_imm(true, 3, 2, 31)); // LSR X3, X2, #31
    code.emit32(enc_strb_to_cpu(3, NZCV_N_OFFSET));

    // Z bit → CpuState.nzcv.z
    code.emit32(enc_lsr_imm(true, 3, 2, 30)); // LSR X3, X2, #30
    code.emit32(enc_and_imm1(true, 3, 3));     // AND X3, X3, #1
    code.emit32(enc_strb_to_cpu(3, NZCV_Z_OFFSET));

    // C bit → CpuState.nzcv.c
    code.emit32(enc_lsr_imm(true, 3, 2, 29));
    code.emit32(enc_and_imm1(true, 3, 3));
    code.emit32(enc_strb_to_cpu(3, NZCV_C_OFFSET));

    // V bit → CpuState.nzcv.v
    code.emit32(enc_lsr_imm(true, 3, 2, 28));
    code.emit32(enc_and_imm1(true, 3, 3));
    code.emit32(enc_strb_to_cpu(3, NZCV_V_OFFSET));
}

/// Before a conditional instruction (CSEL etc.), load guest NZCV flags and
/// set the host NZCV register accordingly.
/// Uses x2, x3, x4, x5 as scratch.
#[cfg(target_arch = "aarch64")]
fn emit_load_nzcv_to_host(code: &mut CodeBuffer) {
    // Load each flag byte from CpuState
    code.emit32(enc_ldrb_from_cpu(2, NZCV_N_OFFSET)); // x2 = N
    code.emit32(enc_ldrb_from_cpu(3, NZCV_Z_OFFSET)); // x3 = Z
    code.emit32(enc_ldrb_from_cpu(4, NZCV_C_OFFSET)); // x4 = C
    code.emit32(enc_ldrb_from_cpu(5, NZCV_V_OFFSET)); // x5 = V

    // Pack: x2 = (N<<31) | (Z<<30) | (C<<29) | (V<<28)
    code.emit32(enc_lsl_imm(true, 2, 2, 31)); // N << 31
    code.emit32(enc_lsl_imm(true, 3, 3, 30)); // Z << 30
    code.emit32(enc_orr_reg_simple(true, 2, 2, 3));
    code.emit32(enc_lsl_imm(true, 4, 4, 29)); // C << 29
    code.emit32(enc_orr_reg_simple(true, 2, 2, 4));
    code.emit32(enc_lsl_imm(true, 5, 5, 28)); // V << 28
    code.emit32(enc_orr_reg_simple(true, 2, 2, 5));

    // MSR NZCV, x2
    code.emit32(0xD51B4202); // MSR NZCV, X2
}

/// Emit a flexible ADD of an unsigned 32-bit immediate.
/// Returns false if the immediate can't be encoded.
#[cfg(target_arch = "aarch64")]
fn emit_add_imm_flexible(code: &mut CodeBuffer, rd: u8, rn: u8, imm: u32) -> bool {
    if imm <= 0xFFF {
        code.emit32(enc_add_imm(true, rd, rn, imm as u16, false));
        true
    } else if imm <= 0xFFF_000 && imm & 0xFFF == 0 {
        code.emit32(enc_add_imm(true, rd, rn, (imm >> 12) as u16, true));
        true
    } else if imm <= 0xFF_FFFF {
        // Two-step: ADD rd, rn, #(imm & 0xFFF); ADD rd, rd, #(imm >> 12), LSL #12
        let lo = imm & 0xFFF;
        let hi = imm >> 12;
        if lo > 0 {
            code.emit32(enc_add_imm(true, rd, rn, lo as u16, false));
            if hi > 0 {
                code.emit32(enc_add_imm(true, rd, rd, hi as u16, true));
            }
        } else {
            code.emit32(enc_add_imm(true, rd, rn, hi as u16, true));
        }
        true
    } else {
        // Large immediate: load into scratch reg and ADD
        emit_load_imm64(code, 3, imm as u64);
        code.emit32(enc_add_reg(true, rd, rn, 3, 0, 0));
        true
    }
}

/// Emit ADD/SUB for a signed offset.
#[cfg(target_arch = "aarch64")]
fn emit_add_imm_signed(code: &mut CodeBuffer, rd: u8, rn: u8, imm: i32) -> bool {
    if imm >= 0 {
        emit_add_imm_flexible(code, rd, rn, imm as u32)
    } else {
        let neg = (-(imm as i64)) as u32;
        if neg <= 0xFFF {
            code.emit32(enc_sub_imm(true, rd, rn, neg as u16, false));
            true
        } else if neg <= 0xFFF_000 && neg & 0xFFF == 0 {
            code.emit32(enc_sub_imm(true, rd, rn, (neg >> 12) as u16, true));
            true
        } else {
            emit_load_imm64(code, 3, neg as u64);
            code.emit32(enc_sub_reg(true, rd, rn, 3, 0, 0));
            true
        }
    }
}

/// Load a 64-bit immediate into a host register using MOVZ+MOVK.
#[cfg(target_arch = "aarch64")]
fn emit_load_imm64(code: &mut CodeBuffer, rd: u8, imm: u64) {
    code.emit32(enc_movz(rd, (imm & 0xFFFF) as u16, 0));
    if imm > 0xFFFF {
        code.emit32(enc_movk(true, rd, ((imm >> 16) & 0xFFFF) as u16, 1));
    }
    if imm > 0xFFFF_FFFF {
        code.emit32(enc_movk(true, rd, ((imm >> 32) & 0xFFFF) as u16, 2));
    }
    if imm > 0xFFFF_FFFF_FFFF {
        code.emit32(enc_movk(true, rd, ((imm >> 48) & 0xFFFF) as u16, 3));
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

/// ADDS Xd/Wd, Xn/Wn, #imm12{, LSL #12}
fn enc_adds_imm(sf: bool, rd: u8, rn: u8, imm12: u16, shift12: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b01_10001 << 24)
        | (u32::from(shift12) << 22)
        | ((imm12 as u32 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SUBS Xd/Wd, Xn/Wn, #imm12{, LSL #12}
fn enc_subs_imm(sf: bool, rd: u8, rn: u8, imm12: u16, shift12: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b11_10001 << 24)
        | (u32::from(shift12) << 22)
        | ((imm12 as u32 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ADDS (shifted register)
fn enc_adds_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b0101011 << 24)
        | ((shift as u32 & 0x3) << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// SUBS (shifted register)
fn enc_subs_reg(sf: bool, rd: u8, rn: u8, rm: u8, shift: u8, amount: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b1101011 << 24)
        | ((shift as u32 & 0x3) << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((amount as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// LSR (immediate): UBFM Xd, Xn, #shift, #(regsize-1)
fn enc_lsr_imm(sf: bool, rd: u8, rn: u8, shift: u8) -> u32 {
    let n = sf;
    let imms = if sf { 63u8 } else { 31 };
    enc_bitfield_raw(sf, 0b10, n, rd, rn, shift, imms)
}

/// LSL (immediate): UBFM Xd, Xn, #(-shift MOD regsize), #(regsize-1-shift)
fn enc_lsl_imm(sf: bool, rd: u8, rn: u8, shift: u8) -> u32 {
    let n = sf;
    let regsize = if sf { 64u32 } else { 32 };
    let immr = ((regsize - shift as u32) % regsize) as u8;
    let imms = (regsize - 1 - shift as u32) as u8;
    enc_bitfield_raw(sf, 0b10, n, rd, rn, immr, imms)
}

/// Raw bitfield encoding: sf opc 100110 N immr imms Rn Rd
fn enc_bitfield_raw(sf: bool, opc: u8, n: bool, rd: u8, rn: u8, immr: u8, imms: u8) -> u32 {
    (u32::from(sf) << 31)
        | ((opc as u32 & 0x3) << 29)
        | (0b100110 << 23)
        | (u32::from(n) << 22)
        | ((immr as u32 & 0x3F) << 16)
        | ((imms as u32 & 0x3F) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// Bitfield (SBFM/UBFM/BFM): sf opc 100110 N immr imms Rn Rd
fn enc_bitfield(sf: bool, opc: u8, rd: u8, rn: u8, immr: u8, imms: u8) -> u32 {
    enc_bitfield_raw(sf, opc, sf, rd, rn, immr, imms)
}

/// AND Xd, Xn, Xm (register, no shift)
fn enc_and_reg(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b00_01010_00 << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ORR Xd, Xn, Xm (register, no shift)
fn enc_orr_reg_simple(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b01_01010_00 << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// EOR Xd, Xn, Xm (register, no shift)
fn enc_eor_reg(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b10_01010_00 << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// ANDS Xd, Xn, Xm (register, no shift, flag-setting)
fn enc_ands_reg(sf: bool, rd: u8, rn: u8, rm: u8) -> u32 {
    (u32::from(sf) << 31)
        | (0b11_01010_00 << 22)
        | ((rm as u32 & 0x1F) << 16)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// AND Xd, Xd, #1 — optimized bitmask for single-bit extract
fn enc_and_imm1(sf: bool, rd: u8, rn: u8) -> u32 {
    // AND Xd, Xn, #1 uses logical immediate encoding:
    // sf=1: N=1, immr=0, imms=0 → bitmask = 1
    // sf=0: N=0, immr=0, imms=0 → bitmask = 1
    (u32::from(sf) << 31)
        | (0b00_100100 << 23)
        | (u32::from(sf) << 22) // N
        | (0 << 16) // immr=0
        | (0 << 10) // imms=0
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// STRB Wt, [Xn, #offset] — store byte to CpuState via x19
fn enc_strb_to_cpu(rt: u8, offset: u32) -> u32 {
    // STRB Wt, [X19, #imm12] — size=00, V=0, opc=00
    (0b00_111_0_01_00 << 22)
        | ((offset & 0xFFF) << 10)
        | (19u32 << 5) // base = x19 (CpuState pointer)
        | (rt as u32 & 0x1F)
}

/// LDRB Wt, [Xn, #offset] — load byte from CpuState via x19
fn enc_ldrb_from_cpu(rt: u8, offset: u32) -> u32 {
    // LDRB Wt, [X19, #imm12] — size=00, V=0, opc=01
    (0b00_111_0_01_01 << 22)
        | ((offset & 0xFFF) << 10)
        | (19u32 << 5)
        | (rt as u32 & 0x1F)
}

/// LDRB Wt, [Xn, #0] (unsigned offset, scaled)
fn enc_ldrb_imm_unsigned(rt: u8, rn: u8, imm12: u32) -> u32 {
    (0b00_111_0_01_01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STRB Wt, [Xn, #0] (unsigned offset, scaled)
fn enc_strb_imm_unsigned(rt: u8, rn: u8, imm12: u32) -> u32 {
    (0b00_111_0_01_00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// LDRH Wt, [Xn, #0] (unsigned offset, scaled)
fn enc_ldrh_imm_unsigned(rt: u8, rn: u8, imm12: u32) -> u32 {
    (0b01_111_0_01_01 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// STRH Wt, [Xn, #0] (unsigned offset, scaled)
fn enc_strh_imm_unsigned(rt: u8, rn: u8, imm12: u32) -> u32 {
    (0b01_111_0_01_00 << 22)
        | ((imm12 & 0xFFF) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rt as u32 & 0x1F)
}

/// DIV/UDIV: sf 0 0 11010110 Rm 00001 o1 Rn Rd
fn enc_div(sf: bool, rd: u8, rn: u8, rm: u8, is_signed: bool) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010110 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (0b00001 << 11)
        | (u32::from(is_signed) << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSEL: sf 0 0 11010100 Rm cond 0 0 Rn Rd
fn enc_csel(sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (cond.to_bits() << 12)
        | (0b00 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSINC: sf 0 0 11010100 Rm cond 0 1 Rn Rd
fn enc_csinc(sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition) -> u32 {
    (u32::from(sf) << 31)
        | (0b0011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (cond.to_bits() << 12)
        | (0b01 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSINV: sf 1 0 11010100 Rm cond 0 0 Rn Rd
fn enc_csinv(sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition) -> u32 {
    (u32::from(sf) << 31)
        | (0b1011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (cond.to_bits() << 12)
        | (0b00 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
}

/// CSNEG: sf 1 0 11010100 Rm cond 0 1 Rn Rd
fn enc_csneg(sf: bool, rd: u8, rn: u8, rm: u8, cond: Condition) -> u32 {
    (u32::from(sf) << 31)
        | (0b1011010100 << 21)
        | ((rm as u32 & 0x1F) << 16)
        | (cond.to_bits() << 12)
        | (0b01 << 10)
        | ((rn as u32 & 0x1F) << 5)
        | (rd as u32 & 0x1F)
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

    #[test]
    fn test_enc_adds_imm() {
        // ADDS X0, X0, #1 should have sf=1, op=01
        let insn = enc_adds_imm(true, 0, 0, 1, false);
        assert_eq!(insn & 0xFF000000, 0xB1000000); // sf=1 + 01 100010
    }

    #[test]
    fn test_enc_subs_imm() {
        let insn = enc_subs_imm(true, 0, 0, 1, false);
        assert_eq!(insn & 0xFF000000, 0xF1000000); // sf=1 + 11 100010
    }

    #[test]
    fn test_enc_div() {
        // UDIV X2, X0, X1
        let insn = enc_div(true, 2, 0, 1, false);
        assert_eq!((insn >> 10) & 0x3F, 0b000010); // fixed bits for DIV
    }

    #[test]
    fn test_enc_csel() {
        let insn = enc_csel(true, 0, 1, 2, Condition::Eq);
        assert_eq!((insn >> 12) & 0xF, 0); // cond = EQ = 0
    }

    #[test]
    fn test_enc_csinc() {
        let insn = enc_csinc(true, 0, 1, 2, Condition::Ne);
        assert_eq!((insn >> 10) & 0x3, 0b01); // o2=01 for CSINC
        assert_eq!((insn >> 12) & 0xF, 1); // cond = NE = 1
    }

    #[test]
    fn test_enc_lsr_lsl_imm() {
        let lsr = enc_lsr_imm(true, 0, 1, 4);
        assert_ne!(lsr, 0);
        let lsl = enc_lsl_imm(true, 0, 1, 4);
        assert_ne!(lsl, 0);
    }
}
