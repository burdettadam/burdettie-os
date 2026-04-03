//! `impl Renderer for GpuRenderer` and factory function.

use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Point, Rect};
use crate::renderer::Renderer;
use crate::text_layout::AttributedLayout;

use super::atlas::GlyphEntry;
use super::renderer::GpuRenderer;

impl Renderer for GpuRenderer {
    fn clear(&mut self, color: Color) {
        // Fill the entire viewport with the clear color
        self.push_quad(0.0, 0.0, self.target_width as f32, self.target_height as f32, color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.push_quad(
            rect.x as f32,
            rect.y as f32,
            (rect.x + rect.width) as f32,
            (rect.y + rect.height) as f32,
            color,
        );
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        let w = width as f32;
        let x0 = rect.x as f32;
        let y0 = rect.y as f32;
        let x1 = (rect.x + rect.width) as f32;
        let y1 = (rect.y + rect.height) as f32;

        // Top
        self.push_quad(x0, y0, x1, y0 + w, color);
        // Bottom
        self.push_quad(x0, y1 - w, x1, y1, color);
        // Left
        self.push_quad(x0, y0 + w, x0 + w, y1 - w, color);
        // Right
        self.push_quad(x1 - w, y0 + w, x1, y1 - w, color);
    }

    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        if self.font_manager.is_none() {
            return;
        }

        let px_size = font.size as f32;
        let mut cursor_x = origin.x as f32;
        let cursor_y = origin.y as f32 + px_size; // Baseline

        // Phase 1: rasterize glyphs (borrows font_manager + atlas)
        let mut prepared: Vec<(GlyphEntry, f32, f32)> = Vec::new();
        {
            let fm = self.font_manager.as_ref().unwrap();
            for ch in text.chars() {
                if let Some(face) = fm.face_by_index(0) {
                    let font_inner = face.inner();
                    let glyph_id = ab_glyph::Font::glyph_id(font_inner, ch);
                    let scaled = ab_glyph::Font::as_scaled(font_inner, px_size);
                    let advance = ab_glyph::ScaleFont::h_advance(&scaled, glyph_id);

                    if let Some(&entry) = self.atlas.rasterize_glyph(fm, glyph_id.0 as u32, px_size, 0) {
                        prepared.push((entry, cursor_x + entry.offset_x, cursor_y + entry.offset_y));
                    }
                    cursor_x += advance;
                }
            }
        }

        // Phase 2: push quads (fm borrow dropped)
        for (entry, gx, gy) in prepared {
            self.atlas_dirty = true;
            self.push_textured_quad(
                gx, gy,
                gx + entry.width, gy + entry.height,
                entry.u0, entry.v0, entry.u1, entry.v1,
                color,
            );
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64) {
        let dx = (to.x - from.x) as f32;
        let dy = (to.y - from.y) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let hw = width as f32 * 0.5;
        let nx = -dy / len * hw;
        let ny = dx / len * hw;

        let [r, g, b, a] = [color.r as f32, color.g as f32, color.b as f32, color.a as f32];
        let p0 = self.to_ndc(from.x as f32 + nx, from.y as f32 + ny);
        let p1 = self.to_ndc(from.x as f32 - nx, from.y as f32 - ny);
        let p2 = self.to_ndc(to.x as f32 + nx, to.y as f32 + ny);
        let p3 = self.to_ndc(to.x as f32 - nx, to.y as f32 - ny);

        use super::vertex::Vertex;
        self.vertices.push(Vertex::colored(p0[0], p0[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p2[0], p2[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p1[0], p1[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p2[0], p2[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p3[0], p3[1], r, g, b, a));
        self.vertices.push(Vertex::colored(p1[0], p1[1], r, g, b, a));
    }

    fn draw_attributed_text(
        &mut self,
        layout: &AttributedLayout<'_>,
        origin: Point,
        default_color: Color,
    ) {
        if self.font_manager.is_none() {
            return;
        }

        // Phase 1: collect glyph entries and missing rects
        enum GlyphOp {
            Textured { entry: GlyphEntry, gx: f32, gy: f32, color: Color },
            Missing { rect: Rect, color: Color },
        }
        let mut ops: Vec<GlyphOp> = Vec::new();
        {
            let fm = self.font_manager.as_ref().unwrap();
            for glyph in &layout.glyphs {
                let color = glyph.color.unwrap_or(default_color);
                let font_index = glyph.font_index;
                let px_size = glyph.size as f32;

                if glyph.missing {
                    let bw = px_size * 0.6;
                    let bh = px_size * 0.8;
                    ops.push(GlyphOp::Missing {
                        rect: Rect::new(origin.x + glyph.x, origin.y + glyph.y, bw as f64, bh as f64),
                        color,
                    });
                    continue;
                }

                if let Some(&entry) = self.atlas.rasterize_glyph(fm, glyph.glyph_id.0 as u32, px_size, font_index) {
                    ops.push(GlyphOp::Textured {
                        entry,
                        gx: origin.x as f32 + glyph.x as f32 + entry.offset_x,
                        gy: origin.y as f32 + glyph.y as f32 + entry.offset_y,
                        color,
                    });
                }
            }
        }

        // Phase 2: push quads (fm borrow dropped)
        for op in ops {
            match op {
                GlyphOp::Textured { entry, gx, gy, color } => {
                    self.atlas_dirty = true;
                    self.push_textured_quad(
                        gx, gy,
                        gx + entry.width, gy + entry.height,
                        entry.u0, entry.v0, entry.u1, entry.v1,
                        color,
                    );
                }
                GlyphOp::Missing { rect, color } => {
                    self.stroke_rect(rect, color, 1.0);
                }
            }
        }
    }

    fn supports_attributed_text(&self) -> bool {
        self.font_manager.is_some()
    }
}

/// Try to create a GpuRenderer. Returns None if no GPU is available.
pub fn try_create_gpu_renderer(width: u32, height: u32) -> Option<GpuRenderer> {
    GpuRenderer::new(width, height)
}
