//! Runtime method mutation: class_addMethod, replaceMethod, method swizzling.

use std::ffi::{c_char, CStr};

use super::dynamic_class::add_method_to_staged_class;
use crate::class::OBJC_RUNTIME;

/// `BOOL class_addMethod(Class cls, SEL name, IMP imp, const char *types)`
pub extern "C" fn shim_class_add_method(cls: u64, sel: u64, imp: u64, types: *const c_char) -> u8 {
    if cls == 0 || sel == 0 || imp == 0 {
        return 0;
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");
    let type_enc = if types.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(types) }
            .to_str()
            .unwrap_or("")
            .to_string()
    };

    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    let entry = crate::MethodEntry {
        selector: sel_str.to_string().into(),
        imp,
        type_encoding: type_enc.into(),
    };

    let added = if rt.add_method_to_class(cls, entry.clone()) {
        true
    } else {
        drop(guard);
        add_method_to_staged_class(cls, entry)
    };
    log::debug!(
        "class_addMethod: cls={:#x} sel='{}' imp={:#x} → {}",
        cls,
        sel_str,
        imp,
        added,
    );
    u8::from(added)
}

/// `IMP class_replaceMethod(Class cls, SEL name, IMP imp, const char *types)`
///
/// If the method already exists on the class, replaces its IMP and returns the
/// old IMP. If it doesn't exist, adds it (like `class_addMethod`) and returns NULL (0).
pub extern "C" fn shim_class_replace_method(
    cls: u64,
    sel: u64,
    imp: u64,
    types: *const c_char,
) -> u64 {
    if cls == 0 || sel == 0 || imp == 0 {
        return 0;
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");
    let type_enc = if types.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(types) }
            .to_str()
            .unwrap_or("")
            .to_string()
    };

    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    let Some(class) = rt.get_class_by_addr_mut(cls) else {
        return 0;
    };

    // Look for existing method
    if let Some(entry) = class.methods.iter_mut().find(|m| m.selector == sel_str) {
        let old_imp = entry.imp;
        entry.imp = imp;
        entry.type_encoding = type_enc.into();
        log::debug!(
            "class_replaceMethod: cls={:#x} sel='{}' old_imp={:#x} new_imp={:#x}",
            cls,
            sel_str,
            old_imp,
            imp
        );
        old_imp
    } else {
        // Method not found — add it (like class_addMethod)
        let idx = class.methods.len();
        let sel_owned = sel_str.to_string();
        class.methods.push(crate::MethodEntry {
            selector: sel_owned.clone().into(),
            imp,
            type_encoding: type_enc.into(),
        });
        class.method_index.insert(sel_owned.into(), idx);
        log::debug!(
            "class_replaceMethod: cls={:#x} sel='{}' added new method imp={:#x}",
            cls,
            sel_str,
            imp
        );
        0
    }
}

/// `IMP method_setImplementation(Method method, IMP imp)`
///
/// Sets a new IMP on a Method struct and on the corresponding `MethodEntry`
/// in the class. Returns the old IMP.
pub extern "C" fn shim_method_set_implementation(method: u64, imp: u64) -> u64 {
    if method == 0 || imp == 0 {
        return 0;
    }

    // Read old IMP and selector from the fake Method struct
    let old_imp = unsafe { *((method as usize + 16) as *const u64) };
    let sel_ptr = unsafe { *(method as *const u64) };

    // Update the fake Method struct
    unsafe {
        *((method as usize + 16) as *mut u64) = imp;
    }

    // Also update the real MethodEntry in the class
    if sel_ptr != 0 {
        let sel_str = unsafe { CStr::from_ptr(sel_ptr as *const c_char) }
            .to_str()
            .unwrap_or("");

        let mut guard = match OBJC_RUNTIME.write() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_mut() {
            // Search all classes for a method matching this selector + old IMP
            for class in rt.classes_by_name_iter_mut() {
                if let Some(entry) = class
                    .methods
                    .iter_mut()
                    .find(|m| m.selector == sel_str && m.imp == old_imp)
                {
                    entry.imp = imp;
                    break;
                }
            }
        }
    }

    log::debug!(
        "method_setImplementation: method={:#x} old_imp={:#x} new_imp={:#x}",
        method,
        old_imp,
        imp
    );
    old_imp
}

