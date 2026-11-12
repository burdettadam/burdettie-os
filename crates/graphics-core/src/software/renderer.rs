//! SoftwareRenderer struct, constructors, helper methods, and text metrics.

use ab_glyph::{Font as AbFont, ScaleFont};

use crate::color::Color;
use crate::font::Font;
use crate::font_manager::FontManager;
use crate::geometry::{Point, Rect};
use crate::renderer::{LineCap, LineJoin};
use crate::surface::Surface;
use crate::unicode::{is_combining_mark, is_zero_width, normalize_nfc};

use super::font::glyph_for;

/// Graphics state that can be saved/restored.
#[derive(Clone)]
pub(super) struct GraphicsState {
    pub(super) translate_x: f64,
    pub(super) translate_y: f64,
    pub(super) scale_x: f64,
    pub(super) scale_y: f64,
    pub(super) rotation: f64,
    pub(super) clip_stack_depth: usize,
    pub(super) line_join: LineJoin,
    pub(super) line_cap: LineCap,
    pub(super) opacity: f64,
}

/// A basic software renderer that draws into a `Surface`.
///
/// When constructed with [`SoftwareRenderer::with_fonts`], text is rendered
/// using TrueType glyph rasterization via `ab_glyph`. When constructed with
/// [`SoftwareRenderer::new`], the legacy 5×7 bitmap font is used.
pub struct SoftwareRenderer<'a> {
    pub(super) surface: &'a mut Surface,
    pub(super) font_manager: Option<&'a FontManager>,
    pub(super) translate_x: f64,
    pub(super) translate_y: f64,
    pub(super) scale_x: f64,
    pub(super) scale_y: f64,
    pub(super) rotation: f64,
    pub(super) clip_stack: Vec<Rect>,
    pub(super) state_stack: Vec<GraphicsState>,
    pub(super) line_join: LineJoin,
    pub(super) line_cap: LineCap,
    pub(super) opacity: f64,
}

impl<'a> SoftwareRenderer<'a> {
    /// Create a renderer using the legacy 5×7 bitmap font.
    pub fn new(surface: &'a mut Surface) -> Self {
        Self {
            surface,
            font_manager: None,
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            clip_stack: Vec::new(),
            state_stack: Vec::new(),
            line_join: LineJoin::Miter,
            line_cap: LineCap::Butt,
            opacity: 1.0,
        }
    }

    /// Create a renderer with TrueType font support.
    pub fn with_fonts(surface: &'a mut Surface, fonts: &'a FontManager) -> Self {
        Self {
            surface,
            font_manager: Some(fonts),
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            clip_stack: Vec::new(),
            state_stack: Vec::new(),
            line_join: LineJoin::Miter,
            line_cap: LineCap::Butt,
            opacity: 1.0,
        }
    }

