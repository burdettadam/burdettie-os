//! NSLayoutManager — manages glyph layout for a text storage.
//!
//! Wires `usedRectForTextContainer:` and `ensureLayoutForTextContainer:`
//! through to `graphics_core::text_system::measure_shaped_text` so callers
//! get real text extents.  Adds `glyphRangeForTextContainer:` and
//! `boundingRectForGlyphRange:inTextContainer:`.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  text_storage_ptr  (weak ref)
//!   [16..24): text_container_ptr (first container)

use crate::cg_types::CGRect;

const SIZE: usize = 24;
const TEXT_STORAGE_OFF: usize = 8;
const TEXT_CONTAINER_OFF: usize = 16;

/// Default font size used when no font attribute is stored on the text storage.
const DEFAULT_FONT_SIZE: f64 = 12.0;
const DEFAULT_FONT_NAME: &str = "System";

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut u64) = val;
    }
}

// ── Internal helpers ───────────────────────────────────────────

/// Extract the UTF-8 text from the NSTextStorage linked to this layout manager.
fn storage_text(obj: u64) -> Option<String> {
    let ts = read_u64(obj, TEXT_STORAGE_OFF);
    if ts == 0 {
        return None;
    }
    let ns_str = crate::nstextstorage::shim_nstextstorage_string(ts, 0);
    if ns_str == 0 {
        return None;
    }
    let cptr = crate::cfstring::shim_nsstring_utf8string(ns_str, 0);
    if cptr.is_null() {
        return None;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(cptr) };
    c_str.to_str().ok().map(|s| s.to_owned())
}

/// Get the container width from the text container pointer, falling back to
/// `None` (infinite width) if the container is nil.
fn container_width(tc: u64) -> Option<f64> {
    if tc == 0 {
        return None;
    }
    let w = crate::nstextcontainer::shim_nstextcontainer_size_width(tc, 0);
    if w >= 1.0e6 {
        None
    } else {
        Some(w)
    }
}

/// Measure the text in this layout manager's text storage, constrained to
/// the given text container's width.
fn measure(obj: u64, tc: u64) -> graphics_core::text_system::ShapedTextMetrics {
    let text = storage_text(obj).unwrap_or_default();
    let max_w = container_width(tc);
    graphics_core::text_system::measure_shaped_text(
        &text,
        DEFAULT_FONT_SIZE,
        DEFAULT_FONT_NAME,
        max_w,
    )
}

// ── ObjC shims ─────────────────────────────────────────────────

/// `+[NSLayoutManager alloc]`
pub extern "C" fn shim_nslayoutmanager_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nslayoutmanager_class_addr();
    }
    obj
}

/// `-[NSLayoutManager init]`
pub extern "C" fn shim_nslayoutmanager_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    obj
}

/// `-[NSLayoutManager textStorage]`
pub extern "C" fn shim_nslayoutmanager_text_storage(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_u64(obj, TEXT_STORAGE_OFF)
}

/// `-[NSLayoutManager setTextStorage:]`  (called by NSTextStorage -addLayoutManager:)
pub extern "C" fn shim_nslayoutmanager_set_text_storage(obj: u64, _cmd: u64, ts: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, TEXT_STORAGE_OFF, ts);
}

/// `-[NSLayoutManager addTextContainer:]` — stores container and sets its
/// `layoutManager` back-pointer.
pub extern "C" fn shim_nslayoutmanager_add_text_container(obj: u64, _cmd: u64, tc: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, TEXT_CONTAINER_OFF, tc);
    if tc != 0 {
        crate::nstextcontainer::shim_nstextcontainer_set_layout_manager(tc, 0, obj);
    }
}

/// `-[NSLayoutManager removeTextContainerAtIndex:]` → no-op
pub extern "C" fn shim_nslayoutmanager_remove_text_container(_obj: u64, _cmd: u64, _idx: u64) {}

/// `-[NSLayoutManager textContainers]` → returns first container (simplified).
pub extern "C" fn shim_nslayoutmanager_text_containers(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_u64(obj, TEXT_CONTAINER_OFF)
}

