//! Core runner — executes a binary under the runtime and captures gaps.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

use macho_runtime::{EventKind, Runtime, RuntimeConfig, RuntimeTracer};

use crate::report::{BinaryResult, Stages};

/// Serializes runtime calls (global ObjC state is process-wide).
static RUNNER_LOCK: Mutex<()> = Mutex::new(());

/// A corpus entry describing a binary to test.
#[derive(Debug, Clone)]
pub struct CorpusEntry {
    /// Display name.
    pub name: String,
    /// Path to the arm64 Mach-O binary.
    pub path: std::path::PathBuf,
    /// Category tag for grouping.
    pub category: String,
    /// Expected exit code.
    pub expected_exit: i32,
}

/// Run a single binary and produce a structured result.
pub fn run_binary(entry: &CorpusEntry) -> BinaryResult {
    let path = &entry.path;

    if !path.exists() {
        return BinaryResult {
            name: entry.name.clone(),
            path: path.display().to_string(),
            category: entry.category.clone(),
            success: false,
            exit_code: None,
            expected_exit: entry.expected_exit,
            stages: Stages {
                parse: false,
                load: false,
                fixups: false,
                enter_main: false,
                complete: false,
                exit_correct: false,
            },
            missing_symbols: Vec::new(),
            missing_classes: Vec::new(),
            missing_selectors: Vec::new(),
            images_loaded: 0,
            shims_installed: 0,
            error: Some(format!("binary not found: {}", path.display())),
        };
    }

    let mut config = RuntimeConfig::permissive(path);
    let tracer = RuntimeTracer::new();
    config.tracer = Some(tracer.clone());

    let _lock = RUNNER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    match Runtime::load_and_run(path, &config) {
        Ok(result) => {
            let (missing_symbols, missing_classes, missing_selectors) =
                extract_gaps_from_tracer(&tracer);

            let exit_correct = result.exit_code == entry.expected_exit;
            BinaryResult {
                name: entry.name.clone(),
                path: path.display().to_string(),
                category: entry.category.clone(),
                success: exit_correct,
                exit_code: Some(result.exit_code),
                expected_exit: entry.expected_exit,
                stages: Stages {
                    parse: true,
                    load: true,
                    fixups: true,
                    enter_main: true,
                    complete: true,
                    exit_correct,
                },
                missing_symbols,
                missing_classes,
                missing_selectors,
                images_loaded: result.images_loaded,
                shims_installed: result.shims_installed,
                error: None,
            }
        }
        Err(e) => {
            let msg = format!("{e}");
            let (missing_symbols, missing_classes, missing_selectors) =
                extract_gaps_from_tracer(&tracer);
            let stages = infer_stages(&msg);

            BinaryResult {
                name: entry.name.clone(),
                path: path.display().to_string(),
                category: entry.category.clone(),
                success: false,
                exit_code: None,
                expected_exit: entry.expected_exit,
                stages,
                missing_symbols,
                missing_classes,
                missing_selectors,
                images_loaded: 0,
                shims_installed: 0,
                error: Some(msg),
            }
        }
    }
}

/// Scan a directory for corpus entries. Each subdirectory should contain
/// an `app` binary or a binary named after the directory, plus an optional
/// `meta.json` with `{ "category": "...", "expected_exit": N }`.
pub fn discover_corpus(dir: &Path) -> Vec<CorpusEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let sub = entry.path();
        if !sub.is_dir() {
            continue;
        }

        let name = sub.file_name().unwrap().to_string_lossy().to_string();

        // Find binary: prefer `app`, then directory name
        let bin = sub.join("app");
        let bin = if bin.is_file() {
            bin
        } else {
            let alt = sub.join(&name);
            if alt.is_file() {
                alt
            } else {
                continue;
            }
        };

        // Read optional metadata
        let (category, expected_exit) = read_meta(&sub);

        entries.push(CorpusEntry {
            name,
            path: bin,
            category,
            expected_exit,
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Read `meta.json` from a corpus entry directory.
fn read_meta(dir: &Path) -> (String, i32) {
    let meta_path = dir.join("meta.json");
    if let Ok(data) = std::fs::read_to_string(&meta_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&data) {
            let category = val
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let expected_exit = val
                .get("expected_exit")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            return (category, expected_exit);
        }
    }
    ("unknown".to_string(), 0)
}

/// Extract missing symbols/classes/selectors from tracer events.
fn extract_gaps_from_tracer(
    tracer: &RuntimeTracer,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut missing_symbols = BTreeSet::new();
    let mut missing_classes = BTreeSet::new();
    let mut missing_selectors = BTreeSet::new();

    for event in tracer.events() {
        let msg = &event.message;
        match event.kind {
            EventKind::Pipeline => {
                // Look for symbol resolution failures logged by the runtime
                if msg.contains("unresolved symbol") || msg.contains("symbol not found") {
                    if let Some(sym) = extract_quoted(msg) {
                        missing_symbols.insert(sym);
                    }
                }
            }
            EventKind::MethodLookup => {
                if msg.contains("miss") || msg.contains("not found") {
                    // Try to extract selector and class
                    if let Some(sel) = extract_after(msg, "selector '") {
                        missing_selectors.insert(sel);
                    }
                    if let Some(cls) = extract_after(msg, "class '") {
                        missing_classes.insert(cls);
                    }
                }
            }
            _ => {}
        }
    }

    (
        missing_symbols.into_iter().collect(),
        missing_classes.into_iter().collect(),
        missing_selectors.into_iter().collect(),
    )
}

/// Extract a single-quoted substring from a message.
fn extract_quoted(msg: &str) -> Option<String> {
    let start = msg.find('\'')?;
    let rest = &msg[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Extract text between `prefix` and the next `'` character.
fn extract_after(msg: &str, prefix: &str) -> Option<String> {
    let idx = msg.find(prefix)?;
    let rest = &msg[idx + prefix.len()..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Infer which stages succeeded from an error message.
fn infer_stages(msg: &str) -> Stages {
    if msg.contains("parse") || msg.contains("magic") || msg.contains("truncat") {
        Stages {
            parse: false,
            load: false,
            fixups: false,
            enter_main: false,
            complete: false,
            exit_correct: false,
        }
    } else if msg.contains("dependency") || msg.contains("not found") || msg.contains("map") {
        Stages {
            parse: true,
            load: false,
            fixups: false,
            enter_main: false,
            complete: false,
            exit_correct: false,
        }
    } else if msg.contains("fixup") || msg.contains("bind") || msg.contains("rebase") {
        Stages {
            parse: true,
            load: true,
            fixups: false,
            enter_main: false,
            complete: false,
            exit_correct: false,
        }
    } else if msg.contains("entry") {
        Stages {
            parse: true,
            load: true,
            fixups: true,
            enter_main: false,
            complete: false,
            exit_correct: false,
        }
    } else {
        // Crashed during execution
        Stages {
            parse: true,
            load: true,
            fixups: true,
            enter_main: true,
            complete: false,
            exit_correct: false,
        }
    }
}
