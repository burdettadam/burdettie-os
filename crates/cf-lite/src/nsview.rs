//! NSView shim: view stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa           (u64)
//!   offset  8: superview     (u64 — parent NSView*, or 0)
//!   offset 16: window        (u64 — owning NSWindow*, or 0)
//!   offset 24: frame_x       (f64)
//!   offset 32: frame_y       (f64)
//!   offset 40: frame_w       (f64)
//!   offset 48: frame_h       (f64)
//!   offset 56: subviews_ptr  (u64 — malloc'd id[] array, or 0)
//!   offset 64: subview_count (u64)
//!   offset 72: flags         (u64 — bit 0: needs_display, bit 1: wants_layer)
//!
//! Total: 80 bytes.

use crate::cg_types::{CGRect, CGPoint, CGSize};

const SUPERVIEW_OFF: usize = 8;
const WINDOW_OFF: usize = 16;
const FRAME_X_OFF: usize = 24;
const FRAME_Y_OFF: usize = 32;
const FRAME_W_OFF: usize = 40;
const FRAME_H_OFF: usize = 48;
const SUBVIEWS_PTR_OFF: usize = 56;
const SUBVIEW_COUNT_OFF: usize = 64;
const FLAGS_OFF: usize = 72;
pub(crate) const VIEW_SIZE: usize = 80;

const FLAG_NEEDS_DISPLAY: u64 = 1 << 0;
const FLAG_WANTS_LAYER: u64 = 1 << 1;

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

fn write_frame(view: u64, x: f64, y: f64, w: f64, h: f64) {
    write_f64(view, FRAME_X_OFF, x);
    write_f64(view, FRAME_Y_OFF, y);
    write_f64(view, FRAME_W_OFF, w);
    write_f64(view, FRAME_H_OFF, h);
}

fn snapshot_subviews(view: u64) -> Vec<u64> {
    let count = read_u64(view, SUBVIEW_COUNT_OFF) as usize;
    let ptr = read_u64(view, SUBVIEWS_PTR_OFF);
    if count == 0 || ptr == 0 {
        return Vec::new();
    }

    let mut children = Vec::with_capacity(count);
    for index in 0..count {
        children.push(unsafe { *((ptr as usize + index * 8) as *const u64) });
    }
    children
}

fn write_subviews(view: u64, children: &[u64]) -> bool {
    if view == 0 {
        return false;
    }

    let old_ptr = read_u64(view, SUBVIEWS_PTR_OFF);
    if children.is_empty() {
        if old_ptr != 0 {
            crate::pal_alloc().free(old_ptr as *mut u8);
        }
        write_u64(view, SUBVIEWS_PTR_OFF, 0);
        write_u64(view, SUBVIEW_COUNT_OFF, 0);
        return true;
    }

    let new_ptr = crate::pal_alloc().realloc(old_ptr as *mut u8, children.len() * 8) as u64;
    if new_ptr == 0 {
        return false;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(children.as_ptr(), new_ptr as *mut u64, children.len());
    }
    write_u64(view, SUBVIEWS_PTR_OFF, new_ptr);
    write_u64(view, SUBVIEW_COUNT_OFF, children.len() as u64);
    true
}

fn append_subview(parent: u64, child: u64) -> bool {
    if parent == 0 || child == 0 {
        return false;
    }

    let mut children = snapshot_subviews(parent);
    if children.contains(&child) {
        return true;
    }
    children.push(child);
    write_subviews(parent, &children)
}

fn remove_subview(parent: u64, child: u64) {
    if parent == 0 || child == 0 {
        return;
    }

    let mut children = snapshot_subviews(parent);
    let original_len = children.len();
    children.retain(|&candidate| candidate != child);
    if children.len() != original_len {
        let _ = write_subviews(parent, &children);
    }
}

fn would_create_cycle(parent: u64, child: u64) -> bool {
    let mut cursor = parent;
    while cursor != 0 {
        if cursor == child {
            return true;
        }
        cursor = read_u64(cursor, SUPERVIEW_OFF);
    }
    false
}

