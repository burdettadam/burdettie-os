//! Integration tests for the runtime orchestration layer.
//!
//! These tests exercise the full pipeline: parse → map → shim → fixup → run.
//! Tests that call `load_and_run` use a global lock because they share
//! mutable process-wide state (mapped memory, ObjC runtime globals).

use std::path::PathBuf;
use std::sync::Mutex;

use macho_runtime::{EventKind, Runtime, RuntimeConfig, RuntimeTracer, ShimSet};

/// Serializes tests that call `load_and_run` to avoid global state races.
static RUNTIME_LOCK: Mutex<()> = Mutex::new(());

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("fixtures"))
        .unwrap_or_else(|| PathBuf::from("fixtures"))
}

// ── Config construction tests ────────────────────────────────────────

#[test]
fn permissive_config_defaults() {
    let path = PathBuf::from("/tmp/test-binary");
    let config = RuntimeConfig::permissive(&path);
    assert!(config.host_fallback);
    assert!(config.security_policy.is_none());
    assert_eq!(config.process_name, "test-binary");
}

#[test]
fn secure_config_has_policy() {
    let path = PathBuf::from("/tmp/test-binary");
    let config = RuntimeConfig::secure(&path);
    assert!(config.security_policy.is_some());
    assert!(config.host_fallback);
}

#[test]
fn shim_set_defaults_all_enabled() {
    let shim_set = ShimSet::default();
    assert!(shim_set.process);
    assert!(shim_set.tls);
    assert!(shim_set.dispatch);
    assert!(shim_set.pthread);
    assert!(shim_set.mach);
    assert!(shim_set.lifecycle);
    assert!(shim_set.objc);
}

// ── Runtime context tests ────────────────────────────────────────────

#[test]
fn context_installs_all_shims() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    // 5 process + 2 TLS + 5 dispatch + 42 pthread + 3 mach + 2 ptrauth + 2 lifecycle + 66 objc + 105 foundation = 232 shims (macOS)
    #[cfg(target_os = "macos")]
    assert_eq!(ctx.shim_table.len(), 232);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(ctx.shim_table.len(), 225);
}

#[test]
fn context_respects_shim_set() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet {
        process: false,
        tls: true,
        dispatch: false,
        pthread: false,
        mach: false,
        lifecycle: false,
        ptrauth: false,
        objc: false,
        foundation: false,
    };
    let ctx = RuntimeContext::new(&config, &shim_set);

    // Only TLS shims: 2
    assert_eq!(ctx.shim_table.len(), 2);
}

// ── Resolver composition tests ────────────────────────────────────────

#[test]
fn composite_resolver_shim_priority() {
    use libsystem_shims::ShimTable;
    use macho_exec::{StubResolver, SymbolResolver};
    use macho_runtime::CompositeResolver;

    let mut table = ShimTable::new();
    table.register("_printf", 0xCAFE, "test override");

    let stub = {
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        StubResolver::new(host)
    };
    let resolver = CompositeResolver::new(&table, Some(&stub));

    // Should get our shim address, not the host's
    assert_eq!(resolver.resolve("_printf"), Some(0xCAFE));
}

#[test]
fn composite_resolver_host_fallback() {
    use libsystem_shims::ShimTable;
    use macho_exec::{StubResolver, SymbolResolver};
    use macho_runtime::CompositeResolver;

    let table = ShimTable::new();
    let stub = {
        let host: std::sync::Arc<dyn platform_core::symbols::HostSymbolResolver> =
            std::sync::Arc::new(platform_darwin_host::DarwinSymbolResolver::new());
        StubResolver::new(host)
    };
    let resolver = CompositeResolver::new(&table, Some(&stub));

    // strlen should resolve via host
    assert!(resolver.resolve("_strlen").is_some());
}

// ── Single-image runtime execution tests ─────────────────────────────

#[test]
fn runtime_return42() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/return42");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 42, "return42 should exit with code 42");
    assert_eq!(result.images_loaded, 1);
    assert!(result.shims_installed > 0);
}

#[test]
fn runtime_simple_cli() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("simple_cli/simple_cli");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "simple_cli should exit with code 0");
    assert!(result.shims_installed > 0);
}

#[test]
fn runtime_constructor_test() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("constructor_test/constructor_test");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0);
    assert!(
        result.initializers_run > 0,
        "constructor_test should have run constructors"
    );
}

// ── Multi-image runtime execution tests ─────────────────────────────

#[test]
fn runtime_dylib_chain() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "dylib_chain should exit with code 0");
    assert!(
        result.images_loaded > 1,
        "dylib_chain should load multiple images (got {})",
        result.images_loaded
    );
}

#[test]
fn runtime_weak_dylib() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("weak_dylib/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // weak_dylib should still run (uses else branch when dylib is absent)
    assert_eq!(result.exit_code, 0);
}

#[test]
fn runtime_fat_binary() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("fat_binary/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "fat_binary should exit with code 0");
    assert_eq!(result.images_loaded, 1);
}

#[test]
fn runtime_framework_style() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("framework_style/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("framework_style execution failed");

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
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("tlv_basic/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("TLS runtime execution failed");

    assert_eq!(result.exit_code, 0, "tlv_basic should exit with code 0");
}

#[test]
fn runtime_process_info() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("process_info/process_info");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("process_info execution failed");

    assert_eq!(result.exit_code, 0, "process_info should exit with code 0");
}

#[test]
fn runtime_dispatch_test() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("dispatch_test/dispatch_test");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("dispatch_test execution failed");

    assert_eq!(result.exit_code, 0, "dispatch_test should exit with code 0");
}

// ── Error path tests ────────────────────────────────────────────────

#[test]
fn runtime_nonexistent_file() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = PathBuf::from("/tmp/nonexistent-binary-xyz");
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
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

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
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/return42");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // With all shim categories: 5+2+5+42+3+2+2+66+105 = 232 (macOS)
    #[cfg(target_os = "macos")]
    assert_eq!(result.shims_installed, 232, "expected 232 shims installed");
    #[cfg(not(target_os = "macos"))]
    assert_eq!(result.shims_installed, 221, "expected 221 shims installed");
}

// ── ObjC runtime tests ─────────────────────────────────────────────

#[test]
fn runtime_objc_minimal() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_minimal/objc_minimal");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_minimal should exit 0");
}

