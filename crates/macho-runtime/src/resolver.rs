//! Composite symbol resolver: shims → registry → host fallback.
//!
//! The resolution chain tries each source in order. This is the core
//! mechanism that lets us intercept system library calls with our shims
//! while still letting through the 70+ safe symbols via StubResolver.

use libsystem_shims::ShimTable;
use macho_exec::{StubResolver, SymbolResolver};

/// Layered resolver: shim table first, then optional host fallback.
pub struct CompositeResolver<'a> {
    shim_table: &'a ShimTable,
    host_fallback: Option<&'a StubResolver>,
}

impl<'a> CompositeResolver<'a> {
    pub fn new(shim_table: &'a ShimTable, host_fallback: Option<&'a StubResolver>) -> Self {
        Self {
            shim_table,
            host_fallback,
        }
    }
}

impl SymbolResolver for CompositeResolver<'_> {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        // 1. Try shim table (exact match, then stripped)
        if let Some(addr) = self.shim_table.resolve(symbol_name) {
            log::trace!("resolved '{}' via shim table", symbol_name);
            return Some(addr);
        }
        if let Some(addr) = self.shim_table.resolve_stripped(symbol_name) {
            log::trace!("resolved '{}' via shim table (stripped)", symbol_name);
            return Some(addr);
        }

        // 2. Try host fallback (StubResolver with safe allowlist)
        if let Some(stub) = self.host_fallback {
            if let Some(addr) = stub.resolve(symbol_name) {
                log::trace!("resolved '{}' via host fallback", symbol_name);
                return Some(addr);
            }
        }

        log::debug!("unresolved symbol: '{}'", symbol_name);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stub_resolver() -> StubResolver {
        #[cfg(target_os = "macos")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        #[cfg(target_os = "linux")]
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_linux::LinuxSymbolResolver::new());
        StubResolver::new(host)
    }

    #[test]
    fn shim_table_takes_priority() {
        let mut table = ShimTable::new();
        table.register("_printf", 0x1234, "test shim");

        let stub = test_stub_resolver();
        let composite = CompositeResolver::new(&table, Some(&stub));

        // Should resolve to our shim address, not the host's printf
        let addr = composite.resolve("_printf").unwrap();
        assert_eq!(addr, 0x1234);
    }

    #[test]
    fn falls_through_to_host() {
        let table = ShimTable::new(); // empty
        let stub = test_stub_resolver();
        let composite = CompositeResolver::new(&table, Some(&stub));

        // strlen should resolve via host StubResolver
        assert!(composite.resolve("_strlen").is_some());
    }

    #[test]
    fn no_fallback_returns_none() {
        let table = ShimTable::new();
        let composite = CompositeResolver::new(&table, None);

        assert!(composite.resolve("_strlen").is_none());
    }
}
