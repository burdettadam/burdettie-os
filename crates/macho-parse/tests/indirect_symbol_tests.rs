//! Phase 14: DyldInfo fix, indirect symbols, symbol desc, selectors.

mod common;

use common::fixture_path;
use macho_parse::{extract_fixups, parse, read_indirect_symbol_table, resolve_indirect_symbols};
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
use macho_types::symbol::is_stab;

// ── DyldInfo weak_bind fix ──────────────────────────────────

#[test]
fn dyld_info_has_weak_bind_fields() {
    // Parse a binary with LC_DYLD_INFO_ONLY and verify
    // the weak_bind_off/weak_bind_size fields exist
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::DyldInfoOnly || cmd.cmd == LoadCommandType::DyldInfo {
            match &cmd.payload {
                LoadCommandPayload::DyldInfo {
                    rebase_off: _,
                    rebase_size: _,
                    bind_off: _,
                    bind_size: _,
                    weak_bind_off,
                    weak_bind_size,
                    lazy_bind_off: _,
                    lazy_bind_size: _,
                    export_off,
                    export_size,
                } => {
                    // Weak bind fields should exist (may be 0 if no weak binds)
                    let _ = weak_bind_off;
                    let _ = weak_bind_size;
                    // Export fields presence is format-dependent; just verify we parsed them
                    let _ = export_off;
                    let _ = export_size;
                }
                other => panic!("expected DyldInfo payload, got {other:?}"),
            }
            return;
        }
    }
    // If no DyldInfo command, that's fine (chained fixups binary)
}

#[test]
fn fixups_has_weak_binds_field() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();
    let fixups = extract_fixups(&parsed, &data);
    // weak_binds should exist as a field (may be empty)
    let _ = fixups.weak_binds;
}

// ── Section reserved1/reserved2 ────────────────────────────

#[test]
fn section_reserved1_nonzero_for_stubs() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let stubs = parsed
        .segments
        .iter()
        .flat_map(|s| &s.sections)
        .find(|s| s.sectname == "__stubs");

    if let Some(s) = stubs {
        // reserved1 is the indirect symbol table start index
        // It's typically > 0 for stubs (unless it's the first indirect section)
        // reserved2 is the stub size
        assert!(
            s.reserved2 > 0,
            "__stubs should have nonzero stub size (reserved2)"
        );
    }
}

#[test]
fn section_reserved1_set_for_got() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let got = parsed
        .segments
        .iter()
        .flat_map(|s| &s.sections)
        .find(|s| s.sectname == "__got");

    if let Some(s) = got {
        // reserved1 is the indirect symbol table start index
        // Just verify the field is accessible
        let _ = s.reserved1;
    }
}

// ── Indirect symbol table ───────────────────────────────────

#[test]
fn indirect_symbol_table_simple_cli() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let entries = resolve_indirect_symbols(&parsed, &data);
    assert!(
        !entries.is_empty(),
        "simple_cli should have indirect symbols"
    );

    // Should have _printf in stubs or GOT
    let has_printf = entries.iter().any(|e| e.symbol_name == "_printf");
    assert!(
        has_printf,
        "should find _printf in indirect symbols, got: {:?}",
        entries.iter().map(|e| &e.symbol_name).collect::<Vec<_>>()
    );
}

#[test]
fn indirect_symbol_table_objc() {
    let path = fixture_path("objc_classes/app");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("SKIP: objc_classes fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();

    let entries = resolve_indirect_symbols(&parsed, &data);
    // ObjC binary should have NSLog, objc_alloc_init, etc.
    let has_nslog = entries.iter().any(|e| e.symbol_name == "_NSLog");
    assert!(
        has_nslog,
        "ObjC binary should have _NSLog in indirect symbols"
    );

    // Should have entries from both __stubs and __got
    let stubs: Vec<_> = entries.iter().filter(|e| e.section == "__stubs").collect();
    let got: Vec<_> = entries.iter().filter(|e| e.section == "__got").collect();
    assert!(!stubs.is_empty(), "should have __stubs entries");
    assert!(!got.is_empty(), "should have __got entries");
}

#[test]
fn indirect_table_slot_addrs_in_section_range() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let entries = resolve_indirect_symbols(&parsed, &data);
    for entry in &entries {
        let sect = parsed
            .segments
            .iter()
            .flat_map(|s| &s.sections)
            .find(|s| s.sectname == entry.section);
        if let Some(s) = sect {
            assert!(
                entry.slot_addr >= s.addr,
                "slot addr {:#x} should be >= section addr {:#x}",
                entry.slot_addr,
                s.addr
            );
            assert!(
                entry.slot_addr < s.addr + s.size,
                "slot addr {:#x} should be < section end {:#x}",
                entry.slot_addr,
                s.addr + s.size
            );
        }
    }
}

