//! Backend-agnostic application scene — shared rendering and interaction kernel.
//!
//! `AppScene` holds labels, buttons, text fields, a menu, and a click handler.
//! It provides methods for rendering into a [`Surface`], hit-testing clicks,
//! and dispatching keyboard input — the exact same logic used by both the
//! native `GuiApp` (winit) and `HeadlessApp` (testing).

use app_kit_lite::menu::{ModifierFlags, NSMenu};
use graphics_core::{Color, FontManager};
use ui_view::layout::LayoutConstraint;
use window_backend::KeyboardLayout;

use crate::{
    Button, Checkbox, ComboBox, ImageView, Label, ProgressBar, RadioButton, ScrollView, Slider,
    TabView, TextField,
};

/// Button click handler: receives button index and mutable access to labels/buttons/textfields.
pub type ButtonClickHandler =
    Box<dyn FnMut(usize, &mut Vec<Label>, &mut Vec<Button>, &mut Vec<TextField>)>;

/// Custom key-down handler: receives the key and mutable access to labels/buttons/textfields.
/// Return `true` to consume the event and stop default scene navigation.
pub type KeyDownHandler = Box<
    dyn FnMut(
        window_backend::KeyCode,
        &mut Vec<Label>,
        &mut Vec<Button>,
        &mut Vec<TextField>,
    ) -> bool,
>;

/// Keyboard layout change handler.
pub type LayoutChangeHandler = Box<dyn FnMut(KeyboardLayout)>;

/// Backend-agnostic scene containing all GUI controls and interaction logic.
///
/// Both `GuiApp` (native windowing) and `HeadlessApp` (testing) delegate
/// to this struct, ensuring identical behaviour regardless of backend.
pub struct AppScene {
    pub(crate) background: Color,
    pub(crate) labels: Vec<Label>,
    pub(crate) buttons: Vec<Button>,
    pub(crate) textfields: Vec<TextField>,
    pub(crate) checkboxes: Vec<Checkbox>,
    pub(crate) radiobuttons: Vec<RadioButton>,
    pub(crate) sliders: Vec<Slider>,
    pub(crate) progressbars: Vec<ProgressBar>,
    pub(crate) comboboxes: Vec<ComboBox>,
    pub(crate) tabviews: Vec<TabView>,
    pub(crate) imageviews: Vec<ImageView>,
    pub(crate) scrollviews: Vec<ScrollView>,
    pub(crate) on_button_click: Option<ButtonClickHandler>,
    pub(crate) on_key_down: Option<KeyDownHandler>,
    pub(crate) on_layout_change: Option<LayoutChangeHandler>,
    pub(crate) main_menu: Option<NSMenu>,
    pub(crate) focused_textfield: Option<usize>,
    pub(crate) modifier_flags: ModifierFlags,
    pub(crate) font_manager: FontManager,
    pub(crate) constraints: Vec<LayoutConstraint>,
    pub(crate) keyboard_layout: KeyboardLayout,
}

mod impl_scene;

impl Default for AppScene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_kit_lite::{NSPasteboard, NS_PASTEBOARD_TYPE_STRING};
    use graphics_core::{Point, Rect};
    use std::sync::{Mutex, Once};
    use window_backend::KeyCode;

    /// Serialize tests that read/write the shared in-memory pasteboard.
    static CLIPBOARD_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn force_in_memory_pasteboard() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            std::env::set_var("DARWIN_LOADER_FORCE_IN_MEMORY_PASTEBOARD", "1");
        });
    }

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

    #[test]
    fn scene_command_a_selects_all_text() {
        let mut scene = AppScene::new();
        let mut textfield = TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0));
        textfield.set_text("hello");
        scene.add_textfield(textfield);

        scene.handle_click(Point::new(50.0, 20.0));
        scene.set_modifier_flags(ModifierFlags::COMMAND);

        assert!(scene.handle_key_down(KeyCode::A));
        assert_eq!(scene.textfields[0].selection_range(), Some((0, 5)));
    }

    #[test]
    fn scene_command_c_copies_selected_text() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        force_in_memory_pasteboard();

        let mut scene = AppScene::new();
        let mut textfield = TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0));
        textfield.set_text("hello");
        textfield.view_mut().set_needs_display();
        scene.add_textfield(textfield);

        scene.handle_click(Point::new(50.0, 20.0));
        scene.textfields[0].set_cursor_position(1);
        scene.textfields[0].move_cursor_right(true);
        scene.textfields[0].move_cursor_right(true);
        scene.textfields[0].move_cursor_right(true);
        scene.set_modifier_flags(ModifierFlags::COMMAND);

        assert_eq!(scene.textfields[0].selection_range(), Some((1, 4)));
        assert_eq!(scene.textfields[0].selected_text(), Some("ell"));

        assert!(scene.handle_key_down(KeyCode::C));

        let pasteboard = NSPasteboard::general();
        assert_eq!(
            pasteboard.string_for_type(NS_PASTEBOARD_TYPE_STRING),
            Some("ell")
        );
        assert_eq!(scene.textfield_text(0), "hello");
    }

    #[test]
    fn scene_command_x_cuts_selected_text() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        force_in_memory_pasteboard();

        let mut scene = AppScene::new();
        let mut textfield = TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0));
        textfield.set_text("hello");
        scene.add_textfield(textfield);

        scene.handle_click(Point::new(50.0, 20.0));
        scene.textfields[0].set_cursor_position(1);
        scene.textfields[0].move_cursor_right(true);
        scene.textfields[0].move_cursor_right(true);
        scene.textfields[0].move_cursor_right(true);
        scene.set_modifier_flags(ModifierFlags::COMMAND);

        assert_eq!(scene.textfields[0].selection_range(), Some((1, 4)));
        assert_eq!(scene.textfields[0].selected_text(), Some("ell"));

        assert!(scene.handle_key_down(KeyCode::X));
        assert_eq!(scene.textfield_text(0), "ho");

        let pasteboard = NSPasteboard::general();
        assert_eq!(
            pasteboard.string_for_type(NS_PASTEBOARD_TYPE_STRING),
            Some("ell")
        );
    }

    #[test]
    fn scene_command_v_pastes_into_focused_textfield() {
        let _guard = CLIPBOARD_TEST_LOCK.lock().unwrap();
        force_in_memory_pasteboard();

        let mut pasteboard = NSPasteboard::general();
        pasteboard.clear_contents();
        pasteboard.set_string("clipboard", NS_PASTEBOARD_TYPE_STRING);

        let mut scene = AppScene::new();
        let mut textfield = TextField::new(Rect::new(10.0, 10.0, 200.0, 24.0));
        textfield.set_text("hello");
        textfield.set_cursor_position(5);
        scene.add_textfield(textfield);

        scene.handle_click(Point::new(50.0, 20.0));
        scene.set_modifier_flags(ModifierFlags::COMMAND);

        assert!(scene.handle_key_down(KeyCode::V));
        assert_eq!(scene.textfield_text(0), "helloclipboard");
    }
}
