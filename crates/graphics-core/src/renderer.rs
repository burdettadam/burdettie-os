use super::color::Color;
use super::font::Font;
use super::geometry::{Point, Rect};
use super::gradient::Gradient;
use super::image_buf::ImageBuffer;
use super::path::BezierPath;
use super::shadow::Shadow;
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

    // ── F3: Image & Drawing Primitives ─────────────────────────────

    /// Draw an image at the given destination rect, scaling as needed.
    fn draw_image(&mut self, _image: &ImageBuffer, _dest: Rect) {}

    /// Draw an image at its natural size at the given origin.
    fn draw_image_at(&mut self, image: &ImageBuffer, origin: Point) {
        self.draw_image(
            image,
            Rect::new(origin.x, origin.y, image.width() as f64, image.height() as f64),
        );
    }

    /// Fill a rounded rectangle.
    fn fill_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color) {
        // Default: fall back to regular fill_rect (no rounding).
        let _ = radius;
        self.fill_rect(rect, color);
    }

    /// Stroke a rounded rectangle.
    fn stroke_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color, width: f64) {
        let _ = radius;
        self.stroke_rect(rect, color, width);
    }

    /// Fill an ellipse inscribed in the given bounding rect.
    fn fill_ellipse(&mut self, rect: Rect, color: Color) {
        // Default: fill the bounding rect.
        self.fill_rect(rect, color);
    }

    /// Stroke an ellipse inscribed in the given bounding rect.
    fn stroke_ellipse(&mut self, rect: Rect, color: Color, width: f64) {
        self.stroke_rect(rect, color, width);
    }

    /// Fill a circle.
    fn fill_circle(&mut self, center: Point, radius: f64, color: Color) {
        self.fill_ellipse(
            Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0),
            color,
        );
    }

    /// Fill a Bézier path.
    fn fill_path(&mut self, _path: &BezierPath, _color: Color) {}

    /// Stroke a Bézier path.
    fn stroke_path(&mut self, _path: &BezierPath, _color: Color, _width: f64) {}

    /// Fill a rect with a gradient.
    fn fill_gradient(&mut self, rect: Rect, gradient: &Gradient) {
        // Default: sample the gradient at the center and use a solid fill.
        let center = rect.center();
        self.fill_rect(rect, gradient.color_at(center.x, center.y));
    }

    /// Draw with a drop shadow. The `draw_fn` closure performs the actual
    /// drawing; the shadow is rendered beneath it.
    fn with_shadow<F: FnOnce(&mut Self)>(&mut self, _shadow: &Shadow, draw_fn: F)
    where
        Self: Sized,
    {
        // Default: just draw without shadow.
        draw_fn(self);
    }
}
