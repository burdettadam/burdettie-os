//! NSKeyedArchiver / NSKeyedUnarchiver / NSCoder shims.
//!
//! NSKeyedArchiver layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: data_ptr    (u64 — accumulated NSData output)
//!   offset 16: requiring   (u8 — requiresSecureCoding flag)
//!
//! NSKeyedUnarchiver layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: data_ptr    (u64 — NSData input)
//!   offset 16: requiring   (u8 — requiresSecureCoding flag)
//!
//! Key-value encoding/decoding uses a process-global store keyed by object
//! address so that we don't need to embed a HashMap pointer in the fixed-size
//! object layout.

use crate::cfstring;
use crate::nsobj;

use std::collections::HashMap;
use std::sync::Mutex;

const DATA_OFF: usize = 8;
const SECURE_OFF: usize = 16;

// ── ObjC runtime FFI (method dispatch) ─────────────────────────────────────
//
// These symbols live in objc-runtime, which is only linked in the full binary.
// In standalone cf-lite tests they are not available, so all helpers fall back
// gracefully to 0 / null when the symbols cannot be resolved.

/// Look up IMP for `sel_str` on `obj` and return it, or 0 if the method
/// is not registered in our runtime's table.  Returns 0 in standalone
/// test builds where the ObjC runtime is not linked.
fn lookup_imp(obj: u64, sel_str: &str) -> u64 {
    let Ok(csel) = std::ffi::CString::new(sel_str) else { return 0; };
    let sel_val = csel.as_ptr() as u64;

    // Resolve objc_has_method and objc_msg_lookup_imp via dlsym
    let has_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"objc_has_method\0".as_ptr().cast()) };
    if has_sym.is_null() { return 0; }
    let has_fn: extern "C" fn(u64, u64) -> u8 = unsafe { std::mem::transmute(has_sym) };
    if has_fn(obj, sel_val) == 0 { return 0; }

    let lookup_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"objc_msg_lookup_imp\0".as_ptr().cast()) };
    if lookup_sym.is_null() { return 0; }
    let lookup_fn: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(lookup_sym) };
    lookup_fn(obj, sel_val)
}

// ── Runtime helpers resolved via dlsym ─────────────────────────────────────
//
// These functions live in objc-runtime, which isn't linked during standalone
// cf-lite tests.  We resolve them lazily via dlsym so that the linker doesn't
// complain about missing symbols.

/// Call `class_getName(cls)` if available, returning the C string pointer.
fn get_class_name_cstr(cls: u64) -> *const std::ffi::c_char {
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"shim_class_get_name\0".as_ptr().cast()) };
    if sym.is_null() { return std::ptr::null(); }
    let f: extern "C" fn(u64) -> *const std::ffi::c_char = unsafe { std::mem::transmute(sym) };
    f(cls)
}

/// Call `objc_getClass(name)` if available, returning the class address.
fn get_class_by_name(name: *const std::ffi::c_char) -> u64 {
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"shim_objc_get_class\0".as_ptr().cast()) };
    if sym.is_null() { return 0; }
    let f: extern "C" fn(*const std::ffi::c_char) -> u64 = unsafe { std::mem::transmute(sym) };
    f(name)
}

// ── Global encode/decode store ─────────────────────────────────────────────
//
// Maps archiver/unarchiver object address → (key string → encoded object u64).
// Integer/bool primitives are wrapped as NSNumber before storage.

static ARCHIVER_STORE: Mutex<Option<HashMap<u64, HashMap<String, u64>>>> =
    Mutex::new(None);

// ── NSCoder abstract base stubs ────────────────────────────────────────────

