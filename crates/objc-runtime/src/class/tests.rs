use super::*;

use macho_objc::{ObjcClassRef, ObjcMetadata, ObjcMethodInfo};

fn make_metadata(name: &str, methods: Vec<(&str, u64)>) -> macho_objc::ObjcMetadata {
    let class_ref = ObjcClassRef {
        pointer_addr: 0x1000,
        class_data_addr: 0x2000,
        name: Some(name.to_string()),
        methods: methods
            .into_iter()
            .map(|(sel, imp)| ObjcMethodInfo {
                selector: sel.to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: imp,
            })
            .collect(),
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        ro_flags: 0x02,
        superclass_addr: 0,
        metaclass_addr: 0,
    };
    macho_objc::ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_classlist".to_string()],
        },
        class_refs: vec![class_ref],
        categories: vec![],
        protocols: vec![],
        category_count: 0,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    }
}

#[test]
fn register_and_lookup_class() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("MyClass", vec![("doStuff", 0x1234)]);
    rt.register_image(&meta, 0);

    assert_eq!(rt.class_count(), 1);
    let cls = rt.get_class("MyClass").unwrap();
    assert_eq!(cls.name, "MyClass");
    assert_eq!(cls.methods.len(), 1);
    assert!(cls.is_root);
}

#[test]
fn lookup_class_by_addr() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Answer", vec![("value", 0x5678)]);
    rt.register_image(&meta, 0x100);

    let cls = rt.get_class_by_addr(0x2100).unwrap();
    assert_eq!(cls.name, "Answer");
}

#[test]
fn msg_lookup_finds_method() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Answer", vec![("value", 0x5678)]);
    rt.register_image(&meta, 0x100);

    let imp = rt.msg_lookup(0x2100, "value").unwrap();
    assert_eq!(imp, 0x5778);
}

#[test]
fn msg_lookup_unknown_class_returns_none() {
    let rt = ObjcRuntime::new();
    assert!(rt.msg_lookup(0xDEAD, "value").is_none());
}

#[test]
fn msg_lookup_unknown_selector_returns_none() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Answer", vec![("value", 0x5678)]);
    rt.register_image(&meta, 0);

    assert!(rt.msg_lookup(0x2000, "nonexistent").is_none());
}

#[test]
fn selector_interning() {
    let mut rt = ObjcRuntime::new();
    let p1 = rt.intern_selector("value");
    let p2 = rt.intern_selector("value");
    assert_eq!(p1, p2);
    assert_eq!(rt.selector_count(), 1);

    rt.intern_selector("init");
    assert_eq!(rt.selector_count(), 2);
}

#[test]
fn root_class_zero_size_gets_eight() {
    let mut rt = ObjcRuntime::new();
    let class_ref = ObjcClassRef {
        pointer_addr: 0x1000,
        class_data_addr: 0x2000,
        name: Some("Root".to_string()),
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 0,
        ro_flags: 0x02,
        superclass_addr: 0,
        metaclass_addr: 0,
    };
    let meta = macho_objc::ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec![],
        },
        class_refs: vec![class_ref],
        categories: vec![],
        protocols: vec![],
        category_count: 0,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&meta, 0);
    assert_eq!(rt.get_class("Root").unwrap().instance_size, 8);
}

#[test]
fn category_methods_merged_into_class() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Greeter", vec![("greet", 0x1000)]);
    rt.register_image(&meta, 0);

    let cat_meta = macho_objc::ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_catlist".to_string()],
        },
        class_refs: vec![],
        categories: vec![macho_objc::ObjcCategoryRef {
            name: Some("Extras".to_string()),
            class_vmaddr: 0x2000,
            cat_struct_vmaddr: 0x3000,
            instance_methods: vec![ObjcMethodInfo {
                selector: "farewell".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: 0x2000,
            }],
            class_methods: vec![],
        }],
        protocols: vec![],
        category_count: 1,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&cat_meta, 0);

    let cls = rt.get_class("Greeter").unwrap();
    assert_eq!(cls.methods.len(), 2);
    assert!(cls.methods.iter().any(|m| m.selector == "greet"));
    assert!(cls.methods.iter().any(|m| m.selector == "farewell"));
}

