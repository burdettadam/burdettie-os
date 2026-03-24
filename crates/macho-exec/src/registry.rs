//! Multi-image registry for loading dylib dependency graphs.
//!
//! `ImageRegistry` holds multiple mapped Mach-O images and provides
//! cross-image symbol resolution via export tries.  Images should be
//! loaded bottom-up (leaf dependencies first) so that when fixups are
//! applied, all required symbols are already available.

use std::collections::HashMap;
use std::sync::Arc;

use macho_map::MappingPlan;
use macho_parse::{
    extract_exports, extract_fixups, parse, ExportedSymbol, FixupModel, ParsedMachO,
};
use platform_core::memory::MemoryMapper;

use crate::{ExecError, MappedImage, SymbolResolver};

/// A single Mach-O image that has been parsed and mapped into process memory.
pub struct LoadedImage {
    pub image: MappedImage,
    pub parsed: ParsedMachO,
    pub data: Vec<u8>,
    pub install_name: String,
    pub exports: Vec<ExportedSymbol>,
}

/// Registry of mapped images with cross-image symbol resolution.
pub struct ImageRegistry {
    images: Vec<LoadedImage>,
    name_index: HashMap<String, usize>,
    mapper: Arc<dyn MemoryMapper>,
}

impl ImageRegistry {
    pub fn new(mapper: Arc<dyn MemoryMapper>) -> Self {
        Self {
            images: Vec::new(),
            name_index: HashMap::new(),
            mapper,
        }
    }

    /// Parse, map, and register a Mach-O binary under the given install name.
    ///
    /// Returns the index of the newly loaded image.
    pub fn load(&mut self, install_name: &str, data: Vec<u8>) -> Result<usize, ExecError> {
        let parsed = parse(&data).map_err(|e| ExecError::SecurityDenied {
            reason: format!("parse failed: {e}"),
        })?;
        let plan = MappingPlan::from_parsed(&parsed);
        let image = MappedImage::map(&plan, &data, self.mapper.clone())?;
        let exports = extract_exports(&parsed, &data);

        let idx = self.images.len();
        self.name_index.insert(install_name.to_string(), idx);
        self.images.push(LoadedImage {
            image,
            parsed,
            data,
            install_name: install_name.to_string(),
            exports,
        });
        Ok(idx)
    }

    /// Look up a loaded image by its install name.
    pub fn get(&self, install_name: &str) -> Option<&LoadedImage> {
        self.name_index.get(install_name).map(|&i| &self.images[i])
    }

    /// All loaded images, in load order.
    pub fn images(&self) -> &[LoadedImage] {
        &self.images
    }

    /// Apply fixups (chained or classic rebases/binds) to every loaded image,
    /// resolving cross-image symbols via export tries and falling back to
    /// the provided system resolver.
    pub fn apply_all_fixups(&self, system_resolver: &dyn SymbolResolver) -> Result<(), ExecError> {
        let composite = RegistryResolver {
            registry: self,
            fallback: system_resolver,
        };
        for loaded in &self.images {
            let fixups = extract_fixups(&loaded.parsed, &loaded.data);
            match &fixups.model {
                FixupModel::Chained { .. } => {
                    crate::apply_chained_fixups(
                        &loaded.image,
                        &loaded.parsed,
                        &loaded.data,
                        &composite,
                    )?;
                }
                FixupModel::Classic => {
                    // Apply classic rebase opcodes (add slide)
                    crate::apply_rebases(&loaded.image, &fixups)?;
                    // Apply classic bind opcodes via composite resolver
                    // Build a BindingReport-compatible lookup from the composite resolver
                    apply_classic_binds_with_resolver(&loaded.image, &fixups, &composite)?;
                }
                FixupModel::None => {
                    // No fixups to apply
                }
            }
        }
        Ok(())
    }

    /// Build a `RegistryResolver` that searches all loaded images'
    /// export tries before falling back to the given system resolver.
    pub fn resolver<'a>(&'a self, fallback: &'a dyn SymbolResolver) -> RegistryResolver<'a> {
        RegistryResolver {
            registry: self,
            fallback,
        }
    }
}

