//! Parser for macOS __unwind_info and the public generate_eh_frame API.

use std::collections::HashMap;

use super::constants::*;
use super::emitter::{
    emit_cie_arm64, emit_fde_arm64,
    read_u16, read_u32,
};

pub struct CompactEntry {
    /// Function start offset relative to `__TEXT` base.
    pub func_offset: u32,
    /// Function length in bytes.
    pub func_length: u32,
    /// Compact encoding value.
    pub encoding: u32,
    /// Slot offset (relative to image base) holding the resolved personality pointer.
    pub personality_slot_offset: Option<u32>,
    /// LSDA offset relative to `__TEXT` base / image base.
    pub lsda_offset: Option<u32>,
}

/// Parse the `__unwind_info` section into a list of per-function entries.
///
/// `data` is the raw bytes of the `__unwind_info` section.
/// Returns entries sorted by `func_offset`.
pub fn parse_compact_unwind(data: &[u8]) -> Vec<CompactEntry> {
    if data.len() < 28 {
        return Vec::new();
    }

    let version = read_u32(data, 0);
    if version != 1 {
        log::warn!("compact_to_ehframe: unsupported version {version}");
        return Vec::new();
    }

    let common_enc_off = read_u32(data, 4) as usize;
    let common_enc_count = read_u32(data, 8) as usize;
    let personality_off = read_u32(data, 12) as usize;
    let personality_count = read_u32(data, 16) as usize;
    let index_off = read_u32(data, 20) as usize;
    let index_count = read_u32(data, 24) as usize;

    // Read common encodings table.
    let common_encodings: Vec<u32> = (0..common_enc_count)
        .filter_map(|i| {
            let off = common_enc_off + i * 4;
            (off + 4 <= data.len()).then(|| read_u32(data, off))
        })
        .collect();

    let personality_offsets: Vec<u32> = (0..personality_count)
        .filter_map(|i| {
            let off = personality_off + i * 4;
            (off + 4 <= data.len()).then(|| read_u32(data, off))
        })
        .collect();

    // Read first-level index entries (each 12 bytes).
    let mut entries = Vec::new();

    for idx in 0..index_count.saturating_sub(1) {
        let ie_off = index_off + idx * 12;
        if ie_off + 12 > data.len() {
            break;
        }
        let _first_func_offset = read_u32(data, ie_off);
        let second_level_off = read_u32(data, ie_off + 4) as usize;
        let lsda_index_off = read_u32(data, ie_off + 8) as usize;

        // Next index entry gives the upper bound function offset.
        let next_ie_off = index_off + (idx + 1) * 12;
        let next_func_offset = if next_ie_off + 4 <= data.len() {
            read_u32(data, next_ie_off)
        } else {
            u32::MAX
        };
        let next_lsda_index_off = if next_ie_off + 12 <= data.len() {
            read_u32(data, next_ie_off + 8) as usize
        } else {
            data.len()
        };

        let lsda_map = parse_lsda_descriptors(
            data,
            lsda_index_off,
            next_lsda_index_off,
            next_func_offset,
        );

        if second_level_off == 0 || second_level_off + 4 > data.len() {
            continue;
        }

        let kind = read_u32(data, second_level_off);
        match kind {
            UNWIND_SECOND_LEVEL_REGULAR => {
                parse_regular_page(
                    data,
                    second_level_off,
                    next_func_offset,
                    &personality_offsets,
                    &lsda_map,
                    &mut entries,
                );
            }
            UNWIND_SECOND_LEVEL_COMPRESSED => {
                parse_compressed_page(
                    data,
                    second_level_off,
                    _first_func_offset,
                    next_func_offset,
                    &common_encodings,
                    &personality_offsets,
                    &lsda_map,
                    &mut entries,
                );
            }
            _ => {
                log::warn!("compact_to_ehframe: unknown page kind {kind}");
            }
        }
    }

    // Sort and compute function lengths (distance to next entry).
    entries.sort_by_key(|e| e.func_offset);
    for i in 0..entries.len() {
        if i + 1 < entries.len() {
            entries[i].func_length = entries[i + 1].func_offset - entries[i].func_offset;
        }
        // Last entry: we don't know the true end; use a conservative default.
        // The caller can clamp to __TEXT size.
    }
    if let Some(last) = entries.last_mut() {
        if last.func_length == 0 {
            last.func_length = 64; // conservative default for last function
        }
    }

    entries
}

