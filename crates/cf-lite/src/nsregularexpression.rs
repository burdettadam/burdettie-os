//! NSRegularExpression shim: regex matching via the `regex` crate.
//!
//! Object layout (40 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: pattern_ptr  (u64 → NSString)
//!   offset 16: options      (u64 → NSRegularExpressionOptions)
//!   offset 24: regex_ptr    (u64 → Box<regex::Regex> leaked)
//!   offset 32: reserved     (u64)

use crate::cfstring;
use crate::nsarray;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 40;

/// Read the `regex::Regex` pointer stored at offset 24.
unsafe fn get_regex(obj: u64) -> Option<&'static regex::Regex> {
    let regex_ptr = *((obj + 24) as *const u64);
    if regex_ptr == 0 {
        return None;
    }
    Some(&*(regex_ptr as *const regex::Regex))
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSRegularExpression regularExpressionWithPattern:options:error:]`
pub extern "C" fn shim_regex_with_pattern(
    _cls: u64,
    _cmd: u64,
    pattern: u64,
    options: u64,
    error_out: u64,
) -> u64 {
    let c_str = cfstring::extract_cfstring_cstr(pattern);
    let pat = match c_str {
        Some(s) => s.to_string(),
        None => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror(
                    "NSCocoaErrorDomain",
                    2048, // NSFormattingError
                );
                unsafe { *(error_out as *mut u64) = err; }
            }
            return 0;
        }
    };

    // Build regex flags from NSRegularExpressionOptions
    let case_insensitive = (options & 1) != 0;
    let multiline = (options & 8) != 0;
    let dot_matches_newline = (options & 2) != 0;

    let mut regex_pat = String::new();
    if case_insensitive || multiline || dot_matches_newline {
        regex_pat.push_str("(?");
        if case_insensitive { regex_pat.push('i'); }
        if multiline { regex_pat.push('m'); }
        if dot_matches_newline { regex_pat.push('s'); }
        regex_pat.push(')');
    }
    regex_pat.push_str(&pat);

    match regex::Regex::new(&regex_pat) {
        Ok(re) => {
            let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
            unsafe {
                *(obj as *mut u64) = nsobj::nsregularexpression_class_addr();
                *((obj + 8) as *mut u64) = pattern;
                *((obj + 16) as *mut u64) = options;
                *((obj + 24) as *mut u64) = Box::into_raw(Box::new(re)) as u64;
            }
            obj
        }
        Err(e) => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror(
                    "NSCocoaErrorDomain",
                    2048, // NSFormattingError
                );
                unsafe { *(error_out as *mut u64) = err; }
                log::debug!("NSRegularExpression: invalid pattern: {}", e);
            }
            0
        }
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSRegularExpression pattern]`
pub extern "C" fn shim_regex_pattern(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSRegularExpression options]`
pub extern "C" fn shim_regex_options(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 16) as *const u64) }
}

/// `-[NSRegularExpression numberOfMatchesInString:options:range:]`
pub extern "C" fn shim_regex_number_of_matches(
    obj: u64,
    _cmd: u64,
    string: u64,
    _options: u64,
    _range_loc: u64,
    _range_len: u64,
) -> u64 {
    if obj == 0 || string == 0 { return 0; }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    re.find_iter(c_str).count() as u64
}

/// `-[NSRegularExpression firstMatchInString:options:range:]`
///
/// Returns an NSArray [matchStart, matchLen] or 0 (nil) if no match.
/// (Simplified: real NSTextCheckingResult not implemented.)
pub extern "C" fn shim_regex_first_match(
    obj: u64,
    _cmd: u64,
    string: u64,
    _options: u64,
    _range_loc: u64,
    _range_len: u64,
) -> u64 {
    if obj == 0 || string == 0 { return 0; }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    match re.find(c_str) {
        Some(m) => {
            // Return an NSArray of [location, length] as NSNumbers
            let loc = crate::nsnumber::make_nsnumber_int(m.start() as i64);
            let len = crate::nsnumber::make_nsnumber_int(m.len() as i64);
            let items = [loc, len];
            nsarray::make_nsarray(&items)
        }
        None => 0,
    }
}

/// `-[NSRegularExpression stringByReplacingMatchesInString:options:range:withTemplate:]`
pub extern "C" fn shim_regex_replace(
    obj: u64,
    _cmd: u64,
    string: u64,
    _options: u64,
    _range_loc: u64,
    _range_len: u64,
    template: u64,
) -> u64 {
    if obj == 0 || string == 0 || template == 0 { return 0; }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(input) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    let Some(tmpl) = cfstring::extract_cfstring_cstr(template) else {
        return 0;
    };
    let result = re.replace_all(input, tmpl);
    let c_result = std::ffi::CString::new(result.as_ref()).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_result.as_ptr(), 0)
}

/// `-[NSRegularExpression description]`
pub extern "C" fn shim_regex_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let pattern = unsafe { *((obj + 8) as *const u64) };
    let pat_str = cfstring::extract_cfstring_cstr(pattern).unwrap_or("<nil>");
    let desc = format!("<NSRegularExpression: /{pat_str}/>");
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}
