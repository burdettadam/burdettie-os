//! DYLD interposing support.
//!
//! Parses `__DATA,__interpose` sections (SectionType 0x0D) from loaded images
//! and builds an interpose table mapping replacee addresses to replacement
//! addresses.  Also provides `InterposeResolver` which wraps an inner resolver
//! and applies symbol-name-level interposing from insert library exports.

use std::collections::HashMap;

use crate::SymbolResolver;

/// Parsed interpose tuple (resolved addresses).
#[derive(Debug, Clone, Copy)]
pub struct InterposeTuple {
    /// Address of the replacement function (from the insert library).
    pub replacement: u64,
    /// Address of the original function to replace.
    pub replacee: u64,
}

/// Address-level interpose table built from `__DATA,__interpose` sections.
///
/// After fixups are applied to insert libraries, their `__interpose` sections
/// contain resolved (replacement, replacee) address pairs.  When applying
/// fixups to other images, any bind that resolves to a `replacee` address
/// should be redirected to the corresponding `replacement`.
#[derive(Debug, Default)]
pub struct InterposeTable {
    map: HashMap<u64, u64>,
}

impl InterposeTable {
    /// Create an empty interpose table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a (replacee → replacement) mapping.
    pub fn insert(&mut self, replacee: u64, replacement: u64) {
        self.map.insert(replacee, replacement);
    }

    /// Look up a replacement for the given address.
    pub fn get(&self, addr: u64) -> Option<u64> {
        self.map.get(&addr).copied()
    }

    /// Number of interpose entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Extract interpose tuples from a mapped image's `__DATA,__interpose`
    /// sections.  The image must already have fixups applied so the
    /// replacement/replacee pointers are resolved.
    pub fn collect_from_image(
        &mut self,
        image: &crate::MappedImage,
        parsed: &macho_parse::ParsedMachO,
    ) {
        for seg in &parsed.segments {
            for sec in &seg.sections {
                if sec.section_type_name() == macho_types::SectionType::Interposing {
                    self.collect_from_section(image, sec);
                }
            }
        }
    }

    fn collect_from_section(
        &mut self,
        image: &crate::MappedImage,
        section: &macho_types::Section,
    ) {
        // Each tuple is 16 bytes: [replacement: u64, replacee: u64]
        let tuple_size = 16usize;
        let count = section.size as usize / tuple_size;
        if count == 0 {
            return;
        }

        // Find the mapped segment that contains this section.
        let section_vm_addr = section.addr;
        for mapped_seg in &image.segments {
            let seg_start = mapped_seg.vmaddr;
            let seg_end = seg_start + mapped_seg.size as u64;
            if section_vm_addr >= seg_start && section_vm_addr < seg_end {
                let offset_in_seg = (section_vm_addr - seg_start) as usize;
                let base = mapped_seg.base as *const u8;
                for i in 0..count {
                    let tuple_ptr = unsafe { base.add(offset_in_seg + i * tuple_size) };
                    let replacement = unsafe { *(tuple_ptr as *const u64) };
                    let replacee = unsafe { *(tuple_ptr.add(8) as *const u64) };
                    if replacement != 0 && replacee != 0 {
                        log::debug!(
                            "interpose: {:#x} → {:#x}",
                            replacee,
                            replacement,
                        );
                        self.insert(replacee, replacement);
                    }
                }
                break;
            }
        }
    }
}

/// Symbol-name-level interposing resolver.
///
/// Wraps an inner `SymbolResolver` and overrides resolution for symbols
/// that appear in the insert library export tables.  This handles the
/// common `DYLD_INSERT_LIBRARIES` case where the insert library simply
/// re-exports symbols that shadow the originals.
pub struct InterposeResolver<'a> {
    /// Export map from insert libraries: symbol name → address.
    overrides: HashMap<String, u64>,
    /// The inner resolver to fall back to.
    inner: &'a dyn SymbolResolver,
}

impl<'a> InterposeResolver<'a> {
    /// Create a resolver with the given override map and inner fallback.
    pub fn new(overrides: HashMap<String, u64>, inner: &'a dyn SymbolResolver) -> Self {
        Self { overrides, inner }
    }

    /// Number of override symbols.
    pub fn len(&self) -> usize {
        self.overrides.len()
    }

    /// Whether there are any overrides.
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }
}

impl SymbolResolver for InterposeResolver<'_> {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        // Check interpose overrides first.
        if let Some(&addr) = self.overrides.get(symbol_name) {
            log::trace!("resolved '{}' via interpose override", symbol_name);
            return Some(addr);
        }
        self.inner.resolve(symbol_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyResolver;
    impl SymbolResolver for DummyResolver {
        fn resolve(&self, name: &str) -> Option<u64> {
            if name == "_original" { Some(0xAAAA) } else { None }
        }
    }

    #[test]
    fn interpose_table_basic() {
        let mut table = InterposeTable::new();
        assert!(table.is_empty());
        table.insert(0x1000, 0x2000);
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(0x1000), Some(0x2000));
        assert_eq!(table.get(0x3000), None);
    }

    #[test]
    fn interpose_resolver_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("_original".to_string(), 0xBBBB);
        let resolver = InterposeResolver::new(overrides, &DummyResolver);
        // Should return override, not the inner resolver's value.
        assert_eq!(resolver.resolve("_original"), Some(0xBBBB));
    }

    #[test]
    fn interpose_resolver_falls_through() {
        let overrides = HashMap::new(); // empty
        let resolver = InterposeResolver::new(overrides, &DummyResolver);
        assert_eq!(resolver.resolve("_original"), Some(0xAAAA));
        assert_eq!(resolver.resolve("_missing"), None);
    }

    #[test]
    fn interpose_table_duplicate_replaces() {
        let mut table = InterposeTable::new();
        table.insert(0x1000, 0x2000);
        table.insert(0x1000, 0x3000);
        assert_eq!(table.get(0x1000), Some(0x3000));
        assert_eq!(table.len(), 1);
    }
}
