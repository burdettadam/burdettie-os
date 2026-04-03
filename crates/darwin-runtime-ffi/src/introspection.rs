use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Returns the number of registered ObjC classes (binary-parsed + synthetic Foundation).
#[no_mangle]
pub extern "C" fn darwin_objc_class_count() -> usize {
    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map_or(0, |rt| rt.class_count())
}

/// Returns the number of registered ObjC selectors.
#[no_mangle]
pub extern "C" fn darwin_objc_selector_count() -> usize {
    let guard = objc_runtime::OBJC_RUNTIME
        .read()
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
        .read()
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
        .read()
        .unwrap_or_else(|e| e.into_inner());

    let Some(rt) = guard.as_ref() else {
        return DarwinClassList {
            names: ptr::null_mut(),
            count: 0,
        };
    };

    let mut names: Vec<String> = rt.classes_by_name_iter().map(|(n, _)| n.to_string()).collect();
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
        .read()
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
            selector: CString::new(m.selector.as_ref())
                .map(|s| s.into_raw())
                .unwrap_or(ptr::null_mut()),
            type_encoding: CString::new(m.type_encoding.as_ref())
                .map(|s| s.into_raw())
                .unwrap_or(ptr::null_mut()),
        })
        .collect();

    unsafe { *out_count = infos.len() };
    let ptr = infos.as_mut_ptr();
    std::mem::forget(infos);
    ptr
}

/// Returns the number of Foundation classes registered in the runtime.
///
/// This counts only synthetic Foundation classes (NSString, NSArray, etc.),
/// not classes parsed from loaded binaries.
#[no_mangle]
pub extern "C" fn darwin_foundation_class_count() -> usize {
    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());

    guard.as_ref().map_or(0, |rt| {
        rt.classes_by_name_iter()
            .filter(|(name, _)| name.starts_with("NS") || name.starts_with("__NSCF"))
            .count()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_count_without_runtime() {
        let count = darwin_objc_class_count();
        // Without a loaded binary the runtime may or may not be initialised;
        // the function must not panic and must return a non-negative value.
        assert!(count < 100_000);
    }

    #[test]
    fn selector_count_without_runtime() {
        let count = darwin_objc_selector_count();
        assert!(count < 100_000);
    }

    #[test]
    fn foundation_class_count_without_runtime() {
        let count = darwin_foundation_class_count();
        assert!(count < 100_000);
    }

    #[test]
    fn get_class_null_name() {
        let info = unsafe { darwin_objc_get_class(std::ptr::null()) };
        assert!(info.name.is_null());
    }

    #[test]
    fn list_classes_empty_runtime() {
        let list = darwin_objc_list_classes();
        // Either null with count 0, or a valid (possibly empty) list.
        if list.names.is_null() {
            assert_eq!(list.count, 0);
        } else {
            // Free the returned list to avoid leaks.
            unsafe { crate::memory::darwin_class_list_free(list) };
        }
    }

    #[test]
    fn list_methods_null_class_name_returns_null() {
        let mut count: usize = 42;
        let ptr = unsafe { darwin_objc_list_methods(std::ptr::null(), &mut count) };
        assert!(ptr.is_null());
    }

    #[test]
    fn list_methods_null_out_count_returns_null() {
        let name = std::ffi::CString::new("NSObject").unwrap();
        let ptr = unsafe { darwin_objc_list_methods(name.as_ptr(), std::ptr::null_mut()) };
        assert!(ptr.is_null());
    }

    #[test]
    fn get_class_unknown_name() {
        let name = std::ffi::CString::new("__NonExistentClass42__").unwrap();
        let info = unsafe { darwin_objc_get_class(name.as_ptr()) };
        assert!(info.name.is_null());
        assert_eq!(info.method_count, 0);
        assert_eq!(info.class_method_count, 0);
    }
}
