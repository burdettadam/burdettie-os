//! `_keymgr_*` stubs for C++ exception-handling frame registration.
//!
//! The keymgr API provides process-wide and per-thread key-value storage
//! that libunwind uses to register and look up EH frame information.
//! Locking semantics are advisory — we lock/unlock within each call
//! rather than holding guards across FFI boundaries.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Process-wide key-value store (shared across all threads).
fn process_store() -> &'static Mutex<HashMap<u32, u64>> {
    static INSTANCE: OnceLock<Mutex<HashMap<u32, u64>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// Per-thread key-value store.
    static THREAD_STORE: std::cell::RefCell<HashMap<u32, u64>> =
        std::cell::RefCell::new(HashMap::new());
}

/// `_keymgr_global[3]` — legacy data symbols (initialized to zero).
static KEYMGR_GLOBAL: [u64; 3] = [0; 3];

pub struct KeymgrStubs;

impl KeymgrStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("__keymgr_get_and_lock_processwide_ptr", shim_keymgr_get_and_lock_processwide_ptr as u64, "keymgr get_and_lock_processwide_ptr"),
            ("__keymgr_set_and_unlock_processwide_ptr", shim_keymgr_set_and_unlock_processwide_ptr as u64, "keymgr set_and_unlock_processwide_ptr"),
            ("__keymgr_get_processwide_ptr", shim_keymgr_get_processwide_ptr as u64, "keymgr get_processwide_ptr"),
            ("__keymgr_get_per_thread_data", shim_keymgr_get_per_thread_data as u64, "keymgr get_per_thread_data"),
            ("__keymgr_set_per_thread_data", shim_keymgr_set_per_thread_data as u64, "keymgr set_per_thread_data"),
            ("__keymgr_global", KEYMGR_GLOBAL.as_ptr() as u64, "keymgr_global data symbol"),
        );
        log::debug!("KeymgrStubs: registered 6 stubs");
    }
}

/// `void *_keymgr_get_and_lock_processwide_ptr(uint32_t key)`
///
/// Lock, read value, unlock, return. Returns 0 if key not set.
pub extern "C" fn shim_keymgr_get_and_lock_processwide_ptr(key: u32) -> u64 {
    let store = process_store().lock().unwrap_or_else(|e| e.into_inner());
    store.get(&key).copied().unwrap_or(0)
}

/// `int _keymgr_set_and_unlock_processwide_ptr(uint32_t key, void *ptr)`
///
/// Lock, set value, unlock. Returns 0 (success).
pub extern "C" fn shim_keymgr_set_and_unlock_processwide_ptr(key: u32, ptr: u64) -> i32 {
    let mut store = process_store().lock().unwrap_or_else(|e| e.into_inner());
    store.insert(key, ptr);
    0
}

/// `void *_keymgr_get_processwide_ptr(uint32_t key)`
///
/// Simple (non-locking-semantic) read. Lock, read, unlock, return.
pub extern "C" fn shim_keymgr_get_processwide_ptr(key: u32) -> u64 {
    let store = process_store().lock().unwrap_or_else(|e| e.into_inner());
    store.get(&key).copied().unwrap_or(0)
}

/// `void *_keymgr_get_per_thread_data(uint32_t key)`
///
/// Read from per-thread store. Returns 0 if not set.
pub extern "C" fn shim_keymgr_get_per_thread_data(key: u32) -> u64 {
    THREAD_STORE.with(|s| s.borrow().get(&key).copied().unwrap_or(0))
}

/// `int _keymgr_set_per_thread_data(uint32_t key, void *ptr)`
///
/// Write to per-thread store. Returns 0 (success).
pub extern "C" fn shim_keymgr_set_per_thread_data(key: u32, ptr: u64) -> i32 {
    THREAD_STORE.with(|s| {
        s.borrow_mut().insert(key, ptr);
    });
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_unset_returns_null() {
        assert_eq!(shim_keymgr_get_and_lock_processwide_ptr(9999), 0);
    }

    #[test]
    fn test_set_and_get_roundtrip() {
        let key = 302; // KEYMGR_GCC3_DW2_OBJ_LIST
        shim_keymgr_set_and_unlock_processwide_ptr(key, 0x1234);
        assert_eq!(shim_keymgr_get_and_lock_processwide_ptr(key), 0x1234);
    }

    #[test]
    fn test_overwrite_updates() {
        let key = 10_001;
        shim_keymgr_set_and_unlock_processwide_ptr(key, 0xAAAA);
        shim_keymgr_set_and_unlock_processwide_ptr(key, 0xBBBB);
        assert_eq!(shim_keymgr_get_and_lock_processwide_ptr(key), 0xBBBB);
    }

    #[test]
    fn test_get_processwide_ptr() {
        let key = 10_002;
        shim_keymgr_set_and_unlock_processwide_ptr(key, 0xCAFE);
        assert_eq!(shim_keymgr_get_processwide_ptr(key), 0xCAFE);
    }

    #[test]
    fn test_per_thread_data_roundtrip() {
        let key = 13; // KEYMGR_EH_GLOBALS_KEY
        shim_keymgr_set_per_thread_data(key, 0xDEAD);
        assert_eq!(shim_keymgr_get_per_thread_data(key), 0xDEAD);
    }

    #[test]
    fn test_per_thread_isolation() {
        let key = 10_003;
        shim_keymgr_set_per_thread_data(key, 0x42);

        let handle = std::thread::spawn(move || {
            // Different thread should see 0, not 0x42.
            shim_keymgr_get_per_thread_data(key)
        });
        assert_eq!(handle.join().unwrap(), 0);
    }

    #[test]
    fn test_keymgr_global_non_null() {
        let addr = KEYMGR_GLOBAL.as_ptr() as u64;
        assert_ne!(addr, 0);
    }
}
