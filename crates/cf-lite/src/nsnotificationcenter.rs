//! NSNotificationCenter shim: synchronous, in-process notification dispatch.
//!
//! Notifications carry name, object and userInfo in a 32-byte allocation.
//! Delivery snapshots registrations without holding the registry lock while
//! invoking guest code. This shim does not implement queues or block copying;
//! observers, blocks and captured objects must remain live during delivery.
//! Delivery and selector removal are scoped to the receiving center. A known
//! block token cancels its registration regardless of the removal receiver or
//! filters, matching the isolated native Foundation observations in M1.
//! Center pointers are borrowed identities, not retained/validated capabilities;
//! callers must remove registrations before releasing a custom center.

use crate::cfstring;
use crate::nsobj;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const NOTIF_SIZE: usize = 32;

pub fn make_nsnotification(name: u64, object: u64, user_info: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NOTIF_SIZE) as u64;
    if obj != 0 {
        // SAFETY: calloc returned a writable NOTIF_SIZE-byte allocation.
        unsafe {
            *(obj as *mut u64) = nsobj::nsnotification_class_addr();
            *((obj + 8) as *mut u64) = name;
            *((obj + 16) as *mut u64) = object;
            *((obj + 24) as *mut u64) = user_info;
        }
    }
    obj
}

pub extern "C" fn shim_nsnotification_with_name_object_userinfo(
    _cls: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    user_info: u64,
) -> u64 {
    make_nsnotification(name, object, user_info)
}

pub extern "C" fn shim_nsnotification_with_name_object(
    _cls: u64,
    _cmd: u64,
    name: u64,
    object: u64,
) -> u64 {
    make_nsnotification(name, object, 0)
}

pub extern "C" fn shim_nsnotification_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // SAFETY: callers provide an NSNotification allocation with this field.
    unsafe { *((obj + 8) as *const u64) }
}

pub extern "C" fn shim_nsnotification_object(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // SAFETY: callers provide an NSNotification allocation with this field.
    unsafe { *((obj + 16) as *const u64) }
}

pub extern "C" fn shim_nsnotification_userinfo(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // SAFETY: callers provide an NSNotification allocation with this field.
    unsafe { *((obj + 24) as *const u64) }
}

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy)]
enum ObserverKind {
    Selector { observer: u64, sel_ptr: u64 },
    Block { block: u64, handle: u64 },
}

struct Observer {
    center: u64,
    name: Option<String>,
    object: u64,
    kind: ObserverKind,
    active: Arc<AtomicBool>,
}

impl Observer {
    fn matches_observer(&self, observer: u64) -> bool {
        match self.kind {
            ObserverKind::Selector {
                observer: value, ..
            } => value == observer,
            ObserverKind::Block { handle, .. } => handle == observer,
        }
    }
}

static OBSERVERS: OnceLock<Mutex<Vec<Observer>>> = OnceLock::new();

fn observers() -> &'static Mutex<Vec<Observer>> {
    OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn remove_registered_observer(center: u64, observer: u64, name_filter: Option<&str>, object: u64) {
    observers().lock().unwrap().retain(|entry| {
        // A block token identifies its own registration. Selector identities
        // still require the receiving center and supplied removal filters.
        // Never dereference an arbitrary observer to guess whether it is a token.
        let matches = entry.matches_observer(observer)
            && match entry.kind {
                ObserverKind::Block { .. } => true,
                ObserverKind::Selector { .. } => {
                    entry.center == center
                        && name_filter.is_none_or(|name| entry.name.as_deref() == Some(name))
                        && (object == 0 || entry.object == object)
                }
            };
        if matches {
            entry.active.store(false, Ordering::Release);
            false
        } else {
            true
        }
    });
}

/// Clear registrations and invalidate pending delivery snapshots for runtime reuse.
pub fn clear_all_observers() {
    let mut entries = observers().lock().unwrap_or_else(|e| e.into_inner());
    for entry in entries.iter() {
        entry.active.store(false, Ordering::Release);
    }
    entries.clear();
    NEXT_TOKEN.store(1, Ordering::Relaxed);
}

