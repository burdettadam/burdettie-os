//! NSInvocation, ARM64e/PAC, and Phase B6 additional Foundation class tests.

mod common;

use common::{RUNTIME_LOCK, ShimSet};

// ── Phase B4: NSInvocation & Dynamic Method Resolution ─────────────

#[test]
fn runtime_nsinvocation_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsinvocation_basics/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "nsinvocation_basics fixture should exit 0"
    );
}

#[test]
fn runtime_forwarding_basics_exit_zero() {
    let Some(result) = common::run_fixture("forwarding_basics/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "forwarding_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nsinvocation_data_symbols_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_OBJC_CLASS_$_NSMethodSignature").is_some(),
        "NSMethodSignature class data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_METACLASS_$_NSMethodSignature").is_some(),
        "NSMethodSignature metaclass data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_CLASS_$_NSInvocation").is_some(),
        "NSInvocation class data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_METACLASS_$_NSInvocation").is_some(),
        "NSInvocation metaclass data symbol should be registered"
    );
}

// ── ARM64e / PAC tests ─────────────────────────────────────────────

#[test]
fn runtime_arm64e_basic_exit_zero() {
    let Some(result) = common::run_fixture("arm64e_basic/app") else { return };

    assert_eq!(result.exit_code, 0, "arm64e_basic should exit with code 0");
    assert!(result.shims_installed > 0);
}

#[test]
fn ptrauth_shims_are_registered() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_ptrauth_strip").is_some(),
        "ptrauth_strip shim should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_ptrauth_sign_unauthenticated").is_some(),
        "ptrauth_sign_unauthenticated shim should be registered"
    );
}

// ── Phase B6: Additional Foundation Classes ────────────────────────

#[test]
fn runtime_nsjson_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsjson_basics/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "nsjson_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nsregex_basics_exit_zero() {
    let Some(result) = common::run_fixture("nsregex_basics/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "nsregex_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nscache_basics_exit_zero() {
    let Some(result) = common::run_fixture("nscache_basics/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "nscache_basics fixture should exit 0"
    );
}

#[test]
fn runtime_b6_data_symbols_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    let classes = [
        "NSRegularExpression",
        "NSPredicate",
        "NSAttributedString",
        "NSMutableAttributedString",
        "NSCache",
        "NSMapTable",
        "NSHashTable",
        "NSOperationQueue",
        "NSOperation",
        "NSBlockOperation",
        "NSJSONSerialization",
        "NSPropertyListSerialization",
    ];

    for name in &classes {
        let class_sym = format!("_OBJC_CLASS_$_{}", name);
        let meta_sym = format!("_OBJC_METACLASS_$_{}", name);
        assert!(
            ctx.shim_table.resolve(&class_sym).is_some(),
            "{} class data symbol should be registered",
            name
        );
        assert!(
            ctx.shim_table.resolve(&meta_sym).is_some(),
            "{} metaclass data symbol should be registered",
            name
        );
    }
}
