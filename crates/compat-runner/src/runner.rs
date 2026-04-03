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

#[cfg(test)]
mod tests {
    use super::*;

    // ── infer_stages ───────────────────────────────────────────

    #[test]
    fn infer_stages_parse_error() {
        let stages = infer_stages("invalid magic number");
        assert!(!stages.parse);
        assert!(!stages.load);
    }

    #[test]
    fn infer_stages_truncated() {
        let stages = infer_stages("file truncated at offset 0x100");
        assert!(!stages.parse);
    }

    #[test]
    fn infer_stages_dependency_not_found() {
        let stages = infer_stages("dependency not found: libFoo.dylib");
        assert!(stages.parse);
        assert!(!stages.load);
        assert!(!stages.fixups);
    }

    #[test]
    fn infer_stages_map_failure() {
        let stages = infer_stages("failed to map segment __TEXT");
        assert!(stages.parse);
        assert!(!stages.load);
    }

    #[test]
    fn infer_stages_fixup_failure() {
        let stages = infer_stages("fixup target segment index 5 out of range");
        assert!(stages.parse);
        assert!(stages.load);
        assert!(!stages.fixups);
        assert!(!stages.enter_main);
    }

    #[test]
    fn infer_stages_bind_failure() {
        let stages = infer_stages("failed to bind symbol _foo");
        assert!(stages.parse);
        assert!(stages.load);
        assert!(!stages.fixups);
    }

    #[test]
    fn infer_stages_rebase_failure() {
        let stages = infer_stages("rebase offset out of bounds");
        assert!(stages.parse);
        assert!(stages.load);
        assert!(!stages.fixups);
    }

    #[test]
    fn infer_stages_entry_failure() {
        let stages = infer_stages("no entry point found");
        assert!(stages.parse);
        assert!(stages.load);
        assert!(stages.fixups);
        assert!(!stages.enter_main);
    }

    #[test]
    fn infer_stages_crash() {
        let stages = infer_stages("SIGSEGV at 0xdeadbeef");
        assert!(stages.parse);
        assert!(stages.load);
        assert!(stages.fixups);
        assert!(stages.enter_main);
        assert!(!stages.complete);
    }

    #[test]
    fn infer_stages_unknown_error() {
        let stages = infer_stages("something completely unexpected happened");
        // Falls through to default — assumed crash during execution
        assert!(stages.parse);
        assert!(stages.load);
        assert!(stages.fixups);
        assert!(stages.enter_main);
        assert!(!stages.complete);
        assert!(!stages.exit_correct);
    }

    // ── extract_quoted ─────────────────────────────────────────

    #[test]
    fn extract_quoted_simple() {
        assert_eq!(extract_quoted("unresolved symbol '_foo'"), Some("_foo".into()));
    }

    #[test]
    fn extract_quoted_no_quotes() {
        assert_eq!(extract_quoted("no quotes here"), None);
    }

    #[test]
    fn extract_quoted_single_quote_only() {
        assert_eq!(extract_quoted("just one ' quote"), None);
    }

    #[test]
    fn extract_quoted_empty_quotes() {
        assert_eq!(extract_quoted("empty '' value"), Some("".into()));
    }

    // ── extract_after ──────────────────────────────────────────

    #[test]
    fn extract_after_found() {
        assert_eq!(
            extract_after("selector 'doSomething:' not found", "selector '"),
            Some("doSomething:".into())
        );
    }

    #[test]
    fn extract_after_prefix_missing() {
        assert_eq!(extract_after("no match here", "selector '"), None);
    }

    #[test]
    fn extract_after_no_closing_quote() {
        assert_eq!(extract_after("selector 'broken", "selector '"), None);
    }

    // ── discover_corpus ────────────────────────────────────────

    #[test]
    fn discover_corpus_nonexistent_dir() {
        let entries = discover_corpus(std::path::Path::new("/tmp/nonexistent_corpus_dir_xyz"));
        assert!(entries.is_empty());
    }

    // ── read_meta ──────────────────────────────────────────────

    #[test]
    fn read_meta_missing_file() {
        let (cat, exit) = read_meta(std::path::Path::new("/tmp/no_such_dir_xyz"));
        assert_eq!(cat, "unknown");
        assert_eq!(exit, 0);
    }

    #[test]
    fn read_meta_with_valid_json() {
        let dir = std::env::temp_dir().join("compat_runner_test_meta");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("meta.json"),
            r#"{"category": "foundation", "expected_exit": 42}"#,
        )
        .unwrap();
        let (cat, exit) = read_meta(&dir);
        assert_eq!(cat, "foundation");
        assert_eq!(exit, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_meta_with_invalid_json() {
        let dir = std::env::temp_dir().join("compat_runner_test_meta_bad");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("meta.json"), "not json!").unwrap();
        let (cat, exit) = read_meta(&dir);
        assert_eq!(cat, "unknown");
        assert_eq!(exit, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── generate_report ────────────────────────────────────────

    #[test]
    fn generate_report_empty_entries() {
        let report = crate::generate_report(&[]);
        assert_eq!(report.summary.total, 0);
        assert_eq!(report.summary.passed, 0);
        assert_eq!(report.summary.pass_rate, 0.0);
    }

    // ── run_binary missing path ────────────────────────────────

    #[test]
    fn run_binary_missing_path() {
        let entry = CorpusEntry {
            name: "ghost".into(),
            path: "/tmp/no_such_binary_xyz".into(),
            category: "test".into(),
            expected_exit: 0,
        };
        let result = run_binary(&entry);
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not found"));
        assert!(!result.stages.parse);
    }
}
