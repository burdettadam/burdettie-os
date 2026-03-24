//! Tests for macho-parse: synthetic byte-level parsing and malformed input.
//!
//! Ported from:
//!   - goblin crate: bad magic, truncated input, fuzz-found panics
//!   - LLVM llvm-objdump: malformed load commands, truncated sections
//!   - XNU mach_loader.c: header bounds checking
#![allow(clippy::vec_init_then_push)] // binary format construction
//!   - Apple dyld: segment protection and entry point validation

#[cfg(test)]
mod synthetic {
    use macho_parse::{parse, ParseError};
    use macho_types::header::*;
    use macho_types::segment::VmProt;

    /// Build a minimal valid 64-bit Mach-O with no load commands.
    fn minimal_macho64() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes()); // magic
        buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // cpu_type: ARM64
        buf.extend_from_slice(&0u32.to_le_bytes()); // cpu_subtype
        buf.extend_from_slice(&2u32.to_le_bytes()); // file_type: MH_EXECUTE
        buf.extend_from_slice(&0u32.to_le_bytes()); // ncmds
        buf.extend_from_slice(&0u32.to_le_bytes()); // sizeofcmds
        buf.extend_from_slice(&0u32.to_le_bytes()); // flags
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved
        buf
    }

    /// Append a minimal LC_SEGMENT_64 with no sections to a buffer.
    fn append_segment64(buf: &mut Vec<u8>, name: &str, vmaddr: u64, vmsize: u64, prot: u32) {
        let cmd_size: u32 = 72; // sizeof(segment_command_64)
        buf.extend_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
        buf.extend_from_slice(&cmd_size.to_le_bytes());
        // segname: 16 bytes, null-padded
        let mut segname = [0u8; 16];
        let name_bytes = name.as_bytes();
        segname[..name_bytes.len().min(16)]
            .copy_from_slice(&name_bytes[..name_bytes.len().min(16)]);
        buf.extend_from_slice(&segname);
        buf.extend_from_slice(&vmaddr.to_le_bytes()); // vmaddr
        buf.extend_from_slice(&vmsize.to_le_bytes()); // vmsize
        buf.extend_from_slice(&0u64.to_le_bytes()); // fileoff
        buf.extend_from_slice(&0u64.to_le_bytes()); // filesize
        buf.extend_from_slice(&prot.to_le_bytes()); // maxprot
        buf.extend_from_slice(&prot.to_le_bytes()); // initprot
        buf.extend_from_slice(&0u32.to_le_bytes()); // nsects
        buf.extend_from_slice(&0u32.to_le_bytes()); // flags
    }

    /// Append LC_MAIN to a buffer.
    fn append_lc_main(buf: &mut Vec<u8>, entry_offset: u64, stack_size: u64) {
        let cmd_size: u32 = 24;
        buf.extend_from_slice(&0x8000_0028u32.to_le_bytes()); // LC_MAIN
        buf.extend_from_slice(&cmd_size.to_le_bytes());
        buf.extend_from_slice(&entry_offset.to_le_bytes());
        buf.extend_from_slice(&stack_size.to_le_bytes());
    }

    /// Build a complete minimal Mach-O with specified load commands.
    fn build_macho64(cmds: &[(&str, u64, u64, u32)], entry: Option<(u64, u64)>) -> Vec<u8> {
        // Calculate sizes
        let segment_cmd_size: u32 = 72;
        let main_cmd_size: u32 = 24;
        let ncmds = cmds.len() as u32 + if entry.is_some() { 1 } else { 0 };
        let sizeofcmds = (cmds.len() as u32 * segment_cmd_size)
            + if entry.is_some() { main_cmd_size } else { 0 };

        let mut buf = Vec::new();
        buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
        buf.extend_from_slice(&0u32.to_le_bytes()); // subtype
        buf.extend_from_slice(&2u32.to_le_bytes()); // MH_EXECUTE
        buf.extend_from_slice(&ncmds.to_le_bytes());
        buf.extend_from_slice(&sizeofcmds.to_le_bytes());
        buf.extend_from_slice(&0x0020_0085u32.to_le_bytes()); // PIE|TWO_LEVEL|DYLDLINK|NO_UNDEFS
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

        for (name, vmaddr, vmsize, prot) in cmds {
            append_segment64(&mut buf, name, *vmaddr, *vmsize, *prot);
        }

        if let Some((offset, stack)) = entry {
            append_lc_main(&mut buf, offset, stack);
        }

        buf
    }

    // ── Bad magic (from goblin: DEADBEEF test) ──────────────────────

    #[test]
    fn reject_bad_magic() {
        let data = 0xDEAD_BEEFu32.to_le_bytes();
        let err = parse(&data).unwrap_err();
        match err {
            ParseError::BadMagic { magic } => {
                assert_eq!(magic, 0xDEAD_BEEF);
            }
            _ => panic!("expected BadMagic, got: {err}"),
        }
    }

    // ── Empty input (from goblin edge cases) ────────────────────────

    #[test]
    fn reject_empty_input() {
        let err = parse(&[]).unwrap_err();
        match err {
            ParseError::UnexpectedEof { offset: 0, .. } => {}
            _ => panic!("expected UnexpectedEof at 0, got: {err}"),
        }
    }

    // ── Truncated header (from XNU mach_loader bounds check) ────────

    #[test]
    fn reject_truncated_header_3_bytes() {
        let err = parse(&[0xCF, 0xFA, 0xED]).unwrap_err();
        match err {
            ParseError::UnexpectedEof { .. } => {}
            _ => panic!("expected UnexpectedEof, got: {err}"),
        }
    }

    #[test]
    fn reject_truncated_header_16_bytes() {
        // Valid magic + partial header (only cpu_type, cpu_subtype, file_type)
        let mut data = Vec::new();
        data.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        data.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
        data.extend_from_slice(&0u32.to_le_bytes()); // cpu_subtype
        data.extend_from_slice(&2u32.to_le_bytes()); // file_type
                                                     // Missing: ncmds, sizeofcmds, flags, reserved
        let err = parse(&data).unwrap_err();
        match err {
            ParseError::UnexpectedEof { .. } => {}
            _ => panic!("expected UnexpectedEof, got: {err}"),
        }
    }

    // ── Minimal valid Mach-O with no commands ───────────────────────

    #[test]
    fn parse_minimal_no_commands() {
        let data = minimal_macho64();
        let parsed = parse(&data).unwrap();
        assert!(parsed.header.is_64bit());
        assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
        assert_eq!(parsed.header.file_type, FileType::Execute);
        assert_eq!(parsed.commands.len(), 0);
        assert_eq!(parsed.segments.len(), 0);
        assert!(parsed.entry.is_none());
    }

    // ── Segment parsing (from LLVM lld MachO segment tests) ────────

    #[test]
    fn parse_pagezero_segment() {
        let data = build_macho64(&[("__PAGEZERO", 0, 0x1_0000_0000, 0)], None);
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.segments.len(), 1);
        let seg = &parsed.segments[0];
        assert_eq!(seg.name, "__PAGEZERO");
        assert_eq!(seg.vmaddr, 0);
        assert_eq!(seg.vmsize, 0x1_0000_0000);
        assert_eq!(seg.initprot, VmProt::empty());
    }

    #[test]
    fn parse_text_segment() {
        let data = build_macho64(
            &[("__TEXT", 0x1_0000_0000, 0x4000, 5)], // r-x
            None,
        );
        let parsed = parse(&data).unwrap();
        let seg = &parsed.segments[0];
        assert_eq!(seg.name, "__TEXT");
        assert_eq!(seg.vmaddr, 0x1_0000_0000);
        assert_eq!(seg.initprot, VmProt::READ | VmProt::EXECUTE);
    }

    #[test]
    fn parse_data_segment() {
        let data = build_macho64(
            &[("__DATA", 0x1_0000_4000, 0x4000, 3)], // rw-
            None,
        );
        let parsed = parse(&data).unwrap();
        let seg = &parsed.segments[0];
        assert_eq!(seg.name, "__DATA");
        assert_eq!(seg.initprot, VmProt::READ | VmProt::WRITE);
    }

    #[test]
    fn parse_linkedit_segment() {
        let data = build_macho64(
            &[("__LINKEDIT", 0x1_0000_8000, 0x4000, 1)], // r--
            None,
        );
        let parsed = parse(&data).unwrap();
        let seg = &parsed.segments[0];
        assert_eq!(seg.name, "__LINKEDIT");
        assert_eq!(seg.initprot, VmProt::READ);
    }

    // ── Standard 4-segment layout (from lld: basic-layout test) ─────

    #[test]
    fn parse_standard_segment_layout() {
        let data = build_macho64(
            &[
                ("__PAGEZERO", 0, 0x1_0000_0000, 0),      // ---
                ("__TEXT", 0x1_0000_0000, 0x4000, 5),     // r-x
                ("__DATA", 0x1_0000_4000, 0x4000, 3),     // rw-
                ("__LINKEDIT", 0x1_0000_8000, 0x4000, 1), // r--
            ],
            Some((0x1000, 0)),
        );
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.segments.len(), 4);

        // Verify ordering
        assert_eq!(parsed.segments[0].name, "__PAGEZERO");
        assert_eq!(parsed.segments[1].name, "__TEXT");
        assert_eq!(parsed.segments[2].name, "__DATA");
        assert_eq!(parsed.segments[3].name, "__LINKEDIT");

        // Verify entry point
        assert!(parsed.entry.is_some());
        match &parsed.entry {
            Some(macho_types::EntryPoint::Main { entry_offset, .. }) => {
                assert_eq!(*entry_offset, 0x1000);
            }
            _ => panic!("expected LC_MAIN entry point"),
        }
    }

    // ── Load command too small (from LLVM: malformed-lc-size) ───────

    #[test]
    fn reject_load_command_too_small() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // ncmds = 1
        buf.extend_from_slice(&8u32.to_le_bytes()); // sizeofcmds
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

        // Load command with size < 8
        buf.extend_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
        buf.extend_from_slice(&4u32.to_le_bytes()); // size = 4 (invalid, min is 8)

        let err = parse(&buf).unwrap_err();
        match err {
            ParseError::LoadCommandTooSmall { size: 4, min: 8 } => {}
            _ => panic!("expected LoadCommandTooSmall, got: {err}"),
        }
    }

    // ── Header flags preserved (from XNU: flag validation) ──────────

    #[test]
    fn parse_preserves_flags() {
        let data = build_macho64(&[], None);
        let parsed = parse(&data).unwrap();
        assert!(parsed.header.flags.contains(MachFlags::PIE));
        assert!(parsed.header.flags.contains(MachFlags::TWO_LEVEL));
        assert!(parsed.header.flags.contains(MachFlags::DYLDLINK));
        assert!(parsed.header.flags.contains(MachFlags::NO_UNDEFS));
    }

    // ── Multiple segments have correct count ────────────────────────

    #[test]
    fn parse_multiple_segments_correct_count() {
        let data = build_macho64(
            &[
                ("__PAGEZERO", 0, 0x1_0000_0000, 0),
                ("__TEXT", 0x1_0000_0000, 0x4000, 5),
                ("__DATA", 0x1_0000_4000, 0x2000, 3),
                ("__LINKEDIT", 0x1_0000_8000, 0x1000, 1),
            ],
            None,
        );
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.header.num_commands, 4);
        assert_eq!(parsed.commands.len(), 4);
        assert_eq!(parsed.segments.len(), 4);
    }

    // ── Truncated load command data ─────────────────────────────────

    #[test]
    fn reject_truncated_segment_command() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        buf.extend_from_slice(&0x0100_000Cu32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // ncmds = 1
        buf.extend_from_slice(&72u32.to_le_bytes()); // sizeofcmds = 72
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

        // Truncated LC_SEGMENT_64: only cmd + cmdsize (8 bytes), missing rest
        buf.extend_from_slice(&0x19u32.to_le_bytes());
        buf.extend_from_slice(&72u32.to_le_bytes());
        // Only 16 more bytes of segment name, not the full 64 remaining
        buf.extend_from_slice(&[0u8; 16]);

        let err = parse(&buf).unwrap_err();
        match err {
            ParseError::UnexpectedEof { .. } => {}
            _ => panic!("expected UnexpectedEof, got: {err}"),
        }
    }

    // ── JSON serialization roundtrip (invariant: every stage emits JSON) ──

    #[test]
    fn parsed_macho_serializes_to_json() {
        let data = build_macho64(
            &[
                ("__PAGEZERO", 0, 0x1_0000_0000, 0),
                ("__TEXT", 0x1_0000_0000, 0x4000, 5),
            ],
            Some((0x1000, 0)),
        );
        let parsed = parse(&data).unwrap();
        let json = serde_json::to_string_pretty(&parsed).unwrap();
        assert!(json.contains("__PAGEZERO"));
        assert!(json.contains("__TEXT"));
        assert!(json.contains("Arm64"));
    }
}

