//! Convert macOS compact unwind info to DWARF `.eh_frame`.

mod constants;
mod emitter;
mod parser;

pub use parser::{
    CompactEntry, generate_eh_frame, generate_eh_frame_runtime, parse_compact_unwind,
};

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::constants::*;
    use super::emitter::{write_uleb128, write_sleb128};

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
            CompactEntry {
                func_offset: 0x1000,
                func_length: 0x80,
                encoding,
                personality_slot_offset: None,
                lsda_offset: None,
            },
            CompactEntry {
                func_offset: 0x1080,
                func_length: 0x40,
                encoding: UNWIND_ARM64_MODE_FRAME,
                personality_slot_offset: None,
                lsda_offset: None,
            },
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
        let entries = vec![CompactEntry {
            func_offset: 0,
            func_length: 0x20,
            encoding,
            personality_slot_offset: None,
            lsda_offset: None,
        }];
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
            CompactEntry {
                func_offset: 0,
                func_length: 0x20,
                encoding: 0,
                personality_slot_offset: None,
                lsda_offset: None,
            },
        ];
        let eh = generate_eh_frame(&entries, 0x4000);
        assert!(eh.is_empty(), "zero-encoding entries should not emit terminator-only eh_frame data");
    }

    #[test]
    fn dwarf_mode_entries_are_skipped_without_emitting_eh_frame() {
        let entries = vec![CompactEntry {
            func_offset: 0,
            func_length: 0x20,
            encoding: UNWIND_ARM64_MODE_DWARF,
            personality_slot_offset: None,
            lsda_offset: None,
        }];

        let eh = generate_eh_frame(&entries, 0x4000);
        assert!(eh.is_empty(), "DWARF-mode entries should not emit terminator-only eh_frame data");
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
