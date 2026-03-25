//! Phase 17: LC types, export trie, data-in-code, cstrings, fat subtype.

mod common;

use common::fixture_binary;

// ── LC_REEXPORT_DYLIB ordinals ──

#[test]
fn reexport_dylib_type_parsed() {
    // Parse a real system dylib that uses LC_REEXPORT_DYLIB
    // (e.g. /usr/lib/libSystem.B.dylib re-exports libdyld, libxpc, etc.)
    let path = std::path::Path::new("/usr/lib/libSystem.B.dylib");
    if !path.exists() {
        return; // Skip on non-macOS
    }
    let data = std::fs::read(path).unwrap();
    let parsed = if macho_parse::is_fat(&data) {
        macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)
            .or_else(|_| macho_parse::parse_auto(&data, macho_types::CpuType::X86_64))
            .unwrap()
    } else {
        macho_parse::parse(&data).unwrap()
    };
    // libSystem.B.dylib should have many dylib dependencies including re-exports
    assert!(
        !parsed.dylibs.is_empty(),
        "libSystem.B.dylib should have dylib deps"
    );
    // Check that ReexportDylib commands exist
    let has_reexport = parsed
        .commands
        .iter()
        .any(|c| c.cmd == macho_types::load_command::LoadCommandType::ReexportDylib);
    assert!(
        has_reexport,
        "libSystem.B.dylib should have LC_REEXPORT_DYLIB"
    );
}

#[test]
fn reexport_contributes_to_ordinals() {
    let path = std::path::Path::new("/usr/lib/libSystem.B.dylib");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
    let parsed = if macho_parse::is_fat(&data) {
        macho_parse::parse_auto(&data, macho_types::CpuType::Arm64)
            .or_else(|_| macho_parse::parse_auto(&data, macho_types::CpuType::X86_64))
            .unwrap()
    } else {
        macho_parse::parse(&data).unwrap()
    };
    // Count all ordinal-contributing commands
    let ordinal_cmds: Vec<_> = parsed
        .commands
        .iter()
        .filter(|c| {
            matches!(
                c.cmd,
                macho_types::load_command::LoadCommandType::LoadDylib
                    | macho_types::load_command::LoadCommandType::LoadWeakDylib
                    | macho_types::load_command::LoadCommandType::ReexportDylib
                    | macho_types::load_command::LoadCommandType::LazyLoadDylib
                    | macho_types::load_command::LoadCommandType::LoadUpwardDylib
            )
        })
        .collect();
    // parsed.dylibs should match the count of ordinal commands
    assert_eq!(
        parsed.dylibs.len(),
        ordinal_cmds.len(),
        "dylibs count should match ordinal-contributing load commands"
    );
}

// ── Export trie STUB_AND_RESOLVER ──

#[test]
fn export_stub_and_resolver_reads_resolver() {
    // Build a synthetic trie with STUB_AND_RESOLVER flag
    // Node: terminal_size=ULEB, flags=ULEB(0x10), stub_offset=ULEB, resolver_offset=ULEB
    let trie = vec![
        6,                      // terminal size: 6 bytes (flags + stub + resolver ULEBs)
        0x10,                   // flags = STUB_AND_RESOLVER
        0x42,                   // stub_offset = 0x42
        0x99u8 & 0x7F | 0x80,  // resolver_offset = 0x1099 (multi-byte ULEB)
        0x21,                   // continuation
        0,                      // 0 children
    ];

    let exports = macho_parse::parse_export_trie(&trie);
    assert_eq!(exports.len(), 1);
    let sym = &exports[0];
    assert_eq!(sym.flags, 0x10);
    // Should have the resolver address (0x1099 = 0x19 << 7 | 0x21 ← wait, recalculate)
    // 0x99 & 0x7F = 0x19, high bit set, next byte 0x21
    // value = 0x19 | (0x21 << 7) = 0x19 | 0x1080 = 0x1099
    assert_eq!(sym.address, 0x1099, "should use resolver address, not stub");
}

// ── Data-in-code ──

#[test]
fn data_in_code_from_simple_cli() {
    let path = fixture_binary("simple_cli");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(&path).unwrap();
    let parsed = macho_parse::parse(&data).unwrap();
    // simple_cli may or may not have data-in-code; the parser shouldn't crash
    let _entries = macho_parse::parse_data_in_code(&parsed, &data);
}

