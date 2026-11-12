//! Message lookup callbacks (called from assembly trampolines).

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;
use runtime_diagnostics::DiagEvent;

use super::forwarding::forwarding_trampoline;

#[inline]
fn normalize_isa(rt: &crate::class::ObjcRuntime, raw_isa: u64) -> u64 {
    if rt.is_known_isa(raw_isa) {
        return raw_isa;
    }

    #[cfg(target_arch = "aarch64")]
    {
        raw_isa & 0x007ffffffffffff8
    }
    #[cfg(target_arch = "x86_64")]
    {
        raw_isa & 0x00007ffffffffff8
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        raw_isa
    }
}

/// Resolve (self, SEL) → IMP.
#[no_mangle]
pub extern "C" fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let known_class_dispatch_isa = {
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard
            .as_ref()
            .and_then(|rt| rt.dispatch_isa_for_receiver(obj))
    };

    if obj & (1u64 << 63) != 0 && known_class_dispatch_isa.is_none() {
        // Tagged pointer — look up class from tagged pointer ISA table.
        let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
            .to_str()
            .unwrap_or("");
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else {
            return unrecognized_selector_stub as *const () as u64;
        };
        if let Some(isa) = rt.tagged_class_for_pointer(obj) {
            if let Some(imp) = rt.msg_lookup(isa, sel_str) {
                log::trace!(
                    "objc_msgSend: tagged [{sel_str}] on {obj:#x} (isa={isa:#x}) → imp={imp:#x}"
                );
                return imp;
            }
        }
        log::debug!("objc_msgSend: unhandled tagged pointer {obj:#x} for '{sel_str}'");
        runtime_diagnostics::emit(DiagEvent::HostFallback {
            reason: "tagged pointer — no handler".into(),
            selector: sel_str.into(),
            receiver: obj,
        });
        return unrecognized_selector_stub as *const () as u64;
    }

    if obj % 8 != 0 {
        let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
            .to_str()
            .unwrap_or("");
        log::warn!("objc_msgSend: unaligned pointer {obj:#x} for '{sel_str}'");
        return unrecognized_selector_stub as *const () as u64;
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSend: ObjC runtime not initialized");
        std::process::abort();
    };

    let isa = if let Some(dispatch_isa) =
        known_class_dispatch_isa.or_else(|| rt.dispatch_isa_for_receiver(obj))
    {
        dispatch_isa
    } else {
        let raw_isa = unsafe { *(obj as *const u64) };
        normalize_isa(rt, raw_isa)
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
                        let guard2 = match OBJC_RUNTIME.read() {
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

                    log::debug!("objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})");
                    runtime_diagnostics::emit(DiagEvent::ForwardingEntered {
                        selector: sel_str.into(),
                        receiver: obj,
                        isa,
                    });
                    return forwarding_trampoline as *const () as u64;
                }

                drop(guard);
                log::debug!("objc_msgSend: forwarding '{sel_str}' on {obj:#x} (isa={isa:#x})");
                runtime_diagnostics::emit(DiagEvent::ForwardingEntered {
                    selector: sel_str.into(),
                    receiver: obj,
                    isa,
                });
                return forwarding_trampoline as *const () as u64;
            }

            drop(guard);

            // Unknown ISA — object's class is not registered in the guest
            // ObjC runtime.  Return the nil stub rather than forwarding to
            // any host runtime.
            log::warn!(
                "objc_msgSend: unrecognized selector '{}' sent to instance {:#x} (isa={:#x})",
                sel_str,
                obj,
                isa
            );
            runtime_diagnostics::emit(DiagEvent::UnknownIsa {
                isa,
                selector: sel_str.into(),
                receiver: obj,
            });
            unrecognized_selector_stub as *const () as u64
        }
    }
}

