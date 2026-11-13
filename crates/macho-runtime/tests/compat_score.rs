//! Compatibility scoring — runs every fixture through the runtime pipeline
//! and produces a per-fixture scorecard across 7 dimensions.
//!
//! Most fixtures run **in-process** for speed.  A small number of fixtures
//! still run in a **subprocess** so a fatal regression cannot take down the
//! entire scorecard binary.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::IsolatedResult;
use macho_fixtures::fixtures_dir;
use macho_runtime::{Runtime, RuntimeConfig};

/// Subprocess entry point — runs one fixture when invoked as a child.
#[test]
fn __subprocess_runner() {
    common::subprocess_entry_point();
}

/// Serializes in-process load_and_run calls (global ObjC state is process-wide).
static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

/// Per-fixture compatibility score.
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

/// Discover all fixture binaries.  Most use `app`; a few use the directory name.
fn discover_fixtures() -> Vec<(String, PathBuf)> {
    let base = fixtures_dir();
    let mut fixtures = Vec::new();

    let Ok(entries) = std::fs::read_dir(&base) else {
        return fixtures;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();

        // Skip malformed — intentionally unparseable (not a valid Mach-O)
        if name == "malformed" {
            continue;
        }

        // Try `app` first, then the directory name
        let bin = dir.join("app");
        if bin.is_file() {
            fixtures.push((name, bin));
            continue;
        }
        let bin = dir.join(&name);
        if bin.is_file() {
            fixtures.push((name, bin));
            continue;
        }

        // Skip directories without a known binary (e.g. objc_category has only .o)
    }

    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

/// Expected exit code per fixture.
fn expected_exit(name: &str) -> i32 {
    match name {
        "return42" => 42,
        _ => 0,
    }
}

/// Fixtures that should keep subprocess isolation because exception-regression
/// failures can still be process-fatal.
fn requires_subprocess(name: &str) -> bool {
    matches!(name, "minimal_throw" | "try_catch_test")
}

/// Infer which pipeline stages succeeded from a runtime error message.
fn infer_stages(msg: &str) -> (bool, bool, bool, bool) {
    if msg.contains("parse") || msg.contains("magic") || msg.contains("truncat") {
        (false, false, false, false)
    } else if msg.contains("dependency") || msg.contains("not found") || msg.contains("map") {
        (true, false, false, false)
    } else if msg.contains("fixup") || msg.contains("bind") || msg.contains("rebase") {
        (true, true, false, false)
    } else if msg.contains("entry") {
        (true, true, true, false)
    } else {
        (true, true, true, true)
    }
}

/// Run a fixture in a subprocess and score it.
fn score_fixture_subprocess(name: &str, binary: &Path) -> FixtureScore {
    let expected = expected_exit(name);

    match common::run_in_subprocess(binary) {
        IsolatedResult::Completed { exit_code } => FixtureScore {
            name: name.to_string(),
            parse: true,
            load: true,
            fixups: true,
            enter_main: true,
            complete: true,
            exit_correct: exit_code == expected,
            expected_exit: expected,
            actual_exit: Some(exit_code),
            error: if exit_code != expected {
                Some(format!("exit {exit_code} (expected {expected})"))
            } else {
                None
            },
        },
        IsolatedResult::RuntimeError(msg) => {
            let (parse, load, fixups, enter_main) = infer_stages(&msg);
            FixtureScore {
                name: name.to_string(),
                parse,
                load,
                fixups,
                enter_main,
                complete: false,
                exit_correct: false,
                expected_exit: expected,
                actual_exit: None,
                error: Some(msg),
            }
        }
        IsolatedResult::Crashed(msg) => FixtureScore {
            name: name.to_string(),
            parse: true,
            load: true,
            fixups: true,
            enter_main: true,
            complete: false,
            exit_correct: false,
            expected_exit: expected,
            actual_exit: None,
            error: Some(msg),
        },
    }
}

/// Run a fixture in-process and score it (fast path).
fn score_fixture_inprocess(name: &str, binary: &Path) -> FixtureScore {
    eprintln!("[compat fixture] {name}");
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
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

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
            let (parse, load, fixups, enter_main) = infer_stages(&msg);
            score.parse = parse;
            score.load = load;
            score.fixups = fixups;
            score.enter_main = enter_main;
            score.error = Some(msg);
        }
    }

    score
}

