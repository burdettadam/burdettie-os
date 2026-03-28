//! Backend-agnostic application scene — shared rendering and interaction kernel.
//!
//! `AppScene` holds labels, buttons, text fields, a menu, and a click handler.
//! It provides methods for rendering into a [`Surface`], hit-testing clicks,
//! and dispatching keyboard input — the exact same logic used by both the
//! native `GuiApp` (winit) and `HeadlessApp` (testing).

use app_kit_lite::menu::{ModifierFlags, NSMenu};
use graphics_core::{
    Color, Font, FontManager, Point, Rect, Renderer as _, SoftwareRenderer, Surface,
};
use ui_view::layout::{LayoutConstraint, LayoutEngine};
use window_backend::KeyCode;

use crate::{Button, Label, TextField};

/// Button click handler: receives button index and mutable access to labels/textfields.
pub type ButtonClickHandler = Box<dyn FnMut(usize, &mut Vec<Label>, &mut Vec<TextField>)>;

/// Backend-agnostic scene containing all GUI controls and interaction logic.
///
/// Both `GuiApp` (native windowing) and `HeadlessApp` (testing) delegate
/// to this struct, ensuring identical behaviour regardless of backend.
pub struct AppScene {
    pub(crate) background: Color,
    pub(crate) labels: Vec<Label>,
    pub(crate) buttons: Vec<Button>,
    pub(crate) textfields: Vec<TextField>,
    pub(crate) on_button_click: Option<ButtonClickHandler>,
    pub(crate) main_menu: Option<NSMenu>,
    pub(crate) focused_textfield: Option<usize>,
    pub(crate) modifier_flags: ModifierFlags,
    pub(crate) font_manager: FontManager,
    pub(crate) constraints: Vec<LayoutConstraint>,
}

impl AppScene {
    pub fn new() -> Self {
        Self {
            background: Color::WHITE,
            labels: Vec::new(),
            buttons: Vec::new(),
            textfields: Vec::new(),
            on_button_click: None,
            main_menu: None,
            focused_textfield: None,
            modifier_flags: ModifierFlags::default(),
            font_manager: FontManager::new(),
            constraints: Vec::new(),
        }
    }

    // ── Builder API ────────────────────────────────────

    pub fn set_background(&mut self, color: Color) {
        self.background = color;
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

    pub fn set_main_menu(&mut self, menu: NSMenu) {
        self.main_menu = Some(menu);
    }

    /// Add a layout constraint.
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) {
        self.constraints.push(constraint);
    }

    /// Add multiple layout constraints.
    pub fn add_constraints(&mut self, constraints: impl IntoIterator<Item = LayoutConstraint>) {
        self.constraints.extend(constraints);
    }

