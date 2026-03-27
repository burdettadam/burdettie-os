//! Fetch-decode-execute interpreter loop.
//!
//! Steps through ARM64 machine code one instruction at a time, using:
//! - `CpuState` for register/flag state
//! - `Memory` for load/store operations
//! - `FfiBridge` for host function calls
//! - `decode()` for instruction decoding

use crate::cpu::CpuState;
use crate::decode::{self, Instruction};
use crate::ffi::FfiBridge;
use crate::mem::Memory;

/// Why the interpreter stopped.
#[derive(Debug, Clone)]
pub enum ExitReason {
    /// The program returned from main (x0 = exit code).
    Returned { code: i64 },
    /// An explicit exit() call was made.
    Exit { code: i64 },
    /// RET executed with a return address that's outside mapped code.
    ReturnedToHost { lr: u64 },
    /// Hit the maximum instruction count.
    InstructionLimit { count: u64 },
    /// Encountered an instruction the interpreter doesn't implement.
    Unimplemented { pc: u64, raw: u32 },
    /// Memory access fault.
    MemoryFault { pc: u64, addr: u64, is_write: bool },
    /// Branch to unmapped address (not FFI).
    BadBranch { pc: u64, target: u64 },
}

/// Sentinel return address pushed as LR when calling main.
/// When RET pops this, we know main() returned.
const SENTINEL_LR: u64 = 0xDEAD_CAFE_FEED_F00D;

/// Maximum instructions before forced exit (infinite loop guard).
const MAX_INSTRUCTIONS: u64 = 100_000_000;

/// The ARM64 interpreter.
pub struct Interpreter {
    pub cpu: CpuState,
    pub mem: Memory,
    pub ffi: FfiBridge,
    /// Number of instructions executed in the last `run()` call.
    pub instruction_count: u64,
}

impl Interpreter {
    pub fn new(cpu: CpuState, mem: Memory, ffi: FfiBridge) -> Self {
        Self { cpu, mem, ffi, instruction_count: 0 }
    }

    /// Run from the current PC until exit.
    pub fn run(&mut self) -> ExitReason {
        let mut count: u64 = 0;
        loop {
            if count >= MAX_INSTRUCTIONS {
                self.instruction_count = count;
                return ExitReason::InstructionLimit { count };
            }
            count += 1;

            let pc = self.cpu.pc;

            // Fetch (strict: no host passthrough for instruction memory)
            let raw = match self.mem.read_u32_mapped(pc) {
                Some(v) => v,
                None => {
                    self.instruction_count = count;
                    return ExitReason::MemoryFault {
                        pc,
                        addr: pc,
                        is_write: false,
                    };
                }
            };

            // Decode
            let insn = decode::decode(raw, pc);

            // Execute
            match self.execute(insn, pc, raw) {
                StepResult::Continue => {}
                StepResult::Exit(reason) => {
                    self.instruction_count = count;
                    log::info!("Interpreter exited after {count} instructions: {reason:?}");
                    return reason;
                }
            }
        }
    }

