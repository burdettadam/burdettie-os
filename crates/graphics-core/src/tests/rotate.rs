use super::*;

use std::f64::consts::PI;

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

const WHITE_ARGB: u32 = 0xFFFFFFFF;
const RED_ARGB: u32 = 0xFFFF0000;

// ── Rotation tests ─────────────────────────────────────────────────

#[test]
fn rotate_zero_is_noop() {
    // Rotating by 0 should produce identical output to no rotation.
    let mut s1 = Surface::new(50, 50);
    s1.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut s1);
        r.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::RED);
    }

    let mut s2 = Surface::new(50, 50);
    s2.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut s2);
        r.rotate(0.0);
        r.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::RED);
    }

    assert_eq!(s1.pixels(), s2.pixels());
}

#[test]
fn rotate_saves_and_restores() {
    // save, rotate, restore — drawing position should be unaffected.
    let mut surface = Surface::new(50, 50);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.save();
        r.rotate(PI / 4.0);
        r.restore();
        // After restore, rect at (5,5) should land exactly at (5,5).
        r.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::RED);
    }
    assert_eq!(px(&surface, 7, 7), RED_ARGB);
    assert_eq!(px(&surface, 0, 0), WHITE_ARGB);
}

#[test]
fn rotate_quarter_turn() {
    // Rotate PI/2 (90°). Point (5, 0) should map to (0, 5).
    let mut surface = Surface::new(20, 20);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.rotate(PI / 2.0);
        // Draw a single-pixel rect at (5,0). After 90° rotation:
        //   rx = 5*cos(90°) - 0*sin(90°) = 0
        //   ry = 5*sin(90°) + 0*cos(90°) = 5
        r.fill_rect(Rect::new(5.0, 0.0, 1.0, 1.0), Color::RED);
    }
    // The pixel should land near (0, 5).
    assert_eq!(px(&surface, 0, 5), RED_ARGB);
    // The original location (5, 0) should still be white.
    assert_eq!(px(&surface, 5, 0), WHITE_ARGB);
}

#[test]
fn rotation_field_saved() {
    // save + rotate + restore verifies rotation is restored to previous value.
    let mut surface = Surface::new(40, 40);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.rotate(PI / 6.0); // 30°
        r.save();
        r.rotate(PI / 3.0); // adds 60° → total 90°
        r.restore(); // should be back to 30°

        // At 30° rotation, a 5×5 rect at (10, 0) maps its center ~(10.8, 7.4)
        // to a bounding box roughly covering (6..13, 5..12).
        r.fill_rect(Rect::new(10.0, 0.0, 5.0, 5.0), Color::RED);
    }
    // Should be red somewhere in the bounding box (e.g. center area).
    assert_eq!(px(&surface, 9, 8), RED_ARGB);
    // (0, 12) should be white — rotation was restored to 30°, not 90°.
    assert_eq!(px(&surface, 0, 12), WHITE_ARGB);
}

// ── draw_image clip tests ──────────────────────────────────────────

fn make_red_image(w: u32, h: u32) -> ImageBuffer {
    // RGBA8: every pixel is opaque red.
    let data = vec![255u8, 0, 0, 255].repeat((w * h) as usize);
    ImageBuffer::from_rgba(w, h, data).unwrap()
}

#[test]
fn draw_image_respects_clip() {
    let img = make_red_image(20, 20);
    let mut surface = Surface::new(40, 40);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // Clip to (5,5)-(15,15)
        r.push_clip(Rect::new(5.0, 5.0, 10.0, 10.0));
        // Draw image at (0,0) spanning 20x20 — only the clipped region should appear.
        r.draw_image(&img, Rect::new(0.0, 0.0, 20.0, 20.0));
    }
    // Inside clip: should be red.
    assert_eq!(px(&surface, 7, 7), RED_ARGB);
    assert_eq!(px(&surface, 10, 10), RED_ARGB);
    // Outside clip: should remain white.
    assert_eq!(px(&surface, 0, 0), WHITE_ARGB);
    assert_eq!(px(&surface, 20, 20), WHITE_ARGB);
    assert_eq!(px(&surface, 4, 4), WHITE_ARGB);
}

#[test]
fn draw_image_no_clip() {
    let img = make_red_image(10, 10);
    let mut surface = Surface::new(30, 30);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.draw_image(&img, Rect::new(5.0, 5.0, 10.0, 10.0));
    }
    // Image area should be red.
    assert_eq!(px(&surface, 7, 7), RED_ARGB);
    assert_eq!(px(&surface, 14, 14), RED_ARGB);
    // Outside image: should be white.
    assert_eq!(px(&surface, 0, 0), WHITE_ARGB);
    assert_eq!(px(&surface, 20, 20), WHITE_ARGB);
}

// ── stroke_path width test ─────────────────────────────────────────

#[test]
fn stroke_path_with_width() {
    // Stroke a horizontal line path with width=3 and verify more than 1px of coverage.
    let mut path = BezierPath::new();
    path.move_to(Point::new(5.0, 20.0));
    path.line_to(Point::new(40.0, 20.0));

    let mut surface = Surface::new(50, 50);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.stroke_path(&path, Color::RED, 3.0);
    }
    // The center line y=20 should have red (AA may blend slightly).
    let p = px(&surface, 20, 20);
    let r_ch = (p >> 16) & 0xFF;
    assert!(r_ch > 128, "center should be strongly red, got 0x{p:08X}");
    // At least one pixel above and below the center should also have red,
    // proving the stroke is wider than 1px.
    let p_above = px(&surface, 20, 19);
    let r_above = (p_above >> 16) & 0xFF;
    assert!(
        r_above > 0,
        "pixel above center should have red component, got 0x{p_above:08X}"
    );
    let p_below = px(&surface, 20, 21);
    let r_below = (p_below >> 16) & 0xFF;
    assert!(
        r_below > 0,
        "pixel below center should have red component, got 0x{p_below:08X}"
    );
    // Several pixels away should remain white.
    assert_eq!(px(&surface, 20, 17), WHITE_ARGB);
    assert_eq!(px(&surface, 20, 23), WHITE_ARGB);
}

#[test]
fn stroke_path_width_one_is_thin() {
    // Width=1 should only cover approximately the center line.
    let mut path = BezierPath::new();
    path.move_to(Point::new(5.0, 20.0));
    path.line_to(Point::new(40.0, 20.0));

    let mut surface = Surface::new(50, 50);
    surface.clear(Color::WHITE);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.stroke_path(&path, Color::RED, 1.0);
    }
    // Center line should have red (may be blended due to AA).
    let p = px(&surface, 20, 20);
    let r_ch = (p >> 16) & 0xFF;
    assert!(r_ch > 0, "center should have red component, got 0x{p:08X}");
    // Two pixels away should be white (AA bleeding is at most ~1px).
    assert_eq!(px(&surface, 20, 18), WHITE_ARGB);
    assert_eq!(px(&surface, 20, 22), WHITE_ARGB);
}
