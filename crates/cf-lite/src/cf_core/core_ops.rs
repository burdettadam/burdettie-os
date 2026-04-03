//! CFTypeID constants and core CF operations.

use crate::cfstring;
use crate::nsobj;

// ── CFTypeID constants ──────────────────────────────────────────────

pub(crate) const CF_STRING_TYPE_ID: u64 = 7;
pub(crate) const CF_ARRAY_TYPE_ID: u64 = 19;
pub(crate) const CF_DICTIONARY_TYPE_ID: u64 = 18;
pub(crate) const CF_DATA_TYPE_ID: u64 = 20;
pub(crate) const CF_URL_TYPE_ID: u64 = 29;
pub(crate) const CF_BUNDLE_TYPE_ID: u64 = 47;
pub(crate) const CF_BOOLEAN_TYPE_ID: u64 = 21;
pub(crate) const CF_NUMBER_TYPE_ID: u64 = 22;
pub(crate) const CF_NULL_TYPE_ID: u64 = 16;
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

fn normalize_isa(isa: u64) -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        isa & 0x007f_ffff_ffff_fff8
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        isa
    }
}

fn alloc_user_defaults() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsuserdefaults_class_addr();
        }
    }
    obj
}

fn preferences_defaults_for_app(app: u64) -> u64 {
    if app == 0 {
        return crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0);
    }

    let Some(domain) = cfstring::extract_cfstring_cstr(app) else {
        return crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0);
    };

    if domain.is_empty() || domain == "NSGlobalDomain" {
        return crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0);
    }

    let defaults = alloc_user_defaults();
    if defaults == 0 {
        return 0;
    }

    let suite_defaults = crate::nsuserdefaults::shim_nsuserdefaults_init_with_suite_name(defaults, 0, app);
    if suite_defaults != 0 {
        suite_defaults
    } else {
        crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0)
    }
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
    let type_id = type_id_for_isa(isa);
    if type_id != CF_UNKNOWN_TYPE_ID {
        return type_id;
    }

    let normalized_isa = normalize_isa(isa);
    let normalized_type_id = type_id_for_isa(normalized_isa);
    if normalized_type_id != CF_UNKNOWN_TYPE_ID {
        return normalized_type_id;
    }

    CF_UNKNOWN_TYPE_ID
}

/// `Boolean __NSIsNSString(id obj)`
pub extern "C" fn shim_ns_is_nsstring(obj: u64) -> u8 {
    if obj == 0 {
        return 0;
    }

    u8::from(shim_cf_get_type_id(obj) == CF_STRING_TYPE_ID)
}

/// `CFPropertyListRef CFPreferencesCopyValue(CFStringRef key, CFStringRef applicationID,
///                                           CFStringRef userName, CFStringRef hostName)`
pub extern "C" fn shim_cf_preferences_copy_value(key: u64, app: u64, user: u64, host: u64) -> u64 {
    if key == 0 {
        return 0;
    }

    let defaults = preferences_defaults_for_app(app);
    if defaults == 0 {
        return 0;
    }

    let value = crate::nsuserdefaults::shim_nsuserdefaults_object_for_key(defaults, 0, key);
    if value != 0 {
        return value;
    }

    if app != 0 {
        return crate::nsuserdefaults::shim_nsuserdefaults_object_for_key(
            crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0),
            0,
            key,
        );
    }

    let _ = user;
    let _ = host;
    0
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

// ── CFString encoding conversions ──────────────────────────────────

/// `NSStringEncoding CFStringConvertEncodingToNSStringEncoding(CFStringEncoding enc)`
///
/// Maps CF string encoding constants to their NSStringEncoding equivalents.
/// The mapping is a 1:1 identity for the common encodings that genstrings uses.
pub extern "C" fn shim_cf_string_convert_encoding_to_nsstring_encoding(enc: u32) -> u64 {
    // Common CFStringEncoding → NSStringEncoding mappings:
    //   kCFStringEncodingMacRoman     = 0      → NSMacOSRomanStringEncoding  = 30
    //   kCFStringEncodingWindowsLatin1= 0x0500 → NSWindowsCP1252             = 12
    //   kCFStringEncodingISOLatin1    = 0x0201 → NSISOLatin1StringEncoding   = 5
    //   kCFStringEncodingUTF8         = 0x0800_0100 → NSUTF8StringEncoding   = 4
    //   kCFStringEncodingUTF16        = 0x0100 → NSUTF16StringEncoding       = 10
    //   kCFStringEncodingUTF16BE      = 0x1000_0100 → NSUTF16BigEndian       = 0x9000_0100
    //   kCFStringEncodingUTF16LE      = 0x1400_0100 → NSUTF16LittleEndian    = 0x9400_0100
    //   kCFStringEncodingASCII        = 0x0600 → NSASCIIStringEncoding       = 1
    match enc {
        0x0800_0100 => 4,           // UTF-8
        0 => 30,                     // MacRoman
        0x0500 => 12,               // WindowsLatin1/CP1252
        0x0201 => 5,                // ISOLatin1
        0x0100 => 10,               // UTF-16
        0x1000_0100 => 0x9000_0100, // UTF-16BE
        0x1400_0100 => 0x9400_0100, // UTF-16LE
        0x0600 => 1,                // ASCII
        0x0C00_0100 => 0x8C00_0100, // UTF-32
        0xFFFF_FFFF => 0,           // kCFStringEncodingInvalidId → 0 (not found)
        other => other as u64,       // pass-through for unknown
    }
}