    /// Execute a single instruction. Returns whether to continue or exit.
    fn execute(&mut self, insn: Instruction, pc: u64, _raw: u32) -> StepResult {
        match insn {
            // ── NOP / System ───────────────────────────────────────
            Instruction::Nop => {
                self.cpu.pc = pc + 4;
            }
            Instruction::Svc { imm } => {
                log::debug!("SVC #{imm} at {pc:#x} — treating as exit(x0)");
                return StepResult::Exit(ExitReason::Exit {
                    code: self.cpu.x[0] as i64,
                });
            }

            // ── ADD/SUB immediate ──────────────────────────────────
            Instruction::AddImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift);
                let base = self.cpu.read_reg(rn);
                let result = if sf {
                    base.wrapping_add(op2 as u64)
                } else {
                    (base as u32).wrapping_add(op2) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AddsImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift) as u64;
                let base = self.cpu.read_reg(rn);
                if sf {
                    let (result, carry) = base.overflowing_add(op2);
                    let overflow = ((base ^ !op2) & (base ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let a = base as u32;
                    let b = op2 as u32;
                    let (result, carry) = a.overflowing_add(b);
                    let overflow = ((a ^ !b) & (a ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::SubImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift);
                let base = self.cpu.read_reg(rn);
                let result = if sf {
                    base.wrapping_sub(op2 as u64)
                } else {
                    (base as u32).wrapping_sub(op2) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::SubsImm { sf, rd, rn, imm, shift } => {
                let op2 = shift_imm12(imm, shift) as u64;
                let base = self.cpu.read_reg(rn);
                if sf {
                    let (result, borrow) = base.overflowing_sub(op2);
                    let carry = !borrow;
                    let overflow = ((base ^ op2) & (base ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let a = base as u32;
                    let b = op2 as u32;
                    let (result, borrow) = a.overflowing_sub(b);
                    let carry = !borrow;
                    let overflow = ((a ^ b) & (a ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }

            // ── MOV wide ───────────────────────────────────────────
            Instruction::Movz { sf, rd, imm16, shift } => {
                let val = (imm16 as u64) << shift;
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Movk { sf, rd, imm16, shift } => {
                let mask = 0xFFFFu64 << shift;
                let old = self.cpu.read_reg(rd);
                let val = (old & !mask) | ((imm16 as u64) << shift);
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Movn { sf, rd, imm16, shift } => {
                let val = !((imm16 as u64) << shift);
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }

            // ── PC-relative addressing ─────────────────────────────
            Instruction::Adrp { rd, imm } => {
                // imm is already the full computed address (page(PC) + offset)
                self.cpu.write_reg(rd, imm as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::Adr { rd, imm } => {
                self.cpu.write_reg(rd, imm as u64);
                self.cpu.pc = pc + 4;
            }

            // ── Logical immediate ──────────────────────────────────
            Instruction::LogicalImm { sf, opc, rd, rn, imm } => {
                let src = self.cpu.read_reg_zr(rn);
                let result = match opc {
                    0b00 => src & imm,        // AND
                    0b01 => src | imm,        // ORR
                    0b10 => src ^ imm,        // EOR
                    0b11 => {                 // ANDS
                        let r = src & imm;
                        if sf {
                            self.cpu.update_nzcv64(r, false, false);
                        } else {
                            self.cpu.update_nzcv32(r as u32, false, false);
                        }
                        r
                    }
                    _ => unreachable!(),
                };
                let result = if sf { result } else { result & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── ADD/SUB register ───────────────────────────────────
            Instruction::AddReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                let result = if sf { a.wrapping_add(b) } else { (a as u32).wrapping_add(b as u32) as u64 };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::AddsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                if sf {
                    let (result, carry) = a.overflowing_add(b);
                    let overflow = ((a ^ !b) & (a ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let (result, carry) = (a as u32).overflowing_add(b as u32);
                    let overflow = (((a as u32) ^ !(b as u32)) & ((a as u32) ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::SubReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                let result = if sf { a.wrapping_sub(b) } else { (a as u32).wrapping_sub(b as u32) as u64 };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::SubsReg { sf, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg(rn);
                let b = decode::apply_shift(self.cpu.read_reg(rm), shift_type, shift_amount, sf);
                if sf {
                    let (result, borrow) = a.overflowing_sub(b);
                    let carry = !borrow;
                    let overflow = ((a ^ b) & (a ^ result)) >> 63 != 0;
                    self.cpu.update_nzcv64(result, carry, overflow);
                    self.cpu.write_reg(rd, result);
                } else {
                    let (result, borrow) = (a as u32).overflowing_sub(b as u32);
                    let carry = !borrow;
                    let overflow = (((a as u32) ^ (b as u32)) & ((a as u32) ^ result)) >> 31 != 0;
                    self.cpu.update_nzcv32(result, carry, overflow);
                    self.cpu.write_reg(rd, result as u64);
                }
                self.cpu.pc = pc + 4;
            }

            // ── Logical register ───────────────────────────────────
            Instruction::LogicalReg { sf, opc, n, rd, rn, rm, shift_type, shift_amount } => {
                let a = self.cpu.read_reg_zr(rn);
                let mut b = decode::apply_shift(self.cpu.read_reg_zr(rm), shift_type, shift_amount, sf);
                if n { b = !b; } // N bit inverts Rm
                let result = match opc {
                    0b00 => a & b,  // AND / BIC
                    0b01 => a | b,  // ORR / ORN
                    0b10 => a ^ b,  // EOR / EON
                    0b11 => {       // ANDS / BICS
                        let r = a & b;
                        if sf {
                            self.cpu.update_nzcv64(r, false, false);
                        } else {
                            self.cpu.update_nzcv32(r as u32, false, false);
                        }
                        r
                    }
                    _ => unreachable!(),
                };
                let result = if sf { result } else { result & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Multiply-add / Multiply-sub ────────────────────────
            Instruction::Madd { sf, rd, rn, rm, ra } => {
                let a = self.cpu.read_reg(ra);
                let n = self.cpu.read_reg(rn);
                let m = self.cpu.read_reg(rm);
                let result = if sf {
                    a.wrapping_add(n.wrapping_mul(m))
                } else {
                    (a as u32).wrapping_add((n as u32).wrapping_mul(m as u32)) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }
            Instruction::Msub { sf, rd, rn, rm, ra } => {
                let a = self.cpu.read_reg(ra);
                let n = self.cpu.read_reg(rn);
                let m = self.cpu.read_reg(rm);
                let result = if sf {
                    a.wrapping_sub(n.wrapping_mul(m))
                } else {
                    (a as u32).wrapping_sub((n as u32).wrapping_mul(m as u32)) as u64
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Division ───────────────────────────────────────────
            Instruction::Div { sf, rd, rn, rm, is_signed } => {
                let a = self.cpu.read_reg(rn);
                let b = self.cpu.read_reg(rm);
                let result = if sf {
                    if b == 0 {
                        0
                    } else if is_signed {
                        ((a as i64).wrapping_div(b as i64)) as u64
                    } else {
                        a / b
                    }
                } else {
                    let a32 = a as u32;
                    let b32 = b as u32;
                    if b32 == 0 {
                        0
                    } else if is_signed {
                        ((a32 as i32).wrapping_div(b32 as i32)) as u32 as u64
                    } else {
                        (a32 / b32) as u64
                    }
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Conditional select ─────────────────────────────────
            Instruction::Csel { sf, rd, rn, rm, cond } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    self.cpu.read_reg_zr(rm)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csinc { sf, rd, rn, rm, cond } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    self.cpu.read_reg_zr(rm).wrapping_add(1)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csinv { sf, rd, rn, rm, cond } => {
                let val = if cond.eval(self.cpu.nzcv) {
                    self.cpu.read_reg_zr(rn)
                } else {
                    !self.cpu.read_reg_zr(rm)
                };
                let val = if sf { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg_zr(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Csneg { sf, rd, rn, rm, cond } => {
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
            Instruction::Ccmp { sf, rn, imm5_or_rm, nzcv, cond, is_imm } => {
                if cond.eval(self.cpu.nzcv) {
                    let a = self.cpu.read_reg(rn);
                    let b = if is_imm { imm5_or_rm as u64 } else { self.cpu.read_reg(imm5_or_rm) };
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
            Instruction::Ccmn { sf, rn, imm5_or_rm, nzcv, cond, is_imm } => {
                if cond.eval(self.cpu.nzcv) {
                    let a = self.cpu.read_reg(rn);
                    let b = if is_imm { imm5_or_rm as u64 } else { self.cpu.read_reg(imm5_or_rm) };
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
            Instruction::ShiftVar { sf, rd, rn, rm, shift_type } => {
                let val = self.cpu.read_reg(rn);
                let shift_amount = self.cpu.read_reg(rm);
                let result = if sf {
                    let amt = (shift_amount & 63) as u32;
                    crate::decode::apply_shift(val, shift_type, amt as u8, true)
                } else {
                    let amt = (shift_amount & 31) as u32;
                    let r = crate::decode::apply_shift(val & 0xFFFF_FFFF, shift_type, amt as u8, false);
                    r & 0xFFFF_FFFF
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Bitfield (SBFM/BFM/UBFM) ──────────────────────────
            Instruction::Bitfield { sf, opc, rd, rn, immr, imms } => {
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
                        if sf { extended as u64 } else { extended as u64 & 0xFFFF_FFFF }
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
                        if sf { bot } else { bot & 0xFFFF_FFFF }
                    }
                    _ => 0, // reserved, won't reach (decoder filters)
                };
                self.cpu.write_reg(rd, result);
                self.cpu.pc = pc + 4;
            }

            // ── Load immediate (unsigned offset) ───────────────────
            Instruction::LdrImm { sf, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = if sf {
                    match self.mem.read_u64(addr) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(addr) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrImm { sf, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = self.cpu.read_reg(rt);
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
                    match self.mem.read_u64(addr) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(addr) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stur { sf, rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let val = self.cpu.read_reg(rt);
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
                    match self.mem.read_u64(base) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(base) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrPre { sf, rt, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let val = self.cpu.read_reg(rt);
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
                    match self.mem.read_u64(base) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(base) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrPost { sf, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg(rt);
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

            // ── Load/Store pair ────────────────────────────────────
            Instruction::Ldp { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    let a = self.mem.read_u64(base);
                    let b = self.mem.read_u64(base + size);
                    match (a, b) {
                        (Some(a), Some(b)) => (a, b),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                } else {
                    let a = self.mem.read_u32(base);
                    let b = self.mem.read_u32(base + size);
                    match (a, b) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, v1);
                self.cpu.write_reg(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stp { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg(rt);
                let v2 = self.cpu.read_reg(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32) && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdpPre { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    match (self.mem.read_u64(base), self.mem.read_u64(base + size)) {
                        (Some(a), Some(b)) => (a, b),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                } else {
                    match (self.mem.read_u32(base), self.mem.read_u32(base + size)) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, v1);
                self.cpu.write_reg(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPre { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg(rt);
                let v2 = self.cpu.read_reg(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32) && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdpPost { sf, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    match (self.mem.read_u64(base), self.mem.read_u64(base + size)) {
                        (Some(a), Some(b)) => (a, b),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                } else {
                    match (self.mem.read_u32(base), self.mem.read_u32(base + size)) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, v1);
                self.cpu.write_reg(rt2, v2);
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPost { sf, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg(rt);
                let v2 = self.cpu.read_reg(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32) && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }

            // ── Byte/Halfword/SignedWord loads/stores ──────────────
            Instruction::Ldrb { rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                match self.mem.read_u8(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as u64),
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Strb { rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = self.cpu.read_reg(rt) as u8;
                if !self.mem.write_u8(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrh { rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                match self.mem.read_u16(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as u64),
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Strh { rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                let val = self.cpu.read_reg(rt) as u16;
                if !self.mem.write_u16(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsw { rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                match self.mem.read_u32(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as i32 as i64 as u64), // sign-extend
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsb { rt, rn, imm, sf } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                match self.mem.read_u8(addr) {
                    Some(v) => {
                        let val = if sf { v as i8 as i64 as u64 } else { v as i8 as i32 as u32 as u64 };
                        self.cpu.write_reg(rt, val);
                    }
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsh { rt, rn, imm, sf } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as u64);
                match self.mem.read_u16(addr) {
                    Some(v) => {
                        let val = if sf { v as i16 as i64 as u64 } else { v as i16 as i32 as u32 as u64 };
                        self.cpu.write_reg(rt, val);
                    }
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }

            // ── Register-indexed loads/stores ──────────────────────
            Instruction::LdrReg { sf, rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let val = if sf {
                    match self.mem.read_u64(addr) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(addr) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrReg { sf, rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let ok = if sf {
                    self.mem.write_u64(addr, self.cpu.read_reg(rt))
                } else {
                    self.mem.write_u32(addr, self.cpu.read_reg(rt) as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrbReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                match self.mem.read_u8(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as u64),
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::StrbReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                if !self.mem.write_u8(addr, self.cpu.read_reg(rt) as u8) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrhReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                match self.mem.read_u16(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as u64),
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::StrhReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                if !self.mem.write_u16(addr, self.cpu.read_reg(rt) as u16) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrsbReg { rt, rn, rm, extend, s, sf } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                match self.mem.read_u8(addr) {
                    Some(v) => {
                        let val = if sf { v as i8 as i64 as u64 } else { v as i8 as i32 as u32 as u64 };
                        self.cpu.write_reg(rt, val);
                    }
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrshReg { rt, rn, rm, extend, s, sf } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                match self.mem.read_u16(addr) {
                    Some(v) => {
                        let val = if sf { v as i16 as i64 as u64 } else { v as i16 as i32 as u32 as u64 };
                        self.cpu.write_reg(rt, val);
                    }
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrswReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 2);
                match self.mem.read_u32(addr) {
                    Some(v) => self.cpu.write_reg(rt, v as i32 as i64 as u64),
                    None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                }
                self.cpu.pc = pc + 4;
            }

            // ── PC-relative literal load ───────────────────────────
            Instruction::LdrLiteral { sf, rt, imm } => {
                let addr = imm as u64;
                let val = if sf {
                    match self.mem.read_u64(addr) {
                        Some(v) => v,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                } else {
                    match self.mem.read_u32(addr) {
                        Some(v) => v as u64,
                        None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                    }
                };
                self.cpu.write_reg(rt, val);
                self.cpu.pc = pc + 4;
            }

            // ── Branches ───────────────────────────────────────────
            Instruction::B { offset } => {
                self.cpu.pc = offset as u64;
            }
            Instruction::Bl { offset } => {
                self.cpu.x[30] = pc + 4; // LR = return address
                let target = offset as u64;
                // Check if this is an FFI call (registered or unmapped).
                if self.ffi.is_ffi_target(target) || !self.mem.is_valid(target) {
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Br { rn } => {
                let target = self.cpu.read_reg(rn);
                if !self.mem.is_valid(target) {
                    // Target is outside mapped memory — treat as FFI call.
                    // BR is a tail-call (no LR update), so we return to caller's LR.
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Blr { rn } => {
                let target = self.cpu.read_reg(rn);
                self.cpu.x[30] = pc + 4;
                if self.ffi.is_ffi_target(target) || !self.mem.is_valid(target) {
                    return self.do_ffi_call(target, pc);
                }
                self.cpu.pc = target;
            }
            Instruction::Ret { rn } => {
                let target = self.cpu.read_reg(rn);
                if target == SENTINEL_LR {
                    return StepResult::Exit(ExitReason::Returned {
                        code: self.cpu.x[0] as i64,
                    });
                }
                if !self.mem.is_valid(target) {
                    return StepResult::Exit(ExitReason::ReturnedToHost { lr: target });
                }
                self.cpu.pc = target;
            }
            Instruction::Bcond { cond, offset } => {
                if cond.eval(self.cpu.nzcv) {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Cbz { sf, rt, offset } => {
                let val = if sf {
                    self.cpu.read_reg(rt)
                } else {
                    self.cpu.read_reg(rt) & 0xFFFF_FFFF
                };
                if val == 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Cbnz { sf, rt, offset } => {
                let val = if sf {
                    self.cpu.read_reg(rt)
                } else {
                    self.cpu.read_reg(rt) & 0xFFFF_FFFF
                };
                if val != 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Tbz { rt, bit, offset } => {
                let val = self.cpu.read_reg(rt);
                if (val >> bit) & 1 == 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }
            Instruction::Tbnz { rt, bit, offset } => {
                let val = self.cpu.read_reg(rt);
                if (val >> bit) & 1 != 0 {
                    self.cpu.pc = offset as u64;
                } else {
                    self.cpu.pc = pc + 4;
                }
            }

            // ── FP/SIMD Load/Store ──────────────────────────────────

            Instruction::FpLdrImm { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrImm { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdur { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStur { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrPre { size, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrPre { size, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrPost { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_load(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrPost { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_store(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdp { size, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: false });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStp { size, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdpPre { size, rt, rt2, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: false });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStpPre { size, rt, rt2, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: true });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdpPost { size, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_load(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: false });
                }
                let addr2 = base.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: false });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStpPost { size, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_store(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: base, is_write: true });
                }
                let addr2 = base.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr: addr2, is_write: true });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrReg { size, rt, rn, rm, extend, s } => {
                let log2_size = match size { 1 => 0u8, 2 => 1, 4 => 2, 8 => 3, 16 => 4, _ => 0 };
                let addr = self.reg_index_addr(rn, rm, extend, s, log2_size);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrReg { size, rt, rn, rm, extend, s } => {
                let log2_size = match size { 1 => 0u8, 2 => 1, 4 => 2, 8 => 3, 16 => 4, _ => 0 };
                let addr = self.reg_index_addr(rn, rm, extend, s, log2_size);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrLiteral { size, rt, imm } => {
                let addr = imm as u64;
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false });
                }
                self.cpu.pc = pc + 4;
            }

            // ── FP Data Processing ─────────────────────────────────

            Instruction::FMovReg { sf, rd, rn } => {
                if sf {
                    self.cpu.write_d(rd, self.cpu.read_d(rn));
                } else {
                    self.cpu.write_s(rd, self.cpu.read_s(rn));
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FMovToFp { sf, rd, rn } => {
                if sf {
                    self.cpu.write_d(rd, self.cpu.read_reg(rn));
                } else {
                    self.cpu.write_s(rd, self.cpu.read_reg(rn) as u32);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FMovToGpr { sf, rd, rn } => {
                if sf {
                    self.cpu.write_reg(rd, self.cpu.read_d(rn));
                } else {
                    self.cpu.write_reg(rd, self.cpu.read_s(rn) as u64);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FMovImm { sf, rd, imm } => {
                if sf {
                    self.cpu.write_d(rd, imm);
                } else {
                    self.cpu.write_s(rd, imm as u32);
                }
                self.cpu.pc = pc + 4;
            }

            Instruction::Scvtf { sf_fp, sf_int, rd, rn } => {
                if sf_fp {
                    // to double
                    let val = if sf_int {
                        self.cpu.read_reg(rn) as i64 as f64
                    } else {
                        self.cpu.read_reg(rn) as i32 as f64
                    };
                    self.cpu.write_d(rd, val.to_bits());
                } else {
                    // to single
                    let val = if sf_int {
                        self.cpu.read_reg(rn) as i64 as f32
                    } else {
                        self.cpu.read_reg(rn) as i32 as f32
                    };
                    self.cpu.write_s(rd, val.to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ucvtf { sf_fp, sf_int, rd, rn } => {
                if sf_fp {
                    let val = if sf_int {
                        self.cpu.read_reg(rn) as f64
                    } else {
                        self.cpu.read_reg(rn) as u32 as f64
                    };
                    self.cpu.write_d(rd, val.to_bits());
                } else {
                    let val = if sf_int {
                        self.cpu.read_reg(rn) as f32
                    } else {
                        self.cpu.read_reg(rn) as u32 as f32
                    };
                    self.cpu.write_s(rd, val.to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fcvtzs { sf_int, sf_fp, rd, rn } => {
                if sf_fp {
                    // from double
                    let f = f64::from_bits(self.cpu.read_d(rn));
                    let val = if sf_int { f as i64 as u64 } else { f as i32 as u32 as u64 };
                    self.cpu.write_reg(rd, val);
                } else {
                    // from single
                    let f = f32::from_bits(self.cpu.read_s(rn));
                    let val = if sf_int { f as i64 as u64 } else { f as i32 as u32 as u64 };
                    self.cpu.write_reg(rd, val);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fcvtzu { sf_int, sf_fp, rd, rn } => {
                if sf_fp {
                    let f = f64::from_bits(self.cpu.read_d(rn));
                    let val = if sf_int { f as u64 } else { f as u32 as u64 };
                    self.cpu.write_reg(rd, val);
                } else {
                    let f = f32::from_bits(self.cpu.read_s(rn));
                    let val = if sf_int { f as u64 } else { f as u32 as u64 };
                    self.cpu.write_reg(rd, val);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fcvt { rd, rn, to_double } => {
                if to_double {
                    // single → double
                    let f = f32::from_bits(self.cpu.read_s(rn));
                    self.cpu.write_d(rd, (f as f64).to_bits());
                } else {
                    // double → single
                    let f = f64::from_bits(self.cpu.read_d(rn));
                    self.cpu.write_s(rd, (f as f32).to_bits());
                }
                self.cpu.pc = pc + 4;
            }

            Instruction::Fcmp { sf, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.fp_compare_64(a, b);
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.fp_compare_32(a, b);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FcmpZero { sf, rn } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    self.fp_compare_64(a, 0.0);
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    self.fp_compare_32(a, 0.0);
                }
                self.cpu.pc = pc + 4;
            }

            Instruction::Fadd { sf, rd, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.cpu.write_d(rd, (a + b).to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.cpu.write_s(rd, (a + b).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fsub { sf, rd, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.cpu.write_d(rd, (a - b).to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.cpu.write_s(rd, (a - b).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fmul { sf, rd, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.cpu.write_d(rd, (a * b).to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.cpu.write_s(rd, (a * b).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fdiv { sf, rd, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.cpu.write_d(rd, (a / b).to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.cpu.write_s(rd, (a / b).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fneg { sf, rd, rn } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    self.cpu.write_d(rd, (-a).to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    self.cpu.write_s(rd, (-a).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fabs { sf, rd, rn } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    self.cpu.write_d(rd, a.abs().to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    self.cpu.write_s(rd, a.abs().to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fsqrt { sf, rd, rn } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    self.cpu.write_d(rd, a.sqrt().to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    self.cpu.write_s(rd, a.sqrt().to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fcsel { sf, rd, rn, rm, cond } => {
                let take = cond.eval(self.cpu.nzcv);
                if sf {
                    let val = if take { self.cpu.read_d(rn) } else { self.cpu.read_d(rm) };
                    self.cpu.write_d(rd, val);
                } else {
                    let val = if take { self.cpu.read_s(rn) } else { self.cpu.read_s(rm) };
                    self.cpu.write_s(rd, val);
                }
                self.cpu.pc = pc + 4;
            }

            // ── Unknown ────────────────────────────────────────────
            Instruction::Unknown { raw: r } => {
                log::warn!("Unknown instruction {r:#010x} at {pc:#x}");
                return StepResult::Exit(ExitReason::Unimplemented { pc, raw: r });
            }
        }
        StepResult::Continue
    }

    /// Compute address for register-indexed load/store.
    ///
    /// `extend` (option field): 010=UXTW, 011=LSL, 110=SXTW, 111=SXTX
    /// `s`: if true, shift the index register by `log2_size`
    fn reg_index_addr(&self, rn: u8, rm: u8, extend: u8, s: bool, log2_size: u8) -> u64 {
        let base = self.cpu.read_reg(rn);
        let raw_index = self.cpu.read_reg(rm);
        let index = match extend {
            0b011 | 0b111 => raw_index,                           // LSL / SXTX (64-bit)
            0b010 => raw_index & 0xFFFF_FFFF,                    // UXTW (zero-extend 32→64)
            0b110 => (raw_index as u32) as i32 as i64 as u64,    // SXTW (sign-extend 32→64)
            _ => raw_index,
        };
        let shift = if s { log2_size } else { 0 };
        base.wrapping_add(index << shift)
    }

    /// Handle an FFI call: extract args from registers, call the host function,
    /// and process the result (return, exit, or tail-call).
    fn do_ffi_call(&mut self, target: u64, _call_pc: u64) -> StepResult {
        let args: [u64; 8] = [
            self.cpu.x[0], self.cpu.x[1], self.cpu.x[2], self.cpu.x[3],
            self.cpu.x[4], self.cpu.x[5], self.cpu.x[6], self.cpu.x[7],
        ];
        let result = self.ffi.call_or_stub(target, &args);
        match result {
            crate::ffi::FfiResult::Return(val) => {
                self.cpu.x[0] = val;
                self.cpu.pc = self.cpu.x[30];
                StepResult::Continue
            }
            crate::ffi::FfiResult::Exit(code) => {
                StepResult::Exit(ExitReason::Exit { code: code as i64 })
            }
            crate::ffi::FfiResult::TailCall(addr) => {
                self.cpu.pc = addr;
                StepResult::Continue
            }
        }
    }

    // ── FP helpers ──────────────────────────────────────────────

    /// Load a SIMD/FP register from memory. `size`: 1=B, 2=H, 4=S, 8=D, 16=Q.
    fn fp_load(&mut self, rt: u8, addr: u64, size: u8) -> bool {
        match size {
            1 => {
                if let Some(v) = self.mem.read_u8(addr) {
                    self.cpu.write_q(rt, v as u128);
                    true
                } else { false }
            }
            2 => {
                if let Some(v) = self.mem.read_u16(addr) {
                    self.cpu.write_q(rt, v as u128);
                    true
                } else { false }
            }
            4 => {
                if let Some(v) = self.mem.read_u32(addr) {
                    self.cpu.write_s(rt, v);
                    true
                } else { false }
            }
            8 => {
                if let Some(v) = self.mem.read_u64(addr) {
                    self.cpu.write_d(rt, v);
                    true
                } else { false }
            }
            16 => {
                if let Some(v) = self.mem.read_u128(addr) {
                    self.cpu.write_q(rt, v);
                    true
                } else { false }
            }
            _ => false,
        }
    }

    /// Store a SIMD/FP register to memory. `size`: 1=B, 2=H, 4=S, 8=D, 16=Q.
    fn fp_store(&self, rt: u8, addr: u64, size: u8) -> bool {
        match size {
            1 => self.mem.write_u8(addr, self.cpu.read_q(rt) as u8),
            2 => self.mem.write_u16(addr, self.cpu.read_q(rt) as u16),
            4 => self.mem.write_u32(addr, self.cpu.read_s(rt)),
            8 => self.mem.write_u64(addr, self.cpu.read_d(rt)),
            16 => self.mem.write_u128(addr, self.cpu.read_q(rt)),
            _ => false,
        }
    }

    /// Compare two f64 values and set NZCV (ARM64 FP comparison semantics).
    fn fp_compare_64(&mut self, a: f64, b: f64) {
        if a.is_nan() || b.is_nan() {
            // Unordered
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: false, c: true, v: true };
        } else if a == b {
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: true, c: true, v: false };
        } else if a < b {
            self.cpu.nzcv = crate::cpu::Nzcv { n: true, z: false, c: false, v: false };
        } else {
            // a > b
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: false, c: true, v: false };
        }
    }

    /// Compare two f32 values and set NZCV.
    fn fp_compare_32(&mut self, a: f32, b: f32) {
        if a.is_nan() || b.is_nan() {
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: false, c: true, v: true };
        } else if a == b {
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: true, c: true, v: false };
        } else if a < b {
            self.cpu.nzcv = crate::cpu::Nzcv { n: true, z: false, c: false, v: false };
        } else {
            self.cpu.nzcv = crate::cpu::Nzcv { n: false, z: false, c: true, v: false };
        }
    }
}

/// Internal step result.
enum StepResult {
    Continue,
    Exit(ExitReason),
}

/// Shift a 12-bit immediate by 0 or 12.
fn shift_imm12(imm: u32, shift: u8) -> u32 {
    if shift == 1 {
        imm << 12
    } else {
        imm
    }
}

/// Set up an interpreter to run a Mach-O binary's main().
///
/// - `entry_pc`: address of the entry point (text_base + entry_offset)
/// - `argc`, `argv_ptr`, `envp_ptr`: C-style arguments (host pointers
///   cast to u64, placed in x0/x1/x2)
pub fn setup_for_main(
    mem: Memory,
    ffi: FfiBridge,
    entry_pc: u64,
    argc: i32,
    argv_ptr: u64,
    envp_ptr: u64,
) -> Interpreter {
    let mut cpu = CpuState::new();
    cpu.pc = entry_pc;
    cpu.sp = mem.initial_sp();
    cpu.x[0] = argc as u64;
    cpu.x[1] = argv_ptr;
    cpu.x[2] = envp_ptr;
    cpu.x[30] = SENTINEL_LR; // LR = sentinel so RET from main triggers exit
    Interpreter::new(cpu, mem, ffi)
}
