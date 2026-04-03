//! NSFileManager shim: file system queries and mutation operations.
//!
//! Object layout (heap-allocated, singleton for defaultManager):
//!
//!   offset  0: isa        (u64 — pointer to NSFileManager class)
//!
//! Total: 8 bytes (all operations delegate to std::fs / PAL).

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

/// `-[NSFileManager changeCurrentDirectoryPath:]` — changes the process cwd.
pub extern "C" fn shim_nsfilemanager_change_current_directory_path(
    obj: u64,
    _cmd: u64,
    path: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }

    let Some(path_str) = extract_path(path) else {
        return 0;
    };

    u8::from(std::env::set_current_dir(std::path::Path::new(&path_str)).is_ok())
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

/// Helper: set `*err_ptr` to an NSCocoaErrorDomain NSError with the given code.
fn set_cocoa_error(err_ptr: u64, code: i64) {
    if err_ptr != 0 {
        let domain_cstr = c"NSCocoaErrorDomain";
        let domain = crate::cfstring::alloc_nsstring_from_cstr(domain_cstr.as_ptr());
        let err = shim_nserror_factory(domain, code);
        unsafe {
            *(err_ptr as *mut u64) = err;
        }
    }
}

/// Helper: extract a Rust path string from an NSString pointer.
fn extract_path(nsstr: u64) -> Option<String> {
    if nsstr == 0 {
        return None;
    }
    let utf8 = crate::cfstring::shim_nsstring_utf8string(nsstr, 0);
    if utf8.is_null() {
        return None;
    }
    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) };
    path_str.to_str().ok().map(|s| s.to_string())
}

/// `-[NSFileManager copyItemAtPath:toPath:error:]`
pub extern "C" fn shim_nsfilemanager_copy_item(
    _obj: u64,
    _cmd: u64,
    src: u64,
    dst: u64,
    err_ptr: u64,
) -> u64 {
    let Some(src_path) = extract_path(src) else {
        set_cocoa_error(err_ptr, 4); // NSFileNoSuchFileError
        return 0;
    };
    let Some(dst_path) = extract_path(dst) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };

    match std::fs::copy(&src_path, &dst_path) {
        Ok(_) => 1,
        Err(_) => {
            set_cocoa_error(err_ptr, 512); // NSFileWriteUnknownError
            0
        }
    }
}

/// `-[NSFileManager moveItemAtPath:toPath:error:]`
pub extern "C" fn shim_nsfilemanager_move_item(
    _obj: u64,
    _cmd: u64,
    src: u64,
    dst: u64,
    err_ptr: u64,
) -> u64 {
    let Some(src_path) = extract_path(src) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };
    let Some(dst_path) = extract_path(dst) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };

    match std::fs::rename(&src_path, &dst_path) {
        Ok(()) => 1,
        Err(_) => {
            set_cocoa_error(err_ptr, 512);
            0
        }
    }
}

/// `-[NSFileManager removeItemAtPath:error:]`
pub extern "C" fn shim_nsfilemanager_remove_item(
    _obj: u64,
    _cmd: u64,
    path: u64,
    err_ptr: u64,
) -> u64 {
    let Some(p) = extract_path(path) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };

    let meta = match std::fs::metadata(&p) {
        Ok(m) => m,
        Err(_) => {
            set_cocoa_error(err_ptr, 4);
            return 0;
        }
    };

    let result = if meta.is_dir() {
        std::fs::remove_dir_all(&p)
    } else {
        std::fs::remove_file(&p)
    };

    match result {
        Ok(()) => 1,
        Err(_) => {
            set_cocoa_error(err_ptr, 512);
            0
        }
    }
}

/// `-[NSFileManager createDirectoryAtPath:withIntermediateDirectories:attributes:error:]`
pub extern "C" fn shim_nsfilemanager_create_directory(
    _obj: u64,
    _cmd: u64,
    path: u64,
    intermediate: u64,
    _attributes: u64,
    err_ptr: u64,
) -> u64 {
    let Some(p) = extract_path(path) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };

    let result = if intermediate != 0 {
        std::fs::create_dir_all(&p)
    } else {
        std::fs::create_dir(&p)
    };

    match result {
        Ok(()) => 1,
        Err(_) => {
            set_cocoa_error(err_ptr, 512);
            0
        }
    }
}

