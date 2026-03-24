//! Chained fixup walking: LC_DYLD_CHAINED_FIXUPS processing.

use macho_parse::ParsedMachO;
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

use super::resolver::SymbolResolver;
use crate::ptrauth::AuthMetadata;
use crate::{ExecError, MappedImage, MappedSegment};

/// A parsed import entry from the chained fixups imports table.
#[derive(Debug)]
struct ChainedImport {
    #[allow(dead_code)]
    lib_ordinal: u8,
    weak: bool,
    name: String,
}

/// Apply chained fixups to a mapped image.
///
/// Returns (rebase_count, bind_count).
pub fn apply_chained_fixups(
    image: &MappedImage,
    parsed: &ParsedMachO,
    binary_data: &[u8],
    resolver: &dyn SymbolResolver,
) -> Result<(usize, usize), ExecError> {
    let (fixup_off, fixup_size) = parsed
        .commands
        .iter()
        .find_map(|cmd| {
            if cmd.cmd == LoadCommandType::DyldChainedFixups {
                if let LoadCommandPayload::LinkEdit {
                    data_off,
                    data_size,
                } = &cmd.payload
                {
                    return Some((*data_off, *data_size));
                }
            }
            None
        })
        .ok_or(ExecError::NoEntryPoint)?;

    let base = fixup_off as usize;
    let end = base + fixup_size as usize;
    if end > binary_data.len() || fixup_size < 32 {
        return Ok((0, 0));
    }

    let buf = &binary_data[base..end];

    // Parse dyld_chained_fixups_header
    let starts_offset = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
    let imports_offset = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    let symbols_offset = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]) as usize;
    let imports_count = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]) as usize;
    let imports_format = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);

    let imports = parse_imports(
        buf,
        imports_offset,
        imports_count,
        symbols_offset,
        imports_format,
    );

    if starts_offset + 4 > buf.len() {
        return Ok((0, 0));
    }
    let seg_count = u32::from_le_bytes([
        buf[starts_offset],
        buf[starts_offset + 1],
        buf[starts_offset + 2],
        buf[starts_offset + 3],
    ]) as usize;

    let offsets_start = starts_offset + 4;
    if offsets_start + seg_count * 4 > buf.len() {
        return Ok((0, 0));
    }

    let mut rebase_count = 0usize;
    let mut bind_count = 0usize;

    let preferred_base = parsed
        .segments
        .iter()
        .find(|s| s.name != "__PAGEZERO")
        .map(|s| s.vmaddr)
        .unwrap_or(0);

    for seg_i in 0..seg_count {
        let off_pos = offsets_start + seg_i * 4;
        let seg_info_offset = u32::from_le_bytes([
            buf[off_pos],
            buf[off_pos + 1],
            buf[off_pos + 2],
            buf[off_pos + 3],
        ]) as usize;

        if seg_info_offset == 0 {
            continue;
        }

        let abs = starts_offset + seg_info_offset;
        if abs + 22 > buf.len() {
            continue;
        }

        let page_size = u16::from_le_bytes([buf[abs + 4], buf[abs + 5]]) as usize;
        let pointer_format = u16::from_le_bytes([buf[abs + 6], buf[abs + 7]]);
        let segment_offset = u64::from_le_bytes([
            buf[abs + 8],
            buf[abs + 9],
            buf[abs + 10],
            buf[abs + 11],
            buf[abs + 12],
            buf[abs + 13],
            buf[abs + 14],
            buf[abs + 15],
        ]);
        let page_count = u16::from_le_bytes([buf[abs + 20], buf[abs + 21]]) as usize;

        let page_starts_base = abs + 22;
        if page_starts_base + page_count * 2 > buf.len() {
            continue;
        }

        let mapped_seg = find_mapped_segment_for_chain(image, parsed, seg_i, segment_offset);
        let Some(mapped_seg) = mapped_seg else {
            continue;
        };

        for page_i in 0..page_count {
            let ps_off = page_starts_base + page_i * 2;
            let page_start = u16::from_le_bytes([buf[ps_off], buf[ps_off + 1]]);
            if page_start == 0xFFFF {
                continue;
            }

            let chain_base = page_i * page_size + page_start as usize;
            let (r, b) = walk_chain(
                mapped_seg,
                chain_base,
                pointer_format,
                image.slide,
                preferred_base,
                &imports,
                resolver,
            );
            rebase_count += r;
            bind_count += b;
        }
    }

    Ok((rebase_count, bind_count))
}

