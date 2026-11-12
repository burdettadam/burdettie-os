//! Floating-point and SIMD instruction execution.

use super::{ExitReason, Interpreter, StepResult};
use crate::decode::Instruction;

impl Interpreter {
    pub(super) fn exec_fp(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            // ── FP/SIMD Load/Store ──────────────────────────────────
            Instruction::FpLdrImm { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrImm { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdur { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStur { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrPre { size, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrPre { size, rt, rn, imm } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrPost { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_load(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: false,
                    });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrPost { size, rt, rn, imm } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_store(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: true,
                    });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdp {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: false,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStp {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                let addr = base.wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdpPre {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: false,
                    });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStpPre {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let addr = self.cpu.read_reg(rn).wrapping_add(imm as i64 as u64);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                let addr2 = addr.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: true,
                    });
                }
                self.cpu.write_reg(rn, addr);
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdpPost {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_load(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: false,
                    });
                }
                let addr2 = base.wrapping_add(size as u64);
                if !self.fp_load(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: false,
                    });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStpPost {
                size,
                rt,
                rt2,
                rn,
                imm,
            } => {
                let base = self.cpu.read_reg(rn);
                if !self.fp_store(rt, base, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: base,
                        is_write: true,
                    });
                }
                let addr2 = base.wrapping_add(size as u64);
                if !self.fp_store(rt2, addr2, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr: addr2,
                        is_write: true,
                    });
                }
                self.cpu.write_reg(rn, base.wrapping_add(imm as i64 as u64));
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrReg {
                size,
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let log2_size = match size {
                    1 => 0u8,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    16 => 4,
                    _ => 0,
                };
                let addr = self.reg_index_addr(rn, rm, extend, s, log2_size);
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpStrReg {
                size,
                rt,
                rn,
                rm,
                extend,
                s,
            } => {
                let log2_size = match size {
                    1 => 0u8,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    16 => 4,
                    _ => 0,
                };
                let addr = self.reg_index_addr(rn, rm, extend, s, log2_size);
                if !self.fp_store(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: true,
                    });
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::FpLdrLiteral { size, rt, imm } => {
                let addr = imm as u64;
                if !self.fp_load(rt, addr, size) {
                    return StepResult::Exit(ExitReason::MemoryFault {
                        pc,
                        addr,
                        is_write: false,
                    });
                }
                self.cpu.pc = pc + 4;
            }

            // ── Delegate FP data processing variants ───────────────────────────────
            insn => return self.exec_fp_dp(insn, pc),
        }
        StepResult::Continue
    }

    // ── FP helpers ──────────────────────────────────────────────

    /// Load a SIMD/FP register from memory. `size`: 1=B, 2=H, 4=S, 8=D, 16=Q.
    fn fp_load(&mut self, rt: u8, addr: u64, size: u8) -> bool {
        match size {
            1 => {
                if let Some(v) = self.mem.read_u8(addr) {
                    self.cpu.write_q(rt, v as u128);
                    true
                } else {
                    false
                }
            }
            2 => {
                if let Some(v) = self.mem.read_u16(addr) {
                    self.cpu.write_q(rt, v as u128);
                    true
                } else {
                    false
                }
            }
            4 => {
                if let Some(v) = self.mem.read_u32(addr) {
                    self.cpu.write_s(rt, v);
                    true
                } else {
                    false
                }
            }
            8 => {
                if let Some(v) = self.mem.read_u64(addr) {
                    self.cpu.write_d(rt, v);
                    true
                } else {
                    false
                }
            }
            16 => {
                if let Some(v) = self.mem.read_u128(addr) {
                    self.cpu.write_q(rt, v);
                    true
                } else {
                    false
                }
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
    pub(super) fn fp_compare_64(&mut self, a: f64, b: f64) {
        if a.is_nan() || b.is_nan() {
            // Unordered
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: false,
                c: true,
                v: true,
            };
        } else if a == b {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: true,
                c: true,
                v: false,
            };
        } else if a < b {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: true,
                z: false,
                c: false,
                v: false,
            };
        } else {
            // a > b
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: false,
                c: true,
                v: false,
            };
        }
    }

    /// Compare two f32 values and set NZCV.
    pub(super) fn fp_compare_32(&mut self, a: f32, b: f32) {
        if a.is_nan() || b.is_nan() {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: false,
                c: true,
                v: true,
            };
        } else if a == b {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: true,
                c: true,
                v: false,
            };
        } else if a < b {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: true,
                z: false,
                c: false,
                v: false,
            };
        } else {
            self.cpu.nzcv = crate::cpu::Nzcv {
                n: false,
                z: false,
                c: true,
                v: false,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    // ── FP Load/Store ───────────────────────────────────────────

    #[test]
    fn test_exec_fp_str_ldr_imm64() {
        let mut t = test_interp(&[
            enc_fp_str_imm64(0, 2, 0), // STR D0, [X2, #0]
            enc_fp_ldr_imm64(1, 2, 0), // LDR D1, [X2, #0]
        ]);
        t.interp.cpu.write_d(0, f64::to_bits(3.14));
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(1)), 3.14);
    }

    #[test]
    fn test_exec_fp_str_ldr_imm32() {
        let mut t = test_interp(&[
            enc_fp_str_imm32(0, 2, 0), // STR S0, [X2, #0]
            enc_fp_ldr_imm32(1, 2, 0), // LDR S1, [X2, #0]
        ]);
        t.interp.cpu.write_s(0, f32::to_bits(2.5));
        t.interp.cpu.x[2] = t.data_base;
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(1)), 2.5);
    }

    // ── FMOV (GPR ↔ FP) ────────────────────────────────────────

    #[test]
    fn test_exec_fmov_to_fp64() {
        let mut t = test_interp(&[enc_fmov_to_fp64(0, 1)]);
        t.interp.cpu.x[1] = f64::to_bits(99.5);
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 99.5);
    }

    #[test]
    fn test_exec_fmov_to_gpr64() {
        let mut t = test_interp(&[enc_fmov_to_gpr64(3, 0)]);
        t.interp.cpu.write_d(0, f64::to_bits(-7.25));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.x[3]), -7.25);
    }

    // ── SCVTF (int → float) ────────────────────────────────────

    #[test]
    fn test_exec_scvtf_x_to_double() {
        // SCVTF D0, X1 (sf_fp=true, sf_int=true)
        let mut t = test_interp(&[enc_scvtf(true, true, 0, 1)]);
        t.interp.cpu.x[1] = (-5i64) as u64;
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), -5.0);
    }

    #[test]
    fn test_exec_scvtf_w_to_single() {
        // SCVTF S0, W1 (sf_fp=false, sf_int=false)
        let mut t = test_interp(&[enc_scvtf(false, false, 0, 1)]);
        t.interp.cpu.x[1] = 42;
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 42.0);
    }

    // ── FCVTZS (float → int) ───────────────────────────────────

    #[test]
    fn test_exec_fcvtzs_double_to_x_positive() {
        // FCVTZS X0, D1 (sf_int=true, sf_fp=true)
        let mut t = test_interp(&[enc_fcvtzs(true, true, 0, 1)]);
        t.interp.cpu.write_d(1, f64::to_bits(3.7));
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0] as i64, 3);
    }

    #[test]
    fn test_exec_fcvtzs_double_to_x_negative() {
        let mut t = test_interp(&[enc_fcvtzs(true, true, 0, 1)]);
        t.interp.cpu.write_d(1, f64::to_bits(-3.7));
        let cpu = t.run_ok();
        assert_eq!(cpu.x[0] as i64, -3);
    }

    // ── FMOV (FP register copy) ────────────────────────────────

    #[test]
    fn test_exec_fmov_reg_double() {
        // FMOV D2, D0
        let mut t = test_interp(&[enc_fmov_reg(true, 2, 0)]);
        t.interp.cpu.write_d(0, f64::to_bits(1.234));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(2)), 1.234);
    }

    // ── FADD ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fadd_double() {
        let mut t = test_interp(&[enc_fadd(true, 0, 1, 2)]);
        t.interp.cpu.write_d(1, f64::to_bits(1.5));
        t.interp.cpu.write_d(2, f64::to_bits(2.5));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 4.0);
    }

    // ── FSUB ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fsub_double() {
        let mut t = test_interp(&[enc_fsub(true, 0, 1, 2)]);
        t.interp.cpu.write_d(1, f64::to_bits(10.0));
        t.interp.cpu.write_d(2, f64::to_bits(3.5));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 6.5);
    }

    // ── FMUL ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fmul_double() {
        let mut t = test_interp(&[enc_fmul(true, 0, 1, 2)]);
        t.interp.cpu.write_d(1, f64::to_bits(3.0));
        t.interp.cpu.write_d(2, f64::to_bits(4.0));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 12.0);
    }

    // ── FDIV ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fdiv_double() {
        let mut t = test_interp(&[enc_fdiv(true, 0, 1, 2)]);
        t.interp.cpu.write_d(1, f64::to_bits(9.0));
        t.interp.cpu.write_d(2, f64::to_bits(2.0));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 4.5);
    }

    // ── FCMP ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fcmp_equal() {
        let mut t = test_interp(&[enc_fcmp(true, 0, 1)]);
        t.interp.cpu.write_d(0, f64::to_bits(5.0));
        t.interp.cpu.write_d(1, f64::to_bits(5.0));
        let cpu = t.run_ok();
        // Equal: N=0, Z=1, C=1, V=0
        assert!(!cpu.nzcv.n);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    #[test]
    fn test_exec_fcmp_less_than() {
        let mut t = test_interp(&[enc_fcmp(true, 0, 1)]);
        t.interp.cpu.write_d(0, f64::to_bits(1.0));
        t.interp.cpu.write_d(1, f64::to_bits(5.0));
        let cpu = t.run_ok();
        // a < b: N=1, Z=0, C=0, V=0
        assert!(cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
        assert!(!cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }
}
