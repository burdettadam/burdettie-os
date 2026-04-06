//! Chain walking logic for chained fixups.

use super::resolve_and_write_bind;
use super::resolve_bind_addr;
use super::ChainedImport;
use crate::ptrauth::{pac_sign_pointer, AuthMetadata};
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
                log::trace!(
                    "arm64e chain at +{offset:#x}: raw={raw:#018x}, is_auth={is_auth}, is_bind={is_bind}"
                );

                if is_bind {
                    let (ordinal, next) = if pointer_format == 13 {
                        let ord = (raw & 0x00FFFFFF) as usize;
                        let n = ((raw >> 24) & 0x7FF) as usize;
                        (ord, n)
                    } else {
                        let ord = (raw & 0xFFFF) as usize;
                        // next is always bits 51-61 for formats 1/12
                        let n = ((raw >> 51) & 0x7FF) as usize;
                        (ord, n)
                    };
                    log::trace!("  bind: ordinal={ordinal}, next={next} (stride={stride})");

                    if is_auth {
                        let meta = AuthMetadata::from_auth_bind(raw);
                        log::trace!(
                            "arm64e auth-bind at +{offset:#x}: ordinal={ordinal}, key={}, addrDiv={}, diversity={:#06x}",
                            meta.key, meta.addr_diversity, meta.diversity
                        );
                        // Auth bind: resolve, PAC-sign, then write.
                        // On failure install the trap address (unsigned).
                        let addr = match resolve_bind_addr(ordinal, 0, imports, resolver) {
                            Some(a) => {
                                let slot_addr = seg.base as u64 + offset as u64;
                                unsafe {
                                    pac_sign_pointer(a, meta.key, slot_addr, meta.diversity, meta.addr_diversity)
                                }
                            }
                            None => {
                                super::UNRESOLVED_BIND_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                // Auth-GOT stubs use BRAA/BLRAA which require a signed pointer.
                                // Sign the trap address so the authenticated branch succeeds and
                                // lands on the trap function instead of faulting on PAC mismatch.
                                let slot_addr = seg.base as u64 + offset as u64;
                                unsafe {
                                    pac_sign_pointer(super::trap_addr(), meta.key, slot_addr, meta.diversity, meta.addr_diversity)
                                }
                            }
                        };
                        unsafe {
                            let ptr = seg.base.add(offset) as *mut u64;
                            ptr.write_unaligned(addr);
                        }
                        binds += 1;
                    } else if resolve_and_write_bind(seg, offset, ordinal, 0, imports, resolver) {
                        binds += 1;
                    }
                    if next == 0 {
                        break;
                    }
                    offset += next * stride;
                } else {
                    // next is always bits 51-61 for formats 1/12
                    let next = if pointer_format == 13 {
                        ((raw >> 24) & 0x7FF) as usize
                    } else {
                        ((raw >> 51) & 0x7FF) as usize
                    };

                    // Format 12/13 (USERLAND): targets are offsets from preferred_base.
                    // Format 1: targets are absolute vmaddrs.
                    let is_userland = pointer_format == 12 || pointer_format == 13;
                    let base_for_rebase = if is_userland { preferred_base } else { 0 };

                    let target = if is_auth {
                        let meta = AuthMetadata::from_auth_entry(raw);
                        let raw_target = base_for_rebase + (raw & 0xFFFFFFFF);
                        log::trace!(
                            "arm64e auth-rebase at +{offset:#x}: target_off={:#x}, resolved={:#x}, key={}, addrDiv={}, diversity={:#06x}",
                            raw & 0xFFFFFFFF, raw_target, meta.key, meta.addr_diversity, meta.diversity
                        );
                        let rebased = (raw_target as i64 + slide) as u64;
                        // Auth rebase: PAC-sign the rebased pointer
                        let slot_addr = seg.base as u64 + offset as u64;
                        unsafe {
                            pac_sign_pointer(rebased, meta.key, slot_addr, meta.diversity, meta.addr_diversity)
                        }
                    } else {
                        let target43 = raw & 0x7FF_FFFF_FFFF;
                        let high8 = (raw >> 43) & 0xFF;
                        let raw_target = base_for_rebase + ((high8 << 56) | target43);
                        (raw_target as i64 + slide) as u64
                    };

                    unsafe {
                        let ptr = seg.base.add(offset) as *mut u64;
                        ptr.write_unaligned(target);
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
                log::trace!(
                    "ptr64_offset chain at +{offset:#x}: raw={raw:#018x}, is_bind={is_bind}, next={next}"
                );

                if is_bind {
                    let ordinal = (raw & 0x00FFFFFF) as usize;
                    let addend = ((raw >> 24) & 0xFF) as u64;
                    log::trace!("  ptr64_offset bind: ordinal={ordinal}, addend={addend}");
                    if resolve_and_write_bind(seg, offset, ordinal, addend, imports, resolver) {
                        binds += 1;
                    }
                } else {
                    let target = raw & 0x0F_FFFF_FFFF;
                    let high8 = (raw >> 36) & 0xFF;
                    let rebased = (preferred_base + (high8 << 56 | target)) as i64 + slide;
                    let rebased_u = rebased as u64;
                    log::trace!(
                        "  ptr64_offset rebase: target={target:#x}, high8={high8:#x}, preferred_base={preferred_base:#x}, slide={slide:#x}, result={rebased_u:#x}"
                    );
                    unsafe {
                        let ptr = seg.base.add(offset) as *mut u64;
                        ptr.write_unaligned(rebased_u);
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
