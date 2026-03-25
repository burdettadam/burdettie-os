//! Convert macOS compact unwind info (`__unwind_info`) to DWARF `.eh_frame`.
//!
//! ARM64 macOS binaries typically ship only `__unwind_info` (compact unwind)
//! without `__eh_frame`.  On Linux the system unwinder requires `.eh_frame`
//! (DWARF CFI).  This module converts one format to the other so that
//! `__register_frame` can be used on non-macOS hosts.

// ── ARM64 compact unwind encoding constants ────────────────────────

const UNWIND_ARM64_MODE_MASK: u32 = 0x0F00_0000;
const UNWIND_ARM64_MODE_FRAMELESS: u32 = 0x0200_0000;
#[allow(dead_code)]
const UNWIND_ARM64_MODE_DWARF: u32 = 0x0300_0000;
const UNWIND_ARM64_MODE_FRAME: u32 = 0x0400_0000;

const UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK: u32 = 0x00FFF_000;

/// Register-pair bit flags in the low 12 bits of a FRAME-mode encoding.
const FRAME_REG_PAIRS: &[(u32, u8, u8)] = &[
    // (bit_flag, low_dwarf_reg, high_dwarf_reg)
    (0x0000_0001, 19, 20), // X19/X20
    (0x0000_0002, 21, 22), // X21/X22
    (0x0000_0004, 23, 24), // X23/X24
    (0x0000_0008, 25, 26), // X25/X26
    (0x0000_0010, 27, 28), // X27/X28
    (0x0000_0100, 72, 73), // D8/D9   (DWARF reg = 64 + Dn)
    (0x0000_0200, 74, 75), // D10/D11
    (0x0000_0400, 76, 77), // D12/D13
    (0x0000_0800, 78, 79), // D14/D15
];

// ── Compact unwind section format constants ────────────────────────

const UNWIND_SECOND_LEVEL_REGULAR: u32 = 2;
const UNWIND_SECOND_LEVEL_COMPRESSED: u32 = 3;

// ── DWARF ARM64 register numbers ───────────────────────────────────

const DW_REG_FP: u64 = 29;
const DW_REG_LR: u64 = 30;
const DW_REG_SP: u64 = 31;

// ── DWARF CFA opcodes ─────────────────────────────────────────────

const DW_CFA_NOP: u8 = 0x00;
const DW_CFA_DEF_CFA: u8 = 0x0c;

// ── Public API ─────────────────────────────────────────────────────

/// Parsed compact unwind entry.
#[derive(Debug, Clone)]
pub struct CompactEntry {
    /// Function start offset relative to `__TEXT` base.
    pub func_offset: u32,
    /// Function length in bytes.
    pub func_length: u32,
    /// Compact encoding value.
    pub encoding: u32,
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
    let index_off = read_u32(data, 20) as usize;
    let index_count = read_u32(data, 24) as usize;

