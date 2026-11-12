//! Method dispatch, selref fixup, and +load collection.

use std::collections::HashSet;

use macho_objc::ObjcMetadata;

use super::ObjcRuntime;

impl ObjcRuntime {
    /// Look up a method on a class by selector name.
    /// Returns the IMP address if found.
    pub fn lookup_method(&self, class_name: &str, selector: &str) -> Option<u64> {
        let class = self.classes_by_name.get(class_name)?;
        class.find_method(selector).map(|m| m.imp)
    }

    /// Look up a method by isa address and selector name.
    /// Walks the superclass chain if the method is not found on the direct class.
    /// When ISA is a metaclass address, searches class_methods instead of
    /// instance methods (this is the path for `[MyClass classMethod]`).
    pub fn msg_lookup(&self, isa: u64, selector: &str) -> Option<u64> {
        self.lookup_method_entry(isa, selector).map(|e| e.imp)
    }

    /// Look up a `MethodEntry` by isa address and selector name.
    /// Walks the superclass chain. Returns a reference to the entry if found.
    pub fn lookup_method_entry(&self, isa: u64, selector: &str) -> Option<&super::MethodEntry> {
        let is_meta = self.metaclass_addrs.contains(&isa);
        let mut current_addr = Some(isa);
        let mut depth = 0;
        while let Some(addr) = current_addr {
            if depth > 64 {
                break;
            }
            if let Some(class) = self.get_class_by_addr(addr) {
                let entry = if is_meta {
                    class.find_class_method(selector)
                } else {
                    class.find_method(selector)
                };
                if let Some(entry) = entry {
                    return Some(entry);
                }
                current_addr = class.superclass_addr;
            } else {
                break;
            }
            depth += 1;
        }
        None
    }

    /// Collect all +load method IMPs in correct Apple-compatible order:
    /// 1. Class +load methods sorted by hierarchy (superclasses before subclasses)
    /// 2. Category +load methods in registration order
    pub fn collect_load_methods(&self) -> Vec<(String, u64)> {
        // Build set of IMPs that came from categories.
        let cat_imp_set: HashSet<u64> = self
            .category_load_imps
            .iter()
            .map(|(_, imp)| *imp)
            .collect();

        // Collect class-defined +load (excluding category-contributed IMPs).
        let mut class_loads: Vec<(String, u64)> = Vec::new();
        for (name, info) in &self.classes_by_name {
            for m in &info.class_methods {
                if m.selector == "load" && !cat_imp_set.contains(&m.imp) {
                    class_loads.push((name.to_string(), m.imp));
                }
            }
        }

        // Topological sort: superclasses before subclasses.
        let load_names: HashSet<&str> = class_loads.iter().map(|(n, _)| n.as_str()).collect();
        let mut sorted: Vec<(String, u64)> = Vec::with_capacity(class_loads.len());
        let mut visited: HashSet<String> = HashSet::new();
        // Build a quick name→imp map for reconstruction after DFS.
        let imp_map: std::collections::HashMap<&str, u64> = class_loads
            .iter()
            .map(|(n, imp)| (n.as_str(), *imp))
            .collect();
        for (name, _) in &class_loads {
            self.visit_load_topo(name, &load_names, &imp_map, &mut visited, &mut sorted);
        }

        // Append category +loads in registration order.
        sorted.extend(self.category_load_imps.iter().cloned());
        sorted
    }

    /// DFS helper: visit a class's superclass first if it also has +load.
    fn visit_load_topo(
        &self,
        name: &str,
        load_names: &HashSet<&str>,
        imp_map: &std::collections::HashMap<&str, u64>,
        visited: &mut HashSet<String>,
        sorted: &mut Vec<(String, u64)>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());
        if let Some(class) = self.classes_by_name.get(name) {
            if let Some(super_addr) = class.superclass_addr {
                if let Some(super_name) = self.classes_by_addr.get(&super_addr) {
                    if load_names.contains(super_name.as_ref()) {
                        self.visit_load_topo(super_name, load_names, imp_map, visited, sorted);
                    }
                }
            }
        }
        if let Some(&imp) = imp_map.get(name) {
            sorted.push((name.to_string(), imp));
        }
    }

    /// Fix up `__objc_selrefs` in mapped memory to point to interned selectors.
    pub fn fixup_selrefs(&mut self, metadata: &ObjcMetadata, slide: i64) {
        let mut fixed = 0usize;
        for (name, &vmaddr) in metadata.selectors.iter().zip(metadata.selref_addrs.iter()) {
            let slot_addr = (vmaddr as i64 + slide) as u64;
            let interned = self.intern_selector(name);
            unsafe {
                let ptr = slot_addr as *mut u64;
                ptr.write(interned as u64);
            }
            fixed += 1;
        }
        if fixed > 0 {
            log::debug!("ObjcRuntime: fixed up {} selrefs", fixed);
        }
    }
}
