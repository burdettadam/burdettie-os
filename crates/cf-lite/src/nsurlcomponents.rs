//! NSURLComponents shim: URL builder/parser with component accessors.
//!
//! Object layout (56 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: scheme_ptr  (u64 — NSString)
//!   offset 16: host_ptr    (u64 — NSString)
//!   offset 24: port_ptr    (u64 — NSNumber, 0=nil)
//!   offset 32: path_ptr    (u64 — NSString)
//!   offset 40: query_ptr   (u64 — NSString)
//!   offset 48: fragment_ptr(u64 — NSString)

use crate::cfstring;
use crate::nsobj;

const SCHEME_OFF: usize = 8;
const HOST_OFF: usize = 16;
const PORT_OFF: usize = 24;
const PATH_OFF: usize = 32;
const QUERY_OFF: usize = 40;
const FRAGMENT_OFF: usize = 48;
const OBJ_SIZE: usize = 56;

fn alloc_components() -> u64 {
    let obj = crate::pal_alloc().calloc(1, OBJ_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlcomponents_class_addr();
    }
    obj
}

fn set_str_field(obj: u64, off: usize, val: u64) {
    unsafe {
        *((obj as usize + off) as *mut u64) = val;
    }
}

fn get_str_field(obj: u64, off: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + off) as *const u64) }
}

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

/// Parse a URL string into its components.
fn parse_url(url: &str, obj: u64) {
    // scheme
    if let Some(pos) = url.find("://") {
        let scheme = &url[..pos];
        set_str_field(obj, SCHEME_OFF, make_nsstring(scheme));
        let rest = &url[pos + 3..];
        parse_authority_and_path(rest, obj);
    } else {
        // No scheme — treat as path
        set_str_field(obj, PATH_OFF, make_nsstring(url));
    }
}

