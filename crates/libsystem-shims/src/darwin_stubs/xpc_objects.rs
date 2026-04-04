//! XPC object store — real dictionary/array/primitive storage.
//!
//! Replaces the sentinel-only stubs with a handle-based object system.
//! Each `xpc_object_t` (u64 handle) maps to an `XpcValue` in a global table.
//! Handles are allocated from a monotonic counter starting at 0x5850_0100_0000
//! to avoid collisions with the type-constant range (0x5850_43xx) and error
//! range (0x5850_E0xx).

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

// ── XPC type tag constants (must match data_symbols.rs) ────────

pub const XPC_TYPE_TAG_DICTIONARY: u64 = 0x5850_4306;
pub const XPC_TYPE_TAG_ARRAY: u64     = 0x5850_4302;
pub const XPC_TYPE_TAG_STRING: u64    = 0x5850_4308;
pub const XPC_TYPE_TAG_INT64: u64     = 0x5850_4309;
pub const XPC_TYPE_TAG_UINT64: u64    = 0x5850_430A;
pub const XPC_TYPE_TAG_BOOL: u64      = 0x5850_4303;
pub const XPC_TYPE_TAG_DATA: u64      = 0x5850_4305;
pub const XPC_TYPE_TAG_DOUBLE: u64    = 0x5850_430B; // reusing DATE slot; see note
pub const XPC_TYPE_TAG_DATE: u64      = 0x5850_430B;
pub const XPC_TYPE_TAG_UUID: u64      = 0x5850_430F;
pub const XPC_TYPE_TAG_FD: u64        = 0x5850_430C;
pub const XPC_TYPE_TAG_NULL: u64      = 0x5850_430D;
pub const XPC_TYPE_TAG_ERROR: u64     = 0x5850_4307;
pub const XPC_TYPE_TAG_CONNECTION: u64 = 0x5850_4304;
pub const XPC_TYPE_TAG_ENDPOINT: u64  = 0x5850_4310;

// ── Value types ────────────────────────────────────────────────

/// The concrete value stored for each XPC object handle.
#[derive(Clone, Debug)]
pub enum XpcValue {
    Null,
    Bool(bool),
    Int64(i64),
    Uint64(u64),
    Double(f64),
    String(CString),
    Data(Vec<u8>),
    Date(i64),           // nanoseconds since epoch
    Uuid([u8; 16]),
    Fd(i32),
    Array(Vec<u64>),     // child handles
    Dictionary(Vec<(CString, u64)>), // ordered key-value pairs (preserves insertion order)
    Error(u64),          // error sentinel id
    Connection(u64),     // connection id (see xpc_transport)
    Endpoint(u64),
}

impl XpcValue {
    /// Return the type tag pointer for `xpc_get_type()`.
    pub fn type_tag(&self) -> u64 {
        match self {
            Self::Null => XPC_TYPE_TAG_NULL,
            Self::Bool(_) => XPC_TYPE_TAG_BOOL,
            Self::Int64(_) => XPC_TYPE_TAG_INT64,
            Self::Uint64(_) => XPC_TYPE_TAG_UINT64,
            Self::Double(_) => XPC_TYPE_TAG_DOUBLE,
            Self::String(_) => XPC_TYPE_TAG_STRING,
            Self::Data(_) => XPC_TYPE_TAG_DATA,
            Self::Date(_) => XPC_TYPE_TAG_DATE,
            Self::Uuid(_) => XPC_TYPE_TAG_UUID,
            Self::Fd(_) => XPC_TYPE_TAG_FD,
            Self::Array(_) => XPC_TYPE_TAG_ARRAY,
            Self::Dictionary(_) => XPC_TYPE_TAG_DICTIONARY,
            Self::Error(_) => XPC_TYPE_TAG_ERROR,
            Self::Connection(_) => XPC_TYPE_TAG_CONNECTION,
            Self::Endpoint(_) => XPC_TYPE_TAG_ENDPOINT,
        }
    }
}

