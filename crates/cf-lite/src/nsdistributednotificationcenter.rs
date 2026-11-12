//! NSDistributedNotificationCenter shim: in-process distributed notifications.
//!
//! On macOS, distributed notifications are cross-process via `distnoted` daemon.
//! On Linux (BurdettieOS), we provide an in-process implementation that behaves
//! identically for single-process use cases (which covers all guest binaries).
//!
//! The distributed center has its own observer storage, separate from the
//! local `NSNotificationCenter`.
//!
//! NSDistributedNotificationCenter layout (16 bytes):
//!   offset  0: isa       (u64)
//!   offset  8: reserved  (u64)

use crate::cfstring;
use crate::nsnotificationcenter::make_nsnotification;
use crate::nsobj;
use std::sync::{Mutex, OnceLock};

// ── Suspension behavior constants ──────────────────────────────────

/// NSNotificationSuspensionBehavior values
#[allow(dead_code)]
#[repr(u64)]
pub enum SuspensionBehavior {
    Drop = 1,
    Coalesce = 2,
    Hold = 3,
    DeliverImmediately = 4,
}

/// NSDistributedNotificationOptions
#[allow(dead_code)]
pub const NS_DISTRIBUTED_NOTIFICATION_DELIVER_IMMEDIATELY: u64 = 1 << 0;
#[allow(dead_code)]
pub const NS_DISTRIBUTED_NOTIFICATION_POST_TO_ALL_SESSIONS: u64 = 1 << 1;

// ── Observer storage ───────────────────────────────────────────────

static NEXT_TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

enum ObserverKind {
    Selector { observer: u64, sel_ptr: u64 },
    Block { block: u64 },
}

struct DistObserver {
    #[allow(dead_code)]
    token: u64,
    name: Option<String>,
    kind: ObserverKind,
}

/// Distributed center observers — separate from local center.
static DIST_OBSERVERS: OnceLock<Mutex<Vec<DistObserver>>> = OnceLock::new();

fn dist_observers() -> &'static Mutex<Vec<DistObserver>> {
    DIST_OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Clear all distributed notification observers (for runtime reuse).
pub fn clear_all_distributed_observers() {
    let mut obs = dist_observers().lock().unwrap_or_else(|e| e.into_inner());
    obs.clear();
    NEXT_TOKEN.store(1, std::sync::atomic::Ordering::Relaxed);
}

// ── Notification delivery ──────────────────────────────────────────

fn deliver_to_distributed_observers(name_str: &str, notification: u64) {
    let obs = dist_observers().lock().unwrap();
    for o in obs.iter() {
        if let Some(ref n) = o.name {
            if n != name_str {
                continue;
            }
        }
        match &o.kind {
            ObserverKind::Selector { observer, sel_ptr } => {
                extern "C" {
                    fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64;
                }
                let imp = unsafe { objc_msg_lookup_imp(*observer, *sel_ptr) };
                if imp != 0 {
                    let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
                    f(*observer, *sel_ptr, notification);
                }
            }
            ObserverKind::Block { block } => {
                let invoke_ptr = unsafe { *(((*block) as usize + 16) as *const u64) };
                if invoke_ptr != 0 {
                    let invoke: extern "C" fn(u64, u64) =
                        unsafe { std::mem::transmute(invoke_ptr) };
                    invoke(*block, notification);
                }
            }
        }
    }
}

// ── Singleton ──────────────────────────────────────────────────────

static DISTRIBUTED_CENTER: OnceLock<u64> = OnceLock::new();

fn distributed_center_ptr() -> u64 {
    *DISTRIBUTED_CENTER.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nsdistributednotificationcenter_class_addr();
            }
        }
        obj
    })
}

// ── Shim functions ─────────────────────────────────────────────────

/// `+[NSDistributedNotificationCenter defaultCenter]`
pub extern "C" fn shim_nsdistributed_default_center(_cls: u64, _cmd: u64) -> u64 {
    distributed_center_ptr()
}

/// `+[NSDistributedNotificationCenter notificationCenterForType:]`
///
/// macOS supports different center types via NSString constants, but in
/// practice only `NSLocalNotificationCenterType` is used. We return the
/// same singleton for all types.
pub extern "C" fn shim_nsdistributed_center_for_type(_cls: u64, _cmd: u64, _type: u64) -> u64 {
    distributed_center_ptr()
}

