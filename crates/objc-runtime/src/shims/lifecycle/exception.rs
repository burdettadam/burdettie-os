//! ObjC exception handling shims: throw, catch, rethrow.

/// Global storage for the current in-flight exception thrown via
/// `objc_exception_throw`. Single-threaded — no locking needed.
static CURRENT_EXCEPTION: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

/// Resolve a host symbol by name via the global `HostSymbolResolver`.
/// Falls back to `libc::dlsym` if no resolver is installed.
#[cfg(target_os = "macos")]
fn resolve_host_symbol(name: &str) -> *mut libc::c_void {
    if let Some(resolver) = crate::host_symbols() {
        resolver
            .resolve(name)
            .map(|p| p as *mut libc::c_void)
            .unwrap_or(std::ptr::null_mut())
    } else {
        // Fallback for tests / cases where resolver isn't initialized
        let c_name = std::ffi::CString::new(name).unwrap_or_default();
        unsafe { libc::dlsym((-2isize) as *mut libc::c_void, c_name.as_ptr()) }
    }
}

/// `void objc_exception_throw(id exception)`
///
/// Throws an ObjC exception. On macOS, delegates to the host libobjc's
/// `objc_exception_throw` after converting our cf-lite exception to a
/// real host NSException (needed because the host applies ISA_MASK).
///
/// ## Safety
/// Calls the host's `objc_exception_throw` via dlsym, which will `__cxa_throw`
/// under the hood and unwind through the mapped binary's stack frames.
pub extern "C" fn shim_objc_exception_throw(exception: u64) {
    log::debug!("shim_objc_exception_throw: exception={:#x}", exception);
    // Store for potential inspection
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = Some(exception);

    // Log the exception details
    log::error!("objc_exception_throw: exception={:#x}", exception);

    // Attempt to delegate to host libobjc
    #[cfg(target_os = "macos")]
    {
        type ThrowFn = unsafe extern "C" fn(u64);
        let throw_sym = resolve_host_symbol("objc_exception_throw");
        if throw_sym.is_null() {
            eprintln!("*** Terminating: could not find host objc_exception_throw");
            std::process::abort();
        }

        // Try to create a real host NSException from our cf-lite object.
        // Our cf-lite NSException layout: [isa(8), name(8), reason(8), userInfo(8)]
        let host_exc = create_host_nsexception(exception);

        let throw_fn = unsafe { std::mem::transmute::<*mut libc::c_void, ThrowFn>(throw_sym) };
        unsafe {
            throw_fn(host_exc);
        }
        // objc_exception_throw never returns
        std::process::abort();
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback: abort with message if not on macOS
        eprintln!(
            "*** Terminating app due to uncaught exception (object {:#x})",
            exception
        );
        std::process::abort();
    }
}

/// Create a host NSException from a cf-lite exception object.
///
/// Reads name, reason, and userInfo from the cf-lite layout, then calls
/// the host's Foundation to create a real NSException with proper ISA.
///
/// Falls back to returning the original object if host creation fails.
#[cfg(target_os = "macos")]
fn create_host_nsexception(cf_lite_exc: u64) -> u64 {
    if cf_lite_exc == 0 {
        return 0;
    }

    // Read cf-lite NSException fields:
    //   offset 0: isa (u64)
    //   offset 8: name (u64 — CFString pointer)
    //   offset 16: reason (u64 — CFString pointer, or 0)
    //   offset 24: userInfo (u64 — NSDictionary pointer, or 0)
    let name_cfstr = unsafe { *((cf_lite_exc as usize + 8) as *const u64) };
    let reason_cfstr = unsafe { *((cf_lite_exc as usize + 16) as *const u64) };

    // Resolve host APIs via HostSymbolResolver
    let objc_get_class = resolve_host_symbol("objc_getClass");
    let sel_register = resolve_host_symbol("sel_registerName");
    let msg_send = resolve_host_symbol("objc_msgSend");

    if objc_get_class.is_null() || sel_register.is_null() || msg_send.is_null() {
        return cf_lite_exc;
    }

    type GetClassFn = unsafe extern "C" fn(*const i8) -> u64;
    type SelRegFn = unsafe extern "C" fn(*const i8) -> u64;
    type MsgSend5Fn = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
    type MsgSend3PtrFn = unsafe extern "C" fn(u64, u64, *const i8) -> u64;

    let get_class: GetClassFn = unsafe { std::mem::transmute(objc_get_class) };
    let sel_reg: SelRegFn = unsafe { std::mem::transmute(sel_register) };
    let send5: MsgSend5Fn = unsafe { std::mem::transmute(msg_send) };
    let send3ptr: MsgSend3PtrFn = unsafe { std::mem::transmute(msg_send) };

    // Convert cf-lite CFStrings to host NSStrings.
    let name_host = cfstring_to_host_nsstring(name_cfstr, get_class, sel_reg, send3ptr);
    let reason_host = cfstring_to_host_nsstring(reason_cfstr, get_class, sel_reg, send3ptr);

    let exc_class = unsafe { get_class(c"NSException".as_ptr()) };
    if exc_class == 0 {
        return cf_lite_exc;
    }

    let sel = unsafe { sel_reg(c"exceptionWithName:reason:userInfo:".as_ptr()) };
    let host_exc = unsafe { send5(exc_class, sel, name_host, reason_host, 0) };
    if host_exc == 0 {
        return cf_lite_exc;
    }

    log::debug!("create_host_nsexception: {host_exc:#x}");
    host_exc
}