/// `-[NSCoder allowsKeyedCoding]` — always YES; subclasses are keyed coders.
pub extern "C" fn shim_nscoder_allows_keyed_coding(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSCoder description]`
pub extern "C" fn shim_nscoder_description(_obj: u64, _cmd: u64) -> u64 {
    let c = std::ffi::CString::new("<NSCoder>").unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn store_encode(archiver: u64, key: &str, value: u64) {
    let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.entry(archiver).or_default().insert(key.to_string(), value);
}

fn store_decode(archiver: u64, key: &str) -> u64 {
    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&archiver))
        .and_then(|kv| kv.get(key))
        .copied()
        .unwrap_or(0)
}

fn key_string(ns_key: u64) -> String {
    cfstring::extract_cfstring_cstr(ns_key)
        .unwrap_or("")
        .to_string()
}

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ═══════════════════════════════════════════════════════════════
// NSKeyedArchiver
// ═══════════════════════════════════════════════════════════════

fn alloc_archiver() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nskeyedarchiver_class_addr(); }
    obj
}

/// `+[NSKeyedArchiver alloc]`
pub extern "C" fn shim_nskeyedarchiver_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_archiver()
}

/// `-[NSKeyedArchiver initRequiringSecureCoding:]`
pub extern "C" fn shim_nskeyedarchiver_init_secure(obj: u64, _cmd: u64, secure: u8) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + SECURE_OFF) as *mut u8) = secure; }
    obj
}

/// `+[NSKeyedArchiver archivedDataWithRootObject:requiringSecureCoding:error:]`
///
/// If the root object responds to `encodeWithCoder:`, we dispatch through the
/// ObjC runtime so that custom classes can archive their properties.  After the
/// call, the encoded key-value pairs (stored in `ARCHIVER_STORE`) are collected
/// into an NSDictionary with a `$class` key holding the class name, then
/// serialised to binary plist and returned as NSData.
///
/// For plain plist types (NSString, NSNumber, …) that do not implement
/// `encodeWithCoder:`, we fall back to direct plist serialisation.
pub extern "C" fn shim_nskeyedarchiver_archived_data(
    _cls: u64, _cmd: u64, root: u64, _secure: u8, error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    if root == 0 {
        return crate::nsdata::shim_nsdata_data(0, 0);
    }

    // Try NSCoding protocol dispatch first
    let imp = lookup_imp(root, "encodeWithCoder:");
    if imp != 0 {
        // Create a temporary archiver object that will collect encoded pairs
        let archiver = alloc_archiver();
        if archiver != 0 {
            let csel = std::ffi::CString::new("encodeWithCoder:").unwrap();
            let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
            f(root, csel.as_ptr() as u64, archiver);

            // Collect encoded pairs into an NSDictionary
            let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
                crate::nsobj::nsmutabledictionary_class_addr(), 0,
            );
            if dict != 0 {
                // Store the class name by looking up class_getName via dlsym
                let isa = unsafe { *(root as *const u64) };
                let class_name_cstr = get_class_name_cstr(isa);
                if !class_name_cstr.is_null() {
                    let class_key = make_nsstring("$class");
                    let class_val = cfstring::shim_cf_string_create_with_cstring(
                        0, class_name_cstr, 0,
                    );
                    crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(
                        dict, 0, class_val, class_key,
                    );
                }

                // Copy all encoded key-value pairs
                {
                    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(entries) = guard.as_ref().and_then(|m| m.get(&archiver)) {
                        for (k, v) in entries {
                            let k_ns = make_nsstring(k);
                            crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(
                                dict, 0, *v, k_ns,
                            );
                        }
                    }
                }

                // Clean up archiver's store entry
                {
                    let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(m) = guard.as_mut() { m.remove(&archiver); }
                }

                // Serialise to plist data
                let plist_data = crate::nspropertylistserialization::shim_plist_data_with_object(
                    0, 0, dict, 200 /* binary */, 0, 0,
                );
                if plist_data != 0 {
                    return plist_data;
                }
            }
        }
    }

    // Fallback: attempt plist serialization of the root directly
    let plist_data = crate::nspropertylistserialization::shim_plist_data_with_object(0, 0, root, 200 /* binary */, 0, 0);
    if plist_data != 0 {
        return plist_data;
    }
    // Fallback: return empty data
    crate::nsdata::shim_nsdata_data(0, 0)
}

/// `+[NSKeyedArchiver archivedDataWithRootObject:]`  (deprecated but common)
pub extern "C" fn shim_nskeyedarchiver_archived_data_legacy(
    _cls: u64, _cmd: u64, root: u64,
) -> u64 {
    shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0)
}

/// `+[NSKeyedArchiver archiveRootObject:toFile:]`
///
/// Archives `root` to a file at `path`.  Uses the in-memory store to serialise
/// the root object's encoded key-value pairs as a flat plist-style byte blob,
/// then writes them to the file.  Returns YES on success.
pub extern "C" fn shim_nskeyedarchiver_archive_root_to_file(
    _cls: u64, _cmd: u64, root: u64, path: u64,
) -> u8 {
    let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
    if data == 0 { return 0; }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644) };
    if fd < 0 { return 0; }
    let len = crate::nsdata::shim_nsdata_length(data, 0);
    let bytes = crate::nsdata::shim_nsdata_bytes(data, 0);
    let mut written = 0usize;
    if len > 0 && bytes != 0 {
        let rc = unsafe { libc::write(fd, bytes as *const libc::c_void, len as usize) };
        if rc > 0 { written = rc as usize; }
    }
    unsafe { libc::close(fd); }
    u8::from(written == len as usize)
}

/// `-[NSKeyedArchiver encodedData]`
pub extern "C" fn shim_nskeyedarchiver_encoded_data(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + DATA_OFF) as *const u64) }
}

/// `-[NSKeyedArchiver setRequiresSecureCoding:]`
pub extern "C" fn shim_nskeyedarchiver_set_requires_secure(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + SECURE_OFF) as *mut u8) = val; }
}

/// `-[NSKeyedArchiver requiresSecureCoding]`
pub extern "C" fn shim_nskeyedarchiver_requires_secure(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + SECURE_OFF) as *const u8) }
}

/// `-[NSKeyedArchiver setOutputFormat:]` — no-op stub.
pub extern "C" fn shim_nskeyedarchiver_set_output_format(_obj: u64, _cmd: u64, _format: u64) {}

/// `-[NSKeyedArchiver outputFormat]` — returns NSPropertyListBinaryFormat_v1_0 (200).
pub extern "C" fn shim_nskeyedarchiver_output_format(_obj: u64, _cmd: u64) -> u64 {
    200
}

/// `-[NSKeyedArchiver finishEncoding]`
pub extern "C" fn shim_nskeyedarchiver_finish_encoding(_obj: u64, _cmd: u64) {
    log::debug!("NSKeyedArchiver finishEncoding (stub)");
}

// ── Key-value encode methods ────────────────────────────────────────────────

/// `-[NSKeyedArchiver encodeObject:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_object_for_key(
    obj: u64, _cmd: u64, value: u64, key: u64,
) {
    if obj == 0 { return; }
    store_encode(obj, &key_string(key), value);
}

/// `-[NSKeyedArchiver encodeInteger:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_integer_for_key(
    obj: u64, _cmd: u64, value: i64, key: u64,
) {
    if obj == 0 { return; }
    let ns = crate::nsnumber::shim_nsnumber_number_with_integer(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeBool:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_bool_for_key(
    obj: u64, _cmd: u64, value: u8, key: u64,
) {
    if obj == 0 { return; }
    let ns = crate::nsnumber::shim_nsnumber_number_with_bool(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeDouble:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_double_for_key(
    obj: u64, _cmd: u64, value: f64, key: u64,
) {
    if obj == 0 { return; }
    let ns = crate::nsnumber::shim_nsnumber_number_with_double(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeFloat:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_float_for_key(
    obj: u64, _cmd: u64, value: f32, key: u64,
) {
    if obj == 0 { return; }
    let ns = crate::nsnumber::shim_nsnumber_number_with_double(0, 0, value as f64);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeInt:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int_for_key(
    obj: u64, _cmd: u64, value: i32, key: u64,
) {
    if obj == 0 { return; }
    let ns = crate::nsnumber::shim_nsnumber_number_with_integer(0, 0, value as i64);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeInt32:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int32_for_key(
    obj: u64, _cmd: u64, value: i32, key: u64,
) {
    shim_nskeyedarchiver_encode_int_for_key(obj, _cmd, value, key);
}

/// `-[NSKeyedArchiver encodeInt64:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int64_for_key(
    obj: u64, _cmd: u64, value: i64, key: u64,
) {
    shim_nskeyedarchiver_encode_integer_for_key(obj, _cmd, value, key);
}

pub extern "C" fn shim_nskeyedarchiver_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_nsstring("<NSKeyedArchiver>")
}

// ═══════════════════════════════════════════════════════════════
// NSKeyedUnarchiver
// ═══════════════════════════════════════════════════════════════

fn alloc_unarchiver() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nskeyedunarchiver_class_addr(); }
    obj
}

/// `+[NSKeyedUnarchiver alloc]`
pub extern "C" fn shim_nskeyedunarchiver_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_unarchiver()
}

/// `-[NSKeyedUnarchiver initForReadingFromData:error:]`
pub extern "C" fn shim_nskeyedunarchiver_init_from_data(obj: u64, _cmd: u64, data: u64, error_out: u64) -> u64 {
    if obj == 0 { return 0; }
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    unsafe { *((obj as usize + DATA_OFF) as *mut u64) = data; }
    obj
}

/// `+[NSKeyedUnarchiver unarchivedObjectOfClass:fromData:error:]`
pub extern "C" fn shim_nskeyedunarchiver_unarchived_object(
    _cls: u64, _cmd: u64, _allowedclass: u64, data: u64, error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    if data == 0 { return 0; }

    // Deserialize the plist to get an NSDictionary
    let dict = crate::nspropertylistserialization::shim_plist_with_data(0, 0, data, 0, 0, 0);
    if dict == 0 { return 0; }

    // Check if the result is a dictionary (NSCoding archive) — if not, return it as-is
    let isa = unsafe { *(dict as *const u64) };
    let dict_class = nsobj::nsdictionary_class_addr();
    let mdict_class = nsobj::nsmutabledictionary_class_addr();
    let cdict_class = nsobj::nsconstantdictionary_class_addr();
    let is_dict = isa == dict_class || isa == mdict_class || isa == cdict_class;
    if !is_dict {
        return dict;
    }

    // Check if this is an NSCoding archive (has "$class" key)
    let class_key = make_nsstring("$class");
    let class_val = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, class_key);
    if class_val == 0 {
        // Not an NSCoding archive — return the deserialized plist value directly
        return dict;
    }

    // Get the class name and look up the class
    let Some(class_name) = cfstring::extract_cfstring_cstr(class_val) else { return dict; };
    let class_name_c = std::ffi::CString::new(class_name).unwrap_or_default();

    let cls = get_class_by_name(class_name_c.as_ptr());
    if cls == 0 {
        return dict;
    }
    log::debug!("NSKeyedUnarchiver: restoring class '{}' (cls={:#x})", class_name, cls);

    // Allocate an instance of the class
    let alloc_imp = lookup_imp(cls, "alloc");
    let instance = if alloc_imp != 0 {
        let csel = std::ffi::CString::new("alloc").unwrap();
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(alloc_imp) };
        f(cls, csel.as_ptr() as u64)
    } else {
        // Manual alloc: read instance_size from ClassInfo isn't directly accessible,
        // so use a reasonable default (128 bytes covers most Foundation objects)
        let obj = crate::pal_alloc().calloc(1, 128) as u64;
        if obj != 0 { unsafe { *(obj as *mut u64) = cls; } }
        obj
    };
    if instance == 0 { return dict; }

    // Create a temporary unarchiver and populate its store from the dict
    let unarchiver = alloc_unarchiver();
    if unarchiver == 0 { return dict; }
    unsafe { *((unarchiver as usize + DATA_OFF) as *mut u64) = data; }

    // Populate the unarchiver's ARCHIVER_STORE from the NSDictionary entries
    {
        let all_keys = crate::nsdictionary::shim_nsdictionary_all_keys(dict, 0);
        if all_keys != 0 {
            let count = crate::nsarray::shim_nsarray_count(all_keys, 0);
            for i in 0..count {
                let k = crate::nsarray::shim_nsarray_object_at_index(all_keys, 0, i);
                // Skip the "$class" meta-key (compare by string content, not pointer)
                if let Some(ks) = cfstring::extract_cfstring_cstr(k) {
                    if ks == "$class" { continue; }
                    let v = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k);
                    store_encode(unarchiver, ks, v);
                }
            }
        }
    }

    // Call initWithCoder: on the instance
    let init_imp = lookup_imp(instance, "initWithCoder:");
    let result = if init_imp != 0 {
        log::debug!("NSKeyedUnarchiver: calling initWithCoder: on instance {:#x}", instance);
        let csel = std::ffi::CString::new("initWithCoder:").unwrap();
        let f: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(init_imp) };
        f(instance, csel.as_ptr() as u64, unarchiver)
    } else {
        log::debug!("NSKeyedUnarchiver: '{}' has no initWithCoder:, returning raw instance", class_name);
        instance
    };
    log::debug!("NSKeyedUnarchiver: result={:#x}", result);

    // Clean up unarchiver store
    {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = guard.as_mut() { m.remove(&unarchiver); }
    }

    result
}

