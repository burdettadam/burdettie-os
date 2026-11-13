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

const NSMUTABLESTRING_SIZE: usize = 32;

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_nsmutablestring_format_asm();
}

/// Returns the address of the +[NSMutableString stringWithFormat:] trampoline.
pub fn string_with_format_trampoline_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_nsmutablestring_format_asm as *const () as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        shim_nsmutablestring_string_with_format_inner as *const () as u64
    }
}
const FLAGS_OFF: usize = 8;
const C_STRING_PTR_OFF: usize = 16;
const LENGTH_OFF: usize = 24;

/// Read the current bytes from a CFString/NSString/NSMutableString object.
fn read_string_bytes(obj: u64) -> Vec<u8> {
    if obj == 0 {
        return Vec::new();
    }
    unsafe {
        let ptr = *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const u8;
        let len = *((obj as usize + LENGTH_OFF) as *const u64) as usize;
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
    let old_ptr = *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *mut c_char;
    if !old_ptr.is_null() {
        drop(CString::from_raw(old_ptr));
    }
    let new_cs = CString::new(new_bytes).unwrap_or_default();
    let new_ptr = new_cs.into_raw();
    *((obj as usize + C_STRING_PTR_OFF) as *mut u64) = new_ptr as u64;
    *((obj as usize + LENGTH_OFF) as *mut u64) = new_bytes.len() as u64;
}

fn init_mutable_string_with_bytes(obj: u64, initial_bytes: &[u8]) -> u64 {
    if obj == 0 {
        return 0;
    }

    unsafe {
        if *(obj as *const u64) == 0 {
            *(obj as *mut u64) = nsobj::nsmutablestring_class_addr();
        }
        *((obj as usize + FLAGS_OFF) as *mut u32) = cfstring::CF_STRING_FLAGS_UTF8;
        replace_backing_store(obj, initial_bytes);
    }

    obj
}

/// `+[NSMutableString string]` — creates an empty mutable string.
pub extern "C" fn shim_nsmutablestring_string(cls: u64, _cmd: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    let obj = crate::pal_alloc().calloc(1, NSMUTABLESTRING_SIZE) as u64;
    if obj == 0 {
        return 0;
    }

    init_mutable_string_with_bytes(obj, b"");

    log::debug!("NSMutableString string → {:#x}", obj);
    obj
}

/// Inner implementation for `+[NSMutableString stringWithFormat:]`.
///
/// Returns an NSMutableString (not NSString) with the formatted content.
#[no_mangle]
pub extern "C" fn shim_nsmutablestring_string_with_format_inner(
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    if format == 0 {
        return 0;
    }
    let fmt_str = cfstring::extract_cfstring_cstr(format).unwrap_or("");
    let expanded = crate::nslog::expand_format(fmt_str, &[a1, a2, a3]);

    let obj = crate::pal_alloc().calloc(1, NSMUTABLESTRING_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    init_mutable_string_with_bytes(obj, expanded.as_bytes());
    log::debug!(
        "NSMutableString stringWithFormat: → {:#x}, len={}",
        obj,
        expanded.len()
    );
    obj
}

/// `+[NSMutableString stringWithCapacity:]` — creates an empty mutable
/// string. Capacity is currently advisory only.
pub extern "C" fn shim_nsmutablestring_string_with_capacity(
    cls: u64,
    _cmd: u64,
    capacity: u64,
) -> u64 {
    let obj = shim_nsmutablestring_string(cls, _cmd);
    if obj != 0 {
        log::debug!(
            "NSMutableString stringWithCapacity:{} → {:#x}",
            capacity,
            obj
        );
    }
    obj
}

/// `-[NSMutableString initWithCapacity:]` — initializes an allocated mutable
/// string as empty. Capacity is currently advisory only.
pub extern "C" fn shim_nsmutablestring_init_with_capacity(
    obj: u64,
    _cmd: u64,
    capacity: u64,
) -> u64 {
    let obj = init_mutable_string_with_bytes(obj, b"");
    if obj != 0 {
        log::debug!("NSMutableString initWithCapacity:{} → {:#x}", capacity, obj);
    }
    obj
}

/// `-[NSMutableString initWithString:]` — init from another string.
pub extern "C" fn shim_nsmutablestring_init_with_string(obj: u64, _cmd: u64, source: u64) -> u64 {
    let bytes = read_string_bytes(source);
    let obj = init_mutable_string_with_bytes(obj, &bytes);

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

    0 // appendString: returns void
}

/// `-[NSMutableString initWithFormat:]` — variadic initializer.
///
/// On ARM64: x0=self, x1=_cmd, x2=format, variadics on stack.
/// Creates a mutable string with formatted content.
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
pub extern "C" fn shim_nsmutablestring_init_with_format(
    _obj: u64,
    _cmd: u64,
    _format: u64,
    _a1: u64,
    _a2: u64,
    _a3: u64,
) -> u64 {
    std::arch::naked_asm!(
        "ldp x3, x4, [sp]",
        "ldr x5, [sp, #16]",
        "b {inner}",
        inner = sym shim_nsmutablestring_init_with_format_inner,
    )
}

#[cfg(not(target_arch = "aarch64"))]
pub extern "C" fn shim_nsmutablestring_init_with_format(
    obj: u64,
    _cmd: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    shim_nsmutablestring_init_with_format_inner(obj, _cmd, format, a1, a2, a3)
}

extern "C" fn shim_nsmutablestring_init_with_format_inner(
    obj: u64,
    _cmd: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    if format == 0 {
        return obj;
    }
    let fmt_str = crate::cfstring::extract_cfstring_cstr(format).unwrap_or("");
    let expanded = crate::nslog::expand_format(fmt_str, &[a1, a2, a3]);
    // Use the pre-allocated obj (from alloc) to initialize in-place.
    init_mutable_string_with_bytes(obj, expanded.as_bytes())
}

/// `-[NSMutableString appendFormat:]`
///
/// Formats the arguments using the format string and appends the result.
///
/// On ARM64 (Apple), `appendFormat:` is a variadic ObjC method.
/// Fixed args: x0=self, x1=_cmd, x2=format
/// Variadic args are passed on the **stack** per AAPCS64 variadic convention.
///
/// We use a naked assembly wrapper to read the stack-passed variadic args
/// and forward them in registers to the inner Rust function.
#[cfg(target_arch = "aarch64")]
#[unsafe(naked)]
pub extern "C" fn shim_nsmutablestring_append_format(
    _obj: u64,
    _cmd: u64,
    _format: u64,
    _a1: u64,
    _a2: u64,
    _a3: u64,
) -> u64 {
    // x0=self, x1=_cmd, x2=format are already in registers.
    // Read variadic args from the stack (caller placed them at [sp]).
    std::arch::naked_asm!(
        "ldp x3, x4, [sp]",    // x3 = va_arg[0], x4 = va_arg[1]
        "ldr x5, [sp, #16]",   // x5 = va_arg[2]
        "b {inner}",
        inner = sym shim_nsmutablestring_append_format_inner,
    )
}

#[cfg(not(target_arch = "aarch64"))]
pub extern "C" fn shim_nsmutablestring_append_format(
    obj: u64,
    _cmd: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    shim_nsmutablestring_append_format_inner(obj, _cmd, format, a1, a2, a3)
}

extern "C" fn shim_nsmutablestring_append_format_inner(
    obj: u64,
    _cmd: u64,
    format: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    if obj == 0 || format == 0 {
        return 0;
    }
    let fmt_str = crate::cfstring::extract_cfstring_cstr(format).unwrap_or("");
    let expanded = crate::nslog::expand_format(fmt_str, &[a1, a2, a3]);

    let mut current = read_string_bytes(obj);
    current.extend_from_slice(expanded.as_bytes());
    unsafe {
        replace_backing_store(obj, &current);
    }
    0
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

/// `-[NSMutableString clear]` — removes all characters from the string.
///
/// Equivalent to `[str setString:@""]`. Used internally by genstrings and
/// other tools to reset a scratch buffer before reuse.
pub extern "C" fn shim_nsmutablestring_clear(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        replace_backing_store(obj, &[]);
    }
    log::debug!("NSMutableString clear at {:#x}", obj);
}

/// `-[NSMutableString replaceOccurrencesOfString:withString:options:range:]`
///
/// On ARM64: x2 = target, x3 = replacement, x4 = options, x5 = range.location, x6 = range.length.
/// Returns the number of replacements made as u64.
pub extern "C" fn shim_nsmutablestring_replace_occurrences(
    obj: u64,
    _cmd: u64,
    target: u64,
    replacement: u64,
    _options: u64,
    _range_loc: u64,
    _range_len: u64,
) -> u64 {
    if obj == 0 || target == 0 {
        return 0;
    }

    let current = read_string_bytes(obj);
    let target_bytes = read_string_bytes(target);
    let repl_bytes = read_string_bytes(replacement);

    if target_bytes.is_empty() {
        return 0;
    }

    // Convert to strings for replace; fall back to empty if not valid UTF-8.
    let current_str = String::from_utf8_lossy(&current);
    let target_str = String::from_utf8_lossy(&target_bytes);
    let repl_str = String::from_utf8_lossy(&repl_bytes);

    let count = current_str.matches(target_str.as_ref()).count() as u64;
    if count == 0 {
        return 0;
    }

    let result = current_str.replace(target_str.as_ref(), repl_str.as_ref());
    unsafe {
        replace_backing_store(obj, result.as_bytes());
    }

    log::debug!(
        "NSMutableString replaceOccurrences at {:#x}: {} replacements, new len={}",
        obj,
        count,
        result.len()
    );
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutable_string_create_empty() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);
        assert_ne!(obj, 0);

        let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
        assert_eq!(len, 0);
    }

    #[test]
    fn mutable_string_init_with_capacity_resets_to_empty() {
        let obj = crate::pal_alloc().calloc(1, NSMUTABLESTRING_SIZE) as u64;
        assert_ne!(obj, 0);

        let seed = cfstring::shim_cf_string_create_with_cstring(0, c"seed".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, seed);

        let result = shim_nsmutablestring_init_with_capacity(obj, 0, 128);
        assert_eq!(result, obj);

        let isa = unsafe { *(obj as *const u64) };
        assert_eq!(isa, nsobj::nsmutablestring_class_addr());

        let flags = unsafe { *((obj as usize + FLAGS_OFF) as *const u32) };
        assert_eq!(flags, cfstring::CF_STRING_FLAGS_UTF8);

        let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
        assert_eq!(len, 0);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "");
    }

    #[test]
    fn mutable_string_string_with_capacity_creates_empty_string() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string_with_capacity(cls, 0, 64);
        assert_ne!(obj, 0);

        let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
        assert_eq!(len, 0);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "");
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

        let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
        assert_eq!(len, 11);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
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

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
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

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
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

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
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

        let len = unsafe { *((obj as usize + LENGTH_OFF) as *const u64) };
        assert_eq!(len, 6);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "second");
    }

    #[test]
    fn mutable_string_replace_occurrences() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let base =
            cfstring::shim_cf_string_create_with_cstring(0, c"foo bar foo baz foo".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, base);

        let target = cfstring::shim_cf_string_create_with_cstring(0, c"foo".as_ptr(), 0);
        let repl = cfstring::shim_cf_string_create_with_cstring(0, c"qux".as_ptr(), 0);

        let count = shim_nsmutablestring_replace_occurrences(obj, 0, target, repl, 0, 0, 19);
        assert_eq!(count, 3);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "qux bar qux baz qux");
    }

    #[test]
    fn mutable_string_replace_occurrences_no_match() {
        let cls = nsobj::nsmutablestring_class_addr();
        let obj = shim_nsmutablestring_string(cls, 0);

        let base = cfstring::shim_cf_string_create_with_cstring(0, c"hello".as_ptr(), 0);
        shim_nsmutablestring_init_with_string(obj, 0, base);

        let target = cfstring::shim_cf_string_create_with_cstring(0, c"xyz".as_ptr(), 0);
        let repl = cfstring::shim_cf_string_create_with_cstring(0, c"abc".as_ptr(), 0);

        let count = shim_nsmutablestring_replace_occurrences(obj, 0, target, repl, 0, 0, 5);
        assert_eq!(count, 0);

        let ptr = unsafe { *((obj as usize + C_STRING_PTR_OFF) as *const u64) as *const c_char };
        let result = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(result.to_str().unwrap(), "hello");
    }

    #[test]
    fn mutable_string_replace_occurrences_nil_safe() {
        assert_eq!(
            shim_nsmutablestring_replace_occurrences(0, 0, 0, 0, 0, 0, 0),
            0
        );
    }
}
