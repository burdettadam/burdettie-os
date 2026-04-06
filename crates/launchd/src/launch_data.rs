//! `launch_data_t` — the opaque data container used by Apple's launch API.
//!
//! Mirrors the exact C API from `<launch.h>`:
//! - Type-tagged variant (dictionary, array, fd, integer, real, bool, string, opaque, errno, machport)
//! - Dictionary/array operations with interior mutability
//! - Deep copy, type query, free
//!
//! The C API uses `launch_data_t` as an opaque `*mut _launch_data`.
//! We represent this as a `Box<LaunchData>` leaked to a raw pointer.

use std::collections::HashMap;
use std::ffi::{CStr, CString};

/// The type tag for launch data values, matching Apple's `launch_data_type_t`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDataType {
    Dictionary = 1,
    Array = 2,
    Fd = 3,
    Integer = 4,
    Real = 5,
    Bool = 6,
    String = 7,
    Opaque = 8,
    Errno = 9,
    MachPort = 10,
}

impl LaunchDataType {
    /// Convert from raw u32 (as used by C callers).
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            1 => Some(Self::Dictionary),
            2 => Some(Self::Array),
            3 => Some(Self::Fd),
            4 => Some(Self::Integer),
            5 => Some(Self::Real),
            6 => Some(Self::Bool),
            7 => Some(Self::String),
            8 => Some(Self::Opaque),
            9 => Some(Self::Errno),
            10 => Some(Self::MachPort),
            _ => None,
        }
    }
}

/// The inner value of a `launch_data_t`.
#[derive(Debug, Clone)]
pub enum LaunchData {
    Dictionary(HashMap<String, Box<LaunchData>>),
    Array(Vec<Box<LaunchData>>),
    Fd(i32),
    Integer(i64),
    Real(f64),
    Bool(bool),
    String(CString),
    Opaque(Vec<u8>),
    Errno(i32),
    MachPort(u32),
}

impl LaunchData {
    /// Get the type tag.
    pub fn get_type(&self) -> LaunchDataType {
        match self {
            Self::Dictionary(_) => LaunchDataType::Dictionary,
            Self::Array(_) => LaunchDataType::Array,
            Self::Fd(_) => LaunchDataType::Fd,
            Self::Integer(_) => LaunchDataType::Integer,
            Self::Real(_) => LaunchDataType::Real,
            Self::Bool(_) => LaunchDataType::Bool,
            Self::String(_) => LaunchDataType::String,
            Self::Opaque(_) => LaunchDataType::Opaque,
            Self::Errno(_) => LaunchDataType::Errno,
            Self::MachPort(_) => LaunchDataType::MachPort,
        }
    }
}

// ── Opaque handle type ──────────────────────────────────────────────

/// Opaque handle matching Apple's `typedef struct _launch_data *launch_data_t`.
///
/// This is a raw pointer to a heap-allocated `LaunchData` obtained via
/// `Box::into_raw`.  The caller is responsible for calling
/// `launch_data_free` when done (like C does).
pub type LaunchDataT = *mut LaunchData;

/// Sentinel null handle.
pub const LAUNCH_DATA_NULL: LaunchDataT = std::ptr::null_mut();

// ── Helpers ─────────────────────────────────────────────────────────

/// Wrap a `LaunchData` into a leaked raw pointer suitable for return
/// to C callers.
fn into_raw(data: LaunchData) -> LaunchDataT {
    Box::into_raw(Box::new(data))
}

/// Borrow the `LaunchData` behind a handle, returning `None` if null.
///
/// # Safety
/// Caller must ensure `handle` points to a valid `LaunchData` (or is null).
unsafe fn borrow(handle: LaunchDataT) -> Option<&'static LaunchData> {
    handle.as_ref()
}

/// Mutably borrow the `LaunchData` behind a handle.
///
/// # Safety
/// Caller must ensure `handle` points to a valid `LaunchData` (or is null)
/// and that no other references exist.
unsafe fn borrow_mut(handle: LaunchDataT) -> Option<&'static mut LaunchData> {
    handle.as_mut()
}

// ═══════════════════════════════════════════════════════════════════
//  Public C API — exact match to <launch.h>
// ═══════════════════════════════════════════════════════════════════

