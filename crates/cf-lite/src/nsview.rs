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

use crate::cg_types::{CGPoint, CGRect};

use std::collections::HashMap;
use std::sync::Mutex;

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
// Bit 2: translates_autoresize (inverted, managed by nslayoutconstraint.rs)
// Bit 3: needs_layout (managed by layout_solver.rs)

// ── Autoresizing mask (bits 4-9) — matches Apple values shifted by 4 ──
// Apple's NSAutoresizingMaskOptions:
//   NSViewMinXMargin     = 1
//   NSViewWidthSizable   = 2
//   NSViewMaxXMargin     = 4
//   NSViewMinYMargin     = 8
//   NSViewHeightSizable  = 16
//   NSViewMaxYMargin     = 32
const AUTORESIZE_SHIFT: u32 = 4;
const AUTORESIZE_MASK_BITS: u64 = 0x3F << AUTORESIZE_SHIFT; // 6 bits

// ── Per-view layout priority storage (outside the 80-byte object) ───

/// Content hugging and compression resistance priorities per view.
struct ViewLayoutPriorities {
    hugging_h: f64,
    hugging_v: f64,
    compression_h: f64,
    compression_v: f64,
}

impl Default for ViewLayoutPriorities {
    fn default() -> Self {
        Self {
            hugging_h: 250.0, // NSLayoutPriorityDefaultLow
            hugging_v: 250.0,
            compression_h: 750.0, // NSLayoutPriorityDefaultHigh
            compression_v: 750.0,
        }
    }
}

static VIEW_PRIORITIES: Mutex<Option<HashMap<u64, ViewLayoutPriorities>>> = Mutex::new(None);

fn view_layout_priorities<F, R>(view: u64, f: F) -> R
where
    F: FnOnce(&ViewLayoutPriorities) -> R,
{
    let guard = VIEW_PRIORITIES.lock().unwrap_or_else(|e| e.into_inner());
    let default = ViewLayoutPriorities::default();
    match &*guard {
        Some(map) => f(map.get(&view).unwrap_or(&default)),
        None => f(&default),
    }
}

fn view_layout_priorities_mut<F>(view: u64, f: F)
where
    F: FnOnce(&mut ViewLayoutPriorities),
{
    let mut guard = VIEW_PRIORITIES.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map.entry(view).or_default());
}

/// Reset priority state (called from `reset_runtime_state_for_reuse`).
pub(crate) fn reset_layout_priority_state() {
    let mut guard = VIEW_PRIORITIES.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
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

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn object_is_kind_of(obj: u64, class_addr: u64) -> bool {
    if obj == 0 || class_addr == 0 {
        return false;
    }

    let isa = read_u64(obj, 0);
    if isa == class_addr {
        return true;
    }

    crate::nsobj::shim_nsobject_is_kind_of_class(obj, 0, class_addr) != 0
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

/// Public accessor for the accessibility bridge.
pub(crate) fn snapshot_subviews_pub(view: u64) -> Vec<u64> {
    snapshot_subviews(view)
}

/// Public accessor: get view frame as CGRect.
pub(crate) fn view_frame(view: u64) -> crate::cg_types::CGRect {
    crate::cg_types::CGRect {
        origin: crate::cg_types::CGPoint {
            x: read_f64(view, FRAME_X_OFF),
            y: read_f64(view, FRAME_Y_OFF),
        },
        size: crate::cg_types::CGSize {
            width: read_f64(view, FRAME_W_OFF),
            height: read_f64(view, FRAME_H_OFF),
        },
    }
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

fn add_archived_subviews(view: u64, archived_subviews: u64) {
    if view == 0 || archived_subviews == 0 {
        return;
    }

    let isa = read_u64(archived_subviews, 0);
    if isa == crate::nsobj::nsarray_class_addr() || isa == crate::nsobj::nsmutablearray_class_addr()
    {
        let count = crate::nsarray::shim_nsarray_count(archived_subviews, 0);
        for index in 0..count {
            let child = crate::nsarray::shim_nsarray_object_at_index(archived_subviews, 0, index);
            if object_is_kind_of(child, crate::nsobj::nsview_class_addr()) {
                shim_nsview_add_subview(view, 0, child);
            }
        }
        return;
    }

    if object_is_kind_of(archived_subviews, crate::nsobj::nsview_class_addr()) {
        shim_nsview_add_subview(view, 0, archived_subviews);
    }
}

/// `-[NSView initWithCoder:]`.
///
/// Compiled nibs/storyboards usually create views via keyed unarchiving.  We
/// decode the common AppKit keys here so generic scene archives produce a real
/// frame and child tree instead of a zeroed placeholder.
pub extern "C" fn shim_nsview_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    if let Some((x, y, w, h)) = crate::nskeyedarchiver::decode_rect_for_any_key(
        coder,
        &["NSFrame", "NSFrameRect", "NS.frame", "frame", "_frame"],
    ) {
        write_frame(obj, x, y, w, h);
    }

    if let Some(mask) = crate::nskeyedarchiver::decode_integer_for_any_key(
        coder,
        &[
            "NSAutoresizingMask",
            "NSAutoresizingMaskOptions",
            "autoresizingMask",
            "_autoresizingMask",
        ],
    ) {
        shim_nsview_set_autoresizing_mask(obj, 0, mask.max(0) as u64);
    }

    if let Some(wants_layer) = crate::nskeyedarchiver::decode_bool_for_any_key(
        coder,
        &["NSWantsLayer", "wantsLayer", "_wantsLayer"],
    ) {
        shim_nsview_set_wants_layer(obj, 0, u8::from(wants_layer));
    }

    let subviews = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &["NSSubviews", "NS.subviews", "subviews", "_subviews"],
    );
    add_archived_subviews(obj, subviews);

    obj
}

/// `-[NSView initWithFrame:]`
///
/// If obj was already allocated by `+[CustomViewSubclass alloc]`, we must preserve
/// the original ISA (class identity) and just initialize the frame fields.
/// This is critical for dynamic subclasses that override `drawRect:`.
pub extern "C" fn shim_nsview_init_with_frame(
    obj: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> u64 {
    if obj == 0 {
        // No object provided — allocate with base NSView ISA.
        return alloc_view_with_frame(x, y, w, h);
    }
    // Object is already allocated (via +alloc). Initialize frame fields in-place,
    // preserving the ISA that was set by the alloc step.
    // Ensure the object has enough space — alloc should have used instance_size.
    write_frame(obj, x, y, w, h);
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
pub extern "C" fn shim_nsview_set_frame(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    let old_w = read_f64(obj, FRAME_W_OFF);
    let old_h = read_f64(obj, FRAME_H_OFF);
    write_frame(obj, x, y, w, h);
    // Trigger autoresizing for subviews if size changed.
    if old_w != w || old_h != h {
        apply_autoresizing(obj, old_w, old_h);
    }
}

/// `-[NSView setFrameSize:]`
pub extern "C" fn shim_nsview_set_frame_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let old_w = read_f64(obj, FRAME_W_OFF);
    let old_h = read_f64(obj, FRAME_H_OFF);
    write_f64(obj, FRAME_W_OFF, w);
    write_f64(obj, FRAME_H_OFF, h);
    if old_w != w || old_h != h {
        apply_autoresizing(obj, old_w, old_h);
    }
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

    // If marking dirty, request a redraw from the bridge.
    if flag != 0 {
        let window = read_u64(obj, WINDOW_OFF);
        if window != 0 {
            if let Some(bridge) = crate::gui_bridge::gui_bridge() {
                bridge.request_redraw(window);
            }
        }
    }
}

/// `-[NSView needsDisplay]`
pub extern "C" fn shim_nsview_needs_display(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, FLAGS_OFF) & FLAG_NEEDS_DISPLAY != 0)
}

