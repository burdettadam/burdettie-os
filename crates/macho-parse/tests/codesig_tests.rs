//! Tests for code signature parsing and security-gated execution.

use std::path::PathBuf;

use macho_parse::parse;
use macho_types::codesig::*;

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

// ── Code signature parsing tests ────────────────────────────────────

#[test]
fn simple_cli_has_code_signature() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    assert!(
        parsed.code_signature.is_some(),
        "simple_cli should have a parsed code signature"
    );
}

#[test]
fn simple_cli_superblob_structure() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().unwrap();

    assert_eq!(cs.super_blob.magic, CSMAGIC_EMBEDDED_SIGNATURE);
    assert!(cs.super_blob.count >= 1, "should have at least 1 blob");
    assert_eq!(cs.super_blob.index.len(), cs.super_blob.count as usize);

    // First blob should be CodeDirectory
    assert_eq!(cs.super_blob.index[0].slot_type, CsSlotType::CodeDirectory);
}

#[test]
fn simple_cli_code_directory() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().unwrap();
    let cd = cs
        .code_directory
        .as_ref()
        .expect("should have CodeDirectory");

    // Version should be a known value (e.g., 0x20400)
    assert!(
        cd.version >= 0x20001,
        "version should be at least 2.0.1: {:#x}",
        cd.version
    );

    // Signature type: ad-hoc, linker-signed
    assert!(
        cd.flags.contains(CsFlags::ADHOC),
        "simple_cli should be ad-hoc signed"
    );
    assert!(
        cd.flags.contains(CsFlags::LINKER_SIGNED),
        "simple_cli should be linker-signed"
    );

    // Hash type should be SHA-256
    assert_eq!(cd.hash_type, CsHashType::Sha256);
    assert_eq!(cd.hash_size, 32, "SHA-256 hash size should be 32 bytes");

    // Should have page hashes
    assert!(cd.n_code_slots > 0, "should have code slot hashes");
    assert_eq!(cd.page_hashes.len(), cd.n_code_slots as usize);

    // Each hash should be 32 bytes
    for (i, hash) in cd.page_hashes.iter().enumerate() {
        assert_eq!(hash.len(), 32, "page hash {i} should be 32 bytes");
    }

    // Identity should be "app"
    assert_eq!(cd.identity, "app");

    // Team ID should be empty for ad-hoc
    assert!(
        cd.team_id.is_empty(),
        "ad-hoc signature should have no team ID"
    );
}

#[test]
fn simple_cli_cd_hash_matches_codesign() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().unwrap();
    let cd = cs.code_directory.as_ref().unwrap();

    // CDHash should be 32 bytes (SHA-256)
    assert_eq!(cd.cd_hash.len(), 32, "CDHash should be 32 bytes");

    // CDHash should be non-zero
    assert!(
        cd.cd_hash.iter().any(|&b| b != 0),
        "CDHash should not be all zeros"
    );

    // Verify it matches the codesign output (first 20 bytes = CDHash displayed by codesign)
    let hash_hex: String = cd.cd_hash.iter().map(|b| format!("{b:02x}")).collect();
    // codesign reports CDHash=7cad795285f7f4f76e40c3fe1055d1904416f746
    // (first 20 bytes = 40 hex chars of the full SHA-256)
    let expected_prefix = "7cad795285f7f4f76e40c3fe1055d1904416f746";
    assert!(
        hash_hex.starts_with(expected_prefix),
        "CDHash should start with {expected_prefix}, got {hash_hex}"
    );
}

#[test]
fn simple_cli_no_entitlements() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().unwrap();

    // Linker-signed fixture should not have entitlements
    assert!(
        cs.entitlements_plist.is_none(),
        "simple_cli should not have entitlements"
    );
}

#[test]
fn simple_cli_no_cms_signature() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().unwrap();

    // Ad-hoc signatures have no CMS blob
    assert!(
        !cs.has_cms_signature,
        "ad-hoc signature should not have CMS blob"
    );
}

#[test]
fn return42_has_code_signature() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    let cs = parsed
        .code_signature
        .as_ref()
        .expect("return42 should have code signature");
    let cd = cs.code_directory.as_ref().unwrap();

    assert!(cd.flags.contains(CsFlags::ADHOC));
    assert!(cd.flags.contains(CsFlags::LINKER_SIGNED));
    assert_eq!(cd.hash_type, CsHashType::Sha256);
}

