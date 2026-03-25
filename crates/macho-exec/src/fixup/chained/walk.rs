//! Chain walking logic for chained fixups.

use super::resolve_and_write_bind;
use super::ChainedImport;
use crate::ptrauth::AuthMetadata;
use crate::MappedSegment;

use super::super::resolver::SymbolResolver;

pub(super) fn walk_chain(
    seg: &MappedSegment,
    start_offset: usize,
    pointer_format: u16,
    slide: i64,
    preferred_base: u64,
    imports: &[ChainedImport],
    resolver: &dyn SymbolResolver,
) -> (usize, usize) {
    let mut offset = start_offset;
    let mut rebases = 0;
    let mut binds = 0;
    let stride = chain_stride(pointer_format);

    loop {
        if offset + 8 > seg.size {
            break;
        }

        let raw = unsafe {
            let ptr = seg.base.add(offset) as *const u64;
            ptr.read_unaligned()
        };

        match pointer_format {
            // ── ARM64e formats ──────────────────────────────────
            1 | 12 | 13 => {
                let is_auth = (raw >> 63) & 1 == 1;
                let is_bind = (raw >> 62) & 1 == 1;

                if is_bind {
                    let (ordinal, next) = if pointer_format == 13 {
                        let ord = (raw & 0x00FFFFFF) as usize;
                        let n = ((raw >> 24) & 0x7FF) as usize;
                        (ord, n)
                    } else {
                        let ord = (raw & 0xFFFF) as usize;
                        let n = if is_auth {
                            ((raw >> 32) & 0x7FF) as usize
                        } else {
                            ((raw >> 51) & 0x7FF) as usize
                        };
                        (ord, n)
                    };

                    if is_auth {
                        let meta = AuthMetadata::from_auth_bind(raw);
                        log::trace!(
                            "arm64e auth-bind at +{offset:#x}: ordinal={ordinal}, key={}, addrDiv={}, diversity={:#06x}",
                            meta.key, meta.addr_diversity, meta.diversity
                        );
                    }

                    if resolve_and_write_bind(seg, offset, ordinal, 0, imports, resolver) {
                        binds += 1;
                    }
                    if next == 0 {
                        break;
                    }
                    offset += next * stride;
                } else {
                    let next = if is_auth {
                        ((raw >> 32) & 0x7FF) as usize
                    } else {
                        ((raw >> 51) & 0x7FF) as usize
                    };

                    let target = if is_auth {
                        let meta = AuthMetadata::from_auth_entry(raw);
                        log::trace!(
                            "arm64e auth-rebase at +{offset:#x}: target={:#x}, key={}, addrDiv={}, diversity={:#06x}",
                            raw & 0xFFFFFFFF, meta.key, meta.addr_diversity, meta.diversity
                        );
                        (raw & 0xFFFFFFFF) as i64 + slide
                    } else {
                        let target43 = raw & 0x7FF_FFFF_FFFF;
                        let high8 = (raw >> 43) & 0xFF;
                        ((high8 << 56) | target43) as i64 + slide
                    };

                    unsafe {
                        let ptr = seg.base.add(offset) as *mut u64;
                        ptr.write_unaligned(target as u64);
                    }
                    rebases += 1;
                    if next == 0 {
                        break;
                    }
                    offset += next * stride;
                }
            }

            // ── PTR_64_OFFSET (format 6) ────────────────────────
            6 => {
                let is_bind = (raw >> 63) & 1 == 1;
                let next = ((raw >> 51) & 0x0FFF) as usize;

                if is_bind {
                    let ordinal = (raw & 0x00FFFFFF) as usize;
                    if resolve_and_write_bind(seg, offset, ordinal, 0, imports, resolver) {
                        binds += 1;
                    }
                } else {
                    let target = raw & 0x0F_FFFF_FFFF;
                    let high8 = (raw >> 36) & 0xFF;
                    let rebased = (preferred_base + (high8 << 56 | target)) as i64 + slide;
                    unsafe {
                        let ptr = seg.base.add(offset) as *mut u64;
                        ptr.write_unaligned(rebased as u64);
                    }
                    rebases += 1;
                }
                if next == 0 {
                    break;
                }
                offset += next * stride;
            }

            // ── PTR_64 (format 2) and default ───────────────────
            _ => {
                let is_bind = (raw >> 63) & 1 == 1;
                let next = ((raw >> 51) & 0x0FFF) as usize;

                if is_bind {
                    let ordinal = (raw & 0x00FFFFFF) as usize;
                    let addend = (raw >> 24) & 0xFF;
                    if resolve_and_write_bind(seg, offset, ordinal, addend, imports, resolver) {
                        binds += 1;
                    }
                } else {
                    let target = raw & 0x0F_FFFF_FFFF;
                    let high8 = (raw >> 36) & 0xFF;
                    let rebased = ((high8 << 56) | target) as i64 + slide;
                    unsafe {
                        let ptr = seg.base.add(offset) as *mut u64;
                        ptr.write_unaligned(rebased as u64);
                    }
                    rebases += 1;
                }
                if next == 0 {
                    break;
                }
                offset += next * stride;
            }
        }
    }

    (rebases, binds)
}

pub(super) fn chain_stride(pointer_format: u16) -> usize {
    match pointer_format {
        1 => 8,  // DYLD_CHAINED_PTR_ARM64E
        2 => 4,  // DYLD_CHAINED_PTR_64
        6 => 4,  // DYLD_CHAINED_PTR_64_OFFSET
        12 => 8, // DYLD_CHAINED_PTR_ARM64E_USERLAND
        13 => 8, // DYLD_CHAINED_PTR_ARM64E_USERLAND24
        _ => 4,
    }
}