/// Internal accessor for other modules.
pub(crate) fn view_needs_display(obj: u64) -> bool {
    read_u64(obj, FLAGS_OFF) & FLAG_NEEDS_DISPLAY != 0
}

/// Internal accessor: frame x position.
pub(crate) fn view_frame_x(obj: u64) -> f64 {
    read_f64(obj, FRAME_X_OFF)
}

/// Internal accessor: frame y position.
pub(crate) fn view_frame_y(obj: u64) -> f64 {
    read_f64(obj, FRAME_Y_OFF)
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

/// `-[NSView layer]` — returns a lazily-created CALayer when wantsLayer is YES.
pub extern "C" fn shim_nsview_layer(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Only return a layer if wantsLayer is set.
    if read_u64(obj, FLAGS_OFF) & FLAG_WANTS_LAYER == 0 {
        return 0;
    }
    view_layer_or_create(obj)
}

/// Lazily create and cache a CALayer for a view.
fn view_layer_or_create(obj: u64) -> u64 {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static VIEW_LAYERS: std::sync::OnceLock<Mutex<HashMap<u64, u64>>> = std::sync::OnceLock::new();
    let map = VIEW_LAYERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    *guard.entry(obj).or_insert_with(crate::calayer::alloc_layer)
}

/// `-[NSView superview]`
pub extern "C" fn shim_nsview_superview(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SUPERVIEW_OFF)
}

/// `-[NSView frame]` — returns CGRect (32 bytes, x8 indirect return on ARM64).
pub extern "C" fn shim_nsview_frame(obj: u64, _cmd: u64) -> CGRect {
    if obj == 0 {
        return CGRect::ZERO;
    }
    CGRect::new(
        read_f64(obj, FRAME_X_OFF),
        read_f64(obj, FRAME_Y_OFF),
        read_f64(obj, FRAME_W_OFF),
        read_f64(obj, FRAME_H_OFF),
    )
}

/// `-[NSView bounds]` — origin at (0,0), same size as frame.
pub extern "C" fn shim_nsview_bounds(obj: u64, _cmd: u64) -> CGRect {
    let f = shim_nsview_frame(obj, _cmd);
    CGRect::new(0.0, 0.0, f.size.width, f.size.height)
}

/// `-[NSView window]`
pub extern "C" fn shim_nsview_window(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, WINDOW_OFF)
}

/// `-[NSView isFlipped]` — returns NO.
pub extern "C" fn shim_nsview_is_flipped(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSView hitTest:]` — find the deepest subview containing the point.
///
/// `point` is in the receiver's superview coordinate system.
/// Returns the deepest subview that contains the point, or self, or 0 if
/// the point is outside this view's frame.
pub extern "C" fn shim_nsview_hit_test(obj: u64, _cmd: u64, px: f64, py: f64) -> u64 {
    if obj == 0 {
        return 0;
    }

    // Check if point is inside this view's frame
    let fx = read_f64(obj, FRAME_X_OFF);
    let fy = read_f64(obj, FRAME_Y_OFF);
    let fw = read_f64(obj, FRAME_W_OFF);
    let fh = read_f64(obj, FRAME_H_OFF);
    if px < fx || px > fx + fw || py < fy || py > fy + fh {
        return 0;
    }

    // Convert to local coordinates
    let local_x = px - fx;
    let local_y = py - fy;

    // Check subviews in reverse order (front-to-back)
    let children = snapshot_subviews(obj);
    for &child in children.iter().rev() {
        let hit = shim_nsview_hit_test(child, 0, local_x, local_y);
        if hit != 0 {
            return hit;
        }
    }

    // No child hit — self is the target
    obj
}

// ── Coordinate conversion ───────────────────────────────────────

/// Walk the view hierarchy from `view` up to the root, accumulating frame
/// origins.  Returns the view's frame origin in *window* coordinates.
fn view_origin_in_window(view: u64) -> (f64, f64) {
    let mut x = 0.0_f64;
    let mut y = 0.0_f64;
    let mut cursor = view;
    let mut depth = 0;
    while cursor != 0 && depth < 128 {
        x += read_f64(cursor, FRAME_X_OFF);
        y += read_f64(cursor, FRAME_Y_OFF);
        cursor = read_u64(cursor, SUPERVIEW_OFF);
        depth += 1;
    }
    (x, y)
}

/// `-[NSView convertPoint:fromView:]`
///
/// Converts a point from the coordinate system of `from_view` to this view.
/// If `from_view` is nil (0), the point is assumed to be in window coordinates.
pub extern "C" fn shim_nsview_convert_point_from_view(
    obj: u64,
    _cmd: u64,
    px: f64,
    py: f64,
    from_view: u64,
) -> CGPoint {
    if obj == 0 {
        return CGPoint { x: px, y: py };
    }

    // Convert from source to window coordinates.
    let (wx, wy) = if from_view != 0 {
        let (ox, oy) = view_origin_in_window(from_view);
        (px + ox, py + oy)
    } else {
        (px, py)
    };

    // Convert from window coordinates to destination (self).
    let (dx, dy) = view_origin_in_window(obj);
    CGPoint {
        x: wx - dx,
        y: wy - dy,
    }
}

/// `-[NSView convertPoint:toView:]`
///
/// Converts a point from this view's coordinate system to `to_view`.
/// If `to_view` is nil (0), the point is converted to window coordinates.
pub extern "C" fn shim_nsview_convert_point_to_view(
    obj: u64,
    _cmd: u64,
    px: f64,
    py: f64,
    to_view: u64,
) -> CGPoint {
    if obj == 0 {
        return CGPoint { x: px, y: py };
    }

    // Convert from self to window coordinates.
    let (ox, oy) = view_origin_in_window(obj);
    let wx = px + ox;
    let wy = py + oy;

    // Convert from window to destination view.
    if to_view != 0 {
        let (dx, dy) = view_origin_in_window(to_view);
        CGPoint {
            x: wx - dx,
            y: wy - dy,
        }
    } else {
        CGPoint { x: wx, y: wy }
    }
}

/// `-[NSView convertRect:fromView:]`
pub extern "C" fn shim_nsview_convert_rect_from_view(
    obj: u64,
    _cmd: u64,
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
    from_view: u64,
) -> CGRect {
    let origin = shim_nsview_convert_point_from_view(obj, 0, rx, ry, from_view);
    CGRect::new(origin.x, origin.y, rw, rh)
}

/// `-[NSView convertRect:toView:]`
pub extern "C" fn shim_nsview_convert_rect_to_view(
    obj: u64,
    _cmd: u64,
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
    to_view: u64,
) -> CGRect {
    let origin = shim_nsview_convert_point_to_view(obj, 0, rx, ry, to_view);
    CGRect::new(origin.x, origin.y, rw, rh)
}

/// `-[NSView acceptsFirstResponder]` — base NSView returns NO.
pub extern "C" fn shim_nsview_accepts_first_responder(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[NSView mouseDown:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_mouse_down(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_mouse_down(obj, 0, event);
}

/// `-[NSView mouseUp:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_mouse_up(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_mouse_up(obj, 0, event);
}

/// `-[NSView mouseDragged:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_mouse_dragged(obj, 0, event);
}

