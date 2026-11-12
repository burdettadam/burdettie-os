//! Rust-only fallbacks (non-aarch64 platforms).

use std::ffi::{c_char, CStr};

use super::lookup::{objc_msg_super2_lookup_imp, objc_msg_super_lookup_imp};
use crate::class::OBJC_RUNTIME;

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

    let guard = match OBJC_RUNTIME.read() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_send_rust_nil_receiver() {
        // obj (a0) == 0 → immediate return 0.
        assert_eq!(shim_objc_msg_send_rust(0, 0, 0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn msg_send_super_rust_null() {
        // Build a super struct with receiver=0, super_class=0.
        let super_struct: [u64; 2] = [0, 0];
        let ptr = super_struct.as_ptr() as u64;
        // receiver == 0 → objc_msg_super_lookup_imp returns 0 → returns 0.
        assert_eq!(shim_objc_msg_send_super_rust(ptr, 0, 0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn msg_send_super2_rust_null() {
        let super_struct: [u64; 2] = [0, 0];
        let ptr = super_struct.as_ptr() as u64;
        assert_eq!(shim_objc_msg_send_super2_rust(ptr, 0, 0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    #[ignore = "shim_objc_msg_send_rust calls process::abort() for unknown ISA"]
    fn msg_send_rust_invalid_obj() {
        // Would require an obj whose ISA isn't registered, but the function
        // calls std::process::abort() in that case — can't assert in-process.
        let fake_obj: Box<u64> = Box::new(0xDEAD_0000_0000_0000);
        let obj_addr = &*fake_obj as *const u64 as u64;
        let _ = shim_objc_msg_send_rust(obj_addr, 0, 0, 0, 0, 0, 0, 0);
    }
}
