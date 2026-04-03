//! NSMutableString, autorelease pools, runtime introspection, and property introspection tests.

mod common;

// ── T21: NSMutableString + NSString enhancements ────────────────────

#[test]
fn runtime_nsstring_mutable_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_mutable/app") else { return };

    assert_eq!(result.exit_code, 0, "nsstring_mutable should exit 0");
}

#[test]
fn runtime_nsstring_mutable_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("nsstring_mutable/app") else { return };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T21 appended=Hello World")),
        "NSLog should output 'T21 appended=Hello World', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T21 replaced=Hello Rust")),
        "NSLog should output 'T21 replaced=Hello Rust', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T21 final=testing")),
        "NSLog should output 'T21 final=testing', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_nsstring_mutable_class_hierarchy() {
    let Some(_result) = common::run_fixture("nsstring_mutable/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // NSMutableString exists and its superclass is NSString
    let ms_info = rt.get_class("NSMutableString");
    assert!(
        ms_info.is_some(),
        "NSMutableString class should be registered"
    );
    let ms = ms_info.unwrap();
    assert_eq!(
        ms.superclass_addr,
        Some(cf_lite::nsobj::nsstring_class_addr())
    );

    // Key selectors should be resolvable
    for sel in &[
        "appendString:",
        "insertString:atIndex:",
        "deleteCharactersInRange:",
        "replaceCharactersInRange:withString:",
        "setString:",
        "length",
        "UTF8String",
    ] {
        let imp = rt.lookup_method("NSMutableString", sel);
        assert!(imp.is_some(), "NSMutableString should respond to {}", sel);
    }

    // NSString should have the new methods
    for sel in &[
        "stringByReplacingOccurrencesOfString:withString:",
        "componentsSeparatedByString:",
        "rangeOfString:",
        "integerValue",
        "doubleValue",
        "mutableCopy",
    ] {
        let imp = rt.lookup_method("NSString", sel);
        assert!(imp.is_some(), "NSString should respond to {}", sel);
    }
}

// ── T22: Autorelease pools + retain/release ─────────────────────────

#[test]
fn runtime_autorelease_exit_zero() {
    let Some(result) = common::run_fixture("autorelease/app") else { return };

    assert_eq!(result.exit_code, 0, "autorelease fixture should exit 0");
}

#[test]
fn runtime_autorelease_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("autorelease/app") else { return };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines
            .iter()
            .any(|l| l.contains("T22 pool1=hello autorelease")),
        "NSLog should output 'T22 pool1=hello autorelease', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T22 done=ok")),
        "NSLog should output 'T22 done=ok', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_autorelease_nsobject_memory_methods() {
    let Some(_result) = common::run_fixture("autorelease/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // NSObject should have memory management methods
    for sel in &["retain", "release", "autorelease", "retainCount", "dealloc"] {
        let imp = rt.lookup_method("NSObject", sel);
        assert!(imp.is_some(), "NSObject should respond to {}", sel);
    }
}

// ── T23: runtime_introspection ─────────────────────────────────────

#[test]
fn runtime_introspection_exit_zero() {
    let Some((result, _lines)) = common::run_fixture_with_nslog("runtime_introspection/app") else { return };
    assert_eq!(result.exit_code, 0, "runtime_introspection should exit 0");
}

#[test]
fn runtime_introspection_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("runtime_introspection/app") else { return };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T23 done=ok")),
        "NSLog should output 'T23 done=ok', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 addMethod=YES")),
        "NSLog should output 'T23 addMethod=YES', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 isMetaClass=NO")),
        "NSLog should output 'T23 isMetaClass=NO', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 methodName=alpha")),
        "NSLog should output 'T23 methodName=alpha', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_introspection_shims_resolvable() {
    let Some(_result) = common::run_fixture("runtime_introspection/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // Probe class should exist with its methods
    assert!(
        rt.lookup_method("Probe", "alpha").is_some(),
        "Probe should have alpha"
    );
    assert!(
        rt.lookup_method("Probe", "beta:").is_some(),
        "Probe should have beta:"
    );
}

// ── T19: property_introspection ────────────────────────────────────

#[test]
fn runtime_property_introspection_exit_zero() {
    let Some((result, _lines)) = common::run_fixture_with_nslog("property_introspection/app") else { return };
    assert_eq!(result.exit_code, 0, "property_introspection should exit 0");
}

#[test]
fn runtime_property_introspection_nslog_captures() {
    let Some((result, lines)) = common::run_fixture_with_nslog("property_introspection/app") else { return };
    assert_eq!(result.exit_code, 0);

    assert!(
        lines.iter().any(|l| l.contains("T19 done=ok")),
        "NSLog should output 'T19 done=ok', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T19 propCount=")),
        "NSLog should output property count, got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T19 ivarCount=")),
        "NSLog should output ivar count, got: {:?}",
        lines,
    );
}

#[test]
fn runtime_property_introspection_class_has_ivars() {
    let Some(_result) = common::run_fixture("property_introspection/app") else { return };

    let rt = objc_runtime::OBJC_RUNTIME.read().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let vehicle = rt.get_class("Vehicle").expect("Vehicle class should exist");
    assert!(!vehicle.ivars.is_empty(), "Vehicle should have ivars");
    assert!(
        !vehicle.properties.is_empty(),
        "Vehicle should have properties"
    );

    // Verify specific ivar names
    let ivar_names: Vec<&str> = vehicle.ivars.iter().map(|iv| iv.name.as_str()).collect();
    assert!(
        ivar_names.contains(&"_year"),
        "Vehicle should have _year ivar, got: {:?}",
        ivar_names
    );
    assert!(
        ivar_names.contains(&"_horsepower"),
        "Vehicle should have _horsepower ivar, got: {:?}",
        ivar_names
    );

    // Verify specific property names
    let prop_names: Vec<&str> = vehicle.properties.iter().map(|p| p.name.as_str()).collect();
    assert!(
        prop_names.contains(&"make"),
        "Vehicle should have make property, got: {:?}",
        prop_names
    );
    assert!(
        prop_names.contains(&"horsepower"),
        "Vehicle should have horsepower property, got: {:?}",
        prop_names
    );
}
