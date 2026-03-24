//! ARC lifecycle, runtime queries, and introspection shims.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

// ── Runtime query shims ────────────────────────────────────────────

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
        return 0;
    };

    let size = std::cmp::max(class_info.instance_size as usize, 8) + extra_bytes as usize;
    let ptr = unsafe { libc::calloc(1, size) } as u64;
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
    // Not interned — return the input pointer (valid C string).
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

// ── ARC / memory management shims ──────────────────────────────────

/// `void *objc_autoreleasePoolPush(void)`
pub extern "C" fn shim_objc_autorelease_pool_push() -> u64 {
    crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().push() as u64)
}

/// `void objc_autoreleasePoolPop(void *token)`
pub extern "C" fn shim_objc_autorelease_pool_pop(token: u64) {
    let drained =
        crate::refcount::AUTORELEASE_POOLS.with(|pools| pools.borrow_mut().pop(token as usize));
    let mut refcounts = crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for obj in drained {
        if obj != 0 {
            let remaining = refcounts.release(obj);
            if remaining == 0 {
                log::debug!("autorelease pool: deallocating {:#x}", obj);
                unsafe { libc::free(obj as *mut libc::c_void) };
            }
        }
    }
}

/// `id objc_retain(id obj)`
pub extern "C" fn shim_objc_retain(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(obj);
    obj
}

/// `void objc_release(id obj)`
pub extern "C" fn shim_objc_release(obj: u64) {
    if obj == 0 {
        return;
    }
    let remaining = crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release(obj);
    if remaining == 0 {
        log::debug!("objc_release: deallocating {:#x}", obj);
        unsafe { libc::free(obj as *mut libc::c_void) };
    }
}

/// `void objc_setProperty_nonatomic(id self, SEL _cmd, id newValue, ptrdiff_t offset)`
///
/// Used by compiler-generated setters for `@property (nonatomic, retain)`.
pub extern "C" fn shim_objc_set_property_nonatomic(
    obj: u64,
    _sel: u64,
    new_value: u64,
    offset: isize,
) {
    if obj == 0 {
        return;
    }
    unsafe {
        let slot = (obj as *mut u8).offset(offset) as *mut u64;
        *slot = new_value;
    }
}

/// `void objc_storeStrong(id *location, id obj)`
pub extern "C" fn shim_objc_store_strong(location: *mut u64, obj: u64) {
    if location.is_null() {
        return;
    }
    let old = unsafe { *location };
    if old == obj {
        return;
    }
    if obj != 0 {
        shim_objc_retain(obj);
    }
    unsafe {
        *location = obj;
    }
    if old != 0 {
        shim_objc_release(old);
    }
}

/// `id objc_alloc(Class cls)`
pub extern "C" fn shim_objc_alloc(cls: u64) -> u64 {
    shim_class_create_instance(cls, 0)
}

/// `id objc_alloc_init(Class cls)`
pub extern "C" fn shim_objc_alloc_init(cls: u64) -> u64 {
    shim_class_create_instance(cls, 0)
}

/// `id objc_opt_self(id obj)`
pub extern "C" fn shim_objc_opt_self(obj: u64) -> u64 {
    obj
}

/// `Class objc_opt_class(id obj)`
///
/// Compiler-emitted fast path for `[obj class]`.
/// For instances, returns the ISA (class of the object).
/// For class objects, returns the class itself (i.e., `+[NSObject class]` returns `self`).
pub extern "C" fn shim_objc_opt_class(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Check if obj itself is a registered class — if so, return it directly
    // (this matches +[NSObject class] which returns self for class objects).
    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        if rt.get_class_by_addr(obj).is_some() {
            return obj;
        }
    }
    // For instances, return the ISA
    unsafe { *(obj as *const u64) }
}

/// `BOOL objc_opt_respondsToSelector(id obj, SEL sel)`
///
/// Compiler-emitted fast path for `[obj respondsToSelector:]`.
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

/// `id objc_autorelease(id obj)`
pub extern "C" fn shim_objc_autorelease(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::AUTORELEASE_POOLS.with(|pools| {
        pools.borrow_mut().autorelease(obj);
    });
    obj
}

// ── Introspection shims ────────────────────────────────────────────

