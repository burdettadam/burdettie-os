//! Launchpad — application launcher for burdettie-os.
//!
//! Discovers Mach-O fixture binaries, corpus tools, and GUI apps,
//! presenting them in a scrollable grid. Click an app icon to launch it
//! via the macho-runtime loader or as a subprocess.
//!
//! Inspired by macOS Launchpad — open source, native-first.
//!
//! Run with: `cargo run -p launchpad`

use app_kit_lite::menu::{KeyEquivalent, NSMenu, NSMenuItem};
use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use window_backend::{KeyCode, WindowConfig};
use window_backend_darwin::GuiApp;

use std::path::{Path, PathBuf};

// ── App discovery ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AppEntry {
    name: String,
    path: PathBuf,
    category: AppCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppCategory {
    Fixture,
    Corpus,
    GuiApp,
}

impl AppCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Fixture => "Fixture",
            Self::Corpus => "Corpus",
            Self::GuiApp => "App",
        }
    }

    #[allow(dead_code)]
    fn color(self) -> Color {
        match self {
            Self::Fixture => Color::new(0.2, 0.5, 0.8, 1.0),
            Self::Corpus => Color::new(0.3, 0.7, 0.3, 1.0),
            Self::GuiApp => Color::new(0.7, 0.3, 0.6, 1.0),
        }
    }
}

fn workspace_root() -> PathBuf {
    if let Ok(root) = std::env::var("DARWIN_LOADER_ROOT") {
        return PathBuf::from(root);
    }
    let mut p = std::env::current_exe().unwrap_or_default();
    for _ in 0..4 {
        p.pop();
    }
    p
}

fn discover_apps(root: &Path) -> Vec<AppEntry> {
    let mut apps = Vec::new();

    // Fixtures
    let fixtures_dir = root.join("fixtures");
    if fixtures_dir.is_dir() {
        if let Ok(dirs) = std::fs::read_dir(&fixtures_dir) {
            for entry in dirs.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let name = dir.file_name().unwrap().to_string_lossy().to_string();
                if name == "malformed" {
                    continue;
                }
                let bin = dir.join("app");
                if bin.is_file() {
                    apps.push(AppEntry {
                        name: name.clone(),
                        path: bin,
                        category: AppCategory::Fixture,
                    });
                    continue;
                }
                let bin2 = dir.join(&name);
                if bin2.is_file() {
                    apps.push(AppEntry {
                        name,
                        path: bin2,
                        category: AppCategory::Fixture,
                    });
                }
            }
        }
    }

    // Corpus tools (source directories with Mach-O binaries)
    let corpus_dir = root.join("corpus");
    if corpus_dir.is_dir() {
        if let Ok(dirs) = std::fs::read_dir(&corpus_dir) {
            for entry in dirs.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let name = dir.file_name().unwrap().to_string_lossy().to_string();
                // Corpus tools have a build script and an `app` binary
                let bin = dir.join("app");
                if bin.is_file() {
                    apps.push(AppEntry {
                        name,
                        path: bin,
                        category: AppCategory::Corpus,
                    });
                }
            }
        }
    }

    // Built GUI apps (look for known app binaries)
    let gui_apps = [
        ("Hello Darwin", "hello-darwin"),
        ("Control Gallery", "control-gallery"),
        ("Foundation Explorer", "foundation-explorer"),
        ("File Explorer", "file-explorer"),
    ];
    for (display_name, bin_name) in gui_apps {
        // Check common cargo output locations
        for target_dir in ["target/debug", "target/release"] {
            let bin_path = root.join(target_dir).join(bin_name);
            if bin_path.is_file() {
                apps.push(AppEntry {
                    name: display_name.to_string(),
                    path: bin_path,
                    category: AppCategory::GuiApp,
                });
                break;
            }
        }
    }

    apps.sort_by(|a, b| a.category.label().cmp(b.category.label()).then(a.name.cmp(&b.name)));
    apps
}

// ── Grid layout constants ──────────────────────────────────────────

const COLS: usize = 5;
const ROWS_VISIBLE: usize = 6;
const CELL_W: f64 = 130.0;
const CELL_H: f64 = 50.0;
const GRID_PAD_X: f64 = 20.0;
const GRID_PAD_Y: f64 = 60.0;
const CELL_GAP: f64 = 8.0;
const WIN_W: f64 = GRID_PAD_X * 2.0 + COLS as f64 * CELL_W + (COLS - 1) as f64 * CELL_GAP;
const WIN_H: f64 = GRID_PAD_Y + ROWS_VISIBLE as f64 * CELL_H + (ROWS_VISIBLE - 1) as f64 * CELL_GAP + 60.0;

