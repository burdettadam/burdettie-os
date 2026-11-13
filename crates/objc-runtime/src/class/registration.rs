//! Class registration, category merging, and superclass repair.

use std::collections::HashMap;
use std::sync::Arc;

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
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
                })
                .collect();

            let class_methods: Vec<MethodEntry> = class_ref
                .class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
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
                name,
                mapped_addr,
                methods.len(),
                ivars.len(),
                properties.len(),
                instance_size,
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

            let name_arc: Arc<str> = Arc::from(name.as_str());
            self.classes_by_addr
                .insert(mapped_addr, Arc::clone(&name_arc));
            self.classes_by_name
                .insert(Arc::clone(&name_arc), Arc::new(info));
            self.mapped_class_addrs
                .insert(Arc::clone(&name_arc), mapped_addr);

            if class_ref.metaclass_addr != 0 {
                let meta_mapped = (class_ref.metaclass_addr as i64 + slide) as u64;
                self.classes_by_addr
                    .insert(meta_mapped, Arc::clone(&name_arc));
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

            let target_name: Arc<str> = match self.classes_by_addr.get(&cat_class_addr) {
                Some(name) => Arc::clone(name),
                None => {
                    log::debug!(
                        "ObjcRuntime: category '{}' targets unknown class at {:#x}",
                        cat.name.as_deref().unwrap_or("?"),
                        cat_class_addr,
                    );
                    runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::CategoryOrphan {
                        class_addr: cat_class_addr,
                    });
                    continue;
                }
            };

            // Record category +load IMPs before merging.
            for m in &cat.class_methods {
                if m.selector == "load" {
                    let imp = (m.imp_vmaddr as i64 + slide) as u64;
                    self.category_load_imps.push((target_name.to_string(), imp));
                }
            }

            if let Some(class_arc) = self.classes_by_name.get_mut(&*target_name) {
                let class_info = Arc::make_mut(class_arc);
                for m in &cat.instance_methods {
                    if class_info.method_index.contains_key(&*m.selector) {
                        log::debug!(
                            "ObjcRuntime: category skip -[{} {}] (shim exists)",
                            target_name,
                            m.selector,
                        );
                        continue;
                    }
                    class_info.methods.push(MethodEntry {
                        selector: m.selector.clone().into(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone().into(),
                    });
                }
                for m in &cat.class_methods {
                    if class_info.class_method_index.contains_key(&*m.selector) {
                        log::debug!(
                            "ObjcRuntime: category skip +[{} {}] (shim exists)",
                            target_name,
                            m.selector,
                        );
                        continue;
                    }
                    class_info.class_methods.push(MethodEntry {
                        selector: m.selector.clone().into(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone().into(),
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
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
                })
                .collect();
            let class_methods: Vec<MethodEntry> = proto
                .class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
                })
                .collect();
            let optional_instance_methods: Vec<MethodEntry> = proto
                .optional_instance_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
                })
                .collect();
            let optional_class_methods: Vec<MethodEntry> = proto
                .optional_class_methods
                .iter()
                .map(|m| MethodEntry {
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
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

            let target_name: Arc<str> = match self.classes_by_addr.get(&resolved_cls) {
                Some(name) => Arc::clone(name),
                None => {
                    log::debug!(
                        "ObjcRuntime: category '{}' targets unknown class at {:#x} (post-fixup)",
                        cat.name.as_deref().unwrap_or("?"),
                        resolved_cls,
                    );
                    runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::CategoryOrphan {
                        class_addr: resolved_cls,
                    });
                    continue;
                }
            };

            // Record category +load IMPs before merging.
            for m in &cat.class_methods {
                if m.selector == "load" {
                    let imp = (m.imp_vmaddr as i64 + slide) as u64;
                    self.category_load_imps.push((target_name.to_string(), imp));
                }
            }

            if let Some(class_arc) = self.classes_by_name.get_mut(&*target_name) {
                let class_info = Arc::make_mut(class_arc);
                for m in &cat.instance_methods {
                    if class_info.method_index.contains_key(&*m.selector) {
                        log::debug!(
                            "ObjcRuntime: category skip -[{} {}] (shim exists)",
                            target_name,
                            m.selector,
                        );
                        continue;
                    }
                    class_info.methods.push(MethodEntry {
                        selector: m.selector.clone().into(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone().into(),
                    });
                }
                for m in &cat.class_methods {
                    if class_info.class_method_index.contains_key(&*m.selector) {
                        log::debug!(
                            "ObjcRuntime: category skip +[{} {}] (shim exists)",
                            target_name,
                            m.selector,
                        );
                        continue;
                    }
                    class_info.class_methods.push(MethodEntry {
                        selector: m.selector.clone().into(),
                        imp: (m.imp_vmaddr as i64 + slide) as u64,
                        type_encoding: m.type_encoding.clone().into(),
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
    ///
    /// Its superclass is supplied in `info`; `mapped_addr` may be an opaque
    /// handle, so superclass repair never reads memory through that address.
    pub fn register_synthetic_class(&mut self, mut info: ClassInfo) {
        let addr = info.mapped_addr;
        let name = info.name.clone();
        info.method_index = HashMap::with_capacity(info.methods.len());
        for (i, m) in info.methods.iter().enumerate() {
            self.intern_selector(&m.selector);
            info.method_index.insert(m.selector.clone(), i);
        }
        info.class_method_index = HashMap::with_capacity(info.class_methods.len());
        for (i, m) in info.class_methods.iter().enumerate() {
            self.intern_selector(&m.selector);
            info.class_method_index.insert(m.selector.clone(), i);
        }
        log::debug!(
            "ObjcRuntime: registered synthetic class '{}' at {:#x}, {} methods",
            name,
            addr,
            info.methods.len(),
        );
        let name_arc: Arc<str> = Arc::from(name.as_str());
        self.mapped_class_addrs.remove(&name_arc);
        self.classes_by_addr.insert(addr, Arc::clone(&name_arc));
        self.classes_by_name.insert(name_arc, Arc::new(info));
    }

    /// Register a class backed by an image's mapped `objc_class_t` record.
    ///
    /// Use this when an exported class lacks parsed metadata but still needs
    /// its superclass resolved from the image after fixups. The mapped record
    /// must remain valid whenever [`Self::repair_superclass_addrs`] is called.
    pub fn register_mapped_class(&mut self, info: ClassInfo) {
        let addr = info.mapped_addr;
        let name: Arc<str> = Arc::from(info.name.as_str());
        self.register_synthetic_class(info);
        self.mapped_class_addrs.insert(name, addr);
    }

    /// Register a metaclass address mapping for a synthetic class.
    pub fn register_metaclass_addr(&mut self, metaclass_addr: u64, class_name: &str) {
        self.classes_by_addr
            .insert(metaclass_addr, Arc::from(class_name));
        self.metaclass_addrs.insert(metaclass_addr);
        log::debug!(
            "ObjcRuntime: registered synthetic metaclass for '{}' at {:#x}",
            class_name,
            metaclass_addr,
        );
    }

    /// Repair `superclass_addr` for image-backed classes by reading from
    /// mapped memory (post-fixup). Synthetic class hierarchies are preserved.
    ///
    /// # Safety
    ///
    /// Must be called after fixups are applied and before `mprotect`.
    /// Each class registered via `register_image` or `register_mapped_class`
    /// must still have a valid, readable mapped `objc_class_t` record at its
    /// registered address. The memory must not be concurrently modified.
    pub unsafe fn repair_superclass_addrs(&mut self) {
        for (name, &addr) in &self.mapped_class_addrs {
            let Some(class_arc) = self.classes_by_name.get_mut(name) else {
                continue;
            };
            if addr == 0 || class_arc.mapped_addr != addr {
                continue;
            }
            // Read the on-disk field layout without assuming a Rust allocation
            // alignment for the image record. The caller guarantees its extent.
            let super_ptr = unsafe { (addr as *const u64).add(1).read_unaligned() };
            let new_super = if super_ptr == 0 {
                None
            } else {
                Some(super_ptr)
            };
            // Preserve shared class metadata when the resolved parent matches.
            if class_arc.superclass_addr != new_super {
                let class = Arc::make_mut(class_arc);
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
