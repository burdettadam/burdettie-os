//! NSURLCredential, NSURLCredentialStorage, NSHTTPCookie, NSHTTPCookieStorage stubs.
//!
//! NSURLCredential layout (32 bytes):
//!   offset  0: isa           (u64)
//!   offset  8: user_ptr      (u64 — NSString)
//!   offset 16: password_ptr  (u64 — NSString)
//!   offset 24: persistence   (u64 — NSURLCredentialPersistence enum)
//!
//! NSHTTPCookie layout (24 bytes):
//!   offset  0: isa            (u64)
//!   offset  8: properties_ptr (u64 — NSDictionary)
//!   offset 16: _reserved      (u64)
//!
//! NSURLCredentialStorage and NSHTTPCookieStorage are singleton stubs
//! with only an ISA pointer (8 bytes). They return empty/nil for all
//! queries and silently drop all writes.

use crate::cfstring;
use crate::nsobj;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ═══════════════════════════════════════════════════════════════
// NSURLCredential
// ═══════════════════════════════════════════════════════════════

const CRED_USER_OFF: usize = 8;
const CRED_PASS_OFF: usize = 16;
const CRED_PERSIST_OFF: usize = 24;

/// `+[NSURLCredential credentialWithUser:password:persistence:]`
pub extern "C" fn shim_nsurlcredential_with_user(
    _cls: u64,
    _cmd: u64,
    user: u64,
    password: u64,
    persistence: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlcredential_class_addr();
        *((obj as usize + CRED_USER_OFF) as *mut u64) = user;
        *((obj as usize + CRED_PASS_OFF) as *mut u64) = password;
        *((obj as usize + CRED_PERSIST_OFF) as *mut u64) = persistence;
    }
    obj
}

/// `-[NSURLCredential user]`
pub extern "C" fn shim_nsurlcredential_user(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CRED_USER_OFF) as *const u64) }
}

/// `-[NSURLCredential password]`
pub extern "C" fn shim_nsurlcredential_password(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CRED_PASS_OFF) as *const u64) }
}

/// `-[NSURLCredential persistence]`
pub extern "C" fn shim_nsurlcredential_persistence(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + CRED_PERSIST_OFF) as *const u64) }
}

/// `-[NSURLCredential hasPassword]`
pub extern "C" fn shim_nsurlcredential_has_password(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let pass = unsafe { *((obj as usize + CRED_PASS_OFF) as *const u64) };
    u8::from(pass != 0)
}

pub extern "C" fn shim_nsurlcredential_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("<NSURLCredential: nil>");
    }
    let user =
        cfstring::extract_cfstring_cstr(unsafe { *((obj as usize + CRED_USER_OFF) as *const u64) })
            .unwrap_or("(nil)");
    make_nsstring(&format!("<NSURLCredential: user={user}>"))
}

// ═══════════════════════════════════════════════════════════════
// NSURLCredentialStorage (singleton stub)
// ═══════════════════════════════════════════════════════════════

static SHARED_CREDENTIAL_STORAGE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

fn credential_storage_singleton() -> u64 {
    *SHARED_CREDENTIAL_STORAGE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nsurlcredentialstorage_class_addr();
            }
        }
        obj
    })
}

/// `+[NSURLCredentialStorage sharedCredentialStorage]`
pub extern "C" fn shim_nsurlcredentialstorage_shared(_cls: u64, _cmd: u64) -> u64 {
    credential_storage_singleton()
}

/// `-[NSURLCredentialStorage allCredentials]` → empty NSDictionary
pub extern "C" fn shim_nsurlcredentialstorage_all_credentials(_obj: u64, _cmd: u64) -> u64 {
    crate::nsmutabledict::shim_nsmutabledict_dictionary(
        crate::nsobj::nsmutabledictionary_class_addr(),
        0,
    )
}

/// `-[NSURLCredentialStorage defaultCredentialForProtectionSpace:]` → nil
pub extern "C" fn shim_nsurlcredentialstorage_default_credential(
    _obj: u64,
    _cmd: u64,
    _space: u64,
) -> u64 {
    0
}

/// `-[NSURLCredentialStorage setDefaultCredential:forProtectionSpace:]`
pub extern "C" fn shim_nsurlcredentialstorage_set_default_credential(
    _obj: u64,
    _cmd: u64,
    _cred: u64,
    _space: u64,
) {
    log::debug!("NSURLCredentialStorage setDefaultCredential:forProtectionSpace: (stub)");
}

