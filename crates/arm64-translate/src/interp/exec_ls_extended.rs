//! Extended load/store: pairs, byte/halfword, register-indexed, PC-relative.

use super::{ExitReason, Interpreter, StepResult};
use crate::decode::Instruction;
use crate::{mem_read_u16, mem_read_u32, mem_read_u64, mem_read_u8};

impl Interpreter {
    pub(super) fn exec_ls_extended(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── Load/Store pair ────────────────────────────────────
            Instruction::Ldp {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    let a = self.mem.read_u64(base);
                    let b = self.mem.read_u64(base + size);
                    match (a, b) {
                        (Some(a), Some(b)) => (a, b),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                } else {
                    let a = self.mem.read_u32(base);
                    let b = self.mem.read_u32(base + size);
                    match (a, b) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                };
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stp {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32)
                        && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdpPre {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    match (self.mem.read_u64(base), self.mem.read_u64(base + size)) {
                        (Some(a), Some(b)) => (a, b),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                } else {
                    match (self.mem.read_u32(base), self.mem.read_u32(base + size)) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                };
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPre {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, base);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32)
                        && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdpPost {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                let size = if sf { 8u64 } else { 4 };
                let (v1, v2) = if sf {
                    match (self.mem.read_u64(base), self.mem.read_u64(base + size)) {
                        (Some(a), Some(b)) => (a, b),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                } else {
                    match (self.mem.read_u32(base), self.mem.read_u32(base + size)) {
                        (Some(a), Some(b)) => (a as u64, b as u64),
                        _ => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr: base,
                                is_write: false,
                            })
                        }
                    }
                };
                self.cpu.write_reg_zr(rt, v1);
                self.cpu.write_reg_zr(rt2, v2);
                let new_base = (base as i64).wrapping_add(imm as i64) as u64;
                self.cpu.write_reg(rn, new_base);
                self.cpu.pc = pc + 4;
            }
            Instruction::StpPost {
                sf,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                let size = if sf { 8u64 } else { 4 };
                let v1 = self.cpu.read_reg_zr(rt);
                let v2 = self.cpu.read_reg_zr(rt2);
                let ok = if sf {
                    self.mem.write_u64(base, v1) && self.mem.write_u64(base + size, v2)
                } else {
                    self.mem.write_u32(base, v1 as u32)
                        && self.mem.write_u32(base + size, v2 as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: true,
                    });
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
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
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
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
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
                let val = if sf {
                    v as i8 as i64 as u64
                } else {
                    v as i8 as i32 as u32 as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Ldrsh { rt, rn, imm, sf } => {
                let addr = (self.cpu.read_reg(rn) as i64).wrapping_add(imm as i64) as u64;
                let v = mem_read_u16!(self, addr, pc);
                let val = if sf {
                    v as i16 as i64 as u64
                } else {
                    v as i16 as i32 as u32 as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }

            // ── Register-indexed loads/stores ──────────────────────
            Instruction::LdrReg {
                sf,
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let val = if sf {
                    mem_read_u64!(self, addr, pc)
                } else {
                    mem_read_u32!(self, addr, pc) as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrReg {
                sf,
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, if sf { 3 } else { 2 });
                let ok = if sf {
                    self.mem.write_u64(addr, self.cpu.read_reg_zr(rt))
                } else {
                    self.mem.write_u32(addr, self.cpu.read_reg_zr(rt) as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrbReg {
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                let v = mem_read_u8!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrbReg {
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                if !self.mem.write_u8(addr, self.cpu.read_reg_zr(rt) as u8) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrhReg {
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                let v = mem_read_u16!(self, addr, pc);
                self.cpu.write_reg_zr(rt, v as u64);
                self.cpu.pc = pc + 4;
            }
            Instruction::StrhReg {
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                if !self.mem.write_u16(addr, self.cpu.read_reg_zr(rt) as u16) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrsbReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 0);
                let v = mem_read_u8!(self, addr, pc);
                let val = if sf {
                    v as i8 as i64 as u64
                } else {
                    v as i8 as i32 as u32 as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrshReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf,
            } => {
                let addr = self.reg_index_addr(rn, rm, extend, s, 1);
                let v = mem_read_u16!(self, addr, pc);
                let val = if sf {
                    v as i16 as i64 as u64
                } else {
                    v as i16 as i32 as u32 as u64
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::LdrswReg {
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
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
            // Use atomic operations for proper acquire/release semantics
            // that are correct for both single- and multi-threaded execution.
            Instruction::Ldar { sf, rt, rn } | Instruction::Ldapr { sf, rt, rn } => {
                let addr = self.cpu.read_reg(rn);
                let val = if sf {
                    match self.mem.atomic_read_u64(addr) {
                        Some(v) => v,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: false,
                            })
                        }
                    }
                } else {
                    match self.mem.atomic_read_u32(addr) {
                        Some(v) => v as u64,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: false,
                            })
                        }
                    }
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::Stlr { sf, rt, rn } => {
                let addr = self.cpu.read_reg(rn);
                let val = self.cpu.read_reg_zr(rt);
                let ok = if sf {
                    self.mem.atomic_write_u64(addr, val)
                } else {
                    self.mem.atomic_write_u32(addr, val as u32)
                };
                if !ok {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
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
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Exclusive loads: set exclusive monitor ───────────
            Instruction::LoadExclusive { sf, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let size = if sf { 8u8 } else { 4 };
                let val = if sf {
                    match self.mem.atomic_read_u64(addr) {
                        Some(v) => v,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: false,
                            })
                        }
                    }
                } else {
                    match self.mem.atomic_read_u32(addr) {
                        Some(v) => v as u64,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: false,
                            })
                        }
                    }
                };
                self.cpu.write_reg_zr(rt, val);
                self.cpu.exclusive_monitor = Some((addr, size));
                self.cpu.pc = pc + 4;
            }
            Instruction::LoadExclusiveByte { rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = mem_read_u8!(self, addr, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.exclusive_monitor = Some((addr, 1));
                self.cpu.pc = pc + 4;
            }
            Instruction::LoadExclusiveHalf { rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let val = mem_read_u16!(self, addr, pc) as u64;
                self.cpu.write_reg_zr(rt, val);
                self.cpu.exclusive_monitor = Some((addr, 2));
                self.cpu.pc = pc + 4;
            }

            // ── Exclusive stores: check exclusive monitor ────────
            // If the monitor matches, perform the store and set Rs=0 (success).
            // If the monitor doesn't match, skip the store and set Rs=1 (failed).
            Instruction::StoreExclusive { sf, rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let size = if sf { 8u8 } else { 4 };
                let monitor_ok = self.cpu.exclusive_monitor == Some((addr, size));
                self.cpu.exclusive_monitor = None; // clear regardless
                if monitor_ok {
                    let val = self.cpu.read_reg_zr(rt);
                    let ok = if sf {
                        self.mem.atomic_write_u64(addr, val)
                    } else {
                        self.mem.atomic_write_u32(addr, val as u32)
                    };
                    if !ok {
                        return StepResult::Exit(ExitReason::MemoryFault {
                            pc,
                            addr,
                            is_write: true,
                        });
                    }
                    self.cpu.write_reg_zr(rs, 0); // success
                } else {
                    self.cpu.write_reg_zr(rs, 1); // failed
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::StoreExclusiveByte { rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let monitor_ok = self.cpu.exclusive_monitor == Some((addr, 1));
                self.cpu.exclusive_monitor = None;
                if monitor_ok {
                    let val = self.cpu.read_reg_zr(rt) as u8;
                    if !self.mem.write_u8(addr, val) {
                        return StepResult::Exit(ExitReason::MemoryFault {
                            pc,
                            addr,
                            is_write: true,
                        });
                    }
                    self.cpu.write_reg_zr(rs, 0);
                } else {
                    self.cpu.write_reg_zr(rs, 1);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::StoreExclusiveHalf { rs, rt, rn, .. } => {
                let addr = self.cpu.read_reg(rn);
                let monitor_ok = self.cpu.exclusive_monitor == Some((addr, 2));
                self.cpu.exclusive_monitor = None;
                if monitor_ok {
                    let val = self.cpu.read_reg_zr(rt) as u16;
                    if !self.mem.write_u16(addr, val) {
                        return StepResult::Exit(ExitReason::MemoryFault {
                            pc,
                            addr,
                            is_write: true,
                        });
                    }
                    self.cpu.write_reg_zr(rs, 0);
                } else {
                    self.cpu.write_reg_zr(rs, 1);
                }
                self.cpu.pc = pc + 4;
            }

            // ── CLREX: clear exclusive monitor ─────────────────────
            Instruction::Clrex => {
                self.cpu.exclusive_monitor = None;
                self.cpu.pc = pc + 4;
            }

            // ── LSE LDADD/LDCLR/LDSET/LDEOR/LDMAX/LDMIN atomics ────
            // Use real atomic operations for 4- and 8-byte sizes.
            // Byte/halfword sizes use non-atomic emulation (rare in practice).
            Instruction::LseLoad {
                size,
                op,
                rs,
                rt,
                rn,
                ..
            } => {
                let addr = self.cpu.read_reg(rn);
                let src = self.cpu.read_reg_zr(rs);
                let orig = match size {
                    8 => match Self::lse_atomic_op64(&self.mem, op, addr, src) {
                        Some(v) => v,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    },
                    4 => match Self::lse_atomic_op32(&self.mem, op, addr, src as u32) {
                        Some(v) => v as u64,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    },
                    2 => {
                        let orig = match self.mem.read_u16(addr) {
                            Some(v) => v as u64,
                            None => {
                                return StepResult::Exit(ExitReason::MemoryFault {
                                    pc,
                                    addr,
                                    is_write: false,
                                })
                            }
                        };
                        let new_val = Self::lse_op_narrow(op, orig as u32, src as u32, 16);
                        if !self.mem.write_u16(addr, new_val as u16) {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            });
                        }
                        orig
                    }
                    _ => {
                        // 1 (byte)
                        let orig = match self.mem.read_u8(addr) {
                            Some(v) => v as u64,
                            None => {
                                return StepResult::Exit(ExitReason::MemoryFault {
                                    pc,
                                    addr,
                                    is_write: false,
                                })
                            }
                        };
                        let new_val = Self::lse_op_narrow(op, orig as u32, src as u32, 8);
                        if !self.mem.write_u8(addr, new_val as u8) {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            });
                        }
                        orig
                    }
                };
                // Rt gets the original value (loaded before the operation)
                self.cpu.write_reg_zr(rt, orig);
                self.cpu.pc = pc + 4;
            }

            // ── SWP: atomic swap ────────────────────────────────────
            Instruction::Swp {
                size, rs, rt, rn, ..
            } => {
                let addr = self.cpu.read_reg(rn);
                let src = self.cpu.read_reg_zr(rs);
                let orig = match size {
                    8 => match self.mem.atomic_swap_u64(addr, src) {
                        Some(v) => v,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    },
                    4 => match self.mem.atomic_swap_u32(addr, src as u32) {
                        Some(v) => v as u64,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    },
                    _ => {
                        // Byte/halfword: non-atomic fallback
                        let loaded = if size == 2 {
                            self.mem.read_u16(addr).map(u64::from)
                        } else {
                            self.mem.read_u8(addr).map(u64::from)
                        };
                        let Some(v) = loaded else {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: false,
                            });
                        };
                        let stored = if size == 2 {
                            self.mem.write_u16(addr, src as u16)
                        } else {
                            self.mem.write_u8(addr, src as u8)
                        };
                        if !stored {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            });
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
                    match self.mem.atomic_cas_u64(addr, expected, new_val) {
                        Some(v) => v,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    }
                } else {
                    match self
                        .mem
                        .atomic_cas_u32(addr, expected as u32, new_val as u32)
                    {
                        Some(v) => v as u64,
                        None => {
                            return StepResult::Exit(ExitReason::MemoryFault {
                                pc,
                                addr,
                                is_write: true,
                            })
                        }
                    }
                };
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
    #[allow(dead_code)]
    fn lse_op64(op: u8, orig: u64, src: u64) -> u64 {
        match op {
            0 => orig.wrapping_add(src), // LDADD
            1 => orig & !src,            // LDCLR (AND NOT)
            2 => orig ^ src,             // LDEOR
            3 => orig | src,             // LDSET
            4 => {
                if (orig as i64) >= (src as i64) {
                    orig
                } else {
                    src
                }
            } // LDSMAX
            5 => {
                if (orig as i64) <= (src as i64) {
                    orig
                } else {
                    src
                }
            } // LDSMIN
            6 => orig.max(src),          // LDUMAX
            7 => orig.min(src),          // LDUMIN
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
            4 => {
                if (orig as i32) >= (src as i32) {
                    orig
                } else {
                    src
                }
            }
            5 => {
                if (orig as i32) <= (src as i32) {
                    orig
                } else {
                    src
                }
            }
            6 => orig.max(src),
            7 => orig.min(src),
            _ => orig,
        }
    }

    /// Byte/halfword operands must be truncated before unsigned comparisons,
    /// and sign-extended before signed comparisons. Results are truncated by
    /// the caller's width-specific write. This fallback is still non-atomic.
    fn lse_op_narrow(op: u8, orig: u32, src: u32, bits: u32) -> u32 {
        let shift = 32 - bits;
        let mask = u32::MAX >> shift;
        let orig = orig & mask;
        let src = src & mask;
        if matches!(op, 4 | 5) {
            let orig = ((orig << shift) as i32 >> shift) as u32;
            let src = ((src << shift) as i32 >> shift) as u32;
            Self::lse_op32(op, orig, src)
        } else {
            Self::lse_op32(op, orig, src)
        }
    }

    /// Perform a 64-bit LSE atomic operation using real atomic RMW.
    /// Returns the original value before the operation.
    fn lse_atomic_op64(mem: &crate::mem::Memory, op: u8, addr: u64, src: u64) -> Option<u64> {
        match op {
            0 => mem.atomic_fetch_add_u64(addr, src),  // LDADD
            1 => mem.atomic_fetch_and_u64(addr, !src), // LDCLR (AND NOT)
            2 => mem.atomic_fetch_xor_u64(addr, src),  // LDEOR
            3 => mem.atomic_fetch_or_u64(addr, src),   // LDSET
            4 => mem.atomic_fetch_smax_u64(addr, src), // LDSMAX
            5 => mem.atomic_fetch_smin_u64(addr, src), // LDSMIN
            6 => mem.atomic_fetch_umax_u64(addr, src), // LDUMAX
            7 => mem.atomic_fetch_umin_u64(addr, src), // LDUMIN
            _ => mem.atomic_read_u64(addr),
        }
    }

    /// Perform a 32-bit LSE atomic operation using real atomic RMW.
    fn lse_atomic_op32(mem: &crate::mem::Memory, op: u8, addr: u64, src: u32) -> Option<u32> {
        match op {
            0 => mem.atomic_fetch_add_u32(addr, src),  // LDADD
            1 => mem.atomic_fetch_and_u32(addr, !src), // LDCLR
            2 => mem.atomic_fetch_xor_u32(addr, src),  // LDEOR
            3 => mem.atomic_fetch_or_u32(addr, src),   // LDSET
            4 => mem.atomic_fetch_smax_u32(addr, src), // LDSMAX
            5 => mem.atomic_fetch_smin_u32(addr, src), // LDSMIN
            6 => mem.atomic_fetch_umax_u32(addr, src), // LDUMAX
            7 => mem.atomic_fetch_umin_u32(addr, src), // LDUMIN
            _ => mem.atomic_read_u32(addr),
        }
    }
    pub(super) fn reg_index_addr(&self, rn: u8, rm: u8, extend: u8, s: bool, log2_size: u8) -> u64 {
        let base = self.cpu.read_reg(rn);
        let raw_index = self.cpu.read_reg(rm);
        let index = match extend {
            0b011 | 0b111 => raw_index,                       // LSL / SXTX (64-bit)
            0b010 => raw_index & 0xFFFF_FFFF,                 // UXTW (zero-extend 32→64)
            0b110 => (raw_index as u32) as i32 as i64 as u64, // SXTW (sign-extend 32→64)
            _ => raw_index,
        };
        let shift = if s { log2_size } else { 0 };
        base.wrapping_add(index << shift)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::*;

    // ── Helper encoders for instructions without test_helpers ──────

    /// LDP X pre-index: LDP Xt1, Xt2, [Xn, #imm]!
    fn enc_ldp_pre64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA9C0_0000
            | (imm7 << 15)
            | ((rt2 as u32 & 0x1F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rt as u32 & 0x1F)
    }

    /// STP X pre-index: STP Xt1, Xt2, [Xn, #imm]!
    fn enc_stp_pre64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA980_0000
            | (imm7 << 15)
            | ((rt2 as u32 & 0x1F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rt as u32 & 0x1F)
    }

    /// LDP X post-index: LDP Xt1, Xt2, [Xn], #imm
    fn enc_ldp_post64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA8C0_0000
            | (imm7 << 15)
            | ((rt2 as u32 & 0x1F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rt as u32 & 0x1F)
    }

    /// STP X post-index: STP Xt1, Xt2, [Xn], #imm
    fn enc_stp_post64(rt: u8, rt2: u8, rn: u8, byte_offset: i32) -> u32 {
        let imm7 = ((byte_offset / 8) as u32) & 0x7F;
        0xA880_0000
            | (imm7 << 15)
            | ((rt2 as u32 & 0x1F) << 10)
            | ((rn as u32 & 0x1F) << 5)
            | (rt as u32 & 0x1F)
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
        let mut t = test_interp(&[enc_stp64(0, 1, 2, 0), enc_ldp64(3, 4, 2, 0)]);
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
        let mut t = test_interp(&[enc_strb_imm(0, 2, 0), enc_ldrb_imm(1, 2, 0)]);
        t.interp.cpu.x[0] = 0x42;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0x42);
    }

    #[test]
    fn test_exec_ldrh_strh_roundtrip() {
        let mut t = test_interp(&[enc_strh_imm(0, 2, 0), enc_ldrh_imm(1, 2, 0)]);
        t.interp.cpu.x[0] = 0xABCD;
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(cpu.x[1], 0xABCD);
    }

    // ── Exclusive monitor tests ───────────────────────────────────

    #[test]
    fn test_exclusive_monitor_ldxr_stxr_success() {
        // LDXR X0, [X2] ; STXR W1, X0, [X2] — should succeed (monitor matches)
        let ldxr = 0xC85F7C40u32; // LDXR X0, [X2]
        let stxr = 0xC8017C40u32; // STXR W1, X0, [X2]
        let mut t = test_interp(&[ldxr, stxr]);
        t.interp.cpu.x[2] = t.data_base;
        t.interp.mem.write_u64(t.data_base, 0xDEAD_BEEF);
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0], 0xDEAD_BEEF); // LDXR loaded the value
        assert_eq!(cpu.x[1], 0); // STXR succeeded (W1=0)
    }

    #[test]
    fn test_exclusive_monitor_clrex_causes_failure() {
        // LDXR X0, [X2] ; CLREX ; STXR W1, X3, [X2]
        let ldxr = 0xC85F7C40u32; // LDXR X0, [X2]
        let clrex = 0xD503305Fu32; // CLREX
        let stxr = 0xC8037C43u32; // STXR W3, X3, [X2] — use different regs to avoid confusion
        let mut t = test_interp(&[ldxr, clrex, stxr]);
        t.interp.cpu.x[2] = t.data_base;
        t.interp.cpu.x[3] = 0x1234;
        t.interp.mem.write_u64(t.data_base, 0xAAAA);
        let cpu = t.run_ok();
        // STXR should fail because CLREX cleared the monitor
        assert_eq!(cpu.x[3], 1, "STXR should report failure (W3=1) after CLREX");
    }

    /// LSE operation with source W1/X1, result W0/X0, and base X2.
    fn enc_lse(size_bits: u32, op: u32) -> u32 {
        0x3821_0040 | (size_bits << 30) | (op << 12)
    }

    #[test]
    fn atomic_regression_lse_transfer_widths() {
        let initial = 0x1122_3344_5566_77FF_u64;
        for size_bits in 0..4 {
            let width = 1 << size_bits;
            let mask = u64::MAX >> (64 - width * 8);
            let mut t = test_interp(&[enc_lse(size_bits, 0)]); // LDADD
            t.interp.cpu.x[1] = 1;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u64(0, initial);
            assert_eq!(t.run_ok().x[0], initial & mask, "width {width}");
            let expected = (initial & !mask) | ((initial & mask).wrapping_add(1) & mask);
            assert_eq!(t.read_data_u64(0), expected, "width {width}");
        }
    }

    #[test]
    fn atomic_regression_lse_unsigned_64_minmax() {
        for (op, original, source, expected) in [
            (6, 5, u64::MAX - 2, u64::MAX - 2),
            (7, u64::MAX - 4, 3, 3),
            (6, 1 << 63, 0, 1 << 63),
            (7, 0, 1 << 63, 0),
        ] {
            let mut t = test_interp(&[enc_lse(3, op)]);
            t.interp.cpu.x[1] = source;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u64(0, original);
            assert_eq!(t.run_ok().x[0], original);
            assert_eq!(t.read_data_u64(0), expected, "operation {op}");
        }
    }

    #[test]
    fn atomic_regression_lse_32_minmax_rejects_read_only_memory() {
        for op in 4..=7 {
            let mut storage = 9_u32;
            let ptr = std::ptr::from_mut(&mut storage).cast::<u8>();
            let addr = ptr as u64;
            // SAFETY: storage outlives mem and remains readable throughout.
            let mem = unsafe { crate::mem::Memory::from_segments(&[(addr, ptr, 4, false)]) };
            assert_eq!(
                super::Interpreter::lse_atomic_op32(&mem, op, addr, 3),
                None,
                "operation {op} must propagate the failed write"
            );
            assert_eq!(storage, 9);
        }
    }

    #[test]
    fn atomic_regression_lse_32_minmax_results() {
        for (op, original, source, expected) in [
            (4, (-5_i32) as u32, 3, 3),
            (5, 5, (-3_i32) as u32, (-3_i32) as u32),
            (6, 5, u32::MAX - 2, u32::MAX - 2),
            (7, u32::MAX - 4, 3, 3),
        ] {
            let mut t = test_interp(&[enc_lse(2, op)]);
            t.interp.cpu.x[1] = u64::from(source);
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u32(0, original);
            assert_eq!(t.run_ok().x[0], u64::from(original));
            assert_eq!(t.read_data_u32(0), expected, "operation {op}");
        }
    }

    #[test]
    fn atomic_regression_lse_signed_byte_minmax() {
        for (op, expected) in [(4, 0x7F), (5, 0x80)] {
            let mut t = test_interp(&[enc_lse(0, op)]);
            t.interp.cpu.x[1] = 0xABCD_007F; // only the low byte is an operand
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u32(0, 0x1122_3380);
            assert_eq!(t.run_ok().x[0], 0x80);
            assert_eq!(t.read_data_u32(0), 0x1122_3300 | expected);
        }
    }

    #[test]
    fn atomic_regression_lse_signed_halfword_minmax() {
        for (op, expected) in [(4, 0x7FFF), (5, 0x8000)] {
            let mut t = test_interp(&[enc_lse(1, op)]);
            t.interp.cpu.x[1] = 0xABCD_7FFF;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u32(0, 0x1122_8000);
            assert_eq!(t.run_ok().x[0], 0x8000);
            assert_eq!(t.read_data_u32(0), 0x1122_0000 | expected);
        }
    }

    #[test]
    fn atomic_regression_swp_byte_preserves_neighboring_bytes() {
        let mut t = test_interp(&[enc_lse(0, 8)]); // SWPB
        t.interp.cpu.x[1] = 0xABCD;
        t.interp.cpu.x[2] = t.data_base + 1;
        t.write_data_u32(0, 0x4433_2211);
        assert_eq!(t.run_ok().x[0], 0x22);
        assert_eq!(t.read_data_u32(0), 0x4433_CD11);
    }

    #[test]
    fn atomic_regression_acquire_release_word_preserves_neighbor() {
        // LDAR W0, [X2]; STLR W1, [X2].
        let mut t = test_interp(&[0x88DF_FC40, 0x889F_FC41]);
        t.interp.cpu.x[1] = 0xAABB_CCDD_0000_5678;
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u64(0, 0x1122_3344_5566_7788);
        assert_eq!(t.run_ok().x[0], 0x5566_7788);
        assert_eq!(t.read_data_u64(0), 0x1122_3344_0000_5678);
    }

    #[test]
    fn atomic_regression_cas_word_preserves_neighbor() {
        // CAS W0, W1, [X2].
        let mut t = test_interp(&[0x88A0_7C41]);
        t.interp.cpu.x[0] = 0x5566_7788;
        t.interp.cpu.x[1] = 0xAABB_CCDD_0000_5678;
        t.interp.cpu.x[2] = t.data_base;
        t.write_data_u64(0, 0x1122_3344_5566_7788);
        assert_eq!(t.run_ok().x[0], 0x5566_7788);
        assert_eq!(t.read_data_u64(0), 0x1122_3344_0000_5678);
    }

    #[test]
    fn atomic_regression_lse_signed_64_minmax() {
        for (op, original, source, expected) in [
            (4, -5_i64, 3_i64, 3_i64),
            (5, 5, -3, -3),
            (4, i64::MIN, i64::MAX, i64::MAX),
            (5, i64::MAX, i64::MIN, i64::MIN),
        ] {
            let mut t = test_interp(&[enc_lse(3, op)]);
            t.interp.cpu.x[1] = source as u64;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u64(0, original as u64);
            assert_eq!(t.run_ok().x[0], original as u64);
            assert_eq!(t.read_data_u64(0), expected as u64);
        }
    }

    #[test]
    fn atomic_regression_lse_unsigned_narrow_truncates_source() {
        for size_bits in 0..2 {
            for op in [6, 7] {
                let mut t = test_interp(&[enc_lse(size_bits, op)]);
                t.interp.cpu.x[1] = 0xFFFF_0001;
                t.interp.cpu.x[2] = t.data_base;
                t.write_data_u32(0, 2);
                assert_eq!(t.run_ok().x[0], 2);
                assert_eq!(t.read_data_u32(0), if op == 6 { 2 } else { 1 });
            }
        }
    }

    #[test]
    fn atomic_regression_swp_transfer_widths() {
        let initial = 0x1122_3344_5566_7788;
        let source = 0x8877_6655_4433_2211;
        for size_bits in 0..4 {
            let width = 1 << size_bits;
            let mask = u64::MAX >> (64 - width * 8);
            let mut t = test_interp(&[enc_lse(size_bits, 8)]);
            t.interp.cpu.x[1] = source;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u64(0, initial);
            assert_eq!(t.run_ok().x[0], initial & mask, "width {width}");
            assert_eq!(t.read_data_u64(0), (initial & !mask) | (source & mask));
        }
    }

    #[test]
    fn atomic_regression_word_minmax_faults_on_read_only_memory() {
        for op in 4..=7 {
            let mut t = test_interp(&[enc_lse(2, op)]);
            t.interp.cpu.x[1] = 3;
            t.interp.cpu.x[2] = t.data_base;
            t.write_data_u32(0, 9);
            // SAFETY: test_interp keeps these allocations alive. This view
            // intentionally revokes writes to the data region.
            t.interp.mem = unsafe {
                crate::mem::Memory::from_segments(&[
                    (t.code_base, t.code_base as *mut u8, 4096, false),
                    (t.data_base, t.data_base as *mut u8, 4096, false),
                ])
            };
            assert!(matches!(t.run(), crate::interp::ExitReason::MemoryFault {
                pc, addr, is_write: true
            } if pc == t.code_base && addr == t.data_base));
            assert_eq!(t.read_data_u32(0), 9);
        }
    }

    #[test]
    fn atomic_regression_unaligned_accesses_report_memory_faults() {
        // LDAR W0, STLR W1, CAS W0/W1, LDADD W1/W0, and SWP W1/W0.
        for (word, is_write) in [
            (0x88DF_FC40, false),
            (0x889F_FC41, true),
            (0x88A0_7C41, true),
            (enc_lse(2, 0), true),
            (enc_lse(2, 8), true),
        ] {
            for sf in [false, true] {
                let raw = word | (u32::from(sf) << 30);
                let mut t = test_interp(&[raw]);
                t.interp.cpu.x[2] = t.data_base + 1;
                t.write_data_u64(0, 0x1122_3344_5566_7788);
                assert!(matches!(t.run(), crate::interp::ExitReason::MemoryFault {
                    pc, addr, is_write: actual_write
                } if pc == t.code_base && addr == t.data_base + 1 && actual_write == is_write));
                assert_eq!(t.read_data_u64(0), 0x1122_3344_5566_7788);
            }
        }
    }

    #[test]
    fn atomic_regression_acquire_release_exclusives_honor_monitor() {
        for size_bits in 0..4 {
            for clear_monitor in [false, true] {
                let mut instructions = vec![0x085F_FC40 | (size_bits << 30)];
                if clear_monitor {
                    instructions.push(0xD503_305F); // CLREX
                }
                instructions.push(0x0803_FC41 | (size_bits << 30));
                let mut t = test_interp(&instructions); // LDAXR; STLXR W3, W1/X1
                t.interp.cpu.x[1] = 0x99;
                t.interp.cpu.x[2] = t.data_base;
                t.interp.cpu.x[3] = 7;
                t.write_data_u64(0, 0x55);
                let cpu = t.run_ok();
                assert_eq!(cpu.x[0], 0x55);
                assert_eq!(cpu.x[3], u64::from(clear_monitor));
                assert_eq!(t.read_data_u64(0), if clear_monitor { 0x55 } else { 0x99 });
            }
        }
    }
}