/// `-[NSFileManager createFileAtPath:contents:attributes:]`
pub extern "C" fn shim_nsfilemanager_create_file(
    _obj: u64,
    _cmd: u64,
    path: u64,
    contents: u64,
    _attrs: u64,
) -> u8 {
    let Some(p) = extract_path(path) else {
        return 0;
    };

    let bytes: &[u8] = if contents == 0 {
        &[]
    } else {
        unsafe {
            let len = *((contents as usize + 8) as *const u64) as usize;
            let ptr = *((contents as usize + 16) as *const u64) as *const u8;
            if ptr.is_null() || len == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(ptr, len)
            }
        }
    };

    u8::from(std::fs::write(p, bytes).is_ok())
}

/// `-[NSFileManager contentsAtPath:]`
pub extern "C" fn shim_nsfilemanager_contents_at_path(
    _obj: u64,
    _cmd: u64,
    path: u64,
) -> u64 {
    let Some(p) = extract_path(path) else {
        return 0;
    };

    match std::fs::read(p) {
        Ok(bytes) => crate::nsdata::make_nsdata(&bytes),
        Err(_) => 0,
    }
}

/// `-[NSFileManager isReadableFileAtPath:]`
pub extern "C" fn shim_nsfilemanager_is_readable_file(
    _obj: u64,
    _cmd: u64,
    path: u64,
) -> u8 {
    let Some(p) = extract_path(path) else {
        return 0;
    };
    u8::from(std::fs::metadata(&p).is_ok())
}

/// `-[NSFileManager isWritableFileAtPath:]`
pub extern "C" fn shim_nsfilemanager_is_writable_file(
    _obj: u64,
    _cmd: u64,
    path: u64,
) -> u8 {
    let Some(p) = extract_path(path) else {
        return 0;
    };
    u8::from(
        std::fs::metadata(&p)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false),
    )
}

/// `-[NSFileManager isDeletableFileAtPath:]`
pub extern "C" fn shim_nsfilemanager_is_deletable_file(
    _obj: u64,
    _cmd: u64,
    path: u64,
) -> u8 {
    let Some(p) = extract_path(path) else {
        return 0;
    };
    // Deletable if the parent directory is writable
    let parent = std::path::Path::new(&p).parent();
    match parent {
        Some(dir) => u8::from(
            std::fs::metadata(dir)
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false),
        ),
        None => 0,
    }
}

/// `-[NSFileManager subpathsOfDirectoryAtPath:error:]`
pub extern "C" fn shim_nsfilemanager_subpaths_of_directory(
    _obj: u64,
    _cmd: u64,
    path: u64,
    error: u64,
) -> u64 {
    let Some(root) = extract_path(path) else {
        set_cocoa_error(error, 4);
        return 0;
    };

    let root_path = std::path::Path::new(&root);
    let mut results: Vec<u64> = Vec::new();

    fn walk(base: &std::path::Path, prefix: &str, out: &mut Vec<u64>) {
        let Ok(entries) = std::fs::read_dir(base) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let subpath = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{prefix}/{name_str}")
            };
            let cstr = std::ffi::CString::new(subpath.as_str()).unwrap_or_default();
            out.push(crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr()));
            let full = base.join(&*name_str);
            if full.is_dir() {
                walk(&full, &subpath, out);
            }
        }
    }

    walk(root_path, "", &mut results);

    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        results.as_ptr() as u64,
        results.len() as u64,
    )
}

