//! NSUserNotificationCenter stub — deprecated user notification API.
//!
//! `NSUserNotificationCenter` was deprecated in macOS 10.14 in favour of
//! `UNUserNotificationCenter`.  Many older apps still reference it on
//! launch.  All methods are safe no-ops.

use std::sync::OnceLock;
use crate::nsobj;

static SHARED_CENTER: OnceLock<u64> = OnceLock::new();

/// `+[NSUserNotificationCenter defaultUserNotificationCenter]`
pub extern "C" fn shim_nsusernotificationcenter_default(_cls: u64, _cmd: u64) -> u64 {
    *SHARED_CENTER.get_or_init(|| {
        let ptr = crate::pal_alloc().calloc(1, 16) as u64;
        unsafe {
            *(ptr as *mut u64) = nsobj::nsusernotificationcenter_class_addr();
        }
        ptr
    })
}

/// `-[NSUserNotificationCenter setDelegate:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_set_delegate(
    _obj: u64,
    _cmd: u64,
    _delegate: u64,
) {
}

/// `-[NSUserNotificationCenter delegate]` → nil.
pub extern "C" fn shim_nsusernotificationcenter_delegate(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSUserNotificationCenter deliverNotification:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_deliver(
    _obj: u64,
    _cmd: u64,
    _notification: u64,
) {
}

/// `-[NSUserNotificationCenter removeDeliveredNotification:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_remove(
    _obj: u64,
    _cmd: u64,
    _notification: u64,
) {
}

/// `-[NSUserNotificationCenter removeAllDeliveredNotifications]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_remove_all(_obj: u64, _cmd: u64) {}

/// `-[NSUserNotificationCenter deliveredNotifications]` → empty NSArray (nil).
pub extern "C" fn shim_nsusernotificationcenter_delivered_notifications(
    _obj: u64,
    _cmd: u64,
) -> u64 {
    0
}

/// `-[NSUserNotificationCenter scheduledNotifications]` → empty NSArray (nil).
pub extern "C" fn shim_nsusernotificationcenter_scheduled_notifications(
    _obj: u64,
    _cmd: u64,
) -> u64 {
    0
}

/// `-[NSUserNotificationCenter setScheduledNotifications:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_set_scheduled(
    _obj: u64,
    _cmd: u64,
    _notifications: u64,
) {
}

/// `-[NSUserNotificationCenter scheduleNotification:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_schedule(
    _obj: u64,
    _cmd: u64,
    _notification: u64,
) {
}