#[test]
fn runtime_objc_minimal_class_registration() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // After running objc_minimal, verify the global ObjC runtime was populated
    let fixture = fixtures_dir().join("objc_minimal/objc_minimal");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // The global runtime should have registered the Answer class
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Answer").is_some(),
            "Answer class should be registered"
        );
        let cls = rt.get_class("Answer").unwrap();
        assert!(!cls.methods.is_empty(), "Answer should have methods");
        assert!(
            cls.methods.iter().any(|m| m.selector == "value"),
            "Answer should have a 'value' method"
        );
    }
}

// ── ObjC category + +load tests ────────────────────────────────────

#[test]
fn runtime_objc_category_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_category/objc_category");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_category should exit 0");
}

#[test]
fn runtime_objc_category_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_category/objc_category");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter class should be registered"
        );
        let cls = rt.get_class("Greeter").unwrap();
        // Should have 'greeting' (original) + 'farewell' (category)
        assert!(
            cls.methods.iter().any(|m| m.selector == "greeting"),
            "Greeter should have 'greeting' method"
        );
        assert!(
            cls.methods.iter().any(|m| m.selector == "farewell"),
            "Greeter should have 'farewell' from category"
        );
    }
}

#[test]
fn runtime_objc_category_load_method_detected() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_category/objc_category");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let loads = rt.collect_load_methods();
        assert!(
            loads.iter().any(|(name, _)| name == "Greeter"),
            "Greeter should have a +load method detected"
        );
    }
}

// ── Foundation / CF-lite tests (Phase 22, T8) ──────────────────────

#[test]
fn runtime_objc_foundation_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_foundation should exit 0");
}

#[test]
fn runtime_objc_foundation_nslog_shim_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    // Verify foundation shims are in the shim table
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_NSLog").is_some(),
        "_NSLog should be in shim table"
    );
    assert!(
        ctx.shim_table
            .resolve("___CFConstantStringClassReference")
            .is_some(),
        "___CFConstantStringClassReference should be in shim table"
    );
}

#[test]
fn runtime_objc_classes_with_foundation_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_classes/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(
        result.exit_code, 0,
        "objc_classes should exit 0 with foundation shims"
    );
}

#[test]
fn runtime_objc_classes_foundation_classes_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_classes/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // Verify classes from the fixture were registered alongside foundation symbols
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter should be registered with foundation shims active"
        );
        assert!(
            rt.get_class("Calculator").is_some(),
            "Calculator should be registered with foundation shims active"
        );
    }
}

// ── Multi-dylib ObjC tests (Phase 23, T10) ────────────────────────

#[test]
fn runtime_objc_dylib_chain_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_dylib_chain should exit 0");
    assert!(
        result.images_loaded >= 3,
        "should load app + libgreeter + libextras"
    );
}

#[test]
fn runtime_objc_dylib_chain_cross_image_class() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // Greeter class should be registered (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter should be registered from libgreeter.dylib"
        );
    }
}

#[test]
fn runtime_objc_dylib_chain_cross_image_category() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // Verify category from libextras was merged into Greeter from libgreeter
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let cls = rt
            .get_class("Greeter")
            .expect("Greeter should be registered");
        assert!(
            cls.methods.iter().any(|m| m.selector == "greeting"),
            "Greeter should have 'greeting' from libgreeter"
        );
        assert!(
            cls.methods.iter().any(|m| m.selector == "farewell"),
            "Greeter should have 'farewell' from libextras category"
        );
    }
}

#[test]
fn runtime_objc_dylib_chain_load_method() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_dylib_chain/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // Verify +load was detected for Greeter (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let loads = rt.collect_load_methods();
        assert!(
            loads.iter().any(|(name, _)| name == "Greeter"),
            "Greeter should have +load from libgreeter"
        );
    }
}

// ── NSString/CFString ops tests (Phase 25, T9) ────────────────────

#[test]
fn runtime_nsstring_ops_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_ops/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_ops should exit 0");
}

#[test]
fn runtime_nsstring_ops_cfstring_shims_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_ops/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table
            .resolve("_CFStringCreateWithCString")
            .is_some(),
        "_CFStringCreateWithCString should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_CFStringGetLength").is_some(),
        "_CFStringGetLength should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_CFStringGetCStringPtr").is_some(),
        "_CFStringGetCStringPtr should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_kCFStringEncodingUTF8").is_some(),
        "_kCFStringEncodingUTF8 should be in shim table"
    );
}

// ── Blocks runtime tests (Phase 26, T11) ──────────────────────────

#[test]
fn runtime_block_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("block_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "block_basics should exit 0");
}

#[test]
fn runtime_block_basics_shims_resolved() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("block_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("__NSConcreteGlobalBlock").is_some(),
        "__NSConcreteGlobalBlock should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__NSConcreteStackBlock").is_some(),
        "__NSConcreteStackBlock should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__Block_copy").is_some(),
        "__Block_copy should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("__Block_release").is_some(),
        "__Block_release should be in shim table"
    );
    assert!(
        ctx.shim_table.resolve("_objc_retainBlock").is_some(),
        "_objc_retainBlock should be in shim table"
    );
}

// ── Inheritance + __objc_stubs tests (Phase 27, T12) ──────────────

#[test]
fn runtime_objc_inherit_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_inherit/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_inherit should exit 0");
}

#[test]
fn runtime_objc_inherit_superclass_chain() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_inherit/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    // Verify superclass chain is parsed
    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let shape = rt.get_class("Shape").expect("Shape should be registered");
        assert!(shape.superclass_addr.is_none(), "Shape is root class");

        let circle = rt.get_class("Circle").expect("Circle should be registered");
        assert!(
            circle.superclass_addr.is_some(),
            "Circle should have superclass"
        );

        let filled = rt
            .get_class("FilledCircle")
            .expect("FilledCircle should be registered");
        assert!(
            filled.superclass_addr.is_some(),
            "FilledCircle should have superclass"
        );

        // FilledCircle inherits area from Circle (chain traversal)
        let area_imp = rt.msg_lookup(filled.mapped_addr, "area");
        assert!(
            area_imp.is_some(),
            "FilledCircle should find 'area' via superclass chain"
        );

        // FilledCircle inherits sides from Shape (two levels up)
        let sides_imp = rt.msg_lookup(filled.mapped_addr, "sides");
        assert!(
            sides_imp.is_some(),
            "FilledCircle should find 'sides' two levels up"
        );
    }
}

// ── NSObject methods + metaclass dispatch tests (Phase 28, T13) ───

#[test]
fn runtime_nsobject_methods_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsobject_methods/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsobject_methods should exit 0");
}

