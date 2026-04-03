//! NSMapTable shim: key-value collection with configurable memory semantics.
//!
//! Object layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: key_options  (u64 → NSPointerFunctionsOptions)
//!   offset 16: val_options  (u64)
//!   offset 24: map_ptr      (u64 → leaked Box<HashMap<u64, u64>>)
//!
//! In the lab environment, weak references are treated as strong
//! (no zeroing-weak-reference support).

use std::collections::HashMap;

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 32;

type MapStorage = HashMap<u64, u64>;

unsafe fn get_map(obj: u64) -> *mut MapStorage {
    let ptr = *((obj + 24) as *const u64);
    ptr as *mut MapStorage
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSMapTable mapTableWithKeyOptions:valueOptions:]`
pub extern "C" fn shim_maptable_with_options(
    _cls: u64,
    _cmd: u64,
    key_opts: u64,
    val_opts: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmaptable_class_addr();
        *((obj + 8) as *mut u64) = key_opts;
        *((obj + 16) as *mut u64) = val_opts;
        *((obj + 24) as *mut u64) = Box::into_raw(Box::new(MapStorage::new())) as u64;
    }
    obj
}

/// `+[NSMapTable strongToStrongObjectsMapTable]`
pub extern "C" fn shim_maptable_strong_strong(_cls: u64, _cmd: u64) -> u64 {
    shim_maptable_with_options(0, 0, 0, 0) // options 0 = strong
}

/// `+[NSMapTable weakToStrongObjectsMapTable]`
pub extern "C" fn shim_maptable_weak_strong(_cls: u64, _cmd: u64) -> u64 {
    shim_maptable_with_options(0, 0, 1, 0) // simplified: weak = 1
}

/// `+[NSMapTable strongToWeakObjectsMapTable]`
pub extern "C" fn shim_maptable_strong_weak(_cls: u64, _cmd: u64) -> u64 {
    shim_maptable_with_options(0, 0, 0, 1)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSMapTable objectForKey:]`
pub extern "C" fn shim_maptable_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 { return 0; }
    let map = unsafe { &mut *get_map(obj) };
    map.get(&key).copied().unwrap_or(0)
}

/// `-[NSMapTable setObject:forKey:]`
pub extern "C" fn shim_maptable_set_object(obj: u64, _cmd: u64, value: u64, key: u64) {
    if obj == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };
    map.insert(key, value);
}

/// `-[NSMapTable removeObjectForKey:]`
pub extern "C" fn shim_maptable_remove_object(obj: u64, _cmd: u64, key: u64) {
    if obj == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };
    map.remove(&key);
}

/// `-[NSMapTable removeAllObjects]`
pub extern "C" fn shim_maptable_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };
    map.clear();
}

/// `-[NSMapTable count]`
pub extern "C" fn shim_maptable_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let map = unsafe { &mut *get_map(obj) };
    map.len() as u64
}

