//! Data types for ObjC metadata parsing results.

use serde::{Deserialize, Serialize};

/// Known Objective-C metadata section names.
pub(crate) const OBJC_SECTIONS: &[&str] = &[
    "__objc_classlist",
    "__objc_catlist",
    "__objc_protolist",
    "__objc_protorefs",
    "__objc_selrefs",
    "__objc_classrefs",
    "__objc_methnames",
    "__objc_methtype",
    "__objc_imageinfo",
];

/// Summary of ObjC metadata presence in a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcPresence {
    pub has_objc: bool,
    pub sections_found: Vec<String>,
}

/// A method found in a class's method list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcMethodInfo {
    pub selector: String,
    pub type_encoding: String,
    pub imp_vmaddr: u64,
}

/// An ivar (instance variable) found in a class's ivar list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcIvarInfo {
    pub name: String,
    pub type_encoding: String,
    pub offset: u32,
    pub size: u32,
    pub alignment: u32,
}

/// A property declared in a class's property list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcPropertyInfo {
    pub name: String,
    pub attributes: String,
}

/// A reference to an ObjC class found in __objc_classlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcClassRef {
    pub pointer_addr: u64,
    pub class_data_addr: u64,
    pub name: Option<String>,
    pub methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
    pub ivars: Vec<ObjcIvarInfo>,
    pub properties: Vec<ObjcPropertyInfo>,
    pub protocol_names: Vec<String>,
    pub instance_size: u32,
    pub ro_flags: u32,
    pub superclass_addr: u64,
    pub metaclass_addr: u64,
}

/// A parsed ObjC protocol from __objc_protolist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcProtocolRef {
    pub name: String,
    pub instance_methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
    pub optional_instance_methods: Vec<ObjcMethodInfo>,
    pub optional_class_methods: Vec<ObjcMethodInfo>,
    pub properties: Vec<ObjcPropertyInfo>,
    pub adopted_protocols: Vec<String>,
}

/// A reference to an ObjC category found in __objc_catlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcCategoryRef {
    pub name: Option<String>,
    pub class_vmaddr: u64,
    pub cat_struct_vmaddr: u64,
    pub instance_methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
}

/// Parsed ObjC metadata from a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcMetadata {
    pub presence: ObjcPresence,
    pub class_refs: Vec<ObjcClassRef>,
    pub categories: Vec<ObjcCategoryRef>,
    pub protocols: Vec<ObjcProtocolRef>,
    pub category_count: usize,
    pub protocol_count: usize,
    pub selectors: Vec<String>,
    pub selref_addrs: Vec<u64>,
    pub image_info: Option<ObjcImageInfo>,
}

/// Parsed __objc_imageinfo section (8 bytes: version + flags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjcImageInfo {
    pub version: u32,
    pub flags: u32,
}

impl ObjcImageInfo {
    pub fn supports_objc2(&self) -> bool {
        self.flags & 0x20 != 0
    }
    pub fn requires_gc(&self) -> bool {
        self.flags & 0x04 != 0
    }
    pub fn is_simulated(&self) -> bool {
        self.flags & 0x200 != 0
    }
    pub fn swift_version(&self) -> u8 {
        ((self.flags >> 8) & 0xFF) as u8
    }
}

/// C/C++ constructor entry from __mod_init_func or __init_offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorEntry {
    pub address: u64,
    pub source: ConstructorSource,
}

/// Where a constructor was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstructorSource {
    ModInitFunc,
    InitOffsets,
}

/// All initializer information for a Mach-O binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializerInfo {
    pub constructors: Vec<ConstructorEntry>,
    pub has_objc_load: bool,
    pub init_steps: Vec<String>,
}
