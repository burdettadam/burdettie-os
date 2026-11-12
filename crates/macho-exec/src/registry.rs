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
use platform_core::unwind::UnwindRegistry;

use crate::{ExecError, MappedImage, SymbolResolver};

/// Normalize an install name by collapsing `.` and `..` path segments
/// in `@rpath/`, `@executable_path/`, or `@loader_path/` prefixed paths.
///
/// Example: `@rpath/XcodeDefault/../lib_Foo.dylib` → `@rpath/lib_Foo.dylib`
pub fn normalize_install_name(name: &str) -> String {
    let (prefix, rest) = if let Some(r) = name.strip_prefix("@rpath/") {
        ("@rpath/", r)
    } else if let Some(r) = name.strip_prefix("@executable_path/") {
        ("@executable_path/", r)
    } else if let Some(r) = name.strip_prefix("@loader_path/") {
        ("@loader_path/", r)
    } else {
        return name.to_string();
    };

    if !rest.contains("..") && !rest.contains("./") {
        return name.to_string();
    }

    let mut parts: Vec<&str> = Vec::new();
    for seg in rest.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(seg),
        }
    }
    format!("{prefix}{}", parts.join("/"))
}

/// A single Mach-O image that has been parsed and mapped into process memory.
pub struct LoadedImage {
    pub image: MappedImage,
    pub parsed: Arc<ParsedMachO>,
    pub data: Arc<[u8]>,
    pub install_name: String,
    pub exports: Vec<ExportedSymbol>,
}

/// Registry of mapped images with cross-image symbol resolution.
pub struct ImageRegistry {
    images: Vec<LoadedImage>,
    name_index: HashMap<String, usize>,
    generated_unwind_frames: HashMap<usize, Vec<u8>>,
    mapper: Arc<dyn MemoryMapper>,
    unwind: Arc<dyn UnwindRegistry>,
}

impl ImageRegistry {
    pub fn new(mapper: Arc<dyn MemoryMapper>, unwind: Arc<dyn UnwindRegistry>) -> Self {
        Self {
            images: Vec::new(),
            name_index: HashMap::new(),
            generated_unwind_frames: HashMap::new(),
            mapper,
            unwind,
        }
    }

    /// Parse, map, and register a Mach-O binary under the given install name.
    ///
    /// Returns the index of the newly loaded image.
    pub fn load(&mut self, install_name: &str, data: Vec<u8>) -> Result<usize, ExecError> {
        let parsed = parse(&data).map_err(|e| ExecError::SecurityDenied {
            reason: format!("parse failed: {e}"),
        })?;
        self.load_preparsed(install_name, Arc::new(parsed), Arc::from(data))
    }

    /// Map and register a Mach-O binary whose parse results are already available.
    ///
    /// `parsed` must correspond to `data`.
    pub fn load_preparsed(
        &mut self,
        install_name: &str,
        parsed: Arc<ParsedMachO>,
        data: Arc<[u8]>,
    ) -> Result<usize, ExecError> {
        let normalized = normalize_install_name(install_name);
        let plan = MappingPlan::from_parsed(&parsed);
        let image = MappedImage::map(&plan, &data, self.mapper.clone(), self.unwind.clone())?;

        let exports = extract_exports(&parsed, &data);

        let idx = self.images.len();
        self.name_index.insert(normalized.clone(), idx);
        self.images.push(LoadedImage {
            image,
            parsed,
            data,
            install_name: normalized,
            exports,
        });
        Ok(idx)
    }

    /// Look up a loaded image by its install name.
    ///
    /// The name is normalized (collapsing `..` segments) before lookup,
    /// so `@rpath/XcodeDefault/../lib_Foo.dylib` matches `@rpath/lib_Foo.dylib`.
    pub fn get(&self, install_name: &str) -> Option<&LoadedImage> {
        let normalized = normalize_install_name(install_name);
        self.name_index.get(&normalized).map(|&i| &self.images[i])
    }

    /// All loaded images, in load order.
    pub fn images(&self) -> &[LoadedImage] {
        &self.images
    }

