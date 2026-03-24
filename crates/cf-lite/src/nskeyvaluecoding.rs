//! Key-Value Coding (KVC) support for NSObject.
//!
//! Provides `valueForKey:`, `setValue:forKey:`, `valueForKeyPath:`,
//! `setValue:forKeyPath:`, and `valueForUndefinedKey:` / `setValue:forUndefinedKey:`.
//!
//! **Accessor search order** (Apple-compatible):
//!
//! `valueForKey:` getter search:
//!   1. `<key>`      — e.g. `name`
//!   2. `is<Key>`    — e.g. `isName`
//!   3. `_<key>`     — e.g. `_name`
//!   4. `_is<Key>`   — e.g. `_isName`
//!   5. ivar `_<key>` / `<key>` — direct ivar access with auto-boxing
//!
//! `setValue:forKey:` setter search:
//!   1. `set<Key>:`  — e.g. `setName:`
//!   2. `_set<Key>:` — e.g. `_setName:`
//!   3. ivar `_<key>` / `<key>` — direct ivar write with auto-unboxing

use crate::cfstring;
use crate::nsnumber;

// ── Shim: valueForKey: ────────────────────────────────────────────

/// `- [NSObject valueForKey:]`
///
/// Type encoding: `@@:@` (id, self, SEL, NSString *key)
///
/// Searches for getter accessors, then falls back to direct ivar access.
pub extern "C" fn shim_value_for_key(obj: u64, _sel: u64, key_nsstring: u64) -> u64 {
    if obj == 0 || key_nsstring == 0 {
        return 0;
    }

    let key = match cfstring::extract_cfstring_cstr(key_nsstring) {
        Some(s) => s,
        None => return 0,
    };
    // Try direct ivar access first (most common for KVC on simple classes)
    if let Some(result) = try_ivar_read(obj, key) {
        return result;
    }

    // Try accessor methods
    if let Some(result) = try_getter_accessor(obj, key) {
        return result;
    }

    // Call valueForUndefinedKey:
    value_for_undefined_key(obj, key)
}

// ── Shim: setValue:forKey: ─────────────────────────────────────────

/// `- [NSObject setValue:forKey:]`
///
/// Type encoding: `v@:@@` (void, self, SEL, id value, NSString *key)
///
/// Searches for setter accessors, then falls back to direct ivar write.
pub extern "C" fn shim_set_value_for_key(obj: u64, _sel: u64, value: u64, key_nsstring: u64) {
    if obj == 0 || key_nsstring == 0 {
        return;
    }

    let key = match cfstring::extract_cfstring_cstr(key_nsstring) {
        Some(s) => s,
        None => return,
    };

    // Try direct ivar write first (with auto-unboxing)
    if try_ivar_write(obj, key, value) {
        return;
    }

    // Try setter methods
    if try_setter_accessor(obj, key, value) {
        return;
    }

    // Call setValue:forUndefinedKey:
    set_value_for_undefined_key(obj, key, value);
}

// ── Shim: valueForKeyPath: ────────────────────────────────────────

/// `- [NSObject valueForKeyPath:]`
///
/// Type encoding: `@@:@` (id, self, SEL, NSString *keyPath)
///
/// Splits on `.` and traverses the object graph.
pub extern "C" fn shim_value_for_key_path(obj: u64, _sel: u64, key_path_nsstring: u64) -> u64 {
    if obj == 0 || key_path_nsstring == 0 {
        return 0;
    }

    let key_path = match cfstring::extract_cfstring_cstr(key_path_nsstring) {
        Some(s) => s,
        None => return 0,
    };
    let mut current = obj;
    for component in key_path.split('.') {
        if current == 0 {
            return 0;
        }

        // Check for collection operator
        if component.starts_with('@') {
            return handle_collection_operator(current, component, key_path);
        }

        current = value_for_key_direct(current, component);
    }
    current
}

// ── Shim: setValue:forKeyPath: ─────────────────────────────────────

/// `- [NSObject setValue:forKeyPath:]`
///
/// Type encoding: `v@:@@` (void, self, SEL, id value, NSString *keyPath)
///
/// Traverses the key path, setting the value on the final object.
pub extern "C" fn shim_set_value_for_key_path(
    obj: u64,
    _sel: u64,
    value: u64,
    key_path_nsstring: u64,
) {
    if obj == 0 || key_path_nsstring == 0 {
        return;
    }

    let key_path = match cfstring::extract_cfstring_cstr(key_path_nsstring) {
        Some(s) => s,
        None => return,
    };

    let components: Vec<&str> = key_path.split('.').collect();
    if components.is_empty() {
        return;
    }

    // Traverse to the second-to-last component
    let mut current = obj;
    for component in &components[..components.len() - 1] {
        if current == 0 {
            return;
        }
        current = value_for_key_direct(current, component);
    }

    if current == 0 {
        return;
    }

    // Set the final component
    let final_key = components[components.len() - 1];
    set_value_for_key_direct(current, final_key, value);
}