/// `Class object_getClass(id obj)`
pub extern "C" fn shim_object_get_class(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

/// `Class objc_lookUpClass(const char *name)`
pub extern "C" fn shim_objc_lookup_class(name: *const c_char) -> u64 {
    shim_objc_get_class(name)
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

// ── ARC return-value optimization shims ────────────────────────────
//
// In real ARC, `objc_autoreleaseReturnValue` + `objc_retainAutoreleasedReturnValue`
// cooperate to skip the autorelease pool entirely when the caller immediately
// retains the returned value. We simplify: autorelease actually adds to the pool,
// and retainAutoreleasedReturnValue does a real retain. This is correct but slower
// than Apple's thread-local fast path — fine for our single-threaded runtime.

/// `id objc_autoreleaseReturnValue(id obj)` — autorelease for a return value.
pub extern "C" fn shim_objc_autorelease_return_value(obj: u64) -> u64 {
    shim_objc_autorelease(obj)
}

/// `id objc_retainAutoreleasedReturnValue(id obj)` — retain a just-returned value.
pub extern "C" fn shim_objc_retain_autoreleased_return_value(obj: u64) -> u64 {
    shim_objc_retain(obj)
}

/// `id objc_unsafeClaimAutoreleasedReturnValue(id obj)` — claim without retain (MRR fast path).
pub extern "C" fn shim_objc_unsafe_claim_autoreleased_return_value(obj: u64) -> u64 {
    // In Apple's impl this is a no-op that prevents duplicate autorelease.
    // We just return obj — the caller takes ownership of the +1.
    obj
}

/// `id objc_retainAutorelease(id obj)` — retain + autorelease combined.
pub extern "C" fn shim_objc_retain_autorelease(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    shim_objc_retain(obj);
    shim_objc_autorelease(obj)
}

// ── NSObject memory-management instance method shims ───────────────
//
// These are dispatched via objc_msgSend when MRC code calls
// [obj retain], [obj release], [obj autorelease], [obj retainCount],
// or [obj dealloc].

/// `-[NSObject retain]`
pub extern "C" fn shim_nsobject_retain(obj: u64, _cmd: u64) -> u64 {
    shim_objc_retain(obj)
}

/// `-[NSObject release]`
pub extern "C" fn shim_nsobject_release(obj: u64, _cmd: u64) {
    shim_objc_release(obj);
}

/// `-[NSObject autorelease]`
pub extern "C" fn shim_nsobject_autorelease(obj: u64, _cmd: u64) -> u64 {
    shim_objc_autorelease(obj)
}

/// `-[NSObject retainCount]`
pub extern "C" fn shim_nsobject_retain_count(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .count(obj) as u64
}

/// `-[NSObject dealloc]` — no-op; actual free happens in objc_release.
pub extern "C" fn shim_nsobject_dealloc(_obj: u64, _cmd: u64) {
    // Dealloc is a no-op in our runtime. The actual `libc::free` is
    // done by `shim_objc_release` when the refcount reaches zero.
    // Subclasses that need custom teardown aren't supported yet.
}

// ── Runtime introspection / mutation shims ─────────────────────────

/// Fake `Method` struct for `class_copyMethodList` / `class_getInstanceMethod`.
///
/// Layout:
///   offset  0: selector_ptr  (u64 — pointer to interned SEL C string)
///   offset  8: type_encoding  (u64 — pointer to owned type encoding C string)
///   offset 16: imp            (u64 — function pointer)
///
/// Total: 24 bytes per Method.
const METHOD_STRUCT_SIZE: usize = 24;

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
        class.methods.push(crate::MethodEntry {
            selector: sel_str.to_string(),
            imp,
            type_encoding: type_enc,
        });
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
/// in the class. Returns the old IMP. The fake Method struct (heap-allocated
/// snapshot) is updated directly; the real `MethodEntry` in the class Vec
/// is also updated so dispatch sees the change immediately.
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
    // We find entries by selector name and current IMP matching the OLD value.
    // Important: do both swaps in one pass to avoid the second search
    // accidentally finding the entry we just modified in step 1.
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
    let ptr = unsafe { libc::calloc(1, METHOD_STRUCT_SIZE) } as u64;
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
    let array = unsafe { libc::calloc(count + 1, 8) } as u64;
    if array == 0 {
        return 0;
    }

    for (i, m) in methods.iter().enumerate() {
        let method_ptr = unsafe { libc::calloc(1, METHOD_STRUCT_SIZE) } as u64;
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

/// `const char *sel_getName(SEL sel)` — returns the selector C string.
pub extern "C" fn shim_sel_get_name(sel: u64) -> u64 {
    // SEL is already a C string pointer in our runtime
    sel
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

// ── Ivar introspection shims ───────────────────────────────────────

/// Fake Ivar struct layout (32 bytes):
///   offset  0: name_ptr          (u64 — pointer to C string)
///   offset  8: type_encoding_ptr (u64 — pointer to C string)
///   offset 16: ivar_offset       (i64 — ptrdiff_t)
///   offset 24: size              (u32)
const IVAR_STRUCT_SIZE: usize = 32;

/// Fake Property struct layout (16 bytes):
///   offset  0: name_ptr       (u64 — pointer to C string)
///   offset  8: attributes_ptr (u64 — pointer to C string)
const PROPERTY_STRUCT_SIZE: usize = 16;

fn allocate_fake_ivar(name: &str, type_enc: &str, offset: u32, size: u32) -> u64 {
    let ptr = unsafe { libc::calloc(1, IVAR_STRUCT_SIZE) } as u64;
    if ptr == 0 {
        return 0;
    }

    let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
    let type_cstr = std::ffi::CString::new(type_enc).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = name_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = type_cstr.into_raw() as u64;
        *((ptr as usize + 16) as *mut i64) = offset as i64;
        *((ptr as usize + 24) as *mut u32) = size;
    }
    ptr
}

fn allocate_fake_property(name: &str, attributes: &str) -> u64 {
    let ptr = unsafe { libc::calloc(1, PROPERTY_STRUCT_SIZE) } as u64;
    if ptr == 0 {
        return 0;
    }

    let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
    let attr_cstr = std::ffi::CString::new(attributes).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = name_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = attr_cstr.into_raw() as u64;
    }
    ptr
}

/// `Ivar *class_copyIvarList(Class cls, unsigned int *outCount)`
pub extern "C" fn shim_class_copy_ivar_list(cls: u64, out_count: *mut u32) -> u64 {
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

    let count = class.ivars.len();
    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }
    if count == 0 {
        return 0;
    }

    // Allocate NULL-terminated array of Ivar pointers
    let array_size = (count + 1) * std::mem::size_of::<u64>();
    let array_ptr = unsafe { libc::calloc(1, array_size) } as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, ivar) in class.ivars.iter().enumerate() {
        let fake = allocate_fake_ivar(&ivar.name, &ivar.type_encoding, ivar.offset, ivar.size);
        unsafe {
            *((array_ptr as usize + i * 8) as *mut u64) = fake;
        }
    }
    array_ptr
}

