//! Class table, selector interning, and method dispatch.

mod dispatch;
mod registration;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

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
    pub method_index: HashMap<Cow<'static, str>, usize>,
    /// Selector → index in `class_methods` for O(1) class method lookup.
    pub class_method_index: HashMap<Cow<'static, str>, usize>,
}

/// A registered method.
#[derive(Debug, Clone)]
pub struct MethodEntry {
    pub selector: Cow<'static, str>,
    pub imp: u64,
    pub type_encoding: Cow<'static, str>,
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
        self.method_index = HashMap::with_capacity(self.methods.len());
        for (i, m) in self.methods.iter().enumerate() {
            self.method_index.insert(m.selector.clone(), i);
        }
        self.class_method_index = HashMap::with_capacity(self.class_methods.len());
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
pub static OBJC_RUNTIME: RwLock<Option<ObjcRuntime>> = RwLock::new(None);

/// Minimal ObjC runtime: class table + selector table + protocol registry.
#[derive(Debug, Clone)]
pub struct ObjcRuntime {
    classes_by_name: HashMap<Arc<str>, Arc<ClassInfo>>,
    classes_by_addr: HashMap<u64, Arc<str>>,
    /// Classes whose superclass field must be reread from mapped image data.
    /// Synthetic class handles need not point at an `objc_class_t` record.
    mapped_class_addrs: HashMap<Arc<str>, u64>,
    metaclass_addrs: HashSet<u64>,
    protocols_by_name: HashMap<Arc<str>, Arc<ProtocolInfo>>,
    selectors: HashMap<Arc<str>, Arc<std::ffi::CString>>,
    /// Category +load IMPs recorded during category merging.
    /// Each entry is (target_class_name, imp) in registration order.
    category_load_imps: Vec<(String, u64)>,
    /// Tagged pointer class table: tag index (0-6) → class address.
    /// On arm64, tagged pointers have bit 63 set and encode the tag class
    /// in bits 60-62 (basic tags). Tag 0 is reserved.
    tagged_classes: [u64; 7],
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
            mapped_class_addrs: HashMap::new(),
            metaclass_addrs: HashSet::new(),
            protocols_by_name: HashMap::new(),
            selectors: HashMap::new(),
            category_load_imps: Vec::new(),
            tagged_classes: [0u64; 7],
        }
    }

    /// Reserve space for bulk class/selector registration to avoid repeated
    /// allocations during startup.
    pub fn reserve_registration_capacity(
        &mut self,
        classes: usize,
        protocols: usize,
        selectors: usize,
        category_load_imps: usize,
    ) {
        self.classes_by_name.reserve(classes);
        self.classes_by_addr.reserve(classes.saturating_mul(2));
        self.mapped_class_addrs.reserve(classes);
        self.metaclass_addrs.reserve(classes);
        self.protocols_by_name.reserve(protocols);
        self.selectors.reserve(selectors);
        self.category_load_imps.reserve(category_load_imps);
    }

    /// Overlay another runtime's registered metadata into this one.
    ///
    /// Overlay another runtime's registered metadata into this one.
    ///
    /// Existing entries are preserved unless `other` defines the same class,
    /// protocol, address alias, or tagged-pointer slot, in which case `other`
    /// wins. Selectors are NOT copied: the caller is responsible for ensuring
    /// that all necessary selectors were already interned before calling this
    /// (e.g. via `fixup_selrefs` which runs before Foundation overlay).
    pub fn overlay_from(&mut self, other: &ObjcRuntime) {
        // Pre-reserve to prevent HashMap rehashing during bulk insert.
        self.classes_by_name.reserve(other.classes_by_name.len());
        self.classes_by_addr.reserve(other.classes_by_addr.len());
        self.protocols_by_name
            .reserve(other.protocols_by_name.len());
        self.category_load_imps
            .reserve(other.category_load_imps.len());

        // Extend directly from references — avoids allocating a temporary
        // cloned HashMap for each field (saves ~10 KB of allocations).
        self.protocols_by_name.extend(
            other
                .protocols_by_name
                .iter()
                .map(|(k, v)| (Arc::clone(k), Arc::clone(v))),
        );
        self.classes_by_addr.extend(
            other
                .classes_by_addr
                .iter()
                .map(|(&k, v)| (k, Arc::clone(v))),
        );
        self.metaclass_addrs
            .extend(other.metaclass_addrs.iter().copied());
        // The winning class definition also owns its storage provenance. In
        // particular, a Foundation overlay can replace an image class with a
        // synthetic class whose address must never be dereferenced for fixups.
        for name in other.classes_by_name.keys() {
            self.mapped_class_addrs.remove(name);
        }
        self.mapped_class_addrs.extend(
            other
                .mapped_class_addrs
                .iter()
                .map(|(name, &addr)| (Arc::clone(name), addr)),
        );
        self.classes_by_name.extend(
            other
                .classes_by_name
                .iter()
                .map(|(k, v)| (Arc::clone(k), Arc::clone(v))),
        );
        self.category_load_imps
            .extend(other.category_load_imps.iter().cloned());

        for (slot, class_addr) in self
            .tagged_classes
            .iter_mut()
            .zip(other.tagged_classes.iter().copied())
        {
            if class_addr != 0 {
                *slot = class_addr;
            }
        }
    }

    /// Look up a class by name.
    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes_by_name.get(name).map(|a| a.as_ref())
    }

    /// Look up a class by its mapped address (isa pointer).
    pub fn get_class_by_addr(&self, addr: u64) -> Option<&ClassInfo> {
        self.classes_by_addr
            .get(&addr)
            .and_then(|name| self.classes_by_name.get(name))
            .map(|a| a.as_ref())
    }

    /// Look up the class name for an ISA (class or metaclass address).
    pub fn class_name_for_isa(&self, isa: u64) -> Option<&str> {
        self.classes_by_addr.get(&isa).map(|n| n.as_ref())
    }

    /// Check if an ISA pointer belongs to our runtime (class or metaclass).
    pub fn is_known_isa(&self, isa: u64) -> bool {
        self.classes_by_addr.contains_key(&isa) || self.metaclass_addrs.contains(&isa)
    }

    /// Check if an address is a registered metaclass.
    pub fn is_metaclass(&self, addr: u64) -> bool {
        self.metaclass_addrs.contains(&addr)
    }

    /// Resolve the dispatch ISA for a receiver that may itself be a class object.
    ///
    /// When the receiver is a class object, Objective-C dispatch should start at
    /// its metaclass so class methods like `alloc` are found correctly.
    pub fn dispatch_isa_for_receiver(&self, receiver: u64) -> Option<u64> {
        if self.is_metaclass(receiver) {
            return Some(receiver);
        }

        let class_name = self.classes_by_addr.get(&receiver)?;
        self.metaclass_addrs.iter().copied().find(|meta_addr| {
            self.classes_by_addr
                .get(meta_addr)
                .is_some_and(|meta_name| meta_name == class_name)
        })
    }

    /// Intern a selector string, returning a stable pointer to the C string.
    pub fn intern_selector(&mut self, name: &str) -> usize {
        if let Some(selector) = self.selectors.get(name) {
            return selector.as_ptr() as usize;
        }
        let selector = Arc::new(
            std::ffi::CString::new(name).unwrap_or_else(|_| std::ffi::CString::new("").unwrap()),
        );
        let ptr = selector.as_ptr() as usize;
        self.selectors.insert(Arc::from(name), selector);
        ptr
    }

    /// Get a stable pointer for a selector name (intern if needed).
    pub fn selector_ptr(&self, name: &str) -> Option<usize> {
        self.selectors
            .get(name)
            .map(|selector| selector.as_ptr() as usize)
    }

    /// Look up a selector name by its stable pointer.
    pub fn selector_name(&self, sel: u64) -> Option<&str> {
        self.selectors.iter().find_map(|(name, selector)| {
            ((selector.as_ptr() as u64) == sel).then_some(name.as_ref())
        })
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
    pub fn classes_by_name_iter(&self) -> impl Iterator<Item = (&str, &ClassInfo)> {
        self.classes_by_name
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    /// Iterate over all registered classes mutably (values only).
    pub fn classes_by_name_iter_mut(&mut self) -> impl Iterator<Item = &mut ClassInfo> {
        self.classes_by_name.values_mut().map(|a| Arc::make_mut(a))
    }

    /// Get mutable access to a class by name.
    pub fn get_class_mut(&mut self, name: &str) -> Option<&mut ClassInfo> {
        self.classes_by_name.get_mut(name).map(|a| Arc::make_mut(a))
    }

    /// Get mutable access to a class by mapped address.
    pub fn get_class_by_addr_mut(&mut self, addr: u64) -> Option<&mut ClassInfo> {
        let name = self.classes_by_addr.get(&addr)?.clone();
        self.classes_by_name
            .get_mut(&name)
            .map(|a| Arc::make_mut(a))
    }

    /// Register a host ISA address as an alias for an existing class.
    ///
    /// This allows objects whose ISA comes from the host runtime (e.g.,
    /// `NSConstantArray` embedded in `__DATA` by the compiler) to dispatch
    /// through our shim implementations for the corresponding class.
    pub fn register_isa_alias(&mut self, host_addr: u64, class_name: &str) {
        if self.classes_by_name.contains_key(class_name) {
            self.classes_by_addr
                .insert(host_addr, Arc::from(class_name));
            log::debug!("ObjcRuntime: registered ISA alias {host_addr:#x} → '{class_name}'");
        } else {
            log::warn!(
                "ObjcRuntime: cannot alias {host_addr:#x} → '{class_name}': class not registered"
            );
        }
    }

    /// Register a host metaclass address as a known metaclass so that
    /// `msg_lookup` dispatches class methods correctly.
    pub fn register_metaclass_alias(&mut self, metaclass_addr: u64, _class_name: &str) {
        self.metaclass_addrs.insert(metaclass_addr);
    }

    /// Rebuild method lookup indices for a class after methods are added.
    pub fn rebuild_method_index(&mut self, class_addr: u64) {
        if let Some(name) = self.classes_by_addr.get(&class_addr).cloned() {
            if let Some(arc) = self.classes_by_name.get_mut(&name) {
                Arc::make_mut(arc).rebuild_indices();
            }
        }
    }

    /// Add a method to an existing class. Returns true if the method was
    /// added (selector not already present), false if it already existed.
    pub fn add_method_to_class(&mut self, class_addr: u64, entry: MethodEntry) -> bool {
        // Intern selector first to avoid double mutable borrow.
        self.intern_selector(&entry.selector);
        let is_metaclass = self.is_metaclass(class_addr);
        let Some(class) = self.get_class_by_addr_mut(class_addr) else {
            return false;
        };
        let methods = if is_metaclass {
            &mut class.class_methods
        } else {
            &mut class.methods
        };
        let method_index = if is_metaclass {
            &mut class.class_method_index
        } else {
            &mut class.method_index
        };
        if methods.iter().any(|m| m.selector == entry.selector) {
            return false;
        }
        let idx = methods.len();
        let sel = entry.selector.clone();
        methods.push(entry);
        method_index.insert(sel, idx);
        true
    }

    // ── Protocol registry ──────────────────────────────────────────

    /// Register a protocol in the runtime.
    pub fn register_protocol(&mut self, info: ProtocolInfo) {
        log::debug!("ObjcRuntime: registered protocol '{}'", info.name);
        let name_arc: Arc<str> = Arc::from(info.name.as_str());
        self.protocols_by_name.insert(name_arc, Arc::new(info));
    }

    /// Look up a protocol by name.
    pub fn get_protocol(&self, name: &str) -> Option<&ProtocolInfo> {
        self.protocols_by_name.get(name).map(|a| a.as_ref())
    }

    /// Number of registered protocols.
    pub fn protocol_count(&self) -> usize {
        self.protocols_by_name.len()
    }

    /// Check if a class conforms to a protocol, walking the superclass chain.
    pub fn class_conforms_to_protocol(&self, class_name: &str, protocol_name: &str) -> bool {
        let mut current: Option<Arc<str>> = Some(Arc::from(class_name));
        let mut depth = 0;
        while let Some(name) = current {
            if depth > 64 {
                break;
            }
            if let Some(class) = self.classes_by_name.get(&*name) {
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
    pub fn protocols_iter(&self) -> impl Iterator<Item = (&str, &ProtocolInfo)> {
        self.protocols_by_name
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    /// Remove a dynamically-created class by its address.
    /// Returns true if the class was found and removed.
    pub fn remove_class_by_addr(&mut self, addr: u64) -> bool {
        let Some(name) = self.classes_by_addr.remove(&addr) else {
            return false;
        };
        self.classes_by_name.remove(&name);
        self.mapped_class_addrs.remove(&name);
        self.metaclass_addrs.remove(&addr);
        log::debug!("ObjcRuntime: removed class '{}' at {:#x}", name, addr);
        true
    }

    // ── Tagged pointer support ─────────────────────────────────────

    /// Register a class address for a tagged pointer tag index (0-6).
    ///
    /// On arm64, tagged pointers (bit 63 set) encode the class in bits 60-62.
    /// This table maps those tag indices to our synthetic class addresses
    /// so dispatch works without the host runtime.
    pub fn register_tagged_class(&mut self, tag: u8, class_addr: u64) {
        if (tag as usize) < self.tagged_classes.len() {
            self.tagged_classes[tag as usize] = class_addr;
            log::debug!(
                "ObjcRuntime: registered tagged class index {} → {:#x}",
                tag,
                class_addr
            );
        }
    }

    /// Look up the class address for a tagged pointer.
    /// Returns None if the tag is out of range or unregistered.
    pub fn tagged_class_for_pointer(&self, obj: u64) -> Option<u64> {
        // ARM64 tagged pointer: bits 60-62 = tag class index
        let tag = ((obj >> 60) & 0x7) as usize;
        if tag < self.tagged_classes.len() && self.tagged_classes[tag] != 0 {
            Some(self.tagged_classes[tag])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