/// `-[NSView rightMouseDown:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_right_mouse_down(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_right_mouse_down(obj, 0, event);
}

/// `-[NSView rightMouseUp:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_right_mouse_up(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_right_mouse_up(obj, 0, event);
}

/// `-[NSView rightMouseDragged:]` — forwards up the responder chain.
pub extern "C" fn shim_nsview_right_mouse_dragged(obj: u64, _cmd: u64, event: u64) {
    crate::nsresponder::shim_nsresponder_right_mouse_dragged(obj, 0, event);
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

// ── Intrinsic content size ──────────────────────────────────────

/// `NSViewNoIntrinsicMetric` — sentinel meaning "no intrinsic size on this axis".
pub const NO_INTRINSIC_METRIC: f64 = -1.0;

/// `-[NSView intrinsicContentSize]` — base implementation returns {-1, -1}.
/// Returns width in x0, height in x1 (two f64s).
pub extern "C" fn shim_nsview_intrinsic_content_size_w(_obj: u64, _cmd: u64) -> f64 {
    NO_INTRINSIC_METRIC
}

/// Second half of intrinsicContentSize (height).
pub extern "C" fn shim_nsview_intrinsic_content_size_h(_obj: u64, _cmd: u64) -> f64 {
    NO_INTRINSIC_METRIC
}

/// `-[NSView invalidateIntrinsicContentSize]` — marks layout as needing update.
pub extern "C" fn shim_nsview_invalidate_intrinsic_content_size(obj: u64, _cmd: u64) {
    crate::layout_solver::set_needs_layout(obj, true);
}

/// `-[NSView contentHuggingPriorityForOrientation:]`
/// Orientation 0 = horizontal, 1 = vertical. Default 250 (NSLayoutPriorityDefaultLow).
pub extern "C" fn shim_nsview_content_hugging_priority(
    obj: u64,
    _cmd: u64,
    orientation: u64,
) -> f64 {
    if obj == 0 {
        return 250.0;
    }
    view_layout_priorities(obj, |p| {
        if orientation == 1 {
            p.hugging_v
        } else {
            p.hugging_h
        }
    })
}

/// `-[NSView setContentHuggingPriority:forOrientation:]`
pub extern "C" fn shim_nsview_set_content_hugging_priority(
    obj: u64,
    _cmd: u64,
    priority: f64,
    orientation: u64,
) {
    if obj == 0 {
        return;
    }
    view_layout_priorities_mut(obj, |p| {
        if orientation == 1 {
            p.hugging_v = priority;
        } else {
            p.hugging_h = priority;
        }
    });
}

/// `-[NSView contentCompressionResistancePriorityForOrientation:]`
/// Default 750 (NSLayoutPriorityDefaultHigh).
pub extern "C" fn shim_nsview_content_compression_resistance_priority(
    obj: u64,
    _cmd: u64,
    orientation: u64,
) -> f64 {
    if obj == 0 {
        return 750.0;
    }
    view_layout_priorities(obj, |p| {
        if orientation == 1 {
            p.compression_v
        } else {
            p.compression_h
        }
    })
}

/// `-[NSView setContentCompressionResistancePriority:forOrientation:]`
pub extern "C" fn shim_nsview_set_content_compression_resistance_priority(
    obj: u64,
    _cmd: u64,
    priority: f64,
    orientation: u64,
) {
    if obj == 0 {
        return;
    }
    view_layout_priorities_mut(obj, |p| {
        if orientation == 1 {
            p.compression_v = priority;
        } else {
            p.compression_h = priority;
        }
    });
}

/// `-[NSView fittingSize]` — returns intrinsic content size (or 0,0 if no intrinsic).
pub extern "C" fn shim_nsview_fitting_size_w(obj: u64, _cmd: u64) -> f64 {
    let w = shim_nsview_intrinsic_content_size_w(obj, 0);
    if w < 0.0 {
        0.0
    } else {
        w
    }
}

/// Second half of fittingSize (height).
pub extern "C" fn shim_nsview_fitting_size_h(obj: u64, _cmd: u64) -> f64 {
    let h = shim_nsview_intrinsic_content_size_h(obj, 0);
    if h < 0.0 {
        0.0
    } else {
        h
    }
}

// ── Safe area / layout margins ──────────────────────────────────

/// Per-view cached layout guides: (safe_area_guide, layout_margins_guide).
static VIEW_GUIDES: Mutex<Option<HashMap<u64, (u64, u64)>>> = Mutex::new(None);

fn get_or_create_guides(view: u64) -> (u64, u64) {
    let mut map = VIEW_GUIDES.lock().unwrap();
    let map = map.get_or_insert_with(HashMap::new);
    if let Some(&pair) = map.get(&view) {
        return pair;
    }
    let safe = crate::nslayoutguide::make_layout_guide(view);
    let margins = crate::nslayoutguide::make_layout_guide(view);
    map.insert(view, (safe, margins));
    (safe, margins)
}

/// `-[NSView safeAreaLayoutGuide]`
///
/// On macOS there is no notch/status bar, so the safe area equals the view bounds.
pub extern "C" fn shim_nsview_safe_area_layout_guide(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    get_or_create_guides(obj).0
}

/// `-[NSView layoutMarginsGuide]`
///
/// Returns an NSLayoutGuide inset by `layoutMargins` (default 8pt on all sides).
pub extern "C" fn shim_nsview_layout_margins_guide(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    get_or_create_guides(obj).1
}

/// `-[NSView safeAreaInsets]` — returns (0,0,0,0) on macOS (no notch).
/// We return the top inset; companion shims below return the other edges.
pub extern "C" fn shim_nsview_safe_area_insets_top(_obj: u64, _cmd: u64) -> f64 {
    0.0
}
/// Safe area inset — left.
pub extern "C" fn shim_nsview_safe_area_insets_left(_obj: u64, _cmd: u64) -> f64 {
    0.0
}
/// Safe area inset — bottom.
pub extern "C" fn shim_nsview_safe_area_insets_bottom(_obj: u64, _cmd: u64) -> f64 {
    0.0
}
/// Safe area inset — right.
pub extern "C" fn shim_nsview_safe_area_insets_right(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// Default layout margin (8pt on each side, matching AppKit defaults).
const DEFAULT_LAYOUT_MARGIN: f64 = 8.0;

/// `-[NSView layoutMargins]` — returns default 8pt edges (top).
pub extern "C" fn shim_nsview_layout_margins_top(_obj: u64, _cmd: u64) -> f64 {
    DEFAULT_LAYOUT_MARGIN
}
/// Layout margins — left.
pub extern "C" fn shim_nsview_layout_margins_left(_obj: u64, _cmd: u64) -> f64 {
    DEFAULT_LAYOUT_MARGIN
}
/// Layout margins — bottom.
pub extern "C" fn shim_nsview_layout_margins_bottom(_obj: u64, _cmd: u64) -> f64 {
    DEFAULT_LAYOUT_MARGIN
}
/// Layout margins — right.
pub extern "C" fn shim_nsview_layout_margins_right(_obj: u64, _cmd: u64) -> f64 {
    DEFAULT_LAYOUT_MARGIN
}

/// `-[NSView addLayoutGuide:]`
pub extern "C" fn shim_nsview_add_layout_guide(obj: u64, _cmd: u64, guide: u64) {
    if obj == 0 || guide == 0 {
        return;
    }
    crate::nslayoutguide::shim_nslayoutguide_set_owning_view(guide, 0, obj);
}

/// `-[NSView removeLayoutGuide:]`
pub extern "C" fn shim_nsview_remove_layout_guide(_obj: u64, _cmd: u64, guide: u64) {
    if guide == 0 {
        return;
    }
    crate::nslayoutguide::shim_nslayoutguide_set_owning_view(guide, 0, 0);
}

// ── Display / drawing pipeline ─────────────────────────────────

/// `-[NSView display]` — trigger the full draw cycle for this view and its subviews.
///
/// 1. Obtains the window's CGContext (created by the bridge from the window surface).
/// 2. Pushes it as the current NSGraphicsContext.
/// 3. Calls `drawRect:` on this view via ObjC dispatch (so guest subclass overrides work).
/// 4. Recursively displays all subviews.
/// 5. Pops the graphics context.
pub extern "C" fn shim_nsview_display(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let w = read_f64(obj, FRAME_W_OFF);
    let h = read_f64(obj, FRAME_H_OFF);
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    // Find or create a per-window CGContext.
    let window = read_u64(obj, WINDOW_OFF);
    let cg_ctx = crate::nswindow::get_or_create_window_cg_context(window);
    if cg_ctx == 0 {
        return;
    }

    display_view_recursive(obj, cg_ctx);
}

/// Recursively display a view tree into the given CGContext.
pub(crate) fn display_view_recursive(view: u64, cg_ctx: u64) {
    if view == 0 || cg_ctx == 0 {
        return;
    }

    // Run the auto-layout pass before drawing, so constraint-solved frames
    // are applied to views before they paint.
    crate::layout_solver::layout_if_needed(view);

    let x = read_f64(view, FRAME_X_OFF);
    let y = read_f64(view, FRAME_Y_OFF);
    let w = read_f64(view, FRAME_W_OFF);
    let h = read_f64(view, FRAME_H_OFF);

    // Check the view's backing CALayer for visual properties.
    let layer = if read_u64(view, FLAGS_OFF) & FLAG_WANTS_LAYER != 0 {
        view_layer_or_create(view)
    } else {
        0
    };

    let props = crate::calayer::get_layer_props(layer);

    // If the layer is hidden, skip this view and all its children.
    if layer != 0 && props.hidden {
        return;
    }

    let needs_offscreen = layer != 0 && crate::layer_compositor::layer_needs_compositing(layer);

    if needs_offscreen {
        // Render this view into an offscreen CG context, then composite
        // the result onto the window surface with layer effects.
        let vw = w.max(1.0) as u32;
        let vh = h.max(1.0) as u32;
        let offscreen_ctx = cg_shims::bridge::context_create(vw, vh);

        // Draw layer backgroundColor into the offscreen buffer.
        let bg_color = crate::calayer::shim_calayer_background_color(layer, 0);
        if bg_color != 0 {
            cg_shims::bridge::with_context(offscreen_ctx, |c| {
                let color = cg_shims::bridge::color_get(bg_color);
                if let Some(col) = color {
                    c.surface
                        .clear(graphics_core::Color::new(col.r, col.g, col.b, col.a));
                }
            });
        }

        // Render layer contents (CGImage) into the offscreen buffer if set.
        if props.contents != 0 {
            cg_shims::bridge::with_context(offscreen_ctx, |c| {
                let sw = c.surface.width();
                let sh = c.surface.height();
                crate::layer_compositor::render_layer_contents(
                    c.surface.pixels_mut(),
                    sw,
                    sh,
                    0,
                    0,
                    vw,
                    vh,
                    props.contents,
                );
            });
        }

        // Push offscreen context so drawRect: renders into it.
        let prev_ctx = crate::nsgraphicscontext::push_current_context(offscreen_ctx);
        call_draw_rect(view, 0.0, 0.0, w, h);

        // Recursively display subviews into the offscreen buffer.
        for child in snapshot_subviews(view) {
            display_view_recursive(child, offscreen_ctx);
        }
        crate::nsgraphicscontext::pop_current_context(prev_ctx);

        // Extract the offscreen pixels and composite them onto the window surface.
        let offscreen_pixels: Option<Vec<u32>> =
            cg_shims::bridge::with_context(offscreen_ctx, |c| c.surface.pixels().to_vec());

        if let Some(src_pixels) = offscreen_pixels {
            // Get the window surface pixels and composite the offscreen result.
            cg_shims::bridge::with_context(cg_ctx, |c| {
                let dst_w = c.surface.width();
                let dst_h = c.surface.height();
                #[allow(clippy::cast_possible_truncation)]
                crate::layer_compositor::composite_layer(
                    &src_pixels,
                    vw,
                    vh,
                    c.surface.pixels_mut(),
                    dst_w,
                    dst_h,
                    x as i32,
                    y as i32,
                    &props,
                );
            });
        }

        cg_shims::bridge::context_release(offscreen_ctx);
    } else {
        // Standard path: paint directly into the shared CGContext (painter's algorithm).

        // Draw layer backgroundColor if present.
        if layer != 0 {
            let bg_color = crate::calayer::shim_calayer_background_color(layer, 0);
            if bg_color != 0 {
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    let dst_w = c.surface.width();
                    let dst_h = c.surface.height();
                    #[allow(clippy::cast_possible_truncation)]
                    crate::layer_compositor::draw_layer_background(
                        c.surface.pixels_mut(),
                        dst_w,
                        dst_h,
                        x as i32,
                        y as i32,
                        w.max(0.0) as u32,
                        h.max(0.0) as u32,
                        bg_color,
                        &props,
                    );
                });
            }

            // Render layer contents (CGImage) directly onto the window surface.
            if props.contents != 0 {
                cg_shims::bridge::with_context(cg_ctx, |c| {
                    let dst_w = c.surface.width();
                    let dst_h = c.surface.height();
                    #[allow(clippy::cast_possible_truncation)]
                    crate::layer_compositor::render_layer_contents(
                        c.surface.pixels_mut(),
                        dst_w,
                        dst_h,
                        x as i32,
                        y as i32,
                        w.max(0.0) as u32,
                        h.max(0.0) as u32,
                        props.contents,
                    );
                });
            }
        }

        // Push the CGContext as the current NSGraphicsContext for this view's draw cycle.
        let prev_ctx = crate::nsgraphicscontext::push_current_context(cg_ctx);

        // Call drawRect: via ObjC dispatch so guest subclass overrides are honored.
        call_draw_rect(view, x, y, w, h);

        // Recursively display subviews.
        for child in snapshot_subviews(view) {
            display_view_recursive(child, cg_ctx);
        }

        crate::nsgraphicscontext::pop_current_context(prev_ctx);
    }

    // Clear the needs-display flag.
    let mut flags = read_u64(view, FLAGS_OFF);
    flags &= !FLAG_NEEDS_DISPLAY;
    write_u64(view, FLAGS_OFF, flags);
}

