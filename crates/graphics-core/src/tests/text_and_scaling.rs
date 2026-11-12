use super::*;
use crate::renderer::Renderer as RendererTrait;

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

const BLACK_ARGB: u32 = 0xFF000000;
const RED_ARGB: u32 = 0xFFFF0000;

// ── measure_text ───────────────────────────────────────────────────

#[test]
fn measure_text_non_zero() {
    let mut surface = Surface::new(100, 100);
    let r = SoftwareRenderer::new(&mut surface);
    let font = Font::new("System", 14.0);
    let size = RendererTrait::measure_text(&r, "Hello", &font);
    assert!(
        size.width > 0.0,
        "width should be non-zero for non-empty string"
    );
    assert!(size.height > 0.0, "height should be non-zero");
}

#[test]
fn measure_text_empty() {
    let mut surface = Surface::new(100, 100);
    let r = SoftwareRenderer::new(&mut surface);
    let font = Font::new("System", 14.0);
    let size = RendererTrait::measure_text(&r, "", &font);
    assert_eq!(size.width, 0.0, "empty string should have zero width");
}

#[test]
fn text_measurement_scales_with_font_size() {
    let mut surface = Surface::new(100, 100);
    let r = SoftwareRenderer::new(&mut surface);
    let small = Font::new("System", 10.0);
    let large = Font::new("System", 20.0);
    let small_size = RendererTrait::measure_text(&r, "Hello", &small);
    let large_size = RendererTrait::measure_text(&r, "Hello", &large);
    assert!(
        large_size.width > small_size.width,
        "larger font should produce larger width: {} vs {}",
        large_size.width,
        small_size.width
    );
    assert!(
        large_size.height > small_size.height,
        "larger font should produce larger height: {} vs {}",
        large_size.height,
        small_size.height
    );
}

// ── draw_text_in_rect ──────────────────────────────────────────────

#[test]
fn draw_text_in_rect_wraps() {
    // Use a narrow rect that can't fit "Hello World" on one line (bitmap font).
    // Bitmap font at size 14: each char ~8.4px wide (14*0.6), "Hello" ~ 42px
    let mut surface = Surface::new(200, 200);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let font = Font::new("System", 14.0);
        // Rect is 50px wide — "Hello World" doesn't fit on one line
        r.draw_text_in_rect(
            "Hello World",
            Rect::new(0.0, 0.0, 50.0, 200.0),
            &font,
            Color::RED,
        );
    }
    // First line should have some red pixels in the top area (y < 17)
    let mut first_line_pixels = 0;
    for y in 0..17 {
        for x in 0..50 {
            if px(&surface, x, y) != BLACK_ARGB {
                first_line_pixels += 1;
            }
        }
    }
    // Second line should have some red pixels below (y >= 17)
    let line_height = (14.0 * 1.2) as u32; // ~16
    let mut second_line_pixels = 0;
    for y in line_height..(line_height + 17) {
        for x in 0..50 {
            if px(&surface, x, y) != BLACK_ARGB {
                second_line_pixels += 1;
            }
        }
    }
    assert!(first_line_pixels > 0, "first line should have drawn pixels");
    assert!(
        second_line_pixels > 0,
        "second line should have drawn pixels (text wrapped)"
    );
}

#[test]
fn draw_text_in_rect_clips_bottom() {
    // Rect only tall enough for 1 line; text with 3 words should clip
    let mut surface = Surface::new(200, 200);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let font = Font::new("System", 14.0);
        let line_height = 14.0 * 1.2; // ~16.8
                                      // Only room for 1 line
        r.draw_text_in_rect(
            "AAA BBB CCC",
            Rect::new(0.0, 0.0, 30.0, line_height),
            &font,
            Color::RED,
        );
    }
    // There should be no drawn pixels below line_height + a small margin
    let check_y = (14.0 * 1.2 * 2.0) as u32 + 5; // well into would-be 3rd line
    let mut late_pixels = 0;
    for y in check_y..200 {
        for x in 0..200 {
            if px(&surface, x, y) != BLACK_ARGB {
                late_pixels += 1;
            }
        }
    }
    assert_eq!(late_pixels, 0, "text below rect height should be clipped");
}

// ── clip_to_path ───────────────────────────────────────────────────

#[test]
fn clip_to_path_basic() {
    // Path-based clip degrades to bounding box. Verify pixels outside bbox untouched.
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        let path = BezierPath::circle(50.0, 50.0, 20.0);
        // bbox of circle(50,50,r=20): ~(30,30) to (70,70)
        r.clip_to_path(&path);
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
    }
    // Inside bounding box should be red
    assert_eq!(px(&surface, 50, 50), RED_ARGB);
    // Outside bounding box should remain black
    assert_eq!(px(&surface, 5, 5), BLACK_ARGB);
    assert_eq!(px(&surface, 95, 95), BLACK_ARGB);
    assert_eq!(px(&surface, 0, 50), BLACK_ARGB);
    assert_eq!(px(&surface, 99, 50), BLACK_ARGB);
}

// ── bilinear image scaling ─────────────────────────────────────────

#[test]
fn bilinear_sample_identity() {
    // 1:1 scale: output should match input exactly
    let mut img = ImageBuffer::new(4, 4);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let v = (((x + y * 4) * 20) & 0xFF) as f64 / 255.0;
            img.set_pixel(x, y, Color::new(v, v, v, 1.0));
        }
    }
    let mut surface = Surface::new(4, 4);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.draw_image(&img, Rect::new(0.0, 0.0, 4.0, 4.0));
    }
    for y in 0..4u32 {
        for x in 0..4u32 {
            let expected = img.argb32_at(x, y);
            let got = px(&surface, x, y);
            // Allow small rounding difference
            let diff_r =
                (((expected >> 16) & 0xFF) as i32 - ((got >> 16) & 0xFF) as i32).unsigned_abs();
            let diff_g =
                (((expected >> 8) & 0xFF) as i32 - ((got >> 8) & 0xFF) as i32).unsigned_abs();
            let diff_b = ((expected & 0xFF) as i32 - (got & 0xFF) as i32).unsigned_abs();
            assert!(
                diff_r <= 2 && diff_g <= 2 && diff_b <= 2,
                "pixel ({x},{y}): expected {expected:#010X}, got {got:#010X}"
            );
        }
    }
}

#[test]
fn bilinear_sample_upscale() {
    // 2×2 gradient → 4×4. Bilinear should produce smooth intermediate values.
    let mut img = ImageBuffer::new(2, 2);
    img.set_pixel(0, 0, Color::new(0.0, 0.0, 0.0, 1.0)); // black
    img.set_pixel(1, 0, Color::new(1.0, 0.0, 0.0, 1.0)); // red
    img.set_pixel(0, 1, Color::new(0.0, 1.0, 0.0, 1.0)); // green
    img.set_pixel(1, 1, Color::new(1.0, 1.0, 0.0, 1.0)); // yellow

    let mut surface = Surface::new(4, 4);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.draw_image(&img, Rect::new(0.0, 0.0, 4.0, 4.0));
    }
    // The center-ish pixel (2,2) should have contributions from all four corners —
    // i.e., it should not be pure black, red, green, or yellow
    let center = px(&surface, 2, 2);
    let cr = (center >> 16) & 0xFF;
    let cg = (center >> 8) & 0xFF;
    // With bilinear interpolation, center should have intermediate R and G values
    assert!(
        cr > 0 && cr < 255,
        "red channel should be interpolated: {cr}"
    );
    assert!(
        cg > 0 && cg < 255,
        "green channel should be interpolated: {cg}"
    );
}