#[test]
fn category_class_methods_merged_into_class() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Counter", vec![("increment", 0x1000)]);
    rt.register_image(&meta, 0);

    let cat_meta = macho_objc::ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_catlist".to_string()],
        },
        class_refs: vec![],
        categories: vec![macho_objc::ObjcCategoryRef {
            name: Some("Singleton".to_string()),
            class_vmaddr: 0x2000,
            cat_struct_vmaddr: 0x3000,
            instance_methods: vec![],
            class_methods: vec![ObjcMethodInfo {
                selector: "sharedInstance".to_string(),
                type_encoding: "@@:".to_string(),
                imp_vmaddr: 0x4000,
            }],
        }],
        protocols: vec![],
        category_count: 1,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&cat_meta, 0);

    let cls = rt.get_class("Counter").unwrap();
    assert_eq!(cls.class_methods.len(), 1);
    assert!(cls
        .class_methods
        .iter()
        .any(|m| m.selector == "sharedInstance"));
    assert_eq!(cls.class_methods[0].imp, 0x4000);
}

#[test]
fn category_class_methods_collected_as_load() {
    let mut rt = ObjcRuntime::new();
    let meta = make_metadata("Widget", vec![("draw", 0x1000)]);
    rt.register_image(&meta, 0);

    let cat_meta = macho_objc::ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_catlist".to_string()],
        },
        class_refs: vec![],
        categories: vec![macho_objc::ObjcCategoryRef {
            name: Some("Startup".to_string()),
            class_vmaddr: 0x2000,
            cat_struct_vmaddr: 0x3000,
            instance_methods: vec![],
            class_methods: vec![ObjcMethodInfo {
                selector: "load".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: 0x5000,
            }],
        }],
        protocols: vec![],
        category_count: 1,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&cat_meta, 0);

    let loads = rt.collect_load_methods();
    assert!(loads.iter().any(|(name, _)| name == "Widget"));
}

// ── Protocol tests ─────────────────────────────────────────────────

