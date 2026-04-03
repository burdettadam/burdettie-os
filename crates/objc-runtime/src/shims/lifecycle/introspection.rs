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
    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
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
        let cstr = std::ffi::CString::new(type_enc.as_ref()).unwrap_or_default();
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

    let guard = match OBJC_RUNTIME.read() {
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
        let type_cstr = std::ffi::CString::new(m.type_encoding.as_ref())
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

    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
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
    let guard = match OBJC_RUNTIME.read() {
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
                    let cstr = std::ffi::CString::new(type_enc.as_ref()).unwrap_or_default();
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

// ── Type encoding helpers ──────────────────────────────────────────

/// Parse an ObjC type encoding string and extract individual type components.
///
/// ObjC type encodings look like `v16@0:8` (void return, id arg0, SEL arg1)
/// or `@24@0:8@16` (object return, id, SEL, object).
/// Each component starts with a type char (possibly with modifiers like ^)
/// and is optionally followed by a stack offset number.
fn split_type_encoding(enc: &str) -> Vec<&str> {
    if enc.is_empty() {
        return Vec::new();
    }
    let bytes = enc.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        // Skip the type character(s)
        match bytes[i] {
            b'^' | b'r' | b'n' | b'N' | b'o' | b'O' | b'R' | b'V' => {
                i += 1;
                continue;
            }
            b'{' => {
                // Skip to closing '}'
                while i < bytes.len() && bytes[i] != b'}' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1; // skip '}'
                }
            }
            b'(' => {
                while i < bytes.len() && bytes[i] != b')' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
        // Skip digits (stack offset)
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        parts.push(&enc[start..i]);
        start = i;
    }
    parts
}

/// Extract just the type portion from a type-encoding component (strip the trailing digits).
fn type_from_component(component: &str) -> &str {
    let end = component
        .bytes()
        .rposition(|b| !b.is_ascii_digit())
        .map(|p| p + 1)
        .unwrap_or(component.len());
    &component[..end]
}

/// `char *method_copyReturnType(Method m)`
///
/// Returns a malloc'd C string with the ObjC type encoding for the return type.
/// Caller is responsible for freeing the result.
pub extern "C" fn shim_method_copy_return_type(method: u64) -> *mut u8 {
    if method == 0 {
        return std::ptr::null_mut();
    }
    let type_enc_ptr = unsafe { *((method as usize + 8) as *const u64) };
    if type_enc_ptr == 0 {
        return std::ptr::null_mut();
    }
    let enc = unsafe { CStr::from_ptr(type_enc_ptr as *const c_char) }
        .to_str()
        .unwrap_or("");
    let parts = split_type_encoding(enc);
    let ret_type = parts
        .first()
        .map(|p| type_from_component(p))
        .unwrap_or("");
    if ret_type.is_empty() {
        return std::ptr::null_mut();
    }
    let cstr = std::ffi::CString::new(ret_type).unwrap_or_default();
    cstr.into_raw() as *mut u8
}

/// `char *method_copyArgumentType(Method m, unsigned int index)`
///
/// Returns a malloc'd C string with the ObjC type encoding for argument
/// at the given index. Index 0 = self, 1 = _cmd, 2+ = user arguments.
/// Caller is responsible for freeing the result.
pub extern "C" fn shim_method_copy_argument_type(method: u64, index: u32) -> *mut u8 {
    if method == 0 {
        return std::ptr::null_mut();
    }
    let type_enc_ptr = unsafe { *((method as usize + 8) as *const u64) };
    if type_enc_ptr == 0 {
        return std::ptr::null_mut();
    }
    let enc = unsafe { CStr::from_ptr(type_enc_ptr as *const c_char) }
        .to_str()
        .unwrap_or("");
    let parts = split_type_encoding(enc);
    // parts[0] is the return type, parts[1..] are arguments
    let arg_index = (index as usize) + 1; // skip return type
    let arg_type = parts
        .get(arg_index)
        .map(|p| type_from_component(p))
        .unwrap_or("");
    if arg_type.is_empty() {
        return std::ptr::null_mut();
    }
    let cstr = std::ffi::CString::new(arg_type).unwrap_or_default();
    cstr.into_raw() as *mut u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ClassInfo, MethodEntry, ObjcRuntime, OBJC_RUNTIME};
    use std::collections::HashMap;
    use std::ffi::CString;

    fn setup_test_class(name: &str, addr: u64, methods: Vec<(&'static str, u64)>) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        let mut info = ClassInfo {
            name: name.to_string(),
            mapped_addr: addr,
            methods: methods
                .iter()
                .map(|(s, imp)| MethodEntry {
                    selector: (*s).into(),
                    imp: *imp,
                    type_encoding: "v@:".into(),
                })
                .collect(),
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
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
    fn object_get_class_null() {
        assert_eq!(shim_object_get_class(0), 0);
    }

    #[test]
    fn object_set_class_null() {
        assert_eq!(shim_object_set_class(0, 0x1234), 0);
    }

    #[test]
    fn class_get_superclass_null() {
        assert_eq!(shim_class_get_superclass(0), 0);
    }

    #[test]
    fn class_get_superclass_root() {
        setup_test_class("IntroTestA", 0xA001_0000, vec![]);
        assert_eq!(shim_class_get_superclass(0xA001_0000), 0);
    }

    #[test]
    fn class_responds_to_selector_null_cls() {
        let sel = CString::new("init").unwrap();
        assert_eq!(shim_class_responds_to_selector(0, sel.as_ptr() as u64), 0);
    }

    #[test]
    fn class_responds_to_selector_null_sel() {
        setup_test_class("IntroTestB", 0xA002_0000, vec![("init", 0xBEEF)]);
        assert_eq!(shim_class_responds_to_selector(0xA002_0000, 0), 0);
    }

    #[test]
    fn class_responds_to_selector_found() {
        setup_test_class("IntroTestC", 0xA003_0000, vec![("init", 0xBEEF)]);
        let sel = CString::new("init").unwrap();
        assert_eq!(
            shim_class_responds_to_selector(0xA003_0000, sel.as_ptr() as u64),
            1
        );
    }

    #[test]
    fn class_responds_to_selector_not_found() {
        setup_test_class("IntroTestD", 0xA004_0000, vec![("init", 0xBEEF)]);
        let sel = CString::new("dealloc").unwrap();
        assert_eq!(
            shim_class_responds_to_selector(0xA004_0000, sel.as_ptr() as u64),
            0
        );
    }

    #[test]
    fn class_get_instance_method_null() {
        assert_eq!(shim_class_get_instance_method(0, 0), 0);
    }

    #[test]
    fn class_get_instance_method_found() {
        let imp_val = 0xCAFE_0000u64;
        setup_test_class("IntroTestE", 0xA005_0000, vec![("alloc", imp_val)]);
        let sel = CString::new("alloc").unwrap();
        let method = shim_class_get_instance_method(0xA005_0000, sel.as_ptr() as u64);
        assert_ne!(method, 0, "should return a Method pointer");
        let got_imp = shim_method_get_implementation(method);
        assert_eq!(got_imp, imp_val);
    }

    #[test]
    fn class_copy_method_list_null() {
        let mut count: u32 = 99;
        let arr = shim_class_copy_method_list(0, &mut count as *mut u32);
        assert_eq!(arr, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn class_copy_method_list_empty() {
        setup_test_class("IntroTestF", 0xA006_0000, vec![]);
        let mut count: u32 = 99;
        let arr = shim_class_copy_method_list(0xA006_0000, &mut count as *mut u32);
        assert_eq!(arr, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn class_get_instance_size_null() {
        assert_eq!(shim_class_get_instance_size(0), 0);
    }

    #[test]
    fn class_get_instance_size_valid() {
        setup_test_class("IntroTestG", 0xA007_0000, vec![]);
        assert_eq!(shim_class_get_instance_size(0xA007_0000), 16);
    }

    #[test]
    fn class_is_meta_class_not_meta() {
        setup_test_class("IntroTestH", 0xA008_0000, vec![]);
        assert_eq!(shim_class_is_meta_class(0xA008_0000), 0);
    }

    // ── split_type_encoding tests ──────────────────────────────────

    #[test]
    fn split_type_encoding_empty() {
        assert!(split_type_encoding("").is_empty());
    }

    #[test]
    fn split_type_encoding_void_at_colon() {
        // "v16@0:8" → void return, id self, SEL _cmd
        let parts = split_type_encoding("v16@0:8");
        assert_eq!(parts.len(), 3);
        assert_eq!(type_from_component(parts[0]), "v");
        assert_eq!(type_from_component(parts[1]), "@");
        assert_eq!(type_from_component(parts[2]), ":");
    }

    #[test]
    fn split_type_encoding_object_return() {
        // "@24@0:8@16" → object return, id, SEL, object
        let parts = split_type_encoding("@24@0:8@16");
        assert_eq!(parts.len(), 4);
        assert_eq!(type_from_component(parts[0]), "@");
        assert_eq!(type_from_component(parts[3]), "@");
    }

    // ── method_copyReturnType tests ────────────────────────────────

    fn make_fake_method(type_encoding: &str) -> u64 {
        let ptr = crate::pal_alloc().calloc(1, METHOD_STRUCT_SIZE) as u64;
        if !type_encoding.is_empty() {
            let cstr = CString::new(type_encoding).unwrap();
            let type_ptr = cstr.into_raw() as u64;
            unsafe {
                *((ptr as usize + 8) as *mut u64) = type_ptr;
            }
        }
        ptr
    }

    #[test]
    fn method_copy_return_type_null() {
        assert!(shim_method_copy_return_type(0).is_null());
    }

    #[test]
    fn method_copy_return_type_void() {
        let method = make_fake_method("v16@0:8");
        let ret = shim_method_copy_return_type(method);
        assert!(!ret.is_null());
        let s = unsafe { CStr::from_ptr(ret as *const c_char) }
            .to_str()
            .unwrap();
        assert_eq!(s, "v");
        // Free the returned string
        unsafe { drop(CString::from_raw(ret as *mut c_char)); }
    }

    #[test]
    fn method_copy_return_type_object() {
        let method = make_fake_method("@24@0:8@16");
        let ret = shim_method_copy_return_type(method);
        assert!(!ret.is_null());
        let s = unsafe { CStr::from_ptr(ret as *const c_char) }
            .to_str()
            .unwrap();
        assert_eq!(s, "@");
        unsafe { drop(CString::from_raw(ret as *mut c_char)); }
    }

    // ── method_copyArgumentType tests ──────────────────────────────

    #[test]
    fn method_copy_argument_type_null() {
        assert!(shim_method_copy_argument_type(0, 0).is_null());
    }

    #[test]
    fn method_copy_argument_type_self() {
        // "v16@0:8" → arg 0 = @ (self), arg 1 = : (_cmd)
        let method = make_fake_method("v16@0:8");
        let arg0 = shim_method_copy_argument_type(method, 0);
        assert!(!arg0.is_null());
        let s = unsafe { CStr::from_ptr(arg0 as *const c_char) }
            .to_str()
            .unwrap();
        assert_eq!(s, "@");
        unsafe { drop(CString::from_raw(arg0 as *mut c_char)); }
    }

    #[test]
    fn method_copy_argument_type_cmd() {
        let method = make_fake_method("v16@0:8");
        let arg1 = shim_method_copy_argument_type(method, 1);
        assert!(!arg1.is_null());
        let s = unsafe { CStr::from_ptr(arg1 as *const c_char) }
            .to_str()
            .unwrap();
        assert_eq!(s, ":");
        unsafe { drop(CString::from_raw(arg1 as *mut c_char)); }
    }

    #[test]
    fn method_copy_argument_type_out_of_range() {
        let method = make_fake_method("v16@0:8");
        let arg5 = shim_method_copy_argument_type(method, 5);
        assert!(arg5.is_null());
    }
}
