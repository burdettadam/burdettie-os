//! NSButton shim: push-button / checkbox control for AppKit.
//!
//! Object layout (56 bytes, inherits NSControl at 48):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: title_ptr            (u64 — NSString*)
//!   offset 16: alternate_title_ptr  (u64 — NSString*)
//!   offset 24: image_ptr            (u64 — NSImage*)
//!   offset 32: target_ptr           (u64)
//!   offset 40: action_sel           (u64 — SEL)
//!   offset 48: flags                (u64 — bit 0: is_enabled, bit 1: is_bordered,
//!                                           bit 2: allows_mixed_state, bit 3: state_on)
//!
//! Total: 56 bytes.

const TITLE_OFF: usize = 8;
const ALT_TITLE_OFF: usize = 16;
const IMAGE_OFF: usize = 24;
const TARGET_OFF: usize = 32;
const ACTION_OFF: usize = 40;
const FLAGS_OFF: usize = 48;
pub(crate) const BUTTON_SIZE: usize = 56;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_BORDERED: u64 = 1 << 1;
const FLAG_ALLOWS_MIXED_STATE: u64 = 1 << 2;
const FLAG_STATE_ON: u64 = 1 << 3;

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

fn read_flags(obj: u64) -> u64 {
    read_u64(obj, FLAGS_OFF)
}

fn write_flags(obj: u64, flags: u64) {
    write_u64(obj, FLAGS_OFF, flags);
}

fn set_flag(obj: u64, flag: u64, val: bool) {
    if obj == 0 {
        return;
    }
    let mut flags = read_flags(obj);
    if val {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_flags(obj, flags);
}

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSButton title]`
pub extern "C" fn shim_nsbutton_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, TITLE_OFF);
    if ptr != 0 {
        return ptr;
    }
    // Return empty string if none set
    crate::cfstring::shim_cf_string_create_with_cstring(0, b"\0".as_ptr().cast(), 0)
}

/// `-[NSButton setTitle:]`
pub extern "C" fn shim_nsbutton_set_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, TITLE_OFF, title);
}

/// `-[NSButton alternateTitle]`
pub extern "C" fn shim_nsbutton_alternate_title(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ALT_TITLE_OFF)
}

/// `-[NSButton setAlternateTitle:]`
pub extern "C" fn shim_nsbutton_set_alternate_title(obj: u64, _cmd: u64, title: u64) {
    write_u64(obj, ALT_TITLE_OFF, title);
}

/// `-[NSButton image]`
pub extern "C" fn shim_nsbutton_image(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IMAGE_OFF)
}

/// `-[NSButton setImage:]`
pub extern "C" fn shim_nsbutton_set_image(obj: u64, _cmd: u64, img: u64) {
    write_u64(obj, IMAGE_OFF, img);
}

/// `-[NSButton target]`
pub extern "C" fn shim_nsbutton_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSButton setTarget:]`
pub extern "C" fn shim_nsbutton_set_target(obj: u64, _cmd: u64, tgt: u64) {
    write_u64(obj, TARGET_OFF, tgt);
}

/// `-[NSButton action]`
pub extern "C" fn shim_nsbutton_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSButton setAction:]`
pub extern "C" fn shim_nsbutton_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

/// `-[NSButton state]`
pub extern "C" fn shim_nsbutton_state(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    if read_flags(obj) & FLAG_STATE_ON != 0 {
        1
    } else {
        0
    }
}

/// `-[NSButton setState:]`
pub extern "C" fn shim_nsbutton_set_state(obj: u64, _cmd: u64, state: i64) {
    set_flag(obj, FLAG_STATE_ON, state != 0);
}

/// `-[NSButton isEnabled]`
pub extern "C" fn shim_nsbutton_is_enabled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_ENABLED)
}

/// `-[NSButton setEnabled:]`
pub extern "C" fn shim_nsbutton_set_enabled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_ENABLED, val != 0);
}

/// `-[NSButton isBordered]`
pub extern "C" fn shim_nsbutton_is_bordered(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BORDERED)
}

/// `-[NSButton setBordered:]`
pub extern "C" fn shim_nsbutton_set_bordered(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BORDERED, val != 0);
}

/// `-[NSButton allowsMixedState]`
pub extern "C" fn shim_nsbutton_allows_mixed_state(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_ALLOWS_MIXED_STATE)
}

/// `-[NSButton setAllowsMixedState:]`
pub extern "C" fn shim_nsbutton_set_allows_mixed_state(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_ALLOWS_MIXED_STATE, val != 0);
}

/// `-[NSButton setButtonType:]` — no-op stub.
pub extern "C" fn shim_nsbutton_set_button_type(_obj: u64, _cmd: u64, _type: u64) {}

/// `-[NSButton setBezelStyle:]` — no-op stub.
pub extern "C" fn shim_nsbutton_set_bezel_style(_obj: u64, _cmd: u64, _style: u64) {}

/// `-[NSButton highlight:]` — no-op stub.
pub extern "C" fn shim_nsbutton_highlight(_obj: u64, _cmd: u64, _flag: u8) {}

/// `-[NSButton performClick:]` — no-op stub.
pub extern "C" fn shim_nsbutton_perform_click(_obj: u64, _cmd: u64, _sender: u64) {}

/// `-[NSButton sizeToFit]` — no-op stub.
pub extern "C" fn shim_nsbutton_size_to_fit(_obj: u64, _cmd: u64) {}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSButton buttonWithTitle:target:action:]`
pub extern "C" fn shim_nsbutton_button_with_title(
    _cls: u64,
    _cmd: u64,
    title: u64,
    target: u64,
    action: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BUTTON_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbutton_class_addr();
    }
    write_u64(obj, TITLE_OFF, title);
    write_u64(obj, TARGET_OFF, target);
    write_u64(obj, ACTION_OFF, action);
    // Enabled + bordered by default
    write_flags(obj, FLAG_IS_ENABLED | FLAG_IS_BORDERED);
    obj
}

