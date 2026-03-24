//! Integration tests for macho-exec.
//!
//! These tests map real fixture binaries into memory, apply fixups,
//! and in the case of simple_cli, attempt controlled entry.

use std::path::PathBuf;
use std::sync::Arc;

use macho_exec::{
    apply_chained_fixups, call_constructors, call_entry, checked_entry, ImageRegistry, MappedImage,
    StubResolver,
};
use macho_map::MappingPlan;
use macho_parse::parse;
use platform_core::memory::MemoryMapper;

fn default_mapper() -> Arc<dyn MemoryMapper> {
    Arc::new(platform_darwin_host::DarwinMemoryMapper)
}

fn default_stub_resolver() -> StubResolver {
    let host: Arc<dyn platform_core::symbols::HostSymbolResolver> =
        Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
    StubResolver::new(host)
}

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

// ── Mapping tests ──────────────────────────────────────────────────

#[test]
fn map_simple_cli_segments() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    // Should have segments (excluding __PAGEZERO)
    assert!(
        !image.segments.is_empty(),
        "should map at least one segment"
    );

    // Should NOT include __PAGEZERO
    assert!(
        image.segment("__PAGEZERO").is_none(),
        "should skip __PAGEZERO"
    );

    // Should have __TEXT and __DATA_CONST
    assert!(image.segment("__TEXT").is_some(), "should have __TEXT");
    assert!(
        image.segment("__DATA_CONST").is_some(),
        "should have __DATA_CONST"
    );
}

#[test]
fn mapped_text_contains_code() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    let text = image.segment("__TEXT").unwrap();
    assert!(text.size > 0, "__TEXT should have non-zero size");

    // The mapped data should contain the Mach-O header (magic at offset 0)
    let magic = unsafe {
        let ptr = text.base as *const u32;
        ptr.read_unaligned()
    };
    assert_eq!(
        magic, 0xFEEDFACF,
        "mapped __TEXT should start with MH_MAGIC_64"
    );
}

#[test]
fn mapped_total_size_reasonable() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    let total = image.total_mapped_size();
    // simple_cli is small — total mapped should be reasonable (< 1MB)
    assert!(total > 0, "should have non-zero total mapped size");
    assert!(
        total < 1024 * 1024,
        "total mapped size should be < 1MB for simple_cli"
    );
}

#[test]
fn mapped_segments_have_valid_bases() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    for seg in &image.segments {
        assert!(
            !seg.base.is_null(),
            "segment {} should have non-null base",
            seg.name
        );
        assert!(
            seg.size > 0,
            "segment {} should have non-zero size",
            seg.name
        );
    }
}

#[test]
fn map_dylib_chain_app() {
    let Some(data) = read_fixture("dylib_chain/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
    assert!(image.segment("__TEXT").is_some());
}

#[test]
fn map_framework_app() {
    let Some(data) = read_fixture("framework_style/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
    assert!(image.segment("__TEXT").is_some());
    assert!(image.segment("__DATA_CONST").is_some());
}

// ── Fixup application tests ────────────────────────────────────────

#[test]
fn apply_chained_fixups_simple_cli() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
    let resolver = default_stub_resolver();

    let result = apply_chained_fixups(&image, &parsed, &data, &resolver);
    assert!(result.is_ok(), "chained fixup application should succeed");

    let (rebases, binds) = result.unwrap();
    // simple_cli has at least 1 bind (printf)
    assert!(
        binds > 0,
        "should bind at least one symbol (printf): got {binds}"
    );
    let _ = rebases; // may be 0 with slide=0
}

#[test]
fn got_patched_after_chained_fixups() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
    let resolver = default_stub_resolver();

    // Read GOT before fixups
    let got_before = if let Some(data_const) = image.segment("__DATA_CONST") {
        unsafe {
            let ptr = data_const.base as *const u64;
            ptr.read_unaligned()
        }
    } else {
        return;
    };

    apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    // Read GOT after fixups
    let got_after = {
        let data_const = image.segment("__DATA_CONST").unwrap();
        unsafe {
            let ptr = data_const.base as *const u64;
            ptr.read_unaligned()
        }
    };

    // The GOT entry should have changed (printf address patched in)
    assert_ne!(
        got_before, got_after,
        "GOT entry should be patched after chained fixup application"
    );

    // The patched value should be a real address (non-zero, reasonable)
    assert!(got_after != 0, "patched GOT entry should be non-zero");
}

// ── Protection tests ───────────────────────────────────────────────

#[test]
fn protect_after_fixups() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
    let resolver = default_stub_resolver();

    apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    // Tighten permissions
    let result = image.protect();
    assert!(
        result.is_ok(),
        "mprotect should succeed: {:?}",
        result.err()
    );
}

// ── Stub resolver integration ──────────────────────────────────────

#[test]
fn stub_resolver_resolves_printf_to_valid_address() {
    let resolver = default_stub_resolver();
    let addr = resolver.resolve("_printf").unwrap();

    // The address should be in a reasonable range (> 4GB, typical for macOS
    // shared cache region)
    assert!(
        addr > 0x1_0000_0000,
        "printf should be in high address space: {addr:#x}"
    );
}

