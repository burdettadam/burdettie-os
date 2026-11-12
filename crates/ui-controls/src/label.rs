//! Label control — displays static text.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A text label control.
pub struct Label {
    view: View,
    text: String,
    font: Font,
    text_color: Color,
}

impl Label {
    /// Create a label with the given frame and text.
    pub fn new(frame: Rect, text: impl Into<String>) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::TRANSPARENT;
        Self {
            view,
            text: text.into(),
            font: Font::default(),
            text_color: Color::BLACK,
        }
    }

    /// Get the current text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the label text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.view.set_needs_display();
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.view.set_needs_display();
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
        self.view.set_needs_display();
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Compute the intrinsic content size based on text length and font.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        // Approximate: ~0.6 * font_size per character width, font_size + 4 height
        let char_width = self.font.size * 0.6;
        let width = self.text.len() as f64 * char_width + 8.0; // +8 for padding
        let height = self.font.size + 4.0;
        IntrinsicSize::new(width, height)
    }

    /// Draw the label.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        if self.view.background_color.a > 0.0 {
            renderer.fill_rect(self.view.frame, self.view.background_color);
        }

        // Draw text (with a small padding from the origin)
        let origin = Point::new(self.view.frame.x + 4.0, self.view.frame.y + 2.0);
        renderer.draw_text(&self.text, origin, &self.font, self.text_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphics_core::{SoftwareRenderer, Surface};

    #[test]
    fn label_basic() {
        let l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Hello");
        assert_eq!(l.text(), "Hello");
    }

    #[test]
    fn label_set_text() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Hello");
        l.set_text("World");
        assert_eq!(l.text(), "World");
        assert!(l.view().needs_display());
    }

    #[test]
    fn label_draw_no_panic() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 80.0, 20.0), "Draw me");
        let mut surface = Surface::new(100, 30);
        let mut renderer = SoftwareRenderer::new(&mut surface);
        l.draw(&mut renderer);
    }

    #[test]
    fn label_intrinsic_content_size() {
        let l = Label::new(Rect::new(0.0, 0.0, 200.0, 30.0), "Some text");
        let size = l.intrinsic_content_size();
        assert!(
            size.width.unwrap() > 0.0,
            "width should be positive for non-empty text"
        );
        assert!(size.height.unwrap() > 0.0, "height should be positive");
    }

    #[test]
    fn label_intrinsic_empty_text() {
        let l = Label::new(Rect::new(0.0, 0.0, 200.0, 30.0), "");
        let size = l.intrinsic_content_size();
        // Width should be minimal (just padding) for empty text
        let l2 = Label::new(Rect::new(0.0, 0.0, 200.0, 30.0), "Hello");
        let size2 = l2.intrinsic_content_size();
        assert!(
            size.width.unwrap() < size2.width.unwrap(),
            "empty text should have smaller width"
        );
    }

    #[test]
    fn label_set_font() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 200.0, 30.0), "Text");
        let size_before = l.intrinsic_content_size();
        l.set_font(Font {
            size: 24.0,
            ..Font::default()
        });
        let size_after = l.intrinsic_content_size();
        // Bigger font should change intrinsic size
        assert_ne!(size_before.width.unwrap(), size_after.width.unwrap());
    }

    #[test]
    fn label_set_text_color() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Colored");
        l.set_text_color(Color::RED);
        // text_color is private, but set_text_color should not panic
        // and should mark needs_display
        assert!(l.view().needs_display());
    }

    #[test]
    fn label_hidden() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Hidden");
        l.view_mut().hidden = true;
        assert!(l.view().hidden);

        // Drawing a hidden label should be a no-op (no panic)
        let mut surface = Surface::new(100, 30);
        let mut renderer = SoftwareRenderer::new(&mut surface);
        l.draw(&mut renderer);
    }

    #[test]
    fn label_view_frame() {
        let frame = Rect::new(10.0, 20.0, 150.0, 40.0);
        let l = Label::new(frame, "Frame test");
        assert_eq!(l.view().frame(), frame);
    }

    #[test]
    fn label_view_tag() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Tagged");
        l.view_mut().tag = 42;
        assert_eq!(l.view().tag, 42);
    }

    #[test]
    fn label_set_text_updates() {
        let mut l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Initial");
        assert_eq!(l.text(), "Initial");
        l.set_text("Updated text content");
        assert_eq!(l.text(), "Updated text content");
    }

    #[test]
    fn label_default_text_color() {
        let l = Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Default color");
        // text_color defaults to BLACK; since it's a private field in the same
        // module, we can access it directly in tests.
        assert_eq!(l.text_color, Color::BLACK);
    }
}
