//! Message dispatch: objc_msgSend trampoline linkage + lookup callbacks.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

// ── ARM64 assembly trampoline linkage ──────────────────────────────

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_objc_msg_send_asm();
    fn shim_objc_msg_send_super_asm();
    fn shim_objc_msg_send_super2_asm();
}

/// Cached host `objc_msgSend` address for fallback dispatch.
/// Resolved once via dlsym and reused for all subsequent lookups.
#[cfg(target_os = "macos")]
static HOST_MSG_SEND: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

#[cfg(target_os = "macos")]
fn host_msg_send_addr() -> u64 {
    *HOST_MSG_SEND.get_or_init(|| {
        let sym = unsafe { libc::dlsym((-2isize) as *mut libc::c_void, c"objc_msgSend".as_ptr()) };
        sym as u64
    })
}

/// Cached host `sel_registerName` for translating our selectors to host selectors.
#[cfg(target_os = "macos")]
static HOST_SEL_REGISTER_NAME: std::sync::OnceLock<unsafe extern "C" fn(*const c_char) -> u64> =
    std::sync::OnceLock::new();

/// Translate one of our selector strings into the host runtime's canonical selector pointer.
#[cfg(target_os = "macos")]
fn host_sel_register_name(name: &str) -> u64 {
    let f = *HOST_SEL_REGISTER_NAME.get_or_init(|| unsafe {
        let sym = libc::dlsym((-2isize) as *mut libc::c_void, c"sel_registerName".as_ptr());
        std::mem::transmute(sym)
    });
    let cname = std::ffi::CString::new(name).unwrap_or_default();
    unsafe { f(cname.as_ptr()) }
}

pub fn objc_msg_send_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        shim_objc_msg_send_rust as u64
    }
}

pub fn objc_msg_send_super_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        shim_objc_msg_send_super_rust as u64
    }
}

pub fn objc_msg_send_super2_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super2_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        shim_objc_msg_send_super2_rust as u64
    }
}

// ── Lookup callbacks (called from assembly trampolines) ────────────

/// Result of `objc_msg_lookup_imp`: the IMP to call and an optional
/// translated selector for host fallback.
#[repr(C)]
pub struct LookupResult {
    /// The resolved IMP (or host_msg_send_addr for fallback).
    pub imp: u64,
    /// Non-zero if the selector was translated for host dispatch;
    /// the trampoline writes this into the saved x1 slot before restore.
    pub translated_sel: u64,
}

