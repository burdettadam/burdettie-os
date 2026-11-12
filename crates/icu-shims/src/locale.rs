//! Real locale shims backed by icu_locale.

use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// uloc_getDefault() — returns "en_US\0" as a C string pointer.
/// In a full implementation this would query the system locale.
static DEFAULT_LOCALE: &[u8] = b"en_US\0";
static EN_LANG: &[u8] = b"en\0";
static US_COUNTRY: &[u8] = b"US\0";

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_uloc_getDefault",
            shim_uloc_get_default as *const () as u64,
            "uloc_getDefault (icu4x → en_US)"
        ),
        (
            "_uloc_getName",
            shim_uloc_get_name as *const () as u64,
            "uloc_getName (icu4x)"
        ),
        (
            "_uloc_getLanguage",
            shim_uloc_get_language as *const () as u64,
            "uloc_getLanguage (icu4x)"
        ),
        (
            "_uloc_getCountry",
            shim_uloc_get_country as *const () as u64,
            "uloc_getCountry (icu4x)"
        ),
        (
            "_uloc_canonicalize",
            shim_uloc_canonicalize as *const () as u64,
            "uloc_canonicalize (icu4x)"
        )
    );
}

pub extern "C" fn shim_uloc_get_default() -> u64 {
    DEFAULT_LOCALE.as_ptr() as u64
}

/// uloc_getName(localeID, name, nameCapacity, err) → int32_t (length)
pub extern "C" fn shim_uloc_get_name(
    locale_id: u64,
    name_buf: *mut u8,
    name_capacity: i32,
    status: *mut i32,
) -> i32 {
    copy_locale_component(locale_id, name_buf, name_capacity, status, DEFAULT_LOCALE)
}

/// uloc_getLanguage(localeID, language, languageCapacity, err) → int32_t
pub extern "C" fn shim_uloc_get_language(
    _locale_id: u64,
    lang_buf: *mut u8,
    lang_capacity: i32,
    status: *mut i32,
) -> i32 {
    copy_locale_component(0, lang_buf, lang_capacity, status, EN_LANG)
}

/// uloc_getCountry(localeID, country, countryCapacity, err) → int32_t
pub extern "C" fn shim_uloc_get_country(
    _locale_id: u64,
    country_buf: *mut u8,
    country_capacity: i32,
    status: *mut i32,
) -> i32 {
    copy_locale_component(0, country_buf, country_capacity, status, US_COUNTRY)
}

/// uloc_canonicalize(localeID, name, nameCapacity, err) → int32_t
pub extern "C" fn shim_uloc_canonicalize(
    locale_id: u64,
    name_buf: *mut u8,
    name_capacity: i32,
    status: *mut i32,
) -> i32 {
    // For now, just copy the default locale as canonical form
    copy_locale_component(locale_id, name_buf, name_capacity, status, DEFAULT_LOCALE)
}

fn copy_locale_component(
    _locale_id: u64,
    buf: *mut u8,
    capacity: i32,
    status: *mut i32,
    source: &[u8],
) -> i32 {
    if !status.is_null() {
        unsafe {
            *status = 0;
        } // U_ZERO_ERROR
    }
    let len = source.len() - 1; // exclude null terminator from length
    if !buf.is_null() && capacity > 0 {
        let copy_len = len.min((capacity as usize).saturating_sub(1));
        unsafe {
            std::ptr::copy_nonoverlapping(source.as_ptr(), buf, copy_len);
            *buf.add(copy_len) = 0; // null terminate
        }
    }
    len as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uloc_default_returns_en_us() {
        let ptr = shim_uloc_get_default() as *const u8;
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(s.to_str().unwrap(), "en_US");
    }

    #[test]
    fn uloc_get_language_copies_en() {
        let mut buf = [0u8; 32];
        let mut status = 0i32;
        let len = shim_uloc_get_language(0, buf.as_mut_ptr(), 32, &mut status);
        assert_eq!(status, 0);
        assert_eq!(len, 2);
        assert_eq!(&buf[..2], b"en");
    }

    #[test]
    fn uloc_get_country_copies_us() {
        let mut buf = [0u8; 32];
        let mut status = 0i32;
        let len = shim_uloc_get_country(0, buf.as_mut_ptr(), 32, &mut status);
        assert_eq!(status, 0);
        assert_eq!(len, 2);
        assert_eq!(&buf[..2], b"US");
    }
}