/// `Ivar class_getInstanceVariable(Class cls, const char *name)`
pub extern "C" fn shim_class_get_instance_variable(cls: u64, name: u64) -> u64 {
    if cls == 0 || name == 0 {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    let Some(class) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    class
        .ivars
        .iter()
        .find(|iv| iv.name == name_str)
        .map(|iv| allocate_fake_ivar(&iv.name, &iv.type_encoding, iv.offset, iv.size))
        .unwrap_or(0)
}

/// `const char *ivar_getName(Ivar ivar)`
pub extern "C" fn shim_ivar_get_name(ivar: u64) -> u64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *(ivar as *const u64) }
}

/// `const char *ivar_getTypeEncoding(Ivar ivar)`
pub extern "C" fn shim_ivar_get_type_encoding(ivar: u64) -> u64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *((ivar as usize + 8) as *const u64) }
}

/// `ptrdiff_t ivar_getOffset(Ivar ivar)`
pub extern "C" fn shim_ivar_get_offset(ivar: u64) -> i64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *((ivar as usize + 16) as *const i64) }
}

/// `objc_property_t *class_copyPropertyList(Class cls, unsigned int *outCount)`
pub extern "C" fn shim_class_copy_property_list(cls: u64, out_count: *mut u32) -> u64 {
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

    let count = class.properties.len();
    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }
    if count == 0 {
        return 0;
    }

    let array_size = (count + 1) * std::mem::size_of::<u64>();
    let array_ptr = unsafe { libc::calloc(1, array_size) } as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, prop) in class.properties.iter().enumerate() {
        let fake = allocate_fake_property(&prop.name, &prop.attributes);
        unsafe {
            *((array_ptr as usize + i * 8) as *mut u64) = fake;
        }
    }
    array_ptr
}

