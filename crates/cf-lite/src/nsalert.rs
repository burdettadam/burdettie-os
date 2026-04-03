//! NSAlert shim: alert dialog stub for AppKit.
//!
//! Object layout:
//!
//!   offset  0: isa                   (u64)
//!   offset  8: message_text_ptr      (u64 — NSString*, or 0)
//!   offset 16: informative_text_ptr  (u64 — NSString*, or 0)
//!   offset 24: alert_style           (u64 — 0=warning, 1=informational, 2=critical)
//!   offset 32: button_count          (u64 — number of buttons added)
//!
//! Total: 40 bytes.

const MESSAGE_TEXT_OFF: usize = 8;
const INFORMATIVE_TEXT_OFF: usize = 16;
const ALERT_STYLE_OFF: usize = 24;
const BUTTON_COUNT_OFF: usize = 32;
pub(crate) const ALERT_SIZE: usize = 40;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSAlert messageText]`
pub extern "C" fn shim_nsalert_message_text(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, MESSAGE_TEXT_OFF);
    if ptr != 0 {
        return ptr;
    }
    // Return empty string if none set
    crate::cfstring::shim_cf_string_create_with_cstring(0, b"\0".as_ptr().cast(), 0)
}

/// `-[NSAlert setMessageText:]`
pub extern "C" fn shim_nsalert_set_message_text(obj: u64, _cmd: u64, text: u64) {
    write_u64(obj, MESSAGE_TEXT_OFF, text);
}

/// `-[NSAlert informativeText]`
pub extern "C" fn shim_nsalert_informative_text(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, INFORMATIVE_TEXT_OFF)
}

/// `-[NSAlert setInformativeText:]`
pub extern "C" fn shim_nsalert_set_informative_text(obj: u64, _cmd: u64, text: u64) {
    write_u64(obj, INFORMATIVE_TEXT_OFF, text);
}

/// `-[NSAlert alertStyle]`
pub extern "C" fn shim_nsalert_alert_style(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ALERT_STYLE_OFF)
}

/// `-[NSAlert setAlertStyle:]`
pub extern "C" fn shim_nsalert_set_alert_style(obj: u64, _cmd: u64, style: u64) {
    write_u64(obj, ALERT_STYLE_OFF, style);
}

/// `-[NSAlert addButtonWithTitle:]`
pub extern "C" fn shim_nsalert_add_button_with_title(obj: u64, _cmd: u64, _title: u64) {
    if obj == 0 {
        return;
    }
    let count = read_u64(obj, BUTTON_COUNT_OFF);
    write_u64(obj, BUTTON_COUNT_OFF, count + 1);
}

/// `-[NSAlert runModal]` — returns `NSAlertFirstButtonReturn` (1000).
pub extern "C" fn shim_nsalert_run_modal(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    1000 // NSAlertFirstButtonReturn
}

/// `-[NSAlert window]` — stub, returns 0.
pub extern "C" fn shim_nsalert_window(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSAlert buttons]` — returns 0 (no live NSArray available).
pub extern "C" fn shim_nsalert_buttons(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn message_text_round_trip() {
        let alert = alloc_obj(ALERT_SIZE, 0xA1E8);
        let s = make_string("Something went wrong");
        shim_nsalert_set_message_text(alert, 0, s);
        assert_eq!(shim_nsalert_message_text(alert, 0), s);
        free_obj(alert);
    }

    #[test]
    fn informative_text_round_trip() {
        let alert = alloc_obj(ALERT_SIZE, 0xA1E8);
        assert_eq!(shim_nsalert_informative_text(alert, 0), 0);
        let s = make_string("Details here");
        shim_nsalert_set_informative_text(alert, 0, s);
        assert_eq!(shim_nsalert_informative_text(alert, 0), s);
        free_obj(alert);
    }

    #[test]
    fn alert_style_round_trip() {
        let alert = alloc_obj(ALERT_SIZE, 0xA1E8);
        assert_eq!(shim_nsalert_alert_style(alert, 0), 0); // default warning
        shim_nsalert_set_alert_style(alert, 0, 2); // critical
        assert_eq!(shim_nsalert_alert_style(alert, 0), 2);
        free_obj(alert);
    }

    #[test]
    fn button_count_increments() {
        let alert = alloc_obj(ALERT_SIZE, 0xA1E8);
        assert_eq!(read_field(alert, BUTTON_COUNT_OFF), 0);
        shim_nsalert_add_button_with_title(alert, 0, 0xBB);
        assert_eq!(read_field(alert, BUTTON_COUNT_OFF), 1);
        shim_nsalert_add_button_with_title(alert, 0, 0xCC);
        assert_eq!(read_field(alert, BUTTON_COUNT_OFF), 2);
        free_obj(alert);
    }

    #[test]
    fn run_modal_returns_1000() {
        let alert = alloc_obj(ALERT_SIZE, 0xA1E8);
        assert_eq!(shim_nsalert_run_modal(alert, 0), 1000);
        free_obj(alert);
    }
}
