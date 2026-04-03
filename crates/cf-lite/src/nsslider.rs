//! NSSlider shim: horizontal/vertical slider control for AppKit.
//!
//! Object layout (64 bytes, inherits NSControl at 48):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: current_value_bits   (u64 — raw f64 bits)
//!   offset 16: min_value_bits       (u64 — raw f64 bits)
//!   offset 24: max_value_bits       (u64 — raw f64 bits)
//!   offset 32: flags                (u64 — bit 0: continuous,
//!                                          bit 1: vertical_orientation)
//!   offset 40: reserved / padding   (u64)
//!   offset 48: reserved / padding   (u64)
//!   offset 56: reserved / padding   (u64)
//!
//! Total: 64 bytes.

const CURRENT_VALUE_OFF: usize = 8;
const MIN_VALUE_OFF: usize = 16;
const MAX_VALUE_OFF: usize = 24;
const FLAGS_OFF: usize = 32;
pub(crate) const SLIDER_SIZE: usize = 64;

const FLAG_CONTINUOUS: u64 = 1 << 0;
const FLAG_VERTICAL: u64 = 1 << 1;

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

fn read_f64(obj: u64, offset: usize) -> f64 {
    f64::from_bits(read_u64(obj, offset))
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    write_u64(obj, offset, value.to_bits());
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

// ── Instance method shims ──────────────────────────────────────

/// `-[NSSlider doubleValue]`
pub extern "C" fn shim_nsslider_double_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, CURRENT_VALUE_OFF)
}

/// `-[NSSlider setDoubleValue:]`
pub extern "C" fn shim_nsslider_set_double_value(obj: u64, _cmd: u64, value: f64) {
    write_f64(obj, CURRENT_VALUE_OFF, value);
}

/// `-[NSSlider minValue]`
pub extern "C" fn shim_nsslider_min_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MIN_VALUE_OFF)
}

/// `-[NSSlider setMinValue:]`
pub extern "C" fn shim_nsslider_set_min_value(obj: u64, _cmd: u64, value: f64) {
    write_f64(obj, MIN_VALUE_OFF, value);
}

/// `-[NSSlider maxValue]`
pub extern "C" fn shim_nsslider_max_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MAX_VALUE_OFF)
}

/// `-[NSSlider setMaxValue:]`
pub extern "C" fn shim_nsslider_set_max_value(obj: u64, _cmd: u64, value: f64) {
    write_f64(obj, MAX_VALUE_OFF, value);
}

/// `-[NSSlider isContinuous]`
pub extern "C" fn shim_nsslider_is_continuous(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & FLAG_CONTINUOUS != 0)
}

/// `-[NSSlider setContinuous:]`
pub extern "C" fn shim_nsslider_set_continuous(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_CONTINUOUS, val != 0);
}

/// `-[NSSlider orientation]`
pub extern "C" fn shim_nsslider_orientation(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    u64::from(read_flags(obj) & FLAG_VERTICAL != 0)
}