/// `-[NSDistributedNotificationCenter postNotificationName:object:]`
pub extern "C" fn shim_nsdistributed_post_name_object(
    _obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name)
        .unwrap_or("")
        .to_string();
    log::debug!(
        "NSDistributedNotificationCenter postNotificationName:\"{}\"",
        name_str
    );
    let notif = make_nsnotification(name, object, 0);
    deliver_to_distributed_observers(&name_str, notif);
}

/// `-[NSDistributedNotificationCenter postNotificationName:object:userInfo:]`
pub extern "C" fn shim_nsdistributed_post_name_object_userinfo(
    _obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name)
        .unwrap_or("")
        .to_string();
    log::debug!(
        "NSDistributedNotificationCenter postNotificationName:\"{}\" userInfo",
        name_str
    );
    let notif = make_nsnotification(name, object, userinfo);
    deliver_to_distributed_observers(&name_str, notif);
}

/// `-[NSDistributedNotificationCenter postNotificationName:object:userInfo:deliverImmediately:]`
pub extern "C" fn shim_nsdistributed_post_name_object_userinfo_deliver(
    _obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
    _deliver: u8,
) {
    let name_str = cfstring::extract_cfstring_cstr(name)
        .unwrap_or("")
        .to_string();
    let notif = make_nsnotification(name, object, userinfo);
    deliver_to_distributed_observers(&name_str, notif);
}

/// `-[NSDistributedNotificationCenter postNotificationName:object:userInfo:options:]`
pub extern "C" fn shim_nsdistributed_post_name_object_userinfo_options(
    _obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
    _options: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name)
        .unwrap_or("")
        .to_string();
    let notif = make_nsnotification(name, object, userinfo);
    deliver_to_distributed_observers(&name_str, notif);
}

/// `-[NSDistributedNotificationCenter addObserver:selector:name:object:]`
pub extern "C" fn shim_nsdistributed_add_observer(
    _obj: u64,
    _cmd: u64,
    observer: u64,
    sel: u64,
    name: u64,
    _object: u64,
) {
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };
    let token = NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut obs = dist_observers().lock().unwrap();
    obs.push(DistObserver {
        token,
        name: name_str,
        kind: ObserverKind::Selector {
            observer,
            sel_ptr: sel,
        },
    });
}

/// `-[NSDistributedNotificationCenter addObserver:selector:name:object:suspensionBehavior:]`
pub extern "C" fn shim_nsdistributed_add_observer_suspension(
    _obj: u64,
    _cmd: u64,
    observer: u64,
    sel: u64,
    name: u64,
    _object: u64,
    _suspension_behavior: u64,
) {
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };
    let token = NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut obs = dist_observers().lock().unwrap();
    obs.push(DistObserver {
        token,
        name: name_str,
        kind: ObserverKind::Selector {
            observer,
            sel_ptr: sel,
        },
    });
}

/// `-[NSDistributedNotificationCenter removeObserver:]`
pub extern "C" fn shim_nsdistributed_remove_observer(_obj: u64, _cmd: u64, observer: u64) {
    if observer == 0 {
        return;
    }
    let mut obs = dist_observers().lock().unwrap();
    obs.retain(|o| match &o.kind {
        ObserverKind::Selector {
            observer: obs_ptr, ..
        } => *obs_ptr != observer,
        ObserverKind::Block { block } => *block != observer,
    });
}

/// `-[NSDistributedNotificationCenter removeObserver:name:object:]`
pub extern "C" fn shim_nsdistributed_remove_observer_name_object(
    _obj: u64,
    _cmd: u64,
    observer: u64,
    name: u64,
    _object: u64,
) {
    if observer == 0 {
        return;
    }
    let name_filter = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };

    let mut obs = dist_observers().lock().unwrap();
    obs.retain(|o| {
        let is_target = match &o.kind {
            ObserverKind::Selector {
                observer: obs_ptr, ..
            } => *obs_ptr == observer,
            ObserverKind::Block { block } => *block == observer,
        };
        if !is_target {
            return true; // keep
        }
        // If name filter specified, only remove matching names
        if let Some(ref nf) = name_filter {
            if let Some(ref on) = o.name {
                return on != nf; // keep if doesn't match
            }
        }
        false // remove
    });
}

/// `-[NSDistributedNotificationCenter setSuspended:]`
pub extern "C" fn shim_nsdistributed_set_suspended(_obj: u64, _cmd: u64, _suspended: u8) {
    // No-op — we always deliver immediately
}

/// `-[NSDistributedNotificationCenter suspended]`
pub extern "C" fn shim_nsdistributed_suspended(_obj: u64, _cmd: u64) -> u8 {
    0 // not suspended
}