fn find_mapped_segment_for_chain<'a>(
    image: &'a MappedImage,
    parsed: &ParsedMachO,
    seg_index: usize,
    _segment_offset: u64,
) -> Option<&'a MappedSegment> {
    let seg = parsed.segments.get(seg_index)?;
    if seg.name == "__PAGEZERO" {
        return None;
    }
    image.segment(&seg.name)
}

/// Resolve a chained bind import and write the address, returning true if bound.
fn resolve_and_write_bind(
    seg: &MappedSegment,
    offset: usize,
    ordinal: usize,
    addend: u64,
    imports: &[ChainedImport],
    resolver: &dyn SymbolResolver,
) -> bool {
    let Some(import) = imports.get(ordinal) else {
        log::warn!(
            "chained bind: ordinal {ordinal} out of range (have {})",
            imports.len()
        );
        return false;
    };

    if let Some(addr) = resolver.resolve(&import.name) {
        unsafe {
            let ptr = seg.base.add(offset) as *mut u64;
            ptr.write_unaligned(addr + addend);
        }
        true
    } else if import.weak {
        unsafe {
            let ptr = seg.base.add(offset) as *mut u64;
            ptr.write_unaligned(0);
        }
        true
    } else {
        log::debug!(
            "chained bind: could not resolve import #{ordinal} '{}'",
            import.name
        );
        false
    }
}

fn walk_chain(
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

fn chain_stride(pointer_format: u16) -> usize {
    match pointer_format {
        1 => 8,  // DYLD_CHAINED_PTR_ARM64E
        2 => 4,  // DYLD_CHAINED_PTR_64
        6 => 4,  // DYLD_CHAINED_PTR_64_OFFSET
        12 => 8, // DYLD_CHAINED_PTR_ARM64E_USERLAND
        13 => 8, // DYLD_CHAINED_PTR_ARM64E_USERLAND24
        _ => 4,
    }
}

const DYLD_CHAINED_IMPORT_ADDEND: u32 = 2;
const DYLD_CHAINED_IMPORT_ADDEND64: u32 = 3;

fn parse_imports(
    buf: &[u8],
    imports_offset: usize,
    imports_count: usize,
    symbols_offset: usize,
    imports_format: u32,
) -> Vec<ChainedImport> {
    let mut imports = Vec::with_capacity(imports_count);

    let entry_size = match imports_format {
        DYLD_CHAINED_IMPORT_ADDEND => 8,
        DYLD_CHAINED_IMPORT_ADDEND64 => 16,
        _ => 4,
    };

    for i in 0..imports_count {
        let pos = imports_offset + i * entry_size;
        if pos + entry_size > buf.len() {
            break;
        }

        let (lib_ordinal, weak, name_offset) = match imports_format {
            DYLD_CHAINED_IMPORT_ADDEND64 => {
                let lib = u16::from_le_bytes([buf[pos], buf[pos + 1]]) as u8;
                let flags = u16::from_le_bytes([buf[pos + 2], buf[pos + 3]]);
                let w = (flags & 1) != 0;
                let noff =
                    u32::from_le_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]])
                        as usize;
                (lib, w, noff)
            }
            _ => {
                let raw = u32::from_le_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]);
                let lib = (raw & 0xFF) as u8;
                let w = ((raw >> 8) & 1) != 0;
                let noff = (raw >> 9) as usize;
                (lib, w, noff)
            }
        };

        let name = read_cstring_at(buf, symbols_offset + name_offset);
        imports.push(ChainedImport {
            lib_ordinal,
            weak,
            name,
        });
    }

    imports
}

fn read_cstring_at(buf: &[u8], offset: usize) -> String {
    if offset >= buf.len() {
        return String::new();
    }
    let end = buf[offset..]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(buf.len() - offset);
    String::from_utf8_lossy(&buf[offset..offset + end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_stride_values() {
        assert_eq!(chain_stride(1), 8);
        assert_eq!(chain_stride(2), 4);
        assert_eq!(chain_stride(6), 4);
        assert_eq!(chain_stride(12), 8);
    }

    #[test]
    fn parse_imports_empty() {
        let buf = vec![0u8; 64];
        let imports = parse_imports(&buf, 0, 0, 32, 1);
        assert!(imports.is_empty());
    }
}