// ── Global object table ────────────────────────────────────────

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0x5850_0100_0000);

static OBJECTS: Mutex<Option<HashMap<u64, XpcValue>>> = Mutex::new(None);

pub(crate) fn with_table<R>(f: impl FnOnce(&mut HashMap<u64, XpcValue>) -> R) -> R {
    let mut guard = OBJECTS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

/// Allocate a new handle and store the value.
pub fn alloc(value: XpcValue) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    with_table(|t| { t.insert(handle, value); });
    handle
}

/// Look up a value by handle.  Returns `None` for unknown handles.
pub fn get(handle: u64) -> Option<XpcValue> {
    with_table(|t| t.get(&handle).cloned())
}

/// Remove a value (xpc_release).
pub fn release(handle: u64) {
    with_table(|t| { t.remove(&handle); });
}

/// Check if a handle is known.
pub fn is_valid(handle: u64) -> bool {
    with_table(|t| t.contains_key(&handle))
}

// ── Convenience constructors ───────────────────────────────────

pub fn create_null() -> u64 {
    alloc(XpcValue::Null)
}

pub fn create_bool(val: bool) -> u64 {
    alloc(XpcValue::Bool(val))
}

pub fn create_int64(val: i64) -> u64 {
    alloc(XpcValue::Int64(val))
}

pub fn create_uint64(val: u64) -> u64 {
    alloc(XpcValue::Uint64(val))
}

pub fn create_double(val: f64) -> u64 {
    alloc(XpcValue::Double(val))
}

pub fn create_string(cstr: &CStr) -> u64 {
    alloc(XpcValue::String(cstr.to_owned()))
}

pub fn create_string_from_ptr(ptr: u64) -> u64 {
    if ptr == 0 {
        return alloc(XpcValue::String(CString::new("").unwrap()));
    }
    let cstr = unsafe { CStr::from_ptr(ptr as *const std::ffi::c_char) };
    create_string(cstr)
}

pub fn create_data(bytes: &[u8]) -> u64 {
    alloc(XpcValue::Data(bytes.to_vec()))
}

pub fn create_data_from_ptr(ptr: u64, len: u64) -> u64 {
    if ptr == 0 || len == 0 {
        return alloc(XpcValue::Data(Vec::new()));
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    create_data(slice)
}

pub fn create_date(nanos: i64) -> u64 {
    alloc(XpcValue::Date(nanos))
}

pub fn create_uuid(bytes: &[u8; 16]) -> u64 {
    alloc(XpcValue::Uuid(*bytes))
}

pub fn create_uuid_from_ptr(ptr: u64) -> u64 {
    if ptr == 0 {
        return alloc(XpcValue::Uuid([0u8; 16]));
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 16) };
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(slice);
    alloc(XpcValue::Uuid(uuid))
}

pub fn create_fd(fd: i32) -> u64 {
    alloc(XpcValue::Fd(fd))
}

pub fn create_array_empty() -> u64 {
    alloc(XpcValue::Array(Vec::new()))
}

pub fn create_dictionary_empty() -> u64 {
    alloc(XpcValue::Dictionary(Vec::new()))
}

pub fn create_error(sentinel_id: u64) -> u64 {
    alloc(XpcValue::Error(sentinel_id))
}

// ── Dictionary operations ──────────────────────────────────────

/// Set a key-value pair in a dictionary.  Replaces any existing value for the key.
pub fn dict_set(dict: u64, key: &CStr, value: u64) {
    with_table(|t| {
        if let Some(XpcValue::Dictionary(pairs)) = t.get_mut(&dict) {
            let key_owned = key.to_owned();
            // Replace existing key if present
            if let Some(entry) = pairs.iter_mut().find(|(k, _)| k == &key_owned) {
                entry.1 = value;
            } else {
                pairs.push((key_owned, value));
            }
        }
    });
}

