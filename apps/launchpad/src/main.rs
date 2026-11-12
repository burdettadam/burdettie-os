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
use graphics_core::{Color, Font, Rect};
use plist::Value;
use serde::Deserialize;
use ui_controls::{Button, Label};
use window_backend::{KeyCode, WindowConfig};
use window_backend_darwin::GuiApp;

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

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
            Self::Corpus => "Corpus Tool",
            Self::GuiApp => "GUI App",
        }
    }

    fn badge(self) -> &'static str {
        match self {
            Self::Fixture => "FIX",
            Self::Corpus => "TOOL",
            Self::GuiApp => "APP",
        }
    }

    fn sort_rank(self) -> u8 {
        match self {
            Self::GuiApp => 0,
            Self::Fixture => 1,
            Self::Corpus => 2,
        }
    }

    fn color(self) -> Color {
        match self {
            Self::GuiApp => SHEIKAH_BLUE,
            Self::Fixture => SHEIKAH_ORANGE,
            Self::Corpus => ACTIVE_GREEN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterMode {
    Desktop,
    All,
    Fixtures,
    Corpus,
}

impl FilterMode {
    fn nav_title(self) -> &'static str {
        match self {
            Self::Desktop => "[D] Desktop",
            Self::All => "[A] All Apps",
            Self::Fixtures => "[F] Fixtures",
            Self::Corpus => "[T] Tools",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::All => "All Apps",
            Self::Fixtures => "Fixtures",
            Self::Corpus => "Tools",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Desktop => "Window-ready apps and curated runtime fixtures",
            Self::All => "The full launcher catalog across apps, fixtures, and tools",
            Self::Fixtures => "Runtime smoke targets and GUI guest-app probes",
            Self::Corpus => "Command-style tools for long-tail compatibility work",
        }
    }

    fn accent(self) -> Color {
        match self {
            Self::Desktop => SHEIKAH_BLUE,
            Self::All => NEUTRAL_TAN,
            Self::Fixtures => SHEIKAH_ORANGE,
            Self::Corpus => ACTIVE_GREEN,
        }
    }

    fn rail_name(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop Deck",
            Self::All => "Full Catalog",
            Self::Fixtures => "Fixture Shelf",
            Self::Corpus => "Tool Shelf",
        }
    }

    fn matches(self, entry: &AppEntry) -> bool {
        match self {
            Self::Desktop => matches!(entry.category, AppCategory::GuiApp | AppCategory::Fixture),
            Self::All => true,
            Self::Fixtures => entry.category == AppCategory::Fixture,
            Self::Corpus => entry.category == AppCategory::Corpus,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct QuickLaunchSpec {
    title: &'static str,
    app_name: &'static str,
    accent: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusZone {
    Carousel,
    Grid,
    OpenApps,
}

impl FocusZone {
    fn label(self) -> &'static str {
        match self {
            Self::Carousel => "Carousel",
            Self::Grid => "Inventory",
            Self::OpenApps => "Open Apps",
        }
    }
}

#[derive(Debug, Clone)]
struct LauncherState {
    filter: FilterMode,
    current_page: usize,
    selected_path: Option<PathBuf>,
    open_paths: Vec<PathBuf>,
    active_open_path: Option<PathBuf>,
    overlay_visible: bool,
    focus_zone: FocusZone,
    last_message: String,
}

impl LauncherState {
    fn new(apps: &[AppEntry], page_size: usize) -> Self {
        let filter = FilterMode::Desktop;
        let current_page = 0;
        let selected_path = resolved_selection_path(apps, filter, current_page, page_size, None);
        Self {
            filter,
            current_page,
            selected_path,
            open_paths: Vec::new(),
            active_open_path: None,
            overlay_visible: true,
            focus_zone: FocusZone::Grid,
            last_message:
                "Use arrows to browse, Enter to launch, and Esc to drop back to live apps."
                    .to_string(),
        }
    }

    fn sync_selection(&mut self, apps: &[AppEntry], page_size: usize) {
        let total_pages = page_count(filtered_entries(apps, self.filter).len(), page_size);
        self.current_page = self.current_page.min(total_pages.saturating_sub(1));
        self.selected_path = resolved_selection_path(
            apps,
            self.filter,
            self.current_page,
            page_size,
            self.selected_path.as_ref(),
        );

        if self
            .active_open_path
            .as_ref()
            .is_some_and(|active| !self.open_paths.iter().any(|path| path == active))
        {
            self.active_open_path = self.open_paths.last().cloned();
        }

        if !self.overlay_visible && self.open_paths.is_empty() {
            self.overlay_visible = true;
            self.focus_zone = FocusZone::Grid;
        }
    }

    fn register_open_app(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if let Some(existing) = self
            .open_paths
            .iter()
            .position(|candidate| candidate == &path)
        {
            self.open_paths.remove(existing);
        }
        self.open_paths.push(path.clone());
        if self.open_paths.len() > OPEN_APP_SLOTS {
            let overflow = self.open_paths.len() - OPEN_APP_SLOTS;
            self.open_paths.drain(0..overflow);
        }
        self.active_open_path = Some(path);
    }
}

#[derive(Debug, Clone, Copy)]
struct DesktopMetrics {
    total: usize,
    gui_apps: usize,
    fixtures: usize,
    corpus: usize,
    desktop: usize,
}

impl DesktopMetrics {
    fn from_apps(apps: &[AppEntry]) -> Self {
        let gui_apps = apps
            .iter()
            .filter(|entry| entry.category == AppCategory::GuiApp)
            .count();
        let fixtures = apps
            .iter()
            .filter(|entry| entry.category == AppCategory::Fixture)
            .count();
        let corpus = apps
            .iter()
            .filter(|entry| entry.category == AppCategory::Corpus)
            .count();
        Self {
            total: apps.len(),
            gui_apps,
            fixtures,
            corpus,
            desktop: gui_apps + fixtures,
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

#[derive(Debug, Deserialize)]
struct ImportedArtifactsManifest {
    artifacts: Vec<ImportedArtifact>,
}

#[derive(Debug, Deserialize)]
struct ImportedArtifact {
    kind: String,
    storage: ImportedArtifactStorage,
    #[serde(default)]
    classification: Option<ImportedArtifactClassification>,
    entries: Vec<ImportedArtifactEntry>,
}

#[derive(Debug, Deserialize)]
struct ImportedArtifactClassification {
    tier: String,
    dependency_class: String,
    scoreable_in_docker: bool,
}

#[derive(Debug, Deserialize)]
struct ImportedArtifactStorage {
    overlay_path: String,
    container_path: String,
}

#[derive(Debug, Deserialize)]
struct ImportedArtifactEntry {
    relative_path: String,
    role: String,
    executable: Option<bool>,
}

fn imported_manifest_paths(root: &Path) -> [PathBuf; 3] {
    [
        PathBuf::from("/opt/imported-artifacts/manifest.json"),
        root.join("imports/manifest.json"),
        root.join("docker/imported-artifacts.example.json"),
    ]
}

fn imported_app_dirs(root: &Path) -> [PathBuf; 2] {
    [
        PathBuf::from("/opt/imported-artifacts/apps"),
        root.join("imports/apps"),
    ]
}

fn discover_imported_apps(root: &Path) -> Vec<AppEntry> {
    let mut discovered = Vec::new();
    let mut seen_names = HashSet::new();

    for app in discover_imported_apps_from_manifest(root) {
        if seen_names.insert(app.name.clone()) {
            discovered.push(app);
        }
    }

    for app in discover_imported_apps_from_dirs(root) {
        if seen_names.insert(app.name.clone()) {
            discovered.push(app);
        }
    }

    discovered
}

fn discover_imported_apps_from_manifest(root: &Path) -> Vec<AppEntry> {
    let mut discovered = Vec::new();

    for manifest_path in imported_manifest_paths(root) {
        let Ok(manifest_bytes) = std::fs::read(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<ImportedArtifactsManifest>(&manifest_bytes)
        else {
            continue;
        };

        for artifact in manifest
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == "app-bundle")
        {
            if let Some(app) = imported_app_from_manifest(root, artifact) {
                discovered.push(app);
            }
        }
    }

    discovered
}

fn imported_app_from_manifest(root: &Path, artifact: &ImportedArtifact) -> Option<AppEntry> {
    let entry = artifact
        .entries
        .iter()
        .find(|entry| entry.role == "main-executable" && entry.executable != Some(false))?;

    for bundle_root in imported_bundle_roots(root, &artifact.storage) {
        let executable = bundle_root.join(&entry.relative_path);
        if executable.is_file() {
            return Some(AppEntry {
                name: bundle_display_name(&bundle_root)?,
                category: imported_manifest_app_category(
                    &executable,
                    artifact.classification.as_ref(),
                ),
                path: executable,
            });
        }
    }

    None
}

fn imported_bundle_roots(root: &Path, storage: &ImportedArtifactStorage) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let container_root = PathBuf::from(&storage.container_path);
    if container_root.is_dir() {
        candidates.push(container_root);
    }

    let overlay_root = root.join(&storage.overlay_path);
    if overlay_root.is_dir() {
        candidates.push(overlay_root);
    }

    candidates
}

fn discover_imported_apps_from_dirs(root: &Path) -> Vec<AppEntry> {
    let mut discovered = Vec::new();

    for apps_dir in imported_app_dirs(root) {
        let Ok(entries) = std::fs::read_dir(&apps_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let bundle_path = entry.path();
            if !bundle_path.is_dir()
                || bundle_path.extension().and_then(|ext| ext.to_str()) != Some("app")
            {
                continue;
            }

            let Some(executable) = bundle_executable_path(&bundle_path) else {
                continue;
            };
            let Some(name) = bundle_display_name(&bundle_path) else {
                continue;
            };

            discovered.push(AppEntry {
                name,
                category: discovered_import_app_category(&executable),
                path: executable,
            });
        }
    }

    discovered
}

fn imported_manifest_app_category(
    executable: &Path,
    classification: Option<&ImportedArtifactClassification>,
) -> AppCategory {
    let Some(classification) = classification else {
        return AppCategory::Corpus;
    };

    if manifest_declares_scoreable_app(classification)
        && classify_imported_executable(executable).allows_scoreable_app()
    {
        AppCategory::GuiApp
    } else {
        AppCategory::Corpus
    }
}

fn discovered_import_app_category(executable: &Path) -> AppCategory {
    if classify_imported_executable(executable).allows_scoreable_app() {
        AppCategory::GuiApp
    } else {
        AppCategory::Corpus
    }
}

fn manifest_declares_scoreable_app(classification: &ImportedArtifactClassification) -> bool {
    classification.scoreable_in_docker
        && matches!(
            normalized_manifest_value(&classification.tier).as_str(),
            "tier-a" | "tier-b"
        )
        && normalized_manifest_value(&classification.dependency_class) == "public-surface"
}

fn normalized_manifest_value(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportedDependencyClass {
    PublicSurface,
    Mixed,
    PrivateHeavy,
    Unparseable,
}

impl ImportedDependencyClass {
    fn allows_scoreable_app(self) -> bool {
        self == Self::PublicSurface
    }
}

fn classify_imported_executable(executable: &Path) -> ImportedDependencyClass {
    let Ok(bytes) = std::fs::read(executable) else {
        return ImportedDependencyClass::Unparseable;
    };
    let Ok(parsed) = macho_parse::parse_auto(&bytes, macho_types::CpuType::Arm64) else {
        return ImportedDependencyClass::Unparseable;
    };

    classify_imported_dependencies(parsed.dylibs.iter().map(|dylib| dylib.name.as_str()))
}

fn classify_imported_dependencies<'a>(
    install_names: impl IntoIterator<Item = &'a str>,
) -> ImportedDependencyClass {
    let mut has_mixed = false;

    for name in install_names {
        match classify_imported_install_name(name) {
            ImportedDependencyClass::PrivateHeavy => {
                return ImportedDependencyClass::PrivateHeavy;
            }
            ImportedDependencyClass::Mixed => has_mixed = true,
            ImportedDependencyClass::PublicSurface => {}
            ImportedDependencyClass::Unparseable => {
                unreachable!("install names are already parsed")
            }
        }
    }

    if has_mixed {
        ImportedDependencyClass::Mixed
    } else {
        ImportedDependencyClass::PublicSurface
    }
}

fn classify_imported_install_name(name: &str) -> ImportedDependencyClass {
    let name = name.trim();
    if name.is_empty() {
        return ImportedDependencyClass::Mixed;
    }

    let lower = name.to_ascii_lowercase();
    if lower.contains("/privateframeworks/") {
        return ImportedDependencyClass::PrivateHeavy;
    }

    if lower.starts_with("@rpath/")
        || lower.starts_with("@loader_path/")
        || lower.starts_with("@executable_path/")
    {
        return ImportedDependencyClass::Mixed;
    }

    if lower.starts_with("/usr/lib/") || lower.starts_with("/system/library/") {
        ImportedDependencyClass::PublicSurface
    } else {
        ImportedDependencyClass::Mixed
    }
}

fn bundle_display_name(bundle_path: &Path) -> Option<String> {
    bundle_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
}

fn bundle_executable_path(bundle_path: &Path) -> Option<PathBuf> {
    let macos_dir = bundle_path.join("Contents/MacOS");
    if !macos_dir.is_dir() {
        return None;
    }

    if let Some(executable_name) = bundle_executable_name(bundle_path) {
        let executable = macos_dir.join(executable_name);
        if executable.is_file() {
            return Some(executable);
        }
    }

    let mut executables: Vec<PathBuf> = std::fs::read_dir(&macos_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    executables.sort();
    executables.into_iter().next()
}

fn bundle_executable_name(bundle_path: &Path) -> Option<String> {
    let info_plist = Value::from_file(bundle_path.join("Contents/Info.plist")).ok()?;
    info_plist
        .as_dictionary()?
        .get("CFBundleExecutable")?
        .as_string()
        .map(ToString::to_string)
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

    apps.extend(discover_imported_apps(root));

    apps.sort_by(|a, b| {
        a.category
            .sort_rank()
            .cmp(&b.category.sort_rank())
            .then(a.name.cmp(&b.name))
    });
    apps
}

// ── Grid layout constants ──────────────────────────────────────────

const COLS: usize = 5;
const ROWS_VISIBLE: usize = 4;
const PAGE_SIZE: usize = COLS * ROWS_VISIBLE;
const WIN_W: f64 = 1288.0;
const WIN_H: f64 = 886.0;
const MARGIN: f64 = 28.0;
const HEADER_Y: f64 = 28.0;
const HEADER_H: f64 = 98.0;
const TOP_NAV_Y: f64 = 78.0;
const CONTENT_Y: f64 = 146.0;
const OPEN_APP_SLOTS: usize = 5;
const OPEN_PANEL_H: f64 = 88.0;
const OPEN_PANEL_Y: f64 = WIN_H - MARGIN - OPEN_PANEL_H;
const CONTENT_H: f64 = OPEN_PANEL_Y - CONTENT_Y - 18.0;
const LEFT_PANEL_X: f64 = MARGIN;
const LEFT_PANEL_W: f64 = 732.0;
const RIGHT_PANEL_X: f64 = LEFT_PANEL_X + LEFT_PANEL_W + 22.0;
const RIGHT_PANEL_W: f64 = WIN_W - RIGHT_PANEL_X - MARGIN;
const GRID_TITLE_Y: f64 = CONTENT_Y + 18.0;
const GRID_MESSAGE_Y: f64 = CONTENT_Y + 48.0;
const GRID_SUMMARY_Y: f64 = CONTENT_Y + 76.0;
const GRID_Y: f64 = CONTENT_Y + 112.0;
const CELL_W: f64 = 136.0;
const CELL_H: f64 = 116.0;
const CELL_GAP: f64 = 12.0;
const NAV_Y: f64 = CONTENT_Y + CONTENT_H - 56.0;
const PREVIEW_X: f64 = RIGHT_PANEL_X + 24.0;
const PREVIEW_Y: f64 = CONTENT_Y + 54.0;
const PREVIEW_W: f64 = RIGHT_PANEL_W - 48.0;
const PREVIEW_H: f64 = 156.0;
const STATS_X: f64 = RIGHT_PANEL_X + 24.0;
const STATS_Y: f64 = PREVIEW_Y + PREVIEW_H + 26.0;
const ACTION_PANEL_Y: f64 = CONTENT_Y + CONTENT_H - 188.0;
const ACTION_PANEL_H: f64 = 164.0;
const ACTION_BUTTON_W: f64 = (PREVIEW_W - 12.0) / 2.0;
const ACTION_BUTTON_H: f64 = 40.0;
const OPEN_BUTTON_GAP: f64 = 12.0;
const OPEN_BUTTON_X: f64 = MARGIN + 18.0;
const OPEN_BUTTON_Y: f64 = OPEN_PANEL_Y + 38.0;
const OPEN_BUTTON_H: f64 = 34.0;
const OPEN_BUTTON_W: f64 =
    (WIN_W - MARGIN * 2.0 - 36.0 - (OPEN_APP_SLOTS as f64 - 1.0) * OPEN_BUTTON_GAP)
        / OPEN_APP_SLOTS as f64;
const GAMEPLAY_LABEL_Y: f64 = CONTENT_Y + 274.0;
const GAMEPLAY_LABEL_H: f64 = 40.0;

const DESKTOP_BG: Color = Color::new(0.06, 0.08, 0.10, 1.0);
const PANEL_BG: Color = Color::new(0.14, 0.14, 0.14, 0.98);
const PANEL_ALT_BG: Color = Color::new(0.10, 0.15, 0.17, 0.98);
const SLOT_BG: Color = Color::new(0.16, 0.17, 0.18, 1.0);
const SLOT_EMPTY_BG: Color = Color::new(0.12, 0.13, 0.14, 1.0);
const PANEL_MUTED: Color = Color::new(0.21, 0.22, 0.22, 1.0);
const SHEIKAH_BLUE: Color = Color::new(0.125, 0.890, 1.000, 1.0);
const SHEIKAH_ORANGE: Color = Color::new(1.000, 0.580, 0.169, 1.0);
const NEUTRAL_TAN: Color = Color::new(0.725, 0.624, 0.396, 1.0);
const ACTIVE_GREEN: Color = Color::new(0.572, 0.773, 0.510, 1.0);
const TEXT_PRIMARY: Color = Color::new(0.992, 1.000, 0.878, 1.0);
const TEXT_MUTED: Color = Color::new(0.70, 0.74, 0.72, 1.0);
const TEXT_DARK: Color = Color::new(0.08, 0.10, 0.12, 1.0);

const FILTER_MODES: [FilterMode; 4] = [
    FilterMode::Desktop,
    FilterMode::All,
    FilterMode::Fixtures,
    FilterMode::Corpus,
];

const QUICK_LAUNCHES: [QuickLaunchSpec; 3] = [
    QuickLaunchSpec {
        title: "File Explorer",
        app_name: "File Explorer",
        accent: SHEIKAH_BLUE,
    },
    QuickLaunchSpec {
        title: "Foundation",
        app_name: "Foundation Explorer",
        accent: NEUTRAL_TAN,
    },
    QuickLaunchSpec {
        title: "Smoke Draw",
        app_name: "cocoa_draw",
        accent: SHEIKAH_ORANGE,
    },
];

const FILTER_BUTTON_START: usize = 0;
const GRID_BUTTON_START: usize = FILTER_BUTTON_START + FILTER_MODES.len();

const LABEL_HUD_COUNTS: usize = 7;
const LABEL_HEADER_SUMMARY: usize = 8;
const LABEL_GRID_TITLE: usize = 9;
const LABEL_GRID_MESSAGE: usize = 10;
const LABEL_GRID_SUMMARY: usize = 11;
const LABEL_INSPECTOR_HEADING: usize = 12;
const LABEL_PREVIEW: usize = 13;
const LABEL_PREVIEW_CAPTION: usize = 14;
const LABEL_SELECTED_TITLE: usize = 15;
const LABEL_CATEGORY: usize = 16;
const LABEL_SIZE: usize = 17;
const LABEL_MODIFIED: usize = 18;
const LABEL_PATH: usize = 19;
const LABEL_DESC_1: usize = 20;
const LABEL_DESC_2: usize = 21;
const LABEL_DESC_3: usize = 22;
const LABEL_ACTION_HINT: usize = 23;
const LABEL_FOOTER_HINT: usize = 24;
const LABEL_OPEN_STRIP_PANEL: usize = 25;
const LABEL_OPEN_STRIP_TITLE: usize = 26;
const LABEL_OPEN_STRIP_HINT: usize = 27;
const LABEL_GAMEPLAY_STATE: usize = 28;

fn open_button_index(page_size: usize) -> usize {
    GRID_BUTTON_START + page_size
}

fn quick_button_start(page_size: usize) -> usize {
    open_button_index(page_size) + 1
}

fn prev_button_index(page_size: usize) -> usize {
    quick_button_start(page_size) + QUICK_LAUNCHES.len()
}

fn next_button_index(page_size: usize) -> usize {
    prev_button_index(page_size) + 1
}

fn open_app_button_start(page_size: usize) -> usize {
    next_button_index(page_size) + 1
}

fn page_count(total: usize, page_size: usize) -> usize {
    total.max(1).div_ceil(page_size)
}

fn filter_index(filter: FilterMode) -> usize {
    FILTER_MODES
        .iter()
        .position(|candidate| *candidate == filter)
        .unwrap_or(0)
}

fn carousel_offset(active: FilterMode, candidate: FilterMode) -> isize {
    let len = FILTER_MODES.len() as isize;
    let mut delta = filter_index(candidate) as isize - filter_index(active) as isize;
    if delta > len / 2 {
        delta -= len;
    }
    if delta < -(len / 2) {
        delta += len;
    }
    delta
}

fn carousel_frame(offset: isize) -> Rect {
    match offset {
        -1 => Rect::new(332.0, TOP_NAV_Y + 5.0, 168.0, 28.0),
        0 => Rect::new((WIN_W - 228.0) / 2.0, TOP_NAV_Y - 6.0, 228.0, 46.0),
        1 => Rect::new(788.0, TOP_NAV_Y + 5.0, 168.0, 28.0),
        _ => Rect::new(-200.0, TOP_NAV_Y, 1.0, 1.0),
    }
}

fn open_app_frame(slot: usize) -> Rect {
    Rect::new(
        OPEN_BUTTON_X + slot as f64 * (OPEN_BUTTON_W + OPEN_BUTTON_GAP),
        OPEN_BUTTON_Y,
        OPEN_BUTTON_W,
        OPEN_BUTTON_H,
    )
}

fn hud_counts(metrics: DesktopMetrics) -> String {
    format!(
        "GUI {} | FIX {} | TOOL {} | DESKTOP {}",
        metrics.gui_apps, metrics.fixtures, metrics.corpus, metrics.desktop
    )
}

fn summary_text(
    filter: FilterMode,
    visible: usize,
    current_page: usize,
    total_pages: usize,
) -> String {
    format!(
        "{} | {} entries | Page {}/{}",
        filter.rail_name(),
        visible,
        current_page + 1,
        total_pages
    )
}

fn page_summary_text(
    filter: FilterMode,
    visible: usize,
    current_page: usize,
    total_pages: usize,
) -> String {
    format!(
        "{} | {} live slots | Page {}/{}",
        filter.description(),
        visible,
        current_page + 1,
        total_pages
    )
}

fn open_app_entries<'a>(apps: &'a [AppEntry], open_paths: &[PathBuf]) -> Vec<&'a AppEntry> {
    open_paths
        .iter()
        .filter_map(|path| apps.iter().find(|entry| entry.path == *path))
        .collect()
}

fn open_entry_slot<'a>(
    apps: &'a [AppEntry],
    open_paths: &[PathBuf],
    slot: usize,
) -> Option<&'a AppEntry> {
    open_paths
        .get(slot)
        .and_then(|path| apps.iter().find(|entry| entry.path == *path))
}

fn active_open_entry<'a>(apps: &'a [AppEntry], state: &LauncherState) -> Option<&'a AppEntry> {
    entry_by_path(apps, state.active_open_path.as_ref())
}

