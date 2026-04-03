use super::*;

fn pixel_at(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

fn pixel_matches_approx(pixel: u32, expected: u32, tolerance: u32) -> bool {
    let r_diff = ((pixel >> 16) & 0xFF).abs_diff((expected >> 16) & 0xFF);
    let g_diff = ((pixel >> 8) & 0xFF).abs_diff((expected >> 8) & 0xFF);
    let b_diff = (pixel & 0xFF).abs_diff(expected & 0xFF);
    r_diff <= tolerance && g_diff <= tolerance && b_diff <= tolerance
}

// ── draw_line ──────────────────────────────────────────────────────

#[test]
fn draw_line_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.draw_line(Point::new(5.0, 5.0), Point::new(50.0, 5.0), Color::RED, 1.0);
    }
    // Several pixels along the horizontal line should be non-zero
    for x in [5, 15, 25, 35, 50] {
        assert_ne!(pixel_at(&s, x, 5), 0, "pixel at ({x}, 5) should be drawn");
    }
    // A pixel well away from the line should still be zero
    assert_eq!(pixel_at(&s, 50, 50), 0);
}

#[test]
fn draw_line_diagonal() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.draw_line(Point::new(0.0, 0.0), Point::new(30.0, 30.0), Color::GREEN, 1.0);
    }
    // Bresenham diagonal: pixels along y==x should be set
    for i in [0, 10, 20, 30] {
        assert_ne!(pixel_at(&s, i, i), 0, "pixel at ({i}, {i}) should be drawn");
    }
}

// ── draw_image ─────────────────────────────────────────────────────

#[test]
fn draw_image_basic() {
    let mut img = ImageBuffer::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            img.set_pixel(x, y, Color::RED);
        }
    }

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.draw_image(&img, Rect::new(10.0, 10.0, 20.0, 20.0));
    }
    // Pixels inside the dest rect should be red
    let red = Color::RED.to_argb32();
    assert!(pixel_matches_approx(pixel_at(&s, 15, 15), red, 2));
    assert!(pixel_matches_approx(pixel_at(&s, 20, 20), red, 2));
    // Pixel outside should be zero
    assert_eq!(pixel_at(&s, 5, 5), 0);
}

#[test]
fn draw_image_at_basic() {
    let mut img = ImageBuffer::new(2, 2);
    img.set_pixel(0, 0, Color::BLUE);
    img.set_pixel(1, 0, Color::BLUE);
    img.set_pixel(0, 1, Color::BLUE);
    img.set_pixel(1, 1, Color::BLUE);

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.draw_image_at(&img, Point::new(5.0, 5.0));
    }
    let blue = Color::BLUE.to_argb32();
    assert!(pixel_matches_approx(pixel_at(&s, 5, 5), blue, 2));
    assert!(pixel_matches_approx(pixel_at(&s, 6, 6), blue, 2));
}

#[test]
fn draw_image_scaling() {
    // 2x2 red image scaled to 20x20 dest
    let mut img = ImageBuffer::new(2, 2);
    for y in 0..2 {
        for x in 0..2 {
            img.set_pixel(x, y, Color::RED);
        }
    }

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.draw_image(&img, Rect::new(10.0, 10.0, 20.0, 20.0));
    }
    let red = Color::RED.to_argb32();
    // Pixels across the scaled area should be red
    assert!(pixel_matches_approx(pixel_at(&s, 12, 12), red, 2));
    assert!(pixel_matches_approx(pixel_at(&s, 25, 25), red, 2));
    // Outside region should be untouched
    assert_eq!(pixel_at(&s, 5, 5), 0);
}

// ── fill_rounded_rect ──────────────────────────────────────────────

#[test]
fn fill_rounded_rect_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_rounded_rect(Rect::new(10.0, 10.0, 80.0, 40.0), 8.0, Color::RED);
    }
    // Center should be filled
    assert_ne!(pixel_at(&s, 50, 30), 0, "center should be filled");
    // Extreme corner of the bounding box (10,10) should be empty because of rounding
    assert_eq!(pixel_at(&s, 10, 10), 0, "top-left corner should be empty due to rounding");
}

// ── stroke_rounded_rect ────────────────────────────────────────────