/// `-[NSFileManager attributesOfItemAtPath:error:]`
///
/// Returns an NSDictionary with `NSFileSize` → NSNumber.
pub extern "C" fn shim_nsfilemanager_attributes_of_item(
    _obj: u64,
    _cmd: u64,
    path: u64,
    err_ptr: u64,
) -> u64 {
    let Some(p) = extract_path(path) else {
        set_cocoa_error(err_ptr, 4);
        return 0;
    };

    let meta = match std::fs::metadata(&p) {
        Ok(m) => m,
        Err(_) => {
            set_cocoa_error(err_ptr, 260); // NSFileReadNoSuchFileError
            return 0;
        }
    };

    // Build NSDictionary with NSFileSize key
    let key_cstr = c"NSFileSize";
    let key = crate::cfstring::alloc_nsstring_from_cstr(key_cstr.as_ptr());
    let val = crate::nsnumber::make_nsnumber_int(meta.len() as i64);

    crate::nsdictionary::make_nsdictionary(&[key], &[val])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use std::sync::{Arc, Mutex};

    static FILE_MANAGER_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CurrentDirRestore(std::path::PathBuf);

    impl Drop for CurrentDirRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    fn ensure_platform_for_tests() {
        #[cfg(target_os = "macos")]
        crate::set_platform(Arc::new(platform_darwin_host::DarwinHostPlatform::new()));

        #[cfg(target_os = "linux")]
        crate::set_platform(Arc::new(platform_linux::LinuxHostPlatform::new()));
    }

    #[test]
    fn default_manager_non_zero() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        assert_ne!(mgr, 0);
    }

    #[test]
    fn default_manager_is_singleton() {
        let a = shim_nsfilemanager_default_manager(0, 0);
        let b = shim_nsfilemanager_default_manager(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn default_manager_has_isa() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let isa = read_field(mgr, 0);
        assert_eq!(isa, crate::nsobj::nsfilemanager_class_addr());
    }

    #[test]
    fn file_exists_nil_path_returns_zero() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        assert_eq!(shim_nsfilemanager_file_exists_at_path(mgr, 0, 0), 0);
    }

    #[test]
    fn file_exists_is_directory_nil_path_returns_zero() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        assert_eq!(
            shim_nsfilemanager_file_exists_at_path_is_directory(mgr, 0, 0, 0),
            0
        );
    }

    #[test]
    fn change_current_directory_path_nil_receiver_returns_zero() {
        assert_eq!(shim_nsfilemanager_change_current_directory_path(0, 0, 0), 0);
    }

    #[test]
    fn change_current_directory_path_updates_process_cwd() {
        let _guard = FILE_MANAGER_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        ensure_platform_for_tests();
        let restore = CurrentDirRestore(std::env::current_dir().expect("capture current dir"));
        let dir = temp_path("change_cwd");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp cwd");

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(dir.to_str().unwrap());
        let ret = shim_nsfilemanager_change_current_directory_path(mgr, 0, path_ns);
        let canonical_dir = dir.canonicalize().expect("canonicalize temp cwd");
        assert_eq!(ret, 1);
        assert_eq!(std::env::current_dir().unwrap(), canonical_dir);
        assert_string_eq(
            shim_nsfilemanager_current_directory_path(mgr, 0),
            canonical_dir.to_str().unwrap(),
        );

        drop(restore);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Mutation operation tests ───────────────────────────────────

    fn temp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("cf_lite_test_{name}_{}", std::process::id()));
        std::fs::write(&p, content).expect("write temp file");
        p
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("cf_lite_test_{name}_{}", std::process::id()))
    }

    #[test]
    fn copy_item_creates_copy() {
        let src = temp_file("copy_src", b"hello copy");
        let dst = temp_path("copy_dst");
        let _ = std::fs::remove_file(&dst);

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let src_ns = make_string(src.to_str().unwrap());
        let dst_ns = make_string(dst.to_str().unwrap());
        let ret = shim_nsfilemanager_copy_item(mgr, 0, src_ns, dst_ns, 0);
        assert_eq!(ret, 1);
        assert!(src.exists(), "source should still exist after copy");
        assert!(dst.exists(), "destination should exist after copy");
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello copy");

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn move_item_moves_file() {
        let src = temp_file("move_src", b"hello move");
        let dst = temp_path("move_dst");
        let _ = std::fs::remove_file(&dst);

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let src_ns = make_string(src.to_str().unwrap());
        let dst_ns = make_string(dst.to_str().unwrap());
        let ret = shim_nsfilemanager_move_item(mgr, 0, src_ns, dst_ns, 0);
        assert_eq!(ret, 1);
        assert!(!src.exists(), "source should be gone after move");
        assert!(dst.exists(), "destination should exist after move");
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello move");

        let _ = std::fs::remove_file(&dst);
    }

    #[test]
    fn remove_item_deletes_file() {
        let f = temp_file("remove_me", b"bye");

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        let ret = shim_nsfilemanager_remove_item(mgr, 0, path_ns, 0);
        assert_eq!(ret, 1);
        assert!(!f.exists(), "file should be gone after remove");
    }

    #[test]
    fn create_directory_basic() {
        let dir = temp_path("mkdir_basic");
        let _ = std::fs::remove_dir_all(&dir);

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(dir.to_str().unwrap());
        let ret = shim_nsfilemanager_create_directory(mgr, 0, path_ns, 0, 0, 0);
        assert_eq!(ret, 1);
        assert!(dir.is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_directory_intermediate() {
        let dir = temp_path("mkdir_inter").join("a").join("b");
        let _ = std::fs::remove_dir_all(temp_path("mkdir_inter"));

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(dir.to_str().unwrap());
        let ret = shim_nsfilemanager_create_directory(mgr, 0, path_ns, 1, 0, 0);
        assert_eq!(ret, 1);
        assert!(dir.is_dir());

        let _ = std::fs::remove_dir_all(temp_path("mkdir_inter"));
    }

    #[test]
    fn attributes_returns_file_size() {
        let f = temp_file("attrs", b"0123456789"); // 10 bytes

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        let dict = shim_nsfilemanager_attributes_of_item(mgr, 0, path_ns, 0);
        assert_ne!(dict, 0, "should return a dictionary");

        // Look up NSFileSize
        let key_ns = make_string("NSFileSize");
        let val = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key_ns);
        assert_ne!(val, 0, "NSFileSize key should exist");

        let size = crate::nsnumber::shim_nsnumber_integer_value(val, 0);
        assert_eq!(size, 10, "file size should be 10 bytes");

        let _ = std::fs::remove_file(&f);
    }

    // ── New selector tests ─────────────────────────────────────────

    #[test]
    fn create_file_writes_contents() {
        let p = temp_path("create_file_test");
        let _ = std::fs::remove_file(&p);

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(p.to_str().unwrap());
        let data = crate::nsdata::make_nsdata(b"hello create");
        let ret = shim_nsfilemanager_create_file(mgr, 0, path_ns, data, 0);
        assert_eq!(ret, 1);
        assert_eq!(std::fs::read(&p).unwrap(), b"hello create");

        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn create_file_nil_contents_creates_empty() {
        let p = temp_path("create_file_empty");
        let _ = std::fs::remove_file(&p);

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(p.to_str().unwrap());
        let ret = shim_nsfilemanager_create_file(mgr, 0, path_ns, 0, 0);
        assert_eq!(ret, 1);
        assert_eq!(std::fs::read(&p).unwrap(), b"");

        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn create_file_nil_path_returns_zero() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        assert_eq!(shim_nsfilemanager_create_file(mgr, 0, 0, 0, 0), 0);
    }

    #[test]
    fn contents_at_path_reads_file() {
        let f = temp_file("contents_read", b"read me");
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        let data = shim_nsfilemanager_contents_at_path(mgr, 0, path_ns);
        assert_ne!(data, 0);

        let len = unsafe { *((data as usize + 8) as *const u64) };
        assert_eq!(len, 7);

        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn contents_at_path_nonexistent_returns_zero() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string("/nonexistent/file/xyz");
        assert_eq!(shim_nsfilemanager_contents_at_path(mgr, 0, path_ns), 0);
    }

    #[test]
    fn is_readable_file_existing() {
        let f = temp_file("readable_test", b"x");
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        assert_eq!(shim_nsfilemanager_is_readable_file(mgr, 0, path_ns), 1);
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn is_readable_file_nonexistent() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string("/nonexistent/readable_check");
        assert_eq!(shim_nsfilemanager_is_readable_file(mgr, 0, path_ns), 0);
    }

    #[test]
    fn is_writable_file_existing() {
        let f = temp_file("writable_test", b"x");
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        assert_eq!(shim_nsfilemanager_is_writable_file(mgr, 0, path_ns), 1);
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn is_writable_file_nonexistent() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string("/nonexistent/writable_check");
        assert_eq!(shim_nsfilemanager_is_writable_file(mgr, 0, path_ns), 0);
    }

    #[test]
    fn is_deletable_file_existing() {
        let f = temp_file("deletable_test", b"x");
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(f.to_str().unwrap());
        assert_eq!(shim_nsfilemanager_is_deletable_file(mgr, 0, path_ns), 1);
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn is_deletable_file_nil_path() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        assert_eq!(shim_nsfilemanager_is_deletable_file(mgr, 0, 0), 0);
    }

    #[test]
    fn subpaths_of_directory_lists_recursively() {
        let dir = temp_path("subpaths_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(dir.join("top.txt"), b"top").unwrap();
        std::fs::write(dir.join("a").join("nested.txt"), b"nested").unwrap();

        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string(dir.to_str().unwrap());
        let arr = shim_nsfilemanager_subpaths_of_directory(mgr, 0, path_ns, 0);
        assert_ne!(arr, 0);

        // Should have at least 3 entries: "a", "top.txt", "a/nested.txt"
        let count = crate::shim_test_helpers::read_field(arr, 8);
        assert!(count >= 3, "expected >= 3 subpaths, got {count}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn subpaths_of_directory_nonexistent_returns_empty_array() {
        let mgr = shim_nsfilemanager_default_manager(0, 0);
        let path_ns = make_string("/nonexistent/dir/subpaths");
        let arr = shim_nsfilemanager_subpaths_of_directory(mgr, 0, path_ns, 0);
        // walk returns an empty vec for non-existent dir, so we get an empty array
        assert_ne!(arr, 0);
    }
}