fn open_strip_hint(state: &LauncherState, open_count: usize) -> String {
    if open_count == 0 {
        return "Slots [1]-[5] stay reserved for open apps. D/A/F/T jump rails and arrows wrap the grid."
            .to_string();
    }

    if state.overlay_visible {
        return format!(
            "Focus {} | Tab cycles zones | 1-5 jump live apps | PageUp/PageDown flips pages | Esc returns",
            state.focus_zone.label()
        );
    }

    "Gameplay view | Esc reopens launcher | 1-5 jump live apps | Left/Right wrap the strip"
        .to_string()
}

fn open_slot_key_index(key: KeyCode) -> Option<usize> {
    match key {
        KeyCode::Num1 => Some(0),
        KeyCode::Num2 => Some(1),
        KeyCode::Num3 => Some(2),
        KeyCode::Num4 => Some(3),
        KeyCode::Num5 => Some(4),
        _ => None,
    }
}

fn gameplay_state_text(entry: Option<&AppEntry>) -> String {
    match entry {
        Some(entry) => format!("{} is live on the desktop", entry.name),
        None => "No live application is pinned yet".to_string(),
    }
}

fn cycle_focus_zone(state: &mut LauncherState) {
    if !state.overlay_visible {
        state.overlay_visible = true;
        state.focus_zone = FocusZone::Grid;
        state.last_message = "Launcher re-opened. Use arrows to browse the inventory.".to_string();
        return;
    }

    state.focus_zone = match (state.focus_zone, state.open_paths.is_empty()) {
        (FocusZone::Carousel, _) => FocusZone::Grid,
        (FocusZone::Grid, false) => FocusZone::OpenApps,
        (FocusZone::Grid, true) => FocusZone::Carousel,
        (FocusZone::OpenApps, _) => FocusZone::Carousel,
    };
    state.last_message = format!("{} focus ready.", state.focus_zone.label());
}

