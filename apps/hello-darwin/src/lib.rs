//! Shared logic for hello-darwin — fixture discovery and scorecard.

use macho_runtime::{Runtime, RuntimeConfig};
use std::path::{Path, PathBuf};

/// Locate the fixtures directory by checking:
/// 1. `DARWIN_LOADER_ROOT` env var
/// 2. Walking up from the current executable
pub fn fixtures_root() -> PathBuf {
    if let Ok(root) = std::env::var("DARWIN_LOADER_ROOT") {
        let p = PathBuf::from(root).join("fixtures");
        if p.is_dir() {
            return p;
        }
    }

    // Walk up from target/debug/<bin> to workspace root (3 levels)
    let mut p = std::env::current_exe().unwrap_or_default();
    for _ in 0..3 {
        p.pop();
    }
    p.join("fixtures")
}

/// Discover fixture binaries under a root directory.
///
/// Returns sorted `(name, binary_path)` pairs, skipping `malformed/`.
pub fn discover_fixtures(root: &Path) -> Vec<(String, PathBuf)> {
    let mut entries = Vec::new();
    let Ok(dirs) = std::fs::read_dir(root) else {
        return entries;
    };
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
            entries.push((name, bin));
            continue;
        }
        let bin2 = dir.join(&name);
        if bin2.is_file() {
            entries.push((name, bin2));
        }
    }
    entries.sort();
    entries
}

/// Run all fixtures and return a summary string like "Scorecard: 60/62 fixtures passed".
pub fn run_scorecard() -> String {
    let root = fixtures_root();
    run_scorecard_from(&root)
}

/// Run all fixtures under `root` and return a summary string.
pub fn run_scorecard_from(root: &Path) -> String {
    if !root.is_dir() {
        return "Fixtures directory not found.".into();
    }

    let entries = discover_fixtures(root);
    let total = entries.len();
    let mut passed = 0usize;
    for (name, binary) in &entries {
        let expected = if name == "return42" { 42 } else { 0 };
        let config = RuntimeConfig::permissive(binary);
        if let Ok(result) = Runtime::load_and_run(binary, &config) {
            if result.exit_code == expected {
                passed += 1;
            }
        }
    }
    format!("Scorecard: {passed}/{total} fixtures passed")
}
