//! Compatibility scoring — runs every fixture through the runtime pipeline
//! and produces a per-fixture scorecard across 7 dimensions.
//!
//! This test is a reporting tool: it always passes (unless the test infra
//! itself panics). The scorecard is printed to stdout via `--nocapture`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use macho_fixtures::fixtures_dir;
use macho_runtime::{Runtime, RuntimeConfig};

/// Serializes load_and_run calls (global ObjC state is process-wide).
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

        // Skip malformed — intentionally unparseable
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

/// Expected exit code per fixture.  Almost everything expects 0; return42 expects 42.
fn expected_exit(name: &str) -> i32 {
    match name {
        "return42" => 42,
        _ => 0,
    }
}

/// Run a fixture and score it.
fn score_fixture(name: &str, binary: &Path) -> FixtureScore {
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

    // The runtime pipeline handles parse/load/fixup/enter atomically.
    // We infer stage success from the RuntimeResult or error variant.
    let config = RuntimeConfig::permissive(binary);

    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    match Runtime::load_and_run(binary, &config) {
        Ok(result) => {
            // If we got a result, all stages through entry succeeded
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
            // Infer how far we got from the error variant
            if msg.contains("parse") || msg.contains("magic") || msg.contains("truncat") {
                // Failed at parse
            } else if msg.contains("dependency") || msg.contains("not found") || msg.contains("map")
            {
                score.parse = true;
                // Failed at load/dependency resolution
            } else if msg.contains("fixup") || msg.contains("bind") || msg.contains("rebase") {
                score.parse = true;
                score.load = true;
                // Failed at fixups
            } else if msg.contains("entry") {
                score.parse = true;
                score.load = true;
                score.fixups = true;
                // Failed at entry
            } else {
                // Unknown error — assume parse succeeded at minimum
                // (most errors occur after parsing)
                score.parse = true;
                score.load = true;
                score.fixups = true;
                score.enter_main = true;
                // Process crashed/aborted
            }
            score.error = Some(msg);
        }
    }

    score
}

#[test]
fn compatibility_scorecard() {
    let fixtures = discover_fixtures();
    if fixtures.is_empty() {
        eprintln!("No fixtures found — skipping scorecard");
        return;
    }

    let mut scores: Vec<FixtureScore> = Vec::new();
    for (name, binary) in &fixtures {
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
}

/// Regression gate — ensures the compatibility score never drops below
/// the v0.1.0 baseline.  Fails the build if any fixture regresses.
///
/// Baseline: 63 testable fixtures (64 dirs minus `malformed`), all must
/// parse, load, fixup, enter, complete, and exit with the correct code.
#[test]
fn compat_score_regression_gate() {
    let fixtures = discover_fixtures();
    assert!(
        fixtures.len() >= 63,
        "Expected at least 63 testable fixtures, found {}. \
         New fixtures may be added but none should be removed.",
        fixtures.len()
    );

    let mut failures: Vec<String> = Vec::new();
    for (name, binary) in &fixtures {
        let s = score_fixture(name, binary);
        if !s.exit_correct {
            let detail = if let Some(err) = &s.error {
                format!("{name}: error — {err}")
            } else {
                format!(
                    "{name}: exit {} (expected {})",
                    s.actual_exit.unwrap_or(-1),
                    s.expected_exit,
                )
            };
            failures.push(detail);
        }
    }

    assert!(
        failures.is_empty(),
        "Compatibility regression — {} fixture(s) failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
