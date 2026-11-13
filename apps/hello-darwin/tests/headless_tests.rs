//! Headless tests for hello-darwin app logic.
//!
//! These tests exercise the fixture discovery, scorecard, and button click
//! handler logic without opening a window — using `HeadlessApp`.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use graphics_core::Rect;
use macho_runtime::{Runtime, RuntimeConfig};
use ui_controls::{Button, HeadlessApp, Label};
use ui_view::layout::{equal, set_size, LayoutAttribute, ViewId, SUPERVIEW};

use hello_darwin::{discover_fixtures, run_scorecard_from};

/// Serializes in-process load_and_run calls (global ObjC state is process-wide).
static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

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

    // Exception fixtures should now be included.
    let f5 = root.join("minimal_throw");
    fs::create_dir_all(&f5).unwrap();
    fs::write(f5.join("app"), b"fake").unwrap();

    let f6 = root.join("try_catch_test");
    fs::create_dir_all(&f6).unwrap();
    fs::write(f6.join("app"), b"fake").unwrap();

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
fn discover_fixtures_includes_exception_fixtures() {
    let tmp = make_temp_fixtures();
    let fixtures = discover_fixtures(tmp.path());

    let names: Vec<&str> = fixtures.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"minimal_throw"),
        "should include minimal_throw now that exception handling works"
    );
    assert!(
        names.contains(&"try_catch_test"),
        "should include try_catch_test now that exception handling works"
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

/// Resolve the workspace fixtures directory (apps/hello-darwin -> workspace root).
fn real_fixtures_dir() -> Option<PathBuf> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // apps/
    p.pop(); // workspace root
    let fixtures = p.join("fixtures");
    fixtures.is_dir().then_some(fixtures)
}

/// Expected exit code per fixture.
fn expected_exit(name: &str) -> i32 {
    match name {
        "return42" => 42,
        _ => 0,
    }
}

/// Decomposed per-fixture test — runs each fixture individually and reports
/// which specific fixtures pass or fail, instead of a single opaque scorecard.
#[test]
fn scorecard_per_fixture_results() {
    let Some(fixtures_root) = real_fixtures_dir() else {
        return;
    };
    let entries = discover_fixtures(&fixtures_root);

    assert!(
        entries.len() >= 61,
        "Expected at least 61 discoverable fixtures, found {}",
        entries.len()
    );

    let mut passed = Vec::new();
    let mut failed = Vec::new();

    for (name, binary) in &entries {
        eprintln!("[scorecard fixture] {name}");
        let expected = expected_exit(name);
        let config = RuntimeConfig::permissive(binary);

        let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        macho_runtime::reset_runtime_state_for_reuse();
        match Runtime::load_and_run(binary, &config) {
            Ok(result) if result.exit_code == expected => {
                passed.push(name.clone());
            }
            Ok(result) => {
                failed.push(format!(
                    "{name}: exit {} (expected {expected})",
                    result.exit_code
                ));
            }
            Err(e) => {
                failed.push(format!("{name}: runtime error — {e}"));
            }
        }
    }

    // Print summary so --nocapture shows what happened
    let total = passed.len() + failed.len();
    println!("\nScorecard: {}/{total} fixtures passed", passed.len());
    if !failed.is_empty() {
        println!("Failures:");
        for f in &failed {
            println!("  {f}");
        }
    }

    assert!(
        failed.is_empty(),
        "Scorecard regression — {} fixture(s) failed:\n  {}",
        failed.len(),
        failed.join("\n  ")
    );
}

// ── HeadlessApp integration tests ──────────────────────────────────

