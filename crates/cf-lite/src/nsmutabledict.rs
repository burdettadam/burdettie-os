//! NSMutableDictionary shim: mutable key-value collection.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa       (u64 — pointer to NSMutableDictionary class)
//!   offset  8: count     (u64 — number of active key-value pairs)
//!   offset 16: keys_ptr  (u64 — pointer to malloc'd id[] of capacity slots)
//!   offset 24: vals_ptr  (u64 — pointer to malloc'd id[] of capacity slots)
//!   offset 32: capacity  (u64 — number of allocated slots)
//!
//! Total: 40 bytes per mutable dictionary object.
//!
//! Offsets 8, 16, 24 are identical to NSDictionary, so all immutable
//! read methods (count, objectForKey:, allKeys, allValues, description)
//! can be registered with the NSDictionary function pointers directly.

use crate::cfstring;

const COUNT_OFF: usize = 8;
const KEYS_OFF: usize = 16;
const VALS_OFF: usize = 24;
const CAP_OFF: usize = 32;

const INITIAL_CAPACITY: u64 = 4;

/// `+[NSMutableDictionary dictionary]` — creates an empty mutable dictionary.
pub extern "C" fn shim_nsmutabledict_dictionary(cls: u64, _cmd: u64) -> u64 {
    shim_nsmutabledict_dictionary_with_capacity(cls, _cmd, INITIAL_CAPACITY)
}

/// `+[NSMutableDictionary dictionaryWithCapacity:]` — creates an empty
/// mutable dictionary with pre-allocated capacity.
pub extern "C" fn shim_nsmutabledict_dictionary_with_capacity(
    cls: u64,
    _cmd: u64,
    capacity: u64,
) -> u64 {
    if cls == 0 {
        return 0;
    }

    let cap = if capacity == 0 {
        INITIAL_CAPACITY
    } else {
        capacity
    };

    let obj = unsafe { libc::calloc(1, 40) } as u64;
    if obj == 0 {
        return 0;
    }

    // Set ISA = NSMutableDictionary class
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutabledictionary_class_addr();
    }

    // Allocate key and value buffers
    let key_buf = unsafe { libc::calloc(cap as usize, 8) } as u64;
    let val_buf = unsafe { libc::calloc(cap as usize, 8) } as u64;
    unsafe {
        *((obj as usize + KEYS_OFF) as *mut u64) = key_buf;
        *((obj as usize + VALS_OFF) as *mut u64) = val_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = cap;
    }

    log::debug!(
        "NSMutableDictionary dictionary → {:#x} (capacity {})",
        obj,
        cap
    );
    obj
}

/// Grow the key/value buffers when count == capacity.
unsafe fn grow_if_needed(obj: u64) {
    let count = *((obj as usize + COUNT_OFF) as *const u64);
    let cap = *((obj as usize + CAP_OFF) as *const u64);
    if count < cap {
        return;
    }

    let new_cap = if cap == 0 { INITIAL_CAPACITY } else { cap * 2 };

    let old_keys = *((obj as usize + KEYS_OFF) as *const u64);
    let old_vals = *((obj as usize + VALS_OFF) as *const u64);

    let new_keys = libc::realloc(old_keys as *mut libc::c_void, new_cap as usize * 8) as u64;
    let new_vals = libc::realloc(old_vals as *mut libc::c_void, new_cap as usize * 8) as u64;

    if new_keys != 0 {
        // Zero newly allocated key slots
        let start = (new_keys as usize + count as usize * 8) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - count) as usize * 8);
        *((obj as usize + KEYS_OFF) as *mut u64) = new_keys;
    }
    if new_vals != 0 {
        // Zero newly allocated value slots
        let start = (new_vals as usize + count as usize * 8) as *mut u8;
        std::ptr::write_bytes(start, 0, (new_cap - count) as usize * 8);
        *((obj as usize + VALS_OFF) as *mut u64) = new_vals;
    }
    *((obj as usize + CAP_OFF) as *mut u64) = new_cap;
}