/// Composite resolver: searches loaded images' export tries, then falls
/// back to a system-level resolver (typically `StubResolver` / `dlsym`).
pub struct RegistryResolver<'a> {
    registry: &'a ImageRegistry,
    fallback: &'a dyn SymbolResolver,
}

impl<'a> SymbolResolver for RegistryResolver<'a> {
    fn resolve(&self, symbol_name: &str) -> Option<u64> {
        // Search all loaded images' export tries
        for loaded in &self.registry.images {
            if let Some(exp) = loaded.exports.iter().find(|e| e.name == symbol_name) {
                if exp.is_reexport() {
                    // Follow re-export chain: look up in the target dylib
                    let reexport_name = exp.reexport_name.as_deref().unwrap_or(symbol_name);
                    if let Some(ordinal) = exp.reexport_ordinal {
                        if let Some(addr) =
                            self.resolve_reexport(&loaded.parsed, ordinal, reexport_name)
                        {
                            return Some(addr);
                        }
                    }
                    // If chain resolution failed, fall through to other images
                    continue;
                }
                // Export addresses are relative to the image base (vmaddr 0).
                // Add slide to get the actual in-process address.
                let addr = (exp.address as i64 + loaded.image.slide) as u64;
                return Some(addr);
            }
        }
        // Fall back to system symbols
        self.fallback.resolve(symbol_name)
    }
}

impl<'a> RegistryResolver<'a> {
    /// Follow a re-export chain by looking up the symbol in the dylib
    /// identified by the ordinal from the exporting image's dylib table.
    fn resolve_reexport(
        &self,
        source_parsed: &ParsedMachO,
        ordinal: u64,
        symbol_name: &str,
    ) -> Option<u64> {
        // Ordinals are 1-based in the dylib list
        let idx = ordinal.checked_sub(1)? as usize;
        let dylib = source_parsed.dylibs.get(idx)?;
        let install_name = &dylib.name;

        // Look up the target image in the registry
        let target = self.registry.get(install_name)?;
        if let Some(exp) = target.exports.iter().find(|e| e.name == symbol_name) {
            if exp.is_reexport() {
                // Recursive re-export (e.g. libSystem → libpthread → ...)
                let next_name = exp.reexport_name.as_deref().unwrap_or(symbol_name);
                if let Some(ord) = exp.reexport_ordinal {
                    return self.resolve_reexport(&target.parsed, ord, next_name);
                }
                return None;
            }
            let addr = (exp.address as i64 + target.image.slide) as u64;
            return Some(addr);
        }
        None
    }
}

/// Apply classic bind opcodes to a mapped image using a SymbolResolver.
///
/// This is used by `apply_all_fixups` for images that use LC_DYLD_INFO(_ONLY)
/// instead of LC_DYLD_CHAINED_FIXUPS.
fn apply_classic_binds_with_resolver(
    image: &crate::MappedImage,
    fixups: &macho_parse::Fixups,
    resolver: &dyn SymbolResolver,
) -> Result<usize, ExecError> {
    let mut count = 0;

    for bind in fixups.binds.iter().chain(fixups.lazy_binds.iter()) {
        let seg =
            image
                .segments
                .get(bind.segment_index)
                .ok_or(ExecError::FixupSegmentOutOfRange {
                    index: bind.segment_index,
                    count: image.segments.len(),
                })?;

        if bind.segment_offset + 8 > seg.size as u64 {
            return Err(ExecError::FixupOffsetOutOfBounds {
                segment: seg.name.clone(),
                offset: bind.segment_offset,
                size: seg.size as u64,
            });
        }

        if let Some(addr) = resolver.resolve(&bind.symbol_name) {
            let final_addr = (addr as i64 + bind.addend) as u64;
            unsafe {
                let ptr = seg.base.add(bind.segment_offset as usize) as *mut u64;
                ptr.write_unaligned(final_addr);
            }
            count += 1;
        } else if bind.is_weak {
            // Weak import not found — null-fill
            unsafe {
                let ptr = seg.base.add(bind.segment_offset as usize) as *mut u64;
                ptr.write_unaligned(0);
            }
            count += 1;
        }
    }

    Ok(count)
}
