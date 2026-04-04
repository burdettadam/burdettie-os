//! Apple SystemConfiguration.framework stubs.
//!
//! Provides SCNetworkReachability, SCDynamicStore, and SCPreferences
//! shims.  The key insight: stub with "always reachable" so Foundation
//! networking layers don't block waiting for real reachability callbacks.
//!
//! Handle-based approach: opaque non-zero u64 handles from a global
//! `AtomicU64` counter.  No real state is tracked behind them.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::table::ShimTable;

/// Monotonic handle generator — every Create call returns a unique non-zero value.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// `kSCNetworkReachabilityFlagsReachable` = 1 << 1 = 2.
static K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE: u32 = 2;

pub struct SystemConfigurationStubs;

impl SystemConfigurationStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── SCNetworkReachability ───────────────────────────────
        register_shims!(table,
            ("_SCNetworkReachabilityCreateWithName", shim_sc_reachability_create_with_name as *const () as u64, "SCNetworkReachabilityCreateWithName (stub → handle)"),
            ("_SCNetworkReachabilityCreateWithAddress", shim_sc_reachability_create_with_address as *const () as u64, "SCNetworkReachabilityCreateWithAddress (stub → handle)"),
            ("_SCNetworkReachabilityGetFlags", shim_sc_reachability_get_flags as *const () as u64, "SCNetworkReachabilityGetFlags (stub → reachable)"),
            ("_SCNetworkReachabilitySetCallback", shim_sc_reachability_set_callback as *const () as u64, "SCNetworkReachabilitySetCallback (stub → TRUE)"),
            ("_SCNetworkReachabilityScheduleWithRunLoop", shim_sc_reachability_schedule_with_run_loop as *const () as u64, "SCNetworkReachabilityScheduleWithRunLoop (stub → TRUE)"),
            ("_SCNetworkReachabilityUnscheduleFromRunLoop", shim_sc_reachability_unschedule_from_run_loop as *const () as u64, "SCNetworkReachabilityUnscheduleFromRunLoop (stub → TRUE)"),
            ("_SCNetworkReachabilitySetDispatchQueue", shim_sc_reachability_set_dispatch_queue as *const () as u64, "SCNetworkReachabilitySetDispatchQueue (stub → TRUE)")
        );

        // ── SCDynamicStore ──────────────────────────────────────
        register_shims!(table,
            ("_SCDynamicStoreCreate", shim_sc_dynamic_store_create as *const () as u64, "SCDynamicStoreCreate (stub → handle)"),
            ("_SCDynamicStoreCopyValue", shim_sc_dynamic_store_copy_value as *const () as u64, "SCDynamicStoreCopyValue (stub → NULL)"),
            ("_SCDynamicStoreCopyMultiple", shim_sc_dynamic_store_copy_multiple as *const () as u64, "SCDynamicStoreCopyMultiple (stub → NULL)"),
            ("_SCDynamicStoreSetValue", shim_sc_dynamic_store_set_value as *const () as u64, "SCDynamicStoreSetValue (stub → TRUE)"),
            ("_SCDynamicStoreSetNotificationKeys", shim_sc_dynamic_store_set_notification_keys as *const () as u64, "SCDynamicStoreSetNotificationKeys (stub → TRUE)"),
            ("_SCDynamicStoreSetDispatchQueue", shim_sc_dynamic_store_set_dispatch_queue as *const () as u64, "SCDynamicStoreSetDispatchQueue (stub → TRUE)")
        );

        // ── SCPreferences ───────────────────────────────────────
        register_shims!(table,
            ("_SCPreferencesCreate", shim_sc_preferences_create as *const () as u64, "SCPreferencesCreate (stub → handle)"),
            ("_SCPreferencesGetValue", shim_sc_preferences_get_value as *const () as u64, "SCPreferencesGetValue (stub → NULL)"),
            ("_SCPreferencesSetValue", shim_sc_preferences_set_value as *const () as u64, "SCPreferencesSetValue (stub → TRUE)"),
            ("_SCPreferencesCommitChanges", shim_sc_preferences_commit_changes as *const () as u64, "SCPreferencesCommitChanges (stub → TRUE)"),
            ("_SCPreferencesApplyChanges", shim_sc_preferences_apply_changes as *const () as u64, "SCPreferencesApplyChanges (stub → TRUE)")
        );

        // ── SCNetworkConfiguration ──────────────────────────────
        register_shims!(table,
            ("_SCNetworkInterfaceCopyAll", shim_sc_network_interface_copy_all as *const () as u64, "SCNetworkInterfaceCopyAll (stub → NULL)"),
            ("_SCNetworkServiceCopyAll", shim_sc_network_service_copy_all as *const () as u64, "SCNetworkServiceCopyAll (stub → NULL)")
        );

        // ── Data symbols ────────────────────────────────────────
        register_shims!(table,
            ("_kSCNetworkReachabilityFlagsReachable", &K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE as *const u32 as u64, "kSCNetworkReachabilityFlagsReachable data symbol")
        );

        log::debug!("SystemConfigurationStubs: registered 21 stubs");
    }
}

