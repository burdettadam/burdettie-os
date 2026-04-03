//! dyld image introspection APIs.
//!
//! Provides a proper image registry that can be populated at runtime,
//! replacing the stub-only versions in `mach.rs` (which always return
//! zero/null). The loader calls [`register_image`] when mapping each
//! Mach-O image, and the shims query the registry to answer dyld
//! introspection calls from guest code.

use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

use crate::table::ShimTable;

// ---------------------------------------------------------------------------
// Image registry
// ---------------------------------------------------------------------------

struct ImageInfo {
    header: u64,
    name: CString,
    slide: i64,
}

fn registry() -> &'static Mutex<Vec<ImageInfo>> {
    static INSTANCE: OnceLock<Mutex<Vec<ImageInfo>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(Vec::new()))
}

// ---------------------------------------------------------------------------
// Add-image callback list
// ---------------------------------------------------------------------------

fn add_image_callbacks() -> &'static Mutex<Vec<u64>> {
    static INSTANCE: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(Vec::new()))
}

// ---------------------------------------------------------------------------
// Public API for the loader
// ---------------------------------------------------------------------------

/// Called by the loader when a new Mach-O image is mapped.
pub fn register_image(header: u64, name: &str, slide: i64) {
    let cname = CString::new(name).unwrap_or_else(|_| CString::new("").unwrap());
    let mut reg = registry().lock().unwrap();
    reg.push(ImageInfo {
        header,
        name: cname,
        slide,
    });
    // Fire any registered add-image callbacks for the new image.
    let callbacks = add_image_callbacks().lock().unwrap();
    for &cb in callbacks.iter() {
        if cb != 0 {
            unsafe {
                let f: extern "C" fn(u64, i64) = std::mem::transmute(cb);
                f(header, slide);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shim struct + installation
// ---------------------------------------------------------------------------

pub struct DyldIntrospectionStubs;

impl DyldIntrospectionStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("__dyld_image_count", shim_dyld_image_count as u64, "_dyld_image_count (registry)"),
            ("__dyld_get_image_header", shim_dyld_get_image_header as u64, "_dyld_get_image_header (registry)"),
            ("__dyld_get_image_name", shim_dyld_get_image_name as u64, "_dyld_get_image_name (registry)"),
            ("__dyld_get_image_vmaddr_slide", shim_dyld_get_image_vmaddr_slide as u64, "_dyld_get_image_vmaddr_slide (registry)"),
            ("__dyld_register_func_for_add_image", shim_dyld_register_func_for_add_image as u64, "_dyld_register_func_for_add_image"),
            ("_dyld_image_path_containing_address", shim_dyld_image_path_containing_address as u64, "dyld_image_path_containing_address"),
            ("__NSGetExecutablePath", shim_nsgetexecutablepath as u64, "_NSGetExecutablePath"),
        );
        log::debug!("DyldIntrospectionStubs: registered 7 stubs");
    }
}

// ---------------------------------------------------------------------------
// Shim implementations
// ---------------------------------------------------------------------------

/// `uint32_t _dyld_image_count(void)` — number of registered images.
extern "C" fn shim_dyld_image_count() -> u32 {
    let reg = registry().lock().unwrap();
    let count = reg.len() as u32;
    log::trace!("_dyld_image_count: {}", count);
    count
}

/// `const struct mach_header* _dyld_get_image_header(uint32_t index)` —
/// returns mach_header pointer for image at `index`, or 0.
extern "C" fn shim_dyld_get_image_header(index: u32) -> u64 {
    let reg = registry().lock().unwrap();
    let val = reg.get(index as usize).map_or(0, |info| info.header);
    log::trace!("_dyld_get_image_header({}): {:#x}", index, val);
    val
}

/// `const char* _dyld_get_image_name(uint32_t index)` — returns pointer
/// to the install name C-string, or null.
extern "C" fn shim_dyld_get_image_name(index: u32) -> u64 {
    let reg = registry().lock().unwrap();
    let val = reg
        .get(index as usize)
        .map_or(0, |info| info.name.as_ptr() as u64);
    log::trace!("_dyld_get_image_name({}): {:#x}", index, val);
    val
}

/// `intptr_t _dyld_get_image_vmaddr_slide(uint32_t index)` — returns
/// ASLR slide for image at `index`, or 0.
extern "C" fn shim_dyld_get_image_vmaddr_slide(index: u32) -> i64 {
    let reg = registry().lock().unwrap();
    let val = reg.get(index as usize).map_or(0, |info| info.slide);
    log::trace!("_dyld_get_image_vmaddr_slide({}): {:#x}", index, val);
    val
}

/// `void _dyld_register_func_for_add_image(void (*func)(const mach_header*, intptr_t))`
/// — registers a callback that fires for every future (and already-loaded) image.
extern "C" fn shim_dyld_register_func_for_add_image(func: u64) {
    log::trace!("_dyld_register_func_for_add_image({:#x})", func);
    if func == 0 {
        return;
    }
    // Call the callback for all already-registered images.
    let reg = registry().lock().unwrap();
    for info in reg.iter() {
        unsafe {
            let f: extern "C" fn(u64, i64) = std::mem::transmute(func);
            f(info.header, info.slide);
        }
    }
    drop(reg);
    // Add to callback list for future images.
    let mut cbs = add_image_callbacks().lock().unwrap();
    cbs.push(func);
}

/// `const char* dyld_image_path_containing_address(const void* addr)` —
/// if `addr` matches any image header, return that image's name pointer.
/// Otherwise returns null (0).
extern "C" fn shim_dyld_image_path_containing_address(addr: u64) -> u64 {
    let reg = registry().lock().unwrap();
    for info in reg.iter() {
        if info.header == addr {
            return info.name.as_ptr() as u64;
        }
    }
    log::trace!(
        "dyld_image_path_containing_address({:#x}): not found",
        addr
    );
    0
}

/// `int _NSGetExecutablePath(char *buf, uint32_t *bufsize)` — writes the
/// path of the main executable (image index 0) into `buf`. Returns 0 on
/// success, -1 if the buffer is too small (and writes the required size
/// to `*bufsize`).
extern "C" fn shim_nsgetexecutablepath(buf: u64, bufsize: u64) -> i32 {
    if buf == 0 || bufsize == 0 {
        return -1;
    }

    let reg = registry().lock().unwrap();
    let path = if let Some(info) = reg.first() {
        info.name.as_bytes_with_nul()
    } else {
        b"/usr/bin/unknown\0"
    };

    let bufsize_ptr = bufsize as *mut u32;
    let available = unsafe { *bufsize_ptr } as usize;

    if available < path.len() {
        unsafe {
            *bufsize_ptr = path.len() as u32;
        }
        return -1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(path.as_ptr(), buf as *mut u8, path.len());
    }
    0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Serialize tests that mutate shared global state.
    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Clear the global registries so tests are independent.
    fn reset() {
        registry().lock().unwrap().clear();
        add_image_callbacks().lock().unwrap().clear();
    }

    #[test]
    fn test_initial_count_zero() {
        let _g = test_lock().lock().unwrap();
        reset();
        assert_eq!(shim_dyld_image_count(), 0);
    }

    #[test]
    fn test_register_and_count() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x1000, "/usr/lib/libA.dylib", 0);
        register_image(0x2000, "/usr/lib/libB.dylib", 0);
        assert_eq!(shim_dyld_image_count(), 2);
    }

    #[test]
    fn test_get_image_header() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x1000, "/usr/lib/libSystem.B.dylib", 0x50);
        assert_eq!(shim_dyld_get_image_header(0), 0x1000);
    }

    #[test]
    fn test_get_image_name() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x1000, "/usr/lib/libSystem.B.dylib", 0);
        let ptr = shim_dyld_get_image_name(0);
        assert_ne!(ptr, 0);
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(cstr.to_str().unwrap(), "/usr/lib/libSystem.B.dylib");
    }

    #[test]
    fn test_get_image_slide() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x3000, "/usr/lib/libFoo.dylib", 0x100);
        assert_eq!(shim_dyld_get_image_vmaddr_slide(0), 0x100);
    }

    #[test]
    fn test_out_of_bounds_returns_zero() {
        let _g = test_lock().lock().unwrap();
        reset();
        assert_eq!(shim_dyld_get_image_header(99), 0);
        assert_eq!(shim_dyld_get_image_name(99), 0);
        assert_eq!(shim_dyld_get_image_vmaddr_slide(99), 0);
    }

    #[test]
    fn test_register_add_image_callback() {
        let _g = test_lock().lock().unwrap();
        reset();
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        COUNTER.store(0, Ordering::SeqCst);

        extern "C" fn callback(_header: u64, _slide: i64) {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }

        // Register callback — no images yet, counter stays 0.
        shim_dyld_register_func_for_add_image(callback as u64);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

        // Register an image — callback fires once.
        register_image(0x4000, "/usr/lib/libBar.dylib", 0);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

        // Register another image — callback fires again.
        register_image(0x5000, "/usr/lib/libBaz.dylib", 0);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_nsgetexecutablepath() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x1000, "/usr/lib/myapp", 0);

        let mut buf = [0u8; 256];
        let mut bufsize: u32 = buf.len() as u32;

        let ret = shim_nsgetexecutablepath(
            buf.as_mut_ptr() as u64,
            &mut bufsize as *mut u32 as u64,
        );
        assert_eq!(ret, 0);

        let cstr = unsafe {
            std::ffi::CStr::from_ptr(buf.as_ptr() as *const std::ffi::c_char)
        };
        assert_eq!(cstr.to_str().unwrap(), "/usr/lib/myapp");
    }

    #[test]
    fn test_nsgetexecutablepath_buffer_too_small() {
        let _g = test_lock().lock().unwrap();
        reset();
        register_image(0x1000, "/usr/lib/myapp", 0);

        let mut buf = [0u8; 4];
        let mut bufsize: u32 = buf.len() as u32;

        let ret = shim_nsgetexecutablepath(
            buf.as_mut_ptr() as u64,
            &mut bufsize as *mut u32 as u64,
        );
        assert_eq!(ret, -1);
        // bufsize should now hold the required size (path + nul).
        assert_eq!(bufsize as usize, "/usr/lib/myapp".len() + 1);
    }

    #[test]
    fn test_nsgetexecutablepath_no_images() {
        let _g = test_lock().lock().unwrap();
        reset();

        let mut buf = [0u8; 256];
        let mut bufsize: u32 = buf.len() as u32;

        let ret = shim_nsgetexecutablepath(
            buf.as_mut_ptr() as u64,
            &mut bufsize as *mut u32 as u64,
        );
        assert_eq!(ret, 0);

        let cstr = unsafe {
            std::ffi::CStr::from_ptr(buf.as_ptr() as *const std::ffi::c_char)
        };
        assert_eq!(cstr.to_str().unwrap(), "/usr/bin/unknown");
    }
}