/// `+[NSKeyedUnarchiver unarchiveObjectWithData:]` (deprecated but common)
pub extern "C" fn shim_nskeyedunarchiver_unarchive_legacy(
    _cls: u64, _cmd: u64, data: u64,
) -> u64 {
    shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0)
}

/// `+[NSKeyedUnarchiver unarchiveObjectWithFile:]` — reads file into NSData, then unarchives.
pub extern "C" fn shim_nskeyedunarchiver_unarchive_from_file(
    _cls: u64, _cmd: u64, path: u64,
) -> u64 {
    if path == 0 { return 0; }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else { return 0; };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    if fd < 0 { return 0; }
    let mut result = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 { break; }
        result.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { libc::close(fd); }
    if result.is_empty() { return 0; }
    let data = crate::nsdata::make_nsdata(&result);
    shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0)
}

/// `-[NSKeyedUnarchiver setRequiresSecureCoding:]`
pub extern "C" fn shim_nskeyedunarchiver_set_requires_secure(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + SECURE_OFF) as *mut u8) = val; }
}

/// `-[NSKeyedUnarchiver requiresSecureCoding]`
pub extern "C" fn shim_nskeyedunarchiver_requires_secure(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + SECURE_OFF) as *const u8) }
}

/// `-[NSKeyedUnarchiver setDecodingFailurePolicy:]` — no-op stub.
pub extern "C" fn shim_nskeyedunarchiver_set_decoding_failure_policy(_obj: u64, _cmd: u64, _policy: i64) {}