/// `objc_property_t class_getProperty(Class cls, const char *name)`
pub extern "C" fn shim_class_get_property(cls: u64, name: u64) -> u64 {
    if cls == 0 || name == 0 {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    let Some(class) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    class
        .properties
        .iter()
        .find(|p| p.name == name_str)
        .map(|p| allocate_fake_property(&p.name, &p.attributes))
        .unwrap_or(0)
}

/// `const char *property_getName(objc_property_t property)`
pub extern "C" fn shim_property_get_name(property: u64) -> u64 {
    if property == 0 {
        return 0;
    }
    unsafe { *(property as *const u64) }
}

/// `const char *property_getAttributes(objc_property_t property)`
pub extern "C" fn shim_property_get_attributes(property: u64) -> u64 {
    if property == 0 {
        return 0;
    }
    unsafe { *((property as usize + 8) as *const u64) }
}

/// `id object_getIvar(id obj, Ivar ivar)`
///
/// Reads the pointer-sized value at the ivar's offset within the object.
pub extern "C" fn shim_object_get_ivar(obj: u64, ivar: u64) -> u64 {
    if obj == 0 || ivar == 0 {
        return 0;
    }
    let offset = unsafe { *((ivar as usize + 16) as *const i64) };
    if offset < 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset as usize) as *const u64) }
}

/// `void object_setIvar(id obj, Ivar ivar, id value)`
///
/// Writes a pointer-sized value at the ivar's offset within the object.
pub extern "C" fn shim_object_set_ivar(obj: u64, ivar: u64, value: u64) {
    if obj == 0 || ivar == 0 {
        return;
    }
    let offset = unsafe { *((ivar as usize + 16) as *const i64) };
    if offset < 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset as usize) as *mut u64) = value;
    }
}

// ── Exception handling shims ───────────────────────────────────────

/// Global storage for the current in-flight exception thrown via
/// `objc_exception_throw`. Single-threaded — no locking needed.
static CURRENT_EXCEPTION: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

/// `void objc_exception_throw(id exception)`
///
/// Throws an ObjC exception. In our single-threaded runtime, we delegate
/// to the host libobjc's `objc_exception_throw` which uses the C++ unwind
/// mechanism. The `__objc_personality_v0` (already in StubResolver safe-list)
/// handles @catch matching during stack unwinding.
///
/// ## Safety
/// Calls the host's `objc_exception_throw` via dlsym, which will `__cxa_throw`
/// under the hood and unwind through the mapped binary's stack frames.
///
/// Because our cf-lite NSException objects use a heap-allocated ISA pointer
/// that doesn't conform to the host runtime's non-pointer ISA format (arm64),
/// we must create a real host NSException before passing it to the host's
/// `objc_exception_throw`. The host function calls `object_getClass` which
/// applies the ISA_MASK — our raw pointer ISA would be garbled, causing a
/// SIGSEGV.
pub extern "C" fn shim_objc_exception_throw(exception: u64) {
    log::debug!("shim_objc_exception_throw: exception={:#x}", exception);
    // Store for potential inspection
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = Some(exception);

    // Log the exception details
    log::error!("objc_exception_throw: exception={:#x}", exception);

    // Attempt to delegate to host libobjc
    #[cfg(target_os = "macos")]
    {
        type ThrowFn = unsafe extern "C" fn(u64);
        let throw_sym = unsafe {
            libc::dlsym(
                (-2isize) as *mut libc::c_void,
                c"objc_exception_throw".as_ptr(),
            )
        };
        if throw_sym.is_null() {
            eprintln!("*** Terminating: could not find host objc_exception_throw");
            std::process::abort();
        }

        // Try to create a real host NSException from our cf-lite object.
        // Our cf-lite NSException layout: [isa(8), name(8), reason(8), userInfo(8)]
        let host_exc = create_host_nsexception(exception);

        let throw_fn = unsafe { std::mem::transmute::<*mut libc::c_void, ThrowFn>(throw_sym) };
        unsafe {
            throw_fn(host_exc);
        }
        // objc_exception_throw never returns
        std::process::abort();
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback: abort with message if not on macOS
        eprintln!(
            "*** Terminating app due to uncaught exception (object {:#x})",
            exception
        );
        std::process::abort();
    }
}