/// Generate a complete `.eh_frame` section from compact unwind entries.
///
/// `text_base` is the runtime virtual address where `__TEXT` is mapped.
/// Returns a `Vec<u8>` holding the `.eh_frame` bytes, suitable for
/// `__register_frame`.
pub fn generate_eh_frame(entries: &[CompactEntry], text_base: usize) -> Vec<u8> {
    generate_eh_frame_with_personality_resolver(entries, text_base, |slot_offset| {
        Some(text_base + slot_offset as usize)
    })
}

/// Generate a complete `.eh_frame` section using runtime-resolved personality pointers.
///
/// Unlike [`generate_eh_frame`], this variant dereferences compact-unwind personality
/// slots after fixups have been applied so the emitted CIEs contain callable host
/// personality functions instead of unresolved GOT slot addresses.
pub fn generate_eh_frame_runtime(entries: &[CompactEntry], text_base: usize) -> Vec<u8> {
    generate_eh_frame_with_personality_resolver(entries, text_base, |slot_offset| {
        let slot_addr = text_base + slot_offset as usize;
        let resolved = unsafe { std::ptr::read_unaligned(slot_addr as *const usize) };
        if resolved != 0 {
            Some(resolved)
        } else {
            Some(slot_addr)
        }
    })
}

fn generate_eh_frame_with_personality_resolver<F>(
    entries: &[CompactEntry],
    text_base: usize,
    mut personality_resolver: F,
) -> Vec<u8>
where
    F: FnMut(u32) -> Option<usize>,
{
    let mut buf = Vec::with_capacity(entries.len() * 48 + 64);

    struct ResolvedEntry {
        func_addr: usize,
        func_length: usize,
        encoding: u32,
        personality_addr: Option<usize>,
        lsda_addr: Option<usize>,
    }

    let resolved_entries: Vec<ResolvedEntry> = entries
        .iter()
        .filter_map(|entry| {
        let mode = entry.encoding & UNWIND_ARM64_MODE_MASK;
        if entry.encoding == 0 || entry.func_length == 0 {
                return None; // no unwind info for this function
        }
        // Skip DWARF-mode entries (would need original __eh_frame data).
        if mode == UNWIND_ARM64_MODE_DWARF {
                return None;
        }

            Some(ResolvedEntry {
                func_addr: text_base + entry.func_offset as usize,
                func_length: entry.func_length as usize,
                encoding: entry.encoding,
                personality_addr: entry
                    .personality_slot_offset
                    .and_then(&mut personality_resolver),
                lsda_addr: entry.lsda_offset.map(|off| text_base + off as usize),
            })
        })
        .collect();

    if resolved_entries.is_empty() {
        return Vec::new();
    }

    let mut group_uses_lsda: HashMap<Option<usize>, bool> = HashMap::new();
    for entry in &resolved_entries {
        group_uses_lsda
            .entry(entry.personality_addr)
            .and_modify(|uses_lsda| *uses_lsda |= entry.lsda_addr.is_some())
            .or_insert(entry.lsda_addr.is_some());
    }

    let mut cie_offsets: HashMap<Option<usize>, usize> = HashMap::new();

    // ── CIE/FDEs ───────────────────────────────────────────────────
    for entry in &resolved_entries {
        let cie_start = *cie_offsets.entry(entry.personality_addr).or_insert_with(|| {
            let cie_start = buf.len();
            emit_cie_arm64(
                &mut buf,
                entry.personality_addr,
                *group_uses_lsda.get(&entry.personality_addr).unwrap_or(&false),
            );
            cie_start
        });

        emit_fde_arm64(
            &mut buf,
            cie_start,
            entry.func_addr,
            entry.func_length,
            entry.encoding,
            entry.lsda_addr,
        );
    }

    // ── Terminator ─────────────────────────────────────────────────
    buf.extend_from_slice(&0u32.to_ne_bytes());

    buf
}

// ── Internal: section parsing ──────────────────────────────────────

