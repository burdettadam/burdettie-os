use super::make_metadata;
use super::*;

// ── Protocol tests ─────────────────────────────────────────────────

#[test]
fn register_and_lookup_protocol() {
    let mut rt = ObjcRuntime::new();
    rt.register_protocol(ProtocolInfo {
        name: "Greeter".to_string(),
        instance_methods: vec![MethodEntry {
            selector: "greet".into(),
            imp: 0x1000,
            type_encoding: "v@:".into(),
        }],
        class_methods: vec![],
        optional_instance_methods: vec![],
        optional_class_methods: vec![],
        properties: vec![],
        adopted_protocols: vec![],
    });

    assert_eq!(rt.protocol_count(), 1);
    let proto = rt.get_protocol("Greeter").unwrap();
    assert_eq!(proto.name, "Greeter");
    assert_eq!(proto.instance_methods.len(), 1);
    assert!(rt.get_protocol("Unknown").is_none());
}

#[test]
fn class_conforms_to_protocol_direct() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Hero", vec![("save", 0x1000)]);
    rt.register_image(&meta, 0);

    // Add protocol_names to the class
    if let Some(class) = rt.get_class_mut("Hero") {
        class.protocol_names.push("Saveable".to_string());
    }

    assert!(rt.class_conforms_to_protocol("Hero", "Saveable"));
    assert!(!rt.class_conforms_to_protocol("Hero", "Deletable"));
}

#[test]
fn protocol_conforms_to_protocol_recursive() {
    let mut rt = ObjcRuntime::new();

    // Greeter protocol
    rt.register_protocol(ProtocolInfo {
        name: "Greeter".to_string(),
        instance_methods: vec![],
        class_methods: vec![],
        optional_instance_methods: vec![],
        optional_class_methods: vec![],
        properties: vec![],
        adopted_protocols: vec![],
    });

    // Polite adopts Greeter
    rt.register_protocol(ProtocolInfo {
        name: "Polite".to_string(),
        instance_methods: vec![],
        class_methods: vec![],
        optional_instance_methods: vec![],
        optional_class_methods: vec![],
        properties: vec![],
        adopted_protocols: vec!["Greeter".to_string()],
    });

    assert!(rt.protocol_conforms_to_protocol("Polite", "Greeter"));
    assert!(rt.protocol_conforms_to_protocol("Polite", "Polite"));
    assert!(!rt.protocol_conforms_to_protocol("Greeter", "Polite"));
}

#[test]
fn register_image_with_protocols() {
    let mut rt = ObjcRuntime::new();

    let metadata = ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_classlist".to_string()],
        },
        class_refs: vec![ObjcClassRef {
            pointer_addr: 0x1000,
            class_data_addr: 0x2000,
            name: Some("MyClass".to_string()),
            methods: vec![],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec!["Codable".to_string()],
            instance_size: 16,
            ro_flags: 0x02,
            superclass_addr: 0,
            metaclass_addr: 0,
        }],
        categories: vec![],
        protocols: vec![macho_objc::ObjcProtocolRef {
            name: "Codable".to_string(),
            instance_methods: vec![ObjcMethodInfo {
                selector: "encode".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: 0x3000,
            }],
            class_methods: vec![],
            optional_instance_methods: vec![],
            optional_class_methods: vec![],
            properties: vec![],
            adopted_protocols: vec![],
        }],
        category_count: 0,
        protocol_count: 1,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };

    rt.register_image(&metadata, 0);

    assert_eq!(rt.protocol_count(), 1);
    let proto = rt.get_protocol("Codable").unwrap();
    assert_eq!(proto.instance_methods.len(), 1);
    assert_eq!(proto.instance_methods[0].selector, "encode");

    let cls = rt.get_class("MyClass").unwrap();
    assert_eq!(cls.protocol_names, vec!["Codable".to_string()]);
    assert!(rt.class_conforms_to_protocol("MyClass", "Codable"));
}