    // Read common encodings table.
    let common_encodings: Vec<u32> = (0..common_enc_count)
        .filter_map(|i| {
            let off = common_enc_off + i * 4;
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

        // Next index entry gives the upper bound function offset.
        let next_ie_off = index_off + (idx + 1) * 12;
        let next_func_offset = if next_ie_off + 4 <= data.len() {
            read_u32(data, next_ie_off)
        } else {
            u32::MAX
        };

        if second_level_off == 0 || second_level_off + 4 > data.len() {
            continue;
        }

        let kind = read_u32(data, second_level_off);
        match kind {
            UNWIND_SECOND_LEVEL_REGULAR => {
                parse_regular_page(data, second_level_off, next_func_offset, &mut entries);
            }
            UNWIND_SECOND_LEVEL_COMPRESSED => {
                parse_compressed_page(
                    data,
                    second_level_off,
                    _first_func_offset,
                    next_func_offset,
                    &common_encodings,
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
    let mut buf = Vec::with_capacity(entries.len() * 48 + 64);

    // ── CIE ────────────────────────────────────────────────────────
    let cie_start = buf.len();
    emit_cie_arm64(&mut buf);

    // ── FDEs ───────────────────────────────────────────────────────
    for entry in entries {
        let mode = entry.encoding & UNWIND_ARM64_MODE_MASK;
        if entry.encoding == 0 || entry.func_length == 0 {
            continue; // no unwind info for this function
        }
        // Skip DWARF-mode entries (would need original __eh_frame data).
        if mode == UNWIND_ARM64_MODE_DWARF {
            continue;
        }
        let func_addr = text_base + entry.func_offset as usize;
        emit_fde_arm64(&mut buf, cie_start, func_addr, entry.func_length as usize, entry.encoding);
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
            });
        }
    }
}

// ── Internal: .eh_frame generation ─────────────────────────────────

/// Emit a CIE for ARM64 into `buf`.
fn emit_cie_arm64(buf: &mut Vec<u8>) {
    let cie_start = buf.len();
    buf.extend_from_slice(&0u32.to_ne_bytes()); // placeholder for length

    buf.extend_from_slice(&0u32.to_ne_bytes()); // CIE id = 0
    buf.push(1); // version
    buf.extend_from_slice(b"zR\0"); // augmentation string

    write_uleb128(buf, 4); // code_alignment_factor (ARM64 = 4-byte instrs)
    write_sleb128(buf, -8); // data_alignment_factor
    write_uleb128(buf, DW_REG_LR); // return_address_register = x30

    write_uleb128(buf, 1); // augmentation data length
    buf.push(0x00); // DW_EH_PE_absptr — FDE addresses are absolute pointers

    // Initial instructions: DW_CFA_def_cfa SP, 0
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_SP);
    write_uleb128(buf, 0);

    // Pad to pointer-size alignment.
    while (buf.len() - cie_start - 4) % std::mem::size_of::<usize>() != 0 {
        buf.push(DW_CFA_NOP);
    }

    // Patch length (excludes the 4-byte length field itself).
    let length = (buf.len() - cie_start - 4) as u32;
    buf[cie_start..cie_start + 4].copy_from_slice(&length.to_ne_bytes());
}

/// Emit one FDE for an ARM64 function.
fn emit_fde_arm64(
    buf: &mut Vec<u8>,
    cie_start: usize,
    func_addr: usize,
    func_length: usize,
    encoding: u32,
) {
    let fde_start = buf.len();
    buf.extend_from_slice(&0u32.to_ne_bytes()); // placeholder for length

    // CIE pointer: byte offset from *this field* back to CIE start.
    let cie_ptr = (fde_start + 4 - cie_start) as u32;
    buf.extend_from_slice(&cie_ptr.to_ne_bytes());

    // initial_location (absolute, 8 bytes for DW_EH_PE_absptr on 64-bit).
    buf.extend_from_slice(&(func_addr as u64).to_ne_bytes());

    // address_range.
    buf.extend_from_slice(&(func_length as u64).to_ne_bytes());

    // Augmentation data length (for 'z' augmentation — no LSDA pointer).
    write_uleb128(buf, 0);

    // ── CFA instructions based on compact encoding ─────────────────
    let mode = encoding & UNWIND_ARM64_MODE_MASK;
    match mode {
        UNWIND_ARM64_MODE_FRAME => emit_frame_cfa(buf, encoding),
        UNWIND_ARM64_MODE_FRAMELESS => emit_frameless_cfa(buf, encoding),
        _ => {} // unknown / DWARF mode — no-op (shouldn't reach here)
    }

    // Pad to pointer-size alignment.
    while (buf.len() - fde_start - 4) % std::mem::size_of::<usize>() != 0 {
        buf.push(DW_CFA_NOP);
    }

    // Patch length.
    let length = (buf.len() - fde_start - 4) as u32;
    buf[fde_start..fde_start + 4].copy_from_slice(&length.to_ne_bytes());
}

/// Emit CFA instructions for ARM64 FRAME mode.
///
/// Layout: CFA = FP + 16; FP at CFA-16; LR at CFA-8; then saved pairs below FP.
fn emit_frame_cfa(buf: &mut Vec<u8>, encoding: u32) {
    // CFA = FP + 16
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_FP);
    write_uleb128(buf, 16);

    // FP saved at CFA - 16  →  factored offset = 16/8 = 2
    buf.push(0x80 | (DW_REG_FP as u8 & 0x3F));
    write_uleb128(buf, 2);

    // LR saved at CFA - 8  →  factored offset = 8/8 = 1
    buf.push(0x80 | (DW_REG_LR as u8 & 0x3F));
    write_uleb128(buf, 1);

    // Saved register pairs below FP, in order.
    let mut slot = 0u64; // pair index (0 = first pair below FP)
    for &(flag, low_reg, high_reg) in FRAME_REG_PAIRS {
        if encoding & flag != 0 {
            // High register at CFA - (24 + 16*slot) → factored = (24 + 16*slot) / 8
            let high_factored = (24 + 16 * slot) / 8;
            buf.push(0x80 | (high_reg & 0x3F));
            write_uleb128(buf, high_factored);

            // Low register at CFA - (32 + 16*slot) → factored = (32 + 16*slot) / 8
            let low_factored = (32 + 16 * slot) / 8;
            buf.push(0x80 | (low_reg & 0x3F));
            write_uleb128(buf, low_factored);

            slot += 1;
        }
    }
}

