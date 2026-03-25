//! NSObject instance/class method shims.

/// `-[NSObject init]` — returns self unchanged.
pub extern "C" fn shim_nsobject_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSObject self]` — returns self.
pub extern "C" fn shim_nsobject_self(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSObject class]` — returns the ISA pointer (the class).
pub extern "C" fn shim_nsobject_class(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

/// `-[NSObject respondsToSelector:]`
pub extern "C" fn shim_nsobject_responds_to_selector(obj: u64, _cmd: u64, sel: u64) -> u8 {
    if obj == 0 || sel == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_cstr = unsafe { std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char) };
    let sel_str = sel_cstr.to_str().unwrap_or("");

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
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

/// `-[NSObject isKindOfClass:]` — walks the isa/superclass chain.
pub extern "C" fn shim_nsobject_is_kind_of_class(obj: u64, _cmd: u64, cls: u64) -> u8 {
    if obj == 0 || cls == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
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

/// `-[NSObject conformsToProtocol:]` — checks protocol conformance via runtime.
pub extern "C" fn shim_nsobject_conforms_to_protocol(obj: u64, _cmd: u64, protocol: u64) -> u8 {
    if obj == 0 || protocol == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };

    // Delegate to the class_conformsToProtocol shim
    objc_runtime::shim_class_conforms_to_protocol(isa, protocol)
}

// ── NSObject class method shims ────────────────────────────────────

/// `+[NSObject alloc]` — allocates a new instance.
pub extern "C" fn shim_nsobject_class_alloc(cls: u64, _cmd: u64) -> u64 {
    if cls == 0 {
        return 0;
    }

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let Some(class_info) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    let size = std::cmp::max(class_info.instance_size as usize, 8);
    let ptr = crate::pal_alloc().calloc(1, size) as u64;
    if ptr == 0 {
        return 0;
    }

    unsafe {
        *(ptr as *mut u64) = cls;
    }
    ptr
}

/// `+[NSObject new]` — alloc + init.
pub extern "C" fn shim_nsobject_class_new(cls: u64, cmd: u64) -> u64 {
    shim_nsobject_class_alloc(cls, cmd)
}

// ── NSObject forwarding methods ────────────────────────────────────

/// `-[NSObject methodSignatureForSelector:]`
pub extern "C" fn shim_nsobject_method_signature_for_selector(
    obj: u64,
    _cmd: u64,
    sel: u64,
) -> u64 {
    if obj == 0 || sel == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };

    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    if let Some(entry) = rt.lookup_method_entry(isa, sel_str) {
        let encoding = entry.type_encoding.clone();
        drop(guard);
        crate::nsmethodsignature::create_method_signature(&encoding)
    } else {
        0
    }
}

/// `+[NSObject methodSignatureForSelector:]` (class method variant)
pub extern "C" fn shim_nsobject_class_method_signature_for_selector(
    cls: u64,
    _cmd: u64,
    sel: u64,
) -> u64 {
    shim_nsobject_method_signature_for_selector(cls, _cmd, sel)
}

/// `-[NSObject forwardInvocation:]`
pub extern "C" fn shim_nsobject_forward_invocation(obj: u64, _cmd: u64, invocation: u64) {
    if obj == 0 || invocation == 0 {
        return;
    }
    let sel_ptr = unsafe { *((invocation + 24) as *const u64) };
    let sel_str = if sel_ptr != 0 {
        unsafe {
            std::ffi::CStr::from_ptr(sel_ptr as *const std::ffi::c_char)
                .to_str()
                .unwrap_or("<unknown>")
        }
    } else {
        "<unknown>"
    };
    eprintln!(
        "-[NSObject forwardInvocation:]: unrecognized selector '{}' sent to instance {:#x}",
        sel_str, obj
    );
}

/// `-[NSObject doesNotRecognizeSelector:]`
///
/// Throws an `NSInvalidArgumentException` (matching Apple's behaviour).
pub extern "C" fn shim_nsobject_does_not_recognize_selector(obj: u64, _cmd: u64, sel: u64) {
    let sel_str = if sel != 0 {
        unsafe {
            std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
                .to_str()
                .unwrap_or("<unknown>")
        }
    } else {
        "<unknown>"
    };

    // Resolve the class name for the error message.
    let class_name = {
        let guard = match objc_runtime::OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard
            .as_ref()
            .and_then(|rt| {
                let isa = unsafe { *(obj as *const u64) } & 0x007ffffffffffff8;
                rt.get_class_by_addr(isa).map(|c| c.name.clone())
            })
            .unwrap_or_else(|| format!("{:#x}", obj))
    };

    let reason = format!(
        "-[{} {}]: unrecognized selector sent to instance {:#x}",
        class_name, sel_str, obj
    );
    log::error!("{}", reason);

    // Build an NSException and throw it.
    let name_str = crate::cfstring::alloc_nsstring_from_cstr(
        c"NSInvalidArgumentException".as_ptr(),
    );
    let reason_cstr = std::ffi::CString::new(reason).unwrap_or_default();
    let reason_str = crate::cfstring::alloc_nsstring_from_cstr(reason_cstr.as_ptr());

    let exc = crate::nsexception::shim_nsexception_exception_with_name_reason_userinfo(
        crate::nsobj::nsexception_class_addr(),
        0,
        name_str,
        reason_str,
        0,
    );

    objc_runtime::shim_objc_exception_throw(exc);
}

/// `+[NSObject resolveInstanceMethod:]` — default returns NO.
pub extern "C" fn shim_nsobject_resolve_instance_method(
    _cls: u64,
    _cmd: u64,
    _sel: u64,
) -> u8 {
    0
}

/// `+[NSObject resolveClassMethod:]` — default returns NO.
pub extern "C" fn shim_nsobject_resolve_class_method(
    _cls: u64,
    _cmd: u64,
    _sel: u64,
) -> u8 {
    0
}
