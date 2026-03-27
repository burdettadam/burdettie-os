//! Real-app compatibility test — runs all corpus CLI tools through the runtime
//! and reports pass/fail with diagnostic events for gap analysis.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use macho_runtime::{Runtime, RuntimeConfig};

/// Serializes load_and_run calls (global ObjC state is process-wide).
static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

/// Returns the path to the corpus directory relative to workspace root.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("corpus"))
        .unwrap_or_else(|| PathBuf::from("corpus"))
}

/// Discover all corpus binaries (each has corpus/<name>/app).
fn discover_corpus() -> Vec<(String, PathBuf)> {
    let base = corpus_dir();
    let mut tools = Vec::new();

    let Ok(entries) = std::fs::read_dir(&base) else {
        return tools;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let bin = dir.join("app");
        if bin.is_file() {
            tools.push((name, bin));
        }
    }

    tools.sort_by(|a, b| a.0.cmp(&b.0));
    tools
}

struct ToolResult {
    name: String,
    passed: bool,
    exit_code: Option<i32>,
    error: Option<String>,
    diag_events: Vec<String>,
}

fn run_tool(name: &str, binary: &Path) -> ToolResult {
    let config = RuntimeConfig::permissive(binary);
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Drain any pre-existing diagnostic events
    runtime_diagnostics::drain();
    eprintln!("  [run_tool] starting: {name}");

    match Runtime::load_and_run(binary, &config) {
        Ok(result) => {
            let events = runtime_diagnostics::drain();
            let diag: Vec<String> = events.iter().map(|e| format!("{e}")).collect();
            ToolResult {
                name: name.to_string(),
                passed: result.exit_code == 0,
                exit_code: Some(result.exit_code),
                error: None,
                diag_events: diag,
            }
        }
        Err(e) => {
            let events = runtime_diagnostics::drain();
            let diag: Vec<String> = events.iter().map(|e| format!("{e}")).collect();
            ToolResult {
                name: name.to_string(),
                passed: false,
                exit_code: None,
                error: Some(format!("{e}")),
                diag_events: diag,
            }
        }
    }
}

#[test]
fn real_app_compat_report() {
    let tools = discover_corpus();
    if tools.is_empty() {
        eprintln!("No corpus tools found — skipping");
        return;
    }

    let mut results: Vec<ToolResult> = Vec::new();
    for (name, binary) in &tools {
        results.push(run_tool(name, binary));
    }

    // Print report
    println!();
    println!("╔══════════════════════════╦════════╦══════╦═══════════════════════════════════════╗");
    println!("║ Tool                     ║ Status ║ Exit ║ Notes                                 ║");
    println!("╠══════════════════════════╬════════╬══════╬═══════════════════════════════════════╣");

    let mut passed = 0usize;
    let total = results.len();

    for r in &results {
        let status = if r.passed { " PASS " } else { " FAIL " };
        let exit = match r.exit_code {
            Some(c) => format!("  {c:>2}  "),
            None => " err  ".to_string(),
        };
        let notes = if let Some(ref e) = r.error {
            e.chars().take(37).collect::<String>()
        } else if !r.passed {
            format!("exit {}", r.exit_code.unwrap_or(-1))
        } else {
            String::new()
        };
        println!(
            "║ {:<24} ║ {} ║ {} ║ {:<37} ║",
            &r.name[..r.name.len().min(24)],
            status,
            exit,
            notes,
        );
        if r.passed {
            passed += 1;
        }
    }

    println!("╚══════════════════════════╩════════╩══════╩═══════════════════════════════════════╝");
    println!();
    println!(
        "Real-App Compatibility: {passed}/{total} tools fully passing ({:.0}%)",
        (passed as f64 / total as f64) * 100.0,
    );
    println!();

    // Print diagnostic summary — aggregate missing selectors/classes/symbols
    let mut missing_selectors: BTreeMap<String, usize> = BTreeMap::new();
    let mut missing_classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut blocked_symbols: BTreeMap<String, usize> = BTreeMap::new();

    for r in &results {
        for d in &r.diag_events {
            if d.starts_with("selector missing: ") {
                *missing_selectors.entry(d.clone()).or_default() += 1;
            } else if d.starts_with("class missing: ") {
                *missing_classes.entry(d.clone()).or_default() += 1;
            } else if d.starts_with("symbol blocked") {
                *blocked_symbols.entry(d.clone()).or_default() += 1;
            }
        }
    }

    if !missing_selectors.is_empty() || !missing_classes.is_empty() || !blocked_symbols.is_empty() {
        println!("=== Gap Analysis ===");
        if !missing_classes.is_empty() {
            println!("\nMissing Classes:");
            for (cls, count) in &missing_classes {
                println!("  [{count}x] {cls}");
            }
        }
        if !missing_selectors.is_empty() {
            println!("\nMissing Selectors (top 20):");
            let mut sel_vec: Vec<_> = missing_selectors.iter().collect();
            sel_vec.sort_by(|a, b| b.1.cmp(a.1));
            for (sel, count) in sel_vec.iter().take(20) {
                println!("  [{count}x] {sel}");
            }
        }
        if !blocked_symbols.is_empty() {
            println!("\nBlocked Symbols:");
            for (sym, count) in &blocked_symbols {
                println!("  [{count}x] {sym}");
            }
        }
        println!();
    }

    // Regression gate: all tools must pass
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(
        failed.is_empty(),
        "Real-app compatibility regression — {} tool(s) failed:\n{}",
        failed.len(),
        failed
            .iter()
            .map(|r| format!(
                "  {}: {}",
                r.name,
                r.error.as_deref().unwrap_or("non-zero exit")
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
