//! Class/selector lookup and object allocation shims.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

/// Try to resolve a class address to its name using the host ObjC runtime's `class_getName`.
///
/// Only safe to call for addresses that reside in the dyld shared cache (host framework classes).
/// Guest binary addresses are NOT valid host ObjC class pointers and would crash `class_getName`.
#[cfg(unix)]
fn host_class_name(cls: u64) -> Option<String> {
    // Determine shared cache bounds to validate the address.
    type GetCacheRangeFn = unsafe extern "C" fn(*mut usize) -> *const u8;
    let cache_fn_sym = unsafe {
        libc::dlsym(
            libc::RTLD_DEFAULT,
            b"_dyld_get_shared_cache_range\0".as_ptr() as *const c_char,
        )
    };
    if !cache_fn_sym.is_null() {
        let get_range: GetCacheRangeFn = unsafe { std::mem::transmute(cache_fn_sym) };
        let mut size: usize = 0;
        let base = unsafe { get_range(&mut size) } as u64;
        if base != 0 && size > 0 {
            let end = base + size as u64;
            if cls < base || cls >= end {
                return None;
            }
        }
    }

    type ClassGetNameFn = unsafe extern "C" fn(*const u8) -> *const c_char;
    let sym = unsafe {
        libc::dlsym(
            libc::RTLD_DEFAULT,
            b"class_getName\0".as_ptr() as *const c_char,
        )
    };
    if sym.is_null() {
        return None;
    }
    let class_get_name: ClassGetNameFn = unsafe { std::mem::transmute(sym) };

    let ptr = cls as *const u8;
    if ptr.is_null() {
        return None;
    }
    let name_ptr = unsafe { class_get_name(ptr) };
    if name_ptr.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(name_ptr) }.to_str().ok()?;
    if name.is_empty() || name == "nil" {
        return None;
    }
    Some(name.to_string())
}

#[cfg(not(unix))]
fn host_class_name(_cls: u64) -> Option<String> {
    None
}

/// `Class objc_getClass(const char *name)`
#[no_mangle]
pub extern "C" fn shim_objc_get_class(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    match rt.get_class(name_str) {
        Some(cls) => cls.mapped_addr,
        None => {
            log::warn!("objc_getClass: class '{}' not found", name_str);
            runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::ClassMissing {
                name: name_str.into(),
            });
            0
        }
    }
}

/// `Class objc_getMetaClass(const char *name)`
///
/// Like `objc_getClass`, but returns the metaclass pointer instead.
pub extern "C" fn shim_objc_get_meta_class(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    match rt.get_class(name_str) {
        Some(cls) => {
            // The metaclass address is stored at the class ISA (offset 0)
            if cls.mapped_addr == 0 {
                return 0;
            }
            unsafe { *(cls.mapped_addr as *const u64) }
        }
        None => 0,
    }
}

/// `const char *object_getClassName(id obj)`
///
/// Returns the class name for an object (convenience: `class_getName(object_getClass(obj))`).
pub extern "C" fn shim_object_get_class_name(obj: u64) -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    if obj == 0 {
        return EMPTY.as_ptr() as *const c_char;
    }
    let isa = unsafe { *(obj as *const u64) };
    shim_class_get_name(isa)
}

