use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        let sanitized = msg.replace('\0', "");
        *e.borrow_mut() = CString::new(sanitized).ok();
    });
}

pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

/// Returns the last error message, or NULL if no error.
///
/// The returned string is valid until the next FFI call on the same thread.
/// Do NOT free the returned pointer.
#[no_mangle]
pub extern "C" fn darwin_runtime_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn last_error_starts_empty() {
        clear_last_error();
        let ptr = darwin_runtime_last_error();
        assert!(ptr.is_null());
    }

    #[test]
    fn set_and_get_error() {
        set_last_error("something went wrong");
        let ptr = darwin_runtime_last_error();
        assert!(!ptr.is_null());
        let msg = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        assert_eq!(msg, "something went wrong");
        clear_last_error();
    }

    #[test]
    fn error_result_fields() {
        let r = crate::run::error_result();
        assert_eq!(r.exit_code, -1);
        assert_eq!(r.error, 1);
        assert_eq!(r.images_loaded, 0);
    }
}