/// Get a value from a dictionary by key.
pub fn dict_get(dict: u64, key: &CStr) -> u64 {
    with_table(|t| {
        if let Some(XpcValue::Dictionary(pairs)) = t.get(&dict) {
            for (k, v) in pairs {
                if k.as_c_str() == key {
                    return *v;
                }
            }
        }
        0
    })
}

/// Set a string value in a dictionary.
pub fn dict_set_string(dict: u64, key_ptr: u64, value_ptr: u64) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let val_handle = create_string_from_ptr(value_ptr);
    dict_set(dict, key, val_handle);
}

/// Get a string pointer from a dictionary.
pub fn dict_get_string(dict: u64, key_ptr: u64) -> u64 {
    if key_ptr == 0 { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let handle = dict_get(dict, key);
    if handle == 0 { return 0; }
    with_table(|t| {
        if let Some(XpcValue::String(s)) = t.get(&handle) {
            s.as_ptr() as u64
        } else {
            0
        }
    })
}

/// Set an int64 in a dictionary.
pub fn dict_set_int64(dict: u64, key_ptr: u64, value: i64) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let h = create_int64(value);
    dict_set(dict, key, h);
}

/// Get an int64 from a dictionary.
pub fn dict_get_int64(dict: u64, key_ptr: u64) -> i64 {
    if key_ptr == 0 { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let handle = dict_get(dict, key);
    if handle == 0 { return 0; }
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Int64(v)) => *v,
            _ => 0,
        }
    })
}

/// Set a uint64 in a dictionary.
pub fn dict_set_uint64(dict: u64, key_ptr: u64, value: u64) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let h = create_uint64(value);
    dict_set(dict, key, h);
}

/// Get a uint64 from a dictionary.
pub fn dict_get_uint64(dict: u64, key_ptr: u64) -> u64 {
    if key_ptr == 0 { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let handle = dict_get(dict, key);
    if handle == 0 { return 0; }
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Uint64(v)) => *v,
            _ => 0,
        }
    })
}

/// Set a bool in a dictionary.
pub fn dict_set_bool(dict: u64, key_ptr: u64, value: bool) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let h = create_bool(value);
    dict_set(dict, key, h);
}

/// Get a bool from a dictionary.
pub fn dict_get_bool(dict: u64, key_ptr: u64) -> bool {
    if key_ptr == 0 { return false; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let handle = dict_get(dict, key);
    if handle == 0 { return false; }
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Bool(v)) => *v,
            _ => false,
        }
    })
}

/// Set data in a dictionary.
pub fn dict_set_data(dict: u64, key_ptr: u64, data_ptr: u64, len: u64) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let h = create_data_from_ptr(data_ptr, len);
    dict_set(dict, key, h);
}

/// Get data pointer from a dictionary (returns pointer to bytes, sets length).
pub fn dict_get_data(dict: u64, key_ptr: u64) -> u64 {
    if key_ptr == 0 { return 0; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let handle = dict_get(dict, key);
    if handle == 0 { return 0; }
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Data(d)) if !d.is_empty() => d.as_ptr() as u64,
            _ => 0,
        }
    })
}

/// Set an fd in a dictionary.
pub fn dict_set_fd(dict: u64, key_ptr: u64, fd: i32) {
    if key_ptr == 0 { return; }
    let key = unsafe { CStr::from_ptr(key_ptr as *const std::ffi::c_char) };
    let h = create_fd(fd);
    dict_set(dict, key, h);
}

/// Get count of entries in a dictionary.
pub fn dict_get_count(dict: u64) -> u64 {
    with_table(|t| {
        if let Some(XpcValue::Dictionary(pairs)) = t.get(&dict) {
            pairs.len() as u64
        } else {
            0
        }
    })
}

