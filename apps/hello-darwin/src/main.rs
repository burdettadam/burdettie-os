//! Hello Darwin — minimal lifecycle GUI app.
//!
//! Run with: `cargo run -p hello-darwin`

use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use ui_view::layout::{
    equal, set_size, LayoutAttribute, ViewId, SUPERVIEW,
};
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

    // Assign layout IDs.
    let title_id = ViewId(1);
    let result_id = ViewId(2);
    let run_id = ViewId(3);
    let quit_id = ViewId(4);

    // Title label
    let mut title = Label::new(Rect::default(), "Running on burdetty-os runtime v0.1");
    title.set_text_color(Color::new(0.1, 0.1, 0.3, 1.0));
    title.view_mut().layout_id = Some(title_id);
    app.add_label(title);

    // Result label
    let mut result_label = Label::new(Rect::default(), "Press a button to begin.");
    result_label.view_mut().layout_id = Some(result_id);
    app.add_label(result_label);

    // Run Scorecard button
    let mut run_btn = Button::new(Rect::default(), "Run Scorecard");
    run_btn.view_mut().layout_id = Some(run_id);
    app.add_button(run_btn);

    // Quit button
    let mut quit_btn = Button::new(Rect::default(), "Quit");
    quit_btn.view_mut().layout_id = Some(quit_id);
    app.add_button(quit_btn);

    // ── Constraints ────────────────────────────────────
    let scene = app.scene_mut();

    // Title: pinned near top, full width with margins
    scene.add_constraint(equal(title_id, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 30.0));
    scene.add_constraint(equal(title_id, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 25.0));
    scene.add_constraints(set_size(title_id, 460.0, 24.0));

    // Run button: below title
    scene.add_constraint(equal(run_id, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 30.0));
    scene.add_constraint(equal(run_id, LayoutAttribute::Top, title_id, LayoutAttribute::Bottom, 31.0));
    scene.add_constraints(set_size(run_id, 200.0, 40.0));

    // Quit button: right of Run button
    scene.add_constraint(equal(quit_id, LayoutAttribute::Left, run_id, LayoutAttribute::Right, 30.0));
    scene.add_constraint(equal(quit_id, LayoutAttribute::Top, run_id, LayoutAttribute::Top, 0.0));
    scene.add_constraints(set_size(quit_id, 120.0, 40.0));

    // Result label: below buttons
    scene.add_constraint(equal(result_id, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 30.0));
    scene.add_constraint(equal(result_id, LayoutAttribute::Top, run_id, LayoutAttribute::Bottom, 70.0));
    scene.add_constraints(set_size(result_id, 460.0, 24.0));

    // Solve layout before running
    let conflicts = scene.solve_layout(520.0, 300.0);
    assert_eq!(conflicts, 0, "Layout conflicts detected in hello-darwin");

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
