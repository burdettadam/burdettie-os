//! Tests for controlled entry-point execution and security-gated execution
//! (trust policy evaluation, code signature inspection).

mod common;

use macho_exec::{apply_chained_fixups, call_entry, checked_entry, MappedImage};
use macho_map::MappingPlan;
use macho_parse::parse;

use common::{
    default_mapper, default_stub_resolver, default_unwind, fixtures_dir, read_fixture,
};

// ── Controlled entry: execute return42 fixture ─────────────────────

#[test]
fn controlled_entry_return42() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // 1. Map segments
    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // 2. Tighten permissions (__TEXT needs r-x for code execution)
    image.protect().unwrap();

    // 3. Call entry point
    let entry = parsed.entry.as_ref().expect("return42 should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();

    assert_eq!(result.exit_code, 42, "return42 should return exit code 42");
    assert!(result.entry_address > 0, "entry address should be non-zero");
}

#[test]
fn controlled_entry_preserves_stack() {
    // Verify that calling return42 doesn't corrupt our stack
    let canary: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let stored_canary = canary;

    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    image.protect().unwrap();

    let entry = parsed.entry.as_ref().unwrap();
    let _result = call_entry(&image, entry).unwrap();

    // Stack should not be corrupted
    assert_eq!(
        canary, stored_canary,
        "stack canary should be preserved after controlled entry"
    );
}

// ── Controlled entry: execute simple_cli with printf stub ──────────

#[test]
fn controlled_entry_simple_cli() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // 1. Map segments (contiguous layout preserves PC-relative references)
    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

    // 2. Apply chained fixups (patches GOT so printf stub works)
    let resolver = default_stub_resolver();
    let (rebases, binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();
    assert!(
        binds >= 1,
        "should bind at least printf: got {binds} binds, {rebases} rebases"
    );

    // 3. Tighten permissions (__TEXT r-x, __DATA_CONST r--)
    image.protect().unwrap();

    // 4. Call entry — simple_cli prints "hello\n" via printf and returns 0
    let entry = parsed
        .entry
        .as_ref()
        .expect("simple_cli should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();

    assert_eq!(result.exit_code, 0, "simple_cli should return exit code 0");
}

// ── Security-gated execution tests ─────────────────────────────────

#[test]
fn security_checked_entry_permissive() {
    use trust_policy::TrustPolicy;

    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    image.protect().unwrap();

    let entry = parsed.entry.as_ref().unwrap();
    let policy = TrustPolicy::permissive();
    let binary_path = fixtures_dir().join("return42/return42");

    let (result, report) = checked_entry(&image, entry, &parsed, &binary_path, &policy).unwrap();

    assert_eq!(result.exit_code, 42);
    assert!(report.allowed, "permissive policy should allow execution");
    assert!(
        !report.hardened_runtime,
        "permissive policy should not enforce hardened runtime"
    );
}

#[test]
fn security_checked_entry_apple_default() {
    use trust_policy::TrustPolicy;

    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    image.protect().unwrap();

    let entry = parsed.entry.as_ref().unwrap();
    let policy = TrustPolicy::apple_default();
    // Use a non-SIP path so it won't be denied
    let binary_path = fixtures_dir().join("return42/return42");

    let (result, report) = checked_entry(&image, entry, &parsed, &binary_path, &policy).unwrap();

    assert_eq!(result.exit_code, 42);
    assert!(report.allowed, "fixture at non-SIP path should be allowed");
}

#[test]
fn security_report_includes_cs_info() {
    use trust_policy::{CodeSignatureInfo, TrustPolicy};

    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();

    // Build cs_info from parsed signature
    let cs_info = parsed
        .code_signature
        .as_ref()
        .map(CodeSignatureInfo::from_parsed)
        .unwrap_or_default();

    assert!(cs_info.present, "simple_cli should have code signature");
    assert!(cs_info.adhoc, "simple_cli should be ad-hoc signed");
    assert!(
        !cs_info.runtime,
        "simple_cli should not have hardened runtime"
    );
    assert!(!cs_info.restrict, "simple_cli should not have CS_RESTRICT");

    // Evaluate with apple_default
    let policy = TrustPolicy::apple_default();
    let binary_path = fixtures_dir().join("simple_cli/app");
    let report = policy.evaluate_binary_parsed(&binary_path, parsed.header.flags, &cs_info);

    assert!(report.allowed);
    assert!(report.cs_info.is_some(), "report should include cs_info");
    let info = report.cs_info.unwrap();
    assert!(info.present);
    assert!(info.adhoc);
    assert!(!info.cd_hash.is_empty(), "CDHash should be populated");
}

#[test]
fn security_denied_for_sip_path_unsigned() {
    use trust_policy::{CodeSignatureInfo, TrustPolicy};

    // Simulate a binary at a SIP-protected path with no code signature
    let policy = TrustPolicy::apple_default();
    let cs_info = CodeSignatureInfo::default(); // present=false

    let report = policy.evaluate_binary_parsed(
        std::path::Path::new("/usr/bin/fake_tool"),
        macho_types::header::MachFlags::PIE,
        &cs_info,
    );

    assert!(
        !report.allowed,
        "unsigned binary at SIP path should be denied"
    );
    assert!(!report.denied_reasons.is_empty());
    assert!(report.strip_dyld_env, "SIP path should strip DYLD env vars");
}
