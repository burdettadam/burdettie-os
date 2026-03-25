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
    pub protocol_names: Vec<String>,
    pub instance_size: u32,
    pub is_root: bool,
    pub is_meta: bool,
    pub superclass_addr: Option<u64>,
    /// Selector → index in `methods` for O(1) instance method lookup.
    pub method_index: HashMap<String, usize>,
    /// Selector → index in `class_methods` for O(1) class method lookup.
    pub class_method_index: HashMap<String, usize>,
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

/// A registered ObjC protocol.
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub name: String,
    pub instance_methods: Vec<MethodEntry>,
    pub class_methods: Vec<MethodEntry>,
    pub optional_instance_methods: Vec<MethodEntry>,
    pub optional_class_methods: Vec<MethodEntry>,
    pub properties: Vec<PropertyEntry>,
    pub adopted_protocols: Vec<String>,
}

impl ClassInfo {
    /// Rebuild the selector → index maps from the current method vectors.
    pub fn rebuild_indices(&mut self) {
        self.method_index.clear();
        for (i, m) in self.methods.iter().enumerate() {
            self.method_index.insert(m.selector.clone(), i);
        }
        self.class_method_index.clear();
        for (i, m) in self.class_methods.iter().enumerate() {
            self.class_method_index.insert(m.selector.clone(), i);
        }
    }

    /// Look up an instance method by selector in O(1) via the index.
    pub fn find_method(&self, selector: &str) -> Option<&MethodEntry> {
        self.method_index
            .get(selector)
            .and_then(|&idx| self.methods.get(idx))
    }

    /// Look up a class method by selector in O(1) via the index.
    pub fn find_class_method(&self, selector: &str) -> Option<&MethodEntry> {
        self.class_method_index
            .get(selector)
            .and_then(|&idx| self.class_methods.get(idx))
    }
}

/// Global ObjC runtime state — populated after fixups, before constructors.
pub static OBJC_RUNTIME: Mutex<Option<ObjcRuntime>> = Mutex::new(None);

/// Minimal ObjC runtime: class table + selector table + protocol registry.
#[derive(Debug)]
pub struct ObjcRuntime {
    classes_by_name: HashMap<String, ClassInfo>,
    classes_by_addr: HashMap<u64, String>,
    metaclass_addrs: HashSet<u64>,
    protocols_by_name: HashMap<String, ProtocolInfo>,
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
            protocols_by_name: HashMap::new(),
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

    /// Check if an ISA pointer belongs to our runtime (class or metaclass).
    pub fn is_known_isa(&self, isa: u64) -> bool {
        self.classes_by_addr.contains_key(&isa) || self.metaclass_addrs.contains(&isa)
    }

    /// Check if an address is a registered metaclass.
    pub fn is_metaclass(&self, addr: u64) -> bool {
        self.metaclass_addrs.contains(&addr)
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

    /// Iterate over all registered classes mutably (values only).
    pub fn classes_by_name_iter_mut(&mut self) -> impl Iterator<Item = &mut ClassInfo> {
        self.classes_by_name.values_mut()
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

    /// Register a host ISA address as an alias for an existing class.
    ///
    /// This allows objects whose ISA comes from the host runtime (e.g.,
    /// `NSConstantArray` embedded in `__DATA` by the compiler) to dispatch
    /// through our shim implementations for the corresponding class.
    pub fn register_isa_alias(&mut self, host_addr: u64, class_name: &str) {
        if self.classes_by_name.contains_key(class_name) {
            self.classes_by_addr.insert(host_addr, class_name.to_string());
            log::debug!(
                "ObjcRuntime: registered ISA alias {host_addr:#x} → '{class_name}'"
            );
        } else {
            log::warn!(
                "ObjcRuntime: cannot alias {host_addr:#x} → '{class_name}': class not registered"
            );
        }
    }

    /// Rebuild method lookup indices for a class after methods are added.
    pub fn rebuild_method_index(&mut self, class_addr: u64) {
        if let Some(name) = self.classes_by_addr.get(&class_addr).cloned() {
            if let Some(info) = self.classes_by_name.get_mut(&name) {
                info.rebuild_indices();
            }
        }
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
        let idx = class.methods.len();
        let sel = entry.selector.clone();
        class.methods.push(entry);
        class.method_index.insert(sel, idx);
        true
    }

    // ── Protocol registry ──────────────────────────────────────────

    /// Register a protocol in the runtime.
    pub fn register_protocol(&mut self, info: ProtocolInfo) {
        log::debug!("ObjcRuntime: registered protocol '{}'", info.name);
        self.protocols_by_name.insert(info.name.clone(), info);
    }

    /// Look up a protocol by name.
    pub fn get_protocol(&self, name: &str) -> Option<&ProtocolInfo> {
        self.protocols_by_name.get(name)
    }

    /// Number of registered protocols.
    pub fn protocol_count(&self) -> usize {
        self.protocols_by_name.len()
    }

    /// Check if a class conforms to a protocol, walking the superclass chain.
    pub fn class_conforms_to_protocol(&self, class_name: &str, protocol_name: &str) -> bool {
        let mut current = Some(class_name.to_string());
        let mut depth = 0;
        while let Some(name) = current {
            if depth > 64 {
                break;
            }
            if let Some(class) = self.classes_by_name.get(&name) {
                if class.protocol_names.iter().any(|p| p == protocol_name) {
                    return true;
                }
                current = class
                    .superclass_addr
                    .and_then(|addr| self.classes_by_addr.get(&addr))
                    .cloned();
            } else {
                break;
            }
            depth += 1;
        }
        false
    }

    /// Check if a protocol conforms to another protocol (recursive).
    pub fn protocol_conforms_to_protocol(&self, proto: &str, other: &str) -> bool {
        if proto == other {
            return true;
        }
        if let Some(info) = self.protocols_by_name.get(proto) {
            for adopted in &info.adopted_protocols {
                if self.protocol_conforms_to_protocol(adopted, other) {
                    return true;
                }
            }
        }
        false
    }

    /// Iterate over all registered protocols.
    pub fn protocols_iter(&self) -> impl Iterator<Item = (&String, &ProtocolInfo)> {
        self.protocols_by_name.iter()
    }
}

#[cfg(test)]
mod tests;
