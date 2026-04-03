//! Composite symbol resolver: shims → data symbols → eager cache → safe-symbol StubResolver.
//!
//! The resolution chain tries each source in order. This is the core
//! mechanism that lets us intercept system library calls with our shims
//! while still letting through the curated safe-list of C stdlib symbols
//! via the always-active StubResolver.

use std::collections::HashMap;

use libsystem_shims::ShimTable;
use macho_exec::{StubResolver, SymbolResolver, DataSymbolStore};

/// Pre-resolved symbol map from shared cache export tries.
///
/// Built once at startup by `SharedCacheLoader::eager_export_map()`.
/// Provides O(1) HashMap lookups that bypass the per-symbol `dlsym`
/// overhead of the StubResolver.
pub struct EagerCacheResolver {
    exports: HashMap<String, u64>,
}

impl EagerCacheResolver {
    /// Create a resolver from a pre-built export map.
    pub fn new(exports: HashMap<String, u64>) -> Self {
        Self { exports }
    }

    /// Create an empty resolver (no-op, all lookups return None).
    pub fn empty() -> Self {
        Self { exports: HashMap::new() }
    }

    /// Number of pre-resolved symbols.
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    /// Whether the resolver has any pre-resolved symbols.
    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }
}

impl SymbolResolver for EagerCacheResolver {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        self.exports.get(symbol_name).copied()
    }
}

/// Layered resolver: shim table first, then DataSymbolStore, then
/// EagerCacheResolver, then StubResolver.
pub struct CompositeResolver<'a> {
    shim_table: &'a ShimTable,
    data_symbols: &'a DataSymbolStore,
    eager_cache: Option<&'a EagerCacheResolver>,
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
            eager_cache: None,
            stub_resolver,
        }
    }

    /// Set the eager cache resolver tier.
    pub fn with_eager_cache(mut self, cache: &'a EagerCacheResolver) -> Self {
        self.eager_cache = Some(cache);
        self
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

        // 3. Try EagerCacheResolver for pre-resolved shared cache exports
        if let Some(cache) = self.eager_cache {
            if let Some(addr) = cache.resolve(symbol_name) {
                log::trace!("resolved '{}' via EagerCacheResolver", symbol_name);
                return Some(addr);
            }
        }

        // 4. Try StubResolver for safe C stdlib / system symbols (macOS host dlsym)
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

    #[test]
    fn eager_cache_resolver_intercepts() {
        let mut exports = HashMap::new();
        exports.insert("_my_cached_symbol".to_string(), 0xCAFE_1234u64);

        let table = ShimTable::new();
        let data = DataSymbolStore::new();
        let cache = EagerCacheResolver::new(exports);
        let composite = CompositeResolver::new(&table, &data, None)
            .with_eager_cache(&cache);

        // Should resolve via the eager cache tier
        let addr = composite.resolve("_my_cached_symbol");
        assert_eq!(addr, Some(0xCAFE_1234));
    }

    #[test]
    fn eager_cache_resolver_empty_resolves_nothing() {
        let cache = EagerCacheResolver::empty();
        assert!(cache.is_empty());
        assert_eq!(cache.resolve("_anything"), None);
    }

    #[test]
    fn shim_table_beats_eager_cache() {
        let mut table = ShimTable::new();
        table.register("_printf", 0x1111, "shim");

        let mut exports = HashMap::new();
        exports.insert("_printf".to_string(), 0x2222u64);

        let data = DataSymbolStore::new();
        let cache = EagerCacheResolver::new(exports);
        let composite = CompositeResolver::new(&table, &data, None)
            .with_eager_cache(&cache);

        // Shim table should win over eager cache
        assert_eq!(composite.resolve("_printf"), Some(0x1111));
    }
}
