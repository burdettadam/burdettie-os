//! @synchronized, associated objects, and weak references.

use std::collections::HashMap;
use std::sync::Mutex;

use super::retain_release::{shim_objc_release, shim_objc_retain};

// ── @synchronized support ─────────────────────────────────────────

/// Wrapper around a raw mutex pointer that is Send + Sync.
/// SAFETY: pthread mutexes are designed for cross-thread use.
struct SyncMutexPtr(*mut u8);
unsafe impl Send for SyncMutexPtr {}
unsafe impl Sync for SyncMutexPtr {}

/// Per-object mutex table for `@synchronized(obj) { ... }`.
///
/// Maps object addresses to recursive pthread mutexes. The table itself
/// is protected by a global mutex. Each per-object mutex is recursive
/// to support nested `@synchronized` on the same object from the same
/// thread (which is valid in ObjC).
static SYNC_TABLE: Mutex<Option<HashMap<u64, SyncMutexPtr>>> = Mutex::new(None);

/// Get or create the recursive mutex for a given object address.
fn sync_mutex_for(obj: u64) -> *mut u8 {
    let threading = crate::pal_threading();
    let mut guard = SYNC_TABLE.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    table
        .entry(obj)
        .or_insert_with(|| {
            let mutex = crate::pal_alloc().calloc(1, threading.mutex_size());
            threading.mutex_init_recursive(mutex);
            SyncMutexPtr(mutex)
        })
        .0
}

/// `int objc_sync_enter(id obj)` — lock the mutex associated with `obj`.
///
/// If `obj` is nil, this is a no-op (returns success). This matches
/// Apple's runtime behavior where `@synchronized(nil)` is valid but
/// provides no synchronization.
pub extern "C" fn shim_objc_sync_enter(obj: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    crate::pal_threading().mutex_lock(sync_mutex_for(obj));
    0
}

/// `int objc_sync_exit(id obj)` — unlock the mutex associated with `obj`.
pub extern "C" fn shim_objc_sync_exit(obj: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    crate::pal_threading().mutex_unlock(sync_mutex_for(obj));
    0
}

// ── Associated objects ─────────────────────────────────────────────

/// Association policy constants matching <objc/runtime.h>.
#[allow(dead_code)]
const OBJC_ASSOCIATION_ASSIGN: usize = 0;
#[allow(dead_code)]
const OBJC_ASSOCIATION_RETAIN_NONATOMIC: usize = 1;
#[allow(dead_code)]
const OBJC_ASSOCIATION_COPY_NONATOMIC: usize = 3;
#[allow(dead_code)]
const OBJC_ASSOCIATION_RETAIN: usize = 0x301;
#[allow(dead_code)]
const OBJC_ASSOCIATION_COPY: usize = 0x303;

type AssocMap = HashMap<u64, HashMap<u64, (u64, usize)>>;

/// Per-object associated value store: (object, key) → (value, policy).
static ASSOCIATED_OBJECTS: Mutex<Option<AssocMap>> = Mutex::new(None);

fn assoc_map() -> std::sync::MutexGuard<'static, Option<AssocMap>> {
    ASSOCIATED_OBJECTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// `void objc_setAssociatedObject(id object, const void *key, id value, objc_AssociationPolicy policy)`
pub extern "C" fn shim_objc_set_associated_object(
    object: u64,
    key: u64,
    value: u64,
    policy: usize,
) {
    if object == 0 {
        return;
    }
    let mut guard = assoc_map();
    let map = guard.get_or_insert_with(HashMap::new);
    let obj_map = map.entry(object).or_default();

    // Retain the new value if policy requires it
    let is_retain =
        policy == OBJC_ASSOCIATION_RETAIN_NONATOMIC || policy == OBJC_ASSOCIATION_RETAIN;
    if is_retain && value != 0 {
        shim_objc_retain(value);
    }

    // Release old value if it was retained
    if let Some((old_val, old_policy)) = obj_map.insert(key, (value, policy)) {
        let old_retain = old_policy == OBJC_ASSOCIATION_RETAIN_NONATOMIC
            || old_policy == OBJC_ASSOCIATION_RETAIN;
        if old_retain && old_val != 0 {
            shim_objc_release(old_val);
        }
    }

    // Remove the key if value is 0 (nil)
    if value == 0 {
        obj_map.remove(&key);
        if obj_map.is_empty() {
            map.remove(&object);
        }
    }
}

