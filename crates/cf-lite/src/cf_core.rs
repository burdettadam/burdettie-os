//! Core CoreFoundation functions: CFRetain, CFGetTypeID, CFEqual, CFHash,
//! CFCopyDescription, and toll-free bridging infrastructure.
//!
//! Toll-free bridging is trivial in cf-lite: CF objects and their NS
//! counterparts are literally the same objects with the same ISA.
//! `(__bridge NSString *)cfStr` is a no-op pointer cast.

use crate::cfstring;
use crate::nsobj;

// ── CFTypeID constants ──────────────────────────────────────────────

const CF_STRING_TYPE_ID: u64 = 7;
const CF_ARRAY_TYPE_ID: u64 = 19;
const CF_DICTIONARY_TYPE_ID: u64 = 18;
const CF_DATA_TYPE_ID: u64 = 20;
const CF_URL_TYPE_ID: u64 = 29;
const CF_BUNDLE_TYPE_ID: u64 = 47;
const CF_BOOLEAN_TYPE_ID: u64 = 21;
const CF_NUMBER_TYPE_ID: u64 = 22;
const CF_NULL_TYPE_ID: u64 = 16;
const CF_UNKNOWN_TYPE_ID: u64 = 0;

/// Map an ISA address to a CFTypeID.
fn type_id_for_isa(isa: u64) -> u64 {
    if isa == nsobj::nsstring_class_addr() || isa == nsobj::nsmutablestring_class_addr() {
        CF_STRING_TYPE_ID
    } else if isa == nsobj::nsarray_class_addr() || isa == nsobj::nsmutablearray_class_addr() {
        CF_ARRAY_TYPE_ID
    } else if isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
    {
        CF_DICTIONARY_TYPE_ID
    } else if isa == nsobj::nsdata_class_addr() {
        CF_DATA_TYPE_ID
    } else if isa == nsobj::nsurl_class_addr() {
        CF_URL_TYPE_ID
    } else if isa == nsobj::nsbundle_class_addr() {
        CF_BUNDLE_TYPE_ID
    } else if isa == nsobj::nsnumber_class_addr() {
        CF_NUMBER_TYPE_ID
    } else if isa == nsobj::nsnull_class_addr() {
        CF_NULL_TYPE_ID
    } else if isa == crate::foundation::cfconstant_string_class_addr() {
        CF_STRING_TYPE_ID
    } else {
        CF_UNKNOWN_TYPE_ID
    }
}

fn read_isa(obj: u64) -> u64 {
    unsafe { *(obj as *const u64) }
}

// ── CFRetain ────────────────────────────────────────────────────────

/// `CFTypeRef CFRetain(CFTypeRef cf)`
pub extern "C" fn shim_cf_retain(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    objc_runtime::refcount::REFCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(obj);
    obj
}

// ── CFGetTypeID ────────────────────────────────────────────────────

/// `CFTypeID CFGetTypeID(CFTypeRef cf)`
pub extern "C" fn shim_cf_get_type_id(obj: u64) -> u64 {
    if obj == 0 {
        return CF_UNKNOWN_TYPE_ID;
    }
    let isa = read_isa(obj);
    type_id_for_isa(isa)
}

/// `CFTypeID CFStringGetTypeID(void)` — returns the type ID for CFString.
pub extern "C" fn shim_cf_string_get_type_id() -> u64 {
    CF_STRING_TYPE_ID
}

/// `CFTypeID CFArrayGetTypeID(void)`
pub extern "C" fn shim_cf_array_get_type_id() -> u64 {
    CF_ARRAY_TYPE_ID
}

/// `CFTypeID CFDictionaryGetTypeID(void)`
pub extern "C" fn shim_cf_dictionary_get_type_id() -> u64 {
    CF_DICTIONARY_TYPE_ID
}

/// `CFTypeID CFDataGetTypeID(void)`
pub extern "C" fn shim_cf_data_get_type_id() -> u64 {
    CF_DATA_TYPE_ID
}

/// `CFTypeID CFURLGetTypeID(void)`
pub extern "C" fn shim_cf_url_get_type_id() -> u64 {
    CF_URL_TYPE_ID
}