/// `launch_data_alloc(type)` — allocate a new launch data with default value.
pub fn alloc(ty: u32) -> LaunchDataT {
    let Some(data_type) = LaunchDataType::from_raw(ty) else {
        return LAUNCH_DATA_NULL;
    };
    let data = match data_type {
        LaunchDataType::Dictionary => LaunchData::Dictionary(HashMap::new()),
        LaunchDataType::Array => LaunchData::Array(Vec::new()),
        LaunchDataType::Fd => LaunchData::Fd(-1),
        LaunchDataType::Integer => LaunchData::Integer(0),
        LaunchDataType::Real => LaunchData::Real(0.0),
        LaunchDataType::Bool => LaunchData::Bool(false),
        LaunchDataType::String => LaunchData::String(CString::default()),
        LaunchDataType::Opaque => LaunchData::Opaque(Vec::new()),
        LaunchDataType::Errno => LaunchData::Errno(0),
        LaunchDataType::MachPort => LaunchData::MachPort(0),
    };
    into_raw(data)
}

/// `launch_data_copy(d)` — deep copy.
pub fn copy(handle: LaunchDataT) -> LaunchDataT {
    let data = unsafe { borrow(handle) };
    match data {
        Some(d) => into_raw(d.clone()),
        None => LAUNCH_DATA_NULL,
    }
}

/// `launch_data_get_type(d)` — return the type tag.
pub fn get_type(handle: LaunchDataT) -> u32 {
    let data = unsafe { borrow(handle) };
    match data {
        Some(d) => d.get_type() as u32,
        None => 0,
    }
}

/// `launch_data_free(d)` — free a handle.
///
/// # Safety
/// The handle must have been returned by one of the `alloc`/`new_*`/`copy`
/// functions and must not have been freed already.
pub fn free(handle: LaunchDataT) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ── Dictionary operations ───────────────────────────────────────────

/// `launch_data_dict_insert(dict, value, key)` → bool.
///
/// Inserts `value` into `dict` under `key`. On success the dict owns
/// the value (caller must not free it separately). Returns false if
/// `dict` is not a dictionary.
pub fn dict_insert(dict: LaunchDataT, value: LaunchDataT, key: *const libc::c_char) -> bool {
    if dict.is_null() || key.is_null() || value.is_null() {
        return false;
    }
    let key_str = unsafe { CStr::from_ptr(key) };
    let Ok(key_string) = key_str.to_str() else { return false };
    let val = unsafe { Box::from_raw(value) };
    match unsafe { borrow_mut(dict) } {
        Some(LaunchData::Dictionary(map)) => {
            map.insert(key_string.to_string(), val);
            true
        }
        _ => {
            // Caller still owns value — leak it back
            let _ = Box::into_raw(val);
            false
        }
    }
}

/// `launch_data_dict_lookup(dict, key)` → handle (borrowed, not owned by caller).
pub fn dict_lookup(dict: LaunchDataT, key: *const libc::c_char) -> LaunchDataT {
    if dict.is_null() || key.is_null() {
        return LAUNCH_DATA_NULL;
    }
    let key_str = unsafe { CStr::from_ptr(key) };
    let Ok(key_string) = key_str.to_str() else { return LAUNCH_DATA_NULL };
    match unsafe { borrow_mut(dict) } {
        Some(LaunchData::Dictionary(map)) => match map.get_mut(key_string) {
            Some(v) => v.as_mut() as *mut LaunchData,
            None => LAUNCH_DATA_NULL,
        },
        _ => LAUNCH_DATA_NULL,
    }
}

/// `launch_data_dict_remove(dict, key)` → bool.
pub fn dict_remove(dict: LaunchDataT, key: *const libc::c_char) -> bool {
    if dict.is_null() || key.is_null() {
        return false;
    }
    let key_str = unsafe { CStr::from_ptr(key) };
    let Ok(key_string) = key_str.to_str() else { return false };
    match unsafe { borrow_mut(dict) } {
        Some(LaunchData::Dictionary(map)) => map.remove(key_string).is_some(),
        _ => false,
    }
}

/// Callback type for `launch_data_dict_iterate`.
pub type DictIterateCallback = unsafe extern "C" fn(LaunchDataT, *const libc::c_char, *mut libc::c_void);

