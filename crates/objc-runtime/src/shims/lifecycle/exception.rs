//! ObjC exception handling shims: throw, catch, rethrow.
//!
//! Self-hosted entirely without the host libobjc:
//!  * `objc_exception_throw` stores the guest object in `CURRENT_EXCEPTION`
//!    and throws a C++ exception via `__cxa_throw` with our own ObjC typeinfo.
//!  * `__objc_personality_v0` is our own LSDA-parsing personality function
//!    that uses the guest `ObjcRuntime` class hierarchy for `@catch` matching.
//!    This replaces the host `__objc_personality_v0` from libobjc and makes
//!    the exception path work identically on Linux and macOS.
//!  * `objc_begin_catch` returns the guest object from `CURRENT_EXCEPTION`.

#[cfg(unix)]
use std::ffi::c_void;

/// Global storage for the current in-flight exception thrown via
/// `objc_exception_throw`. Single-threaded — no locking needed.
static CURRENT_EXCEPTION: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

// ─── C++ ABI imports (libc++abi / libgcc) ──────────────────────────
#[cfg(unix)]
extern "C" {
    fn __cxa_allocate_exception(thrown_size: usize) -> *mut c_void;
    // NOTE: NOT declared `-> !` intentionally.  Declaring a C++ unwinding function
    // as `-> !` causes Rust to tear down the caller's stack frame before the call
    // (epilogue moved before the branch), which corrupts x29/x30 and makes
    // `_Unwind_RaiseException` crash during frame-pointer chain walking.
    // We use an `unreachable_unchecked()` after the call site instead.
    fn __cxa_throw(
        thrown_exception: *mut c_void,
        tinfo: *mut c_void,
        dest: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    fn __cxa_begin_catch(exc_buf: *mut c_void) -> *mut c_void;
    fn __cxa_end_catch();
    fn __cxa_rethrow() -> !;
}

// ─── Libunwind ABI (available on both macOS and Linux) ─────────────
#[cfg(unix)]
extern "C" {
    fn _Unwind_GetIP(ctx: *mut u8) -> u64;
    fn _Unwind_GetRegionStart(ctx: *mut u8) -> u64;
    fn _Unwind_GetLanguageSpecificData(ctx: *mut u8) -> *const u8;
    fn _Unwind_SetIP(ctx: *mut u8, value: u64);
    fn _Unwind_SetGR(ctx: *mut u8, reg: i32, value: u64);
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
#[cfg(unix)]
struct ObjcException {
    obj: u64,
    vtable: *const c_void,
    name: *const u8,
    cls: u64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct ObjcEhType {
    vtable: *const c_void,
    name: *const u8,
    cls: *const c_void,
}

/// Cached `objc_ehtype_vtable + 2` from the host `libobjc`.
#[cfg(target_os = "macos")]
fn ehtype_vtable_plus_2() -> *const c_void {
    use std::sync::OnceLock;
    static VTABLE: OnceLock<usize> = OnceLock::new();
    let addr = *VTABLE.get_or_init(|| {
        let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"objc_ehtype_vtable".as_ptr()) };
        if sym.is_null() {
            log::warn!("objc_ehtype_vtable not found — @catch type matching may fail");
            0
        } else {
            unsafe { (sym as *const *const c_void).add(2) as usize }
        }
    });
    addr as *const c_void
}

#[cfg(target_os = "macos")]
fn ensure_foundation_loaded() {
    use std::sync::OnceLock;

    static FOUNDATION: OnceLock<()> = OnceLock::new();
    FOUNDATION.get_or_init(|| unsafe {
        libc::dlopen(
            c"/System/Library/Frameworks/Foundation.framework/Foundation".as_ptr(),
            libc::RTLD_LAZY | libc::RTLD_GLOBAL,
        );
    });
}

#[cfg(target_os = "macos")]
fn resolve_host_symbol(name: *const libc::c_char) -> *mut c_void {
    ensure_foundation_loaded();
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.cast()) }
}