/// `id class_createInstance(Class cls, size_t extraBytes)`
pub extern "C" fn shim_class_create_instance(cls: u64, extra_bytes: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    // First try: look up in our runtime.
    let (name, instance_size) = {
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else {
            return 0;
        };
        if let Some(ci) = rt.get_class_by_addr(cls) {
            (ci.name.clone(), ci.instance_size)
        } else {
            // Try to resolve via host runtime and register as ISA alias.
            if let Some(host_name) = host_class_name(cls) {
                drop(guard);
                {
                    let mut wg = match OBJC_RUNTIME.write() {
                        Ok(g) => g,
                        Err(e) => e.into_inner(),
                    };
                    if let Some(rt_mut) = wg.as_mut() {
                        rt_mut.register_isa_alias(cls, &host_name);
                    }
                }
                let guard2 = match OBJC_RUNTIME.read() {
                    Ok(g) => g,
                    Err(e) => e.into_inner(),
                };
                let Some(rt2) = guard2.as_ref() else {
                    return 0;
                };
                if let Some(ci) = rt2.get_class_by_addr(cls) {
                    log::info!(
                        "class_createInstance: resolved host class '{}' at {:#x}",
                        host_name,
                        cls
                    );
                    (ci.name.clone(), ci.instance_size)
                } else {
                    log::warn!(
                        "class_createInstance: host class '{}' at {:#x} has no shim",
                        host_name,
                        cls
                    );
                    return 0;
                }
            } else {
                log::warn!("class_createInstance: unknown class at {:#x}", cls);
                runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::ClassAddrUnknown {
                    addr: cls,
                });
                return 0;
            }
        }
    };

    let size = std::cmp::max(instance_size as usize, 8) + extra_bytes as usize;
    let ptr = crate::pal_alloc().calloc(1, size) as u64;
    if ptr == 0 {
        return 0;
    }

    // SAFETY: ptr is freshly allocated and at least 8 bytes.
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    // Diagnostic: trace alloc for ELS* and MutableDict classes
    if name.contains("ELS") || name.contains("MutableDict") {
        eprintln!("[alloc] class={name} cls={cls:#x} ptr={ptr:#x} size={size}");
    }
    log::debug!(
        "class_createInstance: allocated {} bytes for '{}' at {:#x}",
        size,
        name,
        ptr
    );
    ptr
}

/// `SEL sel_registerName(const char *name)`
pub extern "C" fn shim_sel_register_name(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return name as u64;
    };

    if let Some(ptr) = rt.selector_ptr(name_str) {
        return ptr as u64;
    }
    name as u64
}

/// `const char *class_getName(Class cls)`
#[no_mangle]
pub extern "C" fn shim_class_get_name(cls: u64) -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    if cls == 0 {
        return EMPTY.as_ptr() as *const c_char;
    }

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return EMPTY.as_ptr() as *const c_char;
    };

    match rt.get_class_by_addr(cls) {
        Some(info) => {
            if let Some(ptr) = rt.selector_ptr(&info.name) {
                return ptr as *const c_char;
            }
            // Class name isn't an interned selector — leak a CString for a
            // stable pointer.  Each unique name only leaks once thanks to
            // the selector cache above on subsequent calls.
            let c_str = std::ffi::CString::new(info.name.as_str()).unwrap_or_default();
            c_str.into_raw()
        }
        None => EMPTY.as_ptr() as *const c_char,
    }
}

/// `id objc_alloc(Class cls)`
pub extern "C" fn shim_objc_alloc(cls: u64) -> u64 {
    shim_class_create_instance(cls, 0)
}

/// `id objc_allocWithZone(Class cls, void *zone)`
pub extern "C" fn shim_objc_alloc_with_zone(cls: u64, _zone: u64) -> u64 {
    shim_objc_alloc(cls)
}

/// `id objc_alloc_init(Class cls)`
pub extern "C" fn shim_objc_alloc_init(cls: u64) -> u64 {
    let obj = shim_class_create_instance(cls, 0);
    if obj == 0 {
        return 0;
    }
    let isa = unsafe { *(obj as *const u64) };
    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let init_imp = guard.as_ref().and_then(|rt| rt.msg_lookup(isa, "init"));
    drop(guard);
    if let Some(imp) = init_imp {
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(imp) };
        f(obj, 0)
    } else {
        obj
    }
}

/// `id objc_opt_new(Class cls)` — fast path for `[cls new]`
pub extern "C" fn shim_objc_opt_new(cls: u64) -> u64 {
    shim_objc_alloc_init(cls)
}

/// `id objc_opt_self(id obj)`
pub extern "C" fn shim_objc_opt_self(obj: u64) -> u64 {
    obj
}

