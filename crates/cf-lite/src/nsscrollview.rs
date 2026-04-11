//! NSScrollView shim: scrollable container for AppKit views.
//!
//! Object layout (96 bytes):
//!
//!   [0..72):  inherited from NSView (ISA at [0..8))
//!   [72..80): document_view_ptr  (u64 — the content view being scrolled)
//!   [80..88): content_size_width (f64 — cached content size)
//!   [88..96): flags              (u64 — bit 0: has_vertical_scroller,
//!                                       bit 1: has_horizontal_scroller,
//!                                       bit 2: autohides_scrollers,
//!                                       bit 3: draws_background)

const ISA_OFF: usize = 0;
const DOC_VIEW_OFF: usize = 72;
const CONTENT_WIDTH_OFF: usize = 80;
const FLAGS_OFF: usize = 88;
pub(crate) const SCROLL_VIEW_SIZE: usize = 96;

const FLAG_HAS_VERTICAL_SCROLLER: u64 = 1 << 0;
const FLAG_HAS_HORIZONTAL_SCROLLER: u64 = 1 << 1;
const FLAG_AUTOHIDES_SCROLLERS: u64 = 1 << 2;
const FLAG_DRAWS_BACKGROUND: u64 = 1 << 3;

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

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_u64(obj, FLAGS_OFF) & flag != 0)
}

