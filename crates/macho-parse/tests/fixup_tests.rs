//! Tests for fixup parsing: rebase opcodes, bind opcodes, chained fixups, export trie.
//!
//! Sources:
//!   - Apple mach-o/loader.h: opcode definitions
//!   - LLVM lld: rebase/bind opcode emission tests
//!   - dyld: chained fixup format tests
#![allow(clippy::identity_op, clippy::vec_init_then_push)] // binary format clarity

#[cfg(test)]
mod rebase_tests {
    use macho_parse::fixups::*;

    #[test]
    fn empty_rebase_stream() {
        let entries = parse_rebases(&[], 0, 0);
        assert!(entries.is_empty());
    }

    #[test]
    fn rebase_done_opcode() {
        // Just a DONE opcode
        let data = vec![0x00]; // REBASE_OPCODE_DONE
        let entries = parse_rebases(&data, 0, 1);
        assert!(entries.is_empty());
    }

    #[test]
    fn rebase_set_type_and_do_rebase_imm_times() {
        // SET_TYPE_IMM(1=pointer), SET_SEGMENT_AND_OFFSET_ULEB(seg=2, off=0x10), DO_REBASE_IMM_TIMES(3)
        let mut data = Vec::new();
        data.push(0x10 | 1); // SET_TYPE_IMM, type=pointer
        data.push(0x20 | 2); // SET_SEGMENT_AND_OFFSET_ULEB, seg=2
        data.push(0x10); // offset=0x10 (ULEB128)
        data.push(0x50 | 3); // DO_REBASE_IMM_TIMES, count=3
        data.push(0x00); // DONE

        let entries = parse_rebases(&data, 0, data.len() as u32);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].segment_index, 2);
        assert_eq!(entries[0].segment_offset, 0x10);
        assert_eq!(entries[1].segment_offset, 0x18); // +8 (pointer size)
        assert_eq!(entries[2].segment_offset, 0x20); // +8
    }

    #[test]
    fn rebase_add_addr_imm_scaled() {
        let mut data = Vec::new();
        data.push(0x10 | 1); // SET_TYPE_IMM, type=pointer
        data.push(0x20 | 1); // SET_SEGMENT_AND_OFFSET_ULEB, seg=1
        data.push(0x00); // offset=0 (ULEB128)
        data.push(0x50 | 1); // DO_REBASE_IMM_TIMES, count=1
        data.push(0x40 | 2); // ADD_ADDR_IMM_SCALED, scale=2 → add 2*8=16
        data.push(0x50 | 1); // DO_REBASE_IMM_TIMES, count=1
        data.push(0x00); // DONE

        let entries = parse_rebases(&data, 0, data.len() as u32);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].segment_offset, 0);
        // After first rebase: offset → 8, then ADD_ADDR_IMM_SCALED adds 2*8=16: 8+16=24=0x18
        assert_eq!(entries[1].segment_offset, 0x18);
    }

    #[test]
    fn rebase_uleb_times_skipping() {
        let mut data = Vec::new();
        data.push(0x20 | 0); // SET_SEGMENT_AND_OFFSET_ULEB, seg=0
        data.push(0x00); // offset=0
                         // DO_REBASE_ULEB_TIMES_SKIPPING_ULEB: count=2, skip=0x10
        data.push(0x80);
        data.push(2); // count=2
        data.push(0x10); // skip=0x10
        data.push(0x00); // DONE

        let entries = parse_rebases(&data, 0, data.len() as u32);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].segment_offset, 0);
        // skip + pointer_size = 0x10 + 8 = 0x18
        assert_eq!(entries[1].segment_offset, 0x18);
    }

    #[test]
    fn rebase_with_offset_into_data() {
        // Data with junk before and after
        let mut data = vec![0xAA, 0xBB, 0xCC]; // 3 bytes of junk
        data.push(0x20 | 0); // SET_SEGMENT_AND_OFFSET_ULEB at offset 3
        data.push(0x00);
        data.push(0x50 | 1); // DO_REBASE_IMM_TIMES, count=1
        data.push(0x00); // DONE
        data.extend_from_slice(&[0xDD, 0xEE]); // trailing junk

        let entries = parse_rebases(&data, 3, 4);
        assert_eq!(entries.len(), 1);
    }
}

#[cfg(test)]
mod bind_tests {
    use macho_parse::fixups::*;

