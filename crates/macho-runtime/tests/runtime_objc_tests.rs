//! ObjC runtime, category/load, Foundation/CF-lite, and multi-dylib ObjC tests.

mod common;

use common::{fixtures_dir, ShimSet, RUNTIME_LOCK};

#[test]
fn register_foundation_classes_registers_key_classes_and_tagged_pointers() {
    use macho_runtime::{RuntimeConfig, RuntimeTracer};
    use std::path::Path;

    let mut rt = objc_runtime::ObjcRuntime::new();
    let config = RuntimeConfig::permissive(Path::new("/tmp/foundation-test"));
    macho_runtime::objc_setup::register_foundation_classes(
        &mut rt,
        &config,
        &RuntimeTracer::disabled(),
    );

    for class_name in [
        "NSObject",
        "NSString",
        "NSArray",
        "NSDictionary",
        "NSAttributedString",
        "NSURLSession",
        "NSTask",
        "NSAutoreleasePool",
    ] {
        assert!(
            rt.get_class(class_name).is_some(),
            "expected {class_name} to be registered"
        );
    }

    let number_tagged = rt
        .tagged_class_for_pointer(2u64 << 60)
        .expect("NSNumber tagged pointer class should be registered");
    let string_tagged = rt
        .tagged_class_for_pointer(3u64 << 60)
        .expect("NSString tagged pointer class should be registered");
    let date_tagged = rt
        .tagged_class_for_pointer(6u64 << 60)
        .expect("NSDate tagged pointer class should be registered");

    assert_eq!(
        rt.get_class_by_addr(number_tagged)
            .map(|cls| cls.name.as_str()),
        Some("NSNumber")
    );
    assert_eq!(
        rt.get_class_by_addr(string_tagged)
            .map(|cls| cls.name.as_str()),
        Some("NSString")
    );
    assert_eq!(
        rt.get_class_by_addr(date_tagged)
            .map(|cls| cls.name.as_str()),
        Some("NSDate")
    );
}

#[test]
fn register_foundation_classes_registers_host_isa_aliases_when_available() {
    use macho_runtime::{RuntimeConfig, RuntimeTracer};
    use std::path::Path;

    let mut rt = objc_runtime::ObjcRuntime::new();
    let config = RuntimeConfig::permissive(Path::new("/tmp/foundation-alias-test"));
    macho_runtime::objc_setup::register_foundation_classes(
        &mut rt,
        &config,
        &RuntimeTracer::disabled(),
    );

    #[cfg(target_os = "macos")]
    {
        use platform_core::symbols::HostSymbolResolver;

        let symbols = platform_darwin_host::DarwinSymbolResolver::new();
        for (symbol, class_name) in [
            ("OBJC_CLASS_$_NSConstantArray", "NSArray"),
            ("OBJC_CLASS_$_NSConstantDictionary", "NSConstantDictionary"),
            (
                "OBJC_CLASS_$_NSConstantIntegerNumber",
                "NSConstantIntegerNumber",
            ),
            ("OBJC_CLASS_$_NSException", "NSException"),
        ] {
            if let Some(addr) = symbols.resolve(symbol) {
                assert_eq!(
                    rt.get_class_by_addr(addr as u64)
                        .map(|cls| cls.name.as_str()),
                    Some(class_name),
                    "expected ISA alias for {symbol} to resolve to {class_name}"
                );
            }
        }
    }
}

