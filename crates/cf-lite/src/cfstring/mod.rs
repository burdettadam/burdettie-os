//! CFString / NSString implementation for burdettie-os.
//!
//! Layout matches Apple's CFConstantString (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: flags        (u32 — 0x07C8 = UTF-8 immutable)
//!   offset 12: padding      (u32)
//!   offset 16: c_string_ptr (u64)
//!   offset 24: length       (u64)

mod nsstring;

pub use nsstring::*;

use std::ffi::{c_char, CStr, CString};

use crate::foundation::cfconstant_string_class_addr;

/// Tag for CFStringEncoding — we only support UTF-8 (0x08000100).
pub(crate) const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

/// CFString flags: UTF-8, immutable.
pub(crate) const CF_STRING_FLAGS_UTF8: u32 = 0x07C8;

/// Allocate a new CFString from a C string.
///
/// `CFStringRef CFStringCreateWithCString(CFAllocatorRef alloc, const char *cStr,
///     CFStringEncoding encoding)`
pub extern "C" fn shim_cf_string_create_with_cstring(
    _allocator: u64,
    c_str: *const c_char,
    encoding: u32,
) -> u64 {
    if c_str.is_null() {
        return 0;
    }

    if encoding != K_CF_STRING_ENCODING_UTF8 && encoding != 0 {
        log::warn!(
            "CFStringCreateWithCString: unsupported encoding {:#x}, treating as UTF-8",
            encoding
        );
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
        *(p as *mut u64) = cfconstant_string_class_addr();
        *((p.add(8)) as *mut u32) = CF_STRING_FLAGS_UTF8;
        *((p.add(16)) as *mut u64) = owned_ptr as u64;
        *((p.add(24)) as *mut u64) = len as u64;
    }

    log::debug!(
        "CFStringCreateWithCString: created CFString at {:#x}, len={}",
        obj,
        len
    );
    obj
}

/// `CFIndex CFStringGetLength(CFStringRef theString)`
pub extern "C" fn shim_cf_string_get_length(string: u64) -> i64 {
    if string == 0 {
        return 0;
    }
    unsafe {
        let p = string as *const u8;
        *((p.add(24)) as *const u64) as i64
    }
}

/// `const char *CFStringGetCStringPtr(CFStringRef theString, CFStringEncoding encoding)`
pub extern "C" fn shim_cf_string_get_cstring_ptr(string: u64, _encoding: u32) -> *const c_char {
    if string == 0 {
        return std::ptr::null();
    }
    unsafe {
        let p = string as *const u8;
        let c_str_ptr = *((p.add(16)) as *const u64);
        c_str_ptr as *const c_char
    }
}

/// Constant for kCFStringEncodingUTF8, exported as a data symbol.
static K_CF_STRING_ENCODING_UTF8_VALUE: u32 = K_CF_STRING_ENCODING_UTF8;

/// Returns the address of the kCFStringEncodingUTF8 constant.
pub fn kcf_string_encoding_utf8_addr() -> u64 {
    &K_CF_STRING_ENCODING_UTF8_VALUE as *const u32 as u64
}

static K_CF_ALLOCATOR_DEFAULT: u64 = 0;

/// Returns the address of the kCFAllocatorDefault constant.
pub fn kcf_allocator_default_addr() -> u64 {
    &K_CF_ALLOCATOR_DEFAULT as *const u64 as u64
}

