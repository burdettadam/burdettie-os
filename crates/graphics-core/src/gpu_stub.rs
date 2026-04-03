//! Placeholder GPU renderer that delegates to [`SoftwareRenderer`].
//!
//! Provides API parity with the real GPU backend (`gpu` feature) so code
//! using `dyn Renderer` compiles regardless of which backend is chosen.

use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Point, Rect, Size};
use crate::renderer::Renderer;
use crate::software::SoftwareRenderer;
use crate::surface::Surface;

/// Placeholder GPU renderer backed by software rendering.
///
/// All drawing operations delegate to [`SoftwareRenderer`] internally.
/// This struct exists to provide trait-level API parity so user code
/// written against `dyn Renderer` compiles with either backend.
pub struct GpuStubRenderer {
    surface: Surface,
}

impl GpuStubRenderer {
    /// Create a new placeholder GPU renderer with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            surface: Surface::new(width, height),
        }
    }

    /// Access the underlying pixel buffer.
    pub fn pixels(&self) -> &[u32] {
        self.surface.pixels()
    }

    /// Surface width in pixels.
    pub fn width(&self) -> u32 {
        self.surface.width()
    }

    /// Surface height in pixels.
    pub fn height(&self) -> u32 {
        self.surface.height()
    }

    /// Helper: run a closure with a temporary [`SoftwareRenderer`].
    fn with_sw<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut SoftwareRenderer<'_>) -> R,
    {
        let mut sw = SoftwareRenderer::new(&mut self.surface);
        f(&mut sw)
    }
}

impl Renderer for GpuStubRenderer {
    fn clear(&mut self, color: Color) {
        self.with_sw(|sw| sw.clear(color));
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.with_sw(|sw| sw.fill_rect(rect, color));
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        self.with_sw(|sw| sw.stroke_rect(rect, color, width));
    }

    fn draw_text(&mut self, text: &str, origin: Point, font: &Font, color: Color) {
        self.with_sw(|sw| sw.draw_text(text, origin, font, color));
    }

    fn draw_line(&mut self, from: Point, to: Point, color: Color, width: f64) {
        self.with_sw(|sw| sw.draw_line(from, to, color, width));
    }

    fn measure_text(&self, text: &str, font: &Font) -> Size {
        // Use the default estimate (no FontManager available).
        let avg_char_width = font.size * 0.6;
        let width = text.len() as f64 * avg_char_width;
        let height = font.size * 1.2;
        Size::new(width, height)
    }

    // All other methods use the default trait implementations.
}
