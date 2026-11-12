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

const PATTERN_OFF: u64 = 8;
const OPTIONS_OFF: u64 = 16;
const REGEX_OFF: u64 = 24;
const RANGE_COUNT_OFF: u64 = 8;
const RANGES_OFF: u64 = 16;

/// NSRange ABI-compatible struct for ARM64 return values.
#[repr(C)]
pub struct NSRange {
    pub location: u64,
    pub length: u64,
}

pub const INSTANCE_SIZE: u32 = 40;

/// Read the `regex::Regex` pointer stored at offset 24.
unsafe fn get_regex(obj: u64) -> Option<&'static regex::Regex> {
    let regex_ptr = *((obj + REGEX_OFF) as *const u64);
    if regex_ptr == 0 {
        return None;
    }
    Some(&*(regex_ptr as *const regex::Regex))
}

// ── Class methods ──────────────────────────────────────────────────

/// NSRegularExpressionOptions bit masks (Apple NSRegularExpression.h)
const OPT_CASE_INSENSITIVE: u64 = 1 << 0; // 1
const OPT_ALLOW_COMMENTS_WHITESPACE: u64 = 1 << 1; // 2
const OPT_IGNORE_METACHARACTERS: u64 = 1 << 2; // 4
const OPT_DOT_MATCHES_SEPARATORS: u64 = 1 << 3; // 8
const OPT_ANCHORS_MATCH_LINES: u64 = 1 << 4; // 16

