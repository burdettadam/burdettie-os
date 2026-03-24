//! TextField control — single-line editable text input.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::View;

/// A single-line text field for user input.
pub struct TextField {
    view: View,
    text: String,
    placeholder: String,
    font: Font,
    text_color: Color,
    placeholder_color: Color,
    editable: bool,
    cursor_position: usize,
}

impl TextField {
    /// Create a text field with the given frame.
    pub fn new(frame: Rect) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::WHITE;
        Self {
            view,
            text: String::new(),
            placeholder: String::new(),
            font: Font::default(),
            text_color: Color::BLACK,
            placeholder_color: Color::new(0.6, 0.6, 0.6, 1.0),
            editable: true,
            cursor_position: 0,
        }
    }

    /// Get the current text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_position = self.text.len();
        self.view.set_needs_display();
    }

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set the placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
        self.view.set_needs_display();
    }

    /// Whether the text field is editable.
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Enable or disable editing.
    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// Current cursor position (character index).
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        if !self.editable {
            return;
        }
        if self.cursor_position <= self.text.len() {
            self.text.insert(self.cursor_position, c);
            self.cursor_position += c.len_utf8();
            self.view.set_needs_display();
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_backward(&mut self) {
        if !self.editable || self.cursor_position == 0 {
            return;
        }
        // Find the previous character boundary
        let prev = self.text[..self.cursor_position]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.remove(prev);
        self.cursor_position = prev;
        self.view.set_needs_display();
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
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

    /// Draw the text field.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        renderer.fill_rect(self.view.frame, self.view.background_color);

        // Draw border
        renderer.stroke_rect(self.view.frame, Color::new(0.7, 0.7, 0.7, 1.0), 1.0);

        let origin = Point::new(self.view.frame.x + 4.0, self.view.frame.y + 2.0);

        if self.text.is_empty() && !self.placeholder.is_empty() {
            // Draw placeholder
            renderer.draw_text(
                &self.placeholder,
                origin,
                &self.font,
                self.placeholder_color,
            );
        } else {
            // Draw text
            renderer.draw_text(&self.text, origin, &self.font, self.text_color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textfield_basic() {
        let tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        assert_eq!(tf.text(), "");
        assert!(tf.is_editable());
    }

    #[test]
    fn textfield_insert_chars() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.insert_char('H');
        tf.insert_char('i');
        assert_eq!(tf.text(), "Hi");
        assert_eq!(tf.cursor_position(), 2);
    }

    #[test]
    fn textfield_delete_backward() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("abc");
        tf.delete_backward();
        assert_eq!(tf.text(), "ab");
        assert_eq!(tf.cursor_position(), 2);
    }

    #[test]
    fn textfield_not_editable() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_editable(false);
        tf.insert_char('X');
        assert_eq!(tf.text(), "");
    }

    #[test]
    fn textfield_placeholder() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_placeholder("Enter text...");
        assert_eq!(tf.placeholder(), "Enter text...");
    }
}
