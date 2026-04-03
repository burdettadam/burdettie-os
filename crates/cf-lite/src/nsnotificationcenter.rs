//! NSNotificationCenter shim: in-process notification dispatch.
//!
//! Implements actual observer storage and delivery for both selector-based
//! and block-based observers. NSNotification objects carry name, object,
//! and userInfo.
//!
//! NSNotification layout (32 bytes):
//!   offset  0: isa       (u64)
//!   offset  8: name      (u64 → NSString)
//!   offset 16: object    (u64 → id, may be 0)
//!   offset 24: userInfo  (u64 → NSDictionary, may be 0)

use crate::cfstring;
use crate::nsobj;
use std::sync::{Mutex, OnceLock};

// ── NSNotification ─────────────────────────────────────────────────

const NOTIF_SIZE: usize = 32;

/// Create an NSNotification object.
pub fn make_nsnotification(name: u64, object: u64, user_info: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NOTIF_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsnotification_class_addr();
        *((obj + 8) as *mut u64) = name;
        *((obj + 16) as *mut u64) = object;
        *((obj + 24) as *mut u64) = user_info;
    }
    obj
}

/// `+[NSNotification notificationWithName:object:userInfo:]`
pub extern "C" fn shim_nsnotification_with_name_object_userinfo(
    _cls: u64, _cmd: u64, name: u64, object: u64, user_info: u64,
) -> u64 {
    make_nsnotification(name, object, user_info)
}

/// `+[NSNotification notificationWithName:object:]`
pub extern "C" fn shim_nsnotification_with_name_object(
    _cls: u64, _cmd: u64, name: u64, object: u64,
) -> u64 {
    make_nsnotification(name, object, 0)
}

