use super::*;

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