/// `CFTypeID CFBooleanGetTypeID(void)`
pub extern "C" fn shim_cf_boolean_get_type_id() -> u64 {
    CF_BOOLEAN_TYPE_ID
}

/// `CFTypeID CFNumberGetTypeID(void)`
pub extern "C" fn shim_cf_number_get_type_id() -> u64 {
    CF_NUMBER_TYPE_ID
}

/// `CFTypeID CFNullGetTypeID(void)`
pub extern "C" fn shim_cf_null_get_type_id() -> u64 {
    CF_NULL_TYPE_ID
}

// ── CFEqual ────────────────────────────────────────────────────────

/// `Boolean CFEqual(CFTypeRef cf1, CFTypeRef cf2)`
pub extern "C" fn shim_cf_equal(a: u64, b: u64) -> u8 {
    if a == b {
        return 1;
    }
    if a == 0 || b == 0 {
        return 0;
    }

    let isa_a = read_isa(a);
    let isa_b = read_isa(b);

    // Only compare objects of the same type
    if type_id_for_isa(isa_a) != type_id_for_isa(isa_b) {
        return 0;
    }

    let tid = type_id_for_isa(isa_a);
    match tid {
        CF_STRING_TYPE_ID => {
            // String content comparison
            let sa = cfstring::extract_cfstring_cstr(a);
            let sb = cfstring::extract_cfstring_cstr(b);
            match (sa, sb) {
                (Some(x), Some(y)) => u8::from(x == y),
                _ => 0,
            }
        }
        CF_DATA_TYPE_ID => crate::nsdata::shim_nsdata_is_equal_to_data(a, 0, b),
        _ => 0, // Fall back to pointer inequality
    }
}

// ── CFHash ─────────────────────────────────────────────────────────

/// `CFHashCode CFHash(CFTypeRef cf)`
pub extern "C" fn shim_cf_hash(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let isa = read_isa(obj);
    let tid = type_id_for_isa(isa);

    if tid == CF_STRING_TYPE_ID {
        // DJB2 hash over UTF-8 bytes (same as NSString hash)
        if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
            let mut h: u64 = 5381;
            for byte in s.bytes() {
                h = h.wrapping_mul(33).wrapping_add(byte as u64);
            }
            return h;
        }
    }

    // Default: identity hash
    obj
}

// ── CFCopyDescription ──────────────────────────────────────────────

/// `CFStringRef CFCopyDescription(CFTypeRef cf)`
pub extern "C" fn shim_cf_copy_description(obj: u64) -> u64 {
    if obj == 0 {
        let cstr = std::ffi::CString::new("(null)").unwrap();
        return cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
    }

    let isa = read_isa(obj);
    let tid = type_id_for_isa(isa);

    let desc = match tid {
        CF_STRING_TYPE_ID => {
            if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
                s.to_string()
            } else {
                format!("<CFString {:#x}>", obj)
            }
        }
        CF_ARRAY_TYPE_ID => {
            let count = unsafe { *((obj as usize + 8) as *const u64) };
            format!("<CFArray {:#x} [count={}]>", obj, count)
        }
        CF_DICTIONARY_TYPE_ID => {
            let count = unsafe { *((obj as usize + 8) as *const u64) };
            format!("<CFDictionary {:#x} [count={}]>", obj, count)
        }
        CF_DATA_TYPE_ID => {
            let length = unsafe { *((obj as usize + 8) as *const u64) };
            format!("<CFData {:#x} [length={}]>", obj, length)
        }
        CF_URL_TYPE_ID => {
            let str_ptr = unsafe { *((obj as usize + 8) as *const u64) };
            if let Some(s) = cfstring::extract_cfstring_cstr(str_ptr) {
                s.to_string()
            } else {
                format!("<CFURL {:#x}>", obj)
            }
        }
        _ => format!("<CFObject {:#x}>", obj),
    };

    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
}

// ── CFArray C functions ────────────────────────────────────────────

/// `CFArrayRef CFArrayCreate(CFAllocatorRef, const void **values, CFIndex numValues, const CFArrayCallBacks *)`
pub extern "C" fn shim_cf_array_create(
    _alloc: u64,
    values: *const u64,
    num_values: i64,
    _callbacks: u64,
) -> u64 {
    if num_values <= 0 || values.is_null() {
        return crate::nsarray::make_nsarray(&[]);
    }
    let items =
        unsafe { std::slice::from_raw_parts(values, num_values as usize) };
    crate::nsarray::make_nsarray(items)
}

