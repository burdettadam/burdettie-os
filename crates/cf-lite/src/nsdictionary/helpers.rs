//! NSDictionary helper functions: layout constants, factory, and key lookup.

use crate::cfstring;
use crate::nsobj;

pub(super) const COUNT_OFF: usize = 8;
pub(super) const KEYS_OFF: usize = 16;
pub(super) const VALS_OFF: usize = 24;

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

/// Read count from an NSDictionary object.
pub(super) fn dict_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

/// Read keys pointer from an NSDictionary object.
pub(super) fn dict_keys(obj: u64) -> u64 {
    unsafe { *((obj as usize + KEYS_OFF) as *const u64) }
}

/// Read values pointer from an NSDictionary object.
pub(super) fn dict_vals(obj: u64) -> u64 {
    unsafe { *((obj as usize + VALS_OFF) as *const u64) }
}

/// Find the index of `key` in the dictionary, or None.
pub(super) fn find_key_index(obj: u64, key: u64) -> Option<usize> {
    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    if keys == 0 || count == 0 {
        return None;
    }

    let key_cstr = cfstring::extract_cfstring_cstr(key);

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
    }
    None
}
