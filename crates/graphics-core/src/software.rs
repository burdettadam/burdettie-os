use super::color::Color;
use super::font::Font;
use super::font_manager::FontManager;
use super::geometry::{Point, Rect};
use super::renderer::Renderer;
use super::surface::Surface;
use super::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

use ab_glyph::{Font as AbFont, ScaleFont};

// ── Software renderer ──────────────────────────────────────────────

// 5×7 bitmap font for printable ASCII (32–126).
// Each glyph is 7 rows; each row has 5 bits (bit 4 = leftmost pixel).
#[rustfmt::skip]
const FONT_5X7: [[u8; 7]; 95] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 32 ' '
    [0x04,0x04,0x04,0x04,0x04,0x00,0x04], // 33 '!'
    [0x0A,0x0A,0x0A,0x00,0x00,0x00,0x00], // 34 '"'
    [0x0A,0x0A,0x1F,0x0A,0x1F,0x0A,0x0A], // 35 '#'
    [0x04,0x0F,0x14,0x0E,0x05,0x1E,0x04], // 36 '$'
    [0x18,0x19,0x02,0x04,0x08,0x13,0x03], // 37 '%'
    [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D], // 38 '&'
    [0x06,0x04,0x08,0x00,0x00,0x00,0x00], // 39 "'"
    [0x02,0x04,0x08,0x08,0x08,0x04,0x02], // 40 '('
    [0x08,0x04,0x02,0x02,0x02,0x04,0x08], // 41 ')'
    [0x00,0x04,0x15,0x0E,0x15,0x04,0x00], // 42 '*'
    [0x00,0x04,0x04,0x1F,0x04,0x04,0x00], // 43 '+'
    [0x00,0x00,0x00,0x00,0x06,0x04,0x08], // 44 ','
    [0x00,0x00,0x00,0x1F,0x00,0x00,0x00], // 45 '-'
    [0x00,0x00,0x00,0x00,0x00,0x06,0x06], // 46 '.'
    [0x00,0x01,0x02,0x04,0x08,0x10,0x00], // 47 '/'
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E], // 48 '0'
    [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E], // 49 '1'
    [0x0E,0x11,0x01,0x02,0x04,0x08,0x1F], // 50 '2'
    [0x1F,0x02,0x04,0x02,0x01,0x11,0x0E], // 51 '3'
    [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02], // 52 '4'
    [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E], // 53 '5'
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E], // 54 '6'
    [0x1F,0x01,0x02,0x04,0x08,0x08,0x08], // 55 '7'
    [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E], // 56 '8'
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C], // 57 '9'
    [0x00,0x06,0x06,0x00,0x06,0x06,0x00], // 58 ':'
    [0x00,0x06,0x06,0x00,0x06,0x04,0x08], // 59 ';'
    [0x02,0x04,0x08,0x10,0x08,0x04,0x02], // 60 '<'
    [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00], // 61 '='
    [0x08,0x04,0x02,0x01,0x02,0x04,0x08], // 62 '>'
    [0x0E,0x11,0x01,0x02,0x04,0x00,0x04], // 63 '?'
    [0x0E,0x11,0x17,0x15,0x17,0x10,0x0E], // 64 '@'
    [0x0E,0x11,0x11,0x1F,0x11,0x11,0x11], // 65 'A'
    [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E], // 66 'B'
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E], // 67 'C'
    [0x1C,0x12,0x11,0x11,0x11,0x12,0x1C], // 68 'D'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F], // 69 'E'
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10], // 70 'F'
    [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F], // 71 'G'
    [0x11,0x11,0x11,0x1F,0x11,0x11,0x11], // 72 'H'
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E], // 73 'I'
    [0x07,0x02,0x02,0x02,0x02,0x12,0x0C], // 74 'J'
    [0x11,0x12,0x14,0x18,0x14,0x12,0x11], // 75 'K'
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F], // 76 'L'
    [0x11,0x1B,0x15,0x15,0x11,0x11,0x11], // 77 'M'
    [0x11,0x11,0x19,0x15,0x13,0x11,0x11], // 78 'N'
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E], // 79 'O'
    [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10], // 80 'P'
    [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D], // 81 'Q'
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11], // 82 'R'
    [0x0E,0x11,0x10,0x0E,0x01,0x11,0x0E], // 83 'S'
    [0x1F,0x04,0x04,0x04,0x04,0x04,0x04], // 84 'T'
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E], // 85 'U'
    [0x11,0x11,0x11,0x11,0x11,0x0A,0x04], // 86 'V'
    [0x11,0x11,0x11,0x15,0x15,0x15,0x0A], // 87 'W'
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11], // 88 'X'
    [0x11,0x11,0x0A,0x04,0x04,0x04,0x04], // 89 'Y'
    [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F], // 90 'Z'
    [0x0E,0x08,0x08,0x08,0x08,0x08,0x0E], // 91 '['
    [0x00,0x10,0x08,0x04,0x02,0x01,0x00], // 92 '\'
    [0x0E,0x02,0x02,0x02,0x02,0x02,0x0E], // 93 ']'
    [0x04,0x0A,0x11,0x00,0x00,0x00,0x00], // 94 '^'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x1F], // 95 '_'
    [0x08,0x04,0x02,0x00,0x00,0x00,0x00], // 96 '`'
    [0x00,0x00,0x0E,0x01,0x0F,0x11,0x0F], // 97 'a'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x1E], // 98 'b'
    [0x00,0x00,0x0E,0x10,0x10,0x11,0x0E], // 99 'c'
    [0x01,0x01,0x0D,0x13,0x11,0x11,0x0F], // 100 'd'
    [0x00,0x00,0x0E,0x11,0x1F,0x10,0x0E], // 101 'e'
    [0x06,0x09,0x08,0x1C,0x08,0x08,0x08], // 102 'f'
    [0x00,0x00,0x0F,0x11,0x0F,0x01,0x0E], // 103 'g'
    [0x10,0x10,0x16,0x19,0x11,0x11,0x11], // 104 'h'
    [0x04,0x00,0x0C,0x04,0x04,0x04,0x0E], // 105 'i'
    [0x02,0x00,0x06,0x02,0x02,0x12,0x0C], // 106 'j'
    [0x10,0x10,0x12,0x14,0x18,0x14,0x12], // 107 'k'
    [0x0C,0x04,0x04,0x04,0x04,0x04,0x0E], // 108 'l'
    [0x00,0x00,0x1A,0x15,0x15,0x11,0x11], // 109 'm'
    [0x00,0x00,0x16,0x19,0x11,0x11,0x11], // 110 'n'
    [0x00,0x00,0x0E,0x11,0x11,0x11,0x0E], // 111 'o'
    [0x00,0x00,0x1E,0x11,0x1E,0x10,0x10], // 112 'p'
    [0x00,0x00,0x0D,0x13,0x0F,0x01,0x01], // 113 'q'
    [0x00,0x00,0x16,0x19,0x10,0x10,0x10], // 114 'r'
    [0x00,0x00,0x0E,0x10,0x0E,0x01,0x1E], // 115 's'
    [0x08,0x08,0x1C,0x08,0x08,0x09,0x06], // 116 't'
    [0x00,0x00,0x11,0x11,0x11,0x13,0x0D], // 117 'u'
    [0x00,0x00,0x11,0x11,0x11,0x0A,0x04], // 118 'v'
    [0x00,0x00,0x11,0x11,0x15,0x15,0x0A], // 119 'w'
    [0x00,0x00,0x11,0x0A,0x04,0x0A,0x11], // 120 'x'
    [0x00,0x00,0x11,0x11,0x0F,0x01,0x0E], // 121 'y'
    [0x00,0x00,0x1F,0x02,0x04,0x08,0x1F], // 122 'z'
    [0x02,0x04,0x04,0x08,0x04,0x04,0x02], // 123 '{'
    [0x04,0x04,0x04,0x04,0x04,0x04,0x04], // 124 '|'
    [0x08,0x04,0x04,0x02,0x04,0x04,0x08], // 125 '}'
    [0x00,0x00,0x0D,0x12,0x00,0x00,0x00], // 126 '~'
];

