//! Demo app: one window, one label, one button.
//!
//! Run with: `cargo run -p window-backend-darwin --example demo`

use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use window_backend::WindowConfig;
use window_backend_darwin::GuiApp;

fn main() {
    let config = WindowConfig {
        title: "burdetty-os Demo".into(),
        width: 480.0,
        height: 320.0,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(Color::WHITE);

    // Title label
    let mut title = Label::new(
        Rect::new(40.0, 30.0, 400.0, 30.0),
        "Hello, burdetty-os!",
    );
    title.set_text_color(Color::new(0.1, 0.1, 0.4, 1.0));
    app.add_label(title);

    // Status label (updated by button clicks)
    app.add_label(Label::new(
        Rect::new(40.0, 180.0, 400.0, 30.0),
        "Click the button below.",
    ));

    // Clickable button
    app.add_button(Button::new(Rect::new(40.0, 100.0, 200.0, 40.0), "Click Me"));

    // When the button is clicked, update the status label
    let mut click_count = 0u32;
    app.on_button_click(move |_idx, labels, _textfields| {
        click_count += 1;
        labels[1].set_text(format!("Button clicked {click_count} time(s)!"));
    });

    app.run();
}
