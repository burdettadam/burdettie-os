//! ObjC runtime, category/load, Foundation/CF-lite, and multi-dylib ObjC tests.

mod common;

use common::{RUNTIME_LOCK, ShimSet, fixtures_dir};

// ── ObjC runtime tests ─────────────────────────────────────────────

#[test]
fn runtime_objc_minimal() {
    let Some(result) = common::run_fixture("objc_minimal/objc_minimal") else { return };

    assert_eq!(result.exit_code, 0, "objc_minimal should exit 0");
}

#[test]
fn runtime_objc_minimal_class_registration() {
    // After running objc_minimal, verify the global ObjC runtime was populated
    let Some(_result) = common::run_fixture("objc_minimal/objc_minimal") else { return };

    // The global runtime should have registered the Answer class
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Answer").is_some(),
            "Answer class should be registered"
        );
        let cls = rt.get_class("Answer").unwrap();
        assert!(!cls.methods.is_empty(), "Answer should have methods");
        assert!(
            cls.methods.iter().any(|m| m.selector == "value"),
            "Answer should have a 'value' method"
        );
    }
}

// ── ObjC category + +load tests ────────────────────────────────────

#[test]
fn runtime_objc_category_exit_zero() {
    let Some(result) = common::run_fixture("objc_category/objc_category") else { return };

    assert_eq!(result.exit_code, 0, "objc_category should exit 0");
}

#[test]
fn runtime_objc_category_class_registered() {
    let Some(_result) = common::run_fixture("objc_category/objc_category") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter class should be registered"
        );
        let cls = rt.get_class("Greeter").unwrap();
        // Should have 'greeting' (original) + 'farewell' (category)
        assert!(
            cls.methods.iter().any(|m| m.selector == "greeting"),
            "Greeter should have 'greeting' method"
        );
        assert!(
            cls.methods.iter().any(|m| m.selector == "farewell"),
            "Greeter should have 'farewell' from category"
        );
    }
}

#[test]
fn runtime_objc_category_load_method_detected() {
    let Some(_result) = common::run_fixture("objc_category/objc_category") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let loads = rt.collect_load_methods();
        assert!(
            loads.iter().any(|(name, _)| name == "Greeter"),
            "Greeter should have a +load method detected"
        );
    }
}

// ── Foundation / CF-lite tests (Phase 22, T8) ──────────────────────

#[test]
fn runtime_objc_foundation_exit_zero() {
    let Some(result) = common::run_fixture("objc_foundation/app") else { return };

    assert_eq!(result.exit_code, 0, "objc_foundation should exit 0");
}

#[test]
fn runtime_objc_foundation_nslog_shim_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    // Verify foundation shims are in the shim table
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_NSLog").is_some(),
        "_NSLog should be in shim table"
    );
    assert!(
        ctx.shim_table
            .resolve("___CFConstantStringClassReference")
            .is_some(),
        "___CFConstantStringClassReference should be in shim table"
    );
}

#[test]
fn runtime_objc_classes_with_foundation_exit_zero() {
    let Some(result) = common::run_fixture("objc_classes/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "objc_classes should exit 0 with foundation shims"
    );
}

#[test]
fn runtime_objc_classes_foundation_classes_registered() {
    let Some(_result) = common::run_fixture("objc_classes/app") else { return };

    // Verify classes from the fixture were registered alongside foundation symbols
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter should be registered with foundation shims active"
        );
        assert!(
            rt.get_class("Calculator").is_some(),
            "Calculator should be registered with foundation shims active"
        );
    }
}

// ── Multi-dylib ObjC tests (Phase 23, T10) ────────────────────────

#[test]
fn runtime_objc_dylib_chain_exit_zero() {
    let Some(result) = common::run_fixture("objc_dylib_chain/app") else { return };

    assert_eq!(result.exit_code, 0, "objc_dylib_chain should exit 0");
    assert!(
        result.images_loaded >= 3,
        "should load app + libgreeter + libextras"
    );
}

#[test]
fn runtime_objc_dylib_chain_cross_image_class() {
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else { return };

    // Greeter class should be registered (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter should be registered from libgreeter.dylib"
        );
    }
}

#[test]
fn runtime_objc_dylib_chain_cross_image_category() {
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else { return };

    // Verify category from libextras was merged into Greeter from libgreeter
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let cls = rt
            .get_class("Greeter")
            .expect("Greeter should be registered");
        assert!(
            cls.methods.iter().any(|m| m.selector == "greeting"),
            "Greeter should have 'greeting' from libgreeter"
        );
        assert!(
            cls.methods.iter().any(|m| m.selector == "farewell"),
            "Greeter should have 'farewell' from libextras category"
        );
    }
}

#[test]
fn runtime_objc_dylib_chain_load_method() {
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else { return };

    // Verify +load was detected for Greeter (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let loads = rt.collect_load_methods();
        assert!(
            loads.iter().any(|(name, _)| name == "Greeter"),
            "Greeter should have +load from libgreeter"
        );
    }
}
