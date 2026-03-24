//! NSException shim: exception name + reason + userInfo.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa        (u64 — pointer to NSException class)
//!   offset  8: name       (u64 — pointer to NSString name)
//!   offset 16: reason     (u64 — pointer to NSString reason, or 0)
//!   offset 24: user_info  (u64 — pointer to NSDictionary, or 0)
//!
//! Total: 32 bytes per NSException object.

const NSEXCEPTION_NAME_OFF: usize = 8;
const NSEXCEPTION_REASON_OFF: usize = 16;
const NSEXCEPTION_USERINFO_OFF: usize = 24;

/// `+[NSException exceptionWithName:reason:userInfo:]`
pub extern "C" fn shim_nsexception_exception_with_name_reason_userinfo(
    _cls: u64,
    _cmd: u64,
    name: u64,
    reason: u64,
    user_info: u64,
) -> u64 {
    let obj = unsafe { libc::calloc(1, 32) } as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsexception_class_addr();
        *((obj as usize + NSEXCEPTION_NAME_OFF) as *mut u64) = name;
        *((obj as usize + NSEXCEPTION_REASON_OFF) as *mut u64) = reason;
        *((obj as usize + NSEXCEPTION_USERINFO_OFF) as *mut u64) = user_info;
    }

    log::debug!(
        "NSException exceptionWithName:reason:userInfo: → {:#x}",
        obj
    );
    obj
}

/// `-[NSException initWithName:reason:userInfo:]`
pub extern "C" fn shim_nsexception_init_with_name_reason_userinfo(
    obj: u64,
    _cmd: u64,
    name: u64,
    reason: u64,
    user_info: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    unsafe {
        *((obj as usize + NSEXCEPTION_NAME_OFF) as *mut u64) = name;
        *((obj as usize + NSEXCEPTION_REASON_OFF) as *mut u64) = reason;
        *((obj as usize + NSEXCEPTION_USERINFO_OFF) as *mut u64) = user_info;
    }

    obj
}

/// `-[NSException name]` — returns the exception name NSString.
pub extern "C" fn shim_nsexception_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSEXCEPTION_NAME_OFF) as *const u64) }
}

/// `-[NSException reason]` — returns the reason NSString (or nil).
pub extern "C" fn shim_nsexception_reason(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSEXCEPTION_REASON_OFF) as *const u64) }
}

/// `-[NSException userInfo]` — returns the userInfo dictionary (or nil).
pub extern "C" fn shim_nsexception_user_info(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSEXCEPTION_USERINFO_OFF) as *const u64) }
}

/// `-[NSException description]` — returns a debug string.
pub extern "C" fn shim_nsexception_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let name_ptr = unsafe { *((obj as usize + NSEXCEPTION_NAME_OFF) as *const u64) };
    let reason_ptr = unsafe { *((obj as usize + NSEXCEPTION_REASON_OFF) as *const u64) };

    let name_str = nsstring_to_rust(name_ptr);
    let reason_str = nsstring_to_rust(reason_ptr);

    let desc = if reason_str.is_empty() {
        name_str
    } else {
        format!("{name_str}: {reason_str}")
    };
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSException raise]` — raises (throws) this exception.
///
/// Delegates to `objc_exception_throw` which will use the host
/// C++ exception mechanism.
pub extern "C" fn shim_nsexception_raise(obj: u64, _cmd: u64) {
    objc_runtime::shim_objc_exception_throw(obj);
}

/// Extract a Rust String from an NSString pointer, or "(null)" if nil/invalid.
fn nsstring_to_rust(ptr: u64) -> String {
    if ptr == 0 {
        return "(null)".to_string();
    }
    let utf8 = crate::cfstring::shim_nsstring_utf8string(ptr, 0);
    if utf8.is_null() {
        return "(null)".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(utf8) }
        .to_str()
        .unwrap_or("(null)")
        .to_string()
}
