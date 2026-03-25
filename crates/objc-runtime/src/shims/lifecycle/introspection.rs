//! Class/method introspection: object_getClass, class_getInstanceMethod, etc.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

/// Fake `Method` struct for `class_copyMethodList` / `class_getInstanceMethod`.
///
/// Layout:
///   offset  0: selector_ptr  (u64 — pointer to interned SEL C string)
///   offset  8: type_encoding  (u64 — pointer to owned type encoding C string)
///   offset 16: imp            (u64 — function pointer)
///
/// Total: 24 bytes per Method.
pub(super) const METHOD_STRUCT_SIZE: usize = 24;

/// `Class object_getClass(id obj)`
pub extern "C" fn shim_object_get_class(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

/// `Class object_setClass(id obj, Class cls)` — change an object's ISA.
/// Returns the previous class.
pub extern "C" fn shim_object_set_class(obj: u64, cls: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let old_isa = unsafe { *(obj as *const u64) };
    unsafe { *(obj as *mut u64) = cls; }
    old_isa
}

/// `Class class_getSuperclass(Class cls)`
pub extern "C" fn shim_class_get_superclass(cls: u64) -> u64 {
    if cls == 0 {
        return 0;
    }
    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    match rt.get_class_by_addr(cls) {
        Some(info) => info.superclass_addr.unwrap_or(0),
        None => 0,
    }
}

/// `BOOL class_respondsToSelector(Class cls, SEL sel)`
pub extern "C" fn shim_class_responds_to_selector(cls: u64, sel: u64) -> u8 {
    if cls == 0 || sel == 0 {
        return 0;
    }
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    if rt.msg_lookup(cls, sel_str).is_some() {
        1
    } else {
        0
    }
}

/// `Method class_getInstanceMethod(Class cls, SEL sel)`
///
/// Returns a pointer to a heap-allocated Method struct, or NULL.
/// The struct is intentionally leaked — this matches Apple's runtime
/// behavior where Method pointers remain valid for the process lifetime.
pub extern "C" fn shim_class_get_instance_method(cls: u64, sel: u64) -> u64 {
    if cls == 0 || sel == 0 {
        return 0;
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    // Walk the superclass chain to find the method entry
    let Some(entry) = rt.lookup_method_entry(cls, sel_str) else {
        return 0;
    };
    let imp = entry.imp;
    let type_enc = entry.type_encoding.clone();

    // Allocate a fake Method struct
    let ptr = crate::pal_alloc().calloc(1, METHOD_STRUCT_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }

    // Fill in selector pointer
    let sel_ptr = rt.selector_ptr(sel_str).unwrap_or(sel as usize);
    let type_enc_ptr = if type_enc.is_empty() {
        0u64
    } else {
        let cstr = std::ffi::CString::new(type_enc).unwrap_or_default();
        cstr.into_raw() as u64
    };
    unsafe {
        *(ptr as *mut u64) = sel_ptr as u64;
        *((ptr as usize + 8) as *mut u64) = type_enc_ptr;
        *((ptr as usize + 16) as *mut u64) = imp;
    }

    ptr
}

/// `Method *class_copyMethodList(Class cls, unsigned int *outCount)`
///
/// Allocates and returns a NULL-terminated array of Method pointers.
/// Caller is responsible for `free()`-ing the array.
pub extern "C" fn shim_class_copy_method_list(cls: u64, out_count: *mut u32) -> u64 {
    if cls == 0 {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let Some(class) = rt.get_class_by_addr(cls) else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let methods = &class.methods;
    let count = methods.len();

    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }

    if count == 0 {
        return 0;
    }

    // Allocate array of (count + 1) pointers (NULL-terminated)
    let array = crate::pal_alloc().calloc(count + 1, 8) as u64;
    if array == 0 {
        return 0;
    }

    for (i, m) in methods.iter().enumerate() {
        let method_ptr = crate::pal_alloc().calloc(1, METHOD_STRUCT_SIZE) as u64;
        if method_ptr == 0 {
            continue;
        }
        let sel_ptr = rt.selector_ptr(&m.selector).unwrap_or(0);
        let type_cstr = std::ffi::CString::new(m.type_encoding.as_str())
            .unwrap_or_else(|_| std::ffi::CString::new("").unwrap());
        let type_ptr = type_cstr.into_raw();
        unsafe {
            *(method_ptr as *mut u64) = sel_ptr as u64;
            *((method_ptr as usize + 8) as *mut u64) = type_ptr as u64;
            *((method_ptr as usize + 16) as *mut u64) = m.imp;
            *((array as usize + i * 8) as *mut u64) = method_ptr;
        }
    }

    array
}

/// `SEL method_getName(Method m)` — returns the selector of a method.
pub extern "C" fn shim_method_get_name(method: u64) -> u64 {
    if method == 0 {
        return 0;
    }
    unsafe { *(method as *const u64) }
}

/// `const char *method_getTypeEncoding(Method m)`
pub extern "C" fn shim_method_get_type_encoding(method: u64) -> u64 {
    if method == 0 {
        return 0;
    }
    unsafe { *((method as usize + 8) as *const u64) }
}

/// `IMP method_getImplementation(Method m)`
pub extern "C" fn shim_method_get_implementation(method: u64) -> u64 {
    if method == 0 {
        return 0;
    }
    unsafe { *((method as usize + 16) as *const u64) }
}

/// `unsigned int method_getNumberOfArguments(Method m)`
///
/// Apple convention: count the colons in the selector + 2 (for self + _cmd).
/// If no type encoding and no selector, returns 2.
pub extern "C" fn shim_method_get_number_of_arguments(method: u64) -> u32 {
    if method == 0 {
        return 0;
    }
    let sel_ptr = unsafe { *(method as *const u64) };
    if sel_ptr == 0 {
        return 2;
    }
    let sel_str = unsafe { CStr::from_ptr(sel_ptr as *const c_char) }
        .to_str()
        .unwrap_or("");
    let colons = sel_str.chars().filter(|&c| c == ':').count();
    (colons + 2) as u32 // self + _cmd + one per colon
}

/// `IMP class_getMethodImplementation(Class cls, SEL sel)`
///
/// Returns the IMP that would be called for the given selector,
/// or a forwarding IMP. We return 0 if not found.
pub extern "C" fn shim_class_get_method_implementation(cls: u64, sel: u64) -> u64 {
    if cls == 0 || sel == 0 {
        return 0;
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    rt.msg_lookup(cls, sel_str).unwrap_or(0)
}

/// `unsigned int class_getInstanceSize(Class cls)`
pub extern "C" fn shim_class_get_instance_size(cls: u64) -> u32 {
    if cls == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    rt.get_class_by_addr(cls)
        .map(|c| c.instance_size)
        .unwrap_or(0)
}

/// `BOOL class_isMetaClass(Class cls)`
pub extern "C" fn shim_class_is_meta_class(cls: u64) -> u8 {
    if cls == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    rt.get_class_by_addr(cls)
        .map(|c| u8::from(c.is_meta))
        .unwrap_or(0)
}

/// `Method class_getClassMethod(Class cls, SEL sel)` — look up a class method.
///
/// Returns a heap-allocated fake Method struct, or NULL.
pub extern "C" fn shim_class_get_class_method(cls: u64, sel: u64) -> u64 {
    if cls == 0 || sel == 0 {
        return 0;
    }
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");
    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    // Look up in the class's class_methods list
    let class_info = rt.get_class_by_addr(cls);
    if let Some(info) = class_info {
        for method in &info.class_methods {
            if method.selector == sel_str {
                let imp = method.imp;
                let type_enc = method.type_encoding.clone();
                let sel_ptr = rt.selector_ptr(sel_str).unwrap_or(sel as usize);
                let type_enc_ptr = if type_enc.is_empty() {
                    0u64
                } else {
                    let cstr = std::ffi::CString::new(type_enc).unwrap_or_default();
                    cstr.into_raw() as u64
                };
                let ptr = crate::pal_alloc().calloc(1, METHOD_STRUCT_SIZE) as u64;
                if ptr == 0 {
                    return 0;
                }
                unsafe {
                    *(ptr as *mut u64) = sel_ptr as u64;
                    *((ptr as usize + 8) as *mut u64) = type_enc_ptr;
                    *((ptr as usize + 16) as *mut u64) = imp;
                }
                return ptr;
            }
        }
    }
    0
}
