use super::*;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};

// ── Dynamic class tests ────────────────────────────────────────────

#[test]
fn register_synthetic_class_and_lookup() {
    let mut rt = ObjcRuntime::new();
    let info = ClassInfo {
        name: "DynClass".to_string(),
        mapped_addr: 0xD000_0000_0001_0000,
        methods: vec![MethodEntry {
            selector: "hello".into(),
            imp: 0xBEEF,
            type_encoding: "v@:".into(),
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

#[test]
fn clone_shares_selector_storage_safely() {
    let cloned = {
        let mut rt = ObjcRuntime::new();
        rt.register_synthetic_class(ClassInfo {
            name: "CloneCheck".to_string(),
            mapped_addr: 0xD000_0000_0003_0000,
            methods: vec![MethodEntry {
                selector: "hello".into(),
                imp: 0xCAFE,
                type_encoding: "v@:".into(),
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
        });

        let original_selector = rt
            .selector_ptr("hello")
            .expect("original runtime should intern selector");
        let cloned = rt.clone();
        let cloned_selector = cloned
            .selector_ptr("hello")
            .expect("cloned runtime should keep selector alive");

        assert_eq!(
            original_selector, cloned_selector,
            "cloned runtime should reuse shared selector storage"
        );

        cloned
    };

    let cloned_selector = cloned
        .selector_ptr("hello")
        .expect("cloned runtime should retain selector pointer after original drop");
    let selector_name = unsafe { CStr::from_ptr(cloned_selector as *const c_char) }
        .to_str()
        .expect("shared selector pointer should stay valid");

    assert_eq!(selector_name, "hello");
    assert_eq!(
        cloned.msg_lookup(0xD000_0000_0003_0000, "hello"),
        Some(0xCAFE)
    );
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