fn set_filter(state: &mut LauncherState, apps: &[AppEntry], page_size: usize, filter: FilterMode) {
    state.filter = filter;
    state.current_page = 0;
    state.focus_zone = FocusZone::Carousel;
    state.sync_selection(apps, page_size);
    state.last_message = format!("{} rail centered in the carousel.", state.filter.title());
}

fn change_page(state: &mut LauncherState, apps: &[AppEntry], page_size: usize, delta: isize) {
    let total_pages = page_count(filtered_entries(apps, state.filter).len(), page_size);
    let current = state.current_page as isize;
    let next = (current + delta).clamp(0, total_pages.saturating_sub(1) as isize) as usize;
    if next != state.current_page {
        state.current_page = next;
        state.sync_selection(apps, page_size);
        state.last_message = format!("Page {}/{} loaded.", state.current_page + 1, total_pages);
    } else if delta < 0 {
        state.last_message = "Already on the first page.".to_string();
    } else {
        state.last_message = "Already on the last page.".to_string();
    }
}

fn select_grid_slot(state: &mut LauncherState, apps: &[AppEntry], page_size: usize, slot: usize) {
    let total_pages = page_count(filtered_entries(apps, state.filter).len(), page_size);
    if let Some(entry) = catalog_entry(apps, state.filter, state.current_page, page_size, slot) {
        state.selected_path = Some(entry.path.clone());
        state.focus_zone = FocusZone::Grid;
        state.last_message = format!(
            "{} focused. Press Enter to launch or Esc to return.",
            entry.name
        );
    } else {
        state.last_message = format!(
            "Empty slot on page {}/{}.",
            state.current_page + 1,
            total_pages
        );
    }
}

fn move_grid_selection(
    state: &mut LauncherState,
    apps: &[AppEntry],
    page_size: usize,
    dx: isize,
    dy: isize,
) {
    let visible = page_entries(apps, state.filter, state.current_page, page_size);
    if visible.is_empty() {
        state.selected_path = None;
        state.last_message = format!("{} rail is empty.", state.filter.title());
        return;
    }

    let current = visible
        .iter()
        .position(|entry| {
            state
                .selected_path
                .as_ref()
                .is_some_and(|path| entry.path == *path)
        })
        .unwrap_or(0);
    let step = if dy != 0 { dy * COLS as isize } else { dx };
    let next = (current as isize + step).rem_euclid(visible.len() as isize) as usize;

    let entry = visible[next];
    state.selected_path = Some(entry.path.clone());
    state.focus_zone = FocusZone::Grid;
    state.last_message = format!(
        "{} focused. Press Enter to launch or Esc to return.",
        entry.name
    );
}

fn move_open_selection(state: &mut LauncherState, apps: &[AppEntry], delta: isize) {
    let open = open_app_entries(apps, &state.open_paths);
    if open.is_empty() {
        state.last_message = "No live GUI apps yet. Launch one to populate the strip.".to_string();
        return;
    }

    let current = state
        .active_open_path
        .as_ref()
        .and_then(|path| open.iter().position(|entry| entry.path == *path))
        .unwrap_or(open.len().saturating_sub(1));
    let next = (current as isize + delta).rem_euclid(open.len() as isize) as usize;
    state.active_open_path = Some(open[next].path.clone());
    state.focus_zone = FocusZone::OpenApps;
    state.last_message = format!("{} is ready on the live strip.", open[next].name);
}

