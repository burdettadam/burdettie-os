//! FP data processing instruction execution.

use super::{Interpreter, StepResult};
use crate::decode::Instruction;

/// Round f64 according to ARM FRINT mode.
/// mode: 0=N (ties-even), 1=P (+inf), 2=M (-inf), 3=Z (toward zero),
///       4=A (ties-away), 6=X (exact, raise inexact), 7=I (current rounding → ties-even)
fn fp_round_f64(val: f64, mode: u8) -> f64 {
    match mode {
        0 | 7 => val.round_ties_even(), // FRINTN / FRINTI
        1 => val.ceil(),                // FRINTP
        2 => val.floor(),               // FRINTM
        3 => val.trunc(),               // FRINTZ
        4 => {
            // FRINTA (ties away from zero)
            if val.fract().abs() == 0.5 {
                if val > 0.0 {
                    val.ceil()
                } else {
                    val.floor()
                }
            } else {
                val.round_ties_even()
            }
        }
        6 => val.round_ties_even(), // FRINTX (same as N for our purposes)
        _ => val,
    }
}

/// Round f32 according to ARM FRINT mode.
fn fp_round_f32(val: f32, mode: u8) -> f32 {
    match mode {
        0 | 7 => val.round_ties_even(),
        1 => val.ceil(),
        2 => val.floor(),
        3 => val.trunc(),
        4 => {
            if val.fract().abs() == 0.5 {
                if val > 0.0 {
                    val.ceil()
                } else {
                    val.floor()
                }
            } else {
                val.round_ties_even()
            }
        }
        6 => val.round_ties_even(),
        _ => val,
    }
}

