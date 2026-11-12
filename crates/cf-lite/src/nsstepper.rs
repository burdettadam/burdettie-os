//! NSStepper shim — up/down arrow control for incrementing values.
//!
//! Object layout (inherits NSControl, 120 bytes + stepper fields):
//!
//!   offset   0..120: inherited NSControl fields
//!   offset 120: min_value   (f64)
//!   offset 128: max_value   (f64)
//!   offset 136: increment   (f64)
//!   offset 144: flags       (u64 — bit 0: autorepeat, bit 1: value_wraps)
//!
//! Total: 152 bytes.

pub(crate) const STEPPER_SIZE: usize = 152;

const MIN_VALUE_OFF: usize = 120;
const MAX_VALUE_OFF: usize = 128;
const INCREMENT_OFF: usize = 136;
const FLAGS_OFF: usize = 144;

const FLAG_AUTOREPEAT: u64 = 1 << 0;
const FLAG_VALUE_WRAPS: u64 = 1 << 1;

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut f64) = value;
    }
}

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

/// `-[NSStepper initWithFrame:]`
pub extern "C" fn shim_nsstepper_init_with_frame(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    let obj = crate::nsview::shim_nsview_init_with_frame(obj, 0, x, y, w, h);
    if obj == 0 {
        return 0;
    }
    // Defaults: min=0, max=59, increment=1, autorepeat=YES, wraps=NO
    write_f64(obj, MIN_VALUE_OFF, 0.0);
    write_f64(obj, MAX_VALUE_OFF, 59.0);
    write_f64(obj, INCREMENT_OFF, 1.0);
    write_u64(obj, FLAGS_OFF, FLAG_AUTOREPEAT);
    obj
}

/// `-[NSStepper minValue]`
pub extern "C" fn shim_nsstepper_min_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MIN_VALUE_OFF)
}

/// `-[NSStepper setMinValue:]`
pub extern "C" fn shim_nsstepper_set_min_value(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, MIN_VALUE_OFF, val);
}

/// `-[NSStepper maxValue]`
pub extern "C" fn shim_nsstepper_max_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MAX_VALUE_OFF)
}

/// `-[NSStepper setMaxValue:]`
pub extern "C" fn shim_nsstepper_set_max_value(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, MAX_VALUE_OFF, val);
}

/// `-[NSStepper increment]`
pub extern "C" fn shim_nsstepper_increment(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, INCREMENT_OFF)
}

/// `-[NSStepper setIncrement:]`
pub extern "C" fn shim_nsstepper_set_increment(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, INCREMENT_OFF, val);
}

/// `-[NSStepper autorepeat]`
pub extern "C" fn shim_nsstepper_autorepeat(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_AUTOREPEAT != 0)
}