#[test]
fn register_and_lookup_protocol() {
    let mut rt = ObjcRuntime::new();
    rt.register_protocol(ProtocolInfo {
        name: "Greeter".to_string(),
        instance_methods: vec![MethodEntry {
            selector: "greet".to_string(),
            imp: 0x1000,
            type_encoding: "v@:".to_string(),
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

// ── Dynamic class tests ────────────────────────────────────────────

#[test]
fn register_synthetic_class_and_lookup() {
    let mut rt = ObjcRuntime::new();
    let info = ClassInfo {
        name: "DynClass".to_string(),
        mapped_addr: 0xD000_0000_0001_0000,
        methods: vec![MethodEntry {
            selector: "hello".to_string(),
            imp: 0xBEEF,
            type_encoding: "v@:".to_string(),
        }],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: None,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);

    assert!(rt.get_class("DynClass").is_some());
    assert!(rt.get_class_by_addr(0xD000_0000_0001_0000).is_some());
    assert_eq!(rt.msg_lookup(0xD000_0000_0001_0000, "hello"), Some(0xBEEF));
}

#[test]
fn remove_class_by_addr_works() {
    let mut rt = ObjcRuntime::new();
    let info = ClassInfo {
        name: "Removable".to_string(),
        mapped_addr: 0xD000_0000_0002_0000,
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 8,
        is_root: true,
        is_meta: false,
        superclass_addr: None,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    assert!(rt.get_class("Removable").is_some());

    assert!(rt.remove_class_by_addr(0xD000_0000_0002_0000));
    assert!(rt.get_class("Removable").is_none());
    assert!(rt.get_class_by_addr(0xD000_0000_0002_0000).is_none());
}

#[test]
fn remove_nonexistent_class_returns_false() {
    let mut rt = ObjcRuntime::new();
    assert!(!rt.remove_class_by_addr(0xFFFF));
}

// ── +load ordering tests ───────────────────────────────────────────

/// Helper: build metadata for a class with a +load class method and a
/// superclass address, allowing us to construct a hierarchy.
fn make_loadable_class(
    name: &str,
    class_addr: u64,
    load_imp: u64,
    superclass_addr: u64,
    metaclass_addr: u64,
) -> ObjcMetadata {
    ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_classlist".to_string()],
        },
        class_refs: vec![ObjcClassRef {
            pointer_addr: 0x1000,
            class_data_addr: class_addr,
            name: Some(name.to_string()),
            methods: vec![],
            class_methods: vec![ObjcMethodInfo {
                selector: "load".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: load_imp,
            }],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
            ro_flags: if superclass_addr == 0 { 0x02 } else { 0x00 },
            superclass_addr,
            metaclass_addr,
        }],
        categories: vec![],
        protocols: vec![],
        category_count: 0,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    }
}

#[test]
fn load_methods_superclass_before_subclass() {
    let mut rt = ObjcRuntime::new();
    // Register Child first, then Parent — collect_load_methods must still
    // return Parent before Child.
    let child = make_loadable_class("Child", 0x4000, 0xB000, 0x2000, 0x4100);
    let parent = make_loadable_class("Parent", 0x2000, 0xA000, 0, 0x2100);
    rt.register_image(&child, 0);
    rt.register_image(&parent, 0);

    let loads = rt.collect_load_methods();
    let names: Vec<&str> = loads.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["Parent", "Child"]);
}

#[test]
fn load_methods_class_before_category() {
    let mut rt = ObjcRuntime::new();
    // Class Widget has its own +load, then a category also adds +load.
    let class_meta = make_loadable_class("Widget", 0x2000, 0xA000, 0, 0x2100);
    rt.register_image(&class_meta, 0);

    let cat_meta = ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_catlist".to_string()],
        },
        class_refs: vec![],
        categories: vec![macho_objc::ObjcCategoryRef {
            name: Some("Startup".to_string()),
            class_vmaddr: 0x2000,
            cat_struct_vmaddr: 0x3000,
            instance_methods: vec![],
            class_methods: vec![ObjcMethodInfo {
                selector: "load".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: 0xC000,
            }],
        }],
        protocols: vec![],
        category_count: 1,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&cat_meta, 0);

    let loads = rt.collect_load_methods();
    assert_eq!(loads.len(), 2);
    // Class +load first, then category +load.
    assert_eq!(loads[0], ("Widget".to_string(), 0xA000));
    assert_eq!(loads[1], ("Widget".to_string(), 0xC000));
}

#[test]
fn load_methods_hierarchy_then_categories() {
    let mut rt = ObjcRuntime::new();
    // Grandparent → Parent → Child hierarchy, each with +load.
    // Plus a category +load on Parent.
    let gp = make_loadable_class("GrandParent", 0x2000, 0xA000, 0, 0x2100);
    let parent = make_loadable_class("Parent", 0x4000, 0xB000, 0x2000, 0x4100);
    let child = make_loadable_class("Child", 0x6000, 0xC000, 0x4000, 0x6100);
    // Register in reverse order to ensure topo-sort, not insertion order.
    rt.register_image(&child, 0);
    rt.register_image(&parent, 0);
    rt.register_image(&gp, 0);

    // Category +load on Parent.
    let cat_meta = ObjcMetadata {
        presence: macho_objc::ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_catlist".to_string()],
        },
        class_refs: vec![],
        categories: vec![macho_objc::ObjcCategoryRef {
            name: Some("Extra".to_string()),
            class_vmaddr: 0x4000,
            cat_struct_vmaddr: 0x5000,
            instance_methods: vec![],
            class_methods: vec![ObjcMethodInfo {
                selector: "load".to_string(),
                type_encoding: "v@:".to_string(),
                imp_vmaddr: 0xD000,
            }],
        }],
        protocols: vec![],
        category_count: 1,
        protocol_count: 0,
        selectors: vec![],
        selref_addrs: vec![],
        image_info: None,
    };
    rt.register_image(&cat_meta, 0);

    let loads = rt.collect_load_methods();
    let names_and_imps: Vec<(&str, u64)> = loads.iter().map(|(n, i)| (n.as_str(), *i)).collect();

    // Class +loads in hierarchy order, then category +load last.
    assert_eq!(
        names_and_imps,
        vec![
            ("GrandParent", 0xA000),
            ("Parent", 0xB000),
            ("Child", 0xC000),
            ("Parent", 0xD000), // category +load
        ]
    );
}
