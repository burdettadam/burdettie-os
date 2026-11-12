//! Interpreter tests for ObjC and Foundation fixtures.
//!
//! These tests use `Runtime::load_and_run_interpreted` to execute fixtures
//! through the ARM64 interpreter instead of native execution. This validates
//! the interpreter's ability to handle ObjC message dispatch, Foundation
//! class usage, and CF toll-free bridging.
//!
//! On Linux ARM64, the interpreter is the primary execution path since
//! the native ARM64 trampolines are macOS-only assembly.

mod common;

use std::sync::Mutex;

// Separate lock — interpreted tests must not run concurrently with native
// tests because both mutate the global OBJC_RUNTIME.
static INTERP_LOCK: Mutex<()> = Mutex::new(());

/// Resolve the fixture binary under `fixtures/<dir>`.
/// Tries `<dir>/app` first (current convention), then `<dir>/<dir_name>`
/// (legacy naming for objc_minimal, return42, etc.).
fn fixture_binary(fixture_dir: &str) -> Option<std::path::PathBuf> {
    let base = macho_fixtures::fixtures_dir().join(fixture_dir);
    let app = base.join("app");
    if app.exists() {
        return Some(app);
    }
    // Legacy naming: binary has same name as directory.
    let legacy = base.join(fixture_dir);
    if legacy.exists() {
        return Some(legacy);
    }
    None
}

fn run_interpreted(fixture_dir: &str) -> Option<macho_runtime::RuntimeResult> {
    let _lock = INTERP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Also take the native lock to avoid races with native runtime tests.
    let _native = common::RUNTIME_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let Some(fixture) = fixture_binary(fixture_dir) else {
        eprintln!("Skipping: fixture not found for {fixture_dir}");
        return None;
    };
    let config = macho_runtime::RuntimeConfig::permissive(&fixture);
    Some(
        macho_runtime::Runtime::load_and_run_interpreted(&fixture, &config)
            .expect("interpreted execution failed"),
    )
}

// ── C-only fixtures ────────────────────────────────────────────────

#[test]
fn interp_return42() {
    let Some(r) = run_interpreted("return42") else {
        return;
    };
    assert_eq!(r.exit_code, 42, "return42 should exit 42");
}

#[test]
fn interp_simple_cli() {
    let Some(r) = run_interpreted("simple_cli") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "simple_cli should exit 0");
}

#[test]
fn interp_constructor_test() {
    let Some(r) = run_interpreted("constructor_test") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "constructor_test should exit 0");
}

#[test]
fn interp_process_info() {
    let Some(r) = run_interpreted("process_info") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "process_info should exit 0");
}

// ── ObjC Runtime Fixtures ──────────────────────────────────────────

#[test]
fn interp_objc_minimal() {
    let Some(r) = run_interpreted("objc_minimal") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_minimal should exit 0");
}

#[test]
fn interp_objc_category() {
    let Some(r) = run_interpreted("objc_category") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_category should exit 0");
}

#[test]
fn interp_objc_classes() {
    let Some(r) = run_interpreted("objc_classes") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_classes should exit 0");
}

#[test]
fn interp_objc_inherit() {
    let Some(r) = run_interpreted("objc_inherit") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_inherit should exit 0");
}

#[test]
fn interp_objc_super() {
    let Some(r) = run_interpreted("objc_super") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_super should exit 0");
}

#[test]
fn interp_objc_foundation() {
    let Some(r) = run_interpreted("objc_foundation") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "objc_foundation should exit 0");
}

#[test]
fn interp_protocol_basics() {
    let Some(r) = run_interpreted("protocol_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "protocol_basics should exit 0");
}

#[test]
fn interp_dynamic_class() {
    let Some(r) = run_interpreted("dynamic_class") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "dynamic_class should exit 0");
}

#[test]
fn interp_struct_return() {
    let Some(r) = run_interpreted("struct_return") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "struct_return should exit 0");
}

#[test]
fn interp_cf_bridging() {
    let Some(r) = run_interpreted("cf_bridging") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "cf_bridging should exit 0");
}