/// Build the hello-darwin app UI headlessly — same setup as main.rs.
fn build_hello_app(scorecard_fn: impl FnMut() -> String + 'static) -> HeadlessApp {
    let title_id = ViewId(1);
    let result_id = ViewId(2);
    let run_id = ViewId(3);
    let quit_id = ViewId(4);

    let mut app = HeadlessApp::new();

    // Title label (index 0)
    let mut title = Label::new(Rect::default(), "Running on burdettie-os runtime v0.1");
    title.view_mut().layout_id = Some(title_id);
    app.add_label(title);

    // Result label (index 1)
    let mut result_label = Label::new(Rect::default(), "Press a button to begin.");
    result_label.view_mut().layout_id = Some(result_id);
    app.add_label(result_label);

    // Run Scorecard button (index 0)
    let mut run_btn = Button::new(Rect::default(), "Run Scorecard");
    run_btn.view_mut().layout_id = Some(run_id);
    app.add_button(run_btn);

    // Quit button (index 1)
    let mut quit_btn = Button::new(Rect::default(), "Quit");
    quit_btn.view_mut().layout_id = Some(quit_id);
    app.add_button(quit_btn);

    // Constraints (same as main.rs)
    let scene = app.scene_mut();
    scene.add_constraint(equal(
        title_id,
        LayoutAttribute::Left,
        SUPERVIEW,
        LayoutAttribute::Left,
        30.0,
    ));
    scene.add_constraint(equal(
        title_id,
        LayoutAttribute::Top,
        SUPERVIEW,
        LayoutAttribute::Top,
        25.0,
    ));
    scene.add_constraints(set_size(title_id, 460.0, 24.0));

    scene.add_constraint(equal(
        run_id,
        LayoutAttribute::Left,
        SUPERVIEW,
        LayoutAttribute::Left,
        30.0,
    ));
    scene.add_constraint(equal(
        run_id,
        LayoutAttribute::Top,
        title_id,
        LayoutAttribute::Bottom,
        31.0,
    ));
    scene.add_constraints(set_size(run_id, 200.0, 40.0));

    scene.add_constraint(equal(
        quit_id,
        LayoutAttribute::Left,
        run_id,
        LayoutAttribute::Right,
        30.0,
    ));
    scene.add_constraint(equal(
        quit_id,
        LayoutAttribute::Top,
        run_id,
        LayoutAttribute::Top,
        0.0,
    ));
    scene.add_constraints(set_size(quit_id, 120.0, 40.0));

    scene.add_constraint(equal(
        result_id,
        LayoutAttribute::Left,
        SUPERVIEW,
        LayoutAttribute::Left,
        30.0,
    ));
    scene.add_constraint(equal(
        result_id,
        LayoutAttribute::Top,
        run_id,
        LayoutAttribute::Bottom,
        70.0,
    ));
    scene.add_constraints(set_size(result_id, 460.0, 24.0));

    let conflicts = scene.solve_layout(520.0, 300.0);
    assert_eq!(conflicts, 0, "Layout has constraint conflicts");

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
    assert_eq!(app.label_text(0), "Running on burdettie-os runtime v0.1");
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

    // Click inside the "Run Scorecard" button (solved: x=30 y=80 200×40)
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

#[test]
fn hello_app_zero_constraint_conflicts() {
    // Verify the hello-darwin layout solves without conflicts
    // at various window sizes (the F4 acceptance criterion).
    for (w, h) in [(520.0, 300.0), (800.0, 600.0), (320.0, 200.0)] {
        let title_id = ViewId(1);
        let result_id = ViewId(2);
        let run_id = ViewId(3);
        let quit_id = ViewId(4);

        let mut app = HeadlessApp::new();
        let mut title = Label::new(Rect::default(), "Title");
        title.view_mut().layout_id = Some(title_id);
        app.add_label(title);
        let mut result = Label::new(Rect::default(), "Result");
        result.view_mut().layout_id = Some(result_id);
        app.add_label(result);
        let mut run_btn = Button::new(Rect::default(), "Run");
        run_btn.view_mut().layout_id = Some(run_id);
        app.add_button(run_btn);
        let mut quit_btn = Button::new(Rect::default(), "Quit");
        quit_btn.view_mut().layout_id = Some(quit_id);
        app.add_button(quit_btn);

        let scene = app.scene_mut();
        scene.add_constraint(equal(
            title_id,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            30.0,
        ));
        scene.add_constraint(equal(
            title_id,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            25.0,
        ));
        scene.add_constraints(set_size(title_id, 460.0, 24.0));
        scene.add_constraint(equal(
            run_id,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            30.0,
        ));
        scene.add_constraint(equal(
            run_id,
            LayoutAttribute::Top,
            title_id,
            LayoutAttribute::Bottom,
            31.0,
        ));
        scene.add_constraints(set_size(run_id, 200.0, 40.0));
        scene.add_constraint(equal(
            quit_id,
            LayoutAttribute::Left,
            run_id,
            LayoutAttribute::Right,
            30.0,
        ));
        scene.add_constraint(equal(
            quit_id,
            LayoutAttribute::Top,
            run_id,
            LayoutAttribute::Top,
            0.0,
        ));
        scene.add_constraints(set_size(quit_id, 120.0, 40.0));
        scene.add_constraint(equal(
            result_id,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            30.0,
        ));
        scene.add_constraint(equal(
            result_id,
            LayoutAttribute::Top,
            run_id,
            LayoutAttribute::Bottom,
            70.0,
        ));
        scene.add_constraints(set_size(result_id, 460.0, 24.0));

        let conflicts = scene.solve_layout(w, h);
        assert_eq!(conflicts, 0, "Conflicts at {w}×{h}");
    }
}
