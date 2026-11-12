//! NSError, NSBundle, NSFileManager, Exception, and Real CLI integration tests.

mod common;

// ── T27: NSError basics ────────────────────────────────────────────

#[test]
fn runtime_nserror_basics_exit_zero() {
    let Some(result) = common::run_fixture("nserror_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nserror_basics should exit 0");
}

#[test]
fn runtime_nserror_class_registered() {
    let Some(_result) = common::run_fixture("nserror_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nserror = rt
        .get_class("NSError")
        .expect("NSError should be registered");
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "domain").is_some(),
        "NSError.domain"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "code").is_some(),
        "NSError.code"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "userInfo").is_some(),
        "NSError.userInfo"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "localizedDescription")
            .is_some(),
        "NSError.localizedDescription"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "description").is_some(),
        "NSError.description"
    );
}

// ── T28: NSBundle basics ───────────────────────────────────────────

#[test]
fn runtime_nsbundle_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsbundle_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsbundle_basics should exit 0");
}

#[test]
fn runtime_nsbundle_class_registered() {
    let Some(_result) = common::run_fixture("nsbundle_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsbundle = rt
        .get_class("NSBundle")
        .expect("NSBundle should be registered");
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "bundlePath").is_some(),
        "NSBundle.bundlePath"
    );
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "executablePath")
            .is_some(),
        "NSBundle.executablePath"
    );
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "bundleIdentifier")
            .is_some(),
        "NSBundle.bundleIdentifier"
    );
}

// ── T29: NSFileManager basics ──────────────────────────────────────

#[test]
fn runtime_nsfilemanager_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsfilemanager_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsfilemanager_basics should exit 0");
}

#[test]
fn runtime_nsfilemanager_class_registered() {
    let Some(_result) = common::run_fixture("nsfilemanager_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsfm = rt
        .get_class("NSFileManager")
        .expect("NSFileManager should be registered");
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "fileExistsAtPath:")
            .is_some(),
        "NSFileManager.fileExistsAtPath:"
    );
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "currentDirectoryPath")
            .is_some(),
        "NSFileManager.currentDirectoryPath"
    );
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "contentsOfDirectoryAtPath:error:")
            .is_some(),
        "NSFileManager.contentsOfDirectoryAtPath:error:"
    );
}

// ── T30: Exception basics ──────────────────────────────────────────

#[test]
fn runtime_exception_basics_exit_zero() {
    let Some(result) = common::run_fixture("exception_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "exception_basics should exit 0");
}

#[test]
fn runtime_nsexception_class_registered() {
    let Some(_result) = common::run_fixture("exception_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsexc = rt
        .get_class("NSException")
        .expect("NSException should be registered");
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "name").is_some(),
        "NSException.name"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "reason").is_some(),
        "NSException.reason"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "userInfo").is_some(),
        "NSException.userInfo"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "description").is_some(),
        "NSException.description"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "raise").is_some(),
        "NSException.raise"
    );
}

// ── T31: Real CLI — comprehensive Foundation integration ──────────

#[test]
fn runtime_real_cli_exit_zero() {
    let Some(result) = common::run_fixture("real_cli/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "real_cli should exit 0");
}

#[test]
fn runtime_real_cli_classes_registered() {
    let Some(_result) = common::run_fixture("real_cli/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // Verify all Foundation classes used by real_cli are registered
    for class_name in &[
        "NSString",
        "NSMutableString",
        "NSArray",
        "NSMutableArray",
        "NSDictionary",
        "NSMutableDictionary",
        "NSNumber",
        "NSData",
        "NSProcessInfo",
        "NSFileManager",
        "NSBundle",
        "NSError",
        "NSException",
    ] {
        assert!(
            rt.get_class(class_name).is_some(),
            "{class_name} should be registered"
        );
    }
}