/// `launch_data_dict_iterate(dict, callback, context)`.
///
/// Calls `callback(value, key, context)` for each entry.
pub fn dict_iterate(dict: LaunchDataT, callback: DictIterateCallback, context: *mut libc::c_void) {
    let Some(LaunchData::Dictionary(map)) = (unsafe { borrow_mut(dict) }) else {
        return;
    };
    // Collect keys to avoid borrow issues during iteration
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in &keys {
        if let Some(value) = map.get_mut(key) {
            let c_key = CString::new(key.as_str()).unwrap_or_default();
            unsafe {
                callback(value.as_mut() as *mut LaunchData, c_key.as_ptr(), context);
            }
        }
    }
}

/// `launch_data_dict_get_count(dict)` → number of entries.
pub fn dict_get_count(dict: LaunchDataT) -> usize {
    match unsafe { borrow(dict) } {
        Some(LaunchData::Dictionary(map)) => map.len(),
        _ => 0,
    }
}

// ── Array operations ────────────────────────────────────────────────

/// `launch_data_array_set_index(array, value, index)` → bool.
///
/// Sets or extends the array. If index > len, the gap is filled with
/// empty dictionaries (matching Apple's plist array semantics).
pub fn array_set_index(array: LaunchDataT, value: LaunchDataT, index: usize) -> bool {
    if array.is_null() || value.is_null() {
        return false;
    }
    let val = unsafe { Box::from_raw(value) };
    match unsafe { borrow_mut(array) } {
        Some(LaunchData::Array(vec)) => {
            while vec.len() <= index {
                vec.push(Box::new(LaunchData::Errno(0)));
            }
            vec[index] = val;
            true
        }
        _ => {
            let _ = Box::into_raw(val);
            false
        }
    }
}

/// `launch_data_array_get_index(array, index)` → handle (borrowed).
pub fn array_get_index(array: LaunchDataT, index: usize) -> LaunchDataT {
    match unsafe { borrow_mut(array) } {
        Some(LaunchData::Array(vec)) => match vec.get_mut(index) {
            Some(v) => v.as_mut() as *mut LaunchData,
            None => LAUNCH_DATA_NULL,
        },
        _ => LAUNCH_DATA_NULL,
    }
}

/// `launch_data_array_get_count(array)` → element count.
pub fn array_get_count(array: LaunchDataT) -> usize {
    match unsafe { borrow(array) } {
        Some(LaunchData::Array(vec)) => vec.len(),
        _ => 0,
    }
}

// ── Value constructors ──────────────────────────────────────────────

/// `launch_data_new_fd(fd)` → handle.
pub fn new_fd(fd: i32) -> LaunchDataT {
    into_raw(LaunchData::Fd(fd))
}

/// `launch_data_new_machport(port)` → handle.
pub fn new_machport(port: u32) -> LaunchDataT {
    into_raw(LaunchData::MachPort(port))
}

/// `launch_data_new_integer(val)` → handle.
pub fn new_integer(val: i64) -> LaunchDataT {
    into_raw(LaunchData::Integer(val))
}

/// `launch_data_new_bool(val)` → handle.
pub fn new_bool(val: bool) -> LaunchDataT {
    into_raw(LaunchData::Bool(val))
}

/// `launch_data_new_real(val)` → handle.
pub fn new_real(val: f64) -> LaunchDataT {
    into_raw(LaunchData::Real(val))
}

/// `launch_data_new_string(s)` → handle.
pub fn new_string(s: *const libc::c_char) -> LaunchDataT {
    if s.is_null() {
        return LAUNCH_DATA_NULL;
    }
    let cs = unsafe { CStr::from_ptr(s) };
    into_raw(LaunchData::String(cs.to_owned()))
}

/// `launch_data_new_opaque(data, len)` → handle.
pub fn new_opaque(data: *const libc::c_void, len: usize) -> LaunchDataT {
    if data.is_null() && len > 0 {
        return LAUNCH_DATA_NULL;
    }
    let bytes = if len > 0 {
        unsafe { std::slice::from_raw_parts(data as *const u8, len) }.to_vec()
    } else {
        Vec::new()
    };
    into_raw(LaunchData::Opaque(bytes))
}

// ── Value setters ───────────────────────────────────────────────────

/// `launch_data_set_fd(d, fd)` → bool.
pub fn set_fd(handle: LaunchDataT, fd: i32) -> bool {
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::Fd(_)) => { *d = LaunchData::Fd(fd); true }
        _ => false,
    }
}

/// `launch_data_set_machport(d, port)` → bool.
pub fn set_machport(handle: LaunchDataT, port: u32) -> bool {
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::MachPort(_)) => { *d = LaunchData::MachPort(port); true }
        _ => false,
    }
}