/// `-[NSMapTable description]`
pub extern "C" fn shim_maptable_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let map = unsafe { &mut *get_map(obj) };
    let desc = format!("<NSMapTable: {} entries>", map.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Helper: create an NSMapTable via the class method constructor.
    fn make_table() -> u64 {
        shim_maptable_with_options(0, 0, 0, 0)
    }

    // ── Construction ───────────────────────────────────────────────

    #[test]
    fn with_options_returns_nonzero() {
        let t = shim_maptable_with_options(0, 0, 5, 7);
        assert_ne!(t, 0);
        // Verify options stored
        assert_eq!(read_field(t, 8), 5);  // key_options
        assert_eq!(read_field(t, 16), 7); // val_options
        free_obj(t);
    }

    #[test]
    fn strong_strong_returns_nonzero() {
        let t = shim_maptable_strong_strong(0, 0);
        assert_ne!(t, 0);
        assert_eq!(shim_maptable_count(t, 0), 0);
        free_obj(t);
    }

    #[test]
    fn weak_strong_returns_nonzero() {
        let t = shim_maptable_weak_strong(0, 0);
        assert_ne!(t, 0);
        free_obj(t);
    }

    #[test]
    fn strong_weak_returns_nonzero() {
        let t = shim_maptable_strong_weak(0, 0);
        assert_ne!(t, 0);
        free_obj(t);
    }

    // ── setObject / objectForKey ────────────────────────────────────

    #[test]
    fn set_and_get_object() {
        let t = make_table();
        let k = make_string("key1");
        let v = make_string("value1");

        shim_maptable_set_object(t, 0, v, k);
        let got = shim_maptable_object_for_key(t, 0, k);
        assert_eq!(got, v);

        free_obj(t);
        free_obj(k);
        free_obj(v);
    }

    #[test]
    fn overwrite_key() {
        let t = make_table();
        let k = make_string("k");
        let v1 = make_string("first");
        let v2 = make_string("second");

        shim_maptable_set_object(t, 0, v1, k);
        shim_maptable_set_object(t, 0, v2, k);
        let got = shim_maptable_object_for_key(t, 0, k);
        assert_eq!(got, v2);
        assert_eq!(shim_maptable_count(t, 0), 1);

        free_obj(t);
        free_obj(k);
        free_obj(v1);
        free_obj(v2);
    }

    #[test]
    fn get_nonexistent_key_returns_zero() {
        let t = make_table();
        let k = make_string("missing");
        assert_eq!(shim_maptable_object_for_key(t, 0, k), 0);
        free_obj(t);
        free_obj(k);
    }

    // ── removeObjectForKey ─────────────────────────────────────────

    #[test]
    fn remove_existing_key() {
        let t = make_table();
        let k = make_string("del");
        let v = make_string("val");

        shim_maptable_set_object(t, 0, v, k);
        assert_eq!(shim_maptable_count(t, 0), 1);

        shim_maptable_remove_object(t, 0, k);
        assert_eq!(shim_maptable_count(t, 0), 0);
        assert_eq!(shim_maptable_object_for_key(t, 0, k), 0);

        free_obj(t);
        free_obj(k);
        free_obj(v);
    }

    #[test]
    fn remove_nonexistent_key_is_noop() {
        let t = make_table();
        let k = make_string("ghost");
        // Should not crash
        shim_maptable_remove_object(t, 0, k);
        assert_eq!(shim_maptable_count(t, 0), 0);
        free_obj(t);
        free_obj(k);
    }

    // ── removeAllObjects ───────────────────────────────────────────

    #[test]
    fn remove_all_clears_table() {
        let t = make_table();
        let k1 = make_string("a");
        let k2 = make_string("b");
        let v1 = make_string("1");
        let v2 = make_string("2");

        shim_maptable_set_object(t, 0, v1, k1);
        shim_maptable_set_object(t, 0, v2, k2);
        assert_eq!(shim_maptable_count(t, 0), 2);

        shim_maptable_remove_all(t, 0);
        assert_eq!(shim_maptable_count(t, 0), 0);

        free_obj(t);
        free_obj(k1);
        free_obj(k2);
        free_obj(v1);
        free_obj(v2);
    }

    #[test]
    fn remove_all_on_empty_table() {
        let t = make_table();
        shim_maptable_remove_all(t, 0);
        assert_eq!(shim_maptable_count(t, 0), 0);
        free_obj(t);
    }

    // ── count ──────────────────────────────────────────────────────

    #[test]
    fn count_tracks_inserts() {
        let t = make_table();
        assert_eq!(shim_maptable_count(t, 0), 0);

        for i in 0u64..5 {
            shim_maptable_set_object(t, 0, 0xBEEF, i + 1);
        }
        assert_eq!(shim_maptable_count(t, 0), 5);

        free_obj(t);
    }

    // ── description ────────────────────────────────────────────────

    #[test]
    fn description_empty_table() {
        let t = make_table();
        let desc = shim_maptable_description(t, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSMapTable: 0 entries>");
        free_obj(desc);
        free_obj(t);
    }

    #[test]
    fn description_with_entries() {
        let t = make_table();
        shim_maptable_set_object(t, 0, 42, 1);
        shim_maptable_set_object(t, 0, 43, 2);

        let desc = shim_maptable_description(t, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSMapTable: 2 entries>");
        free_obj(desc);
        free_obj(t);
    }

    // ── Nil safety ─────────────────────────────────────────────────

    #[test]
    fn nil_object_for_key() {
        assert_eq!(shim_maptable_object_for_key(0, 0, 1), 0);
    }

    #[test]
    fn nil_set_object() {
        // Must not crash
        shim_maptable_set_object(0, 0, 42, 1);
    }

    #[test]
    fn nil_remove_object() {
        shim_maptable_remove_object(0, 0, 1);
    }

    #[test]
    fn nil_remove_all() {
        shim_maptable_remove_all(0, 0);
    }

    #[test]
    fn nil_count() {
        assert_eq!(shim_maptable_count(0, 0), 0);
    }

    #[test]
    fn nil_description() {
        assert_eq!(shim_maptable_description(0, 0), 0);
    }
}