/// Check if `obj` actually has `sel` registered in the ObjC runtime (not just a forwarding stub).
/// Returns 1 if the method is found, 0 otherwise.
/// Used by shims that need to call optional delegate methods safely.
#[no_mangle]
pub extern "C" fn objc_has_method(obj: u64, sel: u64) -> u8 {
    if obj == 0 || sel == 0 {
        return 0;
    }
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");
    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return 0;
    };
    let raw_isa = unsafe { *(obj as *const u64) };
    let isa = normalize_isa(rt, raw_isa);
    u8::from(rt.msg_lookup(isa, sel_str).is_some())
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

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        eprintln!("objc_msgSendSuper: ObjC runtime not initialized");
        std::process::abort();
    };

    match rt.msg_lookup(super_class, sel_str) {
        Some(imp) => {
            log::trace!(
                "objc_msgSendSuper: [{sel_str}] on {receiver:#x} (super={super_class:#x}) → imp={imp:#x}"
            );
            imp
        }
        None => {
            runtime_diagnostics::emit(DiagEvent::SuperSelectorMissing {
                selector: sel_str.into(),
                receiver,
                super_class,
            });
            log::warn!(
                "objc_msgSendSuper: unrecognized selector '{}' on {:#x} (super={:#x}) → returning nil",
                sel_str,
                receiver,
                super_class
            );
            unrecognized_selector_stub as *const () as u64
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

    let guard = match OBJC_RUNTIME.read() {
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
                log::warn!(
                    "objc_msgSendSuper2: class '{}' at {:#x} has no superclass → returning nil",
                    info.name,
                    current_class
                );
                return unrecognized_selector_stub as *const () as u64;
            }
        },
        None => {
            runtime_diagnostics::emit(DiagEvent::Super2UnknownClass { current_class });
            log::warn!(
                "objc_msgSendSuper2: unknown class at {:#x} → returning nil",
                current_class
            );
            return unrecognized_selector_stub as *const () as u64;
        }
    };

    match rt.msg_lookup(super_class, sel_str) {
        Some(imp) => {
            log::trace!(
                "objc_msgSendSuper2: [{sel_str}] on {receiver:#x} (super={super_class:#x}) → imp={imp:#x}"
            );
            imp
        }
        None => {
            runtime_diagnostics::emit(DiagEvent::SuperSelectorMissing {
                selector: sel_str.into(),
                receiver,
                super_class,
            });
            log::warn!(
                "objc_msgSendSuper2: unrecognized selector '{}' on {:#x} (super={:#x}) → returning nil",
                sel_str,
                receiver,
                super_class
            );
            unrecognized_selector_stub as *const () as u64
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ClassInfo, MethodEntry, ObjcRuntime, OBJC_RUNTIME};
    use std::collections::HashMap;
    use std::ffi::CString;

    fn setup_lookup_class(name: &str, addr: u64, methods: Vec<(&'static str, u64)>) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        let mut info = ClassInfo {
            name: name.to_string(),
            mapped_addr: addr,
            methods: methods
                .iter()
                .map(|(s, imp)| MethodEntry {
                    selector: (*s).into(),
                    imp: *imp,
                    type_encoding: "v@:".into(),
                })
                .collect(),
            class_methods: vec![],
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 16,
            is_root: true,
            is_meta: false,
            superclass_addr: None,
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_synthetic_class(info);
    }

    fn setup_lookup_metaclass(name: &str, addr: u64, class_methods: Vec<(&'static str, u64)>) {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        let rt = guard.get_or_insert_with(ObjcRuntime::new);
        let mut info = ClassInfo {
            name: name.to_string(),
            mapped_addr: addr,
            methods: vec![],
            class_methods: class_methods
                .iter()
                .map(|(s, imp)| MethodEntry {
                    selector: (*s).into(),
                    imp: *imp,
                    type_encoding: "@@:".into(),
                })
                .collect(),
            ivars: vec![],
            properties: vec![],
            protocol_names: vec![],
            instance_size: 40,
            is_root: false,
            is_meta: true,
            superclass_addr: None,
            method_index: HashMap::new(),
            class_method_index: HashMap::new(),
        };
        info.rebuild_indices();
        rt.register_metaclass_addr(addr, name);
        rt.register_synthetic_class(info);
    }

    #[test]
    fn lookup_nil_receiver() {
        let sel = CString::new("init").unwrap();
        assert_eq!(objc_msg_lookup_imp(0, sel.as_ptr() as u64), 0);
    }

    #[test]
    fn lookup_class_receiver_uses_metaclass_dispatch() {
        let class_addr = 0x0000_0001_0000_3000u64;
        let meta_addr = 0x0000_0001_0000_4000u64;
        let imp_addr = 0x0000_0002_0000_1234u64;

        setup_lookup_class("DispatchClass", class_addr, vec![]);
        setup_lookup_metaclass("DispatchClass", meta_addr, vec![("alloc", imp_addr)]);

        let sel = CString::new("alloc").unwrap();
        assert_eq!(
            objc_msg_lookup_imp(class_addr, sel.as_ptr() as u64),
            imp_addr
        );
    }

    #[test]
    fn lookup_null_sel() {
        // obj == 0 triggers early return before sel is accessed,
        // so this safely tests the nil-receiver path with sel=0.
        assert_eq!(objc_msg_lookup_imp(0, 0), 0);
    }

    #[test]
    fn has_method_nonexistent_class() {
        // obj=0 triggers the early return in objc_has_method.
        assert_eq!(objc_has_method(0, 0), 0);
    }

    #[test]
    fn has_method_found() {
        // Address must survive ISA masking (& 0x007ffffffffffff8 on aarch64).
        let class_addr = 0x0000_0001_0000_1000u64;
        let imp_addr = 0x0000_0002_0000_0001u64;
        setup_lookup_class("LkTestHasA", class_addr, vec![("doStuff", imp_addr)]);

        // Intern the selector so it resolves.
        {
            let mut guard = OBJC_RUNTIME.write().unwrap();
            let rt = guard.as_mut().unwrap();
            rt.intern_selector("doStuff");
        }

        // Create a fake object whose first 8 bytes are the ISA.
        let fake_obj: Box<u64> = Box::new(class_addr);
        let obj_addr = &*fake_obj as *const u64 as u64;

        let sel = CString::new("doStuff").unwrap();
        assert_eq!(objc_has_method(obj_addr, sel.as_ptr() as u64), 1);
    }

    #[test]
    fn lookup_preserves_known_high_synthetic_isa() {
        let class_addr = 0x0050_0000_0001_0000u64;
        let imp_addr = 0x0000_0002_0000_4321u64;
        setup_lookup_class("HighSynthDispatch", class_addr, vec![("init", imp_addr)]);

        let fake_obj: Box<u64> = Box::new(class_addr);
        let obj_addr = &*fake_obj as *const u64 as u64;
        let sel = CString::new("init").unwrap();

        assert_eq!(objc_msg_lookup_imp(obj_addr, sel.as_ptr() as u64), imp_addr);
        assert_eq!(objc_has_method(obj_addr, sel.as_ptr() as u64), 1);
    }

    #[test]
    fn has_method_not_found() {
        let class_addr = 0x0000_0001_0000_2000u64;
        setup_lookup_class("LkTestHasB", class_addr, vec![]);

        let fake_obj: Box<u64> = Box::new(class_addr);
        let obj_addr = &*fake_obj as *const u64 as u64;

        let sel = CString::new("nonexistent").unwrap();
        assert_eq!(objc_has_method(obj_addr, sel.as_ptr() as u64), 0);
    }

    #[test]
    fn super_lookup_null() {
        // receiver == 0 triggers the early return.
        assert_eq!(objc_msg_super_lookup_imp(0, 0, 0), 0);
    }

    #[test]
    fn super2_lookup_null() {
        // receiver == 0 triggers the early return.
        assert_eq!(objc_msg_super2_lookup_imp(0, 0, 0), 0);
    }

    #[test]
    fn lookup_nil_sel_returns_zero_or_stub() {
        // Both nil: obj == 0 causes immediate return of 0.
        assert_eq!(objc_msg_lookup_imp(0, 0), 0);
        // Verify the stub function itself returns 0.
        assert_eq!(unrecognized_selector_stub(0, 0, 0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn has_method_unregistered_isa_returns_zero() {
        // Object pointing to an ISA that was never registered with the runtime.
        let unregistered_isa = 0x0000_0009_0000_8000u64;
        let fake_obj: Box<u64> = Box::new(unregistered_isa);
        let obj_addr = &*fake_obj as *const u64 as u64;
        let sel = CString::new("anything").unwrap();
        assert_eq!(objc_has_method(obj_addr, sel.as_ptr() as u64), 0);
    }
}
