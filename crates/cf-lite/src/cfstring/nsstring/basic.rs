//! Core NSString shims: UTF8String, length, init, create, compare, format, hash.

use std::ffi::{c_char, CStr, CString};

use crate::cfstring::{
    cfstring_parts, extract_cfstring_cstr, shim_cf_string_create_with_cstring,
    shim_cf_string_get_cstring_ptr, K_CF_STRING_ENCODING_UTF8, CF_STRING_FLAGS_UTF8,
};

const NS_CASE_INSENSITIVE_SEARCH: u64 = 1;
const NS_UTF8_STRING_ENCODING: u64 = 4;
const NS_UTF16_STRING_ENCODING: u64 = 10;
const NS_UTF16_BE_STRING_ENCODING: u64 = 0x9000_0100;
const NS_UTF16_LE_STRING_ENCODING: u64 = 0x9400_0100;
const CF_UTF8_STRING_ENCODING: u64 = K_CF_STRING_ENCODING_UTF8 as u64;

fn compare_strings(a: &str, b: &str, options: u64) -> i64 {
    let ordering = if options & NS_CASE_INSENSITIVE_SEARCH != 0 {
        a.to_lowercase().cmp(&b.to_lowercase())
    } else {
        a.cmp(b)
    };

    match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn localized_name_for_encoding(encoding: u64) -> String {
    match encoding {
        1 => "Western (ASCII)".to_string(),
        4 => "Unicode (UTF-8)".to_string(),
        5 => "Western (ISO Latin 1)".to_string(),
        10 => "Unicode (UTF-16)".to_string(),
        12 => "Western (Windows Latin 1)".to_string(),
        30 => "Western (Mac OS Roman)".to_string(),
        0x8C00_0100 => "Unicode (UTF-32)".to_string(),
        0x9000_0100 => "Unicode (UTF-16BE)".to_string(),
        0x9400_0100 => "Unicode (UTF-16LE)".to_string(),
        other => format!("Encoding {}", other),
    }
}

fn clear_error_slot(error: u64) {
    if error != 0 {
        unsafe {
            *(error as *mut u64) = 0;
        }
    }
}

fn normalized_file_encoding(encoding: u64) -> u64 {
    match encoding {
        0 | CF_UTF8_STRING_ENCODING => NS_UTF8_STRING_ENCODING,
        other => other,
    }
}

fn decode_utf16_contents(bytes: &[u8], encoding: u64) -> Option<String> {
    if bytes.len() % 2 != 0 {
        return None;
    }

    let little_endian = match encoding {
        NS_UTF16_LE_STRING_ENCODING => true,
        NS_UTF16_BE_STRING_ENCODING => false,
        _ if bytes.starts_with(&[0xFF, 0xFE]) => true,
        _ if bytes.starts_with(&[0xFE, 0xFF]) => false,
        _ => true,
    };

    let start = if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        2
    } else {
        0
    };

    let units: Vec<u16> = bytes[start..]
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();

    String::from_utf16(&units).ok()
}

fn read_string_contents_from_file(path: &str, encoding: u64) -> Option<(String, u64)> {
    let bytes = std::fs::read(path).ok()?;
    let normalized_encoding = normalized_file_encoding(encoding);
    let contents = match normalized_encoding {
        NS_UTF16_STRING_ENCODING | NS_UTF16_BE_STRING_ENCODING | NS_UTF16_LE_STRING_ENCODING => {
            decode_utf16_contents(&bytes, normalized_encoding)?
        }
        _ => String::from_utf8_lossy(&bytes).into_owned(),
    };

    Some((contents, normalized_encoding))
}

