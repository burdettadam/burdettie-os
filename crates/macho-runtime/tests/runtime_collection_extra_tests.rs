//! NSString manipulation, NSString format, NSMutableArray, NSMutableDictionary,
//! and NSLog format specifier integration tests.

mod common;

// ── T19: NSString manipulation ──────────────────────────────────────

#[test]
fn runtime_nsstring_manip_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_manip/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsstring_manip should exit 0");
}

#[test]
fn runtime_nsstring_manip_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsstring_manip/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T19 appended=foobar")),
        "NSLog should output 'T19 appended=foobar', got: {:?}",
        lines,
    );
}

// ── T20: NSString stringWithFormat: ─────────────────────────────────

#[test]
fn runtime_nsstring_format_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_format/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsstring_format should exit 0");
}

#[test]
fn runtime_nsstring_format_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsstring_format/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines
            .iter()
            .any(|l| l.contains("T20 formatted=item=widget")),
        "NSLog should output 'T20 formatted=item=widget', got: {:?}",
        lines,
    );
}

// ── T21: NSMutableArray basics ──────────────────────────────────────

#[test]
fn runtime_nsmutablearray_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsmutablearray_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsmutablearray_basics should exit 0");
}

#[test]
fn runtime_nsmutablearray_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsmutablearray_basics/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T21 mutarr=")),
        "NSLog should output 'T21 mutarr=...', got: {:?}",
        lines,
    );
}

// ── T22: NSMutableDictionary basics ─────────────────────────────────

#[test]
fn runtime_nsmutabledictionary_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsmutabledictionary_basics/app") else {
        return;
    };

    assert_eq!(
        result.exit_code, 0,
        "nsmutabledictionary_basics should exit 0"
    );
}

#[test]
fn runtime_nsmutabledictionary_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsmutabledictionary_basics/app")
    else {
        return;
    };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T22 mutdict=")),
        "NSLog should output 'T22 mutdict=...', got: {:?}",
        lines,
    );
}

// ── T23: NSLog format specifiers ────────────────────────────────────

#[test]
fn runtime_nslog_format_specifiers_exit_zero() {
    let Some(result) = common::run_fixture("nslog_format_specifiers/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nslog_format_specifiers should exit 0");
}

#[test]
fn runtime_nslog_format_specifiers_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nslog_format_specifiers/app")
    else {
        return;
    };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T23 float=3.14")),
        "expected 'T23 float=3.14...', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 hex=ff")),
        "expected 'T23 hex=ff', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 lld=-123456789")),
        "expected 'T23 lld=-123456789', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 oct=10")),
        "expected 'T23 oct=10', got: {:?}",
        lines,
    );
}