/// Apply a callback to each key-value pair. Returns true if all calls returned true.
///
/// `applier` is an ObjC block: invoke at offset +16, takes (block, key_cstr, value_handle).
pub fn dict_apply(dict: u64, applier: u64) -> bool {
    if applier == 0 { return true; }

    // Snapshot the pairs so we don't hold the lock during callback
    let pairs: Vec<(CString, u64)> = with_table(|t| {
        if let Some(XpcValue::Dictionary(p)) = t.get(&dict) {
            p.clone()
        } else {
            Vec::new()
        }
    });

    let invoke_ptr = unsafe { *((applier + 16) as *const u64) };
    if invoke_ptr == 0 { return true; }
    let invoke: extern "C" fn(u64, u64, u64) -> i32 = unsafe { std::mem::transmute(invoke_ptr) };

    for (key, val) in &pairs {
        let result = invoke(applier, key.as_ptr() as u64, *val);
        if result == 0 { return false; }
    }
    true
}

// ── Array operations ───────────────────────────────────────────

pub fn array_append(arr: u64, value: u64) {
    with_table(|t| {
        if let Some(XpcValue::Array(vec)) = t.get_mut(&arr) {
            vec.push(value);
        }
    });
}

pub fn array_get_count(arr: u64) -> u64 {
    with_table(|t| {
        if let Some(XpcValue::Array(vec)) = t.get(&arr) {
            vec.len() as u64
        } else {
            0
        }
    })
}

pub fn array_get_value(arr: u64, index: u64) -> u64 {
    with_table(|t| {
        if let Some(XpcValue::Array(vec)) = t.get(&arr) {
            vec.get(index as usize).copied().unwrap_or(0)
        } else {
            0
        }
    })
}

/// Apply a callback to each element.
pub fn array_apply(arr: u64, applier: u64) -> bool {
    if applier == 0 { return true; }

    let elements: Vec<u64> = with_table(|t| {
        if let Some(XpcValue::Array(vec)) = t.get(&arr) {
            vec.clone()
        } else {
            Vec::new()
        }
    });

    let invoke_ptr = unsafe { *((applier + 16) as *const u64) };
    if invoke_ptr == 0 { return true; }
    let invoke: extern "C" fn(u64, u64, u64) -> i32 = unsafe { std::mem::transmute(invoke_ptr) };

    for (i, val) in elements.iter().enumerate() {
        let result = invoke(applier, i as u64, *val);
        if result == 0 { return false; }
    }
    true
}

// ── Primitive accessors ────────────────────────────────────────

pub fn get_bool_value(handle: u64) -> bool {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Bool(v)) => *v,
            _ => false,
        }
    })
}

pub fn get_int64_value(handle: u64) -> i64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Int64(v)) => *v,
            _ => 0,
        }
    })
}

pub fn get_uint64_value(handle: u64) -> u64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Uint64(v)) => *v,
            _ => 0,
        }
    })
}

pub fn get_double_value(handle: u64) -> f64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Double(v)) => *v,
            _ => 0.0,
        }
    })
}

pub fn get_string_ptr(handle: u64) -> u64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::String(s)) => s.as_ptr() as u64,
            _ => 0,
        }
    })
}

pub fn get_string_length(handle: u64) -> u64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::String(s)) => s.to_bytes().len() as u64,
            _ => 0,
        }
    })
}

pub fn get_data_ptr(handle: u64) -> u64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Data(d)) if !d.is_empty() => d.as_ptr() as u64,
            _ => 0,
        }
    })
}

pub fn get_data_length(handle: u64) -> u64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Data(d)) => d.len() as u64,
            _ => 0,
        }
    })
}

pub fn get_date_value(handle: u64) -> i64 {
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Date(v)) => *v,
            _ => 0,
        }
    })
}

/// Get the type tag for an object handle (for `xpc_get_type`).
pub fn get_type_tag(handle: u64) -> u64 {
    with_table(|t| {
        t.get(&handle).map(|v| v.type_tag()).unwrap_or(0)
    })
}