fn main() {
    let root = workspace_root();
    let apps = discover_apps(&root);
    let total = apps.len();
    let total_pages = (total + COLS * ROWS_VISIBLE - 1) / (COLS * ROWS_VISIBLE);

    let config = WindowConfig {
        title: "Launchpad".into(),
        width: WIN_W,
        height: WIN_H,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(Color::new(0.12, 0.12, 0.15, 1.0));

    // ── Menu bar ───────────────────────────────────────
    let mut main_menu = NSMenu::new("Main");
    let mut file_item = NSMenuItem::new("File");
    let mut file_menu = NSMenu::new("File");
    file_menu.add_item(NSMenuItem::with_action(
        "Quit",
        Some(KeyEquivalent::command(KeyCode::Q)),
        || std::process::exit(0),
    ));
    file_item.set_submenu(file_menu);
    main_menu.add_item(file_item);
    app.set_main_menu(main_menu);

    // ── Title ──────────────────────────────────────────
    let mut title = Label::new(Rect::new(GRID_PAD_X, 30.0, 300.0, 22.0), "Launchpad");
    title.set_text_color(Color::WHITE);
    app.add_label(title);

    // Status label (shows keyboard layout + app count)
    let status_text = format!(
        "{total} apps found | Layout: QWERTY | Page 1/{total_pages}"
    );
    let mut status = Label::new(
        Rect::new(GRID_PAD_X + 320.0, 30.0, 400.0, 22.0),
        &status_text,
    );
    status.set_text_color(Color::new(0.6, 0.6, 0.7, 1.0));
    app.add_label(status);

    // ── Grid buttons ───────────────────────────────────
    let page_size = COLS * ROWS_VISIBLE;
    let visible_count = total.min(page_size);
    for i in 0..visible_count {
        let col = i % COLS;
        let row = i / COLS;
        let x = GRID_PAD_X + col as f64 * (CELL_W + CELL_GAP);
        let y = GRID_PAD_Y + row as f64 * (CELL_H + CELL_GAP);
        let entry = &apps[i];
        let label = format!("[{}] {}", entry.category.label(), entry.name);
        app.add_button(Button::new(Rect::new(x, y, CELL_W, CELL_H), &label));
    }

    // ── Navigation buttons ─────────────────────────────
    let nav_y = WIN_H - 40.0;
    app.add_button(Button::new(
        Rect::new(GRID_PAD_X, nav_y, 80.0, 30.0),
        "< Prev",
    ));
    app.add_button(Button::new(
        Rect::new(GRID_PAD_X + 90.0, nav_y, 80.0, 30.0),
        "Next >",
    ));

    // ── Click handler ──────────────────────────────────
    let apps_clone = apps.clone();
    let mut current_page: usize = 0;

    app.on_button_click(move |idx, labels, _tfs| {
        let nav_start = visible_count;
        if idx == nav_start {
            // Prev
            if current_page > 0 {
                current_page -= 1;
            }
            labels[1].set_text(format!(
                "{total} apps | Layout: QWERTY | Page {}/{}",
                current_page + 1,
                total_pages
            ));
        } else if idx == nav_start + 1 {
            // Next
            if current_page + 1 < total_pages {
                current_page += 1;
            }
            labels[1].set_text(format!(
                "{total} apps | Layout: QWERTY | Page {}/{}",
                current_page + 1,
                total_pages
            ));
        } else if idx < nav_start {
            // App launch
            let app_idx = current_page * page_size + idx;
            if app_idx < apps_clone.len() {
                let entry = &apps_clone[app_idx];
                labels[1].set_text(format!("Launching: {}...", entry.name));

                match entry.category {
                    AppCategory::Fixture | AppCategory::Corpus => {
                        // Run through macho-runtime loader
                        let config = macho_runtime::RuntimeConfig::permissive(&entry.path);
                        match macho_runtime::Runtime::load_and_run(&entry.path, &config) {
                            Ok(result) => {
                                labels[1].set_text(format!(
                                    "{}: exit {}",
                                    entry.name, result.exit_code
                                ));
                            }
                            Err(e) => {
                                labels[1].set_text(format!("{}: error: {e}", entry.name));
                            }
                        }
                    }
                    AppCategory::GuiApp => {
                        // Launch as subprocess
                        match std::process::Command::new(&entry.path).spawn() {
                            Ok(_) => {
                                labels[1].set_text(format!("{}: launched", entry.name));
                            }
                            Err(e) => {
                                labels[1].set_text(format!("{}: {e}", entry.name));
                            }
                        }
                    }
                }
            }
        }
    });

    // ── Layout change callback ─────────────────────────
    app.scene_mut()
        .on_layout_change(move |_layout| {
            // Layout label update is handled by the status label
            // (We'd need shared state to update from here; the scene
            //  already stores the layout for character remapping.)
        });

    app.run();
}
