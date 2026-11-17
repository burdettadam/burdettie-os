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

impl RunningProcess {
    fn spawn(command: &mut Command) -> Self {
        // Move the named child immediately into its sole cleanup owner. There is
        // no fallible operation between spawning and installing this guard.
        let child = command
            .process_group(0)
            .spawn()
            .expect("cannot start source-owned process");
        Self {
            child,
            reaped: false,
        }
    }
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if !self.reaped {
            // SAFETY: process_group(0) created this process group. Its leader is
            // still our unreaped child, so its PID cannot have been recycled.
            unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
            // Also terminate the owned leader if it changed process groups.
            // Errors for an already exited child do not prevent reaping it.
            let _ = self.child.kill();
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
        .stderr(Stdio::from(stderr));
    let running = RunningProcess::spawn(command);
    let status = wait_bounded(running, work, label, seconds);
    for path in [&stdout_path, &stderr_path] {
        assert!(fs::metadata(path).expect("missing log").len() <= LOG_LIMIT);
    }
    Captured {
        status,
        stdout: fs::read_to_string(stdout_path).expect("stdout is not UTF-8"),
        stderr: fs::read_to_string(stderr_path).expect("stderr is not UTF-8"),
    }
}

fn wait_bounded(mut running: RunningProcess, work: &Path, label: &str, seconds: u64) -> ExitStatus {
    let stdout_path = work.join(format!("{label}.stdout"));
    let stderr_path = work.join(format!("{label}.stderr"));
    let start = Instant::now();
    loop {
        if let Some(status) = running.child.try_wait().expect("cannot poll child") {
            running.reaped = true;
            return status;
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
    assert_eq!(
        value.as_object().expect("report is not an object").len(),
        14
    );
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
    assert_eq!(
        result.exit_code, 0,
        "notification fixture rejected execution"
    );
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
                "--sdk",
                "macosx",
                "clang",
                "-O0",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-fno-objc-arc",
                "-fblocks",
                "-framework",
                "Foundation",
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

#[cfg(test)]
mod process_tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn work() -> PathBuf {
        let number = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "burdettie-notification-process-{}-{number}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("cannot create exclusive regression directory");
        path
    }

    fn sleeper() -> RunningProcess {
        RunningProcess::spawn(
            Command::new("/bin/sleep")
                .arg("30")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        )
    }

    fn assert_reaped(pid: u32) {
        let mut status = 0;
        // SAFETY: this PID belongs to the source-owned child just guarded above.
        // WNOHANG prevents a failing regression from blocking this test.
        let waited = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
        let error = std::io::Error::last_os_error().raw_os_error();
        if waited == 0 {
            // SAFETY: a live child is still ours. Clean it up before reporting
            // the missing-reap failure, rather than leaking a test process.
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
                libc::waitpid(pid as i32, &mut status, 0);
            }
        }
        assert_eq!(waited, -1, "child was live or left as a zombie");
        assert_eq!(error, Some(libc::ECHILD), "child was not reaped");
    }

    fn pid_file(path: &Path) -> u32 {
        fs::read_to_string(path)
            .expect("child did not write its PID")
            .trim()
            .parse()
            .expect("child PID was not an integer")
    }

    #[test]
    fn drop_terminates_and_reaps_running_child() {
        let running = sleeper();
        let pid = running.child.id();
        drop(running);
        assert_reaped(pid);
    }

    #[test]
    fn panic_unwind_terminates_and_reaps_running_child() {
        let running = sleeper();
        let pid = running.child.id();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _running = running;
            panic!("controlled failure after installing process guard");
        }));
        assert!(result.is_err());
        assert_reaped(pid);
    }

    #[test]
    fn deadline_failure_terminates_and_reaps_child() {
        let work = work();
        fs::write(work.join("deadline.stdout"), b"").unwrap();
        fs::write(work.join("deadline.stderr"), b"").unwrap();
        let running = sleeper();
        let pid = running.child.id();
        let result = catch_unwind(AssertUnwindSafe(|| {
            wait_bounded(running, &work, "deadline", 0);
        }));
        assert!(result.is_err(), "a live child passed a zero deadline");
        assert_reaped(pid);
    }

    #[test]
    fn log_metadata_failure_still_reaps_child() {
        let work = work();
        let running = sleeper();
        let pid = running.child.id();
        let result = catch_unwind(AssertUnwindSafe(|| {
            wait_bounded(running, &work, "missing-log", 8);
        }));
        assert!(result.is_err());
        assert_reaped(pid);
    }

    #[test]
    fn real_nonzero_exit_and_logs_are_preserved() {
        let work = work();
        let output = run_bounded(
            Command::new("/bin/sh").args([
                "-c",
                "printf '%s\\n' \"$$\"; printf 'owned failure\\n' >&2; exit 7",
            ]),
            &work,
            "nonzero",
            8,
        );
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stderr, "owned failure\n");
        assert_reaped(output.stdout.trim().parse().unwrap());
    }

    #[test]
    fn real_signal_exit_is_not_a_success() {
        let work = work();
        let output = run_bounded(
            Command::new("/bin/sh").args(["-c", "printf '%s\\n' \"$$\"; kill -TERM \"$$\""]),
            &work,
            "signal",
            8,
        );
        assert!(!output.status.success());
        assert_eq!(output.status.signal(), Some(libc::SIGTERM));
        assert_reaped(output.stdout.trim().parse().unwrap());
    }

    #[test]
    fn real_oversized_streams_fail_and_reap_the_writer() {
        for (label, redirect) in [("stdout-limit", ""), ("stderr-limit", "1>&2")] {
            let work = work();
            let pid_path = work.join("writer.pid");
            // This finite writer produces 2 MiB, not an unbounded flood.
            let script = format!(
                "printf '%s\\n' \"$$\" > \"$1\"; \
                 exec /bin/dd if=/dev/zero bs=1048576 count=2 {redirect}"
            );
            let result = catch_unwind(AssertUnwindSafe(|| {
                run_bounded(
                    Command::new("/bin/sh")
                        .args(["-c", &script, "owned-log-writer"])
                        .arg(&pid_path),
                    &work,
                    label,
                    8,
                );
            }));
            assert!(result.is_err(), "oversized {label} was accepted");
            let stream = if redirect.is_empty() {
                "stdout"
            } else {
                "stderr"
            };
            let length = fs::metadata(work.join(format!("{label}.{stream}")))
                .unwrap()
                .len();
            assert!(length > LOG_LIMIT, "writer did not reach the bound");
            assert_reaped(pid_file(&pid_path));
        }
    }

    #[test]
    fn existing_evidence_is_not_overwritten_or_executed_past() {
        let work = work();
        let output = work.join("existing.stdout");
        let marker = work.join("must-not-exist");
        fs::write(&output, b"preserve prior evidence\n").unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            run_bounded(
                Command::new("/bin/sh")
                    .args(["-c", "printf launched > \"$1\"", "owned-launch"])
                    .arg(&marker),
                &work,
                "existing",
                8,
            );
        }));
        assert!(result.is_err());
        assert_eq!(fs::read(output).unwrap(), b"preserve prior evidence\n");
        assert!(!marker.exists());
    }

    #[test]
    fn failed_spawn_retains_empty_logs_and_cannot_pass() {
        let work = work();
        let nonexistent = work.join("not-an-executable");
        let result = catch_unwind(AssertUnwindSafe(|| {
            run_bounded(&mut Command::new(nonexistent), &work, "spawn", 8);
        }));
        assert!(result.is_err());
        assert_eq!(fs::read(work.join("spawn.stdout")).unwrap(), b"");
        assert_eq!(fs::read(work.join("spawn.stderr")).unwrap(), b"");
    }
}