/// `-[NSLayoutManager numberOfGlyphs]` → returns text length (character count
/// is correct for most Latin text; real TextKit would shape first).
pub extern "C" fn shim_nslayoutmanager_number_of_glyphs(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let ts = read_u64(obj, TEXT_STORAGE_OFF);
    if ts == 0 {
        return 0;
    }
    crate::nstextstorage::shim_nstextstorage_length(ts, 0)
}

/// `-[NSLayoutManager usedRectForTextContainer:]` — measures text using
/// cosmic-text and returns a real CGRect with the text extents.
pub extern "C" fn shim_nslayoutmanager_used_rect(obj: u64, _cmd: u64, tc: u64) -> CGRect {
    if obj == 0 {
        return CGRect::ZERO;
    }
    let m = measure(obj, tc);
    CGRect::new(0.0, 0.0, m.width, m.height)
}

/// `-[NSLayoutManager ensureLayoutForTextContainer:]` — triggers measurement
/// so any cached layout is ready.  Our implementation is stateless, so this
/// is effectively a no-op, but we validate the pipeline.
pub extern "C" fn shim_nslayoutmanager_ensure_layout(obj: u64, _cmd: u64, tc: u64) {
    if obj == 0 {
        return;
    }
    let _ = measure(obj, tc);
}

/// `-[NSLayoutManager glyphRangeForTextContainer:]` — returns {0, numberOfGlyphs}
/// for the given container (i.e., all glyphs fit in the single container).
pub extern "C" fn shim_nslayoutmanager_glyph_range_for_tc(
    obj: u64,
    _cmd: u64,
    _tc: u64,
) -> core_foundation_lite::NSRange {
    if obj == 0 {
        return core_foundation_lite::NSRange {
            location: 0,
            length: 0,
        };
    }
    let n = shim_nslayoutmanager_number_of_glyphs(obj, 0);
    core_foundation_lite::NSRange {
        location: 0,
        length: n,
    }
}

/// `-[NSLayoutManager boundingRectForGlyphRange:inTextContainer:]` — returns
/// a CGRect enclosing the given glyph range.  We approximate by measuring
/// the full text and returning that rect (correct when the range is the
/// entire text, which is the common case).
pub extern "C" fn shim_nslayoutmanager_bounding_rect(
    obj: u64,
    _cmd: u64,
    _loc: u64,
    _len: u64,
    tc: u64,
) -> CGRect {
    if obj == 0 {
        return CGRect::ZERO;
    }
    let m = measure(obj, tc);
    CGRect::new(0.0, 0.0, m.width, m.height)
}

/// `-[NSLayoutManager lineFragmentRectForGlyphAtIndex:effectiveRange:]` —
/// returns the rect of the line containing the glyph at `idx`.
/// Simplified: returns the full used rect (single-line approximation).
pub extern "C" fn shim_nslayoutmanager_line_fragment_rect(
    obj: u64,
    _cmd: u64,
    _idx: u64,
    _effective_range: u64,
) -> CGRect {
    if obj == 0 {
        return CGRect::ZERO;
    }
    let tc = read_u64(obj, TEXT_CONTAINER_OFF);
    let m = measure(obj, tc);
    CGRect::new(0.0, 0.0, m.width, m.height)
}

