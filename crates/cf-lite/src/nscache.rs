//! NSCache shim: auto-evicting in-memory cache.
//!
//! Object layout (40 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: count_limit  (u64)
//!   offset 16: total_cost   (u64)  — sum of cost values
//!   offset 24: entries_ptr  (u64 → leaked Box<HashMap<u64, CacheEntry>>)
//!   offset 32: name_ptr     (u64 → NSString or 0)
//!
//! Uses pointer equality for keys (simplified vs real isEqual: dispatch).

use std::collections::HashMap;

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 40;

struct CacheEntry {
    value: u64,
    cost: u64,
}

type CacheMap = HashMap<u64, CacheEntry>;

unsafe fn get_map(obj: u64) -> *mut CacheMap {
    let ptr = *((obj + 24) as *const u64);
    ptr as *mut CacheMap
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSCache init]`
pub extern "C" fn shim_nscache_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nscache_class_addr();
        *((obj + 24) as *mut u64) = Box::into_raw(Box::new(CacheMap::new())) as u64;
    }
    obj
}

/// `+[NSCache alloc]`
pub extern "C" fn shim_nscache_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nscache_class_addr(); }
    }
    obj
}

/// `-[NSCache objectForKey:]`
pub extern "C" fn shim_nscache_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 { return 0; }
    let map = unsafe { &mut *get_map(obj) };
    map.get(&key).map(|e| e.value).unwrap_or(0)
}

/// `-[NSCache setObject:forKey:]`
pub extern "C" fn shim_nscache_set_object(obj: u64, _cmd: u64, value: u64, key: u64) {
    shim_nscache_set_object_cost(obj, _cmd, value, key, 0);
}

/// `-[NSCache setObject:forKey:cost:]`
pub extern "C" fn shim_nscache_set_object_cost(
    obj: u64,
    _cmd: u64,
    value: u64,
    key: u64,
    cost: u64,
) {
    if obj == 0 || key == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };

    // Update total cost
    if let Some(old) = map.get(&key) {
        let total = unsafe { &mut *((obj + 16) as *mut u64) };
        *total = total.saturating_sub(old.cost);
    }
    map.insert(key, CacheEntry { value, cost });
    unsafe {
        let total = &mut *((obj + 16) as *mut u64);
        *total = total.saturating_add(cost);
    }

    // Simple eviction: if countLimit > 0 and we exceed it, evict oldest
    let count_limit = unsafe { *((obj + 8) as *const u64) };
    if count_limit > 0 && map.len() as u64 > count_limit {
        // Evict first key (arbitrary — real NSCache uses LRU-ish policy)
        if let Some(&evict_key) = map.keys().find(|&&k| k != key) {
            if let Some(removed) = map.remove(&evict_key) {
                unsafe {
                    let total = &mut *((obj + 16) as *mut u64);
                    *total = total.saturating_sub(removed.cost);
                }
            }
        }
    }
}

/// `-[NSCache removeObjectForKey:]`
pub extern "C" fn shim_nscache_remove_object(obj: u64, _cmd: u64, key: u64) {
    if obj == 0 || key == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };
    if let Some(removed) = map.remove(&key) {
        unsafe {
            let total = &mut *((obj + 16) as *mut u64);
            *total = total.saturating_sub(removed.cost);
        }
    }
}

/// `-[NSCache removeAllObjects]`
pub extern "C" fn shim_nscache_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let map = unsafe { &mut *get_map(obj) };
    map.clear();
    unsafe { *((obj + 16) as *mut u64) = 0; }
}

/// `-[NSCache setCountLimit:]`
pub extern "C" fn shim_nscache_set_count_limit(obj: u64, _cmd: u64, limit: u64) {
    if obj == 0 { return; }
    unsafe { *((obj + 8) as *mut u64) = limit; }
}

/// `-[NSCache countLimit]`
pub extern "C" fn shim_nscache_count_limit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSCache setName:]`
pub extern "C" fn shim_nscache_set_name(obj: u64, _cmd: u64, name: u64) {
    if obj == 0 { return; }
    unsafe { *((obj + 32) as *mut u64) = name; }
}