    /// Solve layout constraints and update frames of all controls that have
    /// a `layout_id` set. Returns the number of constraint conflicts (0 = success).
    pub fn solve_layout(&mut self, width: f64, height: f64) -> usize {
        if self.constraints.is_empty() {
            return 0;
        }
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, width, height));
        engine.add_constraints(self.constraints.iter().cloned());
        match engine.solve() {
            Ok(frames) => {
                for label in &mut self.labels {
                    if let Some(vid) = label.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            label.view_mut().set_frame(*rect);
                        }
                    }
                }
                for button in &mut self.buttons {
                    if let Some(vid) = button.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            button.view_mut().set_frame(*rect);
                        }
                    }
                }
                for tf in &mut self.textfields {
                    if let Some(vid) = tf.view().layout_id {
                        if let Some(rect) = frames.get(&vid) {
                            tf.view_mut().set_frame(*rect);
                        }
                    }
                }
                0
            }
            Err(conflicts) => conflicts.len(),
        }
    }

    // ── Rendering ──────────────────────────────────────

    /// Height of the menu bar (0 if no menu is set).
    pub fn menu_bar_height(&self) -> f64 {
        if self.main_menu.is_some() {
            22.0
        } else {
            0.0
        }
    }

    /// Render the entire scene into a surface of the given logical dimensions.
    pub fn render(&mut self, width: u32, height: u32) -> Surface {
        let mut surface = Surface::new(width, height);
        {
            let mut renderer = SoftwareRenderer::with_fonts(&mut surface, &self.font_manager);
            renderer.clear(self.background);
            self.draw_menu_bar(&mut renderer, width as f64);
            for label in &mut self.labels {
                label.draw(&mut renderer);
            }
            for button in &mut self.buttons {
                button.draw(&mut renderer);
            }
            for tf in &mut self.textfields {
                tf.draw(&mut renderer);
            }
        }
        surface
    }

    /// Draw the menu bar into the renderer, if a menu is set.
    fn draw_menu_bar(&self, renderer: &mut SoftwareRenderer<'_>, width: f64) {
        let Some(ref menu) = self.main_menu else {
            return;
        };
        let mb_height = self.menu_bar_height();
        let menu_bg = Color::new(0.95, 0.95, 0.95, 1.0);
        let menu_rect = Rect::new(0.0, 0.0, width, mb_height);
        renderer.fill_rect(menu_rect, menu_bg);
        let font = Font::system(12.0);
        let sys_font = self.font_manager.system();
        let metrics = sys_font.metrics(font.size);
        let text_y = (mb_height - metrics.line_height()) / 2.0 + metrics.ascent;
        let mut x = 12.0;
        for item in menu.items() {
            if !item.is_separator() {
                renderer.draw_text(item.title(), Point::new(x, text_y), &font, Color::BLACK);
                let item_width = sys_font.measure_text(item.title(), font.size);
                x += item_width + 16.0;
            }
        }
        renderer.draw_line(
            Point::new(0.0, mb_height),
            Point::new(width, mb_height),
            Color::new(0.8, 0.8, 0.8, 1.0),
            1.0,
        );
    }

    // ── Input handling ─────────────────────────────────

    /// Handle a click at the given point. Updates textfield focus and
    /// fires button actions. Returns `true` if something was handled.
    pub fn handle_click(&mut self, point: Point) -> bool {
        // Check textfield focus
        self.focused_textfield = self.textfields.iter().enumerate().find_map(|(i, tf)| {
            if tf.is_editable() && tf.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        let clicked_idx = self.buttons.iter().enumerate().find_map(|(i, btn)| {
            if btn.is_enabled() && btn.view().frame.contains(point) {
                Some(i)
            } else {
                None
            }
        });

        if let Some(idx) = clicked_idx {
            self.buttons[idx].perform_click();
            if let Some(ref mut handler) = self.on_button_click {
                handler(idx, &mut self.labels, &mut self.textfields);
            }
            return true;
        }
        false
    }

    /// Update the current modifier key state.
    pub fn set_modifier_flags(&mut self, flags: ModifierFlags) {
        self.modifier_flags = flags;
    }

    /// Handle a key press. Checks menu shortcuts first, then dispatches
    /// to the focused text field. Returns `true` if anything was consumed.
    pub fn handle_key_down(&mut self, key: KeyCode) -> bool {
        // Menu key equivalents first
        if let Some(ref mut menu) = self.main_menu {
            if menu.perform_key_equivalent(key, self.modifier_flags) {
                return true;
            }
        }
        false
    }

    /// Handle character input (typed text). Inserts into the focused
    /// text field. Returns `true` if consumed.
    pub fn handle_character_input(&mut self, text: &str) -> bool {
        let Some(idx) = self.focused_textfield else {
            return false;
        };
        if idx >= self.textfields.len() {
            return false;
        }
        for ch in text.chars() {
            self.textfields[idx].insert_char(ch);
        }
        true
    }

    /// Handle backspace in the focused text field. Returns `true` if consumed.
    pub fn handle_backspace(&mut self) -> bool {
        let Some(idx) = self.focused_textfield else {
            return false;
        };
        if idx >= self.textfields.len() {
            return false;
        }
        self.textfields[idx].delete_backward();
        true
    }

    // ── Inspection (for headless testing) ──────────────

    pub fn label_text(&self, idx: usize) -> &str {
        self.labels[idx].text()
    }

    pub fn textfield_text(&self, idx: usize) -> &str {
        self.textfields[idx].text()
    }

    pub fn button_title(&self, idx: usize) -> &str {
        self.buttons[idx].title()
    }

    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    pub fn textfield_count(&self) -> usize {
        self.textfields.len()
    }

    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    pub fn textfields(&self) -> &[TextField] {
        &self.textfields
    }
}

impl Default for AppScene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_basic() {
        let mut scene = AppScene::new();
        scene.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "Hello"));
        scene.add_button(Button::new(Rect::new(0.0, 30.0, 80.0, 25.0), "OK"));
        scene.add_textfield(TextField::new(Rect::new(0.0, 60.0, 200.0, 24.0)));
        assert_eq!(scene.label_count(), 1);
        assert_eq!(scene.button_count(), 1);
        assert_eq!(scene.textfield_count(), 1);
    }

    #[test]
    fn scene_render() {
        let mut scene = AppScene::new();
        scene.set_background(Color::WHITE);
        scene.add_label(Label::new(Rect::new(10.0, 10.0, 100.0, 20.0), "Test"));
        let surface = scene.render(200, 100);
        assert_eq!(surface.width(), 200);
        assert_eq!(surface.height(), 100);
    }

    #[test]
    fn scene_render_with_menu() {
        let mut scene = AppScene::new();
        let mut menu = NSMenu::new("Main");
        let item = app_kit_lite::menu::NSMenuItem::new("File");
        menu.add_item(item);
        scene.set_main_menu(menu);
        assert_eq!(scene.menu_bar_height(), 22.0);
        let surface = scene.render(300, 200);
        assert_eq!(surface.width(), 300);
    }

    #[test]
    fn scene_click_button() {
        let mut scene = AppScene::new();
        scene.add_label(Label::new(Rect::new(0.0, 0.0, 100.0, 20.0), "before"));
        scene.add_button(Button::new(Rect::new(10.0, 30.0, 80.0, 25.0), "Go"));
        scene.on_button_click(|_idx, labels, _tfs| {
            labels[0].set_text("after");
        });

        assert_eq!(scene.label_text(0), "before");
        let hit = scene.handle_click(Point::new(50.0, 40.0));
        assert!(hit);
        assert_eq!(scene.label_text(0), "after");
    }

    #[test]
    fn scene_click_miss() {
        let mut scene = AppScene::new();
        scene.add_button(Button::new(Rect::new(10.0, 10.0, 80.0, 25.0), "Miss"));
        let hit = scene.handle_click(Point::new(200.0, 200.0));
        assert!(!hit);
    }

    #[test]
    fn scene_textfield_focus_and_input() {
        let mut scene = AppScene::new();
        scene.add_textfield(TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0)));

        // Click on textfield to focus it
        scene.handle_click(Point::new(50.0, 20.0));
        assert_eq!(scene.focused_textfield, Some(0));

        // Type characters
        assert!(scene.handle_character_input("hello"));
        assert_eq!(scene.textfield_text(0), "hello");

        // Backspace
        assert!(scene.handle_backspace());
        assert_eq!(scene.textfield_text(0), "hell");
    }

    #[test]
    fn scene_no_focus_no_input() {
        let mut scene = AppScene::new();
        scene.add_textfield(TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0)));
        // No click — no focus
        assert!(!scene.handle_character_input("x"));
        assert_eq!(scene.textfield_text(0), "");
    }

    #[test]
    fn scene_menu_key_equivalent() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let fired = Arc::new(AtomicBool::new(false));
        let f = fired.clone();

        let mut scene = AppScene::new();
        let mut menu = NSMenu::new("Main");
        let mut file_item = app_kit_lite::menu::NSMenuItem::new("File");
        let mut file_menu = NSMenu::new("File");
        file_menu.add_item(app_kit_lite::menu::NSMenuItem::with_action(
            "Quit",
            Some(app_kit_lite::menu::KeyEquivalent::command(KeyCode::Q)),
            move || {
                f.store(true, Ordering::SeqCst);
            },
        ));
        file_item.set_submenu(file_menu);
        menu.add_item(file_item);
        scene.set_main_menu(menu);

        scene.set_modifier_flags(ModifierFlags::COMMAND);
        let handled = scene.handle_key_down(KeyCode::Q);
        assert!(handled);
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn scene_button_textfield_interaction() {
        let mut scene = AppScene::new();
        scene.add_label(Label::new(Rect::new(0.0, 0.0, 200.0, 20.0), ""));
        scene.add_textfield(TextField::new(Rect::new(0.0, 30.0, 200.0, 25.0)));
        scene.add_button(Button::new(Rect::new(0.0, 60.0, 80.0, 25.0), "Echo"));

        scene.on_button_click(|_idx, labels, tfs| {
            let text = tfs[0].text().to_string();
            labels[0].set_text(format!("Echo: {text}"));
        });

        // Focus the textfield and type
        scene.handle_click(Point::new(50.0, 40.0));
        scene.handle_character_input("test");

        // Click the button
        scene.handle_click(Point::new(40.0, 70.0));
        assert_eq!(scene.label_text(0), "Echo: test");
    }
}
