//! NSEvent shim: event stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa             (u64)
//!   offset  8: event_type      (u64)
//!   offset 16: location_x      (f64)
//!   offset 24: location_y      (f64)
//!   offset 32: modifier_flags  (u64)
//!   offset 40: key_code        (u64)
//!   offset 48: characters_ptr  (u64 — NSString*)
//!   offset 56: window_ptr      (u64 — NSWindow*)
//!
//! Total: 64 bytes.

const EVENT_TYPE_OFF: usize = 8;
const LOCATION_X_OFF: usize = 16;
const MODIFIER_FLAGS_OFF: usize = 32;
const KEY_CODE_OFF: usize = 40;
const CHARACTERS_OFF: usize = 48;
const WINDOW_OFF: usize = 56;

const LEFT_MOUSE_DOWN_TYPE: u64 = 1;
const LEFT_MOUSE_UP_TYPE: u64 = 2;
const RIGHT_MOUSE_DOWN_TYPE: u64 = 3;
const RIGHT_MOUSE_UP_TYPE: u64 = 4;
const MOUSE_MOVED_TYPE: u64 = 5;
const LEFT_MOUSE_DRAGGED_TYPE: u64 = 6;
const RIGHT_MOUSE_DRAGGED_TYPE: u64 = 7;
const KEY_DOWN_TYPE: u64 = 10;
const KEY_UP_TYPE: u64 = 11;
const FLAGS_CHANGED_TYPE: u64 = 12;
const SCROLL_WHEEL_TYPE: u64 = 22;
const OTHER_MOUSE_DOWN_TYPE: u64 = 25;
const OTHER_MOUSE_UP_TYPE: u64 = 26;
const OTHER_MOUSE_DRAGGED_TYPE: u64 = 27;

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

pub(crate) fn event_window(obj: u64) -> u64 {
    read_u64(obj, WINDOW_OFF)
}

pub(crate) fn set_event_window(obj: u64, window: u64) {
    write_u64(obj, WINDOW_OFF, window);
}

pub(crate) fn event_dispatch_selector(obj: u64) -> Option<&'static str> {
    match shim_nsevent_type(obj, 0) {
        KEY_DOWN_TYPE => Some("keyDown:"),
        KEY_UP_TYPE => Some("keyUp:"),
        FLAGS_CHANGED_TYPE => Some("flagsChanged:"),
        LEFT_MOUSE_DOWN_TYPE | RIGHT_MOUSE_DOWN_TYPE | OTHER_MOUSE_DOWN_TYPE => Some("mouseDown:"),
        LEFT_MOUSE_UP_TYPE | RIGHT_MOUSE_UP_TYPE | OTHER_MOUSE_UP_TYPE => Some("mouseUp:"),
        MOUSE_MOVED_TYPE | LEFT_MOUSE_DRAGGED_TYPE | RIGHT_MOUSE_DRAGGED_TYPE | OTHER_MOUSE_DRAGGED_TYPE => Some("mouseMoved:"),
        SCROLL_WHEEL_TYPE => Some("scrollWheel:"),
        _ => None,
    }
}

pub(crate) fn event_targets_first_responder(obj: u64) -> bool {
    matches!(
        shim_nsevent_type(obj, 0),
        KEY_DOWN_TYPE | KEY_UP_TYPE | FLAGS_CHANGED_TYPE
    )
}

/// `-[NSEvent type]`
pub extern "C" fn shim_nsevent_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + EVENT_TYPE_OFF) as *const u64) }
}

/// `-[NSEvent locationInWindow]` — returns x coordinate as f64 stub.
pub extern "C" fn shim_nsevent_location_in_window(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + LOCATION_X_OFF) as *const f64) }
}

/// `-[NSEvent modifierFlags]`
pub extern "C" fn shim_nsevent_modifier_flags(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + MODIFIER_FLAGS_OFF) as *const u64) }
}

/// `-[NSEvent keyCode]`
pub extern "C" fn shim_nsevent_key_code(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + KEY_CODE_OFF) as *const u64) }
}

/// `-[NSEvent characters]`
pub extern "C" fn shim_nsevent_characters(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CHARACTERS_OFF) as *const u64) }
}

/// `-[NSEvent charactersIgnoringModifiers]` — same as characters for stub.
pub extern "C" fn shim_nsevent_characters_ignoring_modifiers(obj: u64, _cmd: u64) -> u64 {
    shim_nsevent_characters(obj, _cmd)
}

