//! Method dispatch, selref fixup, and +load collection.

use macho_objc::ObjcMetadata;

use super::ObjcRuntime;

impl ObjcRuntime {
    /// Look up a method on a class by selector name.
    /// Returns the IMP address if found.
    pub fn lookup_method(&self, class_name: &str, selector: &str) -> Option<u64> {
        let class = self.classes_by_name.get(class_name)?;
        class
            .methods
            .iter()
            .find(|m| m.selector == selector)
            .map(|m| m.imp)
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
                let methods = if is_meta {
                    &class.class_methods
                } else {
                    &class.methods
                };
                if let Some(entry) = methods.iter().find(|m| m.selector == selector) {
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

    /// Collect all +load method IMPs.
    pub fn collect_load_methods(&self) -> Vec<(String, u64)> {
        let mut loads = Vec::new();
        for (name, info) in &self.classes_by_name {
            for m in &info.class_methods {
                if m.selector == "load" {
                    loads.push((name.clone(), m.imp));
                }
            }
        }
        loads
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
