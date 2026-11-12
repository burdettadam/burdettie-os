use crate::color::Color;
use crate::geometry::Rect;
use crate::renderer::Renderer;
use crate::software::SoftwareRenderer;
use crate::surface::Surface;

const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

fn red_argb() -> u32 {
    RED.to_argb32()
}

fn white_argb() -> u32 {
    WHITE.to_argb32()
}

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

#[test]
fn translate_moves_drawing() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.translate(20.0, 10.0);
    r.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), RED);
    drop(r);

    // Pixel at (20,10) should be RED
    assert_eq!(px(&surface, 20, 10), red_argb());
    // Origin should remain WHITE
    assert_eq!(px(&surface, 0, 0), white_argb());
}

#[test]
fn translate_accumulates() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.translate(10.0, 0.0);
    r.translate(10.0, 0.0);
    r.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), RED);
    drop(r);

    // Should be at (20,0)
    assert_eq!(px(&surface, 20, 0), red_argb());
    // (10,0) should still be background
    assert_eq!(px(&surface, 10, 0), white_argb());
    // (0,0) should still be background
    assert_eq!(px(&surface, 0, 0), white_argb());
}

#[test]
fn scale_enlarges() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.scale(2.0, 2.0);
    r.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), RED);
    drop(r);

    // 20x20 area should be filled
    assert_eq!(px(&surface, 0, 0), red_argb());
    assert_eq!(px(&surface, 19, 19), red_argb());
    // Just outside should be background
    assert_eq!(px(&surface, 20, 0), white_argb());
    assert_eq!(px(&surface, 0, 20), white_argb());
}

#[test]
fn save_restore_translate() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);

    r.translate(20.0, 0.0);
    r.save();
    r.translate(20.0, 0.0);
    // First fill at effective (40, 0)
    r.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), RED);
    r.restore();
    // After restore, translate is back to (20, 0)
    r.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), RED);
    drop(r);

    assert_eq!(px(&surface, 40, 0), red_argb());
    assert_eq!(px(&surface, 20, 0), red_argb());
    assert_eq!(px(&surface, 0, 0), white_argb());
}

#[test]
fn save_restore_scale() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);

    r.save();
    r.scale(3.0, 3.0);
    r.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), RED);
    r.restore();
    // After restore, scale is back to 1.0
    // Fill a 5x5 rect at (50, 0) — should be exactly 5x5
    r.fill_rect(Rect::new(50.0, 0.0, 5.0, 5.0), RED);
    drop(r);

    // Scaled fill: 15x15 area
    assert_eq!(px(&surface, 14, 14), red_argb());
    // Un-scaled fill at (50,0): 5x5
    assert_eq!(px(&surface, 50, 0), red_argb());
    assert_eq!(px(&surface, 54, 4), red_argb());
    assert_eq!(px(&surface, 55, 0), white_argb());
}

#[test]
fn save_restore_clips() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);

    r.save();
    r.push_clip(Rect::new(10.0, 10.0, 20.0, 20.0));
    // Clip should restrict drawing
    r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), RED);
    r.restore();
    // After restore, clip is removed — (0,0) should still be background
    // because the fill happened while clipped, and post-restore the clip is gone
    drop(r);

    // Inside clip region should be RED
    assert_eq!(px(&surface, 15, 15), red_argb());
    // Outside clip region should remain WHITE
    assert_eq!(px(&surface, 0, 0), white_argb());
    assert_eq!(px(&surface, 50, 50), white_argb());
}

#[test]
fn identity_transform_no_change() {
    let mut surface = Surface::new(100, 100);
    surface.clear(WHITE);
    let mut r = SoftwareRenderer::new(&mut surface);
    // No transform applied — draw at original coordinates
    r.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), RED);
    drop(r);

    assert_eq!(px(&surface, 5, 5), red_argb());
    assert_eq!(px(&surface, 14, 14), red_argb());
    assert_eq!(px(&surface, 4, 4), white_argb());
    assert_eq!(px(&surface, 15, 5), white_argb());
}