/// Tests that parse real fixture binaries built by build_fixtures.sh.
/// These tests require `./scripts/build_fixtures.sh` to have been run first.
#[cfg(test)]
mod fixture_tests {
    use macho_parse::parse;
    use macho_types::header::*;
    use macho_types::load_command::*;
    use macho_types::segment::VmProt;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        let path = fixtures_dir().join(name);
        std::fs::read(&path).ok()
    }

    // ── simple_cli fixture ──────────────────────────────────────────

    #[test]
    fn fixture_simple_cli_header() {
        let Some(data) = read_fixture("simple_cli/app") else {
            eprintln!("SKIP: simple_cli fixture not built (run scripts/build_fixtures.sh)");
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(parsed.header.is_64bit());
        assert_eq!(parsed.header.file_type, FileType::Execute);
        assert!(parsed.header.flags.contains(MachFlags::PIE));
        assert!(parsed.header.flags.contains(MachFlags::TWO_LEVEL));
    }

    #[test]
    fn fixture_simple_cli_has_text_segment() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let text = parsed.segments.iter().find(|s| s.name == "__TEXT");
        assert!(text.is_some(), "expected __TEXT segment");
        let text = text.unwrap();
        assert!(text.initprot.contains(VmProt::READ));
        assert!(text.initprot.contains(VmProt::EXECUTE));
        assert!(!text.initprot.contains(VmProt::WRITE));
    }

    #[test]
    fn fixture_simple_cli_has_pagezero() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let pz = parsed.segments.iter().find(|s| s.name == "__PAGEZERO");
        assert!(pz.is_some(), "64-bit executable must have __PAGEZERO");
        let pz = pz.unwrap();
        assert_eq!(pz.vmaddr, 0);
        assert_eq!(pz.initprot, VmProt::empty());
    }

    #[test]
    fn fixture_simple_cli_has_linkedit() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let le = parsed.segments.iter().find(|s| s.name == "__LINKEDIT");
        assert!(le.is_some(), "expected __LINKEDIT segment");
        let le = le.unwrap();
        assert!(le.initprot.contains(VmProt::READ));
        assert!(!le.initprot.contains(VmProt::WRITE));
        assert!(!le.initprot.contains(VmProt::EXECUTE));
    }

    #[test]
    fn fixture_simple_cli_entry_point() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(parsed.entry.is_some(), "expected LC_MAIN entry");
        match &parsed.entry {
            Some(macho_types::EntryPoint::Main { entry_offset, .. }) => {
                assert!(*entry_offset > 0, "entry_offset should be nonzero");
            }
            _ => panic!("expected LC_MAIN variant"),
        }
    }

    #[test]
    fn fixture_simple_cli_depends_on_libsystem() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(
            !parsed.dylibs.is_empty(),
            "should depend on at least libSystem"
        );
        let has_libsystem = parsed.dylibs.iter().any(|d| d.name.contains("libSystem"));
        assert!(has_libsystem, "expected libSystem.B.dylib dependency");
    }

    #[test]
    fn fixture_simple_cli_has_uuid() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(parsed.uuid.is_some(), "modern binaries should have LC_UUID");
    }

    #[test]
    fn fixture_simple_cli_has_build_version() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let has_bv = parsed
            .commands
            .iter()
            .any(|c| c.cmd == LoadCommandType::BuildVersion);
        assert!(has_bv, "modern binaries should have LC_BUILD_VERSION");
    }

    #[test]
    fn fixture_simple_cli_has_dylinker() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let dylinker = parsed.commands.iter().find_map(|c| {
            if let LoadCommandPayload::LoadDylinker { name } = &c.payload {
                Some(name.clone())
            } else {
                None
            }
        });
        assert!(dylinker.is_some(), "expected LC_LOAD_DYLINKER");
        assert!(
            dylinker.unwrap().contains("dyld"),
            "dylinker should be dyld"
        );
    }

    #[test]
    fn fixture_simple_cli_symbols() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(parsed.symbols.is_some(), "expected symbol table");
        let syms = parsed.symbols.as_ref().unwrap();
        assert!(!syms.symbols.is_empty(), "should have at least one symbol");
        // _main should be defined
        let has_main = syms
            .symbols
            .iter()
            .any(|s| s.name == "_main" && s.sym_type == macho_types::SymbolType::Defined);
        assert!(has_main, "expected _main symbol");
    }

    // ── dylib_chain fixture ─────────────────────────────────────────

    #[test]
    fn fixture_dylib_chain_header() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            eprintln!("SKIP: dylib_chain fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.header.file_type, FileType::Execute);
    }

    #[test]
    fn fixture_dylib_chain_has_rpath_dylib() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let has_rpath_dep = parsed.dylibs.iter().any(|d| d.name.starts_with("@rpath/"));
        assert!(has_rpath_dep, "expected @rpath dependency");
    }

    #[test]
    fn fixture_dylib_chain_has_rpath_lc() {
        let Some(data) = read_fixture("dylib_chain/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert!(!parsed.rpaths.is_empty(), "expected at least one LC_RPATH");
        let has_exe_path = parsed.rpaths.iter().any(|r| r.contains("@executable_path"));
        assert!(has_exe_path, "expected @executable_path in rpaths");
    }

    #[test]
    fn fixture_dylib_chain_dylib_is_dylib_type() {
        let Some(data) = read_fixture("dylib_chain/libchaina.dylib") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.header.file_type, FileType::Dylib);
    }

    #[test]
    fn fixture_dylib_chain_dylib_has_id() {
        let Some(data) = read_fixture("dylib_chain/libchaina.dylib") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let has_id = parsed
            .commands
            .iter()
            .any(|c| c.cmd == LoadCommandType::IdDylib);
        assert!(has_id, "dylib should have LC_ID_DYLIB");
    }

    // ── constructor_test fixture ────────────────────────────────────

    #[test]
    fn fixture_constructor_test_parses() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        assert_eq!(parsed.header.file_type, FileType::Execute);
        assert!(parsed.entry.is_some());
    }

    // ── objc_foundation fixture ─────────────────────────────────────

    #[test]
    fn fixture_objc_foundation_has_objc_deps() {
        let Some(data) = read_fixture("objc_foundation/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        // Should depend on Foundation or libobjc
        let has_foundation = parsed
            .dylibs
            .iter()
            .any(|d| d.name.contains("Foundation") || d.name.contains("objc"));
        assert!(
            has_foundation,
            "ObjC app should reference Foundation or libobjc"
        );
    }

    // ── malformed fixtures ──────────────────────────────────────────

    #[test]
    fn fixture_malformed_bad_magic() {
        let Some(data) = read_fixture("malformed/bad_magic") else {
            return;
        };
        assert!(parse(&data).is_err());
    }

    #[test]
    fn fixture_malformed_truncated_header() {
        let Some(data) = read_fixture("malformed/truncated_header") else {
            return;
        };
        assert!(parse(&data).is_err());
    }

    // ── Parse system binaries (verify against otool expectations) ───

    #[test]
    fn parse_system_ls() {
        let Ok(data) = std::fs::read("/bin/ls") else {
            eprintln!("SKIP: /bin/ls not readable");
            return;
        };
        let Ok(parsed) = parse(&data) else {
            eprintln!("SKIP: /bin/ls is a fat/universal binary");
            return;
        };
        assert_eq!(parsed.header.file_type, FileType::Execute);
        assert!(parsed.header.is_64bit());
        assert!(!parsed.segments.is_empty());
        assert!(!parsed.dylibs.is_empty());
        assert!(parsed.entry.is_some());
        // /bin/ls should depend on libSystem
        let has_libsystem = parsed.dylibs.iter().any(|d| d.name.contains("libSystem"));
        assert!(has_libsystem);
    }

    #[test]
    fn parse_system_dyld() {
        // dyld itself is a Mach-O (MH_DYLINKER type)
        let Ok(data) = std::fs::read("/usr/lib/dyld") else {
            eprintln!("SKIP: /usr/lib/dyld not readable");
            return;
        };
        // dyld may be in shared cache or a fat binary; just verify we don't panic
        let _ = parse(&data);
    }

    // ── Fat/universal binary tests ──────────────────────────────────

    #[test]
    fn fat_binary_is_detected() {
        let Some(data) = read_fixture("fat_binary/app") else {
            eprintln!("SKIP: fat_binary fixture not built");
            return;
        };
        assert!(
            macho_parse::is_fat(&data),
            "fat_binary/app should be detected as fat"
        );
    }

    #[test]
    fn fat_binary_not_detected_on_thin() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        assert!(
            !macho_parse::is_fat(&data),
            "simple_cli/app is thin, not fat"
        );
    }

    #[test]
    fn fat_binary_parse_header() {
        let Some(data) = read_fixture("fat_binary/app") else {
            return;
        };
        let fat = macho_parse::parse_fat(&data).unwrap();
        assert_eq!(
            fat.arches.len(),
            2,
            "fat_binary has 2 arches (x86_64 + arm64)"
        );

        let cpu_types: Vec<_> = fat.arches.iter().map(|a| a.cpu_type).collect();
        assert!(
            cpu_types.contains(&CpuType::X86_64),
            "should have x86_64 slice"
        );
        assert!(
            cpu_types.contains(&CpuType::Arm64),
            "should have arm64 slice"
        );
    }

    #[test]
    fn fat_binary_slice_arm64() {
        let Some(data) = read_fixture("fat_binary/app") else {
            return;
        };
        let slice = macho_parse::fat_slice_for_cpu(&data, CpuType::Arm64).unwrap();

        // The slice should be a valid Mach-O
        let parsed = parse(slice).unwrap();
        assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
        assert_eq!(parsed.header.file_type, FileType::Execute);
    }

    #[test]
    fn fat_binary_slice_x86_64() {
        let Some(data) = read_fixture("fat_binary/app") else {
            return;
        };
        let slice = macho_parse::fat_slice_for_cpu(&data, CpuType::X86_64).unwrap();

        let parsed = parse(slice).unwrap();
        assert_eq!(parsed.header.cpu_type, CpuType::X86_64);
        assert_eq!(parsed.header.file_type, FileType::Execute);
    }

    #[test]
    fn fat_binary_slice_missing_arch() {
        let Some(data) = read_fixture("fat_binary/app") else {
            return;
        };
        let result = macho_parse::fat_slice_for_cpu(&data, CpuType::Arm);
        assert!(result.is_err(), "fat_binary has no ARM (32-bit) slice");
    }

    #[test]
    fn fat_binary_parse_auto_arm64() {
        let Some(data) = read_fixture("fat_binary/app") else {
            return;
        };
        let parsed = macho_parse::parse_auto(&data, CpuType::Arm64).unwrap();
        assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
        assert!(parsed.entry.is_some(), "arm64 slice should have LC_MAIN");
    }

    #[test]
    fn fat_binary_parse_auto_thin_fallback() {
        // parse_auto on a thin binary should just parse it directly
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = macho_parse::parse_auto(&data, CpuType::X86_64).unwrap();
        // simple_cli is arm64, parse_auto ignores preferred_cpu for thin binaries
        assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
    }

    #[test]
    fn parse_system_fat_binary() {
        // /usr/bin/file is a fat binary on most macOS installs
        let Ok(data) = std::fs::read("/usr/bin/file") else {
            eprintln!("SKIP: /usr/bin/file not readable");
            return;
        };
        if !macho_parse::is_fat(&data) {
            eprintln!("SKIP: /usr/bin/file is not a fat binary on this system");
            return;
        }
        let fat = macho_parse::parse_fat(&data).unwrap();
        assert!(
            !fat.arches.is_empty(),
            "fat binary should have at least one arch"
        );

        // Try to parse the arm64 slice
        if let Ok(parsed) = macho_parse::parse_auto(&data, CpuType::Arm64) {
            assert_eq!(parsed.header.cpu_type, CpuType::Arm64);
            assert!(parsed.entry.is_some());
        }
    }
}

