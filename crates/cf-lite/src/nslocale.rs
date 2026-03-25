//! NSLocale shim: locale information singleton.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  identifier ptr (pointer to heap-allocated C string)
//!   [16..24): reserved
//!
//! Provides `+[NSLocale currentLocale]` as a singleton using the system
//! locale from the `LANG` environment variable, falling back to `en_US`.

use std::sync::OnceLock;

use crate::cfstring;
use crate::nsobj;

const NSLOCALE_SIZE: usize = 24;

static CURRENT_LOCALE: OnceLock<u64> = OnceLock::new();

/// Get the system locale identifier from the environment.
fn system_locale_identifier() -> String {
    std::env::var("LANG")
        .unwrap_or_else(|_| "en_US".to_string())
        .split('.')
        .next()
        .unwrap_or("en_US")
        .to_string()
}

fn alloc_nslocale(identifier: &str) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSLOCALE_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    let c_id = std::ffi::CString::new(identifier).unwrap_or_default();
    let id_ptr = unsafe {
        let len = c_id.as_bytes_with_nul().len();
        let buf = crate::pal_alloc().malloc(len);
        if !buf.is_null() {
            std::ptr::copy_nonoverlapping(c_id.as_ptr() as *const u8, buf, len);
        }
        buf
    };
    unsafe {
        *obj = nsobj::nslocale_class_addr();
        *obj.add(1) = id_ptr as u64;
    }
    obj as u64
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSLocale currentLocale]` — returns the singleton current locale.
pub extern "C" fn shim_nslocale_current_locale(_cls: u64, _cmd: u64) -> u64 {
    *CURRENT_LOCALE.get_or_init(|| {
        let id = system_locale_identifier();
        alloc_nslocale(&id)
    })
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSLocale localeIdentifier]` — returns the locale identifier as NSString.
pub extern "C" fn shim_nslocale_locale_identifier(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if id_ptr.is_null() {
        return 0;
    }
    cfstring::shim_cf_string_create_with_cstring(0, id_ptr, 0)
}

/// `-[NSLocale languageCode]` — returns the language part (before underscore).
pub extern "C" fn shim_nslocale_language_code(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if id_ptr.is_null() {
        return 0;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(id_ptr) };
    let id_str = cstr.to_str().unwrap_or("en");
    let lang = id_str.split('_').next().unwrap_or("en");
    let c_str = std::ffi::CString::new(lang).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSLocale countryCode]` — returns the country part (after underscore).
pub extern "C" fn shim_nslocale_country_code(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    if id_ptr.is_null() {
        return 0;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(id_ptr) };
    let id_str = cstr.to_str().unwrap_or("en_US");
    let country = id_str.split('_').nth(1).unwrap_or("");
    if country.is_empty() {
        return 0; // nil — no country component
    }
    let c_str = std::ffi::CString::new(country).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSLocale objectForKey:]` — returns locale component by key.
///
/// Handles: NSLocaleLanguageCode, NSLocaleCountryCode, NSLocaleIdentifier.
pub extern "C" fn shim_nslocale_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    let Some(key_str) = cfstring::extract_cfstring_cstr(key) else {
        return 0;
    };
    match key_str {
        "NSLocaleLanguageCode" | "kCFLocaleLanguageCodeKey" => {
            shim_nslocale_language_code(obj, _cmd)
        }
        "NSLocaleCountryCode" | "kCFLocaleCountryCodeKey" => shim_nslocale_country_code(obj, _cmd),
        "NSLocaleIdentifier" | "kCFLocaleIdentifierKey" => {
            shim_nslocale_locale_identifier(obj, _cmd)
        }
        _ => 0, // unknown key → nil
    }
}

/// `-[NSLocale displayNameForKey:value:]` — returns a display name; stub returns value itself.
pub extern "C" fn shim_nslocale_display_name_for_key_value(
    obj: u64,
    _cmd: u64,
    _key: u64,
    value: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Stub: return the value NSString as-is
    value
}

/// `-[NSLocale description]` — returns a description string.
pub extern "C" fn shim_nslocale_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const i8;
    let id_str = if !id_ptr.is_null() {
        let cstr = unsafe { std::ffi::CStr::from_ptr(id_ptr) };
        cstr.to_str().unwrap_or("?")
    } else {
        "?"
    };
    let desc = format!("<NSLocale: {id_str}>");
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}
