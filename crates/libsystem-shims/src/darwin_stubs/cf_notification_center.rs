//! CFNotificationCenter — self-contained C-level notification dispatch.
//!
//! Provides CFNotificationCenterGetLocalCenter, CFNotificationCenterGetDistributedCenter,
//! CFNotificationCenterAddObserver, CFNotificationCenterRemoveObserver,
//! CFNotificationCenterPostNotification, and CFNotificationCenterPostNotificationWithOptions.
//!
//! Observers are stored per-center with callback function pointers. On post,
//! matching observers are invoked synchronously. Name matching uses CFString
//! extraction (ISA-tagged objects or C string fallback). A nil name matches all.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::table::ShimTable;

// ── Singleton center handles ────────────────────────────────────

/// Opaque handle for the local notification center.
static LOCAL_CENTER: AtomicU64 = AtomicU64::new(0xCF_A0C0_0001);
/// Opaque handle for the distributed notification center.
static DISTRIBUTED_CENTER: AtomicU64 = AtomicU64::new(0xCF_A0C0_0002);
/// Opaque handle for the Darwin notification center.
static DARWIN_CENTER: AtomicU64 = AtomicU64::new(0xCF_A0C0_0003);

// ── Observer storage ────────────────────────────────────────────

struct CfObserver {
    /// Center handle this observer belongs to.
    center: u64,
    /// Observer context pointer (void *observer in Apple API).
    observer: u64,
    /// Callback function pointer (CFNotificationCallback).
    callback: u64,
    /// Notification name (None = match all).
    name: Option<String>,
    /// Object filter (0 = match all).
    object: u64,
}

fn observers() -> &'static Mutex<Vec<CfObserver>> {
    static STORE: OnceLock<Mutex<Vec<CfObserver>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

// ── CFString extraction ─────────────────────────────────────────

/// Extract a string from a CFStringRef or raw C string pointer.
fn extract_cfstring(ptr: u64) -> Option<String> {
    if ptr == 0 {
        return None;
    }
    let p = ptr as *const u8;
    unsafe {
        let first_word = std::ptr::read_unaligned(p as *const u64);
        // CFString-like object: ISA is small (1..16)
        if first_word > 0 && first_word <= 0x10 {
            let len = std::ptr::read_unaligned(p.add(8) as *const u64) as usize;
            let chars_ptr = std::ptr::read_unaligned(p.add(16) as *const u64) as *const u8;
            if len > 0 && len < 4096 && !chars_ptr.is_null() {
                let slice = std::slice::from_raw_parts(chars_ptr, len);
                return std::str::from_utf8(slice).ok().map(|s| s.to_string());
            }
        }
        // Might be an NSConstantString (ISA at +0, then c_str at +8 or +16)
        // Or a plain C string — try CStr
        let mut slen = 0usize;
        while slen < 256 {
            let b = *p.add(slen);
            if b == 0 {
                break;
            }
            if b > 127 {
                return None;
            }
            slen += 1;
        }
        if slen > 0 && slen < 256 {
            let slice = std::slice::from_raw_parts(p, slen);
            std::str::from_utf8(slice).ok().map(|s| s.to_string())
        } else {
            None
        }
    }
}

// ── Shim functions ──────────────────────────────────────────────

/// CFNotificationCenterRef CFNotificationCenterGetLocalCenter(void)
extern "C" fn shim_cf_notification_center_get_local() -> u64 {
    LOCAL_CENTER.load(Ordering::Relaxed)
}

/// CFNotificationCenterRef CFNotificationCenterGetDistributedCenter(void)
extern "C" fn shim_cf_notification_center_get_distributed() -> u64 {
    DISTRIBUTED_CENTER.load(Ordering::Relaxed)
}

/// CFNotificationCenterRef CFNotificationCenterGetDarwinNotifyCenter(void)
extern "C" fn shim_cf_notification_center_get_darwin() -> u64 {
    DARWIN_CENTER.load(Ordering::Relaxed)
}

/// void CFNotificationCenterAddObserver(
///     CFNotificationCenterRef center, const void *observer,
///     CFNotificationCallback callBack, CFStringRef name,
///     const void *object, CFNotificationSuspensionBehavior suspensionBehavior)
extern "C" fn shim_cf_notification_center_add_observer(
    center: u64,
    observer: u64,
    callback: u64,
    name: u64,
    object: u64,
    _suspension_behavior: u64,
) {
    if callback == 0 {
        return;
    }
    let name_str = extract_cfstring(name);
    log::trace!(
        "CFNotificationCenterAddObserver: center={:#x} name={:?}",
        center,
        name_str
    );
    let mut obs = observers().lock().unwrap();
    obs.push(CfObserver {
        center,
        observer,
        callback,
        name: name_str,
        object,
    });
}

