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

/// `-[NSSplitView adjustSubviews]` — no-op stub.
pub extern "C" fn shim_nssplitview_adjust_subviews(_obj: u64, _cmd: u64) {}

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