#[test]
fn runtime_nsobject_methods_metaclass_dispatch() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsobject_methods/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        // Widget and BaseObj should be registered
        let widget = rt.get_class("Widget").expect("Widget should be registered");
        let base = rt
            .get_class("BaseObj")
            .expect("BaseObj should be registered");

        // Both should have superclass_addr pointing up the chain
        // Widget → BaseObj → NSObject (superclass chain, repaired post-fixup)
        assert!(
            widget.superclass_addr.is_some(),
            "Widget should have superclass"
        );
        assert!(
            base.superclass_addr.is_some(),
            "BaseObj should have superclass (NSObject)"
        );

        // NSObject should be registered as synthetic root
        let nsobject = rt
            .get_class("NSObject")
            .expect("NSObject should be registered");
        assert!(nsobject.superclass_addr.is_none(), "NSObject is root class");

        // Widget should find 'widgetId' (own method)
        let wid_imp = rt.msg_lookup(widget.mapped_addr, "widgetId");
        assert!(wid_imp.is_some(), "Widget should find 'widgetId'");

        // Widget should find 'baseValue' via superclass chain
        let bv_imp = rt.msg_lookup(widget.mapped_addr, "baseValue");
        assert!(
            bv_imp.is_some(),
            "Widget should find 'baseValue' via superclass"
        );

        // Widget should find 'init' via NSObject (two levels up)
        let init_imp = rt.msg_lookup(widget.mapped_addr, "init");
        assert!(
            init_imp.is_some(),
            "Widget should find 'init' from NSObject"
        );

        // Widget should find 'respondsToSelector:' via NSObject
        let rts_imp = rt.msg_lookup(widget.mapped_addr, "respondsToSelector:");
        assert!(
            rts_imp.is_some(),
            "Widget should find 'respondsToSelector:' from NSObject"
        );
    }
}

// ── objc_msgSendSuper / Super2 tests (Phase 29, T14) ──────────────

#[test]
fn runtime_objc_super_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_super/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "objc_super should exit 0");
}

#[test]
fn runtime_objc_super_class_hierarchy() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_super/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        // Three-level hierarchy: Puppy → Dog → Animal → NSObject
        let puppy = rt.get_class("Puppy").expect("Puppy should be registered");
        let dog = rt.get_class("Dog").expect("Dog should be registered");
        let animal = rt.get_class("Animal").expect("Animal should be registered");
        let nsobject = rt
            .get_class("NSObject")
            .expect("NSObject should be registered");

        // Superclass chain integrity
        assert!(
            puppy.superclass_addr.is_some(),
            "Puppy should have superclass"
        );
        assert!(dog.superclass_addr.is_some(), "Dog should have superclass");
        assert!(
            animal.superclass_addr.is_some(),
            "Animal should have superclass (NSObject)"
        );
        assert!(nsobject.superclass_addr.is_none(), "NSObject is root class");

        // Puppy should find 'legs' (own), 'sound' via Dog, 'init' via NSObject
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "legs").is_some(),
            "Puppy.legs"
        );
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "sound").is_some(),
            "Puppy.sound via Dog"
        );
        assert!(
            rt.msg_lookup(puppy.mapped_addr, "init").is_some(),
            "Puppy.init via NSObject"
        );

        // Dog should find 'legs' via Animal (not overridden)
        assert!(
            rt.msg_lookup(dog.mapped_addr, "legs").is_some(),
            "Dog.legs via Animal"
        );
    }
}

// ── T15: Dynamic NSString ──────────────────────────────────────────

#[test]
fn runtime_nsstring_dynamic_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_dynamic/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_dynamic should exit 0");
}

#[test]
fn runtime_nsstring_class_hierarchy() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_dynamic/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        // NSObject → NSString → __NSCFConstantString
        let nsobject = rt.get_class("NSObject").expect("NSObject registered");
        let nsstring = rt.get_class("NSString").expect("NSString registered");
        let cfconst = rt
            .get_class("__NSCFConstantString")
            .expect("__NSCFConstantString registered");

        // NSString superclass = NSObject
        assert_eq!(
            nsstring.superclass_addr,
            Some(nsobject.mapped_addr),
            "NSString super should be NSObject"
        );

        // __NSCFConstantString superclass = NSString
        assert_eq!(
            cfconst.superclass_addr,
            Some(nsstring.mapped_addr),
            "__NSCFConstantString super should be NSString"
        );

        // NSString should have instance methods
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "length").is_some(),
            "NSString.length"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "UTF8String").is_some(),
            "NSString.UTF8String"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "initWithUTF8String:")
                .is_some(),
            "NSString.initWithUTF8String:"
        );
        assert!(
            rt.msg_lookup(nsstring.mapped_addr, "isEqualToString:")
                .is_some(),
            "NSString.isEqualToString:"
        );

        // __NSCFConstantString inherits NSString methods
        assert!(
            rt.msg_lookup(cfconst.mapped_addr, "length").is_some(),
            "CFConst inherits length"
        );
        assert!(
            rt.msg_lookup(cfconst.mapped_addr, "isEqualToString:")
                .is_some(),
            "CFConst inherits isEqualToString:"
        );
    }
}

// ── Phase 31: NSLog variadic trampoline tests ──────────────────────

#[test]
fn runtime_nslog_format_at_sign_outputs_string() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("objc_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    // Clear any prior captured output
    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    // T8 calls NSLog(@"%@", @"hello from Foundation")
    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("hello from Foundation")),
        "NSLog should output 'hello from Foundation', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_nslog_dynamic_nsstring() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_dynamic/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    // T15 calls NSLog(@"T15 str1=%@", str1) where str1 = "hello"
    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T15 str1=hello")),
        "NSLog should output 'T15 str1=hello', got: {:?}",
        lines,
    );
}

// ── T16: NSArray basics ────────────────────────────────────────────

#[test]
fn runtime_nsarray_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsarray_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsarray_basics should exit 0");
}

#[test]
fn runtime_nsarray_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsarray_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsarray = rt.get_class("NSArray").expect("NSArray registered");
        assert_eq!(nsarray.name, "NSArray");
        assert_eq!(nsarray.instance_size, 24);
        assert_eq!(
            nsarray.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsarray_nslog_captures_element() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsarray_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    // T16 calls NSLog(@"T16 first=%@", first) where first = "apple"
    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T16 first=apple")),
        "NSLog should output 'T16 first=apple', got: {:?}",
        lines,
    );
}

// ── T17: NSNumber basics ─────────────────────────────────────────

#[test]
fn runtime_nsnumber_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnumber_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsnumber_basics should exit 0");
}