/// Find the index of `key` in the mutable dictionary, using pointer
/// equality first then string content equality for CFString keys.
fn find_key_index(obj: u64, key: u64) -> Option<usize> {
    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let keys = unsafe { *((obj as usize + KEYS_OFF) as *const u64) };
    if keys == 0 || count == 0 {
        return None;
    }

    let key_cstr = cfstring::extract_cfstring_cstr(key);

    for i in 0..count {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        if k == key {
            return Some(i);
        }
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

/// `-[NSMutableDictionary setObject:forKey:]` — sets a key-value pair.
/// If the key already exists, updates the value. Otherwise appends.
pub extern "C" fn shim_nsmutabledict_set_object_for_key(
    obj: u64,
    _cmd: u64,
    val: u64,
    key: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Check if key already exists → update in place
    if let Some(i) = find_key_index(obj, key) {
        let vals = unsafe { *((obj as usize + VALS_OFF) as *const u64) };
        if vals != 0 {
            unsafe {
                *((vals as usize + i * 8) as *mut u64) = val;
            }
        }
        return 0;
    }

    // Key not found — append
    unsafe {
        grow_if_needed(obj);
    }

    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) };
    let keys = unsafe { *((obj as usize + KEYS_OFF) as *const u64) };
    let vals = unsafe { *((obj as usize + VALS_OFF) as *const u64) };

    if keys != 0 && vals != 0 {
        unsafe {
            *((keys as usize + count as usize * 8) as *mut u64) = key;
            *((vals as usize + count as usize * 8) as *mut u64) = val;
            *((obj as usize + COUNT_OFF) as *mut u64) = count + 1;
        }
    }
    0
}

/// `-[NSMutableDictionary setObject:forKeyedSubscript:]` — subscript setter.
/// Supports `dict[@"key"] = value` syntax. Delegates to setObject:forKey:.
pub extern "C" fn shim_nsmutabledict_set_object_for_keyed_subscript(
    obj: u64,
    _cmd: u64,
    val: u64,
    key: u64,
) -> u64 {
    shim_nsmutabledict_set_object_for_key(obj, _cmd, val, key)
}

/// `-[NSMutableDictionary removeObjectForKey:]` — removes the pair for the given key.
pub extern "C" fn shim_nsmutabledict_remove_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let Some(idx) = find_key_index(obj, key) else {
        return 0;
    };

    let count = unsafe { *((obj as usize + COUNT_OFF) as *const u64) } as usize;
    let keys = unsafe { *((obj as usize + KEYS_OFF) as *const u64) };
    let vals = unsafe { *((obj as usize + VALS_OFF) as *const u64) };

    // Shift entries [idx+1..count) left by one slot
    if idx + 1 < count && keys != 0 && vals != 0 {
        let remaining = (count - idx - 1) * 8;
        unsafe {
            std::ptr::copy(
                (keys as usize + (idx + 1) * 8) as *const u8,
                (keys as usize + idx * 8) as *mut u8,
                remaining,
            );
            std::ptr::copy(
                (vals as usize + (idx + 1) * 8) as *const u8,
                (vals as usize + idx * 8) as *mut u8,
                remaining,
            );
        }
    }

    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = (count - 1) as u64;
    }
    0
}