#[test]
fn all_fixtures_have_code_signatures() {
    let fixtures = [
        "simple_cli/app",
        "return42/return42",
        "dylib_chain/app",
        "framework_style/app",
        "constructor_test/app",
    ];

    for fixture in &fixtures {
        let Some(data) = read_fixture(fixture) else {
            continue;
        };
        let parsed = parse(&data).unwrap();
        assert!(
            parsed.code_signature.is_some(),
            "{fixture} should have a code signature"
        );
    }
}

#[test]
fn cs_flags_no_runtime_for_fixture() {
    // Our fixtures are simple linker-signed binaries — they should NOT
    // have CS_RUNTIME (hardened runtime) set
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cd = parsed
        .code_signature
        .as_ref()
        .unwrap()
        .code_directory
        .as_ref()
        .unwrap();

    assert!(
        !cd.flags.contains(CsFlags::RUNTIME),
        "fixture should not have hardened runtime"
    );
    assert!(
        !cd.flags.contains(CsFlags::RESTRICT),
        "fixture should not have CS_RESTRICT"
    );
}

// ── Malformed code signature edge cases ─────────────────────────────

#[test]
fn parse_codesig_too_short() {
    let result = macho_parse::parse_code_signature(&[0u8; 4]);
    assert!(result.is_err());
}

#[test]
fn parse_codesig_bad_magic() {
    let mut buf = [0u8; 12];
    buf[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
    buf[4..8].copy_from_slice(&12u32.to_be_bytes());
    buf[8..12].copy_from_slice(&0u32.to_be_bytes());
    let result = macho_parse::parse_code_signature(&buf);
    assert!(result.is_err());
}

// ── Page hash verification tests ───────────────────────────────────

#[test]
fn page_hashes_verify_clean_binary() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().expect("should have codesig");
    let cd = cs
        .code_directory
        .as_ref()
        .expect("should have codedirectory");

    assert!(!cd.page_hashes.is_empty(), "should have page hashes");

    let result = macho_parse::verify_page_hashes(&data, cd);
    assert!(
        result.mismatches.is_empty(),
        "untampered binary should have no mismatches, got {} mismatches in {} slots",
        result.mismatches.len(),
        cd.page_hashes.len(),
    );
    assert_eq!(result.verified, cd.n_code_slots as usize);
}

#[test]
fn page_hashes_detect_tampered_binary() {
    let Some(mut data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().expect("should have codesig");
    let cd = cs
        .code_directory
        .as_ref()
        .expect("should have codedirectory");

    // Tamper with a byte in the __TEXT segment (page 0)
    // This should cause a page hash mismatch without breaking the parse.
    let tamper_offset = 0x100; // well within the first page
    data[tamper_offset] ^= 0xFF;

    let result = macho_parse::verify_page_hashes(&data, cd);
    assert!(
        !result.mismatches.is_empty(),
        "tampered binary should have at least one page hash mismatch",
    );
    // The tampered page should be slot 0 (offset 0x100 is in first page)
    assert_eq!(
        result.mismatches[0].slot, 0,
        "mismatch should be in page slot 0",
    );
}

#[test]
fn page_hashes_verify_constructor_test() {
    let Some(data) = read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let cs = parsed.code_signature.as_ref().expect("should have codesig");
    let cd = cs
        .code_directory
        .as_ref()
        .expect("should have codedirectory");

    let result = macho_parse::verify_page_hashes(&data, cd);
    assert!(
        result.mismatches.is_empty(),
        "constructor_test should verify clean: {} mismatches",
        result.mismatches.len(),
    );
}

#[test]
fn page_hashes_verify_dylib_chain() {
    // Check each binary in the dylib_chain fixture
    for name in &["app", "libchaina.dylib", "libchainb.dylib"] {
        let fixture = format!("dylib_chain/{name}");
        let Some(data) = read_fixture(&fixture) else {
            continue;
        };
        let parsed = parse(&data).unwrap();
        let Some(cs) = &parsed.code_signature else {
            continue;
        };
        let Some(cd) = &cs.code_directory else {
            continue;
        };

        let result = macho_parse::verify_page_hashes(&data, cd);
        assert!(
            result.mismatches.is_empty(),
            "dylib_chain/{name} should verify clean: {} mismatches",
            result.mismatches.len(),
        );
    }
}
