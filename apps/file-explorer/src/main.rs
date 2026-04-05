//! File Explorer — Finder-inspired file browser for burdettie-os.
//!
//! Two-pane layout: sidebar with bookmarks on the left, file listing
//! on the right. Navigate directories, view file metadata, and open
//! files. Uses the existing ui-controls toolkit (Labels, Buttons,
//! ScrollView concepts) to build a Finder-like experience.
//!
//! Inspired by macOS Finder and open-source file managers (Thunar, PCManFM).
//!
//! Run with: `cargo run -p file-explorer`

use app_kit_lite::menu::{KeyEquivalent, NSMenu, NSMenuItem};
use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use window_backend::{KeyCode, WindowConfig};
use window_backend_darwin::GuiApp;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ── File entry model ───────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    #[allow(dead_code)]
    modified: Option<SystemTime>,
}

impl FileEntry {
    fn size_display(&self) -> String {
        if self.is_dir {
            return "--".to_string();
        }
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    fn icon(&self) -> &str {
        if self.is_dir {
            "[D]"
        } else if self.name.ends_with(".rs") {
            "[R]"
        } else if self.name.ends_with(".toml") || self.name.ends_with(".json") {
            "[C]"
        } else if self.name.ends_with(".md") || self.name.ends_with(".txt") {
            "[T]"
        } else {
            "[F]"
        }
    }
}

fn read_directory(path: &Path) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let Ok(dir) = fs::read_dir(path) else {
        return entries;
    };
    for entry in dir.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden files by default
        if name.starts_with('.') {
            continue;
        }
        entries.push(FileEntry {
            name,
            path: entry.path(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified: meta.modified().ok(),
        });
    }
    // Directories first, then files, both alphabetical
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

// ── Layout constants ───────────────────────────────────────────────

const SIDEBAR_W: f64 = 180.0;
const WIN_W: f64 = 820.0;
const WIN_H: f64 = 560.0;
const MENU_H: f64 = 22.0;
const PATH_BAR_H: f64 = 28.0;
const ROW_H: f64 = 22.0;
const VISIBLE_ROWS: usize = 20;
const FILE_LIST_X: f64 = SIDEBAR_W + 10.0;
const FILE_LIST_W: f64 = WIN_W - FILE_LIST_X - 10.0;

// ── Sidebar bookmarks ──────────────────────────────────────────────

fn sidebar_bookmarks() -> Vec<(&'static str, PathBuf)> {
    let mut bookmarks = Vec::new();
    if let Some(home) = dirs_home() {
        bookmarks.push(("Home", home.clone()));
        let desktop = home.join("Desktop");
        if desktop.is_dir() {
            bookmarks.push(("Desktop", desktop));
        }
        let documents = home.join("Documents");
        if documents.is_dir() {
            bookmarks.push(("Documents", documents));
        }
        let downloads = home.join("Downloads");
        if downloads.is_dir() {
            bookmarks.push(("Downloads", downloads));
        }
    }
    bookmarks.push(("Root", PathBuf::from("/")));

    // Project workspace
    if let Ok(root) = std::env::var("DARWIN_LOADER_ROOT") {
        bookmarks.push(("Workspace", PathBuf::from(root)));
    } else {
        let mut p = std::env::current_exe().unwrap_or_default();
        for _ in 0..4 {
            p.pop();
        }
        if p.is_dir() {
            bookmarks.push(("Workspace", p));
        }
    }

    bookmarks
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn main() {
    let bookmarks = sidebar_bookmarks();
    let initial_dir = bookmarks
        .first()
        .map(|(_, p)| p.clone())
        .unwrap_or_else(|| PathBuf::from("/"));

    let mut current_dir = initial_dir.clone();
    let mut entries = read_directory(&current_dir);
    let mut scroll_offset: usize = 0;

    let config = WindowConfig {
        title: format!("File Explorer — {}", current_dir.display()),
        width: WIN_W,
        height: WIN_H,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(Color::new(0.97, 0.97, 0.97, 1.0));

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

    let mut go_item = NSMenuItem::new("Go");
    let mut go_menu = NSMenu::new("Go");
    go_menu.add_item(NSMenuItem::with_action(
        "Enclosing Folder",
        Some(KeyEquivalent::command(KeyCode::Up)),
        || {},
    ));
    go_item.set_submenu(go_menu);
    main_menu.add_item(go_item);

    app.set_main_menu(main_menu);

    // ── Path bar (label 0) ─────────────────────────────
    let mut path_label = Label::new(
        Rect::new(
            FILE_LIST_X,
            MENU_H + 4.0,
            FILE_LIST_W,
            PATH_BAR_H - 4.0,
        ),
        &format!("  {}", current_dir.display()),
    );
    path_label.set_text_color(Color::new(0.2, 0.2, 0.3, 1.0));
    app.add_label(path_label);

    // ── Sidebar header (label 1) ───────────────────────
    let mut sidebar_header = Label::new(
        Rect::new(8.0, MENU_H + 4.0, SIDEBAR_W - 16.0, 20.0),
        "FAVORITES",
    );
    sidebar_header.set_text_color(Color::new(0.5, 0.5, 0.5, 1.0));
    app.add_label(sidebar_header);

    // ── Sidebar bookmark buttons ───────────────────────
    for (i, (name, _path)) in bookmarks.iter().enumerate() {
        let y = MENU_H + 28.0 + i as f64 * 26.0;
        app.add_button(Button::new(
            Rect::new(8.0, y, SIDEBAR_W - 16.0, 24.0),
            *name,
        ));
    }
    let bookmark_count = bookmarks.len();

    // ── Column headers (labels 2-4) ────────────────────
    let header_y = MENU_H + PATH_BAR_H + 4.0;
    let mut name_hdr = Label::new(
        Rect::new(FILE_LIST_X, header_y, FILE_LIST_W * 0.55, ROW_H),
        "  Name",
    );
    name_hdr.set_text_color(Color::new(0.3, 0.3, 0.3, 1.0));
    app.add_label(name_hdr);

    let mut size_hdr = Label::new(
        Rect::new(
            FILE_LIST_X + FILE_LIST_W * 0.55,
            header_y,
            FILE_LIST_W * 0.2,
            ROW_H,
        ),
        "Size",
    );
    size_hdr.set_text_color(Color::new(0.3, 0.3, 0.3, 1.0));
    app.add_label(size_hdr);

    let mut kind_hdr = Label::new(
        Rect::new(
            FILE_LIST_X + FILE_LIST_W * 0.75,
            header_y,
            FILE_LIST_W * 0.25,
            ROW_H,
        ),
        "Kind",
    );
    kind_hdr.set_text_color(Color::new(0.3, 0.3, 0.3, 1.0));
    app.add_label(kind_hdr);

    // ── File listing labels (labels 5..5+VISIBLE_ROWS*3) ─
    // Each visible row has 3 labels: name, size, kind
    let list_y_start = MENU_H + PATH_BAR_H + ROW_H + 8.0;
    for row in 0..VISIBLE_ROWS {
        let y = list_y_start + row as f64 * ROW_H;
        let file_idx = row;
        let (name_str, size_str, kind_str) = if file_idx < entries.len() {
            let e = &entries[file_idx];
            (
                format!("  {} {}", e.icon(), e.name),
                e.size_display(),
                if e.is_dir {
                    "Folder".to_string()
                } else {
                    "File".to_string()
                },
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        let mut name_lbl = Label::new(
            Rect::new(FILE_LIST_X, y, FILE_LIST_W * 0.55, ROW_H),
            &name_str,
        );
        if file_idx < entries.len() && entries[file_idx].is_dir {
            name_lbl.set_text_color(Color::new(0.1, 0.3, 0.7, 1.0));
        }
        app.add_label(name_lbl);
        app.add_label(Label::new(
            Rect::new(FILE_LIST_X + FILE_LIST_W * 0.55, y, FILE_LIST_W * 0.2, ROW_H),
            &size_str,
        ));
        app.add_label(Label::new(
            Rect::new(FILE_LIST_X + FILE_LIST_W * 0.75, y, FILE_LIST_W * 0.25, ROW_H),
            &kind_str,
        ));
    }

    // ── File row click buttons (invisible, overlay the file list) ──
    for row in 0..VISIBLE_ROWS {
        let y = list_y_start + row as f64 * ROW_H;
        app.add_button(Button::new(
            Rect::new(FILE_LIST_X, y, FILE_LIST_W, ROW_H),
            "",
        ));
    }
    let file_button_start = bookmark_count;
    let file_button_end = file_button_start + VISIBLE_ROWS;

    // ── Navigation buttons ─────────────────────────────
    let nav_y = WIN_H - 36.0;
    app.add_button(Button::new(
        Rect::new(FILE_LIST_X, nav_y, 60.0, 26.0),
        "Up",
    ));
    app.add_button(Button::new(
        Rect::new(FILE_LIST_X + 70.0, nav_y, 70.0, 26.0),
        "Scroll ^",
    ));
    app.add_button(Button::new(
        Rect::new(FILE_LIST_X + 150.0, nav_y, 70.0, 26.0),
        "Scroll v",
    ));

    // ── Status label (last label) ──────────────────────
    let status_label_idx = 5 + VISIBLE_ROWS * 3;
    app.add_label(Label::new(
        Rect::new(FILE_LIST_X + 240.0, nav_y + 4.0, 300.0, 20.0),
        &format!("{} items", entries.len()),
    ));

    // ── Click handlers ─────────────────────────────────
    let bookmarks_clone = bookmarks.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>();

    app.on_button_click(move |idx, labels, _tfs| {
        let mut needs_refresh = false;

        if idx < bookmark_count {
            // Sidebar bookmark click
            current_dir = bookmarks_clone[idx].clone();
            scroll_offset = 0;
            needs_refresh = true;
        } else if idx >= file_button_start && idx < file_button_end {
            // File row click → navigate into directory
            let row = idx - file_button_start;
            let file_idx = scroll_offset + row;
            if file_idx < entries.len() {
                let entry = &entries[file_idx];
                if entry.is_dir {
                    current_dir = entry.path.clone();
                    scroll_offset = 0;
                    needs_refresh = true;
                }
            }
        } else if idx == file_button_end {
            // "Up" button
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
                scroll_offset = 0;
                needs_refresh = true;
            }
        } else if idx == file_button_end + 1 {
            // Scroll up
            if scroll_offset > 0 {
                scroll_offset = scroll_offset.saturating_sub(VISIBLE_ROWS);
                needs_refresh = true;
            }
        } else if idx == file_button_end + 2 {
            // Scroll down
            if scroll_offset + VISIBLE_ROWS < entries.len() {
                scroll_offset += VISIBLE_ROWS;
                needs_refresh = true;
            }
        }

        if needs_refresh {
            entries = read_directory(&current_dir);
            labels[0].set_text(format!("  {}", current_dir.display()));

            // Update file listing labels
            for row in 0..VISIBLE_ROWS {
                let file_idx = scroll_offset + row;
                let name_label_idx = 5 + row * 3;
                let size_label_idx = 5 + row * 3 + 1;
                let kind_label_idx = 5 + row * 3 + 2;

                if file_idx < entries.len() {
                    let e = &entries[file_idx];
                    labels[name_label_idx]
                        .set_text(format!("  {} {}", e.icon(), e.name));
                    if e.is_dir {
                        labels[name_label_idx]
                            .set_text_color(Color::new(0.1, 0.3, 0.7, 1.0));
                    } else {
                        labels[name_label_idx].set_text_color(Color::BLACK);
                    }
                    labels[size_label_idx].set_text(e.size_display());
                    labels[kind_label_idx].set_text(if e.is_dir {
                        "Folder"
                    } else {
                        "File"
                    });
                } else {
                    labels[name_label_idx].set_text("");
                    labels[size_label_idx].set_text("");
                    labels[kind_label_idx].set_text("");
                }
            }

            // Update status
            labels[status_label_idx].set_text(format!("{} items", entries.len()));
        }
    });

    app.run();
}