/// Create a host NSException from a cf-lite exception object.
///
/// Reads name, reason, and userInfo from the cf-lite layout, then calls
/// the host's Foundation to create a real NSException with proper ISA.
///
/// Falls back to returning the original object if host creation fails
/// (e.g., if the object isn't a cf-lite NSException or host APIs aren't
/// available).
#[cfg(target_os = "macos")]
fn create_host_nsexception(cf_lite_exc: u64) -> u64 {
    if cf_lite_exc == 0 {
        return 0;
    }

    // Read cf-lite NSException fields:
    //   offset 0: isa (u64)
    //   offset 8: name (u64 — CFString pointer)
    //   offset 16: reason (u64 — CFString pointer, or 0)
    //   offset 24: userInfo (u64 — NSDictionary pointer, or 0)
    let name_cfstr = unsafe { *((cf_lite_exc as usize + 8) as *const u64) };
    let reason_cfstr = unsafe { *((cf_lite_exc as usize + 16) as *const u64) };

    // Resolve host APIs via dlsym
    let rtld = (-2isize) as *mut libc::c_void;
    let objc_get_class = unsafe { libc::dlsym(rtld, c"objc_getClass".as_ptr()) };
    let sel_register = unsafe { libc::dlsym(rtld, c"sel_registerName".as_ptr()) };
    let msg_send = unsafe { libc::dlsym(rtld, c"objc_msgSend".as_ptr()) };

    if objc_get_class.is_null() || sel_register.is_null() || msg_send.is_null() {
        return cf_lite_exc;
    }

    type GetClassFn = unsafe extern "C" fn(*const i8) -> u64;
    type SelRegFn = unsafe extern "C" fn(*const i8) -> u64;
    type MsgSend5Fn = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
    type MsgSend3PtrFn = unsafe extern "C" fn(u64, u64, *const i8) -> u64;

    let get_class: GetClassFn = unsafe { std::mem::transmute(objc_get_class) };
    let sel_reg: SelRegFn = unsafe { std::mem::transmute(sel_register) };
    let send5: MsgSend5Fn = unsafe { std::mem::transmute(msg_send) };
    let send3ptr: MsgSend3PtrFn = unsafe { std::mem::transmute(msg_send) };

    // Convert cf-lite CFStrings to host NSStrings. Our cf-lite CFStrings
    // use a fake ___CFConstantStringClassReference that the host runtime
    // can't handle (non-pointer ISA masking produces garbage on arm64).
    // Extract the raw C string data pointer and create real host NSStrings.
    let name_host = cfstring_to_host_nsstring(name_cfstr, get_class, sel_reg, send3ptr);
    let reason_host = cfstring_to_host_nsstring(reason_cfstr, get_class, sel_reg, send3ptr);

    let exc_class = unsafe { get_class(c"NSException".as_ptr()) };
    if exc_class == 0 {
        return cf_lite_exc;
    }

    let sel = unsafe { sel_reg(c"exceptionWithName:reason:userInfo:".as_ptr()) };
    let host_exc = unsafe { send5(exc_class, sel, name_host, reason_host, 0) };
    if host_exc == 0 {
        return cf_lite_exc;
    }

    log::debug!("create_host_nsexception: {host_exc:#x}");
    host_exc
}

/// Convert a cf-lite CFString to a host NSString via C string extraction.
///
/// CFString constant layout: [isa(8), flags(8), data_ptr(8), length(8)]
/// We read data_ptr (a raw C string in mapped __TEXT) and call
/// `[NSString stringWithUTF8String:]` on the host.
#[cfg(target_os = "macos")]
fn cfstring_to_host_nsstring(
    cfstr: u64,
    get_class: unsafe extern "C" fn(*const i8) -> u64,
    sel_reg: unsafe extern "C" fn(*const i8) -> u64,
    send3ptr: unsafe extern "C" fn(u64, u64, *const i8) -> u64,
) -> u64 {
    if cfstr == 0 {
        return 0;
    }

    // data_ptr is at offset 16 in the CFString struct
    let data_ptr = unsafe { *((cfstr as usize + 16) as *const *const i8) };
    if data_ptr.is_null() {
        return 0;
    }

    let nsstring_class = unsafe { get_class(c"NSString".as_ptr()) };
    if nsstring_class == 0 {
        return 0;
    }
    let sel = unsafe { sel_reg(c"stringWithUTF8String:".as_ptr()) };
    unsafe { send3ptr(nsstring_class, sel, data_ptr) }
}

/// `id objc_begin_catch(void *exc_buf)`
///
/// Called at the entry of a `@catch` block. Delegates to host libobjc.
pub extern "C" fn shim_objc_begin_catch(exc_buf: u64) -> u64 {
    #[cfg(target_os = "macos")]
    {
        type BeginCatchFn = unsafe extern "C" fn(u64) -> u64;
        let sym =
            unsafe { libc::dlsym((-2isize) as *mut libc::c_void, c"objc_begin_catch".as_ptr()) };
        if !sym.is_null() {
            return unsafe { std::mem::transmute::<*mut libc::c_void, BeginCatchFn>(sym)(exc_buf) };
        }
    }

    // Fallback: return the stored exception pointer
    CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0)
}

