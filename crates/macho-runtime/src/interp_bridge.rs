//! Bridge between the ARM64 interpreter and the runtime's shim infrastructure.
//!
//! When executing a Mach-O binary via the interpreter, branches to external
//! symbols (ObjC runtime, Foundation, libc) must be routed to the host
//! implementations. This module builds an [`FfiBridge`] that:
//!
//! - Registers special handlers for `objc_msgSend` variants that perform
//!   IMP lookup and either tail-call to guest code or call host shims directly.
//! - Enables `auto_call_host` so all other resolved symbols (shims + libc)
//!   are called transparently through their function pointers.
//!
//! This lets the interpreter run ObjC and Foundation fixtures end-to-end.

use arm64_translate::{FfiBridge, FfiResult};
use objc_runtime::shims::dispatch::{
    objc_msg_send_addr, objc_msg_send_stret_addr, objc_msg_send_fp2ret_addr,
    objc_msg_send_super_addr, objc_msg_send_super_stret_addr,
    objc_msg_send_super2_addr, objc_msg_send_super2_stret_addr,
};

extern "C" {
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const std::ffi::c_char) -> *mut std::ffi::c_void;
}

/// Build an [`FfiBridge`] for interpreter use with the full runtime.
///
/// `guest_ranges` should be `(base_addr, end_addr)` pairs from the mapped
/// image segments. These are used to distinguish guest IMPs (tail-call back
/// into the interpreter) from host IMPs (call directly).
pub fn build_interpreter_bridge(guest_ranges: &[(u64, u64)]) -> FfiBridge {
    let mut bridge = FfiBridge::new();
    bridge.auto_call_host = true;

    // Register objc_msgSend variants with IMP-dispatch logic.
    register_msg_send(&mut bridge, objc_msg_send_addr(), "_objc_msgSend", guest_ranges);
    register_msg_send(&mut bridge, objc_msg_send_stret_addr(), "_objc_msgSend_stret", guest_ranges);
    register_msg_send(&mut bridge, objc_msg_send_fp2ret_addr(), "_objc_msgSend_fp2ret", guest_ranges);
    register_msg_send_super(&mut bridge, objc_msg_send_super_addr(), "_objc_msgSendSuper", guest_ranges);
    register_msg_send_super(&mut bridge, objc_msg_send_super_stret_addr(), "_objc_msgSendSuper_stret", guest_ranges);
    register_msg_send_super2(&mut bridge, objc_msg_send_super2_addr(), "_objc_msgSendSuper2", guest_ranges);
    register_msg_send_super2(&mut bridge, objc_msg_send_super2_stret_addr(), "_objc_msgSendSuper2_stret", guest_ranges);

    // Register exit/_exit handlers so the interpreter terminates cleanly
    // instead of auto_call_host actually calling libc::exit (which would
    // kill the test process).
    for sym in &["_exit", "exit", "__exit"] {
        let c_name = std::ffi::CString::new(*sym).unwrap();
        let addr = unsafe {
            // SAFETY: looking up a well-known libc symbol via dlsym.
            let ptr = dlsym(std::ptr::null_mut(), c_name.as_ptr());
            ptr as u64
        };
        if addr != 0 {
            let name = sym.to_string();
            bridge.register(addr, &name, Box::new(|args| FfiResult::Exit(args[0])));
        }
    }

    bridge
}

