//! Integration tests for compat-runner.
//!
//! Tests corpus scanning, binary execution, and report generation.

use std::path::PathBuf;

use compat_runner::{discover_corpus, generate_report, run_binary, CorpusEntry};
use macho_fixtures::fixtures_dir;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("corpus"))
        .unwrap_or_else(|| PathBuf::from("corpus"))
}

// ── Discovery tests ────────────────────────────────────────────────

#[test]
fn discover_corpus_finds_entries() {
    let entries = discover_corpus(&corpus_dir());
    assert!(
        entries.len() >= 12,
        "Expected at least 12 corpus entries, found {}",
        entries.len()
    );
}

#[test]
fn discover_corpus_reads_metadata() {
    let entries = discover_corpus(&corpus_dir());
    let config = entries.iter().find(|e| e.name == "config_reader");
    assert!(config.is_some(), "config_reader not found in corpus");
    let config = config.unwrap();
    assert_eq!(config.category, "foundation");
    assert_eq!(config.expected_exit, 0);
}

#[test]
fn discover_empty_dir_returns_empty() {
    let entries = discover_corpus(&PathBuf::from("/nonexistent/path"));
    assert!(entries.is_empty());
}

// ── Single binary tests ────────────────────────────────────────────

#[test]
fn run_single_fixture() {
    let fixture = fixtures_dir().join("return42/return42");
    if !fixture.exists() {
        eprintln!("Skipping: return42 fixture not found");
        return;
    }

    let entry = CorpusEntry {
        name: "return42".to_string(),
        path: fixture,
        category: "fixture".to_string(),
        expected_exit: 42,
    };

    let result = run_binary(&entry);
    assert!(result.success, "return42 should succeed");
    assert_eq!(result.exit_code, Some(42));
    assert!(result.stages.parse);
    assert!(result.stages.exit_correct);
}

#[test]
fn run_missing_binary_reports_error() {
    let entry = CorpusEntry {
        name: "nonexistent".to_string(),
        path: PathBuf::from("/nonexistent/binary"),
        category: "test".to_string(),
        expected_exit: 0,
    };

    let result = run_binary(&entry);
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("not found"));
}

// ── Report generation tests ────────────────────────────────────────

#[test]
fn generate_report_from_fixtures() {
    // Run a small subset of fixtures through the report generator
    let fixtures = fixtures_dir();
    let entries = vec![
        CorpusEntry {
            name: "return42".to_string(),
            path: fixtures.join("return42/return42"),
            category: "fixture".to_string(),
            expected_exit: 42,
        },
        CorpusEntry {
            name: "simple_cli".to_string(),
            path: fixtures.join("simple_cli/simple_cli"),
            category: "fixture".to_string(),
            expected_exit: 0,
        },
    ];

    // Skip if fixtures don't exist
    if !entries.iter().all(|e| e.path.exists()) {
        eprintln!("Skipping: some fixtures not found");
        return;
    }

    let report = generate_report(&entries);
    assert_eq!(report.summary.total, 2);
    assert_eq!(report.summary.passed, 2);
    assert_eq!(report.summary.failed, 0);
    assert!(report.summary.pass_rate > 99.0);
    assert_eq!(report.schema_version, 1);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&report).expect("JSON serialization");
    assert!(json.contains("return42"));
    assert!(json.contains("simple_cli"));
}

// ── Full corpus test ────────────────────────────────────────────────

#[test]
fn corpus_regression_gate() {
    let entries = discover_corpus(&corpus_dir());
    if entries.is_empty() {
        eprintln!("No corpus entries found — skipping");
        return;
    }

    let report = generate_report(&entries);

    // Print summary
    println!();
    println!("Corpus Compatibility Report");
    println!("===========================");
    for b in &report.binaries {
        let status = if b.success { "PASS" } else { "FAIL" };
        println!(
            "  {} {} (exit: {:?}, expected: {})",
            status, b.name, b.exit_code, b.expected_exit
        );
        if let Some(err) = &b.error {
            println!("    error: {}", err);
        }
    }
    println!();
    println!(
        "Result: {}/{} passed ({:.0}%)",
        report.summary.passed, report.summary.total, report.summary.pass_rate
    );

    // At least 10 must pass (Phase 39 success criteria)
    assert!(
        report.summary.passed >= 10,
        "Phase 39 requires at least 10 corpus binaries passing, got {}",
        report.summary.passed
    );
}