impl Interpreter {
    pub(super) fn exec_fp_dp(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
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

            Instruction::Scvtf {
                sf_fp,
                sf_int,
                rd,
                rn,
            } => {
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
            Instruction::Ucvtf {
                sf_fp,
                sf_int,
                rd,
                rn,
            } => {
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
            Instruction::Fcvtzs {
                sf_int,
                sf_fp,
                rd,
                rn,
            } => {
                if sf_fp {
                    // from double
                    let f = f64::from_bits(self.cpu.read_d(rn));
                    let val = if sf_int {
                        f as i64 as u64
                    } else {
                        f as i32 as u32 as u64
                    };
                    self.cpu.write_reg(rd, val);
                } else {
                    // from single
                    let f = f32::from_bits(self.cpu.read_s(rn));
                    let val = if sf_int {
                        f as i64 as u64
                    } else {
                        f as i32 as u32 as u64
                    };
                    self.cpu.write_reg(rd, val);
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fcvtzu {
                sf_int,
                sf_fp,
                rd,
                rn,
            } => {
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
            Instruction::Fabd { sf, rd, rn, rm } => {
                if sf {
                    let a = f64::from_bits(self.cpu.read_d(rn));
                    let b = f64::from_bits(self.cpu.read_d(rm));
                    self.cpu.write_d(rd, (a - b).abs().to_bits());
                } else {
                    let a = f32::from_bits(self.cpu.read_s(rn));
                    let b = f32::from_bits(self.cpu.read_s(rm));
                    self.cpu.write_s(rd, (a - b).abs().to_bits());
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
            Instruction::Fcsel {
                sf,
                rd,
                rn,
                rm,
                cond,
            } => {
                let take = cond.eval(self.cpu.nzcv);
                if sf {
                    let val = if take {
                        self.cpu.read_d(rn)
                    } else {
                        self.cpu.read_d(rm)
                    };
                    self.cpu.write_d(rd, val);
                } else {
                    let val = if take {
                        self.cpu.read_s(rn)
                    } else {
                        self.cpu.read_s(rm)
                    };
                    self.cpu.write_s(rd, val);
                }
                self.cpu.pc = pc + 4;
            }

            // ── FP 3-source (fused multiply-add) ───────────────────
            Instruction::Fmadd { sf, rd, rn, rm, ra } => {
                if sf {
                    let n = f64::from_bits(self.cpu.read_d(rn));
                    let m = f64::from_bits(self.cpu.read_d(rm));
                    let a = f64::from_bits(self.cpu.read_d(ra));
                    self.cpu.write_d(rd, n.mul_add(m, a).to_bits());
                } else {
                    let n = f32::from_bits(self.cpu.read_s(rn));
                    let m = f32::from_bits(self.cpu.read_s(rm));
                    let a = f32::from_bits(self.cpu.read_s(ra));
                    self.cpu.write_s(rd, n.mul_add(m, a).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fmsub { sf, rd, rn, rm, ra } => {
                if sf {
                    let n = f64::from_bits(self.cpu.read_d(rn));
                    let m = f64::from_bits(self.cpu.read_d(rm));
                    let a = f64::from_bits(self.cpu.read_d(ra));
                    self.cpu.write_d(rd, (-n).mul_add(m, a).to_bits());
                } else {
                    let n = f32::from_bits(self.cpu.read_s(rn));
                    let m = f32::from_bits(self.cpu.read_s(rm));
                    let a = f32::from_bits(self.cpu.read_s(ra));
                    self.cpu.write_s(rd, (-n).mul_add(m, a).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fnmadd { sf, rd, rn, rm, ra } => {
                // Dd = -(Da + Dn * Dm) = -Dn.mul_add(Dm, Da)
                if sf {
                    let n = f64::from_bits(self.cpu.read_d(rn));
                    let m = f64::from_bits(self.cpu.read_d(rm));
                    let a = f64::from_bits(self.cpu.read_d(ra));
                    self.cpu.write_d(rd, (-n.mul_add(m, a)).to_bits());
                } else {
                    let n = f32::from_bits(self.cpu.read_s(rn));
                    let m = f32::from_bits(self.cpu.read_s(rm));
                    let a = f32::from_bits(self.cpu.read_s(ra));
                    self.cpu.write_s(rd, (-n.mul_add(m, a)).to_bits());
                }
                self.cpu.pc = pc + 4;
            }
            Instruction::Fnmsub { sf, rd, rn, rm, ra } => {
                // Dd = -(Da - Dn * Dm) = Dn.mul_add(Dm, -Da)
                if sf {
                    let n = f64::from_bits(self.cpu.read_d(rn));
                    let m = f64::from_bits(self.cpu.read_d(rm));
                    let a = f64::from_bits(self.cpu.read_d(ra));
                    self.cpu.write_d(rd, n.mul_add(m, -a).to_bits());
                } else {
                    let n = f32::from_bits(self.cpu.read_s(rn));
                    let m = f32::from_bits(self.cpu.read_s(rm));
                    let a = f32::from_bits(self.cpu.read_s(ra));
                    self.cpu.write_s(rd, n.mul_add(m, -a).to_bits());
                }
                self.cpu.pc = pc + 4;
            }

            // ── FP rounding ────────────────────────────────────────
            Instruction::Frint { sf, rd, rn, mode } => {
                if sf {
                    let val = f64::from_bits(self.cpu.read_d(rn));
                    let result = fp_round_f64(val, mode);
                    self.cpu.write_d(rd, result.to_bits());
                } else {
                    let val = f32::from_bits(self.cpu.read_s(rn));
                    let result = fp_round_f32(val, mode);
                    self.cpu.write_s(rd, result.to_bits());
                }
                self.cpu.pc = pc + 4;
            }

            _ => unreachable!("exec_fp called with non-FP instruction"),
        }
        StepResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::*;

    // ── FNEG ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fneg_double() {
        // FNEG D0, D1
        let mut t = test_interp(&[enc_fneg(true, 0, 1)]);
        t.interp.cpu.write_d(1, f64::to_bits(5.0));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), -5.0);
    }

    #[test]
    fn test_exec_fneg_single() {
        // FNEG S0, S1
        let mut t = test_interp(&[enc_fneg(false, 0, 1)]);
        t.interp.cpu.write_s(1, f32::to_bits(3.5));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), -3.5);
    }

    // ── FABS ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fabs_double() {
        // FABS D0, D1 — manual encoding 0x1E60C020
        let mut t = test_interp(&[0x1E60C020]);
        t.interp.cpu.write_d(1, f64::to_bits(-7.25));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 7.25);
    }

    // ── FSQRT ───────────────────────────────────────────────────

    #[test]
    fn test_exec_fsqrt_double() {
        // FSQRT D0, D1 — manual encoding 0x1E61C020
        let mut t = test_interp(&[0x1E61C020]);
        t.interp.cpu.write_d(1, f64::to_bits(9.0));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 3.0);
    }

    #[test]
    fn test_exec_fabd_single() {
        let mut t = test_interp(&[enc_fabd(false, 0, 1, 2)]);
        t.interp.cpu.write_s(1, f32::to_bits(7.25));
        t.interp.cpu.write_s(2, f32::to_bits(2.0));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 5.25);
    }

    // ── FADD ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fadd_single() {
        // FADD S0, S1, S2
        let mut t = test_interp(&[enc_fadd(false, 0, 1, 2)]);
        t.interp.cpu.write_s(1, f32::to_bits(1.5));
        t.interp.cpu.write_s(2, f32::to_bits(2.25));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 3.75);
    }

    // ── FSUB ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fsub_single() {
        // FSUB S0, S1, S2
        let mut t = test_interp(&[enc_fsub(false, 0, 1, 2)]);
        t.interp.cpu.write_s(1, f32::to_bits(10.0));
        t.interp.cpu.write_s(2, f32::to_bits(3.5));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 6.5);
    }

