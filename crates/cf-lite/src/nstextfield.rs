//! NSTextField shim: text label and input control.
//!
//! Object layout:
//!
//!   offset  0: isa               (u64)
//!   offset  8: string_value_ptr  (u64 — NSString*, or 0)
//!   offset 16: target_ptr        (u64 — id, or 0)
//!   offset 24: action_sel        (u64 — SEL addr, or 0)
//!   offset 32: placeholder_ptr   (u64 — NSString*, or 0)
//!   offset 40: flags             (u64 — bit 0: is_enabled, bit 1: is_editable,
//!                                        bit 2: is_bezeled, bit 3: draws_background,
//!                                        bit 4: is_selectable)
//!
//! Total: 48 bytes.

const STRING_VALUE_OFF: usize = 8;
const TARGET_OFF: usize = 16;
const ACTION_OFF: usize = 24;
const PLACEHOLDER_OFF: usize = 32;
const FLAGS_OFF: usize = 40;
pub(crate) const TEXTFIELD_SIZE: usize = 48;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_EDITABLE: u64 = 1 << 1;
const FLAG_IS_BEZELED: u64 = 1 << 2;
const FLAG_DRAWS_BACKGROUND: u64 = 1 << 3;
const FLAG_IS_SELECTABLE: u64 = 1 << 4;

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

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & flag != 0)
}

fn set_flag(obj: u64, flag: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// Set up a text field with label defaults (non-editable, non-bezeled, no bg).
fn init_as_label(obj: u64, string_value: u64) {
    write_u64(obj, STRING_VALUE_OFF, string_value);
    // Label: enabled + selectable, but NOT editable, NOT bezeled, NO bg
    write_u64(obj, FLAGS_OFF, FLAG_IS_ENABLED | FLAG_IS_SELECTABLE);
}

/// Set up a text field with editable defaults.
fn init_as_editable(obj: u64, string_value: u64) {
    write_u64(obj, STRING_VALUE_OFF, string_value);
    // Editable: enabled + editable + bezeled + draws background + selectable
    write_u64(obj, FLAGS_OFF, FLAG_IS_ENABLED | FLAG_IS_EDITABLE | FLAG_IS_BEZELED | FLAG_DRAWS_BACKGROUND | FLAG_IS_SELECTABLE);
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSTextField stringValue]`
pub extern "C" fn shim_nstextfield_string_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, STRING_VALUE_OFF);
    if ptr != 0 {
        return ptr;
    }
    crate::cfstring::shim_cf_string_create_with_cstring(0, b"\0".as_ptr().cast(), 0)
}

/// `-[NSTextField setStringValue:]`
pub extern "C" fn shim_nstextfield_set_string_value(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, STRING_VALUE_OFF, value);
}

/// `-[NSTextField isEditable]`
pub extern "C" fn shim_nstextfield_is_editable(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_EDITABLE)
}

/// `-[NSTextField setEditable:]`
pub extern "C" fn shim_nstextfield_set_editable(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_EDITABLE, val);
}

/// `-[NSTextField isBezeled]`
pub extern "C" fn shim_nstextfield_is_bezeled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BEZELED)
}

/// `-[NSTextField setBezeled:]`
pub extern "C" fn shim_nstextfield_set_bezeled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BEZELED, val);
}

/// `-[NSTextField drawsBackground]`
pub extern "C" fn shim_nstextfield_draws_background(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_DRAWS_BACKGROUND)
}

/// `-[NSTextField setDrawsBackground:]`
pub extern "C" fn shim_nstextfield_set_draws_background(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_DRAWS_BACKGROUND, val);
}

/// `-[NSTextField isSelectable]`
pub extern "C" fn shim_nstextfield_is_selectable(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_SELECTABLE)
}

/// `-[NSTextField setSelectable:]`
pub extern "C" fn shim_nstextfield_set_selectable(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_SELECTABLE, val);
}

/// `-[NSTextField isEnabled]`
pub extern "C" fn shim_nstextfield_is_enabled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_ENABLED)
}

/// `-[NSTextField setEnabled:]`
pub extern "C" fn shim_nstextfield_set_enabled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_ENABLED, val);
}

/// `-[NSTextField placeholderString]`
pub extern "C" fn shim_nstextfield_placeholder_string(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, PLACEHOLDER_OFF)
}

/// `-[NSTextField setPlaceholderString:]`
pub extern "C" fn shim_nstextfield_set_placeholder_string(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, PLACEHOLDER_OFF, value);
}

/// `-[NSTextField target]`
pub extern "C" fn shim_nstextfield_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSTextField setTarget:]`
pub extern "C" fn shim_nstextfield_set_target(obj: u64, _cmd: u64, target: u64) {
    write_u64(obj, TARGET_OFF, target);
}

/// `-[NSTextField action]`
pub extern "C" fn shim_nstextfield_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSTextField setAction:]`
pub extern "C" fn shim_nstextfield_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSTextField labelWithString:]` — creates a non-editable label.
pub extern "C" fn shim_nstextfield_label_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TEXTFIELD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstextfield_class_addr();
    }
    init_as_label(obj, string);
    obj
}

