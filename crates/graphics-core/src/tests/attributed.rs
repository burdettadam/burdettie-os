use super::*;

// ── Attributed text tests ──────────────────────────────────────

#[test]
fn font_bold_italic_flags() {
    let f = Font::system(14.0).with_bold().with_italic();
    assert!(f.bold);
    assert!(f.italic);
    assert_eq!(f.name, "System");
    assert_eq!(f.size, 14.0);
}

#[test]
fn font_manager_has_italic_fonts() {
    let fm = FontManager::new();
    assert_eq!(fm.italic().name(), "System-Italic");
    assert_eq!(fm.bold_italic().name(), "System-BoldItalic");
}

#[test]
fn font_manager_resolve_font() {
    let fm = FontManager::new();
    let regular = Font::system(14.0);
    assert_eq!(fm.resolve_font(&regular).name(), "System");

    let bold = Font::system(14.0).with_bold();
    assert_eq!(fm.resolve_font(&bold).name(), "System-Bold");

    let italic = Font::system(14.0).with_italic();
    assert_eq!(fm.resolve_font(&italic).name(), "System-Italic");

    let bold_italic = Font::system(14.0).with_bold().with_italic();
    assert_eq!(fm.resolve_font(&bold_italic).name(), "System-BoldItalic");
}

#[test]
fn layout_attributed_single_run_bold() {
    let fm = FontManager::new();
    let text = "Hello";
    let runs = vec![TextRun {
        range: 0..5,
        font_name: None,
        size: None,
        color: Some(Color::RED),
        bold: true,
        italic: false,
    }];
    let layout = layout_attributed(text, &runs, &fm, fm.system(), 14.0);
    assert_eq!(layout.glyphs.len(), 5);
    // All glyphs should reference the bold font (index > 0)
    assert!(layout.glyphs.iter().all(|g| g.font_index > 0));
    // All glyphs should have red color
    assert!(layout.glyphs.iter().all(|g| g.color == Some(Color::RED)));
    assert!(layout.width > 0.0);
    assert!(layout.height > 0.0);
}

#[test]
fn layout_attributed_mixed_runs() {
    let fm = FontManager::new();
    let text = "Hello World";
    // "Hello" = bold, " World" = italic red
    let runs = vec![
        TextRun {
            range: 0..5,
            font_name: None,
            size: None,
            color: None,
            bold: true,
            italic: false,
        },
        TextRun {
            range: 5..11,
            font_name: None,
            size: None,
            color: Some(Color::RED),
            bold: false,
            italic: true,
        },
    ];
    let layout = layout_attributed(text, &runs, &fm, fm.system(), 14.0);
    assert_eq!(layout.glyphs.len(), 11); // "Hello World" = 11 chars
                                         // "Hello" glyphs should not have color override
    assert!(layout.glyphs[0].color.is_none());
    // " World" glyphs should have red
    assert_eq!(layout.glyphs[5].color, Some(Color::RED));
    // Should have at least 2 distinct font indices (bold + italic)
    let font_indices: std::collections::HashSet<usize> =
        layout.glyphs.iter().map(|g| g.font_index).collect();
    assert!(font_indices.len() >= 2, "expected multiple font indices");
}

#[test]
fn layout_attributed_size_override() {
    let fm = FontManager::new();
    let text = "Ab";
    let runs = vec![
        TextRun {
            range: 0..1,
            font_name: None,
            size: Some(24.0),
            color: None,
            bold: false,
            italic: false,
        },
        TextRun {
            range: 1..2,
            font_name: None,
            size: Some(10.0),
            color: None,
            bold: false,
            italic: false,
        },
    ];
    let layout = layout_attributed(text, &runs, &fm, fm.system(), 14.0);
    assert_eq!(layout.glyphs.len(), 2);
    assert_eq!(layout.glyphs[0].size, 24.0);
    assert_eq!(layout.glyphs[1].size, 10.0);
}

#[test]
fn draw_attributed_text_produces_pixels() {
    let fm = FontManager::new();
    let text = "Bold Italic";
    let runs = vec![
        TextRun {
            range: 0..4,
            font_name: None,
            size: None,
            color: Some(Color::RED),
            bold: true,
            italic: false,
        },
        TextRun {
            range: 4..11,
            font_name: None,
            size: None,
            color: Some(Color::BLUE),
            bold: false,
            italic: true,
        },
    ];
    let layout = layout_attributed(text, &runs, &fm, fm.system(), 14.0);

    let mut s = Surface::new(300, 50);
    {
        let mut r = SoftwareRenderer::with_fonts(&mut s, &fm);
        r.clear(Color::WHITE);
        r.draw_attributed_text(&layout, Point::new(5.0, 5.0), Color::BLACK);
    }
    let white = Color::WHITE.to_argb32();
    let non_white = s.pixels().iter().filter(|&&p| p != white).count();
    assert!(non_white > 30, "Expected rendered pixels, got {non_white}");
}

#[test]
fn draw_text_with_bold_font() {
    let fm = FontManager::new();
    let mut s = Surface::new(200, 40);
    {
        let mut r = SoftwareRenderer::with_fonts(&mut s, &fm);
        r.clear(Color::WHITE);
        r.draw_text(
            "Bold",
            Point::new(5.0, 5.0),
            &Font::system(14.0).with_bold(),
            Color::BLACK,
        );
    }
    let white = Color::WHITE.to_argb32();
    let non_white = s.pixels().iter().filter(|&&p| p != white).count();
    assert!(non_white > 20, "Expected bold text pixels, got {non_white}");
}

#[test]
fn supports_attributed_text_with_fonts() {
    let fm = FontManager::new();
    let mut s = Surface::new(100, 40);
    let r = SoftwareRenderer::with_fonts(&mut s, &fm);
    assert!(r.supports_attributed_text());
}

#[test]
fn supports_attributed_text_without_fonts() {
    let mut s = Surface::new(100, 40);
    let r = SoftwareRenderer::new(&mut s);
    assert!(!r.supports_attributed_text());
}
