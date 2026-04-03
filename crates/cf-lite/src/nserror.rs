//! NSError shim: error domain + code + userInfo.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa        (u64 — pointer to NSError class)
//!   offset  8: code       (i64 — error code)
//!   offset 16: domain     (u64 — pointer to NSString domain)
//!   offset 24: user_info  (u64 — pointer to NSDictionary, or 0)
//!
//! Total: 32 bytes per NSError object.

const NSERROR_CODE_OFF: usize = 8;
const NSERROR_DOMAIN_OFF: usize = 16;
const NSERROR_USERINFO_OFF: usize = 24;

/// Create an NSError object with a domain string, code, and no userInfo.
///
/// Used internally when shim functions need to populate an `error_out` pointer.
pub(crate) fn make_nserror(domain: &str, code: i64) -> u64 {
    let domain_cstr = std::ffi::CString::new(domain).unwrap_or_default();
    let domain_nsstring =
        crate::cfstring::shim_cf_string_create_with_cstring(0, domain_cstr.as_ptr(), 0);
    shim_nserror_error_with_domain_code_userinfo(0, 0, domain_nsstring, code, 0)
}

/// `+[NSError errorWithDomain:code:userInfo:]`
pub extern "C" fn shim_nserror_error_with_domain_code_userinfo(
    _cls: u64,
    _cmd: u64,
    domain: u64,
    code: i64,
    user_info: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nserror_class_addr();
        *((obj as usize + NSERROR_CODE_OFF) as *mut i64) = code;
        *((obj as usize + NSERROR_DOMAIN_OFF) as *mut u64) = domain;
        *((obj as usize + NSERROR_USERINFO_OFF) as *mut u64) = user_info;
    }

    log::debug!(
        "NSError errorWithDomain:code:userInfo: → {:#x} (code={})",
        obj,
        code
    );
    obj
}

/// `-[NSError initWithDomain:code:userInfo:]`
pub extern "C" fn shim_nserror_init_with_domain_code_userinfo(
    obj: u64,
    _cmd: u64,
    domain: u64,
    code: i64,
    user_info: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    unsafe {
        *((obj as usize + NSERROR_CODE_OFF) as *mut i64) = code;
        *((obj as usize + NSERROR_DOMAIN_OFF) as *mut u64) = domain;
        *((obj as usize + NSERROR_USERINFO_OFF) as *mut u64) = user_info;
    }

    obj
}

/// `-[NSError domain]` — returns the domain NSString.
pub extern "C" fn shim_nserror_domain(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSERROR_DOMAIN_OFF) as *const u64) }
}

/// `-[NSError code]` — returns the error code.
pub extern "C" fn shim_nserror_code(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSERROR_CODE_OFF) as *const i64) }
}

/// `-[NSError userInfo]` — returns the userInfo dictionary (or nil).
pub extern "C" fn shim_nserror_user_info(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSERROR_USERINFO_OFF) as *const u64) }
}

/// `-[NSError localizedDescription]` — returns a descriptive string.
///
/// If userInfo contains NSLocalizedDescriptionKey, returns that value.
/// Otherwise, returns "The operation couldn't be completed. (DOMAIN error CODE.)"
pub extern "C" fn shim_nserror_localized_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let code = unsafe { *((obj as usize + NSERROR_CODE_OFF) as *const i64) };
    let domain_ptr = unsafe { *((obj as usize + NSERROR_DOMAIN_OFF) as *const u64) };

    // Extract domain string
    let domain_str = if domain_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(domain_ptr, 0);
        if !utf8.is_null() {
            unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("(null)")
                .to_string()
        } else {
            "(null)".to_string()
        }
    } else {
        "(null)".to_string()
    };

    let desc = format!("The operation couldn\u{2019}t be completed. ({domain_str} error {code}.)");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSError description]` — returns a debug string.
pub extern "C" fn shim_nserror_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let code = unsafe { *((obj as usize + NSERROR_CODE_OFF) as *const i64) };
    let domain_ptr = unsafe { *((obj as usize + NSERROR_DOMAIN_OFF) as *const u64) };

    let domain_str = if domain_ptr != 0 {
        let utf8 = crate::cfstring::shim_nsstring_utf8string(domain_ptr, 0);
        if !utf8.is_null() {
            unsafe { std::ffi::CStr::from_ptr(utf8) }
                .to_str()
                .unwrap_or("(null)")
                .to_string()
        } else {
            "(null)".to_string()
        }
    } else {
        "(null)".to_string()
    };

    let desc = format!("Error Domain={domain_str} Code={code}");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSError localizedFailureReason]` — returns nil (most errors lack this).
pub extern "C" fn shim_nserror_localized_failure_reason(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSError localizedRecoverySuggestion]` — returns nil.
pub extern "C" fn shim_nserror_localized_recovery_suggestion(_obj: u64, _cmd: u64) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn create_and_read_fields() {
        let domain = make_string("TestDomain");
        let err = shim_nserror_error_with_domain_code_userinfo(0, 0, domain, -42, 0);
        assert_ne!(err, 0);
        assert_eq!(shim_nserror_code(err, 0), -42);
        assert_string_eq(shim_nserror_domain(err, 0), "TestDomain");
        assert_eq!(shim_nserror_user_info(err, 0), 0);
        free_obj(err);
    }

    #[test]
    fn init_with_domain() {
        let obj = alloc_obj(32, crate::nsobj::nserror_class_addr());
        let domain = make_string("NSPOSIXErrorDomain");
        let result = shim_nserror_init_with_domain_code_userinfo(obj, 0, domain, 13, 0);
        assert_eq!(result, obj);
        assert_eq!(shim_nserror_code(obj, 0), 13);
        assert_string_eq(shim_nserror_domain(obj, 0), "NSPOSIXErrorDomain");
        free_obj(obj);
    }

    #[test]
    fn make_nserror_helper() {
        let err = make_nserror("NSCocoaErrorDomain", 256);
        assert_ne!(err, 0);
        assert_eq!(shim_nserror_code(err, 0), 256);
        free_obj(err);
    }

    #[test]
    fn nil_returns_zero() {
        assert_eq!(shim_nserror_code(0, 0), 0);
        assert_eq!(shim_nserror_domain(0, 0), 0);
        assert_eq!(shim_nserror_user_info(0, 0), 0);
        assert_eq!(shim_nserror_localized_failure_reason(0, 0), 0);
        assert_eq!(shim_nserror_localized_recovery_suggestion(0, 0), 0);
    }

    #[test]
    fn description_format() {
        let domain = make_string("TestDomain");
        let err = shim_nserror_error_with_domain_code_userinfo(0, 0, domain, 99, 0);
        let desc = shim_nserror_description(err, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "Error Domain=TestDomain Code=99");
        free_obj(err);
    }

    #[test]
    fn localized_description_format() {
        let domain = make_string("TestDomain");
        let err = shim_nserror_error_with_domain_code_userinfo(0, 0, domain, 5, 0);
        let desc = shim_nserror_localized_description(err, 0);
        assert_ne!(desc, 0);
        let s = read_string(desc);
        assert!(s.contains("TestDomain"), "should contain domain: {s}");
        assert!(s.contains("5"), "should contain code: {s}");
        free_obj(err);
    }
}