fn activate_open_app_slot(state: &mut LauncherState, apps: &[AppEntry], slot: usize) {
    if let Some(entry) = open_entry_slot(apps, &state.open_paths, slot) {
        state.active_open_path = Some(entry.path.clone());
        state.overlay_visible = false;
        state.focus_zone = FocusZone::OpenApps;
        state.last_message = format!("Returned to {}.", entry.name);
    } else {
        state.last_message = format!("Live-app slot {} is empty.", slot + 1);
    }
}

fn toggle_overlay(state: &mut LauncherState, apps: &[AppEntry]) {
    if state.overlay_visible {
        if let Some(entry) = active_open_entry(apps, state) {
            state.overlay_visible = false;
            state.focus_zone = FocusZone::OpenApps;
            state.last_message = format!("Returned to {}.", entry.name);
        } else {
            state.last_message =
                "Launch a GUI app first, then Esc can drop back to it.".to_string();
        }
        return;
    }

    state.overlay_visible = true;
    state.focus_zone = FocusZone::Grid;
    state.last_message = "Launcher re-opened. Use arrows to browse the inventory.".to_string();
}

fn slot_button_text(entry: &AppEntry) -> String {
    format!("{} {}", entry.category.badge(), slot_name(&entry.name))
}

fn slot_name(name: &str) -> String {
    let parts: Vec<String> = name
        .split(|ch| matches!(ch, ' ' | '_' | '-'))
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_uppercase())
        .collect();

    if parts.is_empty() {
        return "ITEM".to_string();
    }

    let first = &parts[0];
    if let Some(second) = parts.get(1) {
        let combined = format!("{} {}", first, truncate_ascii(second, 4));
        if combined.chars().count() <= 12 {
            return combined;
        }
    }

    truncate_ascii(first, 12)
}

fn truncate_ascii(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    text.chars().take(max_chars).collect()
}

fn truncate_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }

    let head = (max_chars.saturating_sub(3)) / 2;
    let tail = max_chars.saturating_sub(3 + head);
    format!(
        "{}...{}",
        chars[..head].iter().collect::<String>(),
        chars[chars.len() - tail..].iter().collect::<String>()
    )
}

fn normalize_display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn human_size(bytes: u64) -> String {
    let units = [
        ("B", 1_u64),
        ("KB", 1024),
        ("MB", 1024 * 1024),
        ("GB", 1024 * 1024 * 1024),
    ];
    for (unit, divisor) in units.iter().rev() {
        if bytes >= *divisor {
            if *divisor == 1 {
                return format!("{} {}", bytes, unit);
            }
            return format!("{:.1} {}", bytes as f64 / *divisor as f64, unit);
        }
    }

    "0 B".to_string()
}

fn time_ago_text(time: SystemTime) -> String {
    match SystemTime::now().duration_since(time) {
        Ok(age) if age.as_secs() < 60 => "moments ago".to_string(),
        Ok(age) if age.as_secs() < 3600 => format!("{}m ago", age.as_secs() / 60),
        Ok(age) if age.as_secs() < 86_400 => format!("{}h ago", age.as_secs() / 3600),
        Ok(age) => format!("{}d ago", age.as_secs() / 86_400),
        Err(_) => "in flux".to_string(),
    }
}

fn monogram(name: &str) -> String {
    let mut letters: Vec<char> = name
        .split(|ch| matches!(ch, ' ' | '_' | '-'))
        .filter_map(|part| part.chars().find(|ch| ch.is_ascii_alphanumeric()))
        .map(|ch| ch.to_ascii_uppercase())
        .collect();

    if letters.len() >= 2 {
        return format!("{}{}", letters[0], letters[1]);
    }

    letters.clear();
    letters.extend(
        name.chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(2)
            .map(|ch| ch.to_ascii_uppercase()),
    );
    match letters.as_slice() {
        [first, second] => format!("{}{}", first, second),
        [only] => format!("{}{}", only, only),
        _ => "--".to_string(),
    }
}

fn panel(frame: Rect, color: Color) -> Label {
    let mut label = Label::new(frame, "");
    label.view_mut().background_color = color;
    label
}

fn styled_label(
    frame: Rect,
    text: impl Into<String>,
    color: Color,
    size: f64,
    bold: bool,
) -> Label {
    let mut label = Label::new(frame, text);
    let font = if bold {
        Font::system(size).with_bold()
    } else {
        Font::system(size)
    };
    label.set_font(font);
    label.set_text_color(color);
    label
}

fn set_button_palette(button: &mut Button, background: Color, text: Color) {
    button.view_mut().background_color = background;
    button.set_title_color(text);
}

fn filtered_entries<'a>(apps: &'a [AppEntry], filter: FilterMode) -> Vec<&'a AppEntry> {
    apps.iter().filter(|entry| filter.matches(entry)).collect()
}

fn page_entries<'a>(
    apps: &'a [AppEntry],
    filter: FilterMode,
    current_page: usize,
    page_size: usize,
) -> Vec<&'a AppEntry> {
    let filtered = filtered_entries(apps, filter);
    let page_start = current_page * page_size;
    filtered
        .into_iter()
        .skip(page_start)
        .take(page_size)
        .collect()
}

fn catalog_entry<'a>(
    apps: &'a [AppEntry],
    filter: FilterMode,
    current_page: usize,
    page_size: usize,
    slot: usize,
) -> Option<&'a AppEntry> {
    let filtered = filtered_entries(apps, filter);
    let page_start = current_page * page_size;
    filtered.get(page_start + slot).copied()
}

fn quick_launch_entry<'a>(apps: &'a [AppEntry], spec: QuickLaunchSpec) -> Option<&'a AppEntry> {
    apps.iter().find(|entry| entry.name == spec.app_name)
}

fn resolved_selection_path(
    apps: &[AppEntry],
    filter: FilterMode,
    current_page: usize,
    page_size: usize,
    selected_path: Option<&PathBuf>,
) -> Option<PathBuf> {
    let visible = page_entries(apps, filter, current_page, page_size);
    selected_path
        .and_then(|path| visible.iter().copied().find(|entry| entry.path == *path))
        .or_else(|| visible.first().copied())
        .map(|entry| entry.path.clone())
}

fn entry_by_path<'a>(
    apps: &'a [AppEntry],
    selected_path: Option<&PathBuf>,
) -> Option<&'a AppEntry> {
    selected_path.and_then(|path| apps.iter().find(|entry| entry.path == *path))
}

fn inspector_footer(filter: FilterMode, metrics: DesktopMetrics) -> String {
    match filter {
        FilterMode::Desktop => format!(
            "Desktop rail keeps {} window-ready slots first.",
            metrics.desktop
        ),
        FilterMode::All => format!("All {} launchers stay one page-turn away.", metrics.total),
        FilterMode::Fixtures => format!(
            "{} fixtures remain ready for smoke passes.",
            metrics.fixtures
        ),
        FilterMode::Corpus => format!("{} tools remain available for deep sweeps.", metrics.corpus),
    }
}

fn entry_story(entry: &AppEntry) -> [String; 3] {
    match entry.name.as_str() {
        "File Explorer" => [
            "Finder-style browsing with folders, files, and previews.".to_string(),
            "Use it when the desktop needs a real document surface.".to_string(),
            "Open launches the native GUI app directly.".to_string(),
        ],
        "Foundation Explorer" => [
            "Collection, object, and runtime coverage in a live window.".to_string(),
            "Useful when the shell needs deeper Cocoa evidence.".to_string(),
            "Open launches the native GUI app directly.".to_string(),
        ],
        "Hello Darwin" => [
            "Small native window proving the shell and event stack.".to_string(),
            "Use it for the fastest possible desktop smoke pass.".to_string(),
            "Open launches the native GUI app directly.".to_string(),
        ],
        "Control Gallery" => [
            "Widgets, layout, and interaction surfaces in one scene.".to_string(),
            "Useful for visual regressions across the native UI stack.".to_string(),
            "Open launches the native GUI app directly.".to_string(),
        ],
        _ => match entry.category {
            AppCategory::GuiApp => [
                "Native desktop app built in-repo for live windowed demos.".to_string(),
                "It stays on the desktop rail because it opens a real GUI.".to_string(),
                "Open launches the binary as a desktop subprocess.".to_string(),
            ],
            AppCategory::Fixture => [
                "Runtime fixture routed through macho-runtime for smoke checks.".to_string(),
                "Useful for guest-app probes and ABI behaviour validation.".to_string(),
                "Open runs the binary through the runtime loader.".to_string(),
            ],
            AppCategory::Corpus => [
                "Corpus tool for long-tail compatibility and CLI coverage.".to_string(),
                "Useful when the desktop needs a deeper runtime sweep.".to_string(),
                "Open executes the binary through the runtime loader.".to_string(),
            ],
        },
    }
}

#[derive(Debug, Clone)]
struct InspectorInfo {
    preview: String,
    badge_line: String,
    title: String,
    category: String,
    size: String,
    modified: String,
    path: String,
    description: [String; 3],
    action: String,
    footer: String,
    accent: Color,
}

