//! Shim registration table.
//!
//! The `ShimTable` maps symbol names to function pointer addresses.
//! It implements the `SymbolResolver` pattern used by the runtime's
//! resolution chain: shim table → image exports → host fallback.

use std::collections::HashMap;

/// Batch-register shims into a [`ShimTable`].
///
/// Each entry is `(symbol_name, address_expr, description)`.
///
/// ```ignore
/// register_shims!(table,
///     ("_getprogname", shim_getprogname as u64, "getprogname shim"),
///     ("_setprogname", shim_setprogname as u64, "setprogname shim"),
/// );
/// ```
#[macro_export]
macro_rules! register_shims {
    ($table:expr, $( ($name:expr, $addr:expr, $desc:expr) ),* $(,)?) => {
        $( $table.register($name, $addr, $desc); )*
    };
}

/// A single shim entry in the table.
#[derive(Debug, Clone)]
pub struct ShimEntry {
    /// The symbol name (with leading underscore, e.g. "_printf").
    pub name: &'static str,
    /// The function pointer address.
    pub address: u64,
    /// Human-readable description for debugging.
    pub description: &'static str,
}

/// Symbol → address mapping for runtime shims.
///
/// The table is populated at runtime startup before fixups are applied.
/// During fixup resolution, the runtime checks the shim table first,
/// then falls back to image exports and finally the host process.
#[derive(Debug, Default, Clone)]
pub struct ShimTable {
    entries: HashMap<&'static str, ShimEntry>,
}

impl ShimTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a shim table with pre-reserved capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
        }
    }

    /// Reserve additional capacity for upcoming shim registrations.
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    /// Register a shim function under the given symbol name.
    ///
    /// The name should include the leading underscore (e.g. `_getprogname`).
    pub fn register(&mut self, name: &'static str, address: u64, description: &'static str) {
        self.entries.insert(
            name,
            ShimEntry {
                name,
                address,
                description,
            },
        );
    }

    /// Look up a symbol in the shim table.
    pub fn resolve(&self, name: &str) -> Option<u64> {
        self.entries.get(name).map(|e| e.address)
    }

    /// Look up without leading underscore (strips `_` prefix if present).
    pub fn resolve_stripped(&self, name: &str) -> Option<u64> {
        // Try exact match first
        if let Some(addr) = self.resolve(name) {
            return Some(addr);
        }
        // Try with leading underscore
        let with_prefix = format!("_{name}");
        self.resolve(&with_prefix)
    }

    /// Number of registered shims.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all registered shims.
    pub fn iter(&self) -> impl Iterator<Item = &ShimEntry> {
        self.entries.values()
    }

    /// Get all registered symbol names (sorted for deterministic output).
    pub fn symbol_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.keys().copied().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve() {
        let mut table = ShimTable::new();
        table.register("_getprogname", 0xDEAD, "process name shim");
        assert_eq!(table.resolve("_getprogname"), Some(0xDEAD));
        assert_eq!(table.resolve("_missing"), None);
    }

    #[test]
    fn resolve_stripped() {
        let mut table = ShimTable::new();
        table.register("_strlen", 0xBEEF, "string length");
        assert_eq!(table.resolve_stripped("strlen"), Some(0xBEEF));
        assert_eq!(table.resolve_stripped("_strlen"), Some(0xBEEF));
    }

    #[test]
    fn len_and_empty() {
        let mut table = ShimTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        table.register("_foo", 1, "test");
        assert!(!table.is_empty());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn symbol_names_sorted() {
        let mut table = ShimTable::new();
        table.register("_zebra", 3, "z");
        table.register("_alpha", 1, "a");
        table.register("_middle", 2, "m");
        let names = table.symbol_names();
        assert_eq!(names, vec!["_alpha", "_middle", "_zebra"]);
    }

    #[test]
    fn overwrite_replaces() {
        let mut table = ShimTable::new();
        table.register("_foo", 1, "first");
        table.register("_foo", 2, "second");
        assert_eq!(table.resolve("_foo"), Some(2));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn with_capacity_and_reserve_preserve_behavior() {
        let mut table = ShimTable::with_capacity(16);
        table.reserve(8);
        table.register("_foo", 1, "test");
        table.register("_bar", 2, "test");

        assert_eq!(table.resolve("_foo"), Some(1));
        assert_eq!(table.resolve("_bar"), Some(2));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn iter_exposes_registered_name_without_duplication_logic() {
        let mut table = ShimTable::new();
        table.register("_foo", 1, "test foo");

        let entries: Vec<&ShimEntry> = table.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "_foo");
        assert_eq!(entries[0].address, 1);
        assert_eq!(entries[0].description, "test foo");
    }
}
