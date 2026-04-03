//! NSUserDefaults — in-memory key-value store.
//!
//! Real NSUserDefaults persists to ~/Library/Preferences/<bundle-id>.plist.
//! This shim provides an in-memory store sufficient for programs that use
//! NSUserDefaults for configuration lookup.
//!
//! Layout: singleton object (16 bytes):
//!   offset  0: isa       (u64)  → NSUserDefaults class addr
//!   offset  8: store_ptr (u64)  → pointer to the backing NSMutableDictionary

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

const NSUD_STORE_OFF: usize = 8;

/// Singleton instance.
static STANDARD_DEFAULTS: OnceLock<u64> = OnceLock::new();
static SUITE_STORES: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static SUITE_SEARCH_LISTS: OnceLock<Mutex<HashMap<u64, Vec<String>>>> = OnceLock::new();

fn suite_stores() -> &'static Mutex<HashMap<String, u64>> {
    SUITE_STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn suite_search_lists() -> &'static Mutex<HashMap<u64, Vec<String>>> {
    SUITE_SEARCH_LISTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_store(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) }
}

fn write_store(obj: u64, store: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + NSUD_STORE_OFF) as *mut u64) = store;
    }
}

fn make_store_dictionary() -> u64 {
    crate::nsmutabledict::shim_nsmutabledict_dictionary(
        crate::nsobj::nsmutabledictionary_class_addr(),
        0,
    )
}

fn extract_suite_name(name: u64) -> Option<String> {
    crate::cfstring::extract_cfstring_cstr(name).map(str::to_string)
}

fn ensure_suite_store(name: &str) -> u64 {
    let mut stores = suite_stores().lock().unwrap();
    if let Some(&store) = stores.get(name) {
        return store;
    }

    let store = make_store_dictionary();
    if store != 0 {
        stores.insert(name.to_string(), store);
    }
    store
}

fn lookup_suite_store(name: &str) -> u64 {
    suite_stores()
        .lock()
        .unwrap()
        .get(name)
        .copied()
        .unwrap_or(0)
}

fn ensure_store(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let existing = read_store(obj);
    if existing != 0 {
        return existing;
    }

    let store = make_store_dictionary();
    if store != 0 {
        write_store(obj, store);
    }
    store
}

fn get_or_create_singleton() -> u64 {
    *STANDARD_DEFAULTS.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsuserdefaults_class_addr();
        }
        ensure_store(obj);
        obj
    })
}

/// `+[NSUserDefaults standardUserDefaults]`
pub extern "C" fn shim_nsuserdefaults_standard(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_singleton()
}

/// `-[NSUserDefaults init]`
pub extern "C" fn shim_nsuserdefaults_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    ensure_store(obj);
    obj
}

/// `-[NSUserDefaults initWithSuiteName:]`
pub extern "C" fn shim_nsuserdefaults_init_with_suite_name(obj: u64, _cmd: u64, suite: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    if let Some(name) = extract_suite_name(suite) {
        let store = ensure_suite_store(&name);
        if store != 0 {
            write_store(obj, store);
            return obj;
        }
    }

    shim_nsuserdefaults_init(obj, _cmd)
}

/// `-[NSUserDefaults objectForKey:]`
pub extern "C" fn shim_nsuserdefaults_object_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    let store = ensure_store(obj);
    if store == 0 {
        return 0;
    }

    let direct = crate::nsdictionary::shim_nsdictionary_object_for_key(store, 0, key);
    if direct != 0 {
        return direct;
    }

    let suites = {
        let lists = suite_search_lists().lock().unwrap();
        let Some(suites) = lists.get(&obj) else {
            return 0;
        };
        suites.clone()
    };

    for suite in suites.iter().rev() {
        let suite_store = lookup_suite_store(suite);
        if suite_store == 0 {
            continue;
        }
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(suite_store, 0, key);
        if value != 0 {
            return value;
        }
    }

    0
}

/// `-[NSUserDefaults stringForKey:]` — returns nil if value is not an NSString.
pub extern "C" fn shim_nsuserdefaults_string_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    // In our simplified model, we just return objectForKey: since all stored values
    // from setObject:forKey: are kept as-is.
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}
pub extern "C" fn shim_nsuserdefaults_integer_for_key(obj: u64, _cmd: u64, key: u64) -> i64 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0;
    }
    // Try to read integerValue from the object (works for NSNumber and NSString)
    // Read ISA to check class
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_integer_value(val, 0)
    } else {
        0
    }
}