#[test]
fn stroke_rounded_rect_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.stroke_rounded_rect(Rect::new(10.0, 10.0, 80.0, 40.0), 8.0, Color::RED, 1.0);
    }
    // Top edge middle should have a pixel from the stroke
    let top_mid = pixel_at(&s, 50, 10);
    assert_ne!(top_mid, 0, "top border should have pixels");
    // Interior center should be empty (not filled, only stroked)
    assert_eq!(pixel_at(&s, 50, 30), 0, "interior should be empty for stroke");
}

// ── fill_ellipse ───────────────────────────────────────────────────

#[test]
fn fill_ellipse_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_ellipse(Rect::new(10.0, 10.0, 80.0, 40.0), Color::GREEN);
    }
    // Center of the ellipse should be filled
    assert_ne!(pixel_at(&s, 50, 30), 0, "ellipse center should be filled");
}

// ── stroke_ellipse ─────────────────────────────────────────────────

#[test]
fn stroke_ellipse_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.stroke_ellipse(Rect::new(10.0, 10.0, 80.0, 40.0), Color::GREEN, 1.0);
    }
    // Top-center of the ellipse outline: pixel at (50, 10) should be drawn
    assert_ne!(pixel_at(&s, 50, 10), 0, "top of ellipse outline should have pixels");
    // Center should be empty (stroke only)
    assert_eq!(pixel_at(&s, 50, 30), 0, "interior should be empty for stroke");
}

// ── fill_circle ────────────────────────────────────────────────────

#[test]
fn fill_circle_basic() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_circle(Point::new(50.0, 50.0), 20.0, Color::BLUE);
    }
    // Center should be filled
    assert_ne!(pixel_at(&s, 50, 50), 0, "circle center should be filled");
    // Corners of bounding box (30,30)-(70,70) should be empty (circle doesn't reach)
    assert_eq!(pixel_at(&s, 30, 30), 0, "top-left corner of bbox should be empty");
    assert_eq!(pixel_at(&s, 70, 70), 0, "bottom-right corner of bbox should be empty");
}

// ── fill_path ──────────────────────────────────────────────────────

#[test]
fn fill_path_triangle() {
    let mut path = BezierPath::new();
    path.move_to(Point::new(50.0, 10.0));
    path.line_to(Point::new(90.0, 90.0));
    path.line_to(Point::new(10.0, 90.0));
    path.close();

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_path(&path, Color::RED);
    }
    // Centroid area should be filled
    assert_ne!(pixel_at(&s, 50, 60), 0, "triangle interior should be filled");
    // Well outside the triangle should be empty
    assert_eq!(pixel_at(&s, 5, 5), 0, "outside triangle should be empty");
}

#[test]
fn fill_path_rect() {
    let path = BezierPath::rect(10.0, 10.0, 50.0, 50.0);

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_path(&path, Color::GREEN);
    }
    assert_ne!(pixel_at(&s, 30, 30), 0, "rect path interior should be filled");
    assert_eq!(pixel_at(&s, 5, 5), 0, "outside rect path should be empty");
}

// ── stroke_path ────────────────────────────────────────────────────

#[test]
fn stroke_path_triangle() {
    let mut path = BezierPath::new();
    path.move_to(Point::new(50.0, 10.0));
    path.line_to(Point::new(90.0, 90.0));
    path.line_to(Point::new(10.0, 90.0));
    path.close();

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.stroke_path(&path, Color::RED, 1.0);
    }
    // Bottom edge: near (50, 90) should have stroke pixels
    assert_ne!(pixel_at(&s, 50, 90), 0, "triangle bottom edge should have stroke");
    // Interior should be empty (stroke only)
    assert_eq!(pixel_at(&s, 50, 60), 0, "triangle interior should be empty for stroke");
}

// ── fill_gradient ──────────────────────────────────────────────────

#[test]
fn fill_gradient_linear() {
    let gradient = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::RED,
        Color::BLUE,
    );

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_gradient(Rect::new(0.0, 0.0, 100.0, 100.0), &gradient);
    }
    let left = pixel_at(&s, 2, 50);
    let right = pixel_at(&s, 97, 50);
    // Left side should be red-ish (high red channel)
    assert!(
        pixel_matches_approx(left, Color::RED.to_argb32(), 30),
        "left side should be red-ish, got 0x{left:08X}"
    );
    // Right side should be blue-ish (high blue channel)
    assert!(
        pixel_matches_approx(right, Color::BLUE.to_argb32(), 30),
        "right side should be blue-ish, got 0x{right:08X}"
    );
}

