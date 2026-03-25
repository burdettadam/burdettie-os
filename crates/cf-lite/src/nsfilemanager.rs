//! NSFileManager shim: file system queries.
//!
//! Object layout (heap-allocated, singleton for defaultManager):
//!
//!   offset  0: isa        (u64 — pointer to NSFileManager class)
//!
//! Total: 8 bytes (all operations delegate to libc).

use std::sync::OnceLock;

static DEFAULT_MANAGER: OnceLock<u64> = OnceLock::new();

fn get_or_create_default_manager() -> u64 {
    *DEFAULT_MANAGER.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = crate::nsobj::nsfilemanager_class_addr();
            }
        }
        obj
    })
}

/// `+[NSFileManager defaultManager]` — returns the singleton.
pub extern "C" fn shim_nsfilemanager_default_manager(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_default_manager()
}

/// `-[NSFileManager fileExistsAtPath:]` — checks if a file exists.
pub extern "C" fn shim_nsfilemanager_file_exists_at_path(_obj: u64, _cmd: u64, path: u64) -> u8 {
    if path == 0 {
        return 0;
    }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(path, 0);
    if utf8.is_null() {
        return 0;
    }

    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) };
    let path_str = path_str.to_str().unwrap_or("");

    u8::from(
        crate::platform()
            .fs()
            .exists(std::path::Path::new(path_str)),
    )
}

/// `-[NSFileManager fileExistsAtPath:isDirectory:]` — checks existence + directory flag.
pub extern "C" fn shim_nsfilemanager_file_exists_at_path_is_directory(
    _obj: u64,
    _cmd: u64,
    path: u64,
    is_dir_ptr: u64,
) -> u8 {
    if path == 0 {
        return 0;
    }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(path, 0);
    if utf8.is_null() {
        return 0;
    }

    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) };
    let path_str = path_str.to_str().unwrap_or("");

    let p = std::path::Path::new(path_str);
    let fs = crate::platform().fs();
    if !fs.exists(p) {
        return 0;
    }

    if is_dir_ptr != 0 {
        let is_dir: u8 = u8::from(fs.is_dir(p));
        unsafe {
            *(is_dir_ptr as *mut u8) = is_dir;
        }
    }

    1
}

/// `-[NSFileManager currentDirectoryPath]` — returns cwd as NSString.
pub extern "C" fn shim_nsfilemanager_current_directory_path(_obj: u64, _cmd: u64) -> u64 {
    let cwd = crate::platform()
        .fs()
        .current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string());

    let cstr = std::ffi::CString::new(cwd).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSFileManager contentsOfDirectoryAtPath:error:]` — returns directory contents as NSArray.
pub extern "C" fn shim_nsfilemanager_contents_of_directory_at_path(
    _obj: u64,
    _cmd: u64,
    path: u64,
    error_ptr: u64,
) -> u64 {
    if path == 0 {
        return 0;
    }

    let utf8 = crate::cfstring::shim_nsstring_utf8string(path, 0);
    if utf8.is_null() {
        return 0;
    }

    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) };
    let path_str = path_str.to_str().unwrap_or("");

    let Ok(entries) = crate::platform()
        .fs()
        .read_dir(std::path::Path::new(path_str))
    else {
        // Set error if pointer provided
        if error_ptr != 0 {
            let domain_cstr = c"NSCocoaErrorDomain";
            let domain = crate::cfstring::alloc_nsstring_from_cstr(domain_cstr.as_ptr());
            let err = shim_nserror_factory(domain, 260); // NSFileReadNoSuchFileError = 260
            unsafe {
                *(error_ptr as *mut u64) = err;
            }
        }
        return 0;
    };

    // Collect entry names as NSStrings
    let mut ptrs: Vec<u64> = Vec::new();
    for name_str in &entries {
        let cstr = std::ffi::CString::new(name_str.as_str()).unwrap_or_default();
        let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
        ptrs.push(nsstr);
    }

    // Build NSArray via the same factory pattern as nsarray
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        ptrs.as_ptr() as u64,
        ptrs.len() as u64,
    )
}

/// Helper: create an NSError with domain string and code.
fn shim_nserror_factory(domain: u64, code: i64) -> u64 {
    crate::nserror::shim_nserror_error_with_domain_code_userinfo(0, 0, domain, code, 0)
}
