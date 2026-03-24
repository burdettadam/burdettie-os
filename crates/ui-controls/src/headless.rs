//! Headless GUI application for testing.
//!
//! [`HeadlessApp`] mirrors the builder API of `GuiApp` from
//! `window-backend-darwin` but never opens a real window. Tests drive
//! interactions programmatically via `click_button()`, `type_in_textfield()`,
//! etc., and inspect state via `label_text()`, `textfield_text()`, etc.

use graphics_core::Point;

use crate::{Button, Label, TextField};

/// Button click handler type — matches the one in `window-backend-darwin`.
type ButtonClickHandler = Box<dyn FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>)>;

/// A headless GUI application for testing.
///
/// # Example
///
/// ```
/// use graphics_core::Rect;
/// use ui_controls::{HeadlessApp, Label, Button};
///
/// let mut app = HeadlessApp::default();
/// app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "before"));
/// app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "Go"));
/// app.on_button_click(|_idx, labels, _tfs| {
///     labels[0].set_text("after");
/// });
///
/// app.click_button(0);
/// assert_eq!(app.label_text(0), "after");
/// ```
pub struct HeadlessApp {
    labels: Vec<Label>,
    buttons: Vec<Button>,
    textfields: Vec<TextField>,
    on_button_click: Option<ButtonClickHandler>,
}

impl Default for HeadlessApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessApp {
    pub fn new() -> Self {
        Self {
            labels: Vec::new(),
            buttons: Vec::new(),
            textfields: Vec::new(),
            on_button_click: None,
        }
    }

    // ── Builder API (same as GuiApp) ──────────────────

    pub fn set_background(&mut self, _color: graphics_core::Color) {
        // no-op in headless mode
    }

    pub fn add_label(&mut self, label: Label) {
        self.labels.push(label);
    }

    pub fn add_button(&mut self, button: Button) {
        self.buttons.push(button);
    }

