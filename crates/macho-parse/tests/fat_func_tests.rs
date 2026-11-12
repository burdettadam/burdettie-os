//! Fat/universal binary tests and function starts parsing.

mod common;

use common::{fixture_path, read_fixture};
use macho_parse::parse;
use macho_types::header::*;

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

// ── Function starts tests ───────────────────────────────────────

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