/// `CFIndex CFArrayGetCount(CFArrayRef theArray)`
pub extern "C" fn shim_cf_array_get_count(array: u64) -> i64 {
    if array == 0 {
        return 0;
    }
    unsafe { *((array as usize + 8) as *const u64) as i64 }
}

/// `const void *CFArrayGetValueAtIndex(CFArrayRef theArray, CFIndex idx)`
pub extern "C" fn shim_cf_array_get_value_at_index(array: u64, idx: i64) -> u64 {
    if array == 0 {
        return 0;
    }
    let count = unsafe { *((array as usize + 8) as *const u64) };
    if idx < 0 || idx as u64 >= count {
        return 0;
    }
    let elems = unsafe { *((array as usize + 16) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + idx as usize * 8) as *const u64) }
}

// ── CFDictionary C functions ───────────────────────────────────────

/// `CFDictionaryRef CFDictionaryCreate(CFAllocatorRef, const void **keys, const void **values, CFIndex numValues, ...)`
pub extern "C" fn shim_cf_dictionary_create(
    _alloc: u64,
    keys: *const u64,
    values: *const u64,
    num_values: i64,
    _key_cbs: u64,
    _val_cbs: u64,
) -> u64 {
    if num_values <= 0 || keys.is_null() || values.is_null() {
        return crate::nsdictionary::make_nsdictionary(&[], &[]);
    }
    let n = num_values as usize;
    let k_slice = unsafe { std::slice::from_raw_parts(keys, n) };
    let v_slice = unsafe { std::slice::from_raw_parts(values, n) };
    crate::nsdictionary::make_nsdictionary(k_slice, v_slice)
}

/// `CFIndex CFDictionaryGetCount(CFDictionaryRef theDict)`
pub extern "C" fn shim_cf_dictionary_get_count(dict: u64) -> i64 {
    if dict == 0 {
        return 0;
    }
    unsafe { *((dict as usize + 8) as *const u64) as i64 }
}

/// `const void *CFDictionaryGetValue(CFDictionaryRef theDict, const void *key)`
pub extern "C" fn shim_cf_dictionary_get_value(dict: u64, key: u64) -> u64 {
    if dict == 0 || key == 0 {
        return 0;
    }
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

// ── CFData C functions ─────────────────────────────────────────────

/// `CFDataRef CFDataCreate(CFAllocatorRef, const UInt8 *bytes, CFIndex length)`
pub extern "C" fn shim_cf_data_create(
    _alloc: u64,
    bytes: *const u8,
    length: i64,
) -> u64 {
    if length <= 0 || bytes.is_null() {
        return crate::nsdata::make_nsdata(&[]);
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, length as usize) };
    crate::nsdata::make_nsdata(slice)
}

/// `const UInt8 *CFDataGetBytePtr(CFDataRef theData)`
pub extern "C" fn shim_cf_data_get_byte_ptr(data: u64) -> u64 {
    if data == 0 {
        return 0;
    }
    unsafe { *((data as usize + 16) as *const u64) }
}

/// `CFIndex CFDataGetLength(CFDataRef theData)`
pub extern "C" fn shim_cf_data_get_length(data: u64) -> i64 {
    if data == 0 {
        return 0;
    }
    unsafe { *((data as usize + 8) as *const u64) as i64 }
}

// ── CFURL C functions ──────────────────────────────────────────────

/// `CFURLRef CFURLCreateWithString(CFAllocatorRef, CFStringRef urlString, CFURLRef baseURL)`
pub extern "C" fn shim_cf_url_create_with_string(
    _alloc: u64,
    url_string: u64,
    _base_url: u64,
) -> u64 {
    if url_string == 0 {
        return 0;
    }
    // Delegate to NSURL's URLWithString: — same object, toll-free
    crate::nsurl::shim_nsurl_url_with_string(nsobj::nsurl_class_addr(), 0, url_string)
}

/// `CFStringRef CFURLGetString(CFURLRef anURL)`
pub extern "C" fn shim_cf_url_get_string(url: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    // NSURL string is at offset 8
    unsafe { *((url as usize + 8) as *const u64) }
}