#[test]
fn runtime_nsnumber_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnumber_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsnumber = rt.get_class("NSNumber").expect("NSNumber registered");
        assert_eq!(nsnumber.name, "NSNumber");
        assert_eq!(nsnumber.instance_size, 24);
        assert_eq!(
            nsnumber.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsnumber_nslog_captures_value() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnumber_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    // T17 calls NSLog(@"T17 n42=%@", n42) where n42 = numberWithInt:42
    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T17 n42=42")),
        "NSLog should output 'T17 n42=42', got: {:?}",
        lines,
    );
}

// ── T18: NSDictionary basics ─────────────────────────────────────

#[test]
fn runtime_nsdictionary_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdictionary_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsdictionary_basics should exit 0");
}

#[test]
fn runtime_nsdictionary_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdictionary_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsdict = rt
            .get_class("NSDictionary")
            .expect("NSDictionary registered");
        assert_eq!(nsdict.name, "NSDictionary");
        assert_eq!(nsdict.instance_size, 32);
        assert_eq!(
            nsdict.superclass_addr,
            Some(cf_lite::nsobj::nsobject_class_addr())
        );
    }
}

#[test]
fn runtime_nsdictionary_nslog_captures_description() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdictionary_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T18 dict=")),
        "NSLog should output 'T18 dict=...', got: {:?}",
        lines,
    );
}

// ── T19: NSString manipulation ──────────────────────────────────────

#[test]
fn runtime_nsstring_manip_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_manip/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_manip should exit 0");
}

#[test]
fn runtime_nsstring_manip_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_manip/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T19 appended=foobar")),
        "NSLog should output 'T19 appended=foobar', got: {:?}",
        lines,
    );
}

// ── T20: NSString stringWithFormat: ─────────────────────────────────

#[test]
fn runtime_nsstring_format_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_format/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_format should exit 0");
}

#[test]
fn runtime_nsstring_format_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_format/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("T20 formatted=item=widget")),
        "NSLog should output 'T20 formatted=item=widget', got: {:?}",
        lines,
    );
}

// ── T21: NSMutableArray basics ──────────────────────────────────────

#[test]
fn runtime_nsmutablearray_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsmutablearray_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsmutablearray_basics should exit 0");
}

#[test]
fn runtime_nsmutablearray_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsmutablearray_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T21 mutarr=")),
        "NSLog should output 'T21 mutarr=...', got: {:?}",
        lines,
    );
}

// ── T22: NSMutableDictionary basics ─────────────────────────────────

#[test]
fn runtime_nsmutabledictionary_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsmutabledictionary_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(
        result.exit_code, 0,
        "nsmutabledictionary_basics should exit 0"
    );
}

#[test]
fn runtime_nsmutabledictionary_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsmutabledictionary_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T22 mutdict=")),
        "NSLog should output 'T22 mutdict=...', got: {:?}",
        lines,
    );
}

// ── T23: NSLog format specifiers ────────────────────────────────────

#[test]
fn runtime_nslog_format_specifiers_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nslog_format_specifiers/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nslog_format_specifiers should exit 0");
}

#[test]
fn runtime_nslog_format_specifiers_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nslog_format_specifiers/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T23 float=3.14")),
        "expected 'T23 float=3.14...', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 hex=ff")),
        "expected 'T23 hex=ff', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 lld=-123456789")),
        "expected 'T23 lld=-123456789', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 oct=10")),
        "expected 'T23 oct=10', got: {:?}",
        lines,
    );
}

// ── T21: NSMutableString + NSString enhancements ────────────────────

#[test]
fn runtime_nsstring_mutable_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_mutable/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_mutable should exit 0");
}

#[test]
fn runtime_nsstring_mutable_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_mutable/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T21 appended=Hello World")),
        "NSLog should output 'T21 appended=Hello World', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T21 replaced=Hello Rust")),
        "NSLog should output 'T21 replaced=Hello Rust', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T21 final=testing")),
        "NSLog should output 'T21 final=testing', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_nsstring_mutable_class_hierarchy() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_mutable/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // NSMutableString exists and its superclass is NSString
    let ms_info = rt.get_class("NSMutableString");
    assert!(
        ms_info.is_some(),
        "NSMutableString class should be registered"
    );
    let ms = ms_info.unwrap();
    assert_eq!(
        ms.superclass_addr,
        Some(cf_lite::nsobj::nsstring_class_addr())
    );

    // Key selectors should be resolvable
    for sel in &[
        "appendString:",
        "insertString:atIndex:",
        "deleteCharactersInRange:",
        "replaceCharactersInRange:withString:",
        "setString:",
        "length",
        "UTF8String",
    ] {
        let imp = rt.lookup_method("NSMutableString", sel);
        assert!(imp.is_some(), "NSMutableString should respond to {}", sel);
    }

    // NSString should have the new methods
    for sel in &[
        "stringByReplacingOccurrencesOfString:withString:",
        "componentsSeparatedByString:",
        "rangeOfString:",
        "integerValue",
        "doubleValue",
        "mutableCopy",
    ] {
        let imp = rt.lookup_method("NSString", sel);
        assert!(imp.is_some(), "NSString should respond to {}", sel);
    }
}

// ── T22: Autorelease pools + retain/release ─────────────────────────

#[test]
fn runtime_autorelease_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("autorelease/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    assert_eq!(result.exit_code, 0, "autorelease fixture should exit 0");
}

#[test]
fn runtime_autorelease_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("autorelease/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("T22 pool1=hello autorelease")),
        "NSLog should output 'T22 pool1=hello autorelease', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T22 done=ok")),
        "NSLog should output 'T22 done=ok', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_autorelease_nsobject_memory_methods() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("autorelease/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // NSObject should have memory management methods
    for sel in &["retain", "release", "autorelease", "retainCount", "dealloc"] {
        let imp = rt.lookup_method("NSObject", sel);
        assert!(imp.is_some(), "NSObject should respond to {}", sel);
    }
}

// ── T23: runtime_introspection ─────────────────────────────────────

#[test]
fn runtime_introspection_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("runtime_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0, "runtime_introspection should exit 0");
}

#[test]
fn runtime_introspection_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("runtime_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T23 done=ok")),
        "NSLog should output 'T23 done=ok', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 addMethod=YES")),
        "NSLog should output 'T23 addMethod=YES', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 isMetaClass=NO")),
        "NSLog should output 'T23 isMetaClass=NO', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T23 methodName=alpha")),
        "NSLog should output 'T23 methodName=alpha', got: {:?}",
        lines,
    );
}

