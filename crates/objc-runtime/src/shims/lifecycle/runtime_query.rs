//! Class/selector lookup and object allocation shims.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

/// `Class objc_getClass(const char *name)`
pub extern "C" fn shim_objc_get_class(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
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

/// `id class_createInstance(Class cls, size_t extraBytes)`
pub extern "C" fn shim_class_create_instance(cls: u64, extra_bytes: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let Some(class_info) = rt.get_class_by_addr(cls) else {
        log::warn!("class_createInstance: unknown class at {:#x}", cls);
        runtime_diagnostics::emit(runtime_diagnostics::DiagEvent::ClassAddrUnknown {
            addr: cls,
        });
        return 0;
    };

    let size = std::cmp::max(class_info.instance_size as usize, 8) + extra_bytes as usize;
    let ptr = crate::pal_alloc().calloc(1, size) as u64;
    if ptr == 0 {
        return 0;
    }

    // SAFETY: ptr is freshly allocated and at least 8 bytes.
    unsafe {
        *(ptr as *mut u64) = cls;
    }
    log::debug!(
        "class_createInstance: allocated {} bytes for '{}' at {:#x}",
        size,
        class_info.name,
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

    let guard = match OBJC_RUNTIME.lock() {
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
pub extern "C" fn shim_class_get_name(cls: u64) -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    if cls == 0 {
        return EMPTY.as_ptr() as *const c_char;
    }

    let guard = match OBJC_RUNTIME.lock() {
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
            EMPTY.as_ptr() as *const c_char
        }
        None => EMPTY.as_ptr() as *const c_char,
    }
}

/// `id objc_alloc(Class cls)`
pub extern "C" fn shim_objc_alloc(cls: u64) -> u64 {
    shim_class_create_instance(cls, 0)
}

/// `id objc_alloc_init(Class cls)`
pub extern "C" fn shim_objc_alloc_init(cls: u64) -> u64 {
    let obj = shim_class_create_instance(cls, 0);
    if obj == 0 {
        return 0;
    }
    let isa = unsafe { *(obj as *const u64) };
    let guard = match OBJC_RUNTIME.lock() {
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

/// `id objc_opt_self(id obj)`
pub extern "C" fn shim_objc_opt_self(obj: u64) -> u64 {
    obj
}

/// `Class objc_opt_class(id obj)`
pub extern "C" fn shim_objc_opt_class(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let guard = match OBJC_RUNTIME.lock() {
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

    let guard = match OBJC_RUNTIME.lock() {
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

    let guard = match OBJC_RUNTIME.lock() {
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
