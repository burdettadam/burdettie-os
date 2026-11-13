//! Opcode decoders for classic (rebase/bind) and chained fixup formats.

use super::{
    read_cstring, read_sleb128, read_uleb128, BindEntry, ChainedFixup, RebaseEntry,
    BIND_IMMEDIATE_MASK, BIND_OPCODE_ADD_ADDR_ULEB, BIND_OPCODE_DONE, BIND_OPCODE_DO_BIND,
    BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED, BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB,
    BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB, BIND_OPCODE_MASK, BIND_OPCODE_SET_ADDEND_SLEB,
    BIND_OPCODE_SET_DYLIB_ORDINAL_IMM, BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB,
    BIND_OPCODE_SET_DYLIB_SPECIAL_IMM, BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB,
    BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM, BIND_OPCODE_SET_TYPE_IMM, BIND_TYPE_POINTER,
    REBASE_IMMEDIATE_MASK, REBASE_OPCODE_ADD_ADDR_IMM_SCALED, REBASE_OPCODE_ADD_ADDR_ULEB,
    REBASE_OPCODE_DONE, REBASE_OPCODE_DO_REBASE_ADD_ADDR_ULEB, REBASE_OPCODE_DO_REBASE_IMM_TIMES,
    REBASE_OPCODE_DO_REBASE_ULEB_TIMES, REBASE_OPCODE_DO_REBASE_ULEB_TIMES_SKIPPING_ULEB,
    REBASE_OPCODE_MASK, REBASE_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB, REBASE_OPCODE_SET_TYPE_IMM,
    REBASE_TYPE_POINTER,
};

// ── Classic rebase opcode decoder ──────────────────────────────────

/// Parse the rebase opcode stream from LC_DYLD_INFO(_ONLY).
pub fn parse_rebases(data: &[u8], rebase_off: u32, rebase_size: u32) -> Vec<RebaseEntry> {
    if rebase_size == 0 {
        return Vec::new();
    }
    let start = rebase_off as usize;
    let end = start + rebase_size as usize;
    if end > data.len() {
        return Vec::new();
    }

    let stream = &data[start..end];
    let mut pos = 0usize;
    let mut entries = Vec::new();
    let mut seg_index: usize = 0;
    let mut seg_offset: u64 = 0;
    let mut rebase_type: u8 = REBASE_TYPE_POINTER;
    let pointer_size: u64 = 8;

    while pos < stream.len() {
        let byte = stream[pos];
        pos += 1;
        let opcode = byte & REBASE_OPCODE_MASK;
        let immediate = byte & REBASE_IMMEDIATE_MASK;

        match opcode {
            REBASE_OPCODE_DONE => break,
            REBASE_OPCODE_SET_TYPE_IMM => {
                rebase_type = immediate;
            }
            REBASE_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB => {
                seg_index = immediate as usize;
                seg_offset = read_uleb128(stream, &mut pos);
            }
            REBASE_OPCODE_ADD_ADDR_ULEB => {
                seg_offset = seg_offset.wrapping_add(read_uleb128(stream, &mut pos));
            }
            REBASE_OPCODE_ADD_ADDR_IMM_SCALED => {
                seg_offset = seg_offset.wrapping_add(u64::from(immediate) * pointer_size);
            }
            REBASE_OPCODE_DO_REBASE_IMM_TIMES => {
                for _ in 0..immediate {
                    entries.push(RebaseEntry {
                        segment_index: seg_index,
                        segment_offset: seg_offset,
                        rebase_type,
                    });
                    seg_offset = seg_offset.wrapping_add(pointer_size);
                }
            }
            REBASE_OPCODE_DO_REBASE_ULEB_TIMES => {
                let count = read_uleb128(stream, &mut pos);
                for _ in 0..count {
                    entries.push(RebaseEntry {
                        segment_index: seg_index,
                        segment_offset: seg_offset,
                        rebase_type,
                    });
                    seg_offset = seg_offset.wrapping_add(pointer_size);
                }
            }
            REBASE_OPCODE_DO_REBASE_ADD_ADDR_ULEB => {
                entries.push(RebaseEntry {
                    segment_index: seg_index,
                    segment_offset: seg_offset,
                    rebase_type,
                });
                let add = read_uleb128(stream, &mut pos);
                seg_offset = seg_offset.wrapping_add(add + pointer_size);
            }
            REBASE_OPCODE_DO_REBASE_ULEB_TIMES_SKIPPING_ULEB => {
                let count = read_uleb128(stream, &mut pos);
                let skip = read_uleb128(stream, &mut pos);
                for _ in 0..count {
                    entries.push(RebaseEntry {
                        segment_index: seg_index,
                        segment_offset: seg_offset,
                        rebase_type,
                    });
                    seg_offset = seg_offset.wrapping_add(skip + pointer_size);
                }
            }
            other => {
                log::debug!("unknown rebase opcode {:#x} at pos {}", other >> 4, pos);
            }
        }
    }

    entries
}

