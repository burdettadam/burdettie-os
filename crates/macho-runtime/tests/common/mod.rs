//! Shared helpers for macho-runtime integration tests.

use std::path::Path;
use std::process::{Command, Stdio};
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
#[allow(
    dead_code,
    reason = "Each integration-test binary compiles this shared module and uses a different subset of its helpers"
)]
pub fn run_fixture(fixture_rel: &str) -> Option<RuntimeResult> {
    eprintln!("[runtime fixture] {fixture_rel}");
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

// ── Subprocess isolation ────────────────────────────────────────────
//
// Fixtures run in-process via `Runtime::load_and_run`.  If a fixture hits
// a fatal signal (SIGBUS, SIGABRT) the entire test process dies, killing
// every remaining test in the binary.
//
// The subprocess helpers below run each fixture in its **own child
// process** by re-invoking the current test binary.  A crash in the child
// is reported as `IsolatedResult::Crashed` instead of taking down the
// parent.
//
// Usage in a test file:
//
// ```rust
// mod common;
//
// #[test]
// fn __subprocess_runner() { common::subprocess_entry_point(); }
//
// #[test]
// fn my_test() {
//     let result = common::run_in_subprocess(Path::new("fixtures/foo/app"));
//     assert!(matches!(result, common::IsolatedResult::Completed { exit_code: 0 }));
// }
// ```

/// Result of running a fixture in an isolated subprocess.
#[derive(Debug)]
#[allow(dead_code)]
pub enum IsolatedResult {
    /// Fixture completed normally.
    Completed { exit_code: i32 },
    /// `Runtime::load_and_run` returned an error before the fixture ran.
    RuntimeError(String),
    /// Subprocess crashed (signal) or had an unexpected failure.
    Crashed(String),
}

/// Full output from an isolated subprocess fixture run.
#[derive(Debug)]
#[allow(dead_code)]
pub struct IsolatedRun {
    pub result: IsolatedResult,
    pub stdout: String,
    pub stderr: String,
}

#[allow(dead_code)]
impl IsolatedResult {
    pub fn exit_code(&self) -> Option<i32> {
        if let Self::Completed { exit_code } = self {
            Some(*exit_code)
        } else {
            None
        }
    }

    pub fn is_crash(&self) -> bool {
        matches!(self, Self::Crashed(_))
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::RuntimeError(msg) | Self::Crashed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Entry point for subprocess fixture execution.
///
/// When `__RUNTIME_FIXTURE_PATH` is set, loads and runs that fixture,
/// prints a structured result tag, and returns.  When not set (normal
/// test execution), returns immediately.
///
/// Call this from a `#[test] fn __subprocess_runner()` in any test file
/// that needs crash isolation.
#[allow(
    dead_code,
    reason = "Each integration-test binary compiles this shared module and uses a different subset of its helpers"
)]
pub fn subprocess_entry_point() {
    let Ok(path_str) = std::env::var("__RUNTIME_FIXTURE_PATH") else {
        return;
    };
    let path = Path::new(&path_str);
    let config = RuntimeConfig::permissive(path);
    match Runtime::load_and_run(path, &config) {
        Ok(result) => println!("__RUNTIME_OK:{}", result.exit_code),
        Err(e) => println!("__RUNTIME_ERR:{e}"),
    }
}

/// Run a fixture binary in an isolated subprocess.
///
/// Spawns the current test binary as a child with the fixture path in an
/// env var.  The child's `__subprocess_runner` test picks it up, runs the
/// fixture, and prints a result tag.  If the fixture crashes, the child
/// dies but the parent survives.
///
/// **Requires** a `#[test] fn __subprocess_runner()` that calls
/// [`subprocess_entry_point()`] in the *calling* test file.
#[allow(dead_code)]
pub fn run_in_subprocess(binary: &Path) -> IsolatedResult {
    run_in_subprocess_with_output(binary).result
}

/// Run a fixture binary in an isolated subprocess and capture stdout/stderr.
#[allow(dead_code)]
pub fn run_in_subprocess_with_output(binary: &Path) -> IsolatedRun {
    let exe = std::env::current_exe().expect("cannot find test binary");

    let output = Command::new(&exe)
        .arg("__subprocess_runner")
        .arg("--exact")
        .arg("--nocapture")
        .env("__RUNTIME_FIXTURE_PATH", binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn fixture subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let result = if let Some(line) = stdout.lines().find(|l| l.starts_with("__RUNTIME_OK:")) {
        let code: i32 = line["__RUNTIME_OK:".len()..].trim().parse().unwrap_or(-1);
        IsolatedResult::Completed { exit_code: code }
    } else if let Some(line) = stdout.lines().find(|l| l.starts_with("__RUNTIME_ERR:")) {
        let msg = line["__RUNTIME_ERR:".len()..].to_string();
        IsolatedResult::RuntimeError(msg)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = output.status.signal() {
                return IsolatedRun {
                    result: IsolatedResult::Crashed(format!("signal {sig}")),
                    stdout,
                    stderr,
                };
            }
        }

        IsolatedResult::Crashed(stderr.lines().last().unwrap_or("unknown").to_string())
    };

    IsolatedRun {
        result,
        stdout,
        stderr,
    }
}