/// Call `-[view drawRect:rect]` through ObjC dispatch so guest overrides work.
fn call_draw_rect(view: u64, x: f64, y: f64, w: f64, h: f64) {
    extern "C" {
        fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64;
    }

    // Get the interned selector pointer for "drawRect:".
    static DRAW_RECT_SEL: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let sel = *DRAW_RECT_SEL.get_or_init(|| {
        let cstr = std::ffi::CString::new("drawRect:").unwrap();
        let ptr = cstr.as_ptr() as u64;
        std::mem::forget(cstr);
        ptr
    });

    let imp = unsafe { objc_msg_lookup_imp(view, sel) };
    if imp == 0 {
        return;
    }

    // drawRect: takes (self, _cmd, CGRect{x,y,w,h}) — on ARM64 CGRect is passed
    // as 4 f64 args in d0-d3 (or x2-x5 on stack for the Apple variadic ABI).
    // Since our shim and guest code both use the same ABI, we call directly with 4 f64s.
    let f: extern "C" fn(u64, u64, f64, f64, f64, f64) = unsafe { std::mem::transmute(imp) };
    f(view, sel, x, y, w, h);
}

/// `-[NSView displayIfNeeded]` — display only if needsDisplay is set.
pub extern "C" fn shim_nsview_display_if_needed(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    if read_u64(obj, FLAGS_OFF) & FLAG_NEEDS_DISPLAY != 0 {
        shim_nsview_display(obj, _cmd);
    }
}