/// Emit CFA instructions for ARM64 FRAMELESS mode.
///
/// Leaf function with known stack allocation. CFA = SP + stack_size.
fn emit_frameless_cfa(buf: &mut Vec<u8>, encoding: u32) {
    let stack_size = ((encoding & UNWIND_ARM64_FRAMELESS_STACK_SIZE_MASK) >> 12) * 16;
    buf.push(DW_CFA_DEF_CFA);
    write_uleb128(buf, DW_REG_SP);
    write_uleb128(buf, stack_size as u64);
}

// ── Helpers ────────────────────────────────────────────────────────

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn write_uleb128(buf: &mut Vec<u8>, mut val: u64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val == 0 {
            buf.push(byte);
            return;
        }
        buf.push(byte | 0x80);
    }
}

fn write_sleb128(buf: &mut Vec<u8>, mut val: i64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        let done = (val == 0 && byte & 0x40 == 0) || (val == -1 && byte & 0x40 != 0);
        if done {
            buf.push(byte);
            return;
        }
        buf.push(byte | 0x80);
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal __unwind_info section with one regular second-level page.
    fn build_test_unwind_info(func_entries: &[(u32, u32)]) -> Vec<u8> {
        // Header (28 bytes).
        let mut data = vec![0u8; 28];

        // version = 1
        data[0..4].copy_from_slice(&1u32.to_le_bytes());

        // No common encodings → offset/count = 28, 0.
        data[4..8].copy_from_slice(&28u32.to_le_bytes());
        data[8..12].copy_from_slice(&0u32.to_le_bytes());

        // No personalities → offset/count = 28, 0.
        data[12..16].copy_from_slice(&28u32.to_le_bytes());
        data[16..20].copy_from_slice(&0u32.to_le_bytes());

        // Index starts right after header.
        let index_off = 28u32;
        data[20..24].copy_from_slice(&index_off.to_le_bytes());
        // 2 index entries (1 real + 1 sentinel).
        data[24..28].copy_from_slice(&2u32.to_le_bytes());

        // First-level index entry 0: funcOffset=0, secondLevel=<page>, lsda=0.
        let page_off = 28 + 2 * 12; // after 2 index entries
        let index0 = [0u32.to_le_bytes(), (page_off as u32).to_le_bytes(), 0u32.to_le_bytes()].concat();
        data.extend_from_slice(&index0);

        // Sentinel index entry 1: funcOffset=0xFFFF (upper bound), secondLevel=0, lsda=0.
        let sentinel_func = func_entries.last().map_or(0x1000, |(off, _)| off + 0x100);
        let index1 = [sentinel_func.to_le_bytes(), 0u32.to_le_bytes(), 0u32.to_le_bytes()].concat();
        data.extend_from_slice(&index1);

        // Regular second-level page header: kind=2, entryPageOffset=8, entryCount=N.
        let entry_count = func_entries.len() as u16;
        let mut page = vec![0u8; 8];
        page[0..4].copy_from_slice(&UNWIND_SECOND_LEVEL_REGULAR.to_le_bytes());
        page[4..6].copy_from_slice(&8u16.to_le_bytes()); // entries start 8 bytes into page
        page[6..8].copy_from_slice(&entry_count.to_le_bytes());

        // Regular entries (8 bytes each): funcOffset, encoding.
        for &(func_off, encoding) in func_entries {
            page.extend_from_slice(&func_off.to_le_bytes());
            page.extend_from_slice(&encoding.to_le_bytes());
        }

        data.extend_from_slice(&page);
        data
    }

    #[test]
    fn parse_empty_section() {
        let entries = parse_compact_unwind(&[]);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_single_frame_entry() {
        let encoding = UNWIND_ARM64_MODE_FRAME | 0x01; // X19/X20 saved
        let data = build_test_unwind_info(&[(0x100, encoding)]);
        let entries = parse_compact_unwind(&data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].func_offset, 0x100);
        assert_eq!(entries[0].encoding, encoding);
    }

    #[test]
    fn parse_multiple_entries_compute_lengths() {
        let enc = UNWIND_ARM64_MODE_FRAME;
        let data = build_test_unwind_info(&[(0x100, enc), (0x200, enc), (0x400, enc)]);
        let entries = parse_compact_unwind(&data);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].func_offset, 0x100);
        assert_eq!(entries[0].func_length, 0x100); // 0x200 - 0x100
        assert_eq!(entries[1].func_offset, 0x200);
        assert_eq!(entries[1].func_length, 0x200); // 0x400 - 0x200
        assert_eq!(entries[2].func_offset, 0x400);
        // Last entry gets a conservative default length.
        assert!(entries[2].func_length > 0);
    }

    #[test]
    fn generate_ehframe_has_cie_and_fdes() {
        let encoding = UNWIND_ARM64_MODE_FRAME | 0x03; // X19/X20 + X21/X22
        let entries = vec![
            CompactEntry { func_offset: 0x1000, func_length: 0x80, encoding },
            CompactEntry { func_offset: 0x1080, func_length: 0x40, encoding: UNWIND_ARM64_MODE_FRAME },
        ];
        let text_base = 0x1_0000_0000usize;
        let eh = generate_eh_frame(&entries, text_base);

        // Should have: CIE + 2 FDEs + terminator.
        assert!(!eh.is_empty());

        // First 4 bytes: CIE length (non-zero).
        let cie_len = u32::from_ne_bytes([eh[0], eh[1], eh[2], eh[3]]);
        assert!(cie_len > 0);

        // CIE id at offset 4: should be 0.
        let cie_id = u32::from_ne_bytes([eh[4], eh[5], eh[6], eh[7]]);
        assert_eq!(cie_id, 0);

        // After CIE (at offset 4 + cie_len): first FDE.
        let fde1_off = 4 + cie_len as usize;
        assert!(fde1_off + 8 <= eh.len());
        let fde1_len = u32::from_ne_bytes([eh[fde1_off], eh[fde1_off+1], eh[fde1_off+2], eh[fde1_off+3]]);
        assert!(fde1_len > 0);

        // FDE CIE pointer should be non-zero (offset back to CIE).
        let fde1_cie_ptr = u32::from_ne_bytes([eh[fde1_off+4], eh[fde1_off+5], eh[fde1_off+6], eh[fde1_off+7]]);
        assert!(fde1_cie_ptr > 0);

        // Terminator at end: last 4 bytes should be 0.
        let term_off = eh.len() - 4;
        let term = u32::from_ne_bytes([eh[term_off], eh[term_off+1], eh[term_off+2], eh[term_off+3]]);
        assert_eq!(term, 0, "eh_frame should end with zero terminator");
    }

    #[test]
    fn frameless_encoding_produces_valid_fde() {
        let stack_size_units = 2u32; // 2 * 16 = 32 bytes of stack
        let encoding = UNWIND_ARM64_MODE_FRAMELESS | (stack_size_units << 12);
        let entries = vec![CompactEntry { func_offset: 0, func_length: 0x20, encoding }];
        let eh = generate_eh_frame(&entries, 0x4000);
        assert!(!eh.is_empty());
        // Should have CIE + 1 FDE + terminator.
        let cie_len = u32::from_ne_bytes([eh[0], eh[1], eh[2], eh[3]]);
        let fde_off = 4 + cie_len as usize;
        let fde_len = u32::from_ne_bytes([eh[fde_off], eh[fde_off+1], eh[fde_off+2], eh[fde_off+3]]);
        assert!(fde_len > 0);
    }

    #[test]
    fn zero_encoding_entries_are_skipped() {
        let entries = vec![
            CompactEntry { func_offset: 0, func_length: 0x20, encoding: 0 },
        ];
        let eh = generate_eh_frame(&entries, 0x4000);
        // Should have: CIE + terminator (no FDEs).
        let cie_len = u32::from_ne_bytes([eh[0], eh[1], eh[2], eh[3]]);
        let after_cie = 4 + cie_len as usize;
        // The terminator should immediately follow the CIE.
        let term = u32::from_ne_bytes([eh[after_cie], eh[after_cie+1], eh[after_cie+2], eh[after_cie+3]]);
        assert_eq!(term, 0);
    }

    #[test]
    fn uleb128_encoding() {
        let mut buf = Vec::new();
        write_uleb128(&mut buf, 0);
        assert_eq!(buf, [0x00]);

        buf.clear();
        write_uleb128(&mut buf, 127);
        assert_eq!(buf, [0x7F]);

        buf.clear();
        write_uleb128(&mut buf, 128);
        assert_eq!(buf, [0x80, 0x01]);

        buf.clear();
        write_uleb128(&mut buf, 624485);
        assert_eq!(buf, [0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn sleb128_encoding() {
        let mut buf = Vec::new();
        write_sleb128(&mut buf, 0);
        assert_eq!(buf, [0x00]);

        buf.clear();
        write_sleb128(&mut buf, -8);
        assert_eq!(buf, [0x78]);

        buf.clear();
        write_sleb128(&mut buf, -1);
        assert_eq!(buf, [0x7F]);

        buf.clear();
        write_sleb128(&mut buf, 63);
        assert_eq!(buf, [0x3F]);
    }
}
