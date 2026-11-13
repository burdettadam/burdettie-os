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

/// `-[NSString stringByReplacingUnicodeCharacterEscapeSequences:]`
///
/// Replaces `\UXXXX` sequences with the corresponding Unicode character.
/// For strings without escape sequences this is an identity operation.
pub extern "C" fn shim_nsstring_string_by_replacing_unicode_escapes(
    obj: u64,
    _sel: u64,
    _flag: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let s_opt = extract_cfstring_cstr(obj);
    let Some(s) = s_opt else {
        return obj;
    };
    // Fast path: no escape sequences → return self.
    if !s.contains("\\U") && !s.contains("\\u") {
        return obj;
    }
    // Slow path: replace \UXXXX / \uXXXX with the corresponding character.
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('U') | Some('u') => {
                    chars.next(); // consume 'U'/'u'
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                                continue;
                            }
                        }
                    }
                    // Failed to parse — emit the original sequence.
                    result.push('\\');
                    result.push('U');
                    result.push_str(&hex);
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    let c_str = CString::new(result).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
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

/// NSStringCompareOptions bits used by `rangeOfString:options:range:`.
const NS_CASE_INSENSITIVE_SEARCH: u64 = 1;
const NS_BACKWARDS_SEARCH: u64 = 4;
const NS_ANCHORED_SEARCH: u64 = 8;

/// `-[NSString rangeOfString:options:range:]`
///
/// Search for `search` within a restricted byte range of `obj`, respecting
/// option flags (case-insensitive, backwards, anchored).
///
/// On ARM64 the NSRange parameter is passed in x4 (range_location) and
/// x5 (range_length).
pub extern "C" fn shim_nsstring_range_of_string_options_range(
    obj: u64,
    _sel: u64,
    search: u64,
    options: u64,
    range_location: u64,
    range_length: u64,
) -> NSRange {
    let not_found = NSRange {
        location: NS_NOT_FOUND,
        length: 0,
    };
    let Some(s) = extract_cfstring_cstr(obj) else {
        return not_found;
    };
    let Some(needle) = extract_cfstring_cstr(search) else {
        return not_found;
    };

    let loc = range_location as usize;
    let len = range_length as usize;
    if loc + len > s.len() || needle.is_empty() {
        return not_found;
    }
    let haystack = &s[loc..loc + len];

    let case_insensitive = (options & NS_CASE_INSENSITIVE_SEARCH) != 0;
    let backwards = (options & NS_BACKWARDS_SEARCH) != 0;
    let anchored = (options & NS_ANCHORED_SEARCH) != 0;

    let needle_bytes = needle.as_bytes();

    let haystack_bytes = haystack.as_bytes();

    if anchored {
        // Anchored: only match at the start (or end if backwards).
        if backwards {
            if haystack_bytes.len() >= needle_bytes.len() {
                let candidate = &haystack_bytes[haystack_bytes.len() - needle_bytes.len()..];
                let matched = if case_insensitive {
                    candidate.eq_ignore_ascii_case(needle_bytes)
                } else {
                    candidate == needle_bytes
                };
                if matched {
                    return NSRange {
                        location: (loc + haystack_bytes.len() - needle_bytes.len()) as u64,
                        length: needle_bytes.len() as u64,
                    };
                }
            }
        } else if haystack_bytes.len() >= needle_bytes.len() {
            let candidate = &haystack_bytes[..needle_bytes.len()];
            let matched = if case_insensitive {
                candidate.eq_ignore_ascii_case(needle_bytes)
            } else {
                candidate == needle_bytes
            };
            if matched {
                return NSRange {
                    location: loc as u64,
                    length: needle_bytes.len() as u64,
                };
            }
        }
        return not_found;
    }

    if case_insensitive {
        let hay_lower: Vec<u8> = haystack.bytes().map(|b| b.to_ascii_lowercase()).collect();
        let needle_lower: Vec<u8> = needle_bytes
            .iter()
            .map(|b| b.to_ascii_lowercase())
            .collect();
        if backwards {
            if let Some(pos) = hay_lower
                .windows(needle_lower.len())
                .rposition(|w| w == needle_lower.as_slice())
            {
                return NSRange {
                    location: (loc + pos) as u64,
                    length: needle_lower.len() as u64,
                };
            }
        } else if let Some(pos) = hay_lower
            .windows(needle_lower.len())
            .position(|w| w == needle_lower.as_slice())
        {
            return NSRange {
                location: (loc + pos) as u64,
                length: needle_lower.len() as u64,
            };
        }
        return not_found;
    }

    // Default: literal search.
    if backwards {
        if let Some(pos) = haystack.rfind(needle) {
            return NSRange {
                location: (loc + pos) as u64,
                length: needle_bytes.len() as u64,
            };
        }
    } else if let Some(pos) = haystack.find(needle) {
        return NSRange {
            location: (loc + pos) as u64,
            length: needle_bytes.len() as u64,
        };
    }

    not_found
}

