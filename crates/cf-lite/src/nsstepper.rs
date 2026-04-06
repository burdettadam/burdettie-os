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
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut f64) = value; }
}

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut u64) = value; }
}

/// `-[NSStepper initWithFrame:]`
pub extern "C" fn shim_nsstepper_init_with_frame(
    obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64,
) -> u64 {
    let obj = crate::nsview::shim_nsview_init_with_frame(obj, 0, x, y, w, h);
    if obj == 0 { return 0; }
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
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_AUTOREPEAT; } else { flags &= !FLAG_AUTOREPEAT; }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSStepper valueWraps]`
pub extern "C" fn shim_nsstepper_value_wraps(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_VALUE_WRAPS != 0)
}

/// `-[NSStepper setValueWraps:]`
pub extern "C" fn shim_nsstepper_set_value_wraps(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 { flags |= FLAG_VALUE_WRAPS; } else { flags &= !FLAG_VALUE_WRAPS; }
    write_u64(obj, FLAGS_OFF, flags);
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
        shim_nsstepper_set_min_value(0, 0, 42.0); // no crash
        shim_nsstepper_set_increment(0, 0, 5.0); // no crash
    }
}
