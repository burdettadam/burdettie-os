//! Tests for constructor/initializer extraction and ObjC image info parsing.
//!
//! Covers: extract_initializers, ConstructorSource, InitializerInfo,
//! ObjcImageInfo flag helpers, system binary parsing.

mod common;

use macho_objc::{extract_initializers, ConstructorSource, InitializerInfo, ObjcImageInfo};
use macho_parse::parse;

// ── Constructor / initializer extraction ────────────────────────────

#[test]
fn simple_cli_has_no_constructors() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);
    assert!(
        info.constructors.is_empty(),
        "plain C binary should have no constructors"
    );
    assert!(
        !info.has_objc_load,
        "plain C binary should not have ObjC +load"
    );
}

#[test]
fn constructor_test_has_constructors() {
    let Some(data) = common::read_fixture("constructor_test/app") else {
        eprintln!("SKIP: constructor_test fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);
    assert!(
        !info.constructors.is_empty(),
        "constructor_test should have at least one constructor"
    );
}

#[test]
fn constructor_test_has_two_constructors() {
    // main.c defines two __attribute__((constructor)) functions
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);
    assert_eq!(
        info.constructors.len(),
        2,
        "constructor_test defines exactly 2 constructors, got {}",
        info.constructors.len()
    );
}

#[test]
fn constructor_test_uses_init_offsets_or_mod_init() {
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);

    for ctor in &info.constructors {
        match ctor.source {
            ConstructorSource::ModInitFunc | ConstructorSource::InitOffsets => {}
        }
        assert!(ctor.address > 0, "constructor address should be nonzero");
    }
}

#[test]
fn constructor_test_has_init_steps() {
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);
    assert!(
        !info.init_steps.is_empty(),
        "constructor_test should have init steps"
    );
    // Each constructor generates an init_step
    for step in &info.init_steps {
        assert!(
            step.contains("constructor") || step.contains("ObjC"),
            "init step should describe a constructor or ObjC load: {step}"
        );
    }
}

#[test]
fn constructor_test_no_objc_load() {
    let Some(data) = common::read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let info = extract_initializers(&parsed, &data);
    assert!(!info.has_objc_load, "C constructor_test has no ObjC");
}

#[test]
fn initializer_info_serializes() {
    let info = InitializerInfo {
        constructors: vec![macho_objc::ConstructorEntry {
            address: 0x100001000,
            source: ConstructorSource::ModInitFunc,
        }],
        has_objc_load: false,
        init_steps: vec!["constructor[0] at 0x100001000 (ModInitFunc)".to_string()],
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("ModInitFunc"));
    let roundtrip: InitializerInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip.constructors.len(), 1);
}

// ── ObjC imageinfo tests ────────────────────────────────────────────

#[test]
fn imageinfo_supports_objc2() {
    let info = ObjcImageInfo {
        version: 0,
        flags: 0x20,
    };
    assert!(info.supports_objc2());
    assert!(!info.requires_gc());
    assert!(!info.is_simulated());
}

#[test]
fn imageinfo_requires_gc() {
    let info = ObjcImageInfo {
        version: 0,
        flags: 0x04,
    };
    assert!(info.requires_gc());
}

#[test]
fn imageinfo_simulated() {
    let info = ObjcImageInfo {
        version: 0,
        flags: 0x200,
    };
    assert!(info.is_simulated());
}

#[test]
fn imageinfo_swift_version() {
    // Swift 5 = 0x0700 in flags (version 7 at bits 8-15)
    let info = ObjcImageInfo {
        version: 0,
        flags: 0x0700,
    };
    assert_eq!(info.swift_version(), 7);
}

#[test]
fn imageinfo_parse_system_binary() {
    // Try a system binary that has ObjC metadata
    let path = std::path::Path::new("/usr/bin/shortcuts");
    if !path.exists() {
        eprintln!("SKIP: /usr/bin/shortcuts not available");
        return;
    }
    let data = std::fs::read(path).unwrap();
    // May be a fat binary — try to extract arm64
    let bin_data = if macho_parse::is_fat(&data) {
        match macho_parse::fat_slice_for_cpu(&data, macho_types::header::CpuType::Arm64) {
            Ok(slice) => slice.to_vec(),
            Err(_) => data.clone(),
        }
    } else {
        data.clone()
    };
    if let Ok(parsed) = macho_parse::parse(&bin_data) {
        let meta = macho_objc::parse_objc_metadata(&parsed, &bin_data);
        // shortcuts may or may not have ObjC; just verify no crash
        let _ = meta;
    }
}
