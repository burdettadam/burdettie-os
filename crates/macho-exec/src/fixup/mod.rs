//! Fixup application: rebases, symbol binds, and chained fixup walking.

mod chained;
#[cfg(any(test, all(target_arch = "aarch64", not(target_os = "macos"))))]
pub mod ctor_thunk;
pub mod data_symbols;
mod resolver;

use macho_parse::Fixups;
use macho_resolve::BindingReport;

use crate::{ExecError, MappedImage};

pub use chained::apply_chained_fixups;
pub(crate) use chained::note_unresolved_bind;
pub use chained::unresolved_bind_trap_addr;
pub use resolver::{StubResolver, SymbolResolver};

/// Apply rebase fixups: add slide to each rebase location.
pub fn apply_rebases(image: &MappedImage, fixups: &Fixups) -> Result<usize, ExecError> {
    let mut count = 0;

    for rebase in &fixups.rebases {
        let seg =
            image
                .segments
                .get(rebase.segment_index)
                .ok_or(ExecError::FixupSegmentOutOfRange {
                    index: rebase.segment_index,
                    count: image.segments.len(),
                })?;

        if rebase.segment_offset + 8 > seg.size as u64 {
            return Err(ExecError::FixupOffsetOutOfBounds {
                segment: seg.name.clone(),
                offset: rebase.segment_offset,
                size: seg.size as u64,
            });
        }

        // SAFETY: seg.base + offset is within the mapped segment bounds.
        unsafe {
            let ptr = seg.base.add(rebase.segment_offset as usize) as *mut u64;
            let current = ptr.read_unaligned();
            let rebased = (current as i64 + image.slide) as u64;
            ptr.write_unaligned(rebased);
        }
        count += 1;
    }

    Ok(count)
}

/// Apply bind fixups: write resolved addresses into GOT slots.
pub fn apply_binds(
    image: &MappedImage,
    fixups: &Fixups,
    bindings: &BindingReport,
) -> Result<usize, ExecError> {
    let mut count = 0;

    for bind in fixups
        .binds
        .iter()
        .chain(fixups.lazy_binds.iter())
        .chain(fixups.weak_binds.iter())
    {
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

        if let Some(resolved_addr) = resolve_bind_address(&bind.symbol_name, bindings) {
            let final_addr = (resolved_addr as i64 + bind.addend) as u64;
            unsafe {
                let ptr = seg.base.add(bind.segment_offset as usize) as *mut u64;
                ptr.write_unaligned(final_addr);
            }
            count += 1;
        } else if bind.is_lazy {
            log::warn!(
                "unresolved lazy bind: {} (segment={} offset={:#x}) — stub will call dyld_stub_binder → 0 → crash",
                bind.symbol_name,
                bind.segment_index,
                bind.segment_offset
            );
        } else if bind.is_weak {
            log::debug!(
                "unresolved weak bind: {} — expected, will be NULL",
                bind.symbol_name
            );
        } else {
            log::error!(
                "unresolved non-lazy bind: {} (segment={} offset={:#x}) — GOT entry left as-is",
                bind.symbol_name,
                bind.segment_index,
                bind.segment_offset
            );
        }
    }

    Ok(count)
}

fn resolve_bind_address(symbol_name: &str, bindings: &BindingReport) -> Option<u64> {
    bindings
        .bindings
        .iter()
        .find(|b| b.name == symbol_name && b.bound)
        .and_then(|b| b.address)
}
