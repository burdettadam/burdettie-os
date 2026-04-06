//! NSSegmentedControl shim: multi-button selector control for AppKit.
//!
//! Object layout (80 bytes, inherits NSControl at 48):
//!
//!   offset  0: isa                  (u64)
//!   offset  8: segment_count        (u64)
//!   offset 16: selected_segment     (i64, -1 if none)
//!   offset 24: flags                (u64 — bit 0: tracking_mode [0=select-one, 1=select-any],
//!                                           bit 1: momentary)
//!   offset 32: titles_ptr           (u64 — pointer to Vec<u64> of NSString pointers)
//!
//! Total: 80 bytes.

use std::vec::Vec;

const SEGMENT_COUNT_OFF: usize = 8;
const SELECTED_SEGMENT_OFF: usize = 16;
const FLAGS_OFF: usize = 24;
const TITLES_PTR_OFF: usize = 32;
pub(crate) const SEGMENTED_CONTROL_SIZE: usize = 80;

const FLAG_TRACKING_MODE: u64 = 1 << 0;  // 0 = select-one, 1 = select-any
const FLAG_MOMENTARY: u64 = 1 << 1;

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
    read_u64(obj, offset) as i64
}

fn write_i64(obj: u64, offset: usize, value: i64) {
    write_u64(obj, offset, value as u64);
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

/// `-[NSSegmentedControl segmentCount]`
pub extern "C" fn shim_nssegmentedcontrol_segment_count(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SEGMENT_COUNT_OFF)
}

/// `-[NSSegmentedControl setSegmentCount:]`
pub extern "C" fn shim_nssegmentedcontrol_set_segment_count(obj: u64, _cmd: u64, count: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, SEGMENT_COUNT_OFF, count);
    write_i64(obj, SELECTED_SEGMENT_OFF, -1);
    // Clear and reallocate titles vec
    let vec_ptr = read_u64(obj, TITLES_PTR_OFF) as *mut Vec<u64>;
    if !vec_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(vec_ptr));
        }
    }
    let new_vec = Box::into_raw(Box::new(vec![0u64; count as usize]));
    write_u64(obj, TITLES_PTR_OFF, new_vec as u64);
}

/// `-[NSSegmentedControl selectedSegment]`
pub extern "C" fn shim_nssegmentedcontrol_selected_segment(obj: u64, _cmd: u64) -> i64 {
    read_i64(obj, SELECTED_SEGMENT_OFF)
}

/// `-[NSSegmentedControl setSelectedSegment:]`
pub extern "C" fn shim_nssegmentedcontrol_set_selected_segment(obj: u64, _cmd: u64, seg: i64) {
    write_i64(obj, SELECTED_SEGMENT_OFF, seg);
}

/// `-[NSSegmentedControl isSelectedForSegment:]`
pub extern "C" fn shim_nssegmentedcontrol_is_selected_for_segment(
    obj: u64,
    _cmd: u64,
    seg: i64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    if read_i64(obj, SELECTED_SEGMENT_OFF) == seg {
        1
    } else {
        0
    }
}

/// `-[NSSegmentedControl setSelected:forSegment:]`
pub extern "C" fn shim_nssegmentedcontrol_set_selected_for_segment(
    obj: u64,
    _cmd: u64,
    flag: u8,
    seg: i64,
) {
    if obj == 0 {
        return;
    }
    if flag != 0 {
        write_i64(obj, SELECTED_SEGMENT_OFF, seg);
    } else {
        write_i64(obj, SELECTED_SEGMENT_OFF, -1);
    }
}

/// `-[NSSegmentedControl labelForSegment:]`
pub extern "C" fn shim_nssegmentedcontrol_label_for_segment(
    obj: u64,
    _cmd: u64,
    seg: i64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let vec_ptr = read_u64(obj, TITLES_PTR_OFF) as *const Vec<u64>;
    if vec_ptr.is_null() {
        return 0;
    }
    unsafe {
        let vec = &*vec_ptr;
        if seg >= 0 && (seg as usize) < vec.len() {
            vec[seg as usize]
        } else {
            0
        }
    }
}

/// `-[NSSegmentedControl setLabel:forSegment:]`
pub extern "C" fn shim_nssegmentedcontrol_set_label_for_segment(
    obj: u64,
    _cmd: u64,
    label: u64,
    seg: i64,
) {
    if obj == 0 {
        return;
    }
    let vec_ptr = read_u64(obj, TITLES_PTR_OFF) as *mut Vec<u64>;
    if vec_ptr.is_null() {
        return;
    }
    unsafe {
        let vec = &mut *vec_ptr;
        if seg >= 0 && (seg as usize) < vec.len() {
            vec[seg as usize] = label;
        }
    }
}

/// `-[NSSegmentedControl trackingMode]`
pub extern "C" fn shim_nssegmentedcontrol_tracking_mode(obj: u64, _cmd: u64) -> u64 {
    (read_flags(obj) & FLAG_TRACKING_MODE) as u64
}

/// `-[NSSegmentedControl setTrackingMode:]`
pub extern "C" fn shim_nssegmentedcontrol_set_tracking_mode(obj: u64, _cmd: u64, mode: u64) {
    set_flag(obj, FLAG_TRACKING_MODE, mode != 0);
}

/// `-[NSSegmentedControl isMomentary]`
pub extern "C" fn shim_nssegmentedcontrol_is_momentary(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_MOMENTARY)
}

/// `-[NSSegmentedControl setMomentary:]`
pub extern "C" fn shim_nssegmentedcontrol_set_momentary(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_MOMENTARY, val != 0);
}

