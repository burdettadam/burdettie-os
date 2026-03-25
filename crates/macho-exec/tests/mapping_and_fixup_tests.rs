//! Tests for segment mapping, chained fixup application, memory protection,
//! stub resolution, entry-point computation, drop safety, and multi-fixture mapping.

mod common;

use macho_exec::{apply_chained_fixups, MappedImage};
use macho_map::MappingPlan;
use macho_parse::parse;

use common::{default_mapper, default_stub_resolver, default_unwind, read_fixture};

// ── Mapping tests ──────────────────────────────────────────────────

#[test]
fn map_simple_cli_segments() {
    let Some(data) = read_fixture("simple_cli/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
    assert!(image.segment("__TEXT").is_some());
}

#[test]
fn map_framework_app() {
    let Some(data) = read_fixture("framework_style/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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
        let _image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();
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

        let result = MappedImage::map(&plan, &data, default_mapper(), default_unwind());
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