/// `-[NSMutableDictionary removeAllObjects]` — sets count to 0 (keeps capacity).
pub extern "C" fn shim_nsmutabledict_remove_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + COUNT_OFF) as *mut u64) = 0;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsdictionary;

    fn make_str(s: &str) -> u64 {
        let c = std::ffi::CString::new(s).unwrap();
        cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
    }

    fn make_empty(cap: u64) -> u64 {
        let obj = unsafe { libc::calloc(1, 40) } as u64;
        unsafe {
            *(obj as *mut u64) = 0xDEAD; // fake ISA
            let key_buf = libc::calloc(cap as usize, 8) as u64;
            let val_buf = libc::calloc(cap as usize, 8) as u64;
            *((obj as usize + KEYS_OFF) as *mut u64) = key_buf;
            *((obj as usize + VALS_OFF) as *mut u64) = val_buf;
            *((obj as usize + CAP_OFF) as *mut u64) = cap;
        }
        obj
    }

    fn get_count(obj: u64) -> u64 {
        unsafe { *((obj as usize + COUNT_OFF) as *const u64) }
    }

    fn free_dict(obj: u64) {
        let k = unsafe { *((obj as usize + KEYS_OFF) as *const u64) };
        let v = unsafe { *((obj as usize + VALS_OFF) as *const u64) };
        if k != 0 {
            unsafe {
                libc::free(k as *mut libc::c_void);
            }
        }
        if v != 0 {
            unsafe {
                libc::free(v as *mut libc::c_void);
            }
        }
        unsafe {
            libc::free(obj as *mut libc::c_void);
        }
    }

    #[test]
    fn set_and_get() {
        let dict = make_empty(4);
        let k1 = make_str("name");
        let v1 = make_str("Alice");
        shim_nsmutabledict_set_object_for_key(dict, 0, v1, k1);
        assert_eq!(get_count(dict), 1);

        let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k1);
        let s = cfstring::extract_cfstring_cstr(result).unwrap();
        assert_eq!(s, "Alice");
        free_dict(dict);
    }

    #[test]
    fn update_existing_key() {
        let dict = make_empty(4);
        let k = make_str("key");
        let v1 = make_str("old");
        let v2 = make_str("new");
        shim_nsmutabledict_set_object_for_key(dict, 0, v1, k);
        shim_nsmutabledict_set_object_for_key(dict, 0, v2, k);
        assert_eq!(get_count(dict), 1); // count stays 1

        let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k);
        let s = cfstring::extract_cfstring_cstr(result).unwrap();
        assert_eq!(s, "new");
        free_dict(dict);
    }

    #[test]
    fn remove_object_for_key() {
        let dict = make_empty(4);
        let k1 = make_str("a");
        let k2 = make_str("b");
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("1"), k1);
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("2"), k2);
        assert_eq!(get_count(dict), 2);

        shim_nsmutabledict_remove_object_for_key(dict, 0, k1);
        assert_eq!(get_count(dict), 1);

        // "a" gone, "b" still present
        assert_eq!(
            nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k1),
            0
        );
        assert_ne!(
            nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k2),
            0
        );
        free_dict(dict);
    }

    #[test]
    fn remove_all_objects() {
        let dict = make_empty(4);
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("k1"));
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("k2"));
        assert_eq!(get_count(dict), 2);

        shim_nsmutabledict_remove_all_objects(dict, 0);
        assert_eq!(get_count(dict), 0);
        free_dict(dict);
    }

    #[test]
    fn grow_beyond_capacity() {
        let dict = make_empty(2);
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v1"), make_str("k1"));
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v2"), make_str("k2"));
        // This triggers grow (count == capacity)
        shim_nsmutabledict_set_object_for_key(dict, 0, make_str("v3"), make_str("k3"));
        assert_eq!(get_count(dict), 3);
        let cap = unsafe { *((dict as usize + CAP_OFF) as *const u64) };
        assert!(cap >= 3, "capacity should have grown");
        free_dict(dict);
    }

    #[test]
    fn subscript_setter() {
        let dict = make_empty(4);
        let k = make_str("x");
        let v = make_str("y");
        shim_nsmutabledict_set_object_for_keyed_subscript(dict, 0, v, k);
        assert_eq!(get_count(dict), 1);
        let result = nsdictionary::shim_nsdictionary_object_for_keyed_subscript(dict, 0, k);
        let s = cfstring::extract_cfstring_cstr(result).unwrap();
        assert_eq!(s, "y");
        free_dict(dict);
    }

    #[test]
    fn nil_operations_safe() {
        assert_eq!(shim_nsmutabledict_set_object_for_key(0, 0, 0xA, 0xB), 0);
        assert_eq!(shim_nsmutabledict_remove_object_for_key(0, 0, 0xA), 0);
        assert_eq!(shim_nsmutabledict_remove_all_objects(0, 0), 0);
    }

    #[test]
    fn string_content_key_lookup() {
        let dict = make_empty(4);
        let k1 = make_str("hello");
        let v1 = make_str("world");
        shim_nsmutabledict_set_object_for_key(dict, 0, v1, k1);

        // Look up with a different CFString that has the same content
        let k2 = make_str("hello");
        assert_ne!(k1, k2); // different pointers
        let result = nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k2);
        let s = cfstring::extract_cfstring_cstr(result).unwrap();
        assert_eq!(s, "world");
        free_dict(dict);
    }
}
