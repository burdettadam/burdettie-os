//! NSLocale shim: locale information singleton.
//!
//! Object layout: 24 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  identifier ptr (pointer to heap-allocated C string)
//!   [16..24): reserved
//!
//! Provides `+[NSLocale currentLocale]` as a singleton using the system
//! locale from the `LANG` environment variable, falling back to `en_US`.

use std::ffi::c_char;
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

/// `+[NSLocale autoupdatingCurrentLocale]` — returns the current locale singleton.
pub extern "C" fn shim_nslocale_autoupdating_current_locale(_cls: u64, _cmd: u64) -> u64 {
    shim_nslocale_current_locale(_cls, _cmd)
}

/// `+[NSLocale systemLocale]` — returns the system locale (same as current for now).
pub extern "C" fn shim_nslocale_system_locale(_cls: u64, _cmd: u64) -> u64 {
    shim_nslocale_current_locale(_cls, _cmd)
}

/// `+[NSLocale preferredLanguages]` — returns NSArray of preferred language codes.
pub extern "C" fn shim_nslocale_preferred_languages(_cls: u64, _cmd: u64) -> u64 {
    let lang = system_locale_identifier();
    let code = lang.split('_').next().unwrap_or("en");
    let c_str = std::ffi::CString::new(code).unwrap_or_default();
    let lang_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    crate::nsarray::make_nsarray(&[lang_str])
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSLocale localeIdentifier]` — returns the locale identifier as NSString.
pub extern "C" fn shim_nslocale_locale_identifier(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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

/// `-[NSLocale displayNameForKey:value:]` — returns a localized display name.
///
/// Supports `NSLocaleIdentifier`, `NSLocaleLanguageCode`, and `NSLocaleCountryCode`
/// keys with a built-in table of common display names.
pub extern "C" fn shim_nslocale_display_name_for_key_value(
    obj: u64,
    _cmd: u64,
    key: u64,
    value: u64,
) -> u64 {
    if obj == 0 || key == 0 || value == 0 {
        return 0;
    }
    let Some(key_str) = cfstring::extract_cfstring_cstr(key) else {
        return value;
    };
    let Some(val_str) = cfstring::extract_cfstring_cstr(value) else {
        return value;
    };

    let display = match key_str {
        "NSLocaleIdentifier" | "kCFLocaleIdentifierKey" => locale_display_name(val_str),
        "NSLocaleLanguageCode" | "kCFLocaleLanguageCodeKey" => {
            language_display_name(val_str).map(String::from)
        }
        "NSLocaleCountryCode" | "kCFLocaleCountryCodeKey" => {
            country_display_name(val_str).map(String::from)
        }
        _ => None,
    };

    match display {
        Some(name) => {
            let c_str = std::ffi::CString::new(name).unwrap_or_default();
            cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
        }
        None => value, // fall back to returning the value string as-is
    }
}

fn language_display_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "en" => "English",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ru" => "Russian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "nl" => "Dutch",
        "sv" => "Swedish",
        "da" => "Danish",
        "fi" => "Finnish",
        "nb" => "Norwegian Bokmål",
        "pl" => "Polish",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "cs" => "Czech",
        "el" => "Greek",
        "he" => "Hebrew",
        "th" => "Thai",
        "id" => "Indonesian",
        "ms" => "Malay",
        "vi" => "Vietnamese",
        "ro" => "Romanian",
        "hu" => "Hungarian",
        "ca" => "Catalan",
        "hr" => "Croatian",
        "sk" => "Slovak",
        "bg" => "Bulgarian",
        _ => return None,
    })
}

fn country_display_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "US" => "United States",
        "GB" => "United Kingdom",
        "CA" => "Canada",
        "AU" => "Australia",
        "NZ" => "New Zealand",
        "IE" => "Ireland",
        "DE" => "Germany",
        "FR" => "France",
        "ES" => "Spain",
        "IT" => "Italy",
        "PT" => "Portugal",
        "BR" => "Brazil",
        "MX" => "Mexico",
        "AR" => "Argentina",
        "CO" => "Colombia",
        "JP" => "Japan",
        "CN" => "China",
        "TW" => "Taiwan",
        "KR" => "South Korea",
        "IN" => "India",
        "RU" => "Russia",
        "NL" => "Netherlands",
        "SE" => "Sweden",
        "NO" => "Norway",
        "DK" => "Denmark",
        "FI" => "Finland",
        "PL" => "Poland",
        "TR" => "Turkey",
        "SA" => "Saudi Arabia",
        "AE" => "United Arab Emirates",
        "IL" => "Israel",
        "TH" => "Thailand",
        "SG" => "Singapore",
        "CZ" | "CK" => "Czech Republic",
        "AT" => "Austria",
        "CH" => "Switzerland",
        "BE" => "Belgium",
        "ZA" => "South Africa",
        _ => return None,
    })
}

fn locale_display_name(identifier: &str) -> Option<String> {
    let parts: Vec<&str> = identifier.split('_').collect();
    let lang = language_display_name(parts.first()?)?;
    if let Some(country) = parts.get(1).and_then(|c| country_display_name(c)) {
        Some(format!("{lang} ({country})"))
    } else {
        Some(lang.to_string())
    }
}

/// `-[NSLocale description]` — returns a description string.
pub extern "C" fn shim_nslocale_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let id_ptr = unsafe { *((obj as *const u64).add(1)) } as *const c_char;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Helper: allocate an NSLocale with the given identifier for testing.
    fn make_locale(identifier: &str) -> u64 {
        alloc_nslocale(identifier)
    }

    // ── alloc_nslocale ─────────────────────────────────────────────

    #[test]
    fn alloc_nslocale_sets_isa_and_identifier() {
        let obj = make_locale("fr_FR");
        assert_ne!(obj, 0);
        // ISA should be the real NSLocale class address
        let isa = read_field(obj, 0);
        assert_eq!(isa, nsobj::nslocale_class_addr());
        // identifier pointer at offset 8 should be non-null
        let id_ptr = read_field(obj, 8);
        assert_ne!(id_ptr, 0);
        let cstr = unsafe { std::ffi::CStr::from_ptr(id_ptr as *const c_char) };
        assert_eq!(cstr.to_str().unwrap(), "fr_FR");
        // Clean up: free the identifier string, then the object
        crate::pal_alloc().free(id_ptr as *mut u8);
        free_obj(obj);
    }

    // ── shim_nslocale_current_locale ───────────────────────────────

    #[test]
    fn current_locale_returns_nonzero_singleton() {
        let loc1 = shim_nslocale_current_locale(0, 0);
        assert_ne!(loc1, 0);
        // Calling again returns the same singleton
        let loc2 = shim_nslocale_current_locale(0, 0);
        assert_eq!(loc1, loc2);
    }

    #[test]
    fn autoupdating_current_locale_matches_current_locale() {
        let current = shim_nslocale_current_locale(0, 0);
        let autoupdating = shim_nslocale_autoupdating_current_locale(0, 0);
        assert_ne!(autoupdating, 0);
        assert_eq!(autoupdating, current);
    }

    // ── shim_nslocale_locale_identifier ────────────────────────────

    #[test]
    fn locale_identifier_nil_returns_zero() {
        assert_eq!(shim_nslocale_locale_identifier(0, 0), 0);
    }

    #[test]
    fn locale_identifier_returns_full_id() {
        let obj = make_locale("en_GB");
        let result = shim_nslocale_locale_identifier(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "en_GB");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn locale_identifier_null_id_ptr_returns_zero() {
        let obj = alloc_obj(NSLOCALE_SIZE, nsobj::nslocale_class_addr());
        // offset 8 is already 0 (calloc), so id_ptr is null
        assert_eq!(shim_nslocale_locale_identifier(obj, 0), 0);
        free_obj(obj);
    }

    // ── shim_nslocale_language_code ────────────────────────────────

    #[test]
    fn language_code_nil_returns_zero() {
        assert_eq!(shim_nslocale_language_code(0, 0), 0);
    }

    #[test]
    fn language_code_extracts_before_underscore() {
        let obj = make_locale("ja_JP");
        let result = shim_nslocale_language_code(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "ja");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn language_code_no_underscore_returns_whole() {
        let obj = make_locale("de");
        let result = shim_nslocale_language_code(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "de");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn language_code_null_id_ptr_returns_zero() {
        let obj = alloc_obj(NSLOCALE_SIZE, nsobj::nslocale_class_addr());
        assert_eq!(shim_nslocale_language_code(obj, 0), 0);
        free_obj(obj);
    }

    // ── shim_nslocale_country_code ─────────────────────────────────

    #[test]
    fn country_code_nil_returns_zero() {
        assert_eq!(shim_nslocale_country_code(0, 0), 0);
    }

    #[test]
    fn country_code_extracts_after_underscore() {
        let obj = make_locale("pt_BR");
        let result = shim_nslocale_country_code(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "BR");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn country_code_no_underscore_returns_zero() {
        let obj = make_locale("zh");
        let result = shim_nslocale_country_code(obj, 0);
        assert_eq!(result, 0); // no country component → nil
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn country_code_null_id_ptr_returns_zero() {
        let obj = alloc_obj(NSLOCALE_SIZE, nsobj::nslocale_class_addr());
        assert_eq!(shim_nslocale_country_code(obj, 0), 0);
        free_obj(obj);
    }

    // ── shim_nslocale_object_for_key ───────────────────────────────

    #[test]
    fn object_for_key_nil_obj_returns_zero() {
        let key = make_string("NSLocaleLanguageCode");
        assert_eq!(shim_nslocale_object_for_key(0, 0, key), 0);
    }

    #[test]
    fn object_for_key_nil_key_returns_zero() {
        let obj = make_locale("en_US");
        assert_eq!(shim_nslocale_object_for_key(obj, 0, 0), 0);
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_language_code() {
        let obj = make_locale("ko_KR");
        let key = make_string("NSLocaleLanguageCode");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "ko");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_country_code() {
        let obj = make_locale("ko_KR");
        let key = make_string("NSLocaleCountryCode");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "KR");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_identifier() {
        let obj = make_locale("es_MX");
        let key = make_string("NSLocaleIdentifier");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "es_MX");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_cf_language_key() {
        let obj = make_locale("sv_SE");
        let key = make_string("kCFLocaleLanguageCodeKey");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "sv");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_cf_country_key() {
        let obj = make_locale("sv_SE");
        let key = make_string("kCFLocaleCountryCodeKey");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "SE");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_cf_identifier_key() {
        let obj = make_locale("sv_SE");
        let key = make_string("kCFLocaleIdentifierKey");
        let result = shim_nslocale_object_for_key(obj, 0, key);
        assert_ne!(result, 0);
        assert_string_eq(result, "sv_SE");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn object_for_key_unknown_returns_zero() {
        let obj = make_locale("en_US");
        let key = make_string("NSLocaleCalendar");
        assert_eq!(shim_nslocale_object_for_key(obj, 0, key), 0);
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    // ── shim_nslocale_display_name_for_key_value ───────────────────

    #[test]
    fn display_name_for_key_value_nil_returns_zero() {
        assert_eq!(shim_nslocale_display_name_for_key_value(0, 0, 0, 0), 0);
    }

    #[test]
    fn display_name_for_key_value_returns_value() {
        let obj = make_locale("en_US");
        let key = make_string("NSLocaleLanguageCode");
        let value = make_string("English");
        let result = shim_nslocale_display_name_for_key_value(obj, 0, key, value);
        // Stub returns value as-is
        assert_eq!(result, value);
        assert_string_eq(result, "English");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    // ── shim_nslocale_description ──────────────────────────────────

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_nslocale_description(0, 0), 0);
    }

    #[test]
    fn description_returns_formatted_string() {
        let obj = make_locale("it_IT");
        let result = shim_nslocale_description(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "<NSLocale: it_IT>");
        crate::pal_alloc().free(read_field(obj, 8) as *mut u8);
        free_obj(obj);
    }

    #[test]
    fn description_null_id_ptr_returns_question_mark() {
        let obj = alloc_obj(NSLOCALE_SIZE, nsobj::nslocale_class_addr());
        let result = shim_nslocale_description(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "<NSLocale: ?>");
        free_obj(obj);
    }

    // ── shim_nslocale_system_locale ────────────────────────────────

    #[test]
    fn system_locale_returns_nonzero() {
        let loc = shim_nslocale_system_locale(0, 0);
        assert_ne!(loc, 0);
    }

    #[test]
    fn system_locale_same_as_current() {
        let cur = shim_nslocale_current_locale(0, 0);
        let sys = shim_nslocale_system_locale(0, 0);
        assert_eq!(cur, sys);
    }

    // ── shim_nslocale_preferred_languages ──────────────────────────

    #[test]
    fn preferred_languages_returns_nonempty_array() {
        let arr = shim_nslocale_preferred_languages(0, 0);
        assert_ne!(arr, 0);
        // Should have at least one element
        let count = crate::nsarray::shim_nsarray_count(arr, 0);
        assert!(
            count >= 1,
            "preferredLanguages should return at least one language"
        );

        // First element should be a non-empty string
        let first = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_ne!(first, 0);
    }
}