/// `-[NSSegmentedControl drawRect:]` — renders segments.
pub extern "C" fn shim_nssegmentedcontrol_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let count = read_u64(obj, SEGMENT_COUNT_OFF) as usize;
    if count == 0 { return; }
    let selected = read_i64(obj, SELECTED_SEGMENT_OFF);
    let seg_w = w / count as f64;

    for i in 0..count {
        let seg_x = x + i as f64 * seg_w;
        let is_selected = i as i64 == selected;

        cg_shims::bridge::with_context(cg_ctx, |c| {
            // Segment background.
            c.fill_color = if is_selected {
                graphics_core::Color::new(0.2, 0.5, 1.0, 1.0)
            } else {
                graphics_core::Color::new(0.92, 0.92, 0.92, 1.0)
            };
            c.fill_rect(seg_x, y, seg_w, h);

            // Border.
            c.stroke_color = graphics_core::Color::new(0.7, 0.7, 0.7, 1.0);
            c.stroke_rect(seg_x, y, seg_w, h);
        });

        // Draw label.
        let label = shim_nssegmentedcontrol_label_for_segment(obj, 0, i as i64);
        if label != 0 {
            let utf8 = crate::cfstring::shim_nsstring_utf8string(label, 0);
            if !utf8.is_null() {
                let text = unsafe { std::ffi::CStr::from_ptr(utf8) }.to_str().unwrap_or("");
                if !text.is_empty() {
                    cg_shims::bridge::with_context(cg_ctx, |c| {
                        c.fill_color = if is_selected {
                            graphics_core::Color::WHITE
                        } else {
                            graphics_core::Color::BLACK
                        };
                        c.draw_text(text, seg_x + 4.0, y + h * 0.65);
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nssegmentedcontrol_segment_count(0, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_selected_segment(0, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_is_selected_for_segment(0, 0, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_label_for_segment(0, 0, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(0, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_is_momentary(0, 0), 0);
    }

    #[test]
    fn segment_count_and_selection() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        
        // Set 3 segments
        shim_nssegmentedcontrol_set_segment_count(seg, 0, 3);
        assert_eq!(shim_nssegmentedcontrol_segment_count(seg, 0), 3);
        
        // Initially -1
        assert_eq!(shim_nssegmentedcontrol_selected_segment(seg, 0), -1);
        
        free_obj(seg);
    }

    #[test]
    fn select_segment() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        shim_nssegmentedcontrol_set_segment_count(seg, 0, 3);
        
        shim_nssegmentedcontrol_set_selected_segment(seg, 0, 1);
        assert_eq!(shim_nssegmentedcontrol_selected_segment(seg, 0), 1);
        assert_eq!(shim_nssegmentedcontrol_is_selected_for_segment(seg, 0, 1), 1);
        assert_eq!(shim_nssegmentedcontrol_is_selected_for_segment(seg, 0, 0), 0);
        
        free_obj(seg);
    }

    #[test]
    fn set_selected_flag_behavior() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        shim_nssegmentedcontrol_set_segment_count(seg, 0, 3);
        
        // Set selected flag for segment 2
        shim_nssegmentedcontrol_set_selected_for_segment(seg, 0, 1, 2);
        assert_eq!(shim_nssegmentedcontrol_selected_segment(seg, 0), 2);
        
        // Clear selected flag for any segment
        shim_nssegmentedcontrol_set_selected_for_segment(seg, 0, 0, 99);
        assert_eq!(shim_nssegmentedcontrol_selected_segment(seg, 0), -1);
        
        free_obj(seg);
    }

    #[test]
    fn label_round_trip() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        shim_nssegmentedcontrol_set_segment_count(seg, 0, 2);
        
        let label1 = make_string("Option 1");
        let label2 = make_string("Option 2");
        
        shim_nssegmentedcontrol_set_label_for_segment(seg, 0, label1, 0);
        shim_nssegmentedcontrol_set_label_for_segment(seg, 0, label2, 1);
        
        assert_eq!(shim_nssegmentedcontrol_label_for_segment(seg, 0, 0), label1);
        assert_eq!(shim_nssegmentedcontrol_label_for_segment(seg, 0, 1), label2);
        
        free_obj(seg);
    }

    #[test]
    fn tracking_mode_flag() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        
        // Default mode is 0 (select-one)
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(seg, 0), 0);
        
        // Set to select-any mode (1)
        shim_nssegmentedcontrol_set_tracking_mode(seg, 0, 1);
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(seg, 0), 1);
        
        // Set back to select-one (0)
        shim_nssegmentedcontrol_set_tracking_mode(seg, 0, 0);
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(seg, 0), 0);
        
        free_obj(seg);
    }

    #[test]
    fn momentary_flag() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        
        assert_eq!(shim_nssegmentedcontrol_is_momentary(seg, 0), 0);
        
        shim_nssegmentedcontrol_set_momentary(seg, 0, 1);
        assert_eq!(shim_nssegmentedcontrol_is_momentary(seg, 0), 1);
        
        shim_nssegmentedcontrol_set_momentary(seg, 0, 0);
        assert_eq!(shim_nssegmentedcontrol_is_momentary(seg, 0), 0);
        
        free_obj(seg);
    }

    #[test]
    fn flags_independent() {
        let seg = alloc_obj(SEGMENTED_CONTROL_SIZE, 0xDEAD);
        
        shim_nssegmentedcontrol_set_tracking_mode(seg, 0, 1);
        shim_nssegmentedcontrol_set_momentary(seg, 0, 1);
        
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(seg, 0), 1);
        assert_eq!(shim_nssegmentedcontrol_is_momentary(seg, 0), 1);
        
        shim_nssegmentedcontrol_set_tracking_mode(seg, 0, 0);
        assert_eq!(shim_nssegmentedcontrol_tracking_mode(seg, 0), 0);
        assert_eq!(shim_nssegmentedcontrol_is_momentary(seg, 0), 1);
        
        free_obj(seg);
    }
}
