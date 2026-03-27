//! Linux symbol resolution via dlsym/dlopen.

use platform_core::symbols::HostSymbolResolver;

pub struct LinuxSymbolResolver {
    default_handle: *mut libc::c_void,
}

// The default_handle is RTLD_DEFAULT which is a process-wide constant.
unsafe impl Send for LinuxSymbolResolver {}
unsafe impl Sync for LinuxSymbolResolver {}

impl LinuxSymbolResolver {
    pub const fn new() -> Self {
        Self {
            // glibc RTLD_DEFAULT is 0 (null); musl also uses 0.
            default_handle: std::ptr::null_mut(),
        }
    }
}

impl Default for LinuxSymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl HostSymbolResolver for LinuxSymbolResolver {
    fn resolve(&self, name: &str) -> Option<*const ()> {
        let c_str = std::ffi::CString::new(name).ok()?;
        let addr = unsafe { libc::dlsym(self.default_handle, c_str.as_ptr()) };
        if addr.is_null() {
            None
        } else {
            Some(addr as *const ())
        }
    }

    fn resolve_in(&self, handle: *mut (), name: &str) -> Option<*const ()> {
        let c_str = std::ffi::CString::new(name).ok()?;
        let addr = unsafe { libc::dlsym(handle as *mut libc::c_void, c_str.as_ptr()) };
        if addr.is_null() {
            None
        } else {
            Some(addr as *const ())
        }
    }

    fn open_library(&self, path: &str) -> Option<*mut ()> {
        let c_str = std::ffi::CString::new(path).ok()?;
        let handle = unsafe { libc::dlopen(c_str.as_ptr(), libc::RTLD_LAZY) };
        if handle.is_null() {
            None
        } else {
            Some(handle as *mut ())
        }
    }

    fn default_handle(&self) -> *mut () {
        self.default_handle as *mut ()
    }
}