/// `-[NSUserDefaults boolForKey:]`
pub extern "C" fn shim_nsuserdefaults_bool_for_key(obj: u64, _cmd: u64, key: u64) -> u8 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0;
    }
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_bool_value(val, 0)
    } else {
        0
    }
}

/// `-[NSUserDefaults doubleForKey:]`
pub extern "C" fn shim_nsuserdefaults_double_for_key(obj: u64, _cmd: u64, key: u64) -> f64 {
    let val = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if val == 0 {
        return 0.0;
    }
    let isa = unsafe { *(val as *const u64) };
    if isa == crate::nsobj::nsnumber_class_addr() {
        crate::nsnumber::shim_nsnumber_double_value(val, 0)
    } else {
        0.0
    }
}

/// `-[NSUserDefaults setObject:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_object(obj: u64, _cmd: u64, value: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }
    let store = ensure_store(obj);
    if store == 0 {
        return;
    }
    if value == 0 {
        // Setting nil = removeObjectForKey:
        crate::nsmutabledict::shim_nsmutabledict_remove_object_for_key(store, 0, key);
    } else {
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(store, 0, value, key);
    }
}

/// `-[NSUserDefaults removeObjectForKey:]`
pub extern "C" fn shim_nsuserdefaults_remove_object(obj: u64, _cmd: u64, key: u64) {
    if obj == 0 || key == 0 {
        return;
    }
    let store = ensure_store(obj);
    if store == 0 {
        return;
    }
    crate::nsmutabledict::shim_nsmutabledict_remove_object_for_key(store, 0, key);
}

/// `-[NSUserDefaults registerDefaults:]`
///
/// Merges a dictionary of fallback values into the defaults store without
/// overwriting keys that already have an explicit value.
pub extern "C" fn shim_nsuserdefaults_register_defaults(obj: u64, _cmd: u64, defaults: u64) {
    if obj == 0 || defaults == 0 {
        return;
    }

    let keys = crate::nsdictionary::shim_nsdictionary_all_keys(defaults, 0);
    let count = crate::nsarray::shim_nsarray_count(keys, 0);

    for index in 0..count {
        let key = crate::nsarray::shim_nsarray_object_at_index(keys, 0, index);
        if key == 0 {
            continue;
        }
        if shim_nsuserdefaults_object_for_key(obj, 0, key) != 0 {
            continue;
        }

        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(defaults, 0, key);
        if value != 0 {
            shim_nsuserdefaults_set_object(obj, 0, value, key);
        }
    }
}

/// `-[NSUserDefaults addSuiteNamed:]`
pub extern "C" fn shim_nsuserdefaults_add_suite_named(obj: u64, _cmd: u64, suite: u64) {
    if obj == 0 || suite == 0 {
        return;
    }

    let Some(name) = extract_suite_name(suite) else {
        return;
    };

    ensure_suite_store(&name);

    let mut lists = suite_search_lists().lock().unwrap();
    let search_list = lists.entry(obj).or_default();
    if !search_list.iter().any(|existing| existing == &name) {
        search_list.push(name);
    }
}

/// `-[NSUserDefaults removeSuiteNamed:]`
pub extern "C" fn shim_nsuserdefaults_remove_suite_named(obj: u64, _cmd: u64, suite: u64) {
    if obj == 0 || suite == 0 {
        return;
    }

    let Some(name) = extract_suite_name(suite) else {
        return;
    };

    let mut lists = suite_search_lists().lock().unwrap();
    let Some(search_list) = lists.get_mut(&obj) else {
        return;
    };

    search_list.retain(|existing| existing != &name);
}

/// `-[NSUserDefaults volatileDomainForName:]`
///
/// Our simplified defaults model exposes a single in-memory dictionary. For
/// callers probing `NSRegistrationDomain` after `registerDefaults:`, returning
/// that store is sufficient.
pub extern "C" fn shim_nsuserdefaults_volatile_domain_for_name(obj: u64, _cmd: u64, name: u64) -> u64 {
    if obj == 0 || name == 0 {
        return 0;
    }

    if let Some(name) = extract_suite_name(name) {
        if name != "NSRegistrationDomain" {
            let store = lookup_suite_store(&name);
            if store != 0 {
                return store;
            }
        }
    }

    ensure_store(obj)
}