    /// Apply current transform to a rect.
    /// When rotation is active, computes the axis-aligned bounding box
    /// of the four rotated corners.
    pub(super) fn transform_rect(&self, rect: Rect) -> Rect {
        if self.rotation == 0.0 {
            return Rect::new(
                rect.x * self.scale_x + self.translate_x,
                rect.y * self.scale_y + self.translate_y,
                rect.width * self.scale_x,
                rect.height * self.scale_y,
            );
        }
        let corners = [
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.width, rect.y),
            Point::new(rect.x + rect.width, rect.y + rect.height),
            Point::new(rect.x, rect.y + rect.height),
        ];
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for c in &corners {
            let t = self.transform_point(*c);
            if t.x < min_x {
                min_x = t.x;
            }
            if t.y < min_y {
                min_y = t.y;
            }
            if t.x > max_x {
                max_x = t.x;
            }
            if t.y > max_y {
                max_y = t.y;
            }
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Apply current transform to a point.
    pub(super) fn transform_point(&self, p: Point) -> Point {
        let (sin, cos) = self.rotation.sin_cos();
        let rx = p.x * cos - p.y * sin;
        let ry = p.x * sin + p.y * cos;
        Point::new(
            rx * self.scale_x + self.translate_x,
            ry * self.scale_y + self.translate_y,
        )
    }

    pub(super) fn fill_pixel_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        let mut x0 = x.max(0);
        let mut y0 = y.max(0);
        let mut x1 = (x + w).min(sw);
        let mut y1 = (y + h).min(sh);

        if let Some(clip) = self.effective_clip() {
            x0 = x0.max(clip.x as i32);
            y0 = y0.max(clip.y as i32);
            x1 = x1.min((clip.x + clip.width) as i32);
            y1 = y1.min((clip.y + clip.height) as i32);
        } else if !self.clip_stack.is_empty() {
            // Non-empty clip stack but no effective clip means degenerate (empty) clip
            return;
        }

        // Apply global opacity to the alpha channel.
        let c = if self.opacity < 1.0 {
            let a = ((color >> 24) & 0xFF) as f64 * self.opacity.clamp(0.0, 1.0);
            (((a as u32) & 0xFF) << 24) | (color & 0x00FF_FFFF)
        } else {
            color
        };

        for py in y0..y1 {
            for px in x0..x1 {
                self.surface.set_pixel(px as u32, py as u32, c);
            }
        }
    }
}

impl SoftwareRenderer<'_> {
    /// Get the effective clip rect (intersection of all clips in the stack).
    /// Returns None if no clips are active.
    pub fn effective_clip(&self) -> Option<Rect> {
        if self.clip_stack.is_empty() {
            return None;
        }
        let mut x0 = self.clip_stack[0].x;
        let mut y0 = self.clip_stack[0].y;
        let mut x1 = x0 + self.clip_stack[0].width;
        let mut y1 = y0 + self.clip_stack[0].height;
        for clip in &self.clip_stack[1..] {
            x0 = x0.max(clip.x);
            y0 = y0.max(clip.y);
            x1 = x1.min(clip.x + clip.width);
            y1 = y1.min(clip.y + clip.height);
        }
        if x1 <= x0 || y1 <= y0 {
            return None; // Degenerate clip - nothing visible
        }
        Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

impl SoftwareRenderer<'_> {
    /// Legacy 5×7 bitmap font rendering (used when no FontManager is set).
    pub(super) fn draw_text_bitmap(
        &mut self,
        text: &str,
        origin: Point,
        font: &Font,
        color: Color,
    ) {
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
    pub(super) fn draw_text_truetype(
        &mut self,
        text: &str,
        origin: Point,
        font: &Font,
        color: Color,
        fm: &FontManager,
    ) {
        let normalized = normalize_nfc(text);
        let face = fm.resolve_font(font);
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
                let glyph = glyph_id
                    .with_scale_and_position(size, ab_glyph::point(last_base_x, baseline_y));
                if let Some(outlined) = ab_font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|rx, ry, coverage| {
                        self.blend_pixel(
                            bounds.min.x as i32 + rx as i32,
                            bounds.min.y as i32 + ry as i32,
                            coverage,
                            cr,
                            cg,
                            cb,
                            ca,
                            sw,
                            sh,
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

            let glyph =
                glyph_id.with_scale_and_position(size, ab_glyph::point(cursor_x, baseline_y));

            // Rasterize and blend
            if let Some(outlined) = ab_font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|rx, ry, coverage| {
                    self.blend_pixel(
                        bounds.min.x as i32 + rx as i32,
                        bounds.min.y as i32 + ry as i32,
                        coverage,
                        cr,
                        cg,
                        cb,
                        ca,
                        sw,
                        sh,
                    );
                });
            }

            cursor_x += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }
    }

    /// Alpha-blend a single pixel at (px, py).
    pub(super) fn blend_pixel(
        &mut self,
        px: i32,
        py: i32,
        coverage: f32,
        cr: u32,
        cg: u32,
        cb: u32,
        ca: u32,
        sw: i32,
        sh: i32,
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
    pub(super) fn draw_placeholder_box(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
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
