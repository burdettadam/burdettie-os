//! `impl Renderer for SoftwareRenderer` - all Renderer trait methods.

use ab_glyph::Font as AbFont;

use crate::color::Color;
use crate::font::Font;
use crate::font_manager::FontManager;
use crate::geometry::{Point, Rect, Size};
use crate::gradient::Gradient;
use crate::image_buf::ImageBuffer;
use crate::path::BezierPath;
use crate::renderer::{LineCap, LineJoin, Renderer};
use crate::shadow::Shadow;
use crate::text_layout::AttributedLayout;

use super::renderer::{GraphicsState, SoftwareRenderer};

/// Helper: measure text width without borrowing the full renderer.
fn measure_text_width(fm: Option<&FontManager>, text: &str, font: &Font) -> f64 {
    if let Some(fm) = fm {
        let face = fm.resolve_font(font);
        return face.measure_text(text, font.size);
    }
    text.len() as f64 * font.size * 0.6
}

/// Bilinear interpolation of an ARGB32 image at fractional coordinates.
fn sample_bilinear(image: &ImageBuffer, w: usize, h: usize, fx: f64, fy: f64) -> u32 {
    let fx = fx.max(0.0);
    let fy = fy.max(0.0);
    let x0 = (fx.floor() as usize).min(w.saturating_sub(1));
    let y0 = (fy.floor() as usize).min(h.saturating_sub(1));
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let frac_x = fx - fx.floor();
    let frac_y = fy - fy.floor();

    let c00 = image.argb32_at(x0 as u32, y0 as u32);
    let c10 = image.argb32_at(x1 as u32, y0 as u32);
    let c01 = image.argb32_at(x0 as u32, y1 as u32);
    let c11 = image.argb32_at(x1 as u32, y1 as u32);

    let lerp_channel = |shift: u32| -> u32 {
        let v00 = ((c00 >> shift) & 0xFF) as f64;
        let v10 = ((c10 >> shift) & 0xFF) as f64;
        let v01 = ((c01 >> shift) & 0xFF) as f64;
        let v11 = ((c11 >> shift) & 0xFF) as f64;
        let top = v00 + (v10 - v00) * frac_x;
        let bot = v01 + (v11 - v01) * frac_x;
        let val = top + (bot - top) * frac_y;
        val.round().clamp(0.0, 255.0) as u32
    };

    let a = lerp_channel(24);
    let r = lerp_channel(16);
    let g = lerp_channel(8);
    let b = lerp_channel(0);
    (a << 24) | (r << 16) | (g << 8) | b
}

