//! Process identity shims.
//!
//! These shims respond to queries about "who am I?" that every C runtime
//! program makes at startup: process name, executable path, argument
//! count/vector, and environment variables.

use std::ffi::CString;
use std::sync::OnceLock;

use crate::table::ShimTable;
use crate::ShimConfig;

/// Holds process identity state set at startup and returned by shims.
struct ProcessState {
    progname: CString,
    exec_path: CString,
    argc: i32,
    /// Keeps CString data alive — argv_ptrs point into these.
    #[allow(dead_code)]
    argv_cstrings: Vec<CString>,
    /// Pointer array for _NSGetArgv — stored as usize for Send+Sync.
    argv_ptrs: Vec<usize>,
    env: Vec<(CString, CString)>,
}

/// Global process state, initialised once before any shim is called.
static PROCESS_STATE: OnceLock<ProcessState> = OnceLock::new();

/// Returns the process name set during shim installation, or `None` if
/// process shims haven't been installed yet.
pub fn process_name() -> Option<&'static str> {
    PROCESS_STATE.get().and_then(|s| s.progname.to_str().ok())
}

/// Returns the executable path set during shim installation, or `None` if
/// process shims haven't been installed yet.
pub fn process_path() -> Option<&'static str> {
    PROCESS_STATE.get().and_then(|s| s.exec_path.to_str().ok())
}

/// Install process identity shims into a `ShimTable`.
pub struct ProcessShims;

impl ProcessShims {
    /// Populate global process state and register shims.
    pub fn install(table: &mut ShimTable, config: &ShimConfig) {
        PROCESS_STATE.get_or_init(|| {
            let argv_cstrings: Vec<CString> = config
                .argv
                .iter()
                .map(|a| CString::new(a.clone()).unwrap_or_else(|_| CString::new("").unwrap()))
                .collect();
            // Build null-terminated pointer array for _NSGetArgv
            // Store as usize to keep ProcessState Send+Sync
            let mut argv_ptrs: Vec<usize> = argv_cstrings
                .iter()
                .map(|cs| cs.as_ptr() as usize)
                .collect();
            argv_ptrs.push(0); // NULL terminator

            ProcessState {
                progname: CString::new(config.process_name.clone())
                    .unwrap_or_else(|_| CString::new("app").unwrap()),
                exec_path: CString::new(config.executable_path.clone())
                    .unwrap_or_else(|_| CString::new("/app").unwrap()),
                argc: config.argc,
                argv_cstrings,
                argv_ptrs,
                env: config
                    .env
                    .iter()
                    .map(|(k, v)| {
                        (
                            CString::new(k.clone()).unwrap_or_else(|_| CString::new("").unwrap()),
                            CString::new(v.clone()).unwrap_or_else(|_| CString::new("").unwrap()),
                        )
                    })
                    .collect(),
            }
        });

        table.register(
            "_getprogname",
            shim_getprogname as u64,
            "getprogname → process name",
        );
        table.register(
            "__NSGetExecutablePath",
            shim_nsgetexecutablepath as u64,
            "_NSGetExecutablePath → executable path",
        );
        table.register("__NSGetArgc", shim_nsgetargc as u64, "_NSGetArgc → &argc");
        table.register("__NSGetArgv", shim_nsgetargv as u64, "_NSGetArgv → &argv");
        table.register(
            "_getenv",
            shim_getenv as u64,
            "getenv → environment variable",
        );

        log::debug!("ProcessShims: registered 5 process identity shims");
    }
}

/// `const char *getprogname(void)`
extern "C" fn shim_getprogname() -> *const std::ffi::c_char {
    let state = PROCESS_STATE.get().expect("process state not initialised");
    state.progname.as_ptr()
}

/// `int _NSGetExecutablePath(char *buf, uint32_t *bufsize)`
///
/// Returns 0 on success, -1 if buffer too small (and sets *bufsize to needed size).
extern "C" fn shim_nsgetexecutablepath(buf: *mut std::ffi::c_char, bufsize: *mut u32) -> i32 {
    let state = PROCESS_STATE.get().expect("process state not initialised");
    let bytes = state.exec_path.as_bytes_with_nul();
    let needed = bytes.len() as u32;

    if buf.is_null() || bufsize.is_null() {
        return -1;
    }

    // SAFETY: bufsize is non-null (checked above), reading a u32.
    let available = unsafe { *bufsize };

    if available < needed {
        // SAFETY: bufsize is non-null (checked above), writing the needed size.
        unsafe { *bufsize = needed };
        return -1;
    }

    // SAFETY: buf is non-null, we checked available >= needed.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
    }
    0
}

/// `int *_NSGetArgc(void)` — returns pointer to argc.
extern "C" fn shim_nsgetargc() -> *const i32 {
    let state = PROCESS_STATE.get().expect("process state not initialised");
    &state.argc as *const i32
}

/// `char ***_NSGetArgv(void)` — returns pointer to argv pointer array.
extern "C" fn shim_nsgetargv() -> *const *const std::ffi::c_char {
    let state = PROCESS_STATE.get().expect("process state not initialised");
    state.argv_ptrs.as_ptr() as *const *const std::ffi::c_char
}

/// `char *getenv(const char *name)`
extern "C" fn shim_getenv(name: *const std::ffi::c_char) -> *const std::ffi::c_char {
    if name.is_null() {
        return std::ptr::null();
    }

    // SAFETY: name is non-null. We assume the caller provides a valid C string.
    let name_cstr = unsafe { std::ffi::CStr::from_ptr(name) };
    let name_bytes = name_cstr.to_bytes();

    let state = PROCESS_STATE.get().expect("process state not initialised");
    for (key, val) in &state.env {
        if key.as_bytes() == name_bytes {
            return val.as_ptr();
        }
    }

    std::ptr::null()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_shims() -> ShimTable {
        let mut table = ShimTable::new();
        let config = ShimConfig {
            process_name: "test-app".to_string(),
            executable_path: "/tmp/test-app".to_string(),
            argc: 2,
            argv: vec!["test-app".to_string(), "--verbose".to_string()],
            env: vec![
                ("HOME".to_string(), "/home/test".to_string()),
                ("PATH".to_string(), "/usr/bin".to_string()),
            ],
        };
        ProcessShims::install(&mut table, &config);
        table
    }

    #[test]
    fn getprogname_registered() {
        let table = setup_shims();
        assert!(table.resolve("_getprogname").is_some());
    }

    #[test]
    fn nsgetexecutablepath_registered() {
        let table = setup_shims();
        assert!(table.resolve("__NSGetExecutablePath").is_some());
    }

    #[test]
    fn nsgetargc_registered() {
        let table = setup_shims();
        assert!(table.resolve("__NSGetArgc").is_some());
    }

    #[test]
    fn getenv_registered() {
        let table = setup_shims();
        assert!(table.resolve("_getenv").is_some());
    }

    #[test]
    fn nsgetargv_registered() {
        let table = setup_shims();
        assert!(table.resolve("__NSGetArgv").is_some());
    }

    #[test]
    fn all_five_shims_registered() {
        let table = setup_shims();
        assert_eq!(table.len(), 5);
    }
}
