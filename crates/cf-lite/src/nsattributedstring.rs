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
        let run_buf = crate::pal_alloc().calloc(1, ATTR_RUN_SIZE) as u64;
        unsafe {
            *(run_buf as *mut u64) = 0;                   // location
            *((run_buf + 8) as *mut u64) = str_len; // length
            *((run_buf + 16) as *mut u64) = attrs;        // dict
            *((obj + 16) as *mut u64) = 1;                // attrs_count
            *((obj + 24) as *mut u64) = run_buf;          // attrs_ptr
        }
    }

    obj
}

/// `+[NSAttributedString alloc]`
pub extern "C" fn shim_attrstr_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nsattributedstring_class_addr(); }
    }
    obj
}

/// `+[NSMutableAttributedString alloc]`
pub extern "C" fn shim_mutable_attrstr_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
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
/// Appends a new attribute run (does not merge overlapping ranges).
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

    let old_count = unsafe { *((obj + 16) as *const u64) } as usize;
    let old_ptr = unsafe { *((obj + 24) as *const u64) };
    let new_count = old_count + 1;
    let new_buf = crate::pal_alloc().calloc(new_count, ATTR_RUN_SIZE) as u64;
    if new_buf == 0 { return; }

    // Copy existing runs
    if old_count > 0 && old_ptr != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                old_ptr as *const u8,
                new_buf as *mut u8,
                old_count * ATTR_RUN_SIZE,
            );
        }
    }

    // Append new run
    let new_run = new_buf + (old_count * ATTR_RUN_SIZE) as u64;
    unsafe {
        *(new_run as *mut u64) = range_loc;
        *((new_run + 8) as *mut u64) = range_len;
        *((new_run + 16) as *mut u64) = dict;
        *((obj + 16) as *mut u64) = new_count as u64;
        *((obj + 24) as *mut u64) = new_buf;
    }
}