/// void CFNotificationCenterRemoveObserver(
///     CFNotificationCenterRef center, const void *observer,
///     CFNotificationName name, const void *object)
extern "C" fn shim_cf_notification_center_remove_observer(
    center: u64,
    observer: u64,
    name: u64,
    _object: u64,
) {
    let name_str = extract_cfstring(name);
    log::trace!(
        "CFNotificationCenterRemoveObserver: center={:#x} observer={:#x} name={:?}",
        center,
        observer,
        name_str
    );
    let mut obs = observers().lock().unwrap();
    obs.retain(|o| {
        if o.center != center || o.observer != observer {
            return true;
        }
        match (&o.name, &name_str) {
            (_, None) => false, // nil name = remove all for this observer
            (Some(a), Some(b)) => a != b,
            (None, Some(_)) => true,
        }
    });
}

/// void CFNotificationCenterRemoveEveryObserver(
///     CFNotificationCenterRef center, const void *observer)
extern "C" fn shim_cf_notification_center_remove_every_observer(center: u64, observer: u64) {
    log::trace!(
        "CFNotificationCenterRemoveEveryObserver: center={:#x} observer={:#x}",
        center,
        observer
    );
    let mut obs = observers().lock().unwrap();
    obs.retain(|o| o.center != center || o.observer != observer);
}

/// void CFNotificationCenterPostNotification(
///     CFNotificationCenterRef center, CFNotificationName name,
///     const void *object, CFDictionaryRef userInfo,
///     Boolean deliverImmediately)
extern "C" fn shim_cf_notification_center_post_notification(
    center: u64,
    name: u64,
    object: u64,
    user_info: u64,
    _deliver_immediately: u8,
) {
    let name_str = extract_cfstring(name);
    log::trace!(
        "CFNotificationCenterPostNotification: center={:#x} name={:?}",
        center,
        name_str
    );

    // Collect matching callbacks (release lock before calling)
    let callbacks: Vec<(u64, u64)> = {
        let obs = observers().lock().unwrap();
        obs.iter()
            .filter(|o| {
                if o.center != center {
                    return false;
                }
                match (&o.name, &name_str) {
                    (None, _) => true,  // observer watches all
                    (_, None) => false, // shouldn't happen (posting nil name)
                    (Some(a), Some(b)) => a == b,
                }
            })
            .map(|o| (o.callback, o.observer))
            .collect()
    };

    // Dispatch to each callback:
    // void (*CFNotificationCallback)(CFNotificationCenterRef center,
    //     void *observer, CFNotificationName name,
    //     const void *object, CFDictionaryRef userInfo)
    for (cb, obs_ptr) in callbacks {
        unsafe {
            let f: extern "C" fn(u64, u64, u64, u64, u64) = std::mem::transmute(cb as usize);
            f(center, obs_ptr, name, object, user_info);
        }
    }
}

/// void CFNotificationCenterPostNotificationWithOptions(
///     CFNotificationCenterRef center, CFNotificationName name,
///     const void *object, CFDictionaryRef userInfo,
///     CFOptionFlags options)
extern "C" fn shim_cf_notification_center_post_with_options(
    center: u64,
    name: u64,
    object: u64,
    user_info: u64,
    _options: u64,
) {
    // Options control coalescing / delivery timing; we just dispatch immediately
    shim_cf_notification_center_post_notification(center, name, object, user_info, 1);
}

// ── Install ─────────────────────────────────────────────────────

pub struct CfNotificationCenterStubs;

