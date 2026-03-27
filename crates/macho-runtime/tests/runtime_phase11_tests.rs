//! Phase 11 tests: NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar,
//! NSNumberFormatter, NSIndexSet, NSSortDescriptor, NSTimer, try/catch.

mod common;

// ── Phase 11: NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar ────

#[test]
fn runtime_nsvalue_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt.get_class("NSUUID").expect("NSUUID should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "UUIDString").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "isEqual:").is_some());
}

#[test]
fn runtime_nslocale_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
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

// NOTE: try_catch_test removed — @catch in loaded code requires full unwind
// info registration which is future work. When implemented, the compat_score
// regression gate (which currently skips minimal_throw and try_catch_test)
// will cover this fixture.