#[test]
fn runtime_introspection_shims_resolvable() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("runtime_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // Probe class should exist with its methods
    assert!(
        rt.lookup_method("Probe", "alpha").is_some(),
        "Probe should have alpha"
    );
    assert!(
        rt.lookup_method("Probe", "beta:").is_some(),
        "Probe should have beta:"
    );
}

// ── T19: property_introspection ────────────────────────────────────

#[test]
fn runtime_property_introspection_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("property_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0, "property_introspection should exit 0");
}

#[test]
fn runtime_property_introspection_nslog_captures() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("property_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    cf_lite::nslog::drain_captured_nslog();

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");
    assert_eq!(result.exit_code, 0);

    let lines = cf_lite::nslog::drain_captured_nslog();
    assert!(
        lines.iter().any(|l| l.contains("T19 done=ok")),
        "NSLog should output 'T19 done=ok', got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T19 propCount=")),
        "NSLog should output property count, got: {:?}",
        lines,
    );
    assert!(
        lines.iter().any(|l| l.contains("T19 ivarCount=")),
        "NSLog should output ivar count, got: {:?}",
        lines,
    );
}

#[test]
fn runtime_property_introspection_class_has_ivars() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("property_introspection/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let vehicle = rt.get_class("Vehicle").expect("Vehicle class should exist");
    assert!(!vehicle.ivars.is_empty(), "Vehicle should have ivars");
    assert!(
        !vehicle.properties.is_empty(),
        "Vehicle should have properties"
    );

    // Verify specific ivar names
    let ivar_names: Vec<&str> = vehicle.ivars.iter().map(|iv| iv.name.as_str()).collect();
    assert!(
        ivar_names.contains(&"_year"),
        "Vehicle should have _year ivar, got: {:?}",
        ivar_names
    );
    assert!(
        ivar_names.contains(&"_horsepower"),
        "Vehicle should have _horsepower ivar, got: {:?}",
        ivar_names
    );

    // Verify specific property names
    let prop_names: Vec<&str> = vehicle.properties.iter().map(|p| p.name.as_str()).collect();
    assert!(
        prop_names.contains(&"make"),
        "Vehicle should have make property, got: {:?}",
        prop_names
    );
    assert!(
        prop_names.contains(&"horsepower"),
        "Vehicle should have horsepower property, got: {:?}",
        prop_names
    );
}

// ── T24: NSData basics ─────────────────────────────────────────────

#[test]
fn runtime_nsdata_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdata_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsdata_basics execution failed");

    assert_eq!(result.exit_code, 0, "nsdata_basics should exit 0");
}

#[test]
fn runtime_nsdata_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdata_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsdata = rt.get_class("NSData").expect("NSData should be registered");
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "length").is_some(),
        "NSData.length"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "bytes").is_some(),
        "NSData.bytes"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "description").is_some(),
        "NSData.description"
    );
    assert!(
        rt.msg_lookup(nsdata.mapped_addr, "isEqualToData:")
            .is_some(),
        "NSData.isEqualToData:"
    );
}

// ── T25: NSString path utilities ───────────────────────────────────

#[test]
fn runtime_nsstring_path_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_path/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsstring_path execution failed");

    assert_eq!(result.exit_code, 0, "nsstring_path should exit 0");
}

#[test]
fn runtime_nsstring_path_methods_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsstring_path/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsstring = rt
        .get_class("NSString")
        .expect("NSString should be registered");
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "pathExtension")
            .is_some(),
        "NSString.pathExtension"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "lastPathComponent")
            .is_some(),
        "NSString.lastPathComponent"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByDeletingLastPathComponent")
            .is_some(),
        "NSString.stringByDeletingLastPathComponent"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByDeletingPathExtension")
            .is_some(),
        "NSString.stringByDeletingPathExtension"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "stringByAppendingPathComponent:")
            .is_some(),
        "NSString.stringByAppendingPathComponent:"
    );
    assert!(
        rt.msg_lookup(nsstring.mapped_addr, "isAbsolutePath")
            .is_some(),
        "NSString.isAbsolutePath"
    );
}

// ── T26: NSProcessInfo basics ──────────────────────────────────────

#[test]
fn runtime_nsprocessinfo_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsprocessinfo_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsprocessinfo_basics execution failed");

    assert_eq!(result.exit_code, 0, "nsprocessinfo_basics should exit 0");
}

#[test]
fn runtime_nsprocessinfo_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsprocessinfo_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nspi = rt
        .get_class("NSProcessInfo")
        .expect("NSProcessInfo should be registered");
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "processName").is_some(),
        "NSProcessInfo.processName"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "processIdentifier")
            .is_some(),
        "NSProcessInfo.processIdentifier"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "hostName").is_some(),
        "NSProcessInfo.hostName"
    );
    assert!(
        rt.msg_lookup(nspi.mapped_addr, "description").is_some(),
        "NSProcessInfo.description"
    );
}

// ── T20: blocks + Foundation (enumerateObjects, sort, dispatch_sync) ──

#[test]
fn runtime_blocks_foundation_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("blocks_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("blocks_foundation execution failed");

    assert_eq!(result.exit_code, 0, "blocks_foundation should exit 0");
}

#[test]
fn runtime_blocks_foundation_methods_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("blocks_foundation/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsarray = rt
        .get_class("NSArray")
        .expect("NSArray should be registered");
    assert!(
        rt.msg_lookup(nsarray.mapped_addr, "enumerateObjectsUsingBlock:")
            .is_some(),
        "NSArray.enumerateObjectsUsingBlock:"
    );
    assert!(
        rt.msg_lookup(nsarray.mapped_addr, "sortedArrayUsingComparator:")
            .is_some(),
        "NSArray.sortedArrayUsingComparator:"
    );

    let nsmutarr = rt
        .get_class("NSMutableArray")
        .expect("NSMutableArray should be registered");
    assert!(
        rt.msg_lookup(nsmutarr.mapped_addr, "sortUsingComparator:")
            .is_some(),
        "NSMutableArray.sortUsingComparator:"
    );

    let nsdict = rt
        .get_class("NSDictionary")
        .expect("NSDictionary should be registered");
    assert!(
        rt.msg_lookup(nsdict.mapped_addr, "enumerateKeysAndObjectsUsingBlock:")
            .is_some(),
        "NSDictionary.enumerateKeysAndObjectsUsingBlock:"
    );
}

// ── T27: NSError basics ────────────────────────────────────────────

#[test]
fn runtime_nserror_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nserror_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nserror_basics execution failed");

    assert_eq!(result.exit_code, 0, "nserror_basics should exit 0");
}