/// `-[NSStepper setAutorepeat:]`
pub extern "C" fn shim_nsstepper_set_autorepeat(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_AUTOREPEAT;
    } else {
        flags &= !FLAG_AUTOREPEAT;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSStepper valueWraps]`
pub extern "C" fn shim_nsstepper_value_wraps(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_VALUE_WRAPS != 0)
}

/// `-[NSStepper setValueWraps:]`
pub extern "C" fn shim_nsstepper_set_value_wraps(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_VALUE_WRAPS;
    } else {
        flags &= !FLAG_VALUE_WRAPS;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

// ── NSControl target/action offsets (inherited at 88/96) ────────

const TARGET_OFF: usize = 88;
const ACTION_OFF: usize = 96;
/// NSControl intValue offset — we store the stepper's current numeric value here.
const INT_VALUE_OFF: usize = 104;

/// `-[NSStepper integerValue]` — reads inherited NSControl int_value.
pub extern "C" fn shim_nsstepper_integer_value(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + INT_VALUE_OFF) as *const i64) }
}

/// `-[NSStepper setIntegerValue:]`
pub extern "C" fn shim_nsstepper_set_integer_value(obj: u64, _cmd: u64, val: i64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + INT_VALUE_OFF) as *mut i64) = val;
    }
}

/// `-[NSStepper doubleValue]` — cast from integer.
pub extern "C" fn shim_nsstepper_double_value(obj: u64, _cmd: u64) -> f64 {
    shim_nsstepper_integer_value(obj, 0) as f64
}

/// `-[NSStepper setDoubleValue:]`
pub extern "C" fn shim_nsstepper_set_double_value(obj: u64, _cmd: u64, val: f64) {
    shim_nsstepper_set_integer_value(obj, 0, val as i64);
}

/// `-[NSStepper drawRect:]` — renders up/down arrows.
pub extern "C" fn shim_nsstepper_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let half_h = h / 2.0;

    cg_shims::bridge::with_context(cg_ctx, |c| {
        // ── Top half (increment / up arrow) ──
        c.fill_color = graphics_core::Color::new(0.92, 0.92, 0.92, 1.0);
        c.fill_rect(x, y, w, half_h);
        c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
        c.stroke_rect(x, y, w, half_h);

        // Draw up arrow "▲" as text.
        c.fill_color = graphics_core::Color::new(0.2, 0.2, 0.2, 1.0);
        c.draw_text("\u{25B2}", x + w * 0.3, y + half_h * 0.72);

        // ── Bottom half (decrement / down arrow) ──
        c.fill_color = graphics_core::Color::new(0.92, 0.92, 0.92, 1.0);
        c.fill_rect(x, y + half_h, w, half_h);
        c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
        c.stroke_rect(x, y + half_h, w, half_h);

        // Draw down arrow "▼" as text.
        c.fill_color = graphics_core::Color::new(0.2, 0.2, 0.2, 1.0);
        c.draw_text("\u{25BC}", x + w * 0.3, y + half_h + half_h * 0.72);
    });
}

/// `-[NSStepper mouseDown:]` — increment or decrement value based on click position.
pub extern "C" fn shim_nsstepper_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 {
        return;
    }

    let (_, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let local_y = loc_y - frame.origin.y;
    let mid = frame.size.height / 2.0;

    let min = read_f64(obj, MIN_VALUE_OFF);
    let max = read_f64(obj, MAX_VALUE_OFF);
    let inc = read_f64(obj, INCREMENT_OFF);
    let wraps = read_u64(obj, FLAGS_OFF) & FLAG_VALUE_WRAPS != 0;

    let current = shim_nsstepper_integer_value(obj, 0) as f64;

    let new_val = if local_y < mid {
        // Top half → increment.
        let v = current + inc;
        if v > max {
            if wraps {
                min
            } else {
                max
            }
        } else {
            v
        }
    } else {
        // Bottom half → decrement.
        let v = current - inc;
        if v < min {
            if wraps {
                max
            } else {
                min
            }
        } else {
            v
        }
    };

    shim_nsstepper_set_integer_value(obj, 0, new_val as i64);

    // Fire target/action.
    let action = read_u64(obj, ACTION_OFF);
    let target = read_u64(obj, TARGET_OFF);
    if action != 0 {
        let nsapp = crate::nsapplication::shim_nsapplication_shared_application(0, 0);
        crate::nsapplication::shim_nsapplication_send_action_to_from(nsapp, 0, action, target, obj);
    }

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn defaults_after_init() {
        let obj = alloc_obj(STEPPER_SIZE, crate::nsobj::nsstepper_class_addr());
        let obj = shim_nsstepper_init_with_frame(obj, 0, 0.0, 0.0, 19.0, 27.0);
        assert_ne!(obj, 0);
        assert_eq!(shim_nsstepper_min_value(obj, 0), 0.0);
        assert_eq!(shim_nsstepper_max_value(obj, 0), 59.0);
        assert_eq!(shim_nsstepper_increment(obj, 0), 1.0);
        assert_eq!(shim_nsstepper_autorepeat(obj, 0), 1);
        assert_eq!(shim_nsstepper_value_wraps(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn min_max_round_trip() {
        let obj = alloc_obj(STEPPER_SIZE, crate::nsobj::nsstepper_class_addr());
        let obj = shim_nsstepper_init_with_frame(obj, 0, 0.0, 0.0, 19.0, 27.0);
        shim_nsstepper_set_min_value(obj, 0, -100.0);
        shim_nsstepper_set_max_value(obj, 0, 100.0);
        assert_eq!(shim_nsstepper_min_value(obj, 0), -100.0);
        assert_eq!(shim_nsstepper_max_value(obj, 0), 100.0);
        free_obj(obj);
    }

    #[test]
    fn flags_round_trip() {
        let obj = alloc_obj(STEPPER_SIZE, crate::nsobj::nsstepper_class_addr());
        let obj = shim_nsstepper_init_with_frame(obj, 0, 0.0, 0.0, 19.0, 27.0);
        shim_nsstepper_set_autorepeat(obj, 0, 0);
        assert_eq!(shim_nsstepper_autorepeat(obj, 0), 0);
        shim_nsstepper_set_value_wraps(obj, 0, 1);
        assert_eq!(shim_nsstepper_value_wraps(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsstepper_min_value(0, 0), 0.0);
        assert_eq!(shim_nsstepper_max_value(0, 0), 0.0);
        shim_nsstepper_set_min_value(0, 0, 42.0);
        shim_nsstepper_set_increment(0, 0, 5.0);
        assert_eq!(shim_nsstepper_integer_value(0, 0), 0);
        shim_nsstepper_set_integer_value(0, 0, 10);
        assert_eq!(shim_nsstepper_double_value(0, 0), 0.0);
        shim_nsstepper_set_double_value(0, 0, 3.14);
        shim_nsstepper_draw_rect(0, 0, 0.0, 0.0, 19.0, 27.0);
        shim_nsstepper_mouse_down(0, 0, 0);
    }

    #[test]
    fn integer_value_round_trip() {
        let obj = alloc_obj(STEPPER_SIZE, crate::nsobj::nsstepper_class_addr());
        let obj = shim_nsstepper_init_with_frame(obj, 0, 0.0, 0.0, 19.0, 27.0);
        shim_nsstepper_set_integer_value(obj, 0, 25);
        assert_eq!(shim_nsstepper_integer_value(obj, 0), 25);
        assert_eq!(shim_nsstepper_double_value(obj, 0), 25.0);
        free_obj(obj);
    }

    #[test]
    fn double_value_truncates_to_integer() {
        let obj = alloc_obj(STEPPER_SIZE, crate::nsobj::nsstepper_class_addr());
        let obj = shim_nsstepper_init_with_frame(obj, 0, 0.0, 0.0, 19.0, 27.0);
        shim_nsstepper_set_double_value(obj, 0, 42.9);
        assert_eq!(shim_nsstepper_integer_value(obj, 0), 42);
        free_obj(obj);
    }
}