/// Register an `objc_msgSend` handler: look up IMP, then tail-call or direct-call.
fn register_msg_send(bridge: &mut FfiBridge, addr: u64, name: &str, guest_ranges: &[(u64, u64)]) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    let handler_name = name.to_string();
    bridge.register(addr, name, Box::new(move |args: &[u64; 8]| {
        let (obj, sel) = (args[0], args[1]);

        // Nil message → return 0 (matches Apple ABI).
        if obj == 0 {
            return FfiResult::Return(0);
        }

        let imp = objc_runtime::shims::dispatch::lookup::objc_msg_lookup_imp(obj, sel);
        log::debug!(
            "{}: obj={obj:#x} sel={sel:#x} → imp={imp:#x} (guest={})",
            handler_name,
            if imp != 0 { is_in_guest(imp, &ranges) } else { false },
        );
        if imp == 0 {
            log::warn!("objc_msgSend: lookup returned 0 for obj={obj:#x} sel={sel:#x}");
            return FfiResult::Return(0);
        }

        // If IMP is within guest memory, tail-call back into the interpreter.
        if is_in_guest(imp, &ranges) {
            return FfiResult::TailCall(imp);
        }

        // Host IMP (a shim function) — call it directly.
        // SAFETY: IMP comes from the ObjC runtime's method tables, which
        // were populated from our shim registrations.
        unsafe {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            FfiResult::Return(f(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7]))
        }
    }));
}

/// Register an `objc_msgSendSuper` handler.
/// x0 = &objc_super, x1 = SEL. The objc_super struct is { id receiver; Class super_class; }.
fn register_msg_send_super(bridge: &mut FfiBridge, addr: u64, name: &str, guest_ranges: &[(u64, u64)]) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    bridge.register(addr, name, Box::new(move |args: &[u64; 8]| {
        let super_ptr = args[0];
        let sel = args[1];

        if super_ptr == 0 {
            return FfiResult::Return(0);
        }

        // Read the objc_super struct: { id receiver; Class super_class; }
        let receiver = unsafe { *(super_ptr as *const u64) };
        let super_class = unsafe { *((super_ptr + 8) as *const u64) };

        // Look up method on the superclass.
        let imp = objc_runtime::shims::dispatch::lookup::objc_msg_super_lookup_imp(receiver, super_class, sel);
        if imp == 0 {
            return FfiResult::Return(0);
        }

        if is_in_guest(imp, &ranges) {
            // For super sends, the real receiver goes in x0 (not the super struct).
            // We can't modify registers from here, so we call the IMP directly
            // for host IMPs, or TailCall for guest IMPs.
            // TODO: For guest IMPs, we'd need to patch x0 to receiver.
            // For now, just call host IMP:
            return FfiResult::TailCall(imp);
        }

        unsafe {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            FfiResult::Return(f(receiver, sel, args[2], args[3], args[4], args[5], args[6], args[7]))
        }
    }));
}

/// Register an `objc_msgSendSuper2` handler (super_class is the class itself, not the superclass).
fn register_msg_send_super2(bridge: &mut FfiBridge, addr: u64, name: &str, guest_ranges: &[(u64, u64)]) {
    let ranges: Vec<(u64, u64)> = guest_ranges.to_vec();
    bridge.register(addr, name, Box::new(move |args: &[u64; 8]| {
        let super_ptr = args[0];
        let sel = args[1];

        if super_ptr == 0 {
            return FfiResult::Return(0);
        }

        let receiver = unsafe { *(super_ptr as *const u64) };
        let class_ptr = unsafe { *((super_ptr + 8) as *const u64) };

        // For msgSendSuper2, the second field is the CLASS (not superclass).
        // We need to get the superclass from the runtime.
        let imp = objc_runtime::shims::dispatch::lookup::objc_msg_super2_lookup_imp(receiver, class_ptr, sel);
        if imp == 0 {
            return FfiResult::Return(0);
        }

        if is_in_guest(imp, &ranges) {
            return FfiResult::TailCall(imp);
        }

        unsafe {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            FfiResult::Return(f(receiver, sel, args[2], args[3], args[4], args[5], args[6], args[7]))
        }
    }));
}

/// Check if an address is within the guest (mapped image) memory ranges.
fn is_in_guest(addr: u64, ranges: &[(u64, u64)]) -> bool {
    ranges.iter().any(|&(lo, hi)| addr >= lo && addr < hi)
}
