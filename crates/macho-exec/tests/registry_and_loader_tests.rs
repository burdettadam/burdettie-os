//! Tests for constructor execution, multi-image loading (ImageRegistry),
//! weak imports, classic fixup dispatch, re-export resolution, and ObjC fixups.

mod common;

use macho_exec::{
    apply_chained_fixups, call_constructors, call_entry, ImageRegistry, MappedImage,
};
use macho_map::MappingPlan;
use macho_parse::parse;

use common::{default_mapper, default_stub_resolver, default_unwind, read_fixture};

// ── Constructor execution tests ────────────────────────────────────

#[test]
fn controlled_entry_constructor_test() {
    let Some(data) = read_fixture("constructor_test/app") else {
        return;
    };
    let parsed = parse(&data).unwrap();
    let plan = MappingPlan::from_parsed(&parsed);

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let mut registry = ImageRegistry::new(default_mapper(), default_unwind());

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

    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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

    let mut registry = ImageRegistry::new(default_mapper(), default_unwind());
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

    let mut registry = ImageRegistry::new(default_mapper(), default_unwind());
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

    let mut registry = ImageRegistry::new(default_mapper(), default_unwind());
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
    let image = MappedImage::map(&plan, &data, default_mapper(), default_unwind()).unwrap();

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
