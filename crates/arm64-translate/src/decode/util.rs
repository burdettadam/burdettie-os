//! Utility helpers shared across decode sub-modules.

/// Sign-extend a `bits`-wide value to i64.
pub(super) fn sign_extend(val: u64, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((val as i64) << shift) >> shift
}

/// Sign-extend a 7-bit value.
pub(super) fn sign_extend7(val: i16) -> i16 {
    if val & 0x40 != 0 {
        val | !0x7F
    } else {
        val & 0x7F
    }
}

/// Sign-extend a 9-bit value.
pub(super) fn sign_extend9(val: i16) -> i16 {
    if val & 0x100 != 0 {
        val | !0x1FF
    } else {
        val & 0x1FF
    }
}

/// Decode an A64 bitmask immediate from N:imms:immr fields.
/// This is one of the most complex parts of the ARM64 encoding.
pub(super) fn decode_bitmask_imm(n: u32, imms: u32, immr: u32, sf: bool) -> u64 {
    let len = if n != 0 {
        6
    } else {
        // highest bit set in ~imms (6-bit field)
        let not_imms = (!imms) & 0x3F;
        if not_imms == 0 {
            return 0;
        }
        31 - not_imms.leading_zeros()
    };

    if len == 0 {
        return 0;
    }

    let size = 1u32 << len;
    let levels = size - 1;
    let s = imms & levels;
    let r = immr & levels;

    if s == levels {
        return 0;
    } // reserved

    let welem: u64 = if s + 1 >= 64 {
        u64::MAX
    } else {
        (1u64 << (s + 1)) - 1
    };
    // Rotate right by r within 'size' bits
    let rotated = if r == 0 {
        welem
    } else {
        let mask = if size >= 64 {
            u64::MAX
        } else {
            (1u64 << size) - 1
        };
        ((welem >> r) | (welem << (size - r))) & mask
    };

    // Replicate the pattern across 64 bits
    let mut result = 0u64;
    let mut pos = 0;
    while pos < 64 {
        result |= rotated << pos;
        pos += size;
    }

    if !sf {
        result &= 0xFFFF_FFFF;
    }
    result
}

