//! Load/store instruction decoding.

use super::util::{sign_extend, sign_extend7, sign_extend9};
use super::Instruction;

pub(super) fn decode_load_store(raw: u32, pc: u64) -> Instruction {
    // LDR (literal): opc 01 1 V 00 imm19 Rt
    if (raw >> 24) & 0x3F == 0b011000 || (raw >> 24) & 0x3F == 0b011001 {
        let top2 = (raw >> 30) & 0x3;
        let v = (raw >> 26) & 1;
        let rt = (raw & 0x1F) as u8;
        let imm19 = ((raw >> 5) & 0x7FFFF) as u64;
        let offset = sign_extend(imm19, 19) << 2;
        if v == 1 {
            // FP/SIMD literal: opc 01 1 1 00 imm19 Rt
            // opc: 00=S(32), 01=D(64), 10=Q(128)
            let size = match top2 {
                0b00 => 4u8,
                0b01 => 8,
                0b10 => 16,
                _ => return Instruction::Unknown { raw },
            };
            return Instruction::FpLdrLiteral {
                size,
                rt,
                imm: pc as i64 + offset,
            };
        }
        if (raw >> 24) & 0xFF == 0b00011000 || (raw >> 24) & 0xFF == 0b01011000 {
            let sf = top2 == 0b01;
            return Instruction::LdrLiteral {
                sf,
                rt,
                imm: pc as i64 + offset,
            };
        }
    }

    // Load/store exclusive, acquire-release, and CAS: bits[29:24] = 001000
    if (raw >> 24) & 0x3F == 0b001000 {
        return decode_exclusive_group(raw);
    }

    // LSE atomics (ARMv8.1): bits[29:24] = 111000, bit[21] = 1, bits[11:10] = 00
    // Register-indexed LDR/STR also uses bits[29:24]=111000, bit[21]=1 but has bits[11:10]=10.
    // CAS uses bits[29:24]=001000, handled by decode_exclusive_group.
    if (raw >> 24) & 0x3F == 0b111000 && (raw >> 21) & 1 == 1 && (raw >> 10) & 0x3 == 0b00 {
        if let Some(insn) = decode_lse_atomics(raw) {
            return insn;
        }
    }

    // Load/store pair: opc 101 V 0xx L ... → bits[29:27] == 0b101
    let is_pair = (raw >> 27) & 0x7 == 0b101;
    if is_pair {
        let v = (raw >> 26) & 1;
        if v == 1 {
            return decode_fp_load_store_pair(raw);
        }
        return decode_load_store_pair(raw);
    }

    // Load/store register (unsigned immediate, unscaled, pre/post-index)
    decode_load_store_reg(raw)
}

