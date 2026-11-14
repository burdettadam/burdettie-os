//! Glyph atlas for GPU text rendering.

use crate::font_manager::FontFace;
use crate::text_layout::{AttributedLayout, LayoutGlyph};

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
    // Font identity, glyph ID, and exact f32 size bits. Glyph IDs are local to
    // each font; fractional sizes have distinct raster coverage.
    entries: std::collections::HashMap<(u64, u16, u32), GlyphEntry>,
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
        face: &FontFace,
        glyph_id: u32,
        px_size: f32,
    ) -> Option<&GlyphEntry> {
        if !px_size.is_finite() || px_size <= 0.0 {
            return None;
        }
        let glyph_id = u16::try_from(glyph_id).ok()?;
        let key = (face.cache_id(), glyph_id, px_size.to_bits());
        if self.entries.contains_key(&key) {
            return self.entries.get(&key);
        }

        // Get the font face and rasterize
        let font_inner = face.inner();

        let font_glyph_id = crate::GlyphId(glyph_id);
        let glyph =
            font_glyph_id.with_scale_and_position(px_size, ab_glyph_rasterizer::point(0.0, 0.0));

        let outlined = font_inner.outline_glyph(glyph)?;
        let bounds = outlined.px_bounds();

        let gw = (bounds.max.x - bounds.min.x).ceil() as u32;
        let gh = (bounds.max.y - bounds.min.y).ceil() as u32;

        if gw == 0 || gh == 0 || gw > ATLAS_SIZE - 2 || gh > ATLAS_SIZE - 2 {
            return None;
        }

        // Try the placement without reserving it. A failed row wrap must
        // leave the current row available for a smaller subsequent glyph.
        let (ox, oy, row_height) = if self.cursor_x + gw + 1 > ATLAS_SIZE {
            (1, self.cursor_y + self.row_height + 1, 0)
        } else {
            (self.cursor_x, self.cursor_y, self.row_height)
        };
        if oy + gh + 1 > ATLAS_SIZE {
            return None; // Atlas full
        }

        // Rasterize into atlas
        outlined.draw(|x, y, c| {
            let px = ox + x;
            let py = oy + y;
            if px < ATLAS_SIZE && py < ATLAS_SIZE {
                self.pixels[(py * ATLAS_SIZE + px) as usize] = (c * 255.0) as u8;
            }
        });

        self.cursor_x = ox + gw + 1;
        self.cursor_y = oy;
        self.row_height = row_height.max(gh);

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

    pub(super) fn rasterize_layout_glyph(
        &mut self,
        layout: &AttributedLayout<'_>,
        glyph: &LayoutGlyph,
    ) -> Option<&GlyphEntry> {
        let face = layout.fonts.get(glyph.font_index)?;
        self.rasterize_glyph(face, glyph.glyph_id.0 as u32, glyph.size as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_manager::FontManager;

    fn bitmap(atlas: &GlyphAtlas, entry: GlyphEntry) -> Vec<u8> {
        let x = (entry.u0 * ATLAS_SIZE as f32) as usize;
        let y = (entry.v0 * ATLAS_SIZE as f32) as usize;
        (y..y + entry.height as usize)
            .flat_map(|row| {
                let start = row * ATLAS_SIZE as usize + x;
                atlas.pixels[start..start + entry.width as usize]
                    .iter()
                    .copied()
            })
            .collect()
    }

    #[test]
    fn glyph_cache_distinguishes_font_faces() {
        let fm = FontManager::new();
        let glyph_id = fm.system().inner().glyph_id('A').0 as u32;
        assert_eq!(glyph_id, fm.bold().inner().glyph_id('A').0 as u32);
        let mut atlas = GlyphAtlas::new();
        let regular = *atlas.rasterize_glyph(fm.system(), glyph_id, 24.0).unwrap();
        let bold = *atlas.rasterize_glyph(fm.bold(), glyph_id, 24.0).unwrap();
        let mut expected_atlas = GlyphAtlas::new();
        let expected = *expected_atlas
            .rasterize_glyph(fm.bold(), glyph_id, 24.0)
            .unwrap();
        assert_eq!(bitmap(&atlas, bold), bitmap(&expected_atlas, expected));
        assert_ne!(bitmap(&atlas, regular), bitmap(&atlas, bold));
    }

    #[test]
    fn glyph_cache_preserves_fractional_sizes() {
        let fm = FontManager::new();
        let glyph_id = fm.system().inner().glyph_id('A').0 as u32;
        let mut atlas = GlyphAtlas::new();
        let small = *atlas.rasterize_glyph(fm.system(), glyph_id, 24.1).unwrap();
        let large = *atlas.rasterize_glyph(fm.system(), glyph_id, 24.9).unwrap();
        let mut expected_atlas = GlyphAtlas::new();
        let expected = *expected_atlas
            .rasterize_glyph(fm.system(), glyph_id, 24.9)
            .unwrap();
        assert_eq!(bitmap(&atlas, large), bitmap(&expected_atlas, expected));
        assert_ne!(bitmap(&atlas, small), bitmap(&atlas, large));
    }

    #[test]
    fn attributed_glyph_uses_the_layouts_font_table() {
        let fm = FontManager::new();
        let runs = [crate::TextRun {
            range: 0..1,
            font_name: None,
            size: None,
            color: None,
            bold: false,
            italic: true,
        }];
        let layout = crate::layout_attributed("A", &runs, &fm, fm.system(), 24.0);
        let glyph = &layout.glyphs[0];
        // Index 1 is italic in this layout, but bold in the FontManager.
        assert_eq!(glyph.font_index, 1);
        assert_eq!(layout.fonts[1].name(), "System-Italic");
        assert_eq!(fm.face_by_index(1).unwrap().name(), "System-Bold");

        let mut atlas = GlyphAtlas::new();
        let actual = *atlas.rasterize_layout_glyph(&layout, glyph).unwrap();
        let mut expected_atlas = GlyphAtlas::new();
        let expected = *expected_atlas
            .rasterize_glyph(fm.italic(), glyph.glyph_id.0 as u32, 24.0)
            .unwrap();
        assert_eq!(bitmap(&atlas, actual), bitmap(&expected_atlas, expected));
    }

    #[test]
    fn rejected_glyphs_do_not_poison_the_atlas() {
        let fm = FontManager::new();
        let glyph_id = fm.system().inner().glyph_id('A').0 as u32;
        let mut atlas = GlyphAtlas::new();
        for size in [0.0, -1.0, f32::NAN, f32::INFINITY, 10_000.0] {
            assert!(atlas.rasterize_glyph(fm.system(), glyph_id, size).is_none());
        }
        assert!(atlas.rasterize_glyph(fm.system(), u32::MAX, 24.0).is_none());
        let actual = *atlas.rasterize_glyph(fm.system(), glyph_id, 24.0).unwrap();
        let mut expected_atlas = GlyphAtlas::new();
        let expected = *expected_atlas
            .rasterize_glyph(fm.system(), glyph_id, 24.0)
            .unwrap();
        assert_eq!((actual.u0, actual.v0), (expected.u0, expected.v0));
        assert_eq!(bitmap(&atlas, actual), bitmap(&expected_atlas, expected));
    }

    #[test]
    fn rejected_row_wrap_preserves_space_for_smaller_glyphs() {
        let fm = FontManager::new();
        let glyph_id = fm.system().inner().glyph_id('A').0 as u32;
        let mut reference = GlyphAtlas::new();
        let expected = *reference
            .rasterize_glyph(fm.system(), glyph_id, 24.0)
            .unwrap();

        // The last row has exactly enough room left for the small glyph. A
        // larger glyph must wrap, but there is no space for another row.
        let mut atlas = GlyphAtlas::new();
        atlas.cursor_x = ATLAS_SIZE - expected.width as u32 - 1;
        atlas.cursor_y = ATLAS_SIZE - expected.height as u32 - 1;
        atlas.row_height = expected.height as u32;
        let remaining_slot = (atlas.cursor_x, atlas.cursor_y);

        assert!(atlas.rasterize_glyph(fm.system(), glyph_id, 48.0).is_none());
        let actual = *atlas
            .rasterize_glyph(fm.system(), glyph_id, 24.0)
            .expect("rejecting a row wrap must preserve the remaining slot");
        assert_eq!(
            (actual.u0, actual.v0),
            (
                remaining_slot.0 as f32 / ATLAS_SIZE as f32,
                remaining_slot.1 as f32 / ATLAS_SIZE as f32,
            )
        );
        assert_eq!(bitmap(&atlas, actual), bitmap(&reference, expected));
    }
}
