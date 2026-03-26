//! NSJSONSerialization shim: JSON ↔ Foundation object conversion.
//!
//! Uses `serde_json` for parsing/generation. Foundation objects are
//! mapped to/from JSON values:
//!   - NSString ↔ JSON string
//!   - NSNumber ↔ JSON number / bool
//!   - NSArray  ↔ JSON array
//!   - NSDictionary ↔ JSON object
//!   - NSNull  ↔ JSON null

use crate::cfstring;
use crate::nsarray;
use crate::nsdictionary;
use crate::nsnull;
use crate::nsnumber;
use crate::nsobj;

/// `NSJSONReadingMutableContainers = 1`
const _MUTABLE_CONTAINERS: u64 = 1;
/// `NSJSONWritingPrettyPrinted = 1`
const PRETTY_PRINTED: u64 = 1;

// ── JSON → Foundation ──────────────────────────────────────────────

fn json_value_to_foundation(val: &serde_json::Value) -> u64 {
    match val {
        serde_json::Value::Null => nsnull::shim_nsnull_null(0, 0),
        serde_json::Value::Bool(b) => nsnumber::make_nsnumber_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                nsnumber::make_nsnumber_int(i)
            } else if let Some(f) = n.as_f64() {
                nsnumber::make_nsnumber_double(f)
            } else {
                nsnumber::make_nsnumber_int(0)
            }
        }
        serde_json::Value::String(s) => {
            let c_str = std::ffi::CString::new(s.as_str()).unwrap_or_default();
            cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<u64> = arr.iter().map(json_value_to_foundation).collect();
            nsarray::make_nsarray(&items)
        }
        serde_json::Value::Object(map) => {
            let mut keys = Vec::with_capacity(map.len());
            let mut vals = Vec::with_capacity(map.len());
            for (k, v) in map {
                let c_key = std::ffi::CString::new(k.as_str()).unwrap_or_default();
                keys.push(cfstring::shim_cf_string_create_with_cstring(0, c_key.as_ptr(), 0));
                vals.push(json_value_to_foundation(v));
            }
            nsdictionary::make_nsdictionary(&keys, &vals)
        }
    }
}

// ── Foundation → JSON ──────────────────────────────────────────────

/// Check whether an address looks like a valid heap/stack pointer.
fn is_valid_ptr(addr: u64) -> bool {
    addr > 0x1000 && addr & (1u64 << 63) == 0
}

fn foundation_to_json_value(obj: u64) -> serde_json::Value {
    if !is_valid_ptr(obj) {
        return serde_json::Value::Null;
    }

    // Safety: all reads use read_unaligned since object memory may come from
    // the loaded binary's mapped pages and is not guaranteed 8-byte aligned.
    let isa = unsafe { std::ptr::read_unaligned(obj as *const u64) };

    // NSNull
    if isa == nsobj::nsnull_class_addr() {
        return serde_json::Value::Null;
    }

    // NSString / __NSCFConstantString
    if isa == nsobj::nsstring_class_addr()
        || isa == crate::foundation::cfconstant_string_class_addr()
        || isa == nsobj::nsmutablestring_class_addr()
    {
        if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
            return serde_json::Value::String(s.to_string());
        }
    }

    // NSNumber / NSConstantIntegerNumber
    if isa == nsobj::nsnumber_class_addr() || isa == nsobj::nsconstantintegernumber_class_addr() {
        let (tag, raw) = nsnumber::read_number_pub(obj);
        return match tag {
            2 => { // TAG_BOOL
                serde_json::Value::Bool(raw != 0)
            }
            0 => { // TAG_INT
                serde_json::Value::Number(serde_json::Number::from(raw as i64))
            }
            1 => { // TAG_DOUBLE
                let val = f64::from_bits(raw);
                serde_json::Number::from_f64(val)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            _ => serde_json::Value::Null,
        };
    }

    // NSArray
    if isa == nsobj::nsarray_class_addr() || isa == nsobj::nsmutablearray_class_addr() {
        let count = unsafe { std::ptr::read_unaligned((obj + 8) as *const u64) } as usize;
        let items_ptr = unsafe { std::ptr::read_unaligned((obj + 16) as *const u64) };
        if !is_valid_ptr(items_ptr) || count > 100_000 {
            return serde_json::Value::Array(Vec::new());
        }
        let mut arr = Vec::with_capacity(count.min(10_000));
        for i in 0..count {
            let item = unsafe { std::ptr::read_unaligned((items_ptr as *const u64).add(i)) };
            arr.push(foundation_to_json_value(item));
        }
        return serde_json::Value::Array(arr);
    }

    // NSDictionary / NSConstantDictionary
    if isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
        || isa == nsobj::nsconstantdictionary_class_addr()
    {
        // Use layout-aware helpers that handle both dynamic and constant dicts
        let count = nsdictionary::helpers_pub::dict_count_pub(obj) as usize;
        let keys_ptr = nsdictionary::helpers_pub::dict_keys_pub(obj);
        let vals_ptr = nsdictionary::helpers_pub::dict_vals_pub(obj);
        if !is_valid_ptr(keys_ptr) || !is_valid_ptr(vals_ptr) || count > 100_000 {
            return serde_json::Value::Object(serde_json::Map::new());
        }
        let mut map = serde_json::Map::with_capacity(count.min(10_000));
        for i in 0..count {
            let key = unsafe { std::ptr::read_unaligned((keys_ptr as *const u64).add(i)) };
            let val = unsafe { std::ptr::read_unaligned((vals_ptr as *const u64).add(i)) };
            let key_str = cfstring::extract_cfstring_cstr(key)
                .unwrap_or("<non-string key>")
                .to_string();
            map.insert(key_str, foundation_to_json_value(val));
        }
        return serde_json::Value::Object(map);
    }

    // Fallback — describe as string
    serde_json::Value::String(format!("<unknown object {obj:#x}>"))
}