/// `-[NSSlider setOrientation:]`
pub extern "C" fn shim_nsslider_set_orientation(obj: u64, _cmd: u64, orientation: u64) {
    set_flag(obj, FLAG_VERTICAL, orientation != 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsslider_double_value(0, 0), 0.0);
        shim_nsslider_set_double_value(0, 0, 12.5);
        assert_eq!(shim_nsslider_min_value(0, 0), 0.0);
        shim_nsslider_set_min_value(0, 0, -5.0);
        assert_eq!(shim_nsslider_max_value(0, 0), 0.0);
        shim_nsslider_set_max_value(0, 0, 99.0);
        assert_eq!(shim_nsslider_is_continuous(0, 0), 0);
        shim_nsslider_set_continuous(0, 0, 1);
        assert_eq!(shim_nsslider_orientation(0, 0), 0);
        shim_nsslider_set_orientation(0, 0, 1);
    }

    #[test]
    fn double_value_round_trip_preserves_bits() {
        let slider = alloc_obj_fake(SLIDER_SIZE);
        let value = -42.625_f64;

        shim_nsslider_set_double_value(slider, 0, value);

        assert_eq!(shim_nsslider_double_value(slider, 0), value);
        assert_eq!(read_field(slider, CURRENT_VALUE_OFF), value.to_bits());
        free_obj(slider);
    }

    #[test]
    fn min_max_round_trip_preserve_bits() {
        let slider = alloc_obj_fake(SLIDER_SIZE);
        let min = -10.5_f64;
        let max = 250.25_f64;

        shim_nsslider_set_min_value(slider, 0, min);
        shim_nsslider_set_max_value(slider, 0, max);

        assert_eq!(shim_nsslider_min_value(slider, 0), min);
        assert_eq!(shim_nsslider_max_value(slider, 0), max);
        assert_eq!(f64::from_bits(read_field(slider, MIN_VALUE_OFF)), min);
        assert_eq!(f64::from_bits(read_field(slider, MAX_VALUE_OFF)), max);
        free_obj(slider);
    }

    #[test]
    fn current_min_and_max_are_independent() {
        let slider = alloc_obj_fake(SLIDER_SIZE);

        shim_nsslider_set_double_value(slider, 0, 7.5);
        shim_nsslider_set_min_value(slider, 0, 1.25);
        shim_nsslider_set_max_value(slider, 0, 9.75);

        assert_eq!(shim_nsslider_double_value(slider, 0), 7.5);
        assert_eq!(shim_nsslider_min_value(slider, 0), 1.25);
        assert_eq!(shim_nsslider_max_value(slider, 0), 9.75);
        free_obj(slider);
    }

    #[test]
    fn continuous_flag_round_trip() {
        let slider = alloc_obj_fake(SLIDER_SIZE);

        assert_eq!(shim_nsslider_is_continuous(slider, 0), 0);
        shim_nsslider_set_continuous(slider, 0, 1);
        assert_eq!(shim_nsslider_is_continuous(slider, 0), 1);
        assert_eq!(read_field(slider, FLAGS_OFF) & FLAG_CONTINUOUS, FLAG_CONTINUOUS);
        shim_nsslider_set_continuous(slider, 0, 0);
        assert_eq!(shim_nsslider_is_continuous(slider, 0), 0);
        assert_eq!(read_field(slider, FLAGS_OFF) & FLAG_CONTINUOUS, 0);
        free_obj(slider);
    }

    #[test]
    fn orientation_flag_round_trip() {
        let slider = alloc_obj_fake(SLIDER_SIZE);

        assert_eq!(shim_nsslider_orientation(slider, 0), 0);
        shim_nsslider_set_orientation(slider, 0, 1);
        assert_eq!(shim_nsslider_orientation(slider, 0), 1);
        assert_eq!(read_field(slider, FLAGS_OFF) & FLAG_VERTICAL, FLAG_VERTICAL);
        shim_nsslider_set_orientation(slider, 0, 0);
        assert_eq!(shim_nsslider_orientation(slider, 0), 0);
        assert_eq!(read_field(slider, FLAGS_OFF) & FLAG_VERTICAL, 0);
        free_obj(slider);
    }

    #[test]
    fn continuous_and_orientation_bits_do_not_clobber_each_other() {
        let slider = alloc_obj_fake(SLIDER_SIZE);

        shim_nsslider_set_continuous(slider, 0, 1);
        shim_nsslider_set_orientation(slider, 0, 1);
        assert_eq!(shim_nsslider_is_continuous(slider, 0), 1);
        assert_eq!(shim_nsslider_orientation(slider, 0), 1);

        shim_nsslider_set_continuous(slider, 0, 0);
        assert_eq!(shim_nsslider_is_continuous(slider, 0), 0);
        assert_eq!(shim_nsslider_orientation(slider, 0), 1);

        shim_nsslider_set_orientation(slider, 0, 0);
        assert_eq!(shim_nsslider_is_continuous(slider, 0), 0);
        assert_eq!(shim_nsslider_orientation(slider, 0), 0);
        free_obj(slider);
    }
}
