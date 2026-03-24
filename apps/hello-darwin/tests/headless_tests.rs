//! Headless tests for hello-darwin app logic.
//!
//! These tests exercise the fixture discovery, scorecard, and button click
//! handler logic without opening a window — using `HeadlessApp`.

use std::fs;
use std::path::PathBuf;

use graphics_core::Rect;
use ui_controls::{Button, HeadlessApp, Label};

use hello_darwin::{discover_fixtures, run_scorecard_from};

/// Create a temporary fixture tree with fake binaries.
fn make_temp_fixtures() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = dir.path();

    // simple_cli/app — an existing real-ish fixture
    let f1 = root.join("simple_cli");
    fs::create_dir_all(&f1).unwrap();
    fs::write(f1.join("app"), b"fake").unwrap();

    // return42/app
    let f2 = root.join("return42");
    fs::create_dir_all(&f2).unwrap();
    fs::write(f2.join("app"), b"fake").unwrap();

    // dispatch_test/dispatch_test (named-binary variant)
    let f3 = root.join("dispatch_test");
    fs::create_dir_all(&f3).unwrap();
    fs::write(f3.join("dispatch_test"), b"fake").unwrap();

    // malformed/ should be skipped
    let f4 = root.join("malformed");
    fs::create_dir_all(&f4).unwrap();
    fs::write(f4.join("bad_magic"), b"junk").unwrap();

    dir
}

// ── Fixture discovery tests ────────────────────────────────────────

#[test]
fn discover_fixtures_finds_app_binaries() {
    let tmp = make_temp_fixtures();
    let fixtures = discover_fixtures(tmp.path());

    let names: Vec<&str> = fixtures.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"simple_cli"), "should find simple_cli");
    assert!(names.contains(&"return42"), "should find return42");
}

#[test]
fn discover_fixtures_finds_named_binaries() {
    let tmp = make_temp_fixtures();
    let fixtures = discover_fixtures(tmp.path());

    let names: Vec<&str> = fixtures.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"dispatch_test"),
        "should find dispatch_test (named binary)"
    );
}

#[test]
fn discover_fixtures_skips_malformed() {
    let tmp = make_temp_fixtures();
    let fixtures = discover_fixtures(tmp.path());

    let names: Vec<&str> = fixtures.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        !names.contains(&"malformed"),
        "should skip malformed directory"
    );
}

#[test]
fn discover_fixtures_returns_sorted() {
    let tmp = make_temp_fixtures();
    let fixtures = discover_fixtures(tmp.path());

    let names: Vec<&str> = fixtures.iter().map(|(n, _)| n.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "fixtures should be sorted by name");
}

#[test]
fn discover_fixtures_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let fixtures = discover_fixtures(tmp.path());
    assert!(fixtures.is_empty());
}

#[test]
fn discover_fixtures_nonexistent_dir() {
    let fixtures = discover_fixtures(&PathBuf::from("/nonexistent/path/12345"));
    assert!(fixtures.is_empty());
}

#[test]
fn discover_fixtures_skips_non_directory_entries() {
    let tmp = tempfile::tempdir().unwrap();
    // Create a plain file at the root level (not a directory)
    fs::write(tmp.path().join("stray_file.txt"), b"oops").unwrap();
    let fixtures = discover_fixtures(tmp.path());
    assert!(fixtures.is_empty());
}

#[test]
fn discover_fixtures_skips_dirs_without_binary() {
    let tmp = tempfile::tempdir().unwrap();
    // Directory exists but has no "app" or same-name binary
    let d = tmp.path().join("empty_fixture");
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("readme.txt"), b"not a binary").unwrap();
    let fixtures = discover_fixtures(tmp.path());
    assert!(fixtures.is_empty());
}

// ── Scorecard logic tests ──────────────────────────────────────────

#[test]
fn scorecard_from_nonexistent_dir() {
    let result = run_scorecard_from(&PathBuf::from("/no/such/path/99999"));
    assert_eq!(result, "Fixtures directory not found.");
}

#[test]
fn scorecard_from_real_fixtures() {
    // Use the actual fixtures directory if available
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // apps/hello-darwin -> workspace root
    p.pop();
    p.pop();
    let fixtures = p.join("fixtures");
    if !fixtures.is_dir() {
        // Skip if fixtures aren't built
        return;
    }

    let result = run_scorecard_from(&fixtures);
    assert!(
        result.starts_with("Scorecard:"),
        "should start with Scorecard: but got: {result}"
    );
    assert!(
        result.contains("fixtures passed"),
        "should contain 'fixtures passed' but got: {result}"
    );
}

// ── HeadlessApp integration tests ──────────────────────────────────

/// Build the hello-darwin app UI headlessly — same setup as main.rs.
fn build_hello_app(scorecard_fn: impl FnMut() -> String + 'static) -> HeadlessApp {
    let mut app = HeadlessApp::new();
    // Title label (index 0)
    app.add_label(Label::new(
        Rect::new(30.0, 25.0, 460.0, 24.0),
        "Running on burdetty-os runtime v0.1",
    ));
    // Result label (index 1)
    app.add_label(Label::new(
        Rect::new(30.0, 190.0, 460.0, 24.0),
        "Press a button to begin.",
    ));
    // Run Scorecard button (index 0)
    app.add_button(Button::new(
        Rect::new(30.0, 80.0, 200.0, 40.0),
        "Run Scorecard",
    ));
    // Quit button (index 1)
    app.add_button(Button::new(Rect::new(260.0, 80.0, 120.0, 40.0), "Quit"));

    let mut scorecard_fn = scorecard_fn;
    app.on_button_click(move |idx, labels, _tfs| {
        if idx == 0 {
            labels[1].set_text("Running scorecard...");
            let result = scorecard_fn();
            labels[1].set_text(result);
        }
    });

    app
}

#[test]
fn hello_app_initial_state() {
    let app = build_hello_app(String::new);

    assert_eq!(app.label_count(), 2);
    assert_eq!(app.button_count(), 2);
    assert_eq!(
        app.label_text(0),
        "Running on burdetty-os runtime v0.1"
    );
    assert_eq!(app.label_text(1), "Press a button to begin.");
    assert_eq!(app.button_title(0), "Run Scorecard");
    assert_eq!(app.button_title(1), "Quit");
}

#[test]
fn hello_app_scorecard_button_updates_label() {
    let mut app = build_hello_app(|| "Scorecard: 62/62 fixtures passed".to_string());

    // Click "Run Scorecard"
    app.click_button(0);
    assert_eq!(app.label_text(1), "Scorecard: 62/62 fixtures passed");
}

#[test]
fn hello_app_scorecard_shows_error_message() {
    let mut app = build_hello_app(|| "Fixtures directory not found.".to_string());

    app.click_button(0);
    assert_eq!(app.label_text(1), "Fixtures directory not found.");
}

#[test]
fn hello_app_scorecard_button_click_at_coordinates() {
    let mut app = build_hello_app(|| "Scorecard: 10/10 fixtures passed".to_string());

    // Click inside the "Run Scorecard" button (30,80 200x40)
    app.click_at(100.0, 100.0);
    assert_eq!(app.label_text(1), "Scorecard: 10/10 fixtures passed");
}

#[test]
fn hello_app_click_outside_buttons_does_nothing() {
    let mut app = build_hello_app(|| "should not appear".to_string());

    // Click in empty space
    app.click_at(450.0, 250.0);
    assert_eq!(app.label_text(1), "Press a button to begin.");
}
