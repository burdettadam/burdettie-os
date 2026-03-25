//! Runtime method mutation: class_addMethod, replaceMethod, method swizzling.

use std::ffi::{c_char, CStr};

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

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    let entry = crate::MethodEntry {
        selector: sel_str.to_string(),
        imp,
        type_encoding: type_enc,
    };

    let added = rt.add_method_to_class(cls, entry);
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

    let mut guard = match OBJC_RUNTIME.lock() {
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
        entry.type_encoding = type_enc;
        log::debug!(
            "class_replaceMethod: cls={:#x} sel='{}' old_imp={:#x} new_imp={:#x}",
            cls, sel_str, old_imp, imp
        );
        old_imp
    } else {
        // Method not found — add it (like class_addMethod)
        let idx = class.methods.len();
        let sel_owned = sel_str.to_string();
        class.methods.push(crate::MethodEntry {
            selector: sel_owned.clone(),
            imp,
            type_encoding: type_enc,
        });
        class.method_index.insert(sel_owned, idx);
        log::debug!(
            "class_replaceMethod: cls={:#x} sel='{}' added new method imp={:#x}",
            cls, sel_str, imp
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

        let mut guard = match OBJC_RUNTIME.lock() {
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
        method, old_imp, imp
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
    let mut guard = match OBJC_RUNTIME.lock() {
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
        imp1, imp2
    );
}