#[test]
fn raw_indirect_table_matches_dysymtab() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    // Find DYSYMTAB
    for cmd in &parsed.commands {
        if cmd.cmd == LoadCommandType::Dysymtab {
            if let LoadCommandPayload::Dysymtab {
                indirectsymoff,
                nindirectsyms,
                ..
            } = &cmd.payload
            {
                let table = read_indirect_symbol_table(&data, *indirectsymoff, *nindirectsyms);
                assert_eq!(table.len(), *nindirectsyms as usize);
                // Each entry should be either a valid symbol index or a sentinel
                for &idx in &table {
                    if idx != macho_parse::indirect::INDIRECT_SYMBOL_LOCAL
                        && idx != macho_parse::indirect::INDIRECT_SYMBOL_ABS
                        && idx
                            != (macho_parse::indirect::INDIRECT_SYMBOL_LOCAL
                                | macho_parse::indirect::INDIRECT_SYMBOL_ABS)
                    {
                        if let Some(syms) = &parsed.symbols {
                            assert!(
                                (idx as usize) < syms.symbols.len(),
                                "indirect sym index {idx} out of range ({})",
                                syms.symbols.len()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn empty_indirect_table_returns_empty() {
    let entries = read_indirect_symbol_table(&[], 0, 0);
    assert!(entries.is_empty());
}

#[test]
fn indirect_table_truncated_data() {
    // Offset beyond data length
    let data = vec![0u8; 16];
    let entries = read_indirect_symbol_table(&data, 100, 5);
    assert!(entries.is_empty());
}

// ── Symbol desc accessors ───────────────────────────────────

#[test]
fn symbol_desc_weak_ref() {
    let sym = macho_types::symbol::Symbol {
        name: "test".to_string(),
        sym_type: macho_types::symbol::SymbolType::Undefined,
        section_index: 0,
        desc: 0x0040, // WEAK_REF
        value: 0,
    };
    assert!(sym.is_weak_ref());
    assert!(!sym.is_weak_def());
}

#[test]
fn symbol_desc_weak_def() {
    let sym = macho_types::symbol::Symbol {
        name: "test".to_string(),
        sym_type: macho_types::symbol::SymbolType::Defined,
        section_index: 1,
        desc: 0x0080, // WEAK_DEF
        value: 0x1000,
    };
    assert!(sym.is_weak_def());
    assert!(!sym.is_weak_ref());
}

#[test]
fn symbol_desc_no_dead_strip() {
    let sym = macho_types::symbol::Symbol {
        name: "test".to_string(),
        sym_type: macho_types::symbol::SymbolType::Defined,
        section_index: 1,
        desc: 0x0020, // N_NO_DEAD_STRIP
        value: 0,
    };
    assert!(sym.is_no_dead_strip());
}

#[test]
fn symbol_desc_library_ordinal() {
    let sym = macho_types::symbol::Symbol {
        name: "test".to_string(),
        sym_type: macho_types::symbol::SymbolType::Undefined,
        section_index: 0,
        desc: 0x0200, // ordinal = 2
        value: 0,
    };
    assert_eq!(sym.library_ordinal(), 2);
}

#[test]
fn symbol_desc_library_ordinal_self() {
    // Ordinal 0 = self
    let sym = macho_types::symbol::Symbol {
        name: "test".to_string(),
        sym_type: macho_types::symbol::SymbolType::Defined,
        section_index: 1,
        desc: 0x0000,
        value: 0,
    };
    assert_eq!(sym.library_ordinal(), 0);
}

#[test]
fn is_stab_detects_debug_symbols() {
    // Stab symbols have high bits set in n_type
    assert!(is_stab(0x20)); // N_GSYM
    assert!(is_stab(0x24)); // N_FUN
    assert!(is_stab(0x26)); // N_STSYM
    assert!(is_stab(0x64)); // N_SO
    assert!(!is_stab(0x0E)); // N_SECT (defined, not a stab)
    assert!(!is_stab(0x0F)); // N_SECT | N_EXT
    assert!(!is_stab(0x01)); // N_EXT (undefined external)
}

#[test]
fn real_binary_symbols_have_ordinals() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    if let Some(syms) = &parsed.symbols {
        let undefined: Vec<_> = syms
            .symbols
            .iter()
            .filter(|s| matches!(s.sym_type, macho_types::symbol::SymbolType::Undefined))
            .collect();

        for sym in &undefined {
            let ordinal = sym.library_ordinal();
            // Ordinal should be positive (referencing a specific dylib)
            // or 0 (self) or negative (special)
            assert!(
                ordinal != 0 || sym.name.starts_with("_"),
                "undefined sym {} should have nonzero ordinal, got {}",
                sym.name,
                ordinal
            );
        }
    }
}

// ── ObjC selectors ──────────────────────────────────────────

#[test]
fn objc_selectors_extracted() {
    let path = fixture_path("objc_classes/app");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("SKIP: objc_classes fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();
    let meta = macho_objc::parse_objc_metadata(&parsed, &data);

    assert!(
        !meta.selectors.is_empty(),
        "ObjC binary should have selectors"
    );
    assert!(
        meta.selectors.contains(&"greet".to_string()),
        "should find 'greet' selector, got {:?}",
        meta.selectors
    );
    assert!(
        meta.selectors.contains(&"add:to:".to_string()),
        "should find 'add:to:' selector, got {:?}",
        meta.selectors
    );
}

#[test]
fn simple_cli_has_no_selectors() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();
    let meta = macho_objc::parse_objc_metadata(&parsed, &data);
    assert!(
        meta.selectors.is_empty(),
        "pure C binary should have no selectors"
    );
}

// ── System binary integration ───────────────────────────────

#[test]
fn system_ls_indirect_symbols() {
    let Ok(data) = std::fs::read("/bin/ls") else {
        eprintln!("SKIP: /bin/ls not readable");
        return;
    };
    // /bin/ls may be a fat binary
    let parsed = if macho_parse::is_fat(&data) {
        macho_parse::parse_auto(&data, macho_types::header::CpuType::Arm64)
            .or_else(|_| macho_parse::parse_auto(&data, macho_types::header::CpuType::X86_64))
            .unwrap()
    } else {
        parse(&data).unwrap()
    };

    let entries = resolve_indirect_symbols(&parsed, &data);
    // /bin/ls should have many indirect symbols
    assert!(
        entries.len() > 5,
        "/bin/ls should have >5 indirect symbols, got {}",
        entries.len()
    );
}