fn parse_regular_page(
    data: &[u8],
    page_off: usize,
    next_func_offset: u32,
    personality_offsets: &[u32],
    lsda_map: &HashMap<u32, u32>,
    out: &mut Vec<CompactEntry>,
) {
    if page_off + 8 > data.len() {
        return;
    }
    let entry_page_offset = read_u16(data, page_off + 4) as usize;
    let entry_count = read_u16(data, page_off + 6) as usize;

    let base = page_off + entry_page_offset;
    for i in 0..entry_count {
        let off = base + i * 8;
        if off + 8 > data.len() {
            break;
        }
        let func_offset = read_u32(data, off);
        let encoding = read_u32(data, off + 4);

        if func_offset < next_func_offset {
            out.push(CompactEntry {
                func_offset,
                func_length: 0, // filled in later
                encoding,
                personality_slot_offset: personality_slot_offset_for_encoding(
                    encoding,
                    personality_offsets,
                ),
                lsda_offset: lsda_offset_for_entry(func_offset, encoding, lsda_map),
            });
        }
    }
}

fn parse_compressed_page(
    data: &[u8],
    page_off: usize,
    first_level_func_offset: u32,
    next_func_offset: u32,
    common_encodings: &[u32],
    personality_offsets: &[u32],
    lsda_map: &HashMap<u32, u32>,
    out: &mut Vec<CompactEntry>,
) {
    if page_off + 12 > data.len() {
        return;
    }
    let entry_page_offset = read_u16(data, page_off + 4) as usize;
    let entry_count = read_u16(data, page_off + 6) as usize;
    let encodings_page_offset = read_u16(data, page_off + 8) as usize;
    let encodings_count = read_u16(data, page_off + 10) as usize;

    // Local encodings table.
    let local_base = page_off + encodings_page_offset;
    let local_encodings: Vec<u32> = (0..encodings_count)
        .filter_map(|i| {
            let off = local_base + i * 4;
            (off + 4 <= data.len()).then(|| read_u32(data, off))
        })
        .collect();

    let entries_base = page_off + entry_page_offset;
    for i in 0..entry_count {
        let off = entries_base + i * 4;
        if off + 4 > data.len() {
            break;
        }
        let packed = read_u32(data, off);
        let rel_func_offset = packed >> 8; // 24-bit offset relative to first_level_func_offset
        let enc_index = (packed & 0xFF) as usize;

        let func_offset = first_level_func_offset + rel_func_offset;
        let encoding = if enc_index < common_encodings.len() {
            common_encodings[enc_index]
        } else {
            let local_idx = enc_index - common_encodings.len();
            local_encodings.get(local_idx).copied().unwrap_or(0)
        };

        if func_offset < next_func_offset {
            out.push(CompactEntry {
                func_offset,
                func_length: 0, // filled in later
                encoding,
                personality_slot_offset: personality_slot_offset_for_encoding(
                    encoding,
                    personality_offsets,
                ),
                lsda_offset: lsda_offset_for_entry(func_offset, encoding, lsda_map),
            });
        }
    }
}

fn parse_lsda_descriptors(
    data: &[u8],
    start_off: usize,
    end_off: usize,
    next_func_offset: u32,
) -> HashMap<u32, u32> {
    let mut lsda_map = HashMap::new();
    if start_off == 0 || start_off >= data.len() {
        return lsda_map;
    }

    let mut off = start_off;
    let end = end_off.min(data.len());
    while off + 8 <= end {
        let func_offset = read_u32(data, off);
        let lsda_offset = read_u32(data, off + 4);
        if func_offset >= next_func_offset {
            break;
        }
        lsda_map.insert(func_offset, lsda_offset);
        off += 8;
    }
    lsda_map
}

fn personality_slot_offset_for_encoding(encoding: u32, personality_offsets: &[u32]) -> Option<u32> {
    let personality_index =
        ((encoding & UNWIND_PERSONALITY_MASK) >> UNWIND_PERSONALITY_SHIFT) as usize;
    personality_index
        .checked_sub(1)
        .and_then(|idx| personality_offsets.get(idx).copied())
}

fn lsda_offset_for_entry(
    func_offset: u32,
    encoding: u32,
    lsda_map: &HashMap<u32, u32>,
) -> Option<u32> {
    if encoding & UNWIND_HAS_LSDA == 0 {
        return None;
    }
    lsda_map.get(&func_offset).copied()
}

