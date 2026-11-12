//! NSInvocation shim — encapsulates target, selector, arguments, and return value.
//!
//! Layout (48 bytes):
//!   offset  0: ISA pointer (u64)
//!   offset  8: methodSignature pointer (u64) — pointer to NSMethodSignature obj
//!   offset 16: target (u64) — the receiver
//!   offset 24: selector (u64) — pointer to selector C string
//!   offset 32: argBuffer pointer (u64) — heap-allocated buffer for arguments
//!   offset 40: retBuffer pointer (u64) — heap-allocated buffer for return value

mod invoke;

pub use invoke::{shim_invoke, shim_invoke_with_target};

use crate::nsobj;

/// Instance size for NSInvocation objects.
pub const INSTANCE_SIZE: u32 = 48;

/// Maximum number of register-passed arguments we support (x0-x7 on ARM64).
const MAX_REG_ARGS: usize = 8;

// ── Field accessors ────────────────────────────────────────────────

#[inline]
pub(super) unsafe fn get_signature(obj: u64) -> u64 {
    *((obj + 8) as *const u64)
}
#[inline]
pub(super) unsafe fn get_target(obj: u64) -> u64 {
    *((obj + 16) as *const u64)
}
#[inline]
pub(super) unsafe fn get_selector(obj: u64) -> u64 {
    *((obj + 24) as *const u64)
}
#[inline]
pub(super) unsafe fn get_arg_buffer(obj: u64) -> u64 {
    *((obj + 32) as *const u64)
}
#[inline]
pub(super) unsafe fn get_ret_buffer(obj: u64) -> u64 {
    *((obj + 40) as *const u64)
}

// ── Shim functions ─────────────────────────────────────────────────

/// `+[NSInvocation invocationWithMethodSignature:]`
pub extern "C" fn shim_invocation_with_method_signature(_cls: u64, _cmd: u64, sig: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        // ISA
        *(obj as *mut u64) = nsobj::nsinvocation_class_addr();
        // methodSignature
        *((obj + 8) as *mut u64) = sig;
    }

    // Allocate argument buffer (8 slots × 8 bytes = 64 bytes)
    let arg_buf = crate::pal_alloc().calloc(MAX_REG_ARGS, 8) as u64;
    if arg_buf != 0 {
        unsafe {
            *((obj + 32) as *mut u64) = arg_buf;
        }
    }

    // Allocate return value buffer (8 bytes)
    let ret_buf = crate::pal_alloc().calloc(1, 8) as u64;
    if ret_buf != 0 {
        unsafe {
            *((obj + 40) as *mut u64) = ret_buf;
        }
    }

    log::debug!("NSInvocation invocationWithMethodSignature: → {:#x}", obj);
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
            crate::pal_alloc().free(arg_buf as *mut u8);
        }
        let ret_buf = get_ret_buffer(obj);
        if ret_buf != 0 {
            crate::pal_alloc().free(ret_buf as *mut u8);
        }
        crate::pal_alloc().free(obj as *mut u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn alloc_inv() -> u64 {
        let obj = alloc_obj(
            INSTANCE_SIZE as usize,
            crate::nsobj::nsinvocation_class_addr(),
        );
        let arg_buf = crate::pal_alloc().calloc(8, 8) as u64;
        let ret_buf = crate::pal_alloc().calloc(1, 8) as u64;
        write_field(obj, 32, arg_buf);
        write_field(obj, 40, ret_buf);
        obj
    }

    fn free_inv(obj: u64) {
        free_buf(read_field(obj, 32));
        free_buf(read_field(obj, 40));
        free_obj(obj);
    }

    #[test]
    fn nil_getters_return_zero() {
        assert_eq!(shim_method_signature(0, 0), 0);
        assert_eq!(shim_target(0, 0), 0);
        assert_eq!(shim_selector(0, 0), 0);
    }

    #[test]
    fn nil_setters_no_panic() {
        shim_set_target(0, 0, 0xABC);
        shim_set_selector(0, 0, 0xDEF);
        shim_set_argument_at_index(0, 0, 0, 0);
    }

    #[test]
    fn create_with_signature_stores_sig() {
        let sig = 0x9999u64;
        let inv = shim_invocation_with_method_signature(0, 0, sig);
        assert_ne!(inv, 0);
        assert_eq!(shim_method_signature(inv, 0), sig);
        shim_dealloc(inv, 0);
    }

    #[test]
    fn target_roundtrip() {
        let inv = alloc_inv();
        shim_set_target(inv, 0, 0x1234);
        assert_eq!(shim_target(inv, 0), 0x1234);
        free_inv(inv);
    }

    #[test]
    fn selector_roundtrip() {
        let inv = alloc_inv();
        shim_set_selector(inv, 0, 0x5678);
        assert_eq!(shim_selector(inv, 0), 0x5678);
        free_inv(inv);
    }

    #[test]
    fn argument_roundtrip() {
        let inv = alloc_inv();
        let val: u64 = 0xABCD;
        shim_set_argument_at_index(inv, 0, &val as *const u64 as u64, 2);
        let mut out: u64 = 0;
        shim_get_argument_at_index(inv, 0, &mut out as *mut u64 as u64, 2);
        assert_eq!(out, 0xABCD);
        free_inv(inv);
    }

    #[test]
    fn return_value_roundtrip() {
        let inv = alloc_inv();
        let val: u64 = 0x7777;
        shim_set_return_value(inv, 0, &val as *const u64 as u64);
        let mut out: u64 = 0;
        shim_get_return_value(inv, 0, &mut out as *mut u64 as u64);
        assert_eq!(out, 0x7777);
        free_inv(inv);
    }

    #[test]
    fn arguments_retained_always_one() {
        assert_eq!(shim_arguments_retained(0, 0), 1);
    }
}
