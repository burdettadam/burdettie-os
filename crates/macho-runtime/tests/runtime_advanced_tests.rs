//! Collection enumeration, NSDate, NSUserDefaults, NSSet, NSURL, NSNull,
//! NSDateFormatter, NSCharacterSet, and NSNotificationCenter tests.

mod common;

// ── T32: Collection Enumeration ─────────────────────────────────────

#[test]
fn runtime_collection_enum_exit_zero() {
    let Some(result) = common::run_fixture("collection_enum/app") else { return };
    assert_eq!(result.exit_code, 0, "collection_enum should exit 0");
}

#[test]
fn runtime_nsenumerator_class_registered() {
    let Some(_result) = common::run_fixture("collection_enum/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsenum = rt
        .get_class("NSEnumerator")
        .expect("NSEnumerator should be registered");
    assert!(
        rt.msg_lookup(nsenum.mapped_addr, "nextObject").is_some(),
        "NSEnumerator.nextObject"
    );
    assert!(
        rt.msg_lookup(nsenum.mapped_addr, "allObjects").is_some(),
        "NSEnumerator.allObjects"
    );
}

// ── T33: NSDate Basics ──────────────────────────────────────────────

#[test]
fn runtime_nsdate_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsdate_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsdate_basics should exit 0");
}

#[test]
fn runtime_nsdate_class_registered() {
    let Some(_result) = common::run_fixture("nsdate_basics/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsdate = rt.get_class("NSDate").expect("NSDate should be registered");
    assert!(rt
        .msg_lookup(nsdate.mapped_addr, "timeIntervalSince1970")
        .is_some());
    assert!(rt.msg_lookup(nsdate.mapped_addr, "description").is_some());
    assert!(rt.msg_lookup(nsdate.mapped_addr, "compare:").is_some());
    assert!(rt
        .msg_lookup(nsdate.mapped_addr, "isEqualToDate:")
        .is_some());
}

// ── T34: NSUserDefaults Basics ──────────────────────────────────────

#[test]
fn runtime_nsuserdefaults_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsuserdefaults_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsuserdefaults_basics should exit 0");
}

#[test]
fn runtime_nsuserdefaults_class_registered() {
    let Some(_result) = common::run_fixture("nsuserdefaults_basics/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsud = rt
        .get_class("NSUserDefaults")
        .expect("NSUserDefaults should be registered");
    assert!(rt.msg_lookup(nsud.mapped_addr, "objectForKey:").is_some());
    assert!(rt
        .msg_lookup(nsud.mapped_addr, "setObject:forKey:")
        .is_some());
    assert!(rt.msg_lookup(nsud.mapped_addr, "synchronize").is_some());
}

// ── NSSet basics ───────────────────────────────────────────────────

#[test]
fn runtime_nsset_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsset_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsset_basics should exit 0");
}

#[test]
fn runtime_nsset_class_registered() {
    let Some(_result) = common::run_fixture("nsset_basics/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsset = rt.get_class("NSSet").expect("NSSet should be registered");
    assert!(rt.msg_lookup(nsset.mapped_addr, "count").is_some());
    assert!(rt
        .msg_lookup(nsset.mapped_addr, "containsObject:")
        .is_some());

    let nsmset = rt
        .get_class("NSMutableSet")
        .expect("NSMutableSet should be registered");
    assert!(rt.msg_lookup(nsmset.mapped_addr, "addObject:").is_some());
    assert!(rt.msg_lookup(nsmset.mapped_addr, "removeObject:").is_some());
}

// ── NSURL basics ───────────────────────────────────────────────────

#[test]
fn runtime_nsurl_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsurl_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsurl_basics should exit 0");
}

#[test]
fn runtime_nsurl_class_registered() {
    let Some(_result) = common::run_fixture("nsurl_basics/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsurl = rt.get_class("NSURL").expect("NSURL should be registered");
    assert!(rt.msg_lookup(nsurl.mapped_addr, "path").is_some());
    assert!(rt.msg_lookup(nsurl.mapped_addr, "absoluteString").is_some());
    assert!(rt.msg_lookup(nsurl.mapped_addr, "isFileURL").is_some());
}

// ── NSNull basics ──────────────────────────────────────────────────

#[test]
fn runtime_nsnull_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsnull_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsnull_basics should exit 0");
}

#[test]
fn runtime_nsnull_class_registered() {
    let Some(_result) = common::run_fixture("nsnull_basics/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsnull = rt.get_class("NSNull").expect("NSNull should be registered");
    assert!(rt.msg_lookup(nsnull.mapped_addr, "description").is_some());
}

// ── NSDateFormatter ────────────────────────────────────────────────

#[test]
fn runtime_nsdateformatter_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSDateFormatter")
        .expect("NSDateFormatter should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "stringFromDate:").is_some());
}

#[test]
fn runtime_nsdateformatter_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsdateformatter_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsdateformatter_basics should exit 0");
}

// ── NSCharacterSet ─────────────────────────────────────────────────

#[test]
fn runtime_nscharacterset_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSCharacterSet")
        .expect("NSCharacterSet should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "characterIsMember:")
        .is_some());
}

#[test]
fn runtime_nscharacterset_basics_exit_zero() {
    let Some(result) = common::run_fixture("nscharacterset_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nscharacterset_basics should exit 0");
}

// ── NSNotificationCenter ───────────────────────────────────────────

#[test]
fn runtime_nsnotificationcenter_class_registered() {
    let Some(_result) = common::run_fixture("return42/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSNotificationCenter")
        .expect("NSNotificationCenter should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "postNotificationName:object:")
        .is_some());
}

#[test]
fn runtime_nsnotification_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsnotification_basics/app") else { return };
    assert_eq!(result.exit_code, 0, "nsnotification_basics should exit 0");
}
