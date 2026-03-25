use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;

use crate::error::{clear_last_error, set_last_error};

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

pub(crate) fn error_result() -> DarwinRuntimeResult {
    DarwinRuntimeResult {
        exit_code: -1,
        images_loaded: 0,
        shims_installed: 0,
        initializers_run: 0,
        error: 1,
    }
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