/// Convert a cf-lite CFString to a host NSString via C string extraction.
///
/// CFString constant layout: [isa(8), flags(8), data_ptr(8), length(8)]
/// We read data_ptr (a raw C string in mapped __TEXT) and call
/// `[NSString stringWithUTF8String:]` on the host.
#[cfg(target_os = "macos")]
fn cfstring_to_host_nsstring(
    cfstr: u64,
    get_class: unsafe extern "C" fn(*const i8) -> u64,
    sel_reg: unsafe extern "C" fn(*const i8) -> u64,
    send3ptr: unsafe extern "C" fn(u64, u64, *const i8) -> u64,
) -> u64 {
    if cfstr == 0 {
        return 0;
    }

    // data_ptr is at offset 16 in the CFString struct
    let data_ptr = unsafe { *((cfstr as usize + 16) as *const *const i8) };
    if data_ptr.is_null() {
        return 0;
    }

    let nsstring_class = unsafe { get_class(c"NSString".as_ptr()) };
    if nsstring_class == 0 {
        return 0;
    }
    let sel = unsafe { sel_reg(c"stringWithUTF8String:".as_ptr()) };
    unsafe { send3ptr(nsstring_class, sel, data_ptr) }
}

/// `id objc_begin_catch(void *exc_buf)`
///
/// Called at the entry of a `@catch` block. Delegates to host libobjc.
pub extern "C" fn shim_objc_begin_catch(exc_buf: u64) -> u64 {
    #[cfg(target_os = "macos")]
    {
        type BeginCatchFn = unsafe extern "C" fn(u64) -> u64;
        let sym = resolve_host_symbol("objc_begin_catch");
        if !sym.is_null() {
            return unsafe { std::mem::transmute::<*mut libc::c_void, BeginCatchFn>(sym)(exc_buf) };
        }
    }

    // Fallback: return the stored exception pointer
    CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0)
}

/// `void objc_end_catch(void)`
///
/// Called at the exit of a `@catch` block. Delegates to host libobjc.
pub extern "C" fn shim_objc_end_catch() {
    #[cfg(target_os = "macos")]
    {
        type EndCatchFn = unsafe extern "C" fn();
        let sym = resolve_host_symbol("objc_end_catch");
        if !sym.is_null() {
            unsafe {
                std::mem::transmute::<*mut libc::c_void, EndCatchFn>(sym)();
            }
            return;
        }
    }

    // Fallback: clear the current exception
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// `void objc_exception_rethrow(void)`
///
/// Re-throws the current exception (used by `@finally` blocks).
pub extern "C" fn shim_objc_exception_rethrow() {
    #[cfg(target_os = "macos")]
    {
        type RethrowFn = unsafe extern "C" fn();
        let sym = resolve_host_symbol("objc_exception_rethrow");
        if !sym.is_null() {
            unsafe {
                std::mem::transmute::<*mut libc::c_void, RethrowFn>(sym)();
            }
            // rethrow never returns
            std::process::abort();
        }
    }

    eprintln!("*** objc_exception_rethrow: no active exception to rethrow");
    std::process::abort();
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