/// Look up a glyph for a character, returning a filled rectangle for unknowns.
fn glyph_for(ch: char) -> &'static [u8; 7] {
    let idx = ch as u32;
    if (32..=126).contains(&idx) {
        &FONT_5X7[(idx - 32) as usize]
    } else {
        // Unknown character → filled block
        static BLOCK: [u8; 7] = [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F];
        &BLOCK
    }
}

/// A basic software renderer that draws into a `Surface`.
///
/// When constructed with [`SoftwareRenderer::with_fonts`], text is rendered
/// using TrueType glyph rasterization via `ab_glyph`. When constructed with
/// [`SoftwareRenderer::new`], the legacy 5×7 bitmap font is used.
pub struct SoftwareRenderer<'a> {
    surface: &'a mut Surface,
    font_manager: Option<&'a FontManager>,
}

impl<'a> SoftwareRenderer<'a> {
    /// Create a renderer using the legacy 5×7 bitmap font.
    pub fn new(surface: &'a mut Surface) -> Self {
        Self {
            surface,
            font_manager: None,
        }
    }

    /// Create a renderer with TrueType font support.
    pub fn with_fonts(surface: &'a mut Surface, fonts: &'a FontManager) -> Self {
        Self {
            surface,
            font_manager: Some(fonts),
        }
    }

