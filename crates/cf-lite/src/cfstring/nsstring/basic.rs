//! Core NSString shims: UTF8String, length, init, create, compare, format, hash.

use std::ffi::{c_char, CStr, CString};

use crate::cfstring::{
    cfstring_parts, extract_cfstring_cstr, shim_cf_string_create_with_cstring,
    shim_cf_string_get_cstring_ptr, K_CF_STRING_ENCODING_UTF8, CF_STRING_FLAGS_UTF8,
};

/// `id -[NSString UTF8String]`
pub extern "C" fn shim_nsstring_utf8string(obj: u64, _sel: u64) -> *const c_char {
    shim_cf_string_get_cstring_ptr(obj, K_CF_STRING_ENCODING_UTF8)
}

/// `NSUInteger -[NSString length]`
pub extern "C" fn shim_nsstring_length(obj: u64, _sel: u64) -> u64 {
    crate::cfstring::shim_cf_string_get_length(obj) as u64
}

/// `id -[NSString description]` — returns self.
pub extern "C" fn shim_nsstring_description(obj: u64, _sel: u64) -> u64 {
    obj
}

/// `-[NSString initWithData:encoding:]`
pub extern "C" fn shim_nsstring_init_with_data_encoding(
    _obj: u64,
    _sel: u64,
    data: u64,
    _encoding: u64,
) -> u64 {
    if data == 0 {
        return 0;
    }
    let length = unsafe { *((data as *const u8).add(8) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((data as *const u8).add(16) as *const u64) } as *const u8;
    if bytes_ptr.is_null() || length == 0 {
        return shim_cf_string_create_with_cstring(0, b"\0".as_ptr() as *const c_char, 0);
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, length) };
    let s = String::from_utf8_lossy(slice);
    let c_str = CString::new(s.as_ref()).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString initWithUTF8String:]`
pub extern "C" fn shim_nsstring_init_with_utf8string(
    obj: u64,
    _sel: u64,
    c_str: *const c_char,
) -> u64 {
    if obj == 0 || c_str.is_null() {
        return 0;
    }

    let rust_str = unsafe { CStr::from_ptr(c_str) };
    let bytes = rust_str.to_bytes();
    let len = bytes.len();

    let owned = CString::new(bytes).unwrap_or_default();
    let owned_ptr = owned.into_raw();

    unsafe {
        let p = obj as *mut u8;
        *((p.add(8)) as *mut u32) = CF_STRING_FLAGS_UTF8;
        *((p.add(16)) as *mut u64) = owned_ptr as u64;
        *((p.add(24)) as *mut u64) = len as u64;
    }

    log::debug!(
        "NSString initWithUTF8String: initialized at {:#x}, len={}",
        obj,
        len
    );
    obj
}

/// `+[NSString stringWithUTF8String:]`
pub extern "C" fn shim_nsstring_string_with_utf8string(
    cls: u64,
    _sel: u64,
    c_str: *const c_char,
) -> u64 {
    if c_str.is_null() {
        return 0;
    }

    let rust_str = unsafe { CStr::from_ptr(c_str) };
    let bytes = rust_str.to_bytes();
    let len = bytes.len();

    let owned = CString::new(bytes).unwrap_or_default();
    let owned_ptr = owned.into_raw();

    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        unsafe {
            drop(CString::from_raw(owned_ptr));
        }
        return 0;
    }

    unsafe {
        let p = obj as *mut u8;
        *(p as *mut u64) = cls;
        *((p.add(8)) as *mut u32) = CF_STRING_FLAGS_UTF8;
        *((p.add(16)) as *mut u64) = owned_ptr as u64;
        *((p.add(24)) as *mut u64) = len as u64;
    }

    log::debug!(
        "NSString stringWithUTF8String: created at {:#x}, len={}",
        obj,
        len
    );
    obj
}

/// `-[NSString isEqualToString:]`
pub extern "C" fn shim_nsstring_is_equal_to_string(obj: u64, _sel: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 {
        return 0;
    }

    unsafe {
        let a_ptr = *((obj as *const u8).add(16) as *const u64) as *const c_char;
        let a_len = *((obj as *const u8).add(24) as *const u64);
        let b_ptr = *((other as *const u8).add(16) as *const u64) as *const c_char;
        let b_len = *((other as *const u8).add(24) as *const u64);

        if a_len != b_len {
            return 0;
        }
        if a_ptr.is_null() || b_ptr.is_null() {
            return if a_ptr == b_ptr { 1 } else { 0 };
        }

        let a_slice = std::slice::from_raw_parts(a_ptr as *const u8, a_len as usize);
        let b_slice = std::slice::from_raw_parts(b_ptr as *const u8, b_len as usize);
        if a_slice == b_slice {
            1
        } else {
            0
        }
    }
}

/// `void CFRelease(CFTypeRef cf)`
pub extern "C" fn shim_cf_release(obj: u64) {
    if obj == 0 {
        return;
    }
    let remaining = objc_runtime::refcount::REFCOUNTS
        .lock()
        .unwrap()
        .release(obj);
    if remaining == 0 {
        log::debug!("CFRelease: deallocating {:#x}", obj);
        crate::pal_alloc().free(obj as *mut u8);
    }
}

// ── +[NSString stringWithFormat:] ──────────────────────────────────

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_nsstring_format_asm();
}

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
extern "C" {
    fn shim_nsstring_format_x86_64_asm();
}

/// Returns the address of the stringWithFormat: trampoline.
pub fn string_with_format_trampoline_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_nsstring_format_asm as u64
    }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        shim_nsstring_format_x86_64_asm as u64
    }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    {
        shim_nsstring_string_with_format_inner as u64
    }
}

/// Inner implementation for `+[NSString stringWithFormat:]`.
#[no_mangle]
pub extern "C" fn shim_nsstring_string_with_format_inner(
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    if format == 0 {
        return 0;
    }

    let Some(fmt_str) = extract_cfstring_cstr(format) else {
        return 0;
    };

    let expanded = crate::nslog::expand_format(fmt_str, &[a1, a2, a3]);

    let c_str = CString::new(expanded).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

// ── String manipulation shims ──────────────────────────────────────

/// `-[NSString hasPrefix:]`
pub extern "C" fn shim_nsstring_has_prefix(obj: u64, _sel: u64, prefix: u64) -> u8 {
    let Some((a, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let Some((b, _)) = cfstring_parts(prefix) else {
        return 0;
    };
    if a.starts_with(b) {
        1
    } else {
        0
    }
}

/// `-[NSString hasSuffix:]`
pub extern "C" fn shim_nsstring_has_suffix(obj: u64, _sel: u64, suffix: u64) -> u8 {
    let Some((a, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let Some((b, _)) = cfstring_parts(suffix) else {
        return 0;
    };
    if a.ends_with(b) {
        1
    } else {
        0
    }
}

/// `-[NSString stringByAppendingString:]`
pub extern "C" fn shim_nsstring_string_by_appending(obj: u64, _sel: u64, other: u64) -> u64 {
    let Some((a, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let Some((b, _)) = cfstring_parts(other) else {
        return obj;
    };

    let mut combined = Vec::with_capacity(a.len() + b.len());
    combined.extend_from_slice(a);
    combined.extend_from_slice(b);

    let c_str = CString::new(combined).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString substringFromIndex:]`