/// Resolve (self, SEL) → LookupResult { imp, translated_sel }.
///
/// Returns a tuple:
///   - x0: the IMP to call
///   - x1: translated selector for host fallback (0 if no translation needed)
///
/// When falling back to the host's `objc_msgSend` (because the object's ISA is
/// not in our runtime), we translate our internal selector to the host's canonical
/// selector pointer. The assembly trampoline writes this translated selector over
/// the saved x1 on the stack before restoring registers and tail-calling the IMP.
#[no_mangle]
pub extern "C" fn objc_msg_lookup_imp(obj: u64, sel: u64) -> LookupResult {
    if obj == 0 {
        return LookupResult {
            imp: 0,
            translated_sel: 0,
        };
    }

    // Tagged pointer check: on arm64 macOS, bit 63 marks a tagged pointer
    // (value stored inline, no heap object to dereference). These are always
    // host runtime objects — forward to host's objc_msgSend with translated sel.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    if obj & (1u64 << 63) != 0 {
        let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
            .to_str()
            .unwrap_or("");
        let host = host_msg_send_addr();
        let host_sel = host_sel_register_name(sel_str);
        log::trace!("objc_msgSend: tagged pointer {obj:#x} '{sel_str}' → host");
        return LookupResult {
            imp: host,
            translated_sel: host_sel,
        };
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSend: ObjC runtime not initialized");
        std::process::abort();
    };

    match rt.msg_lookup(isa, sel_str) {
        Some(imp) => {
            log::trace!("objc_msgSend: [{sel_str}] on {obj:#x} (isa={isa:#x}) → imp={imp:#x}");
            LookupResult {
                imp,
                translated_sel: 0,
            }
        }
        None => {
            // Check if the ISA belongs to our runtime
            if rt.is_known_isa(isa) {
                // Try dynamic method resolution first:
                // For instance methods, call +resolveInstanceMethod: on the class
                // For class methods, call +resolveClassMethod: on the metaclass
                let is_meta = rt.is_metaclass(isa);
                let resolve_sel_name = if is_meta {
                    "resolveClassMethod:"
                } else {
                    "resolveInstanceMethod:"
                };

                // Look up the class's metaclass to find the resolve method
                let resolve_isa = if is_meta {
                    // For metaclass dispatch, the resolver is on the metaclass itself
                    isa
                } else {
                    // For instance dispatch, the resolver is a class method → on the metaclass
                    // ISA is the class addr; read the class's ISA to get the metaclass
                    unsafe { *(isa as *const u64) }
                };

                if let Some(resolve_imp) = rt.msg_lookup(resolve_isa, resolve_sel_name) {
                    // Drop the lock before calling the IMP
                    drop(guard);

                    // Call +resolveInstanceMethod: or +resolveClassMethod:
                    // Args: (class_or_meta, _cmd, selector_to_resolve)
                    let resolve_fn: extern "C" fn(u64, u64, u64) -> u8 =
                        unsafe { std::mem::transmute(resolve_imp) };
                    let resolved = resolve_fn(isa, 0, sel);

                    if resolved != 0 {
                        // Re-acquire lock and retry lookup
                        let guard2 = match OBJC_RUNTIME.lock() {
                            Ok(g) => g,
                            Err(e) => e.into_inner(),
                        };
                        if let Some(rt2) = guard2.as_ref() {
                            if let Some(imp) = rt2.msg_lookup(isa, sel_str) {
                                log::trace!(
                                    "objc_msgSend: [{sel_str}] resolved dynamically → imp={imp:#x}"
                                );
                                return LookupResult {
                                    imp,
                                    translated_sel: 0,
                                };
                            }
                        }
                    }

                    // Resolution failed — try the forwarding chain
                    // Return the forwarding trampoline; it will handle
                    // methodSignatureForSelector: and forwardInvocation:
                    log::debug!(
                        "objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})"
                    );
                    return LookupResult {
                        imp: forwarding_trampoline as u64,
                        translated_sel: 0,
                    };
                }

                // No resolve method found — return forwarding trampoline
                drop(guard);
                log::debug!(
                    "objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})"
                );
                return LookupResult {
                    imp: forwarding_trampoline as u64,
                    translated_sel: 0,
                };
            }

            // ISA not known to our runtime — this may be a host ObjC object
            // (e.g., a real NSException caught via @catch). Fall back to the
            // host's objc_msgSend. Translate our selector to the host's
            // canonical pointer (ObjC uses pointer equality for selectors)
            // and return it so the trampoline can overwrite x1.
            #[cfg(target_os = "macos")]
            {
                let host = host_msg_send_addr();
                if host != 0 {
                    log::debug!(
                        "objc_msgSend: falling back to host for '{sel_str}' on {obj:#x} (isa={isa:#x})"
                    );
                    let host_sel = host_sel_register_name(sel_str);
                    return LookupResult {
                        imp: host,
                        translated_sel: host_sel,
                    };
                }
            }
            eprintln!(
                "objc_msgSend: unrecognized selector '{}' sent to instance {:#x} (isa={:#x})",
                sel_str, obj, isa
            );
            std::process::abort();
        }
    }
}

/// Lookup for `objc_msgSendSuper` — starts search at `super_class`.
#[no_mangle]
pub extern "C" fn objc_msg_super_lookup_imp(receiver: u64, super_class: u64, sel: u64) -> u64 {
    if receiver == 0 {
        return 0;
    }
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSendSuper: ObjC runtime not initialized");
        std::process::abort();
    };

    match rt.msg_lookup(super_class, sel_str) {
        Some(imp) => {
            log::trace!("objc_msgSendSuper: [{sel_str}] on {receiver:#x} (super={super_class:#x}) → imp={imp:#x}");
            imp
        }
        None => {
            eprintln!(
                "objc_msgSendSuper: unrecognized selector '{}' sent to instance {:#x} (super={:#x})",
                sel_str, receiver, super_class
            );
            std::process::abort();
        }
    }
}