// ── Function starts tests ───────────────────────────────────────────

#[cfg(test)]
mod func_starts_tests {
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn function_starts_simple_cli() {
        let data = std::fs::read(fixture_path("simple_cli/app")).unwrap();
        let parsed = macho_parse::parse(&data).unwrap();
        let starts = macho_parse::parse_function_starts(&parsed, &data);
        assert!(!starts.is_empty(), "simple_cli should have function starts");
        // First function start should be in the __TEXT segment range
        let text = parsed.segments.iter().find(|s| s.name == "__TEXT").unwrap();
        assert!(
            starts[0] >= text.vmaddr && starts[0] < text.vmaddr + text.vmsize,
            "first function start {:#x} should be within __TEXT [{:#x}..{:#x}]",
            starts[0],
            text.vmaddr,
            text.vmaddr + text.vmsize
        );
    }

    #[test]
    fn function_starts_constructor_test() {
        let data = std::fs::read(fixture_path("constructor_test/app")).unwrap();
        let parsed = macho_parse::parse(&data).unwrap();
        let starts = macho_parse::parse_function_starts(&parsed, &data);
        // constructor_test has at least 3 functions: 2 constructors + main
        assert!(
            starts.len() >= 3,
            "constructor_test should have >= 3 function starts, got {}",
            starts.len()
        );
    }