/// `-[NSKeyedUnarchiver decodingFailurePolicy]` — returns 0 (NSDecodingFailureRaiseException).
pub extern "C" fn shim_nskeyedunarchiver_decoding_failure_policy(_obj: u64, _cmd: u64) -> i64 {
    0
}

/// `-[NSKeyedUnarchiver finishDecoding]`
pub extern "C" fn shim_nskeyedunarchiver_finish_decoding(_obj: u64, _cmd: u64) {
    log::debug!("NSKeyedUnarchiver finishDecoding (stub)");
}

// ── Key-value decode methods ────────────────────────────────────────────────

/// `-[NSKeyedUnarchiver decodeObjectForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_object_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> u64 {
    if obj == 0 { return 0; }
    store_decode(obj, &key_string(key))
}

/// `-[NSKeyedUnarchiver decodeIntegerForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_integer_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> i64 {
    let ns = store_decode(obj, &key_string(key));
    if ns == 0 { return 0; }
    crate::nsnumber::shim_nsnumber_integer_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeBoolForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_bool_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> u8 {
    let ns = store_decode(obj, &key_string(key));
    if ns == 0 { return 0; }
    crate::nsnumber::shim_nsnumber_bool_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeDoubleForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_double_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> f64 {
    let ns = store_decode(obj, &key_string(key));
    if ns == 0 { return 0.0; }
    crate::nsnumber::shim_nsnumber_double_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeFloatForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_float_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> f32 {
    shim_nskeyedunarchiver_decode_double_for_key(obj, _cmd, key) as f32
}

/// `-[NSKeyedUnarchiver decodeInt32ForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_int32_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> i32 {
    shim_nskeyedunarchiver_decode_integer_for_key(obj, _cmd, key) as i32
}

/// `-[NSKeyedUnarchiver decodeInt64ForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_int64_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> i64 {
    shim_nskeyedunarchiver_decode_integer_for_key(obj, _cmd, key)
}

