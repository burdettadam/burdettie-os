//! NSSortDescriptor shim: key-path-based sort criteria.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  key_ptr (u64) — pointer to heap C string of the property key
//!   [16..24): ascending (u64) — 0=descending, 1=ascending
//!
//! Comparison delegates to `objc_msgSend` on the two objects:
//!   1. Send `key` selector to each object to get the property value.
//!   2. Send `compare:` to the first value with the second value.
//!   3. Flip sign if descending.
//!
//! For our runtime, we support string-key-based sorting using the ObjC
//! runtime's msg_lookup to dispatch the key getter and `compare:`.

use crate::cfstring;
use crate::nsobj;

const NSSORTDESCRIPTOR_SIZE: usize = 24;

/// Forward `compare:` to the host's `objc_msgSend`.
fn host_compare(a: u64, b: u64) -> i64 {
    type SelRegFn = unsafe extern "C" fn(*const i8) -> u64;
    type MsgSend3 = unsafe extern "C" fn(u64, u64, u64) -> i64;
    let rtld = (-2isize) as *mut libc::c_void;
    let sel_reg = unsafe { libc::dlsym(rtld, c"sel_registerName".as_ptr()) };
    let msg_send = unsafe { libc::dlsym(rtld, c"objc_msgSend".as_ptr()) };
    if !sel_reg.is_null() && !msg_send.is_null() {
        let sel_fn: SelRegFn = unsafe { std::mem::transmute(sel_reg) };
        let send_fn: MsgSend3 = unsafe { std::mem::transmute(msg_send) };
        let sel = unsafe { sel_fn(c"compare:".as_ptr()) };
        unsafe { send_fn(a, sel, b) }
    } else {
        0
    }
}

fn alloc_nssortdescriptor(key: &str, ascending: bool) -> u64 {
    let obj = unsafe { libc::calloc(1, NSSORTDESCRIPTOR_SIZE) } as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let c_key = std::ffi::CString::new(key).unwrap_or_default();
    let key_ptr = unsafe {
        let len = c_key.as_bytes_with_nul().len();
        let buf = libc::malloc(len) as *mut u8;
        if !buf.is_null() {
            std::ptr::copy_nonoverlapping(c_key.as_ptr() as *const u8, buf, len);
        }
        buf
    };
    unsafe {
        *obj = nsobj::nssortdescriptor_class_addr();
        *obj.add(1) = key_ptr as u64;
        *obj.add(2) = ascending as u64;
    }
    obj as u64
}

/// Read the key as a &str from the stored C string pointer.
fn read_key(obj: u64) -> &'static str {
    let key_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if key_ptr.is_null() {
        return "";
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(key_ptr) };
    cstr.to_str().unwrap_or("")
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSSortDescriptor sortDescriptorWithKey:ascending:]`
pub extern "C" fn shim_nssortdescriptor_with_key_ascending(
    _cls: u64,
    _cmd: u64,
    key: u64,
    ascending: u64,
) -> u64 {
    if key == 0 {
        return alloc_nssortdescriptor("self", ascending != 0);
    }
    let Some(key_str) = cfstring::extract_cfstring_cstr(key) else {
        return alloc_nssortdescriptor("self", ascending != 0);
    };
    alloc_nssortdescriptor(key_str, ascending != 0)
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSSortDescriptor key]` — returns the key as NSString.
pub extern "C" fn shim_nssortdescriptor_key(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let key_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if key_ptr.is_null() {
        return 0;
    }
    cfstring::shim_cf_string_create_with_cstring(0, key_ptr, 0)
}

/// `-[NSSortDescriptor ascending]`
pub extern "C" fn shim_nssortdescriptor_ascending(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(2)) as u8 }
}

