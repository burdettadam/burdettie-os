//! Compatibility scorecard — runs all fixtures and prints a table.
//!
//! Adapted from the integration test in macho-runtime/tests/compat_score.rs
//! for interactive use in the shell.

use std::path::Path;
use std::sync::Mutex;

use macho_runtime::{Runtime, RuntimeConfig};

struct FixtureScore {
    name: String,
    parse: bool,
    load: bool,
    fixups: bool,
    enter_main: bool,
    complete: bool,
    exit_correct: bool,
    expected_exit: i32,
    actual_exit: Option<i32>,
    error: Option<String>,
}

fn expected_exit(name: &str) -> i32 {
    match name {
        "return42" => 42,
        _ => 0,
    }
}

fn score_fixture(name: &str, binary: &Path, lock: &Mutex<()>) -> FixtureScore {
    let expected = expected_exit(name);
    let mut score = FixtureScore {
        name: name.to_string(),
        parse: false,
        load: false,
        fixups: false,
        enter_main: false,
        complete: false,
        exit_correct: false,
        expected_exit: expected,
        actual_exit: None,
        error: None,
    };

    let config = RuntimeConfig::permissive(binary);
    let _lock = lock.lock().unwrap_or_else(|e| e.into_inner());

    match Runtime::load_and_run(binary, &config) {
        Ok(result) => {
            score.parse = true;
            score.load = true;
            score.fixups = true;
            score.enter_main = true;
            score.complete = true;
            score.actual_exit = Some(result.exit_code);
            score.exit_correct = result.exit_code == expected;
        }
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("parse") || msg.contains("magic") || msg.contains("truncat") {
                // Failed at parse
            } else if msg.contains("dependency") || msg.contains("not found") || msg.contains("map")
            {
                score.parse = true;
            } else if msg.contains("fixup") || msg.contains("bind") || msg.contains("rebase") {
                score.parse = true;
                score.load = true;
            } else if msg.contains("entry") {
                score.parse = true;
                score.load = true;
                score.fixups = true;
            } else {
                score.parse = true;
                score.load = true;
                score.fixups = true;
                score.enter_main = true;
            }
            score.error = Some(msg);
        }
    }

    score
}

/// Discover and score all fixtures, printing a formatted table.
pub fn print_scorecard(fixtures_root: &Path, lock: &Mutex<()>) {
    let Ok(entries) = std::fs::read_dir(fixtures_root) else {
        println!(
            "Cannot read fixtures directory: {}",
            fixtures_root.display()
        );
        return;
    };

    let mut fixtures: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in entries.flatten() {
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
            fixtures.push((name, bin));
            continue;
        }
        let bin = dir.join(&name);
        if bin.is_file() {
            fixtures.push((name, bin));
        }
    }
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));

    if fixtures.is_empty() {
        println!("No fixtures found in {}", fixtures_root.display());
        return;
    }

    println!("Running {} fixtures...", fixtures.len());
    println!();

    let mut scores: Vec<FixtureScore> = Vec::new();
    for (i, (name, binary)) in fixtures.iter().enumerate() {
        print!("\r  [{}/{}] {:<40}", i + 1, fixtures.len(), name);
        let _ = std::io::Write::flush(&mut std::io::stdout());
        scores.push(score_fixture(name, binary, lock));
    }
    println!("\r  Done.{:50}", "");
    println!();

    // Print table
    println!("╔══════════════════════════════════════╦═══════╦══════╦════════╦═══════╦══════════╦══════╦═══════════════════════╗");
    println!("║ Fixture                              ║ Parse ║ Load ║ Fixups ║ Enter ║ Complete ║ Exit ║ Notes                 ║");
    println!("╠══════════════════════════════════════╬═══════╬══════╬════════╬═══════╬══════════╬══════╬═══════════════════════╣");

    let mut total_parse = 0usize;
    let mut total_load = 0usize;
    let mut total_fixups = 0usize;
    let mut total_enter = 0usize;
    let mut total_complete = 0usize;
    let mut total_exit = 0usize;
    let total = scores.len();

    for s in &scores {
        if s.parse {
            total_parse += 1;
        }
        if s.load {
            total_load += 1;
        }
        if s.fixups {
            total_fixups += 1;
        }
        if s.enter_main {
            total_enter += 1;
        }
        if s.complete {
            total_complete += 1;
        }
        if s.exit_correct {
            total_exit += 1;
        }

        let p = if s.parse { "  ✓  " } else { "  ✗  " };
        let l = if s.load { " ✓  " } else { " ✗  " };
        let f = if s.fixups { "  ✓   " } else { "  ✗   " };
        let e = if s.enter_main { "  ✓  " } else { "  ✗  " };
        let c = if s.complete {
            "   ✓    "
        } else {
            "   ✗    "
        };
        let x = if s.exit_correct { " ✓  " } else { " ✗  " };

        let notes = if let Some(err) = &s.error {
            let short: String = err.chars().take(21).collect();
            short
        } else if let Some(code) = s.actual_exit {
            if code != s.expected_exit {
                format!("got {} want {}", code, s.expected_exit)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        println!("║ {:<36} ║{p}║{l}║{f}║{e}║{c}║{x}║ {:<21} ║", s.name, notes);
    }

    println!("╠══════════════════════════════════════╬═══════╬══════╬════════╬═══════╬══════════╬══════╬═══════════════════════╣");
    println!(
        "║ TOTAL ({:>3} fixtures)                 ║ {:>3}/{} ║{:>3}/{} ║ {:>3}/{}  ║ {:>3}/{} ║  {:>3}/{}   ║{:>3}/{} ║                       ║",
        total,
        total_parse, total, total_load, total, total_fixups, total,
        total_enter, total, total_complete, total, total_exit, total,
    );
    println!("╚══════════════════════════════════════╩═══════╩══════╩════════╩═══════╩══════════╩══════╩═══════════════════════╝");

    // Summary
    println!();
    if total_exit == total {
        println!("  ✓ Perfect score: {total}/{total} fixtures pass all stages");
    } else {
        println!(
            "  Score: {total_exit}/{total} fixtures fully correct ({:.0}%)",
            (total_exit as f64 / total as f64) * 100.0
        );
    }
}