/// `CFURLRef CFURLCreateWithFileSystemPath(CFAllocatorRef, CFStringRef path, CFURLPathStyle, Boolean isDirectory)`
pub extern "C" fn shim_cf_url_create_with_file_system_path(
    _alloc: u64,
    path: u64,
    _path_style: i64,
    _is_dir: u8,
) -> u64 {
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, path)
}

// ── CFBundle C functions ───────────────────────────────────────────

/// `CFBundleRef CFBundleGetMainBundle(void)`
pub extern "C" fn shim_cf_bundle_get_main_bundle() -> u64 {
    crate::nsbundle::shim_nsbundle_main_bundle(nsobj::nsbundle_class_addr(), 0)
}

/// `CFURLRef CFBundleCopyBundleURL(CFBundleRef bundle)`
pub extern "C" fn shim_cf_bundle_copy_bundle_url(bundle: u64) -> u64 {
    if bundle == 0 {
        return 0;
    }
    let path = crate::nsbundle::shim_nsbundle_bundle_path(bundle, 0);
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, path)
}

// ── kCFBooleanTrue/False data symbols ──────────────────────────────

use std::sync::Once;

/// Boolean singletons — real NSNumber objects with ISA + tag(2=bool) + value.
/// Layout: [isa: u64, tag: u64, value: u64] = 24 bytes.
static mut K_CF_BOOLEAN_TRUE_OBJ: [u64; 3] = [0, 2, 1];
static mut K_CF_BOOLEAN_FALSE_OBJ: [u64; 3] = [0, 2, 0];
static BOOL_INIT: Once = Once::new();

fn ensure_bool_singletons() {
    BOOL_INIT.call_once(|| {
        let isa = nsobj::nsnumber_class_addr();
        unsafe {
            K_CF_BOOLEAN_TRUE_OBJ[0] = isa;
            K_CF_BOOLEAN_FALSE_OBJ[0] = isa;
        }
    });
}

/// Address of kCFBooleanTrue — a real NSNumber(bool=1) object.
pub fn kcf_boolean_true_addr() -> u64 {
    ensure_bool_singletons();
    unsafe { K_CF_BOOLEAN_TRUE_OBJ.as_ptr() as u64 }
}

