//! Core CoreFoundation functions: CFRetain, CFGetTypeID, CFEqual, CFHash,
//! CFCopyDescription, and toll-free bridging infrastructure.
//!
//! Toll-free bridging is trivial in cf-lite: CF objects and their NS
//! counterparts are literally the same objects with the same ISA.
//! `(__bridge NSString *)cfStr` is a no-op pointer cast.

mod core_ops;
mod collections;
pub mod cf_extras;

pub use core_ops::*;
pub use collections::*;
pub use cf_extras::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfstring;

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
