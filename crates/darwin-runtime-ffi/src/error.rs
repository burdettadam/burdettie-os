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