    #[test]
    fn empty_bind_stream() {
        let entries = parse_binds(&[], 0, 0, false);
        assert!(entries.is_empty());
    }

    #[test]
    fn bind_simple_symbol() {
        let mut data = Vec::new();
        data.push(0x10 | 1); // SET_DYLIB_ORDINAL_IMM, ordinal=1
        data.push(0x40 | 0); // SET_SYMBOL_TRAILING_FLAGS_IMM, flags=0
        data.extend_from_slice(b"_printf\0"); // symbol name
        data.push(0x50 | 1); // SET_TYPE_IMM, type=pointer
        data.push(0x70 | 2); // SET_SEGMENT_AND_OFFSET_ULEB, seg=2
        data.push(0x00); // offset=0
        data.push(0x90); // DO_BIND
        data.push(0x00); // DONE

        let entries = parse_binds(&data, 0, data.len() as u32, false);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].symbol_name, "_printf");
        assert_eq!(entries[0].dylib_ordinal, 1);
        assert_eq!(entries[0].segment_index, 2);
        assert_eq!(entries[0].segment_offset, 0);
        assert!(!entries[0].is_weak);
        assert!(!entries[0].is_lazy);
    }

    #[test]
    fn bind_weak_symbol() {
        let mut data = Vec::new();
        data.push(0x10 | 1); // SET_DYLIB_ORDINAL_IMM
        data.push(0x40 | 1); // SET_SYMBOL_TRAILING_FLAGS_IMM, flags=1 (weak)
        data.extend_from_slice(b"_weak_sym\0");
        data.push(0x70 | 0); // SET_SEGMENT_AND_OFFSET_ULEB
        data.push(0x00);
        data.push(0x90); // DO_BIND
        data.push(0x00); // DONE

        let entries = parse_binds(&data, 0, data.len() as u32, false);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_weak);
    }

    #[test]
    fn bind_lazy_flag_propagates() {
        let mut data = Vec::new();
        data.push(0x10 | 1);
        data.push(0x40 | 0);
        data.extend_from_slice(b"_foo\0");
        data.push(0x70 | 1);
        data.push(0x00);
        data.push(0x90);
        data.push(0x00);

        let entries = parse_binds(&data, 0, data.len() as u32, true);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_lazy, "lazy flag should be propagated");
    }

    #[test]
    fn bind_with_addend() {
        let mut data = Vec::new();
        data.push(0x10 | 1); // ordinal=1
        data.push(0x40 | 0); // symbol flags=0
        data.extend_from_slice(b"_bar\0");
        data.push(0x60); // SET_ADDEND_SLEB
        data.push(0x08); // addend=8
        data.push(0x70 | 0); // seg=0
        data.push(0x00); // offset=0
        data.push(0x90); // DO_BIND
        data.push(0x00); // DONE

        let entries = parse_binds(&data, 0, data.len() as u32, false);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].addend, 8);
    }

    #[test]
    fn bind_multiple_do_bind() {
        let mut data = Vec::new();
        data.push(0x10 | 1);
        data.push(0x40 | 0);
        data.extend_from_slice(b"_sym\0");
        data.push(0x70 | 0);
        data.push(0x00);
        data.push(0x90); // DO_BIND
        data.push(0x90); // DO_BIND (another at next pointer)
        data.push(0x90); // DO_BIND
        data.push(0x00); // DONE

        let entries = parse_binds(&data, 0, data.len() as u32, false);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].segment_offset, 0);
        assert_eq!(entries[1].segment_offset, 8);
        assert_eq!(entries[2].segment_offset, 16);
    }

    #[test]
    fn bind_special_dylib_ordinal_self() {
        let mut data = Vec::new();
        data.push(0x30 | 0); // SET_DYLIB_SPECIAL_IMM, immediate=0 → SELF
        data.push(0x40 | 0);
        data.extend_from_slice(b"_self\0");
        data.push(0x70 | 0);
        data.push(0x00);
        data.push(0x90);
        data.push(0x00);

        let entries = parse_binds(&data, 0, data.len() as u32, false);
        assert_eq!(entries[0].dylib_ordinal, 0); // BIND_SPECIAL_DYLIB_SELF
    }
}

#[cfg(test)]
mod export_trie_tests {
    use macho_parse::exports::*;

    #[test]
    fn empty_trie() {
        let exports = parse_export_trie(&[]);
        assert!(exports.is_empty());
    }