/// `launch_data_set_integer(d, val)` → bool.
pub fn set_integer(handle: LaunchDataT, val: i64) -> bool {
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::Integer(_)) => { *d = LaunchData::Integer(val); true }
        _ => false,
    }
}

/// `launch_data_set_bool(d, val)` → bool.
pub fn set_bool(handle: LaunchDataT, val: bool) -> bool {
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::Bool(_)) => { *d = LaunchData::Bool(val); true }
        _ => false,
    }
}

/// `launch_data_set_real(d, val)` → bool.
pub fn set_real(handle: LaunchDataT, val: f64) -> bool {
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::Real(_)) => { *d = LaunchData::Real(val); true }
        _ => false,
    }
}

/// `launch_data_set_string(d, s)` → bool.
pub fn set_string(handle: LaunchDataT, s: *const libc::c_char) -> bool {
    if s.is_null() {
        return false;
    }
    let cs = unsafe { CStr::from_ptr(s) };
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::String(_)) => {
            *d = LaunchData::String(cs.to_owned());
            true
        }
        _ => false,
    }
}

/// `launch_data_set_opaque(d, data, len)` → bool.
pub fn set_opaque(handle: LaunchDataT, data: *const libc::c_void, len: usize) -> bool {
    if data.is_null() && len > 0 {
        return false;
    }
    let bytes = if len > 0 {
        unsafe { std::slice::from_raw_parts(data as *const u8, len) }.to_vec()
    } else {
        Vec::new()
    };
    match unsafe { borrow_mut(handle) } {
        Some(d @ LaunchData::Opaque(_)) => {
            *d = LaunchData::Opaque(bytes);
            true
        }
        _ => false,
    }
}

// ── Value getters ───────────────────────────────────────────────────

/// `launch_data_get_fd(d)` → fd value or -1.
pub fn get_fd(handle: LaunchDataT) -> i32 {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Fd(fd)) => *fd,
        _ => -1,
    }
}

/// `launch_data_get_machport(d)` → port or 0.
pub fn get_machport(handle: LaunchDataT) -> u32 {
    match unsafe { borrow(handle) } {
        Some(LaunchData::MachPort(port)) => *port,
        _ => 0,
    }
}

/// `launch_data_get_integer(d)` → integer value or 0.
pub fn get_integer(handle: LaunchDataT) -> i64 {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Integer(val)) => *val,
        _ => 0,
    }
}

/// `launch_data_get_bool(d)` → bool or false.
pub fn get_bool(handle: LaunchDataT) -> bool {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Bool(val)) => *val,
        _ => false,
    }
}

/// `launch_data_get_real(d)` → double or 0.0.
pub fn get_real(handle: LaunchDataT) -> f64 {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Real(val)) => *val,
        _ => 0.0,
    }
}

/// `launch_data_get_string(d)` → `const char *` pointer or null.
pub fn get_string(handle: LaunchDataT) -> *const libc::c_char {
    match unsafe { borrow(handle) } {
        Some(LaunchData::String(cs)) => cs.as_ptr(),
        _ => std::ptr::null(),
    }
}

/// `launch_data_get_opaque(d)` → pointer to data or null.
pub fn get_opaque(handle: LaunchDataT) -> *const libc::c_void {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Opaque(bytes)) => bytes.as_ptr() as *const libc::c_void,
        _ => std::ptr::null(),
    }
}

/// `launch_data_get_opaque_size(d)` → byte count.
pub fn get_opaque_size(handle: LaunchDataT) -> usize {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Opaque(bytes)) => bytes.len(),
        _ => 0,
    }
}

