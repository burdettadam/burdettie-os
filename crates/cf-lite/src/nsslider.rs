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
//!   offset 40: target_ptr           (u64 — target object, or 0)
//!   offset 48: action_sel           (u64 — SEL addr, or 0)
//!   offset 56: reserved / padding   (u64)
//!
//! Total: 64 bytes.

const CURRENT_VALUE_OFF: usize = 8;
const MIN_VALUE_OFF: usize = 16;
const MAX_VALUE_OFF: usize = 24;
const FLAGS_OFF: usize = 32;
const TARGET_OFF: usize = 40;
const ACTION_OFF: usize = 48;
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

/// `-[NSSlider drawRect:]` — renders slider track and knob.
pub extern "C" fn shim_nsslider_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let min = read_f64(obj, MIN_VALUE_OFF);
    let max = read_f64(obj, MAX_VALUE_OFF);
    let val = read_f64(obj, CURRENT_VALUE_OFF);
    let flags = read_u64(obj, FLAGS_OFF);
    let is_vertical = flags & FLAG_VERTICAL != 0;
    let range = if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        max - min
    };
    let ratio = ((val - min) / range).clamp(0.0, 1.0);

    cg_shims::bridge::with_context(cg_ctx, |c| {
        // Track background.
        c.fill_color = graphics_core::Color::new(0.85, 0.85, 0.85, 1.0);
        if is_vertical {
            let track_x = x + w / 2.0 - 2.0;
            c.fill_rect(track_x, y, 4.0, h);
            // Knob.
            let knob_y = y + h * (1.0 - ratio) - 6.0;
            c.fill_color = graphics_core::Color::new(0.2, 0.5, 1.0, 1.0);
            c.fill_rect(x + 2.0, knob_y, w - 4.0, 12.0);
        } else {
            let track_y = y + h / 2.0 - 2.0;
            c.fill_rect(x, track_y, w, 4.0);
            // Knob.
            let knob_x = x + w * ratio - 6.0;
            c.fill_color = graphics_core::Color::new(0.2, 0.5, 1.0, 1.0);
            c.fill_rect(knob_x, y + 2.0, 12.0, h - 4.0);
        }
    });
}

/// `-[NSSlider target]`
pub extern "C" fn shim_nsslider_target(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, TARGET_OFF)
}

/// `-[NSSlider setTarget:]`
pub extern "C" fn shim_nsslider_set_target(obj: u64, _cmd: u64, tgt: u64) {
    write_u64(obj, TARGET_OFF, tgt);
}

/// `-[NSSlider action]`
pub extern "C" fn shim_nsslider_action(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ACTION_OFF)
}

/// `-[NSSlider setAction:]`
pub extern "C" fn shim_nsslider_set_action(obj: u64, _cmd: u64, sel: u64) {
    write_u64(obj, ACTION_OFF, sel);
}

/// `-[NSSlider mouseDown:]` — track mouse position, update value, fire action.
pub extern "C" fn shim_nsslider_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);

    let flags = read_flags(obj);
    let is_vertical = flags & FLAG_VERTICAL != 0;
    let min = read_f64(obj, MIN_VALUE_OFF);
    let max = read_f64(obj, MAX_VALUE_OFF);

    // Compute ratio from click position within frame.
    let ratio = if is_vertical {
        let local_y = loc_y - frame.origin.y;
        1.0 - (local_y / frame.size.height).clamp(0.0, 1.0)
    } else {
        let local_x = loc_x - frame.origin.x;
        (local_x / frame.size.width).clamp(0.0, 1.0)
    };

    let new_value = min + ratio * (max - min);
    write_f64(obj, CURRENT_VALUE_OFF, new_value);

    // Fire target/action.
    fire_action(obj);

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

fn fire_action(obj: u64) {
    let action = read_u64(obj, ACTION_OFF);
    let target = read_u64(obj, TARGET_OFF);
    if action == 0 {
        return;
    }
    let nsapp = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
    crate::nsapplication::shim_nsapplication_send_action_to_from(nsapp, 0, action, target, obj);
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
        assert_eq!(shim_nsslider_target(0, 0), 0);
        shim_nsslider_set_target(0, 0, 0x1234);
        assert_eq!(shim_nsslider_action(0, 0), 0);
        shim_nsslider_set_action(0, 0, 0x5678);
        shim_nsslider_mouse_down(0, 0, 0);
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
        assert_eq!(
            read_field(slider, FLAGS_OFF) & FLAG_CONTINUOUS,
            FLAG_CONTINUOUS
        );
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

    #[test]
    fn target_action_round_trip() {
        let slider = alloc_obj_fake(SLIDER_SIZE);
        shim_nsslider_set_target(slider, 0, 0xABCD);
        shim_nsslider_set_action(slider, 0, 0xEF01);
        assert_eq!(shim_nsslider_target(slider, 0), 0xABCD);
        assert_eq!(shim_nsslider_action(slider, 0), 0xEF01);
        free_obj(slider);
    }

    #[test]
    fn target_action_do_not_clobber_value() {
        let slider = alloc_obj_fake(SLIDER_SIZE);
        shim_nsslider_set_double_value(slider, 0, 42.0);
        shim_nsslider_set_target(slider, 0, 0x1111);
        shim_nsslider_set_action(slider, 0, 0x2222);
        assert_eq!(shim_nsslider_double_value(slider, 0), 42.0);
        assert_eq!(shim_nsslider_target(slider, 0), 0x1111);
        assert_eq!(shim_nsslider_action(slider, 0), 0x2222);
        free_obj(slider);
    }
}