/// `-[NSSortDescriptor compareObject:toObject:]`
///
/// Uses the ObjC runtime to send the key selector to both objects,
/// then sends `compare:` on the first result with the second.
/// Returns NSOrderedAscending (-1), NSOrderedSame (0), or NSOrderedDescending (1).
pub extern "C" fn shim_nssortdescriptor_compare_object(obj: u64, _cmd: u64, a: u64, b: u64) -> i64 {
    if obj == 0 || a == 0 || b == 0 {
        return 0;
    }
    let key = read_key(obj);
    let ascending = unsafe { *((obj as *const u64).add(2)) } != 0;

    // Look up the key getter on both objects via the ObjC runtime.
    // Tagged pointers (bit 63 set) can only be dispatched by the host runtime.
    // For key=="self" we short-circuit, so tagged pointers are fine there.
    let (val_a, val_b) = {
        let guard = objc_runtime::OBJC_RUNTIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = guard.as_ref() {
            let va = if key == "self" || key.is_empty() {
                a
            } else {
                // Skip ISA deref on tagged pointers — fall back to identity
                #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
                if a & (1u64 << 63) != 0 {
                    a
                } else {
                    let isa_a = unsafe { *(a as *const u64) };
                    rt.msg_lookup(isa_a, key)
                        .map(|imp| {
                            let f: extern "C" fn(u64, u64) -> u64 =
                                unsafe { std::mem::transmute(imp) };
                            f(a, 0)
                        })
                        .unwrap_or(a)
                }
                #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
                {
                    let isa_a = unsafe { *(a as *const u64) };
                    rt.msg_lookup(isa_a, key)
                        .map(|imp| {
                            let f: extern "C" fn(u64, u64) -> u64 =
                                unsafe { std::mem::transmute(imp) };
                            f(a, 0)
                        })
                        .unwrap_or(a)
                }
            };

            let vb = if key == "self" || key.is_empty() {
                b
            } else {
                #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
                if b & (1u64 << 63) != 0 {
                    b
                } else {
                    let isa_b = unsafe { *(b as *const u64) };
                    rt.msg_lookup(isa_b, key)
                        .map(|imp| {
                            let f: extern "C" fn(u64, u64) -> u64 =
                                unsafe { std::mem::transmute(imp) };
                            f(b, 0)
                        })
                        .unwrap_or(b)
                }
                #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
                {
                    let isa_b = unsafe { *(b as *const u64) };
                    rt.msg_lookup(isa_b, key)
                        .map(|imp| {
                            let f: extern "C" fn(u64, u64) -> u64 =
                                unsafe { std::mem::transmute(imp) };
                            f(b, 0)
                        })
                        .unwrap_or(b)
                }
            };

            (va, vb)
        } else {
            (a, b)
        }
    };

    // Now compare: send `compare:` to val_a with val_b
    let cmp = {
        // Tagged pointer check: bit 63 set = host runtime tagged object.
        // Cannot dereference to read ISA — forward to host objc_msgSend.
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        if val_a & (1u64 << 63) != 0 {
            let result = host_compare(val_a, val_b);
            return if ascending { result } else { -result };
        }

        let guard = objc_runtime::OBJC_RUNTIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = guard.as_ref() {
            let isa_a = unsafe { *(val_a as *const u64) };
            if let Some(imp) = rt.msg_lookup(isa_a, "compare:") {
                let f: extern "C" fn(u64, u64, u64) -> i64 = unsafe { std::mem::transmute(imp) };
                f(val_a, 0, val_b)
            } else {
                // ISA not known or compare: not found — forward to host objc_msgSend
                drop(guard);
                host_compare(val_a, val_b)
            }
        } else {
            host_compare(val_a, val_b)
        }
    };

    if ascending {
        cmp
    } else {
        -cmp
    }
}

/// `-[NSSortDescriptor reversedSortDescriptor]`
pub extern "C" fn shim_nssortdescriptor_reversed(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let key = read_key(obj);
    let ascending = unsafe { *((obj as *const u64).add(2)) } != 0;
    alloc_nssortdescriptor(key, !ascending)
}

/// `-[NSSortDescriptor description]`
pub extern "C" fn shim_nssortdescriptor_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let key = read_key(obj);
    let asc = if unsafe { *((obj as *const u64).add(2)) } != 0 {
        "ascending"
    } else {
        "descending"
    };
    let desc = format!("(key: {key}, {asc})");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
