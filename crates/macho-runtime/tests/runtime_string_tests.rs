//! NSString/CFString ops, blocks, and inheritance integration tests.

mod common;

use common::{fixtures_dir, ShimSet, RUNTIME_LOCK};

// ── NSString/CFString ops tests (Phase 25, T9) ────────────────────

#[test]
fn runtime_nsstring_ops_exit_zero() {
    let Some(result) = common::run_fixture("nsstring_ops/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "nsstring_ops should exit 0");
}

#[test]
fn runtime_nsstring_ops_cfstring_shims_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_ops/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table
            .resolve("_CFStringCreateWithCString")
            .is_some(),
        "_CFStringCreateWithCString should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_CFStringGetLength").is_some(),
        "_CFStringGetLength should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_CFStringGetCStringPtr").is_some(),
        "_CFStringGetCStringPtr should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_kCFStringEncodingUTF8").is_some(),
        "_kCFStringEncodingUTF8 should be in shim table"
    );
}

// ── Blocks runtime tests (Phase 26, T11) ──────────────────────────

#[test]
fn runtime_block_basics_exit_zero() {
    let Some(result) = common::run_fixture("block_basics/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "block_basics should exit 0");
}

#[test]
fn runtime_block_basics_shims_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("block_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("__NSConcreteGlobalBlock").is_some(),
        "__NSConcreteGlobalBlock should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__NSConcreteStackBlock").is_some(),
        "__NSConcreteStackBlock should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__Block_copy").is_some(),
        "__Block_copy should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__Block_release").is_some(),
        "__Block_release should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_objc_retainBlock").is_some(),
        "_objc_retainBlock should be in shim table"
    );
}

// ── Inheritance + __objc_stubs tests (Phase 27, T12) ──────────────

#[test]
fn runtime_objc_inherit_exit_zero() {
    let Some(result) = common::run_fixture("objc_inherit/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "objc_inherit should exit 0");
}

#[test]
fn runtime_objc_inherit_superclass_chain() {
    let Some(_result) = common::run_fixture("objc_inherit/app") else {
        return;
    };

    // Verify superclass chain is parsed
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        let shape = rt.get_class("Shape").expect("Shape should be registered");
        assert!(shape.superclass_addr.is_none(), "Shape is root class");

        let circle = rt.get_class("Circle").expect("Circle should be registered");
        assert!(
            circle.superclass_addr.is_some(),
            "Circle should have superclass"
        );

        let filled = rt
            .get_class("FilledCircle")
            .expect("FilledCircle should be registered");
        assert!(
            filled.superclass_addr.is_some(),
            "FilledCircle should have superclass"
        );

        // FilledCircle inherits area from Circle (chain traversal)
        let area_imp = rt.msg_lookup(filled.mapped_addr, "area");
        assert!(
            area_imp.is_some(),
            "FilledCircle should find 'area' via superclass chain"
        );

        // FilledCircle inherits sides from Shape (two levels up)
        let sides_imp = rt.msg_lookup(filled.mapped_addr, "sides");
        assert!(
            sides_imp.is_some(),
            "FilledCircle should find 'sides' two levels up"
        );
    }
}
