//! KVO, KVC, and method swizzling integration tests.

mod common;

use common::{RUNTIME_LOCK, ShimSet};

// ── KVO (Key-Value Observing) tests (Phase B1) ────────────────────

#[test]
fn runtime_kvo_basics_exit_zero() {
    let Some(result) = common::run_fixture("kvo_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "kvo_basics should exit 0");
}

#[test]
fn runtime_kvo_basics_nsobject_has_kvo_methods() {
    let Some(_result) = common::run_fixture("kvo_basics/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsobject = rt.get_class("NSObject").expect("NSObject should exist");
        let kvo_methods = [
            "addObserver:forKeyPath:options:context:",
            "removeObserver:forKeyPath:",
            "willChangeValueForKey:",
            "didChangeValueForKey:",
            "observeValueForKeyPath:ofObject:change:context:",
        ];
        for sel in &kvo_methods {
            assert!(
                nsobject.methods.iter().any(|m| m.selector == *sel),
                "NSObject should have KVO method '{sel}'"
            );
        }
    }
}

#[test]
fn runtime_kvo_multiple_observers_exit_zero() {
    let Some(result) = common::run_fixture("kvo_multiple_observers/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "kvo_multiple_observers should exit 0"
    );
}

// ── KVC tests ──────────────────────────────────────────────────────

#[test]
fn runtime_kvc_basics_exit_zero() {
    let Some(result) = common::run_fixture("kvc_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "kvc_basics should exit 0");
}

#[test]
fn runtime_kvc_nsobject_has_kvc_methods() {
    let Some(_result) = common::run_fixture("kvc_basics/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(rt) = guard.as_ref() {
        let nsobject = rt.get_class("NSObject").expect("NSObject should exist");
        let kvc_methods = [
            "valueForKey:",
            "setValue:forKey:",
            "valueForKeyPath:",
            "setValue:forKeyPath:",
            "valueForUndefinedKey:",
            "setValue:forUndefinedKey:",
        ];
        for sel in &kvc_methods {
            assert!(
                nsobject.methods.iter().any(|m| m.selector == *sel),
                "NSObject should have KVC method '{sel}'"
            );
        }
    }
}

#[test]
fn runtime_kvc_simple_exit_zero() {
    let Some(result) = common::run_fixture("kvc_simple/app") else { return };

    assert_eq!(result.exit_code, 0, "kvc_simple should exit 0");
}

// ── KVC collection operators (Phase B2 completion) ────────────────

/// Smoke test: fixture that exercises @count, @sum, @avg, @min, @max,
/// @unionOfObjects, and @distinctUnionOfObjects must complete with exit 0.
#[test]
fn runtime_kvc_operators_exit_zero() {
    let Some(result) = common::run_fixture("kvc_operators/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "kvc_operators should exit 0 — all 7 collection operator tests must pass"
    );
}

/// After running the kvc_operators fixture the ObjC runtime should have
/// the Product class registered (confirming the binary loaded fully).
#[test]
fn runtime_kvc_operators_product_class_registered() {
    let Some(_result) = common::run_fixture("kvc_operators/app") else { return };

    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Product").is_some(),
            "Product class must be registered after kvc_operators fixture runs"
        );
    }
}

// ── Method Swizzling tests ─────────────────────────────────────────

#[test]
fn runtime_swizzle_basics_exit_zero() {
    let Some(result) = common::run_fixture("swizzle_basics/app") else { return };

    assert_eq!(result.exit_code, 0, "swizzle_basics should exit 0");
}

#[test]
fn runtime_swizzle_restore_exit_zero() {
    let Some(result) = common::run_fixture("swizzle_restore/app") else { return };

    assert_eq!(result.exit_code, 0, "swizzle_restore should exit 0");
}

#[test]
fn runtime_swizzle_shims_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    // All three swizzling APIs should be in the shim table
    assert!(
        ctx.shim_table.resolve("_method_exchangeImplementations").is_some(),
        "method_exchangeImplementations should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_method_setImplementation").is_some(),
        "method_setImplementation should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_class_replaceMethod").is_some(),
        "class_replaceMethod should be registered"
    );
}