fn decode_load_store_pair(raw: u32) -> Instruction {
    let sf = (raw >> 31) & 1 == 1;
    let l = (raw >> 22) & 1; // 1=load, 0=store
    let idx = (raw >> 23) & 0x3; // 01=post, 11=pre, 10=signed offset
    let imm7 = ((raw >> 15) & 0x7F) as i16;
    let rt2 = ((raw >> 10) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    // Sign-extend 7-bit immediate, scale by register size
    let scale = if sf { 8 } else { 4 };
    let offset = sign_extend7(imm7) * scale;

    match (l, idx) {
        (1, 0b01) => Instruction::LdpPost {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (1, 0b11) => Instruction::LdpPre {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (1, 0b10) => Instruction::Ldp {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b01) => Instruction::StpPost {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b11) => Instruction::StpPre {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b10) => Instruction::Stp {
            sf,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        _ => Instruction::Unknown { raw },
    }
}

fn decode_fp_load_store_pair(raw: u32) -> Instruction {
    let opc = (raw >> 30) & 0x3; // 00=S(32-bit), 01=D(64-bit), 10=Q(128-bit)
    let l = (raw >> 22) & 1; // 1=load, 0=store
    let idx = (raw >> 23) & 0x3; // 01=post, 11=pre, 10=signed offset
    let imm7 = ((raw >> 15) & 0x7F) as i16;
    let rt2 = ((raw >> 10) & 0x1F) as u8;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    let (size, scale): (u8, i16) = match opc {
        0b00 => (4, 4),   // S registers
        0b01 => (8, 8),   // D registers
        0b10 => (16, 16), // Q registers
        _ => return Instruction::Unknown { raw },
    };
    let offset = sign_extend7(imm7) * scale;

    match (l, idx) {
        (1, 0b01) => Instruction::FpLdpPost {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (1, 0b11) => Instruction::FpLdpPre {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (1, 0b10) => Instruction::FpLdp {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b01) => Instruction::FpStpPost {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b11) => Instruction::FpStpPre {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        (0, 0b10) => Instruction::FpStp {
            size,
            rt,
            rt2,
            rn,
            imm: offset,
        },
        _ => Instruction::Unknown { raw },
    }
}

pub(super) fn decode_load_store_reg(raw: u32) -> Instruction {
    let size = (raw >> 30) & 0x3;
    let v = (raw >> 26) & 1; // SIMD/FP
    let opc = (raw >> 22) & 0x3;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;

    if v == 1 {
        return decode_fp_load_store_reg(raw, size, opc, rn, rt);
    }

    // Unsigned offset: size 11 1 0 01 opc imm12 Rn Rt
    if (raw >> 24) & 0x3F == 0b111001
        || (raw >> 24) & 0x3F == 0b101001
        || (raw >> 24) & 0x3F == 0b011001
        || (raw >> 24) & 0x3F == 0b001001
    {
        let imm12 = (raw >> 10) & 0xFFF;
        let scale = 1u32 << size;
        let offset = imm12 * scale;

        return match (size, opc) {
            (0b11, 0b01) => Instruction::LdrImm {
                sf: true,
                rt,
                rn,
                imm: offset,
            },
            (0b11, 0b00) => Instruction::StrImm {
                sf: true,
                rt,
                rn,
                imm: offset,
            },
            (0b10, 0b01) => Instruction::LdrImm {
                sf: false,
                rt,
                rn,
                imm: offset,
            },
            (0b10, 0b00) => Instruction::StrImm {
                sf: false,
                rt,
                rn,
                imm: offset,
            },
            (0b10, 0b10) => Instruction::Ldrsw {
                rt,
                rn,
                imm: offset as i32,
            },
            (0b00, 0b01) => Instruction::Ldrb {
                rt,
                rn,
                imm: offset as i32,
            },
            (0b00, 0b00) => Instruction::Strb {
                rt,
                rn,
                imm: offset as i32,
            },
            (0b01, 0b01) => Instruction::Ldrh {
                rt,
                rn,
                imm: offset as i32,
            },
            (0b01, 0b00) => Instruction::Strh {
                rt,
                rn,
                imm: offset as i32,
            },
            // Signed byte/halfword loads
            (0b00, 0b10) => Instruction::Ldrsb {
                rt,
                rn,
                imm: offset as i32,
                sf: true,
            }, // LDRSB Xt
            (0b00, 0b11) => Instruction::Ldrsb {
                rt,
                rn,
                imm: offset as i32,
                sf: false,
            }, // LDRSB Wt
            (0b01, 0b10) => Instruction::Ldrsh {
                rt,
                rn,
                imm: offset as i32,
                sf: true,
            }, // LDRSH Xt
            (0b01, 0b11) => Instruction::Ldrsh {
                rt,
                rn,
                imm: offset as i32,
                sf: false,
            }, // LDRSH Wt
            // PRFM (prefetch memory): size=11, opc=10 — treated as NOP
            (0b11, 0b10) => Instruction::Prfm,
            _ => Instruction::Unknown { raw },
        };
    }

    // Unscaled / pre-index / post-index: size 11 1 0 00 opc 0 imm9 idx Rn Rt
    // Guard: bit[21]==0 excludes register-indexed encodings (handled below)
    if ((raw >> 21) & 1) == 0
        && ((raw >> 24) & 0x3F == 0b111000
            || (raw >> 24) & 0x3F == 0b101000
            || (raw >> 24) & 0x3F == 0b011000
            || (raw >> 24) & 0x3F == 0b001000)
    {
        let imm9 = ((raw >> 12) & 0x1FF) as i16;
        let imm = sign_extend9(imm9);
        let idx = (raw >> 10) & 0x3;
        let sf = size == 0b11;

        let is_load = opc == 0b01;
        let is_signed_load = opc >= 0b10; // LDRSW, LDRSB, LDRSH
        let is_sw = size == 0b10 && opc == 0b10; // LDRSW

        return match idx {
            0b00 => {
                // Unscaled
                if is_sw {
                    Instruction::Ldrsw {
                        rt,
                        rn,
                        imm: imm as i32,
                    }
                } else if is_signed_load {
                    // LDURSB / LDURSH (unscaled signed byte/halfword)
                    match (size, opc) {
                        (0b00, 0b10) => Instruction::Ldrsb {
                            rt,
                            rn,
                            imm: imm as i32,
                            sf: true,
                        },
                        (0b00, 0b11) => Instruction::Ldrsb {
                            rt,
                            rn,
                            imm: imm as i32,
                            sf: false,
                        },
                        (0b01, 0b10) => Instruction::Ldrsh {
                            rt,
                            rn,
                            imm: imm as i32,
                            sf: true,
                        },
                        (0b01, 0b11) => Instruction::Ldrsh {
                            rt,
                            rn,
                            imm: imm as i32,
                            sf: false,
                        },
                        _ => Instruction::Unknown { raw },
                    }
                } else if is_load {
                    match size {
                        0b00 => Instruction::Ldrb {
                            rt,
                            rn,
                            imm: imm as i32,
                        },
                        0b01 => Instruction::Ldrh {
                            rt,
                            rn,
                            imm: imm as i32,
                        },
                        _ => Instruction::Ldur { sf, rt, rn, imm },
                    }
                } else {
                    match size {
                        0b00 => Instruction::Strb {
                            rt,
                            rn,
                            imm: imm as i32,
                        },
                        0b01 => Instruction::Strh {
                            rt,
                            rn,
                            imm: imm as i32,
                        },
                        _ => Instruction::Stur { sf, rt, rn, imm },
                    }
                }
            }
            0b01 => {
                // Post-index
                match (size, opc) {
                    (0b00, 0b00) => Instruction::StrbPost { rt, rn, imm },
                    (0b00, 0b01) => Instruction::LdrbPost { rt, rn, imm },
                    (0b00, 0b10) => Instruction::LdrsbPost {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b00, 0b11) => Instruction::LdrsbPost {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b01, 0b00) => Instruction::StrhPost { rt, rn, imm },
                    (0b01, 0b01) => Instruction::LdrhPost { rt, rn, imm },
                    (0b01, 0b10) => Instruction::LdrshPost {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b01, 0b11) => Instruction::LdrshPost {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b00) => Instruction::StrPost {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b01) => Instruction::LdrPost {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b10) => Instruction::LdrswPost { rt, rn, imm },
                    (0b11, 0b00) => Instruction::StrPost {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b11, 0b01) => Instruction::LdrPost {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    _ => Instruction::Unknown { raw },
                }
            }
            0b11 => {
                // Pre-index
                match (size, opc) {
                    (0b00, 0b00) => Instruction::StrbPre { rt, rn, imm },
                    (0b00, 0b01) => Instruction::LdrbPre { rt, rn, imm },
                    (0b00, 0b10) => Instruction::LdrsbPre {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b00, 0b11) => Instruction::LdrsbPre {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b01, 0b00) => Instruction::StrhPre { rt, rn, imm },
                    (0b01, 0b01) => Instruction::LdrhPre { rt, rn, imm },
                    (0b01, 0b10) => Instruction::LdrshPre {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b01, 0b11) => Instruction::LdrshPre {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b00) => Instruction::StrPre {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b01) => Instruction::LdrPre {
                        sf: false,
                        rt,
                        rn,
                        imm,
                    },
                    (0b10, 0b10) => Instruction::LdrswPre { rt, rn, imm },
                    (0b11, 0b00) => Instruction::StrPre {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    (0b11, 0b01) => Instruction::LdrPre {
                        sf: true,
                        rt,
                        rn,
                        imm,
                    },
                    _ => Instruction::Unknown { raw },
                }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Register offset: size 11 1 0 00 opc 1 Rm option S 10 Rn Rt
    // bits[11:10] == 0b10 (register offset), bit[21] == 1
    if ((raw >> 21) & 1) == 1 && ((raw >> 10) & 0x3) == 0b10 {
        let rm = ((raw >> 16) & 0x1F) as u8;
        let extend = ((raw >> 13) & 0x7) as u8; // option field
        let s = ((raw >> 12) & 1) == 1; // shift flag
        return match (size, opc) {
            (0b11, 0b01) => Instruction::LdrReg {
                sf: true,
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b11, 0b00) => Instruction::StrReg {
                sf: true,
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b10, 0b01) => Instruction::LdrReg {
                sf: false,
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b10, 0b00) => Instruction::StrReg {
                sf: false,
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b10, 0b10) => Instruction::LdrswReg {
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b00, 0b01) => Instruction::LdrbReg {
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b00, 0b00) => Instruction::StrbReg {
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b01, 0b01) => Instruction::LdrhReg {
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b01, 0b00) => Instruction::StrhReg {
                rt,
                rn,
                rm,
                extend,
                s,
            },
            (0b00, 0b10) => Instruction::LdrsbReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf: true,
            },
            (0b00, 0b11) => Instruction::LdrsbReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf: false,
            },
            (0b01, 0b10) => Instruction::LdrshReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf: true,
            },
            (0b01, 0b11) => Instruction::LdrshReg {
                rt,
                rn,
                rm,
                extend,
                s,
                sf: false,
            },
            _ => Instruction::Unknown { raw },
        };
    }

    Instruction::Unknown { raw }
}

/// Decode SIMD/FP scalar load/store (v=1 path from decode_load_store_reg).
///
/// size:opc determines the register width:
///   size=00, opc=0x → B (8-bit)
///   size=01, opc=0x → H (16-bit)
///   size=10, opc=0x → S (32-bit)
///   size=11, opc=0x → D (64-bit)
///   size=00, opc=1x → Q (128-bit)
fn decode_fp_load_store_reg(raw: u32, size: u32, opc: u32, rn: u8, rt: u8) -> Instruction {
    // Compute access size in bytes
    let access_size: u8 = if opc & 0b10 != 0 {
        // opc bit[1]=1 → 128-bit (Q)
        16
    } else {
        1u8 << size // 1,2,4,8
    };
    let is_load = opc & 1 == 1;
    let log2_sz = match access_size {
        1 => 0u32,
        2 => 1,
        4 => 2,
        8 => 3,
        16 => 4,
        _ => 0,
    };

    // Unsigned offset: size 11 1 1 01 opc imm12 Rn Rt
    if (raw >> 24) & 0x3F == 0b111101
        || (raw >> 24) & 0x3F == 0b101101
        || (raw >> 24) & 0x3F == 0b011101
        || (raw >> 24) & 0x3F == 0b001101
    {
        let imm12 = (raw >> 10) & 0xFFF;
        let offset = imm12 << log2_sz;
        return if is_load {
            Instruction::FpLdrImm {
                size: access_size,
                rt,
                rn,
                imm: offset,
            }
        } else {
            Instruction::FpStrImm {
                size: access_size,
                rt,
                rn,
                imm: offset,
            }
        };
    }

    // Unscaled/pre/post-index: size 11 1 1 00 opc 0 imm9 idx Rn Rt
    if (raw >> 24) & 0x3F == 0b111100
        || (raw >> 24) & 0x3F == 0b101100
        || (raw >> 24) & 0x3F == 0b011100
        || (raw >> 24) & 0x3F == 0b001100
    {
        let imm9 = ((raw >> 12) & 0x1FF) as i16;
        let imm = sign_extend9(imm9);
        let idx = (raw >> 10) & 0x3;

        return match idx {
            0b00 => {
                // Unscaled
                if is_load {
                    Instruction::FpLdur {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                } else {
                    Instruction::FpStur {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                }
            }
            0b01 => {
                if is_load {
                    Instruction::FpLdrPost {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                } else {
                    Instruction::FpStrPost {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                }
            }
            0b11 => {
                if is_load {
                    Instruction::FpLdrPre {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                } else {
                    Instruction::FpStrPre {
                        size: access_size,
                        rt,
                        rn,
                        imm,
                    }
                }
            }
            _ => Instruction::Unknown { raw },
        };
    }

    // Register offset: size 11 1 1 00 opc 1 Rm option S 10 Rn Rt
    if ((raw >> 21) & 1) == 1 && ((raw >> 10) & 0x3) == 0b10 {
        let rm = ((raw >> 16) & 0x1F) as u8;
        let extend = ((raw >> 13) & 0x7) as u8;
        let s = ((raw >> 12) & 1) == 1;
        return if is_load {
            Instruction::FpLdrReg {
                size: access_size,
                rt,
                rn,
                rm,
                extend,
                s,
            }
        } else {
            Instruction::FpStrReg {
                size: access_size,
                rt,
                rn,
                rm,
                extend,
                s,
            }
        };
    }

    Instruction::Unknown { raw }
}

/// Decode the load/store exclusive and acquire-release group.
///
/// All instructions in this group have bits[29:24] = 001000.
/// Discriminated by:
/// - bit[23] = o2  (0=exclusive, 1=non-exclusive or CAS)
/// - bit[22] = L   (1=load, 0=store)
/// - bit[21] = o1  (0=single register, 1=pair or CAS)
/// - bit[15] = o0  (acquire/release ordering for exclusive operations)
fn decode_exclusive_group(raw: u32) -> Instruction {
    let size = (raw >> 30) & 0x3; // 00=byte, 01=half, 10=word, 11=dword
    let o2 = (raw >> 23) & 1;
    let l = (raw >> 22) & 1; // 1=load, 0=store
    let o1 = (raw >> 21) & 1;
    let rs = ((raw >> 16) & 0x1F) as u8; // exclusive status or CAS compare reg
    let o0 = (raw >> 15) & 1;
    let rn = ((raw >> 5) & 0x1F) as u8;
    let rt = (raw & 0x1F) as u8;
    let sf = size == 0b11;

    // Single-register CAS variants. Pair and byte/halfword CAS encodings are
    // not represented by Instruction::Cas, so do not silently widen them.
    if o2 == 1 && o1 == 1 && size >= 0b10 {
        let acquire = l == 1;
        let release = o0 == 1;
        return Instruction::Cas {
            sf,
            acquire,
            release,
            rs,
            rt,
            rn,
        };
    }

    if o1 == 1 {
        return Instruction::Unknown { raw };
    }

    // LDAR / STLR: non-exclusive (o2=1), ordered (o0=1).
    if o2 == 1 {
        if o0 == 0 {
            return Instruction::Unknown { raw };
        }
        return match (size, l) {
            (0b11, 1) | (0b10, 1) => Instruction::Ldar { sf, rt, rn },
            (0b11, 0) | (0b10, 0) => Instruction::Stlr { sf, rt, rn },
            (0b00, 1) => Instruction::Ldarb { rt, rn },
            (0b00, 0) => Instruction::Stlrb { rt, rn },
            (0b01, 1) => Instruction::Ldarh { rt, rn },
            (0b01, 0) => Instruction::Stlrh { rt, rn },
            _ => Instruction::Unknown { raw },
        };
    }

    // LDXR / LDAXR: exclusive (o2=0), L=1, o0 selects acquire ordering.
    if l == 1 {
        let acquire = o0 == 1;
        return match size {
            0b11 | 0b10 => Instruction::LoadExclusive {
                sf,
                acquire,
                rt,
                rn,
            },
            0b00 => Instruction::LoadExclusiveByte { acquire, rt, rn },
            0b01 => Instruction::LoadExclusiveHalf { acquire, rt, rn },
            _ => Instruction::Unknown { raw },
        };
    }

    // STXR / STLXR: exclusive (o2=0), L=0, o0 selects release ordering.
    {
        let release = o0 == 1;
        match size {
            0b11 | 0b10 => Instruction::StoreExclusive {
                sf,
                release,
                rs,
                rt,
                rn,
            },
            0b00 => Instruction::StoreExclusiveByte {
                release,
                rs,
                rt,
                rn,
            },
            0b01 => Instruction::StoreExclusiveHalf {
                release,
                rs,
                rt,
                rn,
            },
            _ => Instruction::Unknown { raw },
        }
    }
}

/// Decode LSE atomic operations (ARMv8.1).
///
/// Requires bits[29:24] = 111000 AND bit[21] = 1.
/// Returns `None` if the encoding is not a recognized LSE atomic.
fn decode_lse_atomics(raw: u32) -> Option<Instruction> {
    let size = (raw >> 30) & 0x3; // 00=byte, 01=half, 10=word, 11=dword
    let a = (raw >> 23) & 1; // acquire
    let r = (raw >> 22) & 1; // release
                             // bit[21] = 1 already checked by caller
    let rs = ((raw >> 16) & 0x1F) as u8; // source register
    let opc = ((raw >> 12) & 0xF) as u8; // operation code
    let rn = ((raw >> 5) & 0x1F) as u8; // address register
    let rt = (raw & 0x1F) as u8; // result register

    let acquire = a == 1;
    let release = r == 1;

    // SWP: opc = 1000
    if opc == 0b1000 {
        return Some(Instruction::Swp {
            size: 1 << size,
            acquire,
            release,
            rs,
            rt,
            rn,
        });
    }

    // LDADD/LDCLR/LDEOR/LDSET/LDSMAX/LDSMIN/LDUMAX/LDUMIN: opc in 0b0000..0b0111
    if opc < 8 {
        // opc 0:LDADD, 1:LDCLR, 2:LDEOR, 3:LDSET, 4:LDSMAX, 5:LDSMIN, 6:LDUMAX, 7:LDUMIN
        return Some(Instruction::LseLoad {
            size: 1 << size,
            acquire,
            release,
            op: opc,
            rs,
            rt,
            rn,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::decode::{decode, Instruction};
    use crate::test_helpers::*;

    // 1. LdrImm 64-bit
    #[test]
    fn test_decode_ldr_imm_64() {
        let insn = decode(enc_ldr_imm64(3, 1, 16), 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrImm {
                sf: true,
                rt: 3,
                rn: 1,
                imm: 16
            }
        ));
    }

    // 2. LdrImm 32-bit
    #[test]
    fn test_decode_ldr_imm_32() {
        let insn = decode(enc_ldr_imm32(3, 1, 8), 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrImm {
                sf: false,
                rt: 3,
                rn: 1,
                imm: 8
            }
        ));
    }

    // 3. StrImm 64-bit
    #[test]
    fn test_decode_str_imm_64() {
        let insn = decode(enc_str_imm64(3, 1, 16), 0x1000);
        assert!(matches!(
            insn,
            Instruction::StrImm {
                sf: true,
                rt: 3,
                rn: 1,
                imm: 16
            }
        ));
    }

    // 4. StrImm 32-bit
    #[test]
    fn test_decode_str_imm_32() {
        let insn = decode(enc_str_imm32(3, 1, 8), 0x1000);
        assert!(matches!(
            insn,
            Instruction::StrImm {
                sf: false,
                rt: 3,
                rn: 1,
                imm: 8
            }
        ));
    }

    // 5. Ldrb unsigned immediate
    #[test]
    fn test_decode_ldrb_imm() {
        let insn = decode(enc_ldrb_imm(3, 1, 5), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldrb {
                rt: 3,
                rn: 1,
                imm: 5
            }
        ));
    }

    // 6. Strb unsigned immediate
    #[test]
    fn test_decode_strb_imm() {
        let insn = decode(enc_strb_imm(3, 1, 5), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Strb {
                rt: 3,
                rn: 1,
                imm: 5
            }
        ));
    }

    // 7. Ldrh unsigned immediate
    #[test]
    fn test_decode_ldrh_imm() {
        let insn = decode(enc_ldrh_imm(3, 1, 6), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldrh {
                rt: 3,
                rn: 1,
                imm: 6
            }
        ));
    }

    // 8. Strh unsigned immediate
    #[test]
    fn test_decode_strh_imm() {
        let insn = decode(enc_strh_imm(3, 1, 6), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Strh {
                rt: 3,
                rn: 1,
                imm: 6
            }
        ));
    }

    // 9. Ldp 64-bit (signed offset)
    #[test]
    fn test_decode_ldp_64() {
        let insn = decode(enc_ldp64(1, 2, 3, 16), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldp {
                sf: true,
                rt: 1,
                rt2: 2,
                rn: 3,
                imm: 16
            }
        ));
    }

    // 10. Stp 64-bit (signed offset)
    #[test]
    fn test_decode_stp_64() {
        let insn = decode(enc_stp64(1, 2, 3, 16), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Stp {
                sf: true,
                rt: 1,
                rt2: 2,
                rn: 3,
                imm: 16
            }
        ));
    }

    // 11. LdrLiteral 64-bit: 01_011000_imm19_Rt, imm19 = offset_bytes/4
    //     rt=3, offset=+0x100 → imm19=0x40 → raw = 0x58000803
    //     decoded imm = pc + 0x100 = 0x1100
    #[test]
    fn test_decode_ldr_literal_64() {
        let insn = decode(0x58000803, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrLiteral {
                sf: true,
                rt: 3,
                imm: 0x1100
            }
        ));
    }

    // 12. Ldur 64-bit: 11_111000_01_0_simm9_00_Rn_Rt
    //     rt=5, rn=1, simm9=16 → raw = 0xF8410025
    #[test]
    fn test_decode_ldur_64() {
        let insn = decode(0xF8410025, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldur {
                sf: true,
                rt: 5,
                rn: 1,
                imm: 16
            }
        ));
    }

    // 13. Stur 64-bit: 11_111000_00_0_simm9_00_Rn_Rt
    //     rt=5, rn=1, simm9=16 → raw = 0xF8010025
    #[test]
    fn test_decode_stur_64() {
        let insn = decode(0xF8010025, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Stur {
                sf: true,
                rt: 5,
                rn: 1,
                imm: 16
            }
        ));
    }

    // 14. LdrPre 64-bit: 11_111000_01_0_simm9_11_Rn_Rt
    //     rt=5, rn=1, simm9=16 → raw = 0xF8410C25
    #[test]
    fn test_decode_ldr_pre_64() {
        let insn = decode(0xF8410C25, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrPre {
                sf: true,
                rt: 5,
                rn: 1,
                imm: 16
            }
        ));
    }

    // 15. LdrPost 64-bit: 11_111000_01_0_simm9_01_Rn_Rt
    //     rt=5, rn=1, simm9=16 → raw = 0xF8410425
    #[test]
    fn test_decode_ldr_post_64() {
        let insn = decode(0xF8410425, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrPost {
                sf: true,
                rt: 5,
                rn: 1,
                imm: 16
            }
        ));
    }

    // --- Edge cases ---

    // 16. LdrImm 64-bit with zero offset
    #[test]
    fn test_decode_ldr_imm_64_zero_offset() {
        let insn = decode(enc_ldr_imm64(0, 31, 0), 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrImm {
                sf: true,
                rt: 0,
                rn: 31,
                imm: 0
            }
        ));
    }

    // 17. Ldp 64-bit with negative offset
    #[test]
    fn test_decode_ldp_64_negative_offset() {
        let insn = decode(enc_ldp64(4, 5, 3, -16), 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldp {
                sf: true,
                rt: 4,
                rt2: 5,
                rn: 3,
                imm: -16
            }
        ));
    }

    // 18. LdrLiteral 64-bit with negative offset
    //     rt=7, offset=-0x80 → imm19=(-32)&0x7FFFF=0x7FFE0 → raw = 0x58FFFC07
    //     decoded imm = 0x1000 + (-0x80) = 0xF80
    #[test]
    fn test_decode_ldr_literal_64_negative() {
        let insn = decode(0x58FFFC07, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LdrLiteral {
                sf: true,
                rt: 7,
                imm: 0xF80
            }
        ));
    }

    // 19. Ldur 64-bit with negative simm9
    //     rt=5, rn=1, simm9=-8 → bits[20:12]=0x1F8 → raw = 0xF85F8025
    #[test]
    fn test_decode_ldur_64_negative() {
        let insn = decode(0xF85F8025, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldur {
                sf: true,
                rt: 5,
                rn: 1,
                imm: -8
            }
        ));
    }

    // 20. Stur 64-bit with zero offset
    //     rt=5, rn=1, simm9=0 → raw = 0xF8000025
    #[test]
    fn test_decode_stur_64_zero_offset() {
        let insn = decode(0xF8000025, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Stur {
                sf: true,
                rt: 5,
                rn: 1,
                imm: 0
            }
        ));
    }

    #[test]
    fn test_decode_ldar_64() {
        // LDAR X0, [X1]: sf=1,o2=1,L=1,Rs=31,o0=1,Rt2=31,Rn=1,Rt=0
        // = 0xC8DFFC20
        let insn = decode(0xC8DFFC20, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Ldar {
                sf: true,
                rt: 0,
                rn: 1
            }
        ));
    }

    #[test]
    fn test_decode_stlr_64() {
        // STLR X1, [X2]: sf=1,o2=1,L=0,Rs=31,o0=1,Rt2=31,Rn=2,Rt=1
        // = 0xC89FFC41
        let insn = decode(0xC89FFC41, 0x1000);
        assert!(matches!(
            insn,
            Instruction::Stlr {
                sf: true,
                rt: 1,
                rn: 2
            }
        ));
    }

    #[test]
    fn test_decode_ldxr_64() {
        // LDXR X0, [X1]: sf=1,o2=0,L=1,Rs=31,o0=0,Rt2=31,Rn=1,Rt=0
        // = 0xC85F7C20
        let insn = decode(0xC85F7C20, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LoadExclusive {
                sf: true,
                acquire: false,
                rt: 0,
                rn: 1
            }
        ));
    }

    #[test]
    fn test_decode_ldaxr_64() {
        // LDAXR X0, [X1]: size=3,o2=0,L=1,Rs=31,o0=1,Rt2=31,Rn=1,Rt=0
        let insn = decode(0xC85FFC20, 0x1000);
        assert!(matches!(
            insn,
            Instruction::LoadExclusive {
                sf: true,
                acquire: true,
                rt: 0,
                rn: 1
            }
        ));
    }

    #[test]
    fn test_decode_clrex() {
        // CLREX = 0xD503305F (system instruction group)
        let insn = decode(0xD503305F, 0x1000);
        assert!(matches!(insn, Instruction::Clrex));
    }

    #[test]
    fn atomic_regression_exclusive_word_and_doubleword_widths() {
        // LDAR, STLR, LDXR, STXR, and CAS in their 32-bit encodings.
        for raw in [
            0x88DF_FC20,
            0x889F_FC41,
            0x885F_7C20,
            0x8801_7C40,
            0x88A0_7C41,
        ] {
            for expected_sf in [false, true] {
                let raw = raw | (u32::from(expected_sf) << 30);
                let decoded_sf = match decode(raw, 0x1000) {
                    Instruction::Ldar { sf, .. }
                    | Instruction::Stlr { sf, .. }
                    | Instruction::LoadExclusive { sf, .. }
                    | Instruction::StoreExclusive { sf, .. }
                    | Instruction::Cas { sf, .. } => sf,
                    other => panic!("unexpected instruction {other:?}"),
                };
                assert_eq!(decoded_sf, expected_sf, "encoding {raw:#010x}");
            }
        }
    }

    #[test]
    fn atomic_regression_lse_sizes_are_byte_widths() {
        for size_bits in 0..4 {
            for op in [0, 8] {
                // LDADD or SWP with source W1/X1, result W0/X0, base X2.
                let raw = 0x3821_0040 | (size_bits << 30) | (op << 12);
                let size = match decode(raw, 0x1000) {
                    Instruction::LseLoad { size, .. } | Instruction::Swp { size, .. } => size,
                    other => panic!("unexpected instruction {other:?}"),
                };
                assert_eq!(size, 1 << size_bits, "encoding {raw:#010x}");
            }
        }
    }

    #[test]
    fn atomic_regression_cas_ordering_flags() {
        for sf in [false, true] {
            for acquire in [false, true] {
                for release in [false, true] {
                    let raw = 0x88A0_7C41
                        | (u32::from(sf) << 30)
                        | (u32::from(acquire) << 22)
                        | (u32::from(release) << 15);
                    assert!(matches!(decode(raw, 0x1000), Instruction::Cas {
                        sf: actual_sf, acquire: actual_acquire, release: actual_release,
                        rs: 0, rt: 1, rn: 2,
                    } if actual_sf == sf && actual_acquire == acquire && actual_release == release));
                }
            }
        }
    }

    #[test]
    fn atomic_regression_unsupported_exclusive_widths_are_not_widened() {
        // CASB, CASH, LDXP W0/W1, and LDXP X0/X1 are not represented by
        // the single-register, word/doubleword Instruction::Cas variant.
        for raw in [0x08A0_7C41, 0x48A0_7C41, 0x887F_0440, 0xC87F_0440] {
            assert!(matches!(decode(raw, 0x1000), Instruction::Unknown { .. }));
        }
    }
}