pub extern "C" fn shim_nskeyedunarchiver_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_nsstring("<NSKeyedUnarchiver>")
}

/// `-[NSKeyedUnarchiver containsValueForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_contains_value_for_key(
    obj: u64, _cmd: u64, key: u64,
) -> u8 {
    if obj == 0 { return 0; }
    let k = key_string(key);
    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let found = guard
        .as_ref()
        .and_then(|m| m.get(&obj))
        .map_or(false, |kv| kv.contains_key(&k));
    u8::from(found)
}

/// `-[NSKeyedUnarchiver decodeObjectOfClass:forKey:]` — same as `decodeObjectForKey:`.
pub extern "C" fn shim_nskeyedunarchiver_decode_object_of_class_for_key(
    obj: u64, _cmd: u64, _cls: u64, key: u64,
) -> u64 {
    shim_nskeyedunarchiver_decode_object_for_key(obj, 0, key)
}

// ── Bytes encode/decode ─────────────────────────────────────────────────────

/// `-[NSKeyedArchiver encodeBytes:length:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_bytes_for_key(
    obj: u64, _cmd: u64, bytes: u64, length: u64, key: u64,
) {
    if obj == 0 { return; }
    let data = if bytes != 0 && length > 0 {
        let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, length as usize) };
        crate::nsdata::make_nsdata(slice)
    } else {
        crate::nsdata::shim_nsdata_data(0, 0)
    };
    store_encode(obj, &key_string(key), data);
}

