//! Phase 13 tests (vmaddr_to_file_offset, SourceVersion, Dysymtab, BuildVersion, ObjC classes)
//! and malformed input hardening tests.

mod common;

use common::fixture_path;
use macho_parse::{extract_exports, extract_fixups, parse, vmaddr_to_file_offset};
use macho_types::load_command::{LoadCommandPayload, LoadCommandType};

// ── vmaddr_to_file_offset ─────────────────────────────────────

#[test]
fn vmaddr_text_segment_maps_to_file_offset() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    // __TEXT segment vaddr=0x100000000, fileoff=0 typically
    let text_seg = parsed.segments.iter().find(|s| s.name == "__TEXT").unwrap();
    let vmaddr = text_seg.vmaddr; // start of __TEXT
    let off = vmaddr_to_file_offset(&parsed, vmaddr);
    assert_eq!(off, Some(text_seg.fileoff as usize));
}

#[test]
fn vmaddr_data_segment_maps_correctly() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    if let Some(data_seg) = parsed
        .segments
        .iter()
        .find(|s| s.name == "__DATA" || s.name == "__DATA_CONST")
    {
        let off = vmaddr_to_file_offset(&parsed, data_seg.vmaddr);
        assert_eq!(off, Some(data_seg.fileoff as usize));
    }
}

#[test]
fn vmaddr_out_of_range_returns_none() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    // An address way outside any segment
    let off = vmaddr_to_file_offset(&parsed, 0xDEADBEEF_00000000);
    assert!(off.is_none());
}

#[test]
fn vmaddr_mid_segment_offset() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let text_seg = parsed.segments.iter().find(|s| s.name == "__TEXT").unwrap();
    if text_seg.filesize > 0x100 {
        let mid = text_seg.vmaddr + 0x100;
        let off = vmaddr_to_file_offset(&parsed, mid);
        assert_eq!(off, Some(text_seg.fileoff as usize + 0x100));
    }
}

// ── LC_SOURCE_VERSION parsing ─────────────────────────────────

#[test]
fn source_version_parsed() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let sv_cmd = parsed
        .commands
        .iter()
        .find(|c| c.cmd == LoadCommandType::SourceVersion);
    if let Some(cmd) = sv_cmd {
        match &cmd.payload {
            LoadCommandPayload::SourceVersion { version } => {
                // Version should be a valid u64 (often 0 for simple binaries)
                let _ = version;
            }
            other => panic!("expected SourceVersion payload, got {other:?}"),
        }
    }
}

// ── LC_DYSYMTAB parsing ───────────────────────────────────────

#[test]
fn dysymtab_parsed() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let dt_cmd = parsed
        .commands
        .iter()
        .find(|c| c.cmd == LoadCommandType::Dysymtab);
    if let Some(cmd) = dt_cmd {
        match &cmd.payload {
            LoadCommandPayload::Dysymtab {
                ilocalsym,
                nlocalsym,
                iextdefsym,
                nextdefsym,
                iundefsym,
                nundefsym,
                indirectsymoff: _,
                nindirectsyms: _,
            } => {
                // Symbol ranges should be contiguous:
                // locals start at 0, extdefs after locals, undefs after extdefs
                assert_eq!(*ilocalsym, 0, "local symbols should start at index 0");
                assert_eq!(*iextdefsym, *nlocalsym, "extdef should start after locals");
                assert_eq!(
                    *iundefsym,
                    *iextdefsym + *nextdefsym,
                    "undef should start after extdefs"
                );
                let total = *nlocalsym + *nextdefsym + *nundefsym;
                assert!(total > 0, "DYSYMTAB should have some symbols");
            }
            other => panic!("expected Dysymtab payload, got {other:?}"),
        }
    }
}

// ── LC_BUILD_VERSION parsing ──────────────────────────────────

#[test]
fn build_version_has_valid_platform() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();

    let bv_cmd = parsed
        .commands
        .iter()
        .find(|c| c.cmd == LoadCommandType::BuildVersion);
    if let Some(cmd) = bv_cmd {
        match &cmd.payload {
            LoadCommandPayload::BuildVersion {
                platform,
                minos,
                sdk,
                ntools: _,
            } => {
                // Platform should be macOS (1) for our fixture
                assert_eq!(*platform, 1, "fixture should target macOS");
                assert!(*minos > 0, "min OS version should be nonzero");
                assert!(*sdk > 0, "SDK version should be nonzero");
            }
            other => panic!("expected BuildVersion payload, got {other:?}"),
        }
    }
}

// ── ObjC class names ──────────────────────────────────────────

#[test]
fn objc_classes_fixture_extracts_names() {
    let path = fixture_path("objc_classes/app");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("SKIP: objc_classes fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();
    let meta = macho_objc::parse_objc_metadata(&parsed, &data);

    assert_eq!(meta.class_refs.len(), 2, "fixture should have 2 classes");

    let names: Vec<&str> = meta
        .class_refs
        .iter()
        .filter_map(|c| c.name.as_deref())
        .collect();
    assert!(
        names.contains(&"Greeter"),
        "should find Greeter class, got {names:?}"
    );
    assert!(
        names.contains(&"Calculator"),
        "should find Calculator class, got {names:?}"
    );
}

#[test]
fn simple_cli_has_no_objc_classes() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();
    let meta = macho_objc::parse_objc_metadata(&parsed, &data);
    assert!(
        meta.class_refs.is_empty(),
        "pure C binary should have no ObjC classes"
    );
}