pub extern "C" fn shim_nsstring_substring_from_index(obj: u64, _sel: u64, index: u64) -> u64 {
    let Some((bytes, len)) = cfstring_parts(obj) else {
        return 0;
    };
    let idx = index as usize;
    if idx > len {
        return shim_cf_string_create_with_cstring(0, c"".as_ptr(), 0);
    }
    let sub = &bytes[idx..];
    let c_str = CString::new(sub).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString substringToIndex:]`
pub extern "C" fn shim_nsstring_substring_to_index(obj: u64, _sel: u64, index: u64) -> u64 {
    let Some((bytes, len)) = cfstring_parts(obj) else {
        return 0;
    };
    let idx = index as usize;
    let end = if idx > len { len } else { idx };
    let sub = &bytes[..end];
    let c_str = CString::new(sub).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString lowercaseString]`
pub extern "C" fn shim_nsstring_lowercase(obj: u64, _sel: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let c_str = CString::new(lower).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString uppercaseString]`
pub extern "C" fn shim_nsstring_uppercase(obj: u64, _sel: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let upper: Vec<u8> = bytes.iter().map(|b| b.to_ascii_uppercase()).collect();
    let c_str = CString::new(upper).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString compare:]`
pub extern "C" fn shim_nsstring_compare(obj: u64, _sel: u64, other: u64) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `-[NSString copy]` — immutable strings return self.
pub extern "C" fn shim_nsstring_copy(obj: u64, _sel: u64) -> u64 {
    obj
}

/// `-[NSString hash]` — DJB2 hash for dictionary keys.
pub extern "C" fn shim_nsstring_hash(obj: u64, _sel: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let mut hash: u64 = 5381;
    for &b in bytes {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}