/// `-[NSEvent window]`
pub extern "C" fn shim_nsevent_window(obj: u64, _cmd: u64) -> u64 {
    event_window(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = 64;

    fn make_event() -> u64 {
        alloc_obj(SIZE, crate::nsobj::nsevent_class_addr())
    }

    // --- shim_nsevent_type ---

    #[test]
    fn type_returns_event_type() {
        let obj = make_event();
        write_field(obj, EVENT_TYPE_OFF, 3);
        assert_eq!(shim_nsevent_type(obj, 0), 3);
        free_obj(obj);
    }

    #[test]
    fn type_nil_returns_zero() {
        assert_eq!(shim_nsevent_type(0, 0), 0);
    }

    // --- shim_nsevent_location_in_window ---

    #[test]
    fn location_in_window_returns_x() {
        let obj = make_event();
        let x: f64 = 42.5;
        write_field(obj, LOCATION_X_OFF, x.to_bits());
        let result = shim_nsevent_location_in_window(obj, 0);
        assert!((result - 42.5).abs() < f64::EPSILON);
        free_obj(obj);
    }

    #[test]
    fn location_in_window_nil_returns_zero() {
        assert!((shim_nsevent_location_in_window(0, 0)).abs() < f64::EPSILON);
    }

    #[test]
    fn location_in_window_negative() {
        let obj = make_event();
        let x: f64 = -100.25;
        write_field(obj, LOCATION_X_OFF, x.to_bits());
        let result = shim_nsevent_location_in_window(obj, 0);
        assert!((result - (-100.25)).abs() < f64::EPSILON);
        free_obj(obj);
    }

    // --- shim_nsevent_modifier_flags ---

    #[test]
    fn modifier_flags_returns_flags() {
        let obj = make_event();
        write_field(obj, MODIFIER_FLAGS_OFF, 0x00040000);
        assert_eq!(shim_nsevent_modifier_flags(obj, 0), 0x00040000);
        free_obj(obj);
    }

    #[test]
    fn modifier_flags_nil_returns_zero() {
        assert_eq!(shim_nsevent_modifier_flags(0, 0), 0);
    }

    // --- shim_nsevent_key_code ---

    #[test]
    fn key_code_returns_code() {
        let obj = make_event();
        write_field(obj, KEY_CODE_OFF, 36);
        assert_eq!(shim_nsevent_key_code(obj, 0), 36);
        free_obj(obj);
    }

    #[test]
    fn key_code_nil_returns_zero() {
        assert_eq!(shim_nsevent_key_code(0, 0), 0);
    }

    // --- shim_nsevent_characters ---

    #[test]
    fn characters_returns_string_ptr() {
        let obj = make_event();
        let s = make_string("hello");
        write_field(obj, CHARACTERS_OFF, s);
        let result = shim_nsevent_characters(obj, 0);
        assert_eq!(result, s);
        assert_string_eq(result, "hello");
        free_obj(s);
        free_obj(obj);
    }

    #[test]
    fn characters_nil_returns_zero() {
        assert_eq!(shim_nsevent_characters(0, 0), 0);
    }

    #[test]
    fn characters_empty_string() {
        let obj = make_event();
        let s = make_string("");
        write_field(obj, CHARACTERS_OFF, s);
        let result = shim_nsevent_characters(obj, 0);
        assert_string_eq(result, "");
        free_obj(s);
        free_obj(obj);
    }

    // --- shim_nsevent_characters_ignoring_modifiers ---

    #[test]
    fn characters_ignoring_modifiers_delegates() {
        let obj = make_event();
        let s = make_string("A");
        write_field(obj, CHARACTERS_OFF, s);
        let result = shim_nsevent_characters_ignoring_modifiers(obj, 0);
        assert_eq!(result, s);
        assert_string_eq(result, "A");
        free_obj(s);
        free_obj(obj);
    }

    #[test]
    fn characters_ignoring_modifiers_nil_returns_zero() {
        assert_eq!(shim_nsevent_characters_ignoring_modifiers(0, 0), 0);
    }

    // --- shim_nsevent_window ---

    #[test]
    fn window_returns_assigned_window_pointer() {
        let obj = make_event();
        write_field(obj, WINDOW_OFF, 0xCAFE);
        assert_eq!(shim_nsevent_window(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    #[test]
    fn window_nil_returns_zero() {
        assert_eq!(shim_nsevent_window(0, 0), 0);
    }

    #[test]
    fn set_event_window_updates_window_slot() {
        let obj = make_event();
        set_event_window(obj, 0xD00D);
        assert_eq!(shim_nsevent_window(obj, 0), 0xD00D);
        free_obj(obj);
    }

    #[test]
    fn event_dispatch_selector_maps_supported_types() {
        let obj = make_event();
        write_field(obj, EVENT_TYPE_OFF, KEY_DOWN_TYPE);
        assert_eq!(event_dispatch_selector(obj), Some("keyDown:"));
        assert!(event_targets_first_responder(obj));

        write_field(obj, EVENT_TYPE_OFF, LEFT_MOUSE_DOWN_TYPE);
        assert_eq!(event_dispatch_selector(obj), Some("mouseDown:"));
        assert!(!event_targets_first_responder(obj));

        free_obj(obj);
    }

    #[test]
    fn event_dispatch_selector_returns_none_for_unknown_type() {
        let obj = make_event();
        write_field(obj, EVENT_TYPE_OFF, 999);
        assert_eq!(event_dispatch_selector(obj), None);
        assert!(!event_targets_first_responder(obj));
        free_obj(obj);
    }

    // --- edge: zero-initialized event ---

    #[test]
    fn zeroed_event_returns_defaults() {
        let obj = make_event();
        assert_eq!(shim_nsevent_type(obj, 0), 0);
        assert!((shim_nsevent_location_in_window(obj, 0)).abs() < f64::EPSILON);
        assert_eq!(shim_nsevent_modifier_flags(obj, 0), 0);
        assert_eq!(shim_nsevent_key_code(obj, 0), 0);
        assert_eq!(shim_nsevent_characters(obj, 0), 0);
        assert_eq!(shim_nsevent_window(obj, 0), 0);
        free_obj(obj);
    }
}
