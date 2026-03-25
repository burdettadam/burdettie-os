//! Class registration, category merging, and superclass repair.

use std::collections::HashMap;

use macho_objc::ObjcMetadata;

use super::{ClassInfo, IvarEntry, MethodEntry, ObjcRuntime, PropertyEntry, ProtocolInfo};

impl ObjcRuntime {
    /// Register classes from parsed ObjC metadata of a loaded image.
    ///
    /// `slide` is the ASLR slide (mapped base - preferred vmaddr).
    /// Method IMP addresses are adjusted by slide.
    pub fn register_image(&mut self, metadata: &ObjcMetadata, slide: i64) {
        for class_ref in &metadata.class_refs {
            let Some(name) = &class_ref.name else {
                continue;
            };

            let mapped_addr = (class_ref.class_data_addr as i64 + slide) as u64;

            let is_root = (class_ref.ro_flags & 0x02) != 0;
            let is_meta = (class_ref.ro_flags & 0x01) != 0;

            let instance_size = if class_ref.instance_size == 0 && is_root {
                8
            } else {
                class_ref.instance_size
            };

            let methods: Vec<MethodEntry> = class_ref
                .methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();

            let class_methods: Vec<MethodEntry> = class_ref
                .class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();

            for m in &class_ref.methods {
                self.intern_selector(&m.selector);
            }
            for m in &class_ref.class_methods {
                self.intern_selector(&m.selector);
            }

            let ivars: Vec<IvarEntry> = class_ref
                .ivars
                .iter()
                .map(|iv| IvarEntry {
                    name: iv.name.clone(),
                    type_encoding: iv.type_encoding.clone(),
                    offset: iv.offset,
                    size: iv.size,
                })
                .collect();

            let properties: Vec<PropertyEntry> = class_ref
                .properties
                .iter()
                .map(|p| PropertyEntry {
                    name: p.name.clone(),
                    attributes: p.attributes.clone(),
                })
                .collect();

            log::debug!(
                "ObjcRuntime: registered class '{}' at {:#x}, {} methods, {} ivars, {} props, size={}",
                name, mapped_addr, methods.len(), ivars.len(), properties.len(), instance_size,
            );

            let mut info = ClassInfo {
                name: name.clone(),
                mapped_addr,
                methods,
                class_methods,
                ivars,
                properties,
                protocol_names: class_ref.protocol_names.clone(),
                instance_size,
                is_root,
                is_meta,
                superclass_addr: if class_ref.superclass_addr != 0 {
                    Some((class_ref.superclass_addr as i64 + slide) as u64)
                } else {
                    None
                },
                method_index: HashMap::new(),
                class_method_index: HashMap::new(),
            };
            info.rebuild_indices();

            self.classes_by_addr.insert(mapped_addr, name.clone());
            self.classes_by_name.insert(name.clone(), info);

            if class_ref.metaclass_addr != 0 {
                let meta_mapped = (class_ref.metaclass_addr as i64 + slide) as u64;
                self.classes_by_addr.insert(meta_mapped, name.clone());
                self.metaclass_addrs.insert(meta_mapped);
                log::debug!(
                    "ObjcRuntime: registered metaclass for '{}' at {:#x}",
                    name,
                    meta_mapped,
                );
            }
        }

        // Merge same-image categories
        for cat in &metadata.categories {
            let cat_class_addr = (cat.class_vmaddr as i64 + slide) as u64;

            let target_name = match self.classes_by_addr.get(&cat_class_addr) {
                Some(name) => name.clone(),
                None => {
                    log::debug!(
                        "ObjcRuntime: category '{}' targets unknown class at {:#x}",
                        cat.name.as_deref().unwrap_or("?"),
                        cat_class_addr,
                    );
                    runtime_diagnostics::emit(
                        runtime_diagnostics::DiagEvent::CategoryOrphan { class_addr: cat_class_addr },
                    );
                    continue;
                }
            };

            if let Some(class_info) = self.classes_by_name.get_mut(&target_name) {
                for m in &cat.instance_methods {
                    class_info.methods.push(MethodEntry {
                        selector: m.selector.clone(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone(),
                    });
                }
                for m in &cat.class_methods {
                    class_info.class_methods.push(MethodEntry {
                        selector: m.selector.clone(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone(),
                    });
                }
                class_info.rebuild_indices();
                log::debug!(
                    "ObjcRuntime: merged category '{}' into '{}' ({} instance, {} class methods)",
                    cat.name.as_deref().unwrap_or("?"),
                    target_name,
                    cat.instance_methods.len(),
                    cat.class_methods.len(),
                );
            }

            for m in &cat.instance_methods {
                self.intern_selector(&m.selector);
            }
            for m in &cat.class_methods {
                self.intern_selector(&m.selector);
            }
        }

        // Register protocols
        for proto in &metadata.protocols {
            let instance_methods: Vec<MethodEntry> = proto
                .instance_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();
            let class_methods: Vec<MethodEntry> = proto
                .class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();
            let optional_instance_methods: Vec<MethodEntry> = proto
                .optional_instance_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();
            let optional_class_methods: Vec<MethodEntry> = proto
                .optional_class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone(),
                })
                .collect();
            let properties: Vec<PropertyEntry> = proto
                .properties
                .iter()
                .map(|p| PropertyEntry {
                    name: p.name.clone(),
                    attributes: p.attributes.clone(),
                })
                .collect();
            self.register_protocol(ProtocolInfo {
                name: proto.name.clone(),
                instance_methods,
                class_methods,
                optional_instance_methods,
                optional_class_methods,
                properties,
                adopted_protocols: proto.adopted_protocols.clone(),
            });
        }
    }

