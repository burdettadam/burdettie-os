//! NSString path utilities, CharacterSet operations, and additional methods.

use std::ffi::CString;

use crate::cfstring::{
    alloc_nsstring_from_cstr, extract_cfstring_cstr, shim_cf_string_create_with_cstring,
};
use super::search::{NSRange, NS_NOT_FOUND};
use super::basic::{shim_nsstring_is_equal_to_string, shim_nsstring_string_with_format_inner};

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
    alloc_nsstring_from_cstr(cstr.as_ptr())
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
    alloc_nsstring_from_cstr(cstr.as_ptr())
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
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString stringByDeletingPathExtension]`
pub extern "C" fn shim_nsstring_string_by_deleting_path_extension(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let last_slash = s.rfind('/').map(|p| p + 1).unwrap_or(0);
    let result = match s[last_slash..].rfind('.') {
        Some(dot) if dot > 0 => &s[..last_slash + dot],
        _ => s,
    };
    let cstr = CString::new(result).unwrap_or_default();
    alloc_nsstring_from_cstr(cstr.as_ptr())
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
    alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSString isAbsolutePath]`
pub extern "C" fn shim_nsstring_is_absolute_path(obj: u64, _sel: u64) -> u8 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    u8::from(s.starts_with('/'))
}

// ── CharacterSet operations ────────────────────────────────────────

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
    alloc_nsstring_from_cstr(cstr.as_ptr())
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
            alloc_nsstring_from_cstr(cstr.as_ptr())
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
    }
}

// ── Additional methods ─────────────────────────────────────────────

/// `-[NSString containsString:]`
pub extern "C" fn shim_nsstring_contains_string(obj: u64, _sel: u64, search: u64) -> u8 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let Some(needle) = extract_cfstring_cstr(search) else {
        return 0;
    };
    if s.contains(needle) { 1 } else { 0 }
}

/// `-[NSString substringWithRange:]`
///
/// On aarch64, NSRange is passed in x2+x3 (two 64-bit registers).
pub extern "C" fn shim_nsstring_substring_with_range(
    obj: u64,
    _sel: u64,
    location: u64,
    length: u64,
) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let loc = location as usize;
    let len = length as usize;
    if loc + len > s.len() {
        log::warn!("substringWithRange: range {},{} out of bounds for len={}", loc, len, s.len());
        return 0;
    }
    let sub = &s[loc..loc + len];
    let c_str = CString::new(sub).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString characterAtIndex:]`
pub extern "C" fn shim_nsstring_character_at_index(obj: u64, _sel: u64, index: u64) -> u16 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let idx = index as usize;
    if idx < utf16.len() {
        utf16[idx]
    } else {
        log::warn!("characterAtIndex: index {} out of bounds for len={}", idx, utf16.len());
        0
    }
}

/// `-[NSString initWithFormat:]` — variadic, handled via same trampoline as stringWithFormat:
#[allow(dead_code)]
pub extern "C" fn shim_nsstring_init_with_format(
    _obj: u64,
    _sel: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    shim_nsstring_string_with_format_inner(format, a1, a2, a3)
}

/// `-[NSString initWithString:]`
pub extern "C" fn shim_nsstring_init_with_string(
    _obj: u64,
    _sel: u64,
    other: u64,
) -> u64 {
    if other == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(other) else {
        return 0;
    };
    let c_str = CString::new(s).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString capitalizedString]`
pub extern "C" fn shim_nsstring_capitalized_string(obj: u64, _sel: u64) -> u64 {
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch.is_whitespace() || ch == '-' || ch == '_' {
            result.push(ch);
            capitalize_next = true;
        } else if capitalize_next {
            for c in ch.to_uppercase() {
                result.push(c);
            }
            capitalize_next = false;
        } else {
            for c in ch.to_lowercase() {
                result.push(c);
            }
        }
    }
    let c_str = CString::new(result).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString caseInsensitiveCompare:]`
pub extern "C" fn shim_nsstring_case_insensitive_compare(obj: u64, _sel: u64, other: u64) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    match la.cmp(&lb) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `-[NSString isEqual:]` — delegates to isEqualToString: for string objects
pub extern "C" fn shim_nsstring_is_equal(obj: u64, _sel: u64, other: u64) -> u8 {
    if obj == other {
        return 1;
    }
    if other == 0 {
        return 0;
    }
    shim_nsstring_is_equal_to_string(obj, _sel, other)
}
