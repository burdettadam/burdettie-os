use super::*;

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

const RED_ARGB: u32 = 0xFFFF0000;

// ── Line join / cap defaults ───────────────────────────────────────

#[test]
fn line_join_default_miter() {
    let mut surface = Surface::new(10, 10);
    let r = SoftwareRenderer::new(&mut surface);
    assert_eq!(r.line_join(), LineJoin::Miter);
}

#[test]
fn line_cap_default_butt() {
    let mut surface = Surface::new(10, 10);
    let r = SoftwareRenderer::new(&mut surface);
    assert_eq!(r.line_cap(), LineCap::Butt);
}

#[test]
fn set_line_join_stored() {
    let mut surface = Surface::new(10, 10);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.set_line_join(LineJoin::Round);
    assert_eq!(r.line_join(), LineJoin::Round);
}

#[test]
fn set_line_cap_stored() {
    let mut surface = Surface::new(10, 10);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.set_line_cap(LineCap::Square);
    assert_eq!(r.line_cap(), LineCap::Square);
}

#[test]
fn line_join_saved_and_restored() {
    let mut surface = Surface::new(10, 10);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.save();
    r.set_line_join(LineJoin::Bevel);
    assert_eq!(r.line_join(), LineJoin::Bevel);
    r.restore();
    assert_eq!(r.line_join(), LineJoin::Miter);
}

// ── Opacity ────────────────────────────────────────────────────────

#[test]
fn opacity_default_is_one() {
    let mut surface = Surface::new(10, 10);
    let r = SoftwareRenderer::new(&mut surface);
    assert_eq!(r.opacity(), 1.0);
}

#[test]
fn set_opacity_changes_alpha() {
    let mut surface = Surface::new(10, 10);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.set_opacity(0.5);
        r.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::RED);
    }
    let p = px(&surface, 5, 5);
    let alpha = (p >> 24) & 0xFF;
    // Full-alpha RED (0xFF) * 0.5 → ~0x7F
    assert!(alpha >= 0x7D && alpha <= 0x80, "expected ~127, got {alpha}");
}

#[test]
fn opacity_saved_and_restored() {
    let mut surface = Surface::new(10, 10);
    let mut r = SoftwareRenderer::new(&mut surface);
    assert_eq!(r.opacity(), 1.0);
    r.save();
    r.set_opacity(0.25);
    assert_eq!(r.opacity(), 0.25);
    r.restore();
    assert_eq!(r.opacity(), 1.0);
}

// ── Arc / sector ───────────────────────────────────────────────────

#[test]
fn fill_arc_quarter() {
    // Quarter circle from 0 to π/2 (right-top quadrant in screen coords:
    // atan2 is positive-y = downward, so 0..π/2 covers the bottom-right sector).
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.fill_arc(
            Point::new(50.0, 50.0),
            40.0,
            0.0,
            std::f64::consts::FRAC_PI_2,
            Color::RED,
        );
    }
    // Center-right (dx>0, dy=0, angle=0) should be filled.
    assert_eq!(px(&surface, 70, 50), RED_ARGB);
    // A point well outside the arc angle (top-left quadrant) should remain black.
    assert_eq!(px(&surface, 20, 20), 0xFF000000);
}

#[test]
fn stroke_arc_produces_pixels() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.stroke_arc(
            Point::new(50.0, 50.0),
            30.0,
            0.0,
            std::f64::consts::PI,
            Color::RED,
            2.0,
        );
    }
    // At least some pixel on the arc should be red. Sample at (80, 50) which
    // is on angle=0 at radius=30 from center 50,50 → x=80.
    let p = px(&surface, 80, 50);
    assert_eq!(p, RED_ARGB);
}
