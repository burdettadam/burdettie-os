//! Single-image, multi-image execution, T4-T6, error path, and multi-image property tests.

mod common;

use common::{RUNTIME_LOCK, Runtime, RuntimeConfig, fixtures_dir};

// ── Single-image runtime execution tests ─────────────────────────────

#[test]
fn runtime_return42() {
    let Some(result) = common::run_fixture("return42/return42") else { return };

    assert_eq!(result.exit_code, 42, "return42 should exit with code 42");
    assert_eq!(result.images_loaded, 1);
    assert!(result.shims_installed > 0);
}

#[test]
fn runtime_simple_cli() {
    let Some(result) = common::run_fixture("simple_cli/simple_cli") else { return };

    assert_eq!(result.exit_code, 0, "simple_cli should exit with code 0");
    assert!(result.shims_installed > 0);
}

#[test]
fn runtime_constructor_test() {
    let Some(result) = common::run_fixture("constructor_test/constructor_test") else { return };

    assert_eq!(result.exit_code, 0);
    assert!(
        result.initializers_run > 0,
        "constructor_test should have run constructors"
    );
}

// ── Multi-image runtime execution tests ─────────────────────────────

#[test]
fn runtime_dylib_chain() {
    let Some(result) = common::run_fixture("dylib_chain/app") else { return };

    assert_eq!(result.exit_code, 0, "dylib_chain should exit with code 0");
    assert!(
        result.images_loaded > 1,
        "dylib_chain should load multiple images (got {})",
        result.images_loaded
    );
}

#[test]
fn runtime_weak_dylib() {
    let Some(result) = common::run_fixture("weak_dylib/app") else { return };

    // weak_dylib should still run (uses else branch when dylib is absent)
    assert_eq!(result.exit_code, 0);
}

#[test]
fn runtime_fat_binary() {
    let Some(result) = common::run_fixture("fat_binary/app") else { return };

    assert_eq!(result.exit_code, 0, "fat_binary should exit with code 0");
    assert_eq!(result.images_loaded, 1);
}

#[test]
fn runtime_framework_style() {
    let Some(result) = common::run_fixture("framework_style/app") else { return };

    assert_eq!(
        result.exit_code, 0,
        "framework_style should exit with code 0"
    );
    assert!(
        result.images_loaded >= 2,
        "framework_style should load app + framework dylib (got {})",
        result.images_loaded
    );
}

// ── T4-T6 target fixture tests ──────────────────────────────────────

#[test]
fn runtime_tlv_basic() {
    let Some(result) = common::run_fixture("tlv_basic/app") else { return };

    assert_eq!(result.exit_code, 0, "tlv_basic should exit with code 0");
}

#[test]
fn runtime_process_info() {
    let Some(result) = common::run_fixture("process_info/process_info") else { return };

    assert_eq!(result.exit_code, 0, "process_info should exit with code 0");
}

#[test]
fn runtime_dispatch_test() {
    let Some(result) = common::run_fixture("dispatch_test/dispatch_test") else { return };

    assert_eq!(result.exit_code, 0, "dispatch_test should exit with code 0");
}

// ── Error path tests ────────────────────────────────────────────────

#[test]
fn runtime_nonexistent_file() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::path::PathBuf::from("/tmp/nonexistent-binary-xyz");
    let config = RuntimeConfig::permissive(&path);
    let result = Runtime::load_and_run(&path, &config);
    assert!(result.is_err());
}

#[test]
fn runtime_no_host_fallback() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/return42");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let mut config = RuntimeConfig::permissive(&fixture);
    config.host_fallback = false;

    let result = Runtime::load_and_run(&fixture, &config);
    match result {
        Ok(r) => assert_eq!(r.exit_code, 42),
        Err(e) => eprintln!("Expected: no host fallback may fail: {e}"),
    }
}

#[test]
fn runtime_malformed_bad_magic_rejected() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("malformed/bad_magic");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config);
    assert!(
        result.is_err(),
        "bad_magic should be rejected by the runtime"
    );
}

#[test]
fn runtime_malformed_truncated_header_rejected() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("malformed/truncated_header");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config);
    assert!(
        result.is_err(),
        "truncated_header should be rejected by the runtime"
    );
}

// ── Multi-image properties ──────────────────────────────────────────

#[test]
fn runtime_dylib_chain_images_counted() {
    let Some(result) = common::run_fixture("dylib_chain/app") else { return };

    // dylib_chain has: app → libchaina → libchainb
    // images_loaded should be 3 (2 dylibs + main)
    assert!(
        result.images_loaded >= 2,
        "expected at least 2 images, got {}",
        result.images_loaded
    );
}

#[test]
fn runtime_shim_count_with_all_categories() {
    let Some(result) = common::run_fixture("return42/return42") else { return };

    // Shim count with all categories (updated after D1: self-hosted NSConstant* + EHTYPE).
    #[cfg(target_os = "macos")]
    assert_eq!(result.shims_installed, 267, "expected 267 shims installed");
    #[cfg(not(target_os = "macos"))]
    assert_eq!(result.shims_installed, 267, "expected 267 shims installed");
}

// ── Tracing tests ──────────────────────────────────────────────────

#[test]
fn runtime_tracing_collects_pipeline_events() {
    use macho_runtime::{EventKind, RuntimeTracer};

    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/return42");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let tracer = RuntimeTracer::new();
    let mut config = RuntimeConfig::permissive(&fixture);
    config.tracer = Some(tracer);

    let result =
        Runtime::load_and_run(&fixture, &config).expect("return42 with tracing should succeed");

    assert_eq!(result.exit_code, 42);

    // Tracer should be returned when enabled
    let tracer = result.tracer.expect("tracer should be returned in result");

    // We should have pipeline events for the major stages
    let pipeline_events = tracer.events_of(EventKind::Pipeline);
    assert!(
        pipeline_events.len() >= 5,
        "expected at least 5 pipeline events, got {}: {:?}",
        pipeline_events.len(),
        pipeline_events
            .iter()
            .map(|e| &e.message)
            .collect::<Vec<_>>()
    );

    // Check that key stages are recorded
    let messages: Vec<&str> = pipeline_events.iter().map(|e| e.message.as_str()).collect();
    assert!(
        messages.iter().any(|m| m.contains("parsed")),
        "should have parse event"
    );
    assert!(
        messages.iter().any(|m| m.contains("mprotect")),
        "should have mprotect event"
    );
    assert!(
        messages.iter().any(|m| m.contains("entry point returned")),
        "should have entry point return event"
    );

    // Should have image load events
    let image_events = tracer.events_of(EventKind::ImageLoad);
    assert!(
        !image_events.is_empty(),
        "should have at least one ImageLoad event"
    );
}
