//! NSNotificationCenter shim: in-process notification dispatch.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa          (u64 — pointer to NSNotificationCenter class)
//!   offset  8: (reserved)   (u64 — unused, for future observer storage)
//!
//! Total: 16 bytes per center object.
//!
//! This is a minimal stub: `defaultCenter` returns a singleton,
//! `postNotificationName:object:` is a no-op (observers are not tracked),
//! and `addObserver:selector:name:object:` / `removeObserver:` are no-ops.
//! This allows binaries that reference NSNotificationCenter to load and
//! run without crashing, even though notifications are silently dropped.

use crate::nsobj;
use std::sync::OnceLock;

/// Singleton storage for `+[NSNotificationCenter defaultCenter]`.
static DEFAULT_CENTER: OnceLock<u64> = OnceLock::new();

fn default_center_ptr() -> u64 {
    *DEFAULT_CENTER.get_or_init(|| {
        let obj = unsafe { libc::calloc(1, 16) } as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nsnotificationcenter_class_addr();
            }
        }
        obj
    })
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSNotificationCenter defaultCenter]`
pub extern "C" fn shim_nsnotificationcenter_default_center(_cls: u64, _cmd: u64) -> u64 {
    default_center_ptr()
}

// ── Instance methods (no-ops) ──────────────────────────────────────

/// `-[NSNotificationCenter postNotificationName:object:]` — no-op stub.
pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object(
    _obj: u64,
    _cmd: u64,
    _name: u64,
    _object: u64,
) {
    // Silently drop — no observers tracked.
}

/// `-[NSNotificationCenter postNotificationName:object:userInfo:]` — no-op stub.
pub extern "C" fn shim_nsnotificationcenter_post_notification_name_object_userinfo(
    _obj: u64,
    _cmd: u64,
    _name: u64,
    _object: u64,
    _userinfo: u64,
) {
    // Silently drop — no observers tracked.
}

/// `-[NSNotificationCenter addObserver:selector:name:object:]` — no-op stub.
pub extern "C" fn shim_nsnotificationcenter_add_observer(
    _obj: u64,
    _cmd: u64,
    _observer: u64,
    _sel: u64,
    _name: u64,
    _object: u64,
) {
    // No-op — observer registration not implemented.
}

/// `-[NSNotificationCenter removeObserver:]` — no-op stub.
pub extern "C" fn shim_nsnotificationcenter_remove_observer(_obj: u64, _cmd: u64, _observer: u64) {
    // No-op — observer removal not implemented.
}

/// `-[NSNotificationCenter removeObserver:name:object:]` — no-op stub.
pub extern "C" fn shim_nsnotificationcenter_remove_observer_name_object(
    _obj: u64,
    _cmd: u64,
    _observer: u64,
    _name: u64,
    _object: u64,
) {
    // No-op — observer removal not implemented.
}