/// Score a fixture — uses subprocess isolation for known-crash fixtures,
/// in-process execution for everything else.
fn score_fixture(name: &str, binary: &Path) -> FixtureScore {
    if requires_subprocess(name) {
        score_fixture_subprocess(name, binary)
    } else {
        score_fixture_inprocess(name, binary)
    }
}

/// Combined compatibility scorecard and regression gate.
///
/// Runs every fixture once (in-process for speed, subprocess for
/// subprocess-isolated exception fixtures), prints the scorecard, then asserts
/// every fixture passes.
#[test]
fn compat_score_regression_gate() {
    let fixtures = discover_fixtures();
    assert!(
        fixtures.len() >= 63,
        "Expected at least 63 testable fixtures, found {}. \
         New fixtures may be added but none should be removed.",
        fixtures.len()
    );

    let mut scores: Vec<FixtureScore> = Vec::new();
    for (name, binary) in &fixtures {
        eprintln!("[compat] {name}");
        scores.push(score_fixture(name, binary));
    }

    // Print scorecard
    println!();
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
        let x = if s.exit_correct {
            " ✓  ".to_string()
        } else if let Some(code) = s.actual_exit {
            format!(" {:<3}", code)
        } else {
            " ✗  ".to_string()
        };

        let notes = if let Some(err) = &s.error {
            let truncated: String = err.chars().take(21).collect();
            truncated
        } else if s.exit_correct {
            String::new()
        } else {
            format!(
                "exit {} (expected {})",
                s.actual_exit.unwrap_or(-1),
                s.expected_exit
            )
        };

        println!(
            "║ {:<36} ║{}║{}║{}║{}║{}║{}║ {:<21} ║",
            s.name, p, l, f, e, c, x, notes
        );
    }

    println!("╠══════════════════════════════════════╬═══════╬══════╬════════╬═══════╬══════════╬══════╬═══════════════════════╣");
    println!(
        "║ TOTAL ({:>2}/{:>2})                        ║ {:>2}/{:<2} ║ {:>2}/{:<1} ║  {:>2}/{:<2} ║ {:>2}/{:<1} ║  {:>2}/{:<2}  ║ {:>2}/{:<1} ║                       ║",
        total_exit, total, total_parse, total, total_load, total, total_fixups, total, total_enter, total, total_complete, total, total_exit, total
    );
    println!("╚══════════════════════════════════════╩═══════╩══════╩════════╩═══════╩══════════╩══════╩═══════════════════════╝");
    println!();

    // Summary
    let pct = if total > 0 {
        (total_exit * 100) / total
    } else {
        0
    };
    println!(
        "Compatibility: {}/{} fixtures fully correct ({}%)",
        total_exit, total, pct
    );
    println!(
        "  Parse: {}/{}  Load: {}/{}  Fixups: {}/{}  Enter: {}/{}  Complete: {}/{}  Exit-correct: {}/{}",
        total_parse, total, total_load, total, total_fixups, total, total_enter, total, total_complete, total, total_exit, total
    );
    println!();

    // Regression gate: assert every fixture passes.
    let failures: Vec<String> = scores
        .iter()
        .filter(|s| !s.exit_correct)
        .map(|s| {
            if let Some(err) = &s.error {
                format!("{}: error — {err}", s.name)
            } else {
                format!(
                    "{}: exit {} (expected {})",
                    s.name,
                    s.actual_exit.unwrap_or(-1),
                    s.expected_exit,
                )
            }
        })
        .collect();

    assert!(
        failures.is_empty(),
        "Compatibility regression — {} fixture(s) failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