#[test]
fn fill_gradient_radial() {
    let gradient = Gradient::radial_two(
        Point::new(50.0, 50.0),
        50.0,
        Color::WHITE,
        Color::BLACK,
    );

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_gradient(Rect::new(0.0, 0.0, 100.0, 100.0), &gradient);
    }
    let center = pixel_at(&s, 50, 50);
    let edge = pixel_at(&s, 1, 50);
    // Center should be close to white
    assert!(
        pixel_matches_approx(center, Color::WHITE.to_argb32(), 30),
        "center should be white-ish, got 0x{center:08X}"
    );
    // Edge should be close to black
    assert!(
        pixel_matches_approx(edge, Color::BLACK.to_argb32(), 30),
        "edge should be black-ish, got 0x{edge:08X}"
    );
}

// ── with_shadow ────────────────────────────────────────────────────

#[test]
fn with_shadow_runs_draw_fn() {
    let mut s = Surface::new(100, 100);
    let shadow = Shadow::standard();
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.with_shadow(&shadow, |r| {
            r.fill_rect(Rect::new(20.0, 20.0, 30.0, 30.0), Color::RED);
        });
    }
    // The draw function's content should appear regardless of shadow rendering
    let red = Color::RED.to_argb32();
    assert!(
        pixel_matches_approx(pixel_at(&s, 35, 35), red, 2),
        "draw_fn content should be rendered"
    );
}

// ── complex path tests ─────────────────────────────────────────────

#[test]
fn fill_path_complex_quad() {
    let mut path = BezierPath::new();
    path.move_to(Point::new(10.0, 50.0));
    path.quad_to(Point::new(50.0, 10.0), Point::new(90.0, 50.0));
    path.line_to(Point::new(90.0, 90.0));
    path.line_to(Point::new(10.0, 90.0));
    path.close();

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_path(&path, Color::BLUE);
    }
    // Lower interior should be filled
    assert_ne!(pixel_at(&s, 50, 70), 0, "path with quad_to should fill interior");
}

#[test]
fn fill_path_circle_shape() {
    let path = BezierPath::circle(50.0, 50.0, 20.0);

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_path(&path, Color::GREEN);
    }
    let center = pixel_at(&s, 50, 50);
    let green = Color::GREEN.to_argb32();
    assert!(
        pixel_matches_approx(center, green, 2),
        "circle center should be green, got 0x{center:08X}"
    );
}

// ── shape geometry verification ────────────────────────────────────

#[test]
fn fill_ellipse_produces_round_shape() {
    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        r.fill_ellipse(Rect::new(10.0, 10.0, 80.0, 40.0), Color::RED);
    }
    // The corner of the bounding box should NOT be filled (ellipse is round)
    assert_eq!(pixel_at(&s, 10, 10), 0, "top-left corner should be empty");
    assert_eq!(pixel_at(&s, 89, 49), 0, "bottom-right corner should be empty");
    // But center should be filled
    assert_ne!(pixel_at(&s, 50, 30), 0, "center should be filled");
}

// ── alpha blending ─────────────────────────────────────────────────

#[test]
fn draw_image_alpha_blend() {
    // Create a semi-transparent green image
    let mut img = ImageBuffer::new(10, 10);
    let semi_green = Color::new(0.0, 1.0, 0.0, 0.5);
    for y in 0..10 {
        for x in 0..10 {
            img.set_pixel(x, y, semi_green);
        }
    }

    let mut s = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut s);
        // Fill background with red
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
        // Overlay semi-transparent green
        r.draw_image(&img, Rect::new(10.0, 10.0, 10.0, 10.0));
    }
    let blended = pixel_at(&s, 15, 15);
    // Should be a blend of red and green — not pure red and not pure green
    let r_chan = (blended >> 16) & 0xFF;
    let g_chan = (blended >> 8) & 0xFF;
    assert!(
        r_chan > 0 && r_chan < 255 && g_chan > 0,
        "pixel should be blended (r={r_chan}, g={g_chan}), not pure red or green"
    );
}
