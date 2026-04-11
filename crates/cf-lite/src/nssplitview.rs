//! NSSplitView shim: split-view container that divides content into panes.
//!
//! Inherits from NSView (72 bytes).
//!
//! Object layout (88 bytes):
//!
//!   offset  0: isa                  (u64 — inherited from NSView)
//!   [0..72): inherited NSView fields
//!   offset 72: delegate_ptr         (u64 — id, or 0)
//!   offset 80: flags                (u64 — bit 0: is_vertical,
//!                                          bits 1-2: divider_style
//!                                            0=thin, 1=thick, 2=paneSplitter)
//!
//! Total: 88 bytes.

const DELEGATE_OFF: usize = 72;
const FLAGS_OFF: usize = 80;
pub(crate) const SPLIT_VIEW_SIZE: usize = 88;

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

// ── Instance method shims ──────────────────────────────────────

/// `-[NSSplitView isVertical]`
pub extern "C" fn shim_nssplitview_is_vertical(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    (read_u64(obj, FLAGS_OFF) & 1) as u8
}

/// `-[NSSplitView setVertical:]`
pub extern "C" fn shim_nssplitview_set_vertical(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        write_u64(obj, FLAGS_OFF, flags | 1);
    } else {
        write_u64(obj, FLAGS_OFF, flags & !1);
    }
}

/// `-[NSSplitView delegate]`
pub extern "C" fn shim_nssplitview_delegate(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DELEGATE_OFF)
}

/// `-[NSSplitView setDelegate:]`
pub extern "C" fn shim_nssplitview_set_delegate(obj: u64, _cmd: u64, del: u64) {
    write_u64(obj, DELEGATE_OFF, del);
}

/// `-[NSSplitView dividerStyle]`
pub extern "C" fn shim_nssplitview_divider_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    (read_u64(obj, FLAGS_OFF) >> 1) & 0x3
}

/// `-[NSSplitView setDividerStyle:]`
pub extern "C" fn shim_nssplitview_set_divider_style(obj: u64, _cmd: u64, style: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    let cleared = flags & !0b110; // clear bits 1-2
    write_u64(obj, FLAGS_OFF, cleared | ((style & 0x3) << 1));
}

/// `-[NSSplitView adjustSubviews]` — distributes subviews evenly within the split view frame.
pub extern "C" fn shim_nssplitview_adjust_subviews(obj: u64, _cmd: u64) {
    if obj == 0 { return; }
    let subs = crate::nsview::snapshot_subviews_pub(obj);
    let count = subs.len();
    if count == 0 { return; }

    let frame = crate::nsview::view_frame(obj);
    let is_vertical = (read_u64(obj, FLAGS_OFF) & 1) != 0;
    let thickness = shim_nssplitview_divider_thickness(obj, 0);
    let total_dividers = (count - 1) as f64 * thickness;

    if is_vertical {
        let pane_w = (frame.size.width - total_dividers) / count as f64;
        let mut cur_x = frame.origin.x;
        for (i, &sub) in subs.iter().enumerate() {
            crate::nsview::shim_nsview_set_frame(sub, 0, cur_x, frame.origin.y, pane_w, frame.size.height);
            cur_x += pane_w;
            if i < count - 1 { cur_x += thickness; }
        }
    } else {
        let pane_h = (frame.size.height - total_dividers) / count as f64;
        let mut cur_y = frame.origin.y;
        for (i, &sub) in subs.iter().enumerate() {
            crate::nsview::shim_nsview_set_frame(sub, 0, frame.origin.x, cur_y, frame.size.width, pane_h);
            cur_y += pane_h;
            if i < count - 1 { cur_y += thickness; }
        }
    }
}

/// `-[NSSplitView drawRect:]` — renders divider lines between subviews.
pub extern "C" fn shim_nssplitview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let subs = crate::nsview::snapshot_subviews_pub(obj);
    let count = subs.len();
    if count <= 1 { return; }

    let is_vertical = (read_u64(obj, FLAGS_OFF) & 1) != 0;
    let thickness = shim_nssplitview_divider_thickness(obj, 0);

    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::new(0.78, 0.78, 0.78, 1.0);

        for &sub in subs.iter().take(count - 1) {
            let sf = crate::nsview::view_frame(sub);
            if is_vertical {
                let div_x = sf.origin.x + sf.size.width;
                c.fill_rect(div_x, y, thickness, h);
            } else {
                let div_y = sf.origin.y + sf.size.height;
                c.fill_rect(x, div_y, w, thickness);
            }
        }
    });
}