fn inspector_info(
    entry: Option<&AppEntry>,
    root: &Path,
    filter: FilterMode,
    metrics: DesktopMetrics,
) -> InspectorInfo {
    match entry {
        Some(entry) => {
            let metadata = std::fs::metadata(&entry.path).ok();
            let relative = entry.path.strip_prefix(root).unwrap_or(&entry.path);
            let description = entry_story(entry);
            InspectorInfo {
                preview: monogram(&entry.name),
                badge_line: format!("{} | {}", entry.category.badge(), entry.category.label()),
                title: entry.name.clone(),
                category: format!("Class {}", entry.category.label()),
                size: format!(
                    "Load {}",
                    metadata
                        .as_ref()
                        .map(|file| human_size(file.len()))
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                modified: format!(
                    "Durability {}",
                    metadata
                        .as_ref()
                        .and_then(|file| file.modified().ok())
                        .map(time_ago_text)
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                path: format!(
                    "Path {}",
                    truncate_middle(&normalize_display_path(relative), 44)
                ),
                description,
                action: match entry.category {
                    AppCategory::GuiApp => "Open launches the selected desktop window.".to_string(),
                    AppCategory::Fixture => {
                        "Open runs the selected fixture through macho-runtime.".to_string()
                    }
                    AppCategory::Corpus => {
                        "Open executes the selected tool inside the loader.".to_string()
                    }
                },
                footer: inspector_footer(filter, metrics),
                accent: entry.category.color(),
            }
        }
        None => InspectorInfo {
            preview: "--".to_string(),
            badge_line: format!("{} | Empty Rail", filter.title()),
            title: format!("{} rail is empty", filter.title()),
            category: "Class unavailable".to_string(),
            size: "Load unavailable".to_string(),
            modified: "Durability unavailable".to_string(),
            path: "Path unavailable".to_string(),
            description: [
                "This rail has no entries on the current page.".to_string(),
                "Switch rails or page back to repopulate the deck.".to_string(),
                "The inspector will update as soon as a slot is focused.".to_string(),
            ],
            action: "Open is idle until a slot is selected.".to_string(),
            footer: inspector_footer(filter, metrics),
            accent: filter.accent(),
        },
    }
}

fn style_quick_button(button: &mut Button, spec: QuickLaunchSpec, available: bool) {
    button.view_mut().hidden = false;
    button.set_title(spec.title);
    button.set_enabled(true);
    set_button_palette(
        button,
        if available { spec.accent } else { PANEL_MUTED },
        if available { TEXT_DARK } else { TEXT_MUTED },
    );
}

fn style_filter_button(button: &mut Button, mode: FilterMode, active: FilterMode) {
    let offset = carousel_offset(active, mode);
    button.view_mut().hidden = false;
    button.view_mut().set_frame(carousel_frame(offset));
    button.set_title(mode.nav_title());
    button.set_enabled(true);
    set_button_palette(
        button,
        match offset {
            0 => mode.accent(),
            -1 | 1 => PANEL_ALT_BG,
            _ => PANEL_MUTED,
        },
        if offset == 0 { TEXT_DARK } else { TEXT_PRIMARY },
    );
}

fn style_inventory_button(button: &mut Button, entry: Option<&AppEntry>, selected: bool) {
    match entry {
        Some(entry) => {
            button.view_mut().hidden = false;
            button.set_enabled(true);
            button.set_title(slot_button_text(entry));
            set_button_palette(
                button,
                if selected { SHEIKAH_BLUE } else { SLOT_BG },
                if selected {
                    TEXT_DARK
                } else {
                    entry.category.color()
                },
            );
        }
        None => {
            button.view_mut().hidden = false;
            button.set_enabled(true);
            button.set_title("");
            set_button_palette(button, SLOT_EMPTY_BG, TEXT_MUTED);
        }
    }
}

fn style_open_button(button: &mut Button, available: bool) {
    button.view_mut().hidden = false;
    button.set_title("Open Selected");
    button.set_enabled(true);
    set_button_palette(
        button,
        if available { SHEIKAH_BLUE } else { PANEL_MUTED },
        if available { TEXT_DARK } else { TEXT_MUTED },
    );
}

fn style_nav_button(button: &mut Button, title: &str, enabled: bool) {
    button.view_mut().hidden = false;
    button.set_title(title);
    button.set_enabled(true);
    set_button_palette(
        button,
        if enabled { PANEL_ALT_BG } else { PANEL_MUTED },
        if enabled { TEXT_PRIMARY } else { TEXT_MUTED },
    );
}

fn style_open_app_button(button: &mut Button, entry: Option<&AppEntry>, active: bool, slot: usize) {
    match entry {
        Some(entry) => {
            button.view_mut().hidden = false;
            button.set_enabled(true);
            button.set_title(format!("[{}] {}", slot + 1, slot_name(&entry.name)));
            set_button_palette(
                button,
                if active { ACTIVE_GREEN } else { PANEL_ALT_BG },
                if active {
                    TEXT_DARK
                } else {
                    entry.category.color()
                },
            );
        }
        None => {
            button.view_mut().hidden = false;
            button.set_enabled(true);
            button.set_title(format!("[{}] ---", slot + 1));
            set_button_palette(button, PANEL_MUTED, TEXT_MUTED);
        }
    }
}

#[derive(Debug, Clone)]
struct LaunchOutcome {
    message: String,
    enters_gameplay: bool,
}

fn sync_desktop(
    buttons: &mut [Button],
    labels: &mut [Label],
    apps: &[AppEntry],
    root: &Path,
    metrics: DesktopMetrics,
    state: &LauncherState,
    page_size: usize,
) {
    for label in labels.iter_mut().take(LABEL_FOOTER_HINT + 1) {
        label.view_mut().hidden = !state.overlay_visible;
    }
    labels[LABEL_OPEN_STRIP_PANEL].view_mut().hidden = false;
    labels[LABEL_OPEN_STRIP_TITLE].view_mut().hidden = false;
    labels[LABEL_OPEN_STRIP_HINT].view_mut().hidden = false;
    labels[LABEL_GAMEPLAY_STATE].view_mut().hidden = state.overlay_visible;

    for (idx, mode) in FILTER_MODES.iter().enumerate() {
        style_filter_button(&mut buttons[FILTER_BUTTON_START + idx], *mode, state.filter);
        buttons[FILTER_BUTTON_START + idx].view_mut().hidden = !state.overlay_visible;
    }

    let filtered = filtered_entries(apps, state.filter);
    let total_pages = page_count(filtered.len(), page_size);
    let page = state.current_page.min(total_pages.saturating_sub(1));
    let visible = page_entries(apps, state.filter, page, page_size);
    let selected = state
        .selected_path
        .as_ref()
        .and_then(|path| visible.iter().copied().find(|entry| entry.path == *path))
        .or_else(|| visible.first().copied());
    let inspector = inspector_info(selected, root, state.filter, metrics);

    for slot in 0..page_size {
        let entry = visible.get(slot).copied();
        style_inventory_button(
            &mut buttons[GRID_BUTTON_START + slot],
            entry,
            selected.is_some_and(|selected_entry| {
                entry.is_some_and(|candidate| candidate.path == selected_entry.path)
            }),
        );
        buttons[GRID_BUTTON_START + slot].view_mut().hidden = !state.overlay_visible;
    }

    style_open_button(
        &mut buttons[open_button_index(page_size)],
        selected.is_some(),
    );
    buttons[open_button_index(page_size)].view_mut().hidden = !state.overlay_visible;
    for (idx, spec) in QUICK_LAUNCHES.iter().enumerate() {
        style_quick_button(
            &mut buttons[quick_button_start(page_size) + idx],
            *spec,
            quick_launch_entry(apps, *spec).is_some(),
        );
        buttons[quick_button_start(page_size) + idx]
            .view_mut()
            .hidden = !state.overlay_visible;
    }

    style_nav_button(
        &mut buttons[prev_button_index(page_size)],
        "< Prev",
        page > 0,
    );
    buttons[prev_button_index(page_size)].view_mut().hidden = !state.overlay_visible;
    style_nav_button(
        &mut buttons[next_button_index(page_size)],
        "Next >",
        page + 1 < total_pages,
    );
    buttons[next_button_index(page_size)].view_mut().hidden = !state.overlay_visible;

    let open = open_app_entries(apps, &state.open_paths);
    for slot in 0..OPEN_APP_SLOTS {
        let button = &mut buttons[open_app_button_start(page_size) + slot];
        button.view_mut().set_frame(open_app_frame(slot));
        let entry = open.get(slot).copied();
        style_open_app_button(
            button,
            entry,
            state
                .active_open_path
                .as_ref()
                .is_some_and(|path| entry.is_some_and(|candidate| candidate.path == *path)),
            slot,
        );
    }

    let live_entry = active_open_entry(apps, state);

    labels[LABEL_HUD_COUNTS].set_text(hud_counts(metrics));
    labels[LABEL_HEADER_SUMMARY].set_text(summary_text(
        state.filter,
        filtered.len(),
        page,
        total_pages,
    ));
    labels[LABEL_GRID_TITLE].set_text(state.filter.rail_name());
    labels[LABEL_GRID_MESSAGE].set_text(&state.last_message);
    labels[LABEL_GRID_SUMMARY].set_text(page_summary_text(
        state.filter,
        visible.len(),
        page,
        total_pages,
    ));
    labels[LABEL_INSPECTOR_HEADING].set_text("Selected Item");
    labels[LABEL_PREVIEW].set_text(&inspector.preview);
    labels[LABEL_PREVIEW].set_text_color(inspector.accent);
    labels[LABEL_PREVIEW_CAPTION].set_text(&inspector.badge_line);
    labels[LABEL_PREVIEW_CAPTION].set_text_color(inspector.accent);
    labels[LABEL_SELECTED_TITLE].set_text(&inspector.title);
    labels[LABEL_CATEGORY].set_text(&inspector.category);
    labels[LABEL_SIZE].set_text(&inspector.size);
    labels[LABEL_MODIFIED].set_text(&inspector.modified);
    labels[LABEL_PATH].set_text(&inspector.path);
    labels[LABEL_DESC_1].set_text(&inspector.description[0]);
    labels[LABEL_DESC_2].set_text(&inspector.description[1]);
    labels[LABEL_DESC_3].set_text(&inspector.description[2]);
    labels[LABEL_ACTION_HINT].set_text(&inspector.action);
    labels[LABEL_ACTION_HINT].set_text_color(inspector.accent);
    labels[LABEL_FOOTER_HINT].set_text(&inspector.footer);
    labels[LABEL_FOOTER_HINT].set_text_color(TEXT_MUTED);
    labels[LABEL_OPEN_STRIP_TITLE].set_text("Open Applications");
    labels[LABEL_OPEN_STRIP_TITLE].set_text_color(
        if state.focus_zone == FocusZone::OpenApps || !state.overlay_visible {
            ACTIVE_GREEN
        } else {
            TEXT_PRIMARY
        },
    );
    labels[LABEL_OPEN_STRIP_HINT].set_text(open_strip_hint(state, open.len()));
    labels[LABEL_OPEN_STRIP_HINT].set_text_color(TEXT_MUTED);
    labels[LABEL_GAMEPLAY_STATE].set_text(gameplay_state_text(live_entry));
    labels[LABEL_GAMEPLAY_STATE].set_text_color(
        live_entry
            .map(|entry| entry.category.color())
            .unwrap_or(TEXT_MUTED),
    );
    labels[LABEL_OPEN_STRIP_PANEL].view_mut().background_color = if state.overlay_visible {
        PANEL_BG
    } else {
        PANEL_ALT_BG
    };
}

fn launch_entry(entry: &AppEntry) -> LaunchOutcome {
    match entry.category {
        AppCategory::Fixture | AppCategory::Corpus => {
            let config = macho_runtime::RuntimeConfig::permissive(&entry.path);
            match macho_runtime::Runtime::load_and_run(&entry.path, &config) {
                Ok(result) => LaunchOutcome {
                    message: format!("{}: exit {}", entry.name, result.exit_code),
                    enters_gameplay: false,
                },
                Err(error) => LaunchOutcome {
                    message: format!("{}: error: {error}", entry.name),
                    enters_gameplay: false,
                },
            }
        }
        AppCategory::GuiApp => match std::process::Command::new(&entry.path).spawn() {
            Ok(_) => LaunchOutcome {
                message: format!("{}: launched in desktop session", entry.name),
                enters_gameplay: true,
            },
            Err(error) => LaunchOutcome {
                message: format!("{}: {error}", entry.name),
                enters_gameplay: false,
            },
        },
    }
}

fn apply_launch(state: &mut LauncherState, entry: &AppEntry, outcome: LaunchOutcome) {
    state.last_message = outcome.message;
    if outcome.enters_gameplay {
        state.register_open_app(&entry.path);
        state.overlay_visible = false;
        state.focus_zone = FocusZone::OpenApps;
    }
}

fn launch_selected_entry(state: &mut LauncherState, apps: &[AppEntry]) {
    if let Some(entry) = entry_by_path(apps, state.selected_path.as_ref()) {
        apply_launch(state, entry, launch_entry(entry));
    } else {
        state.last_message = format!("{} rail has nothing selected.", state.filter.title());
    }
}

fn launch_quick_entry(state: &mut LauncherState, apps: &[AppEntry], spec: QuickLaunchSpec) {
    if let Some(entry) = quick_launch_entry(apps, spec) {
        apply_launch(state, entry, launch_entry(entry));
    } else {
        state.last_message = format!("{} is unavailable in this build.", spec.title);
    }
}

fn handle_launcher_key(
    key: KeyCode,
    state: &mut LauncherState,
    apps: &[AppEntry],
    page_size: usize,
) -> bool {
    if let Some(slot) = open_slot_key_index(key) {
        activate_open_app_slot(state, apps, slot);
        return true;
    }

    match key {
        KeyCode::Escape => {
            toggle_overlay(state, apps);
            true
        }
        KeyCode::Tab => {
            cycle_focus_zone(state);
            true
        }
        KeyCode::D => {
            if state.overlay_visible {
                set_filter(state, apps, page_size, FilterMode::Desktop);
            }
            true
        }
        KeyCode::A => {
            if state.overlay_visible {
                set_filter(state, apps, page_size, FilterMode::All);
            }
            true
        }
        KeyCode::F => {
            if state.overlay_visible {
                set_filter(state, apps, page_size, FilterMode::Fixtures);
            }
            true
        }
        KeyCode::T => {
            if state.overlay_visible {
                set_filter(state, apps, page_size, FilterMode::Corpus);
            }
            true
        }
        KeyCode::PageUp => {
            if state.overlay_visible {
                change_page(state, apps, page_size, -1);
            } else {
                move_open_selection(state, apps, -1);
            }
            true
        }
        KeyCode::PageDown => {
            if state.overlay_visible {
                change_page(state, apps, page_size, 1);
            } else {
                move_open_selection(state, apps, 1);
            }
            true
        }
        KeyCode::Left => {
            if !state.overlay_visible {
                move_open_selection(state, apps, -1);
            } else {
                match state.focus_zone {
                    FocusZone::Carousel => {
                        let index = filter_index(state.filter);
                        let next = (index + FILTER_MODES.len() - 1) % FILTER_MODES.len();
                        set_filter(state, apps, page_size, FILTER_MODES[next]);
                    }
                    FocusZone::Grid => move_grid_selection(state, apps, page_size, -1, 0),
                    FocusZone::OpenApps => move_open_selection(state, apps, -1),
                }
            }
            true
        }
        KeyCode::Right => {
            if !state.overlay_visible {
                move_open_selection(state, apps, 1);
            } else {
                match state.focus_zone {
                    FocusZone::Carousel => {
                        let index = filter_index(state.filter);
                        let next = (index + 1) % FILTER_MODES.len();
                        set_filter(state, apps, page_size, FILTER_MODES[next]);
                    }
                    FocusZone::Grid => move_grid_selection(state, apps, page_size, 1, 0),
                    FocusZone::OpenApps => move_open_selection(state, apps, 1),
                }
            }
            true
        }
        KeyCode::Up => {
            if !state.overlay_visible {
                state.overlay_visible = true;
                state.focus_zone = FocusZone::Grid;
                state.last_message =
                    "Launcher re-opened. Use arrows to browse the inventory.".to_string();
            } else {
                match state.focus_zone {
                    FocusZone::Carousel => {
                        state.last_message = "Carousel is already focused.".to_string();
                    }
                    FocusZone::Grid => {
                        state.focus_zone = FocusZone::Carousel;
                        state.last_message =
                            "Carousel focused. Left and right spin the rails.".to_string();
                    }
                    FocusZone::OpenApps => {
                        state.focus_zone = FocusZone::Grid;
                        state.last_message =
                            "Inventory focused. Use arrows to select a slot.".to_string();
                    }
                }
            }
            true
        }
        KeyCode::Down => {
            if !state.overlay_visible {
                move_open_selection(state, apps, 1);
            } else {
                match state.focus_zone {
                    FocusZone::Carousel => {
                        state.focus_zone = FocusZone::Grid;
                        state.last_message =
                            "Inventory focused. Use arrows to select a slot.".to_string();
                    }
                    FocusZone::Grid => {
                        if state.open_paths.is_empty() {
                            state.last_message =
                                "Launch a GUI app first to unlock the open-app strip.".to_string();
                        } else {
                            state.focus_zone = FocusZone::OpenApps;
                            state.last_message =
                                "Open-app strip focused. Left and right choose the live app."
                                    .to_string();
                        }
                    }
                    FocusZone::OpenApps => {
                        state.last_message = "Open-app strip is already focused.".to_string();
                    }
                }
            }
            true
        }
        KeyCode::Return | KeyCode::Space => {
            if !state.overlay_visible {
                if let Some(entry) = active_open_entry(apps, state) {
                    state.last_message = format!("Returned to {}.", entry.name);
                } else {
                    state.overlay_visible = true;
                    state.focus_zone = FocusZone::Grid;
                    state.last_message =
                        "Launcher re-opened. No live app was selected.".to_string();
                }
            } else {
                match state.focus_zone {
                    FocusZone::Carousel => {
                        state.focus_zone = FocusZone::Grid;
                        state.last_message = format!(
                            "{} rail primed. Move through the grid to launch.",
                            state.filter.title()
                        );
                    }
                    FocusZone::Grid => launch_selected_entry(state, apps),
                    FocusZone::OpenApps => {
                        if let Some(slot) = state.active_open_path.as_ref().and_then(|path| {
                            state
                                .open_paths
                                .iter()
                                .position(|candidate| candidate == path)
                        }) {
                            activate_open_app_slot(state, apps, slot);
                        } else {
                            state.last_message =
                                "No live app is selected on the strip.".to_string();
                        }
                    }
                }
            }
            true
        }
        _ => false,
    }
}

fn main() {
    let root = workspace_root();
    let apps = discover_apps(&root);
    let metrics = DesktopMetrics::from_apps(&apps);
    let page_size = PAGE_SIZE;
    let state = Rc::new(RefCell::new(LauncherState::new(&apps, page_size)));
    let initial_state = state.borrow().clone();
    let total_pages = page_count(
        filtered_entries(&apps, initial_state.filter).len(),
        page_size,
    );
    let initial_visible = page_entries(
        &apps,
        initial_state.filter,
        initial_state.current_page,
        page_size,
    );
    let initial_inspector = inspector_info(
        entry_by_path(&apps, initial_state.selected_path.as_ref()),
        &root,
        initial_state.filter,
        metrics,
    );

    let config = WindowConfig {
        title: "Launchpad".into(),
        width: WIN_W,
        height: WIN_H,
        resizable: true,
    };

    let mut app = GuiApp::new(config);
    app.set_background(DESKTOP_BG);

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

    // ── Shell panels ───────────────────────────────────
    app.add_label(panel(
        Rect::new(MARGIN, HEADER_Y, WIN_W - MARGIN * 2.0, HEADER_H),
        PANEL_BG,
    ));
    app.add_label(panel(
        Rect::new(LEFT_PANEL_X, CONTENT_Y, LEFT_PANEL_W, CONTENT_H),
        PANEL_BG,
    ));
    app.add_label(panel(
        Rect::new(RIGHT_PANEL_X, CONTENT_Y, RIGHT_PANEL_W, CONTENT_H),
        PANEL_BG,
    ));
    app.add_label(panel(
        Rect::new(PREVIEW_X, PREVIEW_Y, PREVIEW_W, PREVIEW_H),
        PANEL_ALT_BG,
    ));
    app.add_label(panel(
        Rect::new(PREVIEW_X, ACTION_PANEL_Y, PREVIEW_W, ACTION_PANEL_H),
        PANEL_ALT_BG,
    ));
    app.add_label(panel(
        Rect::new(MARGIN, OPEN_PANEL_Y, WIN_W - MARGIN * 2.0, OPEN_PANEL_H),
        PANEL_BG,
    ));

    // ── Header labels ──────────────────────────────────
    app.add_label(styled_label(
        Rect::new(MARGIN + 18.0, HEADER_Y + 14.0, 220.0, 28.0),
        "BurdettieOS",
        NEUTRAL_TAN,
        28.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(MARGIN + 282.0, HEADER_Y + 12.0, 260.0, 34.0),
        "Inventory",
        TEXT_PRIMARY,
        34.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(MARGIN + 18.0, HEADER_Y + 48.0, 420.0, 20.0),
        hud_counts(metrics),
        TEXT_PRIMARY,
        15.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(WIN_W - 360.0, HEADER_Y + 22.0, 320.0, 20.0),
        summary_text(
            initial_state.filter,
            filtered_entries(&apps, initial_state.filter).len(),
            0,
            total_pages,
        ),
        TEXT_MUTED,
        14.0,
        false,
    ));

    // ── Grid labels ────────────────────────────────────
    app.add_label(styled_label(
        Rect::new(LEFT_PANEL_X + 20.0, GRID_TITLE_Y, 240.0, 20.0),
        initial_state.filter.rail_name(),
        TEXT_PRIMARY,
        16.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(LEFT_PANEL_X + 20.0, GRID_MESSAGE_Y, 640.0, 20.0),
        &initial_state.last_message,
        TEXT_PRIMARY,
        15.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(LEFT_PANEL_X + 20.0, GRID_SUMMARY_Y, 360.0, 18.0),
        page_summary_text(initial_state.filter, initial_visible.len(), 0, total_pages),
        TEXT_MUTED,
        13.0,
        false,
    ));

    // ── Inspector labels ───────────────────────────────
    app.add_label(styled_label(
        Rect::new(STATS_X, CONTENT_Y + 18.0, 180.0, 20.0),
        "Selected Item",
        TEXT_PRIMARY,
        16.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(PREVIEW_X + 132.0, PREVIEW_Y + 20.0, 180.0, 80.0),
        &initial_inspector.preview,
        initial_inspector.accent,
        72.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(PREVIEW_X + 22.0, PREVIEW_Y + 122.0, PREVIEW_W - 44.0, 18.0),
        &initial_inspector.badge_line,
        initial_inspector.accent,
        14.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y, PREVIEW_W, 26.0),
        &initial_inspector.title,
        TEXT_PRIMARY,
        24.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 36.0, PREVIEW_W, 18.0),
        &initial_inspector.category,
        TEXT_MUTED,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 64.0, PREVIEW_W, 18.0),
        &initial_inspector.size,
        TEXT_PRIMARY,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 92.0, PREVIEW_W, 18.0),
        &initial_inspector.modified,
        TEXT_PRIMARY,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 120.0, PREVIEW_W, 18.0),
        &initial_inspector.path,
        TEXT_MUTED,
        13.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 154.0, PREVIEW_W, 18.0),
        &initial_inspector.description[0],
        TEXT_MUTED,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 182.0, PREVIEW_W, 18.0),
        &initial_inspector.description[1],
        TEXT_MUTED,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(STATS_X, STATS_Y + 210.0, PREVIEW_W, 18.0),
        &initial_inspector.description[2],
        TEXT_MUTED,
        14.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(
            PREVIEW_X + 18.0,
            ACTION_PANEL_Y + 18.0,
            PREVIEW_W - 36.0,
            18.0,
        ),
        &initial_inspector.action,
        initial_inspector.accent,
        14.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(
            PREVIEW_X + 18.0,
            ACTION_PANEL_Y + 144.0,
            PREVIEW_W - 36.0,
            18.0,
        ),
        &initial_inspector.footer,
        TEXT_MUTED,
        12.0,
        false,
    ));
    app.add_label(styled_label(
        Rect::new(MARGIN + 18.0, OPEN_PANEL_Y + 10.0, 220.0, 18.0),
        "Open Applications",
        TEXT_PRIMARY,
        14.0,
        true,
    ));
    app.add_label(styled_label(
        Rect::new(
            MARGIN + 244.0,
            OPEN_PANEL_Y + 10.0,
            WIN_W - MARGIN * 2.0 - 262.0,
            18.0,
        ),
        open_strip_hint(&initial_state, 0),
        TEXT_MUTED,
        12.0,
        false,
    ));
    let mut gameplay_label = styled_label(
        Rect::new(
            MARGIN + 120.0,
            GAMEPLAY_LABEL_Y,
            WIN_W - 240.0,
            GAMEPLAY_LABEL_H,
        ),
        gameplay_state_text(None),
        TEXT_MUTED,
        30.0,
        true,
    );
    gameplay_label.view_mut().hidden = true;
    app.add_label(gameplay_label);

    // ── Filter buttons ────────────────────────────────
    for mode in FILTER_MODES {
        let mut button = Button::new(
            carousel_frame(carousel_offset(initial_state.filter, mode)),
            mode.nav_title(),
        );
        style_filter_button(&mut button, mode, initial_state.filter);
        app.add_button(button);
    }

    // ── Grid buttons ───────────────────────────────────
    for i in 0..page_size {
        let col = i % COLS;
        let row = i / COLS;
        let x = LEFT_PANEL_X + 2.0 + col as f64 * (CELL_W + CELL_GAP);
        let y = GRID_Y + row as f64 * (CELL_H + CELL_GAP);
        let mut button = Button::new(Rect::new(x, y, CELL_W, CELL_H), "");
        let entry = catalog_entry(&apps, initial_state.filter, 0, page_size, i);
        let selected = initial_state
            .selected_path
            .as_ref()
            .is_some_and(|path| entry.is_some_and(|candidate| candidate.path == *path));
        style_inventory_button(&mut button, entry, selected);
        app.add_button(button);
    }

    // ── Action buttons ─────────────────────────────────
    let mut open_button = Button::new(
        Rect::new(
            PREVIEW_X + 18.0,
            ACTION_PANEL_Y + 46.0,
            ACTION_BUTTON_W,
            ACTION_BUTTON_H,
        ),
        "Open Selected",
    );
    style_open_button(&mut open_button, initial_state.selected_path.is_some());
    app.add_button(open_button);

    for (idx, spec) in QUICK_LAUNCHES.iter().enumerate() {
        let (x, y) = match idx {
            0 => (
                PREVIEW_X + 18.0 + ACTION_BUTTON_W + 12.0,
                ACTION_PANEL_Y + 46.0,
            ),
            1 => (PREVIEW_X + 18.0, ACTION_PANEL_Y + 94.0),
            _ => (
                PREVIEW_X + 18.0 + ACTION_BUTTON_W + 12.0,
                ACTION_PANEL_Y + 94.0,
            ),
        };
        let mut button = Button::new(
            Rect::new(x, y, ACTION_BUTTON_W, ACTION_BUTTON_H),
            spec.title,
        );
        style_quick_button(
            &mut button,
            *spec,
            quick_launch_entry(&apps, *spec).is_some(),
        );
        app.add_button(button);
    }

    // ── Navigation buttons ─────────────────────────────
    let prev_x = LEFT_PANEL_X + LEFT_PANEL_W - 296.0;
    let mut prev_button = Button::new(Rect::new(prev_x, NAV_Y, 136.0, 36.0), "< Prev");
    style_nav_button(&mut prev_button, "< Prev", false);
    app.add_button(prev_button);
    let mut next_button = Button::new(Rect::new(prev_x + 152.0, NAV_Y, 136.0, 36.0), "Next >");
    style_nav_button(&mut next_button, "Next >", total_pages > 1);
    app.add_button(next_button);
    for slot in 0..OPEN_APP_SLOTS {
        let mut button = Button::new(open_app_frame(slot), "");
        style_open_app_button(&mut button, None, false, slot);
        app.add_button(button);
    }

    // ── Click handler ──────────────────────────────────
    let apps_clone = apps.clone();
    let metrics_clone = metrics;
    let root_clone = root.clone();
    let click_state = Rc::clone(&state);

    app.on_button_click_with_buttons(move |idx, labels, buttons, _tfs| {
        let mut state = click_state.borrow_mut();

        if idx < GRID_BUTTON_START {
            set_filter(
                &mut state,
                &apps_clone,
                page_size,
                FILTER_MODES[idx - FILTER_BUTTON_START],
            );
        } else if idx < GRID_BUTTON_START + page_size {
            select_grid_slot(&mut state, &apps_clone, page_size, idx - GRID_BUTTON_START);
        } else if idx == open_button_index(page_size) {
            launch_selected_entry(&mut state, &apps_clone);
        } else if idx < quick_button_start(page_size) + QUICK_LAUNCHES.len() {
            let spec = QUICK_LAUNCHES[idx - quick_button_start(page_size)];
            launch_quick_entry(&mut state, &apps_clone, spec);
        } else if idx == prev_button_index(page_size) {
            change_page(&mut state, &apps_clone, page_size, -1);
        } else if idx == next_button_index(page_size) {
            change_page(&mut state, &apps_clone, page_size, 1);
        } else if idx < open_app_button_start(page_size) + OPEN_APP_SLOTS {
            activate_open_app_slot(
                &mut state,
                &apps_clone,
                idx - open_app_button_start(page_size),
            );
        }

        state.sync_selection(&apps_clone, page_size);
        sync_desktop(
            buttons,
            labels,
            &apps_clone,
            &root_clone,
            metrics_clone,
            &state,
            page_size,
        );
    });

    let apps_key = apps.clone();
    let metrics_key = metrics;
    let root_key = root.clone();
    let key_state = Rc::clone(&state);
    app.on_key_down(move |key, labels, buttons, _tfs| {
        let mut state = key_state.borrow_mut();
        if !handle_launcher_key(key, &mut state, &apps_key, page_size) {
            return false;
        }

        state.sync_selection(&apps_key, page_size);
        sync_desktop(
            buttons,
            labels,
            &apps_key,
            &root_key,
            metrics_key,
            &state,
            page_size,
        );
        true
    });

    // ── Layout change callback ─────────────────────────
    app.scene_mut().on_layout_change(move |_layout| {
        // Layout label update is handled by the status label
        // (We'd need shared state to update from here; the scene
        //  already stores the layout for character remapping.)
    });

    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_apps(count: usize) -> Vec<AppEntry> {
        (0..count)
            .map(|idx| AppEntry {
                name: format!("App {idx:02}"),
                path: PathBuf::from(format!("/tmp/app-{idx:02}")),
                category: match idx % 3 {
                    0 => AppCategory::GuiApp,
                    1 => AppCategory::Corpus,
                    _ => AppCategory::Fixture,
                },
            })
            .collect()
    }

    fn pinned_sample_apps() -> Vec<AppEntry> {
        vec![
            AppEntry {
                name: "File Explorer".to_string(),
                path: PathBuf::from("/tmp/file-explorer"),
                category: AppCategory::GuiApp,
            },
            AppEntry {
                name: "cocoa_draw".to_string(),
                path: PathBuf::from("/tmp/cocoa_draw"),
                category: AppCategory::Fixture,
            },
        ]
    }

    fn fixture_apps(count: usize) -> Vec<AppEntry> {
        (0..count)
            .map(|idx| AppEntry {
                name: format!("Fixture {idx:02}"),
                path: PathBuf::from(format!("/tmp/fixture-{idx:02}")),
                category: AppCategory::Fixture,
            })
            .collect()
    }

    #[test]
    fn desktop_filter_keeps_gui_apps_and_fixtures() {
        let apps = sample_apps(12);
        let filtered = filtered_entries(&apps, FilterMode::Desktop);

        assert!(!filtered.is_empty());
        assert!(filtered
            .iter()
            .all(|entry| { matches!(entry.category, AppCategory::GuiApp | AppCategory::Fixture) }));
    }

    #[test]
    fn catalog_entry_returns_none_for_empty_last_page_slot() {
        let page_size = 5;
        let apps = fixture_apps(page_size + 1);
        let last_page = page_count(apps.len(), page_size) - 1;

        assert!(catalog_entry(&apps, FilterMode::Fixtures, last_page, page_size, 1).is_none());
    }

    #[test]
    fn quick_launch_entry_resolves_named_target() {
        let apps = pinned_sample_apps();

        assert_eq!(
            quick_launch_entry(&apps, QUICK_LAUNCHES[0]).map(|entry| entry.name.as_str()),
            Some("File Explorer")
        );
        assert_eq!(
            quick_launch_entry(&apps, QUICK_LAUNCHES[2]).map(|entry| entry.name.as_str()),
            Some("cocoa_draw")
        );
    }

    #[test]
    fn slot_name_condenses_multiword_titles() {
        assert_eq!(slot_name("File Explorer"), "FILE EXPL");
        assert_eq!(slot_name("Hello Darwin"), "HELLO DARW");
        assert_eq!(slot_name("dispatch_test"), "DISPATCH");
    }

    #[test]
    fn carousel_offset_wraps_edges() {
        assert_eq!(carousel_offset(FilterMode::Desktop, FilterMode::Corpus), -1);
        assert_eq!(carousel_offset(FilterMode::Corpus, FilterMode::Desktop), 1);
        assert_eq!(carousel_offset(FilterMode::All, FilterMode::Fixtures), 1);
    }

    #[test]
    fn register_open_app_keeps_recent_slots_only() {
        let apps = sample_apps(10);
        let mut state = LauncherState::new(&apps, PAGE_SIZE);

        for entry in apps.iter().take(OPEN_APP_SLOTS + 2) {
            state.register_open_app(&entry.path);
        }

        assert_eq!(state.open_paths.len(), OPEN_APP_SLOTS);
        assert_eq!(
            state.active_open_path,
            Some(apps[OPEN_APP_SLOTS + 1].path.clone())
        );
        assert_eq!(state.open_paths.first(), Some(&apps[2].path));
    }

    #[test]
    fn escape_hides_overlay_when_live_app_exists() {
        let apps = sample_apps(6);
        let mut state = LauncherState::new(&apps, PAGE_SIZE);
        state.register_open_app(&apps[0].path);

        assert!(state.overlay_visible);
        assert!(handle_launcher_key(
            KeyCode::Escape,
            &mut state,
            &apps,
            PAGE_SIZE
        ));
        assert!(!state.overlay_visible);
        assert_eq!(state.focus_zone, FocusZone::OpenApps);
    }

    #[test]
    fn move_grid_selection_wraps_across_visible_entries() {
        let apps = sample_apps(7);
        let mut state = LauncherState::new(&apps, PAGE_SIZE);
        state.filter = FilterMode::All;
        state.selected_path = Some(apps[0].path.clone());

        move_grid_selection(&mut state, &apps, PAGE_SIZE, -1, 0);
        assert_eq!(state.selected_path, Some(apps[6].path.clone()));

        move_grid_selection(&mut state, &apps, PAGE_SIZE, 1, 0);
        assert_eq!(state.selected_path, Some(apps[0].path.clone()));
    }

    #[test]
    fn numeric_hotkey_activates_open_app_slot() {
        let apps = sample_apps(6);
        let mut state = LauncherState::new(&apps, PAGE_SIZE);
        state.register_open_app(&apps[1].path);
        state.register_open_app(&apps[2].path);
        state.overlay_visible = true;

        assert!(handle_launcher_key(
            KeyCode::Num2,
            &mut state,
            &apps,
            PAGE_SIZE
        ));
        assert_eq!(state.active_open_path, Some(apps[2].path.clone()));
        assert!(!state.overlay_visible);
    }

    #[test]
    fn imported_dependency_classifier_accepts_public_surface() {
        let class = classify_imported_dependencies([
            "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
            "/System/Library/Frameworks/Foundation.framework/Versions/C/Foundation",
            "/usr/lib/libobjc.A.dylib",
            "/usr/lib/libSystem.B.dylib",
        ]);

        assert_eq!(class, ImportedDependencyClass::PublicSurface);
    }

    #[test]
    fn imported_dependency_classifier_rejects_private_frameworks() {
        let class = classify_imported_dependencies([
            "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
            "/System/Library/PrivateFrameworks/DVTFoundation.framework/DVTFoundation",
        ]);

        assert_eq!(class, ImportedDependencyClass::PrivateHeavy);
    }

    #[test]
    fn imported_dependency_classifier_marks_bundled_links_mixed() {
        let class = classify_imported_dependencies([
            "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
            "@rpath/libwezterm_gui.dylib",
        ]);

        assert_eq!(class, ImportedDependencyClass::Mixed);
    }

    #[test]
    fn discover_apps_includes_imported_manifest_bundle() {
        let root = tempdir().unwrap();
        let bundle_dir = root.path().join("imports/apps/WezTerm.app/Contents/MacOS");
        std::fs::create_dir_all(&bundle_dir).unwrap();
        std::fs::write(bundle_dir.join("wezterm-gui"), b"#!/bin/sh\n").unwrap();
        std::fs::create_dir_all(root.path().join("imports")).unwrap();
        std::fs::write(
            root.path().join("imports/manifest.json"),
            r#"{
    "schema_version": 1,
    "workflow": "docker-first",
    "generated_at": "2026-04-12T00:00:00Z",
    "source_host": {
        "capture_kind": "manual-export",
        "machine_name": "test-mac",
        "macos_version": "15.4",
        "macos_build": "24E214"
    },
    "artifacts": [
        {
            "id": "wezterm-app",
            "kind": "app-bundle",
            "label": "WezTerm.app imported terminal target",
            "source": {
                "host_path": "/Applications/WezTerm.app",
                "extracted_at": "2026-04-12T00:00:00Z",
                "transfer_method": "manual-copy"
            },
            "storage": {
                "overlay_path": "imports/apps/WezTerm.app",
                "container_path": "/opt/imported-artifacts/apps/WezTerm.app",
                "read_only": true
            },
            "integrity": {
                "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
                "architectures": ["arm64"]
            },
            "classification": {
                "tier": "validation-only",
                "dependency_class": "mixed",
                "scoreable_in_docker": false
            },
            "entries": [
                {
                    "relative_path": "Contents/MacOS/wezterm-gui",
                    "role": "main-executable",
                    "executable": true
                }
            ]
        }
    ]
}"#,
        )
        .unwrap();

        let apps = discover_apps(root.path());
        let imported = apps.iter().find(|entry| entry.name == "WezTerm").unwrap();

        assert_eq!(imported.category, AppCategory::Corpus);
        assert_eq!(
            imported.path,
            root.path()
                .join("imports/apps/WezTerm.app/Contents/MacOS/wezterm-gui")
        );
    }

    #[test]
    fn discover_apps_falls_back_to_bundle_metadata_without_manifest() {
        let root = tempdir().unwrap();
        let bundle_root = root.path().join("imports/apps/iTerm.app");
        let macos_dir = bundle_root.join("Contents/MacOS");
        std::fs::create_dir_all(&macos_dir).unwrap();
        std::fs::write(macos_dir.join("iTerm2"), b"#!/bin/sh\n").unwrap();
        std::fs::write(
            bundle_root.join("Contents/Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>iTerm2</string>
</dict>
</plist>
"#,
        )
        .unwrap();

        let apps = discover_apps(root.path());
        let imported = apps.iter().find(|entry| entry.name == "iTerm").unwrap();

        assert_eq!(imported.category, AppCategory::Corpus);
        assert_eq!(
            imported.path,
            root.path()
                .join("imports/apps/iTerm.app/Contents/MacOS/iTerm2")
        );
    }
}
