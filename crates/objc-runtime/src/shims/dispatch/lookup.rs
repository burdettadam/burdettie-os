//! Message lookup callbacks (called from assembly trampolines).

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;
use runtime_diagnostics::DiagEvent;

use super::forwarding::forwarding_trampoline;

/// Resolve (self, SEL) → IMP.
#[no_mangle]
pub extern "C" fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    if obj & (1u64 << 63) != 0 {
        // Tagged pointer — look up class from tagged pointer ISA table.
        let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
            .to_str()
            .unwrap_or("");
        let guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else {
            return unrecognized_selector_stub as u64;
        };
        if let Some(isa) = rt.tagged_class_for_pointer(obj) {
            if let Some(imp) = rt.msg_lookup(isa, sel_str) {
                log::trace!(
                    "objc_msgSend: tagged [{sel_str}] on {obj:#x} (isa={isa:#x}) → imp={imp:#x}"
                );
                return imp;
            }
        }
        log::debug!(
            "objc_msgSend: unhandled tagged pointer {obj:#x} for '{sel_str}'"
        );
        runtime_diagnostics::emit(DiagEvent::HostFallback {
            reason: "tagged pointer — no handler".into(),
            selector: sel_str.into(),
            receiver: obj,
        });
        return unrecognized_selector_stub as u64;
    }

    if obj % 8 != 0 {
        let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
            .to_str()
            .unwrap_or("");
        log::warn!(
            "objc_msgSend: unaligned pointer {obj:#x} for '{sel_str}'"
        );
        return unrecognized_selector_stub as u64;
    }

    let raw_isa = unsafe { *(obj as *const u64) };

    #[cfg(target_arch = "aarch64")]
    let isa = raw_isa & 0x007ffffffffffff8;
    #[cfg(not(target_arch = "aarch64"))]
    let isa = raw_isa;

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
            runtime_diagnostics::emit(DiagEvent::MethodDispatch {
                selector: sel_str.into(),
                isa,
                imp,
            });
            imp
        }
        None => {
            if rt.is_known_isa(isa) {
                let is_meta = rt.is_metaclass(isa);
                let resolve_sel_name = if is_meta {
                    "resolveClassMethod:"
                } else {
                    "resolveInstanceMethod:"
                };

                let resolve_isa = if is_meta {
                    isa
                } else {
                    unsafe { *(isa as *const u64) }
                };

                if let Some(resolve_imp) = rt.msg_lookup(resolve_isa, resolve_sel_name) {
                    drop(guard);

                    let resolve_fn: extern "C" fn(u64, u64, u64) -> u8 =
                        unsafe { std::mem::transmute(resolve_imp) };
                    let resolved = resolve_fn(isa, 0, sel);

                    if resolved != 0 {
                        let guard2 = match OBJC_RUNTIME.lock() {
                            Ok(g) => g,
                            Err(e) => e.into_inner(),
                        };
                        if let Some(rt2) = guard2.as_ref() {
                            if let Some(imp) = rt2.msg_lookup(isa, sel_str) {
                                log::trace!(
                                    "objc_msgSend: [{sel_str}] resolved dynamically → imp={imp:#x}"
                                );
                                return imp;
                            }
                        }
                    }

                    log::debug!(
                        "objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})"
                    );
                    runtime_diagnostics::emit(DiagEvent::ForwardingEntered {
                        selector: sel_str.into(),
                        receiver: obj,
                        isa,
                    });
                    return forwarding_trampoline as u64;
                }

                drop(guard);
                log::debug!(
                    "objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})"
                );
                runtime_diagnostics::emit(DiagEvent::ForwardingEntered {
                    selector: sel_str.into(),
                    receiver: obj,
                    isa,
                });
                return forwarding_trampoline as u64;
            }

            drop(guard);
            log::warn!(
                "objc_msgSend: unrecognized selector '{}' sent to instance {:#x} (isa={:#x})",
                sel_str, obj, isa
            );
            runtime_diagnostics::emit(DiagEvent::UnknownIsa {
                isa,
                selector: sel_str.into(),
                receiver: obj,
            });
            unrecognized_selector_stub as u64
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
            runtime_diagnostics::emit(DiagEvent::SuperSelectorMissing {
                selector: sel_str.into(),
                receiver,
                super_class,
            });
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
            runtime_diagnostics::emit(DiagEvent::Super2UnknownClass {
                current_class,
            });
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
            runtime_diagnostics::emit(DiagEvent::SuperSelectorMissing {
                selector: sel_str.into(),
                receiver,
                super_class,
            });
            eprintln!(
                "objc_msgSendSuper2: unrecognized selector '{}' sent to instance {:#x} (super={:#x})",
                sel_str, receiver, super_class
            );
            std::process::abort();
        }
    }
}

/// Stub IMP returned when the ISA is unknown to our runtime.
pub(super) extern "C" fn unrecognized_selector_stub(
    _obj: u64,
    _sel: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
    _a7: u64,
) -> u64 {
    0
}
