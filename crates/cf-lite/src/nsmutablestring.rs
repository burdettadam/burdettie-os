//! NSMutableString shim: mutable string backed by heap-allocated CString.
//!
//! Uses the same 32-byte layout as NSString/CFString:
//!
//!   offset  0: isa          (u64 — NSMutableString class)
//!   offset  8: flags        (u32 — 0x07C8 = UTF-8)
//!   offset 12: padding      (u32)
//!   offset 16: c_string_ptr (u64 — heap-allocated via CString::into_raw)
//!   offset 24: length       (u64)
//!
//! Key invariant: c_string_ptr is ALWAYS from `CString::into_raw`,
//! making it safe to `CString::from_raw` + drop during mutation.

use std::ffi::{c_char, CString};

use crate::cfstring;
use crate::nsobj;

/// Read the current bytes from a CFString/NSString/NSMutableString object.
fn read_string_bytes(obj: u64) -> Vec<u8> {
    if obj == 0 {
        return Vec::new();
    }
    unsafe {
        let ptr = *((obj as usize + 16) as *const u64) as *const u8;
        let len = *((obj as usize + 24) as *const u64) as usize;
        if ptr.is_null() || len == 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(ptr, len).to_vec()
    }
}

/// Replace the backing store of a mutable string object.
///
/// # Safety
///
/// `obj` must be an NSMutableString whose `c_string_ptr` (offset 16)
/// was allocated via `CString::into_raw`.
unsafe fn replace_backing_store(obj: u64, new_bytes: &[u8]) {
    let old_ptr = *((obj as usize + 16) as *const u64) as *mut c_char;
    if !old_ptr.is_null() {
        drop(CString::from_raw(old_ptr));
    }
    let new_cs = CString::new(new_bytes).unwrap_or_default();
    let new_ptr = new_cs.into_raw();
    *((obj as usize + 16) as *mut u64) = new_ptr as u64;
    *((obj as usize + 24) as *mut u64) = new_bytes.len() as u64;
}

/// `+[NSMutableString string]` — creates an empty mutable string.
pub extern "C" fn shim_nsmutablestring_string(cls: u64, _cmd: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }

    let empty = CString::default();
    let ptr = empty.into_raw();

    unsafe {
        *(obj as *mut u64) = nsobj::nsmutablestring_class_addr();
        *((obj as usize + 8) as *mut u32) = cfstring::CF_STRING_FLAGS_UTF8;
        *((obj as usize + 16) as *mut u64) = ptr as u64;
        *((obj as usize + 24) as *mut u64) = 0;
    }

    log::debug!("NSMutableString string → {:#x}", obj);
    obj
}

/// `-[NSMutableString initWithString:]` — init from another string.
pub extern "C" fn shim_nsmutablestring_init_with_string(obj: u64, _cmd: u64, source: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let bytes = read_string_bytes(source);
    let cs = CString::new(bytes.clone()).unwrap_or_default();
    let ptr = cs.into_raw();

    unsafe {
        *((obj as usize + 8) as *mut u32) = cfstring::CF_STRING_FLAGS_UTF8;
        *((obj as usize + 16) as *mut u64) = ptr as u64;
        *((obj as usize + 24) as *mut u64) = bytes.len() as u64;
    }

    log::debug!(
        "NSMutableString initWithString: → {:#x}, len={}",
        obj,
        bytes.len()
    );
    obj
}

/// `-[NSMutableString appendString:]`
pub extern "C" fn shim_nsmutablestring_append_string(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let mut current = read_string_bytes(obj);
    let append = read_string_bytes(other);
    current.extend_from_slice(&append);

    unsafe {
        replace_backing_store(obj, &current);
    }

    log::debug!(
        "NSMutableString appendString: at {:#x}, new len={}",
        obj,
        current.len()
    );
    0 // appendString: returns void
}

/// `-[NSMutableString insertString:atIndex:]`
///
/// On ARM64: x2 = string, x3 = index.
pub extern "C" fn shim_nsmutablestring_insert_string_at_index(
    obj: u64,
    _cmd: u64,
    string: u64,
    index: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let current = read_string_bytes(obj);
    let insert = read_string_bytes(string);
    let idx = (index as usize).min(current.len());

    let mut result = Vec::with_capacity(current.len() + insert.len());
    result.extend_from_slice(&current[..idx]);
    result.extend_from_slice(&insert);
    result.extend_from_slice(&current[idx..]);

    unsafe {
        replace_backing_store(obj, &result);
    }

    log::debug!(
        "NSMutableString insertString:atIndex: at {:#x}, idx={}, new len={}",
        obj,
        idx,
        result.len()
    );
    0 // returns void
}

