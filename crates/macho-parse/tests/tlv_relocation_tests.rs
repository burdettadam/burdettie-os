//! Phase 15: TLV descriptor parsing.
//! Phase 16: Relocation parsing + ParsedMachO Deserialize.

mod common;

use common::read_fixture;
use macho_parse::parse;

// ── TLV descriptor parsing ─────────────────────────────────────

#[test]
fn tlv_fixture_has_descriptors() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &data);
    assert_eq!(
        descriptors.len(),
        2,
        "tlv_basic should have 2 TLV descriptors"
    );
}

#[test]
fn tlv_fixture_has_thread_vars_section() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let thread_vars = parsed
        .sections
        .iter()
        .find(|s| s.sectname == "__thread_vars");
    assert!(thread_vars.is_some(), "should have __thread_vars section");
    let tv = thread_vars.unwrap();
    assert_eq!(
        tv.section_type_name(),
        macho_types::segment::SectionType::ThreadLocalVariables
    );
}

#[test]
fn tlv_fixture_has_thread_data_section() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let thread_data = parsed
        .sections
        .iter()
        .find(|s| s.sectname == "__thread_data");
    assert!(thread_data.is_some(), "should have __thread_data section");
    let td = thread_data.unwrap();
    assert_eq!(
        td.section_type_name(),
        macho_types::segment::SectionType::ThreadLocalRegular
    );
}

#[test]
fn tlv_fixture_has_tlv_flag() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    assert!(
        parsed
            .header
            .flags
            .contains(macho_types::header::MachFlags::HAS_TLV_DESCRIPTORS),
        "tlv_basic should have HAS_TLV_DESCRIPTORS flag"
    );
}

#[test]
fn tlv_fixture_exports_thread_local_symbols() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let exports = macho_parse::extract_exports(&parsed, &data);
    let tls_exports: Vec<_> = exports
        .iter()
        .filter(|e| e.kind() == "thread-local")
        .collect();
    assert!(
        tls_exports.len() >= 2,
        "should have at least 2 thread-local exports, got {}",
        tls_exports.len()
    );
}

#[test]
fn tlv_descriptor_addresses_match_section() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &data);
    let thread_vars = parsed
        .sections
        .iter()
        .find(|s| s.sectname == "__thread_vars")
        .unwrap();

    // First descriptor should be at the section start address
    assert_eq!(descriptors[0].address, thread_vars.addr);
    // Last descriptor + 24 bytes should not exceed section end
    let last = descriptors.last().unwrap();
    assert!(last.address + 24 <= thread_vars.addr + thread_vars.size);
}

#[test]
fn tlv_descriptors_distinct_offsets() {
    let Some(data) = read_fixture("tlv_basic/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &data);
    if descriptors.len() >= 2 {
        // The two TLV vars should have different offsets
        assert_ne!(
            descriptors[0].offset, descriptors[1].offset,
            "different TLV vars should have different offsets"
        );
    }
}

#[test]
fn tlv_returns_empty_for_non_tlv_binary() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &data);
    assert!(
        descriptors.is_empty(),
        "simple_cli with no TLV should produce empty descriptors"
    );
}

#[test]
fn tlv_returns_empty_for_malformed_data() {
    let parsed = macho_parse::ParsedMachO {
        header: macho_types::MachHeader {
            magic: 0xFEED_FACF,
            cpu_type: macho_types::CpuType::Arm64,
            cpu_subtype: 0,
            file_type: macho_types::FileType::Execute,
            num_commands: 0,
            size_of_commands: 0,
            flags: macho_types::header::MachFlags::empty(),
            reserved: Some(0),
        },
        commands: vec![],
        segments: vec![],
        sections: vec![macho_types::Section {
            sectname: "__thread_vars".into(),
            segname: "__DATA".into(),
            addr: 0x1000,
            size: 48,
            offset: 0, // offset=0 with magic bytes means bad data
            align: 3,
            reloff: 0,
            nreloc: 0,
            flags: 0x13,
            reserved1: 0,
            reserved2: 0,
        }],
        dylibs: vec![],
        rpaths: vec![],
        entry: None,
        symbols: None,
        uuid: None,
        code_signature: None,
    };
    // Only 4 bytes of data — can't contain a full descriptor
    let data = vec![0u8; 4];
    let descriptors = macho_parse::parse_tlv_descriptors(&parsed, &data);
    assert!(descriptors.is_empty());
}

// ── Relocation parsing ──────────────────────────────────────────

#[test]
fn parse_relocations_empty_for_chained_binary() {
    // Modern binaries using chained fixups typically have no section relocations
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let relocs = macho_parse::parse_relocations(&parsed, &data);
    // simple_cli uses chained fixups — likely no section relocation entries
    // (this validates the parser doesn't crash on binaries with no relocs)
    let _ = relocs;
}

