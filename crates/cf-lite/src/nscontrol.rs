//! NSControl shim: base class for interactive AppKit controls.
//!
//! Object layout:
//!
//!   offset  0: isa               (u64)
//!   offset  8: string_value_ptr  (u64 — NSString*, or 0)
//!   offset 16: target_ptr        (u64 — id, or 0)
//!   offset 24: action_sel        (u64 — SEL addr, or 0)
//!   offset 32: int_value         (i64)
//!   offset 40: flags             (u64 — bit 0: is_enabled, bit 1: is_continuous)
//!
//! Total: 48 bytes.

const STRING_VALUE_OFF: usize = 8;
const TARGET_OFF: usize = 16;
const ACTION_OFF: usize = 24;
const INT_VALUE_OFF: usize = 32;
const FLAGS_OFF: usize = 40;
pub(crate) const CONTROL_SIZE: usize = 48;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_CONTINUOUS: u64 = 1 << 1;

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

fn read_i64(obj: u64, offset: usize) -> i64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const i64) }
}

fn write_i64(obj: u64, offset: usize, value: i64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut i64) = value;
    }
}

/// Allocate a default-enabled NSControl.
pub(crate) fn init_control_defaults(obj: u64) {
    if obj == 0 {
        return;
    }
    // NSControl starts enabled by default
    write_u64(obj, FLAGS_OFF, FLAG_IS_ENABLED);
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSControl isEnabled]`
pub extern "C" fn shim_nscontrol_is_enabled(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_IS_ENABLED != 0)
}

/// `-[NSControl setEnabled:]`
pub extern "C" fn shim_nscontrol_set_enabled(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= FLAG_IS_ENABLED;
    } else {
        flags &= !FLAG_IS_ENABLED;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSControl target]`
pub extern "C" fn shim_nscontrol_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSControl setTarget:]`
pub extern "C" fn shim_nscontrol_set_target(obj: u64, _cmd: u64, target: u64) {
    write_u64(obj, TARGET_OFF, target);
}

/// `-[NSControl action]`
pub extern "C" fn shim_nscontrol_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSControl setAction:]`
pub extern "C" fn shim_nscontrol_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

/// `-[NSControl stringValue]`
pub extern "C" fn shim_nscontrol_string_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ptr = read_u64(obj, STRING_VALUE_OFF);
    if ptr != 0 {
        return ptr;
    }
    // Return empty string if none set
    crate::cfstring::shim_cf_string_create_with_cstring(
        0,
        b"\0".as_ptr().cast(),
        0,
    )
}

/// `-[NSControl setStringValue:]`
pub extern "C" fn shim_nscontrol_set_string_value(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, STRING_VALUE_OFF, value);
}

/// `-[NSControl integerValue]`
pub extern "C" fn shim_nscontrol_integer_value(obj: u64, _cmd: u64) -> i64 {
    read_i64(obj, INT_VALUE_OFF)
}

/// `-[NSControl setIntegerValue:]`
pub extern "C" fn shim_nscontrol_set_integer_value(obj: u64, _cmd: u64, value: i64) {
    write_i64(obj, INT_VALUE_OFF, value);
}

/// `-[NSControl doubleValue]`
pub extern "C" fn shim_nscontrol_double_value(obj: u64, _cmd: u64) -> f64 {
    read_i64(obj, INT_VALUE_OFF) as f64
}

/// `-[NSControl setDoubleValue:]`
pub extern "C" fn shim_nscontrol_set_double_value(obj: u64, _cmd: u64, value: f64) {
    write_i64(obj, INT_VALUE_OFF, value as i64);
}

/// `-[NSControl objectValue]` — alias for stringValue.
pub extern "C" fn shim_nscontrol_object_value(obj: u64, _cmd: u64) -> u64 {
    shim_nscontrol_string_value(obj, _cmd)
}

/// `-[NSControl setObjectValue:]` — alias for setStringValue.
pub extern "C" fn shim_nscontrol_set_object_value(obj: u64, _cmd: u64, value: u64) {
    shim_nscontrol_set_string_value(obj, _cmd, value);
}

/// `-[NSControl isContinuous]`
pub extern "C" fn shim_nscontrol_is_continuous(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    u8::from(flags & FLAG_IS_CONTINUOUS != 0)
}

