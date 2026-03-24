//! NSString shims: UTF8String, length, description, init, create, compare,
//! release, and stringWithFormat:.

use std::ffi::{c_char, CStr, CString};

use super::{
    cfstring_parts, extract_cfstring_cstr, shim_cf_string_create_with_cstring,
    shim_cf_string_get_cstring_ptr, K_CF_STRING_ENCODING_UTF8,
};

/// `id -[NSString UTF8String]`
pub extern "C" fn shim_nsstring_utf8string(obj: u64, _sel: u64) -> *const c_char {
    shim_cf_string_get_cstring_ptr(obj, K_CF_STRING_ENCODING_UTF8)
}

/// `NSUInteger -[NSString length]`
pub extern "C" fn shim_nsstring_length(obj: u64, _sel: u64) -> u64 {
    super::shim_cf_string_get_length(obj) as u64
}

/// `id -[NSString description]` — returns self.
pub extern "C" fn shim_nsstring_description(obj: u64, _sel: u64) -> u64 {
    obj
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

    let owned = CString::new(bytes).unwrap_or_else(|_| CString::new("").unwrap());
    let owned_ptr = owned.into_raw();

    unsafe {
        let p = obj as *mut u8;
        *((p.add(8)) as *mut u32) = super::CF_STRING_FLAGS_UTF8;
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

    let owned = CString::new(bytes).unwrap_or_else(|_| CString::new("").unwrap());
    let owned_ptr = owned.into_raw();

    let obj = unsafe { libc::calloc(1, 32) as u64 };
    if obj == 0 {
        unsafe {
            drop(CString::from_raw(owned_ptr));
        }
        return 0;
    }

    unsafe {
        let p = obj as *mut u8;
        *(p as *mut u64) = cls;
        *((p.add(8)) as *mut u32) = super::CF_STRING_FLAGS_UTF8;
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
        unsafe { libc::free(obj as *mut libc::c_void) };
    }
}

// ── +[NSString stringWithFormat:] ──────────────────────────────────

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_nsstring_format_asm();
}

/// Returns the address of the stringWithFormat: trampoline.
pub fn string_with_format_trampoline_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_nsstring_format_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
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

    let c_str = CString::new(expanded).unwrap_or_else(|_| CString::new("").unwrap());
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

    let c_str = CString::new(combined).unwrap_or_else(|_| CString::new("").unwrap());
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
    let c_str = CString::new(sub).unwrap_or_else(|_| CString::new("").unwrap());
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
    let c_str = CString::new(sub).unwrap_or_else(|_| CString::new("").unwrap());
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString lowercaseString]`
pub extern "C" fn shim_nsstring_lowercase(obj: u64, _sel: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let c_str = CString::new(lower).unwrap_or_else(|_| CString::new("").unwrap());
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString uppercaseString]`
pub extern "C" fn shim_nsstring_uppercase(obj: u64, _sel: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let upper: Vec<u8> = bytes.iter().map(|b| b.to_ascii_uppercase()).collect();
    let c_str = CString::new(upper).unwrap_or_else(|_| CString::new("").unwrap());
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

// ── Additional NSString methods ────────────────────────────────────

/// NSNotFound = NSIntegerMax on 64-bit.
const NS_NOT_FOUND: u64 = 0x7FFF_FFFF_FFFF_FFFF;

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
    let c_str = CString::new(result).unwrap_or_else(|_| CString::new("").unwrap());
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
        let c_str = CString::new(*part).unwrap_or_else(|_| CString::new("").unwrap());
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

    let owned = CString::new(bytes).unwrap_or_else(|_| CString::new("").unwrap());
    let owned_ptr = owned.into_raw();

    let new_obj = unsafe { libc::calloc(1, 32) } as u64;
    if new_obj == 0 {
        unsafe {
            drop(CString::from_raw(owned_ptr));
        }
        return 0;
    }

    unsafe {
        *(new_obj as *mut u64) = crate::nsobj::nsmutablestring_class_addr();
        *((new_obj as usize + 8) as *mut u32) = super::CF_STRING_FLAGS_UTF8;
        *((new_obj as usize + 16) as *mut u64) = owned_ptr as u64;
        *((new_obj as usize + 24) as *mut u64) = len as u64;
    }

    log::debug!("NSString mutableCopy → {:#x}, len={}", new_obj, len);
    new_obj
}

