//! Phase 11 tests: NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar,
//! NSNumberFormatter, NSIndexSet, NSSortDescriptor, NSTimer, try/catch.

mod common;

/// Subprocess entry point — runs one fixture when invoked as a child.
#[test]
fn __subprocess_runner() {
    common::subprocess_entry_point();
}

fn run_exception_fixture(name: &str) -> Option<common::IsolatedRun> {
    let binary = common::fixtures_dir().join(name).join("app");
    if !binary.exists() {
        eprintln!("Skipping: fixture not found at {}", binary.display());
        return None;
    }
    Some(common::run_in_subprocess_with_output(&binary))
}

fn assert_output_contains_in_order(output: &str, required: &[&str]) {
    let mut pos = 0usize;
    for needle in required {
        let remaining = &output[pos..];
        let found = remaining.find(needle).unwrap_or_else(|| {
            panic!(
                "expected output to contain '{needle}' after byte {pos}, full output:\n{output}"
            )
        });
        pos += found + needle.len();
    }
}

// ── Phase 11: NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar ────

#[test]
fn runtime_nsvalue_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSValue")
        .expect("NSValue should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "rangeValue").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "pointerValue").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "objCType").is_some());
}

#[test]
fn runtime_nsuuid_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt.get_class("NSUUID").expect("NSUUID should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "UUIDString").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "isEqual:").is_some());
}

#[test]
fn runtime_nslocale_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSLocale")
        .expect("NSLocale should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "localeIdentifier").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "languageCode").is_some());
}

#[test]
fn runtime_nstimezone_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSTimeZone")
        .expect("NSTimeZone should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "name").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "secondsFromGMT").is_some());
}

#[test]
fn runtime_nscalendar_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSCalendar")
        .expect("NSCalendar should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "calendarIdentifier")
        .is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "components:fromDate:")
        .is_some());
}

#[test]
fn runtime_nsvalue_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsvalue_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsvalue_basics should exit 0");
}

#[test]
fn runtime_nsuuid_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsuuid_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsuuid_basics should exit 0");
}

#[test]
fn runtime_nslocale_basics_exit_zero() {
    let Some(result) = common::run_fixture("nslocale_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nslocale_basics should exit 0");
}

#[test]
fn runtime_nstimezone_basics_exit_zero() {
    let Some(result) = common::run_fixture("nstimezone_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nstimezone_basics should exit 0");
}

#[test]
fn runtime_nscalendar_basics_exit_zero() {
    let Some(result) = common::run_fixture("nscalendar_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nscalendar_basics should exit 0");
}

// ── NSNumberFormatter ──────────────────────────────────────────────

#[test]
fn runtime_nsnumberformatter_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSNumberFormatter")
        .expect("NSNumberFormatter should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "stringFromNumber:")
        .is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "numberFromString:")
        .is_some());
}

#[test]
fn runtime_nsnumberformatter_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsnumberformatter_basics/app") else { return };
    assert_eq!(
        result.exit_code, 0,
        "nsnumberformatter_basics should exit 0"
    );
}

// ── NSIndexSet ─────────────────────────────────────────────────────

#[test]
fn runtime_nsindexset_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSIndexSet")
        .expect("NSIndexSet should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "count").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "containsIndex:").is_some());

    let mcls = rt
        .get_class("NSMutableIndexSet")
        .expect("NSMutableIndexSet should be registered");
    assert!(rt.msg_lookup(mcls.mapped_addr, "addIndex:").is_some());
}

#[test]
fn runtime_nsindexset_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsindexset_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsindexset_basics should exit 0");
}

// ── NSSortDescriptor ───────────────────────────────────────────────

#[test]
fn runtime_nssortdescriptor_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSSortDescriptor")
        .expect("NSSortDescriptor should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "key").is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "compareObject:toObject:")
        .is_some());
}

#[test]
fn runtime_nssortdescriptor_basics_exit_zero() {
    let Some(result) = common::run_fixture("nssortdescriptor_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nssortdescriptor_basics should exit 0");
}

// ── NSTimer ────────────────────────────────────────────────────────

#[test]
fn runtime_nstimer_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSTimer")
        .expect("NSTimer should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "fire").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "isValid").is_some());
}

#[test]
fn runtime_nstimer_basics_exit_zero() {
    let Some(result) = common::run_fixture("nstimer_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nstimer_basics should exit 0");
}

// ── Exception fixtures via subprocess isolation ───────────────────

#[test]
fn runtime_minimal_throw_catches_exception_in_subprocess() {
    let Some(run) = run_exception_fixture("minimal_throw") else { return };

    match &run.result {
        common::IsolatedResult::Completed { exit_code } => {
            assert_eq!(*exit_code, 0, "minimal_throw should exit 0");
        }
        common::IsolatedResult::RuntimeError(msg) => {
            panic!(
                "minimal_throw returned a runtime error: {msg}\nstdout:\n{}\nstderr:\n{}",
                run.stdout, run.stderr
            );
        }
        common::IsolatedResult::Crashed(msg) => {
            panic!(
                "minimal_throw crashed: {msg}\nstdout:\n{}\nstderr:\n{}",
                run.stdout, run.stderr
            );
        }
    }

    assert_output_contains_in_order(&run.stdout, &["start", "in try", "caught: Test", "OK"]);
    assert!(
        !run.stdout.contains("FAIL"),
        "minimal_throw should not print failure markers, got:\n{}",
        run.stdout
    );
}

#[test]
fn runtime_try_catch_test_handles_nested_catch_and_finally_in_subprocess() {
    let Some(run) = run_exception_fixture("try_catch_test") else { return };

    match &run.result {
        common::IsolatedResult::Completed { exit_code } => {
            assert_eq!(*exit_code, 0, "try_catch_test should exit 0");
        }
        common::IsolatedResult::RuntimeError(msg) => {
            panic!(
                "try_catch_test returned a runtime error: {msg}\nstdout:\n{}\nstderr:\n{}",
                run.stdout, run.stderr
            );
        }
        common::IsolatedResult::Crashed(msg) => {
            panic!(
                "try_catch_test crashed: {msg}\nstdout:\n{}\nstderr:\n{}",
                run.stdout, run.stderr
            );
        }
    }

    assert_output_contains_in_order(
        &run.stdout,
        &[
            "caught: TestException",
            "in try",
            "inner caught: Inner",
            "outer caught: Outer",
            "OK",
        ],
    );
    assert!(
        !run.stdout.contains("FAIL"),
        "try_catch_test should not print failure markers, got:\n{}",
        run.stdout
    );
}
