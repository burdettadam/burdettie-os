// Test helpers and module declarations.

pub use super::*;

pub use macho_objc::{ObjcClassRef, ObjcMetadata, ObjcMethodInfo};

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

mod class_tests;
mod dynamic_and_load_tests;
mod protocol_tests;
mod superclass_repair_tests;