// ── Path utility methods ───────────────────────────────────────────

/// `-[NSString pathExtension]` — returns extension after last '.'.
pub extern "C" fn shim_nsstring_path_extension(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let last = s.rsplit('/').next().unwrap_or(s);
    let ext = match last.rfind('.') {
        Some(dot) if dot > 0 => &last[dot + 1..],
        _ => "",
    };
    let cstr = CString::new(ext).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString lastPathComponent]` — returns component after last '/'.
pub extern "C" fn shim_nsstring_last_path_component(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let trimmed = s.trim_end_matches('/');
    let comp = if trimmed.is_empty() {
        "/"
    } else {
        trimmed.rsplit('/').next().unwrap_or(trimmed)
    };
    let cstr = CString::new(comp).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByDeletingLastPathComponent]`
pub extern "C" fn shim_nsstring_string_by_deleting_last_path_component(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let trimmed = s.trim_end_matches('/');
    let result = match trimmed.rfind('/') {
        Some(0) => "/",
        Some(pos) => &trimmed[..pos],
        None => "",
    };
    let cstr = CString::new(result).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByDeletingPathExtension]`
pub extern "C" fn shim_nsstring_string_by_deleting_path_extension(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    // Find the last '.' in the last path component
    let last_slash = s.rfind('/').map(|p| p + 1).unwrap_or(0);
    let result = match s[last_slash..].rfind('.') {
        Some(dot) if dot > 0 => &s[..last_slash + dot],
        _ => s,
    };
    let cstr = CString::new(result).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByAppendingPathComponent:]`
pub extern "C" fn shim_nsstring_string_by_appending_path_component(
    obj: u64,
    _sel: u64,
    component: u64,
) -> u64 {
    let Some(base) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(comp) = extract_cfstring_cstr(component) else {
        return 0;
    };
    let trimmed_base = base.trim_end_matches('/');
    let trimmed_comp = comp.trim_start_matches('/');
    let result = format!("{trimmed_base}/{trimmed_comp}");
    let cstr = CString::new(result).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString isAbsolutePath]`
pub extern "C" fn shim_nsstring_is_absolute_path(obj: u64, _sel: u64) -> u8 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    u8::from(s.starts_with('/'))
}

/// `-[NSString stringByTrimmingCharactersInSet:]`
pub extern "C" fn shim_nsstring_string_by_trimming_characters_in_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    if charset == 0 {
        return obj;
    }
    let trimmed: String = s
        .trim_start_matches(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .trim_end_matches(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .to_string();
    let cstr = CString::new(trimmed).unwrap_or_default();
    super::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString componentsSeparatedByCharactersInSet:]`
pub extern "C" fn shim_nsstring_components_separated_by_characters_in_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    if charset == 0 {
        // Return array with self
        return crate::nsarray::shim_nsarray_array_with_objects_count(
            crate::nsobj::nsarray_class_addr(),
            0,
            &obj as *const u64 as u64,
            1,
        );
    }
    let parts: Vec<&str> = s
        .split(|c: char| crate::nscharacterset::character_set_contains(charset, c))
        .collect();
    let ns_parts: Vec<u64> = parts
        .iter()
        .map(|p| {
            let cstr = CString::new(*p).unwrap_or_default();
            super::alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();
    let buf = ns_parts.as_ptr() as u64;
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        buf,
        ns_parts.len() as u64,
    )
}

/// `-[NSString rangeOfCharacterFromSet:]`
pub extern "C" fn shim_nsstring_range_of_character_from_set(
    obj: u64,
    _sel: u64,
    charset: u64,
) -> NSRange {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    };
    if charset == 0 {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    }
    for (i, ch) in s.char_indices() {
        if crate::nscharacterset::character_set_contains(charset, ch) {
            return NSRange {
                location: i as u64,
                length: ch.len_utf8() as u64,
            };
        }
    }
    NSRange {
        location: NS_NOT_FOUND,
        length: 0,
    } // NSNotFound
}
