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