/// `id objc_getAssociatedObject(id object, const void *key)`
pub extern "C" fn shim_objc_get_associated_object(object: u64, key: u64) -> u64 {
    if object == 0 {
        return 0;
    }
    let guard = assoc_map();
    guard
        .as_ref()
        .and_then(|m| m.get(&object))
        .and_then(|obj_map| obj_map.get(&key))
        .map(|(val, _policy)| *val)
        .unwrap_or(0)
}

/// `void objc_removeAssociatedObjects(id object)`
pub extern "C" fn shim_objc_remove_associated_objects(object: u64) {
    if object == 0 {
        return;
    }
    let mut guard = assoc_map();
    if let Some(map) = guard.as_mut() {
        if let Some(obj_map) = map.remove(&object) {
            for (_key, (val, policy)) in obj_map {
                let is_retain = policy == OBJC_ASSOCIATION_RETAIN_NONATOMIC
                    || policy == OBJC_ASSOCIATION_RETAIN;
                if is_retain && val != 0 {
                    shim_objc_release(val);
                }
            }
        }
    }
}

// ── Weak references ────────────────────────────────────────────────

/// Weak reference table: maps a weak variable location → the object it points to.
static WEAK_TABLE: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

fn weak_map() -> std::sync::MutexGuard<'static, Option<HashMap<u64, u64>>> {
    WEAK_TABLE.lock().unwrap_or_else(|e| e.into_inner())
}

/// `id objc_initWeak(id *location, id newValue)`
pub extern "C" fn shim_objc_init_weak(location: u64, new_value: u64) -> u64 {
    if location == 0 {
        return 0;
    }
    unsafe {
        *(location as *mut u64) = new_value;
    }
    let mut guard = weak_map();
    let map = guard.get_or_insert_with(HashMap::new);
    if new_value != 0 {
        map.insert(location, new_value);
    }
    new_value
}

/// `void objc_destroyWeak(id *location)`
pub extern "C" fn shim_objc_destroy_weak(location: u64) {
    if location == 0 {
        return;
    }
    unsafe {
        *(location as *mut u64) = 0;
    }
    let mut guard = weak_map();
    if let Some(map) = guard.as_mut() {
        map.remove(&location);
    }
}

/// `id objc_loadWeakRetained(id *location)` — load and retain the weak reference.
pub extern "C" fn shim_objc_load_weak_retained(location: u64) -> u64 {
    if location == 0 {
        return 0;
    }
    let val = unsafe { *(location as *const u64) };
    if val != 0 {
        shim_objc_retain(val);
    }
    val
}

/// `id objc_storeWeak(id *location, id newValue)` — store a new weak reference.
pub extern "C" fn shim_objc_store_weak(location: u64, new_value: u64) -> u64 {
    if location == 0 {
        return 0;
    }
    unsafe {
        *(location as *mut u64) = new_value;
    }
    let mut guard = weak_map();
    let map = guard.get_or_insert_with(HashMap::new);
    if new_value != 0 {
        map.insert(location, new_value);
    } else {
        map.remove(&location);
    }
    new_value
}

/// `id objc_loadWeak(id *location)` — load a weak reference (no retain).
pub extern "C" fn shim_objc_load_weak(location: u64) -> u64 {
    if location == 0 {
        return 0;
    }
    unsafe { *(location as *const u64) }
}

/// `void objc_copyWeak(id *dst, id *src)` — copy a weak variable.
pub extern "C" fn shim_objc_copy_weak(dst: u64, src: u64) {
    if dst == 0 || src == 0 {
        return;
    }
    let val = unsafe { *(src as *const u64) };
    unsafe {
        *(dst as *mut u64) = val;
    }
    let mut guard = weak_map();
    let map = guard.get_or_insert_with(HashMap::new);
    if val != 0 {
        map.insert(dst, val);
    }
}

