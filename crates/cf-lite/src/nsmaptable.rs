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

unsafe fn get_map(obj: u64) -> &'static mut MapStorage {
    let ptr = *((obj + 24) as *const u64);
    &mut *(ptr as *mut MapStorage)
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSMapTable mapTableWithKeyOptions:valueOptions:]`
pub extern "C" fn shim_maptable_with_options(
    _cls: u64,
    _cmd: u64,
    key_opts: u64,
    val_opts: u64,
) -> u64 {
    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) } as u64;
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
    let map = unsafe { get_map(obj) };
    map.get(&key).copied().unwrap_or(0)
}

/// `-[NSMapTable setObject:forKey:]`
pub extern "C" fn shim_maptable_set_object(obj: u64, _cmd: u64, value: u64, key: u64) {
    if obj == 0 { return; }
    let map = unsafe { get_map(obj) };
    map.insert(key, value);
}

/// `-[NSMapTable removeObjectForKey:]`
pub extern "C" fn shim_maptable_remove_object(obj: u64, _cmd: u64, key: u64) {
    if obj == 0 { return; }
    let map = unsafe { get_map(obj) };
    map.remove(&key);
}

/// `-[NSMapTable removeAllObjects]`
pub extern "C" fn shim_maptable_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let map = unsafe { get_map(obj) };
    map.clear();
}

/// `-[NSMapTable count]`
pub extern "C" fn shim_maptable_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let map = unsafe { get_map(obj) };
    map.len() as u64
}

/// `-[NSMapTable description]`
pub extern "C" fn shim_maptable_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let map = unsafe { get_map(obj) };
    let desc = format!("<NSMapTable: {} entries>", map.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}
