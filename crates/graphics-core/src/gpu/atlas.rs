//! Glyph atlas for GPU text rendering.

use crate::font_manager::FontManager;

/// Side length of the glyph atlas texture (square).
pub(super) const ATLAS_SIZE: u32 = 1024;

/// Cached position and metrics of a single rasterized glyph.
#[derive(Clone, Copy)]
pub(super) struct GlyphEntry {
    pub(super) u0: f32,
    pub(super) v0: f32,
    pub(super) u1: f32,
    pub(super) v1: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) offset_x: f32,
    pub(super) offset_y: f32,
}

/// Greyscale glyph atlas backed by a flat pixel buffer.
///
/// Glyphs are rasterized on demand and packed left-to-right, top-to-bottom
/// with one pixel of padding between rows and columns.
pub(super) struct GlyphAtlas {
    pub(super) pixels: Vec<u8>,
    pub(super) entries: std::collections::HashMap<(u32, u16), GlyphEntry>, // (glyph_id, px_size) → entry
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl GlyphAtlas {
    pub(super) fn new() -> Self {
        Self {
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            entries: std::collections::HashMap::new(),
            cursor_x: 1,
            cursor_y: 1,
            row_height: 0,
        }
    }

    pub(super) fn rasterize_glyph(
        &mut self,
        font_manager: &FontManager,
        glyph_id: u32,
        px_size: f32,
        font_index: usize,
    ) -> Option<&GlyphEntry> {
        let key = (glyph_id, px_size as u16);
        if self.entries.contains_key(&key) {
            return self.entries.get(&key);
        }

        // Get the font face and rasterize
        let face = font_manager.face_by_index(font_index)?;
        let font_inner = face.inner();

        let ab_glyph_id = ab_glyph::GlyphId(glyph_id as u16);
        let glyph = ab_glyph_id.with_scale_and_position(px_size, ab_glyph::point(0.0, 0.0));

        let outlined = ab_glyph::Font::outline_glyph(font_inner, glyph)?;
        let bounds = outlined.px_bounds();

        let gw = (bounds.max.x - bounds.min.x).ceil() as u32;
        let gh = (bounds.max.y - bounds.min.y).ceil() as u32;

        if gw == 0 || gh == 0 {
            return None;
        }

        // Advance cursor
        if self.cursor_x + gw + 1 >= ATLAS_SIZE {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.cursor_y + gh + 1 >= ATLAS_SIZE {
            return None; // Atlas full
        }
        self.row_height = self.row_height.max(gh);

        let ox = self.cursor_x;
        let oy = self.cursor_y;

        // Rasterize into atlas
        outlined.draw(|x, y, c| {
            let px = ox + x;
            let py = oy + y;
            if px < ATLAS_SIZE && py < ATLAS_SIZE {
                self.pixels[(py * ATLAS_SIZE + px) as usize] = (c * 255.0) as u8;
            }
        });

        self.cursor_x = ox + gw + 1;

        let u0 = ox as f32 / ATLAS_SIZE as f32;
        let v0 = oy as f32 / ATLAS_SIZE as f32;
        let u1 = (ox + gw) as f32 / ATLAS_SIZE as f32;
        let v1 = (oy + gh) as f32 / ATLAS_SIZE as f32;

        self.entries.insert(
            key,
            GlyphEntry {
                u0,
                v0,
                u1,
                v1,
                width: gw as f32,
                height: gh as f32,
                offset_x: bounds.min.x,
                offset_y: bounds.min.y,
            },
        );

        self.entries.get(&key)
    }
}