/// `-[NSURLCredentialStorage removeCredential:forProtectionSpace:]`
pub extern "C" fn shim_nsurlcredentialstorage_remove_credential(
    _obj: u64,
    _cmd: u64,
    _cred: u64,
    _space: u64,
) {
    log::debug!("NSURLCredentialStorage removeCredential:forProtectionSpace: (stub)");
}

pub extern "C" fn shim_nsurlcredentialstorage_description(_obj: u64, _cmd: u64) -> u64 {
    make_nsstring("<NSURLCredentialStorage>")
}

// ═══════════════════════════════════════════════════════════════
// NSHTTPCookie
// ═══════════════════════════════════════════════════════════════

const COOKIE_PROPS_OFF: usize = 8;

/// `+[NSHTTPCookie cookieWithProperties:]`
pub extern "C" fn shim_nshttpcookie_with_properties(_cls: u64, _cmd: u64, props: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nshttpcookie_class_addr();
        *((obj as usize + COOKIE_PROPS_OFF) as *mut u64) = props;
    }
    obj
}

/// `-[NSHTTPCookie properties]`
pub extern "C" fn shim_nshttpcookie_properties(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) }
}

/// `-[NSHTTPCookie name]`
pub extern "C" fn shim_nshttpcookie_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("");
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return make_nsstring("");
    }
    let key = make_nsstring("Name");
    crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key)
}

/// `-[NSHTTPCookie value]`
pub extern "C" fn shim_nshttpcookie_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("");
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return make_nsstring("");
    }
    let key = make_nsstring("Value");
    crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key)
}

/// `-[NSHTTPCookie domain]`
pub extern "C" fn shim_nshttpcookie_domain(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("");
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return make_nsstring("");
    }
    let key = make_nsstring("Domain");
    crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key)
}

/// `-[NSHTTPCookie path]`
pub extern "C" fn shim_nshttpcookie_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("/");
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return make_nsstring("/");
    }
    let key = make_nsstring("Path");
    let v = crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key);
    if v == 0 {
        make_nsstring("/")
    } else {
        v
    }
}

/// `-[NSHTTPCookie isSecure]` — reads "Secure" key from property dict.
pub extern "C" fn shim_nshttpcookie_is_secure(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return 0;
    }
    let key = make_nsstring("Secure");
    let val = crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key);
    if val == 0 {
        return 0;
    }
    // Any non-nil value for the key means the cookie is secure
    let s = cfstring::extract_cfstring_cstr(val).unwrap_or("");
    u8::from(
        !s.eq_ignore_ascii_case("false")
            && !s.eq_ignore_ascii_case("no")
            && !s.eq_ignore_ascii_case("0"),
    )
}

/// `-[NSHTTPCookie isHTTPOnly]` — reads "HttpOnly" key from property dict.
pub extern "C" fn shim_nshttpcookie_is_http_only(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let props = unsafe { *((obj as usize + COOKIE_PROPS_OFF) as *const u64) };
    if props == 0 {
        return 0;
    }
    let key = make_nsstring("HttpOnly");
    let val = crate::nsdictionary::shim_nsdictionary_object_for_key(props, 0, key);
    if val == 0 {
        return 0;
    }
    let s = cfstring::extract_cfstring_cstr(val).unwrap_or("");
    u8::from(
        !s.eq_ignore_ascii_case("false")
            && !s.eq_ignore_ascii_case("no")
            && !s.eq_ignore_ascii_case("0"),
    )
}

/// `-[NSHTTPCookie expiresDate]` → nil
pub extern "C" fn shim_nshttpcookie_expires_date(_obj: u64, _cmd: u64) -> u64 {
    0
}

pub extern "C" fn shim_nshttpcookie_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return make_nsstring("<NSHTTPCookie: nil>");
    }
    let name = cfstring::extract_cfstring_cstr(shim_nshttpcookie_name(obj, 0)).unwrap_or("?");
    make_nsstring(&format!("<NSHTTPCookie: {name}>"))
}

// ═══════════════════════════════════════════════════════════════
// NSHTTPCookieStorage (singleton stub)
// ═══════════════════════════════════════════════════════════════

static SHARED_COOKIE_STORAGE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

fn cookie_storage_singleton() -> u64 {
    *SHARED_COOKIE_STORAGE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nshttpcookiestorage_class_addr();
            }
        }
        obj
    })
}

/// `+[NSHTTPCookieStorage sharedHTTPCookieStorage]`
pub extern "C" fn shim_nshttpcookiestorage_shared(_cls: u64, _cmd: u64) -> u64 {
    cookie_storage_singleton()
}

