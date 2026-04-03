//! Extended load/store: pairs, byte/halfword, register-indexed, PC-relative.

use super::{ExitReason, Interpreter, StepResult};
use crate::decode::Instruction;
use crate::{mem_read_u64, mem_read_u32, mem_read_u16, mem_read_u8};

impl Interpreter {
    pub(super) fn exec_ls_extended(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
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
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stp { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
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
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPre { sf, rt, rt2, rn, imm } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
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
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPost { sf, rt, rt2, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
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
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u8!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::Strb { rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let val = self.cpu.read_reg_zr(rt) as u8;
                if !self.mem.write_u8(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrh { rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u16!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::Strh { rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let val = self.cpu.read_reg_zr(rt) as u16;
                if !self.mem.write_u16(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsw { rt, rn, imm } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u32!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as i32 as i64 as u64); // sign-extend
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsb { rt, rn, imm, sf } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u8!(self, addr, pc);
                let val = if sf { v as i8 as i64 as u64 } else { v as i8 as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsh { rt, rn, imm, sf } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u16!(self, addr, pc);
                let val = if sf { v as i16 as i64 as u64 } else { v as i16 as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }

            // ── Register-indexed loads/stores ──────────────────────
            Instruction::LdrReg { sf, rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrReg { sf, rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let ok = if sf {
                    self.mem.write_u64(addr, self.cpu.read_reg_zr(rt))
                } else {
                    self.mem.write_u32(addr, self.cpu.read_reg_zr(rt) as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrbReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                let v = mem_read_u8!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrbReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                if !self.mem.write_u8(addr, self.cpu.read_reg_zr(rt) as u8) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrhReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                let v = mem_read_u16!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrhReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                if !self.mem.write_u16(addr, self.cpu.read_reg_zr(rt) as u16) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrsbReg { rt, rn, rm, extend, s, sf } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                let v = mem_read_u8!(self, addr, pc);
                let val = if sf { v as i8 as i64 as u64 } else { v as i8 as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrshReg { rt, rn, rm, extend, s, sf } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                let v = mem_read_u16!(self, addr, pc);
                let val = if sf { v as i16 as i64 as u64 } else { v as i16 as i32 as u32 as u64 };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrswReg { rt, rn, rm, extend, s } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 2);
                let v = mem_read_u32!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as i32 as i64 as u64);
                self.cpu.pc = pc + 4;
            }

            // ── PC-relative literal load ───────────────────────────
            Instruction::LdrLiteral { sf, rt, imm } => {
                let addr = imm as u64;
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }

            // ── Acquire-release loads/stores (LDAR/STLR) ──────────
            // In a single-threaded interpreter, these are identical to
            // ordinary loads/stores (no hardware ordering needed).
            Instruction::Ldar { sf, rt, rn } | Instruction::Ldapr { sf, rt, rn } => {
                let addr = self.cpu.read_reg(rn);
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stlr { sf, rt, rn } => {
                let addr = self.cpu.read_reg(rn);
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
            Instruction::Ldarb { rt, rn } | Instruction::Ldarh { rt, rn } => {
                let addr = self.cpu.read_reg(rn);
                let is_half = matches!(insn, Instruction::Ldarh { .. });
                let val = if is_half {
                    mem_read_u16!(self, addr, pc) as u64
                } else {
                    mem_read_u8!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stlrb { rt, rn } | Instruction::Stlrh { rt, rn } => {
                let addr = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt);
                let is_half = matches!(insn, Instruction::Stlrh { .. });
                let ok = if is_half {
                    self.mem.write_u16(addr, val as u16)
                } else {
                    self.mem.write_u8(addr, val as u8)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Exclusive loads (single-threaded: behave like LDAR) ──
            Instruction::LoadExclusive { sf, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                // Set exclusive monitor (fake: always succeeds in single-threaded)
                self.cpu.pc = pc + 4;
            }
            Instruction::LoadExclusiveByte { rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = mem_read_u8!(self, addr, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LoadExclusiveHalf { rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = mem_read_u16!(self, addr, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }

            // ── Exclusive stores (single-threaded: always succeed → Rs=0) ──
            Instruction::StoreExclusive { sf, rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.write_u64(addr, val)
                } else {
                    self.mem.write_u32(addr, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                // Write 0 to Rs: exclusive succeeded
                self.cpu.write_reg_zr(rs, 0);
                self.cpu.pc = pc + 4;
            }
            Instruction::StoreExclusiveByte { rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt) as u8;
                if !self.mem.write_u8(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.write_reg_zr(rs, 0);
                self.cpu.pc = pc + 4;
            }
            Instruction::StoreExclusiveHalf { rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt) as u16;
                if !self.mem.write_u16(addr, val) {
                    return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                }
                self.cpu.write_reg_zr(rs, 0);
                self.cpu.pc = pc + 4;
            }

            // ── CLREX: clear exclusive monitor (no-op in interpreter) ──
            Instruction::Clrex => {
                self.cpu.pc = pc + 4;
            }

            // ── LSE LDADD/LDCLR/LDSET/LDEOR/LDMAX/LDMIN atomics ────
            // In a single-threaded interpreter we can implement these
            // non-atomically (there's no concurrent access).
            Instruction::LseLoad { size, op, rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let src = self.cpu.read_reg_zr(rs);
                let (orig, new_val) = match size {
                    8 => {
                        let orig = match self.mem.read_u64(addr) {
                            Some(v) => v,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        let new_val = Self::lse_op64(op, orig, src);
                        if !self.mem.write_u64(addr, new_val) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        (orig, new_val)
                    }
                    4 => {
                        let orig = match self.mem.read_u32(addr) {
                            Some(v) => v as u64,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        let new_val = Self::lse_op32(op, orig as u32, src as u32) as u64;
                        if !self.mem.write_u32(addr, new_val as u32) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        (orig, new_val)
                    }
                    2 => {
                        let orig = match self.mem.read_u16(addr) {
                            Some(v) => v as u64,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        let new_val = Self::lse_op32(op, orig as u32, src as u32) as u64;
                        if !self.mem.write_u16(addr, new_val as u16) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        (orig, new_val)
                    }
                    _ => { // 1 (byte)
                        let orig = match self.mem.read_u8(addr) {
                            Some(v) => v as u64,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        let new_val = Self::lse_op32(op, orig as u32, src as u32) as u64;
                        if !self.mem.write_u8(addr, new_val as u8) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        (orig, new_val)
                    }
                };
                let _ = new_val;
                // Rt gets the original value (loaded before the operation)
                self.cpu.write_reg_zr(rt, orig);
                self.cpu.pc = pc + 4;
            }

            // ── SWP: atomic swap ────────────────────────────────────
            Instruction::Swp { size, rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let src = self.cpu.read_reg_zr(rs);
                let orig = match size {
                    8 => {
                        let v = match self.mem.read_u64(addr) {
                            Some(v) => v,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        if !self.mem.write_u64(addr, src) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        v
                    }
                    4 => {
                        let v = match self.mem.read_u32(addr) {
                            Some(v) => v as u64,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        if !self.mem.write_u32(addr, src as u32) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        v
                    }
                    _ => {
                        let v = match self.mem.read_u16(addr) {
                            Some(v) => v as u64,
                            None => return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: false }),
                        };
                        if !self.mem.write_u16(addr, src as u16) {
                            return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                        }
                        v
                    }
                };
                self.cpu.write_reg_zr(rt, orig);
                self.cpu.pc = pc + 4;
            }

            // ── CAS: compare-and-swap ────────────────────────────────
            Instruction::Cas { sf, rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let expected = self.cpu.read_reg_zr(rs);
                let new_val = self.cpu.read_reg_zr(rt);
                let orig = if sf {
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
                let cmp = if sf { orig == expected } else { orig as u32 == expected as u32 };
                if cmp {
                    let ok = if sf {
                        self.mem.write_u64(addr, new_val)
                    } else {
                        self.mem.write_u32(addr, new_val as u32)
                    };
                    if !ok {
                        return StepResult::Exit(ExitReason::MemoryFault { pc, addr, is_write: true });
                    }
                }
                // Rs is written with the original value (whether or not swap occurred)
                self.cpu.write_reg_zr(rs, orig);
                self.cpu.pc = pc + 4;
            }

            _ => unreachable!("exec_ls called with non-LS instruction"),
        }
        StepResult::Continue
    }

/// Apply a 64-bit LSE atomic operation.
/// op: 0=LDADD, 1=LDCLR, 2=LDEOR, 3=LDSET, 4=LDSMAX, 5=LDSMIN, 6=LDUMAX, 7=LDUMIN
fn lse_op64(op: u8, orig: u64, src: u64) -> u64 {
    match op {
        0 => orig.wrapping_add(src),                                   // LDADD
        1 => orig & !src,                                              // LDCLR (AND NOT)
        2 => orig ^ src,                                               // LDEOR
        3 => orig | src,                                               // LDSET
        4 => if (orig as i64) >= (src as i64) { orig } else { src },  // LDSMAX
        5 => if (orig as i64) <= (src as i64) { orig } else { src },  // LDSMIN
        6 => orig.max(src),                                            // LDUMAX
        7 => orig.min(src),                                            // LDUMIN
        _ => orig,
    }
}

/// Apply a 32-bit LSE atomic operation.
fn lse_op32(op: u8, orig: u32, src: u32) -> u32 {
    match op {
        0 => orig.wrapping_add(src),
        1 => orig & !src,
        2 => orig ^ src,
        3 => orig | src,
        4 => if (orig as i32) >= (src as i32) { orig } else { src },
        5 => if (orig as i32) <= (src as i32) { orig } else { src },
        6 => orig.max(src),
        7 => orig.min(src),
        _ => orig,
    }
}
    pub(super) fn reg_index_addr(&self, rn: u8, rm: u8, extend: u8, s: bool, log2_size: u8) -> u64 {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    // ── Helper encoders for instructions without test_helpers ──────

    /// LDP X pre-index: LDP Xt1, Xt2, [Xn, #imm]!
    fn enc_ldp_pre64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA9C0_0000 | (imm7 << 15) | ((rt2 as u32 & 0x1F) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// STP X pre-index: STP Xt1, Xt2, [Xn, #imm]!
    fn enc_stp_pre64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA980_0000 | (imm7 << 15) | ((rt2 as u32 & 0x1F) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// LDP X post-index: LDP Xt1, Xt2, [Xn], #imm
    fn enc_ldp_post64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA8C0_0000 | (imm7 << 15) | ((rt2 as u32 & 0x1F) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// STP X post-index: STP Xt1, Xt2, [Xn], #imm
    fn enc_stp_post64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA880_0000 | (imm7 << 15) | ((rt2 as u32 & 0x1F) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// LDRSW Xt, [Xn, #imm] (unsigned offset, imm must be 4-aligned)
    fn enc_ldrsw(rt: u8, rn: u8, byte_offset: u32) -> u32 {
        let imm12 = byte_offset / 4;
        0xB980_0000 | ((imm12 & 0xFFF) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// LDRSB Xt, [Xn, #imm] (unsigned offset, sign-extend to 64-bit)
    fn enc_ldrsb_64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
        0x3980_0000 | ((byte_offset & 0xFFF) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// LDRSH Xt, [Xn, #imm] (unsigned offset, sign-extend to 64-bit, imm 2-aligned)
    fn enc_ldrsh_64(rt: u8, rn: u8, byte_offset: u32) -> u32 {
        let imm12 = byte_offset / 2;
        0x7980_0000 | ((imm12 & 0xFFF) << 10) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// LDR Xt, [Xn, Xm, LSL #3] (register offset, extend=011/LSL, S=1)
    fn enc_ldr_reg64(rt: u8, rn: u8, rm: u8) -> u32 {
        0xF860_7800 | ((rm as u32 & 0x1F) << 16) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    /// STR Xt, [Xn, Xm, LSL #3] (register offset, extend=011/LSL, S=1)
    fn enc_str_reg64(rt: u8, rn: u8, rm: u8) -> u32 {
        0xF820_7800 | ((rm as u32 & 0x1F) << 16) | ((rn as u32 & 0x1F) << 5) | (rt as u32 & 0x1F)
    }

    // ── Pair load/store tests ─────────────────────────────────────

    #[test]
    fn test_exec_ldp_64() {
        let mut t = test_interp(&[enc_ldp64(0, 1, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u64(0, 0xAAAA_BBBB_CCCC_DDDD);
        t.write_data_u64(8, 0x1111_2222_3333_4444);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xAAAA_BBBB_CCCC_DDDD);
        assert_eq!(cpu.x[1], 0x1111_2222_3333_4444);
    }

    #[test]
    fn test_exec_stp_64() {
        let mut t = test_interp(&[
            enc_stp64(0, 1, 2, 0),
            enc_ldp64(3, 4, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xDEAD_BEEF;
        t.interp.cpu.x[1] = 0xCAFE_BABE;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[3], 0xDEAD_BEEF);
        assert_eq!(cpu.x[4], 0xCAFE_BABE);
    }

    #[test]
    fn test_exec_ldp_32() {
        let mut t = test_interp(&[enc_ldp32(0, 1, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u32(0, 0xAABBCCDD);
        t.write_data_u32(4, 0x11223344);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xAABBCCDD);
        assert_eq!(cpu.x[1], 0x11223344);
    }

    // ── Pair pre/post-index tests ─────────────────────────────────

    #[test]
    fn test_exec_ldp_pre_64() {
        // LDP X0, X1, [X2, #16]!  → base updated to data_base+16, loads from there
        let mut t = test_interp(&[enc_ldp_pre64(0, 1, 2, 16)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u64(16, 0x1234_5678_9ABC_DEF0);
        t.write_data_u64(24, 0xFEDC_BA98_7654_3210);
        let expected_base = t.data_base + 16;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x1234_5678_9ABC_DEF0);
        assert_eq!(cpu.x[1], 0xFEDC_BA98_7654_3210);
        assert_eq!(cpu.x[2], expected_base, "base register must be updated");
    }

    #[test]
    fn test_exec_stp_pre_64() {
        // STP X0, X1, [X2, #16]!  → base updated, stores at new base
        let mut t = test_interp(&[enc_stp_pre64(0, 1, 2, 16)]);
        t.interp.cpu.x[0] = 0xAAAA;
        t.interp.cpu.x[1] = 0xBBBB;
        t.interp.cpu.x[2] = t.data_base;
        let expected_base = t.data_base + 16;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[2], expected_base, "base register must be updated");
        drop(cpu);
        assert_eq!(t.read_data_u64(16), 0xAAAA);
        assert_eq!(t.read_data_u64(24), 0xBBBB);
    }

    #[test]
    fn test_exec_ldp_post_64() {
        // LDP X0, X1, [X2], #16  → loads from base, THEN updates base
        let mut t = test_interp(&[enc_ldp_post64(0, 1, 2, 16)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u64(0, 0x1111);
        t.write_data_u64(8, 0x2222);
        let expected_base = t.data_base + 16;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x1111);
        assert_eq!(cpu.x[1], 0x2222);
        assert_eq!(cpu.x[2], expected_base, "base updated AFTER load");
    }

    #[test]
    fn test_exec_stp_post_64() {
        // STP X0, X1, [X2], #16  → stores at base, THEN updates base
        let mut t = test_interp(&[enc_stp_post64(0, 1, 2, 16)]);
        t.interp.cpu.x[0] = 0x5555;
        t.interp.cpu.x[1] = 0x6666;
        t.interp.cpu.x[2] = t.data_base;
        let expected_base = t.data_base + 16;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[2], expected_base, "base updated AFTER store");
        drop(cpu);
        assert_eq!(t.read_data_u64(0), 0x5555);
        assert_eq!(t.read_data_u64(8), 0x6666);
    }

    // ── Signed loads ──────────────────────────────────────────────

    #[test]
    fn test_exec_ldrsw_negative() {
        // Write 0xFFFFFFFF (−1 as i32), LDRSW sign-extends to 0xFFFFFFFF_FFFFFFFF
        let mut t = test_interp(&[enc_ldrsw(0, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u32(0, 0xFFFF_FFFF);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn test_exec_ldrsw_positive() {
        // Write 0x7FFFFFFF (max positive i32), no sign extension
        let mut t = test_interp(&[enc_ldrsw(0, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u32(0, 0x7FFF_FFFF);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0x7FFF_FFFF);
    }

    #[test]
    fn test_exec_ldrsb_sign_ext_64() {
        // Write 0x80 (−128 as i8), LDRSB Xt → sign-extended to 0xFFFFFFFF_FFFFFF80
        let mut t = test_interp(&[enc_ldrsb_64(0, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u8(0, 0x80);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF_FFFF_FF80);
    }

    #[test]
    fn test_exec_ldrsh_sign_ext_64() {
        // Write 0x8000 (−32768 as i16), LDRSH Xt → sign-extended to 0xFFFFFFFF_FFFF8000
        let mut t = test_interp(&[enc_ldrsh_64(0, 2, 0)]);
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u16(0, 0x8000);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xFFFF_FFFF_FFFF_8000);
    }

    // ── Register-indexed load/store ───────────────────────────────

    #[test]
    fn test_exec_ldr_reg_64() {
        // LDR X0, [X2, X3, LSL #3]  → addr = X2 + X3*8
        let mut t = test_interp(&[enc_ldr_reg64(0, 2, 3)]);
        t.interp.cpu.x[2] = t.data_base;
        t.interp.cpu.x[3] = 2; // index=2, shift by 3 → offset=16
        t.write_data_u64(16, 0xBEEF_CAFE_1234);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xBEEF_CAFE_1234);
    }

    #[test]
    fn test_exec_str_reg_64() {
        // STR X0, [X2, X3, LSL #3] then load back to verify
        let mut t = test_interp(&[
            enc_str_reg64(0, 2, 3),
            enc_ldr_imm64(4, 2, 16), // load back from data_base+16
        ]);
        t.interp.cpu.x[0] = 0xDEAD_FACE;
        t.interp.cpu.x[2] = t.data_base;
        t.interp.cpu.x[3] = 2; // offset = 2 * 8 = 16
        let cpu = t.run_ok();
        assert_eq!(cpu.x[4], 0xDEAD_FACE);
    }

    // ── Byte / halfword load/store ────────────────────────────────

    #[test]
    fn test_exec_ldrb_strb_roundtrip() {
        let mut t = test_interp(&[
            enc_strb_imm(0, 2, 0),
            enc_ldrb_imm(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0x42;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0x42);
    }

    #[test]
    fn test_exec_ldrh_strh_roundtrip() {
        let mut t = test_interp(&[
            enc_strh_imm(0, 2, 0),
            enc_ldrh_imm(1, 2, 0),
        ]);
        t.interp.cpu.x[0] = 0xABCD;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xABCD);
    }
}