/// Lookup for `objc_msgSendSuper2` — follows `current_class.superclass`.
#[no_mangle]
pub extern "C" fn objc_msg_super2_lookup_imp(receiver: u64, current_class: u64, sel: u64) -> u64 {
    if receiver == 0 {
        return 0;
    }
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSendSuper2: ObjC runtime not initialized");
        std::process::abort();
    };

    let super_class = match rt.get_class_by_addr(current_class) {
        Some(info) => match info.superclass_addr {
            Some(addr) => addr,
            None => {
                eprintln!(
                    "objc_msgSendSuper2: class '{}' at {:#x} has no superclass",
                    info.name, current_class
                );
                std::process::abort();
            }
        },
        None => {
            eprintln!("objc_msgSendSuper2: unknown class at {:#x}", current_class);
            std::process::abort();
        }
    };

    match rt.msg_lookup(super_class, sel_str) {
        Some(imp) => {
            log::trace!("objc_msgSendSuper2: [{sel_str}] on {receiver:#x} (super={super_class:#x}) → imp={imp:#x}");
            imp
        }
        None => {
            eprintln!(
                "objc_msgSendSuper2: unrecognized selector '{}' sent to instance {:#x} (super={:#x})",
                sel_str, receiver, super_class
            );
            std::process::abort();
        }
    }
}

// ── Rust-only fallbacks (non-aarch64) ──────────────────────────────

#[allow(dead_code)]
extern "C" fn shim_objc_msg_send_rust(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let obj = a0;
    let sel = a1;
    if obj == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSend: ObjC runtime not initialized");
        std::process::abort();
    };

    let Some(imp) = rt.msg_lookup(isa, sel_str) else {
        eprintln!(
            "objc_msgSend: unrecognized selector '{}' sent to instance {:#x} (isa={:#x})",
            sel_str, obj, isa
        );
        std::process::abort();
    };

    drop(guard);
    let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
        unsafe { std::mem::transmute(imp) };
    f(a0, a1, a2, a3, a4, a5, a6, a7)
}

#[allow(dead_code)]
extern "C" fn shim_objc_msg_send_super_rust(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let super_ptr = a0;
    let receiver = unsafe { *(super_ptr as *const u64) };
    let super_class = unsafe { *((super_ptr as *const u64).add(1)) };

    let imp = objc_msg_super_lookup_imp(receiver, super_class, a1);
    if imp == 0 {
        return 0;
    }

    let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
        unsafe { std::mem::transmute(imp) };
    f(receiver, a1, a2, a3, a4, a5, a6, a7)
}

#[allow(dead_code)]
extern "C" fn shim_objc_msg_send_super2_rust(
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    let super_ptr = a0;
    let receiver = unsafe { *(super_ptr as *const u64) };
    let current_class = unsafe { *((super_ptr as *const u64).add(1)) };

    let imp = objc_msg_super2_lookup_imp(receiver, current_class, a1);
    if imp == 0 {
        return 0;
    }

    let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
        unsafe { std::mem::transmute(imp) };
    f(receiver, a1, a2, a3, a4, a5, a6, a7)
}

// ── Forwarding trampoline ──────────────────────────────────────────

