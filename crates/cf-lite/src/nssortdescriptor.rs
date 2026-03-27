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

use std::ffi::c_char;

use crate::cfstring;
use crate::nsobj;

const NSSORTDESCRIPTOR_SIZE: usize = 24;

/// Resolve the ISA for an object, handling tagged pointers via the runtime's
/// tagged class table and applying the ARM64 ISA mask for heap objects.
fn resolve_isa(rt: &objc_runtime::ObjcRuntime, obj: u64) -> Option<u64> {
    if obj == 0 {
        return None;
    }
    if obj & (1u64 << 63) != 0 {
        rt.tagged_class_for_pointer(obj)
    } else if obj % 8 == 0 {
        Some(unsafe { *(obj as *const u64) } & 0x007ffffffffffff8)
    } else {
        None
    }
}

fn alloc_nssortdescriptor(key: &str, ascending: bool) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSSORTDESCRIPTOR_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let c_key = std::ffi::CString::new(key).unwrap_or_default();
    let key_ptr = unsafe {
        let len = c_key.as_bytes_with_nul().len();
        let buf = crate::pal_alloc().malloc(len);
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
    let key_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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
    let key_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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

    // ── 1. Resolve property values via key getter ──────────────────
    let (val_a, val_b) = {
        let guard = objc_runtime::OBJC_RUNTIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = guard.as_ref() {
            let va = if key == "self" || key.is_empty() {
                a
            } else {
                resolve_isa(rt, a)
                    .and_then(|isa| rt.msg_lookup(isa, key))
                    .map(|imp| {
                        let f: extern "C" fn(u64, u64) -> u64 =
                            unsafe { std::mem::transmute(imp) };
                        f(a, 0)
                    })
                    .unwrap_or(a)
            };
            let vb = if key == "self" || key.is_empty() {
                b
            } else {
                resolve_isa(rt, b)
                    .and_then(|isa| rt.msg_lookup(isa, key))
                    .map(|imp| {
                        let f: extern "C" fn(u64, u64) -> u64 =
                            unsafe { std::mem::transmute(imp) };
                        f(b, 0)
                    })
                    .unwrap_or(b)
            };
            (va, vb)
        } else {
            (a, b)
        }
    };

    // ── 2. Send `compare:` to val_a ────────────────────────────────
    let cmp = {
        let guard = objc_runtime::OBJC_RUNTIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(rt) = guard.as_ref() {
            resolve_isa(rt, val_a)
                .and_then(|isa| rt.msg_lookup(isa, "compare:"))
                .map(|imp| {
                    let f: extern "C" fn(u64, u64, u64) -> i64 =
                        unsafe { std::mem::transmute(imp) };
                    f(val_a, 0, val_b)
                })
                .unwrap_or(0)
        } else {
            0
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