/// Build a `regex`-crate pattern string from a raw pattern + NSRegularExpressionOptions.
fn build_regex_pattern(raw_pat: &str, options: u64) -> String {
    // NSRegularExpressionIgnoreMetacharacters: treat pattern as literal string
    let pat = if (options & OPT_IGNORE_METACHARACTERS) != 0 {
        regex::escape(raw_pat)
    } else {
        raw_pat.to_string()
    };

    let ci = (options & OPT_CASE_INSENSITIVE) != 0;
    let x = (options & OPT_ALLOW_COMMENTS_WHITESPACE) != 0;
    let s = (options & OPT_DOT_MATCHES_SEPARATORS) != 0;
    let m = (options & OPT_ANCHORS_MATCH_LINES) != 0;

    if ci || x || s || m {
        let mut flags = String::from("(?");
        if ci {
            flags.push('i');
        }
        if x {
            flags.push('x');
        }
        if s {
            flags.push('s');
        }
        if m {
            flags.push('m');
        }
        flags.push(')');
        flags + &pat
    } else {
        pat
    }
}

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
                unsafe {
                    *(error_out as *mut u64) = err;
                }
            }
            return 0;
        }
    };

    let regex_pat = build_regex_pattern(&pat, options);

    match regex::Regex::new(&regex_pat) {
        Ok(re) => {
            let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
            unsafe {
                *(obj as *mut u64) = nsobj::nsregularexpression_class_addr();
                *((obj + PATTERN_OFF) as *mut u64) = pattern;
                *((obj + OPTIONS_OFF) as *mut u64) = options;
                *((obj + REGEX_OFF) as *mut u64) = Box::into_raw(Box::new(re)) as u64;
            }
            obj
        }
        Err(e) => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror(
                    "NSCocoaErrorDomain",
                    2048, // NSFormattingError
                );
                unsafe {
                    *(error_out as *mut u64) = err;
                }
                log::debug!("NSRegularExpression: invalid pattern: {}", e);
            }
            0
        }
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSRegularExpression initWithPattern:options:error:]`
pub extern "C" fn shim_regex_init_with_pattern(
    obj: u64,
    _cmd: u64,
    pattern: u64,
    options: u64,
    error_out: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let c_str = cfstring::extract_cfstring_cstr(pattern);
    let pat = match c_str {
        Some(s) => s.to_string(),
        None => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror("NSCocoaErrorDomain", 2048);
                unsafe {
                    *(error_out as *mut u64) = err;
                }
            }
            return 0;
        }
    };

    let regex_pat = build_regex_pattern(&pat, options);

    match regex::Regex::new(&regex_pat) {
        Ok(re) => {
            unsafe {
                *(obj as *mut u64) = nsobj::nsregularexpression_class_addr();
                *((obj + PATTERN_OFF) as *mut u64) = pattern;
                *((obj + OPTIONS_OFF) as *mut u64) = options;
                *((obj + REGEX_OFF) as *mut u64) = Box::into_raw(Box::new(re)) as u64;
            }
            log::debug!("NSRegularExpression initWithPattern: obj={:#x}", obj);
            obj
        }
        Err(e) => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror("NSCocoaErrorDomain", 2048);
                unsafe {
                    *(error_out as *mut u64) = err;
                }
                log::debug!("NSRegularExpression: invalid pattern: {}", e);
            }
            0
        }
    }
}

/// `-[NSRegularExpression pattern]`
pub extern "C" fn shim_regex_pattern(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj + PATTERN_OFF) as *const u64) }
}

/// `-[NSRegularExpression options]`
pub extern "C" fn shim_regex_options(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj + OPTIONS_OFF) as *const u64) }
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
    if obj == 0 || string == 0 {
        return 0;
    }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    re.find_iter(c_str).count() as u64
}

fn make_text_checking_result_for_range(location: u64, length: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    unsafe {
        *(obj as *mut u64) = nsobj::nstextcheckingresult_class_addr();
        *((obj + RANGE_COUNT_OFF) as *mut u64) = 1;
        *((obj + RANGES_OFF) as *mut u64) = location;
        *((obj + RANGES_OFF + 8) as *mut u64) = length;
    }
    obj
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
    if obj == 0 || string == 0 {
        return 0;
    }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };

    if re.captures_len() <= 1 {
        return re
            .find(c_str)
            .map(|m| make_text_checking_result_for_range(m.start() as u64, m.len() as u64))
            .unwrap_or(0);
    }

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
    if obj == 0 || string == 0 {
        return 0;
    }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };

    if re.captures_len() <= 1 {
        let results: Vec<u64> = re
            .find_iter(c_str)
            .map(|m| make_text_checking_result_for_range(m.start() as u64, m.len() as u64))
            .collect();
        return nsarray::make_nsarray(&results);
    }

    let results: Vec<u64> = re
        .captures_iter(c_str)
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
    if obj == 0 || string == 0 || block == 0 {
        return 0;
    }
    let Some(re) = (unsafe { get_regex(obj) }) else {
        return 0;
    };
    let Some(c_str) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };

    // Block layout: isa(0), flags(8), invoke(16)
    let invoke = unsafe { std::ptr::read_unaligned((block + 16) as *const u64) };
    let invoke_fn: extern "C" fn(u64, u64, u64, *mut u8) = unsafe { std::mem::transmute(invoke) };

    let mut stop: u8 = 0;
    if re.captures_len() <= 1 {
        for m in re.find_iter(c_str) {
            let result = make_text_checking_result_for_range(m.start() as u64, m.len() as u64);
            invoke_fn(block, result, 0, &mut stop);
            if stop != 0 {
                break;
            }
        }
        return 0;
    }

    for caps in re.captures_iter(c_str) {
        let result = make_text_checking_result(&caps);
        invoke_fn(block, result, 0, &mut stop);
        if stop != 0 {
            break;
        }
    }
    0
}

// ── NSTextCheckingResult ───────────────────────────────────────────
//
// Layout (24 + 16 * range_count bytes):
//   offset  0: isa          (u64)
//   offset  8: range_count  (u64)
//   offset 16: ranges       (inline array of (u64 location, u64 length))

fn make_text_checking_result(caps: &regex::Captures<'_>) -> u64 {
    let n = caps.len();
    let size = 16 + 16 * n;
    let obj = crate::pal_alloc().calloc(1, size) as u64;
    unsafe {
        *(obj as *mut u64) = nsobj::nstextcheckingresult_class_addr();
        *((obj + RANGE_COUNT_OFF) as *mut u64) = n as u64;
        for i in 0..n {
            let (loc, len) = match caps.get(i) {
                Some(m) => (m.start() as u64, m.len() as u64),
                None => (u64::MAX, 0), // NSNotFound = NSUIntegerMax
            };
            *((obj + RANGES_OFF + (i * 16) as u64) as *mut u64) = loc;
            *((obj + RANGES_OFF + (i * 16) as u64 + 8) as *mut u64) = len;
        }
    }
    obj
}

/// `-[NSTextCheckingResult range]` — returns NSRange in (x0, x1).
pub extern "C" fn shim_textcheckingresult_range(obj: u64, _cmd: u64) -> NSRange {
    if obj == 0 {
        return NSRange {
            location: u64::MAX,
            length: 0,
        };
    }
    let loc = unsafe { *((obj + RANGES_OFF) as *const u64) };
    let len = unsafe { *((obj + RANGES_OFF + 8) as *const u64) };
    NSRange {
        location: loc,
        length: len,
    }
}

/// `-[NSTextCheckingResult rangeAtIndex:]` — returns NSRange for capture group i.
pub extern "C" fn shim_textcheckingresult_range_at_index(
    obj: u64,
    _cmd: u64,
    index: u64,
) -> NSRange {
    if obj == 0 {
        return NSRange {
            location: u64::MAX,
            length: 0,
        };
    }
    let count = unsafe { *((obj + RANGE_COUNT_OFF) as *const u64) };
    if index >= count {
        return NSRange {
            location: u64::MAX,
            length: 0,
        };
    }
    let offset = RANGES_OFF + index * 16;
    let loc = unsafe { *((obj + offset) as *const u64) };
    let len = unsafe { *((obj + offset + 8) as *const u64) };
    NSRange {
        location: loc,
        length: len,
    }
}

/// `-[NSTextCheckingResult numberOfRanges]`
pub extern "C" fn shim_textcheckingresult_number_of_ranges(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj + RANGE_COUNT_OFF) as *const u64) }
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
    if obj == 0 || string == 0 || template == 0 {
        return 0;
    }
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

/// `-[NSRegularExpression replacementStringForResult:inString:offset:template:]`
///
/// Returns the template string for a specific match result. Simplified:
/// returns the template as-is.
pub extern "C" fn shim_nsregex_replacement_string_for_result(
    obj: u64,
    _cmd: u64,
    _result: u64,
    _in_string: u64,
    _offset: u64,
    template: u64,
) -> u64 {
    if obj == 0 || template == 0 {
        return 0;
    }
    template
}

/// `+[NSRegularExpression escapedPatternForString:]`
///
/// Escapes regex metacharacters in the input string.
pub extern "C" fn shim_nsregex_escaped_pattern_for_string(
    _cls: u64,
    _cmd: u64,
    string: u64,
) -> u64 {
    if string == 0 {
        return 0;
    }
    let Some(s) = cfstring::extract_cfstring_cstr(string) else {
        return 0;
    };
    let mut escaped = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '[' | ']' | '|' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    let c_str = std::ffi::CString::new(escaped).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSRegularExpression description]`