    // ── FMUL ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fmul_single() {
        // FMUL S0, S1, S2
        let mut t = test_interp(&[enc_fmul(false, 0, 1, 2)]);
        t.interp.cpu.write_s(1, f32::to_bits(4.0));
        t.interp.cpu.write_s(2, f32::to_bits(2.5));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 10.0);
    }

    // ── FDIV ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fdiv_single() {
        // FDIV S0, S1, S2
        let mut t = test_interp(&[enc_fdiv(false, 0, 1, 2)]);
        t.interp.cpu.write_s(1, f32::to_bits(9.0));
        t.interp.cpu.write_s(2, f32::to_bits(4.0));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 2.25);
    }

    // ── FCMP ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fcmp_equal() {
        // FCMP D0, D1 where D0 == D1 → Z=1, C=1
        let mut t = test_interp(&[enc_fcmp(true, 0, 1)]);
        t.interp.cpu.write_d(0, f64::to_bits(42.0));
        t.interp.cpu.write_d(1, f64::to_bits(42.0));
        let cpu = t.run_ok();
        assert!(!cpu.nzcv.n);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    #[test]
    fn test_exec_fcmp_less() {
        // FCMP D0, D1 where D0 < D1 → N=1
        let mut t = test_interp(&[enc_fcmp(true, 0, 1)]);
        t.interp.cpu.write_d(0, f64::to_bits(1.0));
        t.interp.cpu.write_d(1, f64::to_bits(5.0));
        let cpu = t.run_ok();
        assert!(cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
        assert!(!cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    #[test]
    fn test_exec_fcmp_greater() {
        // FCMP D0, D1 where D0 > D1 → C=1, Z=0
        let mut t = test_interp(&[enc_fcmp(true, 0, 1)]);
        t.interp.cpu.write_d(0, f64::to_bits(10.0));
        t.interp.cpu.write_d(1, f64::to_bits(2.0));
        let cpu = t.run_ok();
        assert!(!cpu.nzcv.n);
        assert!(!cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    // ── FcmpZero ────────────────────────────────────────────────

    #[test]
    fn test_exec_fcmp_zero() {
        // FCMP D1, #0.0 — manual encoding 0x1E602028
        let mut t = test_interp(&[0x1E602028]);
        t.interp.cpu.write_d(1, f64::to_bits(0.0));
        let cpu = t.run_ok();
        assert!(!cpu.nzcv.n);
        assert!(cpu.nzcv.z);
        assert!(cpu.nzcv.c);
        assert!(!cpu.nzcv.v);
    }

    // ── FCVT ────────────────────────────────────────────────────

    #[test]
    fn test_exec_fcvt_single_to_double() {
        // FCVT D0, S1 — manual encoding 0x1E22C020
        let mut t = test_interp(&[0x1E22C020]);
        t.interp.cpu.write_s(1, f32::to_bits(2.5));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 2.5);
    }

    #[test]
    fn test_exec_fcvt_double_to_single() {
        // FCVT S0, D1 — manual encoding 0x1E624020
        let mut t = test_interp(&[0x1E624020]);
        t.interp.cpu.write_d(1, f64::to_bits(1.75));
        let cpu = t.run_ok();
        assert_eq!(f32::from_bits(cpu.read_s(0)), 1.75);
    }

    // ── FCSEL ───────────────────────────────────────────────────

    #[test]
    fn test_exec_fcsel_cond_true() {
        // FCSEL D0, D1, D2, EQ — manual encoding 0x1E620C20
        // Set Z=1 so EQ is true → should select D1
        let mut t = test_interp(&[0x1E620C20]);
        t.interp.cpu.nzcv.z = true;
        t.interp.cpu.nzcv.c = true;
        t.interp.cpu.write_d(1, f64::to_bits(99.0));
        t.interp.cpu.write_d(2, f64::to_bits(0.0));
        let cpu = t.run_ok();
        assert_eq!(f64::from_bits(cpu.read_d(0)), 99.0);
    }
}
