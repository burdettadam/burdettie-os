//! ObjC exception handling shims: throw, catch, rethrow.
//!
//! Self-hosted using the C++ ABI (`__cxa_throw` / `__cxa_begin_catch` /
//! `__cxa_end_catch` / `__cxa_rethrow`) instead of delegating to the host
//! `libobjc`.  On macOS, we resolve the `objc_ehtype_vtable` data symbol
//! once so that `__objc_personality_v0` (in the mapped binary's eh-frame)
//! can recognise our thrown exception as an ObjC type.

use std::ffi::c_void;

/// Global storage for the current in-flight exception thrown via
/// `objc_exception_throw`. Single-threaded — no locking needed.
static CURRENT_EXCEPTION: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

// ─── C++ ABI imports (libc++abi / libgcc) ──────────────────────────
extern "C" {
    fn __cxa_allocate_exception(thrown_size: usize) -> *mut c_void;
    fn __cxa_throw(
        thrown_exception: *mut c_void,
        tinfo: *mut c_void,
        dest: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> !;
    fn __cxa_begin_catch(exc_buf: *mut c_void) -> *mut c_void;
    fn __cxa_end_catch();
    fn __cxa_rethrow() -> !;
}

// ─── ObjC exception wrapper (matches Apple objc4 layout) ──────────
/// ```text
/// struct objc_exception {
///     id    obj;         // 0: the thrown ObjC object
///     // --- inline objc_typeinfo ---
///     void **vtable;     // 8: &objc_ehtype_vtable[2]
///     char  *name;       // 16: class name (C-string)
///     Class  cls;        // 24: class address for @catch matching
/// };
/// ```
#[repr(C)]
struct ObjcException {
    obj: u64,
    vtable: *const c_void,
    name: *const u8,
    cls: u64,
}

/// Cached `objc_ehtype_vtable + 2` from the host `libobjc`.
#[cfg(target_os = "macos")]
fn ehtype_vtable_plus_2() -> *const c_void {
    use std::sync::OnceLock;
    static VTABLE: OnceLock<usize> = OnceLock::new();
    let addr = *VTABLE.get_or_init(|| {
        let sym = unsafe {
            libc::dlsym(
                libc::RTLD_DEFAULT as *mut c_void,
                c"objc_ehtype_vtable".as_ptr(),
            )
        };
        if sym.is_null() {
            log::warn!("objc_ehtype_vtable not found — @catch type matching may fail");
            0
        } else {
            unsafe { (sym as *const *const c_void).add(2) as usize }
        }
    });
    addr as *const c_void
}

/// Destructor passed to `__cxa_throw`.  The ObjC object itself is
/// managed by the runtime; we only free the wrapper slot.
unsafe extern "C" fn _objc_exception_destructor(_exc: *mut c_void) {}

/// Static class name used when we cannot resolve the real class.
static FALLBACK_CLASS_NAME: &[u8] = b"NSException\0";

/// `void objc_exception_throw(id exception)`
///
/// Throws an ObjC exception using the C++ ABI directly.
pub extern "C" fn shim_objc_exception_throw(exception: u64) {
    log::debug!("shim_objc_exception_throw: exception={:#x}", exception);

    // Store for inspection / fallback paths.
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = Some(exception);
    log::error!("objc_exception_throw: exception={:#x}", exception);

    #[cfg(target_os = "macos")]
    {
        let (cls_addr, cls_name_ptr) = resolve_exception_class(exception);

        unsafe {
            let exc_ptr = __cxa_allocate_exception(std::mem::size_of::<ObjcException>())
                as *mut ObjcException;
            (*exc_ptr).obj = exception;
            (*exc_ptr).vtable = ehtype_vtable_plus_2();
            (*exc_ptr).name = cls_name_ptr;
            (*exc_ptr).cls = cls_addr;

            // tinfo must point to the vtable field (start of objc_typeinfo).
            let tinfo = &(*exc_ptr).vtable as *const _ as *mut c_void;
            __cxa_throw(
                exc_ptr as *mut c_void,
                tinfo,
                Some(_objc_exception_destructor),
            );
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!(
            "*** Terminating app due to uncaught exception (object {:#x})",
            exception
        );
        std::process::abort();
    }
}

/// Resolve the class address and a stable name pointer for an exception
/// object by looking up its ISA in our `ObjcRuntime`.
fn resolve_exception_class(exc: u64) -> (u64, *const u8) {
    if exc == 0 {
        return (0, FALLBACK_CLASS_NAME.as_ptr());
    }

    let guard = match crate::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        // Read ISA with ARM64 mask.
        let raw_isa = unsafe { *(exc as *const u64) };
        let isa = raw_isa & 0x007ffffffffffff8;
        if let Some(_info) = rt.get_class_by_addr(isa) {
            // Use the NSException class address for @catch matching so that
            // `@catch(NSException *)` matches all exception subclasses.
            if let Some(ns_exc) = rt.get_class("NSException") {
                return (ns_exc.mapped_addr, FALLBACK_CLASS_NAME.as_ptr());
            }
            return (isa, FALLBACK_CLASS_NAME.as_ptr());
        }
    }
    (0, FALLBACK_CLASS_NAME.as_ptr())
}

/// `id objc_begin_catch(void *exc_buf)`
///
/// Called at the entry of a `@catch` block.  Delegates to `__cxa_begin_catch`
/// and extracts the ObjC object from our `ObjcException` wrapper.
pub extern "C" fn shim_objc_begin_catch(exc_buf: u64) -> u64 {
    #[cfg(target_os = "macos")]
    {
        let thrown = unsafe { __cxa_begin_catch(exc_buf as *mut c_void) };
        if !thrown.is_null() {
            let exc = thrown as *const ObjcException;
            let obj = unsafe { (*exc).obj };
            if obj != 0 {
                return obj;
            }
        }
    }

    // Fallback: return the stored exception pointer.
    CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0)
}

/// `void objc_end_catch(void)`
///
/// Called at the exit of a `@catch` block.
pub extern "C" fn shim_objc_end_catch() {
    #[cfg(target_os = "macos")]
    {
        unsafe { __cxa_end_catch() };
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// `void objc_exception_rethrow(void)`
///
/// Re-throws the current exception (used by `@finally` blocks).
pub extern "C" fn shim_objc_exception_rethrow() {
    #[cfg(target_os = "macos")]
    {
        unsafe { __cxa_rethrow() };
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("*** objc_exception_rethrow: no active exception to rethrow");
        std::process::abort();
    }
}

/// `void objc_terminate(void)`
///
/// Called when an exception propagates out of a `noexcept`/`nothrow` context.
pub extern "C" fn shim_objc_terminate() {
    let exc = CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(obj) = *exc {
        eprintln!(
            "*** Terminating due to uncaught ObjC exception (object {:#x})",
            obj
        );
    } else {
        eprintln!("*** Terminating due to uncaught ObjC exception");
    }
    std::process::abort();
}
