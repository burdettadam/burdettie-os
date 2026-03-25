//! Chained fixup walking: LC_DYLD_CHAINED_FIXUPS processing.

mod imports;
mod walk;

use macho_parse::ParsedMachO;
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

use super::resolver::SymbolResolver;
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

    let imports = imports::parse_imports(
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
            let (r, b) = walk::walk_chain(
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
        runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::BindFailed {
            ordinal,
            symbol: import.name.clone(),
        });
        false
    }
}

#[cfg(test)]
mod tests {
    use super::walk::chain_stride;

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
        let imports = super::imports::parse_imports(&buf, 0, 0, 32, 1);
        assert!(imports.is_empty());
    }
}
