//! NSScanner shim: scanf-style string scanning.
//!
//! Object layout (40 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: string_ptr   (u64 → NSString being scanned)
//!   offset 16: scan_loc     (u64 — current scan position, character index)
//!   offset 24: case_sens    (u64 — 1 = case sensitive, 0 = insensitive)
//!   offset 32: skip_set     (u64 → NSCharacterSet to skip, 0 = none)

use crate::cfstring;
use crate::nscharacterset;
use crate::nsobj;

const STRING_OFF: usize = 8;
const SCANLOC_OFF: usize = 16;
const CASESENS_OFF: usize = 24;
const SKIPSET_OFF: usize = 32;

fn alloc_scanner(string: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsscanner_class_addr();
        *((obj as usize + STRING_OFF) as *mut u64) = string;
        *((obj as usize + CASESENS_OFF) as *mut u64) = 1; // default: case sensitive
                                                          // Default: skip whitespace (matching Apple's default)
        let ws = nscharacterset::shim_nscharacterset_whitespace_and_newline(0, 0);
        *((obj as usize + SKIPSET_OFF) as *mut u64) = ws;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSScanner scannerWithString:]`
pub extern "C" fn shim_nsscanner_scanner_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    alloc_scanner(string)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSScanner string]`
pub extern "C" fn shim_nsscanner_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + STRING_OFF) as *const u64) }
}

/// `-[NSScanner scanLocation]`
pub extern "C" fn shim_nsscanner_scan_location(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SCANLOC_OFF) as *const u64) }
}

/// `-[NSScanner setScanLocation:]`
pub extern "C" fn shim_nsscanner_set_scan_location(obj: u64, _cmd: u64, loc: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc;
    }
    0
}

/// `-[NSScanner isAtEnd]`
pub extern "C" fn shim_nsscanner_is_at_end(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 1;
    }
    let string = unsafe { *((obj as usize + STRING_OFF) as *const u64) };
    let loc = unsafe { *((obj as usize + SCANLOC_OFF) as *const u64) };
    let len = if string != 0 {
        cfstring::shim_nsstring_length(string, 0)
    } else {
        0
    };
    u8::from(loc >= len)
}

/// `-[NSScanner setCaseSensitive:]`
pub extern "C" fn shim_nsscanner_set_case_sensitive(obj: u64, _cmd: u64, flag: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + CASESENS_OFF) as *mut u64) = flag;
    }
    0
}

/// `-[NSScanner caseSensitive]`
pub extern "C" fn shim_nsscanner_case_sensitive(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 1;
    }
    unsafe { *((obj as usize + CASESENS_OFF) as *const u64) as u8 }
}

/// `-[NSScanner setCharactersToBeSkipped:]`
pub extern "C" fn shim_nsscanner_set_characters_to_be_skipped(
    obj: u64,
    _cmd: u64,
    set: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + SKIPSET_OFF) as *mut u64) = set;
    }
    0
}

/// `-[NSScanner charactersToBeSkipped]`
pub extern "C" fn shim_nsscanner_characters_to_be_skipped(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SKIPSET_OFF) as *const u64) }
}

/// Helper: get the remaining string from scan_location onwards.
fn remaining_str(obj: u64) -> Option<(String, u64)> {
    let string = unsafe { *((obj as usize + STRING_OFF) as *const u64) };
    let loc = unsafe { *((obj as usize + SCANLOC_OFF) as *const u64) };
    let full = cfstring::extract_cfstring_cstr(string)?;
    if loc as usize >= full.len() {
        return None;
    }
    Some((full[loc as usize..].to_string(), loc))
}

