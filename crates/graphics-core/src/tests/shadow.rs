use crate::color::Color;
use crate::geometry::Rect;
use crate::renderer::Renderer;
use crate::shadow::Shadow;
use crate::software::SoftwareRenderer;
use crate::surface::Surface;

#[test]
fn shadow_draws_content() {
    let mut surface = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::new(5.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
        });
    }
    // Content pixel at (25, 25) should be red
    let p = surface.pixels()[(25 * 100 + 25) as usize];
    let r = (p >> 16) & 0xFF;
    assert!(r > 200, "expected red content pixel, got 0x{:08X}", p);
}

#[test]
fn shadow_produces_offset_pixels() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::new(5.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
        });
    }
    // Shadow pixel at offset (25+5, 25+5) = (30,30) should be shadow-tinted
    // but (30,30) also overlaps content (20..30 range), so check (27,27):
    // content is at x=20..30, y=20..30; shadow offset by (5,5) => shadow at x=25..35, y=25..35
    // Pixel (32, 32) is in shadow region but NOT in content region
    let shadow_px = surface.pixels()[(32 * 100 + 32) as usize];
    let shadow_a = (shadow_px >> 24) & 0xFF;
    // Should have some shadow alpha composited onto white
    assert!(
        shadow_a > 0,
        "expected shadow pixel at (32,32), got 0x{:08X}",
        shadow_px
    );
    // The pixel should be darker than pure white (0xFFFFFFFF)
    let shadow_r = (shadow_px >> 16) & 0xFF;
    assert!(
        shadow_r < 255,
        "expected darkened pixel from shadow, got 0x{:08X}",
        shadow_px
    );
}

#[test]
fn shadow_no_blur_sharp_edge() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::new(10.0, 10.0, 0.0, Color::new(0.0, 0.0, 0.0, 1.0));
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
        });
    }
    // Shadow region: (30..40, 30..40) with blur=0 => pixel at (35,35) should be shadow
    let inside = surface.pixels()[(35 * 100 + 35) as usize];
    // Just outside at (29, 35) — no shadow (before shadow x range)
    let outside = surface.pixels()[(35 * 100 + 29) as usize];
    // Inside should be black (shadow color with alpha=1.0)
    let inside_r = (inside >> 16) & 0xFF;
    assert!(
        inside_r < 10,
        "expected dark shadow pixel at (35,35), got 0x{:08X}",
        inside
    );
    // Outside should be white (untouched)
    assert_eq!(
        outside, 0xFFFFFFFF,
        "expected white outside shadow, got 0x{:08X}",
        outside
    );
}

#[test]
fn shadow_blur_softens() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::new(10.0, 10.0, 3.0, Color::new(0.0, 0.0, 0.0, 1.0));
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
        });
    }
    // With blur_radius=3, shadow should extend beyond the sharp (30..40, 30..40) region.
    // Pixel at (42, 35) is 2px past the sharp edge — should have some shadow due to blur.
    let extended = surface.pixels()[(35 * 100 + 42) as usize];
    let ext_r = (extended >> 16) & 0xFF;
    // Should be somewhat darkened (less than pure white 255)
    assert!(
        ext_r < 255,
        "expected blur to extend shadow, pixel at (42,35) = 0x{:08X}",
        extended
    );
}

#[test]
fn shadow_standard_does_not_panic() {
    let mut surface = Surface::new(80, 80);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::standard();
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(10.0, 10.0, 30.0, 30.0), Color::BLUE);
        });
    }
    // Just verify it didn't panic and content is present
    let p = surface.pixels()[(20 * 80 + 20) as usize];
    let b = p & 0xFF;
    assert!(b > 200, "expected blue content, got 0x{:08X}", p);
}

#[test]
fn shadow_zero_offset_zero_blur() {
    let mut surface = Surface::new(60, 60);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let shadow = Shadow::new(0.0, 0.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(10.0, 10.0, 20.0, 20.0), Color::RED);
        });
    }
    // Content should still be present
    let content = surface.pixels()[(15 * 60 + 15) as usize];
    let cr = (content >> 16) & 0xFF;
    assert!(cr > 200, "expected red content, got 0x{:08X}", content);

    // Outside content area should be white (shadow with zero offset/blur overlaps content exactly)
    let outside = surface.pixels()[(5 * 60 + 5) as usize];
    assert_eq!(
        outside, 0xFFFFFFFF,
        "expected white outside, got 0x{:08X}",
        outside
    );
}
