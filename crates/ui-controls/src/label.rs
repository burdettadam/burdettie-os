//! Label control — displays static text.

use graphics_core::{Color, Font, Point, Rect, Renderer};
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
}
