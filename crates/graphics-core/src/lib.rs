//! 2D drawing primitives and renderer trait.
//!
//! Provides geometry types (`Rect`, `Point`, `Color`, `Font`), a pixel
//! buffer (`Surface`), and a `Renderer` trait for drawing shapes and text.
//! Backends implement `Renderer` for their specific rendering target.

mod geometry;
mod color;
mod font;
mod font_manager;
mod text_layout;
mod surface;
mod renderer;
mod software;
pub mod unicode;

pub use geometry::*;
pub use color::*;
pub use font::*;
pub use font_manager::*;
pub use text_layout::*;
pub use surface::*;
pub use renderer::*;
pub use software::*;

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_basic() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(5.0, 30.0)));
        assert!(!r.contains(Point::new(50.0, 61.0)));
        // Edge: top-left is inside
        assert!(r.contains(Point::new(10.0, 10.0)));
        // Edge: bottom-right is outside (exclusive)
        assert!(!r.contains(Point::new(110.0, 60.0)));
    }

    #[test]
    fn rect_intersects() {
        let a = Rect::new(0.0, 0.0, 50.0, 50.0);
        let b = Rect::new(25.0, 25.0, 50.0, 50.0);
        let c = Rect::new(100.0, 100.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn rect_inset() {
        let r = Rect::new(10.0, 10.0, 100.0, 80.0);
        let inset = r.inset(5.0, 10.0);
        assert_eq!(inset.x, 15.0);
        assert_eq!(inset.y, 20.0);
        assert_eq!(inset.width, 90.0);
        assert_eq!(inset.height, 60.0);
    }

    #[test]
    fn color_to_argb32() {
        assert_eq!(Color::WHITE.to_argb32(), 0xFFFFFFFF);
        assert_eq!(Color::BLACK.to_argb32(), 0xFF000000);
        assert_eq!(Color::RED.to_argb32(), 0xFFFF0000);
        assert_eq!(Color::TRANSPARENT.to_argb32(), 0x00000000);
    }

    #[test]
    fn surface_clear_and_pixel() {
        let mut s = Surface::new(10, 10);
        s.clear(Color::RED);
        assert_eq!(s.pixels()[0], Color::RED.to_argb32());

        s.set_pixel(5, 5, Color::BLUE.to_argb32());
        assert_eq!(s.pixels()[55], Color::BLUE.to_argb32());
    }

    #[test]
    fn software_renderer_fill_rect() {
        let mut s = Surface::new(100, 100);
        {
            let mut r = SoftwareRenderer::new(&mut s);
            r.clear(Color::WHITE);
            r.fill_rect(Rect::new(10.0, 10.0, 5.0, 5.0), Color::RED);
        }
        // Check a pixel inside the filled rect
        assert_eq!(s.pixels()[(12 * 100 + 12) as usize], Color::RED.to_argb32());
        // Check a pixel outside
        assert_eq!(s.pixels()[0], Color::WHITE.to_argb32());
    }

    #[test]
    fn software_renderer_stroke_rect() {
        let mut s = Surface::new(100, 100);
        {
            let mut r = SoftwareRenderer::new(&mut s);
            r.clear(Color::WHITE);
            r.stroke_rect(Rect::new(10.0, 10.0, 20.0, 20.0), Color::BLACK, 1.0);
        }
        // Top edge
        assert_eq!(
            s.pixels()[(10 * 100 + 15) as usize],
            Color::BLACK.to_argb32()
        );
        // Interior should still be white
        assert_eq!(
            s.pixels()[(20 * 100 + 20) as usize],
            Color::WHITE.to_argb32()
        );
    }

    #[test]
    fn font_default() {
        let f = Font::default();
        assert_eq!(f.name, "System");
        assert_eq!(f.size, 14.0);
    }

    // ── FontManager tests ──────────────────────────────────

    #[test]
    fn font_manager_loads_default_fonts() {
        let fm = FontManager::new();
        assert_eq!(fm.system().name(), "System");
        assert_eq!(fm.bold().name(), "System-Bold");
        assert_eq!(fm.monospace().name(), "Monospace");
    }

    #[test]
    fn font_manager_get_fallback() {
        let fm = FontManager::new();
        // Unknown name falls back to system font
        let face = fm.get("NonExistent");
        assert_eq!(face.name(), "System");
    }

    #[test]
    fn font_face_metrics() {
        let fm = FontManager::new();
        let m = fm.system().metrics(14.0);
        // Ascent should be positive, descent negative
        assert!(m.ascent > 0.0);
        assert!(m.descent < 0.0);
        assert!(m.line_height() > 0.0);
    }

    #[test]
    fn font_face_measure_text() {
        let fm = FontManager::new();
        let w = fm.system().measure_text("Hello", 14.0);
        assert!(w > 0.0);
        // Longer text should be wider
        let w2 = fm.system().measure_text("Hello, World!", 14.0);
        assert!(w2 > w);
    }

    #[test]
    fn font_face_char_width() {
        let fm = FontManager::new();
        let w = fm.system().char_width('A', 14.0);
        assert!(w > 0.0);
    }

    #[test]
    fn font_face_line_height() {
        let fm = FontManager::new();
        let lh = fm.system().line_height(14.0);
        assert!(lh > 0.0);
        // Line height should scale with size
        let lh_big = fm.system().line_height(28.0);
        assert!(lh_big > lh);
    }

    // ── TextLayout tests ───────────────────────────────────

    #[test]
    fn text_layout_single_line() {
        let fm = FontManager::new();
        let layout = layout_single_line("Hello", fm.system(), 14.0);
        assert_eq!(layout.line_count, 1);
        assert_eq!(layout.glyphs.len(), 5);
        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);
    }

    #[test]
    fn text_layout_empty_string() {
        let fm = FontManager::new();
        let layout = layout_single_line("", fm.system(), 14.0);
        assert_eq!(layout.glyphs.len(), 0);
        assert_eq!(layout.line_count, 1);
    }

    #[test]
    fn text_layout_wrapped_no_wrap() {
        let fm = FontManager::new();
        let layout = layout_wrapped("Hello World", fm.system(), 14.0, 1000.0, WrapMode::NoWrap);
        assert_eq!(layout.line_count, 1);
    }

    #[test]
    fn text_layout_wrapped_word_wrap() {
        let fm = FontManager::new();
        // Use a narrow width to force wrapping
        let layout = layout_wrapped("Hello World", fm.system(), 14.0, 50.0, WrapMode::Word);
        assert!(layout.line_count >= 2, "Expected multi-line, got {}", layout.line_count);
    }

    #[test]
    fn text_layout_wrapped_char_wrap() {
        let fm = FontManager::new();
        let layout = layout_wrapped("ABCDEFGHIJ", fm.system(), 14.0, 30.0, WrapMode::Character);
        assert!(layout.line_count >= 2, "Expected multi-line, got {}", layout.line_count);
    }

    #[test]
    fn text_layout_newline_handling() {
        let fm = FontManager::new();
        let layout = layout_wrapped("Line1\nLine2\nLine3", fm.system(), 14.0, 1000.0, WrapMode::NoWrap);
        assert_eq!(layout.line_count, 3);
    }

    // ── TrueType rendering tests ───────────────────────────

    #[test]
    fn truetype_render_produces_pixels() {
        let fm = FontManager::new();
        let mut s = Surface::new(200, 50);
        {
            let mut r = SoftwareRenderer::with_fonts(&mut s, &fm);
            r.clear(Color::WHITE);
            r.draw_text("Hello", Point::new(10.0, 5.0), &Font::system(14.0), Color::BLACK);
        }
        // At least some pixels should differ from white after rendering text
        let white = Color::WHITE.to_argb32();
        let non_white = s.pixels().iter().filter(|&&p| p != white).count();
        assert!(non_white > 0, "Expected non-white pixels from TrueType rendering");
    }

    #[test]
    fn truetype_measure_text() {
        let fm = FontManager::new();
        let mut s = Surface::new(100, 50);
        let r = SoftwareRenderer::with_fonts(&mut s, &fm);
        let w = r.measure_text("Test", &Font::system(14.0));
        assert!(w > 0.0);
    }

    // ── Unicode integration tests ──────────────────────────

    #[test]
    fn unicode_nfc_layout_combining_acute() {
        // NFD "é" (e + combining acute) should produce the same layout as NFC "é"
        let fm = FontManager::new();
        let nfd = layout_single_line("e\u{0301}", fm.system(), 14.0);
        let nfc = layout_single_line("é", fm.system(), 14.0);
        // After NFC normalization, both should have 1 base glyph
        // (NFD composes to the single precomposed é)
        assert_eq!(nfc.glyphs.len(), 1);
        assert_eq!(nfd.glyphs.len(), 1);
        // Widths should be equal
        assert!((nfd.width - nfc.width).abs() < 0.01,
            "NFD width {} != NFC width {}", nfd.width, nfc.width);
    }

    #[test]
    fn unicode_combining_mark_no_advance() {
        let fm = FontManager::new();
        // U+0308 (combining diaeresis) — use 'q' which has no precomposed form
        // with diaeresis, so NFC won't collapse it
        let layout = layout_single_line("q\u{0308}", fm.system(), 14.0);
        // Should have 2 glyphs (base + combining mark)
        assert_eq!(layout.glyphs.len(), 2);
        // The combining mark should be at the same x as the base character
        assert!((layout.glyphs[0].x - layout.glyphs[1].x).abs() < 0.01,
            "Combining mark at x={} should overlap base at x={}", layout.glyphs[1].x, layout.glyphs[0].x);
        // Total width should be roughly the same as just 'q' (combining mark doesn't advance)
        let q_layout = layout_single_line("q", fm.system(), 14.0);
        assert!((layout.width - q_layout.width).abs() < 0.01,
            "q+diaeresis width {} should equal q width {}", layout.width, q_layout.width);
    }

    #[test]
    fn unicode_zero_width_chars_skipped() {
        let fm = FontManager::new();
        // Insert ZWJ and ZWNJ between characters — should not affect width
        let plain = layout_single_line("ab", fm.system(), 14.0);
        let with_zwj = layout_single_line("a\u{200D}b", fm.system(), 14.0);
        let with_zwnj = layout_single_line("a\u{200C}b", fm.system(), 14.0);
        let with_zwsp = layout_single_line("a\u{200B}b", fm.system(), 14.0);
        assert!((plain.width - with_zwj.width).abs() < 0.01);
        assert!((plain.width - with_zwnj.width).abs() < 0.01);
        assert!((plain.width - with_zwsp.width).abs() < 0.01);
    }

    #[test]
    fn unicode_zero_width_chars_excluded_from_glyphs() {
        let fm = FontManager::new();
        let layout = layout_single_line("a\u{200D}b", fm.system(), 14.0);
        // Should be 2 glyphs (a and b), ZWJ skipped
        assert_eq!(layout.glyphs.len(), 2);
    }

    #[test]
    fn unicode_emoji_missing_glyph() {
        let fm = FontManager::new();
        // Use a character outside DejaVu's coverage — a rare CJK ideograph
        // that DejaVu Sans definitely doesn't include
        let layout = layout_single_line("\u{2F800}", fm.system(), 14.0); // CJK Compatibility Ideograph
        assert_eq!(layout.glyphs.len(), 1);
        assert!(layout.glyphs[0].missing,
            "CJK ideograph should be marked as missing glyph, glyph_id={:?}", layout.glyphs[0].glyph_id);
    }

    #[test]
    fn unicode_emoji_renders_in_dejavu() {
        // DejaVu Sans has monochrome emoji — verify they render successfully
        let fm = FontManager::new();
        assert!(fm.system().has_glyph('😀'), "DejaVu Sans should have 😀");
        let layout = layout_single_line("😀", fm.system(), 14.0);
        assert_eq!(layout.glyphs.len(), 1);
        assert!(!layout.glyphs[0].missing);
    }

    #[test]
    fn unicode_emoji_placeholder_renders() {
        let fm = FontManager::new();
        let mut s = Surface::new(200, 50);
        {
            let mut r = SoftwareRenderer::with_fonts(&mut s, &fm);
            r.clear(Color::WHITE);
            r.draw_text("😀", Point::new(10.0, 5.0), &Font::system(14.0), Color::BLACK);
        }
        // Placeholder box should produce some non-white pixels
        let white = Color::WHITE.to_argb32();
        let non_white = s.pixels().iter().filter(|&&p| p != white).count();
        assert!(non_white > 0, "Expected placeholder box pixels for emoji");
    }

    #[test]
    fn unicode_mixed_text_layout() {
        let fm = FontManager::new();
        // Mix of ASCII, accented, and CJK-range characters
        let layout = layout_single_line("Héllo", fm.system(), 14.0);
        assert_eq!(layout.glyphs.len(), 5);
        assert!(layout.width > 0.0);
    }

    #[test]
    fn unicode_bom_filtered() {
        let fm = FontManager::new();
        // BOM at start should be filtered
        let with_bom = layout_single_line("\u{FEFF}Hello", fm.system(), 14.0);
        let without = layout_single_line("Hello", fm.system(), 14.0);
        assert_eq!(with_bom.glyphs.len(), without.glyphs.len());
        assert!((with_bom.width - without.width).abs() < 0.01);
    }

    #[test]
    fn unicode_soft_hyphen_filtered() {
        let fm = FontManager::new();
        let with_shy = layout_single_line("Hel\u{00AD}lo", fm.system(), 14.0);
        let without = layout_single_line("Hello", fm.system(), 14.0);
        assert_eq!(with_shy.glyphs.len(), without.glyphs.len());
        assert!((with_shy.width - without.width).abs() < 0.01);
    }

    #[test]
    fn unicode_measure_text_nfc() {
        let fm = FontManager::new();
        // NFD and NFC should measure the same
        let w_nfd = fm.system().measure_text("e\u{0301}", 14.0);
        let w_nfc = fm.system().measure_text("é", 14.0);
        assert!((w_nfd - w_nfc).abs() < 0.01,
            "NFD measure {} != NFC measure {}", w_nfd, w_nfc);
    }

    #[test]
    fn unicode_wrapped_combining_not_split() {
        let fm = FontManager::new();
        // Word with combining mark should not be split at the combining mark
        let layout = layout_wrapped("x\u{0308} test", fm.system(), 14.0, 1000.0, WrapMode::Word);
        assert_eq!(layout.line_count, 1);
        // x + combining diaeresis + space + test = 2 + 1 + 4 = 7 glyphs
        // (combining mark present as separate glyph at same x)
        assert!(layout.glyphs.len() >= 6);
    }

    #[test]
    fn unicode_multiple_combining_marks() {
        let fm = FontManager::new();
        // q + combining acute + combining diaeresis (stacked diacritics)
        // Neither composes with q in NFC, so both marks remain
        let layout = layout_single_line("q\u{0301}\u{0308}", fm.system(), 14.0);
        // Should have 3 glyphs: q (base) + acute (combining) + diaeresis (combining)
        assert_eq!(layout.glyphs.len(), 3);
        // Both combining marks should be at the same x as the base character
        assert!((layout.glyphs[0].x - layout.glyphs[1].x).abs() < 0.01);
        assert!((layout.glyphs[0].x - layout.glyphs[2].x).abs() < 0.01);
    }

    #[test]
    fn unicode_render_accented_text() {
        let fm = FontManager::new();
        let mut s = Surface::new(300, 50);
        {
            let mut r = SoftwareRenderer::with_fonts(&mut s, &fm);
            r.clear(Color::WHITE);
            r.draw_text("Ü Ö Ä é à ñ", Point::new(5.0, 5.0), &Font::system(14.0), Color::BLACK);
        }
        let white = Color::WHITE.to_argb32();
        let non_white = s.pixels().iter().filter(|&&p| p != white).count();
        assert!(non_white > 50, "Expected many non-white pixels for accented text, got {}", non_white);
    }
}