// ── SCNetworkReachability ───────────────────────────────────────

/// `SCNetworkReachabilityCreateWithName(alloc, name) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_name(_alloc: u64, name: u64) -> u64 {
    let hostname = if name != 0 {
        unsafe { std::ffi::CStr::from_ptr(name as *const std::ffi::c_char) }
            .to_str()
            .unwrap_or("<invalid>")
    } else {
        "<null>"
    };
    let handle = next_handle();
    log::trace!("SCNetworkReachabilityCreateWithName: host={hostname} → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityCreateWithAddress(alloc, addr) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_address(_alloc: u64, _addr: u64) -> u64 {
    let handle = next_handle();
    log::trace!("SCNetworkReachabilityCreateWithAddress → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityGetFlags(target, &flags) -> Boolean`
///
/// Always writes `kSCNetworkReachabilityFlagsReachable` (bit 1 = 2) and
/// returns TRUE (1).  This makes all reachability checks think the
/// network is available.
pub extern "C" fn shim_sc_reachability_get_flags(_target: u64, flags: u64) -> u8 {
    if flags != 0 {
        unsafe {
            std::ptr::write(flags as *mut u32, 2); // kSCNetworkReachabilityFlagsReachable
        }
    }
    1 // TRUE
}

/// `SCNetworkReachabilitySetCallback(target, callback, context) -> Boolean`
pub extern "C" fn shim_sc_reachability_set_callback(_target: u64, _callback: u64, _context: u64) -> u8 {
    1 // TRUE — no-op, we never fire callbacks
}

/// `SCNetworkReachabilityScheduleWithRunLoop(target, rl, mode) -> Boolean`
pub extern "C" fn shim_sc_reachability_schedule_with_run_loop(_target: u64, _rl: u64, _mode: u64) -> u8 {
    1 // TRUE
}

/// `SCNetworkReachabilityUnscheduleFromRunLoop(target, rl, mode) -> Boolean`
pub extern "C" fn shim_sc_reachability_unschedule_from_run_loop(_target: u64, _rl: u64, _mode: u64) -> u8 {
    1 // TRUE
}

/// `SCNetworkReachabilitySetDispatchQueue(target, queue) -> Boolean`
pub extern "C" fn shim_sc_reachability_set_dispatch_queue(_target: u64, _queue: u64) -> u8 {
    1 // TRUE
}

// ── SCDynamicStore ──────────────────────────────────────────────

/// `SCDynamicStoreCreate(alloc, name, callback, ctx) -> SCDynamicStoreRef`
pub extern "C" fn shim_sc_dynamic_store_create(_alloc: u64, _name: u64, _callback: u64, _ctx: u64) -> u64 {
    next_handle()
}

/// `SCDynamicStoreCopyValue(store, key) -> CFPropertyListRef`
pub extern "C" fn shim_sc_dynamic_store_copy_value(_store: u64, _key: u64) -> u64 {
    0 // NULL — key not found
}

/// `SCDynamicStoreCopyMultiple(store, keys, patterns) -> CFDictionaryRef`
pub extern "C" fn shim_sc_dynamic_store_copy_multiple(_store: u64, _keys: u64, _patterns: u64) -> u64 {
    0 // NULL
}

/// `SCDynamicStoreSetValue(store, key, value) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_value(_store: u64, _key: u64, _value: u64) -> u8 {
    1 // TRUE
}

/// `SCDynamicStoreSetNotificationKeys(store, keys, patterns) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_notification_keys(_store: u64, _keys: u64, _patterns: u64) -> u8 {
    1 // TRUE
}