/// `Class objc_opt_class(id obj)`
pub extern "C" fn shim_objc_opt_class(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        if rt.get_class_by_addr(obj).is_some() {
            return obj;
        }
    }
    unsafe { *(obj as *const u64) }
}

/// `BOOL objc_opt_isKindOfClass(id obj, Class cls)` — fast path for `[obj isKindOfClass:]`
pub extern "C" fn shim_objc_opt_is_kind_of_class(obj: u64, cls: u64) -> u8 {
    if obj == 0 || cls == 0 {
        return 0;
    }
    let isa = unsafe { *(obj as *const u64) };

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let mut current = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current {
        if depth > 64 {
            break;
        }
        if addr == cls {
            return 1;
        }
        if let Some(class_info) = rt.get_class_by_addr(addr) {
            current = class_info.superclass_addr;
        } else {
            break;
        }
        depth += 1;
    }
    0
}

/// `BOOL objc_opt_respondsToSelector(id obj, SEL sel)`
pub extern "C" fn shim_objc_opt_responds_to_selector(obj: u64, sel: u64) -> u8 {
    if obj == 0 || sel == 0 {
        return 0;
    }
    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    if rt.msg_lookup(isa, sel_str).is_some() {
        1
    } else {
        0
    }
}

/// `Class objc_lookUpClass(const char *name)`
pub extern "C" fn shim_objc_lookup_class(name: *const c_char) -> u64 {
    shim_objc_get_class(name)
}

/// `BOOL sel_isEqual(SEL lhs, SEL rhs)`
pub extern "C" fn shim_sel_is_equal(lhs: u64, rhs: u64) -> u8 {
    if lhs == rhs {
        return 1;
    }
    if lhs == 0 || rhs == 0 {
        return 0;
    }
    let a = unsafe { CStr::from_ptr(lhs as *const c_char) };
    let b = unsafe { CStr::from_ptr(rhs as *const c_char) };
    if a == b {
        1
    } else {
        0
    }
}

/// `const char *sel_getName(SEL sel)`
pub extern "C" fn shim_sel_get_name(sel: u64) -> u64 {
    sel
}

// ── Class enumeration ──────────────────────────────────────────────

/// `int objc_getClassList(Class *buffer, int bufferCount)`
///
/// If buffer is null, returns the total number of registered classes.
/// If buffer is non-null, fills up to bufferCount class pointers and
/// returns the total number of registered classes.
pub extern "C" fn shim_objc_get_class_list(buffer: *mut u64, buffer_count: i32) -> i32 {
    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    let addrs: Vec<u64> = rt
        .classes_by_name_iter()
        .map(|(_, c)| c.mapped_addr)
        .collect();
    let total = addrs.len() as i32;
    if !buffer.is_null() && buffer_count > 0 {
        let n = std::cmp::min(buffer_count as usize, addrs.len());
        unsafe {
            std::ptr::copy_nonoverlapping(addrs.as_ptr(), buffer, n);
        }
    }
    total
}

/// `Class *objc_copyClassList(unsigned int *outCount)`
///
/// Allocates and returns a NULL-terminated array of all registered class
/// pointers. Caller is responsible for freeing the returned array.
pub extern "C" fn shim_objc_copy_class_list(out_count: *mut u32) -> *mut u64 {
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
        return std::ptr::null_mut();
    };
    let addrs: Vec<u64> = rt
        .classes_by_name_iter()
        .map(|(_, c)| c.mapped_addr)
        .collect();
    let count = addrs.len();
    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }
    if count == 0 {
        return std::ptr::null_mut();
    }
    // Allocate NULL-terminated array
    let array = crate::pal_alloc().calloc(count + 1, 8) as *mut u64;
    if array.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(addrs.as_ptr(), array, count);
        // NULL terminator already set by calloc
    }
    array
}

