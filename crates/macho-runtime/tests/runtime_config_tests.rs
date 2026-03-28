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
    assert!(config.host_fallback);
    assert!(config.security_policy.is_none());
    assert_eq!(config.process_name, "test-binary");
}

#[test]
fn secure_config_has_policy() {
    let path = PathBuf::from("/tmp/test-binary");
    let config = RuntimeConfig::secure(&path);
    assert!(config.security_policy.is_some());
    assert!(config.host_fallback);
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

    // Shim count (updated after adding objc_setProperty_nonatomic_copy shim)
    #[cfg(target_os = "macos")]
    assert_eq!(ctx.shim_table.len(), 351);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(ctx.shim_table.len(), 351);
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
    };
    let ctx = RuntimeContext::new(&config, &shim_set);

    // Only TLS shims: 2
    assert_eq!(ctx.shim_table.len(), 2);
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
    let resolver = CompositeResolver::new(&table, Some(&stub));

    // Should get our shim address, not the host's
    assert_eq!(resolver.resolve("_printf"), Some(0xCAFE));
}

#[test]
fn composite_resolver_host_fallback() {
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
    let resolver = CompositeResolver::new(&table, Some(&stub));

    // strlen should resolve via host
    assert!(resolver.resolve("_strlen").is_some());
}