fn deliver_to_observers(center: u64, name_str: &str, notification: u64) {
    if notification == 0 {
        return;
    }
    let object = shim_nsnotification_object(notification, 0);
    let targets: Vec<_> = observers()
        .lock()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry.center == center
                && entry.name.as_deref().is_none_or(|name| name == name_str)
                && (entry.object == 0 || entry.object == object)
        })
        .map(|entry| (Arc::clone(&entry.active), entry.kind))
        .collect();
    // The mutex guard is gone before any guest callback. Registrations added
    // during delivery wait for a later post; removed pending entries are skipped.
    for (active, kind) in targets {
        if !active.load(Ordering::Acquire) {
            continue;
        }
        match kind {
            ObserverKind::Selector { observer, sel_ptr } => {
                extern "C" {
                    fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64;
                }
                // SAFETY: the registering guest supplies a live object and SEL.
                let imp = unsafe { objc_msg_lookup_imp(observer, sel_ptr) };
                if imp != 0 {
                    // SAFETY: notification selectors use (self, _cmd, notification).
                    let invoke: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
                    invoke(observer, sel_ptr, notification);
                }
            }
            ObserverKind::Block { block, .. } => {
                // SAFETY: registration requires a live ObjC block; its invoke
                // pointer is at byte offset 16 in the supported 64-bit ABI.
                let invoke_ptr = unsafe { *((block + 16) as *const u64) };
                if invoke_ptr != 0 {
                    // SAFETY: a notification block takes (block, notification).
                    let invoke: extern "C" fn(u64, u64) =
                        unsafe { std::mem::transmute(invoke_ptr) };
                    invoke(block, notification);
                }
            }
        }
    }
}

static DEFAULT_CENTER: OnceLock<u64> = OnceLock::new();

fn default_center_ptr() -> u64 {
    *DEFAULT_CENTER.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj != 0 {
            // SAFETY: calloc returned a writable 16-byte object allocation.
            unsafe { *(obj as *mut u64) = nsobj::nsnotificationcenter_class_addr() };
        }
        obj
    })
}

// Direct internal shim calls historically use zero for the default center.
// Preserve that bridge convention without conflating nonzero custom centers.
fn center_key(center: u64) -> u64 {
    if center == 0 {
        default_center_ptr()
    } else {
        center
    }
}

pub fn default_center_addr() -> u64 {
    default_center_ptr()
}

pub extern "C" fn shim_nsnotificationcenter_default_center(_cls: u64, _cmd: u64) -> u64 {
    default_center_ptr()
}

pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object(
    obj: u64,
    cmd: u64,
    name: u64,
    object: u64,
) {
    shim_nsnotificationcenter_post_notification_name_object_userinfo(obj, cmd, name, object, 0);
}

pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object_userinfo(
    obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("");
    let notification = make_nsnotification(name, object, userinfo);
    deliver_to_observers(center_key(obj), name_str, notification);
}

pub extern "C" fn shim_nsnotificationcenter_post_notification(
    obj: u64,
    _cmd: u64,
    notification: u64,
) {
    if notification != 0 {
        let name = shim_nsnotification_name(notification, 0);
        let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("");
        deliver_to_observers(center_key(obj), name_str, notification);
    }
}

pub extern "C" fn shim_nsnotificationcenter_add_observer(
    obj: u64,
    _cmd: u64,
    observer: u64,
    sel: u64,
    name: u64,
    object: u64,
) {
    if observer == 0 || sel == 0 {
        return;
    }
    let center = center_key(obj);
    observers().lock().unwrap().push(Observer {
        center,
        name: cfstring::extract_cfstring_cstr(name).map(str::to_owned),
        object,
        kind: ObserverKind::Selector {
            observer,
            sel_ptr: sel,
        },
        active: Arc::new(AtomicBool::new(true)),
    });
}

pub extern "C" fn shim_nsnotificationcenter_add_observer_for_name(
    obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    _queue: u64,
    block: u64,
) -> u64 {
    if block == 0 {
        return 0;
    }
    let handle = crate::pal_alloc().calloc(1, 16) as u64;
    if handle == 0 {
        return 0; // Do not install an observer that cannot be removed.
    }
    let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    // SAFETY: calloc returned a writable 16-byte sentinel allocation.
    unsafe {
        *(handle as *mut u64) = nsobj::nsobject_class_addr();
        *((handle + 8) as *mut u64) = token;
    }
    let center = center_key(obj);
    observers().lock().unwrap().push(Observer {
        center,
        name: cfstring::extract_cfstring_cstr(name).map(str::to_owned),
        object,
        kind: ObserverKind::Block { block, handle },
        active: Arc::new(AtomicBool::new(true)),
    });
    handle
}

pub extern "C" fn shim_nsnotificationcenter_remove_observer(obj: u64, _cmd: u64, observer: u64) {
    if observer != 0 {
        remove_registered_observer(center_key(obj), observer, None, 0);
    }
}

pub extern "C" fn shim_nsnotificationcenter_remove_observer_name_object(
    obj: u64,
    _cmd: u64,
    observer: u64,
    name: u64,
    object: u64,
) {
    if observer == 0 {
        return;
    }
    let name_filter = cfstring::extract_cfstring_cstr(name);
    remove_registered_observer(center_key(obj), observer, name_filter, object);
}

#[cfg(test)]
#[path = "nsnotificationcenter_tests.rs"]
mod tests;