/// `-[NSMutableString deleteCharactersInRange:]`
///
/// On ARM64, `NSRange { location, length }` is passed in x2 and x3.
pub extern "C" fn shim_nsmutablestring_delete_characters_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    range_length: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let mut current = read_string_bytes(obj);
    let loc = (location as usize).min(current.len());
    let len = (range_length as usize).min(current.len().saturating_sub(loc));

    current.drain(loc..loc + len);

    unsafe {
        replace_backing_store(obj, &current);
    }

    log::debug!(
        "NSMutableString deleteCharactersInRange:({},{}) at {:#x}, new len={}",
        loc,
        len,
        obj,
        current.len()
    );
    0 // returns void
}

/// `-[NSMutableString replaceCharactersInRange:withString:]`
///
/// On ARM64: x2 = location, x3 = range_length, x4 = replacement string.
pub extern "C" fn shim_nsmutablestring_replace_characters_in_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    range_length: u64,
    replacement: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let current = read_string_bytes(obj);
    let repl = read_string_bytes(replacement);
    let loc = (location as usize).min(current.len());
    let len = (range_length as usize).min(current.len().saturating_sub(loc));

    let mut result = Vec::with_capacity(current.len() - len + repl.len());
    result.extend_from_slice(&current[..loc]);
    result.extend_from_slice(&repl);
    result.extend_from_slice(&current[loc + len..]);

    unsafe {
        replace_backing_store(obj, &result);
    }

    log::debug!(
        "NSMutableString replaceCharactersInRange:({},{}) at {:#x}, new len={}",
        loc,
        len,
        obj,
        result.len()
    );
    0 // returns void
}

/// `-[NSMutableString setString:]`
pub extern "C" fn shim_nsmutablestring_set_string(obj: u64, _cmd: u64, string: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let new_bytes = read_string_bytes(string);
    unsafe {
        replace_backing_store(obj, &new_bytes);
    }

    log::debug!(
        "NSMutableString setString: at {:#x}, new len={}",
        obj,
        new_bytes.len()
    );
    0 // returns void
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutable_string_create_empty() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);
        assert_ne!(obj, 0);

        let len = unsafe { *((obj as usize + 24) as *const u64) };
        assert_eq!(len, 0);
    }

    #[test]
    fn mutable_string_append() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        // Create a source string to init from
        let hello = cfstring::shim_cf_string_create_with_cstring(0, c"Hello".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, hello);

        let world = cfstring::shim_cf_string_create_with_cstring(0, c" World".as_ptr(), 0);
        shim_nsmutablestring_append_string(obj, 0, world);

        let len = unsafe { *((obj as usize + 24) as *const u64) };
        assert_eq!(len, 11);

        let ptr = unsafe { *((obj as usize + 16) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello World");
    }

    #[test]
    fn mutable_string_insert() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let base = cfstring::shim_cf_string_create_with_cstring(0, c"HelloWorld".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, base);

        let space = cfstring::shim_cf_string_create_with_cstring(0, c" ".as_ptr(), 0);
        shim_nsmutablestring_insert_string_at_index(obj, 0, space, 5);

        let ptr = unsafe { *((obj as usize + 16) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello World");
    }

    #[test]
    fn mutable_string_delete_range() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let base = cfstring::shim_cf_string_create_with_cstring(0, c"Hello World".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, base);

        // Delete " World" (range: location=5, length=6)
        shim_nsmutablestring_delete_characters_in_range(obj, 0, 5, 6);

        let ptr = unsafe { *((obj as usize + 16) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello");
    }

    #[test]
    fn mutable_string_replace_range() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let base = cfstring::shim_cf_string_create_with_cstring(0, c"Hello World".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, base);

        let repl = cfstring::shim_cf_string_create_with_cstring(0, c"Rust".as_ptr(), 0);
        // Replace "World" (range: location=6, length=5) with "Rust"
        shim_nsmutablestring_replace_characters_in_range(obj, 0, 6, 5, repl);

        let ptr = unsafe { *((obj as usize + 16) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello Rust");
    }

    #[test]
    fn mutable_string_set_string() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let v1 = cfstring::shim_cf_string_create_with_cstring(0, c"first".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, v1);

        let v2 = cfstring::shim_cf_string_create_with_cstring(0, c"second".as_ptr(), 0);
        shim_nsmutablestring_set_string(obj, 0, v2);

        let len = unsafe { *((obj as usize + 24) as *const u64) };
        assert_eq!(len, 6);

        let ptr = unsafe { *((obj as usize + 16) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "second");
    }
}
