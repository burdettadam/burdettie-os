//! C-ABI FFI wrapper for the burdetty-os runtime.
//!
//! Exposes the runtime's core functionality through stable `extern "C"` functions
//! that can be called from C, Swift, Python, or any language with C FFI support.
//!
//! # Usage from C
//!
//! ```c
//! #include "darwin_runtime.h"
//!
//! DarwinVersion v = darwin_runtime_version();
//! printf("darwin-runtime %d.%d.%d\n", v.major, v.minor, v.patch);
//!
//! DarwinRuntimeResult result = darwin_runtime_run("/path/to/binary");
//! printf("exit code: %d, images loaded: %zu\n", result.exit_code, result.images_loaded);
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Version information for the darwin-runtime library.
#[repr(C)]
pub struct DarwinVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Returns the library version.
#[no_mangle]
pub extern "C" fn darwin_runtime_version() -> DarwinVersion {
    DarwinVersion {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

// ---------------------------------------------------------------------------
// Runtime execution
// ---------------------------------------------------------------------------

/// Result of running a binary through the darwin runtime.
#[repr(C)]
pub struct DarwinRuntimeResult {
    /// Process exit code (0 = success). -1 if the run failed.
    pub exit_code: i32,
    /// Number of Mach-O images loaded (main binary + dylibs).
    pub images_loaded: usize,
    /// Number of shim functions installed.
    pub shims_installed: usize,
    /// Number of initializer functions executed.
    pub initializers_run: usize,
    /// Non-zero if an error occurred. Call `darwin_runtime_last_error` for details.
    pub error: i32,
}

/// Run a Mach-O binary through the darwin runtime with permissive security.
///
/// # Safety
///
/// `path` must be a valid, NUL-terminated C string pointing to a real file.
#[no_mangle]
pub unsafe extern "C" fn darwin_runtime_run(path: *const c_char) -> DarwinRuntimeResult {
    if path.is_null() {
        set_last_error("path is null");
        return error_result();
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let Ok(path_str) = c_str.to_str() else {
        set_last_error("path is not valid UTF-8");
        return error_result();
    };

    let binary_path = Path::new(path_str);
    let config = macho_runtime::RuntimeConfig::permissive(binary_path);

    match macho_runtime::Runtime::load_and_run(binary_path, &config) {
        Ok(result) => {
            clear_last_error();
            DarwinRuntimeResult {
                exit_code: result.exit_code,
                images_loaded: result.images_loaded,
                shims_installed: result.shims_installed,
                initializers_run: result.initializers_run,
                error: 0,
            }
        }
        Err(e) => {
            set_last_error(&format!("{e}"));
            error_result()
        }
    }
}

/// Run a Mach-O binary with Apple-like (secure) security policy.
///
/// # Safety
///
/// `path` must be a valid, NUL-terminated C string pointing to a real file.
#[no_mangle]
pub unsafe extern "C" fn darwin_runtime_run_secure(path: *const c_char) -> DarwinRuntimeResult {
    if path.is_null() {
        set_last_error("path is null");
        return error_result();
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let Ok(path_str) = c_str.to_str() else {
        set_last_error("path is not valid UTF-8");
        return error_result();
    };

    let binary_path = Path::new(path_str);
    let config = macho_runtime::RuntimeConfig::secure(binary_path);

    match macho_runtime::Runtime::load_and_run(binary_path, &config) {
        Ok(result) => {
            clear_last_error();
            DarwinRuntimeResult {
                exit_code: result.exit_code,
                images_loaded: result.images_loaded,
                shims_installed: result.shims_installed,
                initializers_run: result.initializers_run,
                error: 0,
            }
        }
        Err(e) => {
            set_last_error(&format!("{e}"));
            error_result()
        }
    }
}

// ---------------------------------------------------------------------------
// ObjC runtime introspection
// ---------------------------------------------------------------------------

/// Returns the number of registered ObjC classes (binary-parsed + synthetic Foundation).
#[no_mangle]
pub extern "C" fn darwin_objc_class_count() -> usize {
    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map_or(0, |rt| rt.class_count())
}

/// Returns the number of registered ObjC selectors.
#[no_mangle]
pub extern "C" fn darwin_objc_selector_count() -> usize {
    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map_or(0, |rt| rt.selector_count())
}

/// Information about a single ObjC class, returned by FFI queries.
#[repr(C)]
pub struct DarwinClassInfo {
    /// Class name (owned C string — caller must free with `darwin_string_free`).
    pub name: *mut c_char,
    /// Number of instance methods on this class.
    pub method_count: usize,
    /// Number of class methods.
    pub class_method_count: usize,
    /// Instance size in bytes.
    pub instance_size: u32,
    /// Whether this class is a root class (no superclass).
    pub is_root: u8,
}

/// Information about a single ObjC method, returned by FFI queries.
#[repr(C)]
pub struct DarwinMethodInfo {
    /// Selector name (owned C string — caller must free with `darwin_string_free`).
    pub selector: *mut c_char,
    /// ObjC type encoding (owned C string — caller must free with `darwin_string_free`).
    pub type_encoding: *mut c_char,
}

/// Look up an ObjC class by name.
///
/// Returns a `DarwinClassInfo` with `name` set to the class name on success.
/// On failure (class not found), returns a struct with `name == NULL`.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string.
/// The caller must free the returned `name` field with `darwin_string_free`.
#[no_mangle]
pub unsafe extern "C" fn darwin_objc_get_class(name: *const c_char) -> DarwinClassInfo {
    let empty = DarwinClassInfo {
        name: ptr::null_mut(),
        method_count: 0,
        class_method_count: 0,
        instance_size: 0,
        is_root: 0,
    };

    if name.is_null() {
        return empty;
    }

    let c_str = unsafe { CStr::from_ptr(name) };
    let Ok(name_str) = c_str.to_str() else {
        return empty;
    };

    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let Some(rt) = guard.as_ref() else {
        return empty;
    };
    let Some(class) = rt.get_class(name_str) else {
        return empty;
    };

    let c_name = match CString::new(class.name.as_str()) {
        Ok(s) => s.into_raw(),
        Err(_) => return empty,
    };

    DarwinClassInfo {
        name: c_name,
        method_count: class.methods.len(),
        class_method_count: class.class_methods.len(),
        instance_size: class.instance_size,
        is_root: u8::from(class.is_root),
    }
}

/// A list of class names returned by `darwin_objc_list_classes`.
#[repr(C)]
pub struct DarwinClassList {
    /// Array of C-string pointers (each owned — free the list with `darwin_class_list_free`).
    pub names: *mut *mut c_char,
    /// Number of entries in `names`.
    pub count: usize,
}

/// Returns a list of all registered ObjC class names.
///
/// The caller must free the returned list with `darwin_class_list_free`.
#[no_mangle]
pub extern "C" fn darwin_objc_list_classes() -> DarwinClassList {
    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let Some(rt) = guard.as_ref() else {
        return DarwinClassList {
            names: ptr::null_mut(),
            count: 0,
        };
    };

    let mut names: Vec<String> = rt.classes_by_name_iter().map(|(n, _)| n.clone()).collect();
    names.sort();

    let mut c_names: Vec<*mut c_char> = Vec::with_capacity(names.len());
    for n in &names {
        match CString::new(n.as_str()) {
            Ok(s) => c_names.push(s.into_raw()),
            Err(_) => c_names.push(ptr::null_mut()),
        }
    }

    let count = c_names.len();
    let ptr = c_names.as_mut_ptr();
    std::mem::forget(c_names);

    DarwinClassList { names: ptr, count }
}

/// Returns instance methods for a class as a flat array.
///
/// # Safety
///
/// `class_name` must be a valid, NUL-terminated C string.
/// The caller must free the returned array with `darwin_method_list_free`.
#[no_mangle]
pub unsafe extern "C" fn darwin_objc_list_methods(
    class_name: *const c_char,
    out_count: *mut usize,
) -> *mut DarwinMethodInfo {
    if class_name.is_null() || out_count.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(class_name) };
    let Ok(name_str) = c_str.to_str() else {
        unsafe { *out_count = 0 };
        return ptr::null_mut();
    };

    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let methods = match guard.as_ref().and_then(|rt| rt.get_class(name_str)) {
        Some(class) => &class.methods,
        None => {
            unsafe { *out_count = 0 };
            return ptr::null_mut();
        }
    };

    let mut infos: Vec<DarwinMethodInfo> = methods
        .iter()
        .map(|m| DarwinMethodInfo {
            selector: CString::new(m.selector.as_str())
                .map(|s| s.into_raw())
                .unwrap_or(ptr::null_mut()),
            type_encoding: CString::new(m.type_encoding.as_str())
                .map(|s| s.into_raw())
                .unwrap_or(ptr::null_mut()),
        })
        .collect();

    unsafe { *out_count = infos.len() };
    let ptr = infos.as_mut_ptr();
    std::mem::forget(infos);
    ptr
}

// ---------------------------------------------------------------------------
// Foundation class count
// ---------------------------------------------------------------------------

/// Returns the number of Foundation classes registered in the runtime.
///
/// This counts only synthetic Foundation classes (NSString, NSArray, etc.),
/// not classes parsed from loaded binaries.
#[no_mangle]
pub extern "C" fn darwin_foundation_class_count() -> usize {
    // Foundation classes are registered as synthetic classes with known names.
    // We count classes whose names start with "NS" or "__NSCF" from the runtime.
    let guard = objc_runtime::OBJC_RUNTIME
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    guard.as_ref().map_or(0, |rt| {
        rt.classes_by_name_iter()
            .filter(|(name, _)| name.starts_with("NS") || name.starts_with("__NSCF"))
            .count()
    })
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

fn error_result() -> DarwinRuntimeResult {
    DarwinRuntimeResult {
        exit_code: -1,
        images_loaded: 0,
        shims_installed: 0,
        initializers_run: 0,
        error: 1,
    }
}

/// Returns the last error message, or NULL if no error.
///
/// The returned string is valid until the next FFI call on the same thread.
/// Do NOT free the returned pointer.
#[no_mangle]
pub extern "C" fn darwin_runtime_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}

// ---------------------------------------------------------------------------
// Memory management helpers
// ---------------------------------------------------------------------------

/// Free a C string allocated by this library.
///
/// # Safety
///
/// `s` must have been returned by a `darwin_*` function, or be NULL.
#[no_mangle]
pub unsafe extern "C" fn darwin_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

/// Free a `DarwinClassList` returned by `darwin_objc_list_classes`.
///
/// # Safety
///
/// `list` must have been returned by `darwin_objc_list_classes`.
#[no_mangle]
pub unsafe extern "C" fn darwin_class_list_free(list: DarwinClassList) {
    if !list.names.is_null() {
        let names = unsafe { Vec::from_raw_parts(list.names, list.count, list.count) };
        for name in names {
            if !name.is_null() {
                drop(unsafe { CString::from_raw(name) });
            }
        }
    }
}

/// Free a method info array returned by `darwin_objc_list_methods`.
///
/// # Safety
///
/// `methods` and `count` must match a return from `darwin_objc_list_methods`.
#[no_mangle]
pub unsafe extern "C" fn darwin_method_list_free(methods: *mut DarwinMethodInfo, count: usize) {
    if !methods.is_null() {
        let infos = unsafe { Vec::from_raw_parts(methods, count, count) };
        for info in infos {
            if !info.selector.is_null() {
                drop(unsafe { CString::from_raw(info.selector) });
            }
            if !info.type_encoding.is_null() {
                drop(unsafe { CString::from_raw(info.type_encoding) });
            }
        }
    }
}
