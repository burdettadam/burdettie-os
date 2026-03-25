//! NSInvocation invoke/call_imp logic and helper constructor.

use crate::nsobj;

/// Maximum number of register-passed arguments we support (x0-x7 on ARM64).
const MAX_REG_ARGS: usize = 8;

/// `-[NSInvocation invoke]`
///
/// Calls the target with the stored selector and arguments.
/// The return value is stored in the return buffer.
pub extern "C" fn shim_invoke(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }

    let (target, sel, arg_buf, ret_buf, num_args) = unsafe {
        let target = super::get_target(obj);
        let sel = super::get_selector(obj);
        let arg_buf = super::get_arg_buffer(obj);
        let ret_buf = super::get_ret_buffer(obj);

        // Get numberOfArguments from the method signature
        let sig = super::get_signature(obj);
        let num_args = if sig != 0 {
            *((sig + 16) as *const u64) as usize
        } else {
            2 // default: self + _cmd
        };

        (target, sel, arg_buf, ret_buf, num_args)
    };

    if target == 0 || sel == 0 {
        return;
    }

    // Look up the IMP via the runtime
    let guard = match objc_runtime::OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return;
    };

    let isa = unsafe { *(target as *const u64) };
    let sel_str = unsafe {
        std::ffi::CStr::from_ptr(sel as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };

    let Some(imp) = rt.msg_lookup(isa, sel_str) else {
        return;
    };

    // Drop the lock before calling the IMP
    drop(guard);

    // Read arguments from the buffer
    let args: [u64; MAX_REG_ARGS] = if arg_buf != 0 {
        unsafe {
            let mut a = [0u64; MAX_REG_ARGS];
            std::ptr::copy_nonoverlapping(arg_buf as *const u64, a.as_mut_ptr(), MAX_REG_ARGS);
            // Ensure target and selector are current
            a[0] = target;
            a[1] = sel;
            a
        }
    } else {
        let mut a = [0u64; MAX_REG_ARGS];
        a[0] = target;
        a[1] = sel;
        a
    };

    // Call the IMP with the appropriate number of arguments
    let result = unsafe { call_imp(imp, &args, num_args) };

    // Store return value
    if ret_buf != 0 {
        unsafe {
            *(ret_buf as *mut u64) = result;
        }
    }
}

/// `-[NSInvocation invokeWithTarget:]`
pub extern "C" fn shim_invoke_with_target(obj: u64, _cmd: u64, target: u64) {
    if obj == 0 {
        return;
    }
    super::shim_set_target(obj, 0, target);
    shim_invoke(obj, 0);
}

// ── Internal: call IMP with N arguments ────────────────────────────

/// Call an IMP function pointer with the given arguments.
///
/// Supports up to 8 register-passed arguments (ARM64 x0-x7).
unsafe fn call_imp(imp: u64, args: &[u64; MAX_REG_ARGS], num_args: usize) -> u64 {
    match num_args {
        0 | 1 => {
            let f: extern "C" fn(u64) -> u64 = std::mem::transmute(imp);
            f(args[0])
        }
        2 => {
            let f: extern "C" fn(u64, u64) -> u64 = std::mem::transmute(imp);
            f(args[0], args[1])
        }
        3 => {
            let f: extern "C" fn(u64, u64, u64) -> u64 = std::mem::transmute(imp);
            f(args[0], args[1], args[2])
        }
        4 => {
            let f: extern "C" fn(u64, u64, u64, u64) -> u64 = std::mem::transmute(imp);
            f(args[0], args[1], args[2], args[3])
        }
        5 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64) -> u64 = std::mem::transmute(imp);
            f(args[0], args[1], args[2], args[3], args[4])
        }
        6 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            f(args[0], args[1], args[2], args[3], args[4], args[5])
        }
        7 => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            f(args[0], args[1], args[2], args[3], args[4], args[5], args[6])
        }
        _ => {
            let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64 =
                std::mem::transmute(imp);
            f(
                args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
            )
        }
    }
}

// ── Helper for creating NSInvocation from Rust ─────────────────────

/// Create an NSInvocation with the given method signature, target, selector,
/// and arguments. Used by the forwarding trampoline.
#[allow(dead_code)]
pub fn create_invocation(sig: u64, target: u64, sel: u64, args: &[u64]) -> u64 {
    let inv = super::shim_invocation_with_method_signature(
        nsobj::nsinvocation_class_addr(),
        0,
        sig,
    );
    if inv == 0 {
        return 0;
    }
    super::shim_set_target(inv, 0, target);
    super::shim_set_selector(inv, 0, sel);

    // Copy additional arguments (index 2+)
    for (i, &arg) in args.iter().enumerate() {
        let index = (i + 2) as u64;
        if index >= MAX_REG_ARGS as u64 {
            break;
        }
        // Write directly to the buffer
        unsafe {
            let buf = super::get_arg_buffer(inv);
            if buf != 0 {
                *((buf as *mut u64).add(index as usize)) = arg;
            }
        }
    }

    inv
}