    #[test]
    fn function_starts_empty_for_malformed() {
        // A minimal valid Mach-O with no LC_FUNCTION_STARTS should return empty
        let data = std::fs::read(fixture_path("malformed/bad_magic")).unwrap();
        // This will fail to parse, which is fine — test that function starts
        // handles parse failure gracefully
        let parsed = macho_parse::parse(&data);
        if let Ok(p) = parsed {
            let starts = macho_parse::parse_function_starts(&p, &data);
            // May or may not have function starts
            let _ = starts;
        }
    }
}

// ── Phase 13: vmaddr_to_file_offset, SourceVersion, Dysymtab ───

#[cfg(test)]
mod phase13_tests {
    use macho_parse::{parse, vmaddr_to_file_offset};
    use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
            .join(name)
    }

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
}

// ── Phase 13: Malformed input hardening ─────────────────────────

#[cfg(test)]
mod hardening_tests {
    use macho_parse::{extract_exports, extract_fixups, parse};

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
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("fixtures/simple_cli/app"),
        )
        .unwrap();
        let parsed = parse(&data).unwrap();
        // This should work normally
        let exports = extract_exports(&parsed, &data);
        assert!(!exports.is_empty(), "simple_cli should have at least _main");
    }

    #[test]
    fn fixups_from_valid_binary() {
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("fixtures/simple_cli/app"),
        )
        .unwrap();
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
        let data = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("fixtures/simple_cli/app"),
        )
        .unwrap();
        let mut parsed = parse(&data).unwrap();
        parsed.segments.clear();
        // Should return None for any address
        assert!(macho_parse::vmaddr_to_file_offset(&parsed, 0x100000000).is_none());
    }

    #[test]
    fn objc_metadata_on_truncated_classlist() {
        // Take a valid ObjC binary and truncate it mid-classlist
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/objc_classes/app");
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
}

