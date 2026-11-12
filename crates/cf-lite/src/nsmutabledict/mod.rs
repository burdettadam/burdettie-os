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
use crate::nsnumber;

const COUNT_OFF: usize = 8;
const KEYS_OFF: usize = 16;
const VALS_OFF: usize = 24;
const CAP_OFF: usize = 32;

const INITIAL_CAPACITY: u64 = 4;

/// Retain an object stored in a collection so it stays alive.
/// Skips nil and objects that aren't heap-allocated.
fn retain_stored(obj: u64) {
    if obj != 0 {
        objc_runtime::refcount::REFCOUNTS.retain(obj);
    }
}

/// Copy a dictionary key.
///
/// Apple's NSDictionary always calls `-[key copyWithZone:]` before
/// storing. For CFString/NSString keys we snapshot the content into a
/// new immutable string so that mutable-buffer reuse by callers does
/// not corrupt existing entries.  Non-string keys are passed through.
fn copy_key(key: u64) -> u64 {
    if key == 0 {
        return 0;
    }
    if let Some(s) = cfstring::extract_cfstring_cstr(key) {
        let c = std::ffi::CString::new(s).unwrap_or_default();
        cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
    } else {
        key
    }
}

fn normalized_capacity(capacity: u64) -> u64 {
    if capacity == 0 {
        INITIAL_CAPACITY
    } else {
        capacity
    }
}

fn init_dictionary_storage(obj: u64, capacity: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let cap = normalized_capacity(capacity);
    let key_buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;
    let val_buf = crate::pal_alloc().calloc(cap as usize, 8) as u64;

    unsafe {
        if *(obj as *const u64) == 0 {
            *(obj as *mut u64) = crate::nsobj::nsmutabledictionary_class_addr();
        }

        let old_keys = *((obj as usize + KEYS_OFF) as *const u64);
        let old_vals = *((obj as usize + VALS_OFF) as *const u64);
        if old_keys != 0 {
            crate::pal_alloc().free(old_keys as *mut u8);
        }
        if old_vals != 0 {
            crate::pal_alloc().free(old_vals as *mut u8);
        }

        *((obj as usize + COUNT_OFF) as *mut u64) = 0;
        *((obj as usize + KEYS_OFF) as *mut u64) = key_buf;
        *((obj as usize + VALS_OFF) as *mut u64) = val_buf;
        *((obj as usize + CAP_OFF) as *mut u64) = cap;
    }

    obj
}

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

    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmutabledictionary_class_addr();
    }

    init_dictionary_storage(obj, capacity);

    let cap = normalized_capacity(capacity);

    log::debug!(
        "NSMutableDictionary dictionary → {:#x} (capacity {})",
        obj,
        cap
    );
    obj
}

/// `-[NSMutableDictionary initWithCapacity:]` — initializes an allocated
/// mutable dictionary instance with storage for the requested capacity.
pub extern "C" fn shim_nsmutabledict_init_with_capacity(obj: u64, _cmd: u64, capacity: u64) -> u64 {
    init_dictionary_storage(obj, capacity)
}

/// `-[NSMutableDictionary initWithDictionary:]` — initializes an allocated
/// mutable dictionary instance by copying all key-value pairs from the source.
pub extern "C" fn shim_nsmutabledict_init_with_dictionary(obj: u64, _cmd: u64, source: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let count = if source == 0 {
        0
    } else {
        crate::nsdictionary::helpers_pub::dict_count_pub(source)
    };
    let cap = if count == 0 { INITIAL_CAPACITY } else { count };
    init_dictionary_storage(obj, cap);

    if source == 0 {
        return obj;
    }

    let keys = crate::nsdictionary::helpers_pub::dict_keys_pub(source);
    let vals = crate::nsdictionary::helpers_pub::dict_vals_pub(source);
    if keys == 0 || vals == 0 {
        return obj;
    }

    for i in 0..count as usize {
        let key = unsafe { *((keys as usize + i * 8) as *const u64) };
        let val = unsafe { *((vals as usize + i * 8) as *const u64) };
        shim_nsmutabledict_set_object_for_key(obj, 0, val, key);
    }

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

    let new_keys = crate::pal_alloc().realloc(old_keys as *mut u8, new_cap as usize * 8) as u64;
    let new_vals = crate::pal_alloc().realloc(old_vals as *mut u8, new_cap as usize * 8) as u64;

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
    let key_is_num = nsnumber::is_nsnumber(key);

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
        if key_is_num && nsnumber::is_nsnumber(k) && nsnumber::nsnumber_values_equal(key, k) {
            return Some(i);
        }
    }
    None
}

/// `-[NSMutableDictionary setObject:forKey:]` — sets a key-value pair.
/// If the key already exists, updates the value. Otherwise appends.
///
/// Apple's NSDictionary always copies keys via `-[key copy]`.  We
/// snapshot string content here so that callers who reuse a mutable
/// buffer for the key get correct behaviour.
pub extern "C" fn shim_nsmutabledict_set_object_for_key(
    obj: u64,
    _cmd: u64,
    val: u64,
    key: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Snapshot the key: Apple's NSDictionary copies keys on insertion.
    let key_copy = copy_key(key);

    // Retain the value so it stays alive while stored in the dict.
    retain_stored(val);
    retain_stored(key_copy);

    // Check if key already exists → update in place
    if let Some(i) = find_key_index(obj, key_copy) {
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
            *((keys as usize + count as usize * 8) as *mut u64) = key_copy;
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

/// `-[NSMutableDictionary addEntriesFromDictionary:]`
pub extern "C" fn shim_nsmutabledict_add_entries_from_dictionary(
    obj: u64,
    _cmd: u64,
    other: u64,
) -> u64 {
    if obj == 0 || other == 0 {
        return 0;
    }
    let count = crate::nsdictionary::shim_nsdictionary_count(other, 0);
    let keys = unsafe { *((other as usize + KEYS_OFF) as *const u64) };
    let vals = unsafe { *((other as usize + VALS_OFF) as *const u64) };
    if keys == 0 || vals == 0 {
        return 0;
    }
    for i in 0..count as usize {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        let v = unsafe { *((vals as usize + i * 8) as *const u64) };
        shim_nsmutabledict_set_object_for_key(obj, 0, v, k);
    }
    0
}

/// `-[NSMutableDictionary removeObjectsForKeys:]` — remove all entries whose
/// key appears in the given NSArray.
pub extern "C" fn shim_nsmutabledict_remove_objects_for_keys(
    obj: u64,
    _cmd: u64,
    keys: u64,
) -> u64 {
    if obj == 0 || keys == 0 {
        return 0;
    }
    let n = crate::nsarray::shim_nsarray_count(keys, 0);
    for i in 0..n {
        let k = crate::nsarray::shim_nsarray_object_at_index(keys, 0, i);
        shim_nsmutabledict_remove_object_for_key(obj, 0, k);
    }
    0
}

/// `-[NSMutableDictionary setDictionary:]` — replace all contents with entries
/// from `other`.  Clears current entries then copies from `other`.
pub extern "C" fn shim_nsmutabledict_set_dictionary(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    shim_nsmutabledict_remove_all_objects(obj, 0);
    if other != 0 {
        shim_nsmutabledict_add_entries_from_dictionary(obj, 0, other);
    }
    0
}

#[cfg(test)]
mod tests;