fn parse_authority_and_path(s: &str, obj: u64) {
    // Split authority from path at first /
    let (authority, path_query_frag) = if let Some(pos) = s.find('/') {
        (&s[..pos], &s[pos..])
    } else {
        (s, "")
    };

    // Parse host:port from authority
    if !authority.is_empty() {
        if let Some(colon) = authority.rfind(':') {
            let host = &authority[..colon];
            let port_str = &authority[colon + 1..];
            if let Ok(port) = port_str.parse::<i64>() {
                set_str_field(obj, HOST_OFF, make_nsstring(host));
                set_str_field(obj, PORT_OFF, crate::nsnumber::make_nsnumber_int(port));
            } else {
                set_str_field(obj, HOST_OFF, make_nsstring(authority));
            }
        } else {
            set_str_field(obj, HOST_OFF, make_nsstring(authority));
        }
    }

    if path_query_frag.is_empty() {
        return;
    }

    // Split fragment
    let (path_query, fragment) = if let Some(pos) = path_query_frag.find('#') {
        (&path_query_frag[..pos], Some(&path_query_frag[pos + 1..]))
    } else {
        (path_query_frag, None)
    };

    // Split query
    let (path, query) = if let Some(pos) = path_query.find('?') {
        (&path_query[..pos], Some(&path_query[pos + 1..]))
    } else {
        (path_query, None)
    };

    if !path.is_empty() {
        set_str_field(obj, PATH_OFF, make_nsstring(path));
    }
    if let Some(q) = query {
        set_str_field(obj, QUERY_OFF, make_nsstring(q));
    }
    if let Some(f) = fragment {
        set_str_field(obj, FRAGMENT_OFF, make_nsstring(f));
    }
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSURLComponents componentsWithString:]`
pub extern "C" fn shim_nsurlcomponents_with_string(_cls: u64, _cmd: u64, url_string: u64) -> u64 {
    let obj = alloc_components();
    if obj == 0 || url_string == 0 {
        return obj;
    }
    if let Some(s) = cfstring::extract_cfstring_cstr(url_string) {
        parse_url(s, obj);
    }
    obj
}

/// `+[NSURLComponents new]`
pub extern "C" fn shim_nsurlcomponents_new(_cls: u64, _cmd: u64) -> u64 {
    alloc_components()
}

// ── Instance methods (getters) ─────────────────────────────────

pub extern "C" fn shim_nsurlcomponents_scheme(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, SCHEME_OFF)
}
pub extern "C" fn shim_nsurlcomponents_host(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, HOST_OFF)
}
pub extern "C" fn shim_nsurlcomponents_port(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, PORT_OFF)
}
pub extern "C" fn shim_nsurlcomponents_path(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, PATH_OFF)
}
pub extern "C" fn shim_nsurlcomponents_query(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, QUERY_OFF)
}
pub extern "C" fn shim_nsurlcomponents_fragment(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, FRAGMENT_OFF)
}

// ── Instance methods (setters) ─────────────────────────────────

pub extern "C" fn shim_nsurlcomponents_set_scheme(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, SCHEME_OFF, val);
    0
}
pub extern "C" fn shim_nsurlcomponents_set_host(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, HOST_OFF, val);
    0
}
pub extern "C" fn shim_nsurlcomponents_set_port(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, PORT_OFF, val);
    0
}
pub extern "C" fn shim_nsurlcomponents_set_path(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, PATH_OFF, val);
    0
}
pub extern "C" fn shim_nsurlcomponents_set_query(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, QUERY_OFF, val);
    0
}
pub extern "C" fn shim_nsurlcomponents_set_fragment(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, FRAGMENT_OFF, val);
    0
}

/// `-[NSURLComponents URL]` — reconstruct URL from components.
pub extern "C" fn shim_nsurlcomponents_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let mut result = String::new();

    if let Some(scheme) = cfstring::extract_cfstring_cstr(get_str_field(obj, SCHEME_OFF)) {
        result.push_str(scheme);
        result.push_str("://");
    }
    if let Some(host) = cfstring::extract_cfstring_cstr(get_str_field(obj, HOST_OFF)) {
        result.push_str(host);
    }
    let port_ptr = get_str_field(obj, PORT_OFF);
    if port_ptr != 0 {
        let port = crate::nsnumber::shim_nsnumber_integer_value(port_ptr, 0);
        result.push_str(&format!(":{port}"));
    }
    if let Some(path) = cfstring::extract_cfstring_cstr(get_str_field(obj, PATH_OFF)) {
        result.push_str(path);
    }
    if let Some(query) = cfstring::extract_cfstring_cstr(get_str_field(obj, QUERY_OFF)) {
        result.push('?');
        result.push_str(query);
    }
    if let Some(frag) = cfstring::extract_cfstring_cstr(get_str_field(obj, FRAGMENT_OFF)) {
        result.push('#');
        result.push_str(frag);
    }

    let ns = make_nsstring(&result);
    crate::nsurl::shim_nsurl_url_with_string(0, 0, ns)
}

/// `-[NSURLComponents string]`
pub extern "C" fn shim_nsurlcomponents_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let url = shim_nsurlcomponents_url(obj, 0);
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_absolute_string(url, 0)
}

/// `-[NSURLComponents queryItems]` — returns NSArray of NSURLQueryItem.
pub extern "C" fn shim_nsurlcomponents_query_items(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let query_str = get_str_field(obj, QUERY_OFF);
    if query_str == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    let Some(qs) = cfstring::extract_cfstring_cstr(query_str) else {
        return crate::nsarray::make_nsarray(&[]);
    };
    let mut items = Vec::new();
    for pair in qs.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = if let Some(eq) = pair.find('=') {
            (&pair[..eq], Some(&pair[eq + 1..]))
        } else {
            (pair, None)
        };
        let item = alloc_query_item(make_nsstring(name), value.map_or(0, make_nsstring));
        items.push(item);
    }
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSURLComponents setQueryItems:]`
pub extern "C" fn shim_nsurlcomponents_set_query_items(obj: u64, _cmd: u64, items: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if items == 0 {
        set_str_field(obj, QUERY_OFF, 0);
        return 0;
    }
    let count = crate::nsarray::shim_nsarray_count(items, 0);
    let mut parts = Vec::new();
    for i in 0..count {
        let item = crate::nsarray::shim_nsarray_object_at_index(items, 0, i);
        if let Some(name) = cfstring::extract_cfstring_cstr(shim_nsurlqueryitem_name(item, 0)) {
            let val_ptr = shim_nsurlqueryitem_value(item, 0);
            if val_ptr != 0 {
                if let Some(val) = cfstring::extract_cfstring_cstr(val_ptr) {
                    parts.push(format!("{name}={val}"));
                } else {
                    parts.push(name.to_string());
                }
            } else {
                parts.push(name.to_string());
            }
        }
    }
    set_str_field(obj, QUERY_OFF, make_nsstring(&parts.join("&")));
    0
}

