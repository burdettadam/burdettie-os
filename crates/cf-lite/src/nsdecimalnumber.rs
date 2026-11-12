//! NSDecimalNumber shims.
//!
//! Object layout: 24 bytes (same as NSNumber)
//!   [0..8):   ISA pointer  (nsdecimalnumber_class_addr)
//!   [8..16):  type_tag     (u64 — always TAG_DOUBLE = 1)
//!   [16..24): value        (u64 — f64 bits)
//!
//! NSDecimalNumber is a concrete subclass of NSNumber that uses
//! `NSDecimal` (a base-10 floating-point struct) internally.
//! For our purposes, we back it by f64 which covers 15+ significant
//! digits — sufficient for the vast majority of real-world uses.

use crate::cfstring;
use crate::nsnumber::{self, TAG_DOUBLE};

const SIZE: usize = 24;
const TYPE_TAG_OFF: usize = 8;
const VALUE_OFF: usize = 16;

/// Allocate an NSDecimalNumber backed by an f64 value.
pub fn alloc_decimal(val: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdecimalnumber_class_addr();
        *((obj as usize + TYPE_TAG_OFF) as *mut u64) = TAG_DOUBLE;
        *((obj as usize + VALUE_OFF) as *mut u64) = val.to_bits();
    }
    obj
}

fn read_double(obj: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let bits = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    f64::from_bits(bits)
}

// ── Class methods ───────────────────────────────────────────────

/// `+[NSDecimalNumber alloc]`
pub extern "C" fn shim_nsdecimalnumber_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_decimal(0.0)
}

/// `+[NSDecimalNumber decimalNumberWithString:]`
pub extern "C" fn shim_nsdecimalnumber_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    if string == 0 {
        return alloc_decimal(0.0);
    }
    let Some(s) = cfstring::extract_cfstring_cstr(string) else {
        return alloc_decimal(0.0);
    };
    let val: f64 = s.parse().unwrap_or(0.0);
    alloc_decimal(val)
}

/// `+[NSDecimalNumber decimalNumberWithMantissa:exponent:isNegative:]`
pub extern "C" fn shim_nsdecimalnumber_with_mantissa(
    _cls: u64,
    _cmd: u64,
    mantissa: u64,
    exponent: i16,
    is_negative: u8,
) -> u64 {
    let mut val = mantissa as f64 * 10.0_f64.powi(exponent as i32);
    if is_negative != 0 {
        val = -val;
    }
    alloc_decimal(val)
}

/// `+[NSDecimalNumber zero]`
pub extern "C" fn shim_nsdecimalnumber_zero(_cls: u64, _cmd: u64) -> u64 {
    alloc_decimal(0.0)
}

/// `+[NSDecimalNumber one]`
pub extern "C" fn shim_nsdecimalnumber_one(_cls: u64, _cmd: u64) -> u64 {
    alloc_decimal(1.0)
}

/// `+[NSDecimalNumber notANumber]`
pub extern "C" fn shim_nsdecimalnumber_nan(_cls: u64, _cmd: u64) -> u64 {
    alloc_decimal(f64::NAN)
}

// ── Instance methods ────────────────────────────────────────────

/// `-[NSDecimalNumber initWithString:]`
pub extern "C" fn shim_nsdecimalnumber_init_with_string(obj: u64, _cmd: u64, string: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let val = if string == 0 {
        0.0
    } else {
        cfstring::extract_cfstring_cstr(string)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    unsafe {
        *((obj as usize + TYPE_TAG_OFF) as *mut u64) = TAG_DOUBLE;
        *((obj as usize + VALUE_OFF) as *mut u64) = val.to_bits();
    }
    obj
}

/// `-[NSDecimalNumber initWithMantissa:exponent:isNegative:]`
pub extern "C" fn shim_nsdecimalnumber_init_with_mantissa(
    obj: u64,
    _cmd: u64,
    mantissa: u64,
    exponent: i16,
    is_negative: u8,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let mut val = mantissa as f64 * 10.0_f64.powi(exponent as i32);
    if is_negative != 0 {
        val = -val;
    }
    unsafe {
        *((obj as usize + TYPE_TAG_OFF) as *mut u64) = TAG_DOUBLE;
        *((obj as usize + VALUE_OFF) as *mut u64) = val.to_bits();
    }
    obj
}

/// `-[NSDecimalNumber decimalNumberByAdding:]`
pub extern "C" fn shim_nsdecimalnumber_by_adding(obj: u64, _cmd: u64, other: u64) -> u64 {
    alloc_decimal(read_double(obj) + read_double(other))
}

/// `-[NSDecimalNumber decimalNumberBySubtracting:]`
pub extern "C" fn shim_nsdecimalnumber_by_subtracting(obj: u64, _cmd: u64, other: u64) -> u64 {
    alloc_decimal(read_double(obj) - read_double(other))
}

/// `-[NSDecimalNumber decimalNumberByMultiplyingBy:]`
pub extern "C" fn shim_nsdecimalnumber_by_multiplying(obj: u64, _cmd: u64, other: u64) -> u64 {
    alloc_decimal(read_double(obj) * read_double(other))
}

/// `-[NSDecimalNumber decimalNumberByDividingBy:]`
pub extern "C" fn shim_nsdecimalnumber_by_dividing(obj: u64, _cmd: u64, other: u64) -> u64 {
    let divisor = read_double(other);
    if divisor == 0.0 {
        return alloc_decimal(f64::NAN);
    }
    alloc_decimal(read_double(obj) / divisor)
}

/// `-[NSDecimalNumber decimalNumberByRaisingToPower:]`
pub extern "C" fn shim_nsdecimalnumber_by_raising_to_power(obj: u64, _cmd: u64, power: u64) -> u64 {
    alloc_decimal(read_double(obj).powi(power as i32))
}

/// `-[NSDecimalNumber compare:]`
pub extern "C" fn shim_nsdecimalnumber_compare(obj: u64, _cmd: u64, other: u64) -> i64 {
    let a = read_double(obj);
    let b = read_double(other);
    if a < b {
        -1
    } else if a > b {
        1
    } else {
        0
    }
}

/// `-[NSDecimalNumber doubleValue]`
pub extern "C" fn shim_nsdecimalnumber_double_value(obj: u64, _cmd: u64) -> f64 {
    read_double(obj)
}

/// `-[NSDecimalNumber stringValue]`
pub extern "C" fn shim_nsdecimalnumber_string_value(obj: u64, _cmd: u64) -> u64 {
    nsnumber::shim_nsnumber_string_value(obj, _cmd)
}

/// `-[NSDecimalNumber descriptionWithLocale:]`
pub extern "C" fn shim_nsdecimalnumber_description_with_locale(
    obj: u64,
    _cmd: u64,
    _locale: u64,
) -> u64 {
    shim_nsdecimalnumber_string_value(obj, _cmd)
}

/// `-[NSDecimalNumber description]`
pub extern "C" fn shim_nsdecimalnumber_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsdecimalnumber_string_value(obj, _cmd)
}