    fn fill_pixel_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(sw);
        let y1 = (y + h).min(sh);

        for py in y0..y1 {
            for px in x0..x1 {
                self.surface.set_pixel(px as u32, py as u32, color);
            }
        }
    }
}

impl Renderer for SoftwareRenderer<'_> {
    fn clear(&mut self, color: Color) {
        self.surface.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let c = color.to_argb32();
        self.fill_pixel_rect(
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
            c,
        );
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        let c = color.to_argb32();
        let w = width as i32;
        let x = rect.x as i32;
        let y = rect.y as i32;
        let rw = rect.width as i32;
        let rh = rect.height as i32;

        // Top
        self.fill_pixel_rect(x, y, rw, w, c);
        // Bottom
        self.fill_pixel_rect(x, y + rh - w, rw, w, c);
        // Left
        self.fill_pixel_rect(x, y, w, rh, c);
        // Right
        self.fill_pixel_rect(x + rw - w, y, w, rh, c);
    }

    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        if let Some(fm) = self.font_manager {
            self.draw_text_truetype(text, origin, font, color, fm);
        } else {
            self.draw_text_bitmap(text, origin, font, color);
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, _width: f64) {
        // Bresenham's line algorithm
        let c = color.to_argb32();
        let mut x0 = from.x as i32;
        let mut y0 = from.y as i32;
        let x1 = to.x as i32;
        let y1 = to.y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.surface.set_pixel(x0 as u32, y0 as u32, c);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

// ── Text rendering implementations ─────────────────────────────────

impl SoftwareRenderer<'_> {
    /// Legacy 5×7 bitmap font rendering (used when no FontManager is set).
    fn draw_text_bitmap(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        let c = color.to_argb32();
        let scale = (font.size / 7.0).max(1.0).round() as i32;
        let char_w = 5 * scale;
        let spacing = char_w + scale; // 1-pixel gap scaled

        for (i, ch) in text.chars().enumerate() {
            let glyph = glyph_for(ch);
            let bx = origin.x as i32 + (i as i32) * spacing;
            let by = origin.y as i32;
            for (row, &bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        let px = bx + col * scale;
                        let py = by + (row as i32) * scale;
                        self.fill_pixel_rect(px, py, scale, scale, c);
                    }
                }
            }
        }
    }

    /// TrueType glyph rasterization via ab_glyph.
    ///
    /// Handles NFC normalization, combining marks (overlay on base character),
    /// zero-width chars (skipped), and missing glyphs (emoji placeholder box).
    fn draw_text_truetype(
        &mut self,
        text: &str,
        origin: Point,
        font: &Font,
        color: Color,
        fm: &FontManager,
    ) {
        let normalized = normalize_nfc(text);
        let face = fm.get(&font.name);
        let ab_font = face.inner();
        let size = font.size as f32;
        let scaled = ab_font.as_scaled(size);
        let ascent = scaled.ascent();

        let cr = (color.r.clamp(0.0, 1.0) * 255.0) as u32;
        let cg = (color.g.clamp(0.0, 1.0) * 255.0) as u32;
        let cb = (color.b.clamp(0.0, 1.0) * 255.0) as u32;
        let ca = (color.a.clamp(0.0, 1.0) * 255.0) as u32;

        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        let mut cursor_x = origin.x as f32;
        let baseline_y = origin.y as f32 + ascent;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
        let mut last_base_x = cursor_x;

        // Precompute placeholder box size for missing glyphs
        let placeholder_w = (size * 0.6) as i32;
        let placeholder_h = (ascent * 0.9) as i32;

        for ch in normalized.chars() {
            // Skip zero-width formatting characters
            if is_zero_width(ch) {
                continue;
            }

            let glyph_id = ab_font.glyph_id(ch);

            // Combining marks: position at same x as last base, no advance
            if is_combining_mark(ch) {
                let glyph = glyph_id.with_scale_and_position(
                    size,
                    ab_glyph::point(last_base_x, baseline_y),
                );
                if let Some(outlined) = ab_font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|rx, ry, coverage| {
                        self.blend_pixel(
                            bounds.min.x as i32 + rx as i32,
                            bounds.min.y as i32 + ry as i32,
                            coverage, cr, cg, cb, ca, sw, sh,
                        );
                    });
                }
                continue;
            }

            // Kerning for base characters
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }

            last_base_x = cursor_x;

            // Missing glyph → draw placeholder rectangle
            if glyph_id == ab_glyph::GlyphId(0) {
                let px = cursor_x as i32;
                let py = (baseline_y - ascent * 0.85) as i32;
                self.draw_placeholder_box(px, py, placeholder_w, placeholder_h, color);
                cursor_x += placeholder_w as f32 + 2.0;
                prev_glyph = None;
                continue;
            }

            let glyph = glyph_id.with_scale_and_position(
                size,
                ab_glyph::point(cursor_x, baseline_y),
            );

            // Rasterize and blend
            if let Some(outlined) = ab_font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|rx, ry, coverage| {
                    self.blend_pixel(
                        bounds.min.x as i32 + rx as i32,
                        bounds.min.y as i32 + ry as i32,
                        coverage, cr, cg, cb, ca, sw, sh,
                    );
                });
            }

            cursor_x += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }
    }

    /// Alpha-blend a single pixel at (px, py).
    fn blend_pixel(
        &mut self,
        px: i32, py: i32,
        coverage: f32,
        cr: u32, cg: u32, cb: u32, ca: u32,
        sw: i32, sh: i32,
    ) {
        if coverage <= 0.0 || px < 0 || py < 0 || px >= sw || py >= sh {
            return;
        }
        let idx = (py * sw + px) as usize;
        let pixels = self.surface.pixels_mut();
        let alpha = (ca as f32 * coverage).round() as u32;
        if alpha == 0 {
            return;
        }
        let dst = pixels[idx];
        if alpha >= 255 {
            pixels[idx] = (255 << 24) | (cr << 16) | (cg << 8) | cb;
        } else {
            let inv = 255 - alpha;
            let dr = (dst >> 16) & 0xFF;
            let dg = (dst >> 8) & 0xFF;
            let db = dst & 0xFF;
            let da = (dst >> 24) & 0xFF;
            let or = (cr * alpha + dr * inv) / 255;
            let og = (cg * alpha + dg * inv) / 255;
            let ob = (cb * alpha + db * inv) / 255;
            let oa = alpha + (da * inv) / 255;
            pixels[idx] = (oa << 24) | (or << 16) | (og << 8) | ob;
        }
    }

    /// Draw a placeholder rectangle for a missing glyph (tofu box).
    fn draw_placeholder_box(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        let c = color.to_argb32();
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;
        // Draw top and bottom edges
        for dx in 0..w {
            let px = x + dx;
            if px >= 0 && px < sw {
                if y >= 0 && y < sh {
                    self.surface.pixels_mut()[(y * sw + px) as usize] = c;
                }
                let by = y + h - 1;
                if by >= 0 && by < sh {
                    self.surface.pixels_mut()[(by * sw + px) as usize] = c;
                }
            }
        }
        // Draw left and right edges
        for dy in 0..h {
            let py = y + dy;
            if py >= 0 && py < sh {
                if x >= 0 && x < sw {
                    self.surface.pixels_mut()[(py * sw + x) as usize] = c;
                }
                let rx = x + w - 1;
                if rx >= 0 && rx < sw {
                    self.surface.pixels_mut()[(py * sw + rx) as usize] = c;
                }
            }
        }
    }

    /// Measure text width using TrueType metrics (if font manager available)
    /// or bitmap metrics (fallback). NFC-normalizes the input.
    pub fn measure_text(&self, text: &str, font: &Font) -> f64 {
        if let Some(fm) = self.font_manager {
            fm.get(&font.name).measure_text(text, font.size)
        } else {
            let scale = (font.size / 7.0).max(1.0).round();
            let spacing = 5.0 * scale + scale;
            text.chars().count() as f64 * spacing
        }
    }
}
