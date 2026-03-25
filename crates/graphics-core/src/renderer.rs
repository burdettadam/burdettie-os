use super::color::Color;
use super::font::Font;
use super::geometry::{Point, Rect};

// ── Renderer trait ─────────────────────────────────────────────────

/// 2D drawing operations. Backends implement this for their target surface.
pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64);
    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color);
    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64);
}
