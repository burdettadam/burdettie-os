//! SIMD vector instruction execution.

use super::{Interpreter, StepResult};
use crate::decode::Instruction;

impl Interpreter {
    pub(super) fn exec_simd(&mut self, insn: Instruction, pc: u64) -> StepResult {
        match insn {
            Instruction::SimdDupGeneral { q, size, rd, rn } => {
                let val = self.cpu.read_reg(rn);
                let elem_bytes = 1u32 << size;
                let elem_bits = elem_bytes * 8;
                let mask = if elem_bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << elem_bits) - 1
                };
                let elem = val & mask;
                let total_bytes = if q { 16 } else { 8 };
                let count = total_bytes / elem_bytes;
                let mut result = 0u128;
                for i in 0..count {
                    result |= (elem as u128) << (i * elem_bits);
                }
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdDupElement { q, imm5, rd, rn } => {
                let (size, index) = simd_decode_imm5(imm5);
                let src = self.cpu.v[rn as usize];
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 128 {
                    u128::MAX
                } else {
                    (1u128 << elem_bits) - 1
                };
                let elem = (src >> (index * elem_bits)) & mask;
                let total_bytes = if q { 16u32 } else { 8 };
                let count = total_bytes / (1u32 << size);
                let mut result = 0u128;
                for i in 0..count {
                    result |= elem << (i * elem_bits);
                }
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdInsGeneral { imm5, rd, rn } => {
                let (size, index) = simd_decode_imm5(imm5);
                let val = self.cpu.read_reg(rn);
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 128 {
                    u128::MAX
                } else {
                    (1u128 << elem_bits) - 1
                };
                let shift = index * elem_bits;
                let mut v = self.cpu.v[rd as usize];
                v &= !(mask << shift);
                v |= ((val as u128) & mask) << shift;
                self.cpu.v[rd as usize] = v;
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdInsElement { imm5, imm4, rd, rn } => {
                let (size, dst_idx) = simd_decode_imm5(imm5);
                let src_idx = imm4 as u32 >> size;
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 128 {
                    u128::MAX
                } else {
                    (1u128 << elem_bits) - 1
                };
                let src_val = (self.cpu.v[rn as usize] >> (src_idx * elem_bits)) & mask;
                let shift = dst_idx * elem_bits;
                let mut v = self.cpu.v[rd as usize];
                v &= !(mask << shift);
                v |= src_val << shift;
                self.cpu.v[rd as usize] = v;
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdUmov { q: _, imm5, rd, rn } => {
                let (size, index) = simd_decode_imm5(imm5);
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << elem_bits) - 1
                };
                let val = ((self.cpu.v[rn as usize] >> (index * elem_bits)) as u64) & mask;
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdSmov { q, imm5, rd, rn } => {
                let (size, index) = simd_decode_imm5(imm5);
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << elem_bits) - 1
                };
                let raw_val = ((self.cpu.v[rn as usize] >> (index * elem_bits)) as u64) & mask;
                // Sign-extend
                let sign_bit = 1u64 << (elem_bits - 1);
                let val = if raw_val & sign_bit != 0 {
                    raw_val | !mask
                } else {
                    raw_val
                };
                let val = if q { val } else { val & 0xFFFF_FFFF };
                self.cpu.write_reg(rd, val);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdMovi { q, rd, imm } => {
                // Simplified: replicate imm across all bytes
                let byte = (imm & 0xFF) as u128;
                let mut result = 0u128;
                let total = if q { 16 } else { 8 };
                for i in 0..total {
                    result |= byte << (i * 8);
                }
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdAdd {
                q,
                size,
                rd,
                rn,
                rm,
            } => {
                simd_binop(self, q, size, rd, rn, rm, |a, b, _| a.wrapping_add(b));
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdSub {
                q,
                size,
                rd,
                rn,
                rm,
            } => {
                simd_binop(self, q, size, rd, rn, rm, |a, b, _| a.wrapping_sub(b));
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdAnd { q, rd, rn, rm } => {
                let a = self.cpu.v[rn as usize];
                let b = self.cpu.v[rm as usize];
                let result = a & b;
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdOrr { q, rd, rn, rm } => {
                let a = self.cpu.v[rn as usize];
                let b = self.cpu.v[rm as usize];
                let result = a | b;
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdEor { q, rd, rn, rm } => {
                let a = self.cpu.v[rn as usize];
                let b = self.cpu.v[rm as usize];
                let result = a ^ b;
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdCmp {
                q,
                size,
                rd,
                rn,
                rm,
                op,
            } => {
                simd_binop(self, q, size, rd, rn, rm, |a, b, elem_bits| {
                    let mask = if elem_bits >= 128 {
                        u128::MAX
                    } else {
                        (1u128 << elem_bits) - 1
                    };
                    let result = match op {
                        0 => a == b, // CMEQ
                        1 => {
                            // CMGT (signed)
                            let sa = sign_extend_elem(a, elem_bits);
                            let sb = sign_extend_elem(b, elem_bits);
                            sa > sb
                        }
                        2 => {
                            // CMGE (signed)
                            let sa = sign_extend_elem(a, elem_bits);
                            let sb = sign_extend_elem(b, elem_bits);
                            sa >= sb
                        }
                        3 => a > b,  // CMHI (unsigned)
                        4 => a >= b, // CMHS (unsigned)
                        _ => false,
                    };
                    if result {
                        mask
                    } else {
                        0
                    }
                });
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdCnt { q, rd, rn } => {
                // Count bits in each byte
                let src = self.cpu.v[rn as usize];
                let total = if q { 16u32 } else { 8 };
                let mut result = 0u128;
                for i in 0..total {
                    let byte = ((src >> (i * 8)) & 0xFF) as u8;
                    result |= (byte.count_ones() as u128) << (i * 8);
                }
                self.cpu.write_v(rd, result, q);
                self.cpu.pc = pc + 4;
            }
            Instruction::SimdAddv { q, size, rd, rn } => {
                let src = self.cpu.v[rn as usize];
                let elem_bits = (1u32 << size) * 8;
                let mask = if elem_bits >= 128 {
                    u128::MAX
                } else {
                    (1u128 << elem_bits) - 1
                };
                let total_bytes = if q { 16u32 } else { 8 };
                let count = total_bytes / (1u32 << size);
                let mut sum = 0u128;
                for i in 0..count {
                    let elem = (src >> (i * elem_bits)) & mask;
                    sum = (sum + elem) & mask;
                }
                // Result goes into the scalar register (lowest element), upper bits zero
                self.cpu.v[rd as usize] = sum;
                self.cpu.pc = pc + 4;
            }

            _ => unreachable!("exec_simd called with non-SIMD instruction"),
        }
        StepResult::Continue
    }
}

/// Decode imm5 into (size, index) for SIMD copy instructions.
/// The lowest set bit of imm5 determines element size:
///   imm5[0]=1 → B (size=0, index=imm5[4:1])
///   imm5[1]=1 → H (size=1, index=imm5[4:2])
///   imm5[2]=1 → S (size=2, index=imm5[4:3])
///   imm5[3]=1 → D (size=3, index=imm5[4])
fn simd_decode_imm5(imm5: u8) -> (u32, u32) {
    if imm5 & 1 != 0 {
        (0, (imm5 >> 1) as u32)
    } else if imm5 & 2 != 0 {
        (1, (imm5 >> 2) as u32)
    } else if imm5 & 4 != 0 {
        (2, (imm5 >> 3) as u32)
    } else if imm5 & 8 != 0 {
        (3, (imm5 >> 4) as u32)
    } else {
        (0, 0) // fallback
    }
}

/// Sign-extend a value from `bits` width to i128.
fn sign_extend_elem(val: u128, bits: u32) -> i128 {
    if bits >= 128 {
        return val as i128;
    }
    let sign_bit = 1u128 << (bits - 1);
    if val & sign_bit != 0 {
        (val | !((1u128 << bits) - 1)) as i128
    } else {
        val as i128
    }
}

/// Apply a binary operation element-wise across SIMD vectors.
fn simd_binop<F>(interp: &mut Interpreter, q: bool, size: u8, rd: u8, rn: u8, rm: u8, op: F)
where
    F: Fn(u128, u128, u32) -> u128,
{
    let a = interp.cpu.v[rn as usize];
    let b = interp.cpu.v[rm as usize];
    let elem_bits = (1u32 << size) * 8;
    let mask = if elem_bits >= 128 {
        u128::MAX
    } else {
        (1u128 << elem_bits) - 1
    };
    let total_bytes = if q { 16u32 } else { 8 };
    let count = total_bytes / (1u32 << size);
    let mut result = 0u128;
    for i in 0..count {
        let ea = (a >> (i * elem_bits)) & mask;
        let eb = (b >> (i * elem_bits)) & mask;
        let er = op(ea, eb, elem_bits) & mask;
        result |= er << (i * elem_bits);
    }
    interp.cpu.write_v(rd, result, q);
}