/// `-[NSControl setContinuous:]`
pub extern "C" fn shim_nscontrol_set_continuous(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        flags |= FLAG_IS_CONTINUOUS;
    } else {
        flags &= !FLAG_IS_CONTINUOUS;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSControl sendAction:to:]` — no-op stub (would need dispatch).
pub extern "C" fn shim_nscontrol_send_action(_obj: u64, _cmd: u64, _action: u64, _target: u64) -> u8 {
    1 // pretend success
}

/// `-[NSControl sizeToFit]` — no-op stub.
pub extern "C" fn shim_nscontrol_size_to_fit(_obj: u64, _cmd: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nscontrol_is_enabled(0, 0), 0);
        shim_nscontrol_set_enabled(0, 0, 1);
        assert_eq!(shim_nscontrol_target(0, 0), 0);
        shim_nscontrol_set_target(0, 0, 0x1234);
        assert_eq!(shim_nscontrol_action(0, 0), 0);
        shim_nscontrol_set_action(0, 0, 0x1234);
        assert_eq!(shim_nscontrol_string_value(0, 0), 0);
        shim_nscontrol_set_string_value(0, 0, 0x1234);
        assert_eq!(shim_nscontrol_integer_value(0, 0), 0);
        shim_nscontrol_set_integer_value(0, 0, 42);
        assert_eq!(shim_nscontrol_is_continuous(0, 0), 0);
        shim_nscontrol_set_continuous(0, 0, 1);
        shim_nscontrol_size_to_fit(0, 0);
    }

    #[test]
    fn enabled_toggle() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        init_control_defaults(ctrl);
        assert_eq!(shim_nscontrol_is_enabled(ctrl, 0), 1);
        shim_nscontrol_set_enabled(ctrl, 0, 0);
        assert_eq!(shim_nscontrol_is_enabled(ctrl, 0), 0);
        shim_nscontrol_set_enabled(ctrl, 0, 1);
        assert_eq!(shim_nscontrol_is_enabled(ctrl, 0), 1);
        free_obj(ctrl);
    }

    #[test]
    fn target_action_round_trip() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        shim_nscontrol_set_target(ctrl, 0, 0xAABB);
        shim_nscontrol_set_action(ctrl, 0, 0xCCDD);
        assert_eq!(shim_nscontrol_target(ctrl, 0), 0xAABB);
        assert_eq!(shim_nscontrol_action(ctrl, 0), 0xCCDD);
        free_obj(ctrl);
    }

    #[test]
    fn string_value_round_trip() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        let s = make_string("Hello");
        shim_nscontrol_set_string_value(ctrl, 0, s);
        assert_eq!(shim_nscontrol_string_value(ctrl, 0), s);
        free_obj(ctrl);
    }

    #[test]
    fn integer_value_round_trip() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        shim_nscontrol_set_integer_value(ctrl, 0, -42);
        assert_eq!(shim_nscontrol_integer_value(ctrl, 0), -42);
        free_obj(ctrl);
    }

    #[test]
    fn double_value_round_trip() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        shim_nscontrol_set_double_value(ctrl, 0, 3.0);
        assert_eq!(shim_nscontrol_double_value(ctrl, 0), 3.0);
        free_obj(ctrl);
    }

    #[test]
    fn continuous_toggle() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        assert_eq!(shim_nscontrol_is_continuous(ctrl, 0), 0);
        shim_nscontrol_set_continuous(ctrl, 0, 1);
        assert_eq!(shim_nscontrol_is_continuous(ctrl, 0), 1);
        shim_nscontrol_set_continuous(ctrl, 0, 0);
        assert_eq!(shim_nscontrol_is_continuous(ctrl, 0), 0);
        free_obj(ctrl);
    }

    #[test]
    fn send_action_returns_success() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        assert_eq!(shim_nscontrol_send_action(ctrl, 0, 0x1234, 0x5678), 1);
        free_obj(ctrl);
    }

    #[test]
    fn object_value_aliases_string_value() {
        let ctrl = alloc_obj(CONTROL_SIZE, 0xDEAD);
        let s = make_string("test");
        shim_nscontrol_set_object_value(ctrl, 0, s);
        assert_eq!(shim_nscontrol_object_value(ctrl, 0), s);
        free_obj(ctrl);
    }
}