/// `-[NSView lockFocus]` — push a drawing context for this view.
pub extern "C" fn shim_nsview_lock_focus(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    let window = read_u64(obj, WINDOW_OFF);
    let cg_ctx = crate::nswindow::get_or_create_window_cg_context(window);
    if cg_ctx != 0 {
        crate::nsgraphicscontext::push_current_context(cg_ctx);
    }
}

/// `-[NSView unlockFocus]` — pop the drawing context.
pub extern "C" fn shim_nsview_unlock_focus(_obj: u64, _cmd: u64) {
    crate::nsgraphicscontext::pop_current_context(0);
}

// ── Autoresizing mask ───────────────────────────────────────────

/// `-[NSView autoresizingMask]` — returns the 6-bit mask (NSAutoresizingMaskOptions).
pub extern "C" fn shim_nsview_autoresizing_mask(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    (read_u64(obj, FLAGS_OFF) & AUTORESIZE_MASK_BITS) >> AUTORESIZE_SHIFT
}

/// `-[NSView setAutoresizingMask:]`
pub extern "C" fn shim_nsview_set_autoresizing_mask(obj: u64, _cmd: u64, mask: u64) {
    if obj == 0 {
        return;
    }
    let mut flags = read_u64(obj, FLAGS_OFF);
    flags &= !AUTORESIZE_MASK_BITS; // clear old mask
    flags |= (mask & 0x3F) << AUTORESIZE_SHIFT; // set new
    write_u64(obj, FLAGS_OFF, flags);
}

