//! Config construction, runtime context, and resolver composition tests.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use common::{RuntimeConfig, ShimSet};

// ── Config construction tests ────────────────────────────────────────

#[test]
fn permissive_config_defaults() {
    let path = PathBuf::from("/tmp/test-binary");
    let config = RuntimeConfig::permissive(&path);
    assert!(config.security_policy.is_none());
    assert_eq!(config.process_name, "test-binary");
}

#[test]
fn secure_config_has_policy() {
    let path = PathBuf::from("/tmp/test-binary");
    let config = RuntimeConfig::secure(&path);
    assert!(config.security_policy.is_some());
}

#[test]
fn shim_set_defaults_all_enabled() {
    let shim_set = ShimSet::default();
    assert!(shim_set.process);
    assert!(shim_set.tls);
    assert!(shim_set.dispatch);
    assert!(shim_set.pthread);
    assert!(shim_set.mach);
    assert!(shim_set.lifecycle);
    assert!(shim_set.objc);
}

// ── Runtime context tests ────────────────────────────────────────────

#[test]
fn context_installs_all_shims() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    // Keep this as a floor rather than an exact value: the runtime keeps
    // growing shim coverage, and exact-count assertions become noisy churn.
    assert!(
        ctx.shim_table.len() >= 450,
        "expected full runtime shim install to retain broad coverage"
    );
    assert_eq!(ctx.shim_table.len(), ctx.shim_table.symbol_names().len());
}

#[test]
fn context_respects_shim_set() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet {
        process: false,
        tls: true,
        dispatch: false,
        pthread: false,
        mach: false,
        lifecycle: false,
        ptrauth: false,
        objc: false,
        foundation: false,
        random: false,
        darwin_stubs: false,
    };
    let ctx = RuntimeContext::new(&config, &shim_set);

    // Only TLS shims: 2
    assert_eq!(ctx.shim_table.len(), 2);
}

#[test]
fn context_installs_representative_symbols_for_each_shim_family() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let ctx = RuntimeContext::new(&config, &ShimSet::default());

    for symbol in [
        "__NSGetExecutablePath",
        "__tlv_bootstrap",
        "_dispatch_once",
        "_pthread_self",
        "_mach_absolute_time",
        "___cxa_atexit",
        "_ptrauth_strip",
        "_objc_msgSend",
        "_NSLog",
        "_arc4random",
    ] {
        assert!(
            ctx.shim_table.resolve(symbol).is_some(),
            "expected representative shim symbol {symbol} to be installed"
        );
    }
}

#[test]
fn context_can_disable_foundation_without_affecting_objc_or_tls() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet {
        process: false,
        tls: true,
        dispatch: false,
        pthread: false,
        mach: false,
        lifecycle: false,
        ptrauth: false,
        objc: true,
        foundation: false,
        random: false,
        darwin_stubs: false,
    };
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(ctx.shim_table.resolve("__tlv_bootstrap").is_some());
    assert!(ctx.shim_table.resolve("_objc_msgSend").is_some());
    assert!(
        ctx.shim_table.resolve("_NSLog").is_none(),
        "Foundation shims should be absent when foundation=false"
    );
    assert!(
        ctx.shim_table
            .resolve("___CFConstantStringClassReference")
            .is_none(),
        "CF/Foundation symbols should be absent when foundation=false"
    );
}

// ── Resolver composition tests ────────────────────────────────────────

#[test]
fn composite_resolver_shim_priority() {
    use libsystem_shims::ShimTable;
    use macho_exec::{StubResolver, SymbolResolver};
    use macho_runtime::CompositeResolver;

    let mut table = ShimTable::new();
    table.register("_printf", 0xCAFE, "test override");

    let stub = {
        #[cfg(target_os = "macos")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());
        StubResolver::new(host)
    };
    let data = macho_exec::DataSymbolStore::new();
    let resolver = CompositeResolver::new(&table, &data, Some(&stub));

    // Should get our shim address, not the host's
    assert_eq!(resolver.resolve("_printf"), Some(0xCAFE));
}

#[test]
fn composite_resolver_always_uses_stub() {
    use libsystem_shims::ShimTable;
    use macho_exec::{StubResolver, SymbolResolver};
    use macho_runtime::CompositeResolver;

    let table = ShimTable::new();
    let stub = {
        #[cfg(target_os = "macos")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());
        StubResolver::new(host)
    };
    let data = macho_exec::DataSymbolStore::new();
    let resolver = CompositeResolver::new(&table, &data, Some(&stub));

    // strlen should resolve via the always-active safe-symbol StubResolver
    assert!(resolver.resolve("_strlen").is_some());
}