impl CfNotificationCenterStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_CFNotificationCenterGetLocalCenter",
                shim_cf_notification_center_get_local as *const () as u64,
                "CFNotificationCenterGetLocalCenter → handle"
            ),
            (
                "_CFNotificationCenterGetDistributedCenter",
                shim_cf_notification_center_get_distributed as *const () as u64,
                "CFNotificationCenterGetDistributedCenter → handle"
            ),
            (
                "_CFNotificationCenterGetDarwinNotifyCenter",
                shim_cf_notification_center_get_darwin as *const () as u64,
                "CFNotificationCenterGetDarwinNotifyCenter → handle"
            ),
            (
                "_CFNotificationCenterAddObserver",
                shim_cf_notification_center_add_observer as *const () as u64,
                "CFNotificationCenterAddObserver → add callback"
            ),
            (
                "_CFNotificationCenterRemoveObserver",
                shim_cf_notification_center_remove_observer as *const () as u64,
                "CFNotificationCenterRemoveObserver → remove"
            ),
            (
                "_CFNotificationCenterRemoveEveryObserver",
                shim_cf_notification_center_remove_every_observer as *const () as u64,
                "CFNotificationCenterRemoveEveryObserver → remove all"
            ),
            (
                "_CFNotificationCenterPostNotification",
                shim_cf_notification_center_post_notification as *const () as u64,
                "CFNotificationCenterPostNotification → dispatch"
            ),
            (
                "_CFNotificationCenterPostNotificationWithOptions",
                shim_cf_notification_center_post_with_options as *const () as u64,
                "CFNotificationCenterPostNotificationWithOptions → dispatch"
            )
        );

        log::debug!("CfNotificationCenterStubs: registered 8 shims");
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    /// Serialize tests that share global observer state.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Counter incremented by test callback.
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    extern "C" fn test_callback(
        _center: u64,
        _observer: u64,
        _name: u64,
        _object: u64,
        _user_info: u64,
    ) {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    fn reset() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap();
        TEST_COUNTER.store(0, Ordering::SeqCst);
        observers().lock().unwrap().clear();
        guard
    }

    #[test]
    fn get_local_center_returns_stable_handle() {
        let a = shim_cf_notification_center_get_local();
        let b = shim_cf_notification_center_get_local();
        assert_ne!(a, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn get_distributed_center_returns_stable_handle() {
        let a = shim_cf_notification_center_get_distributed();
        let b = shim_cf_notification_center_get_distributed();
        assert_ne!(a, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn local_and_distributed_are_different() {
        let local = shim_cf_notification_center_get_local();
        let dist = shim_cf_notification_center_get_distributed();
        assert_ne!(local, dist);
    }

    #[test]
    fn add_and_post_invokes_callback() {
        let _g = reset();
        let center = shim_cf_notification_center_get_local();
        // Use a C-string-like name for simplicity in tests
        let name = c"TestNotification".as_ptr() as u64;

        shim_cf_notification_center_add_observer(
            center,
            0x42,
            test_callback as *const () as u64,
            name,
            0,
            0,
        );

        shim_cf_notification_center_post_notification(center, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1);

        // Post again
        shim_cf_notification_center_post_notification(center, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn remove_observer_stops_delivery() {
        let _g = reset();
        let center = shim_cf_notification_center_get_local();
        let name = c"RemoveTest".as_ptr() as u64;

        shim_cf_notification_center_add_observer(
            center,
            0x43,
            test_callback as *const () as u64,
            name,
            0,
            0,
        );

        shim_cf_notification_center_post_notification(center, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1);

        // Remove with matching name
        shim_cf_notification_center_remove_observer(center, 0x43, name, 0);

        shim_cf_notification_center_post_notification(center, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1); // still 1
    }

    #[test]
    fn remove_every_observer_clears_all_for_observer() {
        let _g = reset();
        let center = shim_cf_notification_center_get_local();
        let name_a = c"EveryA".as_ptr() as u64;
        let name_b = c"EveryB".as_ptr() as u64;

        shim_cf_notification_center_add_observer(
            center,
            0x44,
            test_callback as *const () as u64,
            name_a,
            0,
            0,
        );
        shim_cf_notification_center_add_observer(
            center,
            0x44,
            test_callback as *const () as u64,
            name_b,
            0,
            0,
        );

        shim_cf_notification_center_remove_every_observer(center, 0x44);

        shim_cf_notification_center_post_notification(center, name_a, 0, 0, 0);
        shim_cf_notification_center_post_notification(center, name_b, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn nil_name_observer_matches_all_posts() {
        let _g = reset();
        let center = shim_cf_notification_center_get_local();

        // Add observer with nil name (matches all)
        shim_cf_notification_center_add_observer(
            center,
            0x45,
            test_callback as *const () as u64,
            0,
            0,
            0,
        );

        let name_x = c"AnyName".as_ptr() as u64;
        shim_cf_notification_center_post_notification(center, name_x, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1);

        let name_y = c"Other".as_ptr() as u64;
        shim_cf_notification_center_post_notification(center, name_y, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 2);

        // Cleanup
        shim_cf_notification_center_remove_every_observer(center, 0x45);
    }

    #[test]
    fn cross_center_isolation() {
        let _g = reset();
        let local = shim_cf_notification_center_get_local();
        let dist = shim_cf_notification_center_get_distributed();
        let name = c"CrossCenter".as_ptr() as u64;

        // Add observer on local center
        shim_cf_notification_center_add_observer(
            local,
            0x46,
            test_callback as *const () as u64,
            name,
            0,
            0,
        );

        // Post on distributed center — should NOT invoke callback
        shim_cf_notification_center_post_notification(dist, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 0);

        // Post on local center — should invoke
        shim_cf_notification_center_post_notification(local, name, 0, 0, 0);
        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1);

        // Cleanup
        shim_cf_notification_center_remove_every_observer(local, 0x46);
    }

    #[test]
    fn null_callback_ignored() {
        let _g = reset();
        let center = shim_cf_notification_center_get_local();
        let name = c"NullCb".as_ptr() as u64;

        // Adding with callback=0 should be silently ignored
        shim_cf_notification_center_add_observer(center, 0x47, 0, name, 0, 0);

        let obs = observers().lock().unwrap();
        assert!(obs.iter().all(|o| o.observer != 0x47));
    }
}