/// Extract the C string from a CFString/NSString object.
///
/// # Safety invariant
///
/// The returned `&'static str` is valid because CFString objects allocated
/// by this shim runtime are intentionally leaked (never freed). The backing
/// `CString` is created via `CString::into_raw` and never reclaimed.
pub fn extract_cfstring_cstr(obj: u64) -> Option<&'static str> {
    let ptr = obj as *const u8;
    if ptr.is_null() {
        return None;
    }
    // Reject obviously-invalid pointers before doing any unsafe reads:
    //   - Values below the first page (0x1000) are always unmapped.
    //   - Bit 63 set means tagged pointer (handled elsewhere).
    if obj < 0x1000 || (obj >> 63) != 0 {
        return None;
    }
    // Use read_unaligned: __CFConstantString objects from loaded binaries
    // may reside at addresses that are not 8-byte aligned.
    let c_str_ptr = unsafe { std::ptr::read_unaligned((ptr.add(16)) as *const u64) };
    if c_str_ptr == 0 {
        return None;
    }
    // Reject garbage c_str_ptr values by the same bounds check.
    if c_str_ptr < 0x1000 || (c_str_ptr >> 63) != 0 {
        return None;
    }
    // SAFETY: c_str_ptr was produced by CString::into_raw, which guarantees
    // a valid NUL-terminated UTF-8 string. The memory is never freed.
    let c_str = unsafe { CStr::from_ptr(c_str_ptr as *const c_char) };
    c_str.to_str().ok()
}

/// Read the raw C string pointer and length from a CFString/NSString.
///
/// # Safety invariant
///
/// Same as [`extract_cfstring_cstr`]: CFString objects are intentionally
/// leaked (never freed), so the returned slice is valid for `'static`.
pub(crate) fn cfstring_parts(obj: u64) -> Option<(&'static [u8], usize)> {
    if obj == 0 {
        return None;
    }
    unsafe {
        let ptr = std::ptr::read_unaligned((obj as usize + 16) as *const u64);
        let len = std::ptr::read_unaligned((obj as usize + 24) as *const u64) as usize;
        if ptr == 0 {
            return None;
        }
        Some((std::slice::from_raw_parts(ptr as *const u8, len), len))
    }
}

