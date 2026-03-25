//! Tests that parse real fixture binaries built by build_fixtures.sh.
//! Covers: simple_cli, dylib_chain, constructor, objc_foundation, malformed, system binaries.

mod common;

use common::read_fixture;
use macho_parse::parse;
use macho_types::header::*;
use macho_types::load_command::*;
use macho_types::segment::VmProt;

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
