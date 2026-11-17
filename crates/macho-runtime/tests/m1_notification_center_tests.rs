//! Native Foundation/reference and Burdettie-loader coverage, not Darwin guest qualification.
#![cfg(target_os = "macos")]

use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use macho_fixtures::fixtures_dir;
use macho_runtime::{Runtime, RuntimeConfig};

const CHILD_BINARY: &str = "__BURDETTIE_M1_NOTIFICATION_BINARY";
const SCOPE: &str = "m1_notification_center_execution";
const LOG_LIMIT: u64 = 1024 * 1024;

struct RunningProcess {
    child: Child,
    reaped: bool,
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if !self.reaped {
            // SAFETY: process_group(0) created this process group. Its leader is
            // still our unreaped child, so its PID cannot have been recycled.
            unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
            let _ = self.child.wait();
        }
    }
}

struct Captured {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

fn run_bounded(command: &mut Command, work: &Path, label: &str, seconds: u64) -> Captured {
    let stdout_path = work.join(format!("{label}.stdout"));
    let stderr_path = work.join(format!("{label}.stderr"));
    let open = |path: &Path| {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .expect("cannot exclusively create a process log")
    };
    let stdout = open(&stdout_path);
    let stderr = open(&stderr_path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .process_group(0);
    let mut running = RunningProcess {
        child: command.spawn().expect("cannot start source-owned process"),
        reaped: false,
    };
    let start = Instant::now();
    let status = loop {
        if let Some(status) = running.child.try_wait().expect("cannot poll child") {
            running.reaped = true;
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(seconds),
            "{label} exceeded deadline; evidence retained at {}",
            work.display()
        );
        for path in [&stdout_path, &stderr_path] {
            assert!(
                fs::metadata(path).expect("log disappeared").len() <= LOG_LIMIT,
                "{label} exceeded log bound; evidence retained at {}",
                work.display()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    for path in [&stdout_path, &stderr_path] {
        assert!(fs::metadata(path).expect("missing log").len() <= LOG_LIMIT);
    }
    Captured {
        status,
        stdout: fs::read_to_string(stdout_path).expect("stdout is not UTF-8"),
        stderr: fs::read_to_string(stderr_path).expect("stderr is not UTF-8"),
    }
}

fn report(output: &Captured) -> serde_json::Value {
    let records: Vec<serde_json::Value> = output
        .stdout
        .lines()
        .filter_map(|line| line.find('{').map(|start| &line[start..]))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["scope"] == SCOPE)
        .collect();
    assert_eq!(
        records.len(),
        1,
        "missing/repeated notification report: {}\n{}\n{}",
        output.status,
        output.stdout,
        output.stderr
    );
    let value = records.into_iter().next().expect("one report required");
    assert_eq!(value.as_object().expect("report is not an object").len(), 14);
    value
}

fn assert_success(output: &Captured) -> serde_json::Value {
    assert!(
        output.status.success(),
        "notification process failed: {}\n{}\n{}",
        output.status,
        output.stdout,
        output.stderr
    );
    let value = report(output);
    assert_eq!(value["status"], "pass");
    assert_eq!(value["platform"], "darwin");
    let architecture = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => panic!("unsupported reference architecture: {other}"),
    };
    assert_eq!(value["architecture"], architecture);
    for (field, expected) in [
        ("schema_version", 1),
        ("checks", 31),
        ("failures", 0),
        ("shared_calls", 5),
        ("default_calls", 1),
        ("filtered_calls", 3),
        ("reentrant_calls", 1),
        ("foreign_calls", 2),
        ("block_calls", 2),
        ("field_checks", 9),
    ] {
        assert_eq!(value[field].as_u64(), Some(expected), "counter: {field}");
    }
    value
}

#[test]
fn __notification_center_child() {
    let Some(binary) = std::env::var_os(CHILD_BINARY) else {
        return;
    };
    let binary = Path::new(&binary);
    assert!(binary.is_absolute() && binary.is_file());
    let result = Runtime::load_and_run(binary, &RuntimeConfig::permissive(binary))
        .expect("notification runtime execution failed");
    assert_eq!(result.exit_code, 0, "notification fixture rejected execution");
}

#[test]
fn native_notification_centers_match_foundation_and_reject_collapsed_identity() {
    let source = fixtures_dir().join("m1_notification_center_probe/probe.m");
    assert!(source.is_file(), "required notification source is missing");
    let work = std::env::temp_dir().join(format!(
        "burdettie-m1-notification-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock predates epoch")
            .as_nanos()
    ));
    fs::create_dir(&work).expect("cannot create exclusive fixture directory");
    let binary = work.join("probe");
    let compiled = run_bounded(
        Command::new("xcrun")
            .args([
                "--sdk", "macosx", "clang", "-O0", "-Wall", "-Wextra", "-Werror",
                "-fno-objc-arc", "-fblocks", "-framework", "Foundation",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&binary),
        &work,
        "compile",
        15,
    );
    assert!(
        compiled.status.success(),
        "native notification compilation failed: {}",
        compiled.stderr
    );
    let reference = run_bounded(&mut Command::new(&binary), &work, "reference", 8);
    let expected = assert_success(&reference);
    println!("Native Foundation notification reference: {expected}");

    let negative = run_bounded(
        Command::new(&binary).arg("--collapse-centers"),
        &work,
        "collapsed-centers",
        8,
    );
    assert_eq!(negative.status.code(), Some(1));
    let rejected = report(&negative);
    assert_eq!(rejected["status"], "fail");
    assert_eq!(rejected["checks"].as_u64(), Some(31));
    assert!(rejected["failures"].as_u64().expect("missing failures") > 0);
    println!("Native center-collapse negative control: {rejected}");

    let runtime = run_bounded(
        Command::new(std::env::current_exe().expect("cannot locate test executable"))
            .args(["__notification_center_child", "--exact", "--nocapture"])
            .env(CHILD_BINARY, &binary),
        &work,
        "runtime",
        8,
    );
    let observed = assert_success(&runtime);
    assert_eq!(observed, expected, "runtime and Foundation reports differ");
    println!("Burdettie native notification execution: {observed}");
}