/// `launch_data_get_errno(d)` → errno value or 0.
pub fn get_errno(handle: LaunchDataT) -> i32 {
    match unsafe { borrow(handle) } {
        Some(LaunchData::Errno(val)) => *val,
        _ => 0,
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Rust-native helpers (not part of C API)
// ═══════════════════════════════════════════════════════════════════

/// Create a dictionary from Rust key-value pairs.
pub fn dict_from_pairs(pairs: &[(&str, LaunchDataT)]) -> LaunchDataT {
    let dict = alloc(LaunchDataType::Dictionary as u32);
    for (key, value) in pairs {
        let c_key = CString::new(*key).unwrap_or_default();
        dict_insert(dict, *value, c_key.as_ptr());
    }
    dict
}

/// Read a string value from a dictionary key. Returns `None` if missing/wrong type.
pub fn dict_get_string_value(dict: LaunchDataT, key: &str) -> Option<String> {
    let c_key = CString::new(key).ok()?;
    let val = dict_lookup(dict, c_key.as_ptr());
    if val.is_null() {
        return None;
    }
    let ptr = get_string(val);
    if ptr.is_null() {
        return None;
    }
    let cs = unsafe { CStr::from_ptr(ptr) };
    cs.to_str().ok().map(|s| s.to_string())
}

/// Read an integer value from a dictionary key.
pub fn dict_get_integer_value(dict: LaunchDataT, key: &str) -> Option<i64> {
    let c_key = CString::new(key).ok()?;
    let val = dict_lookup(dict, c_key.as_ptr());
    if val.is_null() {
        return None;
    }
    match unsafe { borrow(val) } {
        Some(LaunchData::Integer(v)) => Some(*v),
        _ => None,
    }
}

/// Read a bool value from a dictionary key.
pub fn dict_get_bool_value(dict: LaunchDataT, key: &str) -> Option<bool> {
    let c_key = CString::new(key).ok()?;
    let val = dict_lookup(dict, c_key.as_ptr());
    if val.is_null() {
        return None;
    }
    match unsafe { borrow(val) } {
        Some(LaunchData::Bool(v)) => Some(*v),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_dictionary() {
        let d = alloc(LaunchDataType::Dictionary as u32);
        assert!(!d.is_null());
        assert_eq!(get_type(d), LaunchDataType::Dictionary as u32);
        assert_eq!(dict_get_count(d), 0);
        free(d);
    }

    #[test]
    fn alloc_array() {
        let a = alloc(LaunchDataType::Array as u32);
        assert!(!a.is_null());
        assert_eq!(get_type(a), LaunchDataType::Array as u32);
        assert_eq!(array_get_count(a), 0);
        free(a);
    }

    #[test]
    fn dict_insert_and_lookup() {
        let dict = alloc(LaunchDataType::Dictionary as u32);
        let val = new_integer(42);
        let key = c"foo";
        assert!(dict_insert(dict, val, key.as_ptr()));
        assert_eq!(dict_get_count(dict), 1);

        let found = dict_lookup(dict, key.as_ptr());
        assert!(!found.is_null());
        assert_eq!(get_integer(found), 42);
        free(dict);
    }

    #[test]
    fn dict_remove_entry() {
        let dict = alloc(LaunchDataType::Dictionary as u32);
        let val = new_string(c"hello".as_ptr());
        assert!(dict_insert(dict, val, c"key".as_ptr()));
        assert_eq!(dict_get_count(dict), 1);
        assert!(dict_remove(dict, c"key".as_ptr()));
        assert_eq!(dict_get_count(dict), 0);
        assert!(!dict_remove(dict, c"missing".as_ptr()));
        free(dict);
    }

    #[test]
    fn dict_iterate_callback() {
        let dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(dict, new_integer(1), c"a".as_ptr());
        dict_insert(dict, new_integer(2), c"b".as_ptr());

        static mut COUNT: usize = 0;
        unsafe extern "C" fn cb(_val: LaunchDataT, _key: *const libc::c_char, _ctx: *mut libc::c_void) {
            unsafe { COUNT += 1; }
        }
        unsafe { COUNT = 0; }
        dict_iterate(dict, cb, std::ptr::null_mut());
        assert_eq!(unsafe { COUNT }, 2);
        free(dict);
    }

    #[test]
    fn array_set_and_get() {
        let arr = alloc(LaunchDataType::Array as u32);
        let val = new_bool(true);
        assert!(array_set_index(arr, val, 0));
        assert_eq!(array_get_count(arr), 1);
        let found = array_get_index(arr, 0);
        assert!(get_bool(found));
        free(arr);
    }

    #[test]
    fn array_sparse_set() {
        let arr = alloc(LaunchDataType::Array as u32);
        let val = new_integer(99);
        assert!(array_set_index(arr, val, 5));
        assert_eq!(array_get_count(arr), 6);
        assert_eq!(get_integer(array_get_index(arr, 5)), 99);
        free(arr);
    }

    #[test]
    fn value_constructors_and_getters() {
        let fd = new_fd(7);
        assert_eq!(get_fd(fd), 7);
        free(fd);

        let mp = new_machport(0x1234);
        assert_eq!(get_machport(mp), 0x1234);
        free(mp);

        let i = new_integer(-100);
        assert_eq!(get_integer(i), -100);
        free(i);

        let b = new_bool(true);
        assert!(get_bool(b));
        free(b);

        let r = new_real(3.14);
        assert!((get_real(r) - 3.14).abs() < f64::EPSILON);
        free(r);

        let s = new_string(c"test".as_ptr());
        assert_eq!(unsafe { CStr::from_ptr(get_string(s)) }.to_str().unwrap_or(""), "test");
        free(s);
    }

    #[test]
    fn opaque_roundtrip() {
        let data = b"hello world";
        let o = new_opaque(data.as_ptr() as *const libc::c_void, data.len());
        assert_eq!(get_opaque_size(o), 11);
        let ptr = get_opaque(o) as *const u8;
        assert!(!ptr.is_null());
        let slice = unsafe { std::slice::from_raw_parts(ptr, 11) };
        assert_eq!(slice, b"hello world");
        free(o);
    }

    #[test]
    fn deep_copy() {
        let dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(dict, new_integer(42), c"num".as_ptr());
        dict_insert(dict, new_string(c"hi".as_ptr()), c"str".as_ptr());

        let dict2 = copy(dict);
        assert!(!dict2.is_null());
        assert_eq!(dict_get_count(dict2), 2);
        assert_eq!(get_integer(dict_lookup(dict2, c"num".as_ptr())), 42);

        // Mutate original, copy unaffected
        dict_remove(dict, c"num".as_ptr());
        assert_eq!(dict_get_count(dict), 1);
        assert_eq!(dict_get_count(dict2), 2);

        free(dict);
        free(dict2);
    }

    #[test]
    fn setters() {
        let d = alloc(LaunchDataType::Fd as u32);
        assert!(set_fd(d, 42));
        assert_eq!(get_fd(d), 42);
        free(d);

        let d = alloc(LaunchDataType::Integer as u32);
        assert!(set_integer(d, 999));
        assert_eq!(get_integer(d), 999);
        free(d);

        let d = alloc(LaunchDataType::Bool as u32);
        assert!(set_bool(d, true));
        assert!(get_bool(d));
        free(d);

        let d = alloc(LaunchDataType::Real as u32);
        assert!(set_real(d, 2.5));
        assert!((get_real(d) - 2.5).abs() < f64::EPSILON);
        free(d);

        let d = alloc(LaunchDataType::String as u32);
        assert!(set_string(d, c"updated".as_ptr()));
        assert_eq!(
            unsafe { CStr::from_ptr(get_string(d)) }.to_str().unwrap_or(""),
            "updated"
        );
        free(d);
    }

    #[test]
    fn errno_type() {
        let d = alloc(LaunchDataType::Errno as u32);
        assert_eq!(get_errno(d), 0);
        free(d);
    }

    #[test]
    fn null_handle_safety() {
        assert_eq!(get_type(LAUNCH_DATA_NULL), 0);
        assert_eq!(get_integer(LAUNCH_DATA_NULL), 0);
        assert_eq!(get_string(LAUNCH_DATA_NULL), std::ptr::null());
        assert!(copy(LAUNCH_DATA_NULL).is_null());
        free(LAUNCH_DATA_NULL); // should not crash
    }

    #[test]
    fn invalid_type_alloc_returns_null() {
        assert!(alloc(255).is_null());
        assert!(alloc(0).is_null());
    }

    #[test]
    fn type_mismatch_setters_return_false() {
        let d = new_integer(10);
        assert!(!set_fd(d, 1));
        assert!(!set_bool(d, true));
        assert!(!set_string(d, c"x".as_ptr()));
        free(d);
    }

    #[test]
    fn dict_helpers() {
        let dict = alloc(LaunchDataType::Dictionary as u32);
        dict_insert(dict, new_string(c"world".as_ptr()), c"hello".as_ptr());
        dict_insert(dict, new_integer(42), c"num".as_ptr());

        assert_eq!(dict_get_string_value(dict, "hello").as_deref(), Some("world"));
        assert_eq!(dict_get_integer_value(dict, "num"), Some(42));
        assert!(dict_get_string_value(dict, "missing").is_none());
        assert!(dict_get_integer_value(dict, "hello").is_none());

        free(dict);
    }
}