/// `-[NSScanner scanInt:]` — scans an integer, writes to pointer if non-null.
pub extern "C" fn shim_nsscanner_scan_int(obj: u64, _cmd: u64, result_ptr: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let trimmed = rem.trim_start();
    let skipped = rem.len() - trimmed.len();

    // Find the longest prefix that parses as an integer
    let mut end = 0usize;
    for (i, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() || (i == 0 && (ch == '-' || ch == '+')) {
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return 0;
    }
    let Ok(val) = trimmed[..end].parse::<i32>() else {
        return 0;
    };
    if result_ptr != 0 {
        unsafe {
            *(result_ptr as *mut i32) = val;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + (skipped + end) as u64;
    }
    1
}

/// `-[NSScanner scanDouble:]`
pub extern "C" fn shim_nsscanner_scan_double(obj: u64, _cmd: u64, result_ptr: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let trimmed = rem.trim_start();
    let skipped = rem.len() - trimmed.len();

    let mut end = 0usize;
    let mut has_dot = false;
    for (i, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() || (i == 0 && (ch == '-' || ch == '+')) {
            end = i + ch.len_utf8();
        } else if ch == '.' && !has_dot {
            has_dot = true;
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return 0;
    }
    let Ok(val) = trimmed[..end].parse::<f64>() else {
        return 0;
    };
    if result_ptr != 0 {
        unsafe {
            *(result_ptr as *mut f64) = val;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + (skipped + end) as u64;
    }
    1
}

/// `-[NSScanner scanString:intoString:]`
pub extern "C" fn shim_nsscanner_scan_string(
    obj: u64,
    _cmd: u64,
    target: u64,
    result_ptr: u64,
) -> u8 {
    if obj == 0 || target == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let Some(target_str) = cfstring::extract_cfstring_cstr(target) else {
        return 0;
    };
    if !rem.starts_with(target_str) {
        return 0;
    }
    if result_ptr != 0 {
        unsafe {
            *(result_ptr as *mut u64) = target;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + target_str.len() as u64;
    }
    1
}

/// `-[NSScanner scanUpToString:intoString:]`
pub extern "C" fn shim_nsscanner_scan_up_to_string(
    obj: u64,
    _cmd: u64,
    stop_str: u64,
    result_ptr: u64,
) -> u8 {
    if obj == 0 || stop_str == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let Some(stop) = cfstring::extract_cfstring_cstr(stop_str) else {
        return 0;
    };
    let found_pos = rem.find(stop).unwrap_or(rem.len());
    if found_pos == 0 {
        return 0;
    }
    if result_ptr != 0 {
        let scanned = &rem[..found_pos];
        let c_str = std::ffi::CString::new(scanned).unwrap_or_default();
        let ns = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        unsafe {
            *(result_ptr as *mut u64) = ns;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + found_pos as u64;
    }
    1
}

/// `-[NSScanner scanUpToCharactersFromSet:intoString:]`
pub extern "C" fn shim_nsscanner_scan_up_to_characters_from_set(
    obj: u64,
    _cmd: u64,
    char_set: u64,
    result_ptr: u64,
) -> u8 {
    if obj == 0 || char_set == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let found_pos = rem
        .char_indices()
        .find(|(_, ch)| nscharacterset::character_set_contains(char_set, *ch))
        .map(|(i, _)| i)
        .unwrap_or(rem.len());
    if found_pos == 0 {
        return 0;
    }
    if result_ptr != 0 {
        let scanned = &rem[..found_pos];
        let c_str = std::ffi::CString::new(scanned).unwrap_or_default();
        let ns = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        unsafe {
            *(result_ptr as *mut u64) = ns;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + found_pos as u64;
    }
    1
}

/// `-[NSScanner scanCharactersFromSet:intoString:]`
pub extern "C" fn shim_nsscanner_scan_characters_from_set(
    obj: u64,
    _cmd: u64,
    char_set: u64,
    result_ptr: u64,
) -> u8 {
    if obj == 0 || char_set == 0 {
        return 0;
    }
    let Some((rem, loc)) = remaining_str(obj) else {
        return 0;
    };
    let end_pos = rem
        .char_indices()
        .find(|(_, ch)| !nscharacterset::character_set_contains(char_set, *ch))
        .map(|(i, _)| i)
        .unwrap_or(rem.len());
    if end_pos == 0 {
        return 0;
    }
    if result_ptr != 0 {
        let scanned = &rem[..end_pos];
        let c_str = std::ffi::CString::new(scanned).unwrap_or_default();
        let ns = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        unsafe {
            *(result_ptr as *mut u64) = ns;
        }
    }
    unsafe {
        *((obj as usize + SCANLOC_OFF) as *mut u64) = loc + end_pos as u64;
    }
    1
}

/// `-[NSScanner description]`
pub extern "C" fn shim_nsscanner_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let loc = unsafe { *((obj as usize + SCANLOC_OFF) as *const u64) };
    let desc = format!("<NSScanner: scanLocation={loc}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_scanner(s: &str) -> u64 {
        let ns = make_string(s);
        shim_nsscanner_scanner_with_string(0, 0, ns)
    }

    #[test]
    fn scanner_initial_state() {
        let sc = make_scanner("hello");
        assert_ne!(sc, 0);
        assert_eq!(shim_nsscanner_scan_location(sc, 0), 0);
        assert_eq!(shim_nsscanner_is_at_end(sc, 0), 0);
        free_obj(sc);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsscanner_string(0, 0), 0);
        assert_eq!(shim_nsscanner_scan_location(0, 0), 0);
        assert_eq!(shim_nsscanner_is_at_end(0, 0), 1);
        assert_eq!(shim_nsscanner_description(0, 0), 0);
        assert_eq!(shim_nsscanner_scan_int(0, 0, 0), 0);
        assert_eq!(shim_nsscanner_scan_double(0, 0, 0), 0);
    }

    #[test]
    fn scan_int_basic() {
        let sc = make_scanner("42 rest");
        let mut val: i32 = 0;
        let ok = shim_nsscanner_scan_int(sc, 0, &raw mut val as u64);
        assert_eq!(ok, 1);
        assert_eq!(val, 42);
        assert!(shim_nsscanner_scan_location(sc, 0) >= 2);
        free_obj(sc);
    }

    #[test]
    fn scan_double_basic() {
        let sc = make_scanner("3.14");
        let mut val: f64 = 0.0;
        let ok = shim_nsscanner_scan_double(sc, 0, &raw mut val as u64);
        assert_eq!(ok, 1);
        assert!((val - 3.14).abs() < 1e-9, "expected 3.14, got {val}");
        free_obj(sc);
    }

    #[test]
    fn scan_string_match_advances_location() {
        let sc = make_scanner("foobar");
        let target = make_string("foo");
        let ok = shim_nsscanner_scan_string(sc, 0, target, 0);
        assert_eq!(ok, 1);
        assert_eq!(shim_nsscanner_scan_location(sc, 0), 3);
        free_obj(sc);
    }

    #[test]
    fn is_at_end_after_full_scan() {
        let sc = make_scanner("x");
        let target = make_string("x");
        shim_nsscanner_scan_string(sc, 0, target, 0);
        assert_eq!(shim_nsscanner_is_at_end(sc, 0), 1);
        free_obj(sc);
    }

    #[test]
    fn description_format() {
        let sc = make_scanner("abc");
        let desc = shim_nsscanner_description(sc, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSScanner: scanLocation=0>");
        free_obj(sc);
    }

    #[test]
    fn case_sensitive_default_and_toggle() {
        let sc = make_scanner("test");
        assert_eq!(shim_nsscanner_case_sensitive(sc, 0), 1);
        shim_nsscanner_set_case_sensitive(sc, 0, 0);
        assert_eq!(shim_nsscanner_case_sensitive(sc, 0), 0);
        free_obj(sc);
    }
}
