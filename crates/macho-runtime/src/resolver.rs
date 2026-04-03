//! Composite symbol resolver: shims → data symbols → safe-symbol StubResolver.
//!
//! The resolution chain tries each source in order. This is the core
//! mechanism that lets us intercept system library calls with our shims
//! while still letting through the curated safe-list of C stdlib symbols
//! via the always-active StubResolver.

use libsystem_shims::ShimTable;
use macho_exec::{StubResolver, SymbolResolver, DataSymbolStore};

/// Layered resolver: shim table first, then DataSymbolStore, then StubResolver.
pub struct CompositeResolver<'a> {
    shim_table: &'a ShimTable,
    data_symbols: &'a DataSymbolStore,
    stub_resolver: Option<&'a StubResolver>,
}

impl<'a> CompositeResolver<'a> {
    pub fn new(
        shim_table: &'a ShimTable,
        data_symbols: &'a DataSymbolStore,
        stub_resolver: Option<&'a StubResolver>,
    ) -> Self {
        Self {
            shim_table,
            data_symbols,
            stub_resolver,
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

        // 2. Try DataSymbolStore for constant data symbols (works on Linux too)
        if let Some(addr) = self.data_symbols.resolve(symbol_name) {
            log::trace!("resolved '{}' via DataSymbolStore", symbol_name);
            return Some(addr);
        }

        // 3. Try StubResolver for safe C stdlib / system symbols (macOS host dlsym)
        if let Some(stub) = self.stub_resolver {
            if let Some(addr) = stub.resolve(symbol_name) {
                log::trace!("resolved '{}' via StubResolver safe-list", symbol_name);
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
        let data = DataSymbolStore::new();
        let composite = CompositeResolver::new(&table, &data, Some(&stub));

        // Should resolve to our shim address, not the host's printf
        let addr = composite.resolve("_printf").unwrap();
        assert_eq!(addr, 0x1234);
    }

    #[test]
    fn data_symbol_store_tier() {
        let table = ShimTable::new(); // empty
        let stub = test_stub_resolver();
        let data = DataSymbolStore::new();
        let composite = CompositeResolver::new(&table, &data, Some(&stub));

        // XPC type tokens should resolve via DataSymbolStore
        let addr = composite.resolve("__xpc_type_dictionary");
        assert!(addr.is_some(), "DataSymbolStore should resolve __xpc_type_dictionary");
        assert_ne!(addr.unwrap(), 0, "resolved address must be non-NULL");
    }

    #[test]
    fn falls_through_to_stub_resolver() {
        let table = ShimTable::new(); // empty
        let stub = test_stub_resolver();
        let data = DataSymbolStore::new();
        let composite = CompositeResolver::new(&table, &data, Some(&stub));

        // strlen should resolve via the safe-symbol StubResolver
        assert!(composite.resolve("_strlen").is_some());
    }
}