/// `-[NSString rangeOfString:options:]`
///
/// Search with options but no range restriction (full string).
pub extern "C" fn shim_nsstring_range_of_string_options(
    obj: u64,
    _sel: u64,
    search: u64,
    options: u64,
) -> NSRange {
    let s_len = {
        let Some(s) = extract_cfstring_cstr(obj) else {
            return NSRange {
                location: NS_NOT_FOUND,
                length: 0,
            };
        };
        s.len() as u64
    };
    shim_nsstring_range_of_string_options_range(obj, _sel, search, options, 0, s_len)
}

/// `-[NSString matchMemberOfArray:fromPosition:whichMember:]`
///
/// Returns the earliest matching range for any array member at or after
/// `position`, and writes the matching NSString object into `*which_member`
/// (or nil on no match). This is an ELSAdditions category installed by
/// the `genstrings` tool to scan source files for localisation macros.
pub extern "C" fn shim_nsstring_match_member_of_array_from_position_which_member(
    obj: u64,
    _sel: u64,
    members: u64,
    position: u64,
    which_member: u64,
) -> NSRange {
    // Clear the out-param unconditionally so the caller never sees a stale value.
    if which_member != 0 {
        unsafe {
            *(which_member as *mut u64) = 0;
        }
    }

    let Some(s) = extract_cfstring_cstr(obj) else {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    };
    if members == 0 {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    }

    let pos = position as usize;
    let bytes = s.as_bytes();
    if pos > bytes.len() {
        return NSRange {
            location: NS_NOT_FOUND,
            length: 0,
        };
    }
    let suffix = &bytes[pos..];

    let count = crate::nsarray::shim_nsarray_count(members, 0);
    let mut best_member_idx: u64 = 0;
    let mut best_range = NSRange {
        location: NS_NOT_FOUND,
        length: 0,
    };

    for i in 0..count {
        let candidate_obj = crate::nsarray::shim_nsarray_object_at_index(members, 0, i);
        let Some(candidate) = extract_cfstring_cstr(candidate_obj) else {
            continue;
        };
        let candidate_bytes = candidate.as_bytes();
        if candidate_bytes.is_empty() || candidate_bytes.len() > suffix.len() {
            continue;
        }

        let Some(relative_loc) = suffix
            .windows(candidate_bytes.len())
            .position(|window| window == candidate_bytes)
        else {
            continue;
        };

        let candidate_range = NSRange {
            location: (pos + relative_loc) as u64,
            length: candidate_bytes.len() as u64,
        };
        let should_replace = best_range.location == NS_NOT_FOUND
            || candidate_range.location < best_range.location
            || (candidate_range.location == best_range.location
                && candidate_range.length > best_range.length);

        if should_replace {
            best_range = candidate_range;
            best_member_idx = i;
        }
    }

    // Write the array index of the winning member into the out-param.
    // genstrings uses this as an index into its keyword-pattern index sets.
    if which_member != 0 {
        unsafe {
            *(which_member as *mut i64) = best_member_idx as i64;
        }
    }

    best_range
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn range_of_string_found() {
        let haystack = make_string("hello world");
        let needle = make_string("world");
        let range = shim_nsstring_range_of_string(haystack, 0, needle);
        assert_eq!(range.location, 6);
        assert_eq!(range.length, 5);
        free_obj(haystack);
        free_obj(needle);
    }

    #[test]
    fn range_of_string_not_found() {
        let haystack = make_string("hello");
        let needle = make_string("xyz");
        let range = shim_nsstring_range_of_string(haystack, 0, needle);
        assert_eq!(range.location, NS_NOT_FOUND);
        assert_eq!(range.length, 0);
        free_obj(haystack);
        free_obj(needle);
    }

    #[test]
    fn range_of_string_nil_receiver() {
        let needle = make_string("x");
        let range = shim_nsstring_range_of_string(0, 0, needle);
        assert_eq!(range.location, NS_NOT_FOUND);
        free_obj(needle);
    }

    #[test]
    fn components_separated_by_string() {
        let s = make_string("a,b,c");
        let sep = make_string(",");
        let arr = shim_nsstring_components_separated_by_string(s, 0, sep);
        assert_ne!(arr, 0);
        let count = unsafe { *((arr as usize + 8) as *const u64) };
        assert_eq!(count, 3);
        free_obj(s);
        free_obj(sep);
        free_obj(arr);
    }

    #[test]
    fn integer_value_basic() {
        let s = make_string("42");
        assert_eq!(shim_nsstring_integer_value(s, 0), 42);
        free_obj(s);
    }

    #[test]
    fn integer_value_negative_with_suffix() {
        let s = make_string("-17 extra");
        assert_eq!(shim_nsstring_integer_value(s, 0), -17);
        free_obj(s);
    }

    #[test]
    fn integer_value_nil() {
        assert_eq!(shim_nsstring_integer_value(0, 0), 0);
    }

    #[test]
    fn double_value_basic() {
        let s = make_string("3.25");
        let v = shim_nsstring_double_value(s, 0);
        assert!((v - 3.25).abs() < 1e-6, "expected 3.25 got {v}");
        free_obj(s);
    }

    #[test]
    fn string_by_replacing_occurrences() {
        let s = make_string("foo foo foo");
        let target = make_string("foo");
        let repl = make_string("bar");
        let result = shim_nsstring_string_by_replacing(s, 0, target, repl);
        assert_string_eq(result, "bar bar bar");
        free_obj(s);
        free_obj(target);
        free_obj(repl);
        free_obj(result);
    }

    #[test]
    fn string_by_replacing_nil_receiver() {
        assert_eq!(shim_nsstring_string_by_replacing(0, 0, 0, 0), 0);
    }

    #[test]
    fn match_member_of_array_prefers_longest_match() {
        let receiver = make_string("xxfoobar");
        let foo = make_string("foo");
        let foobar = make_string("foobar");
        let bar = make_string("bar");
        let members = crate::nsarray::make_nsarray(&[foo, foobar, bar]);
        let mut which: u64 = 0;

        let matched = shim_nsstring_match_member_of_array_from_position_which_member(
            receiver,
            0,
            members,
            0,
            (&mut which as *mut u64) as u64,
        );

        assert_eq!(matched.location, 2);
        assert_eq!(matched.length, 6);
        // which_member receives the index of the matching member in the array.
        assert_eq!(which, 1);
    }

    #[test]
    fn match_member_of_array_respects_position() {
        let receiver = make_string("foobar");
        let foo = make_string("foo");
        let bar = make_string("bar");
        let members = crate::nsarray::make_nsarray(&[foo, bar]);
        let mut which: u64 = 0;

        let matched = shim_nsstring_match_member_of_array_from_position_which_member(
            receiver,
            0,
            members,
            3,
            (&mut which as *mut u64) as u64,
        );

        assert_eq!(matched.location, 3);
        assert_eq!(matched.length, 3);
        // which_member receives the index of the matching member in the array.
        assert_eq!(which, 1);
    }

    #[test]
    fn match_member_of_array_prefers_earlier_match_over_later_longer_one() {
        let receiver = make_string("bar foobar");
        let foobar = make_string("foobar");
        let bar = make_string("bar");
        let members = crate::nsarray::make_nsarray(&[foobar, bar]);
        let mut which: u64 = 0;

        let matched = shim_nsstring_match_member_of_array_from_position_which_member(
            receiver,
            0,
            members,
            0,
            (&mut which as *mut u64) as u64,
        );

        assert_eq!(matched.location, 0);
        assert_eq!(matched.length, 3);
        // which_member receives the index of the matching member in the array.
        assert_eq!(which, 1);
    }

    #[test]
    fn match_member_of_array_no_match_clears_out_param() {
        let receiver = make_string("foobar");
        let baz = make_string("baz");
        let members = crate::nsarray::make_nsarray(&[baz]);
        let mut which: u64 = 0xDEAD_BEEF;

        let matched = shim_nsstring_match_member_of_array_from_position_which_member(
            receiver,
            0,
            members,
            0,
            (&mut which as *mut u64) as u64,
        );

        assert_eq!(matched.location, NS_NOT_FOUND);
        assert_eq!(matched.length, 0);
        // which_member must be cleared to nil (0) on no match.
        assert_eq!(which, 0);
    }
}
