//! TextField control — single-line editable text input.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
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
    selection_start: Option<usize>,
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
            selection_start: None,
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

    /// Current cursor position (byte offset).
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Set cursor position, clamped to text length.
    pub fn set_cursor_position(&mut self, pos: usize) {
        self.cursor_position = pos.min(self.text.len());
    }

    /// Returns the selected substring, if a selection is active.
    pub fn selected_text(&self) -> Option<&str> {
        let (start, end) = self.selection_range()?;
        Some(&self.text[start..end])
    }

    /// Returns (start, end) byte offsets of the selection, normalized so start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.selection_start?;
        let a = sel.min(self.cursor_position);
        let b = sel.max(self.cursor_position);
        Some((a, b))
    }

    /// Select the entire text.
    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_position = self.text.len();
        self.view.set_needs_display();
    }

    /// Clear the current selection.
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.view.set_needs_display();
    }

    /// Copy the currently selected text.
    pub fn copy_selection(&self) -> Option<String> {
        self.selected_text().map(ToOwned::to_owned)
    }

    /// Delete the active selection. Returns `true` if any text was removed.
    pub fn delete_selection(&mut self) -> bool {
        if !self.editable {
            return false;
        }

        if self.delete_selection_range().is_some() {
            self.view.set_needs_display();
            return true;
        }

        false
    }

    /// Cut the currently selected text.
    pub fn cut_selection(&mut self) -> Option<String> {
        if !self.editable {
            return None;
        }

        let selected = self.copy_selection()?;
        self.delete_selection();
        Some(selected)
    }

    /// Insert text at the cursor, replacing any active selection.
    pub fn insert_text(&mut self, text: &str) {
        if !self.editable || text.is_empty() {
            return;
        }

        self.delete_selection_range();
        if self.cursor_position <= self.text.len() {
            self.text.insert_str(self.cursor_position, text);
            self.cursor_position += text.len();
            self.view.set_needs_display();
        }
    }

    /// Paste text into this single-line field, stripping line breaks.
    pub fn paste_text(&mut self, text: &str) -> bool {
        if !self.editable {
            return false;
        }

        let sanitized: String = text
            .chars()
            .filter(|ch| *ch != '\r' && *ch != '\n')
            .collect();

        if sanitized.is_empty() {
            return false;
        }

        self.insert_text(&sanitized);
        true
    }

    /// Insert a character at the cursor. If a selection exists, replace it.
    pub fn insert_at_cursor(&mut self, ch: char) {
        let mut buffer = [0_u8; 4];
        let text = ch.encode_utf8(&mut buffer);
        self.insert_text(text);
    }

    /// Delete at cursor. If selection exists, delete it. Otherwise backspace.
    pub fn delete_at_cursor(&mut self) {
        if !self.editable {
            return;
        }

        if self.delete_selection_range().is_some() {
            self.view.set_needs_display();
            return;
        }

        if self.cursor_position == 0 {
            return;
        }
        let prev = self.text[..self.cursor_position]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.remove(prev);
        self.cursor_position = prev;
        self.view.set_needs_display();
    }

    /// Move cursor left one character. If extend_selection, extend/start selection.
    pub fn move_cursor_left(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        if !extend_selection {
            self.selection_start = None;
        }
        if self.cursor_position > 0 {
            let prev = self.text[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor_position = prev;
        }
        self.view.set_needs_display();
    }

    /// Move cursor right one character. If extend_selection, extend/start selection.
    pub fn move_cursor_right(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        if !extend_selection {
            self.selection_start = None;
        }
        if self.cursor_position < self.text.len() {
            let next = self.text[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_position + i)
                .unwrap_or(self.text.len());
            self.cursor_position = next;
        }
        self.view.set_needs_display();
    }

    /// Move cursor to start (Home). If extend_selection, extend/start selection.
    pub fn move_cursor_to_start(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        if !extend_selection {
            self.selection_start = None;
        }
        self.cursor_position = 0;
        self.view.set_needs_display();
    }

    /// Move cursor to end (End). If extend_selection, extend/start selection.
    pub fn move_cursor_to_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        if !extend_selection {
            self.selection_start = None;
        }
        self.cursor_position = self.text.len();
        self.view.set_needs_display();
    }

    /// Map an X coordinate to a cursor byte offset using approximate char width.
    pub fn cursor_position_for_x(&self, x: f64, char_width: f64) -> usize {
        let char_idx = (x / char_width).round() as usize;
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.insert_at_cursor(c);
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_backward(&mut self) {
        self.delete_at_cursor();
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

    /// Compute the intrinsic content size.
    /// Text fields have a fixed intrinsic height but no intrinsic width.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        let height = self.font.size + 8.0; // font + padding
        IntrinsicSize::height_only(height)
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

    fn delete_selection_range(&mut self) -> Option<(usize, usize)> {
        let (start, end) = self.selection_range()?;
        self.text.replace_range(start..end, "");
        self.cursor_position = start;
        self.selection_start = None;
        Some((start, end))
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

    #[test]
    fn textfield_set_text() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("Hello");
        assert_eq!(tf.text(), "Hello");
        assert_eq!(tf.cursor_position(), 5);
    }

    #[test]
    fn textfield_set_text_resets_cursor() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("abc");
        assert_eq!(tf.cursor_position(), 3);
        tf.set_text("x");
        assert_eq!(tf.cursor_position(), 1);
    }

    #[test]
    fn textfield_delete_at_start() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        // empty textfield — delete_backward should do nothing and not panic
        tf.delete_backward();
        assert_eq!(tf.text(), "");
        assert_eq!(tf.cursor_position(), 0);
    }

    #[test]
    fn textfield_insert_unicode() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.insert_char('é'); // 2-byte UTF-8
        assert_eq!(tf.text(), "é");
        assert_eq!(tf.cursor_position(), 2); // byte length of 'é'
    }

    #[test]
    fn textfield_set_font() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        let font = Font::new("Helvetica", 20.0);
        tf.set_font(font);
        // no panic is the success condition
    }

    #[test]
    fn textfield_intrinsic_content_size() {
        let tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        let size = tf.intrinsic_content_size();
        assert!(size.height.unwrap() > 0.0);
    }

    #[test]
    fn textfield_view_access() {
        let mut tf = TextField::new(Rect::new(10.0, 20.0, 200.0, 24.0));
        let frame = tf.view().frame;
        assert_eq!(frame.x, 10.0);
        assert_eq!(frame.y, 20.0);
        tf.view_mut().hidden = true;
        assert!(tf.view().hidden);
    }

    #[test]
    fn textfield_draw_no_panic() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        let mut surface = graphics_core::Surface::new(200, 24);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        tf.draw(&mut renderer);
    }

    #[test]
    fn textfield_draw_with_text_no_panic() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("Hello world");
        let mut surface = graphics_core::Surface::new(200, 24);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        tf.draw(&mut renderer);
    }

    #[test]
    fn textfield_draw_placeholder_no_panic() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_placeholder("Type here...");
        assert!(tf.text().is_empty());
        let mut surface = graphics_core::Surface::new(200, 24);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        tf.draw(&mut renderer);
    }

    #[test]
    fn textfield_draw_hidden_no_panic() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("secret");
        tf.view_mut().hidden = true;
        let mut surface = graphics_core::Surface::new(200, 24);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        tf.draw(&mut renderer);
    }

    #[test]
    fn textfield_not_editable_delete() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("abc");
        tf.set_editable(false);
        tf.delete_backward();
        assert_eq!(tf.text(), "abc");
        assert_eq!(tf.cursor_position(), 3);
    }

    #[test]
    fn textfield_multiple_deletes() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("abc");
        tf.delete_backward();
        tf.delete_backward();
        tf.delete_backward();
        assert_eq!(tf.text(), "");
        assert_eq!(tf.cursor_position(), 0);
    }

    #[test]
    fn textfield_insert_at_middle() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("ac");
        // cursor is at end (2); delete 'c' to get cursor at 1
        tf.delete_backward();
        assert_eq!(tf.text(), "a");
        assert_eq!(tf.cursor_position(), 1);
        // insert 'b' at position 1
        tf.insert_char('b');
        assert_eq!(tf.text(), "ab");
        // re-append 'c'
        tf.insert_char('c');
        assert_eq!(tf.text(), "abc");
    }

    // --- Selection & cursor tests ---

    #[test]
    fn cursor_starts_at_end() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        assert_eq!(tf.cursor_position(), 5);
    }

    #[test]
    fn set_cursor_position_clamps() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(100);
        assert_eq!(tf.cursor_position(), 5);
    }

    #[test]
    fn select_all_selects_entire_text() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.select_all();
        assert_eq!(tf.selection_range(), Some((0, 5)));
    }

    #[test]
    fn selected_text_returns_substring() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(1);
        tf.selection_start = Some(4);
        assert_eq!(tf.selected_text(), Some("ell"));
    }

    #[test]
    fn clear_selection_removes_it() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.select_all();
        assert!(tf.selection_range().is_some());
        tf.clear_selection();
        assert_eq!(tf.selection_range(), None);
        assert_eq!(tf.selected_text(), None);
    }

    #[test]
    fn insert_at_cursor_middle() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(2);
        tf.insert_at_cursor('x');
        assert_eq!(tf.text(), "hexllo");
    }

    #[test]
    fn insert_replaces_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        // select "ell" (bytes 1..4)
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);
        tf.insert_at_cursor('a');
        assert_eq!(tf.text(), "hao");
    }

    #[test]
    fn delete_at_cursor_removes_char() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(3);
        tf.delete_at_cursor();
        assert_eq!(tf.text(), "helo");
    }

    #[test]
    fn delete_removes_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        // select "ell" (bytes 1..4)
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);
        tf.delete_at_cursor();
        assert_eq!(tf.text(), "ho");
        assert_eq!(tf.cursor_position(), 1);
    }

    #[test]
    fn move_cursor_left_right() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(3);
        tf.move_cursor_left(false);
        assert_eq!(tf.cursor_position(), 2);
        tf.move_cursor_right(false);
        assert_eq!(tf.cursor_position(), 3);
    }

    #[test]
    fn move_cursor_with_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.set_cursor_position(2);
        tf.move_cursor_right(true);
        tf.move_cursor_right(true);
        assert_eq!(tf.selection_range(), Some((2, 4)));
        assert_eq!(tf.selected_text(), Some("ll"));
    }

    #[test]
    fn copy_selection_returns_owned_text() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);

        assert_eq!(tf.copy_selection().as_deref(), Some("ell"));
    }

    #[test]
    fn cut_selection_removes_text() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);

        assert_eq!(tf.cut_selection().as_deref(), Some("ell"));
        assert_eq!(tf.text(), "ho");
        assert_eq!(tf.cursor_position(), 1);
    }

    #[test]
    fn paste_text_replaces_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);

        assert!(tf.paste_text("i"));
        assert_eq!(tf.text(), "hio");
        assert_eq!(tf.cursor_position(), 2);
    }

    #[test]
    fn paste_text_strips_line_breaks() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));

        assert!(tf.paste_text("hello\r\nworld"));
        assert_eq!(tf.text(), "helloworld");
    }

    #[test]
    fn typed_input_replaces_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);

        tf.insert_char('a');
        assert_eq!(tf.text(), "hao");
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        tf.selection_start = Some(1);
        tf.set_cursor_position(4);

        tf.delete_backward();
        assert_eq!(tf.text(), "ho");
        assert_eq!(tf.cursor_position(), 1);
    }

    #[test]
    fn cursor_position_for_x_mapping() {
        let mut tf = TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        tf.set_text("hello");
        // char_width=10.0, x=25.0 → round(2.5) = 3 → byte offset 3
        assert_eq!(tf.cursor_position_for_x(25.0, 10.0), 3);
        // x beyond text
        assert_eq!(tf.cursor_position_for_x(100.0, 10.0), 5);
        // x=0 → char 0
        assert_eq!(tf.cursor_position_for_x(0.0, 10.0), 0);
    }
}