#[test]
fn data_in_code_decode_entries() {
    // Manually construct a data_in_code blob: 2 entries of 8 bytes each
    let mut blob = Vec::new();
    // Entry 1: offset=0x100, length=4, kind=DATA(1)
    blob.extend_from_slice(&0x100u32.to_le_bytes());
    blob.extend_from_slice(&4u16.to_le_bytes());
    blob.extend_from_slice(&1u16.to_le_bytes());
    // Entry 2: offset=0x200, length=8, kind=JUMP_TABLE32(4)
    blob.extend_from_slice(&0x200u32.to_le_bytes());
    blob.extend_from_slice(&8u16.to_le_bytes());
    blob.extend_from_slice(&4u16.to_le_bytes());

    // Create a minimal ParsedMachO with LC_DATA_IN_CODE pointing to our blob
    let mut parsed = macho_parse::ParsedMachO {
        header: macho_types::MachHeader {
            magic: 0xFEED_FACF,
            cpu_type: macho_types::CpuType::Arm64,
            cpu_subtype: 0,
            file_type: macho_types::header::FileType::Execute,
            num_commands: 1,
            size_of_commands: 16,
            flags: macho_types::header::MachFlags::empty(),
            reserved: Some(0),
        },
        commands: vec![macho_types::load_command::LoadCommand {
            cmd: macho_types::load_command::LoadCommandType::DataInCode,
            cmd_size: 16,
            payload: macho_types::load_command::LoadCommandPayload::LinkEdit {
                data_off: 0,
                data_size: blob.len() as u32,
            },
        }],
        segments: Vec::new(),
        sections: Vec::new(),
        dylibs: Vec::new(),
        rpaths: Vec::new(),
        entry: None,
        symbols: None,
        uuid: None,
        code_signature: None,
    };
    let _ = &mut parsed; // suppress unused_mut

    let entries = macho_parse::parse_data_in_code(&parsed, &blob);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].offset, 0x100);
    assert_eq!(entries[0].length, 4);
    assert_eq!(
        entries[0].kind,
        macho_types::data_in_code::DataInCodeKind::Data
    );
    assert_eq!(entries[1].offset, 0x200);
    assert_eq!(entries[1].length, 8);
    assert_eq!(
        entries[1].kind,
        macho_types::data_in_code::DataInCodeKind::JumpTable32
    );
}

// ── C-strings extraction ──

#[test]
fn extract_cstrings_from_simple_cli() {
    let path = fixture_binary("simple_cli");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(&path).unwrap();
    let parsed = macho_parse::parse(&data).unwrap();
    let sections = macho_parse::extract_cstrings(&parsed, &data);
    // simple_cli should have at least one cstring section with "Hello" or similar
    if !sections.is_empty() {
        let total: usize = sections.iter().map(|s| s.strings.len()).sum();
        assert!(total > 0, "should find at least one C-string");
    }
}

#[test]
fn extract_cstrings_empty_binary() {
    // ParsedMachO with no sections → no cstrings
    let parsed = macho_parse::ParsedMachO {
        header: macho_types::MachHeader {
            magic: 0xFEED_FACF,
            cpu_type: macho_types::CpuType::Arm64,
            cpu_subtype: 0,
            file_type: macho_types::header::FileType::Execute,
            num_commands: 0,
            size_of_commands: 0,
            flags: macho_types::header::MachFlags::empty(),
            reserved: Some(0),
        },
        commands: Vec::new(),
        segments: Vec::new(),
        sections: Vec::new(),
        dylibs: Vec::new(),
        rpaths: Vec::new(),
        entry: None,
        symbols: None,
        uuid: None,
        code_signature: None,
    };
    let sections = macho_parse::extract_cstrings(&parsed, &[]);
    assert!(sections.is_empty());
}

// ── Fat binary subtype selection ──

#[test]
fn fat_slice_for_cpu_subtype_fallback() {
    let path = common::fixture_path("fat_binary").join("app");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(&path).unwrap();
    if !macho_parse::is_fat(&data) {
        return;
    }
    // Should be able to pick arm64 with subtype preference
    let result = macho_parse::fat_slice_for_cpu_subtype(
        &data,
        macho_types::CpuType::Arm64,
        Some(macho_types::CpuSubtype::Arm64E),
    );
    // If no arm64e slice, it should fall back to arm64 all
    // Either way, it should succeed (we have arm64 in our fat fixture)
    assert!(result.is_ok(), "should find arm64 slice via fallback");
}

#[test]
fn fat_slice_subtype_none_behaves_like_original() {
    let path = common::fixture_path("fat_binary").join("app");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(&path).unwrap();
    if !macho_parse::is_fat(&data) {
        return;
    }
    let orig = macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64).unwrap();
    let sub =
        macho_parse::fat_slice_for_cpu_subtype(&data, macho_types::CpuType::Arm64, None).unwrap();
    assert_eq!(
        orig.len(),
        sub.len(),
        "None subtype should match original behavior"
    );
}

// ── EncryptionInfo parsing ──

#[test]
fn encryption_info_types_recognized() {
    // The load command types should parse correctly
    use macho_types::load_command::LoadCommandType;
    assert_eq!(LoadCommandType::from(0x21), LoadCommandType::EncryptionInfo);
    assert_eq!(
        LoadCommandType::from(0x2C),
        LoadCommandType::EncryptionInfo64
    );
}

// ── Maxprot in segments ──

#[test]
fn segments_have_maxprot() {
    let path = fixture_binary("simple_cli");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(&path).unwrap();
    let parsed = macho_parse::parse(&data).unwrap();
    for seg in &parsed.segments {
        // Every segment should have a non-empty maxprot (at least read)
        if seg.vmsize > 0 && seg.name != "__PAGEZERO" {
            assert!(
                !seg.maxprot.is_empty(),
                "segment {} should have non-empty maxprot",
                seg.name
            );
        }
    }
}