/// `void method_exchangeImplementations(Method m1, Method m2)`
///
/// Swaps the IMP pointers between two Methods. Updates both the fake Method
/// structs and the real `MethodEntry` records in the class Vec.
pub extern "C" fn shim_method_exchange_implementations(m1: u64, m2: u64) {
    if m1 == 0 || m2 == 0 {
        return;
    }

    // Read IMPs from fake Method structs
    let imp1 = unsafe { *((m1 as usize + 16) as *const u64) };
    let imp2 = unsafe { *((m2 as usize + 16) as *const u64) };

    if imp1 == imp2 {
        return;
    }

    // Read selectors
    let sel1_ptr = unsafe { *(m1 as *const u64) };
    let sel2_ptr = unsafe { *(m2 as *const u64) };

    // Swap IMPs in the fake Method structs
    unsafe {
        *((m1 as usize + 16) as *mut u64) = imp2;
        *((m2 as usize + 16) as *mut u64) = imp1;
    }

    // Update real MethodEntry records in the class registry.
    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_mut() {
        let sel1_str = if sel1_ptr != 0 {
            unsafe { CStr::from_ptr(sel1_ptr as *const c_char) }
                .to_str()
                .ok()
        } else {
            None
        };
        let sel2_str = if sel2_ptr != 0 {
            unsafe { CStr::from_ptr(sel2_ptr as *const c_char) }
                .to_str()
                .ok()
        } else {
            None
        };

        // Scan all classes, swapping matching entries in a single pass
        for class in rt.classes_by_name_iter_mut() {
            for entry in class.methods.iter_mut() {
                if let Some(s1) = sel1_str {
                    if entry.selector == s1 && entry.imp == imp1 {
                        entry.imp = imp2;
                        continue;
                    }
                }
                if let Some(s2) = sel2_str {
                    if entry.selector == s2 && entry.imp == imp2 {
                        entry.imp = imp1;
                    }
                }
            }
        }
    }

    log::debug!(
        "method_exchangeImplementations: swapped imp1={:#x} ↔ imp2={:#x}",
        imp1,
        imp2
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ClassInfo, MethodEntry, ObjcRuntime, OBJC_RUNTIME};
    use std::collections::HashMap;
    use std::ffi::CString;

    fn register_empty_class(name: &str, addr: u64) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        let mut info = ClassInfo {
            name: name.to_string(),
            mapped_addr: addr,
            methods: vec![],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 8,
            is_root: true,
            is_meta: false,
            superclass_addr: None,
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn register_class_with_method(name: &str, addr: u64, sel: &str, imp: u64) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        let mut info = ClassInfo {
            name: name.to_string(),
            mapped_addr: addr,
            methods: vec![MethodEntry {
                selector: sel.to_string().into(),
                imp,
                type_encoding: "v@:".into(),
            }],
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 8,
            is_root: true,
            is_meta: false,
            superclass_addr: None,
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    #[test]
    fn add_method_null_cls() {
        let sel = CString::new("foo").unwrap();
        assert_eq!(
            shim_class_add_method(0, sel.as_ptr() as u64, 0x1000, std::ptr::null()),
            0
        );
    }

    #[test]
    fn add_method_null_sel() {
        assert_eq!(
            shim_class_add_method(0xB001_0000, 0, 0x1000, std::ptr::null()),
            0
        );
    }

    #[test]
    fn add_method_success() {
        register_empty_class("MutTestA", 0xB001_0000);
        let sel = CString::new("foo").unwrap();
        let types = CString::new("v@:").unwrap();
        let result =
            shim_class_add_method(0xB001_0000, sel.as_ptr() as u64, 0x1000, types.as_ptr());
        assert_eq!(result, 1);
    }

    #[test]
    fn add_method_duplicate() {
        register_empty_class("MutTestB", 0xB001_0001);
        let sel = CString::new("bar").unwrap();
        let types = CString::new("v@:").unwrap();
        shim_class_add_method(0xB001_0001, sel.as_ptr() as u64, 0x2000, types.as_ptr());
        let result =
            shim_class_add_method(0xB001_0001, sel.as_ptr() as u64, 0x3000, types.as_ptr());
        assert_eq!(result, 0);
    }

    #[test]
    fn add_method_to_staged_class_pair_before_register() {
        use crate::shims::lifecycle::dynamic_class::{
            shim_objc_allocate_class_pair, shim_objc_register_class_pair,
        };

        let name = CString::new("MutTestStaged").unwrap();
        let cls = shim_objc_allocate_class_pair(0, name.as_ptr(), 0);
        assert_ne!(cls, 0);

        let sel = CString::new("applicationDidFinishLaunching:").unwrap();
        let types = CString::new("v@:@").unwrap();
        assert_eq!(
            shim_class_add_method(cls, sel.as_ptr() as u64, 0x7000, types.as_ptr()),
            1
        );

        shim_objc_register_class_pair(cls);

        let guard = OBJC_RUNTIME.read().unwrap();
        let rt = guard.as_ref().unwrap();
        let registered = rt.get_class("MutTestStaged").unwrap();
        assert!(registered
            .find_method("applicationDidFinishLaunching:")
            .is_some());
    }

    #[test]
    fn replace_method_new() {
        register_empty_class("MutTestC", 0xB001_0002);
        let sel = CString::new("baz").unwrap();
        let types = CString::new("v@:").unwrap();
        let old =
            shim_class_replace_method(0xB001_0002, sel.as_ptr() as u64, 0x4000, types.as_ptr());
        assert_eq!(old, 0);
    }

    #[test]
    fn replace_method_existing() {
        register_class_with_method("MutTestD", 0xB001_0003, "qux", 0x5000);
        let sel = CString::new("qux").unwrap();
        let types = CString::new("v@:").unwrap();
        let old =
            shim_class_replace_method(0xB001_0003, sel.as_ptr() as u64, 0x6000, types.as_ptr());
        assert_eq!(old, 0x5000);
    }

    #[test]
    fn set_implementation_null() {
        assert_eq!(shim_method_set_implementation(0, 0x1000), 0);
    }

    #[test]
    fn exchange_implementations_null() {
        shim_method_exchange_implementations(0, 0);
        shim_method_exchange_implementations(0, 0x1000);
        shim_method_exchange_implementations(0x1000, 0);
    }
}