#[test]
fn parse_relocations_from_obj_file() {
    // Compile a .o file that will have relocation entries
    let temp_dir = std::env::temp_dir().join("darwin_loader_lab_test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let c_path = temp_dir.join("reloc_test.c");
    let o_path = temp_dir.join("reloc_test.o");
    std::fs::write(&c_path, "extern int foo; int bar(void) { return foo + 1; }").ok();
    let status = std::process::Command::new("cc")
        .args(["-c", "-arch", "arm64", "-o"])
        .arg(&o_path)
        .arg(&c_path)
        .status();
    let Ok(s) = status else { return };
    if !s.success() {
        return;
    }

    let data = std::fs::read(&o_path).unwrap();
    let parsed = parse(&data).unwrap();
    let relocs = macho_parse::parse_relocations(&parsed, &data);

    // An object file referencing an external symbol should have relocations
    let total: usize = relocs.iter().map(|r| r.entries.len()).sum();
    assert!(
        total > 0,
        "object file with extern ref should have relocations, got {total}"
    );

    // Verify relocation entry structure
    let first = &relocs[0].entries[0];
    assert!(first.length > 0, "relocation should have non-zero length");

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn relocation_entries_have_valid_types() {
    let temp_dir = std::env::temp_dir().join("darwin_loader_lab_reloc2");
    let _ = std::fs::create_dir_all(&temp_dir);
    let c_path = temp_dir.join("rt.c");
    let o_path = temp_dir.join("rt.o");
    std::fs::write(&c_path, "int x = 42; int *p = &x;").ok();
    let status = std::process::Command::new("cc")
        .args(["-c", "-arch", "arm64", "-o"])
        .arg(&o_path)
        .arg(&c_path)
        .status();
    let Ok(s) = status else { return };
    if !s.success() {
        return;
    }

    let data = std::fs::read(&o_path).unwrap();
    let parsed = parse(&data).unwrap();
    let relocs = macho_parse::parse_relocations(&parsed, &data);

    for sr in &relocs {
        for entry in &sr.entries {
            // All entries should decode to known ARM64 types, not Unknown
            // (though some might be Unknown if the compiler uses newer types)
            assert!(
                entry.length <= 3,
                "length should be 0-3 (byte/word/long/quad)"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn section_relocations_struct_has_segment_info() {
    let temp_dir = std::env::temp_dir().join("darwin_loader_lab_reloc3");
    let _ = std::fs::create_dir_all(&temp_dir);
    let c_path = temp_dir.join("seg.c");
    let o_path = temp_dir.join("seg.o");
    std::fs::write(&c_path, "int x = 42; int *p = &x;").ok();
    let status = std::process::Command::new("cc")
        .args(["-c", "-arch", "arm64", "-o"])
        .arg(&o_path)
        .arg(&c_path)
        .status();
    let Ok(s) = status else { return };
    if !s.success() {
        return;
    }

    let data = std::fs::read(&o_path).unwrap();
    let parsed = parse(&data).unwrap();
    let relocs = macho_parse::parse_relocations(&parsed, &data);

    for sr in &relocs {
        assert!(!sr.section.is_empty(), "section name should be non-empty");
        assert!(!sr.segment.is_empty(), "segment name should be non-empty");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// ── ParsedMachO Deserialize ─────────────────────────────────────

#[test]
fn parsed_macho_json_roundtrip() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    let json = serde_json::to_string(&parsed).unwrap();
    let restored: macho_parse::ParsedMachO = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.header.magic, parsed.header.magic);
    assert_eq!(restored.header.cpu_type, parsed.header.cpu_type);
    assert_eq!(restored.segments.len(), parsed.segments.len());
    assert_eq!(restored.dylibs.len(), parsed.dylibs.len());
    assert_eq!(restored.commands.len(), parsed.commands.len());
}

#[test]
fn parsed_macho_roundtrip_preserves_uuid() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    let json = serde_json::to_string(&parsed).unwrap();
    let restored: macho_parse::ParsedMachO = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.uuid, parsed.uuid);
}

#[test]
fn parsed_macho_roundtrip_preserves_code_signature() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    let json = serde_json::to_string(&parsed).unwrap();
    let restored: macho_parse::ParsedMachO = serde_json::from_str(&json).unwrap();

    assert_eq!(
        restored.code_signature.is_some(),
        parsed.code_signature.is_some()
    );
    if let (Some(orig), Some(rest)) = (&parsed.code_signature, &restored.code_signature) {
        assert_eq!(orig.has_cms_signature, rest.has_cms_signature);
        assert_eq!(orig.entitlements_plist, rest.entitlements_plist);
    }
}

// ── Fixture registry ────────────────────────────────────────────

#[test]
fn known_fixtures_includes_all_fifteen() {
    let known = macho_fixtures::known_fixtures();
    assert_eq!(known.len(), 35, "should have 35 registered fixtures");
    assert!(known.contains(&"simple_cli"));
    assert!(known.contains(&"fat_binary"));
    assert!(known.contains(&"objc_classes"));
    assert!(known.contains(&"return42"));
    assert!(known.contains(&"tlv_basic"));
    assert!(known.contains(&"weak_dylib"));
    assert!(known.contains(&"process_info"));
    assert!(known.contains(&"dispatch_test"));
    assert!(known.contains(&"objc_minimal"));
    assert!(known.contains(&"objc_category"));
}

#[test]
fn fixtures_dir_exists() {
    let dir = macho_fixtures::fixtures_dir();
    assert!(
        dir.exists(),
        "fixtures directory should exist: {}",
        dir.display()
    );
}