// ── Class methods (shims) ──────────────────────────────────────────

/// `+[NSJSONSerialization JSONObjectWithData:options:error:]`
pub extern "C" fn shim_json_object_with_data(
    _cls: u64,
    _cmd: u64,
    data: u64,
    _options: u64,
    error_out: u64,
) -> u64 {
    if data == 0 { return 0; }

    // Read NSData bytes
    let length = unsafe { *((data + 8) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((data + 16) as *const u64) } as *const u8;
    if bytes_ptr.is_null() || length == 0 { return 0; }

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, length) };

    match serde_json::from_slice::<serde_json::Value>(slice) {
        Ok(val) => json_value_to_foundation(&val),
        Err(e) => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror(
                    "NSCocoaErrorDomain",
                    3840, // NSPropertyListReadCorruptError
                );
                unsafe { *(error_out as *mut u64) = err; }
                log::debug!("NSJSONSerialization: parse error: {}", e);
            }
            0
        }
    }
}

/// `+[NSJSONSerialization dataWithJSONObject:options:error:]`
pub extern "C" fn shim_data_with_json_object(
    _cls: u64,
    _cmd: u64,
    obj: u64,
    options: u64,
    error_out: u64,
) -> u64 {
    if obj == 0 { return 0; }

    let val = foundation_to_json_value(obj);

    let json_bytes = if (options & PRETTY_PRINTED) != 0 {
        serde_json::to_vec_pretty(&val)
    } else {
        serde_json::to_vec(&val)
    };

    match json_bytes {
        Ok(bytes) => {
            crate::nsdata::make_nsdata(&bytes)
        }
        Err(e) => {
            if error_out != 0 {
                let err = crate::nserror::make_nserror(
                    "NSCocoaErrorDomain",
                    3840, // NSPropertyListWriteStreamError
                );
                unsafe { *(error_out as *mut u64) = err; }
                log::debug!("NSJSONSerialization: serialization error: {}", e);
            }
            0
        }
    }
}

/// `+[NSJSONSerialization isValidJSONObject:]`
pub extern "C" fn shim_is_valid_json_object(_cls: u64, _cmd: u64, obj: u64) -> u8 {
    if obj == 0 { return 0; }
    let isa = unsafe { *(obj as *const u64) };
    // Valid JSON root objects: NSArray or NSDictionary
    if isa == nsobj::nsarray_class_addr()
        || isa == nsobj::nsmutablearray_class_addr()
        || isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
    {
        1
    } else {
        0
    }
}
