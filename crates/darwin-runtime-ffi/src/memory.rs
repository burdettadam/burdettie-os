use std::ffi::CString;
use std::os::raw::c_char;

use crate::introspection::{DarwinClassList, DarwinMethodInfo};

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