// ── CFNotificationCenter bridge ────────────────────────────────────

/// Called by CFNotificationCenterGetDistributedCenter shim
pub fn cf_distributed_center_handle() -> u64 {
    distributed_center_ptr()
}

/// Called by CFNotificationCenterGetLocalCenter shim
pub fn cf_local_center_handle() -> u64 {
    // Return the NSNotificationCenter default center pointer
    crate::nsnotificationcenter::default_center_addr()
}

/// Post a distributed notification by name (C string).
pub fn post_distributed_by_cstr(name: &str, object: Option<&str>, user_info: u64) {
    let name_cf = crate::cfstring::make_cfstring(name);
    let object_cf = object
        .map(|o| crate::cfstring::make_cfstring(o))
        .unwrap_or(0);
    let notif = make_nsnotification(name_cf, object_cf, user_info);
    deliver_to_distributed_observers(name, notif);
}

/// Add a distributed observer for a C-level CFNotificationCenter callback.
pub fn add_distributed_cf_observer(name: Option<&str>, callback: u64, _context: u64) {
    let name_str = name.map(|s| s.to_string());
    let token = NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut obs = dist_observers().lock().unwrap();
    obs.push(DistObserver {
        token,
        name: name_str,
        kind: ObserverKind::Block { block: callback },
    });
}

/// Remove a distributed observer by callback pointer.
pub fn remove_distributed_cf_observer(callback: u64) {
    let mut obs = dist_observers().lock().unwrap();
    obs.retain(|o| match &o.kind {
        ObserverKind::Block { block } => *block != callback,
        _ => true,
    });
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distributed_center_returns_nonzero() {
        let center = distributed_center_ptr();
        assert_ne!(center, 0);
    }

    #[test]
    fn distributed_center_is_singleton() {
        let c1 = distributed_center_ptr();
        let c2 = distributed_center_ptr();
        assert_eq!(c1, c2);
    }

    #[test]
    fn add_and_remove_distributed_observer() {
        // Clear state
        clear_all_distributed_observers();

        let fake_observer = 0x1234u64;
        let fake_sel = 0x5678u64;
        let name = crate::cfstring::make_cfstring("TestDistNotif");

        shim_nsdistributed_add_observer(0, 0, fake_observer, fake_sel, name, 0);

        {
            let obs = dist_observers().lock().unwrap();
            assert_eq!(obs.len(), 1);
        }

        shim_nsdistributed_remove_observer(0, 0, fake_observer);

        {
            let obs = dist_observers().lock().unwrap();
            assert_eq!(obs.len(), 0);
        }
    }

    #[test]
    fn remove_observer_by_name() {
        clear_all_distributed_observers();

        let fake_observer = 0xAAAAu64;
        let fake_sel = 0xBBBBu64;
        let name1 = crate::cfstring::make_cfstring("Notif1");
        let name2 = crate::cfstring::make_cfstring("Notif2");

        shim_nsdistributed_add_observer(0, 0, fake_observer, fake_sel, name1, 0);
        shim_nsdistributed_add_observer(0, 0, fake_observer, fake_sel, name2, 0);

        {
            let obs = dist_observers().lock().unwrap();
            assert_eq!(obs.len(), 2);
        }

        // Remove only Notif1
        shim_nsdistributed_remove_observer_name_object(0, 0, fake_observer, name1, 0);

        {
            let obs = dist_observers().lock().unwrap();
            assert_eq!(obs.len(), 1);
            assert_eq!(obs[0].name.as_deref(), Some("Notif2"));
        }

        clear_all_distributed_observers();
    }

    #[test]
    fn suspended_returns_false() {
        assert_eq!(shim_nsdistributed_suspended(0, 0), 0);
    }

    #[test]
    fn post_with_no_observers_does_not_panic() {
        clear_all_distributed_observers();
        let name = crate::cfstring::make_cfstring("NoOneListening");
        shim_nsdistributed_post_name_object(0, 0, name, 0);
    }

    #[test]
    fn cf_bridge_handles_nonzero() {
        assert_ne!(cf_distributed_center_handle(), 0);
    }

    #[test]
    fn add_observer_with_suspension_behavior() {
        clear_all_distributed_observers();
        let name = crate::cfstring::make_cfstring("TestSuspension");
        shim_nsdistributed_add_observer_suspension(0, 0, 0x1111, 0x2222, name, 0, 3);

        let obs = dist_observers().lock().unwrap();
        assert_eq!(obs.len(), 1);
        drop(obs);
        clear_all_distributed_observers();
    }
}