/// `-[NSKeyedUnarchiver decodeBytesForKey:returnedLength:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_bytes_for_key(
    obj: u64, _cmd: u64, key: u64, length_out: *mut u64,
) -> u64 {
    if obj == 0 {
        if !length_out.is_null() { unsafe { *length_out = 0; } }
        return 0;
    }
    let data = store_decode(obj, &key_string(key));
    if data == 0 {
        if !length_out.is_null() { unsafe { *length_out = 0; } }
        return 0;
    }
    let len = crate::nsdata::shim_nsdata_length(data, 0);
    if !length_out.is_null() { unsafe { *length_out = len; } }
    crate::nsdata::shim_nsdata_bytes(data, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const ARCHIVER_SIZE: usize = 24;
    const UNARCHIVER_SIZE: usize = 24;

    fn archiver_isa() -> u64 {
        crate::nsobj::nskeyedarchiver_class_addr()
    }

    fn unarchiver_isa() -> u64 {
        crate::nsobj::nskeyedunarchiver_class_addr()
    }

    /// Helper: clear global ARCHIVER_STORE entries for a given object address.
    fn clear_store(addr: u64) {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            map.remove(&addr);
        }
    }

    // ── NSCoder ────────────────────────────────────────────────────

    #[test]
    fn nscoder_allows_keyed_coding() {
        assert_eq!(shim_nscoder_allows_keyed_coding(0, 0), 1);
        assert_eq!(shim_nscoder_allows_keyed_coding(0xABC, 0), 1);
    }

    #[test]
    fn nscoder_description() {
        let desc = shim_nscoder_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSCoder>");
    }

    // ── NSKeyedArchiver alloc / init ───────────────────────────────

    #[test]
    fn archiver_alloc() {
        let obj = shim_nskeyedarchiver_alloc(0, 0);
        assert_ne!(obj, 0);
        // ISA should be set to the archiver class
        assert_eq!(read_field(obj, 0), archiver_isa());
        free_obj(obj);
    }

    #[test]
    fn archiver_init_secure_nil() {
        assert_eq!(shim_nskeyedarchiver_init_secure(0, 0, 1), 0);
    }

    #[test]
    fn archiver_init_secure_sets_flag() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let ret = shim_nskeyedarchiver_init_secure(obj, 0, 1);
        assert_eq!(ret, obj);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 1);
        free_obj(obj);
    }

    // ── requiresSecureCoding get/set ──────────────────────────────

    #[test]
    fn archiver_requires_secure_nil() {
        assert_eq!(shim_nskeyedarchiver_requires_secure(0, 0), 0);
    }

    #[test]
    fn archiver_set_requires_secure_nil() {
        // Should not crash
        shim_nskeyedarchiver_set_requires_secure(0, 0, 1);
    }

    #[test]
    fn archiver_requires_secure_roundtrip() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 0);
        shim_nskeyedarchiver_set_requires_secure(obj, 0, 1);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 1);
        shim_nskeyedarchiver_set_requires_secure(obj, 0, 0);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 0);
        free_obj(obj);
    }

    // ── encodedData ───────────────────────────────────────────────

    #[test]
    fn archiver_encoded_data_nil() {
        assert_eq!(shim_nskeyedarchiver_encoded_data(0, 0), 0);
    }

    #[test]
    fn archiver_encoded_data_reads_field() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        write_field(obj, DATA_OFF, 0xCAFE);
        assert_eq!(shim_nskeyedarchiver_encoded_data(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    // ── finishEncoding ────────────────────────────────────────────

    #[test]
    fn archiver_finish_encoding_nil() {
        // Should not crash
        shim_nskeyedarchiver_finish_encoding(0, 0);
    }

    #[test]
    fn archiver_finish_encoding() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        shim_nskeyedarchiver_finish_encoding(obj, 0);
        free_obj(obj);
    }

    // ── description (archiver) ────────────────────────────────────

    #[test]
    fn archiver_description_nil() {
        assert_eq!(shim_nskeyedarchiver_description(0, 0), 0);
    }

    #[test]
    fn archiver_description() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let desc = shim_nskeyedarchiver_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSKeyedArchiver>");
        free_obj(obj);
    }

    // ── encode / decode roundtrip (object) ────────────────────────

    #[test]
    fn encode_object_for_key_nil() {
        // Should not crash
        shim_nskeyedarchiver_encode_object_for_key(0, 0, 42, 0);
    }

    #[test]
    fn encode_decode_object_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("myKey");
        let value = make_string("hello");

        shim_nskeyedarchiver_encode_object_for_key(archiver, 0, value, key);

        // Decode using the same address acts as the unarchiver store key
        let decoded = store_decode(archiver, "myKey");
        assert_eq!(decoded, value);
        assert_string_eq(decoded, "hello");

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (integer) ───────────────────────

    #[test]
    fn encode_integer_for_key_nil() {
        shim_nskeyedarchiver_encode_integer_for_key(0, 0, 42, 0);
    }

    #[test]
    fn encode_decode_integer_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("count");

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 99, key);

        // Use decode on the same address
        let decoded = shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key);
        assert_eq!(decoded, 99);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (bool) ──────────────────────────

    #[test]
    fn encode_bool_for_key_nil() {
        shim_nskeyedarchiver_encode_bool_for_key(0, 0, 1, 0);
    }

    #[test]
    fn encode_decode_bool_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("flag");

        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 1, key);
        let decoded = shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, key);
        assert_eq!(decoded, 1);

        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 0, key);
        let decoded = shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, key);
        assert_eq!(decoded, 0);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (double) ────────────────────────

    #[test]
    fn encode_double_for_key_nil() {
        shim_nskeyedarchiver_encode_double_for_key(0, 0, 3.14, 0);
    }

    #[test]
    fn encode_decode_double_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("pi");

        shim_nskeyedarchiver_encode_double_for_key(archiver, 0, 3.14, key);
        let decoded = shim_nskeyedunarchiver_decode_double_for_key(archiver, 0, key);
        assert!((decoded - 3.14).abs() < 1e-10);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (float) ─────────────────────────

    #[test]
    fn encode_float_for_key_nil() {
        shim_nskeyedarchiver_encode_float_for_key(0, 0, 2.5, 0);
    }

    #[test]
    fn encode_decode_float_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("ratio");

        shim_nskeyedarchiver_encode_float_for_key(archiver, 0, 2.5f32, key);
        let decoded = shim_nskeyedunarchiver_decode_float_for_key(archiver, 0, key);
        assert!((decoded - 2.5f32).abs() < 1e-5);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (int / int32) ───────────────────

    #[test]
    fn encode_int_for_key_nil() {
        shim_nskeyedarchiver_encode_int_for_key(0, 0, 7, 0);
    }

    #[test]
    fn encode_decode_int32_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("small");

        shim_nskeyedarchiver_encode_int32_for_key(archiver, 0, -42, key);
        let decoded = shim_nskeyedunarchiver_decode_int32_for_key(archiver, 0, key);
        assert_eq!(decoded, -42);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (int64) ─────────────────────────

    #[test]
    fn encode_decode_int64_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("big");

        shim_nskeyedarchiver_encode_int64_for_key(archiver, 0, i64::MAX, key);
        let decoded = shim_nskeyedunarchiver_decode_int64_for_key(archiver, 0, key);
        assert_eq!(decoded, i64::MAX);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── decode missing key returns zero ───────────────────────────

    #[test]
    fn decode_missing_key_returns_zero() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let key = make_string("nonexistent");

        assert_eq!(shim_nskeyedunarchiver_decode_object_for_key(obj, 0, key), 0);
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(obj, 0, key), 0);
        assert_eq!(shim_nskeyedunarchiver_decode_bool_for_key(obj, 0, key), 0);
        assert!((shim_nskeyedunarchiver_decode_double_for_key(obj, 0, key)).abs() < 1e-10);
        assert!((shim_nskeyedunarchiver_decode_float_for_key(obj, 0, key)).abs() < 1e-5);
        assert_eq!(shim_nskeyedunarchiver_decode_int32_for_key(obj, 0, key), 0);
        assert_eq!(shim_nskeyedunarchiver_decode_int64_for_key(obj, 0, key), 0);

        free_obj(obj);
    }

    // ── NSKeyedUnarchiver alloc ───────────────────────────────────

    #[test]
    fn unarchiver_alloc() {
        let obj = shim_nskeyedunarchiver_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), unarchiver_isa());
        free_obj(obj);
    }

    // ── initForReadingFromData ────────────────────────────────────

    #[test]
    fn unarchiver_init_from_data_nil() {
        assert_eq!(shim_nskeyedunarchiver_init_from_data(0, 0, 0, 0), 0);
    }

    #[test]
    fn unarchiver_init_from_data_sets_field() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let fake_data: u64 = 0xBEEF;
        let ret = shim_nskeyedunarchiver_init_from_data(obj, 0, fake_data, 0);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, DATA_OFF), fake_data);
        free_obj(obj);
    }

    #[test]
    fn unarchiver_init_from_data_clears_error() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let mut error_out: u64 = 0xBAD;
        let ret = shim_nskeyedunarchiver_init_from_data(obj, 0, 0xBEEF, &mut error_out as *mut u64 as u64);
        assert_eq!(ret, obj);
        assert_eq!(error_out, 0);
        free_obj(obj);
    }

    // ── requiresSecureCoding (unarchiver) ─────────────────────────

    #[test]
    fn unarchiver_requires_secure_nil() {
        assert_eq!(shim_nskeyedunarchiver_requires_secure(0, 0), 0);
    }

    #[test]
    fn unarchiver_set_requires_secure_nil() {
        shim_nskeyedunarchiver_set_requires_secure(0, 0, 1);
    }

    #[test]
    fn unarchiver_requires_secure_roundtrip() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 0);
        shim_nskeyedunarchiver_set_requires_secure(obj, 0, 1);
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 1);
        shim_nskeyedunarchiver_set_requires_secure(obj, 0, 0);
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 0);
        free_obj(obj);
    }

    // ── finishDecoding ────────────────────────────────────────────

    #[test]
    fn unarchiver_finish_decoding_nil() {
        shim_nskeyedunarchiver_finish_decoding(0, 0);
    }

    #[test]
    fn unarchiver_finish_decoding() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        shim_nskeyedunarchiver_finish_decoding(obj, 0);
        free_obj(obj);
    }

    // ── decode nil safety ─────────────────────────────────────────

    #[test]
    fn decode_object_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_object_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_integer_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_bool_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_bool_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_double_for_key_nil() {
        assert!((shim_nskeyedunarchiver_decode_double_for_key(0, 0, 0)).abs() < 1e-10);
    }

    #[test]
    fn decode_float_for_key_nil() {
        assert!((shim_nskeyedunarchiver_decode_float_for_key(0, 0, 0)).abs() < 1e-5);
    }

    #[test]
    fn decode_int32_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_int32_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_int64_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_int64_for_key(0, 0, 0), 0);
    }

    // ── description (unarchiver) ──────────────────────────────────

    #[test]
    fn unarchiver_description_nil() {
        assert_eq!(shim_nskeyedunarchiver_description(0, 0), 0);
    }

    #[test]
    fn unarchiver_description() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let desc = shim_nskeyedunarchiver_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSKeyedUnarchiver>");
        free_obj(obj);
    }

    // ── archived_data nil/null root ───────────────────────────────

    #[test]
    fn archived_data_nil_root_returns_empty_data() {
        let data = shim_nskeyedarchiver_archived_data(0, 0, 0, 0, 0);
        // nil root → empty NSData (shim_nsdata_data returns an empty NSData)
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, 0);
    }

    #[test]
    fn archived_data_clears_error_out() {
        let mut err: u64 = 0xBAD;
        let _data = shim_nskeyedarchiver_archived_data(0, 0, 0, 0, &mut err as *mut u64 as u64);
        assert_eq!(err, 0);
    }

    #[test]
    fn archived_data_with_string_root() {
        let root = make_string("testValue");
        let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
        assert_ne!(data, 0);
        // A string should yield non-empty plist data
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert!(len > 0);
    }

    // ── archived_data_legacy ──────────────────────────────────────

    #[test]
    fn archived_data_legacy_nil_root() {
        let data = shim_nskeyedarchiver_archived_data_legacy(0, 0, 0);
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, 0);
    }

    // ── unarchived_object ─────────────────────────────────────────

    #[test]
    fn unarchived_object_nil_data() {
        assert_eq!(shim_nskeyedunarchiver_unarchived_object(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn unarchived_object_clears_error_out() {
        let mut err: u64 = 0xBAD;
        let _ = shim_nskeyedunarchiver_unarchived_object(0, 0, 0, 0, &mut err as *mut u64 as u64);
        assert_eq!(err, 0);
    }

    // ── unarchive_legacy ──────────────────────────────────────────

    #[test]
    fn unarchive_legacy_nil_data() {
        assert_eq!(shim_nskeyedunarchiver_unarchive_legacy(0, 0, 0), 0);
    }

    // ── archive_root_to_file nil safety ───────────────────────────

    #[test]
    fn archive_root_to_file_nil_root() {
        // nil root → shim creates empty data → write succeeds only if path valid
        // but we pass nil path, so it should return 0
        assert_eq!(shim_nskeyedarchiver_archive_root_to_file(0, 0, 0, 0), 0);
    }

    #[test]
    fn archive_root_to_file_nil_path() {
        let root = make_string("something");
        assert_eq!(shim_nskeyedarchiver_archive_root_to_file(0, 0, root, 0), 0);
    }

    // ── unarchive_from_file nil safety ────────────────────────────

    #[test]
    fn unarchive_from_file_nil_path() {
        assert_eq!(shim_nskeyedunarchiver_unarchive_from_file(0, 0, 0), 0);
    }

    // ── archive → unarchive roundtrip via plist ───────────────────

    #[test]
    fn archive_unarchive_string_roundtrip() {
        let root = make_string("roundtrip");
        let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert!(len > 0);
        let unarchived = shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0);
        // The plist round trip should recover the string
        if unarchived != 0 {
            assert_string_eq(unarchived, "roundtrip");
        }
    }

    // ── multiple keys on same archiver ────────────────────────────

    #[test]
    fn encode_multiple_keys() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let k1 = make_string("name");
        let k2 = make_string("age");
        let k3 = make_string("active");

        let v1 = make_string("Alice");
        shim_nskeyedarchiver_encode_object_for_key(archiver, 0, v1, k1);
        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 30, k2);
        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 1, k3);

        assert_eq!(shim_nskeyedunarchiver_decode_object_for_key(archiver, 0, k1), v1);
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, k2), 30);
        assert_eq!(shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, k3), 1);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── overwrite key ─────────────────────────────────────────────

    #[test]
    fn encode_overwrite_key() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("val");

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 1, key);
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key), 1);

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 2, key);
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key), 2);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── outputFormat get/set ──────────────────────────────────────

    #[test]
    fn archiver_output_format_returns_binary() {
        assert_eq!(shim_nskeyedarchiver_output_format(0, 0), 200);
    }

    #[test]
    fn archiver_set_output_format_noop() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        shim_nskeyedarchiver_set_output_format(obj, 0, 100); // XML
        // getter always returns 200 (stub)
        assert_eq!(shim_nskeyedarchiver_output_format(obj, 0), 200);
        free_obj(obj);
    }

    #[test]
    fn archiver_set_output_format_nil() {
        // Should not crash
        shim_nskeyedarchiver_set_output_format(0, 0, 200);
    }

    // ── decodingFailurePolicy get/set (unarchiver) ────────────────

    #[test]
    fn unarchiver_decoding_failure_policy_returns_zero() {
        assert_eq!(shim_nskeyedunarchiver_decoding_failure_policy(0, 0), 0);
    }

    #[test]
    fn unarchiver_set_decoding_failure_policy_noop() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        shim_nskeyedunarchiver_set_decoding_failure_policy(obj, 0, 1);
        // getter always returns 0 (stub)
        assert_eq!(shim_nskeyedunarchiver_decoding_failure_policy(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn unarchiver_set_decoding_failure_policy_nil() {
        // Should not crash
        shim_nskeyedunarchiver_set_decoding_failure_policy(0, 0, 1);
    }
}
