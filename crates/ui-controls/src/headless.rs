//! Headless GUI application for testing.
//!
//! [`HeadlessApp`] mirrors the builder API of `GuiApp` from
//! `window-backend-darwin` but never opens a real window. Tests drive
//! interactions programmatically via `click_button()`, `type_in_textfield()`,
//! etc., and inspect state via `label_text()`, `textfield_text()`, etc.
//!
//! Internally delegates to [`AppScene`] — the same backend-agnostic rendering
//! kernel that the native `GuiApp` uses.

use graphics_core::Point;

use crate::scene::AppScene;
use crate::{Button, Label, TextField};

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
    scene: AppScene,
}

impl Default for HeadlessApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessApp {
    pub fn new() -> Self {
        Self {
            scene: AppScene::new(),
        }
    }

    /// Access the underlying scene (for advanced use / testing).
    pub fn scene(&self) -> &AppScene {
        &self.scene
    }

    /// Mutable access to the underlying scene.
    pub fn scene_mut(&mut self) -> &mut AppScene {
        &mut self.scene
    }

    // ── Builder API (same as GuiApp) ──────────────────

    pub fn set_background(&mut self, color: graphics_core::Color) {
        self.scene.set_background(color);
    }

    pub fn add_label(&mut self, label: Label) {
        self.scene.add_label(label);
    }

    pub fn add_button(&mut self, button: Button) {
        self.scene.add_button(button);
    }

    pub fn add_textfield(&mut self, tf: TextField) {
        self.scene.add_textfield(tf);
    }

    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.scene.on_button_click(handler);
    }

    // ── Test driver API ───────────────────────────────

    /// Simulate clicking button at `idx`. Fires `perform_click()` on the
    /// button and invokes the `on_button_click` handler with mutable access
    /// to labels and text fields — exactly as `GuiApp` does on mouse click.
    ///
    /// Panics if `idx` is out of bounds.
    pub fn click_button(&mut self, idx: usize) {
        assert!(
            idx < self.scene.buttons.len(),
            "button index {idx} out of bounds"
        );
        self.scene.buttons[idx].perform_click();
        if let Some(ref mut handler) = self.scene.on_button_click {
            handler(idx, &mut self.scene.labels, &mut self.scene.textfields);
        }
    }

    /// Simulate clicking at a logical coordinate. Finds the first button
    /// whose frame contains the point and clicks it.
    pub fn click_at(&mut self, x: f64, y: f64) {
        self.scene.handle_click(Point::new(x, y));
    }

    /// Type text into textfield at `idx` — inserts each character.
    pub fn type_in_textfield(&mut self, idx: usize, text: &str) {
        assert!(
            idx < self.scene.textfields.len(),
            "textfield index {idx} out of bounds"
        );
        for ch in text.chars() {
            self.scene.textfields[idx].insert_char(ch);
        }
    }

    /// Clear textfield at `idx`.
    pub fn clear_textfield(&mut self, idx: usize) {
        assert!(
            idx < self.scene.textfields.len(),
            "textfield index {idx} out of bounds"
        );
        self.scene.textfields[idx].set_text("");
    }

    /// Render the scene and return the surface (for snapshot / visual testing).
    pub fn render(&mut self, width: u32, height: u32) -> graphics_core::Surface {
        self.scene.render(width, height)
    }

    // ── Inspection API ────────────────────────────────

    pub fn label_text(&self, idx: usize) -> &str {
        self.scene.label_text(idx)
    }

    pub fn textfield_text(&self, idx: usize) -> &str {
        self.scene.textfield_text(idx)
    }

    pub fn button_title(&self, idx: usize) -> &str {
        self.scene.button_title(idx)
    }

    pub fn label_count(&self) -> usize {
        self.scene.label_count()
    }

    pub fn button_count(&self) -> usize {
        self.scene.button_count()
    }

    pub fn textfield_count(&self) -> usize {
        self.scene.textfield_count()
    }

    pub fn labels(&self) -> &[Label] {
        self.scene.labels()
    }

    pub fn buttons(&self) -> &[Button] {
        self.scene.buttons()
    }

    pub fn textfields(&self) -> &[TextField] {
        self.scene.textfields()
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

    #[test]
    fn headless_render_produces_surface() {
        let mut app = HeadlessApp::new();
        app.set_background(graphics_core::Color::WHITE);
        app.add_label(Label::new(Rect::new(10.0, 10.0, 200.0, 20.0), "Hello"));
        app.add_button(Button::new(Rect::new(10.0, 40.0, 80.0, 25.0), "OK"));
        let surface = app.render(300, 200);
        assert_eq!(surface.width(), 300);
        assert_eq!(surface.height(), 200);
        // Should not be all-black (we cleared to white)
        assert!(surface.pixels().iter().any(|&p| p != 0));
    }

    #[test]
    fn headless_scene_access() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Test"));
        // Access scene for advanced operations
        assert_eq!(app.scene().label_count(), 1);
        app.scene_mut().set_background(graphics_core::Color::BLACK);
    }

    #[test]
    fn headless_textfield_via_scene() {
        let mut app = HeadlessApp::new();
        app.add_textfield(TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0)));
        // Focus the textfield via click on scene
        app.click_at(50.0, 20.0);
        // Type via scene
        app.scene_mut().handle_character_input("hello");
        assert_eq!(app.textfield_text(0), "hello");
        // Backspace via scene
        app.scene_mut().handle_backspace();
        assert_eq!(app.textfield_text(0), "hell");
    }
}