#[test]
fn runtime_nserror_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nserror_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nserror = rt
        .get_class("NSError")
        .expect("NSError should be registered");
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "domain").is_some(),
        "NSError.domain"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "code").is_some(),
        "NSError.code"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "userInfo").is_some(),
        "NSError.userInfo"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "localizedDescription")
            .is_some(),
        "NSError.localizedDescription"
    );
    assert!(
        rt.msg_lookup(nserror.mapped_addr, "description").is_some(),
        "NSError.description"
    );
}

// ── T28: NSBundle basics ───────────────────────────────────────────

#[test]
fn runtime_nsbundle_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsbundle_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsbundle_basics execution failed");

    assert_eq!(result.exit_code, 0, "nsbundle_basics should exit 0");
}

#[test]
fn runtime_nsbundle_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsbundle_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsbundle = rt
        .get_class("NSBundle")
        .expect("NSBundle should be registered");
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "bundlePath").is_some(),
        "NSBundle.bundlePath"
    );
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "executablePath")
            .is_some(),
        "NSBundle.executablePath"
    );
    assert!(
        rt.msg_lookup(nsbundle.mapped_addr, "bundleIdentifier")
            .is_some(),
        "NSBundle.bundleIdentifier"
    );
}

// ── T29: NSFileManager basics ──────────────────────────────────────

#[test]
fn runtime_nsfilemanager_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsfilemanager_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsfilemanager_basics execution failed");

    assert_eq!(result.exit_code, 0, "nsfilemanager_basics should exit 0");
}

#[test]
fn runtime_nsfilemanager_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsfilemanager_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsfm = rt
        .get_class("NSFileManager")
        .expect("NSFileManager should be registered");
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "fileExistsAtPath:")
            .is_some(),
        "NSFileManager.fileExistsAtPath:"
    );
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "currentDirectoryPath")
            .is_some(),
        "NSFileManager.currentDirectoryPath"
    );
    assert!(
        rt.msg_lookup(nsfm.mapped_addr, "contentsOfDirectoryAtPath:error:")
            .is_some(),
        "NSFileManager.contentsOfDirectoryAtPath:error:"
    );
}

// ── T30: Exception basics ──────────────────────────────────────────

#[test]
fn runtime_exception_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("exception_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("exception_basics execution failed");

    assert_eq!(result.exit_code, 0, "exception_basics should exit 0");
}

#[test]
fn runtime_nsexception_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("exception_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsexc = rt
        .get_class("NSException")
        .expect("NSException should be registered");
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "name").is_some(),
        "NSException.name"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "reason").is_some(),
        "NSException.reason"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "userInfo").is_some(),
        "NSException.userInfo"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "description").is_some(),
        "NSException.description"
    );
    assert!(
        rt.msg_lookup(nsexc.mapped_addr, "raise").is_some(),
        "NSException.raise"
    );
}

// ── T31: Real CLI — comprehensive Foundation integration ──────────

#[test]
fn runtime_real_cli_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("real_cli/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("real_cli execution failed");

    assert_eq!(result.exit_code, 0, "real_cli should exit 0");
}

#[test]
fn runtime_real_cli_classes_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("real_cli/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    // Verify all Foundation classes used by real_cli are registered
    for class_name in &[
        "NSString",
        "NSMutableString",
        "NSArray",
        "NSMutableArray",
        "NSDictionary",
        "NSMutableDictionary",
        "NSNumber",
        "NSData",
        "NSProcessInfo",
        "NSFileManager",
        "NSBundle",
        "NSError",
        "NSException",
    ] {
        assert!(
            rt.get_class(class_name).is_some(),
            "{class_name} should be registered"
        );
    }
}

// ── T32: Collection Enumeration ─────────────────────────────────────

#[test]
fn runtime_collection_enum_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("collection_enum/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("collection_enum execution failed");
    assert_eq!(result.exit_code, 0, "collection_enum should exit 0");
}

#[test]
fn runtime_nsenumerator_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("collection_enum/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsenum = rt
        .get_class("NSEnumerator")
        .expect("NSEnumerator should be registered");
    assert!(
        rt.msg_lookup(nsenum.mapped_addr, "nextObject").is_some(),
        "NSEnumerator.nextObject"
    );
    assert!(
        rt.msg_lookup(nsenum.mapped_addr, "allObjects").is_some(),
        "NSEnumerator.allObjects"
    );
}

// ── T33: NSDate Basics ──────────────────────────────────────────────

#[test]
fn runtime_nsdate_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdate_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsdate_basics execution failed");
    assert_eq!(result.exit_code, 0, "nsdate_basics should exit 0");
}

#[test]
fn runtime_nsdate_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdate_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsdate = rt.get_class("NSDate").expect("NSDate should be registered");
    assert!(rt
        .msg_lookup(nsdate.mapped_addr, "timeIntervalSince1970")
        .is_some());
    assert!(rt.msg_lookup(nsdate.mapped_addr, "description").is_some());
    assert!(rt.msg_lookup(nsdate.mapped_addr, "compare:").is_some());
    assert!(rt
        .msg_lookup(nsdate.mapped_addr, "isEqualToDate:")
        .is_some());
}

// ── T34: NSUserDefaults Basics ──────────────────────────────────────

#[test]
fn runtime_nsuserdefaults_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsuserdefaults_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsuserdefaults execution failed");
    assert_eq!(result.exit_code, 0, "nsuserdefaults_basics should exit 0");
}

#[test]
fn runtime_nsuserdefaults_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsuserdefaults_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsud = rt
        .get_class("NSUserDefaults")
        .expect("NSUserDefaults should be registered");
    assert!(rt.msg_lookup(nsud.mapped_addr, "objectForKey:").is_some());
    assert!(rt
        .msg_lookup(nsud.mapped_addr, "setObject:forKey:")
        .is_some());
    assert!(rt.msg_lookup(nsud.mapped_addr, "synchronize").is_some());
}

#[test]
fn runtime_nsset_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsset_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsset execution failed");
    assert_eq!(result.exit_code, 0, "nsset_basics should exit 0");
}

#[test]
fn runtime_nsset_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsset_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsset = rt.get_class("NSSet").expect("NSSet should be registered");
    assert!(rt.msg_lookup(nsset.mapped_addr, "count").is_some());
    assert!(rt
        .msg_lookup(nsset.mapped_addr, "containsObject:")
        .is_some());

    let nsmset = rt
        .get_class("NSMutableSet")
        .expect("NSMutableSet should be registered");
    assert!(rt.msg_lookup(nsmset.mapped_addr, "addObject:").is_some());
    assert!(rt.msg_lookup(nsmset.mapped_addr, "removeObject:").is_some());
}

