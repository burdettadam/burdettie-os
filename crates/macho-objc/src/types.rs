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

#[cfg(test)]
mod tests {
    use super::*;

    // ── supports_objc2 (bit 0x20) ──────────────────────────────────

    #[test]
    fn supports_objc2_set() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x20,
        };
        assert!(info.supports_objc2());
    }

    #[test]
    fn supports_objc2_unset() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x00,
        };
        assert!(!info.supports_objc2());
    }

    #[test]
    fn supports_objc2_with_other_bits() {
        // Other bits set but not 0x20
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x04,
        };
        assert!(!info.supports_objc2());
    }

    // ── requires_gc (bit 0x04) ─────────────────────────────────────

    #[test]
    fn requires_gc_set() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x04,
        };
        assert!(info.requires_gc());
    }

    #[test]
    fn requires_gc_unset() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x00,
        };
        assert!(!info.requires_gc());
    }

    // ── is_simulated (bit 0x200) ───────────────────────────────────

    #[test]
    fn is_simulated_set() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x200,
        };
        assert!(info.is_simulated());
    }

    #[test]
    fn is_simulated_unset() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x00,
        };
        assert!(!info.is_simulated());
    }

    // ── swift_version (bits [15:8]) ────────────────────────────────

    #[test]
    fn swift_version_five() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 5 << 8,
        };
        assert_eq!(info.swift_version(), 5);
    }

    #[test]
    fn swift_version_zero() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x00,
        };
        assert_eq!(info.swift_version(), 0);
    }

    #[test]
    fn swift_version_seven() {
        let info = ObjcImageInfo {
            version: 0,
            flags: 7 << 8,
        };
        assert_eq!(info.swift_version(), 7);
    }

    // ── combined flags ─────────────────────────────────────────────

    #[test]
    fn combined_flags_objc2_swift5() {
        // 0x20 (objc2) | (5 << 8) (swift 5) = 0x520
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x520,
        };
        assert!(info.supports_objc2());
        assert!(!info.requires_gc());
        assert!(!info.is_simulated());
        assert_eq!(info.swift_version(), 5);
    }

    #[test]
    fn all_flags_set() {
        // is_simulated (bit 9) overlaps with swift_version byte [15:8]
        // flags 0x0724 → swift_version = (0x0724 >> 8) & 0xFF = 7
        let info = ObjcImageInfo {
            version: 0,
            flags: 0x0724,
        };
        assert!(info.supports_objc2());
        assert!(info.requires_gc());
        assert!(info.is_simulated());
        assert_eq!(info.swift_version(), 7);
    }

    #[test]
    fn objc_presence_no_sections() {
        let presence = ObjcPresence {
            has_objc: false,
            sections_found: vec![],
        };
        assert!(!presence.has_objc);
        assert!(presence.sections_found.is_empty());
    }

    #[test]
    fn objc_presence_has_sections_and_objc() {
        let presence = ObjcPresence {
            has_objc: true,
            sections_found: vec!["__objc_classlist".to_string(), "__objc_selrefs".to_string()],
        };
        assert!(presence.has_objc);
        assert_eq!(presence.sections_found.len(), 2);
        assert!(presence
            .sections_found
            .contains(&"__objc_classlist".to_string()));
    }
}