pub(crate) fn set_view_superview(view: u64, superview: u64) {
    write_u64(view, SUPERVIEW_OFF, superview);
}

pub(crate) fn set_view_window(view: u64, window: u64) {
    if view == 0 {
        return;
    }
    write_u64(view, WINDOW_OFF, window);
    for child in snapshot_subviews(view) {
        set_view_window(child, window);
    }
}

pub(crate) fn remove_view_from_superview(view: u64) {
    if view == 0 {
        return;
    }
    let parent = read_u64(view, SUPERVIEW_OFF);
    if parent != 0 {
        remove_subview(parent, view);
    }
    set_view_superview(view, 0);
    set_view_window(view, 0);
}

pub(crate) fn alloc_view_with_frame(x: f64, y: f64, w: f64, h: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, VIEW_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsview_class_addr();
    }
    write_frame(obj, x, y, w, h);
    obj
}

/// `-[NSView initWithFrame:]`
pub extern "C" fn shim_nsview_init_with_frame(
    _obj: u64,
    _cmd: u64,
    x: f64, y: f64, w: f64, h: f64,
) -> u64 {
    let obj = alloc_view_with_frame(x, y, w, h);
    let _ = CGRect { origin: CGPoint { x, y }, size: CGSize { width: w, height: h } };
    obj
}

/// `-[NSView addSubview:]` — attaches the subview to this view.
pub extern "C" fn shim_nsview_add_subview(obj: u64, _cmd: u64, subview: u64) {
    if obj == 0 || subview == 0 || obj == subview || would_create_cycle(obj, subview) {
        return;
    }

    let previous_parent = read_u64(subview, SUPERVIEW_OFF);
    if previous_parent != 0 && previous_parent != obj {
        remove_subview(previous_parent, subview);
    }

    if !append_subview(obj, subview) {
        return;
    }

    set_view_superview(subview, obj);
    let window = shim_nsview_window(obj, 0);
    set_view_window(subview, window);
}

/// `-[NSView subviews]`
pub extern "C" fn shim_nsview_subviews(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::nsarray::make_nsarray(&snapshot_subviews(obj))
}

/// `-[NSView removeFromSuperview]`
pub extern "C" fn shim_nsview_remove_from_superview(obj: u64, _cmd: u64) {
    remove_view_from_superview(obj);
}

/// `-[NSView setFrame:]`
pub extern "C" fn shim_nsview_set_frame(
    obj: u64,
    _cmd: u64,
    x: f64, y: f64, w: f64, h: f64,
) {
    write_frame(obj, x, y, w, h);
}

/// `-[NSView setFrameSize:]`
pub extern "C" fn shim_nsview_set_frame_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    write_f64(obj, FRAME_W_OFF, w);
    write_f64(obj, FRAME_H_OFF, h);
}

/// `-[NSView setFrameOrigin:]`
pub extern "C" fn shim_nsview_set_frame_origin(obj: u64, _cmd: u64, x: f64, y: f64) {
    if obj == 0 {
        return;
    }
    write_f64(obj, FRAME_X_OFF, x);
    write_f64(obj, FRAME_Y_OFF, y);
}

/// `-[NSView setNeedsDisplay:]`
pub extern "C" fn shim_nsview_set_needs_display(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_NEEDS_DISPLAY;
    } else {
        flags &= !FLAG_NEEDS_DISPLAY;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSView needsDisplay]`
pub extern "C" fn shim_nsview_needs_display(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_NEEDS_DISPLAY != 0)
}

/// `-[NSView drawRect:]` — base implementation does nothing.
pub extern "C" fn shim_nsview_draw_rect(_obj: u64, _cmd: u64, _x: f64, _y: f64, _w: f64, _h: f64) {}

/// `-[NSView wantsLayer]`
pub extern "C" fn shim_nsview_wants_layer(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_WANTS_LAYER != 0)
}

/// `-[NSView setWantsLayer:]`
pub extern "C" fn shim_nsview_set_wants_layer(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    if flag != 0 {
        flags |= FLAG_WANTS_LAYER;
    } else {
        flags &= !FLAG_WANTS_LAYER;
    }
    write_u64(obj, FLAGS_OFF, flags);
}

/// `-[NSView layer]` — returns nil (no real CALayer).
pub extern "C" fn shim_nsview_layer(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSView superview]`
pub extern "C" fn shim_nsview_superview(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SUPERVIEW_OFF)
}

