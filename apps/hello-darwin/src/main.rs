//! Hello Darwin — minimal lifecycle GUI app.
//!
//! Run with: `cargo run -p hello-darwin`

use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use window_backend::WindowConfig;
use window_backend_darwin::GuiApp;

use hello_darwin::run_scorecard;

fn main() {
    let config = WindowConfig {
        title: "Hello Darwin".into(),
        width: 520.0,
        height: 300.0,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(Color::WHITE);

    // Title label
    let mut title = Label::new(
        Rect::new(30.0, 25.0, 460.0, 24.0),
        "Running on burdetty-os runtime v0.1",
    );
    title.set_text_color(Color::new(0.1, 0.1, 0.3, 1.0));
    app.add_label(title);

    // Result label
    app.add_label(Label::new(
        Rect::new(30.0, 190.0, 460.0, 24.0),
        "Press a button to begin.",
    ));

    // Run Scorecard button
    app.add_button(Button::new(
        Rect::new(30.0, 80.0, 200.0, 40.0),
        "Run Scorecard",
    ));

    // Quit button
    app.add_button(Button::new(Rect::new(260.0, 80.0, 120.0, 40.0), "Quit"));

    app.on_button_click(move |idx, labels, _tfs| match idx {
        0 => {
            labels[1].set_text("Running scorecard...");
            let result = run_scorecard();
            labels[1].set_text(result);
        }
        1 => std::process::exit(0),
        _ => {}
    });

    app.run();
}
