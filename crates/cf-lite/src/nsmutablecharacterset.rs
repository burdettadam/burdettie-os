//! NSMutableCharacterSet shim: mutable character set.
//!
//! Same layout as NSCharacterSet (32 bytes) but with mutation methods.
//! Predefined sets are "flattened" to a custom bitmap on first mutation.

use crate::cfstring;
use crate::nscharacterset;
use crate::nsobj;

// Re-use same layout offsets as NSCharacterSet
const KIND_OFF: usize = 8;
const BITMAP_PTR_OFF: usize = 16;
const BITMAP_LEN_OFF: usize = 24;

// ── Helpers ────────────────────────────────────────────────────

fn alloc_mutable_empty() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsmutablecharacterset_class_addr(); }
    obj
}

/// Flatten a predefined set into a custom codepoint bitmap (first 128 ASCII chars).
/// After this, kind == 0 and bitmap is a sorted u32 array.
unsafe fn ensure_custom(obj: u64) {
    let kind = *((obj as usize + KIND_OFF) as *const u64);
    if kind == 0 { return; }
    // Expand predefined set to bitmap of matching codepoints (BMP ASCII range)
    let mut codepoints = Vec::new();
    for cp in 0..0x10000u32 {
        if let Some(ch) = char::from_u32(cp) {
            if nscharacterset::character_set_contains(obj, ch) {
                codepoints.push(cp);
            }
        }
    }
    // Write custom bitmap
    let len = codepoints.len();
    let buf = crate::pal_alloc().malloc(len * 4) as *mut u32;
    if !buf.is_null() {
        for (i, &cp) in codepoints.iter().enumerate() {
            *buf.add(i) = cp;
        }
    }
    *((obj as usize + KIND_OFF) as *mut u64) = 0;
    *((obj as usize + BITMAP_PTR_OFF) as *mut u64) = buf as u64;
    *((obj as usize + BITMAP_LEN_OFF) as *mut u64) = len as u64;
}

/// Get the current codepoints as a mutable Vec, returning the vec.
unsafe fn take_codepoints(obj: u64) -> Vec<u32> {
    let ptr = *((obj as usize + BITMAP_PTR_OFF) as *const u64);
    let len = *((obj as usize + BITMAP_LEN_OFF) as *const u64) as usize;
    if ptr == 0 || len == 0 { return Vec::new(); }
    let slice = std::slice::from_raw_parts(ptr as *const u32, len);
    slice.to_vec()
}

/// Write codepoints back to the object, sorted + deduped.
unsafe fn put_codepoints(obj: u64, mut cps: Vec<u32>) {
    cps.sort_unstable();
    cps.dedup();
    let len = cps.len();
    let buf = crate::pal_alloc().malloc(len * 4) as *mut u32;
    if !buf.is_null() {
        for (i, &cp) in cps.iter().enumerate() {
            *buf.add(i) = cp;
        }
    }
    // Free old bitmap if any
    let old = *((obj as usize + BITMAP_PTR_OFF) as *const u64);
    if old != 0 {
        crate::pal_alloc().free(old as *mut u8);
    }
    *((obj as usize + BITMAP_PTR_OFF) as *mut u64) = buf as u64;
    *((obj as usize + BITMAP_LEN_OFF) as *mut u64) = len as u64;
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSMutableCharacterSet characterSetWithCharactersInString:]`
pub extern "C" fn shim_nsmutablecharset_with_characters_in_string(
    _cls: u64, _cmd: u64, string: u64,
) -> u64 {
    let obj = alloc_mutable_empty();
    if obj == 0 || string == 0 { return obj; }
    if let Some(s) = cfstring::extract_cfstring_cstr(string) {
        let mut codepoints: Vec<u32> = s.chars().map(|c| c as u32).collect();
        codepoints.sort_unstable();
        codepoints.dedup();
        let len = codepoints.len();
        if len > 0 {
            let buf = crate::pal_alloc().malloc(len * 4) as *mut u32;
            if !buf.is_null() {
                for (i, &cp) in codepoints.iter().enumerate() {
                    unsafe { *buf.add(i) = cp; }
                }
                unsafe {
                    *((obj as usize + BITMAP_PTR_OFF) as *mut u64) = buf as u64;
                    *((obj as usize + BITMAP_LEN_OFF) as *mut u64) = len as u64;
                }
            }
        }
    }
    obj
}

/// `+[NSMutableCharacterSet whitespaceCharacterSet]` etc — returns a mutable copy.
pub extern "C" fn shim_nsmutablecharset_whitespace(_cls: u64, _cmd: u64) -> u64 {
    let obj = alloc_mutable_empty();
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + KIND_OFF) as *mut u64) = 1; // KIND_WHITESPACE
    }
    obj
}

pub extern "C" fn shim_nsmutablecharset_alphanumeric(_cls: u64, _cmd: u64) -> u64 {
    let obj = alloc_mutable_empty();
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + KIND_OFF) as *mut u64) = 7; // KIND_ALPHANUMERIC
    }
    obj
}

pub extern "C" fn shim_nsmutablecharset_decimal_digit(_cls: u64, _cmd: u64) -> u64 {
    let obj = alloc_mutable_empty();
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + KIND_OFF) as *mut u64) = 3; // KIND_DECIMAL_DIGIT
    }
    obj
}

pub extern "C" fn shim_nsmutablecharset_letter(_cls: u64, _cmd: u64) -> u64 {
    let obj = alloc_mutable_empty();
    if obj == 0 { return 0; }
    unsafe {
        *((obj as usize + KIND_OFF) as *mut u64) = 4; // KIND_LETTER
    }
    obj
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSMutableCharacterSet addCharactersInString:]`
pub extern "C" fn shim_nsmutablecharset_add_characters_in_string(
    obj: u64, _cmd: u64, string: u64,
) -> u64 {
    if obj == 0 || string == 0 { return 0; }
    unsafe { ensure_custom(obj); }
    if let Some(s) = cfstring::extract_cfstring_cstr(string) {
        let mut cps = unsafe { take_codepoints(obj) };
        for ch in s.chars() {
            cps.push(ch as u32);
        }
        unsafe { put_codepoints(obj, cps); }
    }
    0
}