/// `-[NSNotification name]`
pub extern "C" fn shim_nsnotification_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSNotification object]`
pub extern "C" fn shim_nsnotification_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 16) as *const u64) }
}

/// `-[NSNotification userInfo]`
pub extern "C" fn shim_nsnotification_userinfo(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj + 24) as *const u64) }
}

// ── Observer storage ───────────────────────────────────────────────

/// Unique token for observers.
static NEXT_TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

enum ObserverKind {
    /// Selector-based: observer obj + raw SEL (C string pointer)
    Selector { observer: u64, sel_ptr: u64 },
    /// Block-based: ObjC block pointer
    Block { block: u64 },
}

struct Observer {
    token: u64,
    name: Option<String>,
    kind: ObserverKind,
}

/// Global observer list (simple — single default center).
static OBSERVERS: OnceLock<Mutex<Vec<Observer>>> = OnceLock::new();

fn observers() -> &'static Mutex<Vec<Observer>> {
    OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Clear all registered notification observers.
///
/// This is used by in-process runners such as `perf-bench`, where multiple
/// guest executions happen in the same host process and Foundation globals
/// must not leak from warmup runs into measured iterations.
pub fn clear_all_observers() {
    let mut obs = observers().lock().unwrap_or_else(|e| e.into_inner());
    obs.clear();
    NEXT_TOKEN.store(1, std::sync::atomic::Ordering::Relaxed);
}

// ── Notification delivery ──────────────────────────────────────────

fn deliver_to_observers(name_str: &str, notification: u64) {
    let obs = observers().lock().unwrap();
    for o in obs.iter() {
        // Name filter: if observer has a name, it must match
        if let Some(ref n) = o.name {
            if n != name_str {
                continue;
            }
        }
        match &o.kind {
            ObserverKind::Selector { observer, sel_ptr } => {
                // Use objc_msg_lookup_imp to find the IMP, then call it
                extern "C" {
                    fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64;
                }
                let imp = unsafe { objc_msg_lookup_imp(*observer, *sel_ptr) };
                if imp != 0 {
                    let f: extern "C" fn(u64, u64, u64) =
                        unsafe { std::mem::transmute(imp) };
                    f(*observer, *sel_ptr, notification);
                }
            }
            ObserverKind::Block { block } => {
                // Block layout: invoke at offset 16
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

// ── NSNotificationCenter ───────────────────────────────────────────

/// Singleton storage for `+[NSNotificationCenter defaultCenter]`.
static DEFAULT_CENTER: OnceLock<u64> = OnceLock::new();

fn default_center_ptr() -> u64 {
    *DEFAULT_CENTER.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nsnotificationcenter_class_addr();
            }
        }
        obj
    })
}

/// `+[NSNotificationCenter defaultCenter]`
pub extern "C" fn shim_nsnotificationcenter_default_center(_cls: u64, _cmd: u64) -> u64 {
    default_center_ptr()
}

/// `-[NSNotificationCenter postNotificationName:object:]`
pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object(
    _obj: u64, _cmd: u64, name: u64, object: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("").to_string();
    let notif = make_nsnotification(name, object, 0);
    deliver_to_observers(&name_str, notif);
}

/// `-[NSNotificationCenter postNotificationName:object:userInfo:]`
pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object_userinfo(
    _obj: u64, _cmd: u64, name: u64, object: u64, userinfo: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("").to_string();
    let notif = make_nsnotification(name, object, userinfo);
    deliver_to_observers(&name_str, notif);
}

/// `-[NSNotificationCenter postNotification:]`
pub extern "C" fn shim_nsnotificationcenter_post_notification(
    _obj: u64, _cmd: u64, notification: u64,
) {
    if notification == 0 { return; }
    let name = unsafe { *((notification + 8) as *const u64) };
    let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("").to_string();
    deliver_to_observers(&name_str, notification);
}

/// `-[NSNotificationCenter addObserver:selector:name:object:]`
pub extern "C" fn shim_nsnotificationcenter_add_observer(
    _obj: u64, _cmd: u64, observer: u64, sel: u64, name: u64, _object: u64,
) {
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };
    let token = NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut obs = observers().lock().unwrap();
    obs.push(Observer {
        token,
        name: name_str,
        kind: ObserverKind::Selector { observer, sel_ptr: sel },
    });
}

/// `-[NSNotificationCenter addObserverForName:object:queue:usingBlock:]`
pub extern "C" fn shim_nsnotificationcenter_add_observer_for_name(
    _obj: u64, _cmd: u64, name: u64, _object: u64, _queue: u64, block: u64,
) -> u64 {
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };
    let token = NEXT_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut obs = observers().lock().unwrap();
    obs.push(Observer {
        token,
        name: name_str,
        kind: ObserverKind::Block { block },
    });
    // Return a token sentinel that can be used with removeObserver:
    // We encode the token into a fake NSObject pointer
    let sentinel = crate::pal_alloc().calloc(1, 16) as u64;
    if sentinel != 0 {
        unsafe {
            *(sentinel as *mut u64) = nsobj::nsobject_class_addr();
            *((sentinel + 8) as *mut u64) = token;
        }
    }
    sentinel
}

/// `-[NSNotificationCenter removeObserver:]`
pub extern "C" fn shim_nsnotificationcenter_remove_observer(
    _obj: u64, _cmd: u64, observer: u64,
) {
    if observer == 0 { return; }
    let mut obs = observers().lock().unwrap();
    // Check if it's a block token (has token at offset 8)
    let token_val = unsafe { *((observer + 8) as *const u64) };
    if token_val > 0 && token_val < 0x1000_0000 {
        // Token-based removal (block observer)
        obs.retain(|o| o.token != token_val);
    } else {
        // Observer-ptr-based removal (selector observer)
        obs.retain(|o| {
            if let ObserverKind::Selector { observer: obs_ptr, .. } = &o.kind {
                *obs_ptr != observer
            } else {
                true
            }
        });
    }
}

/// `-[NSNotificationCenter removeObserver:name:object:]`
pub extern "C" fn shim_nsnotificationcenter_remove_observer_name_object(
    _obj: u64, _cmd: u64, observer: u64, name: u64, _object: u64,
) {
    if observer == 0 { return; }
    let name_str = if name != 0 {
        cfstring::extract_cfstring_cstr(name).map(|s| s.to_string())
    } else {
        None
    };
    let mut obs = observers().lock().unwrap();
    obs.retain(|o| {
        if let ObserverKind::Selector { observer: obs_ptr, .. } = &o.kind {
            if *obs_ptr != observer { return true; }
            if let Some(ref n) = name_str {
                if let Some(ref on) = o.name {
                    return on != n;
                }
            }
            false
        } else {
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use std::sync::Mutex;

    /// Serializes tests that mutate the global notification observer list.
    static NOTIFICATION_TEST_LOCK: Mutex<()> = Mutex::new(());

    // Observer dispatch is not tested here because it requires ObjC runtime
    // dispatch (objc_msg_lookup_imp) and involves global mutable state.
    // The tests below validate notification object construction and the
    // thread-safe default center singleton.

    #[test]
    fn default_center_returns_nonzero() {
        assert_ne!(shim_nsnotificationcenter_default_center(0, 0), 0);
    }

    #[test]
    fn default_center_is_stable_singleton() {
        let a = shim_nsnotificationcenter_default_center(0, 0);
        let b = shim_nsnotificationcenter_default_center(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn make_notification_stores_fields() {
        let name = make_string("MyNotification");
        let sender = make_string("sender-object");
        let notif = make_nsnotification(name, sender, 0);
        assert_ne!(notif, 0);
        assert_eq!(shim_nsnotification_name(notif, 0), name);
        assert_eq!(shim_nsnotification_object(notif, 0), sender);
        assert_eq!(shim_nsnotification_userinfo(notif, 0), 0);
        free_obj(notif);
    }

    #[test]
    fn notification_with_name_object_userinfo_shim() {
        let name = make_string("TestNotifFull");
        let info = make_string("extra-info");
        let notif = shim_nsnotification_with_name_object_userinfo(0, 0, name, 0, info);
        assert_ne!(notif, 0);
        assert_eq!(shim_nsnotification_name(notif, 0), name);
        assert_eq!(shim_nsnotification_userinfo(notif, 0), info);
        free_obj(notif);
    }

    #[test]
    fn notification_with_name_object_shim_no_userinfo() {
        let name = make_string("TestNotifSimple");
        let notif = shim_nsnotification_with_name_object(0, 0, name, 0);
        assert_ne!(notif, 0);
        assert_eq!(shim_nsnotification_name(notif, 0), name);
        assert_eq!(shim_nsnotification_userinfo(notif, 0), 0);
        free_obj(notif);
    }

    #[test]
    fn notification_accessors_nil_safety() {
        assert_eq!(shim_nsnotification_name(0, 0), 0);
        assert_eq!(shim_nsnotification_object(0, 0), 0);
        assert_eq!(shim_nsnotification_userinfo(0, 0), 0);
    }

    #[test]
    fn remove_observer_nil_is_noop() {
        let center = shim_nsnotificationcenter_default_center(0, 0);
        // observer=0 → early return, must not crash
        shim_nsnotificationcenter_remove_observer(center, 0, 0);
    }

    #[test]
    fn post_notification_with_nil_name_does_not_crash() {
        // posts with name_str="", iterates observers that match ""
        shim_nsnotificationcenter_post_notification_name_object(
            shim_nsnotificationcenter_default_center(0, 0),
            0, 0, 0,
        );
    }

    #[test]
    fn clear_all_observers_resets_registry_and_token_counter() {
        let _lock = NOTIFICATION_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        clear_all_observers();
        {
            let mut obs = observers().lock().unwrap_or_else(|e| e.into_inner());
            obs.push(Observer {
                token: 41,
                name: Some("Alpha".to_string()),
                kind: ObserverKind::Selector {
                    observer: 0x1000,
                    sel_ptr: 0x2000,
                },
            });
            obs.push(Observer {
                token: 42,
                name: Some("Beta".to_string()),
                kind: ObserverKind::Block { block: 0x3000 },
            });
        }
        NEXT_TOKEN.store(99, std::sync::atomic::Ordering::Relaxed);

        clear_all_observers();

        let obs = observers().lock().unwrap_or_else(|e| e.into_inner());
        assert!(obs.is_empty());
        assert_eq!(NEXT_TOKEN.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
