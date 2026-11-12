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
use crate::{
    Button, Checkbox, ComboBox, ImageView, Label, ProgressBar, RadioButton, Slider, TabView,
    TextField,
};

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

    pub fn add_checkbox(&mut self, cb: Checkbox) {
        self.scene.add_checkbox(cb);
    }

    pub fn add_radiobutton(&mut self, rb: RadioButton) {
        self.scene.add_radiobutton(rb);
    }

    pub fn add_slider(&mut self, slider: Slider) {
        self.scene.add_slider(slider);
    }

    pub fn add_progressbar(&mut self, pb: ProgressBar) {
        self.scene.add_progressbar(pb);
    }

    pub fn add_combobox(&mut self, cb: ComboBox) {
        self.scene.add_combobox(cb);
    }

    pub fn add_tabview(&mut self, tv: TabView) {
        self.scene.add_tabview(tv);
    }

    pub fn add_imageview(&mut self, iv: ImageView) {
        self.scene.add_imageview(iv);
    }

    pub fn on_button_click(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>) + 'static,
    ) {
        self.scene.on_button_click(handler);
    }

    pub fn on_button_click_with_buttons(
        &mut self,
        handler: impl FnMut(usize, &mut Vec<Label>, &mut Vec<Button>, &mut Vec<TextField>) + 'static,
    ) {
        self.scene.on_button_click_with_buttons(handler);
    }

    pub fn on_key_down(
        &mut self,
        handler: impl FnMut(
                window_backend::KeyCode,
                &mut Vec<Label>,
                &mut Vec<Button>,
                &mut Vec<TextField>,
            ) -> bool
            + 'static,
    ) {
        self.scene.on_key_down(handler);
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
            handler(
                idx,
                &mut self.scene.labels,
                &mut self.scene.buttons,
                &mut self.scene.textfields,
            );
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

    pub fn checkboxes(&self) -> &[Checkbox] {
        self.scene.checkboxes()
    }

    pub fn checkbox_count(&self) -> usize {
        self.scene.checkbox_count()
    }

    pub fn radiobuttons(&self) -> &[RadioButton] {
        self.scene.radiobuttons()
    }

    pub fn radiobutton_count(&self) -> usize {
        self.scene.radiobutton_count()
    }

    pub fn sliders(&self) -> &[Slider] {
        self.scene.sliders()
    }

    pub fn slider_count(&self) -> usize {
        self.scene.slider_count()
    }

    pub fn progressbars(&self) -> &[ProgressBar] {
        self.scene.progressbars()
    }

    pub fn progressbar_count(&self) -> usize {
        self.scene.progressbar_count()
    }

    pub fn comboboxes(&self) -> &[ComboBox] {
        self.scene.comboboxes()
    }

    pub fn combobox_count(&self) -> usize {
        self.scene.combobox_count()
    }

    pub fn tabviews(&self) -> &[TabView] {
        self.scene.tabviews()
    }

    pub fn tabview_count(&self) -> usize {
        self.scene.tabview_count()
    }

    pub fn imageviews(&self) -> &[ImageView] {
        self.scene.imageviews()
    }

    pub fn imageview_count(&self) -> usize {
        self.scene.imageview_count()
    }

    pub fn add_scrollview(&mut self, sv: crate::ScrollView) {
        self.scene.add_scrollview(sv);
    }

    pub fn scrollviews(&self) -> &[crate::ScrollView] {
        self.scene.scrollviews()
    }

    pub fn scrollview_count(&self) -> usize {
        self.scene.scrollview_count()
    }

    /// Simulate a scroll event with the given delta.
    pub fn scroll(&mut self, dx: f64, dy: f64) -> bool {
        self.scene.handle_scroll(dx, dy)
    }

    /// Simulate a key press.
    pub fn key_down(&mut self, key: window_backend::KeyCode) -> bool {
        self.scene.handle_key_down(key)
    }

    /// Get the currently focused textfield index, if any.
    pub fn focused_textfield_index(&self) -> Option<usize> {
        self.scene.focused_textfield_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Checkbox, ComboBox, ImageView, ProgressBar, RadioButton, Slider, TabView};
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

    #[test]
    fn add_checkbox_and_count() {
        let mut app = HeadlessApp::new();
        app.add_checkbox(Checkbox::new(Rect::new(0.0, 0.0, 120.0, 20.0), "Option A"));
        app.add_checkbox(Checkbox::new(Rect::new(0.0, 30.0, 120.0, 20.0), "Option B"));
        assert_eq!(app.checkbox_count(), 2);
        assert_eq!(app.checkboxes()[0].title(), "Option A");
        assert_eq!(app.checkboxes()[1].title(), "Option B");
    }

    #[test]
    fn add_slider_and_count() {
        let mut app = HeadlessApp::new();
        let slider = Slider::new(Rect::new(0.0, 0.0, 200.0, 20.0), 0.0, 100.0);
        app.add_slider(slider);
        assert_eq!(app.slider_count(), 1);
        assert_eq!(app.sliders()[0].value(), 0.0);
        assert_eq!(app.sliders()[0].max_value(), 100.0);
    }

    #[test]
    fn add_progressbar_and_count() {
        let mut app = HeadlessApp::new();
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(0.5);
        app.add_progressbar(pb);
        assert_eq!(app.progressbar_count(), 1);
        assert!((app.progressbars()[0].progress() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn add_combobox_and_count() {
        let mut app = HeadlessApp::new();
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 150.0, 25.0));
        cb.add_item("Red");
        cb.add_item("Green");
        cb.add_item("Blue");
        app.add_combobox(cb);
        assert_eq!(app.combobox_count(), 1);
        assert_eq!(app.comboboxes()[0].item_count(), 3);
    }

    #[test]
    fn add_tabview_and_count() {
        let mut app = HeadlessApp::new();
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 300.0, 200.0));
        tv.add_tab("Tab 1");
        tv.add_tab("Tab 2");
        app.add_tabview(tv);
        assert_eq!(app.tabview_count(), 1);
        assert_eq!(app.tabviews()[0].tab_count(), 2);
    }

    #[test]
    fn add_imageview_and_count() {
        let mut app = HeadlessApp::new();
        app.add_imageview(ImageView::new(Rect::new(0.0, 0.0, 64.0, 64.0)));
        assert_eq!(app.imageview_count(), 1);
        assert!(!app.imageviews()[0].has_image());
    }

    #[test]
    fn add_radiobutton_and_count() {
        let mut app = HeadlessApp::new();
        app.add_radiobutton(RadioButton::new(
            Rect::new(0.0, 0.0, 120.0, 20.0),
            "Choice A",
            1,
        ));
        assert_eq!(app.radiobutton_count(), 1);
        assert_eq!(app.radiobuttons()[0].title(), "Choice A");
        assert_eq!(app.radiobuttons()[0].group_id(), 1);
    }

    #[test]
    fn render_with_all_controls() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Lbl"));
        app.add_button(Button::new(Rect::new(0.0, 25.0, 80.0, 25.0), "Btn"));
        app.add_textfield(TextField::new(Rect::new(0.0, 55.0, 200.0, 24.0)));
        app.add_checkbox(Checkbox::new(Rect::new(0.0, 85.0, 120.0, 20.0), "CB"));
        app.add_radiobutton(RadioButton::new(
            Rect::new(0.0, 110.0, 120.0, 20.0),
            "RB",
            0,
        ));
        app.add_slider(Slider::new(Rect::new(0.0, 135.0, 200.0, 20.0), 0.0, 1.0));
        app.add_progressbar(ProgressBar::new(Rect::new(0.0, 160.0, 200.0, 8.0)));
        app.add_combobox(ComboBox::new(Rect::new(0.0, 175.0, 150.0, 25.0)));
        app.add_tabview(TabView::new(Rect::new(0.0, 205.0, 300.0, 100.0)));
        app.add_imageview(ImageView::new(Rect::new(0.0, 310.0, 64.0, 64.0)));
        let surface = app.render(400, 400);
        assert_eq!(surface.width(), 400);
        assert_eq!(surface.height(), 400);
    }

    #[test]
    fn mixed_controls_scene() {
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Title"));
        app.add_button(Button::new(Rect::new(0.0, 25.0, 80.0, 25.0), "Go"));
        app.add_checkbox(Checkbox::new(Rect::new(0.0, 55.0, 120.0, 20.0), "Check"));
        app.add_slider(Slider::new(Rect::new(0.0, 80.0, 200.0, 20.0), 0.0, 10.0));
        assert_eq!(app.label_count(), 1);
        assert_eq!(app.button_count(), 1);
        assert_eq!(app.checkbox_count(), 1);
        assert_eq!(app.slider_count(), 1);
    }

    #[test]
    fn scroll_wheel_forwarded_to_scrollview() {
        use crate::ScrollView;
        let mut app = HeadlessApp::new();
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(400.0, 300.0);
        app.add_scrollview(sv);

        assert_eq!(app.scrollview_count(), 1);
        // Initially at origin
        let offset = app.scrollviews()[0].content_offset();
        assert!((offset.x).abs() < f64::EPSILON);
        assert!((offset.y).abs() < f64::EPSILON);

        // Scroll by positive delta
        let consumed = app.scroll(10.0, 20.0);
        assert!(consumed);
        let offset = app.scrollviews()[0].content_offset();
        assert!((offset.x - 10.0).abs() < f64::EPSILON);
        assert!((offset.y - 20.0).abs() < f64::EPSILON);

        // Scroll again (accumulates)
        app.scroll(5.0, 5.0);
        let offset = app.scrollviews()[0].content_offset();
        assert!((offset.x - 15.0).abs() < f64::EPSILON);
        assert!((offset.y - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scroll_no_scrollview_is_noop() {
        let mut app = HeadlessApp::new();
        let consumed = app.scroll(10.0, 10.0);
        assert!(!consumed);
    }

    #[test]
    fn tab_cycles_textfield_focus() {
        use window_backend::KeyCode;
        let mut app = HeadlessApp::new();
        app.add_textfield(TextField::new(Rect::new(0.0, 0.0, 200.0, 24.0)));
        app.add_textfield(TextField::new(Rect::new(0.0, 30.0, 200.0, 24.0)));

        // Initially no focus
        assert_eq!(app.focused_textfield_index(), None);

        // Tab focuses first textfield
        assert!(app.key_down(KeyCode::Tab));
        assert_eq!(app.focused_textfield_index(), Some(0));

        // Tab cycles to second
        assert!(app.key_down(KeyCode::Tab));
        assert_eq!(app.focused_textfield_index(), Some(1));

        // Tab wraps back to first
        assert!(app.key_down(KeyCode::Tab));
        assert_eq!(app.focused_textfield_index(), Some(0));
    }

    #[test]
    fn tab_no_textfields_is_noop() {
        use window_backend::KeyCode;
        let mut app = HeadlessApp::new();
        let consumed = app.key_down(KeyCode::Tab);
        assert!(!consumed);
    }

    #[test]
    fn space_activates_button() {
        use window_backend::KeyCode;
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), "before"));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "Act"));
        app.on_button_click(|_idx, labels, _tfs| {
            labels[0].set_text("activated");
        });

        assert_eq!(app.label_text(0), "before");
        assert!(app.key_down(KeyCode::Space));
        assert_eq!(app.label_text(0), "activated");
    }

    #[test]
    fn return_activates_button() {
        use window_backend::KeyCode;
        let mut app = HeadlessApp::new();
        app.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), "before"));
        app.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "Act"));
        app.on_button_click(|_idx, labels, _tfs| {
            labels[0].set_text("done");
        });

        assert!(app.key_down(KeyCode::Return));
        assert_eq!(app.label_text(0), "done");
    }

    #[test]
    fn arrow_keys_adjust_slider() {
        use window_backend::KeyCode;
        let mut app = HeadlessApp::new();
        app.add_slider(Slider::new(Rect::new(0.0, 0.0, 200.0, 20.0), 0.0, 100.0));
        // Slider starts at 0.0

        // Right arrow increases by 1% of range = 1.0
        assert!(app.key_down(KeyCode::Right));
        assert!((app.sliders()[0].value() - 1.0).abs() < f64::EPSILON);

        // Left arrow decreases by 1% of range = 1.0
        assert!(app.key_down(KeyCode::Left));
        assert!((app.sliders()[0].value() - 0.0).abs() < f64::EPSILON);

        // Left at minimum stays at minimum (clamped)
        assert!(app.key_down(KeyCode::Left));
        assert!((app.sliders()[0].value() - 0.0).abs() < f64::EPSILON);
    }
}