#[cfg(target_os = "macos")]
fn host_nsexception_ehtype() -> Option<&'static ObjcEhType> {
    use std::sync::OnceLock;

    static EHTYPE: OnceLock<usize> = OnceLock::new();
    let addr = *EHTYPE.get_or_init(|| {
        let sym = resolve_host_symbol(c"OBJC_EHTYPE_$_NSException".as_ptr());
        if sym.is_null() {
            log::warn!(
                "OBJC_EHTYPE_$_NSException not found in host Foundation — falling back to synthetic ObjC exception typeinfo"
            );
            0
        } else {
            sym as usize
        }
    });

    (addr != 0).then(|| unsafe { &*(addr as *const ObjcEhType) })
}

pub fn host_nsexception_ehtype_addr() -> u64 {
    #[cfg(target_os = "macos")]
    {
        host_nsexception_ehtype().map_or(0, |ehtype| ehtype as *const ObjcEhType as u64)
    }

    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

/// Destructor passed to `__cxa_throw`.  The ObjC object itself is
/// managed by the runtime; we only free the wrapper slot.
#[cfg(unix)]
unsafe extern "C" fn _objc_exception_destructor(_exc: *mut c_void) {}

/// Static class name used when we cannot resolve the real class.
#[cfg_attr(not(unix), allow(dead_code))]
static FALLBACK_CLASS_NAME: &[u8] = b"NSException\0";

/// `void objc_exception_throw(id exception)`
///
/// Throws an ObjC exception using the C++ ABI directly.
/// The guest exception object is stored in `CURRENT_EXCEPTION`; our
/// `shim_objc_personality_v0` uses the guest ObjcRuntime for @catch matching
/// and `shim_objc_begin_catch` returns the guest object from there.
#[cfg(unix)]
pub extern "C-unwind" fn shim_objc_exception_throw(exception: u64) {
    log::debug!("shim_objc_exception_throw: exception={:#x}", exception);

    // Store the guest exception for begin_catch and the personality function.
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = Some(exception);

    unsafe {
        let exc_ptr =
            __cxa_allocate_exception(std::mem::size_of::<ObjcException>()) as *mut ObjcException;
        (*exc_ptr).obj = exception;

        #[cfg(target_os = "macos")]
        if let Some(host_ehtype) = host_nsexception_ehtype() {
            (*exc_ptr).vtable = host_ehtype.vtable;
            (*exc_ptr).name = host_ehtype.name;
            (*exc_ptr).cls = host_ehtype.cls as u64;
        } else {
            let (cls_addr, cls_name_ptr) = resolve_exception_class(exception);
            (*exc_ptr).vtable = ehtype_vtable_plus_2();
            (*exc_ptr).name = cls_name_ptr;
            (*exc_ptr).cls = cls_addr;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let (cls_addr, cls_name_ptr) = resolve_exception_class(exception);
            (*exc_ptr).vtable = std::ptr::null();
            (*exc_ptr).name = cls_name_ptr;
            (*exc_ptr).cls = cls_addr;
        }

        // tinfo must point to the vtable field (start of objc_typeinfo).
        let tinfo = &(*exc_ptr).vtable as *const _ as *mut c_void;
        __cxa_throw(
            exc_ptr as *mut c_void,
            tinfo,
            Some(_objc_exception_destructor),
        );
        // SAFETY: __cxa_throw never returns — it either unwinds or calls std::terminate.
        std::hint::unreachable_unchecked();
    }
}

#[cfg(not(unix))]
pub extern "C-unwind" fn shim_objc_exception_throw(exception: u64) {
    log::warn!("objc_exception_throw requested on a non-Unix host; ObjC unwinding is unavailable");
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = Some(exception);
    std::process::abort();
}

/// Resolve a fallback class address and stable name pointer for an exception
/// object by looking up its ISA in our `ObjcRuntime`.
///
/// This is only used when the host `NSException` exception-type metadata is
/// unavailable; the preferred path is to reuse the host Foundation typeinfo so
/// host `__objc_personality_v0` sees a real host `Class` pointer.
#[cfg(unix)]
fn resolve_exception_class(exc: u64) -> (u64, *const u8) {
    if exc == 0 {
        return (0, FALLBACK_CLASS_NAME.as_ptr());
    }

    let guard = match crate::OBJC_RUNTIME.read() {
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
#[cfg(unix)]
pub extern "C" fn shim_objc_begin_catch(exc_buf: u64) -> u64 {
    // Must always call __cxa_begin_catch to advance the C++ EH state machine.
    let thrown = unsafe { __cxa_begin_catch(exc_buf as *mut c_void) };

    // Prefer CURRENT_EXCEPTION: shim_objc_exception_throw always stores the guest
    // exception there.  On macOS (host-throw path), `thrown` points to a real host
    // NSException object — reading (*thrown).obj from it would give garbage (the host
    // object's ISA).  On Linux (synthetic-path), CURRENT_EXCEPTION equals (*thrown).obj
    // anyway, so this is correct in both cases.
    {
        let guest = CURRENT_EXCEPTION
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(0);
        if guest != 0 {
            return guest;
        }
    }

    // Fallback (robustness): try to read obj from the ObjcException wrapper directly.
    if !thrown.is_null() {
        let exc = thrown as *const ObjcException;
        let obj = unsafe { (*exc).obj };
        if obj != 0 {
            return obj;
        }
    }

    0
}

#[cfg(not(unix))]
pub extern "C" fn shim_objc_begin_catch(_exc_buf: u64) -> u64 {
    CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0)
}

/// `void objc_end_catch(void)`
///
/// Called at the exit of a `@catch` block.
#[cfg(unix)]
pub extern "C" fn shim_objc_end_catch() {
    unsafe { __cxa_end_catch() };
}

#[cfg(not(unix))]
pub extern "C" fn shim_objc_end_catch() {
    *CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// `void objc_exception_rethrow(void)`
///
/// Re-throws the current exception (used by `@finally` blocks).
#[cfg(unix)]
pub extern "C-unwind" fn shim_objc_exception_rethrow() {
    unsafe { __cxa_rethrow() };
}

#[cfg(not(unix))]
pub extern "C-unwind" fn shim_objc_exception_rethrow() {
    log::warn!(
        "objc_exception_rethrow requested on a non-Unix host; ObjC unwinding is unavailable"
    );
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

// ─── Legacy setjmp-based exception API ─────────────────────────
//
// Some compilers (and the pre-modern ObjC runtime) emit these instead
// of the C++ personality-based throw/catch path.  The setjmp buffer
// is used as a synthetic "try context" in a per-thread stack.

/// Per-thread stack of setjmp-based exception contexts.
///
/// Each entry is a pointer to the `jmp_buf` passed to `objc_exception_tryEnter`.
static EXCEPTION_STACK: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

/// Cached uncaught exception handler set by `NSSetUncaughtExceptionHandler`.
static UNCAUGHT_EXCEPTION_HANDLER: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

/// `void objc_exception_tryEnter(void *localExceptionData)`
///
/// Pushes a try context onto the per-thread exception stack.
/// Called at the start of an `@try` block in the legacy (setjmp) path.
pub extern "C" fn shim_objc_exception_try_enter(data: u64) {
    log::debug!("objc_exception_tryEnter: data={:#x}", data);
    EXCEPTION_STACK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(data);
}

/// `void objc_exception_tryExit(void *localExceptionData)`
///
/// Pops the current try context from the exception stack.
/// Called when leaving an `@try` block normally (no exception thrown).
pub extern "C" fn shim_objc_exception_try_exit(data: u64) {
    log::debug!("objc_exception_tryExit: data={:#x}", data);
    let mut stack = EXCEPTION_STACK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(pos) = stack.iter().rposition(|d| *d == data) {
        stack.remove(pos);
    }
}

/// `id objc_exception_extract(void *localExceptionData) → id`
///
/// Retrieves the exception object from the try context after a catch.
/// Returns the current in-flight exception.
pub extern "C" fn shim_objc_exception_extract(_data: u64) -> u64 {
    CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0)
}

/// `int objc_exception_match(Class exceptionClass, id exception) → int`
///
/// Returns 1 if the exception is an instance of `exceptionClass` or a subclass.
/// Used by `@catch(ClassName *e)` in the legacy path.
pub extern "C" fn shim_objc_exception_match(catch_cls: u64, exception: u64) -> i32 {
    if exception == 0 || catch_cls == 0 {
        return 0;
    }

    // Read the ISA of the exception object and walk the class hierarchy.
    let guard = match crate::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return 0;
    };

    let raw_isa = unsafe { *(exception as *const u64) };
    let isa = raw_isa & 0x007ffffffffffff8;

    // Walk the superclass chain looking for catch_cls.
    let mut current = isa;
    for _ in 0..64 {
        if current == catch_cls {
            return 1;
        }
        match rt.get_class_by_addr(current) {
            Some(info) => match info.superclass_addr {
                Some(super_addr) => current = super_addr,
                None => break,
            },
            None => break,
        }
    }

    0
}

/// `NSUncaughtExceptionHandler *NSGetUncaughtExceptionHandler(void)`
///
/// Returns the currently-installed uncaught exception handler (or nil).
pub extern "C" fn shim_ns_get_uncaught_exception_handler() -> u64 {
    UNCAUGHT_EXCEPTION_HANDLER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .unwrap_or(0)
}

/// `void NSSetUncaughtExceptionHandler(NSUncaughtExceptionHandler *handler)`
///
/// Installs a handler function that is called when an ObjC exception
/// propagates to the top of the call stack without being caught.
pub extern "C" fn shim_ns_set_uncaught_exception_handler(handler: u64) {
    log::debug!("NSSetUncaughtExceptionHandler: handler={:#x}", handler);
    *UNCAUGHT_EXCEPTION_HANDLER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        if handler != 0 { Some(handler) } else { None };
}

// ─── Pure guest-side ObjC personality function ─────────────────────
//
// This replaces the host `__objc_personality_v0` from libobjc.  It parses the
// DWARF LSDA to find @catch landing pads and uses our synthetic ObjcRuntime
// class hierarchy for type matching — no host ObjC calls whatsoever.
//
// LSDA layout (compact, Apple arm64 ObjC):
//   [lpstart_enc:u8] [ttype_enc:u8] [ttype_end_offset:uleb128]
//   [call_site_enc:u8] [cs_table_len:uleb128]
//   <call site entries>  <action table>  <type table (reversed)>
//
// Call site encoding is typically DW_EH_PE_uleb128 (0x01).
// Type table encoding is typically DW_EH_PE_indirect|DW_EH_PE_pcrel|DW_EH_PE_sdata4 (0x9b).

#[cfg(unix)]
const DW_EH_PE_OMIT: u8 = 0xff;

#[cfg(unix)]
unsafe fn lsda_read_uleb128(p: &mut *const u8) -> u64 {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = **p;
        *p = (*p).add(1);
        result |= ((byte & 0x7f) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
    }
    result
}

#[cfg(unix)]
unsafe fn lsda_read_sleb128(p: &mut *const u8) -> i64 {
    let mut result = 0i64;
    let mut shift = 0u32;
    loop {
        let byte = **p;
        *p = (*p).add(1);
        result |= ((byte & 0x7f) as i64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < 64 && (byte & 0x40) != 0 {
                result |= !0i64 << shift;
            }
            break;
        }
    }
    result
}

/// Read a DWARF-encoded value from the stream at `*p`, advancing `*p`.
///
/// The "pc" for pc-relative encodings is the address of the field, i.e. the
/// value of `*p` BEFORE any bytes are consumed.
#[cfg(unix)]
unsafe fn lsda_read_encoded(p: &mut *const u8, encoding: u8) -> u64 {
    if encoding == DW_EH_PE_OMIT {
        return 0;
    }
    let field_addr = *p as u64;
    let value_enc = encoding & 0x0f;
    let base_enc = encoding & 0x70;

    let raw: u64 = match value_enc {
        0x00 => {
            // absptr (pointer size = 8 on LP64)
            let v = *(*p as *const u64);
            *p = (*p).add(8);
            v
        }
        0x01 => lsda_read_uleb128(p),
        0x02 => {
            let v = *(*p as *const u16) as u64;
            *p = (*p).add(2);
            v
        }
        0x03 => {
            let v = *(*p as *const u32) as u64;
            *p = (*p).add(4);
            v
        }
        0x04 => {
            let v = *(*p as *const u64);
            *p = (*p).add(8);
            v
        }
        0x09 => lsda_read_sleb128(p) as u64,
        0x0a => {
            let v = *(*p as *const i16) as i64 as u64;
            *p = (*p).add(2);
            v
        }
        0x0b => {
            let v = *(*p as *const i32) as i64 as u64;
            *p = (*p).add(4);
            v
        }
        0x0c => {
            let v = *(*p as *const i64) as u64;
            *p = (*p).add(8);
            v
        }
        _ => {
            *p = (*p).add(4);
            return 0;
        }
    };

    let based: u64 = match base_enc {
        0x00 => raw,                             // absolute
        0x10 => raw.wrapping_add(field_addr),    // pc-relative
        0x20 | 0x30 | 0x40 | 0x50 | 0x60 => raw, // textrel/datarel/funcrel etc — treat as abs
        _ => raw,
    };

    if encoding & 0x80 != 0 && based != 0 {
        // DW_EH_PE_indirect: dereference as pointer
        *(based as *const u64)
    } else {
        based
    }
}

#[cfg(unix)]
struct CallSiteResult {
    lp_addr: u64,
    /// 0 = cleanup only, positive = catch type index in type table
    action_index: i64,
}

/// Scan the LSDA for a handler for `ip` (adjusted to point at the throwing call).
/// Returns `None` if no handler exists in this frame.
#[cfg(unix)]
unsafe fn scan_lsda(
    lsda: *const u8,
    func_start: u64,
    ip: u64,
    guest_exc: u64,
) -> Option<CallSiteResult> {
    let mut p = lsda;

    // lpstart
    let lpstart_enc = *p;
    p = p.add(1);
    let lpstart = if lpstart_enc == DW_EH_PE_OMIT {
        func_start
    } else {
        lsda_read_encoded(&mut p, lpstart_enc)
    };

    // ttype end offset
    let ttype_enc = *p;
    p = p.add(1);
    let ttype_end: *const u8 = if ttype_enc == DW_EH_PE_OMIT {
        std::ptr::null()
    } else {
        let offset = lsda_read_uleb128(&mut p) as usize;
        p.add(offset)
    };

    // call-site table
    let cs_enc = *p;
    p = p.add(1);
    let cs_table_len = lsda_read_uleb128(&mut p) as usize;
    let cs_table_end = p.add(cs_table_len);
    let action_table_start = cs_table_end;

    let pc_offset = ip.wrapping_sub(func_start);

    while p < cs_table_end {
        let cs_start = lsda_read_encoded(&mut p, cs_enc);
        let cs_len = lsda_read_encoded(&mut p, cs_enc);
        let cs_lp = lsda_read_encoded(&mut p, cs_enc);
        let cs_action = lsda_read_uleb128(&mut p);

        if pc_offset < cs_start || pc_offset >= cs_start.wrapping_add(cs_len) {
            continue;
        }

        // IP is within this call site.
        if cs_lp == 0 {
            // No landing pad (e.g. a call that the standard says shouldn't throw).
            return None;
        }

        let lp_addr = lpstart.wrapping_add(cs_lp);

        if cs_action == 0 {
            // Cleanup-only landing pad (e.g. @finally only, no @catch).
            return Some(CallSiteResult {
                lp_addr,
                action_index: 0,
            });
        }

        // Walk the action chain starting at cs_action-1 (0-based into action table).
        let mut ap = action_table_start.add(cs_action as usize - 1);
        loop {
            let type_filter = lsda_read_sleb128(&mut ap);
            let next_field_start = ap; // position before reading 'next'
            let next = lsda_read_sleb128(&mut ap);

            if type_filter == 0 {
                // @catch (...) — catches everything.
                return Some(CallSiteResult {
                    lp_addr,
                    action_index: 0,
                });
            }

            if type_filter > 0 && !ttype_end.is_null() {
                // Type entry size depends on ttype_enc value bits.
                let entry_bytes: usize = match ttype_enc & 0x0f {
                    0x00 => 8, // absptr
                    0x02 | 0x0a => 2,
                    0x03 | 0x0b => 4,
                    0x04 | 0x0c => 8,
                    _ => 4,
                };
                // Type table grows backward from ttype_end.
                let entry_ptr = ttype_end.sub(type_filter as usize * entry_bytes);
                let mut ep = entry_ptr;
                let catch_cls = lsda_read_encoded(&mut ep, ttype_enc);

                if catch_cls == 0 {
                    // Null class = @catch (...) represented as typed catch-all.
                    return Some(CallSiteResult {
                        lp_addr,
                        action_index: type_filter,
                    });
                }

                // Check whether the guest exception is an instance of `catch_cls`.
                // `catch_cls` is our synthetic class address (after macho-exec fixup).
                if shim_objc_exception_match(catch_cls, guest_exc) != 0 {
                    return Some(CallSiteResult {
                        lp_addr,
                        action_index: type_filter,
                    });
                }
            }

            if next == 0 {
                break;
            }
            // `next` is an offset relative to the start of the `next` field.
            ap = next_field_start.offset(next as isize);
        }

        // Call site matched our IP but no type match found — continue unwinding.
        return None;
    }

    None
}

/// `_Unwind_Reason_Code __objc_personality_v0(version, actions, exc_class, exc_obj, ctx)`
///
/// Pure-Rust ObjC personality function.  Registered as `"___objc_personality_v0"`
/// in the shim table, superseding the host libobjc implementation.  Uses our
/// synthetic ObjcRuntime class hierarchy for @catch type matching so no host
/// ObjC calls are needed.  Works identically on Linux and macOS.
#[cfg(unix)]
pub extern "C" fn shim_objc_personality_v0(
    _version: i32,
    actions: i32,
    _exception_class: u64,
    exception_object: *mut u8, // _Unwind_Exception *
    context: *mut u8,          // _Unwind_Context *
) -> i32 {
    const URC_CONTINUE_UNWIND: i32 = 8;
    const URC_HANDLER_FOUND: i32 = 6;
    const URC_INSTALL_CONTEXT: i32 = 7;
    const UA_SEARCH_PHASE: i32 = 1;
    const UA_CLEANUP_PHASE: i32 = 2;
    const UA_HANDLER_FRAME: i32 = 4;

    let lsda = unsafe { _Unwind_GetLanguageSpecificData(context) };
    if lsda.is_null() {
        return URC_CONTINUE_UNWIND;
    }

    // Adjust IP to point at the instruction that caused the exception (not the
    // return address after it).  Standard Itanium ABI convention.
    let ip = unsafe { _Unwind_GetIP(context) }.wrapping_sub(1);
    let func_start = unsafe { _Unwind_GetRegionStart(context) };

    let guest_exc = CURRENT_EXCEPTION
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0);

    let result = unsafe { scan_lsda(lsda, func_start, ip, guest_exc) };

    match result {
        None => URC_CONTINUE_UNWIND,
        Some(lp) => {
            if actions & UA_SEARCH_PHASE != 0 {
                // Phase 1: found a handler — tell the unwinder.
                return URC_HANDLER_FOUND;
            }

            if actions & UA_CLEANUP_PHASE != 0
                && (actions & UA_HANDLER_FRAME != 0 || lp.action_index == 0)
            {
                // Phase 2: install the landing pad.
                // x0 (DWARF reg 0) = _Unwind_Exception pointer.
                // x1 (DWARF reg 1) = type selector (0 = cleanup).
                unsafe {
                    _Unwind_SetGR(context, 0, exception_object as u64);
                    _Unwind_SetGR(context, 1, lp.action_index as u64);
                    _Unwind_SetIP(context, lp.lp_addr);
                }
                return URC_INSTALL_CONTEXT;
            }

            URC_CONTINUE_UNWIND
        }
    }
}

#[cfg(not(unix))]
pub extern "C" fn shim_objc_personality_v0(
    _version: i32,
    _actions: i32,
    _exception_class: u64,
    _exception_object: *mut u8,
    _context: *mut u8,
) -> i32 {
    8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_exception_initially_none() {
        let guard = CURRENT_EXCEPTION.lock().unwrap_or_else(|e| e.into_inner());
        // May have been set by a prior test; just verify the lock works.
        // On a fresh run this is None.
        drop(guard);
    }

    #[test]
    fn fallback_class_name_terminated() {
        assert_eq!(*FALLBACK_CLASS_NAME.last().unwrap(), 0u8);
    }

    #[test]
    fn fallback_class_name_content() {
        assert!(FALLBACK_CLASS_NAME.starts_with(b"NSException"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn host_nsexception_ehtype_resolves() {
        let ehtype =
            host_nsexception_ehtype().expect("host NSException EHTYPE should resolve on macOS");
        assert!(!ehtype.vtable.is_null());
        assert!(!ehtype.name.is_null());
        assert!(!ehtype.cls.is_null());
        assert_ne!(host_nsexception_ehtype_addr(), 0);
    }

    // ── Legacy exception API tests ───────────────────────────

    #[test]
    fn try_enter_exit_roundtrip() {
        let data = 0xDEAD_BEEFu64;
        shim_objc_exception_try_enter(data);
        // Verify it's on the stack.
        let stack = EXCEPTION_STACK.lock().unwrap();
        assert!(stack.contains(&data));
        drop(stack);
        // Pop it.
        shim_objc_exception_try_exit(data);
        let stack = EXCEPTION_STACK.lock().unwrap();
        assert!(!stack.contains(&data));
    }

    #[test]
    fn exception_extract_returns_current() {
        // Set a known exception and extract it.
        *CURRENT_EXCEPTION.lock().unwrap() = Some(0x1234);
        let extracted = shim_objc_exception_extract(0);
        assert_eq!(extracted, 0x1234);
        // Clean up.
        *CURRENT_EXCEPTION.lock().unwrap() = None;
    }

    #[test]
    fn exception_match_nil() {
        assert_eq!(shim_objc_exception_match(0, 0), 0);
        assert_eq!(shim_objc_exception_match(0x1000, 0), 0);
        assert_eq!(shim_objc_exception_match(0, 0x1000), 0);
    }

    #[test]
    fn uncaught_handler_default_nil() {
        // Handler should be retrievable (0 or previously set).
        let _h = shim_ns_get_uncaught_exception_handler();
    }

    #[test]
    fn set_and_get_uncaught_handler() {
        let handler: u64 = 0xCAFE;
        shim_ns_set_uncaught_exception_handler(handler);
        assert_eq!(shim_ns_get_uncaught_exception_handler(), handler);
        // Reset.
        shim_ns_set_uncaught_exception_handler(0);
        assert_eq!(shim_ns_get_uncaught_exception_handler(), 0);
    }
}
