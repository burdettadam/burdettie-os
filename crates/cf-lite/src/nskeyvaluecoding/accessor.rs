//! KVC accessor and ivar access logic.

use crate::nsnumber;
use super::boxing::{
    autobox_if_scalar, autounbox_if_scalar, extract_return_type, extract_setter_param_type,
    unbox_double, unbox_int,
};

/// Capitalize the first character of a key for getter/setter name construction.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper: String = c.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}

/// Search for a getter accessor on `obj` and call it if found.
pub(super) fn try_getter_accessor(obj: u64, key: &str) -> Option<u64> {
    use objc_runtime::OBJC_RUNTIME;

    let cap_key = capitalize_first(key);
    let candidates = [
        key.to_string(),
        format!("is{}", cap_key),
        format!("_{}", key),
        format!("_is{}", cap_key),
    ];

    let isa = unsafe { *(obj as *const u64) };

    let mut guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_mut()?;

    for sel_name in &candidates {
        if let Some(entry) = rt.lookup_method_entry(isa, sel_name) {
            let return_type = extract_return_type(&entry.type_encoding);
            let sel_ptr = rt.intern_selector(sel_name) as u64;
            drop(guard);
            let msg_send = objc_runtime::objc_msg_send_addr();
            let send: extern "C" fn(u64, u64) -> u64 =
                unsafe { std::mem::transmute(msg_send) };
            let raw = send(obj, sel_ptr);
            return Some(autobox_if_scalar(raw, return_type));
        }
    }

    None
}

/// Search for a setter accessor on `obj` and call it if found.
pub(super) fn try_setter_accessor(obj: u64, key: &str, value: u64) -> bool {
    use objc_runtime::OBJC_RUNTIME;

    let cap_key = capitalize_first(key);
    let candidates = [
        format!("set{}:", cap_key),
        format!("_set{}:", cap_key),
    ];

    let isa = unsafe { *(obj as *const u64) };

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else {
        return false;
    };

    for sel_name in &candidates {
        if let Some(entry) = rt.lookup_method_entry(isa, sel_name) {
            let param_type = extract_setter_param_type(&entry.type_encoding);
            let sel_ptr = rt.intern_selector(sel_name) as u64;
            drop(guard);
            let msg_send = objc_runtime::objc_msg_send_addr();
            let actual_value = autounbox_if_scalar(value, param_type);
            let send: extern "C" fn(u64, u64, u64) =
                unsafe { std::mem::transmute(msg_send) };
            send(obj, sel_ptr, actual_value);
            return true;
        }
    }

    false
}

/// Try to read an ivar directly. Returns Some(value) if found.
pub(super) fn try_ivar_read(obj: u64, key: &str) -> Option<u64> {
    use objc_runtime::OBJC_RUNTIME;

    let isa = unsafe { *(obj as *const u64) };

    let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_ref()?;
    let class = rt.get_class_by_addr(isa)?;

    let underscore_key = format!("_{}", key);
    let ivar = class
        .ivars
        .iter()
        .find(|iv| iv.name == underscore_key)
        .or_else(|| class.ivars.iter().find(|iv| iv.name == key))?;

    let offset = ivar.offset as usize;
    let type_enc = &ivar.type_encoding;

    let raw = unsafe { *((obj as usize + offset) as *const u64) };

    let result = match type_enc.as_str() {
        "@" | "#" => raw,
        "i" | "I" => nsnumber::alloc_nsnumber(0, raw as i32 as i64 as u64),
        "l" | "L" => nsnumber::alloc_nsnumber(0, raw as i32 as i64 as u64),
        "q" | "Q" => nsnumber::alloc_nsnumber(0, raw),
        "s" | "S" => nsnumber::alloc_nsnumber(0, raw as i16 as i64 as u64),
        "c" | "C" => nsnumber::alloc_nsnumber(0, raw as i8 as i64 as u64),
        "d" => nsnumber::alloc_nsnumber(1, raw),
        "f" => {
            let f = unsafe { *((obj as usize + offset) as *const f32) };
            nsnumber::alloc_nsnumber(1, (f as f64).to_bits())
        }
        "B" => nsnumber::alloc_nsnumber(2, if raw != 0 { 1 } else { 0 }),
        "^" | "*" | ":" | "?" => raw,
        _ => raw,
    };

    Some(result)
}

/// Try to write an ivar directly. Returns true if the ivar was found and written.
pub(super) fn try_ivar_write(obj: u64, key: &str, value: u64) -> bool {
    use objc_runtime::OBJC_RUNTIME;

    let isa = unsafe { *(obj as *const u64) };

    let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_ref();
    let Some(rt) = rt else {
        return false;
    };
    let Some(class) = rt.get_class_by_addr(isa) else {
        return false;
    };

    let underscore_key = format!("_{}", key);
    let ivar = class
        .ivars
        .iter()
        .find(|iv| iv.name == underscore_key)
        .or_else(|| class.ivars.iter().find(|iv| iv.name == key));
    let Some(ivar) = ivar else {
        return false;
    };

    let offset = ivar.offset as usize;
    let type_enc = ivar.type_encoding.clone();
    drop(guard);

    match type_enc.as_str() {
        "@" | "#" => unsafe {
            *((obj as usize + offset) as *mut u64) = value;
        },
        "i" | "I" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut i32) = int_val as i32 };
        }
        "l" | "L" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut i32) = int_val as i32 };
        }
        "q" | "Q" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut i64) = int_val };
        }
        "s" | "S" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut i16) = int_val as i16 };
        }
        "c" | "C" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut i8) = int_val as i8 };
        }
        "d" => {
            let dbl = unbox_double(value);
            unsafe { *((obj as usize + offset) as *mut f64) = dbl };
        }
        "f" => {
            let dbl = unbox_double(value);
            unsafe { *((obj as usize + offset) as *mut f32) = dbl as f32 };
        }
        "B" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut u8) = if int_val != 0 { 1 } else { 0 } };
        }
        _ => unsafe {
            *((obj as usize + offset) as *mut u64) = value;
        },
    }

    true
}
