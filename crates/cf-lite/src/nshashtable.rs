//! NSHashTable shim: set with configurable memory semantics.
//!
//! Object layout (24 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: options      (u64 → NSPointerFunctionsOptions)
//!   offset 16: set_ptr      (u64 → leaked Box<HashSet<u64>>)
//!
//! In the lab environment, weak references are treated as strong.

use std::collections::HashSet;

use crate::cfstring;
use crate::nsobj;

pub const INSTANCE_SIZE: u32 = 24;

type SetStorage = HashSet<u64>;

unsafe fn get_set(obj: u64) -> &'static mut SetStorage {
    let ptr = *((obj + 16) as *const u64);
    &mut *(ptr as *mut SetStorage)
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSHashTable hashTableWithOptions:]`
pub extern "C" fn shim_hashtable_with_options(_cls: u64, _cmd: u64, options: u64) -> u64 {
    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) } as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nshashtable_class_addr();
        *((obj + 8) as *mut u64) = options;
        *((obj + 16) as *mut u64) = Box::into_raw(Box::new(SetStorage::new())) as u64;
    }
    obj
}

/// `+[NSHashTable weakObjectsHashTable]`
pub extern "C" fn shim_hashtable_weak_objects(_cls: u64, _cmd: u64) -> u64 {
    shim_hashtable_with_options(0, 0, 1) // options=1 → weak (treated as strong)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSHashTable addObject:]`
pub extern "C" fn shim_hashtable_add_object(obj: u64, _cmd: u64, member: u64) {
    if obj == 0 { return; }
    let set = unsafe { get_set(obj) };
    set.insert(member);
}

/// `-[NSHashTable removeObject:]`
pub extern "C" fn shim_hashtable_remove_object(obj: u64, _cmd: u64, member: u64) {
    if obj == 0 { return; }
    let set = unsafe { get_set(obj) };
    set.remove(&member);
}

/// `-[NSHashTable removeAllObjects]`
pub extern "C" fn shim_hashtable_remove_all(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let set = unsafe { get_set(obj) };
    set.clear();
}

/// `-[NSHashTable containsObject:]`
pub extern "C" fn shim_hashtable_contains(obj: u64, _cmd: u64, member: u64) -> u8 {
    if obj == 0 { return 0; }
    let set = unsafe { get_set(obj) };
    u8::from(set.contains(&member))
}

/// `-[NSHashTable count]`
pub extern "C" fn shim_hashtable_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let set = unsafe { get_set(obj) };
    set.len() as u64
}

/// `-[NSHashTable allObjects]`
pub extern "C" fn shim_hashtable_all_objects(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let set = unsafe { get_set(obj) };
    let items: Vec<u64> = set.iter().copied().collect();
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSHashTable description]`
pub extern "C" fn shim_hashtable_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let set = unsafe { get_set(obj) };
    let desc = format!("<NSHashTable: {} objects>", set.len());
    let c_desc = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_desc.as_ptr(), 0)
}