#[test]
fn runtime_nsurl_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsurl_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsurl execution failed");
    assert_eq!(result.exit_code, 0, "nsurl_basics should exit 0");
}

#[test]
fn runtime_nsurl_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsurl_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsurl = rt.get_class("NSURL").expect("NSURL should be registered");
    assert!(rt.msg_lookup(nsurl.mapped_addr, "path").is_some());
    assert!(rt.msg_lookup(nsurl.mapped_addr, "absoluteString").is_some());
    assert!(rt.msg_lookup(nsurl.mapped_addr, "isFileURL").is_some());
}

#[test]
fn runtime_nsnull_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnull_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsnull execution failed");
    assert_eq!(result.exit_code, 0, "nsnull_basics should exit 0");
}

#[test]
fn runtime_nsnull_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnull_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let nsnull = rt.get_class("NSNull").expect("NSNull should be registered");
    assert!(rt.msg_lookup(nsnull.mapped_addr, "description").is_some());
}

#[test]
fn runtime_nsdateformatter_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSDateFormatter")
        .expect("NSDateFormatter should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "stringFromDate:").is_some());
}

#[test]
fn runtime_nscharacterset_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSCharacterSet")
        .expect("NSCharacterSet should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "characterIsMember:")
        .is_some());
}

#[test]
fn runtime_nsnotificationcenter_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSNotificationCenter")
        .expect("NSNotificationCenter should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "postNotificationName:object:")
        .is_some());
}

#[test]
fn runtime_nsdateformatter_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsdateformatter_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsdateformatter execution failed");
    assert_eq!(result.exit_code, 0, "nsdateformatter_basics should exit 0");
}

#[test]
fn runtime_nscharacterset_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nscharacterset_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nscharacterset execution failed");
    assert_eq!(result.exit_code, 0, "nscharacterset_basics should exit 0");
}

#[test]
fn runtime_nsnotification_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnotification_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsnotification execution failed");
    assert_eq!(result.exit_code, 0, "nsnotification_basics should exit 0");
}

// ── Phase 11: NSValue, NSUUID, NSLocale, NSTimeZone, NSCalendar ────

#[test]
fn runtime_nsvalue_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSValue")
        .expect("NSValue should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "rangeValue").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "pointerValue").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "objCType").is_some());
}

#[test]
fn runtime_nsuuid_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt.get_class("NSUUID").expect("NSUUID should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "UUIDString").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "isEqual:").is_some());
}

#[test]
fn runtime_nslocale_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSLocale")
        .expect("NSLocale should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "localeIdentifier").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "languageCode").is_some());
}

#[test]
fn runtime_nstimezone_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSTimeZone")
        .expect("NSTimeZone should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "name").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "secondsFromGMT").is_some());
}

#[test]
fn runtime_nscalendar_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSCalendar")
        .expect("NSCalendar should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "calendarIdentifier")
        .is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "components:fromDate:")
        .is_some());
}

#[test]
fn runtime_nsvalue_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsvalue_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsvalue execution failed");
    assert_eq!(result.exit_code, 0, "nsvalue_basics should exit 0");
}

#[test]
fn runtime_nsuuid_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsuuid_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsuuid execution failed");
    assert_eq!(result.exit_code, 0, "nsuuid_basics should exit 0");
}

#[test]
fn runtime_nslocale_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nslocale_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nslocale execution failed");
    assert_eq!(result.exit_code, 0, "nslocale_basics should exit 0");
}

#[test]
fn runtime_nstimezone_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nstimezone_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nstimezone execution failed");
    assert_eq!(result.exit_code, 0, "nstimezone_basics should exit 0");
}

#[test]
fn runtime_nscalendar_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nscalendar_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nscalendar execution failed");
    assert_eq!(result.exit_code, 0, "nscalendar_basics should exit 0");
}

#[test]
fn runtime_nsnumberformatter_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSNumberFormatter")
        .expect("NSNumberFormatter should be registered");
    assert!(rt
        .msg_lookup(cls.mapped_addr, "stringFromNumber:")
        .is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "numberFromString:")
        .is_some());
}

#[test]
fn runtime_nsindexset_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSIndexSet")
        .expect("NSIndexSet should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "count").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "containsIndex:").is_some());

    let mcls = rt
        .get_class("NSMutableIndexSet")
        .expect("NSMutableIndexSet should be registered");
    assert!(rt.msg_lookup(mcls.mapped_addr, "addIndex:").is_some());
}

#[test]
fn runtime_nssortdescriptor_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSSortDescriptor")
        .expect("NSSortDescriptor should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "key").is_some());
    assert!(rt
        .msg_lookup(cls.mapped_addr, "compareObject:toObject:")
        .is_some());
}

#[test]
fn runtime_nstimer_class_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("return42/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("runtime execution failed");

    let rt = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    let rt = rt.as_ref().expect("ObjcRuntime should be initialized");

    let cls = rt
        .get_class("NSTimer")
        .expect("NSTimer should be registered");
    assert!(rt.msg_lookup(cls.mapped_addr, "fire").is_some());
    assert!(rt.msg_lookup(cls.mapped_addr, "isValid").is_some());
}

#[test]
fn runtime_nsnumberformatter_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsnumberformatter_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsnumberformatter execution failed");
    assert_eq!(
        result.exit_code, 0,
        "nsnumberformatter_basics should exit 0"
    );
}

#[test]
fn runtime_nsindexset_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsindexset_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nsindexset execution failed");
    assert_eq!(result.exit_code, 0, "nsindexset_basics should exit 0");
}

#[test]
fn runtime_nssortdescriptor_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nssortdescriptor_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nssortdescriptor execution failed");
    assert_eq!(result.exit_code, 0, "nssortdescriptor_basics should exit 0");
}

#[test]
fn runtime_nstimer_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nstimer_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("nstimer execution failed");
    assert_eq!(result.exit_code, 0, "nstimer_basics should exit 0");
}

#[test]
fn runtime_try_catch_test_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("try_catch_test/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("try_catch_test execution failed");
    assert_eq!(
        result.exit_code, 0,
        "try_catch_test should exit 0 (@try/@catch/@finally)"
    );
}

#[test]
fn runtime_tracing_collects_pipeline_events() {
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

// ── KVO (Key-Value Observing) tests (Phase B1) ────────────────────

#[test]
fn runtime_kvo_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvo_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("kvo_basics should run");

    assert_eq!(result.exit_code, 0, "kvo_basics should exit 0");
}

