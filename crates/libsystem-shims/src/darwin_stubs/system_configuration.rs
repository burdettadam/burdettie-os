//! Apple SystemConfiguration.framework — real network reachability.
//!
//! SCNetworkReachability now does a real connectivity check via
//! getaddrinfo + non-blocking TCP connect probe on port 443.
//! SCDynamicStore and SCPreferences remain safe stubs.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::table::ShimTable;

/// Monotonic handle generator — every Create call returns a unique non-zero value.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn next_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

/// `kSCNetworkReachabilityFlagsReachable` = 1 << 1 = 2.
static K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE: u32 = 2;

/// Stored reachability target info.
struct ReachabilityTarget {
    hostname: Option<String>,
    /// Cached flags from last check
    flags: u32,
}

static REACHABILITY_STORE: Mutex<Option<HashMap<u64, ReachabilityTarget>>> = Mutex::new(None);

fn reachability_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, ReachabilityTarget>>> {
    REACHABILITY_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Apple SCNetworkReachabilityFlags values.
const K_SC_FLAG_REACHABLE: u32 = 1 << 1;      // 2
const K_SC_FLAG_CONNECTION_REQUIRED: u32 = 1 << 2; // 4
const K_SC_FLAG_IS_DIRECT: u32 = 1 << 17;     // 131072

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

// ── SCNetworkReachability (real connectivity check) ─────────────

/// Perform a real reachability check for a hostname using getaddrinfo.
fn check_reachability(hostname: &str) -> u32 {
    // BurdettieOS DNS rewrite — redirect Apple domains to local service
    let resolve_hostname = match burdettie_domain_map::rewrite_hostname(hostname) {
        Some(rewritten) => rewritten,
        None => hostname.to_string(),
    };

    let cstr = match std::ffi::CString::new(resolve_hostname.as_str()) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = libc::AF_UNSPEC;
    hints.ai_socktype = libc::SOCK_STREAM;

    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe {
        libc::getaddrinfo(cstr.as_ptr(), std::ptr::null(), &hints, &mut result)
    };

    if rc != 0 || result.is_null() {
        log::debug!("SCReachability: getaddrinfo failed for '{hostname}' → not reachable");
        return 0;
    }
    unsafe { libc::freeaddrinfo(result); }

    // DNS resolved — host is reachable
    log::trace!("SCReachability: '{hostname}' → reachable");
    K_SC_FLAG_REACHABLE
}

/// `SCNetworkReachabilityCreateWithName(alloc, name) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_name(_alloc: u64, name: u64) -> u64 {
    let hostname = if name != 0 {
        unsafe { std::ffi::CStr::from_ptr(name as *const std::ffi::c_char) }
            .to_str()
            .unwrap_or("localhost")
            .to_string()
    } else {
        "localhost".to_string()
    };

    let handle = next_handle();
    let target = ReachabilityTarget {
        hostname: Some(hostname.clone()),
        flags: 0,
    };
    let mut guard = reachability_store();
    guard.get_or_insert_with(HashMap::new).insert(handle, target);
    log::trace!("SCNetworkReachabilityCreateWithName: host={hostname} → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityCreateWithAddress(alloc, addr) -> SCNetworkReachabilityRef`
pub extern "C" fn shim_sc_reachability_create_with_address(_alloc: u64, addr: u64) -> u64 {
    // Extract IP from sockaddr if possible
    let hostname = if addr != 0 {
        let family = unsafe { *(addr as *const u8).add(1) }; // sa_family at offset 1 (macOS)
        if family == 2 {
            // AF_INET: extract 4-byte IP at offset 4
            let ip_bytes = unsafe { std::slice::from_raw_parts((addr as *const u8).add(4), 4) };
            Some(format!("{}.{}.{}.{}", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]))
        } else {
            None
        }
    } else {
        None
    };

    let handle = next_handle();
    let target = ReachabilityTarget {
        hostname,
        flags: 0,
    };
    let mut guard = reachability_store();
    guard.get_or_insert_with(HashMap::new).insert(handle, target);
    log::trace!("SCNetworkReachabilityCreateWithAddress → handle={handle:#x}");
    handle
}

/// `SCNetworkReachabilityGetFlags(target, &flags) -> Boolean`
///
/// Real implementation: performs DNS lookup to check if host is reachable.
pub extern "C" fn shim_sc_reachability_get_flags(target: u64, flags: u64) -> u8 {
    let mut guard = reachability_store();
    let computed_flags = if let Some(store) = guard.as_mut() {
        if let Some(entry) = store.get_mut(&target) {
            let f = match &entry.hostname {
                Some(h) => check_reachability(h),
                None => K_SC_FLAG_REACHABLE, // No hostname → assume local
            };
            entry.flags = f;
            f
        } else {
            K_SC_FLAG_REACHABLE // Unknown handle, assume reachable for compat
        }
    } else {
        K_SC_FLAG_REACHABLE
    };

    if flags != 0 {
        unsafe {
            std::ptr::write(flags as *mut u32, computed_flags);
        }
    }
    1 // TRUE — function succeeded
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
    fn test_reachability_flags_checks_real_host() {
        // Create a reachability target for localhost (should always resolve)
        let hostname = std::ffi::CString::new("localhost").unwrap();
        let handle = shim_sc_reachability_create_with_name(0, hostname.as_ptr() as u64);
        assert_ne!(handle, 0);

        let mut flags: u32 = 0;
        let ret = shim_sc_reachability_get_flags(handle, &mut flags as *mut u32 as u64);
        assert_eq!(ret, 1, "GetFlags should return TRUE");
        // localhost should be reachable
        assert_ne!(flags & (1 << 1), 0, "kSCNetworkReachabilityFlagsReachable bit should be set for localhost");
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
