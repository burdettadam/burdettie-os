//! NSString URL percent-encoding and byte-length shims.

use std::ffi::CString;

use crate::cfstring::{
    cfstring_parts, extract_cfstring_cstr, shim_cf_string_create_with_cstring, CF_STRING_FLAGS_UTF8,
};
use crate::nscharacterset;

/// `-[NSString stringByAddingPercentEncodingWithAllowedCharacters:]`
///
/// Percent-encodes characters NOT in the given `NSCharacterSet`.
pub extern "C" fn shim_nsstring_string_by_adding_percent_encoding(
    obj: u64,
    _cmd: u64,
    allowed_chars: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };

    let mut encoded = String::with_capacity(s.len());
    for ch in s.chars() {
        let allowed = if allowed_chars != 0 {
            nscharacterset::character_set_contains(allowed_chars, ch)
        } else {
            // Fallback: RFC 3986 unreserved set
            ch.is_ascii_alphanumeric() || "-._~".contains(ch)
        };
        if allowed {
            encoded.push(ch);
        } else {
            // Percent-encode each UTF-8 byte
            let mut buf = [0u8; 4];
            let bytes = ch.encode_utf8(&mut buf);
            for b in bytes.as_bytes() {
                use std::fmt::Write;
                write!(encoded, "%{:02X}", b).unwrap();
            }
        }
    }

    let c_str = CString::new(encoded).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString stringByRemovingPercentEncoding]`
///
/// Decodes percent-encoded sequences (`%XX`) back to their byte values.
pub extern "C" fn shim_nsstring_string_by_removing_percent_encoding(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(s) = extract_cfstring_cstr(obj) else {
        return 0;
    };

    let mut decoded = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                decoded.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    // decoded bytes should be valid UTF-8 in most cases
    let text = String::from_utf8_lossy(&decoded);
    let c_str = CString::new(text.as_ref()).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// `-[NSString initWithBytes:length:encoding:]`
///
/// Initializes an NSString from raw bytes. Treats all encodings as UTF-8.
pub extern "C" fn shim_nsstring_init_with_bytes_length_encoding(
    obj: u64,
    _cmd: u64,
    bytes: u64,
    len: u64,
    _encoding: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let length = len as usize;
    if bytes == 0 || length == 0 {
        // Empty string
        let owned = CString::new("").unwrap();
        let owned_ptr = owned.into_raw();
        unsafe {
            let p = obj as *mut u8;
            *((p.add(8)) as *mut u32) = CF_STRING_FLAGS_UTF8;
            *((p.add(16)) as *mut u64) = owned_ptr as u64;
            *((p.add(24)) as *mut u64) = 0;
        }
        return obj;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, length) };
    let s = String::from_utf8_lossy(slice);
    let owned = CString::new(s.as_ref()).unwrap_or_default();
    let byte_len = owned.as_bytes().len();
    let owned_ptr = owned.into_raw();

    unsafe {
        let p = obj as *mut u8;
        *((p.add(8)) as *mut u32) = CF_STRING_FLAGS_UTF8;
        *((p.add(16)) as *mut u64) = owned_ptr as u64;
        *((p.add(24)) as *mut u64) = byte_len as u64;
    }

    obj
}

/// `-[NSString lengthOfBytesUsingEncoding:]`
///
/// Returns the UTF-8 byte length of the string.
pub extern "C" fn shim_nsstring_length_of_bytes_using_encoding(
    obj: u64,
    _cmd: u64,
    _encoding: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some((_bytes, len)) = cfstring_parts(obj) else {
        return 0;
    };
    len as u64
}

/// `-[NSString maximumLengthOfBytesUsingEncoding:]`
///
/// Returns the worst-case byte length (4 * character count for UTF-8).
pub extern "C" fn shim_nsstring_maximum_length_of_bytes_using_encoding(
    obj: u64,
    _cmd: u64,
    _encoding: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // length returns the UTF-16 code unit count; worst case for UTF-8 is 4 bytes per code unit
    let char_len = crate::cfstring::shim_cf_string_get_length(obj) as u64;
    char_len * 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── stringByAddingPercentEncodingWithAllowedCharacters: ───────

    #[test]
    fn percent_encode_nil_returns_zero() {
        assert_eq!(shim_nsstring_string_by_adding_percent_encoding(0, 0, 0), 0);
    }

    #[test]
    fn percent_encode_unreserved_passthrough() {
        let s = make_string("hello-world_2.0~test");
        let result = shim_nsstring_string_by_adding_percent_encoding(s, 0, 0);
        assert_ne!(result, 0);
        // With nil allowed set, unreserved chars pass through
        assert_string_eq(result, "hello-world_2.0~test");
        free_obj(s);
    }

    #[test]
    fn percent_encode_space_and_special() {
        let s = make_string("hello world&foo=bar");
        let result = shim_nsstring_string_by_adding_percent_encoding(s, 0, 0);
        assert_ne!(result, 0);
        let encoded = crate::cfstring::extract_cfstring_cstr(result)
            .unwrap()
            .to_string();
        assert!(
            encoded.contains("%20"),
            "space should be encoded: {encoded}"
        );
        assert!(encoded.contains("%26"), "& should be encoded: {encoded}");
        assert!(encoded.contains("%3D"), "= should be encoded: {encoded}");
        free_obj(s);
    }

    #[test]
    fn percent_encode_with_url_query_allowed() {
        let s = make_string("key=value&other=test");
        let charset = crate::nscharacterset::shim_nscharacterset_url_query_allowed(0, 0);
        let result = shim_nsstring_string_by_adding_percent_encoding(s, 0, charset);
        assert_ne!(result, 0);
        // All chars are URL-query-allowed, should pass through
        assert_string_eq(result, "key=value&other=test");
        free_obj(s);
        free_obj(charset);
    }

    // ── stringByRemovingPercentEncoding ───────────────────────────

    #[test]
    fn percent_decode_nil_returns_zero() {
        assert_eq!(shim_nsstring_string_by_removing_percent_encoding(0, 0), 0);
    }

    #[test]
    fn percent_decode_basic() {
        let s = make_string("hello%20world");
        let result = shim_nsstring_string_by_removing_percent_encoding(s, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "hello world");
        free_obj(s);
    }

    #[test]
    fn percent_decode_no_encoding() {
        let s = make_string("plain");
        let result = shim_nsstring_string_by_removing_percent_encoding(s, 0);
        assert_string_eq(result, "plain");
        free_obj(s);
    }

    #[test]
    fn percent_decode_multiple_sequences() {
        let s = make_string("%48%65%6C%6C%6F");
        let result = shim_nsstring_string_by_removing_percent_encoding(s, 0);
        assert_string_eq(result, "Hello");
        free_obj(s);
    }

    #[test]
    fn percent_encode_then_decode_roundtrip() {
        let orig = make_string("café résumé");
        let encoded = shim_nsstring_string_by_adding_percent_encoding(orig, 0, 0);
        let decoded = shim_nsstring_string_by_removing_percent_encoding(encoded, 0);
        assert_string_eq(decoded, "café résumé");
        free_obj(orig);
    }

    // ── initWithBytes:length:encoding: ───────────────────────────

    #[test]
    fn init_with_bytes_nil_returns_zero() {
        assert_eq!(
            shim_nsstring_init_with_bytes_length_encoding(0, 0, 0, 0, 4),
            0
        );
    }

    #[test]
    fn init_with_bytes_basic() {
        let obj = alloc_obj(32, crate::nsobj::nsstring_class_addr());
        let data = b"hello bytes";
        let result = shim_nsstring_init_with_bytes_length_encoding(
            obj,
            0,
            data.as_ptr() as u64,
            data.len() as u64,
            4,
        );
        assert_eq!(result, obj);
        assert_string_eq(result, "hello bytes");
        free_obj(obj);
    }

    #[test]
    fn init_with_bytes_empty() {
        let obj = alloc_obj(32, crate::nsobj::nsstring_class_addr());
        let result = shim_nsstring_init_with_bytes_length_encoding(obj, 0, 0, 0, 4);
        assert_eq!(result, obj);
        assert_string_eq(result, "");
        free_obj(obj);
    }

    // ── lengthOfBytesUsingEncoding: ──────────────────────────────

    #[test]
    fn length_of_bytes_nil_returns_zero() {
        assert_eq!(shim_nsstring_length_of_bytes_using_encoding(0, 0, 4), 0);
    }

    #[test]
    fn length_of_bytes_ascii() {
        let s = make_string("hello");
        assert_eq!(shim_nsstring_length_of_bytes_using_encoding(s, 0, 4), 5);
        free_obj(s);
    }

    #[test]
    fn length_of_bytes_multibyte() {
        let s = make_string("café");
        // "café" is 5 UTF-8 bytes (c=1, a=1, f=1, é=2)
        assert_eq!(shim_nsstring_length_of_bytes_using_encoding(s, 0, 4), 5);
        free_obj(s);
    }

    // ── maximumLengthOfBytesUsingEncoding: ────────────────────────

    #[test]
    fn max_length_of_bytes_nil_returns_zero() {
        assert_eq!(
            shim_nsstring_maximum_length_of_bytes_using_encoding(0, 0, 4),
            0
        );
    }

    #[test]
    fn max_length_of_bytes_basic() {
        let s = make_string("hi");
        let max = shim_nsstring_maximum_length_of_bytes_using_encoding(s, 0, 4);
        // length("hi") = 2 chars, max = 2*4 = 8
        assert_eq!(max, 8);
        free_obj(s);
    }
}