/// Forwarding trampoline: called when a known ISA has no method for the selector.
///
/// Implements the ObjC forwarding chain:
/// 1. Call `-methodSignatureForSelector:` on the receiver
/// 2. If non-nil, create an `NSInvocation` and call `-forwardInvocation:`
/// 3. Return the invocation's return value (or 0)
extern "C" fn forwarding_trampoline(
    obj: u64,
    sel: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }

    let isa = unsafe { *(obj as *const u64) };
    let sel_str = unsafe {
        CStr::from_ptr(sel as *const c_char)
            .to_str()
            .unwrap_or("")
    };

    // Step 1: Look up methodSignatureForSelector: IMP
    let sig_imp = {
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else { return 0 };
        rt.msg_lookup(isa, "methodSignatureForSelector:")
    };

    let Some(sig_imp) = sig_imp else {
        // No methodSignatureForSelector: — nothing we can do
        eprintln!(
            "objc_msgSend: unrecognized selector '{}' sent to instance {:#x} (no forwarding)",
            sel_str, obj
        );
        return 0;
    };

    // Call methodSignatureForSelector: with the original selector
    let sig_fn: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(sig_imp) };
    let signature = sig_fn(obj, 0, sel);

    if signature == 0 {
        // No signature — the class doesn't know about this selector
        // Try doesNotRecognizeSelector:
        let dnr_imp = {
            let guard = match OBJC_RUNTIME.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            let Some(rt) = guard.as_ref() else { return 0 };
            rt.msg_lookup(isa, "doesNotRecognizeSelector:")
        };
        if let Some(dnr_imp) = dnr_imp {
            let dnr_fn: extern "C" fn(u64, u64, u64) =
                unsafe { std::mem::transmute(dnr_imp) };
            dnr_fn(obj, 0, sel);
        } else {
            eprintln!(
                "objc_msgSend: unrecognized selector '{}' sent to instance {:#x}",
                sel_str, obj
            );
        }
        return 0;
    }

    // Step 2: Create NSInvocation with the signature and arguments
    // Read numberOfArguments from the signature (offset 16)
    let num_args = unsafe { *((signature + 16) as *const u64) } as usize;

    // Allocate invocation: ISA(8) + sig(8) + target(8) + sel(8) + argBuf(8) + retBuf(8) = 48
    let inv = unsafe { libc::calloc(1, 48) as u64 };
    if inv == 0 {
        return 0;
    }

    // Look up the NSInvocation class address
    let inv_isa = {
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else { return 0 };
        if let Some(cls) = rt.get_class("NSInvocation") {
            cls.mapped_addr
        } else {
            return 0;
        }
    };

    // Set up the invocation manually
    unsafe {
        *(inv as *mut u64) = inv_isa;           // ISA
        *((inv + 8) as *mut u64) = signature;   // methodSignature
        *((inv + 16) as *mut u64) = obj;        // target
        *((inv + 24) as *mut u64) = sel;        // selector

        // Allocate and populate argument buffer
        let arg_buf = libc::calloc(8, 8) as u64; // 8 args max
        *((inv + 32) as *mut u64) = arg_buf;

        if arg_buf != 0 {
            let buf = arg_buf as *mut u64;
            *buf = obj;         // arg[0] = self
            *buf.add(1) = sel;  // arg[1] = _cmd
            if num_args > 2 { *buf.add(2) = a2; }
            if num_args > 3 { *buf.add(3) = a3; }
            if num_args > 4 { *buf.add(4) = a4; }
            if num_args > 5 { *buf.add(5) = a5; }
            if num_args > 6 { *buf.add(6) = a6; }
            if num_args > 7 { *buf.add(7) = a7; }
        }

        // Allocate return buffer
        let ret_buf = libc::calloc(1, 8) as u64;
        *((inv + 40) as *mut u64) = ret_buf;
    }

    // Step 3: Look up and call forwardInvocation:
    let fwd_imp = {
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else { return 0 };
        rt.msg_lookup(isa, "forwardInvocation:")
    };

    if let Some(fwd_imp) = fwd_imp {
        let fwd_fn: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(fwd_imp) };
        fwd_fn(obj, 0, inv);
    }

    // Step 4: Extract return value
    let result = unsafe {
        let ret_buf = *((inv + 40) as *const u64);
        if ret_buf != 0 {
            *(ret_buf as *const u64)
        } else {
            0
        }
    };

    // Clean up the invocation buffers (but not the signature — caller may still need it)
    unsafe {
        let arg_buf = *((inv + 32) as *const u64);
        if arg_buf != 0 {
            libc::free(arg_buf as *mut libc::c_void);
        }
        let ret_buf = *((inv + 40) as *const u64);
        if ret_buf != 0 {
            libc::free(ret_buf as *mut libc::c_void);
        }
        libc::free(inv as *mut libc::c_void);
    }

    result
}