/// `-[NSSplitView mouseDown:]` — begin divider drag if hit in divider region.
pub extern "C" fn shim_nssplitview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let is_vertical = (read_u64(obj, FLAGS_OFF) & 1) != 0;
    let thickness = shim_nssplitview_divider_thickness(obj, 0);
    let subs = crate::nsview::snapshot_subviews_pub(obj);
    let count = subs.len();
    if count <= 1 { return; }

    // Find which divider was hit.
    let hit_pos = if is_vertical { loc_x - frame.origin.x } else { loc_y - frame.origin.y };
    let mut _divider_idx: Option<usize> = None;

    for (i, &sub) in subs.iter().take(count - 1).enumerate() {
        let sf = crate::nsview::view_frame(sub);
        let div_start = if is_vertical {
            sf.origin.x + sf.size.width - frame.origin.x
        } else {
            sf.origin.y + sf.size.height - frame.origin.y
        };
        if hit_pos >= div_start && hit_pos <= div_start + thickness {
            _divider_idx = Some(i);
            break;
        }
    }

    // Store the drag state for mouseDragged.
    if let Some(idx) = _divider_idx {
        let mut guard = DRAG_STATE.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(DragState {
            split_view: obj,
            divider_index: idx,
            start_pos: hit_pos,
        });
    }
}

/// `-[NSSplitView mouseDragged:]` — move divider to new position.
pub extern "C" fn shim_nssplitview_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let drag = {
        let guard = DRAG_STATE.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    };
    let Some(drag) = drag else { return; };
    if drag.split_view != obj { return; }

    let (loc_x, loc_y) = crate::nsevent::event_location_in_window(event);
    let frame = crate::nsview::view_frame(obj);
    let is_vertical = (read_u64(obj, FLAGS_OFF) & 1) != 0;
    let subs = crate::nsview::snapshot_subviews_pub(obj);
    let idx = drag.divider_index;
    if idx + 1 >= subs.len() { return; }

    let current_pos = if is_vertical { loc_x - frame.origin.x } else { loc_y - frame.origin.y };
    let delta = current_pos - drag.start_pos;

    // Resize the two adjacent subviews.
    let left = subs[idx];
    let right = subs[idx + 1];
    let lf = crate::nsview::view_frame(left);
    let rf = crate::nsview::view_frame(right);

    if is_vertical {
        let new_lw = (lf.size.width + delta).max(20.0);
        let new_rw = (rf.size.width - delta).max(20.0);
        let thickness = shim_nssplitview_divider_thickness(obj, 0);
        crate::nsview::shim_nsview_set_frame(left, 0, lf.origin.x, lf.origin.y, new_lw, lf.size.height);
        crate::nsview::shim_nsview_set_frame(right, 0, lf.origin.x + new_lw + thickness, rf.origin.y, new_rw, rf.size.height);
    } else {
        let new_lh = (lf.size.height + delta).max(20.0);
        let new_rh = (rf.size.height - delta).max(20.0);
        let thickness = shim_nssplitview_divider_thickness(obj, 0);
        crate::nsview::shim_nsview_set_frame(left, 0, lf.origin.x, lf.origin.y, lf.size.width, new_lh);
        crate::nsview::shim_nsview_set_frame(right, 0, rf.origin.x, lf.origin.y + new_lh + thickness, rf.size.width, new_rh);
    }

    // Update drag start for next delta.
    {
        let mut guard = DRAG_STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut ds) = *guard {
            ds.start_pos = current_pos;
        }
    }

    crate::nsview::shim_nsview_set_needs_display(obj, 0, 1);
}