#[test]
fn runtime_kvo_basics_nsobject_has_kvo_methods() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvo_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("kvo_basics should run");

    let guard = objc_runtime::OBJC_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        let nsobject = rt.get_class("NSObject").expect("NSObject should exist");
        let kvo_methods = [
            "addObserver:forKeyPath:options:context:",
            "removeObserver:forKeyPath:",
            "willChangeValueForKey:",
            "didChangeValueForKey:",
            "observeValueForKeyPath:ofObject:change:context:",
        ];
        for sel in &kvo_methods {
            assert!(
                nsobject.methods.iter().any(|m| m.selector == *sel),
                "NSObject should have KVO method '{sel}'"
            );
        }
    }
}

#[test]
fn runtime_kvo_multiple_observers_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvo_multiple_observers/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("kvo_multiple_observers should run");

    assert_eq!(
        result.exit_code, 0,
        "kvo_multiple_observers should exit 0"
    );
}

// ── KVC tests ──────────────────────────────────────────────────────

#[test]
fn runtime_kvc_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvc_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("kvc_basics should run");

    assert_eq!(result.exit_code, 0, "kvc_basics should exit 0");
}

#[test]
fn runtime_kvc_nsobject_has_kvc_methods() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvc_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let _result = Runtime::load_and_run(&fixture, &config).expect("kvc_basics should run");

    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(rt) = guard.as_ref() {
        let nsobject = rt.get_class("NSObject").expect("NSObject should exist");
        let kvc_methods = [
            "valueForKey:",
            "setValue:forKey:",
            "valueForKeyPath:",
            "setValue:forKeyPath:",
            "valueForUndefinedKey:",
            "setValue:forUndefinedKey:",
        ];
        for sel in &kvc_methods {
            assert!(
                nsobject.methods.iter().any(|m| m.selector == *sel),
                "NSObject should have KVC method '{sel}'"
            );
        }
    }
}

#[test]
fn runtime_kvc_simple_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("kvc_simple/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("kvc_simple should run");

    assert_eq!(result.exit_code, 0, "kvc_simple should exit 0");
}

// ── Method Swizzling tests ─────────────────────────────────────────

#[test]
fn runtime_swizzle_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("swizzle_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("swizzle_basics should run");

    assert_eq!(result.exit_code, 0, "swizzle_basics should exit 0");
}

#[test]
fn runtime_swizzle_restore_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("swizzle_restore/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("swizzle_restore should run");

    assert_eq!(result.exit_code, 0, "swizzle_restore should exit 0");
}

#[test]
fn runtime_swizzle_shims_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    // All three swizzling APIs should be in the shim table
    assert!(
        ctx.shim_table.resolve("_method_exchangeImplementations").is_some(),
        "method_exchangeImplementations should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_method_setImplementation").is_some(),
        "method_setImplementation should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_class_replaceMethod").is_some(),
        "class_replaceMethod should be registered"
    );
}

// ── Phase B4: NSInvocation & Dynamic Method Resolution ─────────────

#[test]
fn runtime_nsinvocation_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsinvocation_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsinvocation_basics should run");

    assert_eq!(
        result.exit_code, 0,
        "nsinvocation_basics fixture should exit 0"
    );
}

#[test]
fn runtime_forwarding_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("forwarding_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("forwarding_basics should run");

    assert_eq!(
        result.exit_code, 0,
        "forwarding_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nsinvocation_data_symbols_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_OBJC_CLASS_$_NSMethodSignature").is_some(),
        "NSMethodSignature class data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_METACLASS_$_NSMethodSignature").is_some(),
        "NSMethodSignature metaclass data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_CLASS_$_NSInvocation").is_some(),
        "NSInvocation class data symbol should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_OBJC_METACLASS_$_NSInvocation").is_some(),
        "NSInvocation metaclass data symbol should be registered"
    );
}

// ── ARM64e / PAC tests ─────────────────────────────────────────────

#[test]
fn runtime_arm64e_basic_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("arm64e_basic/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result = Runtime::load_and_run(&fixture, &config).expect("arm64e runtime execution failed");

    assert_eq!(result.exit_code, 0, "arm64e_basic should exit with code 0");
    assert!(result.shims_installed > 0);
}

#[test]
fn ptrauth_shims_are_registered() {
    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;

    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    assert!(
        ctx.shim_table.resolve("_ptrauth_strip").is_some(),
        "ptrauth_strip shim should be registered"
    );
    assert!(
        ctx.shim_table.resolve("_ptrauth_sign_unauthenticated").is_some(),
        "ptrauth_sign_unauthenticated shim should be registered"
    );
}

// ── Phase B6: Additional Foundation Classes ────────────────────────

#[test]
fn runtime_nsjson_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsjson_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsjson_basics should run");

    assert_eq!(
        result.exit_code, 0,
        "nsjson_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nsregex_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nsregex_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nsregex_basics should run");

    assert_eq!(
        result.exit_code, 0,
        "nsregex_basics fixture should exit 0"
    );
}

#[test]
fn runtime_nscache_basics_exit_zero() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fixture = fixtures_dir().join("nscache_basics/app");
    if !fixture.exists() {
        eprintln!("Skipping: fixture not found at {}", fixture.display());
        return;
    }

    let config = RuntimeConfig::permissive(&fixture);
    let result =
        Runtime::load_and_run(&fixture, &config).expect("nscache_basics should run");

    assert_eq!(
        result.exit_code, 0,
        "nscache_basics fixture should exit 0"
    );
}

#[test]
fn runtime_b6_data_symbols_registered() {
    let _lock = RUNTIME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use libsystem_shims::ShimConfig;
    use macho_runtime::RuntimeContext;
    let config = ShimConfig::default();
    let shim_set = ShimSet::default();
    let ctx = RuntimeContext::new(&config, &shim_set);

    let classes = [
        "NSRegularExpression",
        "NSPredicate",
        "NSAttributedString",
        "NSMutableAttributedString",
        "NSCache",
        "NSMapTable",
        "NSHashTable",
        "NSOperationQueue",
        "NSOperation",
        "NSBlockOperation",
        "NSJSONSerialization",
        "NSPropertyListSerialization",
    ];

    for name in &classes {
        let class_sym = format!("_OBJC_CLASS_$_{}", name);
        let meta_sym = format!("_OBJC_METACLASS_$_{}", name);
        assert!(
            ctx.shim_table.resolve(&class_sym).is_some(),
            "{} class data symbol should be registered",
            name
        );
        assert!(
            ctx.shim_table.resolve(&meta_sym).is_some(),
            "{} metaclass data symbol should be registered",
            name
        );
    }
}
