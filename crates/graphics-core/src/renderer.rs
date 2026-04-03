use super::color::Color;
use super::font::Font;
use super::geometry::{Point, Rect, Size};
use super::gradient::Gradient;
use super::image_buf::ImageBuffer;
use super::path::BezierPath;
use super::shadow::Shadow;
use super::text_layout::AttributedLayout;

// ── Line style enums ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

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

    /// Push a clip rectangle onto the clip stack. All subsequent drawing
    /// is restricted to this rect (intersected with any existing clip).
    fn push_clip(&mut self, _rect: Rect) {
        // Default: no-op (unclipped)
    }

    /// Pop the most recent clip rectangle, restoring the previous clip.
    fn pop_clip(&mut self) {
        // Default: no-op
    }

    /// Get the current clip rectangle (if any).
    fn current_clip(&self) -> Option<Rect> {
        None
    }

    // ── Transform / State Stack ────────────────────────────────────

    /// Save the current graphics state (clip, transform) onto a stack.
    fn save(&mut self) {
        // Default: no-op
    }

    /// Restore the most recently saved graphics state.
    fn restore(&mut self) {
        // Default: no-op
    }

    /// Translate all subsequent drawing by (dx, dy).
    fn translate(&mut self, _dx: f64, _dy: f64) {
        // Default: no-op
    }

    /// Scale all subsequent drawing by (sx, sy) around the origin.
    fn scale(&mut self, _sx: f64, _sy: f64) {
        // Default: no-op
    }

    /// Rotate all subsequent drawing by the given angle in radians.
    fn rotate(&mut self, _angle_radians: f64) {
        // Default: no-op
    }

    /// Measure the size a text string would occupy when rendered.
    fn measure_text(&self, text: &str, font: &Font) -> Size {
        let avg_char_width = font.size * 0.6;
        let width = text.len() as f64 * avg_char_width;
        let height = font.size * 1.2;
        Size::new(width, height)
    }

    /// Draw text wrapped within a bounding rect.
    fn draw_text_in_rect(&mut self, text: &str, rect: Rect, font: &Font, color: Color) {
        self.draw_text(text, Point::new(rect.x, rect.y), font, color);
    }

    /// Clip subsequent drawing to the interior of a path.
    /// Default implementation degrades to a bounding-box rect clip.
    fn clip_to_path(&mut self, path: &BezierPath) {
        let bbox = path.bounding_box();
        self.push_clip(bbox);
    }

    // ── Line style ─────────────────────────────────────────────────

    fn set_line_join(&mut self, _join: LineJoin) {}
    fn set_line_cap(&mut self, _cap: LineCap) {}
    fn line_join(&self) -> LineJoin { LineJoin::Miter }
    fn line_cap(&self) -> LineCap { LineCap::Butt }

    // ── Opacity ────────────────────────────────────────────────────

    fn set_opacity(&mut self, _opacity: f64) {}
    fn opacity(&self) -> f64 { 1.0 }

    // ── Arc / sector ───────────────────────────────────────────────

    /// Draw a filled arc (pie sector).
    fn fill_arc(&mut self, _center: Point, _radius: f64, _start_angle: f64, _end_angle: f64, _color: Color) {}

    /// Draw an arc outline.
    fn stroke_arc(&mut self, _center: Point, _radius: f64, _start_angle: f64, _end_angle: f64, _color: Color, _width: f64) {}
}