    /// Apply fixups (chained or classic rebases/binds) to every loaded image,
    /// resolving cross-image symbols via export tries and falling back to
    /// the provided system resolver.
    pub fn apply_all_fixups(
        &mut self,
        system_resolver: &dyn SymbolResolver,
    ) -> Result<(), ExecError> {
        let composite = RegistryResolver {
            registry: self,
            fallback: system_resolver,
        };
        for loaded in &self.images {
            let raw_fixups = extract_fixups(&loaded.parsed, &loaded.data);
            let fixups =
                normalize_classic_fixups_for_mapping(&loaded.parsed, &loaded.image, &raw_fixups);
            match &fixups.model {
                FixupModel::Chained { .. } => {
                    let (rebase_count, bind_count) = crate::apply_chained_fixups(
                        &loaded.image,
                        &loaded.parsed,
                        &loaded.data,
                        &composite,
                    )?;
                    log::debug!(
                        "chained fixups for {}: {} rebases, {} binds",
                        loaded.install_name,
                        rebase_count,
                        bind_count
                    );
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

        self.generated_unwind_frames.clear();
        for loaded in &self.images {
            let generated = crate::unwind::synthesize_runtime_eh_frame(
                &loaded.parsed.segments,
                loaded.image.slide,
            );

            let key = loaded
                .image
                .segments
                .iter()
                .find(|seg| seg.name == "__TEXT")
                .map(|seg| seg.base as usize)
                .or_else(|| {
                    loaded
                        .image
                        .segments
                        .iter()
                        .find(|seg| seg.name == "__TEXT_EXEC")
                        .map(|seg| seg.base as usize)
                });

            let generated_ref = if let (Some(text_start), Some(buf)) = (key, generated) {
                self.generated_unwind_frames.insert(text_start, buf);
                self.generated_unwind_frames
                    .get(&text_start)
                    .map(Vec::as_slice)
            } else {
                None
            };

            crate::unwind::register_unwind_info(
                &loaded.parsed.segments,
                loaded.image.slide,
                &*self.unwind,
                generated_ref,
            );
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
                // Export addresses are offsets from the Mach-O header
                // (preferred load address). Compute the in-memory address:
                //   preferred_base + offset + slide
                let preferred_base = loaded.image.preferred_base();
                let addr =
                    (preferred_base.wrapping_add(exp.address) as i64 + loaded.image.slide) as u64;
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
            let preferred_base = target.image.preferred_base();
            let addr =
                (preferred_base.wrapping_add(exp.address) as i64 + target.image.slide) as u64;
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
        let Some(seg) = image.segments.get(bind.segment_index) else {
            if bind.is_lazy {
                log::debug!(
                    "skipping lazy bind '{}' with out-of-range segment {}",
                    bind.symbol_name,
                    bind.segment_index
                );
                continue;
            }
            return Err(ExecError::FixupSegmentOutOfRange {
                index: bind.segment_index,
                count: image.segments.len(),
            });
        };

        if bind.segment_offset + 8 > seg.size as u64 {
            if bind.is_lazy {
                log::debug!(
                    "skipping lazy bind '{}' with out-of-bounds offset {:#x} in {} (size {:#x})",
                    bind.symbol_name,
                    bind.segment_offset,
                    seg.name,
                    seg.size
                );
                continue;
            }
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
        } else {
            // Unresolved strong bind — install trap so calling through
            // this slot lands in a diagnostic handler instead of crashing
            // on raw fixup data.
            log::debug!(
                "classic bind: unresolved '{}', installing trap",
                bind.symbol_name
            );
            crate::fixup::note_unresolved_bind();
            unsafe {
                let ptr = seg.base.add(bind.segment_offset as usize) as *mut u64;
                ptr.write_unaligned(crate::fixup::unresolved_bind_trap_addr());
            }
            count += 1;
        }
    }

    Ok(count)
}

fn normalize_classic_fixups_for_mapping(
    parsed: &macho_parse::ParsedMachO,
    image: &crate::MappedImage,
    fixups: &macho_parse::Fixups,
) -> macho_parse::Fixups {
    if !matches!(fixups.model, FixupModel::Classic) {
        return fixups.clone();
    }

    let parsed_has_pagezero = parsed
        .segments
        .first()
        .is_some_and(|segment| segment.name == "__PAGEZERO");
    let mapped_has_pagezero = image
        .segments
        .first()
        .is_some_and(|segment| segment.name == "__PAGEZERO");

    if !parsed_has_pagezero || mapped_has_pagezero {
        return fixups.clone();
    }

    let mut adjusted = fixups.clone();
    for rebase in &mut adjusted.rebases {
        rebase.segment_index = classic_segment_index_for_mapping(rebase.segment_index);
    }
    for bind in &mut adjusted.binds {
        bind.segment_index = classic_segment_index_for_mapping(bind.segment_index);
    }
    for bind in &mut adjusted.weak_binds {
        bind.segment_index = classic_segment_index_for_mapping(bind.segment_index);
    }
    for bind in &mut adjusted.lazy_binds {
        bind.segment_index = classic_segment_index_for_mapping(bind.segment_index);
    }
    adjusted
}

fn classic_segment_index_for_mapping(segment_index: usize) -> usize {
    segment_index.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_core::unwind::NoopUnwindRegistry;
    use platform_core::{PalError, Protection};

    struct StubMapper;

    impl MemoryMapper for StubMapper {
        fn page_size(&self) -> usize {
            4096
        }
        fn map_anonymous(&self, _size: usize) -> Result<*mut u8, PalError> {
            Err(PalError::Other("stub".into()))
        }
        fn protect(&self, _addr: *mut u8, _size: usize, _prot: Protection) -> Result<(), PalError> {
            Ok(())
        }
        fn unmap(&self, _addr: *mut u8, _size: usize) -> Result<(), PalError> {
            Ok(())
        }
    }

    fn make_registry() -> ImageRegistry {
        ImageRegistry::new(Arc::new(StubMapper), Arc::new(NoopUnwindRegistry))
    }

    #[test]
    fn registry_new_is_empty() {
        let reg = make_registry();
        assert!(reg.images().is_empty());
    }

    #[test]
    fn registry_get_unknown_returns_none() {
        let reg = make_registry();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn registry_get_multiple_unknowns() {
        let reg = make_registry();
        assert!(reg.get("/usr/lib/libSystem.B.dylib").is_none());
        assert!(reg.get("").is_none());
    }

    #[test]
    fn normalize_rpath_with_dotdot() {
        assert_eq!(
            normalize_install_name("@rpath/XcodeDefault/../lib_CompilerSwiftSyntax.dylib"),
            "@rpath/lib_CompilerSwiftSyntax.dylib"
        );
    }

    #[test]
    fn normalize_rpath_no_change() {
        assert_eq!(
            normalize_install_name("@rpath/lib_Foo.dylib"),
            "@rpath/lib_Foo.dylib"
        );
    }

    #[test]
    fn normalize_absolute_path_unchanged() {
        assert_eq!(
            normalize_install_name("/usr/lib/libSystem.B.dylib"),
            "/usr/lib/libSystem.B.dylib"
        );
    }

    #[test]
    fn normalize_executable_path_with_dotdot() {
        assert_eq!(
            normalize_install_name("@executable_path/../lib/libFoo.dylib"),
            "@executable_path/lib/libFoo.dylib"
        );
    }

    #[test]
    fn normalize_multiple_dotdot() {
        assert_eq!(
            normalize_install_name("@rpath/a/b/../../lib_Foo.dylib"),
            "@rpath/lib_Foo.dylib"
        );
    }

    #[test]
    fn normalize_dot_segments() {
        assert_eq!(
            normalize_install_name("@rpath/./lib_Foo.dylib"),
            "@rpath/lib_Foo.dylib"
        );
    }

    #[test]
    fn classic_segment_index_for_mapping_skips_pagezero() {
        assert_eq!(classic_segment_index_for_mapping(0), 0);
        assert_eq!(classic_segment_index_for_mapping(1), 0);
        assert_eq!(classic_segment_index_for_mapping(2), 1);
        assert_eq!(classic_segment_index_for_mapping(4), 3);
    }
}
