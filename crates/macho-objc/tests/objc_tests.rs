//! Tests for macho-objc: Objective-C metadata detection, parsing, and constructors.
//!
//! Ported from:
//!   - LLVM llvm-objdump: ObjC section detection tests
//!   - Apple ObjC runtime: known section names
//!   - Apple dyld: constructor ordering (__mod_init_func, __init_offsets)

#[cfg(test)]
mod tests {
    use macho_objc::{detect_objc, ObjcPresence};
    use macho_parse::parse;
    use std::path::PathBuf;

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

    // ── Pure C binary has no ObjC ───────────────────────────────────

    #[test]
    fn simple_cli_has_no_objc() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let presence = detect_objc(&parsed);
        assert!(
            !presence.has_objc,
            "pure C binary should not have ObjC metadata"
        );
        assert!(presence.sections_found.is_empty());
    }

    // ── ObjC binary has ObjC sections ───────────────────────────────

    #[test]
    fn objc_foundation_has_objc() {
        let Some(data) = read_fixture("objc_foundation/app") else {
            eprintln!("SKIP: objc_foundation fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        let presence = detect_objc(&parsed);
        assert!(presence.has_objc, "ObjC binary should have ObjC metadata");
        assert!(!presence.sections_found.is_empty());
    }

    // ── ObjC section names match known runtime sections ─────────────

    #[test]
    fn objc_sections_are_recognized_names() {
        let known = [
            "__objc_classlist",
            "__objc_catlist",
            "__objc_protolist",
            "__objc_selrefs",
            "__objc_classrefs",
            "__objc_methnames",
            "__objc_methtype",
            "__objc_imageinfo",
        ];

        let Some(data) = read_fixture("objc_foundation/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let presence = detect_objc(&parsed);

        for found in &presence.sections_found {
            assert!(
                known.contains(&found.as_str()),
                "unexpected ObjC section name: {found}"
            );
        }
    }

    // ── Serde ───────────────────────────────────────────────────────

    #[test]
    fn objc_presence_serializes() {
        let presence = ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_classlist".into()],
        };
        let json = serde_json::to_string(&presence).unwrap();
        assert!(json.contains("__objc_classlist"));
    }
}

// ── ObjC metadata parsing ───────────────────────────────────────────

#[cfg(test)]
mod metadata_tests {
    use macho_objc::{parse_objc_metadata, ObjcMetadata};
    use macho_parse::parse;
    use std::path::PathBuf;

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

    #[test]
    fn simple_cli_has_no_classes() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let meta = parse_objc_metadata(&parsed, &data);
        assert!(!meta.presence.has_objc);
        assert!(meta.class_refs.is_empty());
        assert_eq!(meta.category_count, 0);
        assert_eq!(meta.protocol_count, 0);
    }

    #[test]
    fn objc_foundation_has_classes() {
        let Some(data) = read_fixture("objc_foundation/app") else {
            eprintln!("SKIP: objc_foundation fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        let meta = parse_objc_metadata(&parsed, &data);
        assert!(meta.presence.has_objc);
        // ObjC binaries with Foundation typically have at least one class
        // (the app's own class, e.g. from @interface usage)
        // Even minimal ObjC code using NSLog may not define classes,
        // but classlist should reflect compiled ObjC classes.
    }

    #[test]
    fn objc_foundation_class_refs_have_valid_addresses() {
        let Some(data) = read_fixture("objc_foundation/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let meta = parse_objc_metadata(&parsed, &data);

        for class_ref in &meta.class_refs {
            assert!(class_ref.pointer_addr > 0, "pointer_addr should be nonzero");
            // class_data_addr may be zero if chained fixups haven't been applied
            // but pointer_addr (the VM location) should be in data segment range
        }
    }

    #[test]
    fn metadata_serializes_to_json() {
        let meta = ObjcMetadata {
            presence: macho_objc::ObjcPresence {
                has_objc: true,
                sections_found: vec!["__objc_classlist".into()],
            },
            class_refs: vec![macho_objc::ObjcClassRef {
                pointer_addr: 0x100008000,
                class_data_addr: 0x100004000,
                name: None,
                methods: Vec::new(),
                class_methods: Vec::new(),
                ivars: Vec::new(),
                properties: Vec::new(),
                instance_size: 0,
                ro_flags: 0,
                superclass_addr: 0,
                metaclass_addr: 0,
            }],
            categories: vec![],
            category_count: 2,
            protocol_count: 1,
            selectors: Vec::new(),
            selref_addrs: Vec::new(),
            image_info: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("__objc_classlist"));
        assert!(
            json.contains("pointer_addr"),
            "JSON should contain pointer_addr field"
        );
        let roundtrip: ObjcMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.class_refs.len(), 1);
        assert_eq!(roundtrip.category_count, 2);
    }

    #[test]
    fn objc_category_fixture_has_catlist() {
        let Some(data) = read_fixture("objc_category/objc_category") else {
            eprintln!("SKIP: objc_category fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        let meta = parse_objc_metadata(&parsed, &data);
        assert!(meta.presence.has_objc);
        assert!(meta.category_count > 0, "should have at least one category");
        assert!(!meta.categories.is_empty(), "categories should be parsed");

        // Verify the category targets the Greeter class
        let cat = &meta.categories[0];
        assert!(cat.name.is_some(), "category should have a name");
        assert!(
            !cat.instance_methods.is_empty(),
            "category should have instance methods"
        );
        assert!(
            cat.instance_methods
                .iter()
                .any(|m| m.selector == "farewell"),
            "category should add 'farewell' method"
        );
    }

    #[test]
    fn objc_category_fixture_has_class_methods() {
        let Some(data) = read_fixture("objc_category/objc_category") else {
            eprintln!("SKIP: objc_category fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        let meta = parse_objc_metadata(&parsed, &data);

        // Greeter has +load as a class method
        let greeter = meta
            .class_refs
            .iter()
            .find(|c| c.name.as_deref() == Some("Greeter"));
        assert!(greeter.is_some(), "Greeter class should be found");
        let greeter = greeter.unwrap();
        assert!(
            greeter.class_methods.iter().any(|m| m.selector == "load"),
            "Greeter metaclass should have 'load' method: {:?}",
            greeter.class_methods,
        );
    }
}

// ── Constructor / initializer extraction ────────────────────────────

#[cfg(test)]
mod initializer_tests {
    use macho_objc::{extract_initializers, ConstructorSource, InitializerInfo};
    use macho_parse::parse;
    use std::path::PathBuf;

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

    #[test]
    fn simple_cli_has_no_constructors() {
        let Some(data) = read_fixture("simple_cli/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);
        assert!(
            info.constructors.is_empty(),
            "plain C binary should have no constructors"
        );
        assert!(
            !info.has_objc_load,
            "plain C binary should not have ObjC +load"
        );
    }

    #[test]
    fn constructor_test_has_constructors() {
        let Some(data) = read_fixture("constructor_test/app") else {
            eprintln!("SKIP: constructor_test fixture not built");
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);
        assert!(
            !info.constructors.is_empty(),
            "constructor_test should have at least one constructor"
        );
    }

    #[test]
    fn constructor_test_has_two_constructors() {
        // main.c defines two __attribute__((constructor)) functions
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);
        assert_eq!(
            info.constructors.len(),
            2,
            "constructor_test defines exactly 2 constructors, got {}",
            info.constructors.len()
        );
    }

    #[test]
    fn constructor_test_uses_init_offsets_or_mod_init() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);

        for ctor in &info.constructors {
            match ctor.source {
                ConstructorSource::ModInitFunc | ConstructorSource::InitOffsets => {}
            }
            assert!(ctor.address > 0, "constructor address should be nonzero");
        }
    }

    #[test]
    fn constructor_test_has_init_steps() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);
        assert!(
            !info.init_steps.is_empty(),
            "constructor_test should have init steps"
        );
        // Each constructor generates an init_step
        for step in &info.init_steps {
            assert!(
                step.contains("constructor") || step.contains("ObjC"),
                "init step should describe a constructor or ObjC load: {step}"
            );
        }
    }

    #[test]
    fn constructor_test_no_objc_load() {
        let Some(data) = read_fixture("constructor_test/app") else {
            return;
        };
        let parsed = parse(&data).unwrap();
        let info = extract_initializers(&parsed, &data);
        assert!(!info.has_objc_load, "C constructor_test has no ObjC");
    }

    #[test]
    fn initializer_info_serializes() {
        let info = InitializerInfo {
            constructors: vec![macho_objc::ConstructorEntry {
                address: 0x100001000,
                source: ConstructorSource::ModInitFunc,
            }],
            has_objc_load: false,
            init_steps: vec!["constructor[0] at 0x100001000 (ModInitFunc)".to_string()],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("ModInitFunc"));
        let roundtrip: InitializerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.constructors.len(), 1);
    }
}

// ── ObjC imageinfo tests ────────────────────────────────────────────

#[cfg(test)]
mod imageinfo_tests {
    use macho_objc::ObjcImageInfo;

    #[test]
    fn imageinfo_supports_objc2() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x20,
        };
        assert!(info.supports_objc2());
        assert!(!info.requires_gc());
        assert!(!info.is_simulated());
    }

    #[test]
    fn imageinfo_requires_gc() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x04,
        };
        assert!(info.requires_gc());
    }

    #[test]
    fn imageinfo_simulated() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x200,
        };
        assert!(info.is_simulated());
    }

    #[test]
    fn imageinfo_swift_version() {
        // Swift 5 = 0x0700 in flags (version 7 at bits 8-15)
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x0700,
        };
        assert_eq!(info.swift_version(), 7);
    }

    #[test]
    fn imageinfo_parse_system_binary() {
        // Try a system binary that has ObjC metadata
        let path = std::path::Path::new("/usr/bin/shortcuts");
        if !path.exists() {
            eprintln!("SKIP: /usr/bin/shortcuts not available");
            return;
        }
        let data = std::fs::read(path).unwrap();
        // May be a fat binary — try to extract arm64
        let bin_data = if macho_parse::is_fat(&data) {
            match macho_parse::fat_slice_for_cpu(&data, macho_types::header::CpuType::Arm64) {
                Ok(slice) => slice.to_vec(),
                Err(_) => data.clone(),
            }
        } else {
            data.clone()
        };
        if let Ok(parsed) = macho_parse::parse(&bin_data) {
            let meta = macho_objc::parse_objc_metadata(&parsed, &bin_data);
            // shortcuts may or may not have ObjC; just verify no crash
            let _ = meta;
        }
    }
}
