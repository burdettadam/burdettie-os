//! Class table, selector interning, and method dispatch.

mod dispatch;
mod registration;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// A registered ObjC class.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub mapped_addr: u64,
    pub methods: Vec<MethodEntry>,
    pub class_methods: Vec<MethodEntry>,
    pub ivars: Vec<IvarEntry>,
    pub properties: Vec<PropertyEntry>,
    pub instance_size: u32,
    pub is_root: bool,
    pub is_meta: bool,
    pub superclass_addr: Option<u64>,
}

/// A registered method.
#[derive(Debug, Clone)]
pub struct MethodEntry {
    pub selector: String,
    pub imp: u64,
    pub type_encoding: String,
}

/// A registered instance variable.
#[derive(Debug, Clone)]
pub struct IvarEntry {
    pub name: String,
    pub type_encoding: String,
    pub offset: u32,
    pub size: u32,
}

/// A registered property.
#[derive(Debug, Clone)]
pub struct PropertyEntry {
    pub name: String,
    pub attributes: String,
}

/// Global ObjC runtime state — populated after fixups, before constructors.
pub static OBJC_RUNTIME: Mutex<Option<ObjcRuntime>> = Mutex::new(None);

/// Minimal ObjC runtime: class table + selector table.
#[derive(Debug)]
pub struct ObjcRuntime {
    classes_by_name: HashMap<String, ClassInfo>,
    classes_by_addr: HashMap<u64, String>,
    metaclass_addrs: HashSet<u64>,
    selectors: HashMap<String, usize>,
    selector_storage: Vec<std::ffi::CString>,
}

impl Default for ObjcRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjcRuntime {
    /// Create an empty runtime.
    pub fn new() -> Self {
        Self {
            classes_by_name: HashMap::new(),
            classes_by_addr: HashMap::new(),
            metaclass_addrs: HashSet::new(),
            selectors: HashMap::new(),
            selector_storage: Vec::new(),
        }
    }

    /// Look up a class by name.
    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes_by_name.get(name)
    }

    /// Look up a class by its mapped address (isa pointer).
    pub fn get_class_by_addr(&self, addr: u64) -> Option<&ClassInfo> {
        self.classes_by_addr
            .get(&addr)
            .and_then(|name| self.classes_by_name.get(name))
    }

    /// Intern a selector string, returning a stable pointer to the C string.
    pub fn intern_selector(&mut self, name: &str) -> usize {
        if let Some(&ptr) = self.selectors.get(name) {
            return ptr;
        }
        let cstr =
            std::ffi::CString::new(name).unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
        self.selector_storage.push(cstr);
        let ptr = self.selector_storage.last().unwrap().as_ptr() as usize;
        self.selectors.insert(name.to_string(), ptr);
        ptr
    }

    /// Get a stable pointer for a selector name (intern if needed).
    pub fn selector_ptr(&self, name: &str) -> Option<usize> {
        self.selectors.get(name).copied()
    }

    /// Number of registered classes.
    pub fn class_count(&self) -> usize {
        self.classes_by_name.len()
    }

    /// Number of interned selectors.
    pub fn selector_count(&self) -> usize {
        self.selectors.len()
    }

    /// Iterate over all registered classes by name.
    pub fn classes_by_name_iter(&self) -> impl Iterator<Item = (&String, &ClassInfo)> {
        self.classes_by_name.iter()
    }

    /// Get mutable access to a class by name.
    pub fn get_class_mut(&mut self, name: &str) -> Option<&mut ClassInfo> {
        self.classes_by_name.get_mut(name)
    }

    /// Get mutable access to a class by mapped address.
    pub fn get_class_by_addr_mut(&mut self, addr: u64) -> Option<&mut ClassInfo> {
        let name = self.classes_by_addr.get(&addr)?.clone();
        self.classes_by_name.get_mut(&name)
    }

    /// Add a method to an existing class. Returns true if the method was
    /// added (selector not already present), false if it already existed.
    pub fn add_method_to_class(&mut self, class_addr: u64, entry: MethodEntry) -> bool {
        // Intern selector first to avoid double mutable borrow.
        self.intern_selector(&entry.selector);
        let Some(class) = self.get_class_by_addr_mut(class_addr) else {
            return false;
        };
        if class.methods.iter().any(|m| m.selector == entry.selector) {
            return false;
        }
        class.methods.push(entry);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use macho_objc::{ObjcClassRef, ObjcMethodInfo};

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
}
