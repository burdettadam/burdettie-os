//! NSDistributedNotificationCenter shim: synchronous, in-process notifications.
//!
//! This is not a cross-process `distnoted` implementation. Suspension, queues,
//! and session options are not implemented; notifications are delivered now.
//! Observer objects and callback contexts must remain valid during delivery.
//!
//! NSDistributedNotificationCenter layout (16 bytes):
//!   offset  0: isa       (u64)
//!   offset  8: reserved  (u64)

use crate::cfstring;
use crate::nsnotificationcenter::{
    make_nsnotification, shim_nsnotification_name, shim_nsnotification_object,
    shim_nsnotification_userinfo,
};
use crate::nsobj;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// NSNotificationSuspensionBehavior values (not implemented by this shim).
#[allow(dead_code)]
#[repr(u64)]
pub enum SuspensionBehavior {
    Drop = 1,
    Coalesce = 2,
    Hold = 3,
    DeliverImmediately = 4,
}

#[allow(dead_code)]
pub const NS_DISTRIBUTED_NOTIFICATION_DELIVER_IMMEDIATELY: u64 = 1 << 0;
#[allow(dead_code)]
pub const NS_DISTRIBUTED_NOTIFICATION_POST_TO_ALL_SESSIONS: u64 = 1 << 1;

#[derive(Clone, Copy)]
enum ObserverKind {
    Selector { observer: u64, sel_ptr: u64 },
    CfCallback { callback: u64, context: u64 },
}

struct DistObserver {
    name: Option<String>,
    object: Option<String>,
    kind: ObserverKind,
    active: Arc<AtomicBool>,
}

impl DistObserver {
    fn matches_observer(&self, observer: u64) -> bool {
        match self.kind {
            ObserverKind::Selector {
                observer: value, ..
            } => value == observer,
            ObserverKind::CfCallback { context, .. } => context == observer,
        }
    }
}

static DIST_OBSERVERS: OnceLock<Mutex<Vec<DistObserver>>> = OnceLock::new();

fn dist_observers() -> &'static Mutex<Vec<DistObserver>> {
    DIST_OBSERVERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn remove_matching(mut matches: impl FnMut(&DistObserver) -> bool) {
    dist_observers().lock().unwrap().retain(|entry| {
        if matches(entry) {
            entry.active.store(false, Ordering::Release);
            false
        } else {
            true
        }
    });
}

/// Clear registrations and invalidate pending snapshots for runtime reuse.
pub fn clear_all_distributed_observers() {
    let mut entries = dist_observers().lock().unwrap_or_else(|e| e.into_inner());
    for entry in entries.iter() {
        entry.active.store(false, Ordering::Release);
    }
    entries.clear();
}

fn deliver_to_distributed_observers(name_str: &str, notification: u64) {
    if notification == 0 {
        return;
    }
    let object = shim_nsnotification_object(notification, 0);
    let object_str = cfstring::extract_cfstring_cstr(object);
    // Never call guest code with the registry locked. A callback can remove
    // itself, clear the registry, add another observer, or post recursively.
    let targets: Vec<_> = dist_observers()
        .lock()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry.name.as_deref().is_none_or(|name| name == name_str)
                && entry
                    .object
                    .as_deref()
                    .is_none_or(|name| Some(name) == object_str)
        })
        .map(|entry| (Arc::clone(&entry.active), entry.kind))
        .collect();
    for (active, kind) in targets {
        // A removal/reset from an earlier callback cancels pending delivery.
        // A fresh registration has a different marker, even after a reset.
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
            ObserverKind::CfCallback { callback, context } => {
                // SAFETY: registration requires a non-null CFNotificationCallback.
                // It is a C function, NOT an Objective-C block with an invoke field.
                let invoke: extern "C" fn(u64, u64, u64, u64, u64) =
                    unsafe { std::mem::transmute(callback) };
                invoke(
                    distributed_center_ptr(),
                    context,
                    shim_nsnotification_name(notification, 0),
                    object,
                    shim_nsnotification_userinfo(notification, 0),
                );
            }
        }
    }
}

static DISTRIBUTED_CENTER: OnceLock<u64> = OnceLock::new();

fn distributed_center_ptr() -> u64 {
    *DISTRIBUTED_CENTER.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj != 0 {
            // SAFETY: calloc returned a writable 16-byte object allocation.
            unsafe { *(obj as *mut u64) = nsobj::nsdistributednotificationcenter_class_addr() };
        }
        obj
    })
}

pub extern "C" fn shim_nsdistributed_default_center(_cls: u64, _cmd: u64) -> u64 {
    distributed_center_ptr()
}