/// `CFStringEncoding CFStringConvertIANACharSetNameToEncoding(CFStringRef name)`
///
/// Converts an IANA charset name (e.g. "utf-8") to a CFStringEncoding constant.
pub extern "C" fn shim_cf_string_convert_iana_charset_name_to_encoding(name: u64) -> u32 {
    if name == 0 {
        return 0xFFFF_FFFF; // kCFStringEncodingInvalidId
    }
    let s = match cfstring::extract_cfstring_cstr(name) {
        Some(s) => s.to_lowercase(),
        None => return 0xFFFF_FFFF,
    };
    match s.as_str() {
        "utf-8" | "utf8" => 0x0800_0100,
        "us-ascii" | "ascii" | "iso-ir-6" => 0x0600,
        "iso-8859-1" | "latin1" | "iso_8859-1" => 0x0201,
        "utf-16" | "utf16" => 0x0100,
        "utf-16be" => 0x1000_0100,
        "utf-16le" => 0x1400_0100,
        "utf-32" | "utf32" => 0x0C00_0100,
        "windows-1252" | "cp1252" => 0x0500,
        "macintosh" | "mac" | "x-mac-roman" => 0,
        "shift_jis" | "shift-jis" => 0x0A01,
        "euc-jp" => 0x0920,
        "iso-2022-jp" => 0x0820,
        "euc-kr" => 0x0940,
        "big5" => 0x0A03,
        "gb2312" | "gb_2312-80" => 0x0A05,
        _ => 0xFFFF_FFFF,
    }
}

// ── ___NSArray0__struct sentinel ───────────────────────────────────

/// Static sentinel for `___NSArray0__struct` — the empty immutable array singleton.
///
/// Layout: [ISA = nsarray_class_addr()][count = 0][items = NULL][capacity = 0]
/// We use OnceLock to lazily allocate so the ISA can be resolved after class setup.
static NSARRAY0_DATA: std::sync::OnceLock<[u64; 4]> = std::sync::OnceLock::new();

