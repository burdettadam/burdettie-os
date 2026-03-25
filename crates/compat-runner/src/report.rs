//! Structured compatibility report types.
//!
//! Serialized to `compatibility-report.json` after scanning a corpus.

use serde::Serialize;

/// Top-level report for a corpus scan.
#[derive(Debug, Clone, Serialize)]
pub struct CompatibilityReport {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Timestamp (ISO 8601) when the report was generated.
    pub generated_at: String,
    /// Per-binary results.
    pub binaries: Vec<BinaryResult>,
    /// Aggregate summary.
    pub summary: Summary,
}

/// Result of running a single binary through the runtime.
#[derive(Debug, Clone, Serialize)]
pub struct BinaryResult {
    /// Display name (typically the binary filename or corpus entry name).
    pub name: String,
    /// Path to the binary on disk.
    pub path: String,
    /// Category tag (cli, objc, foundation, gui, system).
    pub category: String,
    /// Whether the binary ran to completion without error.
    pub success: bool,
    /// Exit code returned by the binary (if it ran).
    pub exit_code: Option<i32>,
    /// Expected exit code.
    pub expected_exit: i32,
    /// Per-stage pass/fail.
    pub stages: Stages,
    /// Missing symbols that could not be resolved.
    pub missing_symbols: Vec<String>,
    /// Missing ObjC classes referenced but not registered.
    pub missing_classes: Vec<String>,
    /// Missing ObjC selectors that dispatch failed on.
    pub missing_selectors: Vec<String>,
    /// Number of images loaded.
    pub images_loaded: usize,
    /// Number of shims used.
    pub shims_installed: usize,
    /// Error message if the binary failed.
    pub error: Option<String>,
}

/// Per-stage pass/fail flags.
#[derive(Debug, Clone, Serialize)]
pub struct Stages {
    pub parse: bool,
    pub load: bool,
    pub fixups: bool,
    pub enter_main: bool,
    pub complete: bool,
    pub exit_correct: bool,
}

/// Aggregate summary across all binaries.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    /// Union of all missing symbols across all binaries.
    pub all_missing_symbols: Vec<String>,
    /// Union of all missing classes across all binaries.
    pub all_missing_classes: Vec<String>,
    /// Union of all missing selectors across all binaries.
    pub all_missing_selectors: Vec<String>,
}

pub const REPORT_SCHEMA_VERSION: u32 = 1;
