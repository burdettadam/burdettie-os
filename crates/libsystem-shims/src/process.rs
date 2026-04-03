//! Process identity shims.
//!
//! These shims respond to queries about "who am I?" that every C runtime
//! program makes at startup: process name, executable path, argument
//! count/vector, and environment variables.

use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

use crate::table::ShimTable;
use crate::ShimConfig;

/// Mutable environment overrides (set via setenv/putenv at runtime).
static ENV_OVERRIDES: Mutex<Vec<(CString, CString)>> = Mutex::new(Vec::new());

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
    /// Stored `argv` head pointer so `_NSGetArgv` can return `&argv`.
    argv_head: usize,
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

/// Returns the configured argv strings set during shim installation.
pub fn process_args() -> Option<Vec<String>> {
    PROCESS_STATE.get().map(|s| {
        s.argv_cstrings
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    })
}

/// Clear runtime environment overrides accumulated via `setenv`/`putenv`.
pub fn clear_env_overrides() {
    let mut overrides = ENV_OVERRIDES.lock().unwrap_or_else(|e| e.into_inner());
    overrides.clear();
}

/// Install process identity shims into a `ShimTable`.
pub struct ProcessShims;

fn collect_execv_args(path: &CString, argv: *const *const std::ffi::c_char) -> Vec<CString> {
    let mut collected = Vec::new();

    if !argv.is_null() {
        let mut current = argv;
        while !current.is_null() {
            let arg_ptr = unsafe { *current };
            if arg_ptr.is_null() {
                break;
            }
            let arg = unsafe { std::ffi::CStr::from_ptr(arg_ptr) };
            collected.push(CString::new(arg.to_bytes()).unwrap_or_default());
            current = unsafe { current.add(1) };
            if collected.len() >= 256 {
                break;
            }
        }
    }

    if collected.is_empty() {
        collected.push(path.clone());
    }

    if collected.len() <= 1 {
        if let Some(current_args) = process_args() {
            if current_args.len() > 1 {
                collected.truncate(1);
                collected[0] = path.clone();
                for arg in current_args.iter().skip(1) {
                    collected.push(CString::new(arg.as_str()).unwrap_or_default());
                }
            }
        }
    }

    collected
}

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
            let argv_head = argv_ptrs.as_ptr() as usize;

            ProcessState {
                progname: CString::new(config.process_name.clone())
                    .unwrap_or_else(|_| CString::new("app").unwrap()),
                exec_path: CString::new(config.executable_path.clone())
                    .unwrap_or_else(|_| CString::new("/app").unwrap()),
                argc: config.argc,
                argv_cstrings,
                argv_ptrs,
                argv_head,
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
        table.register("_setenv", shim_setenv as u64, "setenv → set env var");
        table.register("_putenv", shim_putenv as u64, "putenv → KEY=VALUE env");
        table.register("_unsetenv", shim_unsetenv as u64, "unsetenv → remove env var");
        table.register("_execv", shim_execv as u64, "execv → preserve guest argv");
        table.register("_dladdr", shim_dladdr as u64, "dladdr → guest-aware address lookup");

        log::debug!("ProcessShims: registered 10 process identity shims");
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

/// `char ***_NSGetArgv(void)` — returns pointer to the `argv` pointer.
extern "C" fn shim_nsgetargv() -> *const *const *const std::ffi::c_char {
    let state = PROCESS_STATE.get().expect("process state not initialised");
    &state.argv_head as *const usize as *const *const *const std::ffi::c_char
}

/// `char *getenv(const char *name)`
extern "C" fn shim_getenv(name: *const std::ffi::c_char) -> *const std::ffi::c_char {
    if name.is_null() {
        return std::ptr::null();
    }

    // SAFETY: name is non-null. We assume the caller provides a valid C string.
    let name_cstr = unsafe { std::ffi::CStr::from_ptr(name) };
    let name_bytes = name_cstr.to_bytes();

    // 1. Check runtime overrides (setenv/putenv) first.
    if let Ok(overrides) = ENV_OVERRIDES.lock() {
        for (key, val) in overrides.iter().rev() {
            if key.as_bytes() == name_bytes {
                return val.as_ptr();
            }
        }
    }

    // 2. Check ShimConfig env (set at startup).
    if let Some(state) = PROCESS_STATE.get() {
        for (key, val) in &state.env {
            if key.as_bytes() == name_bytes {
                return val.as_ptr();
            }
        }
    }

    // 3. Fall through to host process environment (gives DEVELOPER_DIR, PATH, SDKROOT, etc.)
    unsafe { libc::getenv(name) }
}

/// `int setenv(const char *name, const char *value, int overwrite)`
extern "C" fn shim_setenv(
    name: *const std::ffi::c_char,
    value: *const std::ffi::c_char,
    overwrite: i32,
) -> i32 {
    if name.is_null() || value.is_null() {
        return -1;
    }
    let name_cstr = unsafe { std::ffi::CStr::from_ptr(name) };
    let val_cstr = unsafe { std::ffi::CStr::from_ptr(value) };
    let Ok(mut overrides) = ENV_OVERRIDES.lock() else { return -1; };
    let name_bytes = name_cstr.to_bytes();
    if let Some(existing) = overrides.iter_mut().find(|(k, _)| k.as_bytes() == name_bytes) {
        if overwrite != 0 {
            existing.1 = CString::from(val_cstr);
        }
        return 0;
    }
    overrides.push((CString::from(name_cstr), CString::from(val_cstr)));
    0
}

/// `int putenv(char *str)` — accepts "KEY=VALUE" or "KEY=" strings.
extern "C" fn shim_putenv(str: *mut std::ffi::c_char) -> i32 {
    if str.is_null() { return -1; }
    let s = unsafe { std::ffi::CStr::from_ptr(str) };
    let bytes = s.to_bytes();
    if let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') {
        let key = &bytes[..eq_pos];
        let val = &bytes[eq_pos + 1..];
        let Ok(key_cs) = CString::new(key) else { return -1; };
        let Ok(val_cs) = CString::new(val) else { return -1; };
        let Ok(mut overrides) = ENV_OVERRIDES.lock() else { return -1; };
        if let Some(existing) = overrides.iter_mut().find(|(k, _)| k.as_bytes() == key) {
            existing.1 = val_cs;
        } else {
            overrides.push((key_cs, val_cs));
        }
        0
    } else {
        -1
    }
}

