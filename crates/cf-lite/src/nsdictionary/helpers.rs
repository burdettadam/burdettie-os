//! NSDictionary helper functions: layout constants, factory, and key lookup.

use crate::cfstring;
use crate::nsnumber;
use crate::nsobj;

// ── Dynamic NSDictionary layout (our objects) ──────────────────
pub(super) const COUNT_OFF: usize = 8;
pub(super) const KEYS_OFF: usize = 16;
pub(super) const VALS_OFF: usize = 24;

// ── NSConstantDictionary layout (compiler-embedded) ────────────
// offset 0: ISA, offset 8: flags, offset 16: count,
// offset 24: keys_ptr, offset 32: vals_ptr
const CONST_COUNT_OFF: usize = 16;
const CONST_KEYS_OFF: usize = 24;
const CONST_VALS_OFF: usize = 32;

/// Check if this dict object has the NSConstantDictionary layout.
fn is_constant_dict(obj: u64) -> bool {
    let isa = unsafe { *(obj as *const u64) };
    isa == nsobj::nsconstantdictionary_class_addr()
}

/// Create an NSDictionary from parallel key/value slices.
pub fn make_nsdictionary(keys: &[u64], vals: &[u64]) -> u64 {
    let cnt = keys.len().min(vals.len());
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsdictionary_class_addr();
        *((obj as usize + COUNT_OFF) as *mut u64) = cnt as u64;
    }
    if cnt > 0 {
        let key_buf = crate::pal_alloc().malloc(cnt * 8) as *mut u64;
        let val_buf = crate::pal_alloc().malloc(cnt * 8) as *mut u64;
        if !key_buf.is_null() && !val_buf.is_null() {
            for i in 0..cnt {
                unsafe {
                    key_buf.add(i).write(keys[i]);
                    val_buf.add(i).write(vals[i]);
                }
            }
        }
        unsafe {
            *((obj as usize + KEYS_OFF) as *mut u64) = key_buf as u64;
            *((obj as usize + VALS_OFF) as *mut u64) = val_buf as u64;
        }
    }
    obj
}

/// Read count from an NSDictionary or NSConstantDictionary object.
pub(super) fn dict_count(obj: u64) -> u64 {
    let off = if is_constant_dict(obj) { CONST_COUNT_OFF } else { COUNT_OFF };
    unsafe { std::ptr::read_unaligned((obj as usize + off) as *const u64) }
}

/// Read keys pointer from an NSDictionary or NSConstantDictionary object.
pub(super) fn dict_keys(obj: u64) -> u64 {
    let off = if is_constant_dict(obj) { CONST_KEYS_OFF } else { KEYS_OFF };
    unsafe { std::ptr::read_unaligned((obj as usize + off) as *const u64) }
}

/// Read values pointer from an NSDictionary or NSConstantDictionary object.
pub(super) fn dict_vals(obj: u64) -> u64 {
    let off = if is_constant_dict(obj) { CONST_VALS_OFF } else { VALS_OFF };
    unsafe { std::ptr::read_unaligned((obj as usize + off) as *const u64) }
}

/// Find the index of `key` in the dictionary, or None.
pub(super) fn find_key_index(obj: u64, key: u64) -> Option<usize> {
    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    if keys == 0 || count == 0 {
        return None;
    }

    let key_cstr = cfstring::extract_cfstring_cstr(key);
    let key_is_num = nsnumber::is_nsnumber(key);

    for i in 0..count {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        // Pointer equality
        if k == key {
            return Some(i);
        }
        // String content equality
        if let Some(target_s) = &key_cstr {
            if let Some(elem_s) = cfstring::extract_cfstring_cstr(k) {
                if elem_s == *target_s {
                    return Some(i);
                }
            }
        }
        // NSNumber value equality
        if key_is_num && nsnumber::is_nsnumber(k) && nsnumber::nsnumber_values_equal(key, k) {
            return Some(i);
        }
    }
    None
}
