use super::*;
use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::gradient::Gradient;
use crate::path::BezierPath;

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

const BLACK: u32 = 0xFF000000;
const RED: u32 = 0xFFFF0000;
const WHITE: u32 = 0xFFFFFFFF;

// ── fill_path ──────────────────────────────────────────────────────

#[test]
fn fill_path_rect() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let path = BezierPath::rect(10.0, 10.0, 40.0, 40.0);
        r.fill_path(&path, Color::RED);
    }
    // Interior pixel should be red
    assert_eq!(px(&surface, 25, 25), RED);
    assert_eq!(px(&surface, 15, 15), RED);
    // Outside the path should remain black
    assert_eq!(px(&surface, 5, 5), BLACK);
    assert_eq!(px(&surface, 60, 60), BLACK);
}

#[test]
fn fill_path_respects_clip() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // Clip to a small region
        r.push_clip(Rect::new(20.0, 20.0, 20.0, 20.0));
        // Fill a large rect path that extends beyond clip
        let path = BezierPath::rect(0.0, 0.0, 100.0, 100.0);
        r.fill_path(&path, Color::RED);
    }
    // Inside clip: should be red
    assert_eq!(px(&surface, 25, 25), RED);
    assert_eq!(px(&surface, 30, 30), RED);
    // Outside clip: should remain black
    assert_eq!(px(&surface, 5, 5), BLACK);
    assert_eq!(px(&surface, 50, 50), BLACK);
    assert_eq!(px(&surface, 15, 15), BLACK);
}

// ── stroke_path ────────────────────────────────────────────────────

#[test]
fn stroke_path_rect() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let path = BezierPath::rect(10.0, 10.0, 40.0, 40.0);
        r.stroke_path(&path, Color::RED, 1.0);
    }
    // Border pixel on the top edge should have red component (AA may blend).
    let p = px(&surface, 25, 10);
    let r_ch = (p >> 16) & 0xFF;
    assert!(
        r_ch > 0,
        "top-edge pixel should have red component, got 0x{p:08X}"
    );
    // Center pixel should remain black (not filled)
    assert_eq!(px(&surface, 30, 30), BLACK);
}

#[test]
fn stroke_path_width_3() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let path = BezierPath::rect(20.0, 20.0, 60.0, 60.0);
        r.stroke_path(&path, Color::RED, 3.0);
    }
    // The top edge at y=20 should be stroked
    assert_ne!(px(&surface, 50, 20), BLACK);
    // With width=3, one pixel inside (y=21) should also be colored
    assert_ne!(px(&surface, 50, 21), BLACK);
    // Center should remain black
    assert_eq!(px(&surface, 50, 50), BLACK);
}

// ── fill_gradient ──────────────────────────────────────────────────

#[test]
fn fill_gradient_horizontal() {
    let mut surface = Surface::new(100, 10);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let grad = Gradient::linear_two(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Color::BLACK,
            Color::WHITE,
        );
        r.fill_gradient(Rect::new(0.0, 0.0, 100.0, 10.0), &grad);
    }
    let left = px(&surface, 2, 5);
    let right = px(&surface, 97, 5);
    // Left should be near-black (low RGB)
    let left_r = (left >> 16) & 0xFF;
    assert!(left_r < 20, "left pixel red={left_r}, expected near-black");
    // Right should be near-white (high RGB)
    let right_r = (right >> 16) & 0xFF;
    assert!(
        right_r > 230,
        "right pixel red={right_r}, expected near-white"
    );
}

#[test]
fn fill_gradient_vertical() {
    let mut surface = Surface::new(10, 100);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let grad = Gradient::linear_two(
            Point::new(0.0, 0.0),
            Point::new(0.0, 100.0),
            Color::BLACK,
            Color::WHITE,
        );
        r.fill_gradient(Rect::new(0.0, 0.0, 10.0, 100.0), &grad);
    }
    let top = px(&surface, 5, 2);
    let bottom = px(&surface, 5, 97);
    // Top should be near-black
    let top_r = (top >> 16) & 0xFF;
    assert!(top_r < 20, "top pixel red={top_r}, expected near-black");
    // Bottom should be near-white
    let bottom_r = (bottom >> 16) & 0xFF;
    assert!(
        bottom_r > 230,
        "bottom pixel red={bottom_r}, expected near-white"
    );
}

// ── stroke_ellipse ─────────────────────────────────────────────────

#[test]
fn stroke_ellipse_hollow() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.stroke_ellipse(Rect::new(20.0, 20.0, 60.0, 60.0), Color::RED, 1.0);
    }
    // Center of the ellipse should remain black (hollow)
    assert_eq!(px(&surface, 50, 50), BLACK);
    // Top edge of the ellipse (midpoint of top arc ≈ y=20) should be colored
    assert_ne!(px(&surface, 50, 20), BLACK);
}

#[test]
fn stroke_ellipse_respects_clip() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // Clip to right half only
        r.push_clip(Rect::new(50.0, 0.0, 50.0, 100.0));
        r.stroke_ellipse(Rect::new(20.0, 20.0, 60.0, 60.0), Color::RED, 1.0);
    }
    // Left edge of the ellipse (x=20, y=50) should be clipped out
    assert_eq!(px(&surface, 20, 50), BLACK);
    // Some pixels on the right portion should be drawn (search near right edge)
    let right_drawn = (75..=80).any(|x| px(&surface, x, 50) != BLACK);
    assert!(
        right_drawn,
        "right side of ellipse should have drawn pixels"
    );
}