// ── Classic bind opcode decoder ────────────────────────────────────

/// Parse a bind opcode stream (regular or lazy).
pub fn parse_binds(data: &[u8], bind_off: u32, bind_size: u32, is_lazy: bool) -> Vec<BindEntry> {
    if bind_size == 0 {
        return Vec::new();
    }
    let start = bind_off as usize;
    let end = start + bind_size as usize;
    if end > data.len() {
        return Vec::new();
    }

    let stream = &data[start..end];
    let mut pos = 0usize;
    let mut entries = Vec::new();
    let mut seg_index: usize = 0;
    let mut seg_offset: u64 = 0;
    let mut bind_type: u8 = BIND_TYPE_POINTER;
    let mut dylib_ordinal: i64 = 0;
    let mut symbol_name = String::new();
    let mut addend: i64 = 0;
    let mut is_weak = false;
    let pointer_size: u64 = 8;

    while pos < stream.len() {
        let byte = stream[pos];
        pos += 1;
        let opcode = byte & BIND_OPCODE_MASK;
        let immediate = byte & BIND_IMMEDIATE_MASK;

        match opcode {
            BIND_OPCODE_DONE => {
                if !is_lazy {
                    break;
                }
            }
            BIND_OPCODE_SET_DYLIB_ORDINAL_IMM => {
                dylib_ordinal = i64::from(immediate);
            }
            BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB => {
                dylib_ordinal = read_uleb128(stream, &mut pos) as i64;
            }
            BIND_OPCODE_SET_DYLIB_SPECIAL_IMM => {
                if immediate == 0 {
                    dylib_ordinal = 0;
                } else {
                    dylib_ordinal = i64::from((0xF0 | immediate) as i8);
                }
            }
            BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM => {
                is_weak = (immediate & 0x01) != 0;
                symbol_name = read_cstring(stream, &mut pos);
            }
            BIND_OPCODE_SET_TYPE_IMM => {
                bind_type = immediate;
            }
            BIND_OPCODE_SET_ADDEND_SLEB => {
                addend = read_sleb128(stream, &mut pos);
            }
            BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB => {
                seg_index = immediate as usize;
                seg_offset = read_uleb128(stream, &mut pos);
            }
            BIND_OPCODE_ADD_ADDR_ULEB => {
                seg_offset = seg_offset.wrapping_add(read_uleb128(stream, &mut pos));
            }
            BIND_OPCODE_DO_BIND => {
                entries.push(BindEntry {
                    segment_index: seg_index,
                    segment_offset: seg_offset,
                    bind_type,
                    dylib_ordinal,
                    symbol_name: symbol_name.clone(),
                    addend,
                    is_weak,
                    is_lazy,
                });
                seg_offset = seg_offset.wrapping_add(pointer_size);
            }
            BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB => {
                entries.push(BindEntry {
                    segment_index: seg_index,
                    segment_offset: seg_offset,
                    bind_type,
                    dylib_ordinal,
                    symbol_name: symbol_name.clone(),
                    addend,
                    is_weak,
                    is_lazy,
                });
                let add = read_uleb128(stream, &mut pos);
                seg_offset = seg_offset.wrapping_add(add + pointer_size);
            }
            BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED => {
                entries.push(BindEntry {
                    segment_index: seg_index,
                    segment_offset: seg_offset,
                    bind_type,
                    dylib_ordinal,
                    symbol_name: symbol_name.clone(),
                    addend,
                    is_weak,
                    is_lazy,
                });
                seg_offset = seg_offset.wrapping_add((u64::from(immediate) + 1) * pointer_size);
            }
            BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB => {
                let count = read_uleb128(stream, &mut pos);
                let skip = read_uleb128(stream, &mut pos);
                for _ in 0..count {
                    entries.push(BindEntry {
                        segment_index: seg_index,
                        segment_offset: seg_offset,
                        bind_type,
                        dylib_ordinal,
                        symbol_name: symbol_name.clone(),
                        addend,
                        is_weak,
                        is_lazy,
                    });
                    seg_offset = seg_offset.wrapping_add(skip + pointer_size);
                }
            }
            other => {
                log::debug!("unknown bind opcode {:#x} at pos {}", other >> 4, pos);
            }
        }
    }

    entries
}