/// `-[NSView frame]` — returns first f64 of frame (x) as simplified stub.
pub extern "C" fn shim_nsview_frame(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, FRAME_X_OFF)
}

/// `-[NSView bounds]` — same as frame for this stub.
pub extern "C" fn shim_nsview_bounds(obj: u64, _cmd: u64) -> f64 {
    shim_nsview_frame(obj, _cmd)
}

/// `-[NSView window]`
pub extern "C" fn shim_nsview_window(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, WINDOW_OFF)
}

/// `-[NSView isFlipped]` — returns NO.
pub extern "C" fn shim_nsview_is_flipped(_obj: u64, _cmd: u64) -> u8 {
    0
}

// ── Auto Layout pass methods ────────────────────────────────────

/// `-[NSView setNeedsLayout:]`
pub extern "C" fn shim_nsview_set_needs_layout(obj: u64, _cmd: u64, flag: u8) {
    crate::layout_solver::set_needs_layout(obj, flag != 0);
}

/// `-[NSView needsLayout]`
pub extern "C" fn shim_nsview_needs_layout(obj: u64, _cmd: u64) -> u8 {
    u8::from(crate::layout_solver::needs_layout(obj))
}

/// `-[NSView layoutIfNeeded]`
pub extern "C" fn shim_nsview_layout_if_needed(obj: u64, _cmd: u64) {
    crate::layout_solver::layout_if_needed(obj);
}

/// `-[NSView layoutSubtreeIfNeeded]` — same as layoutIfNeeded for this impl.
pub extern "C" fn shim_nsview_layout_subtree_if_needed(obj: u64, _cmd: u64) {
    crate::layout_solver::layout_if_needed(obj);
}

/// `-[NSView updateConstraints]` — no-op base implementation.
pub extern "C" fn shim_nsview_update_constraints(_obj: u64, _cmd: u64) {}

