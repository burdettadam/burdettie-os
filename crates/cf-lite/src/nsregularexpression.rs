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

/// NSRange ABI-compatible struct for ARM64 return values.
#[repr(C)]
pub struct NSRange {
    pub location: u64,
    pub length: u64,
}

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
/// Returns an NSTextCheckingResult or 0 (nil) if no match.
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
    match re.captures(c_str) {
        Some(caps) => make_text_checking_result(&caps),
        None => 0,
    }
}

/// `-[NSRegularExpression matchesInString:options:range:]`
pub extern "C" fn shim_regex_matches_in_string(
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
    let results: Vec<u64> = re.captures_iter(c_str)
        .map(|caps| make_text_checking_result(&caps))
        .collect();
    nsarray::make_nsarray(&results)
}

/// `-[NSRegularExpression enumerateMatchesInString:options:range:usingBlock:]`
pub extern "C" fn shim_regex_enumerate_matches(
    obj: u64,
    _cmd: u64,
    string: u64,
    _options: u64,
    _range_loc: u64,
    _range_len: u64,
    block: u64,
) -> u64 {
    if obj == 0 || string == 0 || block == 0 { return 0; }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };

    // Block layout: isa(0), flags(8), invoke(16)
    let invoke = unsafe { std::ptr::read_unaligned((block + 16) as *const u64) };
    let invoke_fn: extern "C" fn(u64, u64, u64, *mut u8) =
        unsafe { std::mem::transmute(invoke) };

    let mut stop: u8 = 0;
    for caps in re.captures_iter(c_str) {
        let result = make_text_checking_result(&caps);
        invoke_fn(block, result, 0, &mut stop);
        if stop != 0 { break; }
    }
    0
}

// ── NSTextCheckingResult ───────────────────────────────────────────
//
// Layout (24 + 16 * range_count bytes):
//   offset  0: isa          (u64)
//   offset  8: range_count  (u64)
//   offset 16: ranges       (inline array of (u64 location, u64 length))

fn make_text_checking_result(caps: &regex::Captures) -> u64 {
    let n = caps.len(); // group 0 = full match + capture groups
    let size = 16 + 16 * n;
    let obj = crate::pal_alloc().calloc(1, size) as u64;
    unsafe {
        *(obj as *mut u64) = nsobj::nstextcheckingresult_class_addr();
        *((obj + 8) as *mut u64) = n as u64;
        for i in 0..n {
            let (loc, len) = match caps.get(i) {
                Some(m) => (m.start() as u64, m.len() as u64),
                None => (u64::MAX, 0), // NSNotFound = NSUIntegerMax
            };
            *((obj + 16 + (i * 16) as u64) as *mut u64) = loc;
            *((obj + 16 + (i * 16) as u64 + 8) as *mut u64) = len;
        }
    }
    obj
}

/// `-[NSTextCheckingResult range]` — returns NSRange in (x0, x1).
pub extern "C" fn shim_textcheckingresult_range(obj: u64, _cmd: u64) -> NSRange {
    if obj == 0 { return NSRange { location: u64::MAX, length: 0 }; }
    let loc = unsafe { *((obj + 16) as *const u64) };
    let len = unsafe { *((obj + 24) as *const u64) };
    NSRange { location: loc, length: len }
}

/// `-[NSTextCheckingResult rangeAtIndex:]` — returns NSRange for capture group i.
pub extern "C" fn shim_textcheckingresult_range_at_index(obj: u64, _cmd: u64, index: u64) -> NSRange {
    if obj == 0 { return NSRange { location: u64::MAX, length: 0 }; }
    let count = unsafe { *((obj + 8) as *const u64) };
    if index >= count { return NSRange { location: u64::MAX, length: 0 }; }
    let offset = 16 + index * 16;
    let loc = unsafe { *((obj + offset) as *const u64) };
    let len = unsafe { *((obj + offset + 8) as *const u64) };
    NSRange { location: loc, length: len }
}

/// `-[NSTextCheckingResult numberOfRanges]`
pub extern "C" fn shim_textcheckingresult_number_of_ranges(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
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