/// `void objc_moveWeak(id *dst, id *src)` — move a weak variable (destroys src).
pub extern "C" fn shim_objc_move_weak(dst: u64, src: u64) {
    if dst == 0 || src == 0 {
        return;
    }
    let val = unsafe { *(src as *const u64) };
    unsafe {
        *(dst as *mut u64) = val;
        *(src as *mut u64) = 0;
    }
    let mut guard = weak_map();
    let map = guard.get_or_insert_with(HashMap::new);
    if val != 0 {
        map.insert(dst, val);
    }
    map.remove(&src);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── @synchronized ──────────────────────────────────────────────

    #[test]
    fn sync_enter_null() {
        assert_eq!(shim_objc_sync_enter(0), 0);
    }

    #[test]
    fn sync_exit_null() {
        assert_eq!(shim_objc_sync_exit(0), 0);
    }

    #[test]
    fn sync_enter_exit_cycle() {
        let obj = 0x1234u64;
        assert_eq!(shim_objc_sync_enter(obj), 0);
        assert_eq!(shim_objc_sync_exit(obj), 0);
    }

    #[test]
    fn sync_enter_twice_same_object() {
        let obj = 0xAAAAu64;
        assert_eq!(shim_objc_sync_enter(obj), 0);
        assert_eq!(shim_objc_sync_enter(obj), 0);
        assert_eq!(shim_objc_sync_exit(obj), 0);
        assert_eq!(shim_objc_sync_exit(obj), 0);
    }

    // ── Associated objects ─────────────────────────────────────────

    #[test]
    fn associated_set_get() {
        let obj = 0x2000u64;
        let key = 0x3000u64;
        let val = 0x4000u64;
        shim_objc_set_associated_object(obj, key, val, OBJC_ASSOCIATION_ASSIGN);
        assert_eq!(shim_objc_get_associated_object(obj, key), val);
    }

    #[test]
    fn associated_get_unset() {
        let obj = 0x5000u64;
        let key = 0x6000u64;
        assert_eq!(shim_objc_get_associated_object(obj, key), 0);
    }

    #[test]
    fn associated_remove() {
        let obj = 0x7000u64;
        let key = 0x8000u64;
        shim_objc_set_associated_object(obj, key, 0x9000, OBJC_ASSOCIATION_ASSIGN);
        shim_objc_remove_associated_objects(obj);
        assert_eq!(shim_objc_get_associated_object(obj, key), 0);
    }

    #[test]
    fn associated_null_obj() {
        shim_objc_set_associated_object(0, 0x100, 0x200, OBJC_ASSOCIATION_ASSIGN);
        assert_eq!(shim_objc_get_associated_object(0, 0x100), 0);
    }

    // ── Weak references ────────────────────────────────────────────

    #[test]
    fn init_weak_null_location() {
        assert_eq!(shim_objc_init_weak(0, 0x5678), 0);
    }

    #[test]
    fn init_weak_valid() {
        let mut location: u64 = 0;
        let addr = &mut location as *mut u64 as u64;
        let ret = shim_objc_init_weak(addr, 0x5678);
        assert_eq!(ret, 0x5678);
        assert_eq!(shim_objc_load_weak(addr), 0x5678);
    }

    #[test]
    fn destroy_weak_null() {
        shim_objc_destroy_weak(0);
    }

    #[test]
    fn store_weak_updates() {
        let mut location: u64 = 0;
        let addr = &mut location as *mut u64 as u64;
        shim_objc_init_weak(addr, 0xAAAA);
        let ret = shim_objc_store_weak(addr, 0xBBBB);
        assert_eq!(ret, 0xBBBB);
        assert_eq!(shim_objc_load_weak(addr), 0xBBBB);
    }

    #[test]
    fn copy_weak_copies_value() {
        let mut src: u64 = 0;
        let mut dst: u64 = 0;
        let src_addr = &mut src as *mut u64 as u64;
        let dst_addr = &mut dst as *mut u64 as u64;
        shim_objc_init_weak(src_addr, 0xCCCC);
        shim_objc_copy_weak(dst_addr, src_addr);
        assert_eq!(shim_objc_load_weak(dst_addr), 0xCCCC);
        // Source still intact after copy
        assert_eq!(shim_objc_load_weak(src_addr), 0xCCCC);
    }

    #[test]
    fn move_weak_transfers() {
        let mut src: u64 = 0;
        let mut dst: u64 = 0;
        let src_addr = &mut src as *mut u64 as u64;
        let dst_addr = &mut dst as *mut u64 as u64;
        shim_objc_init_weak(src_addr, 0xDDDD);
        shim_objc_move_weak(dst_addr, src_addr);
        assert_eq!(shim_objc_load_weak(dst_addr), 0xDDDD);
        // Source cleared after move
        assert_eq!(shim_objc_load_weak(src_addr), 0);
    }
}