// ── Percent-encoded accessors (pass-through to normal path/query) ──

pub extern "C" fn shim_nsurlcomponents_percent_encoded_query(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, QUERY_OFF)
}

pub extern "C" fn shim_nsurlcomponents_set_percent_encoded_query(obj: u64, _cmd: u64, query: u64) {
    if obj == 0 {
        return;
    }
    set_str_field(obj, QUERY_OFF, query);
}

pub extern "C" fn shim_nsurlcomponents_percent_encoded_path(obj: u64, _cmd: u64) -> u64 {
    get_str_field(obj, PATH_OFF)
}

pub extern "C" fn shim_nsurlcomponents_set_percent_encoded_path(obj: u64, _cmd: u64, path: u64) {
    if obj == 0 {
        return;
    }
    set_str_field(obj, PATH_OFF, path);
}

pub extern "C" fn shim_nsurlcomponents_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsurlcomponents_string(obj, 0)
}

// ═══════════════════════════════════════════════════════════════
// NSURLQueryItem
// Layout (24 bytes): isa, name_ptr, value_ptr
// ═══════════════════════════════════════════════════════════════

const QI_NAME_OFF: usize = 8;
const QI_VALUE_OFF: usize = 16;

fn alloc_query_item(name: u64, value: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlqueryitem_class_addr();
        *((obj as usize + QI_NAME_OFF) as *mut u64) = name;
        *((obj as usize + QI_VALUE_OFF) as *mut u64) = value;
    }
    obj
}

/// `+[NSURLQueryItem queryItemWithName:value:]`
pub extern "C" fn shim_nsurlqueryitem_with_name_value(
    _cls: u64,
    _cmd: u64,
    name: u64,
    value: u64,
) -> u64 {
    alloc_query_item(name, value)
}