/// Apply autoresizing (struts-and-springs) for all subviews of `parent`
/// after the parent's frame changed from `old_w × old_h` to `new_w × new_h`.
///
/// Called from `resizeSubviewsWithOldSize:`.
pub(crate) fn apply_autoresizing(parent: u64, old_w: f64, old_h: f64) {
    if parent == 0 {
        return;
    }
    let new_w = read_f64(parent, FRAME_W_OFF);
    let new_h = read_f64(parent, FRAME_H_OFF);
    let dw = new_w - old_w;
    let dh = new_h - old_h;
    if dw == 0.0 && dh == 0.0 {
        return;
    }

    for child in snapshot_subviews(parent) {
        let mask = shim_nsview_autoresizing_mask(child, 0);
        if mask == 0 {
            continue;
        }

        let cx = read_f64(child, FRAME_X_OFF);
        let cy = read_f64(child, FRAME_Y_OFF);
        let cw = read_f64(child, FRAME_W_OFF);
        let ch = read_f64(child, FRAME_H_OFF);

        // Horizontal: MinXMargin(1), WidthSizable(2), MaxXMargin(4)
        let flex_left = mask & 1 != 0;
        let flex_width = mask & 2 != 0;
        let flex_right = mask & 4 != 0;
        let h_flex_count = u32::from(flex_left) + u32::from(flex_width) + u32::from(flex_right);
        let (nx, nw) = if h_flex_count == 0 {
            (cx, cw)
        } else {
            let per = dw / h_flex_count as f64;
            let dx = if flex_left { per } else { 0.0 };
            let dwidth = if flex_width { per } else { 0.0 };
            (cx + dx, (cw + dwidth).max(0.0))
        };

        // Vertical: MinYMargin(8), HeightSizable(16), MaxYMargin(32)
        let flex_bottom = mask & 8 != 0;
        let flex_height = mask & 16 != 0;
        let flex_top = mask & 32 != 0;
        let v_flex_count = u32::from(flex_bottom) + u32::from(flex_height) + u32::from(flex_top);
        let (ny, nh) = if v_flex_count == 0 {
            (cy, ch)
        } else {
            let per = dh / v_flex_count as f64;
            let dy = if flex_bottom { per } else { 0.0 };
            let dheight = if flex_height { per } else { 0.0 };
            (cy + dy, (ch + dheight).max(0.0))
        };

        write_frame(child, nx, ny, nw, nh);
    }
}

/// `-[NSView resizeSubviewsWithOldSize:]`
pub extern "C" fn shim_nsview_resize_subviews_with_old_size(
    obj: u64,
    _cmd: u64,
    old_w: f64,
    old_h: f64,
) {
    apply_autoresizing(obj, old_w, old_h);
}

// ── nextKeyView / previousKeyView ───────────────────────────────

/// Per-view next key view storage.
static NEXT_KEY_VIEW: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

fn with_key_view_map<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<u64, u64>) -> R,
{
    let mut guard = NEXT_KEY_VIEW.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

/// `-[NSView nextKeyView]`
pub extern "C" fn shim_nsview_next_key_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_key_view_map(|m| m.get(&obj).copied().unwrap_or(0))
}

/// `-[NSView setNextKeyView:]`
pub extern "C" fn shim_nsview_set_next_key_view(obj: u64, _cmd: u64, next: u64) {
    if obj == 0 {
        return;
    }
    with_key_view_map(|m| {
        if next == 0 {
            m.remove(&obj);
        } else {
            m.insert(obj, next);
        }
    });
}

/// `-[NSView previousKeyView]` — walks all views looking for one whose nextKeyView == self.
pub extern "C" fn shim_nsview_previous_key_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    with_key_view_map(|m| {
        for (&view, &next) in m.iter() {
            if next == obj {
                return view;
            }
        }
        0
    })
}

/// `-[NSView nextValidKeyView]` — walks the nextKeyView chain to find one that accepts
/// first responder (any non-nil nextKeyView is assumed valid for now).
pub extern "C" fn shim_nsview_next_valid_key_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let cursor = shim_nsview_next_key_view(obj, 0);
    let start = obj;
    while cursor != 0 && cursor != start {
        // For now, any view in the chain is considered valid.
        return cursor;
    }
    0
}

/// `-[NSView previousValidKeyView]` — walks backwards through the key-view chain.
pub extern "C" fn shim_nsview_previous_valid_key_view(obj: u64, _cmd: u64) -> u64 {
    shim_nsview_previous_key_view(obj, _cmd)
}

