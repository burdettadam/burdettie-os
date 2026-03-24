//! NSAttributedString shim: string with attribute ranges.
//!
//! Object layout (40 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: string_ptr   (u64 → NSString)
//!   offset 16: attrs_count  (u64 → number of attribute runs)
//!   offset 24: attrs_ptr    (u64 → malloc'd AttrRun[])
//!   offset 32: reserved     (u64)
//!
//! AttrRun layout (24 bytes each):
//!   offset  0: location  (u64)
//!   offset  8: length    (u64)
//!   offset 16: dict_ptr  (u64 → NSDictionary of attributes)

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 40;
const ATTR_RUN_SIZE: usize = 24;

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSAttributedString alloc]` + `-[NSAttributedString initWithString:]`
/// Simplified: `initWithString:` as class-level convenience.
pub extern "C" fn shim_attrstr_init_with_string(obj: u64, _cmd: u64, string: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsattributedstring_class_addr();
        *((obj + 8) as *mut u64) = string;
        *((obj + 16) as *mut u64) = 0; // no attrs
    }
    obj
}

/// `-[NSAttributedString initWithString:attributes:]`
pub extern "C" fn shim_attrstr_init_with_string_attributes(
    obj: u64,
    _cmd: u64,
    string: u64,
    attrs: u64,
) -> u64 {
    if obj == 0 { return 0; }

    let str_len = if string != 0 {
        cfstring::shim_nsstring_length(string, 0)
    } else {
        0
    };

    unsafe {
        *(obj as *mut u64) = nsobj::nsattributedstring_class_addr();
        *((obj + 8) as *mut u64) = string;
    }

    if attrs != 0 && str_len > 0 {
        // Single run spanning the whole string
        let run_buf = unsafe { libc::calloc(1, ATTR_RUN_SIZE) } as u64;
        unsafe {
            *(run_buf as *mut u64) = 0;                   // location
            *((run_buf + 8) as *mut u64) = str_len as u64; // length
            *((run_buf + 16) as *mut u64) = attrs;        // dict
            *((obj + 16) as *mut u64) = 1;                // attrs_count
            *((obj + 24) as *mut u64) = run_buf;          // attrs_ptr
        }
    }

    obj
}

/// `+[NSAttributedString alloc]`
pub extern "C" fn shim_attrstr_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) } as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nsattributedstring_class_addr(); }
    }
    obj
}

/// `+[NSMutableAttributedString alloc]`
pub extern "C" fn shim_mutable_attrstr_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) } as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nsmutableattributedstring_class_addr(); }
    }
    obj
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSAttributedString string]`
pub extern "C" fn shim_attrstr_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSAttributedString length]`
pub extern "C" fn shim_attrstr_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let string = unsafe { *((obj + 8) as *const u64) };
    if string == 0 { return 0; }
    cfstring::shim_nsstring_length(string, 0)
}

/// `-[NSAttributedString attributesAtIndex:effectiveRange:]`
///
/// Returns NSDictionary of attributes at the given index.
/// effectiveRange is ignored in this simplified implementation.
pub extern "C" fn shim_attrstr_attributes_at_index(
    obj: u64,
    _cmd: u64,
    index: u64,
    _effective_range: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let count = unsafe { *((obj + 16) as *const u64) } as usize;
    let runs_ptr = unsafe { *((obj + 24) as *const u64) };

    if count == 0 || runs_ptr == 0 {
        // Return empty dictionary
        return crate::nsdictionary::make_nsdictionary(&[], &[]);
    }

    for i in 0..count {
        let run = runs_ptr + (i * ATTR_RUN_SIZE) as u64;
        let loc = unsafe { *(run as *const u64) };
        let len = unsafe { *((run + 8) as *const u64) };
        if index >= loc && index < loc + len {
            return unsafe { *((run + 16) as *const u64) };
        }
    }

    crate::nsdictionary::make_nsdictionary(&[], &[])
}

/// `-[NSAttributedString description]`
pub extern "C" fn shim_attrstr_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let string = unsafe { *((obj + 8) as *const u64) };
    let s = cfstring::extract_cfstring_cstr(string).unwrap_or("");
    let count = unsafe { *((obj + 16) as *const u64) };
    let desc = format!("<NSAttributedString: \"{s}\" ({count} attr runs)>");
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

// ── Mutable methods ────────────────────────────────────────────────

/// `-[NSMutableAttributedString initWithString:]`
pub extern "C" fn shim_mutable_attrstr_init_with_string(obj: u64, _cmd: u64, string: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmutableattributedstring_class_addr();
        *((obj + 8) as *mut u64) = string;
    }
    obj
}

/// `-[NSMutableAttributedString addAttribute:value:range:]`
///
/// Simplified: replaces all attribute runs with a single run for the given range.
pub extern "C" fn shim_mutable_attrstr_add_attribute(
    obj: u64,
    _cmd: u64,
    attr_name: u64,
    value: u64,
    range_loc: u64,
    range_len: u64,
) {
    if obj == 0 { return; }

    let keys = [attr_name];
    let vals = [value];
    let dict = crate::nsdictionary::make_nsdictionary(&keys, &vals);

    let run_buf = unsafe { libc::calloc(1, ATTR_RUN_SIZE) } as u64;
    unsafe {
        *(run_buf as *mut u64) = range_loc;
        *((run_buf + 8) as *mut u64) = range_len;
        *((run_buf + 16) as *mut u64) = dict;
        *((obj + 16) as *mut u64) = 1;
        *((obj + 24) as *mut u64) = run_buf;
    }
}

/// `-[NSMutableAttributedString replaceCharactersInRange:withString:]`
///
/// Simplified: replaces the underlying string entirely (ignores range).
pub extern "C" fn shim_mutable_attrstr_replace_chars(
    obj: u64,
    _cmd: u64,
    _range_loc: u64,
    _range_len: u64,
    string: u64,
) {
    if obj == 0 { return; }
    unsafe { *((obj + 8) as *mut u64) = string; }
}
