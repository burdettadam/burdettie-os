//! Native macOS-host runtime coverage, not selected PureDarwin guest qualification.
#![cfg(target_os = "macos")]

use std::process::{Command, Stdio};

use macho_fixtures::fixtures_dir;
use macho_runtime::{Runtime, RuntimeConfig};

const CHILD_FLAG: &str = "__BURDETTIE_M1_THREAD_STATE_CHILD";

#[test]
fn __thread_state_child() {
    if std::env::var_os(CHILD_FLAG).is_none() {
        return;
    }
    // SAFETY: This branch runs only in our dedicated child process. A thread or
    // loader deadlock terminates that child, never the parent test harness. The
    // owned fixture does not change SIGALRM or spawn descendant processes.
    unsafe { libc::alarm(8) };
    let source = fixtures_dir().join("m1_thread_state_probe/probe.c");
    assert!(source.is_file(), "required thread-state source is missing");
    let work = std::env::temp_dir().join(format!(
        "burdettie-m1-thread-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock predates epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&work).expect("cannot create a new native fixture directory");
    let binary = work.join("probe");
    let compiled = Command::new("xcrun")
        .args([
            "--sdk", "macosx", "clang", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
            "-pthread",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("cannot compile the native thread-state fixture");
    assert!(
        compiled.status.success(),
        "native fixture compilation failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let result = Runtime::load_and_run(&binary, &RuntimeConfig::permissive(&binary))
        .expect("thread-state runtime execution failed");
    assert_eq!(
        result.exit_code, 0,
        "thread-state fixture rejected execution"
    );
}

#[test]
fn native_thread_state_has_exact_counts_and_successful_process_exit() {
    let source = fixtures_dir().join("m1_thread_state_probe/probe.c");
    assert!(source.is_file(), "required thread-state source is missing");
    let output = Command::new(std::env::current_exe().expect("cannot locate test executable"))
        .args(["__thread_state_child", "--exact", "--nocapture"])
        .env(CHILD_FLAG, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("cannot start isolated thread-state child");
    let stdout = String::from_utf8(output.stdout).expect("thread-state stdout is not UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "thread-state child failed: {}\n{stdout}\n{stderr}",
        output.status
    );
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|line| line.find('{').map(|start| &line[start..]))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["scope"] == "m1_pthread_state_execution")
        .collect();
    assert_eq!(records.len(), 1, "missing or repeated thread-state report");
    let report = records[0].as_object().expect("report must be an object");
    assert_eq!(report.len(), 17, "unexpected thread-state report fields");
    assert_eq!(report["status"], "pass");
    assert_eq!(report["platform"], "darwin");
    let architecture = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => panic!("unsupported reference host: {other}"),
    };
    assert_eq!(report["architecture"], architecture);
    for (field, expected) in [
        ("schema_version", 1),
        ("workers_requested", 6),
        ("workers_started", 6),
        ("workers_joined", 6),
        ("rounds", 192),
        ("tls_checks", 280),
        ("errno_checks", 256),
        ("errno_address_checks", 384),
        ("once_calls", 1),
        ("destructors", 6),
        ("join_checks", 6),
        ("pthread_errors", 0),
        ("failures", 0),
    ] {
        assert_eq!(report[field].as_u64(), Some(expected), "counter: {field}");
    }
    println!("Native runtime thread-state report: {}", records[0]);
}
