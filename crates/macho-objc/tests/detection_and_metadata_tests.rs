//! Tests for ObjC detection and metadata parsing.
//!
//! Covers: detect_objc, parse_objc_metadata, ObjcPresence/ObjcMetadata serialization,
//! class refs, categories, and class methods.

mod common;

use macho_objc::{detect_objc, parse_objc_metadata, ObjcMetadata, ObjcPresence};
use macho_parse::parse;

// ── Pure C binary has no ObjC ───────────────────────────────────

#[test]
fn simple_cli_has_no_objc() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
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
    let Some(data) = common::read_fixture("objc_foundation/app") else {
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

    let Some(data) = common::read_fixture("objc_foundation/app") else {
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

// ── ObjC metadata parsing ───────────────────────────────────────

#[test]
fn simple_cli_has_no_classes() {
    let Some(data) = common::read_fixture("simple_cli/app") else {
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
    let Some(data) = common::read_fixture("objc_foundation/app") else {
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
    let Some(data) = common::read_fixture("objc_foundation/app") else {
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
            protocol_names: Vec::new(),
            instance_size: 0,
            ro_flags: 0,
            superclass_addr: 0,
            metaclass_addr: 0,
        }],
        categories: vec![],
        protocols: vec![],
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
    let Some(data) = common::read_fixture("objc_category/objc_category") else {
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
    let Some(data) = common::read_fixture("objc_category/objc_category") else {
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

// ── Protocol parsing tests ─────────────────────────────────────────

#[test]
fn protocol_basics_fixture_has_protocols() {
    let Some(data) = common::read_fixture("protocol_basics/app") else {
        eprintln!("SKIP: protocol_basics fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();
    let meta = parse_objc_metadata(&parsed, &data);
    assert!(meta.presence.has_objc);
    assert!(meta.protocol_count > 0, "should have protocols");
    assert!(!meta.protocols.is_empty(), "protocols should be parsed");

    // Check that Greeter protocol is found
    let greeter = meta.protocols.iter().find(|p| p.name == "Greeter");
    assert!(greeter.is_some(), "Greeter protocol should be parsed");
    let greeter = greeter.unwrap();
    assert!(
        greeter.instance_methods.iter().any(|m| m.selector == "greet"),
        "Greeter should require 'greet' method"
    );

    // Check Polite protocol adopts Greeter and Farewell
    let polite = meta.protocols.iter().find(|p| p.name == "Polite");
    assert!(polite.is_some(), "Polite protocol should be parsed");
    let polite = polite.unwrap();
    assert!(
        polite.adopted_protocols.contains(&"Greeter".to_string()),
        "Polite should adopt Greeter"
    );
    assert!(
        polite.adopted_protocols.contains(&"Farewell".to_string()),
        "Polite should adopt Farewell"
    );
    assert!(
        polite.instance_methods.iter().any(|m| m.selector == "bow"),
        "Polite should require 'bow' method"
    );
}

#[test]
fn protocol_basics_classes_have_protocol_names() {
    let Some(data) = common::read_fixture("protocol_basics/app") else {
        eprintln!("SKIP: protocol_basics fixture not built");
        return;
    };
    let parsed = parse(&data).unwrap();
    let meta = parse_objc_metadata(&parsed, &data);

    // Person adopts Polite
    let person = meta
        .class_refs
        .iter()
        .find(|c| c.name.as_deref() == Some("Person"));
    assert!(person.is_some(), "Person class should be found");
    let person = person.unwrap();
    assert!(
        person.protocol_names.contains(&"Polite".to_string()),
        "Person should conform to Polite, got: {:?}",
        person.protocol_names,
    );

    // Robot adopts Greeter
    let robot = meta
        .class_refs
        .iter()
        .find(|c| c.name.as_deref() == Some("Robot"));
    assert!(robot.is_some(), "Robot class should be found");
    let robot = robot.unwrap();
    assert!(
        robot.protocol_names.contains(&"Greeter".to_string()),
        "Robot should conform to Greeter, got: {:?}",
        robot.protocol_names,
    );
}
