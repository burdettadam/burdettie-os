//! Rust-only fallbacks (non-aarch64 platforms).

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;
use super::lookup::{objc_msg_super_lookup_imp, objc_msg_super2_lookup_imp};

#[allow(dead_code)]
pub extern "C" fn shim_objc_msg_send_rust(
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
pub extern "C" fn shim_objc_msg_send_super_rust(
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
pub extern "C" fn shim_objc_msg_send_super2_rust(
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
