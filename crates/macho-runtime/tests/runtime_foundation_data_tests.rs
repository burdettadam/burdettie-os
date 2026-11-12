//! NSData, NSString path, NSProcessInfo, and blocks+Foundation integration tests.

mod common;

// ── T24: NSData basics ─────────────────────────────────────────────

#[test]
fn runtime_nsdata_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsdata_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsdata_basics should exit 0");
}

#[test]
fn runtime_nsdata_class_registered() {
    let Some(_result) = common::run_fixture("nsdata_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsdata = rt.get_class("NSData").expect("NSData should be registered");
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "length").is_some(),
        "NSData.length"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "bytes").is_some(),
        "NSData.bytes"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "description").is_some(),
        "NSData.description"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "isEqualToData:")
            .is_some(),
        "NSData.isEqualToData:"
    );
}

// ── T25: NSString path utilities ───────────────────────────────────

#[test]
fn runtime_nsstring_path_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_path/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsstring_path should exit 0");
}

#[test]
fn runtime_nsstring_path_methods_registered() {
    let Some(_result) = common::run_fixture("nsstring_path/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsstring = rt
        .get_class("NSString")
        .expect("NSString should be registered");
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "pathExtension")
            .is_some(),
        "NSString.pathExtension"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "lastPathComponent")
            .is_some(),
        "NSString.lastPathComponent"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByDeletingLastPathComponent")
            .is_some(),
        "NSString.stringByDeletingLastPathComponent"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByDeletingPathExtension")
            .is_some(),
        "NSString.stringByDeletingPathExtension"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByAppendingPathComponent:")
            .is_some(),
        "NSString.stringByAppendingPathComponent:"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "isAbsolutePath")
            .is_some(),
        "NSString.isAbsolutePath"
    );
}

// ── T26: NSProcessInfo basics ──────────────────────────────────────

#[test]
fn runtime_nsprocessinfo_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsprocessinfo_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsprocessinfo_basics should exit 0");
}

#[test]
fn runtime_nsprocessinfo_class_registered() {
    let Some(_result) = common::run_fixture("nsprocessinfo_basics/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nspi = rt
        .get_class("NSProcessInfo")
        .expect("NSProcessInfo should be registered");
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "processName").is_some(),
        "NSProcessInfo.processName"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "processIdentifier")
            .is_some(),
        "NSProcessInfo.processIdentifier"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "hostName").is_some(),
        "NSProcessInfo.hostName"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "description").is_some(),
        "NSProcessInfo.description"
    );
}

// ── T20: blocks + Foundation (enumerateObjects, sort, dispatch_sync) ──

#[test]
fn runtime_blocks_foundation_exit_zero() {
    let Some(result) = common::run_fixture("blocks_foundation/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "blocks_foundation should exit 0");
}

#[test]
fn runtime_blocks_foundation_methods_registered() {
    let Some(_result) = common::run_fixture("blocks_foundation/app") else {
        return;
    };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsarray = rt
        .get_class("NSArray")
        .expect("NSArray should be registered");
    assert!(
        rt.msg_lookup(nsarray.mapped_addr, "enumerateObjectsUsingBlock:")
            .is_some(),
        "NSArray.enumerateObjectsUsingBlock:"
    );
    assert!(
        rt.msg_lookup(nsarray.mapped_addr, "sortedArrayUsingComparator:")
            .is_some(),
        "NSArray.sortedArrayUsingComparator:"
    );

    let nsmutarr = rt
        .get_class("NSMutableArray")
        .expect("NSMutableArray should be registered");
    assert!(
        rt.msg_lookup(nsmutarr.mapped_addr, "sortUsingComparator:")
            .is_some(),
        "NSMutableArray.sortUsingComparator:"
    );

    let nsdict = rt
        .get_class("NSDictionary")
        .expect("NSDictionary should be registered");
    assert!(
        rt.msg_lookup(nsdict.mapped_addr, "enumerateKeysAndObjectsUsingBlock:")
            .is_some(),
        "NSDictionary.enumerateKeysAndObjectsUsingBlock:"
    );
}
