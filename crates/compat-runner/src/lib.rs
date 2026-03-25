//! Compatibility testing runner for Darwin Loader Lab.
//!
//! Runs Mach-O binaries under the runtime, captures missing symbols,
//! classes, and selectors, and produces structured JSON reports.
//!
//! # Usage
//!
//! As a library:
//! ```no_run
//! use compat_runner::{CorpusEntry, run_binary, generate_report, discover_corpus};
//! use std::path::PathBuf;
//!
//! let entries = discover_corpus(&PathBuf::from("corpus"));
//! let report = generate_report(&entries);
//! println!("{}", serde_json::to_string_pretty(&report).unwrap());
//! ```
//!
//! As a CLI:
//! ```text
//! darwin-compat scan corpus/
//! darwin-compat run path/to/binary
//! ```

pub mod report;
pub mod runner;

pub use report::{BinaryResult, CompatibilityReport, Summary, REPORT_SCHEMA_VERSION};
pub use runner::{discover_corpus, run_binary, CorpusEntry};

use std::collections::BTreeSet;

/// Run all corpus entries and produce a complete report.
pub fn generate_report(entries: &[CorpusEntry]) -> CompatibilityReport {
    let mut binaries = Vec::new();

    for entry in entries {
        let result = run_binary(entry);
        binaries.push(result);
    }

    let total = binaries.len();
    let passed = binaries.iter().filter(|b| b.success).count();
    let failed = total - passed;
    let pass_rate = if total > 0 {
        (passed as f64) / (total as f64) * 100.0
    } else {
        0.0
    };

    // Collect union of all gaps
    let mut all_missing_symbols = BTreeSet::new();
    let mut all_missing_classes = BTreeSet::new();
    let mut all_missing_selectors = BTreeSet::new();

    for b in &binaries {
        for s in &b.missing_symbols {
            all_missing_symbols.insert(s.clone());
        }
        for c in &b.missing_classes {
            all_missing_classes.insert(c.clone());
        }
        for s in &b.missing_selectors {
            all_missing_selectors.insert(s.clone());
        }
    }

    let summary = Summary {
        total,
        passed,
        failed,
        pass_rate,
        all_missing_symbols: all_missing_symbols.into_iter().collect(),
        all_missing_classes: all_missing_classes.into_iter().collect(),
        all_missing_selectors: all_missing_selectors.into_iter().collect(),
    };

    // Generate timestamp
    let generated_at = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{now}")
    };

    CompatibilityReport {
        schema_version: REPORT_SCHEMA_VERSION,
        generated_at,
        binaries,
        summary,
    }
}
