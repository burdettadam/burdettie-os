//! NSLog, NSArray, NSNumber, and NSDictionary integration tests.

mod common;

// ── Phase 31: NSLog variadic trampoline tests ──────────────────────

#[test]
fn runtime_nslog_format_at_sign_outputs_string() {
    let Some((result, lines)) = common::run_fixture_with_nslog("objc_foundation/app") else { return };
    assert_eq!(result.exit_code, 0);

    // T8 calls NSLog(@"%@", @"hello from Foundation")
    assert!(
        lines.iter().any(|l| l.contains("hello from Foundation")),
        "NSLog should output 'hello from Foundation', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_nslog_dynamic_nsstring() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsstring_dynamic/app") else { return };
    assert_eq!(result.exit_code, 0);

    // T15 calls NSLog(@"T15 str1=%@", str1) where str1 = "hello"
    assert!(
        lines.iter().any(|l| l.contains("T15 str1=hello")),
        "NSLog should output 'T15 str1=hello', got: {:?}",
        lines,
    );
}

// ── T16: NSArray basics ────────────────────────────────────────────

#[test]
fn runtime_nsarray_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsarray_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "nsarray_basics should exit 0");
}

#[test]
fn runtime_nsarray_class_registered() {
    let Some(_result) = common::run_fixture("nsarray_basics/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsarray = rt.get_class("NSArray").expect("NSArray registered");
        assert_eq!(nsarray.name, "NSArray");
        assert_eq!(nsarray.instance_size, 24);
        assert_eq!(
            nsarray.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsarray_nslog_captures_element() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsarray_basics/app") else { return };
    assert_eq!(result.exit_code, 0);

    // T16 calls NSLog(@"T16 first=%@", first) where first = "apple"
    assert!(
        lines.iter().any(|l| l.contains("T16 first=apple")),
        "NSLog should output 'T16 first=apple', got: {:?}",
        lines,
    );
}

// ── T17: NSNumber basics ─────────────────────────────────────────

#[test]
fn runtime_nsnumber_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsnumber_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "nsnumber_basics should exit 0");
}

#[test]
fn runtime_nsnumber_class_registered() {
    let Some(_result) = common::run_fixture("nsnumber_basics/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsnumber = rt.get_class("NSNumber").expect("NSNumber registered");
        assert_eq!(nsnumber.name, "NSNumber");
        assert_eq!(nsnumber.instance_size, 24);
        assert_eq!(
            nsnumber.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsnumber_nslog_captures_value() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsnumber_basics/app") else { return };
    assert_eq!(result.exit_code, 0);

    // T17 calls NSLog(@"T17 n42=%@", n42) where n42 = numberWithInt:42
    assert!(
        lines.iter().any(|l| l.contains("T17 n42=42")),
        "NSLog should output 'T17 n42=42', got: {:?}",
        lines,
    );
}

// ── T18: NSDictionary basics ─────────────────────────────────────

#[test]
fn runtime_nsdictionary_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsdictionary_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "nsdictionary_basics should exit 0");
}

#[test]
fn runtime_nsdictionary_class_registered() {
    let Some(_result) = common::run_fixture("nsdictionary_basics/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsdict = rt
            .get_class("NSDictionary")
            .expect("NSDictionary registered");
        assert_eq!(nsdict.name, "NSDictionary");
        assert_eq!(nsdict.instance_size, 32);
        assert_eq!(
            nsdict.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsdictionary_nslog_captures_description() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsdictionary_basics/app") else { return };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T18 dict=")),
        "NSLog should output 'T18 dict=...', got: {:?}",
        lines,
    );
}
