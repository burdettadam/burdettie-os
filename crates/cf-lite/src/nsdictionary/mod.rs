//! NSDictionary shim: immutable key-value collection.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa       (u64 — pointer to NSDictionary class)
//!   offset  8: count     (u64 — number of key-value pairs)
//!   offset 16: keys_ptr  (u64 — pointer to malloc'd [u64; count])
//!   offset 24: vals_ptr  (u64 — pointer to malloc'd [u64; count])
//!
//! Total: 32 bytes per dictionary object.
//!
//! Key lookup uses pointer equality first, then string content equality
//! (via `extract_cfstring_cstr`) for CFString/NSString keys.

mod helpers;
#[cfg(test)]
mod tests;

use crate::cfstring;
use crate::nsarray;
use crate::nsobj;

pub use helpers::make_nsdictionary;
use helpers::{dict_count, dict_keys, dict_vals, find_key_index, COUNT_OFF, KEYS_OFF, VALS_OFF};

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSDictionary dictionaryWithObjects:forKeys:count:]`
///
/// Class factory: creates an immutable dictionary from parallel C arrays.
/// `objects` and `keys` are pointers to `id[]`, `cnt` is the element count.
pub extern "C" fn shim_nsdictionary_with_objects_for_keys_count(
    _cls: u64,
    _cmd: u64,
    objects: u64,
    keys: u64,
    cnt: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = nsobj::nsdictionary_class_addr();
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = cnt;
    }

    if cnt > 0 {
        let key_buf_size = cnt as usize * 8;
        let val_buf_size = cnt as usize * 8;

        let key_buf = crate::pal_alloc().malloc(key_buf_size) as u64;
        let val_buf = crate::pal_alloc().malloc(val_buf_size) as u64;

        if key_buf != 0 && keys != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(keys as *const u8, key_buf as *mut u8, key_buf_size);
            }
        }
        if val_buf != 0 && objects != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    objects as *const u8,
                    val_buf as *mut u8,
                    val_buf_size,
                );
            }
        }

        unsafe {
            *((obj as usize + KEYS_OFF) as *mut u64) = key_buf;
            *((obj as usize + VALS_OFF) as *mut u64) = val_buf;
        }
    }

    log::debug!(
        "NSDictionary dictionaryWithObjects:forKeys:count: → {:#x} ({} pairs)",
        obj,
        cnt
    );
    obj
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSDictionary count]` — returns the number of key-value pairs.
pub extern "C" fn shim_nsdictionary_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    dict_count(obj)
}

/// `-[NSDictionary objectForKey:]` — returns the value for the given key, or nil.
pub extern "C" fn shim_nsdictionary_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    match find_key_index(obj, key) {
        Some(i) => {
            let vals = dict_vals(obj);
            if vals == 0 {
                return 0;
            }
            unsafe { *((vals as usize + i * 8) as *const u64) }
        }
        None => 0,
    }
}

/// `-[NSDictionary objectForKeyedSubscript:]` — same as objectForKey:.
/// Supports `dict[@"key"]` syntax.
pub extern "C" fn shim_nsdictionary_object_for_keyed_subscript(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> u64 {
    shim_nsdictionary_object_for_key(obj, 0, key)
}

/// `-[NSDictionary allKeys]` — returns an NSArray of all keys.
pub extern "C" fn shim_nsdictionary_all_keys(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = dict_count(obj);
    let keys = dict_keys(obj);
    nsarray::shim_nsarray_array_with_objects_count(nsobj::nsarray_class_addr(), 0, keys, count)
}

/// `-[NSDictionary allValues]` — returns an NSArray of all values.
pub extern "C" fn shim_nsdictionary_all_values(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = dict_count(obj);
    let vals = dict_vals(obj);
    nsarray::shim_nsarray_array_with_objects_count(nsobj::nsarray_class_addr(), 0, vals, count)
}

/// `-[NSDictionary description]` — returns a string representation.
pub extern "C" fn shim_nsdictionary_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    let vals = dict_vals(obj);

    let mut parts = Vec::new();
    for i in 0..count {
        let k = if keys != 0 {
            unsafe { *((keys as usize + i * 8) as *const u64) }
        } else {
            0
        };
        let v = if vals != 0 {
            unsafe { *((vals as usize + i * 8) as *const u64) }
        } else {
            0
        };

        let ks = cfstring::extract_cfstring_cstr(k)
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| format!("{:#x}", k));
        let vs = cfstring::extract_cfstring_cstr(v)
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| format!("{:#x}", v));

        parts.push(format!("{} = {}", ks, vs));
    }

    let desc = format!("{{{}}}", parts.join(", "));
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSDictionary enumerateKeysAndObjectsUsingBlock:]`
///
/// Calls `block(key, obj, &stop)` for each key-value pair.
/// Block invoke: `void (^)(void *block, id key, id obj, BOOL *stop)`
pub extern "C" fn shim_nsdictionary_enumerate_keys_and_objects_using_block(
    obj: u64,
    _cmd: u64,
    block: u64,
) {
    if obj == 0 || block == 0 {
        return;
    }
    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    let vals = dict_vals(obj);
    if count == 0 || keys == 0 || vals == 0 {
        return;
    }

    let invoke: extern "C" fn(u64, u64, u64, *mut u8) =
        unsafe { std::mem::transmute(*((block as usize + 16) as *const u64)) };

    let mut stop: u8 = 0;
    for i in 0..count {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        let v = unsafe { *((vals as usize + i * 8) as *const u64) };
        invoke(block, k, v, &mut stop);
        if stop != 0 {
            break;
        }
    }
}