/// Address of kCFBooleanFalse — a real NSNumber(bool=0) object.
pub fn kcf_boolean_false_addr() -> u64 {
    ensure_bool_singletons();
    unsafe { K_CF_BOOLEAN_FALSE_OBJ.as_ptr() as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_string_type_id() {
        let cstr = std::ffi::CString::new("hello").unwrap();
        let s = cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
        assert_ne!(s, 0);
        assert_eq!(shim_cf_get_type_id(s), CF_STRING_TYPE_ID);
        assert_eq!(shim_cf_string_get_type_id(), CF_STRING_TYPE_ID);
    }

    #[test]
    fn cf_retain_release_roundtrip() {
        let cstr = std::ffi::CString::new("test").unwrap();
        let s = cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
        assert_ne!(s, 0);
        let r = shim_cf_retain(s);
        assert_eq!(r, s);
        // Release once (back to implicit 1)
        crate::cfstring::shim_cf_release(s);
    }

    #[test]
    fn cf_equal_strings() {
        let a_cstr = std::ffi::CString::new("abc").unwrap();
        let b_cstr = std::ffi::CString::new("abc").unwrap();
        let c_cstr = std::ffi::CString::new("xyz").unwrap();
        let a = cfstring::shim_cf_string_create_with_cstring(0, a_cstr.as_ptr(), 0);
        let b = cfstring::shim_cf_string_create_with_cstring(0, b_cstr.as_ptr(), 0);
        let c = cfstring::shim_cf_string_create_with_cstring(0, c_cstr.as_ptr(), 0);
        assert_eq!(shim_cf_equal(a, b), 1);
        assert_eq!(shim_cf_equal(a, c), 0);
        assert_eq!(shim_cf_equal(a, a), 1);
        assert_eq!(shim_cf_equal(a, 0), 0);
    }

    #[test]
    fn cf_hash_deterministic() {
        let cstr = std::ffi::CString::new("hash_test").unwrap();
        let s = cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
        let h1 = shim_cf_hash(s);
        let h2 = shim_cf_hash(s);
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn cf_copy_description_string() {
        let cstr = std::ffi::CString::new("described").unwrap();
        let s = cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
        let desc = shim_cf_copy_description(s);
        assert_ne!(desc, 0);
        let text = cfstring::extract_cfstring_cstr(desc).unwrap();
        assert_eq!(text, "described");
    }

    #[test]
    fn cf_array_create_and_access() {
        let a_cstr = std::ffi::CString::new("one").unwrap();
        let b_cstr = std::ffi::CString::new("two").unwrap();
        let a = cfstring::shim_cf_string_create_with_cstring(0, a_cstr.as_ptr(), 0);
        let b = cfstring::shim_cf_string_create_with_cstring(0, b_cstr.as_ptr(), 0);
        let items = [a, b];
        let arr = shim_cf_array_create(0, items.as_ptr(), 2, 0);
        assert_ne!(arr, 0);
        assert_eq!(shim_cf_array_get_count(arr), 2);
        assert_eq!(shim_cf_array_get_value_at_index(arr, 0), a);
        assert_eq!(shim_cf_array_get_value_at_index(arr, 1), b);
        assert_eq!(shim_cf_get_type_id(arr), CF_ARRAY_TYPE_ID);
    }

    #[test]
    fn cf_dictionary_create_and_lookup() {
        let k_cstr = std::ffi::CString::new("key").unwrap();
        let v_cstr = std::ffi::CString::new("val").unwrap();
        let k = cfstring::shim_cf_string_create_with_cstring(0, k_cstr.as_ptr(), 0);
        let v = cfstring::shim_cf_string_create_with_cstring(0, v_cstr.as_ptr(), 0);
        let keys = [k];
        let vals = [v];
        let dict = shim_cf_dictionary_create(0, keys.as_ptr(), vals.as_ptr(), 1, 0, 0);
        assert_ne!(dict, 0);
        assert_eq!(shim_cf_dictionary_get_count(dict), 1);
        // Lookup by same pointer
        assert_eq!(shim_cf_dictionary_get_value(dict, k), v);
        assert_eq!(shim_cf_get_type_id(dict), CF_DICTIONARY_TYPE_ID);
    }

    #[test]
    fn cf_data_create_and_access() {
        let bytes: &[u8] = b"hello data";
        let data = shim_cf_data_create(0, bytes.as_ptr(), bytes.len() as i64);
        assert_ne!(data, 0);
        assert_eq!(shim_cf_data_get_length(data), bytes.len() as i64);
        let ptr = shim_cf_data_get_byte_ptr(data);
        assert_ne!(ptr, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, bytes.len()) };
        assert_eq!(slice, bytes);
        assert_eq!(shim_cf_get_type_id(data), CF_DATA_TYPE_ID);
    }

    #[test]
    fn cf_url_create_and_get_string() {
        let cstr = std::ffi::CString::new("https://example.com/path").unwrap();
        let s = cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
        let url = shim_cf_url_create_with_string(0, s, 0);
        assert_ne!(url, 0);
        let gs = shim_cf_url_get_string(url);
        assert_ne!(gs, 0);
        let text = cfstring::extract_cfstring_cstr(gs).unwrap();
        assert_eq!(text, "https://example.com/path");
        assert_eq!(shim_cf_get_type_id(url), CF_URL_TYPE_ID);
    }

    #[test]
    fn cf_bundle_main_bundle() {
        let bundle = shim_cf_bundle_get_main_bundle();
        assert_ne!(bundle, 0);
        assert_eq!(shim_cf_get_type_id(bundle), CF_BUNDLE_TYPE_ID);
        let url = shim_cf_bundle_copy_bundle_url(bundle);
        assert_ne!(url, 0);
        assert_eq!(shim_cf_get_type_id(url), CF_URL_TYPE_ID);
    }

    #[test]
    fn cf_null_and_boolean_type_ids() {
        assert_eq!(shim_cf_boolean_get_type_id(), CF_BOOLEAN_TYPE_ID);
        assert_eq!(shim_cf_null_get_type_id(), CF_NULL_TYPE_ID);
        assert_eq!(shim_cf_number_get_type_id(), CF_NUMBER_TYPE_ID);
    }
}
