//! NSUserNotificationCenter stub — deprecated user notification API.
//!
//! `NSUserNotificationCenter` was deprecated in macOS 10.14 in favour of
//! `UNUserNotificationCenter`.  Many older apps still reference it on
//! launch.  All methods are safe no-ops.

use crate::nsobj;
use std::sync::OnceLock;

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
pub extern "C" fn shim_nsusernotificationcenter_set_delegate(_obj: u64, _cmd: u64, _delegate: u64) {
}

/// `-[NSUserNotificationCenter delegate]` → nil.
pub extern "C" fn shim_nsusernotificationcenter_delegate(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSUserNotificationCenter deliverNotification:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_deliver(_obj: u64, _cmd: u64, _notification: u64) {}

/// `-[NSUserNotificationCenter removeDeliveredNotification:]` — no-op.
pub extern "C" fn shim_nsusernotificationcenter_remove(_obj: u64, _cmd: u64, _notification: u64) {}

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
pub extern "C" fn shim_nsusernotificationcenter_schedule(_obj: u64, _cmd: u64, _notification: u64) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // -- +[NSUserNotificationCenter defaultUserNotificationCenter] --

    #[test]
    fn test_default_returns_nonzero() {
        let ptr = shim_nsusernotificationcenter_default(0, 0);
        assert_ne!(ptr, 0, "default center must be non-nil");
    }

    #[test]
    fn test_default_is_singleton() {
        let a = shim_nsusernotificationcenter_default(0, 0);
        let b = shim_nsusernotificationcenter_default(0, 0);
        assert_eq!(a, b, "default center must return the same pointer");
    }

    #[test]
    fn test_default_has_correct_isa() {
        let ptr = shim_nsusernotificationcenter_default(0, 0);
        let isa = read_field(ptr, 0);
        assert_eq!(isa, nsobj::nsusernotificationcenter_class_addr());
    }

    // -- -[NSUserNotificationCenter setDelegate:] --

    #[test]
    fn test_set_delegate_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_set_delegate(obj, 0, 0xCAFE);
        free_obj(obj);
    }

    #[test]
    fn test_set_delegate_nil_obj() {
        // nil receiver — must not crash
        shim_nsusernotificationcenter_set_delegate(0, 0, 0xCAFE);
    }

    // -- -[NSUserNotificationCenter delegate] --

    #[test]
    fn test_delegate_returns_zero() {
        let obj = alloc_obj_fake(16);
        assert_eq!(shim_nsusernotificationcenter_delegate(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn test_delegate_nil_obj() {
        assert_eq!(shim_nsusernotificationcenter_delegate(0, 0), 0);
    }

    // -- -[NSUserNotificationCenter deliverNotification:] --

    #[test]
    fn test_deliver_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_deliver(obj, 0, 0xBEEF);
        free_obj(obj);
    }

    #[test]
    fn test_deliver_nil_obj() {
        shim_nsusernotificationcenter_deliver(0, 0, 0xBEEF);
    }

    // -- -[NSUserNotificationCenter removeDeliveredNotification:] --

    #[test]
    fn test_remove_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_remove(obj, 0, 0xBEEF);
        free_obj(obj);
    }

    #[test]
    fn test_remove_nil_obj() {
        shim_nsusernotificationcenter_remove(0, 0, 0xBEEF);
    }

    // -- -[NSUserNotificationCenter removeAllDeliveredNotifications] --

    #[test]
    fn test_remove_all_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_remove_all(obj, 0);
        free_obj(obj);
    }

    #[test]
    fn test_remove_all_nil_obj() {
        shim_nsusernotificationcenter_remove_all(0, 0);
    }

    // -- -[NSUserNotificationCenter deliveredNotifications] --

    #[test]
    fn test_delivered_notifications_returns_zero() {
        let obj = alloc_obj_fake(16);
        assert_eq!(
            shim_nsusernotificationcenter_delivered_notifications(obj, 0),
            0
        );
        free_obj(obj);
    }

    #[test]
    fn test_delivered_notifications_nil_obj() {
        assert_eq!(
            shim_nsusernotificationcenter_delivered_notifications(0, 0),
            0
        );
    }

    // -- -[NSUserNotificationCenter scheduledNotifications] --

    #[test]
    fn test_scheduled_notifications_returns_zero() {
        let obj = alloc_obj_fake(16);
        assert_eq!(
            shim_nsusernotificationcenter_scheduled_notifications(obj, 0),
            0
        );
        free_obj(obj);
    }

    #[test]
    fn test_scheduled_notifications_nil_obj() {
        assert_eq!(
            shim_nsusernotificationcenter_scheduled_notifications(0, 0),
            0
        );
    }

    // -- -[NSUserNotificationCenter setScheduledNotifications:] --

    #[test]
    fn test_set_scheduled_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_set_scheduled(obj, 0, 0xCAFE);
        free_obj(obj);
    }

    #[test]
    fn test_set_scheduled_nil_obj() {
        shim_nsusernotificationcenter_set_scheduled(0, 0, 0xCAFE);
    }

    // -- -[NSUserNotificationCenter scheduleNotification:] --

    #[test]
    fn test_schedule_noop() {
        let obj = alloc_obj_fake(16);
        shim_nsusernotificationcenter_schedule(obj, 0, 0xBEEF);
        free_obj(obj);
    }

    #[test]
    fn test_schedule_nil_obj() {
        shim_nsusernotificationcenter_schedule(0, 0, 0xBEEF);
    }
}