#[test]
fn register_foundation_classes_overlays_foundation_without_losing_guest_classes() {
    use macho_runtime::{RuntimeConfig, RuntimeTracer};
    use std::collections::HashMap;
    use std::path::Path;

    let mut rt = objc_runtime::ObjcRuntime::new();
    rt.register_synthetic_class(objc_runtime::ClassInfo {
        name: "DVTSystemPrerequisitesPaths".to_string(),
        mapped_addr: 0xD470_1000,
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(cf_lite::nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });
    rt.register_metaclass_addr(0xD470_2000, "DVTSystemPrerequisitesPaths");
    rt.register_synthetic_class(objc_runtime::ClassInfo {
        name: "NSString".to_string(),
        mapped_addr: 0x1111,
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 8,
        is_root: false,
        is_meta: false,
        superclass_addr: None,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    });

    let config = RuntimeConfig::permissive(Path::new("/tmp/foundation-overlay-test"));
    macho_runtime::objc_setup::register_foundation_classes(
        &mut rt,
        &config,
        &RuntimeTracer::disabled(),
    );

    let guest_compat = rt
        .get_class("DVTSystemPrerequisitesPaths")
        .expect("guest compat class should remain registered");
    assert_eq!(guest_compat.mapped_addr, 0xD470_1000);
    assert!(
        guest_compat
            .find_class_method("licenseHostAppInfoPlistPath")
            .is_some(),
        "guest compat methods should still be attached after overlay"
    );
    assert!(
        guest_compat
            .find_class_method("hostAppVersionPlistPath")
            .is_some(),
        "guest compat methods should still be attached after overlay"
    );
    assert!(
        guest_compat
            .find_class_method("hostAppProductBuildVersion")
            .is_some(),
        "guest compat methods should still be attached after overlay"
    );

    let nsstring = rt
        .get_class("NSString")
        .expect("Foundation NSString should be registered");
    assert_ne!(
        nsstring.mapped_addr, 0x1111,
        "foundation overlay should replace colliding guest NSString"
    );
    assert!(
        nsstring
            .find_class_method("stringWithUTF8String:")
            .is_some(),
        "Foundation NSString methods should be present after overlay"
    );
}

#[test]
fn run_registration_pipeline_reports_foundation_and_selector_stats() {
    use macho_exec::ImageRegistry;
    use macho_runtime::{RuntimeConfig, RuntimeTracer};
    use std::path::Path;
    use std::sync::Arc;

    #[cfg(target_os = "macos")]
    let unwind_registry: Arc<dyn platform_core::unwind::UnwindRegistry> =
        Arc::new(platform_darwin_host::DarwinUnwindRegistry);
    #[cfg(target_os = "linux")]
    let unwind_registry: Arc<dyn platform_core::unwind::UnwindRegistry> =
        Arc::new(platform_linux::LinuxUnwindRegistry::new());

    let registry = ImageRegistry::new(
        config_platform(Path::new("/tmp/objc-stats-test")).memory(),
        unwind_registry,
    );
    let config = RuntimeConfig::permissive(Path::new("/tmp/objc-stats-test"));
    let tracer = RuntimeTracer::new();

    let stats = macho_runtime::objc_setup::run_registration_pipeline(&registry, &config, &tracer);

    assert_eq!(
        stats.images_scanned, 0,
        "empty registry should scan zero guest images"
    );
    assert_eq!(
        stats.objc_images, 0,
        "empty registry should contain no ObjC guest images"
    );
    assert_eq!(
        stats.foundation_classes_registered, 93,
        "foundation registration count should remain stable"
    );
    assert!(
        stats.foundation_registration_time >= std::time::Duration::ZERO,
        "foundation registration timing should be recorded"
    );

    let events = tracer.events();
    assert!(
        events.iter().any(|event| {
            event.message == "perf|objc_reg|counter|foundation_classes_registered|93"
        }),
        "tracer should emit foundation class count perf event"
    );
    assert!(
        events.iter().any(|event| {
            event
                .message
                .starts_with("perf|objc_reg|timing|foundation_registration_us|")
        }),
        "tracer should emit foundation registration timing perf event"
    );
    assert!(
        events.iter().any(|event| {
            event
                .message
                .starts_with("perf|objc_reg|timing|foundation_template_apply_us|")
        }),
        "tracer should emit foundation template apply timing perf event"
    );
    assert!(
        events.iter().any(|event| {
            event
                .message
                .starts_with("perf|objc_reg|timing|foundation_host_isa_alias_us|")
        }),
        "tracer should emit foundation host ISA alias timing perf event"
    );
}

fn config_platform(path: &std::path::Path) -> std::sync::Arc<dyn platform_core::PlatformProvider> {
    macho_runtime::RuntimeConfig::permissive(path).platform
}

// ── ObjC runtime tests ─────────────────────────────────────────────

#[test]
fn runtime_objc_minimal() {
    let Some(result) = common::run_fixture("objc_minimal/objc_minimal") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "objc_minimal should exit 0");
}

#[test]
fn runtime_objc_minimal_class_registration() {
    // After running objc_minimal, verify the global ObjC runtime was populated
    let Some(_result) = common::run_fixture("objc_minimal/objc_minimal") else {
        return;
    };

    // The global runtime should have registered the Answer class
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
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
    let Some(result) = common::run_fixture("objc_category/objc_category") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "objc_category should exit 0");
}

#[test]
fn runtime_objc_category_class_registered() {
    let Some(_result) = common::run_fixture("objc_category/objc_category") else {
        return;
    };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
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
    let Some(_result) = common::run_fixture("objc_category/objc_category") else {
        return;
    };

    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
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
    let Some(result) = common::run_fixture("objc_foundation/app") else {
        return;
    };

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
    let Some(result) = common::run_fixture("objc_classes/app") else {
        return;
    };

    assert_eq!(
        result.exit_code, 0,
        "objc_classes should exit 0 with foundation shims"
    );
}

#[test]
fn runtime_objc_classes_foundation_classes_registered() {
    let Some(_result) = common::run_fixture("objc_classes/app") else {
        return;
    };

    // Verify classes from the fixture were registered alongside foundation symbols
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
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
    let Some(result) = common::run_fixture("objc_dylib_chain/app") else {
        return;
    };

    assert_eq!(result.exit_code, 0, "objc_dylib_chain should exit 0");
    assert!(
        result.images_loaded >= 3,
        "should load app + libgreeter + libextras"
    );
}

#[test]
fn runtime_objc_dylib_chain_cross_image_class() {
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else {
        return;
    };

    // Greeter class should be registered (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        assert!(
            rt.get_class("Greeter").is_some(),
            "Greeter should be registered from libgreeter.dylib"
        );
    }
}

#[test]
fn runtime_objc_dylib_chain_cross_image_category() {
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else {
        return;
    };

    // Verify category from libextras was merged into Greeter from libgreeter
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
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
    let Some(_result) = common::run_fixture("objc_dylib_chain/app") else {
        return;
    };

    // Verify +load was detected for Greeter (defined in libgreeter.dylib)
    let guard = objc_runtime::OBJC_RUNTIME.read().unwrap();
    if let Some(rt) = guard.as_ref() {
        let loads = rt.collect_load_methods();
        assert!(
            loads.iter().any(|(name, _)| name == "Greeter"),
            "Greeter should have +load from libgreeter"
        );
    }
}