/// `const char *class_getImageName(Class cls)`
///
/// Returns the name of the image (dylib) a class came from.
/// For synthetic/dynamic classes, returns a static "cf-lite" string.
/// Returns NULL for unknown classes.
pub extern "C" fn shim_class_get_image_name(cls: u64) -> *const u8 {
    static CF_LITE: &[u8] = b"cf-lite\0";
    if cls == 0 {
        return std::ptr::null();
    }
    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return std::ptr::null();
    };
    if rt.get_class_by_addr(cls).is_some() {
        CF_LITE.as_ptr()
    } else {
        std::ptr::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ObjcRuntime, OBJC_RUNTIME};
    use std::ffi::CString;

    fn with_runtime<F: FnOnce(&mut ObjcRuntime)>(f: F) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        f(rt);
    }

    #[test]
    fn get_class_null_returns_zero() {
        assert_eq!(shim_objc_get_class(std::ptr::null()), 0);
    }

    #[test]
    fn get_class_empty_returns_zero() {
        with_runtime(|_| {});
        let cstr = CString::new("").unwrap();
        let result = shim_objc_get_class(cstr.as_ptr());
        assert_eq!(result, 0);
    }

    #[test]
    fn get_class_nonexistent() {
        with_runtime(|_| {});
        let cstr = CString::new("NonExistentClass").unwrap();
        let result = shim_objc_get_class(cstr.as_ptr());
        assert_eq!(result, 0);
    }

    #[test]
    fn get_class_found() {
        use macho_objc::{ObjcClassRef, ObjcMetadata, ObjcPresence};

        with_runtime(|rt| {
            let metadata = ObjcMetadata {
                presence: ObjcPresence {
                    has_objc: true,
                    sections_found: vec!["__objc_classlist".to_string()],
                },
                class_refs: vec![ObjcClassRef {
                    pointer_addr: 0x1000,
                    class_data_addr: 0x4000,
                    name: Some("TestRuntimeClass".to_string()),
                    methods: vec![],
                    class_methods: vec![],
                    ivars: vec![],
                    properties: vec![],
                    protocol_names: vec![],
                    instance_size: 16,
                    ro_flags: 0x02,
                    superclass_addr: 0,
                    metaclass_addr: 0,
                }],
                categories: vec![],
                protocols: vec![],
                category_count: 0,
                protocol_count: 0,
                selectors: vec![],
                selref_addrs: vec![],
                image_info: None,
            };
            rt.register_image(&metadata, 0);
        });

        let cstr = CString::new("TestRuntimeClass").unwrap();
        let result = shim_objc_get_class(cstr.as_ptr());
        assert_ne!(result, 0);
        assert_eq!(result, 0x4000);
    }

    #[test]
    fn sel_register_name_null() {
        assert_eq!(shim_sel_register_name(std::ptr::null()), 0);
    }

    #[test]
    fn sel_register_name_valid() {
        with_runtime(|rt| {
            rt.intern_selector("testSelector:");
        });
        let cstr = CString::new("testSelector:").unwrap();
        let result = shim_sel_register_name(cstr.as_ptr());
        assert_ne!(result, 0);
    }

    #[test]
    fn sel_is_equal_same() {
        with_runtime(|rt| {
            rt.intern_selector("sameSelector");
        });
        let cstr = CString::new("sameSelector").unwrap();
        let ptr = shim_sel_register_name(cstr.as_ptr());
        assert_eq!(shim_sel_is_equal(ptr, ptr), 1);
    }

    #[test]
    fn sel_is_equal_different() {
        with_runtime(|rt| {
            rt.intern_selector("selA");
            rt.intern_selector("selB");
        });
        let cstr_a = CString::new("selA").unwrap();
        let cstr_b = CString::new("selB").unwrap();
        let ptr_a = shim_sel_register_name(cstr_a.as_ptr());
        let ptr_b = shim_sel_register_name(cstr_b.as_ptr());
        assert_eq!(shim_sel_is_equal(ptr_a, ptr_b), 0);
    }

    #[test]
    fn opt_self_passthrough() {
        assert_eq!(shim_objc_opt_self(0xBEEF), 0xBEEF);
    }

    #[test]
    fn objc_alloc_with_zone_ignores_zone() {
        use crate::class::ClassInfo;
        use std::collections::HashMap;

        with_runtime(|rt| {
            let mut info = ClassInfo {
                name: "AllocWithZoneTest".to_string(),
                mapped_addr: 0xC105_0000,
                methods: vec![],
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
        });

        let obj = shim_objc_alloc_with_zone(0xC105_0000, 0xDEAD_BEEF);
        assert_ne!(obj, 0);
        assert_eq!(unsafe { *(obj as *const u64) }, 0xC105_0000);
        crate::pal_alloc().free(obj as *mut u8);
    }

    #[test]
    fn opt_self_null() {
        assert_eq!(shim_objc_opt_self(0), 0);
    }

    #[test]
    fn class_get_name_null() {
        let result = shim_class_get_name(0);
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "");
    }

    #[test]
    fn lookup_class_null() {
        assert_eq!(shim_objc_lookup_class(std::ptr::null()), 0);
    }

    // ── objc_getClassList tests ────────────────────────────────────

    #[test]
    fn get_class_list_returns_count() {
        use crate::class::ClassInfo;
        use std::collections::HashMap;
        with_runtime(|rt| {
            let mut info = ClassInfo {
                name: "ClassListTestA".to_string(),
                mapped_addr: 0xC101_0000,
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
        });
        let count = shim_objc_get_class_list(std::ptr::null_mut(), 0);
        assert!(count > 0, "should have at least 1 class");
    }

    #[test]
    fn get_class_list_fills_buffer() {
        use crate::class::ClassInfo;
        use std::collections::HashMap;
        with_runtime(|rt| {
            for i in 0..3u64 {
                let mut info = ClassInfo {
                    name: format!("ClassListTestB{}", i),
                    mapped_addr: 0xC102_0000 + i * 0x1000,
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
        });
        let mut buffer = [0u64; 16];
        let total = shim_objc_get_class_list(buffer.as_mut_ptr(), 16);
        assert!(total >= 3, "should have at least 3 classes");
        // At least some entries should be non-zero
        let filled = buffer.iter().filter(|&&v| v != 0).count();
        assert!(
            filled >= 3,
            "buffer should have at least 3 non-zero entries"
        );
    }

    #[test]
    fn get_class_list_null_buffer_no_crash() {
        with_runtime(|_| {});
        let count = shim_objc_get_class_list(std::ptr::null_mut(), 100);
        assert!(count >= 0);
    }

    // ── objc_copyClassList tests ───────────────────────────────────

    #[test]
    fn copy_class_list_allocates() {
        use crate::class::ClassInfo;
        use std::collections::HashMap;
        with_runtime(|rt| {
            let mut info = ClassInfo {
                name: "CopyClassListTestA".to_string(),
                mapped_addr: 0xC103_0000,
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
        });
        let mut count: u32 = 0;
        let ptr = shim_objc_copy_class_list(&mut count as *mut u32);
        assert!(!ptr.is_null(), "should return non-null array");
        assert!(count > 0, "should report at least 1 class");
        // Verify the array contents are non-zero
        let first = unsafe { *ptr };
        assert_ne!(first, 0, "first entry should be a valid class addr");
    }

    // ── class_getImageName tests ───────────────────────────────────

    #[test]
    fn class_get_image_name_null() {
        assert!(shim_class_get_image_name(0).is_null());
    }

    #[test]
    fn class_get_image_name_unknown() {
        with_runtime(|_| {});
        assert!(shim_class_get_image_name(0xDEAD_BEEF).is_null());
    }

    #[test]
    fn class_get_image_name_known() {
        use crate::class::ClassInfo;
        use std::collections::HashMap;
        with_runtime(|rt| {
            let mut info = ClassInfo {
                name: "ImageNameTest".to_string(),
                mapped_addr: 0xC104_0000,
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
        });
        let ptr = shim_class_get_image_name(0xC104_0000);
        assert!(!ptr.is_null());
        let s = unsafe { CStr::from_ptr(ptr as *const std::ffi::c_char) }
            .to_str()
            .unwrap();
        assert_eq!(s, "cf-lite");
    }
}