/// `void objc_end_catch(void)`
///
/// Called at the exit of a `@catch` block. Delegates to host libobjc.
pub extern "C" fn shim_objc_end_catch() {
    #[cfg(target_os = "macos")]
    {
        type EndCatchFn = unsafe extern "C" fn();
        let sym =
            unsafe { libc::dlsym((-2isize) as *mut libc::c_void, c"objc_end_catch".as_ptr()) };
        if !sym.is_null() {
            unsafe {
                std::mem::transmute::<*mut libc::c_void, EndCatchFn>(sym)();
            }
            return;
        }
    }

    // Fallback: clear the current exception
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// `void objc_exception_rethrow(void)`
///
/// Re-throws the current exception (used by `@finally` blocks).
pub extern "C" fn shim_objc_exception_rethrow() {
    #[cfg(target_os = "macos")]
    {
        type RethrowFn = unsafe extern "C" fn();
        let sym = unsafe {
            libc::dlsym(
                (-2isize) as *mut libc::c_void,
                c"objc_exception_rethrow".as_ptr(),
            )
        };
        if !sym.is_null() {
            unsafe {
                std::mem::transmute::<*mut libc::c_void, RethrowFn>(sym)();
            }
            // rethrow never returns
            std::process::abort();
        }
    }

    eprintln!("*** objc_exception_rethrow: no active exception to rethrow");
    std::process::abort();
}

/// `void objc_terminate(void)`
///
/// Called when an exception propagates out of a `noexcept`/`nothrow` context.
pub extern "C" fn shim_objc_terminate() {
    let exc = CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(obj) = *exc {
        eprintln!(
            "*** Terminating due to uncaught ObjC exception (object {:#x})",
            obj
        );
    } else {
        eprintln!("*** Terminating due to uncaught ObjC exception");
    }
    std::process::abort();
}

// ── @synchronized support ─────────────────────────────────────────

use std::collections::HashMap;

/// Wrapper around a raw mutex pointer that is Send + Sync.
/// SAFETY: pthread mutexes are designed for cross-thread use.
struct SyncMutexPtr(*mut libc::pthread_mutex_t);
unsafe impl Send for SyncMutexPtr {}
unsafe impl Sync for SyncMutexPtr {}

/// Per-object mutex table for `@synchronized(obj) { ... }`.
///
/// Maps object addresses to recursive pthread mutexes. The table itself
/// is protected by a global mutex. Each per-object mutex is recursive
/// to support nested `@synchronized` on the same object from the same
/// thread (which is valid in ObjC).
static SYNC_TABLE: std::sync::Mutex<Option<HashMap<u64, SyncMutexPtr>>> =
    std::sync::Mutex::new(None);

/// Get or create the recursive mutex for a given object address.
fn sync_mutex_for(obj: u64) -> *mut libc::pthread_mutex_t {
    let mut guard = SYNC_TABLE.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    table
        .entry(obj)
        .or_insert_with(|| unsafe {
            let mutex = libc::calloc(1, std::mem::size_of::<libc::pthread_mutex_t>())
                as *mut libc::pthread_mutex_t;
            let mut attr: libc::pthread_mutexattr_t = std::mem::zeroed();
            libc::pthread_mutexattr_init(&mut attr);
            libc::pthread_mutexattr_settype(&mut attr, libc::PTHREAD_MUTEX_RECURSIVE);
            libc::pthread_mutex_init(mutex, &attr);
            libc::pthread_mutexattr_destroy(&mut attr);
            SyncMutexPtr(mutex)
        })
        .0
}

/// `int objc_sync_enter(id obj)` — lock the mutex associated with `obj`.
///
/// If `obj` is nil, this is a no-op (returns success). This matches
/// Apple's runtime behavior where `@synchronized(nil)` is valid but
/// provides no synchronization.
pub extern "C" fn shim_objc_sync_enter(obj: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    let mutex = sync_mutex_for(obj);
    unsafe { libc::pthread_mutex_lock(mutex) }
}

/// `int objc_sync_exit(id obj)` — unlock the mutex associated with `obj`.
pub extern "C" fn shim_objc_sync_exit(obj: u64) -> i32 {
    if obj == 0 {
        return 0;
    }
    let mutex = sync_mutex_for(obj);
    unsafe { libc::pthread_mutex_unlock(mutex) }
}