impl Renderer for SoftwareRenderer<'_> {
    fn clear(&mut self, color: Color) {
        self.surface.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let rect = self.transform_rect(rect);
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
        let rect = self.transform_rect(rect);
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
        let origin = self.transform_point(origin);
        if let Some(fm) = self.font_manager {
            self.draw_text_truetype(text, origin, font, color, fm);
        } else {
            self.draw_text_bitmap(text, origin, font, color);
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64) {
        let from = self.transform_point(from);
        let to = self.transform_point(to);
        let c = color.to_argb32();

        if width > 1.0 {
            // Draw a filled strip along the segment with the given width.
            let dxx = to.x - from.x;
            let dyy = to.y - from.y;
            let len = (dxx * dxx + dyy * dyy).sqrt();
            if len < 1e-12 {
                // Degenerate segment: fill a square
                let half = (width / 2.0) as i32;
                self.fill_pixel_rect(
                    from.x as i32 - half,
                    from.y as i32 - half,
                    width as i32,
                    width as i32,
                    c,
                );
                return;
            }
            // Perpendicular direction, unit length
            let px = -dyy / len;
            let py = dxx / len;
            let half_w = width / 2.0;
            // Compute bounding box of the thick line
            let ox = px * half_w;
            let oy = py * half_w;
            let corners = [
                (from.x + ox, from.y + oy),
                (from.x - ox, from.y - oy),
                (to.x + ox, to.y + oy),
                (to.x - ox, to.y - oy),
            ];
            let min_x = corners.iter().map(|c| c.0).fold(f64::MAX, f64::min) as i32;
            let min_y = corners.iter().map(|c| c.1).fold(f64::MAX, f64::min) as i32;
            let max_x = corners.iter().map(|c| c.0).fold(f64::MIN, f64::max) as i32 + 1;
            let max_y = corners.iter().map(|c| c.1).fold(f64::MIN, f64::max) as i32 + 1;

            let sw = self.surface.width() as i32;
            let sh = self.surface.height() as i32;
            let x0 = min_x.max(0);
            let y0 = min_y.max(0);
            let x1 = max_x.min(sw);
            let y1 = max_y.min(sh);

            // For each pixel in the bounding box, check distance to the segment
            for iy in y0..y1 {
                for ix in x0..x1 {
                    let fx = ix as f64 + 0.5;
                    let fy = iy as f64 + 0.5;
                    // Project onto segment
                    let t = ((fx - from.x) * dxx + (fy - from.y) * dyy) / (len * len);
                    let t = t.clamp(0.0, 1.0);
                    let near_x = from.x + t * dxx;
                    let near_y = from.y + t * dyy;
                    let dist = ((fx - near_x).powi(2) + (fy - near_y).powi(2)).sqrt();
                    if dist <= half_w {
                        self.fill_pixel_rect(ix, iy, 1, 1, c);
                    }
                }
            }
            return;
        }

        // Bresenham's line algorithm for width <= 1
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
            self.fill_pixel_rect(x0, y0, 1, 1, c);
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

    fn draw_attributed_text(
        &mut self,
        layout: &AttributedLayout<'_>,
        origin: Point,
        default_color: Color,
    ) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        for g in &layout.glyphs {
            let color = g.color.unwrap_or(default_color);
            let cr = (color.r.clamp(0.0, 1.0) * 255.0) as u32;
            let cg = (color.g.clamp(0.0, 1.0) * 255.0) as u32;
            let cb = (color.b.clamp(0.0, 1.0) * 255.0) as u32;
            let ca = (color.a.clamp(0.0, 1.0) * 255.0) as u32;

            if g.missing {
                let placeholder_w = (g.size * 0.6) as i32;
                let placeholder_h = (g.y * 0.9) as i32;
                let px = (origin.x + g.x) as i32;
                let py = (origin.y + g.y - g.y * 0.85) as i32;
                self.draw_placeholder_box(px, py, placeholder_w, placeholder_h, color);
                continue;
            }

            let face = layout.fonts[g.font_index];
            let ab_font = face.inner();
            let glyph = g.glyph_id.with_scale_and_position(
                g.size as f32,
                ab_glyph::point(
                    (origin.x + g.x) as f32,
                    (origin.y + g.y) as f32,
                ),
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
        }
    }

    fn supports_attributed_text(&self) -> bool {
        self.font_manager.is_some()
    }

    // ── F3: Image & Drawing Primitives ─────────────────────────────

    fn draw_image(&mut self, image: &ImageBuffer, dest: Rect) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;
        let iw = image.width();
        let ih = image.height();
        if iw == 0 || ih == 0 {
            return;
        }

        let mut dx0 = (dest.x as i32).max(0);
        let mut dy0 = (dest.y as i32).max(0);
        let mut dx1 = ((dest.x + dest.width) as i32).min(sw);
        let mut dy1 = ((dest.y + dest.height) as i32).min(sh);

        // Respect the clip stack
        if let Some(clip) = self.effective_clip() {
            dx0 = dx0.max(clip.x as i32);
            dy0 = dy0.max(clip.y as i32);
            dx1 = dx1.min((clip.x + clip.width) as i32);
            dy1 = dy1.min((clip.y + clip.height) as i32);
        } else if !self.clip_stack.is_empty() {
            return; // degenerate clip
        }

        let pixels = self.surface.pixels_mut();
        let needs_scaling = (dest.width as u32 != iw) || (dest.height as u32 != ih);
        for py in dy0..dy1 {
            for px in dx0..dx1 {
                let fx = (px as f64 - dest.x) / dest.width * iw as f64 - 0.5;
                let fy = (py as f64 - dest.y) / dest.height * ih as f64 - 0.5;
                let src = if needs_scaling {
                    sample_bilinear(image, iw as usize, ih as usize, fx, fy)
                } else {
                    let sx = fx.round().max(0.0) as u32;
                    let sy = fy.round().max(0.0) as u32;
                    image.argb32_at(sx.min(iw - 1), sy.min(ih - 1))
                };
                let sa = (src >> 24) & 0xFF;
                if sa == 0 {
                    continue;
                }
                let idx = (py * sw + px) as usize;
                if sa >= 255 {
                    pixels[idx] = src;
                } else {
                    let inv = 255 - sa;
                    let sr = (src >> 16) & 0xFF;
                    let sg = (src >> 8) & 0xFF;
                    let sb = src & 0xFF;
                    let dst = pixels[idx];
                    let dr = (dst >> 16) & 0xFF;
                    let dg = (dst >> 8) & 0xFF;
                    let db = dst & 0xFF;
                    let da = (dst >> 24) & 0xFF;
                    let or = (sr * sa + dr * inv) / 255;
                    let og = (sg * sa + dg * inv) / 255;
                    let ob = (sb * sa + db * inv) / 255;
                    let oa = sa + (da * inv) / 255;
                    pixels[idx] = (oa << 24) | (or << 16) | (og << 8) | ob;
                }
            }
        }
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color) {
        let path = BezierPath::rounded_rect(rect.x, rect.y, rect.width, rect.height, radius);
        self.fill_path(&path, color);
    }

    fn stroke_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color, width: f64) {
        let path = BezierPath::rounded_rect(rect.x, rect.y, rect.width, rect.height, radius);
        self.stroke_path(&path, color, width);
    }

    fn fill_ellipse(&mut self, rect: Rect, color: Color) {
        let path = BezierPath::ellipse(rect.x, rect.y, rect.width, rect.height);
        self.fill_path(&path, color);
    }

    fn stroke_ellipse(&mut self, rect: Rect, color: Color, width: f64) {
        let path = BezierPath::ellipse(rect.x, rect.y, rect.width, rect.height);
        self.stroke_path(&path, color, width);
    }

    fn fill_path(&mut self, path: &BezierPath, color: Color) {
        let subpaths = path.flatten(0.5);
        if subpaths.is_empty() {
            return;
        }
        let c = color.to_argb32();
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        // Clip bounds
        let (clip_x0, clip_y0, clip_x1, clip_y1) = if let Some(clip) = self.effective_clip() {
            (clip.x as i32, clip.y as i32, (clip.x + clip.width) as i32, (clip.y + clip.height) as i32)
        } else if !self.clip_stack.is_empty() {
            return; // degenerate clip
        } else {
            (0, 0, sw, sh)
        };

        // Compute bounding box
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for sp in &subpaths {
            for p in sp {
                if p.y < min_y { min_y = p.y; }
                if p.y > max_y { max_y = p.y; }
            }
        }
        let y0 = (min_y as i32).max(0).max(clip_y0);
        let y1 = ((max_y as i32) + 1).min(sh).min(clip_y1);

        // Scanline fill with even-odd rule
        let mut crossings: Vec<f64> = Vec::new();
        let pixels = self.surface.pixels_mut();
        for y in y0..y1 {
            crossings.clear();
            let scan_y = y as f64 + 0.5;
            for sp in &subpaths {
                if sp.len() < 2 { continue; }
                for i in 0..sp.len() - 1 {
                    let p0 = sp[i];
                    let p1 = sp[i + 1];
                    let (y_lo, y_hi) = if p0.y <= p1.y { (p0.y, p1.y) } else { (p1.y, p0.y) };
                    if scan_y < y_lo || scan_y >= y_hi {
                        continue;
                    }
                    let dy = p1.y - p0.y;
                    if dy.abs() < 1e-12 { continue; }
                    let t = (scan_y - p0.y) / dy;
                    let x_cross = p0.x + t * (p1.x - p0.x);
                    crossings.push(x_cross);
                }
            }
            crossings.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            // Fill between pairs (even-odd rule)
            let mut i = 0;
            while i + 1 < crossings.len() {
                let x0 = (crossings[i] as i32).max(0).max(clip_x0);
                let x1 = ((crossings[i + 1] as i32) + 1).min(sw).min(clip_x1);
                for x in x0..x1 {
                    let idx = (y * sw + x) as usize;
                    let ca = (c >> 24) & 0xFF;
                    if ca >= 255 {
                        pixels[idx] = c;
                    } else if ca > 0 {
                        let inv = 255 - ca;
                        let sr = (c >> 16) & 0xFF;
                        let sg = (c >> 8) & 0xFF;
                        let sb = c & 0xFF;
                        let dst = pixels[idx];
                        let dr = (dst >> 16) & 0xFF;
                        let dg = (dst >> 8) & 0xFF;
                        let db = dst & 0xFF;
                        let da = (dst >> 24) & 0xFF;
                        let or = (sr * ca + dr * inv) / 255;
                        let og = (sg * ca + dg * inv) / 255;
                        let ob = (sb * ca + db * inv) / 255;
                        let oa = ca + (da * inv) / 255;
                        pixels[idx] = (oa << 24) | (or << 16) | (og << 8) | ob;
                    }
                }
                i += 2;
            }
        }
    }

    fn stroke_path(&mut self, path: &BezierPath, color: Color, width: f64) {
        let subpaths = path.flatten(0.5);
        for sp in &subpaths {
            if sp.len() < 2 { continue; }
            for i in 0..sp.len() - 1 {
                self.draw_line(sp[i], sp[i + 1], color, width);
            }
        }
    }

    fn fill_gradient(&mut self, rect: Rect, gradient: &Gradient) {
        let sw = self.surface.width() as i32;
        let sh = self.surface.height() as i32;

        let mut x0 = (rect.x as i32).max(0);
        let mut y0 = (rect.y as i32).max(0);
        let mut x1 = ((rect.x + rect.width) as i32).min(sw);
        let mut y1 = ((rect.y + rect.height) as i32).min(sh);

        // Respect clip stack
        if let Some(clip) = self.effective_clip() {
            x0 = x0.max(clip.x as i32);
            y0 = y0.max(clip.y as i32);
            x1 = x1.min((clip.x + clip.width) as i32);
            y1 = y1.min((clip.y + clip.height) as i32);
        } else if !self.clip_stack.is_empty() {
            return; // degenerate clip
        }

        let pixels = self.surface.pixels_mut();
        for py in y0..y1 {
            for px in x0..x1 {
                let color = gradient.color_at(px as f64 + 0.5, py as f64 + 0.5);
                let c = color.to_argb32();
                let ca = (c >> 24) & 0xFF;
                if ca == 0 { continue; }
                let idx = (py * sw + px) as usize;
                if ca >= 255 {
                    pixels[idx] = c;
                } else {
                    let inv = 255 - ca;
                    let sr = (c >> 16) & 0xFF;
                    let sg = (c >> 8) & 0xFF;
                    let sb = c & 0xFF;
                    let dst = pixels[idx];
                    let dr = (dst >> 16) & 0xFF;
                    let dg = (dst >> 8) & 0xFF;
                    let db = dst & 0xFF;
                    let da = (dst >> 24) & 0xFF;
                    let or = (sr * ca + dr * inv) / 255;
                    let og = (sg * ca + dg * inv) / 255;
                    let ob = (sb * ca + db * inv) / 255;
                    let oa = ca + (da * inv) / 255;
                    pixels[idx] = (oa << 24) | (or << 16) | (og << 8) | ob;
                }
            }
        }
    }

    fn save(&mut self) {
        self.state_stack.push(GraphicsState {
            translate_x: self.translate_x,
            translate_y: self.translate_y,
            scale_x: self.scale_x,
            scale_y: self.scale_y,
            rotation: self.rotation,
            clip_stack_depth: self.clip_stack.len(),
            line_join: self.line_join,
            line_cap: self.line_cap,
            opacity: self.opacity,
        });
    }

    fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.translate_x = state.translate_x;
            self.translate_y = state.translate_y;
            self.scale_x = state.scale_x;
            self.scale_y = state.scale_y;
            self.rotation = state.rotation;
            self.clip_stack.truncate(state.clip_stack_depth);
            self.line_join = state.line_join;
            self.line_cap = state.line_cap;
            self.opacity = state.opacity;
        }
    }

    fn translate(&mut self, dx: f64, dy: f64) {
        self.translate_x += dx;
        self.translate_y += dy;
    }

    fn scale(&mut self, sx: f64, sy: f64) {
        self.scale_x *= sx;
        self.scale_y *= sy;
        self.translate_x *= sx;
        self.translate_y *= sy;
    }

    fn rotate(&mut self, angle_radians: f64) {
        self.rotation += angle_radians;
    }

    fn with_shadow<F: FnOnce(&mut Self)>(&mut self, shadow: &Shadow, draw_fn: F) {
        let sw = self.surface.width();
        let sh = self.surface.height();

        // 1. Snapshot the current surface
        let snapshot: Vec<u32> = self.surface.pixels().to_vec();

        // 2. Draw the content onto self
        draw_fn(self);

        // 3. Extract the content mask (pixels that changed)
        let after: Vec<u32> = self.surface.pixels().to_vec();
        let len = (sw * sh) as usize;
        let mut content_mask: Vec<u32> = vec![0; len];
        for i in 0..len {
            if after[i] != snapshot[i] {
                content_mask[i] = after[i];
            }
        }

        // 4. Create offset + tinted shadow from the content mask
        let ox = shadow.offset_x as i32;
        let oy = shadow.offset_y as i32;
        let shadow_argb = shadow.color.to_argb32();
        let s_a_f = ((shadow_argb >> 24) & 0xFF) as f64 / 255.0;
        let s_r = (shadow_argb >> 16) & 0xFF;
        let s_g = (shadow_argb >> 8) & 0xFF;
        let s_b = shadow_argb & 0xFF;
        let w = sw as i32;
        let h = sh as i32;

        let mut shadow_buf: Vec<u32> = vec![0; len];
        for py in 0..h {
            for px in 0..w {
                let src_x = px - ox;
                let src_y = py - oy;
                if src_x >= 0 && src_x < w && src_y >= 0 && src_y < h {
                    let src_idx = (src_y * w + src_x) as usize;
                    let src_pixel = content_mask[src_idx];
                    let src_alpha = ((src_pixel >> 24) & 0xFF) as f64 / 255.0;
                    if src_alpha > 0.0 {
                        let alpha = (s_a_f * src_alpha * 255.0) as u32;
                        let dst_idx = (py * w + px) as usize;
                        shadow_buf[dst_idx] = (alpha << 24) | (s_r << 16) | (s_g << 8) | s_b;
                    }
                }
            }
        }

        // 5. Box blur the shadow
        let blur_radius = shadow.blur_radius.max(0.0) as i32;
        if blur_radius > 0 {
            let mut blurred: Vec<u32> = vec![0; len];
            let r = blur_radius;
            for py in 0..h {
                for px in 0..w {
                    let mut a_sum: u32 = 0;
                    let mut r_sum: u32 = 0;
                    let mut g_sum: u32 = 0;
                    let mut b_sum: u32 = 0;
                    let mut count: u32 = 0;
                    for ky in -r..=r {
                        for kx in -r..=r {
                            let sx = px + kx;
                            let sy = py + ky;
                            if sx >= 0 && sx < w && sy >= 0 && sy < h {
                                let idx = (sy * w + sx) as usize;
                                let p = shadow_buf[idx];
                                a_sum += (p >> 24) & 0xFF;
                                r_sum += (p >> 16) & 0xFF;
                                g_sum += (p >> 8) & 0xFF;
                                b_sum += p & 0xFF;
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        let idx = (py * w + px) as usize;
                        blurred[idx] = ((a_sum / count) << 24)
                            | ((r_sum / count) << 16)
                            | ((g_sum / count) << 8)
                            | (b_sum / count);
                    }
                }
            }
            shadow_buf = blurred;
        }

        // 6. Restore snapshot, composite shadow, then content on top
        let pixels = self.surface.pixels_mut();
        pixels.copy_from_slice(&snapshot);

        // Composite shadow onto surface
        for i in 0..len {
            let sp = shadow_buf[i];
            let sa = (sp >> 24) & 0xFF;
            if sa == 0 { continue; }
            let inv = 255 - sa;
            let sr = (sp >> 16) & 0xFF;
            let sg = (sp >> 8) & 0xFF;
            let sb = sp & 0xFF;
            let dp = pixels[i];
            let dr = (dp >> 16) & 0xFF;
            let dg = (dp >> 8) & 0xFF;
            let db = dp & 0xFF;
            let da = (dp >> 24) & 0xFF;
            let o_r = (sr * sa + dr * inv) / 255;
            let o_g = (sg * sa + dg * inv) / 255;
            let o_b = (sb * sa + db * inv) / 255;
            let o_a = sa + (da * inv) / 255;
            pixels[i] = (o_a << 24) | (o_r << 16) | (o_g << 8) | o_b;
        }

        // Composite content on top of shadow
        for i in 0..len {
            let cp = content_mask[i];
            let ca = (cp >> 24) & 0xFF;
            if ca == 0 { continue; }
            if ca >= 255 {
                pixels[i] = cp;
            } else {
                let inv = 255 - ca;
                let cr = (cp >> 16) & 0xFF;
                let cg = (cp >> 8) & 0xFF;
                let cb = cp & 0xFF;
                let dp = pixels[i];
                let dr = (dp >> 16) & 0xFF;
                let dg = (dp >> 8) & 0xFF;
                let db = dp & 0xFF;
                let da = (dp >> 24) & 0xFF;
                let o_r = (cr * ca + dr * inv) / 255;
                let o_g = (cg * ca + dg * inv) / 255;
                let o_b = (cb * ca + db * inv) / 255;
                let o_a = ca + (da * inv) / 255;
                pixels[i] = (o_a << 24) | (o_r << 16) | (o_g << 8) | o_b;
            }
        }
    }

    fn push_clip(&mut self, rect: Rect) {
        self.clip_stack.push(rect);
    }

    fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn current_clip(&self) -> Option<Rect> {
        self.effective_clip()
    }

    fn measure_text(&self, text: &str, font: &Font) -> Size {
        if let Some(fm) = self.font_manager {
            let face = fm.resolve_font(font);
            let width = face.measure_text(text, font.size);
            let height = face.line_height(font.size);
            return Size::new(width, height);
        }
        // Bitmap fallback: each char is font_size * 0.6 wide
        let avg_char_width = font.size * 0.6;
        let width = text.len() as f64 * avg_char_width;
        let height = font.size * 1.2;
        Size::new(width, height)
    }

    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, font: &Font, color: Color) {
        let words: Vec<&str> = text.split_whitespace().collect();
        // Compute measurements up front to avoid borrow conflicts
        let space_width = measure_text_width(self.font_manager, " ", font);
        let word_widths: Vec<f64> = words
            .iter()
            .map(|w| measure_text_width(self.font_manager, w, font))
            .collect();

        let mut lines: Vec<String> = vec![String::new()];
        let mut current_width = 0.0;

        for (i, word) in words.iter().enumerate() {
            let ww = word_widths[i];
            if current_width + ww > rect.width && !lines.last().unwrap().is_empty() {
                lines.push(String::new());
                current_width = 0.0;
            }
            if !lines.last().unwrap().is_empty() {
                lines.last_mut().unwrap().push(' ');
                current_width += space_width;
            }
            lines.last_mut().unwrap().push_str(word);
            current_width += ww;
        }

        let line_height = font.size * 1.2;
        for (i, line) in lines.iter().enumerate() {
            let y = rect.y + i as f64 * line_height;
            if y + line_height > rect.y + rect.height {
                break;
            }
            self.draw_text(line, Point::new(rect.x, y), font, color);
        }
    }

    fn clip_to_path(&mut self, path: &BezierPath) {
        let bbox = path.bounding_box();
        self.push_clip(bbox);
    }

    // ── Line style ─────────────────────────────────────────────────

    fn set_line_join(&mut self, join: LineJoin) {
        self.line_join = join;
    }

    fn set_line_cap(&mut self, cap: LineCap) {
        self.line_cap = cap;
    }

    fn line_join(&self) -> LineJoin {
        self.line_join
    }

    fn line_cap(&self) -> LineCap {
        self.line_cap
    }

    // ── Opacity ────────────────────────────────────────────────────

    fn set_opacity(&mut self, opacity: f64) {
        self.opacity = opacity;
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    // ── Arc / sector ───────────────────────────────────────────────

    fn fill_arc(
        &mut self,
        center: Point,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        color: Color,
    ) {
        let c = color.to_argb32();
        let (clip_x0, clip_y0, clip_x1, clip_y1) =
            if let Some(clip) = self.effective_clip() {
                (clip.x, clip.y, clip.x + clip.width, clip.y + clip.height)
            } else if !self.clip_stack.is_empty() {
                return;
            } else {
                (
                    0.0,
                    0.0,
                    self.surface.width() as f64,
                    self.surface.height() as f64,
                )
            };
        let x0 = ((center.x - radius).max(clip_x0)) as i32;
        let x1 = ((center.x + radius).min(clip_x1)) as i32;
        let y0 = ((center.y - radius).max(clip_y0)) as i32;
        let y1 = ((center.y + radius).min(clip_y1)) as i32;
        let r2 = radius * radius;

        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = px as f64 - center.x;
                let dy = py as f64 - center.y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let angle = dy.atan2(dx);
                if !angle_in_range(angle, start_angle, end_angle) {
                    continue;
                }
                self.fill_pixel_rect(px, py, 1, 1, c);
            }
        }
    }

    fn stroke_arc(
        &mut self,
        center: Point,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        color: Color,
        width: f64,
    ) {
        // Approximate by drawing short line segments along the arc.
        let c = color.to_argb32();
        let sweep = {
            let mut s = end_angle - start_angle;
            while s < 0.0 {
                s += std::f64::consts::TAU;
            }
            while s > std::f64::consts::TAU {
                s -= std::f64::consts::TAU;
            }
            s
        };
        let steps = ((sweep * radius).abs().max(8.0)) as usize;
        let _step = sweep / steps as f64;
        let half_w = width / 2.0;
        let r_inner = (radius - half_w).max(0.0);
        let r_outer = radius + half_w;
        let r_inner2 = r_inner * r_inner;
        let r_outer2 = r_outer * r_outer;

        let (clip_x0, clip_y0, clip_x1, clip_y1) =
            if let Some(clip) = self.effective_clip() {
                (clip.x, clip.y, clip.x + clip.width, clip.y + clip.height)
            } else if !self.clip_stack.is_empty() {
                return;
            } else {
                (
                    0.0,
                    0.0,
                    self.surface.width() as f64,
                    self.surface.height() as f64,
                )
            };
        let x0 = ((center.x - r_outer).max(clip_x0)) as i32;
        let x1 = ((center.x + r_outer).min(clip_x1)) as i32;
        let y0 = ((center.y - r_outer).max(clip_y0)) as i32;
        let y1 = ((center.y + r_outer).min(clip_y1)) as i32;

        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = px as f64 - center.x;
                let dy = py as f64 - center.y;
                let d2 = dx * dx + dy * dy;
                if d2 < r_inner2 || d2 > r_outer2 {
                    continue;
                }
                let angle = dy.atan2(dx);
                if !angle_in_range(angle, start_angle, end_angle) {
                    continue;
                }
                self.fill_pixel_rect(px, py, 1, 1, c);
            }
        }
    }
}

/// Check if `angle` lies within the arc from `start` to `end` (radians),
/// handling wrapping around ±π.
fn angle_in_range(angle: f64, start: f64, end: f64) -> bool {
    // Normalize everything into [0, TAU)
    let tau = std::f64::consts::TAU;
    let norm = |a: f64| -> f64 { ((a % tau) + tau) % tau };
    let a = norm(angle);
    let s = norm(start);
    let e = norm(end);
    if s <= e {
        a >= s && a <= e
    } else {
        // Wraps around 0, e.g. start=350°, end=10°
        a >= s || a <= e
    }
}