/// `int unsetenv(const char *name)`
extern "C" fn shim_unsetenv(name: *const std::ffi::c_char) -> i32 {
    if name.is_null() { return -1; }
    let name_cstr = unsafe { std::ffi::CStr::from_ptr(name) };
    let name_bytes = name_cstr.to_bytes();
    let Ok(mut overrides) = ENV_OVERRIDES.lock() else { return -1; };
    overrides.retain(|(k, _)| k.as_bytes() != name_bytes);
    0
}

/// `int execv(const char *path, char *const argv[])`
extern "C" fn shim_execv(
    path: *const std::ffi::c_char,
    argv: *const *const std::ffi::c_char,
) -> i32 {
    if path.is_null() {
        return -1;
    }

    let path_cstr = unsafe { std::ffi::CStr::from_ptr(path) };
    let path_owned = CString::new(path_cstr.to_bytes()).unwrap_or_default();
    let args = collect_execv_args(&path_owned, argv);
    let mut arg_ptrs: Vec<*const std::ffi::c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    arg_ptrs.push(std::ptr::null());

    unsafe { libc::execv(path_owned.as_ptr(), arg_ptrs.as_ptr()) }
}

/// `int dladdr(const void *addr, Dl_info *info)`
///
/// Guest-aware address lookup.  First try the host `dladdr`; if that
/// fails (e.g. because the address is in guest-mapped memory), fill in
/// `Dl_info` with the guest executable path so that LLVM/Clang path
/// computations produce valid results instead of giving up.
extern "C" fn shim_dladdr(addr: *const u8, info: *mut libc::Dl_info) -> i32 {
    if info.is_null() {
        return 0;
    }

    // Try the real dladdr first — succeeds for host library addresses.
    let host_result = unsafe { libc::dladdr(addr as *const _, info) };
    if host_result != 0 {
        return host_result;
    }

    // Host dladdr failed — the address is likely in guest-mapped memory.
    // Return the guest executable path so LLVM's getMainExecutable() and
    // Clang's resource-dir computation get a valid result.
    let state = PROCESS_STATE.get().expect("process state not initialised");
    unsafe {
        let p = &mut *info;
        p.dli_fname = state.exec_path.as_ptr();
        p.dli_fbase = std::ptr::null_mut();
        p.dli_sname = std::ptr::null();
        p.dli_saddr = std::ptr::null_mut();
    }
    log::trace!(
        "dladdr: guest fallback for {:?} → {}",
        addr,
        state.exec_path.to_string_lossy()
    );
    1 // success
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
    fn process_args_reflects_configured_argv() {
        let _table = setup_shims();
        let args = process_args().expect("process args should be initialised");
        assert!(!args.is_empty(), "process args should include argv[0]");
        assert_eq!(args[0], "test-app");
    }

    #[test]
    fn nsgetargv_returns_pointer_to_argv_head() {
        let _table = setup_shims();
        let argv_ptr = shim_nsgetargv();
        assert!(!argv_ptr.is_null());
        let argv = unsafe { *argv_ptr };
        assert!(!argv.is_null());
        let first = unsafe { std::ffi::CStr::from_ptr(*argv) }
            .to_str()
            .unwrap();
        assert_eq!(first, "test-app");
    }

    #[test]
    fn all_shims_registered() {
        let table = setup_shims();
        assert_eq!(table.len(), 10);
    }

    #[test]
    fn collect_execv_args_preserves_guest_tail_when_only_argv0_is_forwarded() {
        let _table = setup_shims();
        let path = CString::new("/real/altool").unwrap();
        let argv = [path.as_ptr(), std::ptr::null()];
        let args = collect_execv_args(&path, argv.as_ptr());
        let as_strings: Vec<_> = args
            .iter()
            .map(|arg| arg.to_str().unwrap().to_string())
            .collect();
        assert_eq!(as_strings, vec!["/real/altool".to_string(), "--verbose".to_string()]);
    }

    #[test]
    fn collect_execv_args_keeps_existing_extra_args() {
        let _table = setup_shims();
        let path = CString::new("/real/altool").unwrap();
        let help = CString::new("--help").unwrap();
        let argv = [path.as_ptr(), help.as_ptr(), std::ptr::null()];
        let args = collect_execv_args(&path, argv.as_ptr());
        let as_strings: Vec<_> = args
            .iter()
            .map(|arg| arg.to_str().unwrap().to_string())
            .collect();
        assert_eq!(as_strings, vec!["/real/altool".to_string(), "--help".to_string()]);
    }

    #[test]
    fn clear_env_overrides_empties_runtime_entries() {
        let _table = setup_shims();
        let key = CString::new("DARWIN_LOADER_TEST_OVERRIDE").unwrap();
        let value = CString::new("1").unwrap();

        assert_eq!(shim_setenv(key.as_ptr(), value.as_ptr(), 1), 0);
        assert_eq!(ENV_OVERRIDES.lock().unwrap_or_else(|e| e.into_inner()).len(), 1);

        clear_env_overrides();

        assert!(ENV_OVERRIDES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty());
    }
}