/// `+[NSButton checkboxWithTitle:target:action:]`
pub extern "C" fn shim_nsbutton_checkbox_with_title(
    _cls: u64,
    _cmd: u64,
    title: u64,
    target: u64,
    action: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, BUTTON_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbutton_class_addr();
    }
    write_u64(obj, TITLE_OFF, title);
    write_u64(obj, TARGET_OFF, target);
    write_u64(obj, ACTION_OFF, action);
    // Enabled but NOT bordered for checkbox
    write_flags(obj, FLAG_IS_ENABLED);
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsbutton_title(0, 0), 0);
        shim_nsbutton_set_title(0, 0, 0x1234);
        assert_eq!(shim_nsbutton_alternate_title(0, 0), 0);
        assert_eq!(shim_nsbutton_image(0, 0), 0);
        assert_eq!(shim_nsbutton_target(0, 0), 0);
        assert_eq!(shim_nsbutton_action(0, 0), 0);
        assert_eq!(shim_nsbutton_state(0, 0), 0);
        shim_nsbutton_set_state(0, 0, 1);
        assert_eq!(shim_nsbutton_is_enabled(0, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(0, 0), 0);
        assert_eq!(shim_nsbutton_allows_mixed_state(0, 0), 0);
    }

    #[test]
    fn title_round_trip() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        let title = make_string("Click Me");
        shim_nsbutton_set_title(btn, 0, title);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(read_string(title), "Click Me");
        free_obj(btn);
    }

    #[test]
    fn title_returns_empty_when_unset() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        let t = shim_nsbutton_title(btn, 0);
        assert_ne!(t, 0, "should return an empty string object, not nil");
        assert_eq!(read_string(t), "");
        free_obj(btn);
    }

    #[test]
    fn state_toggle() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        assert_eq!(shim_nsbutton_state(btn, 0), 0);
        shim_nsbutton_set_state(btn, 0, 1);
        assert_eq!(shim_nsbutton_state(btn, 0), 1);
        shim_nsbutton_set_state(btn, 0, 0);
        assert_eq!(shim_nsbutton_state(btn, 0), 0);
        free_obj(btn);
    }

    #[test]
    fn enabled_bordered_flags() {
        let btn = alloc_obj(BUTTON_SIZE, 0xDEAD);
        // Initially all flags are 0
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 0);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 0);

        shim_nsbutton_set_enabled(btn, 0, 1);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        shim_nsbutton_set_bordered(btn, 0, 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        shim_nsbutton_set_allows_mixed_state(btn, 0, 1);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 1);

        // Toggling one flag should not affect others
        shim_nsbutton_set_enabled(btn, 0, 0);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 0);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        assert_eq!(shim_nsbutton_allows_mixed_state(btn, 0), 1);
        free_obj(btn);
    }

    #[test]
    fn button_with_title_factory() {
        let title = make_string("OK");
        let target = 0xAAAA_u64;
        let action = 0xBBBB_u64;
        let btn = shim_nsbutton_button_with_title(0, 0, title, target, action);
        assert_ne!(btn, 0);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(shim_nsbutton_target(btn, 0), target);
        assert_eq!(shim_nsbutton_action(btn, 0), action);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 1);
        free_obj(btn);
    }

    #[test]
    fn checkbox_with_title_factory() {
        let title = make_string("Accept Terms");
        let btn = shim_nsbutton_checkbox_with_title(0, 0, title, 0, 0);
        assert_ne!(btn, 0);
        assert_eq!(shim_nsbutton_title(btn, 0), title);
        assert_eq!(shim_nsbutton_is_enabled(btn, 0), 1);
        assert_eq!(shim_nsbutton_is_bordered(btn, 0), 0);
        free_obj(btn);
    }
}