    /// Merge categories by reading the resolved `cls` pointer from mapped memory.
    ///
    /// For cross-image categories, the on-disk `cls` field is a bind slot (0x0).
    /// After fixups, the mapped memory contains the resolved class address.
    pub fn merge_categories_post_fixup(&mut self, metadata: &ObjcMetadata, slide: i64) {
        for cat in &metadata.categories {
            let on_disk_addr = (cat.class_vmaddr as i64 + slide) as u64;
            if self.classes_by_addr.contains_key(&on_disk_addr) {
                continue;
            }

            let cat_mapped_addr = (cat.cat_struct_vmaddr as i64 + slide) as u64;
            let cls_ptr_addr = cat_mapped_addr + 8;

            let resolved_cls = unsafe { *(cls_ptr_addr as *const u64) };

            if resolved_cls == 0 {
                log::debug!(
                    "ObjcRuntime: category '{}' has null cls after fixup",
                    cat.name.as_deref().unwrap_or("?"),
                );
                continue;
            }

            let target_name = match self.classes_by_addr.get(&resolved_cls) {
                Some(name) => name.clone(),
                None => {
                    log::debug!(
                        "ObjcRuntime: category '{}' targets unknown class at {:#x} (post-fixup)",
                        cat.name.as_deref().unwrap_or("?"),
                        resolved_cls,
                    );
                    runtime_diagnostics::emit(
                        runtime_diagnostics::DiagEvent::CategoryOrphan { class_addr: resolved_cls },
                    );
                    continue;
                }
            };

            if let Some(class_info) = self.classes_by_name.get_mut(&target_name) {
                for m in &cat.instance_methods {
                    class_info.methods.push(MethodEntry {
                        selector: m.selector.clone(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone(),
                    });
                }
                for m in &cat.class_methods {
                    class_info.class_methods.push(MethodEntry {
                        selector: m.selector.clone(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone(),
                    });
                }
                class_info.rebuild_indices();
                log::debug!(
                    "ObjcRuntime: merged cross-image category '{}' into '{}' ({} instance, {} class methods)",
                    cat.name.as_deref().unwrap_or("?"),
                    target_name,
                    cat.instance_methods.len(),
                    cat.class_methods.len(),
                );
            }

            for m in &cat.instance_methods {
                self.intern_selector(&m.selector);
            }
            for m in &cat.class_methods {
                self.intern_selector(&m.selector);
            }
        }
    }

    /// Register a synthetic class created by the runtime (not from parsed metadata).
    pub fn register_synthetic_class(&mut self, mut info: ClassInfo) {
        let addr = info.mapped_addr;
        let name = info.name.clone();
        for m in &info.methods {
            self.intern_selector(&m.selector);
        }
        for m in &info.class_methods {
            self.intern_selector(&m.selector);
        }
        info.rebuild_indices();
        log::debug!(
            "ObjcRuntime: registered synthetic class '{}' at {:#x}, {} methods",
            name,
            addr,
            info.methods.len(),
        );
        self.classes_by_addr.insert(addr, name.clone());
        self.classes_by_name.insert(name, info);
    }

    /// Register a metaclass address mapping for a synthetic class.
    pub fn register_metaclass_addr(&mut self, metaclass_addr: u64, class_name: &str) {
        self.classes_by_addr
            .insert(metaclass_addr, class_name.to_string());
        self.metaclass_addrs.insert(metaclass_addr);
        log::debug!(
            "ObjcRuntime: registered synthetic metaclass for '{}' at {:#x}",
            class_name,
            metaclass_addr,
        );
    }

    /// Repair `superclass_addr` for all registered classes by reading from
    /// mapped memory (post-fixup).
    ///
    /// # Safety
    ///
    /// Must be called after fixups are applied and before `mprotect`.
    /// The `mapped_addr` for each class must point to a valid, mapped
    /// `objc_class_t` struct.
    pub unsafe fn repair_superclass_addrs(&mut self) {
        for class in self.classes_by_name.values_mut() {
            let addr = class.mapped_addr;
            if addr == 0 {
                continue;
            }
            let super_ptr = unsafe { *((addr + 8) as *const u64) };
            let new_super = if super_ptr == 0 {
                None
            } else {
                Some(super_ptr)
            };
            if class.superclass_addr != new_super {
                log::debug!(
                    "ObjcRuntime: repaired superclass for '{}': {:?} → {:?}",
                    class.name,
                    class.superclass_addr,
                    new_super,
                );
                class.superclass_addr = new_super;
            }
        }
    }
}