/// `-[NSLayoutManager invalidateLayoutForCharacterRange:actualCharacterRange:]` → no-op
pub extern "C" fn shim_nslayoutmanager_invalidate_layout(
    _obj: u64,
    _cmd: u64,
    _loc: u64,
    _len: u64,
    _actual: u64,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── helpers ────────────────────────────────────────────────

    /// Create a wired TextKit stack: NSTextStorage + NSLayoutManager + NSTextContainer.
    fn make_textkit_stack(text: &str, container_w: f64, container_h: f64) -> (u64, u64, u64) {
        let ts = crate::nstextstorage::shim_nstextstorage_alloc(0, 0);
        let ts = crate::nstextstorage::shim_nstextstorage_init(ts, 0);
        let s = make_string(text);
        crate::nstextstorage::shim_nstextstorage_set_string(ts, 0, s);

        let lm = shim_nslayoutmanager_alloc(0, 0);
        let lm = shim_nslayoutmanager_init(lm, 0);

        let tc = crate::nstextcontainer::shim_nstextcontainer_alloc(0, 0);
        let tc = crate::nstextcontainer::shim_nstextcontainer_init_with_size(
            tc,
            0,
            container_w,
            container_h,
        );

        // Wire: storage → lm, lm → container
        crate::nstextstorage::shim_nstextstorage_add_layout_manager(ts, 0, lm);
        shim_nslayoutmanager_add_text_container(lm, 0, tc);

        (ts, lm, tc)
    }

    fn free_textkit(ts: u64, lm: u64, tc: u64) {
        free_obj(tc);
        free_obj(lm);
        free_obj(ts);
    }

    // ── basic lifecycle ────────────────────────────────────────

    #[test]
    fn alloc_init() {
        let lm = shim_nslayoutmanager_alloc(0, 0);
        let lm = shim_nslayoutmanager_init(lm, 0);
        assert_ne!(lm, 0);
        assert_eq!(read_u64(lm, 0), crate::nsobj::nslayoutmanager_class_addr());
        free_obj(lm);
    }

    #[test]
    fn text_storage_round_trip() {
        let lm = shim_nslayoutmanager_alloc(0, 0);
        let lm = shim_nslayoutmanager_init(lm, 0);
        assert_eq!(shim_nslayoutmanager_text_storage(lm, 0), 0);
        shim_nslayoutmanager_set_text_storage(lm, 0, 0xDEAD);
        assert_eq!(shim_nslayoutmanager_text_storage(lm, 0), 0xDEAD);
        free_obj(lm);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nslayoutmanager_init(0, 0), 0);
        assert_eq!(shim_nslayoutmanager_text_storage(0, 0), 0);
        assert_eq!(shim_nslayoutmanager_number_of_glyphs(0, 0), 0);
        assert_eq!(shim_nslayoutmanager_used_rect(0, 0, 0), CGRect::ZERO);
        assert_eq!(
            shim_nslayoutmanager_glyph_range_for_tc(0, 0, 0),
            core_foundation_lite::NSRange {
                location: 0,
                length: 0
            },
        );
        assert_eq!(
            shim_nslayoutmanager_bounding_rect(0, 0, 0, 0, 0),
            CGRect::ZERO
        );
        assert_eq!(
            shim_nslayoutmanager_line_fragment_rect(0, 0, 0, 0),
            CGRect::ZERO
        );
    }

    // ── wired stack tests ──────────────────────────────────────

    #[test]
    fn add_text_container_sets_back_pointer() {
        let lm = shim_nslayoutmanager_alloc(0, 0);
        let lm = shim_nslayoutmanager_init(lm, 0);
        let tc = crate::nstextcontainer::shim_nstextcontainer_alloc(0, 0);
        let tc = crate::nstextcontainer::shim_nstextcontainer_init(tc, 0);
        shim_nslayoutmanager_add_text_container(lm, 0, tc);
        // Container's layoutManager should point back to lm.
        assert_eq!(
            crate::nstextcontainer::shim_nstextcontainer_layout_manager(tc, 0),
            lm
        );
        free_obj(tc);
        free_obj(lm);
    }

    #[test]
    fn number_of_glyphs_matches_text_length() {
        let (ts, lm, tc) = make_textkit_stack("Hello", 300.0, 200.0);
        assert_eq!(shim_nslayoutmanager_number_of_glyphs(lm, 0), 5);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn used_rect_nonempty_for_text() {
        let (ts, lm, tc) = make_textkit_stack("Hello TextKit", 300.0, 200.0);
        let rect = shim_nslayoutmanager_used_rect(lm, 0, tc);
        assert!(
            rect.size.width > 0.0,
            "width should be > 0, got {}",
            rect.size.width
        );
        assert!(
            rect.size.height > 0.0,
            "height should be > 0, got {}",
            rect.size.height
        );
        assert_eq!(rect.origin.x, 0.0);
        assert_eq!(rect.origin.y, 0.0);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn used_rect_empty_text() {
        let (ts, lm, tc) = make_textkit_stack("", 300.0, 200.0);
        let rect = shim_nslayoutmanager_used_rect(lm, 0, tc);
        // Empty text should still have line height (1 line).
        assert!(rect.size.height > 0.0);
        // Width should be 0 (no characters).
        assert!(rect.size.width < 1.0);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn used_rect_multiline() {
        let (ts, lm, tc) = make_textkit_stack("Line1\nLine2\nLine3", 300.0, 300.0);
        let rect = shim_nslayoutmanager_used_rect(lm, 0, tc);
        // Should have height for 3 lines.
        let single = {
            let (ts1, lm1, tc1) = make_textkit_stack("X", 300.0, 300.0);
            let r = shim_nslayoutmanager_used_rect(lm1, 0, tc1);
            free_textkit(ts1, lm1, tc1);
            r.size.height
        };
        // 3 lines should be roughly 3× the single-line height.
        assert!(
            rect.size.height >= single * 2.5,
            "3-line height {} should be ≥ 2.5 × single-line {} = {}",
            rect.size.height,
            single,
            single * 2.5,
        );
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn used_rect_no_storage() {
        // Layout manager with no text storage → zero rect.
        let lm = shim_nslayoutmanager_alloc(0, 0);
        let lm = shim_nslayoutmanager_init(lm, 0);
        let tc = crate::nstextcontainer::shim_nstextcontainer_alloc(0, 0);
        let tc = crate::nstextcontainer::shim_nstextcontainer_init(tc, 0);
        let rect = shim_nslayoutmanager_used_rect(lm, 0, tc);
        // Should be for empty text (height > 0, width ~ 0).
        assert!(rect.size.height > 0.0);
        free_obj(tc);
        free_obj(lm);
    }

    #[test]
    fn glyph_range_for_container() {
        let (ts, lm, tc) = make_textkit_stack("ABCDE", 300.0, 200.0);
        let r = shim_nslayoutmanager_glyph_range_for_tc(lm, 0, tc);
        assert_eq!(r.location, 0);
        assert_eq!(r.length, 5);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn bounding_rect_matches_used_rect() {
        let (ts, lm, tc) = make_textkit_stack("Measure me", 400.0, 200.0);
        let used = shim_nslayoutmanager_used_rect(lm, 0, tc);
        let bound = shim_nslayoutmanager_bounding_rect(lm, 0, 0, 10, tc);
        assert_eq!(used, bound);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn line_fragment_rect_nonempty() {
        let (ts, lm, tc) = make_textkit_stack("Fragment", 300.0, 200.0);
        let r = shim_nslayoutmanager_line_fragment_rect(lm, 0, 0, 0);
        assert!(r.size.width > 0.0);
        assert!(r.size.height > 0.0);
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn ensure_layout_does_not_crash() {
        let (ts, lm, tc) = make_textkit_stack("Layout test", 300.0, 200.0);
        shim_nslayoutmanager_ensure_layout(lm, 0, tc);
        // If we get here without panic, the pipeline works.
        free_textkit(ts, lm, tc);
    }

    #[test]
    fn wider_text_gives_wider_rect() {
        let (ts1, lm1, tc1) = make_textkit_stack("Hi", 500.0, 200.0);
        let (ts2, lm2, tc2) = make_textkit_stack("Hello, World!", 500.0, 200.0);
        let r1 = shim_nslayoutmanager_used_rect(lm1, 0, tc1);
        let r2 = shim_nslayoutmanager_used_rect(lm2, 0, tc2);
        assert!(
            r2.size.width > r1.size.width,
            "longer text ({}) should be wider than short text ({})",
            r2.size.width,
            r1.size.width,
        );
        free_textkit(ts1, lm1, tc1);
        free_textkit(ts2, lm2, tc2);
    }
}
