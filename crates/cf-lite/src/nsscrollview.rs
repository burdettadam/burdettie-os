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

/// `-[NSScrollView contentSize]` — stub, returns 0 (CGSize would need struct return).
pub extern "C" fn shim_nsscrollview_content_size(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    0
}

/// `-[NSScrollView tile]` — no-op stub.
pub extern "C" fn shim_nsscrollview_tile(_obj: u64, _cmd: u64) {}

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
        let fake_view = 0x4000_u64;
        shim_nsscrollview_set_document_view(sv, 0, fake_view);
        assert_eq!(shim_nsscrollview_document_view(sv, 0), fake_view);
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
    fn content_size_and_tile_stubs() {
        let sv = alloc_obj(SCROLL_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nsscrollview_content_size(sv, 0), 0);
        shim_nsscrollview_tile(sv, 0); // should not panic
        free_obj(sv);
    }
}