/// `SCDynamicStoreSetDispatchQueue(store, queue) -> Boolean`
pub extern "C" fn shim_sc_dynamic_store_set_dispatch_queue(_store: u64, _queue: u64) -> u8 {
    1 // TRUE
}

// ── SCPreferences ───────────────────────────────────────────────

/// `SCPreferencesCreate(alloc, name, prefs_id) -> SCPreferencesRef`
pub extern "C" fn shim_sc_preferences_create(_alloc: u64, _name: u64, _prefs_id: u64) -> u64 {
    next_handle()
}

/// `SCPreferencesGetValue(prefs, key) -> CFPropertyListRef`
pub extern "C" fn shim_sc_preferences_get_value(_prefs: u64, _key: u64) -> u64 {
    0 // NULL
}

/// `SCPreferencesSetValue(prefs, key, value) -> Boolean`
pub extern "C" fn shim_sc_preferences_set_value(_prefs: u64, _key: u64, _value: u64) -> u8 {
    1 // TRUE
}

/// `SCPreferencesCommitChanges(prefs) -> Boolean`
pub extern "C" fn shim_sc_preferences_commit_changes(_prefs: u64) -> u8 {
    1 // TRUE
}

/// `SCPreferencesApplyChanges(prefs) -> Boolean`
pub extern "C" fn shim_sc_preferences_apply_changes(_prefs: u64) -> u8 {
    1 // TRUE
}

// ── SCNetworkConfiguration ──────────────────────────────────────

/// `SCNetworkInterfaceCopyAll() -> CFArrayRef`
pub extern "C" fn shim_sc_network_interface_copy_all() -> u64 {
    0 // NULL — empty
}

/// `SCNetworkServiceCopyAll(prefs) -> CFArrayRef`
pub extern "C" fn shim_sc_network_service_copy_all(_prefs: u64) -> u64 {
    0 // NULL
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reachability_create_returns_handle() {
        let h1 = shim_sc_reachability_create_with_name(0, 0);
        assert_ne!(h1, 0);
        let h2 = shim_sc_reachability_create_with_address(0, 0);
        assert_ne!(h2, 0);
        assert_ne!(h1, h2, "each call should return a unique handle");
    }

    #[test]
    fn test_reachability_flags_always_reachable() {
        let mut flags: u32 = 0;
        let ret = shim_sc_reachability_get_flags(1, &mut flags as *mut u32 as u64);
        assert_eq!(ret, 1, "GetFlags should return TRUE");
        assert_ne!(flags & (1 << 1), 0, "kSCNetworkReachabilityFlagsReachable bit should be set");
    }

    #[test]
    fn test_dynamic_store_copy_returns_null() {
        let store = shim_sc_dynamic_store_create(0, 0, 0, 0);
        assert_ne!(store, 0);
        let val = shim_sc_dynamic_store_copy_value(store, 0);
        assert_eq!(val, 0, "CopyValue should return NULL");
    }

    #[test]
    fn test_preferences_create_returns_handle() {
        let h = shim_sc_preferences_create(0, 0, 0);
        assert_ne!(h, 0, "SCPreferencesCreate should return a non-zero handle");
    }

    #[test]
    fn test_reachability_schedule_succeeds() {
        let ret = shim_sc_reachability_schedule_with_run_loop(1, 0, 0);
        assert_eq!(ret, 1, "ScheduleWithRunLoop should return TRUE");
    }

    #[test]
    fn test_network_interface_copy_all_null() {
        let ret = shim_sc_network_interface_copy_all();
        assert_eq!(ret, 0, "SCNetworkInterfaceCopyAll should return NULL");
    }
}