// ── Chained fixups decoder ─────────────────────────────────────────

/// Parse the chained fixups header from LC_DYLD_CHAINED_FIXUPS data.
pub fn parse_chained_fixups(
    data: &[u8],
    fixup_off: u32,
    fixup_size: u32,
) -> (Vec<ChainedFixup>, u16) {
    if fixup_size == 0 || (fixup_off as usize + fixup_size as usize) > data.len() {
        return (Vec::new(), 0);
    }

    let base = fixup_off as usize;
    let buf = &data[base..base + fixup_size as usize];

    if buf.len() < 32 {
        return (Vec::new(), 0);
    }

    let _fixups_version = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let starts_offset = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
    let _imports_offset = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let _symbols_offset = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
    let _imports_count = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
    let _imports_format = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);

    if starts_offset + 4 > buf.len() {
        return (Vec::new(), 0);
    }
    let seg_count = u32::from_le_bytes([
        buf[starts_offset],
        buf[starts_offset + 1],
        buf[starts_offset + 2],
        buf[starts_offset + 3],
    ]) as usize;

    let offsets_start = starts_offset + 4;
    if offsets_start + seg_count * 4 > buf.len() {
        return (Vec::new(), 0);
    }

    let mut pointer_format: u16 = 0;
    let mut fixups = Vec::new();

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

        let _seg_size = u32::from_le_bytes([buf[abs], buf[abs + 1], buf[abs + 2], buf[abs + 3]]);
        let page_size = u16::from_le_bytes([buf[abs + 4], buf[abs + 5]]);
        pointer_format = u16::from_le_bytes([buf[abs + 6], buf[abs + 7]]);
        let _segment_offset = u64::from_le_bytes([
            buf[abs + 8],
            buf[abs + 9],
            buf[abs + 10],
            buf[abs + 11],
            buf[abs + 12],
            buf[abs + 13],
            buf[abs + 14],
            buf[abs + 15],
        ]);
        let _max_valid_pointer =
            u32::from_le_bytes([buf[abs + 16], buf[abs + 17], buf[abs + 18], buf[abs + 19]]);
        let page_count = u16::from_le_bytes([buf[abs + 20], buf[abs + 21]]) as usize;

        let page_starts_base = abs + 22;
        if page_starts_base + page_count * 2 > buf.len() {
            continue;
        }

        for page_i in 0..page_count {
            let ps_off = page_starts_base + page_i * 2;
            let page_start = u16::from_le_bytes([buf[ps_off], buf[ps_off + 1]]);

            if page_start == 0xFFFF {
                continue;
            }

            fixups.push(ChainedFixup::Rebase {
                segment_index: seg_i,
                page_offset: u32::from(page_start) + (page_i as u32 * u32::from(page_size)),
                target: 0,
            });
        }
    }

    (fixups, pointer_format)
}