/// Build a human-readable description string.
pub fn copy_description(handle: u64) -> u64 {
    let desc = with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Null) => "null".to_string(),
            Some(XpcValue::Bool(v)) => format!("{}", v),
            Some(XpcValue::Int64(v)) => format!("{}", v),
            Some(XpcValue::Uint64(v)) => format!("{}", v),
            Some(XpcValue::Double(v)) => format!("{}", v),
            Some(XpcValue::String(s)) => format!("\"{}\"", s.to_str().unwrap_or("?")),
            Some(XpcValue::Data(d)) => format!("<data: {} bytes>", d.len()),
            Some(XpcValue::Date(v)) => format!("<date: {}>", v),
            Some(XpcValue::Uuid(u)) => {
                format!("{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                    u[0],u[1],u[2],u[3],u[4],u[5],u[6],u[7],u[8],u[9],u[10],u[11],u[12],u[13],u[14],u[15])
            }
            Some(XpcValue::Fd(fd)) => format!("<fd: {}>", fd),
            Some(XpcValue::Array(arr)) => format!("<array: {} elements>", arr.len()),
            Some(XpcValue::Dictionary(d)) => format!("<dictionary: {} entries>", d.len()),
            Some(XpcValue::Error(_)) => "<error>".to_string(),
            Some(XpcValue::Connection(_)) => "<connection>".to_string(),
            Some(XpcValue::Endpoint(_)) => "<endpoint>".to_string(),
            None => "<invalid xpc object>".to_string(),
        }
    });
    // Leak a CString — callers are expected to free with free() but we
    // accept the small leak for correctness (matching Apple's behavior
    // where xpc_copy_description returns a malloc'd string).
    let c = CString::new(desc).unwrap_or_default();
    c.into_raw() as u64
}

/// Check equality between two XPC objects.
pub fn equal(a: u64, b: u64) -> bool {
    if a == b { return true; }
    let va = get(a);
    let vb = get(b);
    match (va, vb) {
        (Some(XpcValue::Bool(a)), Some(XpcValue::Bool(b))) => a == b,
        (Some(XpcValue::Int64(a)), Some(XpcValue::Int64(b))) => a == b,
        (Some(XpcValue::Uint64(a)), Some(XpcValue::Uint64(b))) => a == b,
        (Some(XpcValue::Double(a)), Some(XpcValue::Double(b))) => a == b,
        (Some(XpcValue::String(a)), Some(XpcValue::String(b))) => a == b,
        (Some(XpcValue::Data(a)), Some(XpcValue::Data(b))) => a == b,
        (Some(XpcValue::Null), Some(XpcValue::Null)) => true,
        _ => false,
    }
}

