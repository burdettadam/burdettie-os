//! Control Gallery — exercises Label, Button, TextField, and Menu controls.
//!
//! Run with: `cargo run -p control-gallery`

use app_kit_lite::menu::{KeyEquivalent, NSMenu, NSMenuItem};
use graphics_core::{Color, Rect};
use ui_controls::{Button, Label, TextField};
use window_backend::{KeyCode, WindowConfig};
use window_backend_darwin::GuiApp;

fn main() {
    let config = WindowConfig {
        title: "Control Gallery".into(),
        width: 520.0,
        height: 440.0,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(Color::WHITE);

    // ── Menu bar ─────────────────────────────────────
    let mut main_menu = NSMenu::new("Main");

    let mut file_item = NSMenuItem::new("File");
    let mut file_menu = NSMenu::new("File");
    file_menu.add_item(NSMenuItem::with_action(
        "New",
        Some(KeyEquivalent::command(KeyCode::N)),
        || {},
    ));
    file_menu.add_item(NSMenuItem::with_action(
        "Open",
        Some(KeyEquivalent::command(KeyCode::O)),
        || {},
    ));
    file_menu.add_item(NSMenuItem::separator());
    file_menu.add_item(NSMenuItem::with_action(
        "Quit",
        Some(KeyEquivalent::command(KeyCode::Q)),
        || std::process::exit(0),
    ));
    file_item.set_submenu(file_menu);
    main_menu.add_item(file_item);

    let mut edit_item = NSMenuItem::new("Edit");
    let mut edit_menu = NSMenu::new("Edit");
    edit_menu.add_item(NSMenuItem::with_action(
        "Cut",
        Some(KeyEquivalent::command(KeyCode::X)),
        || {},
    ));
    edit_menu.add_item(NSMenuItem::with_action(
        "Copy",
        Some(KeyEquivalent::command(KeyCode::C)),
        || {},
    ));
    edit_menu.add_item(NSMenuItem::with_action(
        "Paste",
        Some(KeyEquivalent::command(KeyCode::V)),
        || {},
    ));
    edit_item.set_submenu(edit_menu);
    main_menu.add_item(edit_item);

    app.set_main_menu(main_menu);

    // ── Section 1: Labels ────────────────────────────
    let y_offset = 26.0; // account for menu bar
    let mut heading = Label::new(
        Rect::new(30.0, y_offset + 15.0, 460.0, 20.0),
        "Control Gallery",
    );
    heading.set_text_color(Color::new(0.1, 0.1, 0.4, 1.0));
    app.add_label(heading);

    app.add_label(Label::new(
        Rect::new(30.0, y_offset + 45.0, 460.0, 18.0),
        "Type in the field below and press Echo:",
    ));

    // Echo output label (index 2)
    app.add_label(Label::new(
        Rect::new(30.0, y_offset + 320.0, 460.0, 24.0),
        "",
    ));

    // ── Section 2: Text field ────────────────────────
    let mut tf = TextField::new(Rect::new(30.0, y_offset + 75.0, 300.0, 28.0));
    tf.set_placeholder("Type something here...");
    app.add_textfield(tf);

    // ── Section 3: Buttons ───────────────────────────
    // Echo button (index 0)
    app.add_button(Button::new(
        Rect::new(340.0, y_offset + 75.0, 140.0, 28.0),
        "Echo",
    ));

    // Toggle background button (index 1)
    app.add_button(Button::new(
        Rect::new(30.0, y_offset + 130.0, 220.0, 36.0),
        "Toggle Background",
    ));

    // Clear button (index 2)
    app.add_button(Button::new(
        Rect::new(260.0, y_offset + 130.0, 120.0, 36.0),
        "Clear",
    ));

    // ── Section 4: Status labels ─────────────────────
    app.add_label(Label::new(
        Rect::new(30.0, y_offset + 200.0, 460.0, 18.0),
        "Status: Ready",
    ));

    // Button click count label (index 4)
    app.add_label(Label::new(
        Rect::new(30.0, y_offset + 230.0, 460.0, 18.0),
        "Clicks: 0",
    ));

    let mut click_count = 0u32;
    let mut dark_bg = false;

    app.on_button_click(move |idx, labels, textfields| {
        click_count += 1;
        labels[4].set_text(format!("Clicks: {click_count}"));

        match idx {
            0 => {
                // Echo: copy text field content to echo label
                let text = textfields[0].text().to_string();
                if text.is_empty() {
                    labels[2].set_text("(empty)");
                } else {
                    labels[2].set_text(format!("Echo: {text}"));
                }
                labels[3].set_text("Status: Echoed text field");
            }
            1 => {
                // Toggle background
                dark_bg = !dark_bg;
                if dark_bg {
                    labels[3].set_text("Status: Dark mode toggled ON");
                } else {
                    labels[3].set_text("Status: Dark mode toggled OFF");
                }
            }
            2 => {
                // Clear
                textfields[0].set_text("");
                labels[2].set_text("");
                labels[3].set_text("Status: Cleared");
            }
            _ => {}
        }
    });

    app.run();
}