/// `-[NSCache name]`
pub extern "C" fn shim_nscache_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 32) as *const u64) }
}

/// `-[NSCache totalCostLimit]` — return 0 (unlimited).
pub extern "C" fn shim_nscache_total_cost_limit(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSCache setTotalCostLimit:]` — no-op.
pub extern "C" fn shim_nscache_set_total_cost_limit(_obj: u64, _cmd: u64, _limit: u64) {}

/// `-[NSCache delegate]` — return 0 (nil).
pub extern "C" fn shim_nscache_delegate(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSCache setDelegate:]` — no-op.
pub extern "C" fn shim_nscache_set_delegate(_obj: u64, _cmd: u64, _delegate: u64) {}

/// `-[NSCache evictsObjectsWithDiscardedContent]` — return 1 (YES).
pub extern "C" fn shim_nscache_evicts_on_memory_warning(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSCache description]`
pub extern "C" fn shim_nscache_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = unsafe { *((obj + 32) as *const u64) };
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).unwrap_or("<unnamed>")
    } else {
        "<unnamed>"
    };
    let map = unsafe { &mut *get_map(obj) };
    let desc = format!("<NSCache: {} ({} objects)>", name_str, map.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = INSTANCE_SIZE as usize;

    fn make_cache() -> u64 {
        let obj = alloc_obj(SIZE, crate::nsobj::nscache_class_addr());
        shim_nscache_init(obj, 0);
        obj
    }

    // ── init ────────────────────────────────────────────────────

    #[test]
    fn init_returns_self() {
        let obj = alloc_obj(SIZE, crate::nsobj::nscache_class_addr());
        let ret = shim_nscache_init(obj, 0);
        assert_eq!(ret, obj);
        free_obj(obj);
    }

    #[test]
    fn init_nil_returns_zero() {
        assert_eq!(shim_nscache_init(0, 0), 0);
    }

    // ── alloc ───────────────────────────────────────────────────

    #[test]
    fn alloc_returns_nonzero_with_isa() {
        let obj = shim_nscache_alloc(0, 0);
        assert_ne!(obj, 0);
        let isa = read_field(obj, 0);
        assert_eq!(isa, crate::nsobj::nscache_class_addr());
        // can't free_obj safely since map is not initialised; just leak in test
    }

    // ── set / get object ────────────────────────────────────────

    #[test]
    fn set_and_get_object() {
        let cache = make_cache();
        let key = 0xAA;
        let val = 0xBB;
        shim_nscache_set_object(cache, 0, val, key);
        assert_eq!(shim_nscache_object_for_key(cache, 0, key), val);
        free_obj(cache);
    }

    #[test]
    fn get_missing_key_returns_zero() {
        let cache = make_cache();
        assert_eq!(shim_nscache_object_for_key(cache, 0, 0xFF), 0);
        free_obj(cache);
    }

    #[test]
    fn object_for_key_nil_obj() {
        assert_eq!(shim_nscache_object_for_key(0, 0, 1), 0);
    }

    #[test]
    fn object_for_key_nil_key() {
        let cache = make_cache();
        assert_eq!(shim_nscache_object_for_key(cache, 0, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn set_object_nil_obj_no_crash() {
        shim_nscache_set_object(0, 0, 1, 2);
    }

    #[test]
    fn set_object_nil_key_no_crash() {
        let cache = make_cache();
        shim_nscache_set_object(cache, 0, 1, 0);
        // Nothing stored
        assert_eq!(shim_nscache_object_for_key(cache, 0, 0), 0);
        free_obj(cache);
    }

    // ── set object with cost ────────────────────────────────────

    #[test]
    fn set_object_cost_tracks_total() {
        let cache = make_cache();
        shim_nscache_set_object_cost(cache, 0, 10, 1, 100);
        shim_nscache_set_object_cost(cache, 0, 20, 2, 200);
        let total = read_field(cache, 16);
        assert_eq!(total, 300);
        free_obj(cache);
    }

    #[test]
    fn overwrite_key_updates_cost() {
        let cache = make_cache();
        shim_nscache_set_object_cost(cache, 0, 10, 1, 100);
        shim_nscache_set_object_cost(cache, 0, 11, 1, 50);
        let total = read_field(cache, 16);
        assert_eq!(total, 50);
        // Value updated
        assert_eq!(shim_nscache_object_for_key(cache, 0, 1), 11);
        free_obj(cache);
    }

    #[test]
    fn set_object_cost_nil_obj_no_crash() {
        shim_nscache_set_object_cost(0, 0, 1, 2, 10);
    }

    #[test]
    fn set_object_cost_nil_key_no_crash() {
        let cache = make_cache();
        shim_nscache_set_object_cost(cache, 0, 1, 0, 10);
        free_obj(cache);
    }

    // ── remove ──────────────────────────────────────────────────

    #[test]
    fn remove_object_for_key() {
        let cache = make_cache();
        shim_nscache_set_object_cost(cache, 0, 10, 1, 100);
        shim_nscache_remove_object(cache, 0, 1);
        assert_eq!(shim_nscache_object_for_key(cache, 0, 1), 0);
        let total = read_field(cache, 16);
        assert_eq!(total, 0);
        free_obj(cache);
    }

    #[test]
    fn remove_nonexistent_key_no_crash() {
        let cache = make_cache();
        shim_nscache_remove_object(cache, 0, 0xFF);
        free_obj(cache);
    }

    #[test]
    fn remove_object_nil_obj() {
        shim_nscache_remove_object(0, 0, 1);
    }

    #[test]
    fn remove_object_nil_key() {
        let cache = make_cache();
        shim_nscache_remove_object(cache, 0, 0);
        free_obj(cache);
    }

    // ── remove all ──────────────────────────────────────────────

    #[test]
    fn remove_all_clears_entries_and_cost() {
        let cache = make_cache();
        shim_nscache_set_object_cost(cache, 0, 10, 1, 100);
        shim_nscache_set_object_cost(cache, 0, 20, 2, 200);
        shim_nscache_remove_all(cache, 0);
        assert_eq!(shim_nscache_object_for_key(cache, 0, 1), 0);
        assert_eq!(shim_nscache_object_for_key(cache, 0, 2), 0);
        assert_eq!(read_field(cache, 16), 0);
        free_obj(cache);
    }

    #[test]
    fn remove_all_nil_obj() {
        shim_nscache_remove_all(0, 0);
    }

    #[test]
    fn remove_all_empty_cache_no_crash() {
        let cache = make_cache();
        shim_nscache_remove_all(cache, 0);
        free_obj(cache);
    }

    // ── count limit ─────────────────────────────────────────────

    #[test]
    fn set_and_get_count_limit() {
        let cache = make_cache();
        shim_nscache_set_count_limit(cache, 0, 42);
        assert_eq!(shim_nscache_count_limit(cache, 0), 42);
        free_obj(cache);
    }

    #[test]
    fn default_count_limit_is_zero() {
        let cache = make_cache();
        assert_eq!(shim_nscache_count_limit(cache, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn count_limit_nil_obj() {
        assert_eq!(shim_nscache_count_limit(0, 0), 0);
    }

    #[test]
    fn set_count_limit_nil_obj() {
        shim_nscache_set_count_limit(0, 0, 10);
    }

    // ── eviction via count limit ────────────────────────────────

    #[test]
    fn eviction_when_over_count_limit() {
        let cache = make_cache();
        shim_nscache_set_count_limit(cache, 0, 2);
        shim_nscache_set_object(cache, 0, 10, 1);
        shim_nscache_set_object(cache, 0, 20, 2);
        // Third insert should evict one entry
        shim_nscache_set_object(cache, 0, 30, 3);
        // The new key must be present
        assert_eq!(shim_nscache_object_for_key(cache, 0, 3), 30);
        // At most count_limit entries remain
        let map = unsafe { &*get_map(cache) };
        assert!(map.len() <= 2);
        free_obj(cache);
    }

    // ── name ────────────────────────────────────────────────────

    #[test]
    fn set_and_get_name() {
        let cache = make_cache();
        let name = make_string("MyCache");
        shim_nscache_set_name(cache, 0, name);
        assert_eq!(shim_nscache_name(cache, 0), name);
        free_obj(cache);
    }

    #[test]
    fn default_name_is_zero() {
        let cache = make_cache();
        assert_eq!(shim_nscache_name(cache, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn name_nil_obj() {
        assert_eq!(shim_nscache_name(0, 0), 0);
    }

    #[test]
    fn set_name_nil_obj() {
        shim_nscache_set_name(0, 0, 1);
    }

    // ── description ─────────────────────────────────────────────

    #[test]
    fn description_empty_unnamed() {
        let cache = make_cache();
        let desc = shim_nscache_description(cache, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSCache: <unnamed> (0 objects)>");
        free_obj(cache);
    }

    #[test]
    fn description_with_name_and_entries() {
        let cache = make_cache();
        let name = make_string("Images");
        shim_nscache_set_name(cache, 0, name);
        shim_nscache_set_object(cache, 0, 10, 1);
        shim_nscache_set_object(cache, 0, 20, 2);
        let desc = shim_nscache_description(cache, 0);
        assert_string_eq(desc, "<NSCache: Images (2 objects)>");
        free_obj(cache);
    }

    #[test]
    fn description_nil_obj() {
        assert_eq!(shim_nscache_description(0, 0), 0);
    }

    // ── totalCostLimit ──────────────────────────────────────────

    #[test]
    fn total_cost_limit_returns_zero() {
        let cache = make_cache();
        assert_eq!(shim_nscache_total_cost_limit(cache, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn total_cost_limit_nil_obj() {
        assert_eq!(shim_nscache_total_cost_limit(0, 0), 0);
    }

    #[test]
    fn set_total_cost_limit_no_crash() {
        let cache = make_cache();
        shim_nscache_set_total_cost_limit(cache, 0, 1024);
        // Still returns 0 — stub ignores the value
        assert_eq!(shim_nscache_total_cost_limit(cache, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn set_total_cost_limit_nil_obj_no_crash() {
        shim_nscache_set_total_cost_limit(0, 0, 100);
    }

    // ── delegate ────────────────────────────────────────────────

    #[test]
    fn delegate_returns_zero() {
        let cache = make_cache();
        assert_eq!(shim_nscache_delegate(cache, 0), 0);
        free_obj(cache);
    }

    #[test]
    fn delegate_nil_obj() {
        assert_eq!(shim_nscache_delegate(0, 0), 0);
    }

    #[test]
    fn set_delegate_no_crash() {
        let cache = make_cache();
        shim_nscache_set_delegate(cache, 0, 0xBEEF);
        free_obj(cache);
    }

    #[test]
    fn set_delegate_nil_obj_no_crash() {
        shim_nscache_set_delegate(0, 0, 0xBEEF);
    }

    // ── evictsObjectsWithDiscardedContent ───────────────────────

    #[test]
    fn evicts_on_memory_warning_returns_one() {
        let cache = make_cache();
        assert_eq!(shim_nscache_evicts_on_memory_warning(cache, 0), 1);
        free_obj(cache);
    }

    #[test]
    fn evicts_on_memory_warning_nil_obj() {
        assert_eq!(shim_nscache_evicts_on_memory_warning(0, 0), 1);
    }
}