/// Shift a value by (shift_type, amount).
/// shift_type: 0=LSL, 1=LSR, 2=ASR, 3=ROR
pub fn apply_shift(val: u64, shift_type: u8, amount: u8, is_64: bool) -> u64 {
    if amount == 0 {
        return val;
    }
    let amt = amount as u32;
    if is_64 {
        match shift_type {
            0 => val << amt,
            1 => val >> amt,
            2 => ((val as i64) >> amt) as u64,
            3 => val.rotate_right(amt),
            _ => val,
        }
    } else {
        let v32 = val as u32;
        let r = match shift_type {
            0 => v32.wrapping_shl(amt),
            1 => v32.wrapping_shr(amt),
            2 => ((v32 as i32) >> amt) as u32,
            3 => v32.rotate_right(amt),
            _ => v32,
        };
        r as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_extend() {
        // 0x3FFFF = 19 bits all set except bit 18 → positive (max positive 19-bit)
        assert_eq!(sign_extend(0x3FFFF, 19), 0x3FFFF);
        // 0x40000 = bit 18 set → sign bit → negative
        assert_eq!(sign_extend(0x40000, 19), -0x40000_i64);
        // 0x7FFFF = all 19 bits set → -1
        assert_eq!(sign_extend(0x7FFFF, 19), -1);
    }

    #[test]
    fn test_sign_extend7() {
        assert_eq!(sign_extend7(0), 0);
        assert_eq!(sign_extend7(0x3F), 63); // max positive 7-bit
        assert_eq!(sign_extend7(0x40), -64); // sign bit set
        assert_eq!(sign_extend7(0x7F), -1); // all 7 bits set
        assert_eq!(sign_extend7(1), 1);
        assert_eq!(sign_extend7(0x41), -63); // sign bit + 1
    }

    #[test]
    fn test_sign_extend9() {
        assert_eq!(sign_extend9(0), 0);
        assert_eq!(sign_extend9(0xFF), 255); // max positive within 8 bits
        assert_eq!(sign_extend9(0x100), -256); // sign bit (bit 8) set
        assert_eq!(sign_extend9(0x1FF), -1); // all 9 bits set
        assert_eq!(sign_extend9(0x1FE), -2);
        assert_eq!(sign_extend9(1), 1);
    }

    #[test]
    fn test_decode_bitmask_imm_n1_basic() {
        // AND X0, X0, #0xFF → N=1, immr=0, imms=7
        assert_eq!(decode_bitmask_imm(1, 7, 0, true), 0xFF);
        // AND X0, X0, #0xFFFFFFFF → N=1, immr=0, imms=31
        assert_eq!(decode_bitmask_imm(1, 31, 0, true), 0xFFFF_FFFF);
        // ORR X0, XZR, #1 → N=1, immr=0, imms=0
        assert_eq!(decode_bitmask_imm(1, 0, 0, true), 1);
        // 16 consecutive ones: imms=15
        assert_eq!(decode_bitmask_imm(1, 15, 0, true), 0xFFFF);
    }

    #[test]
    fn test_decode_bitmask_imm_n1_rotated() {
        // N=1, imms=7, immr=4: 8 ones rotated right by 4 within 64 bits
        // welem = 0xFF, rotated right 4 → 0xF00000000000000F
        assert_eq!(decode_bitmask_imm(1, 7, 4, true), 0xF000_0000_0000_000F);
        // N=1, imms=0, immr=1: single bit rotated right by 1
        // welem = 1, rotated right 1 → 0x8000000000000000
        assert_eq!(decode_bitmask_imm(1, 0, 1, true), 0x8000_0000_0000_0000);
    }

    #[test]
    fn test_decode_bitmask_imm_n0_replicated() {
        // 2-bit element {01} replicated: N=0, imms=0b111100(60), immr=0
        assert_eq!(decode_bitmask_imm(0, 60, 0, true), 0x5555_5555_5555_5555);
        // 2-bit element {10} replicated: N=0, imms=60, immr=1
        assert_eq!(decode_bitmask_imm(0, 60, 1, true), 0xAAAA_AAAA_AAAA_AAAA);
        // 4-bit element {0001} replicated: N=0, imms=0b111000(56), immr=0
        assert_eq!(decode_bitmask_imm(0, 56, 0, true), 0x1111_1111_1111_1111);
        // 4-bit element {0011} replicated: N=0, imms=57, immr=0
        assert_eq!(decode_bitmask_imm(0, 57, 0, true), 0x3333_3333_3333_3333);
    }

    #[test]
    fn test_decode_bitmask_imm_reserved() {
        // s == levels → reserved, returns 0
        // N=1: levels=63, imms=63
        assert_eq!(decode_bitmask_imm(1, 63, 0, true), 0);
        // N=0 with all imms bits set → not_imms=0, returns 0
        assert_eq!(decode_bitmask_imm(0, 0x3F, 0, true), 0);
        // N=0 len=0 case: imms=0b111110(62), not_imms=1, len=0 → returns 0
        assert_eq!(decode_bitmask_imm(0, 62, 0, true), 0);
    }

    #[test]
    fn test_decode_bitmask_imm_sf_false() {
        // sf=false masks result to 32 bits
        // 2-bit replicated pattern: 64-bit would be 0x5555555555555555
        assert_eq!(decode_bitmask_imm(0, 60, 0, false), 0x5555_5555);
        // Simple 8-bit mask
        assert_eq!(decode_bitmask_imm(1, 7, 0, false), 0xFF);
        // Rotated pattern: N=1, imms=7, immr=4 → 64-bit 0xF00000000000000F
        // masked to 32 bits → 0x0000000F
        assert_eq!(decode_bitmask_imm(1, 7, 4, false), 0x0000_000F);
    }

    #[test]
    fn test_apply_shift_lsl() {
        // 64-bit LSL
        assert_eq!(apply_shift(1, 0, 4, true), 16);
        assert_eq!(apply_shift(0xFF, 0, 8, true), 0xFF00);
        assert_eq!(apply_shift(1, 0, 63, true), 0x8000_0000_0000_0000);
        // 32-bit LSL
        assert_eq!(apply_shift(1, 0, 4, false), 16);
        assert_eq!(apply_shift(1, 0, 31, false), 0x8000_0000);
    }

    #[test]
    fn test_apply_shift_lsr() {
        // 64-bit LSR
        assert_eq!(apply_shift(0x100, 1, 4, true), 0x10);
        assert_eq!(apply_shift(0x8000_0000_0000_0000, 1, 63, true), 1);
        assert_eq!(apply_shift(0xFFFF_FFFF_FFFF_FFFF, 1, 32, true), 0xFFFF_FFFF);
        // 32-bit LSR
        assert_eq!(apply_shift(0x8000_0000, 1, 31, false), 1);
        assert_eq!(apply_shift(0xFFFF_FFFF, 1, 16, false), 0xFFFF);
    }

    #[test]
    fn test_apply_shift_asr() {
        // 64-bit ASR positive
        assert_eq!(apply_shift(0x100, 2, 4, true), 0x10);
        // 64-bit ASR negative (sign bit set)
        assert_eq!(
            apply_shift(0x8000_0000_0000_0000, 2, 4, true),
            0xF800_0000_0000_0000
        );
        assert_eq!(
            apply_shift(0xFFFF_FFFF_FFFF_FFFF, 2, 1, true),
            0xFFFF_FFFF_FFFF_FFFF
        );
        assert_eq!(
            apply_shift(0x8000_0000_0000_0000, 2, 63, true),
            0xFFFF_FFFF_FFFF_FFFF
        );
        // 32-bit ASR negative
        assert_eq!(apply_shift(0x8000_0000, 2, 4, false), 0xF800_0000);
        assert_eq!(apply_shift(0x8000_0000, 2, 31, false), 0xFFFF_FFFF);
        // 32-bit ASR positive
        assert_eq!(apply_shift(0x7FFF_FFFF, 2, 4, false), 0x07FF_FFFF);
    }

    #[test]
    fn test_apply_shift_ror() {
        // 64-bit ROR
        assert_eq!(apply_shift(1, 3, 1, true), 0x8000_0000_0000_0000);
        assert_eq!(
            apply_shift(0x8000_0000_0000_0001, 3, 4, true),
            0x1800_0000_0000_0000
        );
        // 32-bit ROR
        assert_eq!(apply_shift(1, 3, 1, false), 0x8000_0000);
        assert_eq!(apply_shift(0x8000_0001, 3, 4, false), 0x1800_0000);
    }

    #[test]
    fn test_apply_shift_zero_amount() {
        // Zero shift amount returns value unchanged for all shift types
        assert_eq!(apply_shift(42, 0, 0, true), 42);
        assert_eq!(apply_shift(42, 1, 0, true), 42);
        assert_eq!(apply_shift(42, 2, 0, true), 42);
        assert_eq!(apply_shift(42, 3, 0, true), 42);
        assert_eq!(apply_shift(42, 0, 0, false), 42);
    }

    #[test]
    fn test_apply_shift_unknown_type() {
        // Unknown shift type returns value unchanged
        assert_eq!(apply_shift(42, 4, 5, true), 42);
        assert_eq!(apply_shift(42, 4, 5, false), 42);
    }
}