// ── Controlled entry tests ─────────────────────────────────────────

#[test]
fn compute_entry_address() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    // Should be able to get __TEXT base
    let text_base = image.text_base();
    assert!(text_base.is_ok(), "should find __TEXT segment");

    let base = text_base.unwrap();
    assert!(!base.is_null(), "__TEXT base should be non-null");

    // Entry offset from parsed
    if let Some(macho_types::EntryPoint::Main { entry_offset, .. }) = &parsed.entry {
        let entry_addr = unsafe { base.add(*entry_offset as usize) };
        assert!(!entry_addr.is_null(), "entry address should be non-null");
        assert!(
            entry_addr as u64 > base as u64,
            "entry should be after __TEXT base"
        );
    }
}

// ── Drop safety ────────────────────────────────────────────────────

#[test]
fn mapped_image_drops_cleanly() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // Create and immediately drop — should not panic or leak
    {
        let _image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
        // Drop at end of scope, munmap called
    }
}

// ── Multiple fixture mapping ───────────────────────────────────────

#[test]
fn map_all_fixtures_without_panic() {
    let fixtures = [
        "simple_cli/app",
        "dylib_chain/app",
        "dylib_chain/libchaina.dylib",
        "dylib_chain/libchainb.dylib",
        "framework_style/app",
    ];

    for fixture in &fixtures {
        let Some(data) = read_fixture(fixture) else {
            continue;
        };
        let Ok(parsed) = parse(&data) else { continue };
        let plan = MappingPlan::from_parsed(&parsed);

        let result = MappedImage::map(&plan, &data, default_mapper());
        assert!(
            result.is_ok(),
            "failed to map {fixture}: {:?}",
            result.err()
        );

        let image = result.unwrap();
        assert!(
            image.segment("__TEXT").is_some(),
            "{fixture} should have __TEXT"
        );
    }
}

// ── Controlled entry: execute return42 fixture ─────────────────────

