#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::ptr;

    // Re-import FFI functions from the same crate (they're pub extern "C")
    use darwin_runtime_ffi::*;

    #[test]
    fn version_returns_0_1_0() {
        let v = darwin_runtime_version();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn run_null_path_returns_error() {
        let result = unsafe { darwin_runtime_run(ptr::null()) };
        assert_eq!(result.error, 1);
        assert_eq!(result.exit_code, -1);

        let err = darwin_runtime_last_error();
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err) }.to_str().unwrap();
        assert!(msg.contains("null"), "error message: {msg}");
    }

    #[test]
    fn run_nonexistent_path_returns_error() {
        let path = CString::new("/nonexistent/binary").unwrap();
        let result = unsafe { darwin_runtime_run(path.as_ptr()) };
        assert_eq!(result.error, 1);
        assert_eq!(result.exit_code, -1);

        let err = darwin_runtime_last_error();
        assert!(!err.is_null());
    }

    #[test]
    fn run_secure_null_returns_error() {
        let result = unsafe { darwin_runtime_run_secure(ptr::null()) };
        assert_eq!(result.error, 1);
        assert_eq!(result.exit_code, -1);
    }

    #[test]
    fn run_return42_fixture() {
        let fixture = fixture_path("return42/return42");
        if !std::path::Path::new(&fixture).exists() {
            eprintln!("skipping: fixture not found at {fixture}");
            return;
        }
        let path = CString::new(fixture).unwrap();
        let result = unsafe { darwin_runtime_run(path.as_ptr()) };
        assert_eq!(result.error, 0, "unexpected error running return42");
        assert_eq!(result.exit_code, 42);
        assert!(result.images_loaded >= 1);
        assert!(result.shims_installed > 0);
    }

    #[test]
    fn run_simple_cli_fixture() {
        let fixture = fixture_path("simple_cli/app");
        if !std::path::Path::new(&fixture).exists() {
            eprintln!("skipping: fixture not found at {fixture}");
            return;
        }
        let path = CString::new(fixture).unwrap();
        let result = unsafe { darwin_runtime_run(path.as_ptr()) };
        assert_eq!(result.error, 0, "unexpected error running simple_cli");
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn class_count_zero_before_runtime_init() {
        // Before running anything through the runtime, ObjC runtime may not be
        // initialized. The count should not panic — returns 0 or a valid count.
        let count = darwin_objc_class_count();
        // Just ensure it doesn't panic/crash
        let _ = count;
    }

    #[test]
    fn selector_count_no_panic() {
        let count = darwin_objc_selector_count();
        let _ = count;
    }

    #[test]
    fn foundation_class_count_no_panic() {
        let count = darwin_foundation_class_count();
        let _ = count;
    }

    #[test]
    fn get_class_null_name() {
        let info = unsafe { darwin_objc_get_class(ptr::null()) };
        assert!(info.name.is_null());
    }

    #[test]
    fn get_class_unknown_name() {
        let name = CString::new("NonExistentClass12345").unwrap();
        let info = unsafe { darwin_objc_get_class(name.as_ptr()) };
        assert!(info.name.is_null());
    }

    #[test]
    fn list_classes_no_panic() {
        let list = darwin_objc_list_classes();
        // May be empty if runtime not initialized — just verify no crash
        unsafe { darwin_class_list_free(list) };
    }

    #[test]
    fn list_methods_null_name() {
        let mut count: usize = 99;
        let methods = unsafe { darwin_objc_list_methods(ptr::null(), &mut count) };
        assert!(methods.is_null());
    }

    #[test]
    fn string_free_null_is_safe() {
        // Should not crash
        unsafe { darwin_string_free(ptr::null_mut()) };
    }

    #[test]
    fn last_error_null_when_no_error() {
        // After a successful version call, there should be no error
        let _ = darwin_runtime_version();
        // Note: last_error is thread-local and only set by run functions
    }

    #[test]
    fn run_then_introspect_classes() {
        // Run a fixture to populate the ObjC runtime, then introspect
        let fixture = fixture_path("objc_minimal/objc_minimal");
        if !std::path::Path::new(&fixture).exists() {
            eprintln!("skipping: fixture not found at {fixture}");
            return;
        }
        let path = CString::new(fixture).unwrap();
        let result = unsafe { darwin_runtime_run(path.as_ptr()) };
        assert_eq!(result.error, 0);

        // After running, class count should be > 0
        let count = darwin_objc_class_count();
        assert!(count > 0, "expected classes after runtime run, got {count}");

        // List classes
        let list = darwin_objc_list_classes();
        assert!(list.count > 0);
        assert!(!list.names.is_null());

        // Check first name is valid
        let first = unsafe { *list.names };
        assert!(!first.is_null());
        let name = unsafe { CStr::from_ptr(first) }.to_str().unwrap();
        assert!(!name.is_empty());

        unsafe { darwin_class_list_free(list) };
    }

    #[test]
    fn run_then_introspect_foundation_after_nsstring() {
        let fixture = fixture_path("nsstring_ops/app");
        if !std::path::Path::new(&fixture).exists() {
            eprintln!("skipping: fixture not found at {fixture}");
            return;
        }
        let path = CString::new(fixture).unwrap();
        let result = unsafe { darwin_runtime_run(path.as_ptr()) };
        assert_eq!(result.error, 0);

        // NSString should be registered
        let class_name = CString::new("NSString").unwrap();
        let info = unsafe { darwin_objc_get_class(class_name.as_ptr()) };
        assert!(!info.name.is_null(), "NSString class not found");

        let returned_name = unsafe { CStr::from_ptr(info.name) }.to_str().unwrap();
        assert_eq!(returned_name, "NSString");
        assert!(info.method_count > 0, "NSString should have methods");

        unsafe { darwin_string_free(info.name) };

        // List NSString methods
        let mut method_count: usize = 0;
        let methods = unsafe { darwin_objc_list_methods(class_name.as_ptr(), &mut method_count) };
        assert!(method_count > 0);
        assert!(!methods.is_null());

        // Check first method has a valid selector
        let first = unsafe { &*methods };
        assert!(!first.selector.is_null());
        let sel = unsafe { CStr::from_ptr(first.selector) }.to_str().unwrap();
        assert!(!sel.is_empty());

        unsafe { darwin_method_list_free(methods, method_count) };

        // Foundation class count should be positive
        let fc = darwin_foundation_class_count();
        assert!(
            fc > 0,
            "Foundation class count should be > 0 after ns_string fixture"
        );
    }

    /// Helper to locate a fixture binary relative to the workspace root.
    fn fixture_path(relative: &str) -> String {
        let workspace = env!("CARGO_MANIFEST_DIR")
            .strip_suffix("/crates/darwin-runtime-ffi")
            .or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.to_str())
            })
            .unwrap_or(env!("CARGO_MANIFEST_DIR"));
        format!("{workspace}/fixtures/{relative}")
    }
}
