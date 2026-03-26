use super::color::Color;
use super::font::Font;
use super::geometry::{Point, Rect};
use super::text_layout::AttributedLayout;

// ── Renderer trait ─────────────────────────────────────────────────

/// 2D drawing operations. Backends implement this for their target surface.
pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64);
    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color);
    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64);

    /// Draw pre-laid-out attributed text. Each glyph carries its own font
    /// face index and optional color. The `default_color` is used for glyphs
    /// that have `color: None`.
    fn draw_attributed_text(
        &mut self,
        _layout: &AttributedLayout<'_>,
        _origin: Point,
        _default_color: Color,
    ) {
        // Default: no-op. Backends override for attributed text support.
    }

    /// Returns `true` if this renderer supports `draw_attributed_text`.
    fn supports_attributed_text(&self) -> bool {
        false
    }
}