/// `-[NSMutableCharacterSet removeCharactersInString:]`
pub extern "C" fn shim_nsmutablecharset_remove_characters_in_string(
    obj: u64, _cmd: u64, string: u64,
) -> u64 {
    if obj == 0 || string == 0 { return 0; }
    unsafe { ensure_custom(obj); }
    if let Some(s) = cfstring::extract_cfstring_cstr(string) {
        let remove: std::collections::HashSet<u32> = s.chars().map(|c| c as u32).collect();
        let mut cps = unsafe { take_codepoints(obj) };
        cps.retain(|cp| !remove.contains(cp));
        unsafe { put_codepoints(obj, cps); }
    }
    0
}

/// `-[NSMutableCharacterSet formUnionWithCharacterSet:]`
pub extern "C" fn shim_nsmutablecharset_form_union(
    obj: u64, _cmd: u64, other: u64,
) -> u64 {
    if obj == 0 || other == 0 { return 0; }
    unsafe { ensure_custom(obj); }
    // Collect other's codepoints by probing BMP
    let mut cps = unsafe { take_codepoints(obj) };
    for cp in 0..0x10000u32 {
        if let Some(ch) = char::from_u32(cp) {
            if nscharacterset::character_set_contains(other, ch) {
                cps.push(cp);
            }
        }
    }
    unsafe { put_codepoints(obj, cps); }
    0
}

/// `-[NSMutableCharacterSet formIntersectionWithCharacterSet:]`
pub extern "C" fn shim_nsmutablecharset_form_intersection(
    obj: u64, _cmd: u64, other: u64,
) -> u64 {
    if obj == 0 || other == 0 { return 0; }
    unsafe { ensure_custom(obj); }
    let mut cps = unsafe { take_codepoints(obj) };
    cps.retain(|&cp| {
        if let Some(ch) = char::from_u32(cp) {
            nscharacterset::character_set_contains(other, ch)
        } else {
            false
        }
    });
    unsafe { put_codepoints(obj, cps); }
    0
}

/// `-[NSMutableCharacterSet invert]`
pub extern "C" fn shim_nsmutablecharset_invert(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { ensure_custom(obj); }
    let current = unsafe { take_codepoints(obj) };
    let current_set: std::collections::HashSet<u32> = current.into_iter().collect();
    let mut inverted = Vec::new();
    for cp in 0..128u32 { // ASCII range for practicality
        if !current_set.contains(&cp) {
            inverted.push(cp);
        }
    }
    unsafe { put_codepoints(obj, inverted); }
    0
}

/// `-[NSMutableCharacterSet characterIsMember:]`
pub extern "C" fn shim_nsmutablecharset_character_is_member(obj: u64, _cmd: u64, ch: u64) -> u64 {
    // Same implementation as NSCharacterSet
    nscharacterset::shim_nscharacterset_character_is_member(obj, _cmd, ch)
}

/// `-[NSMutableCharacterSet description]`
pub extern "C" fn shim_nsmutablecharset_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let len = unsafe { *((obj as usize + BITMAP_LEN_OFF) as *const u64) };
    let kind = unsafe { *((obj as usize + KIND_OFF) as *const u64) };
    let desc = if kind != 0 {
        format!("<NSMutableCharacterSet: predefined kind {}>", kind)
    } else {
        format!("<NSMutableCharacterSet: {} codepoints>", len)
    };
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
