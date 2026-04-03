//! NSProgressIndicator shim: progress bar / indeterminate spinner for AppKit.
//!
//! Object layout (96 bytes, inherits NSView at 80):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: current_progress     (u64 — raw f64 bits, [0.0–1.0])
//!   offset 16: min_value            (u64 — raw f64 bits)
//!   offset 24: max_value            (u64 — raw f64 bits)
//!   offset 32: flags                (u64 — bit 0: is_indeterminate, bit 1: is_bezeled)
//!
//! Total: 96 bytes (80 inherited from NSView + 16 new fields).

const CURRENT_PROGRESS_OFF: usize = 8;
const MIN_VALUE_OFF: usize = 16;
const MAX_VALUE_OFF: usize = 24;
const FLAGS_OFF: usize = 32;
pub(crate) const PROGRESS_INDICATOR_SIZE: usize = 96;

const FLAG_IS_INDETERMINATE: u64 = 1 << 0;
const FLAG_IS_BEZELED: u64 = 1 << 1;

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

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSProgressIndicator doubleValue]`
pub extern "C" fn shim_nsprogressindicator_double_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, CURRENT_PROGRESS_OFF)
}

/// `-[NSProgressIndicator setDoubleValue:]`
pub extern "C" fn shim_nsprogressindicator_set_double_value(obj: u64, _cmd: u64, val: f64) {
    if obj == 0 {
        return;
    }
    let min = read_f64(obj, MIN_VALUE_OFF);
    let max = read_f64(obj, MAX_VALUE_OFF);
    let clamped = val.clamp(min, max);
    write_f64(obj, CURRENT_PROGRESS_OFF, clamped);
}

/// `-[NSProgressIndicator minValue]`
pub extern "C" fn shim_nsprogressindicator_min_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MIN_VALUE_OFF)
}

/// `-[NSProgressIndicator setMinValue:]`
pub extern "C" fn shim_nsprogressindicator_set_min_value(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, MIN_VALUE_OFF, val);
}

/// `-[NSProgressIndicator maxValue]`
pub extern "C" fn shim_nsprogressindicator_max_value(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MAX_VALUE_OFF)
}

/// `-[NSProgressIndicator setMaxValue:]`
pub extern "C" fn shim_nsprogressindicator_set_max_value(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, MAX_VALUE_OFF, val);
}

/// `-[NSProgressIndicator isIndeterminate]`
pub extern "C" fn shim_nsprogressindicator_is_indeterminate(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_INDETERMINATE)
}

/// `-[NSProgressIndicator setIndeterminate:]`
pub extern "C" fn shim_nsprogressindicator_set_indeterminate(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_INDETERMINATE, val != 0);
}

/// `-[NSProgressIndicator isBezeled]`
pub extern "C" fn shim_nsprogressindicator_is_bezeled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BEZELED)
}

/// `-[NSProgressIndicator setBezeled:]`
pub extern "C" fn shim_nsprogressindicator_set_bezeled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BEZELED, val != 0);
}

/// `-[NSProgressIndicator startAnimation:]` — no-op stub.
pub extern "C" fn shim_nsprogressindicator_start_animation(_obj: u64, _cmd: u64, _sender: u64) {}

/// `-[NSProgressIndicator stopAnimation:]` — no-op stub.
pub extern "C" fn shim_nsprogressindicator_stop_animation(_obj: u64, _cmd: u64, _sender: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsprogressindicator_double_value(0, 0), 0.0);
        shim_nsprogressindicator_set_double_value(0, 0, 0.5);
        assert_eq!(shim_nsprogressindicator_min_value(0, 0), 0.0);
        assert_eq!(shim_nsprogressindicator_max_value(0, 0), 0.0);
        assert_eq!(shim_nsprogressindicator_is_indeterminate(0, 0), 0);
        assert_eq!(shim_nsprogressindicator_is_bezeled(0, 0), 0);
    }

    #[test]
    fn default_values() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        // Initialize with default values
        write_f64(prog, CURRENT_PROGRESS_OFF, 0.0);
        write_f64(prog, MIN_VALUE_OFF, 0.0);
        write_f64(prog, MAX_VALUE_OFF, 100.0);
        
        assert_eq!(shim_nsprogressindicator_double_value(prog, 0), 0.0);
        assert_eq!(shim_nsprogressindicator_min_value(prog, 0), 0.0);
        assert_eq!(shim_nsprogressindicator_max_value(prog, 0), 100.0);
        free_obj(prog);
    }

    #[test]
    fn double_value_round_trip() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        write_f64(prog, MIN_VALUE_OFF, 0.0);
        write_f64(prog, MAX_VALUE_OFF, 100.0);
        
        shim_nsprogressindicator_set_double_value(prog, 0, 50.5);
        assert!((shim_nsprogressindicator_double_value(prog, 0) - 50.5).abs() < 0.001);
        free_obj(prog);
    }

    #[test]
    fn double_value_clamping() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        write_f64(prog, MIN_VALUE_OFF, 10.0);
        write_f64(prog, MAX_VALUE_OFF, 90.0);
        
        // Try to set above max
        shim_nsprogressindicator_set_double_value(prog, 0, 150.0);
        assert!((shim_nsprogressindicator_double_value(prog, 0) - 90.0).abs() < 0.001);
        
        // Try to set below min
        shim_nsprogressindicator_set_double_value(prog, 0, 5.0);
        assert!((shim_nsprogressindicator_double_value(prog, 0) - 10.0).abs() < 0.001);
        free_obj(prog);
    }

    #[test]
    fn indeterminate_flag() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        assert_eq!(shim_nsprogressindicator_is_indeterminate(prog, 0), 0);
        
        shim_nsprogressindicator_set_indeterminate(prog, 0, 1);
        assert_eq!(shim_nsprogressindicator_is_indeterminate(prog, 0), 1);
        
        shim_nsprogressindicator_set_indeterminate(prog, 0, 0);
        assert_eq!(shim_nsprogressindicator_is_indeterminate(prog, 0), 0);
        free_obj(prog);
    }

    #[test]
    fn bezeled_flag() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        assert_eq!(shim_nsprogressindicator_is_bezeled(prog, 0), 0);
        
        shim_nsprogressindicator_set_bezeled(prog, 0, 1);
        assert_eq!(shim_nsprogressindicator_is_bezeled(prog, 0), 1);
        
        shim_nsprogressindicator_set_bezeled(prog, 0, 0);
        assert_eq!(shim_nsprogressindicator_is_bezeled(prog, 0), 0);
        free_obj(prog);
    }

    #[test]
    fn flags_independent() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        
        shim_nsprogressindicator_set_indeterminate(prog, 0, 1);
        shim_nsprogressindicator_set_bezeled(prog, 0, 1);
        
        assert_eq!(shim_nsprogressindicator_is_indeterminate(prog, 0), 1);
        assert_eq!(shim_nsprogressindicator_is_bezeled(prog, 0), 1);
        
        shim_nsprogressindicator_set_indeterminate(prog, 0, 0);
        assert_eq!(shim_nsprogressindicator_is_indeterminate(prog, 0), 0);
        assert_eq!(shim_nsprogressindicator_is_bezeled(prog, 0), 1);
        free_obj(prog);
    }

    #[test]
    fn animation_is_noop() {
        let prog = alloc_obj(PROGRESS_INDICATOR_SIZE, 0xDEAD);
        // Should not crash
        shim_nsprogressindicator_start_animation(prog, 0, 0);
        shim_nsprogressindicator_stop_animation(prog, 0, 0);
        free_obj(prog);
    }
}