    pub fn add_textfield(&mut self, tf: TextField) {
        self.textfields.push(tf);
    }

    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.on_button_click = Some(Box::new(handler));
    }

    // ── Test driver API ───────────────────────────────

    /// Simulate clicking button at `idx`. Fires `perform_click()` on the
    /// button and invokes the `on_button_click` handler with mutable access
    /// to labels and text fields — exactly as `GuiApp` does on mouse click.
    ///
    /// Panics if `idx` is out of bounds.
    pub fn click_button(&mut self, idx: usize) {
        assert!(idx < self.buttons.len(), "button index {idx} out of bounds");
        self.buttons[idx].perform_click();
        if let Some(ref mut handler) = self.on_button_click {
            handler(idx, &mut self.labels, &mut self.textfields);
        }
    }

    /// Simulate clicking at a logical coordinate. Finds the first button
    /// whose frame contains the point and clicks it.
    pub fn click_at(&mut self, x: f64, y: f64) {
        let point = Point::new(x, y);
        let idx = self.buttons.iter().enumerate().find_map(|(i, btn)| {
            if btn.is_enabled() && btn.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });
        if let Some(idx) = idx {
            self.click_button(idx);
        }
    }

    /// Type text into textfield at `idx` — inserts each character.
    pub fn type_in_textfield(&mut self, idx: usize, text: &str) {
        assert!(
            idx < self.textfields.len(),
            "textfield index {idx} out of bounds"
        );
        for ch in text.chars() {
            self.textfields[idx].insert_char(ch);
        }
    }

    /// Clear textfield at `idx`.
    pub fn clear_textfield(&mut self, idx: usize) {
        assert!(
            idx < self.textfields.len(),
            "textfield index {idx} out of bounds"
        );
        self.textfields[idx].set_text("");
    }

    // ── Inspection API ────────────────────────────────

    /// Get the text of label at `idx`.
    pub fn label_text(&self, idx: usize) -> &str {
        self.labels[idx].text()
    }

    /// Get the text of textfield at `idx`.
    pub fn textfield_text(&self, idx: usize) -> &str {
        self.textfields[idx].text()
    }

    /// Get the title of button at `idx`.
    pub fn button_title(&self, idx: usize) -> &str {
        self.buttons[idx].title()
    }

    /// Number of labels.
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    /// Number of buttons.
    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    /// Number of text fields.
    pub fn textfield_count(&self) -> usize {
        self.textfields.len()
    }

    /// Access to labels slice.
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Access to buttons slice.
    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    /// Access to text fields slice.
    pub fn textfields(&self) -> &[TextField] {
        &self.textfields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphics_core::Rect;

    #[test]
    fn add_controls() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Label 1"));
        app.add_label(Label::new(Rect::new(0.0, 30.0, 100.0, 20.0), "Label 2"));
        app.add_button(Button::new(Rect::new(0.0, 60.0, 80.0, 25.0), "OK"));
        app.add_textfield(TextField::new(Rect::new(0.0, 90.0, 200.0, 25.0)));

        assert_eq!(app.label_count(), 2);
        assert_eq!(app.button_count(), 1);
        assert_eq!(app.textfield_count(), 1);
        assert_eq!(app.label_text(0), "Label 1");
        assert_eq!(app.label_text(1), "Label 2");
        assert_eq!(app.button_title(0), "OK");
    }

    #[test]
    fn click_button_fires_handler() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "before"));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "Go"));

        app.on_button_click(|idx, labels, _tfs| {
            assert_eq!(idx, 0);
            labels[0].set_text("after");
        });

        assert_eq!(app.label_text(0), "before");
        app.click_button(0);
        assert_eq!(app.label_text(0), "after");
    }

    #[test]
    fn click_at_coordinate() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), ""));
        app.add_button(Button::new(Rect::new(10.0, 30.0, 80.0, 25.0), "Hit"));

        app.on_button_click(|_idx, labels, _tfs| {
            labels[0].set_text("hit!");
        });

        // Click outside — should not fire
        app.click_at(100.0, 100.0);
        assert_eq!(app.label_text(0), "");

        // Click inside the button
        app.click_at(50.0, 40.0);
        assert_eq!(app.label_text(0), "hit!");
    }

    #[test]
    fn type_in_textfield() {
        let mut app = HeadlessApp::new();
        app.add_textfield(TextField::new(Rect::new(0.0, 0.0, 200.0, 25.0)));

        app.type_in_textfield(0, "hello");
        assert_eq!(app.textfield_text(0), "hello");

        app.type_in_textfield(0, " world");
        assert_eq!(app.textfield_text(0), "hello world");

        app.clear_textfield(0);
        assert_eq!(app.textfield_text(0), "");
    }

    #[test]
    fn button_textfield_interaction() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), ""));
        app.add_textfield(TextField::new(Rect::new(0.0, 30.0, 200.0, 25.0)));
        app.add_button(Button::new(Rect::new(0.0, 60.0, 80.0, 25.0), "Echo"));

        app.on_button_click(|_idx, labels, tfs| {
            let text = tfs[0].text().to_string();
            labels[0].set_text(format!("Echo: {text}"));
        });

        app.type_in_textfield(0, "test");
        app.click_button(0);
        assert_eq!(app.label_text(0), "Echo: test");
    }

    #[test]
    fn multiple_buttons() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), ""));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "A"));
        app.add_button(Button::new(Rect::new(100.0, 30.0, 80.0, 25.0), "B"));

        app.on_button_click(|idx, labels, _tfs| {
            labels[0].set_text(format!("clicked {idx}"));
        });

        app.click_button(1);
        assert_eq!(app.label_text(0), "clicked 1");

        app.click_button(0);
        assert_eq!(app.label_text(0), "clicked 0");
    }

    #[test]
    fn click_at_misses_disabled_button() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), ""));
        let mut btn = Button::new(Rect::new(10.0, 30.0, 80.0, 25.0), "Disabled");
        btn.set_enabled(false);
        app.add_button(btn);

        app.on_button_click(|_idx, labels, _tfs| {
            labels[0].set_text("should not fire");
        });

        app.click_at(50.0, 40.0);
        assert_eq!(app.label_text(0), "");
    }
}