/// `-[NSDecimalNumber objCType]` — returns "d" (double).
pub extern "C" fn shim_nsdecimalnumber_objc_type(_obj: u64, _cmd: u64) -> u64 {
    c"d".as_ptr() as u64
}

/// `-[NSDecimalNumber isEqual:]` — uses NSNumber comparison path.
pub extern "C" fn shim_nsdecimalnumber_is_equal(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 && other == 0 {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }
    u8::from(shim_nsdecimalnumber_compare(obj, 0, other) == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_zero() {
        let n = alloc_decimal(0.0);
        assert_ne!(n, 0);
        assert_eq!(read_double(n), 0.0);
    }

    #[test]
    fn mantissa_exponent() {
        let n = shim_nsdecimalnumber_with_mantissa(0, 0, 12345, -2, 0);
        let val = read_double(n);
        assert!((val - 123.45).abs() < 0.001);
    }

    #[test]
    fn mantissa_negative() {
        let n = shim_nsdecimalnumber_with_mantissa(0, 0, 42, 0, 1);
        assert_eq!(read_double(n), -42.0);
    }

    #[test]
    fn arithmetic_add() {
        let a = alloc_decimal(10.5);
        let b = alloc_decimal(3.25);
        let result = shim_nsdecimalnumber_by_adding(a, 0, b);
        assert!((read_double(result) - 13.75).abs() < 0.001);
    }

    #[test]
    fn arithmetic_subtract() {
        let a = alloc_decimal(10.0);
        let b = alloc_decimal(3.0);
        let result = shim_nsdecimalnumber_by_subtracting(a, 0, b);
        assert!((read_double(result) - 7.0).abs() < 0.001);
    }

    #[test]
    fn arithmetic_multiply() {
        let a = alloc_decimal(6.0);
        let b = alloc_decimal(7.0);
        let result = shim_nsdecimalnumber_by_multiplying(a, 0, b);
        assert!((read_double(result) - 42.0).abs() < 0.001);
    }

    #[test]
    fn arithmetic_divide() {
        let a = alloc_decimal(10.0);
        let b = alloc_decimal(3.0);
        let result = shim_nsdecimalnumber_by_dividing(a, 0, b);
        assert!((read_double(result) - 3.3333).abs() < 0.01);
    }

    #[test]
    fn divide_by_zero() {
        let a = alloc_decimal(10.0);
        let b = alloc_decimal(0.0);
        let result = shim_nsdecimalnumber_by_dividing(a, 0, b);
        assert!(read_double(result).is_nan());
    }

    #[test]
    fn compare_ordering() {
        let a = alloc_decimal(5.0);
        let b = alloc_decimal(10.0);
        assert_eq!(shim_nsdecimalnumber_compare(a, 0, b), -1);
        assert_eq!(shim_nsdecimalnumber_compare(b, 0, a), 1);
        assert_eq!(shim_nsdecimalnumber_compare(a, 0, a), 0);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(read_double(0), 0.0);
        assert_eq!(shim_nsdecimalnumber_double_value(0, 0), 0.0);
        assert_eq!(shim_nsdecimalnumber_compare(0, 0, 0), 0);
    }

    #[test]
    fn power() {
        let n = alloc_decimal(3.0);
        let result = shim_nsdecimalnumber_by_raising_to_power(n, 0, 4);
        assert!((read_double(result) - 81.0).abs() < 0.001);
    }
}
