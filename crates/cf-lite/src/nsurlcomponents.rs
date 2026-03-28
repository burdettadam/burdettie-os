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
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = nsobj::nsurlcomponents_class_addr(); }
    obj
}

fn set_str_field(obj: u64, off: usize, val: u64) {
    unsafe { *((obj as usize + off) as *mut u64) = val; }
}

fn get_str_field(obj: u64, off: usize) -> u64 {
    if obj == 0 { return 0; }
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

    if path_query_frag.is_empty() { return; }

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

    if !path.is_empty() { set_str_field(obj, PATH_OFF, make_nsstring(path)); }
    if let Some(q) = query { set_str_field(obj, QUERY_OFF, make_nsstring(q)); }
    if let Some(f) = fragment { set_str_field(obj, FRAGMENT_OFF, make_nsstring(f)); }
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSURLComponents componentsWithString:]`
pub extern "C" fn shim_nsurlcomponents_with_string(
    _cls: u64, _cmd: u64, url_string: u64,
) -> u64 {
    let obj = alloc_components();
    if obj == 0 || url_string == 0 { return obj; }
    if let Some(s) = cfstring::extract_cfstring_cstr(url_string) {
        parse_url(&s, obj);
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
    set_str_field(obj, SCHEME_OFF, val); 0
}
pub extern "C" fn shim_nsurlcomponents_set_host(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, HOST_OFF, val); 0
}
pub extern "C" fn shim_nsurlcomponents_set_port(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, PORT_OFF, val); 0
}
pub extern "C" fn shim_nsurlcomponents_set_path(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, PATH_OFF, val); 0
}
pub extern "C" fn shim_nsurlcomponents_set_query(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, QUERY_OFF, val); 0
}
pub extern "C" fn shim_nsurlcomponents_set_fragment(obj: u64, _cmd: u64, val: u64) -> u64 {
    set_str_field(obj, FRAGMENT_OFF, val); 0
}

/// `-[NSURLComponents URL]` — reconstruct URL from components.
pub extern "C" fn shim_nsurlcomponents_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let mut result = String::new();

    if let Some(scheme) = cfstring::extract_cfstring_cstr(get_str_field(obj, SCHEME_OFF)) {
        result.push_str(&scheme);
        result.push_str("://");
    }
    if let Some(host) = cfstring::extract_cfstring_cstr(get_str_field(obj, HOST_OFF)) {
        result.push_str(&host);
    }
    let port_ptr = get_str_field(obj, PORT_OFF);
    if port_ptr != 0 {
        let port = crate::nsnumber::shim_nsnumber_integer_value(port_ptr, 0);
        result.push_str(&format!(":{port}"));
    }
    if let Some(path) = cfstring::extract_cfstring_cstr(get_str_field(obj, PATH_OFF)) {
        result.push_str(&path);
    }
    if let Some(query) = cfstring::extract_cfstring_cstr(get_str_field(obj, QUERY_OFF)) {
        result.push('?');
        result.push_str(&query);
    }
    if let Some(frag) = cfstring::extract_cfstring_cstr(get_str_field(obj, FRAGMENT_OFF)) {
        result.push('#');
        result.push_str(&frag);
    }

    let ns = make_nsstring(&result);
    crate::nsurl::shim_nsurl_url_with_string(0, 0, ns)
}

/// `-[NSURLComponents string]`
pub extern "C" fn shim_nsurlcomponents_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let url = shim_nsurlcomponents_url(obj, 0);
    if url == 0 { return 0; }
    crate::nsurl::shim_nsurl_absolute_string(url, 0)
}

/// `-[NSURLComponents queryItems]` — returns NSArray of NSURLQueryItem.
pub extern "C" fn shim_nsurlcomponents_query_items(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let query_str = get_str_field(obj, QUERY_OFF);
    if query_str == 0 { return crate::nsarray::make_nsarray(&[]); }
    let Some(qs) = cfstring::extract_cfstring_cstr(query_str) else {
        return crate::nsarray::make_nsarray(&[]);
    };
    let mut items = Vec::new();
    for pair in qs.split('&') {
        if pair.is_empty() { continue; }
        let (name, value) = if let Some(eq) = pair.find('=') {
            (&pair[..eq], Some(&pair[eq + 1..]))
        } else {
            (pair, None)
        };
        let item = alloc_query_item(make_nsstring(name),
            value.map_or(0, |v| make_nsstring(v)));
        items.push(item);
    }
    crate::nsarray::make_nsarray(&items)
}

/// `-[NSURLComponents setQueryItems:]`
pub extern "C" fn shim_nsurlcomponents_set_query_items(
    obj: u64, _cmd: u64, items: u64,
) -> u64 {
    if obj == 0 { return 0; }
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
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlqueryitem_class_addr();
        *((obj as usize + QI_NAME_OFF) as *mut u64) = name;
        *((obj as usize + QI_VALUE_OFF) as *mut u64) = value;
    }
    obj
}

/// `+[NSURLQueryItem queryItemWithName:value:]`
pub extern "C" fn shim_nsurlqueryitem_with_name_value(
    _cls: u64, _cmd: u64, name: u64, value: u64,
) -> u64 {
    alloc_query_item(name, value)
}

pub extern "C" fn shim_nsurlqueryitem_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + QI_NAME_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlqueryitem_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + QI_VALUE_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlqueryitem_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = cfstring::extract_cfstring_cstr(shim_nsurlqueryitem_name(obj, 0))
        .unwrap_or_default();
    let val = cfstring::extract_cfstring_cstr(shim_nsurlqueryitem_value(obj, 0))
        .unwrap_or("(null)");
    let desc = format!("{name}={val}");
    make_nsstring(&desc)
}