/// `-[NSHTTPCookieStorage cookies]` → empty NSArray
pub extern "C" fn shim_nshttpcookiestorage_cookies(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        0,
        0,
    )
}

/// `-[NSHTTPCookieStorage cookiesForURL:]` → empty NSArray
pub extern "C" fn shim_nshttpcookiestorage_cookies_for_url(_obj: u64, _cmd: u64, _url: u64) -> u64 {
    crate::nsarray::shim_nsarray_array_with_objects_count(
        crate::nsobj::nsarray_class_addr(),
        0,
        0,
        0,
    )
}

/// `-[NSHTTPCookieStorage setCookie:]`
pub extern "C" fn shim_nshttpcookiestorage_set_cookie(_obj: u64, _cmd: u64, _cookie: u64) {
    log::debug!("NSHTTPCookieStorage setCookie: (stub)");
}

/// `-[NSHTTPCookieStorage deleteCookie:]`
pub extern "C" fn shim_nshttpcookiestorage_delete_cookie(_obj: u64, _cmd: u64, _cookie: u64) {
    log::debug!("NSHTTPCookieStorage deleteCookie: (stub)");
}

/// `-[NSHTTPCookieStorage cookieAcceptPolicy]` → 0 (NSHTTPCookieAcceptPolicyAlways)
pub extern "C" fn shim_nshttpcookiestorage_accept_policy(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSHTTPCookieStorage setCookieAcceptPolicy:]`
pub extern "C" fn shim_nshttpcookiestorage_set_accept_policy(_obj: u64, _cmd: u64, _policy: u64) {}

pub extern "C" fn shim_nshttpcookiestorage_description(_obj: u64, _cmd: u64) -> u64 {
    make_nsstring("<NSHTTPCookieStorage>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── NSURLCredential ──────────────────────────────────────────

    #[test]
    fn credential_with_user_creates_object() {
        let user = make_string("alice");
        let pass = make_string("secret");
        let obj = shim_nsurlcredential_with_user(0, 0, user, pass, 2);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nsurlcredential_class_addr());
        assert_eq!(read_field(obj, CRED_USER_OFF), user);
        assert_eq!(read_field(obj, CRED_PASS_OFF), pass);
        assert_eq!(read_field(obj, CRED_PERSIST_OFF), 2);
        free_obj(obj);
    }

    #[test]
    fn credential_user_returns_user_ptr() {
        let user = make_string("bob");
        let pass = make_string("pw");
        let obj = shim_nsurlcredential_with_user(0, 0, user, pass, 0);
        assert_eq!(shim_nsurlcredential_user(obj, 0), user);
        free_obj(obj);
    }

    #[test]
    fn credential_user_nil_returns_zero() {
        assert_eq!(shim_nsurlcredential_user(0, 0), 0);
    }

    #[test]
    fn credential_password_returns_pass_ptr() {
        let user = make_string("u");
        let pass = make_string("p");
        let obj = shim_nsurlcredential_with_user(0, 0, user, pass, 0);
        assert_eq!(shim_nsurlcredential_password(obj, 0), pass);
        free_obj(obj);
    }

    #[test]
    fn credential_password_nil_returns_zero() {
        assert_eq!(shim_nsurlcredential_password(0, 0), 0);
    }

    #[test]
    fn credential_persistence_returns_value() {
        let user = make_string("u");
        let pass = make_string("p");
        let obj = shim_nsurlcredential_with_user(0, 0, user, pass, 3);
        assert_eq!(shim_nsurlcredential_persistence(obj, 0), 3);
        free_obj(obj);
    }

    #[test]
    fn credential_persistence_nil_returns_zero() {
        assert_eq!(shim_nsurlcredential_persistence(0, 0), 0);
    }

    #[test]
    fn credential_has_password_true() {
        let user = make_string("u");
        let pass = make_string("p");
        let obj = shim_nsurlcredential_with_user(0, 0, user, pass, 0);
        assert_eq!(shim_nsurlcredential_has_password(obj, 0), 1);
        free_obj(obj);
    }

    #[test]
    fn credential_has_password_false_when_no_password() {
        let user = make_string("u");
        let obj = shim_nsurlcredential_with_user(0, 0, user, 0, 0);
        assert_eq!(shim_nsurlcredential_has_password(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn credential_has_password_nil_returns_zero() {
        assert_eq!(shim_nsurlcredential_has_password(0, 0), 0);
    }

    #[test]
    fn credential_description_nil() {
        let desc = shim_nsurlcredential_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLCredential: nil>");
    }

    #[test]
    fn credential_description_with_user() {
        let user = make_string("carol");
        let obj = shim_nsurlcredential_with_user(0, 0, user, 0, 0);
        let desc = shim_nsurlcredential_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLCredential: user=carol>");
        free_obj(obj);
    }

    // ── NSURLCredentialStorage ───────────────────────────────────

    #[test]
    fn credential_storage_shared_returns_nonzero() {
        let s = shim_nsurlcredentialstorage_shared(0, 0);
        assert_ne!(s, 0);
    }

    #[test]
    fn credential_storage_shared_is_singleton() {
        let a = shim_nsurlcredentialstorage_shared(0, 0);
        let b = shim_nsurlcredentialstorage_shared(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn credential_storage_all_credentials_returns_dict() {
        let d = shim_nsurlcredentialstorage_all_credentials(0, 0);
        assert_ne!(d, 0);
        // Empty dict — count should be 0
        assert_eq!(crate::nsdictionary::shim_nsdictionary_count(d, 0), 0);
    }

    #[test]
    fn credential_storage_default_credential_returns_nil() {
        assert_eq!(shim_nsurlcredentialstorage_default_credential(0, 0, 0), 0);
    }

    #[test]
    fn credential_storage_set_default_credential_no_crash() {
        shim_nsurlcredentialstorage_set_default_credential(0, 0, 0, 0);
    }

    #[test]
    fn credential_storage_remove_credential_no_crash() {
        shim_nsurlcredentialstorage_remove_credential(0, 0, 0, 0);
    }

    #[test]
    fn credential_storage_description() {
        let desc = shim_nsurlcredentialstorage_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLCredentialStorage>");
    }

    // ── NSHTTPCookie ─────────────────────────────────────────────

    #[test]
    fn cookie_with_properties_creates_object() {
        let props = crate::nsmutabledict::shim_nsmutabledict_dictionary(0, 0);
        let obj = shim_nshttpcookie_with_properties(0, 0, props);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), nsobj::nshttpcookie_class_addr());
        assert_eq!(read_field(obj, COOKIE_PROPS_OFF), props);
        free_obj(obj);
    }

    #[test]
    fn cookie_properties_returns_ptr() {
        let props = crate::nsmutabledict::shim_nsmutabledict_dictionary(0, 0);
        let obj = shim_nshttpcookie_with_properties(0, 0, props);
        assert_eq!(shim_nshttpcookie_properties(obj, 0), props);
        free_obj(obj);
    }

    #[test]
    fn cookie_properties_nil_returns_zero() {
        assert_eq!(shim_nshttpcookie_properties(0, 0), 0);
    }

    fn make_cookie_with_props(name: &str, value: &str, domain: &str, path: &str) -> u64 {
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        let k_name = make_string("Name");
        let v_name = make_string(name);
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v_name, k_name);
        let k_value = make_string("Value");
        let v_value = make_string(value);
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v_value, k_value);
        let k_domain = make_string("Domain");
        let v_domain = make_string(domain);
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v_domain, k_domain);
        let k_path = make_string("Path");
        let v_path = make_string(path);
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v_path, k_path);
        shim_nshttpcookie_with_properties(0, 0, dict)
    }

    #[test]
    fn cookie_name_with_props() {
        let obj = make_cookie_with_props("sid", "abc123", "example.com", "/");
        let name = shim_nshttpcookie_name(obj, 0);
        assert_ne!(name, 0);
        assert_string_eq(name, "sid");
        free_obj(obj);
    }

    #[test]
    fn cookie_name_nil_returns_empty_string() {
        let name = shim_nshttpcookie_name(0, 0);
        assert_ne!(name, 0);
        assert_string_eq(name, "");
    }

    #[test]
    fn cookie_name_nil_props_returns_empty_string() {
        let obj = shim_nshttpcookie_with_properties(0, 0, 0);
        let name = shim_nshttpcookie_name(obj, 0);
        assert_ne!(name, 0);
        assert_string_eq(name, "");
        free_obj(obj);
    }

    #[test]
    fn cookie_value_nil_returns_empty_string() {
        let val = shim_nshttpcookie_value(0, 0);
        assert_ne!(val, 0);
        assert_string_eq(val, "");
    }

    #[test]
    fn cookie_domain_nil_returns_empty_string() {
        let dom = shim_nshttpcookie_domain(0, 0);
        assert_ne!(dom, 0);
        assert_string_eq(dom, "");
    }

    #[test]
    fn cookie_path_nil_returns_slash() {
        let p = shim_nshttpcookie_path(0, 0);
        assert_ne!(p, 0);
        assert_string_eq(p, "/");
    }

    #[test]
    fn cookie_path_nil_props_returns_slash() {
        let obj = shim_nshttpcookie_with_properties(0, 0, 0);
        let p = shim_nshttpcookie_path(obj, 0);
        assert_ne!(p, 0);
        assert_string_eq(p, "/");
        free_obj(obj);
    }

    #[test]
    fn cookie_is_secure_nil_returns_zero() {
        assert_eq!(shim_nshttpcookie_is_secure(0, 0), 0);
    }

    #[test]
    fn cookie_is_secure_no_props_returns_zero() {
        let obj = alloc_obj_fake(24);
        assert_eq!(shim_nshttpcookie_is_secure(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn httpcookie_is_secure() {
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        let k = make_string("Secure");
        let v = make_string("TRUE");
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v, k);
        let k2 = make_string("Name");
        let v2 = make_string("sid");
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v2, k2);
        let cookie = shim_nshttpcookie_with_properties(0, 0, dict);
        assert_eq!(shim_nshttpcookie_is_secure(cookie, 0), 1);
        free_obj(cookie);
    }

    #[test]
    fn cookie_is_http_only_nil_returns_zero() {
        assert_eq!(shim_nshttpcookie_is_http_only(0, 0), 0);
    }

    #[test]
    fn cookie_is_http_only_no_props_returns_zero() {
        let obj = alloc_obj_fake(24);
        assert_eq!(shim_nshttpcookie_is_http_only(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn httpcookie_is_http_only() {
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        let k = make_string("HttpOnly");
        let v = make_string("YES");
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v, k);
        let k2 = make_string("Name");
        let v2 = make_string("token");
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v2, k2);
        let cookie = shim_nshttpcookie_with_properties(0, 0, dict);
        assert_eq!(shim_nshttpcookie_is_http_only(cookie, 0), 1);
        free_obj(cookie);
    }

    #[test]
    fn cookie_expires_date_always_zero() {
        assert_eq!(shim_nshttpcookie_expires_date(0, 0), 0);
        let obj = alloc_obj_fake(24);
        assert_eq!(shim_nshttpcookie_expires_date(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn cookie_description_nil() {
        let desc = shim_nshttpcookie_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHTTPCookie: nil>");
    }

    #[test]
    fn cookie_description_with_no_props() {
        let obj = shim_nshttpcookie_with_properties(0, 0, 0);
        let desc = shim_nshttpcookie_description(obj, 0);
        assert_ne!(desc, 0);
        // name returns make_nsstring("") for nil props → description includes empty name
        assert_string_eq(desc, "<NSHTTPCookie: >");
        free_obj(obj);
    }

    // ── NSHTTPCookieStorage ──────────────────────────────────────

    #[test]
    fn cookie_storage_shared_returns_nonzero() {
        let s = shim_nshttpcookiestorage_shared(0, 0);
        assert_ne!(s, 0);
    }

    #[test]
    fn cookie_storage_shared_is_singleton() {
        let a = shim_nshttpcookiestorage_shared(0, 0);
        let b = shim_nshttpcookiestorage_shared(0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn cookie_storage_cookies_returns_empty_array() {
        let arr = shim_nshttpcookiestorage_cookies(0, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    #[test]
    fn cookie_storage_cookies_for_url_returns_empty_array() {
        let arr = shim_nshttpcookiestorage_cookies_for_url(0, 0, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    #[test]
    fn cookie_storage_set_cookie_no_crash() {
        shim_nshttpcookiestorage_set_cookie(0, 0, 0);
    }

    #[test]
    fn cookie_storage_delete_cookie_no_crash() {
        shim_nshttpcookiestorage_delete_cookie(0, 0, 0);
    }

    #[test]
    fn cookie_storage_accept_policy_returns_zero() {
        assert_eq!(shim_nshttpcookiestorage_accept_policy(0, 0), 0);
    }

    #[test]
    fn cookie_storage_set_accept_policy_no_crash() {
        shim_nshttpcookiestorage_set_accept_policy(0, 0, 42);
    }

    #[test]
    fn cookie_storage_description() {
        let desc = shim_nshttpcookiestorage_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHTTPCookieStorage>");
    }
}
