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

use crate::cfstring;
use crate::nsarray;
use crate::nsobj;

const COUNT_OFF: usize = 8;
const KEYS_OFF: usize = 16;
const VALS_OFF: usize = 24;

/// Create an NSDictionary from parallel key/value slices.
pub fn make_nsdictionary(keys: &[u64], vals: &[u64]) -> u64 {
    let cnt = keys.len().min(vals.len());
    let obj = unsafe { libc::calloc(1, 32) } as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsdictionary_class_addr();
        *((obj as usize + COUNT_OFF) as *mut u64) = cnt as u64;
    }
    if cnt > 0 {
        let key_buf = unsafe { libc::malloc(cnt * 8) } as *mut u64;
        let val_buf = unsafe { libc::malloc(cnt * 8) } as *mut u64;
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

// ── Helpers ────────────────────────────────────────────────────────

/// Read count from an NSDictionary object.
fn dict_count(obj: u64) -> u64 {
    unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
}

/// Read keys pointer from an NSDictionary object.
fn dict_keys(obj: u64) -> u64 {
    unsafe { *((obj as usize + KEYS_OFF) as *const u64) }
}

/// Read values pointer from an NSDictionary object.
fn dict_vals(obj: u64) -> u64 {
    unsafe { *((obj as usize + VALS_OFF) as *const u64) }
}

/// Find the index of `key` in the dictionary, or None.
fn find_key_index(obj: u64, key: u64) -> Option<usize> {
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
    let obj = unsafe { libc::calloc(1, 32) } as u64;
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

        let key_buf = unsafe { libc::malloc(key_buf_size) } as u64;
        let val_buf = unsafe { libc::malloc(val_buf_size) } as u64;

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
    let c_str = std::ffi::CString::new(desc).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cfstring(s: &str) -> u64 {
        let c = std::ffi::CString::new(s).unwrap();
        cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
    }

    #[test]
    fn create_and_count() {
        let k1 = make_cfstring("name");
        let k2 = make_cfstring("age");
        let v1 = make_cfstring("Alice");
        let v2 = make_cfstring("30");

        let keys = [k1, k2];
        let vals = [v1, v2];

        let dict = shim_nsdictionary_with_objects_for_keys_count(
            nsobj::nsdictionary_class_addr(),
            0,
            vals.as_ptr() as u64,
            keys.as_ptr() as u64,
            2,
        );
        assert_ne!(dict, 0);
        assert_eq!(shim_nsdictionary_count(dict, 0), 2);

        unsafe {
            libc::free(dict_keys(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict_vals(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict as *mut libc::c_void);
        }
    }

    #[test]
    fn object_for_key_hit() {
        let k1 = make_cfstring("x");
        let v1 = make_cfstring("hello");

        let keys = [k1];
        let vals = [v1];

        let dict = shim_nsdictionary_with_objects_for_keys_count(
            nsobj::nsdictionary_class_addr(),
            0,
            vals.as_ptr() as u64,
            keys.as_ptr() as u64,
            1,
        );

        // Look up by a *different* CFString with the same content
        let k_lookup = make_cfstring("x");
        let result = shim_nsdictionary_object_for_key(dict, 0, k_lookup);
        assert_ne!(result, 0);
        let s = cfstring::extract_cfstring_cstr(result).unwrap();
        assert_eq!(s, "hello");

        unsafe {
            libc::free(dict_keys(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict_vals(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict as *mut libc::c_void);
        }
    }

    #[test]
    fn object_for_key_miss() {
        let k1 = make_cfstring("x");
        let v1 = make_cfstring("y");
        let keys = [k1];
        let vals = [v1];

        let dict = shim_nsdictionary_with_objects_for_keys_count(
            nsobj::nsdictionary_class_addr(),
            0,
            vals.as_ptr() as u64,
            keys.as_ptr() as u64,
            1,
        );

        let k_miss = make_cfstring("z");
        assert_eq!(shim_nsdictionary_object_for_key(dict, 0, k_miss), 0);

        unsafe {
            libc::free(dict_keys(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict_vals(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict as *mut libc::c_void);
        }
    }

    #[test]
    fn all_keys_returns_nsarray() {
        let k1 = make_cfstring("a");
        let k2 = make_cfstring("b");
        let v1 = make_cfstring("1");
        let v2 = make_cfstring("2");
        let keys = [k1, k2];
        let vals = [v1, v2];

        let dict = shim_nsdictionary_with_objects_for_keys_count(
            nsobj::nsdictionary_class_addr(),
            0,
            vals.as_ptr() as u64,
            keys.as_ptr() as u64,
            2,
        );

        let arr = shim_nsdictionary_all_keys(dict, 0);
        assert_ne!(arr, 0);
        assert_eq!(nsarray::shim_nsarray_count(arr, 0), 2);

        unsafe {
            libc::free(dict_keys(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict_vals(dict) as *mut libc::c_void);
        }
        unsafe {
            libc::free(dict as *mut libc::c_void);
        }
    }

    #[test]
    fn empty_dictionary() {
        let dict = shim_nsdictionary_with_objects_for_keys_count(
            nsobj::nsdictionary_class_addr(),
            0,
            0,
            0,
            0,
        );
        assert_ne!(dict, 0);
        assert_eq!(shim_nsdictionary_count(dict, 0), 0);
        assert_eq!(shim_nsdictionary_object_for_key(dict, 0, 42), 0);
        assert!(shim_nsdictionary_all_keys(dict, 0) != 0);

        unsafe {
            libc::free(dict as *mut libc::c_void);
        }
    }
}
