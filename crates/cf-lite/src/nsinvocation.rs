//! NSInvocation shim — encapsulates target, selector, arguments, and return value.
//!
//! Layout (48 bytes):
//!   offset  0: ISA pointer (u64)
//!   offset  8: methodSignature pointer (u64) — pointer to NSMethodSignature obj
//!   offset 16: target (u64) — the receiver
//!   offset 24: selector (u64) — pointer to selector C string
//!   offset 32: argBuffer pointer (u64) — heap-allocated buffer for arguments
//!   offset 40: retBuffer pointer (u64) — heap-allocated buffer for return value

use crate::nsobj;

/// Instance size for NSInvocation objects.
pub const INSTANCE_SIZE: u32 = 48;

/// Maximum number of register-passed arguments we support (x0-x7 on ARM64).
const MAX_REG_ARGS: usize = 8;

// ── Field accessors ────────────────────────────────────────────────

#[inline]
unsafe fn get_signature(obj: u64) -> u64 {
    *((obj + 8) as *const u64)
}
#[inline]
unsafe fn get_target(obj: u64) -> u64 {
    *((obj + 16) as *const u64)
}
#[inline]
unsafe fn get_selector(obj: u64) -> u64 {
    *((obj + 24) as *const u64)
}
#[inline]
unsafe fn get_arg_buffer(obj: u64) -> u64 {
    *((obj + 32) as *const u64)
}
#[inline]
unsafe fn get_ret_buffer(obj: u64) -> u64 {
    *((obj + 40) as *const u64)
}

// ── Shim functions ─────────────────────────────────────────────────

/// `+[NSInvocation invocationWithMethodSignature:]`
pub extern "C" fn shim_invocation_with_method_signature(
    _cls: u64,
    _cmd: u64,
    sig: u64,
) -> u64 {
    if sig == 0 {
        return 0;
    }

    let obj = unsafe { libc::calloc(1, INSTANCE_SIZE as usize) as u64 };
    if obj == 0 {
        return 0;
    }

    // Set ISA
    let isa = nsobj::nsinvocation_class_addr();
    unsafe {
        *(obj as *mut u64) = isa;
    }

    // Set methodSignature
    unsafe {
        *((obj + 8) as *mut u64) = sig;
    }

    // Allocate argument buffer (MAX_REG_ARGS * 8 bytes)
    let arg_buf = unsafe { libc::calloc(MAX_REG_ARGS, 8) as u64 };
    unsafe {
        *((obj + 32) as *mut u64) = arg_buf;
    }

    // Allocate return buffer (8 bytes for single return value)
    let ret_buf = unsafe { libc::calloc(1, 8) as u64 };
    unsafe {
        *((obj + 40) as *mut u64) = ret_buf;
    }

    obj
}

/// `-[NSInvocation methodSignature]`
pub extern "C" fn shim_method_signature(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { get_signature(obj) }
}

/// `-[NSInvocation target]`
pub extern "C" fn shim_target(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { get_target(obj) }
}

/// `-[NSInvocation setTarget:]`
pub extern "C" fn shim_set_target(obj: u64, _cmd: u64, target: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj + 16) as *mut u64) = target;
        // Also set target in arg buffer at index 0
        let buf = get_arg_buffer(obj);
        if buf != 0 {
            *(buf as *mut u64) = target;
        }
    }
}

/// `-[NSInvocation selector]`
pub extern "C" fn shim_selector(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { get_selector(obj) }
}

/// `-[NSInvocation setSelector:]`
pub extern "C" fn shim_set_selector(obj: u64, _cmd: u64, sel: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj + 24) as *mut u64) = sel;
        // Also set selector in arg buffer at index 1
        let buf = get_arg_buffer(obj);
        if buf != 0 {
            *((buf + 8) as *mut u64) = sel;
        }
    }
}