/// `-[NSMutableAttributedString replaceCharactersInRange:withString:]`
///
/// Replaces characters in the given range with the replacement string,
/// producing a new backing NSString.
pub extern "C" fn shim_mutable_attrstr_replace_chars(
    obj: u64,
    _cmd: u64,
    range_loc: u64,
    range_len: u64,
    replacement: u64,
) {
    if obj == 0 { return; }
    let old_str = unsafe { *((obj + 8) as *const u64) };
    let old_cstr = cfstring::extract_cfstring_cstr(old_str).unwrap_or("");
    let rep_cstr = if replacement != 0 {
        cfstring::extract_cfstring_cstr(replacement).unwrap_or("")
    } else {
        ""
    };

    let loc = range_loc as usize;
    let len = range_len as usize;
    let mut result = String::with_capacity(old_cstr.len());
    // Build prefix + replacement + suffix
    for (i, ch) in old_cstr.char_indices() {
        if i < loc {
            result.push(ch);
        } else if i >= loc + len {
            result.push(ch);
        }
        // characters in [loc, loc+len) are skipped
    }
    // Insert replacement at the right position
    if loc <= result.len() {
        result.insert_str(loc, rep_cstr);
    } else {
        result.push_str(rep_cstr);
    }

    let c_str = std::ffi::CString::new(result).unwrap_or_default();
    let new_nsstr = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    unsafe { *((obj + 8) as *mut u64) = new_nsstr; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── alloc ──────────────────────────────────────────────────────

    #[test]
    fn alloc_returns_nonzero_with_correct_isa() {
        let obj = shim_attrstr_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsattributedstring_class_addr());
        free_obj(obj);
    }

    #[test]
    fn mutable_alloc_returns_nonzero_with_correct_isa() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsmutableattributedstring_class_addr());
        free_obj(obj);
    }

    // ── initWithString ─────────────────────────────────────────────

    #[test]
    fn init_with_string_basic() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("hello");
        let ret = shim_attrstr_init_with_string(obj, 0, s);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, 8), s); // string_ptr
        assert_eq!(read_field(obj, 16), 0); // attrs_count = 0
        free_obj(obj);
    }

    #[test]
    fn init_with_string_nil_obj() {
        assert_eq!(shim_attrstr_init_with_string(0, 0, 42), 0);
    }

    #[test]
    fn init_with_string_empty_string() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("");
        let ret = shim_attrstr_init_with_string(obj, 0, s);
        assert_eq!(ret, obj);
        assert_eq!(shim_attrstr_length(obj, 0), 0);
        free_obj(obj);
    }

    // ── initWithString:attributes: ─────────────────────────────────

    #[test]
    fn init_with_string_attributes_basic() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("hi");
        let key = make_string("font");
        let val = make_string("Helvetica");
        let dict = crate::nsdictionary::make_nsdictionary(&[key], &[val]);
        let ret = shim_attrstr_init_with_string_attributes(obj, 0, s, dict);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, 8), s);
        assert_eq!(read_field(obj, 16), 1); // one attr run
        assert_ne!(read_field(obj, 24), 0); // attrs_ptr set
        free_obj(obj);
    }

    #[test]
    fn init_with_string_attributes_nil_obj() {
        assert_eq!(shim_attrstr_init_with_string_attributes(0, 0, 1, 1), 0);
    }

    #[test]
    fn init_with_string_attributes_nil_attrs() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("test");
        let ret = shim_attrstr_init_with_string_attributes(obj, 0, s, 0);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, 16), 0); // no attr runs
        free_obj(obj);
    }

    #[test]
    fn init_with_string_attributes_nil_string() {
        let obj = shim_attrstr_alloc(0, 0);
        let dict = crate::nsdictionary::make_nsdictionary(&[], &[]);
        let ret = shim_attrstr_init_with_string_attributes(obj, 0, 0, dict);
        assert_eq!(ret, obj);
        // str_len is 0 when string is nil, so no attr run created
        assert_eq!(read_field(obj, 16), 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_string_attributes_empty_string() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("");
        let dict = crate::nsdictionary::make_nsdictionary(&[make_string("k")], &[make_string("v")]);
        let ret = shim_attrstr_init_with_string_attributes(obj, 0, s, dict);
        assert_eq!(ret, obj);
        // Empty string → str_len = 0, no attr run even with dict
        assert_eq!(read_field(obj, 16), 0);
        free_obj(obj);
    }

    // ── string accessor ────────────────────────────────────────────

    #[test]
    fn string_accessor_returns_backing_string() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("world");
        shim_attrstr_init_with_string(obj, 0, s);
        assert_eq!(shim_attrstr_string(obj, 0), s);
        free_obj(obj);
    }

    #[test]
    fn string_accessor_nil_returns_zero() {
        assert_eq!(shim_attrstr_string(0, 0), 0);
    }

    // ── length ─────────────────────────────────────────────────────

    #[test]
    fn length_basic() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("abcde");
        shim_attrstr_init_with_string(obj, 0, s);
        assert_eq!(shim_attrstr_length(obj, 0), 5);
        free_obj(obj);
    }

    #[test]
    fn length_nil_returns_zero() {
        assert_eq!(shim_attrstr_length(0, 0), 0);
    }

    #[test]
    fn length_nil_backing_string() {
        let obj = shim_attrstr_alloc(0, 0);
        // string_ptr is 0 (calloc'd)
        assert_eq!(shim_attrstr_length(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn length_empty_string() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("");
        shim_attrstr_init_with_string(obj, 0, s);
        assert_eq!(shim_attrstr_length(obj, 0), 0);
        free_obj(obj);
    }

    // ── attributesAtIndex:effectiveRange: ──────────────────────────

    #[test]
    fn attributes_at_index_no_attrs_returns_empty_dict() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("test");
        shim_attrstr_init_with_string(obj, 0, s);
        let dict = shim_attrstr_attributes_at_index(obj, 0, 0, 0);
        assert_ne!(dict, 0); // returns empty dict, not nil
        free_obj(obj);
    }

    #[test]
    fn attributes_at_index_with_attrs() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("hi");
        let key = make_string("color");
        let val = make_string("red");
        let dict = crate::nsdictionary::make_nsdictionary(&[key], &[val]);
        shim_attrstr_init_with_string_attributes(obj, 0, s, dict);
        // The single attr run spans [0, 2), so index 0 should hit it
        let result = shim_attrstr_attributes_at_index(obj, 0, 0, 0);
        assert_eq!(result, dict);
        // Index 1 should also be in range
        let result1 = shim_attrstr_attributes_at_index(obj, 0, 1, 0);
        assert_eq!(result1, dict);
        free_obj(obj);
    }

    #[test]
    fn attributes_at_index_out_of_range() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("hi");
        let key = make_string("k");
        let val = make_string("v");
        let dict = crate::nsdictionary::make_nsdictionary(&[key], &[val]);
        shim_attrstr_init_with_string_attributes(obj, 0, s, dict);
        // Index 2 is past the run [0,2)
        let result = shim_attrstr_attributes_at_index(obj, 0, 2, 0);
        // Should return empty dict (not the attrs dict)
        assert_ne!(result, dict);
        assert_ne!(result, 0);
        free_obj(obj);
    }

    #[test]
    fn attributes_at_index_nil_returns_zero() {
        assert_eq!(shim_attrstr_attributes_at_index(0, 0, 0, 0), 0);
    }

    // ── description ────────────────────────────────────────────────

    #[test]
    fn description_basic() {
        let obj = shim_attrstr_alloc(0, 0);
        let s = make_string("hello");
        shim_attrstr_init_with_string(obj, 0, s);
        let desc = shim_attrstr_description(obj, 0);
        assert_ne!(desc, 0);
        let desc_str = read_string(desc);
        assert!(desc_str.contains("hello"), "description should contain the string: {desc_str}");
        assert!(desc_str.contains("0 attr runs"), "description should show attr count: {desc_str}");
        free_obj(obj);
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_attrstr_description(0, 0), 0);
    }

    // ── mutable initWithString ─────────────────────────────────────

    #[test]
    fn mutable_init_with_string_basic() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("mutable");
        let ret = shim_mutable_attrstr_init_with_string(obj, 0, s);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, 0), nsobj::nsmutableattributedstring_class_addr());
        assert_eq!(read_field(obj, 8), s);
        free_obj(obj);
    }

    #[test]
    fn mutable_init_with_string_nil_obj() {
        assert_eq!(shim_mutable_attrstr_init_with_string(0, 0, 42), 0);
    }

    // ── mutable addAttribute:value:range: ──────────────────────────

    #[test]
    fn add_attribute_basic() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("hello");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        let attr_name = make_string("NSFont");
        let value = make_string("Courier");
        shim_mutable_attrstr_add_attribute(obj, 0, attr_name, value, 0, 5);
        assert_eq!(read_field(obj, 16), 1); // attrs_count = 1
        assert_ne!(read_field(obj, 24), 0); // attrs_ptr set
        // Verify the run via attributes_at_index
        let dict = shim_attrstr_attributes_at_index(obj, 0, 0, 0);
        assert_ne!(dict, 0);
        free_obj(obj);
    }

    #[test]
    fn add_attribute_multiple() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("hello world");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        shim_mutable_attrstr_add_attribute(obj, 0, make_string("a"), make_string("1"), 0, 5);
        shim_mutable_attrstr_add_attribute(obj, 0, make_string("b"), make_string("2"), 6, 5);
        assert_eq!(read_field(obj, 16), 2); // two runs
        free_obj(obj);
    }

    #[test]
    fn add_attribute_nil_obj_does_not_crash() {
        shim_mutable_attrstr_add_attribute(0, 0, 1, 1, 0, 1);
    }

    // ── mutable replaceCharactersInRange:withString: ───────────────

    #[test]
    fn replace_chars_basic() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("hello world");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        let replacement = make_string("rust");
        shim_mutable_attrstr_replace_chars(obj, 0, 6, 5, replacement);
        let new_str = shim_attrstr_string(obj, 0);
        assert_string_eq(new_str, "hello rust");
        free_obj(obj);
    }

    #[test]
    fn replace_chars_insert_at_beginning() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("world");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        let replacement = make_string("hello ");
        shim_mutable_attrstr_replace_chars(obj, 0, 0, 0, replacement);
        let new_str = shim_attrstr_string(obj, 0);
        assert_string_eq(new_str, "hello world");
        free_obj(obj);
    }

    #[test]
    fn replace_chars_delete_range() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("abcdef");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        // Replace "cd" (range 2..4) with empty string
        let replacement = make_string("");
        shim_mutable_attrstr_replace_chars(obj, 0, 2, 2, replacement);
        let new_str = shim_attrstr_string(obj, 0);
        assert_string_eq(new_str, "abef");
        free_obj(obj);
    }

    #[test]
    fn replace_chars_nil_replacement() {
        let obj = shim_mutable_attrstr_alloc(0, 0);
        let s = make_string("abcdef");
        shim_mutable_attrstr_init_with_string(obj, 0, s);
        // nil replacement acts as empty string
        shim_mutable_attrstr_replace_chars(obj, 0, 2, 2, 0);
        let new_str = shim_attrstr_string(obj, 0);
        assert_string_eq(new_str, "abef");
        free_obj(obj);
    }

    #[test]
    fn replace_chars_nil_obj_does_not_crash() {
        shim_mutable_attrstr_replace_chars(0, 0, 0, 1, 0);
    }
}