/// Returns the address of the `___NSArray0__struct` singleton.
pub fn nsarray0_struct_addr() -> u64 {
    let data = NSARRAY0_DATA.get_or_init(|| {
        [nsobj::nsarray_class_addr(), 0, 0, 0]
    });
    data.as_ptr() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn type_id_constants_distinct() {
        assert_ne!(shim_cf_string_get_type_id(), shim_cf_array_get_type_id());
        assert_ne!(shim_cf_array_get_type_id(), shim_cf_dictionary_get_type_id());
        assert_ne!(shim_cf_data_get_type_id(), shim_cf_null_get_type_id());
        assert_ne!(shim_cf_boolean_get_type_id(), shim_cf_number_get_type_id());
    }

    #[test]
    fn get_type_id_for_string() {
        let s = make_string("test");
        assert_eq!(shim_cf_get_type_id(s), shim_cf_string_get_type_id());
        free_obj(s);
    }

    #[test]
    fn get_type_id_nil_returns_unknown() {
        assert_eq!(shim_cf_get_type_id(0), CF_UNKNOWN_TYPE_ID);
    }

    #[test]
    fn ns_is_nsstring_accepts_cfstring() {
        let s = make_string("hello");
        assert_eq!(shim_ns_is_nsstring(s), 1);
        free_obj(s);
    }

    #[test]
    fn ns_is_nsstring_rejects_non_string() {
        let obj = alloc_obj(8, nsobj::nsnumber_class_addr());
        assert_eq!(shim_ns_is_nsstring(obj), 0);
        free_obj(obj);
        assert_eq!(shim_ns_is_nsstring(0), 0);
    }

    #[test]
    fn cf_equal_identical_strings() {
        let a = make_string("hello");
        let b = make_string("hello");
        assert_eq!(shim_cf_equal(a, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn cf_equal_different_strings() {
        let a = make_string("foo");
        let b = make_string("bar");
        assert_eq!(shim_cf_equal(a, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn cf_equal_nil_safety() {
        assert_eq!(shim_cf_equal(0, 0), 1);
        let s = make_string("x");
        assert_eq!(shim_cf_equal(0, s), 0);
        assert_eq!(shim_cf_equal(s, 0), 0);
        free_obj(s);
    }

    #[test]
    fn cf_hash_string_consistent() {
        let a = make_string("hello");
        let b = make_string("hello");
        assert_eq!(shim_cf_hash(a), shim_cf_hash(b));
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn cf_hash_nil_is_zero() {
        assert_eq!(shim_cf_hash(0), 0);
    }

    #[test]
    fn cf_preferences_copy_value_nil_key_returns_zero() {
        assert_eq!(shim_cf_preferences_copy_value(0, 0, 0, 0), 0);
    }

    #[test]
    fn cf_preferences_copy_value_reads_standard_defaults() {
        let defaults = crate::nsuserdefaults::shim_nsuserdefaults_standard(0, 0);
        let key = make_string("Theme");
        let value = make_string("Dark");

        crate::nsuserdefaults::shim_nsuserdefaults_set_object(defaults, 0, value, key);

        let result = shim_cf_preferences_copy_value(key, 0, 0, 0);
        assert_eq!(result, value);
    }

    #[test]
    fn cf_preferences_copy_value_reads_suite_defaults() {
        let suite_name = make_string("com.example.testsuite");
        let defaults = alloc_user_defaults();
        assert_ne!(defaults, 0);
        let defaults = crate::nsuserdefaults::shim_nsuserdefaults_init_with_suite_name(defaults, 0, suite_name);
        assert_ne!(defaults, 0);

        let key = make_string("Greeting");
        let value = make_string("Hello");
        crate::nsuserdefaults::shim_nsuserdefaults_set_object(defaults, 0, value, key);

        let result = shim_cf_preferences_copy_value(key, suite_name, 0, 0);
        assert_eq!(result, value);
    }

    #[test]
    fn cf_copy_description_nil() {
        let desc = shim_cf_copy_description(0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "(null)");
        free_obj(desc);
    }

    #[test]
    fn cf_retain_passthrough() {
        let s = make_string("retain_me");
        let r = shim_cf_retain(s);
        assert_eq!(r, s);
        free_obj(s);
    }

    // ── CF encoding conversion tests ──────────────────────────────

    #[test]
    fn convert_encoding_utf8() {
        assert_eq!(shim_cf_string_convert_encoding_to_nsstring_encoding(0x0800_0100), 4);
    }

    #[test]
    fn convert_encoding_ascii() {
        assert_eq!(shim_cf_string_convert_encoding_to_nsstring_encoding(0x0600), 1);
    }

    #[test]
    fn convert_encoding_invalid() {
        assert_eq!(shim_cf_string_convert_encoding_to_nsstring_encoding(0xFFFF_FFFF), 0);
    }

    #[test]
    fn iana_charset_utf8() {
        let name = make_string("utf-8");
        assert_eq!(shim_cf_string_convert_iana_charset_name_to_encoding(name), 0x0800_0100);
        free_obj(name);
    }

    #[test]
    fn iana_charset_ascii() {
        let name = make_string("us-ascii");
        assert_eq!(shim_cf_string_convert_iana_charset_name_to_encoding(name), 0x0600);
        free_obj(name);
    }

    #[test]
    fn iana_charset_unknown() {
        let name = make_string("totally-unknown");
        assert_eq!(shim_cf_string_convert_iana_charset_name_to_encoding(name), 0xFFFF_FFFF);
        free_obj(name);
    }

    #[test]
    fn iana_charset_nil() {
        assert_eq!(shim_cf_string_convert_iana_charset_name_to_encoding(0), 0xFFFF_FFFF);
    }

    #[test]
    fn nsarray0_struct_is_stable() {
        let a = nsarray0_struct_addr();
        let b = nsarray0_struct_addr();
        assert_eq!(a, b);
        assert_ne!(a, 0);
        // count should be 0
        let count = unsafe { *((a as usize + 8) as *const u64) };
        assert_eq!(count, 0);
    }
}

