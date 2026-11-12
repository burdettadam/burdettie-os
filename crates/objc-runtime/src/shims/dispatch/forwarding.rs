//! ObjC forwarding chain implementation.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

/// Forwarding trampoline: called when a known ISA has no method for the selector.
///
/// Implements the ObjC forwarding chain:
/// 1. Call `-methodSignatureForSelector:` on the receiver
/// 2. If non-nil, create an `NSInvocation` and call `-forwardInvocation:`
/// 3. Return the invocation's return value (or 0)
pub extern "C" fn forwarding_trampoline(
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
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char).to_str().unwrap_or("") };

    // Step 1: Look up methodSignatureForSelector: IMP
    let sig_imp = {
        let guard = match OBJC_RUNTIME.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(rt) = guard.as_ref() else { return 0 };
        rt.msg_lookup(isa, "methodSignatureForSelector:")
    };

    let Some(sig_imp) = sig_imp else {
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
        let dnr_imp = {
            let guard = match OBJC_RUNTIME.read() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            let Some(rt) = guard.as_ref() else { return 0 };
            rt.msg_lookup(isa, "doesNotRecognizeSelector:")
        };
        if let Some(dnr_imp) = dnr_imp {
            let dnr_fn: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(dnr_imp) };
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
    let num_args = unsafe { *((signature + 16) as *const u64) } as usize;

    let inv = crate::pal_alloc().calloc(1, 48) as u64;
    if inv == 0 {
        return 0;
    }

    let inv_isa = {
        let guard = match OBJC_RUNTIME.read() {
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

    unsafe {
        *(inv as *mut u64) = inv_isa;
        *((inv + 8) as *mut u64) = signature;
        *((inv + 16) as *mut u64) = obj;
        *((inv + 24) as *mut u64) = sel;

        let arg_buf = crate::pal_alloc().calloc(8, 8) as u64;
        *((inv + 32) as *mut u64) = arg_buf;

        if arg_buf != 0 {
            let buf = arg_buf as *mut u64;
            *buf = obj;
            *buf.add(1) = sel;
            if num_args > 2 {
                *buf.add(2) = a2;
            }
            if num_args > 3 {
                *buf.add(3) = a3;
            }
            if num_args > 4 {
                *buf.add(4) = a4;
            }
            if num_args > 5 {
                *buf.add(5) = a5;
            }
            if num_args > 6 {
                *buf.add(6) = a6;
            }
            if num_args > 7 {
                *buf.add(7) = a7;
            }
        }

        let ret_buf = crate::pal_alloc().calloc(1, 8) as u64;
        *((inv + 40) as *mut u64) = ret_buf;
    }

    // Step 3: Look up and call forwardInvocation:
    let fwd_imp = {
        let guard = match OBJC_RUNTIME.read() {
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

    unsafe {
        let arg_buf = *((inv + 32) as *const u64);
        if arg_buf != 0 {
            crate::pal_alloc().free(arg_buf as *mut u8);
        }
        let ret_buf = *((inv + 40) as *const u64);
        if ret_buf != 0 {
            crate::pal_alloc().free(ret_buf as *mut u8);
        }
        crate::pal_alloc().free(inv as *mut u8);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarding_nil_receiver_returns_zero() {
        assert_eq!(forwarding_trampoline(0, 0, 0, 0, 0, 0, 0, 0), 0);
    }
}