/// Hash an XPC object.
pub fn hash(handle: u64) -> u64 {
    // Simple hash: type tag XOR'd with value bits
    with_table(|t| {
        match t.get(&handle) {
            Some(XpcValue::Int64(v)) => *v as u64 ^ XPC_TYPE_TAG_INT64,
            Some(XpcValue::Uint64(v)) => *v ^ XPC_TYPE_TAG_UINT64,
            Some(XpcValue::Bool(v)) => *v as u64 ^ XPC_TYPE_TAG_BOOL,
            Some(XpcValue::String(s)) => {
                // djb2 hash
                let mut h: u64 = 5381;
                for b in s.to_bytes() {
                    h = h.wrapping_mul(33).wrapping_add(*b as u64);
                }
                h
            }
            _ => handle, // fallback: use handle as hash
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_read_int64() {
        let h = create_int64(42);
        assert_ne!(h, 0);
        assert_eq!(get_int64_value(h), 42);
    }

    #[test]
    fn create_and_read_string() {
        let cstr = CString::new("hello").unwrap();
        let h = create_string(&cstr);
        let ptr = get_string_ptr(h);
        assert_ne!(ptr, 0);
        let read = unsafe { CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(read.to_str().unwrap(), "hello");
        assert_eq!(get_string_length(h), 5);
    }

    #[test]
    fn create_and_read_bool() {
        let h = create_bool(true);
        assert!(get_bool_value(h));
        let h2 = create_bool(false);
        assert!(!get_bool_value(h2));
    }

    #[test]
    fn create_and_read_data() {
        let data = vec![1u8, 2, 3, 4];
        let h = create_data(&data);
        assert_eq!(get_data_length(h), 4);
        let ptr = get_data_ptr(h);
        let read = unsafe { std::slice::from_raw_parts(ptr as *const u8, 4) };
        assert_eq!(read, &[1, 2, 3, 4]);
    }

    #[test]
    fn dictionary_set_get() {
        let dict = create_dictionary_empty();
        let key = CString::new("name").unwrap();
        let val = create_string(&CString::new("test").unwrap());
        dict_set(dict, &key, val);
        assert_eq!(dict_get_count(dict), 1);
        let got = dict_get(dict, &key);
        assert_eq!(got, val);
    }

    #[test]
    fn dictionary_set_get_int64() {
        let dict = create_dictionary_empty();
        dict_set_int64(dict, b"count\0".as_ptr() as u64, 99);
        assert_eq!(dict_get_int64(dict, b"count\0".as_ptr() as u64), 99);
    }

    #[test]
    fn dictionary_set_get_string() {
        let dict = create_dictionary_empty();
        dict_set_string(dict, b"key\0".as_ptr() as u64, b"value\0".as_ptr() as u64);
        let ptr = dict_get_string(dict, b"key\0".as_ptr() as u64);
        assert_ne!(ptr, 0);
        let read = unsafe { CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(read.to_str().unwrap(), "value");
    }

    #[test]
    fn dictionary_set_get_bool() {
        let dict = create_dictionary_empty();
        dict_set_bool(dict, b"flag\0".as_ptr() as u64, true);
        assert!(dict_get_bool(dict, b"flag\0".as_ptr() as u64));
    }

    #[test]
    fn dictionary_replace_key() {
        let dict = create_dictionary_empty();
        dict_set_int64(dict, b"x\0".as_ptr() as u64, 1);
        dict_set_int64(dict, b"x\0".as_ptr() as u64, 2);
        assert_eq!(dict_get_count(dict), 1);
        assert_eq!(dict_get_int64(dict, b"x\0".as_ptr() as u64), 2);
    }

    #[test]
    fn array_ops() {
        let arr = create_array_empty();
        let v1 = create_int64(10);
        let v2 = create_int64(20);
        array_append(arr, v1);
        array_append(arr, v2);
        assert_eq!(array_get_count(arr), 2);
        assert_eq!(array_get_value(arr, 0), v1);
        assert_eq!(array_get_value(arr, 1), v2);
    }

    #[test]
    fn type_tags() {
        assert_eq!(get_type_tag(create_int64(0)), XPC_TYPE_TAG_INT64);
        assert_eq!(get_type_tag(create_string(&CString::new("").unwrap())), XPC_TYPE_TAG_STRING);
        assert_eq!(get_type_tag(create_bool(false)), XPC_TYPE_TAG_BOOL);
        assert_eq!(get_type_tag(create_dictionary_empty()), XPC_TYPE_TAG_DICTIONARY);
        assert_eq!(get_type_tag(create_array_empty()), XPC_TYPE_TAG_ARRAY);
    }

    #[test]
    fn equality() {
        let a = create_int64(42);
        let b = create_int64(42);
        let c = create_int64(99);
        assert!(equal(a, b));
        assert!(!equal(a, c));
        assert!(equal(a, a)); // same handle
    }

    #[test]
    fn null_value() {
        let h = create_null();
        assert_eq!(get_type_tag(h), XPC_TYPE_TAG_NULL);
    }

    #[test]
    fn release_removes() {
        let h = create_int64(1);
        assert!(is_valid(h));
        release(h);
        assert!(!is_valid(h));
    }
}