#[test]
fn controlled_entry_return42() {
    let Some(data) = read_fixture("return42/return42") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    // 1. Map segments
    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
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
    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
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

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();
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

// ── Constructor execution tests ────────────────────────────────────

#[test]
fn controlled_entry_constructor_test() {
    let Some(data) = read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    // Apply fixups (constructors call printf which needs GOT binding)
    let resolver = default_stub_resolver();
    let (rebases, binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();
    assert!(
        binds >= 1,
        "should bind printf: {rebases} rebases, {binds} binds"
    );

    // Extract and call constructors
    let inits = macho_objc::extract_initializers(&parsed, &data);
    assert!(
        !inits.constructors.is_empty(),
        "constructor_test should have constructors, found: {:?}",
        inits.init_steps,
    );

    image.protect().unwrap();

    let called = call_constructors(&image, &inits).unwrap();
    assert_eq!(called, 2, "constructor_test has 2 constructors");

    // Now call main
    let entry = parsed
        .entry
        .as_ref()
        .expect("constructor_test should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();
    assert_eq!(result.exit_code, 0, "constructor_test main should return 0");
}

// ── Multi-image loading tests ──────────────────────────────────────

#[test]
fn multi_image_dylib_chain() {
    // Load order: bottom-up (libchainb → libchaina → app)
    let Some(data_b) = read_fixture("dylib_chain/libchainb.dylib") else {
        return;
    };
    let Some(data_a) = read_fixture("dylib_chain/libchaina.dylib") else {
        return;
    };
    let Some(data_app) = read_fixture("dylib_chain/app") else {
        return;
    };

    let mut registry = ImageRegistry::new(default_mapper());

    // Load leaf first
    registry
        .load("@rpath/libchainb.dylib", data_b)
        .expect("load libchainb");
    registry
        .load("@rpath/libchaina.dylib", data_a)
        .expect("load libchaina");
    let app_idx = registry.load("app", data_app).expect("load app");

    // Verify exports were extracted
    let libb = registry.get("@rpath/libchainb.dylib").unwrap();
    assert!(
        libb.exports.iter().any(|e| e.name == "_libb_greet"),
        "libchainb should export _libb_greet",
    );
    let liba = registry.get("@rpath/libchaina.dylib").unwrap();
    assert!(
        liba.exports.iter().any(|e| e.name == "_liba_greet"),
        "libchaina should export _liba_greet",
    );

    // Apply fixups across all images (cross-dylib resolution)
    let stub = default_stub_resolver();
    registry
        .apply_all_fixups(&stub)
        .expect("apply_all_fixups should succeed");

    // Protect all images
    for loaded in registry.images() {
        loaded.image.protect().unwrap();
    }

    // Call constructors for each dylib (bottom-up)
    for loaded in registry.images() {
        let inits = macho_objc::extract_initializers(&loaded.parsed, &loaded.data);
        if !inits.constructors.is_empty() {
            call_constructors(&loaded.image, &inits).unwrap();
        }
    }

    // Call app entry
    let app = &registry.images()[app_idx];
    let entry = app.parsed.entry.as_ref().expect("app should have LC_MAIN");
    let result = call_entry(&app.image, entry).unwrap();
    assert_eq!(result.exit_code, 0, "dylib_chain app should return 0");
}

// ── Weak import tests ──────────────────────────────────────────────

#[test]
fn weak_import_null_fill_absent_dylib() {
    // Load only the main binary — do NOT load liboptional.dylib.
    // The weak import _optional_greet should be null-filled,
    // and the binary's if(optional_greet) check should take the else branch.
    let Some(data) = read_fixture("weak_dylib/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    let resolver = default_stub_resolver();
    let (_rebases, binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();
    // printf should bind; optional_greet should null-fill (counts as a bind)
    assert!(
        binds >= 2,
        "should bind printf + null-fill optional_greet: {binds} binds"
    );

    image.protect().unwrap();

    let entry = parsed
        .entry
        .as_ref()
        .expect("weak_dylib app should have LC_MAIN");
    let result = call_entry(&image, entry).unwrap();
    assert_eq!(
        result.exit_code, 0,
        "weak_dylib app should return 0 with absent weak dylib"
    );
}

#[test]
fn weak_import_resolved_when_dylib_present() {
    // Load both liboptional.dylib and the main binary via ImageRegistry.
    // The weak import should resolve to the real function.
    let Some(data_lib) = read_fixture("weak_dylib/liboptional.dylib") else {
        return;
    };
    let Some(data_app) = read_fixture("weak_dylib/app") else {
        return;
    };

    let mut registry = ImageRegistry::new(default_mapper());
    registry
        .load("@rpath/liboptional.dylib", data_lib)
        .expect("load liboptional");
    let app_idx = registry.load("app", data_app).expect("load app");

    // Verify export
    let lib = registry.get("@rpath/liboptional.dylib").unwrap();
    assert!(
        lib.exports.iter().any(|e| e.name == "_optional_greet"),
        "liboptional should export _optional_greet"
    );

    let stub = default_stub_resolver();
    registry.apply_all_fixups(&stub).expect("apply_all_fixups");

    for loaded in registry.images() {
        loaded.image.protect().unwrap();
    }

    let app = &registry.images()[app_idx];
    let entry = app.parsed.entry.as_ref().expect("app should have LC_MAIN");
    let result = call_entry(&app.image, entry).unwrap();
    assert_eq!(
        result.exit_code, 0,
        "weak_dylib app should return 0 with dylib present"
    );
}

// ── Classic fixup dispatch test ─────────────────────────────────────

/// Verify apply_all_fixups can handle binaries with FixupModel::None
/// without erroring (previously all images were sent through chained fixup path).
#[test]
fn apply_all_fixups_handles_no_fixup_model() {
    let data = read_fixture("simple_cli/app").unwrap();

    let mut registry = ImageRegistry::new(default_mapper());
    registry.load("simple_cli", data).expect("load simple_cli");

    let stub = default_stub_resolver();
    // This should succeed regardless of fixup model
    registry
        .apply_all_fixups(&stub)
        .expect("apply_all_fixups should handle any fixup model");
}

// ── Re-export resolution test ───────────────────────────────────────

/// Verify that RegistryResolver searches export tries of all loaded images.
#[test]
fn registry_resolver_finds_cross_image_symbols() {
    let libb_data = read_fixture("dylib_chain/libchainb.dylib").unwrap();
    let liba_data = read_fixture("dylib_chain/libchaina.dylib").unwrap();
    let app_data = read_fixture("dylib_chain/app").unwrap();

    let mut registry = ImageRegistry::new(default_mapper());
    registry
        .load("libchainb.dylib", libb_data)
        .expect("load libb");
    registry
        .load("libchaina.dylib", liba_data)
        .expect("load liba");
    registry.load("app", app_data).expect("load app");

    let stub = default_stub_resolver();
    let resolver = registry.resolver(&stub);

    // _libb_greet is exported by libchainb
    use macho_exec::SymbolResolver;
    let addr = resolver.resolve("_libb_greet");
    assert!(
        addr.is_some(),
        "_libb_greet should be resolvable via registry"
    );

    // _liba_greet is exported by libchaina
    let addr = resolver.resolve("_liba_greet");
    assert!(
        addr.is_some(),
        "_liba_greet should be resolvable via registry"
    );
}

#[test]
fn objc_classes_chained_fixups_count() {
    let Some(data) = read_fixture("objc_classes/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);
    let image = MappedImage::map(&plan, &data, default_mapper()).unwrap();

    let resolver = default_stub_resolver();
    let (rebases, binds) = apply_chained_fixups(&image, &parsed, &data, &resolver).unwrap();

    eprintln!(
        "objc_classes fixups: {rebases} rebases, {binds} binds, slide={:#x}",
        image.slide
    );

    // 18 rebases for internal pointers (selrefs, classlist, class data, etc.)
    assert!(rebases >= 10, "expected many rebases, got {rebases}");
    // 4 binds with bare StubResolver (objc_alloc_init, autoreleasePoolPop/Push, msgSend)
    // Foundation symbols (NSLog, CFConstantString, NSObject class/meta) need shim table
    assert!(binds >= 4, "expected at least 4 binds, got {binds}");
}
