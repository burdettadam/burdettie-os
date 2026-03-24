//! NSNumber shim: boxed numeric values (int, double, bool).
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa       (u64 — pointer to NSNumber class)
//!   offset  8: type_tag  (u64 — 0=int64, 1=double, 2=bool)
//!   offset 16: value     (u64 — raw bits: i64 as u64, f64 as u64, bool as 0/1)
//!
//! Total: 24 bytes per number object.

use crate::cfstring;

const TAG_INT: u64 = 0;
const TAG_DOUBLE: u64 = 1;
const TAG_BOOL: u64 = 2;

const TYPE_TAG_OFF: usize = 8;
const VALUE_OFF: usize = 16;

/// Allocate and initialize an NSNumber with the given tag and raw value bits.
pub fn alloc_nsnumber(tag: u64, raw_value: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsnumber_class_addr();
        *((obj as usize + TYPE_TAG_OFF) as *mut u64) = tag;
        *((obj as usize + VALUE_OFF) as *mut u64) = raw_value;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSNumber numberWithInt:]`
pub extern "C" fn shim_nsnumber_number_with_int(_cls: u64, _cmd: u64, value: i32) -> u64 {
    alloc_nsnumber(TAG_INT, value as i64 as u64)
}

/// `+[NSNumber numberWithDouble:]`
///
/// Note: on ARM64, the double argument is passed in d0 (floating-point register).
/// Since our shim is called through objc_msgSend dispatch which preserves
/// the FP registers, the caller's d0 lands in `value` correctly when
/// the Rust function declares `value: f64`.
pub extern "C" fn shim_nsnumber_number_with_double(_cls: u64, _cmd: u64, value: f64) -> u64 {
    alloc_nsnumber(TAG_DOUBLE, value.to_bits())
}

/// `+[NSNumber numberWithBool:]`
pub extern "C" fn shim_nsnumber_number_with_bool(_cls: u64, _cmd: u64, value: u8) -> u64 {
    alloc_nsnumber(TAG_BOOL, if value != 0 { 1 } else { 0 })
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSNumber intValue]`
pub extern "C" fn shim_nsnumber_int_value(obj: u64, _cmd: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    let tag = unsafe { *((obj as usize + TYPE_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    match tag {
        TAG_INT => raw as i64 as i32,
        TAG_DOUBLE => f64::from_bits(raw) as i32,
        TAG_BOOL => raw as i32,
        _ => 0,
    }
}

/// `-[NSNumber doubleValue]`
pub extern "C" fn shim_nsnumber_double_value(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let tag = unsafe { *((obj as usize + TYPE_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    match tag {
        TAG_INT => raw as i64 as f64,
        TAG_DOUBLE => f64::from_bits(raw),
        TAG_BOOL => raw as f64,
        _ => 0.0,
    }
}

/// `-[NSNumber boolValue]`
pub extern "C" fn shim_nsnumber_bool_value(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let tag = unsafe { *((obj as usize + TYPE_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    match tag {
        TAG_INT => {
            if raw as i64 != 0 {
                1
            } else {
                0
            }
        }
        TAG_DOUBLE => {
            if f64::from_bits(raw) != 0.0 {
                1
            } else {
                0
            }
        }
        TAG_BOOL => {
            if raw != 0 {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// `-[NSNumber integerValue]` (NSInteger = i64 on 64-bit)
pub extern "C" fn shim_nsnumber_integer_value(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    let tag = unsafe { *((obj as usize + TYPE_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    match tag {
        TAG_INT => raw as i64,
        TAG_DOUBLE => f64::from_bits(raw) as i64,
        TAG_BOOL => raw as i64,
        _ => 0,
    }
}

/// `-[NSNumber isEqualToNumber:]`
pub extern "C" fn shim_nsnumber_is_equal_to_number(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 {
        return 0;
    }
    // Compare as f64 for cross-type equality
    let self_val = shim_nsnumber_double_value(obj, 0);
    let other_val = shim_nsnumber_double_value(other, 0);
    if self_val == other_val {
        1
    } else {
        0
    }
}

/// `-[NSNumber compare:]`
pub extern "C" fn shim_nsnumber_compare(obj: u64, _cmd: u64, other: u64) -> i64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let a = shim_nsnumber_double_value(obj, 0);
    let b = shim_nsnumber_double_value(other, 0);
    if a < b {
        -1
    } else if a > b {
        1
    } else {
        0
    }
}

/// `-[NSNumber description]`
pub extern "C" fn shim_nsnumber_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let tag = unsafe { *((obj as usize + TYPE_TAG_OFF) as *const u64) };
    let raw = unsafe { *((obj as usize + VALUE_OFF) as *const u64) };
    let desc = match tag {
        TAG_INT => format!("{}", raw as i64),
        TAG_DOUBLE => format!("{}", f64::from_bits(raw)),
        TAG_BOOL => {
            if raw != 0 {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        _ => format!("<NSNumber tag={}>", tag),
    };

    let c_str = std::ffi::CString::new(desc).unwrap();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_roundtrip() {
        let obj = alloc_nsnumber(TAG_INT, 42u64);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsnumber_int_value(obj, 0), 42);
        assert_eq!(shim_nsnumber_double_value(obj, 0), 42.0);
        assert_eq!(shim_nsnumber_bool_value(obj, 0), 1);
        unsafe {
            libc::free(obj as *mut libc::c_void);
        }
    }

    #[test]
    fn double_roundtrip() {
        let obj = alloc_nsnumber(TAG_DOUBLE, 2.72f64.to_bits());
        assert_ne!(obj, 0);
        assert_eq!(shim_nsnumber_double_value(obj, 0), 2.72);
        assert_eq!(shim_nsnumber_int_value(obj, 0), 2); // truncated
        unsafe {
            libc::free(obj as *mut libc::c_void);
        }
    }

    #[test]
    fn bool_roundtrip() {
        let yes = alloc_nsnumber(TAG_BOOL, 1);
        let no = alloc_nsnumber(TAG_BOOL, 0);
        assert_eq!(shim_nsnumber_bool_value(yes, 0), 1);
        assert_eq!(shim_nsnumber_bool_value(no, 0), 0);
        assert_eq!(shim_nsnumber_int_value(yes, 0), 1);
        assert_eq!(shim_nsnumber_int_value(no, 0), 0);
        unsafe {
            libc::free(yes as *mut libc::c_void);
            libc::free(no as *mut libc::c_void);
        }
    }

    #[test]
    fn nil_returns_zero() {
        assert_eq!(shim_nsnumber_int_value(0, 0), 0);
        assert_eq!(shim_nsnumber_double_value(0, 0), 0.0);
        assert_eq!(shim_nsnumber_bool_value(0, 0), 0);
    }

    #[test]
    fn is_equal_to_number_cross_type() {
        let int42 = alloc_nsnumber(TAG_INT, 42u64);
        let dbl42 = alloc_nsnumber(TAG_DOUBLE, 42.0f64.to_bits());
        assert_eq!(shim_nsnumber_is_equal_to_number(int42, 0, dbl42), 1);

        let dbl43 = alloc_nsnumber(TAG_DOUBLE, 43.0f64.to_bits());
        assert_eq!(shim_nsnumber_is_equal_to_number(int42, 0, dbl43), 0);

        unsafe {
            libc::free(int42 as *mut libc::c_void);
            libc::free(dbl42 as *mut libc::c_void);
            libc::free(dbl43 as *mut libc::c_void);
        }
    }
}