    #[test]
    fn single_export() {
        // Manually construct a trie with one export: "_main" at address 0x100
        let mut trie = Vec::new();

        // Root node: terminal_size=0, 1 child
        trie.push(0x00); // terminal_size = 0 (not a terminal)
        trie.push(0x01); // 1 child
                         // Edge label: "_main\0"
        trie.extend_from_slice(b"_main\0");
        // Child offset (ULEB128) — will be at position after this
        let child_offset = trie.len() + 1; // +1 for the offset byte itself
        trie.push(child_offset as u8);

        // Child node: terminal with flags=0, address=0x100
        trie.push(0x03); // terminal_size = 3 bytes (flags + address)
        trie.push(0x00); // flags = 0 (regular)
        trie.push(0x80); // address = 0x100 (ULEB128: 0x80, 0x02)
        trie.push(0x02);
        trie.push(0x00); // 0 children

        let exports = parse_export_trie(&trie);
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "_main");
        assert_eq!(exports[0].address, 0x100);
        assert_eq!(exports[0].kind(), "regular");
        assert!(!exports[0].is_weak());
        assert!(!exports[0].is_reexport());
    }

    #[test]
    fn weak_export() {
        let mut trie = Vec::new();

        // Root: no terminal, 1 child
        trie.push(0x00);
        trie.push(0x01);
        trie.extend_from_slice(b"_weak\0");
        let child_offset = trie.len() + 1;
        trie.push(child_offset as u8);

        // Child: terminal with WEAK flag
        trie.push(0x03);
        trie.push(0x04); // flags = EXPORT_SYMBOL_FLAGS_WEAK_DEFINITION
        trie.push(0x80);
        trie.push(0x01); // address = 0x80
        trie.push(0x00); // 0 children

        let exports = parse_export_trie(&trie);
        assert_eq!(exports.len(), 1);
        assert!(exports[0].is_weak());
    }
}

#[cfg(test)]
mod fixture_fixup_tests {
    use macho_fixtures::read_fixture;
    use macho_parse::{extract_exports, extract_fixups, parse, FixupModel};

    // ── simple_cli uses chained fixups ──────────────────────────────

    #[test]
    fn simple_cli_has_chained_fixups() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let fixups = extract_fixups(&parsed, &data);
        assert!(matches!(fixups.model, FixupModel::Chained { .. }));
    }

    #[test]
    fn simple_cli_has_chained_entries() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let fixups = extract_fixups(&parsed, &data);
        assert!(
            !fixups.chained.is_empty(),
            "should have chained fixup entries"
        );
    }

    // ── simple_cli exports ──────────────────────────────────────────

    #[test]
    fn simple_cli_exports_main() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exports = extract_exports(&parsed, &data);
        assert!(!exports.is_empty());
        let has_main = exports.iter().any(|e| e.name == "_main");
        assert!(has_main, "simple_cli should export _main");
    }

    #[test]
    fn simple_cli_exports_mh_execute_header() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exports = extract_exports(&parsed, &data);
        let has_header = exports.iter().any(|e| e.name == "__mh_execute_header");
        assert!(has_header);
    }

    // ── dylib exports ───────────────────────────────────────────────

    #[test]
    fn dylib_chain_liba_exports_greet() {
        let Some(data) = read_fixture("dylib_chain/libchaina.dylib") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exports = extract_exports(&parsed, &data);
        let has_greet = exports.iter().any(|e| e.name == "_liba_greet");
        assert!(has_greet, "libchaina should export _liba_greet");
    }

    #[test]
    fn dylib_chain_libb_exports_greet() {
        let Some(data) = read_fixture("dylib_chain/libchainb.dylib") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exports = extract_exports(&parsed, &data);
        let has_greet = exports.iter().any(|e| e.name == "_libb_greet");
        assert!(has_greet, "libchainb should export _libb_greet");
    }

    // ── fixups JSON serialization ───────────────────────────────────

    #[test]
    fn fixups_serialize_to_json() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let fixups = extract_fixups(&parsed, &data);
        let json = serde_json::to_string(&fixups).unwrap();
        assert!(json.contains("Chained"));
    }

    #[test]
    fn exports_serialize_to_json() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let exports = extract_exports(&parsed, &data);
        let json = serde_json::to_string(&exports).unwrap();
        assert!(json.contains("_main"));
    }
}