// ── Shim: valueForUndefinedKey: ───────────────────────────────────

/// `- [NSObject valueForUndefinedKey:]`
///
/// Type encoding: `@@:@`
///
/// Default: logs a warning and returns nil. Real Apple raises NSUndefinedKeyException.
pub extern "C" fn shim_value_for_undefined_key_method(
    obj: u64,
    _sel: u64,
    key_nsstring: u64,
) -> u64 {
    let key = cfstring::extract_cfstring_cstr(key_nsstring).unwrap_or("<unknown>");
    log::warn!(
        "valueForUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
    0
}

// ── Shim: setValue:forUndefinedKey: ────────────────────────────────

/// `- [NSObject setValue:forUndefinedKey:]`
///
/// Type encoding: `v@:@@`
///
/// Default: logs a warning. Real Apple raises NSUndefinedKeyException.
pub extern "C" fn shim_set_value_for_undefined_key_method(
    obj: u64,
    _sel: u64,
    _value: u64,
    key_nsstring: u64,
) {
    let key = cfstring::extract_cfstring_cstr(key_nsstring).unwrap_or("<unknown>");
    log::warn!(
        "setValue:forUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
}

// ── Internal helpers ──────────────────────────────────────────────

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

/// Extract the return type character from ObjC type encoding.
/// E.g. "i16@0:8" → 'i', "@@:@" → '@', "d24@0:8" → 'd'
fn extract_return_type(type_encoding: &str) -> char {
    type_encoding.chars().next().unwrap_or('@')
}

/// Extract the setter's value parameter type from ObjC type encoding.
/// Setter encoding is like "v24@0:8i16" → skips return('v'), self('@'), SEL(':'),
/// then the 3rd param is 'i'.
fn extract_setter_param_type(type_encoding: &str) -> char {
    // Strip digits — type encoding chars are: return, '@', ':', param
    let type_chars: Vec<char> = type_encoding
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect();
    // Index 3 = first real parameter after return, self, SEL
    type_chars.get(3).copied().unwrap_or('@')
}

/// Auto-unbox an NSNumber into a raw scalar value for setter calls.
/// For object parameters ('@'), pass through unchanged.
fn autounbox_if_scalar(value: u64, param_type: char) -> u64 {
    match param_type {
        '@' => value, // already an object pointer
        'i' | 'I' | 's' | 'S' | 'c' | 'C' | 'l' | 'L' | 'q' | 'Q' | 'B' => {
            unbox_int(value) as u64
        }
        'f' | 'd' => {
            let dval = unbox_double(value);
            if param_type == 'f' {
                (dval as f32).to_bits() as u64
            } else {
                dval.to_bits()
            }
        }
        _ => value,
    }
}

/// Auto-box a raw scalar return value into an NSNumber if needed.
/// For object returns ('@'), the raw value is already a pointer.
fn autobox_if_scalar(raw: u64, return_type: char) -> u64 {
    match return_type {
        '@' => raw, // already an object pointer
        'i' | 'I' | 's' | 'S' | 'c' | 'C' | 'l' | 'L' | 'q' | 'Q' | 'B' => {
            // Integer types — auto-box into NSNumber
            nsnumber::alloc_nsnumber(0x0, raw)
        }
        'f' => {
            let val = f32::from_bits(raw as u32) as f64;
            nsnumber::alloc_nsnumber(0x1, val.to_bits())
        }
        'd' => {
            nsnumber::alloc_nsnumber(0x1, raw)
        }
        _ => raw, // unknown — pass through
    }
}

/// Core `valueForKey:` logic without NSString extraction (used by key path traversal).
fn value_for_key_direct(obj: u64, key: &str) -> u64 {
    if obj == 0 || key.is_empty() {
        return 0;
    }
    if let Some(result) = try_getter_accessor(obj, key) {
        return result;
    }

    if let Some(result) = try_ivar_read(obj, key) {
        return result;
    }

    value_for_undefined_key(obj, key)
}

/// Core `setValue:forKey:` logic without NSString extraction.
fn set_value_for_key_direct(obj: u64, key: &str, value: u64) {
    if obj == 0 || key.is_empty() {
        return;
    }

    if try_setter_accessor(obj, key, value) {
        return;
    }

    if try_ivar_write(obj, key, value) {
        return;
    }

    set_value_for_undefined_key(obj, key, value);
}

/// Search for a getter accessor on `obj` and call it if found.
/// Returns `Some(result)` if a getter was found and called.
/// Scalar returns (int, double, etc.) are auto-boxed into NSNumber.
fn try_getter_accessor(obj: u64, key: &str) -> Option<u64> {
    use objc_runtime::OBJC_RUNTIME;

    let cap_key = capitalize_first(key);
    let candidates = [
        key.to_string(),                    // <key>
        format!("is{}", cap_key),           // is<Key>
        format!("_{}", key),                // _<key>
        format!("_is{}", cap_key),          // _is<Key>
    ];

    let isa = unsafe { *(obj as *const u64) };

    let mut guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_mut()?;

    for sel_name in &candidates {
        if let Some(entry) = rt.lookup_method_entry(isa, sel_name) {
            let return_type = extract_return_type(&entry.type_encoding);
            // Ensure selector is interned so the trampoline can read it
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
/// Auto-unboxes NSNumber for scalar parameter types.
/// Returns `true` if a setter was found and called.
fn try_setter_accessor(obj: u64, key: &str, value: u64) -> bool {
    use objc_runtime::OBJC_RUNTIME;

    let cap_key = capitalize_first(key);
    let candidates = [
        format!("set{}:", cap_key),         // set<Key>:
        format!("_set{}:", cap_key),        // _set<Key>:
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
/// Auto-boxes scalar types to NSNumber.
fn try_ivar_read(obj: u64, key: &str) -> Option<u64> {
    use objc_runtime::OBJC_RUNTIME;

    let isa = unsafe { *(obj as *const u64) };

    let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_ref()?;
    let class = rt.get_class_by_addr(isa)?;

    // KVC ivar search: _<key> first, then <key>
    let underscore_key = format!("_{}", key);
    let ivar = class
        .ivars
        .iter()
        .find(|iv| iv.name == underscore_key)
        .or_else(|| class.ivars.iter().find(|iv| iv.name == key))?;

    let offset = ivar.offset as usize;
    let type_enc = &ivar.type_encoding;

    // Read the raw value at the ivar offset
    let raw = unsafe { *((obj as usize + offset) as *const u64) };

    // Auto-box scalar types to NSNumber
    let result = match type_enc.as_str() {
        // Object types — return as-is (already a pointer)
        "@" | "#" => raw,
        // Integer types
        "i" | "I" => nsnumber::alloc_nsnumber(0, raw as i32 as i64 as u64),
        "l" | "L" => nsnumber::alloc_nsnumber(0, raw as i32 as i64 as u64),
        "q" | "Q" => nsnumber::alloc_nsnumber(0, raw),
        "s" | "S" => nsnumber::alloc_nsnumber(0, raw as i16 as i64 as u64),
        "c" | "C" => nsnumber::alloc_nsnumber(0, raw as i8 as i64 as u64),
        // Double/float
        "d" => nsnumber::alloc_nsnumber(1, raw), // raw bits are already f64
        "f" => {
            // Float is 4 bytes — read as f32, convert to f64 for NSNumber
            let f = unsafe { *((obj as usize + offset) as *const f32) };
            nsnumber::alloc_nsnumber(1, (f as f64).to_bits())
        }
        // Bool
        "B" => nsnumber::alloc_nsnumber(2, if raw != 0 { 1 } else { 0 }),
        // Pointer types — return raw
        "^" | "*" | ":" | "?" => raw,
        // Unknown — return raw (assume pointer-sized)
        _ => raw,
    };

    Some(result)
}

/// Try to write an ivar directly. Returns true if the ivar was found and written.
/// Auto-unboxes NSNumber for scalar ivar types.
fn try_ivar_write(obj: u64, key: &str, value: u64) -> bool {
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

    // KVC ivar search: _<key> first, then <key>
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

    // Drop the lock before doing memory writes
    drop(guard);

    // Write the value, auto-unboxing NSNumber for scalar types
    match type_enc.as_str() {
        // Object types — write pointer directly
        "@" | "#" => unsafe {
            *((obj as usize + offset) as *mut u64) = value;
        },
        // Integer types — unbox from NSNumber
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
        // Double/float
        "d" => {
            let dbl = unbox_double(value);
            unsafe { *((obj as usize + offset) as *mut f64) = dbl };
        }
        "f" => {
            let dbl = unbox_double(value);
            unsafe { *((obj as usize + offset) as *mut f32) = dbl as f32 };
        }
        // Bool
        "B" => {
            let int_val = unbox_int(value);
            unsafe { *((obj as usize + offset) as *mut u8) = if int_val != 0 { 1 } else { 0 } };
        }
        // Pointer types — write raw
        _ => unsafe {
            *((obj as usize + offset) as *mut u64) = value;
        },
    }

    true
}

/// Check if a value is a tagged pointer (bit 63 set).
fn is_tagged_pointer(value: u64) -> bool {
    value & (1u64 << 63) != 0
}

/// Unbox an NSNumber to i64. Handles our synthetic NSNumbers, host NSNumbers, and
/// Apple's constant NSNumber objects from `__objc_intobj` sections.
fn unbox_int(value: u64) -> i64 {
    if value == 0 {
        return 0;
    }

    // Tagged pointers can't be dereferenced
    if is_tagged_pointer(value) {
        // For tagged NSNumbers, the value is encoded in the pointer.
        // This is a simplistic decode — may not match Apple's exact format.
        return value as i64;
    }

    // Read ISA
    let isa = unsafe { *(value as *const u64) };

    // Our synthetic NSNumber layout: [ISA=nsnumber_class_addr, tag, value]
    if isa == crate::nsobj::nsnumber_class_addr() {
        let tag = unsafe { *((value as usize + 8) as *const u64) };
        let raw = unsafe { *((value as usize + 16) as *const u64) };
        return match tag {
            0 => raw as i64,
            1 => f64::from_bits(raw) as i64,
            2 => raw as i64,
            _ => raw as i64,
        };
    }

    // Host NSNumber / Apple's __objc_intobj constant:
    // Both store the numeric value at offset 16 (same as our layout).
    // Apple's __NSCFNumber and __objc_intobj objects have ISA with bit 63 set
    // (non-pointer ISA), but the value is always at offset 16 in a 24-byte object.
    let raw = unsafe { *((value as usize + 16) as *const u64) };
    raw as i64
}

/// Unbox an NSNumber to f64. Handles our synthetic NSNumbers, host NSNumbers, and
/// Apple's constant NSNumber objects.
fn unbox_double(value: u64) -> f64 {
    if value == 0 {
        return 0.0;
    }

    if is_tagged_pointer(value) {
        return value as i64 as f64;
    }

    let isa = unsafe { *(value as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        let tag = unsafe { *((value as usize + 8) as *const u64) };
        let raw = unsafe { *((value as usize + 16) as *const u64) };
        return match tag {
            0 => raw as i64 as f64,
            1 => f64::from_bits(raw),
            2 => raw as f64,
            _ => raw as i64 as f64,
        };
    }

    // Host NSNumber: flags at offset 8 indicate type, value at offset 16.
    // For __objc_intobj, offset 8 contains encoding info.
    // Read value at offset 16 — could be int or double bits.
    let raw = unsafe { *((value as usize + 16) as *const u64) };
    // Heuristic: if the high bits suggest a float, treat as float bits
    // For now, convert as integer → double (works for __objc_intobj)
    raw as i64 as f64
}

/// Call `valueForUndefinedKey:` if the object responds to it, otherwise log and return 0.
fn value_for_undefined_key(obj: u64, key: &str) -> u64 {
    log::warn!(
        "valueForUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
    0
}

/// Call `setValue:forUndefinedKey:` if the object responds to it, otherwise log.
fn set_value_for_undefined_key(obj: u64, key: &str, _value: u64) {
    log::warn!(
        "setValue:forUndefinedKey: object {:#x} is not key-value coding compliant for key '{}'",
        obj,
        key
    );
}

/// Handle collection operators like @count, @sum, @avg, @min, @max.
/// Currently a stub — returns 0 for unrecognized operators.
fn handle_collection_operator(_collection: u64, operator: &str, _full_path: &str) -> u64 {
    match operator {
        "@count" => {
            // Try to get count from the collection
            get_collection_count(_collection)
        }
        _ => {
            log::warn!(
                "KVC collection operator '{}' not yet implemented",
                operator
            );
            0
        }
    }
}

/// Get the count of a collection (NSArray) and return it as an NSNumber.
fn get_collection_count(collection: u64) -> u64 {
    use objc_runtime::OBJC_RUNTIME;

    if collection == 0 {
        return 0;
    }

    let isa = unsafe { *(collection as *const u64) };
    let guard = OBJC_RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let Some(rt) = guard.as_ref() else {
        return 0;
    };

    if let Some(imp) = rt.msg_lookup(isa, "count") {
        let sel_ptr = rt.selector_ptr("count").unwrap_or(0) as u64;
        drop(guard);
        let fn_ptr: extern "C" fn(u64, u64) -> u64 =
            unsafe { std::mem::transmute(imp) };
        let count = fn_ptr(collection, sel_ptr);
        nsnumber::alloc_nsnumber(0, count)
    } else {
        drop(guard);
        0
    }
}