/// `+[NSTextField wrappingLabelWithString:]` — creates a wrapping label.
pub extern "C" fn shim_nstextfield_wrapping_label_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    // Same as labelWithString: for our purposes
    shim_nstextfield_label_with_string(_cls, _cmd, string)
}

/// `+[NSTextField textFieldWithString:]` — creates an editable text field.
pub extern "C" fn shim_nstextfield_text_field_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TEXTFIELD_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstextfield_class_addr();
    }
    init_as_editable(obj, string);
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstextfield_string_value(0, 0), 0);
        shim_nstextfield_set_string_value(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_is_editable(0, 0), 0);
        shim_nstextfield_set_editable(0, 0, 1);
        assert_eq!(shim_nstextfield_is_bezeled(0, 0), 0);
        shim_nstextfield_set_bezeled(0, 0, 1);
        assert_eq!(shim_nstextfield_draws_background(0, 0), 0);
        shim_nstextfield_set_draws_background(0, 0, 1);
        assert_eq!(shim_nstextfield_is_selectable(0, 0), 0);
        shim_nstextfield_set_selectable(0, 0, 1);
        assert_eq!(shim_nstextfield_is_enabled(0, 0), 0);
        shim_nstextfield_set_enabled(0, 0, 1);
        assert_eq!(shim_nstextfield_placeholder_string(0, 0), 0);
        shim_nstextfield_set_placeholder_string(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_target(0, 0), 0);
        shim_nstextfield_set_target(0, 0, 0x1234);
        assert_eq!(shim_nstextfield_action(0, 0), 0);
        shim_nstextfield_set_action(0, 0, 0x1234);
    }

    #[test]
    fn label_factory() {
        let s = make_string("Hello");
        let tf = shim_nstextfield_label_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        // Label: enabled, selectable, NOT editable, NOT bezeled, NO bg
        assert_eq!(shim_nstextfield_is_enabled(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_selectable(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 0);
        assert_eq!(shim_nstextfield_draws_background(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn editable_factory() {
        let s = make_string("Edit me");
        let tf = shim_nstextfield_text_field_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        // Editable: all flags on
        assert_eq!(shim_nstextfield_is_enabled(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 1);
        assert_eq!(shim_nstextfield_draws_background(tf, 0), 1);
        assert_eq!(shim_nstextfield_is_selectable(tf, 0), 1);
        free_obj(tf);
    }

    #[test]
    fn string_value_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        let s = make_string("test");
        shim_nstextfield_set_string_value(tf, 0, s);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        free_obj(tf);
    }

    #[test]
    fn placeholder_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        let p = make_string("Enter text...");
        shim_nstextfield_set_placeholder_string(tf, 0, p);
        assert_eq!(shim_nstextfield_placeholder_string(tf, 0), p);
        free_obj(tf);
    }

    #[test]
    fn editable_toggle() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        shim_nstextfield_set_editable(tf, 0, 1);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 1);
        shim_nstextfield_set_editable(tf, 0, 0);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn bezeled_toggle() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        shim_nstextfield_set_bezeled(tf, 0, 1);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 1);
        shim_nstextfield_set_bezeled(tf, 0, 0);
        assert_eq!(shim_nstextfield_is_bezeled(tf, 0), 0);
        free_obj(tf);
    }

    #[test]
    fn target_action_round_trip() {
        let tf = alloc_obj(TEXTFIELD_SIZE, 0xDEAD);
        shim_nstextfield_set_target(tf, 0, 0xAABB);
        shim_nstextfield_set_action(tf, 0, 0xCCDD);
        assert_eq!(shim_nstextfield_target(tf, 0), 0xAABB);
        assert_eq!(shim_nstextfield_action(tf, 0), 0xCCDD);
        free_obj(tf);
    }

    #[test]
    fn wrapping_label_same_as_label() {
        let s = make_string("Wrap");
        let tf = shim_nstextfield_wrapping_label_with_string(0, 0, s);
        assert_ne!(tf, 0);
        assert_eq!(shim_nstextfield_string_value(tf, 0), s);
        assert_eq!(shim_nstextfield_is_editable(tf, 0), 0);
        free_obj(tf);
    }
}