fn set_flag(obj: u64, flag: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    if val != 0 {
        write_u64(obj, FLAGS_OFF, flags | flag);
    } else {
        write_u64(obj, FLAGS_OFF, flags & !flag);
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSScrollView documentView]`
pub extern "C" fn shim_nsscrollview_document_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, DOC_VIEW_OFF)
}

/// `-[NSScrollView setDocumentView:]`
pub extern "C" fn shim_nsscrollview_set_document_view(obj: u64, _cmd: u64, view: u64) {
    write_u64(obj, DOC_VIEW_OFF, view);
    // Also add as subview so the display pipeline picks it up.
    if obj != 0 && view != 0 {
        crate::nsview::shim_nsview_add_subview(obj, 0, view);
    }
}

/// `-[NSScrollView hasVerticalScroller]`
pub extern "C" fn shim_nsscrollview_has_vertical_scroller(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_HAS_VERTICAL_SCROLLER)
}

/// `-[NSScrollView setHasVerticalScroller:]`
pub extern "C" fn shim_nsscrollview_set_has_vertical_scroller(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_HAS_VERTICAL_SCROLLER, val);
}

/// `-[NSScrollView hasHorizontalScroller]`
pub extern "C" fn shim_nsscrollview_has_horizontal_scroller(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_HAS_HORIZONTAL_SCROLLER)
}

/// `-[NSScrollView setHasHorizontalScroller:]`
pub extern "C" fn shim_nsscrollview_set_has_horizontal_scroller(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_HAS_HORIZONTAL_SCROLLER, val);
}

/// `-[NSScrollView autohidesScrollers]`
pub extern "C" fn shim_nsscrollview_autohides_scrollers(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_AUTOHIDES_SCROLLERS)
}

/// `-[NSScrollView setAutohidesScrollers:]`
pub extern "C" fn shim_nsscrollview_set_autohides_scrollers(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_AUTOHIDES_SCROLLERS, val);
}

/// `-[NSScrollView drawsBackground]`
pub extern "C" fn shim_nsscrollview_draws_background(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_DRAWS_BACKGROUND)
}

/// `-[NSScrollView setDrawsBackground:]`
pub extern "C" fn shim_nsscrollview_set_draws_background(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_DRAWS_BACKGROUND, val);
}

/// `-[NSScrollView contentSize]` — returns available content area width as u64.
///
/// Real AppKit returns CGSize (struct), but struct return is not yet supported.
/// We compute frame width minus scroller width as a best-effort scalar.
pub extern "C" fn shim_nsscrollview_content_size(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let frame = crate::nsview::view_frame(obj);
    let flags = read_u64(obj, FLAGS_OFF);
    let mut w = frame.size.width;
    let mut _h = frame.size.height;
    // Subtract scroller track widths (15px each, matching AppKit default).
    if flags & FLAG_HAS_VERTICAL_SCROLLER != 0 {
        w -= 15.0;
    }
    if flags & FLAG_HAS_HORIZONTAL_SCROLLER != 0 {
        _h -= 15.0;
    }
    if w < 0.0 { w = 0.0; }
    if _h < 0.0 { _h = 0.0; }
    // Pack width as integer (callers needing full CGSize should use KVC).
    w as u64
}

/// `-[NSScrollView tile]` — no-op stub.
pub extern "C" fn shim_nsscrollview_tile(_obj: u64, _cmd: u64) {}

/// `-[NSScrollView contentView]` — returns the document view as the clip content.
pub extern "C" fn shim_nsscrollview_content_view(obj: u64, _cmd: u64) -> u64 {
    // In a real implementation this returns NSClipView; we return documentView.
    read_u64(obj, DOC_VIEW_OFF)
}

/// `-[NSScrollView scrollWheel:]` — handles scroll events by adjusting the
/// document view's frame origin (simulating a scroll offset).
///
/// The scroll delta comes from NSEvent's scrollingDeltaX/Y.
pub extern "C" fn shim_nsscrollview_scroll_wheel(obj: u64, _cmd: u64, event: u64) {
    if obj == 0 || event == 0 { return; }

    let (dx, dy) = crate::nsevent::event_scroll_deltas(event);
    if dx == 0.0 && dy == 0.0 { return; }

    let doc = read_u64(obj, DOC_VIEW_OFF);
    if doc == 0 { return; }

    // Read the document view's current frame origin (scroll offset is negative origin).
    let cur_x = crate::nsview::view_frame_x(doc);
    let cur_y = crate::nsview::view_frame_y(doc);

    // Apply delta — positive dy scrolls down (moves content up → more negative y origin).
    let new_x = cur_x + dx;
    let new_y = cur_y + dy;

    crate::nsview::shim_nsview_set_frame_origin(doc, 0, new_x, new_y);
    crate::nsview::shim_nsview_set_needs_display(doc, 0, 1);
}

/// `-[NSScrollView drawRect:]` — draws background and lets subviews render.
pub extern "C" fn shim_nsscrollview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 { return; }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 { return; }

    let flags = read_u64(obj, FLAGS_OFF);
    if flags & FLAG_DRAWS_BACKGROUND != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| {
            c.fill_color = graphics_core::Color::WHITE;
            c.fill_rect(x, y, w, h);
        });
    }

    // Draw scrollbar indicators if scrollers are visible.
    if flags & FLAG_HAS_VERTICAL_SCROLLER != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| {
            let scroller_x = x + w - 8.0;
            c.fill_color = graphics_core::Color::new(0.8, 0.8, 0.8, 0.5);
            c.fill_rect(scroller_x, y, 8.0, h);
            // Thumb.
            c.fill_color = graphics_core::Color::new(0.5, 0.5, 0.5, 0.7);
            c.fill_rect(scroller_x + 1.0, y + 4.0, 6.0, h * 0.3);
        });
    }
    if flags & FLAG_HAS_HORIZONTAL_SCROLLER != 0 {
        cg_shims::bridge::with_context(cg_ctx, |c| {
            let scroller_y = y + h - 8.0;
            c.fill_color = graphics_core::Color::new(0.8, 0.8, 0.8, 0.5);
            c.fill_rect(x, scroller_y, w, 8.0);
            // Thumb.
            c.fill_color = graphics_core::Color::new(0.5, 0.5, 0.5, 0.7);
            c.fill_rect(x + 4.0, scroller_y + 1.0, w * 0.3, 6.0);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsscrollview_document_view(0, 0), 0);
        shim_nsscrollview_set_document_view(0, 0, 0x1234);
        assert_eq!(shim_nsscrollview_has_vertical_scroller(0, 0), 0);
        shim_nsscrollview_set_has_vertical_scroller(0, 0, 1);
        assert_eq!(shim_nsscrollview_has_horizontal_scroller(0, 0), 0);
        shim_nsscrollview_set_has_horizontal_scroller(0, 0, 1);
        assert_eq!(shim_nsscrollview_autohides_scrollers(0, 0), 0);
        shim_nsscrollview_set_autohides_scrollers(0, 0, 1);
        assert_eq!(shim_nsscrollview_draws_background(0, 0), 0);
        shim_nsscrollview_set_draws_background(0, 0, 1);
        assert_eq!(shim_nsscrollview_content_size(0, 0), 0);
        shim_nsscrollview_tile(0, 0);
    }

    #[test]
    fn document_view_round_trip() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nsscrollview_document_view(sv, 0), 0);
        // Use a properly-allocated view since setDocumentView now calls addSubview.
        let child = alloc_obj(crate::nsview::VIEW_SIZE, crate::nsobj::nsview_class_addr());
        shim_nsscrollview_set_document_view(sv, 0, child);
        assert_eq!(shim_nsscrollview_document_view(sv, 0), child);
        free_obj(child);
        free_obj(sv);
    }

    #[test]
    fn vertical_scroller_flag() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nsscrollview_has_vertical_scroller(sv, 0), 0);
        shim_nsscrollview_set_has_vertical_scroller(sv, 0, 1);
        assert_eq!(shim_nsscrollview_has_vertical_scroller(sv, 0), 1);
        shim_nsscrollview_set_has_vertical_scroller(sv, 0, 0);
        assert_eq!(shim_nsscrollview_has_vertical_scroller(sv, 0), 0);
        free_obj(sv);
    }

    #[test]
    fn multiple_flags_independent() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        shim_nsscrollview_set_has_vertical_scroller(sv, 0, 1);
        shim_nsscrollview_set_autohides_scrollers(sv, 0, 1);
        shim_nsscrollview_set_draws_background(sv, 0, 1);
        assert_eq!(shim_nsscrollview_has_vertical_scroller(sv, 0), 1);
        assert_eq!(shim_nsscrollview_has_horizontal_scroller(sv, 0), 0);
        assert_eq!(shim_nsscrollview_autohides_scrollers(sv, 0), 1);
        assert_eq!(shim_nsscrollview_draws_background(sv, 0), 1);
        free_obj(sv);
    }

    #[test]
    fn content_size_uses_frame() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        // Give the scroll view a frame so contentSize is non-zero.
        crate::nsview::shim_nsview_set_frame(sv, 0, 0.0, 0.0, 400.0, 300.0);
        assert_eq!(shim_nsscrollview_content_size(sv, 0), 400);
        // With vertical scroller: width shrinks by 15.
        shim_nsscrollview_set_has_vertical_scroller(sv, 0, 1);
        assert_eq!(shim_nsscrollview_content_size(sv, 0), 385);
        shim_nsscrollview_set_has_vertical_scroller(sv, 0, 0);
        assert_eq!(shim_nsscrollview_content_size(sv, 0), 400);
        shim_nsscrollview_tile(sv, 0); // should not panic
        free_obj(sv);
    }

    // ── shim_nsscrollview_scroll_wheel ──────────────────────────

    #[test]
    fn scroll_wheel_nil_obj_no_panic() {
        let evt = crate::nsevent::alloc_scroll_event(5.0, 10.0, 0, 0);
        shim_nsscrollview_scroll_wheel(0, 0, evt);
        free_obj(evt);
    }

    #[test]
    fn scroll_wheel_nil_event_no_panic() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        shim_nsscrollview_scroll_wheel(sv, 0, 0);
        free_obj(sv);
    }

    #[test]
    fn scroll_wheel_updates_document_view_origin() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, crate::nsobj::nsview_class_addr());
        let doc = alloc_obj(crate::nsview::VIEW_SIZE, crate::nsobj::nsview_class_addr());
        shim_nsscrollview_set_document_view(sv, 0, doc);

        // Initial origin should be (0, 0).
        assert!((crate::nsview::view_frame_x(doc)).abs() < f64::EPSILON);
        assert!((crate::nsview::view_frame_y(doc)).abs() < f64::EPSILON);

        // Scroll by (3.0, -5.0).
        let evt = crate::nsevent::alloc_scroll_event(3.0, -5.0, 0, 0);
        shim_nsscrollview_scroll_wheel(sv, 0, evt);
        free_obj(evt);

        assert!((crate::nsview::view_frame_x(doc) - 3.0).abs() < f64::EPSILON);
        assert!((crate::nsview::view_frame_y(doc) - (-5.0)).abs() < f64::EPSILON);

        // Scroll again to verify cumulative.
        let evt2 = crate::nsevent::alloc_scroll_event(1.0, 2.0, 0, 0);
        shim_nsscrollview_scroll_wheel(sv, 0, evt2);
        free_obj(evt2);

        assert!((crate::nsview::view_frame_x(doc) - 4.0).abs() < f64::EPSILON);
        assert!((crate::nsview::view_frame_y(doc) - (-3.0)).abs() < f64::EPSILON);

        free_obj(doc);
        free_obj(sv);
    }

    #[test]
    fn scroll_wheel_zero_delta_no_change() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, crate::nsobj::nsview_class_addr());
        let doc = alloc_obj(crate::nsview::VIEW_SIZE, crate::nsobj::nsview_class_addr());
        crate::nsview::shim_nsview_set_frame_origin(doc, 0, 10.0, 20.0);
        shim_nsscrollview_set_document_view(sv, 0, doc);

        let evt = crate::nsevent::alloc_scroll_event(0.0, 0.0, 0, 0);
        shim_nsscrollview_scroll_wheel(sv, 0, evt);
        free_obj(evt);

        // Origin should remain unchanged because delta was (0, 0).
        assert!((crate::nsview::view_frame_x(doc) - 10.0).abs() < f64::EPSILON);
        assert!((crate::nsview::view_frame_y(doc) - 20.0).abs() < f64::EPSILON);

        free_obj(doc);
        free_obj(sv);
    }
}