/// `-[NSView setNeedsUpdateConstraints]`
pub extern "C" fn shim_nsview_set_needs_update_constraints(obj: u64, _cmd: u64) {
    crate::layout_solver::set_needs_layout(obj, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = VIEW_SIZE;

    fn isa() -> u64 {
        crate::nsobj::nsview_class_addr()
    }

    fn child_at(view: u64, index: usize) -> u64 {
        let ptr = read_field(view, SUBVIEWS_PTR_OFF);
        if ptr == 0 {
            return 0;
        }
        unsafe { *((ptr as usize + index * 8) as *const u64) }
    }

    fn make_view() -> u64 {
        alloc_obj(SIZE, isa())
    }

    // ── shim_nsview_init_with_frame ──────────────────────────────

    #[test]
    fn init_with_frame_basic() {
        let obj = shim_nsview_init_with_frame(0, 0, 10.0, 20.0, 300.0, 400.0);
        assert_ne!(obj, 0);
        // ISA should be set
        assert_eq!(read_field(obj, 0), isa());
        // frame fields
        assert_eq!(read_f64(obj, FRAME_X_OFF), 10.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 8), 20.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 16), 300.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 24), 400.0);
        // superview should be 0 (calloc zeroed)
        assert_eq!(read_field(obj, SUPERVIEW_OFF), 0);
        assert_eq!(read_field(obj, WINDOW_OFF), 0);
        assert_eq!(read_field(obj, SUBVIEWS_PTR_OFF), 0);
        assert_eq!(read_field(obj, SUBVIEW_COUNT_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_frame_zero_size() {
        let obj = shim_nsview_init_with_frame(0, 0, 0.0, 0.0, 0.0, 0.0);
        assert_ne!(obj, 0);
        assert_eq!(read_f64(obj, FRAME_X_OFF), 0.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 8), 0.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 16), 0.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 24), 0.0);
        free_obj(obj);
    }

    #[test]
    fn init_with_frame_negative_coords() {
        let obj = shim_nsview_init_with_frame(0, 0, -5.0, -10.0, 100.0, 200.0);
        assert_ne!(obj, 0);
        assert_eq!(read_f64(obj, FRAME_X_OFF), -5.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF + 8), -10.0);
        free_obj(obj);
    }

    // ── shim_nsview_add_subview ──────────────────────────────────

    #[test]
    fn add_subview_nil_obj() {
        // Should not crash with nil receiver
        shim_nsview_add_subview(0, 0, 0);
    }

    #[test]
    fn add_subview_with_valid_obj() {
        let obj = make_view();
        let sub = make_view();
        write_field(obj, WINDOW_OFF, 0xF00D);
        shim_nsview_add_subview(obj, 0, sub);
        assert_eq!(shim_nsview_superview(sub, 0), obj);
        assert_eq!(shim_nsview_window(sub, 0), 0xF00D);
        assert_eq!(read_field(obj, SUBVIEW_COUNT_OFF), 1);
        assert_eq!(child_at(obj, 0), sub);
        let subviews = shim_nsview_subviews(obj, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(subviews, 0), 1);
        assert_eq!(crate::nsarray::shim_nsarray_object_at_index(subviews, 0, 0), sub);
        free_obj(sub);
        free_obj(obj);
    }

    #[test]
    fn add_subview_reparents_child() {
        let first_parent = make_view();
        let second_parent = make_view();
        let child = make_view();

        write_field(first_parent, WINDOW_OFF, 0xAAAA);
        write_field(second_parent, WINDOW_OFF, 0xBBBB);

        shim_nsview_add_subview(first_parent, 0, child);
        shim_nsview_add_subview(second_parent, 0, child);

        assert_eq!(shim_nsview_superview(child, 0), second_parent);
        assert_eq!(shim_nsview_window(child, 0), 0xBBBB);
        assert_eq!(read_field(first_parent, SUBVIEW_COUNT_OFF), 0);
        assert_eq!(read_field(second_parent, SUBVIEW_COUNT_OFF), 1);

        free_obj(child);
        free_obj(second_parent);
        free_obj(first_parent);
    }

    #[test]
    fn add_subview_rejects_cycles() {
        let root = make_view();
        let child = make_view();

        shim_nsview_add_subview(root, 0, child);
        shim_nsview_add_subview(child, 0, root);

        assert_eq!(shim_nsview_superview(root, 0), 0);
        assert_eq!(read_field(child, SUBVIEW_COUNT_OFF), 0);

        free_obj(child);
        free_obj(root);
    }

    // ── shim_nsview_subviews / remove_from_superview ─────────────

    #[test]
    fn subviews_nil_returns_zero() {
        assert_eq!(shim_nsview_subviews(0, 0), 0);
    }

    #[test]
    fn subviews_default_empty_array() {
        let obj = make_view();
        let subviews = shim_nsview_subviews(obj, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(subviews, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn remove_from_superview_detaches_view() {
        let parent = make_view();
        let child = make_view();
        let grandchild = make_view();
        write_field(parent, WINDOW_OFF, 0xCAFE);

        shim_nsview_add_subview(child, 0, grandchild);
        shim_nsview_add_subview(parent, 0, child);
        shim_nsview_remove_from_superview(child, 0);

        assert_eq!(shim_nsview_superview(child, 0), 0);
        assert_eq!(shim_nsview_window(child, 0), 0);
        assert_eq!(shim_nsview_window(grandchild, 0), 0);
        assert_eq!(read_field(parent, SUBVIEW_COUNT_OFF), 0);

        free_obj(grandchild);
        free_obj(child);
        free_obj(parent);
    }

    // ── frame setters ────────────────────────────────────────────

    #[test]
    fn set_frame_updates_all_components() {
        let obj = make_view();
        shim_nsview_set_frame(obj, 0, 11.0, 22.0, 33.0, 44.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF), 11.0);
        assert_eq!(read_f64(obj, FRAME_Y_OFF), 22.0);
        assert_eq!(read_f64(obj, FRAME_W_OFF), 33.0);
        assert_eq!(read_f64(obj, FRAME_H_OFF), 44.0);
        free_obj(obj);
    }

    #[test]
    fn set_frame_size_preserves_origin() {
        let obj = shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);
        shim_nsview_set_frame_size(obj, 0, 30.0, 40.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF), 1.0);
        assert_eq!(read_f64(obj, FRAME_Y_OFF), 2.0);
        assert_eq!(read_f64(obj, FRAME_W_OFF), 30.0);
        assert_eq!(read_f64(obj, FRAME_H_OFF), 40.0);
        free_obj(obj);
    }

    #[test]
    fn set_frame_origin_preserves_size() {
        let obj = shim_nsview_init_with_frame(0, 0, 1.0, 2.0, 3.0, 4.0);
        shim_nsview_set_frame_origin(obj, 0, 10.0, 20.0);
        assert_eq!(read_f64(obj, FRAME_X_OFF), 10.0);
        assert_eq!(read_f64(obj, FRAME_Y_OFF), 20.0);
        assert_eq!(read_f64(obj, FRAME_W_OFF), 3.0);
        assert_eq!(read_f64(obj, FRAME_H_OFF), 4.0);
        free_obj(obj);
    }

    // ── shim_nsview_set_needs_display ────────────────────────────

    #[test]
    fn set_needs_display_nil_obj() {
        shim_nsview_set_needs_display(0, 0, 1);
    }

    #[test]
    fn set_needs_display_with_valid_obj() {
        let obj = alloc_obj(SIZE, isa());
        shim_nsview_set_needs_display(obj, 0, 1);
        shim_nsview_set_needs_display(obj, 0, 0);
        free_obj(obj);
    }

    // ── shim_nsview_superview ────────────────────────────────────

    #[test]
    fn superview_nil() {
        assert_eq!(shim_nsview_superview(0, 0), 0);
    }

    #[test]
    fn superview_default_zero() {
        let obj = make_view();
        assert_eq!(shim_nsview_superview(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn superview_set_and_read() {
        let parent = make_view();
        let child = make_view();
        write_field(child, SUPERVIEW_OFF, parent);
        assert_eq!(shim_nsview_superview(child, 0), parent);
        free_obj(child);
        free_obj(parent);
    }

    // ── shim_nsview_frame ────────────────────────────────────────

    #[test]
    fn frame_nil() {
        assert_eq!(shim_nsview_frame(0, 0), 0.0);
    }

    #[test]
    fn frame_returns_x() {
        let obj = shim_nsview_init_with_frame(0, 0, 42.5, 0.0, 0.0, 0.0);
        assert_eq!(shim_nsview_frame(obj, 0), 42.5);
        free_obj(obj);
    }

    #[test]
    fn frame_manual_layout() {
        let obj = make_view();
        write_f64(obj, FRAME_X_OFF, 99.0);
        assert_eq!(shim_nsview_frame(obj, 0), 99.0);
        free_obj(obj);
    }

    // ── shim_nsview_bounds ───────────────────────────────────────

    #[test]
    fn bounds_nil() {
        assert_eq!(shim_nsview_bounds(0, 0), 0.0);
    }

    #[test]
    fn bounds_matches_frame() {
        let obj = shim_nsview_init_with_frame(0, 0, 7.5, 0.0, 0.0, 0.0);
        assert_eq!(shim_nsview_bounds(obj, 0), shim_nsview_frame(obj, 0));
        free_obj(obj);
    }

    // ── shim_nsview_window ───────────────────────────────────────

    #[test]
    fn window_nil() {
        assert_eq!(shim_nsview_window(0, 0), 0);
    }

    #[test]
    fn window_reads_assigned_pointer() {
        let obj = make_view();
        write_field(obj, WINDOW_OFF, 0xCAFE);
        assert_eq!(shim_nsview_window(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    #[test]
    fn set_view_window_propagates_to_descendants() {
        let parent = make_view();
        let child = make_view();
        let grandchild = make_view();

        shim_nsview_add_subview(child, 0, grandchild);
        shim_nsview_add_subview(parent, 0, child);
        set_view_window(parent, 0xD00D);

        assert_eq!(shim_nsview_window(parent, 0), 0xD00D);
        assert_eq!(shim_nsview_window(child, 0), 0xD00D);
        assert_eq!(shim_nsview_window(grandchild, 0), 0xD00D);

        free_obj(grandchild);
        free_obj(child);
        free_obj(parent);
    }

    // ── shim_nsview_is_flipped ───────────────────────────────────

    #[test]
    fn is_flipped_nil() {
        assert_eq!(shim_nsview_is_flipped(0, 0), 0);
    }

    #[test]
    fn is_flipped_always_no() {
        let obj = make_view();
        assert_eq!(shim_nsview_is_flipped(obj, 0), 0);
        free_obj(obj);
    }

    // ── shim_nsview_needs_display / set_needs_display ─────────────

    #[test]
    fn needs_display_nil_returns_zero() {
        assert_eq!(shim_nsview_needs_display(0, 0), 0);
    }

    #[test]
    fn needs_display_default_is_no() {
        let obj = make_view();
        assert_eq!(shim_nsview_needs_display(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn set_needs_display_roundtrip() {
        let obj = make_view();
        shim_nsview_set_needs_display(obj, 0, 1);
        assert_eq!(shim_nsview_needs_display(obj, 0), 1);
        shim_nsview_set_needs_display(obj, 0, 0);
        assert_eq!(shim_nsview_needs_display(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn set_needs_display_nil_does_not_crash() {
        shim_nsview_set_needs_display(0, 0, 1);
    }

    // ── shim_nsview_draw_rect ─────────────────────────────────────

    #[test]
    fn draw_rect_does_not_crash() {
        let obj = make_view();
        shim_nsview_draw_rect(obj, 0, 0.0, 0.0, 100.0, 200.0);
        free_obj(obj);
    }

    #[test]
    fn draw_rect_nil_does_not_crash() {
        shim_nsview_draw_rect(0, 0, 0.0, 0.0, 50.0, 50.0);
    }

    // ── shim_nsview_wants_layer / set_wants_layer ─────────────────

    #[test]
    fn wants_layer_nil_returns_zero() {
        assert_eq!(shim_nsview_wants_layer(0, 0), 0);
    }

    #[test]
    fn wants_layer_default_is_no() {
        let obj = make_view();
        assert_eq!(shim_nsview_wants_layer(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn set_wants_layer_roundtrip() {
        let obj = make_view();
        shim_nsview_set_wants_layer(obj, 0, 1);
        assert_eq!(shim_nsview_wants_layer(obj, 0), 1);
        shim_nsview_set_wants_layer(obj, 0, 0);
        assert_eq!(shim_nsview_wants_layer(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn set_wants_layer_nil_does_not_crash() {
        shim_nsview_set_wants_layer(0, 0, 1);
    }

    #[test]
    fn needs_display_and_wants_layer_independent() {
        let obj = make_view();
        shim_nsview_set_needs_display(obj, 0, 1);
        shim_nsview_set_wants_layer(obj, 0, 1);
        assert_eq!(shim_nsview_needs_display(obj, 0), 1);
        assert_eq!(shim_nsview_wants_layer(obj, 0), 1);
        shim_nsview_set_needs_display(obj, 0, 0);
        assert_eq!(shim_nsview_needs_display(obj, 0), 0);
        assert_eq!(shim_nsview_wants_layer(obj, 0), 1);
        free_obj(obj);
    }

    // ── shim_nsview_layer ─────────────────────────────────────────

    #[test]
    fn layer_nil_returns_zero() {
        assert_eq!(shim_nsview_layer(0, 0), 0);
    }

    #[test]
    fn layer_always_returns_zero() {
        let obj = make_view();
        assert_eq!(shim_nsview_layer(obj, 0), 0);
        shim_nsview_set_wants_layer(obj, 0, 1);
        assert_eq!(shim_nsview_layer(obj, 0), 0);
        free_obj(obj);
    }
}
