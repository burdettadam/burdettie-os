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

fn dictionary_from_path(path: &str) -> u64 {
    crate::nspropertylistserialization::parse_plist_file(path)
}

/// Public accessors for dict layout helpers (used by NSJSONSerialization etc.).
pub mod helpers_pub {
    pub fn dict_count_pub(obj: u64) -> u64 { super::helpers::dict_count(obj) }
    pub fn dict_keys_pub(obj: u64) -> u64 { super::helpers::dict_keys(obj) }
    pub fn dict_vals_pub(obj: u64) -> u64 { super::helpers::dict_vals(obj) }
}

fn collect_pairs(obj: u64) -> (Vec<u64>, Vec<u64>) {
    if obj == 0 {
        return (Vec::new(), Vec::new());
    }

    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    let vals = dict_vals(obj);
    if count == 0 || keys == 0 || vals == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut collected_keys = Vec::with_capacity(count);
    let mut collected_vals = Vec::with_capacity(count);
    for i in 0..count {
        collected_keys.push(unsafe { *((keys as usize + i * 8) as *const u64) });
        collected_vals.push(unsafe { *((vals as usize + i * 8) as *const u64) });
    }
    (collected_keys, collected_vals)
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

/// `+[NSDictionary dictionaryWithContentsOfFile:]`
pub extern "C" fn shim_nsdictionary_with_contents_of_file(
    _cls: u64,
    _cmd: u64,
    path: u64,
) -> u64 {
    if path == 0 {
        return 0;
    }

    let Some(path_str) = cfstring::extract_cfstring_cstr(path) else {
        return 0;
    };

    dictionary_from_path(path_str)
}

/// `+[NSDictionary dictionaryWithContentsOfURL:]`
pub extern "C" fn shim_nsdictionary_with_contents_of_url(
    _cls: u64,
    _cmd: u64,
    url: u64,
) -> u64 {
    if url == 0 {
        return 0;
    }

    let path = crate::nsurl::shim_nsurl_path(url, 0);
    if path == 0 {
        return 0;
    }

    shim_nsdictionary_with_contents_of_file(_cls, _cmd, path)
}

/// `+[NSDictionary dictionaryWithContentsOfURL:error:]`
pub extern "C" fn shim_nsdictionary_with_contents_of_url_error(
    _cls: u64,
    _cmd: u64,
    url: u64,
    error: *mut u64,
) -> u64 {
    if !error.is_null() {
        unsafe {
            *error = 0;
        }
    }

    shim_nsdictionary_with_contents_of_url(_cls, _cmd, url)
}

// ── Additional class methods ───────────────────────────────────────

/// `+[NSDictionary dictionaryWithObject:forKey:]` — creates a single-entry dictionary.
pub extern "C" fn shim_nsdictionary_with_object_for_key(
    _cls: u64,
    _cmd: u64,
    object: u64,
    key: u64,
) -> u64 {
    make_nsdictionary(&[key], &[object])
}

/// `+[NSDictionary dictionaryWithDictionary:]` — creates a new dictionary copying all
/// entries from the source dictionary.
pub extern "C" fn shim_nsdictionary_with_dictionary(_cls: u64, _cmd: u64, source: u64) -> u64 {
    if source == 0 {
        return make_nsdictionary(&[], &[]);
    }
    let (keys, vals) = collect_pairs(source);
    make_nsdictionary(&keys, &vals)
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

/// `-[NSDictionary copy]` — immutable dictionaries return self.
pub extern "C" fn shim_nsdictionary_copy(obj: u64, _cmd: u64) -> u64 {
    objc_runtime::shim_nsobject_retain(obj, 0)
}

/// `-[NSDictionary mutableCopy]` — creates an NSMutableDictionary copy.
pub extern "C" fn shim_nsdictionary_mutable_copy(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let (keys, vals) = collect_pairs(obj);
    let copy = crate::nsmutabledict::shim_nsmutabledict_dictionary_with_capacity(
        nsobj::nsmutabledictionary_class_addr(),
        0,
        keys.len() as u64,
    );
    if copy == 0 {
        return 0;
    }

    for i in 0..keys.len() {
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(copy, 0, vals[i], keys[i]);
    }

    copy
}

/// `-[NSMutableDictionary copy]` — creates an immutable NSDictionary copy.
pub extern "C" fn shim_nsmutabledict_copy(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let (keys, vals) = collect_pairs(obj);
    make_nsdictionary(&keys, &vals)
}

/// `-[NSMutableDictionary mutableCopy]` — creates a new mutable copy.
pub extern "C" fn shim_nsmutabledict_mutable_copy(obj: u64, _cmd: u64) -> u64 {
    shim_nsdictionary_mutable_copy(obj, _cmd)
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

/// `-[NSDictionary isEqualToDictionary:]` — compares two dictionaries.
/// Returns 1 if both have the same count and identical key-value pairs
/// (compared by pointer equality and string content), 0 otherwise.
pub extern "C" fn shim_nsdictionary_is_equal_to_dictionary(
    obj: u64,
    _cmd: u64,
    other: u64,
) -> u8 {
    if obj == other {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }
    let count_a = dict_count(obj);
    let count_b = dict_count(other);
    if count_a != count_b {
        return 0;
    }
    let keys_a = dict_keys(obj);
    let vals_a = dict_vals(obj);
    if keys_a == 0 || vals_a == 0 {
        // Both have count 0 (checked above) or missing buffers
        return if count_a == 0 { 1 } else { 0 };
    }
    for i in 0..count_a as usize {
        let ka = unsafe { *((keys_a as usize + i * 8) as *const u64) };
        let va = unsafe { *((vals_a as usize + i * 8) as *const u64) };
        // Look up key in other dict
        let vb = shim_nsdictionary_object_for_key(other, 0, ka);
        if vb == 0 {
            return 0;
        }
        // Compare values
        if va == vb {
            continue;
        }
        // String content comparison
        let sa = cfstring::extract_cfstring_cstr(va);
        let sb = cfstring::extract_cfstring_cstr(vb);
        match (sa, sb) {
            (Some(a), Some(b)) if a == b => continue,
            _ => return 0,
        }
    }
    1
}

/// `-[NSDictionary writeToFile:atomically:]` — stub that always returns 0.
pub extern "C" fn shim_nsdictionary_write_to_file(
    _obj: u64,
    _cmd: u64,
    _path: u64,
    _atomically: u8,
) -> u8 {
    0
}

// Shared mutations counter for fast-enumeration (immutable → never mutates).
static NSDICT_FAST_ENUM_MUTATIONS: u64 = 0;

/// `-[NSDictionary countByEnumeratingWithState:objects:count:]`
///
/// Supports Objective-C fast enumeration (`for (id key in dict)`).
/// Enumerates *keys*, matching Apple's NSDictionary semantics.
pub extern "C" fn shim_nsdictionary_count_by_enumerating_with_state(
    obj: u64,
    _cmd: u64,
    state: u64,
    stackbuf: u64,
    len: u64,
) -> u64 {
    if obj == 0 || state == 0 || stackbuf == 0 || len == 0 {
        return 0;
    }

    let count = dict_count(obj) as usize;
    // state->state (offset 0) holds the current index
    let index = unsafe { *(state as *const u64) as usize };
    if index >= count {
        return 0;
    }

    let keys = dict_keys(obj);
    if keys == 0 {
        return 0;
    }

    let batch = std::cmp::min(len as usize, count - index);
    for offset in 0..batch {
        let key = unsafe { *((keys as usize + (index + offset) * 8) as *const u64) };
        unsafe {
            *((stackbuf as usize + offset * 8) as *mut u64) = key;
        }
    }

    unsafe {
        // state->state = new index
        *(state as *mut u64) = (index + batch) as u64;
        // state->itemsPtr = stackbuf
        *((state as usize + 8) as *mut u64) = stackbuf;
        // state->mutationsPtr
        *((state as usize + 16) as *mut u64) = (&raw const NSDICT_FAST_ENUM_MUTATIONS) as u64;
    }

    batch as u64
}
