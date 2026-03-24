//! NSURL shim: URL representation.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa         (u64 — pointer to NSURL class)
//!   offset  8: string_ptr  (u64 — pointer to the NSString holding the full URL)
//!   offset 16: is_file_url (u64 — 1 if file URL, 0 otherwise)
//!
//! Total: 24 bytes per URL object.
//!
//! This is a minimal implementation sufficient for file-path-based URLs
//! and simple URL string manipulation. Not a full RFC 3986 parser.

use crate::cfstring;
use crate::nsobj;

const STRING_OFF: usize = 8;
const IS_FILE_OFF: usize = 16;

// ── Helpers ────────────────────────────────────────────────────────

fn url_string_ptr(obj: u64) -> u64 {
    unsafe { *((obj as usize + STRING_OFF) as *const u64) }
}

fn url_is_file(obj: u64) -> bool {
    unsafe { *((obj as usize + IS_FILE_OFF) as *const u64) != 0 }
}

fn alloc_url(url_string: u64, is_file: bool) -> u64 {
    let obj = unsafe { libc::calloc(1, 24) } as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurl_class_addr();
        *((obj as usize + STRING_OFF) as *mut u64) = url_string;
        *((obj as usize + IS_FILE_OFF) as *mut u64) = is_file as u64;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSURL fileURLWithPath:]` — creates a file URL from a path string.
pub extern "C" fn shim_nsurl_file_url_with_path(_cls: u64, _cmd: u64, path: u64) -> u64 {
    if path == 0 {
        return 0;
    }
    // Get the path string and prepend file://
    if let Some(path_str) = cfstring::extract_cfstring_cstr(path) {
        let url_str = format!("file://{}", path_str);
        let c_str = std::ffi::CString::new(url_str).unwrap();
        let ns_url_string = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        alloc_url(ns_url_string, true)
    } else {
        0
    }
}

/// `+[NSURL URLWithString:]` — creates a URL from a string.
pub extern "C" fn shim_nsurl_url_with_string(_cls: u64, _cmd: u64, string: u64) -> u64 {
    if string == 0 {
        return 0;
    }
    if let Some(s) = cfstring::extract_cfstring_cstr(string) {
        let is_file = s.starts_with("file://");
        alloc_url(string, is_file)
    } else {
        alloc_url(string, false)
    }
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSURL absoluteString]` — returns the full URL as a string.
pub extern "C" fn shim_nsurl_absolute_string(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    url_string_ptr(obj)
}

/// `-[NSURL path]` — returns the path component (strips "file://").
pub extern "C" fn shim_nsurl_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let str_ptr = url_string_ptr(obj);
    if let Some(s) = cfstring::extract_cfstring_cstr(str_ptr) {
        let path = if let Some(rest) = s.strip_prefix("file://") {
            rest.to_string()
        } else if let Some(idx) = s.find("://") {
            // Generic URL: skip scheme://host, take path from next /
            let after_scheme = &s[idx + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                after_scheme[slash_idx..].to_string()
            } else {
                String::new()
            }
        } else {
            s.to_string()
        };
        let c_str = std::ffi::CString::new(path).unwrap();
        cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
    } else {
        0
    }
}

/// `-[NSURL lastPathComponent]` — returns the last path component.
pub extern "C" fn shim_nsurl_last_path_component(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let str_ptr = url_string_ptr(obj);
    if let Some(s) = cfstring::extract_cfstring_cstr(str_ptr) {
        let offset = if s.starts_with("file://") { 7 } else { 0 };
        let path = &s[offset..];
        let component = path.rsplit('/').next().unwrap_or(path);
        let c_str = std::ffi::CString::new(component).unwrap();
        cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
    } else {
        0
    }
}

/// `-[NSURL pathExtension]` — returns the file extension.
pub extern "C" fn shim_nsurl_path_extension(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let str_ptr = url_string_ptr(obj);
    if let Some(s) = cfstring::extract_cfstring_cstr(str_ptr) {
        let offset = if s.starts_with("file://") { 7 } else { 0 };
        let path = &s[offset..];
        let last = path.rsplit('/').next().unwrap_or(path);
        let ext = match last.rfind('.') {
            Some(dot) if dot > 0 => &last[dot + 1..],
            _ => "",
        };
        let c_str = std::ffi::CString::new(ext).unwrap();
        cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
    } else {
        0
    }
}

/// `-[NSURL URLByAppendingPathComponent:]` — returns a new URL with the component appended.
pub extern "C" fn shim_nsurl_url_by_appending_path_component(
    obj: u64,
    _cmd: u64,
    component: u64,
) -> u64 {
    if obj == 0 || component == 0 {
        return 0;
    }
    let str_ptr = url_string_ptr(obj);
    if let (Some(base), Some(comp)) = (
        cfstring::extract_cfstring_cstr(str_ptr),
        cfstring::extract_cfstring_cstr(component),
    ) {
        let sep = if base.ends_with('/') { "" } else { "/" };
        let new_url = format!("{}{}{}", base, sep, comp);
        let c_str = std::ffi::CString::new(new_url).unwrap();
        let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        alloc_url(ns_str, url_is_file(obj))
    } else {
        0
    }
}

/// `-[NSURL isFileURL]` — returns YES if this is a file URL.
pub extern "C" fn shim_nsurl_is_file_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    url_is_file(obj) as u64
}

/// `-[NSURL scheme]` — returns the URL scheme (e.g. "file", "https").
pub extern "C" fn shim_nsurl_scheme(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let str_ptr = url_string_ptr(obj);
    if let Some(s) = cfstring::extract_cfstring_cstr(str_ptr) {
        if let Some(idx) = s.find("://") {
            let scheme = &s[..idx];
            let c_str = std::ffi::CString::new(scheme).unwrap();
            return cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        }
    }
    0
}

/// `-[NSURL description]` — returns the URL string.
pub extern "C" fn shim_nsurl_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    url_string_ptr(obj)
}