/// Reset key-view state (called from `reset_runtime_state_for_reuse`).
pub(crate) fn reset_key_view_state() {
    let mut guard = NEXT_KEY_VIEW.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
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

    fn make_coder() -> u64 {
        crate::nskeyedarchiver::shim_nskeyedarchiver_alloc(0, 0)
    }

    fn encode_object(coder: u64, key: &str, value: u64) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_object_for_key(
            coder,
            0,
            value,
            make_string(key),
        );
    }

    fn encode_integer(coder: u64, key: &str, value: i64) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_integer_for_key(
            coder,
            0,
            value,
            make_string(key),
        );
    }

    fn encode_bool(coder: u64, key: &str, value: bool) {
        crate::nskeyedarchiver::shim_nskeyedarchiver_encode_bool_for_key(
            coder,
            0,
            u8::from(value),
            make_string(key),
        );
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
    fn init_with_coder_decodes_frame_subviews_and_flags() {
        let obj = make_view();
        let child = alloc_view_with_frame(1.0, 2.0, 30.0, 40.0);
        let subviews = crate::nsarray::make_nsarray(&[child]);
        let coder = make_coder();

        encode_object(coder, "NSFrame", make_string("{{10, 20}, {300, 400}}"));
        encode_object(coder, "NSSubviews", subviews);
        encode_integer(coder, "NSAutoresizingMask", 18);
        encode_bool(coder, "NSWantsLayer", true);

        assert_eq!(shim_nsview_init_with_coder(obj, 0, coder), obj);
        assert_eq!(read_f64(obj, FRAME_X_OFF), 10.0);
        assert_eq!(read_f64(obj, FRAME_Y_OFF), 20.0);
        assert_eq!(read_f64(obj, FRAME_W_OFF), 300.0);
        assert_eq!(read_f64(obj, FRAME_H_OFF), 400.0);
        assert_eq!(read_field(obj, SUBVIEW_COUNT_OFF), 1);
        assert_eq!(child_at(obj, 0), child);
        assert_eq!(read_field(child, SUPERVIEW_OFF), obj);
        assert_eq!(shim_nsview_autoresizing_mask(obj, 0), 18);
        assert_eq!(shim_nsview_wants_layer(obj, 0), 1);

        free_obj(child);
        free_obj(obj);
    }

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
        assert_eq!(
            crate::nsarray::shim_nsarray_object_at_index(subviews, 0, 0),
            sub
        );
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
        assert_eq!(shim_nsview_frame(0, 0), CGRect::ZERO);
    }

    #[test]
    fn frame_returns_x() {
        let obj = shim_nsview_init_with_frame(0, 0, 42.5, 0.0, 0.0, 0.0);
        assert_eq!(shim_nsview_frame(obj, 0).origin.x, 42.5);
        free_obj(obj);
    }

    #[test]
    fn frame_manual_layout() {
        let obj = make_view();
        write_f64(obj, FRAME_X_OFF, 99.0);
        assert_eq!(shim_nsview_frame(obj, 0).origin.x, 99.0);
        free_obj(obj);
    }

    // ── shim_nsview_bounds ───────────────────────────────────────

    #[test]
    fn bounds_nil() {
        assert_eq!(shim_nsview_bounds(0, 0), CGRect::ZERO);
    }

    #[test]
    fn bounds_matches_frame_size() {
        let obj = shim_nsview_init_with_frame(0, 0, 7.5, 3.0, 100.0, 200.0);
        let bounds = shim_nsview_bounds(obj, 0);
        let frame = shim_nsview_frame(obj, 0);
        assert_eq!(bounds.origin.x, 0.0);
        assert_eq!(bounds.origin.y, 0.0);
        assert_eq!(bounds.size.width, frame.size.width);
        assert_eq!(bounds.size.height, frame.size.height);
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
    fn layer_returns_calayer_when_wants_layer() {
        let obj = make_view();
        // Without wantsLayer, layer returns nil
        assert_eq!(shim_nsview_layer(obj, 0), 0);
        // With wantsLayer, layer returns a valid CALayer
        shim_nsview_set_wants_layer(obj, 0, 1);
        let layer = shim_nsview_layer(obj, 0);
        assert_ne!(layer, 0);
        // Calling again returns the same layer (cached)
        assert_eq!(shim_nsview_layer(obj, 0), layer);
        free_obj(obj);
    }

    // ── intrinsic content size ─────────────────────────────────

    #[test]
    fn intrinsic_content_size_default_no_metric() {
        let view = make_view();
        assert!(
            (shim_nsview_intrinsic_content_size_w(view, 0) - NO_INTRINSIC_METRIC).abs()
                < f64::EPSILON
        );
        assert!(
            (shim_nsview_intrinsic_content_size_h(view, 0) - NO_INTRINSIC_METRIC).abs()
                < f64::EPSILON
        );
        free_obj(view);
    }

    #[test]
    fn content_hugging_default_low() {
        let view = make_view();
        assert!((shim_nsview_content_hugging_priority(view, 0, 0) - 250.0).abs() < f64::EPSILON);
        assert!((shim_nsview_content_hugging_priority(view, 0, 1) - 250.0).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn compression_resistance_default_high() {
        let view = make_view();
        assert!(
            (shim_nsview_content_compression_resistance_priority(view, 0, 0) - 750.0).abs()
                < f64::EPSILON
        );
        free_obj(view);
    }

    #[test]
    fn fitting_size_zero_when_no_intrinsic() {
        let view = make_view();
        assert!((shim_nsview_fitting_size_w(view, 0)).abs() < f64::EPSILON);
        assert!((shim_nsview_fitting_size_h(view, 0)).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn invalidate_intrinsic_marks_needs_layout() {
        let view = make_view();
        assert_eq!(shim_nsview_needs_layout(view, 0), 0);
        shim_nsview_invalidate_intrinsic_content_size(view, 0);
        assert_eq!(shim_nsview_needs_layout(view, 0), 1);
        free_obj(view);
    }

    #[test]
    fn intrinsic_nil_safety() {
        assert!(
            (shim_nsview_intrinsic_content_size_w(0, 0) - NO_INTRINSIC_METRIC).abs() < f64::EPSILON
        );
        shim_nsview_invalidate_intrinsic_content_size(0, 0);
        shim_nsview_set_content_hugging_priority(0, 0, 500.0, 0);
        shim_nsview_set_content_compression_resistance_priority(0, 0, 500.0, 0);
    }

    #[test]
    fn set_content_hugging_priority_roundtrip() {
        reset_layout_priority_state();
        let view = make_view();
        shim_nsview_set_content_hugging_priority(view, 0, 500.0, 0); // horizontal
        shim_nsview_set_content_hugging_priority(view, 0, 600.0, 1); // vertical
        assert!((shim_nsview_content_hugging_priority(view, 0, 0) - 500.0).abs() < f64::EPSILON);
        assert!((shim_nsview_content_hugging_priority(view, 0, 1) - 600.0).abs() < f64::EPSILON);
        free_obj(view);
        reset_layout_priority_state();
    }

    #[test]
    fn set_compression_resistance_roundtrip() {
        reset_layout_priority_state();
        let view = make_view();
        shim_nsview_set_content_compression_resistance_priority(view, 0, 900.0, 0);
        shim_nsview_set_content_compression_resistance_priority(view, 0, 800.0, 1);
        assert!(
            (shim_nsview_content_compression_resistance_priority(view, 0, 0) - 900.0).abs()
                < f64::EPSILON
        );
        assert!(
            (shim_nsview_content_compression_resistance_priority(view, 0, 1) - 800.0).abs()
                < f64::EPSILON
        );
        free_obj(view);
        reset_layout_priority_state();
    }

    #[test]
    fn priorities_independent_per_view() {
        reset_layout_priority_state();
        let v1 = make_view();
        let v2 = make_view();
        shim_nsview_set_content_hugging_priority(v1, 0, 400.0, 0);
        shim_nsview_set_content_hugging_priority(v2, 0, 700.0, 0);
        assert!((shim_nsview_content_hugging_priority(v1, 0, 0) - 400.0).abs() < f64::EPSILON);
        assert!((shim_nsview_content_hugging_priority(v2, 0, 0) - 700.0).abs() < f64::EPSILON);
        free_obj(v1);
        free_obj(v2);
        reset_layout_priority_state();
    }

    // ── safe area / layout margins ─────────────────────────────

    #[test]
    fn safe_area_layout_guide_returns_nonzero() {
        let view = make_view();
        let guide = shim_nsview_safe_area_layout_guide(view, 0);
        assert_ne!(guide, 0);
        // Should be stable (cached)
        let guide2 = shim_nsview_safe_area_layout_guide(view, 0);
        assert_eq!(guide, guide2);
        free_obj(view);
    }

    #[test]
    fn layout_margins_guide_returns_nonzero() {
        let view = make_view();
        let guide = shim_nsview_layout_margins_guide(view, 0);
        assert_ne!(guide, 0);
        let guide2 = shim_nsview_layout_margins_guide(view, 0);
        assert_eq!(guide, guide2);
        free_obj(view);
    }

    #[test]
    fn safe_area_and_margins_guide_differ() {
        let view = make_view();
        let sa = shim_nsview_safe_area_layout_guide(view, 0);
        let lm = shim_nsview_layout_margins_guide(view, 0);
        assert_ne!(sa, lm);
        free_obj(view);
    }

    #[test]
    fn safe_area_nil_returns_zero() {
        assert_eq!(shim_nsview_safe_area_layout_guide(0, 0), 0);
        assert_eq!(shim_nsview_layout_margins_guide(0, 0), 0);
    }

    #[test]
    fn safe_area_insets_all_zero() {
        let view = make_view();
        assert_eq!(shim_nsview_safe_area_insets_top(view, 0), 0.0);
        assert_eq!(shim_nsview_safe_area_insets_left(view, 0), 0.0);
        assert_eq!(shim_nsview_safe_area_insets_bottom(view, 0), 0.0);
        assert_eq!(shim_nsview_safe_area_insets_right(view, 0), 0.0);
        free_obj(view);
    }

    #[test]
    fn layout_margins_default_8pt() {
        let view = make_view();
        assert_eq!(shim_nsview_layout_margins_top(view, 0), 8.0);
        assert_eq!(shim_nsview_layout_margins_left(view, 0), 8.0);
        assert_eq!(shim_nsview_layout_margins_bottom(view, 0), 8.0);
        assert_eq!(shim_nsview_layout_margins_right(view, 0), 8.0);
        free_obj(view);
    }

    #[test]
    fn add_layout_guide_sets_owning_view() {
        let view = make_view();
        let guide = crate::nslayoutguide::make_layout_guide(0);
        shim_nsview_add_layout_guide(view, 0, guide);
        assert_eq!(
            crate::nslayoutguide::shim_nslayoutguide_owning_view(guide, 0),
            view
        );
        free_obj(guide);
        free_obj(view);
    }

    #[test]
    fn remove_layout_guide_clears_owning_view() {
        let view = make_view();
        let guide = crate::nslayoutguide::make_layout_guide(view);
        shim_nsview_remove_layout_guide(view, 0, guide);
        assert_eq!(
            crate::nslayoutguide::shim_nslayoutguide_owning_view(guide, 0),
            0
        );
        free_obj(guide);
        free_obj(view);
    }

    #[test]
    fn add_remove_layout_guide_nil_safety() {
        shim_nsview_add_layout_guide(0, 0, 0);
        shim_nsview_remove_layout_guide(0, 0, 0);
    }

    // ── convertPoint:fromView: / convertPoint:toView: ────────────

    #[test]
    fn convert_point_from_view_nil_from_is_window_coords() {
        // from_view == 0 means point is in window coordinates.
        // View at (10, 20) in window: window point (15, 25) → view local (5, 5).
        let view = alloc_view_with_frame(10.0, 20.0, 100.0, 80.0);
        let pt = shim_nsview_convert_point_from_view(view, 0, 15.0, 25.0, 0);
        assert!((pt.x - 5.0).abs() < f64::EPSILON);
        assert!((pt.y - 5.0).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn convert_point_to_view_nil_to_is_window_coords() {
        // to_view == 0 means convert to window coordinates.
        // View at (10, 20): local point (5, 5) → window (15, 25).
        let view = alloc_view_with_frame(10.0, 20.0, 100.0, 80.0);
        let pt = shim_nsview_convert_point_to_view(view, 0, 5.0, 5.0, 0);
        assert!((pt.x - 15.0).abs() < f64::EPSILON);
        assert!((pt.y - 25.0).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn convert_point_round_trip_to_window_and_back() {
        let view = alloc_view_with_frame(30.0, 40.0, 200.0, 150.0);
        let local = CGPoint { x: 7.0, y: 3.0 };
        // local → window
        let window_pt = shim_nsview_convert_point_to_view(view, 0, local.x, local.y, 0);
        // window → local
        let back = shim_nsview_convert_point_from_view(view, 0, window_pt.x, window_pt.y, 0);
        assert!((back.x - local.x).abs() < f64::EPSILON);
        assert!((back.y - local.y).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn convert_point_between_nested_views() {
        // Parent at (10, 20), child at (5, 10) within parent.
        let parent = alloc_view_with_frame(10.0, 20.0, 200.0, 200.0);
        let child = alloc_view_with_frame(5.0, 10.0, 50.0, 50.0);
        shim_nsview_add_subview(parent, 0, child);

        // Point (0, 0) in child → window: child origin = (10+5, 20+10) = (15, 30)
        let to_win = shim_nsview_convert_point_to_view(child, 0, 0.0, 0.0, 0);
        assert!((to_win.x - 15.0).abs() < f64::EPSILON);
        assert!((to_win.y - 30.0).abs() < f64::EPSILON);

        // Convert point from parent coords to child coords:
        // Parent local (5, 10) is child origin → child local (0, 0)
        let in_child = shim_nsview_convert_point_from_view(child, 0, 5.0, 10.0, parent);
        assert!((in_child.x).abs() < f64::EPSILON);
        assert!((in_child.y).abs() < f64::EPSILON);

        // Convert point from child to parent:
        // Child local (3, 4) → parent local: child is at (5,10) in parent, so (5+3, 10+4) = (8, 14)
        let in_parent = shim_nsview_convert_point_to_view(child, 0, 3.0, 4.0, parent);
        assert!((in_parent.x - 8.0).abs() < f64::EPSILON);
        assert!((in_parent.y - 14.0).abs() < f64::EPSILON);

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn convert_rect_from_view_preserves_size() {
        let view = alloc_view_with_frame(10.0, 20.0, 100.0, 80.0);
        let rect = shim_nsview_convert_rect_from_view(view, 0, 15.0, 25.0, 50.0, 30.0, 0);
        // Origin should be converted: (15-10, 25-20) = (5, 5)
        assert!((rect.origin.x - 5.0).abs() < f64::EPSILON);
        assert!((rect.origin.y - 5.0).abs() < f64::EPSILON);
        // Size must be preserved.
        assert!((rect.size.width - 50.0).abs() < f64::EPSILON);
        assert!((rect.size.height - 30.0).abs() < f64::EPSILON);
        free_obj(view);
    }

    #[test]
    fn convert_point_nil_obj_returns_same_point() {
        let pt = shim_nsview_convert_point_from_view(0, 0, 42.0, 99.0, 0);
        assert!((pt.x - 42.0).abs() < f64::EPSILON);
        assert!((pt.y - 99.0).abs() < f64::EPSILON);

        let pt2 = shim_nsview_convert_point_to_view(0, 0, 42.0, 99.0, 0);
        assert!((pt2.x - 42.0).abs() < f64::EPSILON);
        assert!((pt2.y - 99.0).abs() < f64::EPSILON);
    }
}
