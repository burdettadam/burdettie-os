use super::*;
use super::make_metadata;
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

