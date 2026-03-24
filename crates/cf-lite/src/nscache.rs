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

unsafe fn get_map(obj: u64) -> &'static mut CacheMap {
    let ptr = *((obj + 24) as *const u64);
    &mut *(ptr as *mut CacheMap)
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
    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) } as u64;
    if obj != 0 {
        unsafe { *(obj as *mut u64) = nsobj::nscache_class_addr(); }
    }
    obj
}

/// `-[NSCache objectForKey:]`
pub extern "C" fn shim_nscache_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 { return 0; }
    let map = unsafe { get_map(obj) };
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
    let map = unsafe { get_map(obj) };

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
    let map = unsafe { get_map(obj) };
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
    let map = unsafe { get_map(obj) };
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

/// `-[NSCache description]`
pub extern "C" fn shim_nscache_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = unsafe { *((obj + 32) as *const u64) };
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).unwrap_or("<unnamed>")
    } else {
        "<unnamed>"
    };
    let map = unsafe { get_map(obj) };
    let desc = format!("<NSCache: {} ({} objects)>", name_str, map.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}
