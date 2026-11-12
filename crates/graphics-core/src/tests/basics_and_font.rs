use super::*;

#[allow(unused_imports)]
use super::*;

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