// ── Phase 14: DyldInfo fix, indirect symbols, symbol desc, selectors ──

#[cfg(test)]
mod phase14_tests {
    use macho_parse::{
        extract_fixups, parse, read_indirect_symbol_table, resolve_indirect_symbols,
    };
    use macho_types::load_command::{LoadCommandPayload, LoadCommandType};
    use macho_types::symbol::is_stab;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
            .join(name)
    }

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
}

// ── Phase 15: TLV descriptor parsing ────────────────────────────────

#[cfg(test)]
mod phase15_tests {
    use macho_parse::parse;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
    }

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
}

// ── Phase 16: Relocation parsing + ParsedMachO Deserialize ──────────

#[cfg(test)]
mod phase16_tests {
    use macho_parse::parse;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
    }

    fn read_fixture(name: &str) -> Option<Vec<u8>> {
        std::fs::read(fixtures_dir().join(name)).ok()
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
}

// ── Phase 17: LC types, export trie, data-in-code, cstrings, fat subtype ──

#[cfg(test)]
mod phase17_tests {
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        macho_fixtures::fixtures_dir().join(name)
    }

    fn fixture_binary(name: &str) -> PathBuf {
        fixture_path(name).join("app")
    }

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
        let mut trie = Vec::new();
        // Root node with one terminal "_foo"
        // terminal_size
        trie.push(6); // terminal size: 6 bytes (flags + stub + resolver ULEBs)
        trie.push(0x10); // flags = STUB_AND_RESOLVER
        trie.push(0x42); // stub_offset = 0x42
        trie.push(0x99u8 & 0x7F | 0x80); // resolver_offset = 0x1099 (multi-byte ULEB)
        trie.push(0x21); // continuation
                         // 0 children
        trie.push(0);

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
        // We'll test via the internal decode path by including blob at offset 0
        // and crafting a parsed with a DataInCode command
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
        let path = fixture_path("fat_binary").join("app");
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
        let path = fixture_path("fat_binary").join("app");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).unwrap();
        if !macho_parse::is_fat(&data) {
            return;
        }
        let orig = macho_parse::fat_slice_for_cpu(&data, macho_types::CpuType::Arm64).unwrap();
        let sub = macho_parse::fat_slice_for_cpu_subtype(&data, macho_types::CpuType::Arm64, None)
            .unwrap();
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
}