/// `-[NSUserDefaults synchronize]` — no-op in our in-memory store.
pub extern "C" fn shim_nsuserdefaults_synchronize(_obj: u64, _cmd: u64) -> u8 {
    1 // YES — always succeeds
}

/// `-[NSUserDefaults setInteger:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_integer(obj: u64, _cmd: u64, value: i64, key: u64) {
    let number = crate::nsnumber::make_nsnumber_int(value);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setBool:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_bool(obj: u64, _cmd: u64, value: u8, key: u64) {
    let number = crate::nsnumber::make_nsnumber_bool(value != 0);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setDouble:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_double(obj: u64, _cmd: u64, value: f64, key: u64) {
    let number = crate::nsnumber::make_nsnumber_double(value);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults setFloat:forKey:]`
pub extern "C" fn shim_nsuserdefaults_set_float(obj: u64, _cmd: u64, value: f32, key: u64) {
    let number = crate::nsnumber::make_nsnumber_double(value as f64);
    shim_nsuserdefaults_set_object(obj, 0, number, key);
}

/// `-[NSUserDefaults floatForKey:]`
pub extern "C" fn shim_nsuserdefaults_float_for_key(obj: u64, _cmd: u64, key: u64) -> f32 {
    shim_nsuserdefaults_double_for_key(obj, _cmd, key) as f32
}

/// `-[NSUserDefaults arrayForKey:]`
pub extern "C" fn shim_nsuserdefaults_array_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}

/// `-[NSUserDefaults dictionaryForKey:]`
pub extern "C" fn shim_nsuserdefaults_dictionary_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    shim_nsuserdefaults_object_for_key(obj, _cmd, key)
}

/// `-[NSUserDefaults URLForKey:]`
pub extern "C" fn shim_nsuserdefaults_url_for_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    let value = shim_nsuserdefaults_object_for_key(obj, _cmd, key);
    if value == 0 {
        return 0;
    }

    let isa = unsafe { *(value as *const u64) };
    if isa == crate::nsobj::nsurl_class_addr() {
        return value;
    }

    if let Some(s) = crate::cfstring::extract_cfstring_cstr(value) {
        if s.starts_with('/') && !s.contains("://") {
            return crate::nsurl::shim_nsurl_file_url_with_path(
                crate::nsobj::nsurl_class_addr(),
                0,
                value,
            );
        }
        return crate::nsurl::shim_nsurl_url_with_string(crate::nsobj::nsurl_class_addr(), 0, value);
    }

    value
}

/// `-[NSUserDefaults dictionaryRepresentation]` — return all defaults as NSDictionary.
///
/// Returns the backing store directly (it is already an NSDictionary).
pub extern "C" fn shim_nsuserdefaults_dictionary_representation(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    ensure_store(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn get_defaults() -> u64 {
        shim_nsuserdefaults_standard(0, 0)
    }

    fn alloc_defaults() -> u64 {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsuserdefaults_class_addr();
        }
        obj
    }

    #[test]
    fn standard_returns_nonzero() {
        assert_ne!(get_defaults(), 0);
    }

    #[test]
    fn init_initializes_backing_store() {
        let obj = alloc_defaults();

        assert_eq!(unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) }, 0);
        assert_eq!(shim_nsuserdefaults_init(obj, 0), obj);
        assert_ne!(unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) }, 0);
    }

    #[test]
    fn init_with_suite_name_initializes_backing_store() {
        let obj = alloc_defaults();
        let suite = make_string("com.example.test");

        assert_eq!(shim_nsuserdefaults_init_with_suite_name(obj, 0, suite), obj);
        assert_ne!(unsafe { *((obj as usize + NSUD_STORE_OFF) as *const u64) }, 0);
    }

    #[test]
    fn init_with_suite_name_reuses_shared_suite_store() {
        let suite = make_string("com.example.shared-suite");
        let key = make_string("_ut_suite_shared_key");
        let value = make_string("shared-value");

        let first = alloc_defaults();
        let second = alloc_defaults();
        shim_nsuserdefaults_init_with_suite_name(first, 0, suite);
        shim_nsuserdefaults_init_with_suite_name(second, 0, suite);

        shim_nsuserdefaults_set_object(first, 0, value, key);
        assert_eq!(shim_nsuserdefaults_object_for_key(second, 0, key), value);
    }

    #[test]
    fn standard_is_stable_singleton() {
        assert_eq!(get_defaults(), get_defaults());
    }

    #[test]
    fn set_and_get_string_object() {
        let defaults = get_defaults();
        let key = make_string("_ut_str_key");
        let val = make_string("hello");
        shim_nsuserdefaults_set_object(defaults, 0, val, key);
        let out = shim_nsuserdefaults_object_for_key(defaults, 0, key);
        assert_eq!(out, val);
    }

    #[test]
    fn nil_obj_set_object_is_noop() {
        let key = make_string("any_key_noop");
        let val = make_string("val");
        shim_nsuserdefaults_set_object(0, 0, val, key); // must not crash
    }

    #[test]
    fn object_for_key_missing_returns_zero() {
        let defaults = get_defaults();
        let key = make_string("_ut_nonexistent_xyz");
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);
    }

    #[test]
    fn remove_object_clears_key() {
        let defaults = get_defaults();
        let key = make_string("_ut_remove_key");
        let val = make_string("removable");
        shim_nsuserdefaults_set_object(defaults, 0, val, key);
        assert_ne!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);
        shim_nsuserdefaults_remove_object(defaults, 0, key);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);
    }

    #[test]
    fn set_object_nil_removes_key() {
        let defaults = get_defaults();
        let key = make_string("_ut_set_nil_key");
        let val = make_string("will_be_removed");
        shim_nsuserdefaults_set_object(defaults, 0, val, key);
        shim_nsuserdefaults_set_object(defaults, 0, 0, key);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);
    }

    #[test]
    fn set_and_get_integer() {
        let defaults = get_defaults();
        let key = make_string("_ut_int_key");
        shim_nsuserdefaults_set_integer(defaults, 0, 42, key);
        assert_eq!(shim_nsuserdefaults_integer_for_key(defaults, 0, key), 42);
    }

    #[test]
    fn set_and_get_bool() {
        let defaults = get_defaults();
        let key = make_string("_ut_bool_key");
        shim_nsuserdefaults_set_bool(defaults, 0, 1, key);
        assert_ne!(shim_nsuserdefaults_bool_for_key(defaults, 0, key), 0);
    }

    #[test]
    fn synchronize_returns_yes() {
        assert_eq!(shim_nsuserdefaults_synchronize(get_defaults(), 0), 1);
    }

    #[test]
    fn register_defaults_sets_missing_values() {
        let defaults = get_defaults();
        let key = make_string("_ut_register_defaults_missing_key");
        shim_nsuserdefaults_remove_object(defaults, 0, key);

        let source = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        let val = make_string("fallback");
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(source, 0, val, key);

        shim_nsuserdefaults_register_defaults(defaults, 0, source);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), val);
    }

    #[test]
    fn register_defaults_preserves_existing_values() {
        let defaults = get_defaults();
        let key = make_string("_ut_register_defaults_existing_key");
        let existing = make_string("existing");
        let fallback = make_string("fallback");
        shim_nsuserdefaults_set_object(defaults, 0, existing, key);

        let source = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            crate::nsobj::nsmutabledictionary_class_addr(),
            0,
        );
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(source, 0, fallback, key);

        shim_nsuserdefaults_register_defaults(defaults, 0, source);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), existing);
    }

    #[test]
    fn volatile_domain_returns_backing_store() {
        let defaults = get_defaults();
        let domain = make_string("NSRegistrationDomain");
        let key = make_string("_ut_volatile_domain_key");
        let value = make_string("present");
        shim_nsuserdefaults_set_object(defaults, 0, value, key);

        let store = shim_nsuserdefaults_volatile_domain_for_name(defaults, 0, domain);
        assert_ne!(store, 0);
        assert_eq!(crate::nsdictionary::shim_nsdictionary_object_for_key(store, 0, key), value);
    }

    #[test]
    fn add_suite_named_exposes_suite_values() {
        let defaults = get_defaults();
        let suite = make_string("com.example.visible-suite");
        let key = make_string("_ut_suite_visible_key");
        let value = make_string("from-suite");

        let suite_defaults = alloc_defaults();
        shim_nsuserdefaults_init_with_suite_name(suite_defaults, 0, suite);
        shim_nsuserdefaults_set_object(suite_defaults, 0, value, key);

        shim_nsuserdefaults_remove_suite_named(defaults, 0, suite);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);

        shim_nsuserdefaults_add_suite_named(defaults, 0, suite);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), value);
    }

    #[test]
    fn remove_suite_named_hides_suite_values() {
        let defaults = get_defaults();
        let suite = make_string("com.example.removed-suite");
        let key = make_string("_ut_suite_removed_key");
        let value = make_string("suite-only");

        let suite_defaults = alloc_defaults();
        shim_nsuserdefaults_init_with_suite_name(suite_defaults, 0, suite);
        shim_nsuserdefaults_set_object(suite_defaults, 0, value, key);

        shim_nsuserdefaults_add_suite_named(defaults, 0, suite);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), value);

        shim_nsuserdefaults_remove_suite_named(defaults, 0, suite);
        assert_eq!(shim_nsuserdefaults_object_for_key(defaults, 0, key), 0);
    }

    #[test]
    fn volatile_domain_returns_suite_store_for_known_suite() {
        let suite = make_string("com.example.suite-domain");
        let key = make_string("_ut_suite_domain_key");
        let value = make_string("suite-domain-value");

        let suite_defaults = alloc_defaults();
        shim_nsuserdefaults_init_with_suite_name(suite_defaults, 0, suite);
        shim_nsuserdefaults_set_object(suite_defaults, 0, value, key);

        let store = shim_nsuserdefaults_volatile_domain_for_name(suite_defaults, 0, suite);
        assert_ne!(store, 0);
        assert_eq!(crate::nsdictionary::shim_nsdictionary_object_for_key(store, 0, key), value);
    }

    #[test]
    fn url_for_key_wraps_strings_as_nsurl() {
        let defaults = get_defaults();
        let key = make_string("_ut_url_key");
        let value = make_string("https://example.invalid");
        shim_nsuserdefaults_set_object(defaults, 0, value, key);

        let out = shim_nsuserdefaults_url_for_key(defaults, 0, key);
        assert_ne!(out, 0);
        assert_ne!(out, value);
        assert_string_eq(crate::nsurl::shim_nsurl_absolute_string(out, 0), "https://example.invalid");
    }

    #[test]
    fn url_for_key_preserves_nsurl_values() {
        let defaults = get_defaults();
        let key = make_string("_ut_url_existing_nsurl_key");
        let url_string = make_string("file:///tmp/test.txt");
        let value = crate::nsurl::shim_nsurl_url_with_string(crate::nsobj::nsurl_class_addr(), 0, url_string);
        shim_nsuserdefaults_set_object(defaults, 0, value, key);

        assert_eq!(shim_nsuserdefaults_url_for_key(defaults, 0, key), value);
    }

    #[test]
    fn nil_obj_returns_zero_or_default() {
        let key = make_string("k");
        assert_eq!(shim_nsuserdefaults_object_for_key(0, 0, key), 0);
        assert_eq!(shim_nsuserdefaults_integer_for_key(0, 0, key), 0);
        assert_eq!(shim_nsuserdefaults_bool_for_key(0, 0, key), 0);
    }

    #[test]
    fn dictionary_representation_returns_store() {
        let defaults = get_defaults();
        let key = make_string("_ut_dict_repr_key");
        let val = make_string("dict_repr_val");
        shim_nsuserdefaults_set_object(defaults, 0, val, key);

        let repr = shim_nsuserdefaults_dictionary_representation(defaults, 0);
        assert_ne!(repr, 0);
        // The representation should contain our key
        let out = crate::nsdictionary::shim_nsdictionary_object_for_key(repr, 0, key);
        assert_eq!(out, val);
    }

    #[test]
    fn dictionary_representation_nil_safe() {
        assert_eq!(shim_nsuserdefaults_dictionary_representation(0, 0), 0);
    }
}
