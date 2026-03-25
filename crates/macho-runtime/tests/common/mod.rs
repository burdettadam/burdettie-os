//! Shared helpers for macho-runtime integration tests.

use std::sync::Mutex;

#[allow(unused_imports)]
pub use macho_fixtures::fixtures_dir;
#[allow(unused_imports)]
pub use macho_runtime::{Runtime, RuntimeConfig, RuntimeResult, ShimSet};

/// Serializes tests that call `load_and_run` to avoid global state races.
pub static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

/// Run a fixture through the full runtime pipeline.
///
/// Acquires the `RUNTIME_LOCK`, resolves the fixture path, skips if not built,
/// and returns the `RuntimeResult`.  Panics on load failure.
///
/// `fixture_rel` is the relative path under `fixtures/`, e.g. `"return42/return42"`.
pub fn run_fixture(fixture_rel: &str) -> Option<RuntimeResult> {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join(fixture_rel);
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return None;
    }
    let config = RuntimeConfig::permissive(&fixture);
    Some(Runtime::load_and_run(&fixture, &config).expect("runtime execution failed"))
}

/// Run a fixture and capture its NSLog output.
///
/// Like [`run_fixture`] but drains the NSLog buffer before and after execution,
/// returning both the result and the captured log lines.
#[allow(dead_code)]
pub fn run_fixture_with_nslog(fixture_rel: &str) -> Option<(RuntimeResult, Vec<String>)> {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join(fixture_rel);
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return None;
    }
    cf_lite::nslog::drain_captured_nslog();
    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    let lines = cf_lite::nslog::drain_captured_nslog();
    Some((result, lines))
}