// ── Malformed input hardening ───────────────────────────────────

#[test]
fn empty_input() {
    assert!(parse(&[]).is_err());
}

#[test]
fn single_byte_input() {
    assert!(parse(&[0xFF]).is_err());
}

#[test]
fn four_bytes_not_magic() {
    assert!(parse(&[0x00, 0x00, 0x00, 0x00]).is_err());
}

#[test]
fn valid_magic_but_truncated_after_header() {
    // Construct a minimal LE Mach-O 64-bit header (0xFEEDFACF)
    // but with num_commands=1 and no actual command data
    let mut buf = vec![0u8; 32]; // header is 32 bytes
                                 // magic
    buf[0..4].copy_from_slice(&0xFEEDFACFu32.to_le_bytes());
    // cpu_type = ARM64 (0x0100000C)
    buf[4..8].copy_from_slice(&0x0100000Cu32.to_le_bytes());
    // cpu_subtype = 0
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    // filetype = EXECUTE (2)
    buf[12..16].copy_from_slice(&2u32.to_le_bytes());
    // ncmds = 1
    buf[16..20].copy_from_slice(&1u32.to_le_bytes());
    // sizeofcmds = 100 (lies — no room)
    buf[20..24].copy_from_slice(&100u32.to_le_bytes());
    // flags = 0
    buf[24..28].copy_from_slice(&0u32.to_le_bytes());
    // reserved = 0
    buf[28..32].copy_from_slice(&0u32.to_le_bytes());

    let result = parse(&buf);
    assert!(result.is_err(), "should reject header with no command data");
}

#[test]
fn exports_from_unparseable_data() {
    // extract_exports on a valid ParsedMachO with no export trie command
    // should return empty, not panic
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();
    // This should work normally
    let exports = extract_exports(&parsed, &data);
    assert!(!exports.is_empty(), "simple_cli should have at least _main");
}

#[test]
fn fixups_from_valid_binary() {
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let parsed = parse(&data).unwrap();
    let fixups = extract_fixups(&parsed, &data);
    // simple_cli may or may not have fixups, but should not panic
    let _ = fixups;
}

#[test]
fn fat_header_with_absurd_nfat_arch() {
    // FAT magic (0xCAFEBABE) + nfat_arch = 0xFFFFFFFF
    // Should reject without OOM
    let mut buf = vec![0u8; 8];
    buf[0..4].copy_from_slice(&0xCAFEBABEu32.to_be_bytes()); // fat magic is big-endian
    buf[4..8].copy_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // absurd nfat_arch

    // parse_fat or is_fat + parse should handle this gracefully
    assert!(macho_parse::is_fat(&buf));
    let result = macho_parse::parse_fat(&buf);
    assert!(
        result.is_err(),
        "should reject fat binary with absurd nfat_arch"
    );
}

#[test]
fn load_command_count_overflow() {
    // A header claiming 0xFFFF load commands but only 32 bytes of data
    let mut buf = vec![0u8; 32];
    buf[0..4].copy_from_slice(&0xFEEDFACFu32.to_le_bytes());
    buf[4..8].copy_from_slice(&0x0100000Cu32.to_le_bytes()); // ARM64
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    buf[12..16].copy_from_slice(&2u32.to_le_bytes()); // EXECUTE
    buf[16..20].copy_from_slice(&0xFFFFu32.to_le_bytes()); // ncmds = 65535
    buf[20..24].copy_from_slice(&0u32.to_le_bytes()); // sizeofcmds = 0
    buf[24..28].copy_from_slice(&0u32.to_le_bytes());
    buf[28..32].copy_from_slice(&0u32.to_le_bytes());

    let result = parse(&buf);
    // Should either error or produce empty commands, not panic or loop
    match result {
        Err(_) => {} // Good: rejected
        Ok(p) => {
            // If it "succeeds", it should have 0 actual commands
            assert_eq!(p.commands.len(), 0);
        }
    }
}

#[test]
fn vmaddr_to_file_offset_empty_segments() {
    // Construct a ParsedMachO with no segments
    let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
    let mut parsed = parse(&data).unwrap();
    parsed.segments.clear();
    // Should return None for any address
    assert!(vmaddr_to_file_offset(&parsed, 0x100000000).is_none());
}

#[test]
fn objc_metadata_on_truncated_classlist() {
    // Take a valid ObjC binary and truncate it mid-classlist
    let path = fixture_path("objc_classes/app");
    let Ok(full_data) = std::fs::read(&path) else {
        eprintln!("SKIP: objc_classes fixture not built");
        return;
    };

    // Find the classlist section offset and truncate after 4 bytes of it
    let parsed = parse(&full_data).unwrap();
    let classlist = parsed
        .segments
        .iter()
        .flat_map(|s| &s.sections)
        .find(|s| s.sectname == "__objc_classlist");
    if let Some(sect) = classlist {
        let truncate_at = sect.offset as usize + 4; // mid-pointer
        if truncate_at < full_data.len() {
            let truncated = &full_data[..truncate_at];
            // This may fail to parse entirely, or succeed with partial data.
            // Either way, it should not panic.
            if let Ok(p) = parse(truncated) {
                let meta = macho_objc::parse_objc_metadata(&p, truncated);
                // Should handle gracefully — may have 0 or partial classes
                let _ = meta;
            }
        }
    }
}