pub extern "C" fn shim_nsurlqueryitem_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + QI_NAME_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlqueryitem_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + QI_VALUE_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlqueryitem_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let name =
        cfstring::extract_cfstring_cstr(shim_nsurlqueryitem_name(obj, 0)).unwrap_or_default();
    let val =
        cfstring::extract_cfstring_cstr(shim_nsurlqueryitem_value(obj, 0)).unwrap_or("(null)");
    let desc = format!("{name}={val}");
    make_nsstring(&desc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── NSURLComponents: new ───────────────────────────────────

    #[test]
    fn new_returns_valid_obj() {
        let obj = shim_nsurlcomponents_new(0, 0);
        assert_ne!(obj, 0);
        // All fields should be zero/nil
        assert_eq!(read_field(obj, SCHEME_OFF), 0);
        assert_eq!(read_field(obj, HOST_OFF), 0);
        assert_eq!(read_field(obj, PORT_OFF), 0);
        assert_eq!(read_field(obj, PATH_OFF), 0);
        assert_eq!(read_field(obj, QUERY_OFF), 0);
        assert_eq!(read_field(obj, FRAGMENT_OFF), 0);
        free_obj(obj);
    }

    // ── NSURLComponents: componentsWithString: ─────────────────

    #[test]
    fn with_string_parses_full_url() {
        let url = make_string("https://example.com:8080/path?q=1#frag");
        let obj = shim_nsurlcomponents_with_string(0, 0, url);
        assert_ne!(obj, 0);
        assert_string_eq(read_field(obj, SCHEME_OFF), "https");
        assert_string_eq(read_field(obj, HOST_OFF), "example.com");
        // port is an NSNumber
        let port_ptr = read_field(obj, PORT_OFF);
        assert_ne!(port_ptr, 0);
        assert_eq!(
            crate::nsnumber::shim_nsnumber_integer_value(port_ptr, 0),
            8080
        );
        assert_string_eq(read_field(obj, PATH_OFF), "/path");
        assert_string_eq(read_field(obj, QUERY_OFF), "q=1");
        assert_string_eq(read_field(obj, FRAGMENT_OFF), "frag");
        free_obj(obj);
    }

    #[test]
    fn with_string_no_port_no_query_no_fragment() {
        let url = make_string("http://host/some/path");
        let obj = shim_nsurlcomponents_with_string(0, 0, url);
        assert_ne!(obj, 0);
        assert_string_eq(read_field(obj, SCHEME_OFF), "http");
        assert_string_eq(read_field(obj, HOST_OFF), "host");
        assert_eq!(read_field(obj, PORT_OFF), 0);
        assert_string_eq(read_field(obj, PATH_OFF), "/some/path");
        assert_eq!(read_field(obj, QUERY_OFF), 0);
        assert_eq!(read_field(obj, FRAGMENT_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn with_string_nil_url() {
        let obj = shim_nsurlcomponents_with_string(0, 0, 0);
        // Returns an allocated but empty components object
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, SCHEME_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn with_string_no_scheme() {
        let url = make_string("/just/a/path");
        let obj = shim_nsurlcomponents_with_string(0, 0, url);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, SCHEME_OFF), 0);
        assert_string_eq(read_field(obj, PATH_OFF), "/just/a/path");
        free_obj(obj);
    }

    // ── Getters: nil safety ────────────────────────────────────

    #[test]
    fn scheme_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_scheme(0, 0), 0);
    }

    #[test]
    fn host_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_host(0, 0), 0);
    }

    #[test]
    fn port_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_port(0, 0), 0);
    }

    #[test]
    fn path_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_path(0, 0), 0);
    }

    #[test]
    fn query_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_query(0, 0), 0);
    }

    #[test]
    fn fragment_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_fragment(0, 0), 0);
    }

    // ── Getters: normal operation ──────────────────────────────

    #[test]
    fn getters_return_fields() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("val");
        write_field(obj, SCHEME_OFF, s);
        assert_eq!(shim_nsurlcomponents_scheme(obj, 0), s);
        assert_eq!(shim_nsurlcomponents_host(obj, 0), 0);
        free_obj(obj);
    }

    // ── Setters ────────────────────────────────────────────────

    #[test]
    fn set_scheme() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("https");
        let ret = shim_nsurlcomponents_set_scheme(obj, 0, s);
        assert_eq!(ret, 0);
        assert_string_eq(shim_nsurlcomponents_scheme(obj, 0), "https");
        free_obj(obj);
    }

    #[test]
    fn set_host() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("example.com");
        shim_nsurlcomponents_set_host(obj, 0, s);
        assert_string_eq(shim_nsurlcomponents_host(obj, 0), "example.com");
        free_obj(obj);
    }

    #[test]
    fn set_port() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let port = crate::nsnumber::make_nsnumber_int(443);
        shim_nsurlcomponents_set_port(obj, 0, port);
        let p = shim_nsurlcomponents_port(obj, 0);
        assert_eq!(crate::nsnumber::shim_nsnumber_integer_value(p, 0), 443);
        free_obj(obj);
    }

    #[test]
    fn set_path() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("/api/v1");
        shim_nsurlcomponents_set_path(obj, 0, s);
        assert_string_eq(shim_nsurlcomponents_path(obj, 0), "/api/v1");
        free_obj(obj);
    }

    #[test]
    fn set_query() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("key=val");
        shim_nsurlcomponents_set_query(obj, 0, s);
        assert_string_eq(shim_nsurlcomponents_query(obj, 0), "key=val");
        free_obj(obj);
    }

    #[test]
    fn set_fragment() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let s = make_string("section1");
        shim_nsurlcomponents_set_fragment(obj, 0, s);
        assert_string_eq(shim_nsurlcomponents_fragment(obj, 0), "section1");
        free_obj(obj);
    }

    // ── URL reconstruction ─────────────────────────────────────

    #[test]
    fn url_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_url(0, 0), 0);
    }

    #[test]
    fn url_reconstructs_full() {
        let obj = shim_nsurlcomponents_new(0, 0);
        shim_nsurlcomponents_set_scheme(obj, 0, make_string("https"));
        shim_nsurlcomponents_set_host(obj, 0, make_string("example.com"));
        shim_nsurlcomponents_set_port(obj, 0, crate::nsnumber::make_nsnumber_int(8080));
        shim_nsurlcomponents_set_path(obj, 0, make_string("/path"));
        shim_nsurlcomponents_set_query(obj, 0, make_string("q=1"));
        shim_nsurlcomponents_set_fragment(obj, 0, make_string("top"));
        let url = shim_nsurlcomponents_url(obj, 0);
        assert_ne!(url, 0);
        let abs = crate::nsurl::shim_nsurl_absolute_string(url, 0);
        assert_string_eq(abs, "https://example.com:8080/path?q=1#top");
        free_obj(obj);
    }

    #[test]
    fn url_empty_components() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let url = shim_nsurlcomponents_url(obj, 0);
        // Empty URL string — nsurl_url_with_string returns an NSURL wrapping empty string
        // Just verify no crash
        let _ = url;
        free_obj(obj);
    }

    // ── string ─────────────────────────────────────────────────

    #[test]
    fn string_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_string(0, 0), 0);
    }

    #[test]
    fn string_returns_reconstructed() {
        let obj = shim_nsurlcomponents_new(0, 0);
        shim_nsurlcomponents_set_scheme(obj, 0, make_string("http"));
        shim_nsurlcomponents_set_host(obj, 0, make_string("host"));
        shim_nsurlcomponents_set_path(obj, 0, make_string("/p"));
        let s = shim_nsurlcomponents_string(obj, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "http://host/p");
        free_obj(obj);
    }

    // ── description ────────────────────────────────────────────

    #[test]
    fn description_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_description(0, 0), 0);
    }

    // ── queryItems ─────────────────────────────────────────────

    #[test]
    fn query_items_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_query_items(0, 0), 0);
    }

    #[test]
    fn query_items_no_query_returns_empty_array() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let arr = shim_nsurlcomponents_query_items(obj, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn query_items_parses_pairs() {
        let obj = shim_nsurlcomponents_new(0, 0);
        shim_nsurlcomponents_set_query(obj, 0, make_string("a=1&b=2"));
        let arr = shim_nsurlcomponents_query_items(obj, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 2);

        let item0 = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_string_eq(shim_nsurlqueryitem_name(item0, 0), "a");
        assert_string_eq(shim_nsurlqueryitem_value(item0, 0), "1");

        let item1 = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1);
        assert_string_eq(shim_nsurlqueryitem_name(item1, 0), "b");
        assert_string_eq(shim_nsurlqueryitem_value(item1, 0), "2");
        free_obj(obj);
    }

    #[test]
    fn query_items_name_only_no_value() {
        let obj = shim_nsurlcomponents_new(0, 0);
        shim_nsurlcomponents_set_query(obj, 0, make_string("flag"));
        let arr = shim_nsurlcomponents_query_items(obj, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        let item = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_string_eq(shim_nsurlqueryitem_name(item, 0), "flag");
        assert_eq!(shim_nsurlqueryitem_value(item, 0), 0);
        free_obj(obj);
    }

    // ── setQueryItems: ─────────────────────────────────────────

    #[test]
    fn set_query_items_nil_obj() {
        assert_eq!(shim_nsurlcomponents_set_query_items(0, 0, 0), 0);
    }

    #[test]
    fn set_query_items_nil_items_clears_query() {
        let obj = shim_nsurlcomponents_new(0, 0);
        shim_nsurlcomponents_set_query(obj, 0, make_string("old=val"));
        shim_nsurlcomponents_set_query_items(obj, 0, 0);
        assert_eq!(read_field(obj, QUERY_OFF), 0);
        free_obj(obj);
    }

    #[test]
    fn set_query_items_builds_query_string() {
        let obj = shim_nsurlcomponents_new(0, 0);

        let qi1 = shim_nsurlqueryitem_with_name_value(0, 0, make_string("x"), make_string("10"));
        let qi2 = shim_nsurlqueryitem_with_name_value(0, 0, make_string("y"), make_string("20"));
        let items = crate::nsarray::make_nsarray(&[qi1, qi2]);

        shim_nsurlcomponents_set_query_items(obj, 0, items);
        assert_string_eq(read_field(obj, QUERY_OFF), "x=10&y=20");
        free_obj(obj);
    }

    #[test]
    fn set_query_items_name_only() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let qi = shim_nsurlqueryitem_with_name_value(0, 0, make_string("solo"), 0);
        let items = crate::nsarray::make_nsarray(&[qi]);
        shim_nsurlcomponents_set_query_items(obj, 0, items);
        assert_string_eq(read_field(obj, QUERY_OFF), "solo");
        free_obj(obj);
    }

    // ── NSURLQueryItem ─────────────────────────────────────────

    #[test]
    fn queryitem_with_name_value() {
        let name = make_string("key");
        let val = make_string("val");
        let qi = shim_nsurlqueryitem_with_name_value(0, 0, name, val);
        assert_ne!(qi, 0);
        assert_string_eq(shim_nsurlqueryitem_name(qi, 0), "key");
        assert_string_eq(shim_nsurlqueryitem_value(qi, 0), "val");
        free_obj(qi);
    }

    #[test]
    fn queryitem_nil_value() {
        let qi = shim_nsurlqueryitem_with_name_value(0, 0, make_string("k"), 0);
        assert_ne!(qi, 0);
        assert_string_eq(shim_nsurlqueryitem_name(qi, 0), "k");
        assert_eq!(shim_nsurlqueryitem_value(qi, 0), 0);
        free_obj(qi);
    }

    #[test]
    fn queryitem_name_nil_returns_0() {
        assert_eq!(shim_nsurlqueryitem_name(0, 0), 0);
    }

    #[test]
    fn queryitem_value_nil_returns_0() {
        assert_eq!(shim_nsurlqueryitem_value(0, 0), 0);
    }

    #[test]
    fn queryitem_description_nil_returns_0() {
        assert_eq!(shim_nsurlqueryitem_description(0, 0), 0);
    }

    #[test]
    fn queryitem_description_format() {
        let qi =
            shim_nsurlqueryitem_with_name_value(0, 0, make_string("color"), make_string("red"));
        let desc = shim_nsurlqueryitem_description(qi, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "color=red");
        free_obj(qi);
    }

    #[test]
    fn queryitem_description_nil_value() {
        let qi = shim_nsurlqueryitem_with_name_value(0, 0, make_string("flag"), 0);
        let desc = shim_nsurlqueryitem_description(qi, 0);
        assert_string_eq(desc, "flag=(null)");
        free_obj(qi);
    }

    // ── Roundtrip ──────────────────────────────────────────────

    #[test]
    fn parse_then_reconstruct_roundtrip() {
        let url = make_string("https://api.example.com/v2/items?sort=name&limit=10#results");
        let obj = shim_nsurlcomponents_with_string(0, 0, url);
        let s = shim_nsurlcomponents_string(obj, 0);
        assert_ne!(s, 0);
        assert_string_eq(
            s,
            "https://api.example.com/v2/items?sort=name&limit=10#results",
        );
        free_obj(obj);
    }

    #[test]
    fn with_string_scheme_only() {
        let url = make_string("ftp://");
        let obj = shim_nsurlcomponents_with_string(0, 0, url);
        assert_ne!(obj, 0);
        assert_string_eq(read_field(obj, SCHEME_OFF), "ftp");
        // host/path/etc should be nil
        assert_eq!(read_field(obj, HOST_OFF), 0);
        assert_eq!(read_field(obj, PATH_OFF), 0);
        free_obj(obj);
    }

    // ── percent-encoded accessors ──────────────────────────────

    #[test]
    fn percent_encoded_query_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_percent_encoded_query(0, 0), 0);
    }

    #[test]
    fn percent_encoded_query_returns_query() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let q = make_string("a%20b=1");
        shim_nsurlcomponents_set_percent_encoded_query(obj, 0, q);
        assert_string_eq(
            shim_nsurlcomponents_percent_encoded_query(obj, 0),
            "a%20b=1",
        );
        // Also visible through regular query getter
        assert_string_eq(shim_nsurlcomponents_query(obj, 0), "a%20b=1");
        free_obj(obj);
    }

    #[test]
    fn set_percent_encoded_query_nil_obj() {
        // Must not crash
        shim_nsurlcomponents_set_percent_encoded_query(0, 0, 0);
    }

    #[test]
    fn percent_encoded_path_nil_returns_0() {
        assert_eq!(shim_nsurlcomponents_percent_encoded_path(0, 0), 0);
    }

    #[test]
    fn percent_encoded_path_returns_path() {
        let obj = shim_nsurlcomponents_new(0, 0);
        let p = make_string("/foo%20bar/baz");
        shim_nsurlcomponents_set_percent_encoded_path(obj, 0, p);
        assert_string_eq(
            shim_nsurlcomponents_percent_encoded_path(obj, 0),
            "/foo%20bar/baz",
        );
        // Also visible through regular path getter
        assert_string_eq(shim_nsurlcomponents_path(obj, 0), "/foo%20bar/baz");
        free_obj(obj);
    }

    #[test]
    fn set_percent_encoded_path_nil_obj() {
        // Must not crash
        shim_nsurlcomponents_set_percent_encoded_path(0, 0, 0);
    }
}
