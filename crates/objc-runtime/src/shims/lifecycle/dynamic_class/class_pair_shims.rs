use std::collections::HashMap;
use std::ffi::{c_char, CStr};

use crate::class::{ClassInfo, OBJC_RUNTIME};

use super::{alloc_synth_addr, StagedClassInit, StagedClassPair, STAGED_CLASSES};

/// to `objc_getClass`; call `objc_registerClassPair` to finalize.
pub extern "C" fn shim_objc_allocate_class_pair(
    superclass: u64,
    name: *const c_char,
    _extra_bytes: u64,
) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    // Check if the name is already registered or staged.
    {
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_ref() {
            if rt.get_class(name_str).is_some() {
                return 0; // already exists
            }
        }
    }

    // Resolve superclass info.
    let (superclass_addr, instance_size, is_root) = if superclass == 0 {
        (None, 8u32, true) // root class — minimum 8 bytes for ISA
    } else {
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_ref() {
            if let Some(super_info) = rt.get_class_by_addr(superclass) {
                (Some(superclass), super_info.instance_size, false)
            } else {
                return 0; // superclass not found
            }
        } else {
            return 0;
        }
    };

    let cls_addr = alloc_synth_addr();
    let meta_addr = alloc_synth_addr();

    let class_info = ClassInfo {
        name: name_str.to_string(),
        mapped_addr: cls_addr,
        methods: Vec::new(),
        class_methods: Vec::new(),
        ivars: Vec::new(),
        properties: Vec::new(),
        protocol_names: Vec::new(),
        instance_size,
        is_root,
        is_meta: false,
        superclass_addr: superclass_addr,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };

    STAGED_CLASSES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            cls_addr,
            StagedClassPair {
                class: class_info,
                metaclass_addr: meta_addr,
            },
        );

    log::debug!(
        "objc_allocateClassPair: allocated '{}' at {:#x} (meta {:#x})",
        name_str,
        cls_addr,
        meta_addr
    );

    cls_addr
}

/// `void objc_registerClassPair(Class cls)`
///
/// Finalizes a previously allocated class pair and makes it visible to
/// `objc_getClass` and method dispatch.
pub extern "C" fn shim_objc_register_class_pair(cls: u64) {
    if cls == 0 {
        return;
    }

    let pair = STAGED_CLASSES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&cls);

    let Some(pair) = pair else { return };

    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return };

    let name = pair.class.name.clone();
    let meta_addr = pair.metaclass_addr;

    rt.register_synthetic_class(pair.class);
    rt.register_metaclass_addr(meta_addr, &name);

    log::debug!(
        "objc_registerClassPair: registered '{}' at {:#x}",
        name,
        cls
    );
}

/// `void objc_disposeClassPair(Class cls)`
///
/// Destroys a previously created class pair. In our runtime this removes
/// the class from the class table. Only works for dynamically created classes.
pub extern "C" fn shim_objc_dispose_class_pair(cls: u64) {
    if cls == 0 {
        return;
    }

    // First check if it's still in staging (never registered).
    {
        let mut staged = STAGED_CLASSES.lock().unwrap_or_else(|e| e.into_inner());
        if staged.remove(&cls).is_some() {
            log::debug!("objc_disposeClassPair: removed staged class {:#x}", cls);
            return;
        }
    }

    // Only dispose dynamically-created classes (high sentinel range).
    if cls < 0x0050_0000_0000_0000 {
        log::warn!(
            "objc_disposeClassPair: refusing to dispose non-dynamic class {:#x}",
            cls
        );
        return;
    }

    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return };

    rt.remove_class_by_addr(cls);
}

/// `Class objc_duplicateClass(Class original, const char *name, size_t extraBytes)`
///
/// Creates a copy of a class with a new name. Used by KVO to create
/// `NSKVONotifying_ClassName` subclasses.
pub extern "C" fn shim_objc_duplicate_class(
    original: u64,
    name: *const c_char,
    _extra_bytes: u64,
) -> u64 {
    if original == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    // Already exists?
    if rt.get_class(name_str).is_some() {
        return 0;
    }

    let Some(orig) = rt.get_class_by_addr(original) else {
        return 0;
    };

    let new_addr = alloc_synth_addr();
    let meta_addr = alloc_synth_addr();

    let mut dup = orig.clone();
    dup.name = name_str.to_string();
    dup.mapped_addr = new_addr;
    dup.superclass_addr = Some(original); // duplicate is a subclass of original

    rt.register_synthetic_class(dup);
    rt.register_metaclass_addr(meta_addr, name_str);

    log::debug!(
        "objc_duplicateClass: duplicated {:#x} as '{}' at {:#x}",
        original,
        name_str,
        new_addr
    );

    new_addr
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ObjcRuntime, OBJC_RUNTIME};
    use std::ffi::CString;

    fn ensure_runtime() {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        guard.get_or_insert_with(ObjcRuntime::new);
    }

    #[test]
    fn allocate_class_pair_null_name() {
        ensure_runtime();
        assert_eq!(shim_objc_allocate_class_pair(0, std::ptr::null(), 0), 0);
    }

    #[test]
    fn allocate_class_pair_empty_name() {
        ensure_runtime();
        let name = CString::new("").unwrap();
        assert_eq!(shim_objc_allocate_class_pair(0, name.as_ptr(), 0), 0);
    }

    #[test]
    fn allocate_and_register_class_pair() {
        let name = CString::new("DynTestA").unwrap();
        ensure_runtime();
        let cls = shim_objc_allocate_class_pair(0, name.as_ptr(), 0);
        assert_ne!(cls, 0);
        shim_objc_register_class_pair(cls);
        let guard = OBJC_RUNTIME.read().unwrap();
        let rt = guard.as_ref().unwrap();
        assert!(rt.get_class("DynTestA").is_some());
    }

    #[test]
    fn allocate_duplicate_name() {
        let name = CString::new("DynTestB").unwrap();
        ensure_runtime();
        let cls = shim_objc_allocate_class_pair(0, name.as_ptr(), 0);
        assert_ne!(cls, 0);
        shim_objc_register_class_pair(cls);
        let cls2 = shim_objc_allocate_class_pair(0, name.as_ptr(), 0);
        assert_eq!(cls2, 0);
    }

    #[test]
    fn dispose_class_pair_null() {
        shim_objc_dispose_class_pair(0);
    }

    #[test]
    fn dispose_staged_class() {
        let name = CString::new("DynTestC").unwrap();
        ensure_runtime();
        let cls = shim_objc_allocate_class_pair(0, name.as_ptr(), 0);
        assert_ne!(cls, 0);
        shim_objc_dispose_class_pair(cls);
    }

    #[test]
    fn register_class_pair_null() {
        shim_objc_register_class_pair(0);
    }
}
