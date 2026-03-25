//! NSString search, replace, parse, and mutable-copy shims.

use std::ffi::CString;

use crate::cfstring::{
    cfstring_parts, extract_cfstring_cstr, shim_cf_string_create_with_cstring, CF_STRING_FLAGS_UTF8,
};

/// NSNotFound = NSIntegerMax on 64-bit.
pub(super) const NS_NOT_FOUND: u64 = 0x7FFF_FFFF_FFFF_FFFF;

/// Return type for `-[NSString rangeOfString:]`.
///
/// On ARM64, a two-register struct is returned in x0 (location) and x1 (length).
#[repr(C)]
pub struct NSRange {
    pub location: u64,
    pub length: u64,
}

/// `-[NSString stringByReplacingOccurrencesOfString:withString:]`
pub extern "C" fn shim_nsstring_string_by_replacing(
    obj: u64,
    _sel: u64,
    target: u64,
    replacement: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(t) = extract_cfstring_cstr(target) else {
        return obj;
    };
    let Some(r) = extract_cfstring_cstr(replacement) else {
        return obj;
    };

    let result = s.replace(t, r);
    let c_str = CString::new(result).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString componentsSeparatedByString:]`
///
/// Returns an NSArray of NSString objects.
pub extern "C" fn shim_nsstring_components_separated_by_string(
    obj: u64,
    _sel: u64,
    separator: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(sep) = extract_cfstring_cstr(separator) else {
        return 0;
    };

    let parts: Vec<&str> = s.split(sep).collect();

    // Create NSString objects for each component
    let mut string_addrs: Vec<u64> = Vec::with_capacity(parts.len());
    for part in &parts {
        let c_str = CString::new(*part).unwrap_or_default();
        let addr = shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        string_addrs.push(addr);
    }

    // Create NSArray from the string objects
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        string_addrs.as_ptr() as u64,
        string_addrs.len() as u64,
    )
}

/// `-[NSString rangeOfString:]` — find substring, return NSRange.
///
/// On ARM64 the 16-byte struct is returned in `x0` (location) and
/// `x1` (length).  If not found, returns `{NSNotFound, 0}`.
pub extern "C" fn shim_nsstring_range_of_string(obj: u64, _sel: u64, search: u64) -> NSRange {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    };
    let Some(needle) = extract_cfstring_cstr(search) else {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    };

    match s.find(needle) {
        Some(pos) => NSRange {
            location: pos as u64,
            length: needle.len() as u64,
        },
        None => NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        },
    }
}

/// `-[NSString integerValue]` — parse leading digits as integer.
pub extern "C" fn shim_nsstring_integer_value(obj: u64, _sel: u64) -> i64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let trimmed = s.trim();
    let mut end = 0;
    for (i, c) in trimmed.char_indices() {
        if i == 0 && (c == '+' || c == '-') {
            end = 1;
            continue;
        }
        if c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return 0;
    }
    trimmed[..end].parse::<i64>().unwrap_or(0)
}

/// `-[NSString doubleValue]` — parse leading numeric chars as double.
pub extern "C" fn shim_nsstring_double_value(obj: u64, _sel: u64) -> f64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0.0;
    };
    let trimmed = s.trim();
    // Parse as many valid float characters as possible
    let mut end = 0;
    let mut saw_dot = false;
    for (i, c) in trimmed.char_indices() {
        if i == 0 && (c == '+' || c == '-') {
            end = 1;
            continue;
        }
        if c == '.' && !saw_dot {
            saw_dot = true;
            end = i + 1;
            continue;
        }
        if c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return 0.0;
    }
    trimmed[..end].parse::<f64>().unwrap_or(0.0)
}

/// `-[NSString mutableCopy]` — creates an NSMutableString with the same content.
pub extern "C" fn shim_nsstring_mutable_copy(obj: u64, _sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some((bytes, len)) = cfstring_parts(obj) else {
        return 0;
    };

    let owned = CString::new(bytes).unwrap_or_default();
    let owned_ptr = owned.into_raw();

    let new_obj = crate::pal_alloc().calloc(1, 32) as u64;
    if new_obj == 0 {
        unsafe {
            drop(CString::from_raw(owned_ptr));
        }
        return 0;
    }

    unsafe {
        *(new_obj as *mut u64) = crate::nsobj::nsmutablestring_class_addr();
        *((new_obj as usize + 8) as *mut u32) = CF_STRING_FLAGS_UTF8;
        *((new_obj as usize + 16) as *mut u64) = owned_ptr as u64;
        *((new_obj as usize + 24) as *mut u64) = len as u64;
    }

    log::debug!("NSString mutableCopy → {:#x}, len={}", new_obj, len);
    new_obj
}