pub extern "C" fn shim_nsdistributed_center_for_type(_cls: u64, _cmd: u64, _type: u64) -> u64 {
    distributed_center_ptr()
}

pub extern "C" fn shim_nsdistributed_post_name_object(obj: u64, cmd: u64, name: u64, object: u64) {
    shim_nsdistributed_post_name_object_userinfo(obj, cmd, name, object, 0);
}

pub extern "C" fn shim_nsdistributed_post_name_object_userinfo(
    _obj: u64,
    _cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
) {
    let name_str = cfstring::extract_cfstring_cstr(name).unwrap_or("");
    let notification = make_nsnotification(name, object, userinfo);
    deliver_to_distributed_observers(name_str, notification);
}

pub extern "C" fn shim_nsdistributed_post_name_object_userinfo_deliver(
    obj: u64,
    cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
    _deliver: u8,
) {
    shim_nsdistributed_post_name_object_userinfo(obj, cmd, name, object, userinfo);
}

pub extern "C" fn shim_nsdistributed_post_name_object_userinfo_options(
    obj: u64,
    cmd: u64,
    name: u64,
    object: u64,
    userinfo: u64,
    _options: u64,
) {
    shim_nsdistributed_post_name_object_userinfo(obj, cmd, name, object, userinfo);
}

pub extern "C" fn shim_nsdistributed_add_observer(
    obj: u64,
    cmd: u64,
    observer: u64,
    sel: u64,
    name: u64,
    object: u64,
) {
    shim_nsdistributed_add_observer_suspension(obj, cmd, observer, sel, name, object, 2);
}

pub extern "C" fn shim_nsdistributed_add_observer_suspension(
    _obj: u64,
    _cmd: u64,
    observer: u64,
    sel: u64,
    name: u64,
    object: u64,
    _suspension_behavior: u64,
) {
    if observer == 0 || sel == 0 {
        return;
    }
    dist_observers().lock().unwrap().push(DistObserver {
        name: cfstring::extract_cfstring_cstr(name).map(str::to_owned),
        object: cfstring::extract_cfstring_cstr(object).map(str::to_owned),
        kind: ObserverKind::Selector {
            observer,
            sel_ptr: sel,
        },
        active: Arc::new(AtomicBool::new(true)),
    });
}

pub extern "C" fn shim_nsdistributed_remove_observer(_obj: u64, _cmd: u64, observer: u64) {
    if observer != 0 {
        remove_matching(|entry| entry.matches_observer(observer));
    }
}

pub extern "C" fn shim_nsdistributed_remove_observer_name_object(
    _obj: u64,
    _cmd: u64,
    observer: u64,
    name: u64,
    object: u64,
) {
    if observer == 0 {
        return;
    }
    let name_filter = cfstring::extract_cfstring_cstr(name);
    let object_filter = cfstring::extract_cfstring_cstr(object);
    remove_matching(|entry| {
        entry.matches_observer(observer)
            && name_filter.is_none_or(|name| entry.name.as_deref() == Some(name))
            && object_filter.is_none_or(|name| entry.object.as_deref() == Some(name))
    });
}

pub extern "C" fn shim_nsdistributed_set_suspended(_obj: u64, _cmd: u64, _suspended: u8) {}

pub extern "C" fn shim_nsdistributed_suspended(_obj: u64, _cmd: u64) -> u8 {
    0
}

pub fn cf_distributed_center_handle() -> u64 {
    distributed_center_ptr()
}

pub fn cf_local_center_handle() -> u64 {
    crate::nsnotificationcenter::default_center_addr()
}

pub fn post_distributed_by_cstr(name: &str, object: Option<&str>, user_info: u64) {
    let name_cf = cfstring::make_cfstring(name);
    let object_cf = object.map(cfstring::make_cfstring).unwrap_or(0);
    shim_nsdistributed_post_name_object_userinfo(0, 0, name_cf, object_cf, user_info);
}

/// Register an in-process CF callback; context is passed as the observer argument.
/// This narrow bridge has no sender-filter argument. It does not implement IPC.
pub fn add_distributed_cf_observer(name: Option<&str>, callback: u64, context: u64) {
    if callback == 0 {
        return;
    }
    dist_observers().lock().unwrap().push(DistObserver {
        name: name.map(str::to_owned),
        object: None,
        kind: ObserverKind::CfCallback { callback, context },
        active: Arc::new(AtomicBool::new(true)),
    });
}

pub fn remove_distributed_cf_observer(callback: u64) {
    remove_matching(
        |entry| matches!(entry.kind, ObserverKind::CfCallback { callback: value, .. } if value == callback),
    );
}

#[cfg(test)]
#[path = "nsdistributednotificationcenter_tests.rs"]
mod tests;