// ── Foundation Fixtures ────────────────────────────────────────────

#[test]
fn interp_nsstring_ops() {
    let Some(r) = run_interpreted("nsstring_ops") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsstring_ops should exit 0");
}

#[test]
fn interp_nsstring_format() {
    let Some(r) = run_interpreted("nsstring_format") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsstring_format should exit 0");
}

#[test]
fn interp_nsstring_manip() {
    let Some(r) = run_interpreted("nsstring_manip") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsstring_manip should exit 0");
}

#[test]
fn interp_nsnumber_basics() {
    let Some(r) = run_interpreted("nsnumber_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsnumber_basics should exit 0");
}

#[test]
fn interp_nsnull_basics() {
    let Some(r) = run_interpreted("nsnull_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsnull_basics should exit 0");
}

#[test]
fn interp_nserror_basics() {
    let Some(r) = run_interpreted("nserror_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nserror_basics should exit 0");
}

#[test]
fn interp_nsdata_basics() {
    let Some(r) = run_interpreted("nsdata_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsdata_basics should exit 0");
}

#[test]
fn interp_nsarray_basics() {
    let Some(r) = run_interpreted("nsarray_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsarray_basics should exit 0");
}

#[test]
fn interp_nsmutablearray_basics() {
    let Some(r) = run_interpreted("nsmutablearray_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsmutablearray_basics should exit 0");
}

#[test]
fn interp_nsdictionary_basics() {
    let Some(r) = run_interpreted("nsdictionary_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsdictionary_basics should exit 0");
}

#[test]
fn interp_nsmutabledictionary_basics() {
    let Some(r) = run_interpreted("nsmutabledictionary_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsmutabledictionary_basics should exit 0");
}

#[test]
fn interp_nsdate_basics() {
    let Some(r) = run_interpreted("nsdate_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsdate_basics should exit 0");
}

#[test]
fn interp_nsset_basics() {
    let Some(r) = run_interpreted("nsset_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsset_basics should exit 0");
}

#[test]
fn interp_nslog_format_specifiers() {
    let Some(r) = run_interpreted("nslog_format_specifiers") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nslog_format_specifiers should exit 0");
}

#[test]
fn interp_nsvalue_basics() {
    let Some(r) = run_interpreted("nsvalue_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsvalue_basics should exit 0");
}

#[test]
fn interp_nsurl_basics() {
    let Some(r) = run_interpreted("nsurl_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsurl_basics should exit 0");
}

#[test]
fn interp_nsuuid_basics() {
    let Some(r) = run_interpreted("nsuuid_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsuuid_basics should exit 0");
}

#[test]
fn interp_nsindexset_basics() {
    let Some(r) = run_interpreted("nsindexset_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsindexset_basics should exit 0");
}

#[test]
fn interp_nsnotification_basics() {
    let Some(r) = run_interpreted("nsnotification_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsnotification_basics should exit 0");
}

#[test]
fn interp_nscache_basics() {
    let Some(r) = run_interpreted("nscache_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nscache_basics should exit 0");
}

#[test]
fn interp_nsjson_basics() {
    let Some(r) = run_interpreted("nsjson_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsjson_basics should exit 0");
}

#[test]
fn interp_nscharacterset_basics() {
    let Some(r) = run_interpreted("nscharacterset_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nscharacterset_basics should exit 0");
}

#[test]
fn interp_nsobject_methods() {
    let Some(r) = run_interpreted("nsobject_methods") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nsobject_methods should exit 0");
}

#[test]
fn interp_nscalendar_basics() {
    let Some(r) = run_interpreted("nscalendar_basics") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "nscalendar_basics should exit 0");
}

#[test]
fn interp_collection_enum() {
    let Some(r) = run_interpreted("collection_enum") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "collection_enum should exit 0");
}

#[test]
fn interp_kvc_simple() {
    let Some(r) = run_interpreted("kvc_simple") else {
        return;
    };
    assert_eq!(r.exit_code, 0, "kvc_simple should exit 0");
}