/// Convenience: allocate an NSString from a NUL-terminated C string.
pub fn alloc_nsstring_from_cstr(c_str: *const c_char) -> u64 {
    shim_cf_string_create_with_cstring(0, c_str, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_read_cfstring() {
        let c_str = b"hello world\0";
        let obj = shim_cf_string_create_with_cstring(
            0,
            c_str.as_ptr() as *const c_char,
            K_CF_STRING_ENCODING_UTF8,
        );
        assert_ne!(obj, 0);
        assert_eq!(shim_cf_string_get_length(obj), 11);

        let ptr = shim_cf_string_get_cstring_ptr(obj, K_CF_STRING_ENCODING_UTF8);
        assert!(!ptr.is_null());
        let result = unsafe { CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "hello world");

        crate::pal_alloc().free(obj as *mut u8);
    }

    #[test]
    fn null_cfstring_returns_zero() {
        assert_eq!(shim_cf_string_get_length(0), 0);
        assert!(shim_cf_string_get_cstring_ptr(0, 0).is_null());
    }

    #[test]
    fn create_empty_string() {
        let c_str = b"\0";
        let obj = shim_cf_string_create_with_cstring(
            0,
            c_str.as_ptr() as *const c_char,
            K_CF_STRING_ENCODING_UTF8,
        );
        assert_ne!(obj, 0);
        assert_eq!(shim_cf_string_get_length(obj), 0);
        crate::pal_alloc().free(obj as *mut u8);
    }

    #[test]
    fn nsstring_methods() {
        let c_str = b"test\0";
        let obj = shim_cf_string_create_with_cstring(
            0,
            c_str.as_ptr() as *const c_char,
            K_CF_STRING_ENCODING_UTF8,
        );

        assert_eq!(shim_nsstring_length(obj, 0), 4);
        assert_eq!(shim_nsstring_description(obj, 0), obj);

        let ptr = shim_nsstring_utf8string(obj, 0);
        let s = unsafe { CStr::from_ptr(ptr) };
        assert_eq!(s.to_str().unwrap(), "test");

        crate::pal_alloc().free(obj as *mut u8);
    }

    fn make_str(s: &str) -> u64 {
        let c = CString::new(s).unwrap();
        shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
    }

    #[test]
    fn has_prefix_suffix() {
        let s = make_str("Hello, World!");
        let hello = make_str("Hello");
        let world = make_str("World!");
        let nope = make_str("xyz");

        assert_eq!(shim_nsstring_has_prefix(s, 0, hello), 1);
        assert_eq!(shim_nsstring_has_prefix(s, 0, world), 0);
        assert_eq!(shim_nsstring_has_suffix(s, 0, world), 1);
        assert_eq!(shim_nsstring_has_suffix(s, 0, hello), 0);
        assert_eq!(shim_nsstring_has_prefix(s, 0, nope), 0);
        assert_eq!(shim_nsstring_has_suffix(s, 0, nope), 0);
    }

    #[test]
    fn string_by_appending() {
        let a = make_str("Hello");
        let b = make_str(", World!");
        let result = shim_nsstring_string_by_appending(a, 0, b);
        assert_eq!(extract_cfstring_cstr(result).unwrap(), "Hello, World!");
    }

    #[test]
    fn substring_from_to() {
        let s = make_str("Hello, World!");
        let from7 = shim_nsstring_substring_from_index(s, 0, 7);
        assert_eq!(extract_cfstring_cstr(from7).unwrap(), "World!");
        let to5 = shim_nsstring_substring_to_index(s, 0, 5);
        assert_eq!(extract_cfstring_cstr(to5).unwrap(), "Hello");
    }

    #[test]
    fn lowercase_uppercase() {
        let s = make_str("Hello World");
        let lo = shim_nsstring_lowercase(s, 0);
        assert_eq!(extract_cfstring_cstr(lo).unwrap(), "hello world");
        let hi = shim_nsstring_uppercase(s, 0);
        assert_eq!(extract_cfstring_cstr(hi).unwrap(), "HELLO WORLD");
    }

    #[test]
    fn compare_ordering() {
        let a = make_str("apple");
        let b = make_str("banana");
        let a2 = make_str("apple");
        assert_eq!(shim_nsstring_compare(a, 0, b), -1);
        assert_eq!(shim_nsstring_compare(b, 0, a), 1);
        assert_eq!(shim_nsstring_compare(a, 0, a2), 0);
    }

    #[test]
    fn copy_returns_self_for_immutable() {
        let s = make_str("test");
        let c = shim_nsstring_copy(s, 0);
        assert_eq!(c, s, "immutable NSString copy should return self");
    }

    #[test]
    fn hash_consistency() {
        let a = make_str("hello");
        let b = make_str("hello");
        let c = make_str("world");
        assert_eq!(shim_nsstring_hash(a, 0), shim_nsstring_hash(b, 0));
        assert_ne!(shim_nsstring_hash(a, 0), shim_nsstring_hash(c, 0));
        assert_ne!(shim_nsstring_hash(a, 0), 0);
    }

    #[test]
    fn string_with_format_literal() {
        let fmt = make_str("hello world");
        let result = shim_nsstring_string_with_format_inner(fmt, 0, 0, 0);
        assert_ne!(result, 0);
        assert_eq!(extract_cfstring_cstr(result).unwrap(), "hello world");
    }

    #[test]
    fn string_with_format_int() {
        let fmt = make_str("value=%d");
        let result = shim_nsstring_string_with_format_inner(fmt, 42, 0, 0);
        assert_ne!(result, 0);
        assert_eq!(extract_cfstring_cstr(result).unwrap(), "value=42");
    }

    #[test]
    fn string_with_format_obj() {
        let name = make_str("world");
        let fmt = make_str("Hello, %@!");
        let result = shim_nsstring_string_with_format_inner(fmt, name, 0, 0);
        assert_ne!(result, 0);
        assert_eq!(extract_cfstring_cstr(result).unwrap(), "Hello, world!");
    }

    #[test]
    fn string_with_format_mixed() {
        let obj = make_str("item");
        let fmt = make_str("%@=%d");
        let result = shim_nsstring_string_with_format_inner(fmt, obj, 7, 0);
        assert_ne!(result, 0);
        assert_eq!(extract_cfstring_cstr(result).unwrap(), "item=7");
    }

    #[test]
    fn string_with_format_null_format() {
        let result = shim_nsstring_string_with_format_inner(0, 0, 0, 0);
        assert_eq!(result, 0);
    }
}