/// `-[NSSplitView mouseUp:]` — end divider drag.
pub extern "C" fn shim_nssplitview_mouse_up(obj: u64, _cmd: u64, _event: u64) {
    if obj == 0 { return; }
    let mut guard = DRAG_STATE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().map(|d| d.split_view) == Some(obj) {
        *guard = None;
    }
}

use std::sync::Mutex;

#[derive(Clone)]
struct DragState {
    split_view: u64,
    divider_index: usize,
    start_pos: f64,
}

static DRAG_STATE: Mutex<Option<DragState>> = Mutex::new(None);

/// `-[NSSplitView dividerThickness]`
pub extern "C" fn shim_nssplitview_divider_thickness(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let style = (read_u64(obj, FLAGS_OFF) >> 1) & 0x3;
    match style {
        0 => 1.0,  // thin
        _ => 10.0, // thick (1) or paneSplitter (2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nssplitview_is_vertical(0, 0), 0);
        shim_nssplitview_set_vertical(0, 0, 1);
        assert_eq!(shim_nssplitview_delegate(0, 0), 0);
        shim_nssplitview_set_delegate(0, 0, 0x1234);
        assert_eq!(shim_nssplitview_divider_style(0, 0), 0);
        shim_nssplitview_set_divider_style(0, 0, 1);
        shim_nssplitview_adjust_subviews(0, 0);
        assert_eq!(shim_nssplitview_divider_thickness(0, 0), 0.0);
        shim_nssplitview_draw_rect(0, 0, 0.0, 0.0, 100.0, 100.0);
        shim_nssplitview_mouse_down(0, 0, 0);
        shim_nssplitview_mouse_dragged(0, 0, 0);
        shim_nssplitview_mouse_up(0, 0, 0);
    }

    #[test]
    fn vertical_toggle() {
        let sv = alloc_obj(SPLIT_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nssplitview_is_vertical(sv, 0), 0);
        shim_nssplitview_set_vertical(sv, 0, 1);
        assert_eq!(shim_nssplitview_is_vertical(sv, 0), 1);
        shim_nssplitview_set_vertical(sv, 0, 0);
        assert_eq!(shim_nssplitview_is_vertical(sv, 0), 0);
        free_obj(sv);
    }

    #[test]
    fn delegate_round_trip() {
        let sv = alloc_obj(SPLIT_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nssplitview_delegate(sv, 0), 0);
        shim_nssplitview_set_delegate(sv, 0, 0xCAFE);
        assert_eq!(shim_nssplitview_delegate(sv, 0), 0xCAFE);
        free_obj(sv);
    }

    #[test]
    fn divider_style_round_trip() {
        let sv = alloc_obj(SPLIT_VIEW_SIZE, 0xDEAD);
        // Default: thin (0)
        assert_eq!(shim_nssplitview_divider_style(sv, 0), 0);
        assert_eq!(shim_nssplitview_divider_thickness(sv, 0), 1.0);
        // Thick (1)
        shim_nssplitview_set_divider_style(sv, 0, 1);
        assert_eq!(shim_nssplitview_divider_style(sv, 0), 1);
        assert_eq!(shim_nssplitview_divider_thickness(sv, 0), 10.0);
        // PaneSplitter (2)
        shim_nssplitview_set_divider_style(sv, 0, 2);
        assert_eq!(shim_nssplitview_divider_style(sv, 0), 2);
        assert_eq!(shim_nssplitview_divider_thickness(sv, 0), 10.0);
        free_obj(sv);
    }

    #[test]
    fn vertical_and_divider_style_independent() {
        let sv = alloc_obj(SPLIT_VIEW_SIZE, 0xDEAD);
        shim_nssplitview_set_vertical(sv, 0, 1);
        shim_nssplitview_set_divider_style(sv, 0, 2);
        // Vertical flag should survive divider style change
        assert_eq!(shim_nssplitview_is_vertical(sv, 0), 1);
        assert_eq!(shim_nssplitview_divider_style(sv, 0), 2);
        // Changing vertical shouldn't clobber divider style
        shim_nssplitview_set_vertical(sv, 0, 0);
        assert_eq!(shim_nssplitview_divider_style(sv, 0), 2);
        free_obj(sv);
    }
}