/// `-[NSInvocation setArgument:atIndex:]`
///
/// Copies 8 bytes from the pointer into the argument buffer at the given index.
/// Index 0 = self (target), index 1 = _cmd (selector), index 2+ = method args.
pub extern "C" fn shim_set_argument_at_index(obj: u64, _cmd: u64, arg_ptr: u64, index: u64) {
    if obj == 0 || arg_ptr == 0 {
        return;
    }
    let idx = index as usize;
    if idx >= MAX_REG_ARGS {
        return;
    }
    unsafe {
        let buf = get_arg_buffer(obj);
        if buf == 0 {
            return;
        }
        let src = arg_ptr as *const u64;
        let dst = (buf as *mut u64).add(idx);
        *dst = *src;

        // Keep target/selector fields in sync
        if idx == 0 {
            *((obj + 16) as *mut u64) = *src;
        } else if idx == 1 {
            *((obj + 24) as *mut u64) = *src;
        }
    }
}

/// `-[NSInvocation getArgument:atIndex:]`
///
/// Copies 8 bytes from the argument buffer at the given index to the pointer.
pub extern "C" fn shim_get_argument_at_index(obj: u64, _cmd: u64, arg_ptr: u64, index: u64) {
    if obj == 0 || arg_ptr == 0 {
        return;
    }
    let idx = index as usize;
    if idx >= MAX_REG_ARGS {
        return;
    }
    unsafe {
        let buf = get_arg_buffer(obj);
        if buf == 0 {
            return;
        }
        let src = (buf as *const u64).add(idx);
        let dst = arg_ptr as *mut u64;
        *dst = *src;
    }
}

/// `-[NSInvocation invoke]`
///
/// Calls the target with the stored selector and arguments.
/// The return value is stored in the return buffer.
pub extern "C" fn shim_invoke(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }

    let (target, sel, arg_buf, ret_buf, num_args) = unsafe {
        let target = get_target(obj);
        let sel = get_selector(obj);
        let arg_buf = get_arg_buffer(obj);
        let ret_buf = get_ret_buffer(obj);

        // Get numberOfArguments from the method signature
        let sig = get_signature(obj);
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

    let imp = match rt.msg_lookup(isa, sel_str) {
        Some(imp) => imp,
        None => return,
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
    shim_set_target(obj, 0, target);
    shim_invoke(obj, 0);
}

/// `-[NSInvocation getReturnValue:]`
///
/// Copies the return value (8 bytes) to the given pointer.
pub extern "C" fn shim_get_return_value(obj: u64, _cmd: u64, ret_ptr: u64) {
    if obj == 0 || ret_ptr == 0 {
        return;
    }
    unsafe {
        let buf = get_ret_buffer(obj);
        if buf == 0 {
            return;
        }
        *(ret_ptr as *mut u64) = *(buf as *const u64);
    }
}

/// `-[NSInvocation setReturnValue:]`
pub extern "C" fn shim_set_return_value(obj: u64, _cmd: u64, ret_ptr: u64) {
    if obj == 0 || ret_ptr == 0 {
        return;
    }
    unsafe {
        let buf = get_ret_buffer(obj);
        if buf == 0 {
            return;
        }
        *(buf as *mut u64) = *(ret_ptr as *const u64);
    }
}

/// `-[NSInvocation retainArguments]` — no-op in our runtime (no refcounting needed)
pub extern "C" fn shim_retain_arguments(_obj: u64, _cmd: u64) {}

/// `-[NSInvocation argumentsRetained]` — always returns YES
pub extern "C" fn shim_arguments_retained(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSInvocation dealloc]`
pub extern "C" fn shim_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        let arg_buf = get_arg_buffer(obj);
        if arg_buf != 0 {
            libc::free(arg_buf as *mut libc::c_void);
        }
        let ret_buf = get_ret_buffer(obj);
        if ret_buf != 0 {
            libc::free(ret_buf as *mut libc::c_void);
        }
        libc::free(obj as *mut libc::c_void);
    }
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
pub fn create_invocation(sig: u64, target: u64, sel: u64, args: &[u64]) -> u64 {
    let inv = shim_invocation_with_method_signature(nsobj::nsinvocation_class_addr(), 0, sig);
    if inv == 0 {
        return 0;
    }
    shim_set_target(inv, 0, target);
    shim_set_selector(inv, 0, sel);

    // Copy additional arguments (index 2+)
    for (i, &arg) in args.iter().enumerate() {
        let index = (i + 2) as u64;
        if index >= MAX_REG_ARGS as u64 {
            break;
        }
        // Write directly to the buffer
        unsafe {
            let buf = get_arg_buffer(inv);
            if buf != 0 {
                *((buf as *mut u64).add(index as usize)) = arg;
            }
        }
    }

    inv
}
