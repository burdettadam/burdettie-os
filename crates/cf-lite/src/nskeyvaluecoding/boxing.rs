//! NSNumber auto-boxing/unboxing for KVC scalar types.

use crate::nsnumber;

/// Extract the return type character from ObjC type encoding.
pub(super) fn extract_return_type(type_encoding: &str) -> char {
    type_encoding.chars().next().unwrap_or('@')
}

/// Extract the setter's value parameter type from ObjC type encoding.
pub(super) fn extract_setter_param_type(type_encoding: &str) -> char {
    let type_chars: Vec<char> = type_encoding
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect();
    type_chars.get(3).copied().unwrap_or('@')
}

/// Auto-unbox an NSNumber into a raw scalar value for setter calls.
pub(super) fn autounbox_if_scalar(value: u64, param_type: char) -> u64 {
    match param_type {
        '@' => value,
        'i' | 'I' | 's' | 'S' | 'c' | 'C' | 'l' | 'L' | 'q' | 'Q' | 'B' => unbox_int(value) as u64,
        'f' | 'd' => {
            let dval = unbox_double(value);
            if param_type == 'f' {
                (dval as f32).to_bits() as u64
            } else {
                dval.to_bits()
            }
        }
        _ => value,
    }
}

/// Auto-box a raw scalar return value into an NSNumber if needed.
pub(super) fn autobox_if_scalar(raw: u64, return_type: char) -> u64 {
    match return_type {
        '@' => raw,
        'i' | 'I' | 's' | 'S' | 'c' | 'C' | 'l' | 'L' | 'q' | 'Q' | 'B' => {
            nsnumber::alloc_nsnumber(0x0, raw)
        }
        'f' => {
            let val = f32::from_bits(raw as u32) as f64;
            nsnumber::alloc_nsnumber(0x1, val.to_bits())
        }
        'd' => nsnumber::alloc_nsnumber(0x1, raw),
        _ => raw,
    }
}

/// Check if a value is a tagged pointer (bit 63 set).
fn is_tagged_pointer(value: u64) -> bool {
    value & (1u64 << 63) != 0
}

/// Unbox an NSNumber to i64.
pub(super) fn unbox_int(value: u64) -> i64 {
    if value == 0 {
        return 0;
    }

    if is_tagged_pointer(value) {
        return value as i64;
    }

    let isa = unsafe { *(value as *const u64) };

    if isa == crate::nsobj::nsnumber_class_addr() {
        let tag = unsafe { *((value as usize + 8) as *const u64) };
        let raw = unsafe { *((value as usize + 16) as *const u64) };
        return match tag {
            0 => raw as i64,
            1 => f64::from_bits(raw) as i64,
            2 => raw as i64,
            _ => raw as i64,
        };
    }

    let raw = unsafe { *((value as usize + 16) as *const u64) };
    raw as i64
}

/// Unbox an NSNumber to f64.
pub(super) fn unbox_double(value: u64) -> f64 {
    if value == 0 {
        return 0.0;
    }

    if is_tagged_pointer(value) {
        return value as i64 as f64;
    }

    let isa = unsafe { *(value as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        let tag = unsafe { *((value as usize + 8) as *const u64) };
        let raw = unsafe { *((value as usize + 16) as *const u64) };
        return match tag {
            0 => raw as i64 as f64,
            1 => f64::from_bits(raw),
            2 => raw as f64,
            _ => raw as i64 as f64,
        };
    }

    let raw = unsafe { *((value as usize + 16) as *const u64) };
    raw as i64 as f64
}