pub extern "C" fn shim_regex_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let pattern = unsafe { *((obj + PATTERN_OFF) as *const u64) };
    let pat_str = cfstring::extract_cfstring_cstr(pattern).unwrap_or("<nil>");
    let desc = format!("<NSRegularExpression: /{pat_str}/>");
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_regex(pattern: &str) -> u64 {
        let pat = make_string(pattern);
        shim_regex_with_pattern(0, 0, pat, 0, 0)
    }

    #[test]
    fn create_simple_pattern() {
        let re = make_regex(r"\d+");
        assert_ne!(re, 0, "valid pattern should produce non-nil");
        free_obj(re);
    }

    #[test]
    fn invalid_pattern_returns_nil() {
        let bad = make_string(r"[invalid");
        assert_eq!(shim_regex_with_pattern(0, 0, bad, 0, 0), 0);
    }

    #[test]
    fn nil_pattern_returns_nil() {
        assert_eq!(shim_regex_with_pattern(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn pattern_stored_correctly() {
        let pat_str = make_string(r"\w+");
        let re = shim_regex_with_pattern(0, 0, pat_str, 0, 0);
        assert_ne!(re, 0);
        assert_eq!(shim_regex_pattern(re, 0), pat_str);
        free_obj(re);
    }

    #[test]
    fn options_stored_correctly() {
        let pat = make_string("test");
        let re = shim_regex_with_pattern(0, 0, pat, 1, 0);
        assert_ne!(re, 0);
        assert_eq!(shim_regex_options(re, 0), 1);
        free_obj(re);
    }

    #[test]
    fn number_of_matches_basic() {
        let re = make_regex(r"\d+");
        let s = make_string("abc 123 def 456");
        assert_eq!(shim_regex_number_of_matches(re, 0, s, 0, 0, 0), 2);
        free_obj(re);
    }

    #[test]
    fn number_of_matches_no_match() {
        let re = make_regex(r"\d+");
        let s = make_string("no digits here");
        assert_eq!(shim_regex_number_of_matches(re, 0, s, 0, 0, 0), 0);
        free_obj(re);
    }

    #[test]
    fn number_of_matches_nil_safety() {
        let s = make_string("hello 42");
        assert_eq!(shim_regex_number_of_matches(0, 0, s, 0, 0, 0), 0);
        let re = make_regex(r"\d+");
        assert_eq!(shim_regex_number_of_matches(re, 0, 0, 0, 0, 0), 0);
        free_obj(re);
    }

    #[test]
    fn first_match_location_and_length() {
        let re = make_regex(r"\d+");
        let s = make_string("abc 42 def");
        let result = shim_regex_first_match(re, 0, s, 0, 0, 0);
        assert_ne!(result, 0);
        let range = shim_textcheckingresult_range(result, 0);
        assert_eq!(range.location, 4);
        assert_eq!(range.length, 2);
        free_obj(result);
        free_obj(re);
    }

    #[test]
    fn first_match_no_match_returns_nil() {
        let re = make_regex(r"\d+");
        let s = make_string("no digits");
        assert_eq!(shim_regex_first_match(re, 0, s, 0, 0, 0), 0);
        free_obj(re);
    }

    #[test]
    fn first_match_without_capture_groups_has_single_range() {
        let re = make_regex(r"\d+");
        let s = make_string("abc 42 def");
        let result = shim_regex_first_match(re, 0, s, 0, 0, 0);
        assert_ne!(result, 0);
        assert_eq!(shim_textcheckingresult_number_of_ranges(result, 0), 1);
        let range = shim_textcheckingresult_range_at_index(result, 0, 0);
        assert_eq!(range.location, 4);
        assert_eq!(range.length, 2);
        free_obj(result);
        free_obj(re);
    }

    #[test]
    fn matches_in_string_without_capture_groups_uses_single_range_results() {
        let re = make_regex(r"\d+");
        let s = make_string("abc 12 def 345");
        let arr = shim_regex_matches_in_string(re, 0, s, 0, 0, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);

        let first = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_ne!(first, 0);
        assert_eq!(shim_textcheckingresult_number_of_ranges(first, 0), 1);
        let range = shim_textcheckingresult_range(first, 0);
        assert_eq!(range.location, 4);
        assert_eq!(range.length, 2);

        free_obj(re);
    }

    #[test]
    fn matches_in_string_with_capture_groups_preserves_group_ranges() {
        let re = make_regex(r"(\w+)=(\w+)");
        let s = make_string("host=localhost port=5432");
        let arr = shim_regex_matches_in_string(re, 0, s, 0, 0, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);

        let first = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_ne!(first, 0);
        assert_eq!(shim_textcheckingresult_number_of_ranges(first, 0), 3);

        let whole = shim_textcheckingresult_range_at_index(first, 0, 0);
        let key = shim_textcheckingresult_range_at_index(first, 0, 1);
        let value = shim_textcheckingresult_range_at_index(first, 0, 2);
        assert_eq!((whole.location, whole.length), (0, 14));
        assert_eq!((key.location, key.length), (0, 4));
        assert_eq!((value.location, value.length), (5, 9));

        free_obj(re);
    }

    #[test]
    fn replace_all_matches() {
        let re = make_regex(r"\d+");
        let s = make_string("a1b2c3");
        let tmpl = make_string("X");
        let out = shim_regex_replace(re, 0, s, 0, 0, 0, tmpl);
        assert_ne!(out, 0);
        assert_string_eq(out, "aXbXcX");
        free_obj(re);
    }

    #[test]
    fn description_contains_pattern() {
        let re = make_regex("hello");
        let desc = shim_regex_description(re, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSRegularExpression: /hello/>");
        free_obj(re);
    }

    #[test]
    fn nil_obj_accessors_return_zero() {
        assert_eq!(shim_regex_pattern(0, 0), 0);
        assert_eq!(shim_regex_options(0, 0), 0);
        assert_eq!(shim_regex_description(0, 0), 0);
    }

    #[test]
    fn replacement_string_for_result_returns_template() {
        let re = make_regex(r"\d+");
        let tmpl = make_string("replacement");
        let result = shim_nsregex_replacement_string_for_result(re, 0, 0, 0, 0, tmpl);
        assert_eq!(result, tmpl);
        free_obj(re);
    }

    #[test]
    fn replacement_string_nil_safety() {
        assert_eq!(
            shim_nsregex_replacement_string_for_result(0, 0, 0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn escaped_pattern_for_string_escapes_metacharacters() {
        let input = make_string("hello.world*foo+bar");
        let result = shim_nsregex_escaped_pattern_for_string(0, 0, input);
        assert_ne!(result, 0);
        assert_string_eq(result, r"hello\.world\*foo\+bar");
    }

    #[test]
    fn escaped_pattern_plain_string_unchanged() {
        let input = make_string("hello world");
        let result = shim_nsregex_escaped_pattern_for_string(0, 0, input);
        assert_ne!(result, 0);
        assert_string_eq(result, "hello world");
    }

    #[test]
    fn escaped_pattern_nil_returns_zero() {
        assert_eq!(shim_nsregex_escaped_pattern_for_string(0, 0, 0), 0);
    }
}