fn read_cstring_from_file_path(
    path_obj: u64,
    encoding: u64,
    used_encoding: u64,
    error: u64,
) -> Option<CString> {
    clear_error_slot(error);
    if used_encoding != 0 {
        unsafe {
            *(used_encoding as *mut u64) = 0;
        }
    }

    let path = extract_cfstring_cstr(path_obj)?;
    let (contents, resolved_encoding) = read_string_contents_from_file(path, encoding)?;

    if used_encoding != 0 {
        unsafe {
            *(used_encoding as *mut u64) = resolved_encoding;
        }
    }

    Some(CString::new(contents).unwrap_or_default())
}

/// `id -[NSString UTF8String]`
pub extern "C" fn shim_nsstring_utf8string(obj: u64, _sel: u64) -> *const c_char {
    if obj == 0 {
        return std::ptr::null();
    }
    shim_cf_string_get_cstring_ptr(obj, K_CF_STRING_ENCODING_UTF8)
}

/// `const char *-[NSString cStringUsingEncoding:]`
pub extern "C" fn shim_nsstring_cstring_using_encoding(
    obj: u64,
    _sel: u64,
    _encoding: u64,
) -> *const c_char {
    shim_nsstring_utf8string(obj, _sel)
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

/// `+[NSString stringWithCString:encoding:]`
pub extern "C" fn shim_nsstring_string_with_cstring_encoding(
    cls: u64,
    _sel: u64,
    c_str: *const c_char,
    _encoding: u64,
) -> u64 {
    // Delegate to stringWithUTF8String: — encoding is ignored (we treat all as UTF-8)
    shim_nsstring_string_with_utf8string(cls, _sel, c_str)
}

/// `+[NSString stringWithString:]`
pub extern "C" fn shim_nsstring_string_with_string(
    _cls: u64,
    _sel: u64,
    source: u64,
) -> u64 {
    if source == 0 {
        return 0;
    }
    // Extract the C string from the source NSString and create a new copy
    let Some(s) = extract_cfstring_cstr(source) else {
        return 0;
    };
    let c_str = CString::new(s).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `+[NSString localizedNameOfStringEncoding:]`
pub extern "C" fn shim_nsstring_localized_name_of_string_encoding(
    _cls: u64,
    _sel: u64,
    encoding: u64,
) -> u64 {
    let name = localized_name_for_encoding(encoding);
    let c_str = CString::new(name).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `+[NSString stringWithContentsOfFile:encoding:error:]`
pub extern "C" fn shim_nsstring_string_with_contents_of_file_encoding_error(
    _cls: u64,
    _sel: u64,
    path: u64,
    encoding: u64,
    error: u64,
) -> u64 {
    let Some(c_str) = read_cstring_from_file_path(path, encoding, 0, error) else {
        let path_s = extract_cfstring_cstr(path).unwrap_or("?");
        eprintln!("[stringWithContentsOfFile] FAILED path={:?}", path_s);
        return 0;
    };

    let result = shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    let path_s = extract_cfstring_cstr(path).unwrap_or("?");
    let len = c_str.to_bytes().len();
    eprintln!("[stringWithContentsOfFile] path={:?} len={} → {:#x}", path_s, len, result);
    result
}

/// `+[NSString stringWithContentsOfURL:usedEncoding:error:]`
pub extern "C" fn shim_nsstring_string_with_contents_of_url_used_encoding_error(
    _cls: u64,
    _sel: u64,
    url: u64,
    used_encoding: u64,
    error: u64,
) -> u64 {
    if url == 0 {
        clear_error_slot(error);
        return 0;
    }

    let path_ns = crate::nsurl::shim_nsurl_path(url, 0);
    let Some(c_str) = read_cstring_from_file_path(path_ns, 0, used_encoding, error) else {
        return 0;
    };

    shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSString initWithContentsOfFile:encoding:error:]`
pub extern "C" fn shim_nsstring_init_with_contents_of_file_encoding_error(
    obj: u64,
    _sel: u64,
    path: u64,
    encoding: u64,
    error: u64,
) -> u64 {
    if obj == 0 {
        clear_error_slot(error);
        return 0;
    }

    let Some(c_str) = read_cstring_from_file_path(path, encoding, 0, error) else {
        return 0;
    };

    shim_nsstring_init_with_utf8string(obj, 0, c_str.as_ptr())
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
    let remaining = objc_runtime::refcount::REFCOUNTS.release(obj);
    if remaining == 0 {
        objc_runtime::refcount::deallocate_tracked_object(obj, "CFRelease");
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
    compare_strings(a, b, 0)
}

/// `-[NSString compare:options:]`
pub extern "C" fn shim_nsstring_compare_options(
    obj: u64,
    _sel: u64,
    other: u64,
    options: u64,
) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    compare_strings(a, b, options)
}

/// `-[NSString copy]` — snapshot the string content into a new allocation.
///
/// Apple's real NSString returns self for immutable strings, and creates a
/// snapshot copy for mutable strings. We match that: check the ISA to decide.
pub extern "C" fn shim_nsstring_copy(obj: u64, _sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Mutable strings must be deep-copied to snapshot current content.
    let isa = unsafe { *(obj as *const u64) };
    if isa == crate::nsobj::nsmutablestring_class_addr() {
        if let Some(s) = extract_cfstring_cstr(obj) {
            let c = std::ffi::CString::new(s).unwrap_or_default();
            return shim_cf_string_create_with_cstring(0, c.as_ptr(), 0);
        }
    }
    // Immutable strings: return self (retain semantics).
    obj
}

/// `-[NSString dataUsingEncoding:]` — returns NSData containing the UTF-8 bytes.
/// Encoding argument is ignored; we always produce UTF-8 bytes.
pub extern "C" fn shim_nsstring_data_using_encoding(obj: u64, _sel: u64, _encoding: u64) -> u64 {
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return crate::nsdata::make_nsdata(&[]);
    };
    crate::nsdata::make_nsdata(bytes)
}

/// `-[NSString dataUsingEncoding:allowLossyConversion:]`
pub extern "C" fn shim_nsstring_data_using_encoding_lossy(
    obj: u64,
    _sel: u64,
    _encoding: u64,
    _lossy: u8,
) -> u64 {
    shim_nsstring_data_using_encoding(obj, _sel, _encoding)
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

/// `-[NSString writeToFile:atomically:encoding:error:]`
///
/// Writes the string to `path` using the given encoding.
/// Supported encodings: UTF-8 (4), UTF-16 (10, with BOM).
/// Returns 1 on success, 0 on failure.
pub extern "C" fn shim_nsstring_write_to_file(
    obj: u64,
    _sel: u64,
    path: u64,
    _atomically: u8,
    encoding: u64,
    error: u64,
) -> u8 {
    if obj == 0 || path == 0 {
        return 0;
    }
    let Some(path_str) = extract_cfstring_cstr(path) else {
        return 0;
    };
    log::info!("NSString writeToFile: path={}, encoding={}", path_str, encoding);
    let Some((bytes, _)) = cfstring_parts(obj) else {
        return 0;
    };
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let data: Vec<u8> = match encoding {
        // UTF-16 or UTF-16LE: add BOM then UTF-16LE bytes
        10 | 0x9400_0100 => {
            let mut buf = Vec::with_capacity(2 + text.len() * 2);
            buf.extend_from_slice(&[0xFF, 0xFE]); // UTF-16LE BOM
            for ch in text.encode_utf16() {
                buf.extend_from_slice(&ch.to_le_bytes());
            }
            buf
        }
        // UTF-16BE
        0x9000_0100 => {
            let mut buf = Vec::with_capacity(2 + text.len() * 2);
            buf.extend_from_slice(&[0xFE, 0xFF]); // UTF-16BE BOM
            for ch in text.encode_utf16() {
                buf.extend_from_slice(&ch.to_be_bytes());
            }
            buf
        }
        // UTF-8 (4) or anything else: write raw UTF-8
        _ => bytes.to_vec(),
    };

    match std::fs::write(path_str, &data) {
        Ok(()) => {
            if error != 0 {
                unsafe { *(error as *mut u64) = 0; }
            }
            1
        }
        Err(e) => {
            log::warn!("NSString writeToFile:{} failed: {}", path_str, e);
            0
        }
    }
}

/// `-[NSString localizedCaseInsensitiveCompare:]`
pub extern "C" fn shim_nsstring_localized_case_insensitive_compare(
    obj: u64,
    _sel: u64,
    other: u64,
) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    compare_strings(a, b, NS_CASE_INSENSITIVE_SEARCH)
}

/// `-[NSString localizedCompare:]`
pub extern "C" fn shim_nsstring_localized_compare(obj: u64, _sel: u64, other: u64) -> i64 {
    let a = extract_cfstring_cstr(obj).unwrap_or("");
    let b = extract_cfstring_cstr(other).unwrap_or("");
    compare_strings(a, b, 0)
}

/// `-[NSString precomposedStringWithCanonicalMapping]` — NFC no-op for ASCII.
pub extern "C" fn shim_nsstring_precomposed_string_with_canonical_mapping(
    obj: u64,
    _sel: u64,
) -> u64 {
    obj
}

/// `-[NSString decomposedStringWithCanonicalMapping]` — NFD no-op for ASCII.
pub extern "C" fn shim_nsstring_decomposed_string_with_canonical_mapping(
    obj: u64,
    _sel: u64,
) -> u64 {
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn length_basic() {
        let s = make_string("hello");
        assert_eq!(shim_nsstring_length(s, 0), 5);
        free_obj(s);
    }

    #[test]
    fn length_empty() {
        let s = make_string("");
        assert_eq!(shim_nsstring_length(s, 0), 0);
        free_obj(s);
    }

    #[test]
    fn utf8string_roundtrip() {
        let s = make_string("abc");
        let ptr = shim_nsstring_utf8string(s, 0);
        assert!(!ptr.is_null());
        let got = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap();
        assert_eq!(got, "abc");
        free_obj(s);
    }

    #[test]
    fn cstring_using_encoding_roundtrip() {
        let s = make_string("abc");
        let ptr = shim_nsstring_cstring_using_encoding(s, 0, K_CF_STRING_ENCODING_UTF8 as u64);
        assert!(!ptr.is_null());
        let got = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap();
        assert_eq!(got, "abc");
        free_obj(s);
    }

    #[test]
    fn is_equal_to_string_same() {
        let a = make_string("same");
        let b = make_string("same");
        assert_eq!(shim_nsstring_is_equal_to_string(a, 0, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_string_different() {
        let a = make_string("a");
        let b = make_string("b");
        assert_eq!(shim_nsstring_is_equal_to_string(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_string_nil() {
        let a = make_string("x");
        assert_eq!(shim_nsstring_is_equal_to_string(0, 0, a), 0);
        assert_eq!(shim_nsstring_is_equal_to_string(a, 0, 0), 0);
        free_obj(a);
    }

    #[test]
    fn has_prefix_and_suffix() {
        let s = make_string("hello world");
        let pre = make_string("hello");
        let suf = make_string("world");
        assert_eq!(shim_nsstring_has_prefix(s, 0, pre), 1);
        assert_eq!(shim_nsstring_has_suffix(s, 0, suf), 1);
        assert_eq!(shim_nsstring_has_prefix(s, 0, suf), 0);
        free_obj(s);
        free_obj(pre);
        free_obj(suf);
    }

    #[test]
    fn string_by_appending() {
        let a = make_string("foo");
        let b = make_string("bar");
        let result = shim_nsstring_string_by_appending(a, 0, b);
        assert_string_eq(result, "foobar");
        free_obj(a);
        free_obj(b);
        free_obj(result);
    }

    #[test]
    fn lowercase_and_uppercase() {
        let s = make_string("Hello");
        let lo = shim_nsstring_lowercase(s, 0);
        let hi = shim_nsstring_uppercase(s, 0);
        assert_string_eq(lo, "hello");
        assert_string_eq(hi, "HELLO");
        free_obj(s);
        free_obj(lo);
        free_obj(hi);
    }

    #[test]
    fn substring_from_index() {
        let s = make_string("abcde");
        let sub = shim_nsstring_substring_from_index(s, 0, 2);
        assert_string_eq(sub, "cde");
        free_obj(s);
        free_obj(sub);
    }

    #[test]
    fn substring_to_index() {
        let s = make_string("abcde");
        let sub = shim_nsstring_substring_to_index(s, 0, 3);
        assert_string_eq(sub, "abc");
        free_obj(s);
        free_obj(sub);
    }

    #[test]
    fn hash_consistent_for_equal_strings() {
        let a = make_string("key");
        let b = make_string("key");
        assert_eq!(shim_nsstring_hash(a, 0), shim_nsstring_hash(b, 0));
        assert_ne!(shim_nsstring_hash(a, 0), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn compare_ordering() {
        let a = make_string("abc");
        let b = make_string("abd");
        let c = make_string("abc");
        assert_eq!(shim_nsstring_compare(a, 0, b), -1);
        assert_eq!(shim_nsstring_compare(b, 0, a), 1);
        assert_eq!(shim_nsstring_compare(a, 0, c), 0);
        free_obj(a);
        free_obj(b);
        free_obj(c);
    }

    #[test]
    fn compare_options_case_insensitive() {
        let a = make_string("Xcode");
        let b = make_string("xcode");
        assert_eq!(shim_nsstring_compare_options(a, 0, b, NS_CASE_INSENSITIVE_SEARCH), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn compare_options_without_flags_matches_compare() {
        let a = make_string("abc");
        let b = make_string("abd");
        assert_eq!(shim_nsstring_compare_options(a, 0, b, 0), -1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn copy_returns_self_for_immutable() {
        let s = make_string("copy");
        let copy = shim_nsstring_copy(s, 0);
        assert_eq!(copy, s, "immutable NSString copy should return self");

        objc_runtime::shim_nsobject_release(s, 0);
    }

    #[test]
    fn string_with_cstring_encoding_basic() {
        let cls = crate::nsobj::nsstring_class_addr();
        let c_str = c"hello cstring";
        // encoding 4 = NSUTF8StringEncoding (ignored, but realistic)
        let obj = shim_nsstring_string_with_cstring_encoding(cls, 0, c_str.as_ptr(), 4);
        assert_ne!(obj, 0);
        assert_string_eq(obj, "hello cstring");
        free_obj(obj);
    }

    #[test]
    fn string_with_cstring_encoding_nil() {
        let cls = crate::nsobj::nsstring_class_addr();
        assert_eq!(
            shim_nsstring_string_with_cstring_encoding(cls, 0, std::ptr::null(), 4),
            0
        );
    }

    #[test]
    fn localized_name_of_string_encoding_known_value() {
        let name = shim_nsstring_localized_name_of_string_encoding(
            crate::nsobj::nsstring_class_addr(),
            0,
            4,
        );
        assert_string_eq(name, "Unicode (UTF-8)");
        free_obj(name);
    }

    #[test]
    fn localized_name_of_string_encoding_unknown_value_falls_back() {
        let name = shim_nsstring_localized_name_of_string_encoding(
            crate::nsobj::nsstring_class_addr(),
            0,
            999,
        );
        assert_string_eq(name, "Encoding 999");
        free_obj(name);
    }

    #[test]
    fn string_with_contents_of_file_encoding_reads_file() {
        let path = std::env::temp_dir().join(format!(
            "cf-lite-string-file-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, "hello from file").unwrap();

        let path_ns = make_string(path.to_string_lossy().as_ref());
        let mut error = 321u64;
        let result = shim_nsstring_string_with_contents_of_file_encoding_error(
            crate::nsobj::nsstring_class_addr(),
            0,
            path_ns,
            0,
            &mut error as *mut u64 as u64,
        );

        assert_ne!(result, 0);
        assert_string_eq(result, "hello from file");
        assert_eq!(error, 0);

        let _ = std::fs::remove_file(path);
        free_obj(result);
        free_obj(path_ns);
    }

    #[test]
    fn init_with_contents_of_file_encoding_reads_file() {
        let path = std::env::temp_dir().join(format!(
            "cf-lite-init-file-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, "initialized from file").unwrap();

        let obj = crate::shim_test_helpers::alloc_obj(32, crate::nsobj::nsstring_class_addr());
        let path_ns = make_string(path.to_string_lossy().as_ref());
        let mut error = 654u64;
        let result = shim_nsstring_init_with_contents_of_file_encoding_error(
            obj,
            0,
            path_ns,
            0,
            &mut error as *mut u64 as u64,
        );

        assert_eq!(result, obj);
        assert_string_eq(result, "initialized from file");
        assert_eq!(error, 0);

        let _ = std::fs::remove_file(path);
        free_obj(path_ns);
        free_obj(obj);
    }

    #[test]
    fn string_with_contents_of_url_reads_file() {
        let path = std::env::temp_dir().join(format!(
            "cf-lite-string-contents-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, "hello from disk").unwrap();

        let path_ns = make_string(path.to_string_lossy().as_ref());
        let url = crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path_ns);
        let mut encoding = 0u64;
        let mut error = 123u64;
        let result = shim_nsstring_string_with_contents_of_url_used_encoding_error(
            crate::nsobj::nsstring_class_addr(),
            0,
            url,
            &mut encoding as *mut u64 as u64,
            &mut error as *mut u64 as u64,
        );

        assert_ne!(result, 0);
        assert_string_eq(result, "hello from disk");
        assert_eq!(encoding, NS_UTF8_STRING_ENCODING);
        assert_eq!(error, 0);

        let _ = std::fs::remove_file(path);
        free_obj(result);
        free_obj(url);
        free_obj(path_ns);
    }

    #[test]
    fn localized_case_insensitive_compare_equal() {
        let a = make_string("Hello");
        let b = make_string("hello");
        assert_eq!(shim_nsstring_localized_case_insensitive_compare(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn localized_case_insensitive_compare_ordering() {
        let a = make_string("apple");
        let b = make_string("Banana");
        assert_eq!(shim_nsstring_localized_case_insensitive_compare(a, 0, b), -1);
        assert_eq!(shim_nsstring_localized_case_insensitive_compare(b, 0, a), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn localized_case_insensitive_compare_nil() {
        let a = make_string("x");
        // nil other → compare "" with "x", should be -1 (empty < "x")
        let result = shim_nsstring_localized_case_insensitive_compare(0, 0, a);
        assert_eq!(result, -1);
        free_obj(a);
    }

    #[test]
    fn localized_compare_basic() {
        let a = make_string("abc");
        let b = make_string("abd");
        assert_eq!(shim_nsstring_localized_compare(a, 0, b), -1);
        assert_eq!(shim_nsstring_localized_compare(b, 0, a), 1);
        assert_eq!(shim_nsstring_localized_compare(a, 0, a), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn precomposed_returns_self() {
        let s = make_string("hello");
        assert_eq!(shim_nsstring_precomposed_string_with_canonical_mapping(s, 0), s);
        free_obj(s);
    }

    #[test]
    fn precomposed_nil_returns_nil() {
        assert_eq!(shim_nsstring_precomposed_string_with_canonical_mapping(0, 0), 0);
    }

    #[test]
    fn decomposed_returns_self() {
        let s = make_string("hello");
        assert_eq!(shim_nsstring_decomposed_string_with_canonical_mapping(s, 0), s);
        free_obj(s);
    }

    #[test]
    fn decomposed_nil_returns_nil() {
        assert_eq!(shim_nsstring_decomposed_string_with_canonical_mapping(0, 0), 0);
    }
}
