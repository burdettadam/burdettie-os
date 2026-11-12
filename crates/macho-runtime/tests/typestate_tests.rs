//! Integration tests for the typestate loader pipeline.

mod common;

use std::sync::Mutex;

use macho_runtime::typestate::Loader;
use macho_runtime::RuntimeConfig;

/// Serializes typestate tests to avoid ObjC global state races.
static TS_LOCK: Mutex<()> = Mutex::new(());

fn fixtures_dir() -> std::path::PathBuf {
    macho_fixtures::fixtures_dir()
}

/// Helper: run a fixture through the full typestate pipeline.
fn run_typestate(fixture_rel: &str) -> Option<macho_runtime::RuntimeResult> {
    let _lock = TS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = fixtures_dir().join(fixture_rel);
    if !path.exists() {
        eprintln!("Skipping: fixture not found at {}", path.display());
        return None;
    }
    let config = RuntimeConfig::permissive(&path);
    Some(
        Loader::parse(&path, config)
            .expect("parse")
            .check_security()
            .expect("security")
            .map_images()
            .expect("map")
            .apply_fixups()
            .expect("fixups")
            .register_objc()
            .expect("objc")
            .protect()
            .expect("protect")
            .initialize()
            .expect("init")
            .run()
            .expect("run"),
    )
}

// ── Basic pipeline tests ────────────────────────────────────────────

#[test]
fn typestate_return42() {
    let Some(result) = run_typestate("return42/return42") else {
        return;
    };
    assert_eq!(result.exit_code, 42);
}

#[test]
fn typestate_simple_cli() {
    let Some(result) = run_typestate("simple_cli/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);
}

#[test]
fn typestate_constructor_test() {
    let Some(result) = run_typestate("constructor_test/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);
    assert!(result.initializers_run > 0);
}

// ── Accessor tests ──────────────────────────────────────────────────

#[test]
fn typestate_accessors_at_each_phase() {
    let _lock = TS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = fixtures_dir().join("return42/return42");
    if !path.exists() {
        eprintln!("Skipping: fixture not built");
        return;
    }
    let config = RuntimeConfig::permissive(&path);

    let parsed = Loader::parse(&path, config).expect("parse");
    assert!(!parsed.parsed().segments.is_empty());
    assert!(parsed.security_report().is_none());
    assert_eq!(parsed.images_loaded(), 0);

    let checked = parsed.check_security().expect("check");
    // permissive config has no security policy → report is None
    assert!(checked.security_report().is_none());

    let mapped = checked.map_images().expect("map");
    assert!(mapped.images_loaded() >= 1);

    let fixedup = mapped.apply_fixups().expect("fixups");
    assert!(fixedup.images_loaded() >= 1);

    let registered = fixedup.register_objc().expect("objc");
    assert!(registered.images_loaded() >= 1);

    let protected = registered.protect().expect("protect");
    assert!(protected.images_loaded() >= 1);

    let initialized = protected.initialize().expect("init");
    assert!(initialized.images_loaded() >= 1);

    let result = initialized.run().expect("run");
    assert_eq!(result.exit_code, 42);
}

// ── ObjC fixture through typestate ──────────────────────────────────

#[test]
fn typestate_objc_minimal() {
    let Some(result) = run_typestate("objc_minimal/objc_minimal") else {
        return;
    };
    assert_eq!(result.exit_code, 0);
}

// ── Dylib chain (multi-image) through typestate ─────────────────────

#[test]
fn typestate_dylib_chain() {
    let Some(result) = run_typestate("dylib_chain/app") else {
        return;
    };
    assert_eq!(result.exit_code, 0);
    assert!(
        result.images_loaded > 1,
        "dylib_chain should load multiple images"
    );
}
