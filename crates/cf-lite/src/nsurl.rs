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

#[cfg(target_os = "macos")]
use std::ffi::c_char;
use std::ffi::CString;

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

fn is_synthetic_url(obj: u64) -> bool {
    obj != 0 && normalize_isa(read_isa(obj)) == nsobj::nsurl_class_addr()
}

#[cfg(target_os = "macos")]
fn host_cf_symbol(name: &[u8]) -> *mut libc::c_void {
    static COREFOUNDATION_HANDLE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

    let handle = *COREFOUNDATION_HANDLE.get_or_init(|| unsafe {
        libc::dlopen(
            c"/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation".as_ptr(),
            libc::RTLD_LAZY,
        ) as usize
    }) as *mut libc::c_void;

    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let symbol = unsafe { libc::dlsym(handle, name.as_ptr() as *const c_char) };
    if symbol.is_null() {
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr() as *const c_char) }
    } else {
        symbol
    }
}

#[cfg(target_os = "macos")]
fn host_cf_release(obj: u64) {
    type HostCFReleaseFn = unsafe extern "C" fn(u64);

    if obj == 0 {
        return;
    }

    let symbol = host_cf_symbol(b"CFRelease\0");
    if symbol.is_null() {
        return;
    }

    let cf_release: HostCFReleaseFn = unsafe { std::mem::transmute(symbol) };
    unsafe { cf_release(obj) };
}

#[cfg(target_os = "macos")]
fn host_cfstring_to_rust(obj: u64) -> Option<String> {
    type HostCFStringGetLengthFn = unsafe extern "C" fn(u64) -> i64;
    type HostCFStringGetCStringFn = unsafe extern "C" fn(u64, *mut c_char, i64, u32) -> u8;

    if obj == 0 {
        return None;
    }

    let get_length_sym = host_cf_symbol(b"CFStringGetLength\0");
    let get_cstring_sym = host_cf_symbol(b"CFStringGetCString\0");
    if get_length_sym.is_null() || get_cstring_sym.is_null() {
        return None;
    }

    let cf_string_get_length: HostCFStringGetLengthFn =
        unsafe { std::mem::transmute(get_length_sym) };
    let cf_string_get_cstring: HostCFStringGetCStringFn =
        unsafe { std::mem::transmute(get_cstring_sym) };

    let len = unsafe { cf_string_get_length(obj) };
    if len < 0 {
        return None;
    }

    let capacity = len.saturating_mul(4).saturating_add(1) as usize;
    let mut buffer = vec![0 as c_char; capacity.max(1)];
    let ok = unsafe {
        cf_string_get_cstring(
            obj,
            buffer.as_mut_ptr(),
            buffer.len() as i64,
            crate::cfstring::K_CF_STRING_ENCODING_UTF8,
        )
    };
    if ok == 0 {
        return None;
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };
    c_str.to_str().ok().map(str::to_string)
}

#[cfg(target_os = "macos")]
fn host_absolute_url_string(obj: u64) -> Option<String> {
    type HostCFURLGetStringFn = unsafe extern "C" fn(u64) -> u64;

    let symbol = host_cf_symbol(b"CFURLGetString\0");
    if symbol.is_null() {
        return None;
    }

    let cf_url_get_string: HostCFURLGetStringFn = unsafe { std::mem::transmute(symbol) };
    let host_string = unsafe { cf_url_get_string(obj) };
    host_cfstring_to_rust(host_string)
}

#[cfg(target_os = "macos")]
fn host_url_path_string(obj: u64) -> Option<String> {
    type HostCFURLCopyFileSystemPathFn = unsafe extern "C" fn(u64, i64) -> u64;

    let symbol = host_cf_symbol(b"CFURLCopyFileSystemPath\0");
    if symbol.is_null() {
        return None;
    }

    let cf_url_copy_file_system_path: HostCFURLCopyFileSystemPathFn =
        unsafe { std::mem::transmute(symbol) };
    let host_path = unsafe { cf_url_copy_file_system_path(obj, 0) };
    if host_path == 0 {
        return None;
    }

    let result = host_cfstring_to_rust(host_path);
    host_cf_release(host_path);
    result
}

#[cfg(not(target_os = "macos"))]
fn host_absolute_url_string(_obj: u64) -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
fn host_url_path_string(_obj: u64) -> Option<String> {
    None
}

fn extract_absolute_url_string(obj: u64) -> Option<String> {
    if is_synthetic_url(obj) {
        return cfstring::extract_cfstring_cstr(url_string_ptr(obj)).map(str::to_string);
    }

    host_absolute_url_string(obj)
}

fn extract_url_path_string(obj: u64) -> Option<String> {
    if is_synthetic_url(obj) {
        let s = cfstring::extract_cfstring_cstr(url_string_ptr(obj))?;
        return Some(if let Some(rest) = s.strip_prefix("file://") {
            rest.to_string()
        } else if let Some(idx) = s.find("://") {
            let after_scheme = &s[idx + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                after_scheme[slash_idx..].to_string()
            } else {
                String::new()
            }
        } else {
            s.to_string()
        });
    }

    host_url_path_string(obj).or_else(|| {
        let s = host_absolute_url_string(obj)?;
        Some(if let Some(rest) = s.strip_prefix("file://") {
            rest.to_string()
        } else if let Some(idx) = s.find("://") {
            let after_scheme = &s[idx + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                after_scheme[slash_idx..].to_string()
            } else {
                String::new()
            }
        } else {
            s
        })
    })
}

fn alloc_url(url_string: u64, is_file: bool) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
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

fn build_file_url_string(path_str: &str, is_directory: Option<bool>) -> String {
    let mut normalized = path_str.to_string();

    match is_directory {
        Some(true) => {
            if normalized != "/" && !normalized.ends_with('/') {
                normalized.push('/');
            }
        }
        Some(false) => {
            while normalized.len() > 1 && normalized.ends_with('/') {
                normalized.pop();
            }
        }
        None => {}
    }

    format!("file://{}", normalized)
}

fn make_file_url(path: u64, is_directory: Option<bool>) -> u64 {
    if path == 0 {
        return 0;
    }

    if let Some(path_str) = cfstring::extract_cfstring_cstr(path) {
        let url_str = build_file_url_string(path_str, is_directory);
        let c_str = std::ffi::CString::new(url_str).unwrap_or_default();
        let ns_url_string = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        alloc_url(ns_url_string, true)
    } else {
        0
    }
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSURL fileURLWithPath:]` — creates a file URL from a path string.
pub extern "C" fn shim_nsurl_file_url_with_path(_cls: u64, _cmd: u64, path: u64) -> u64 {
    make_file_url(path, None)
}

/// `+[NSURL fileURLWithPath:isDirectory:]` — creates a file URL and preserves directory intent.
pub extern "C" fn shim_nsurl_file_url_with_path_is_directory(
    _cls: u64,
    _cmd: u64,
    path: u64,
    is_directory: u8,
) -> u64 {
    make_file_url(path, Some(is_directory != 0))
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
    if is_synthetic_url(obj) {
        return url_string_ptr(obj);
    }

    let Some(s) = extract_absolute_url_string(obj) else {
        return 0;
    };
    let c_str = CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSURL path]` — returns the path component (strips "file://").
pub extern "C" fn shim_nsurl_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(path) = extract_url_path_string(obj) else {
        return 0;
    };
    let c_str = CString::new(path).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSURL lastPathComponent]` — returns the last path component.
pub extern "C" fn shim_nsurl_last_path_component(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(path) = extract_url_path_string(obj) else {
        return 0;
    };
    let component = path.rsplit('/').next().unwrap_or(path.as_str());
    let c_str = CString::new(component).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSURL pathExtension]` — returns the file extension.
pub extern "C" fn shim_nsurl_path_extension(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(path) = extract_url_path_string(obj) else {
        return 0;
    };
    let last = path.rsplit('/').next().unwrap_or(path.as_str());
    let ext = match last.rfind('.') {
        Some(dot) if dot > 0 => &last[dot + 1..],
        _ => "",
    };
    let c_str = CString::new(ext).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
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
    if let (Some(base), Some(comp)) = (
        extract_absolute_url_string(obj),
        cfstring::extract_cfstring_cstr(component),
    ) {
        let sep = if base.ends_with('/') { "" } else { "/" };
        let new_url = format!("{}{}{}", base, sep, comp);
        let c_str = CString::new(new_url).unwrap_or_default();
        let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        alloc_url(
            ns_str,
            if is_synthetic_url(obj) {
                url_is_file(obj)
            } else {
                base.starts_with("file://")
            },
        )
    } else {
        0
    }
}

/// `-[NSURL isFileURL]` — returns YES if this is a file URL.
pub extern "C" fn shim_nsurl_is_file_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if is_synthetic_url(obj) {
        return url_is_file(obj) as u64;
    }
    extract_absolute_url_string(obj)
        .map(|s| u64::from(s.starts_with("file://")))
        .unwrap_or(0)
}

/// `-[NSURL scheme]` — returns the URL scheme (e.g. "file", "https").
pub extern "C" fn shim_nsurl_scheme(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if let Some(s) = extract_absolute_url_string(obj) {
        if let Some(idx) = s.find("://") {
            let scheme = &s[..idx];
            let c_str = CString::new(scheme).unwrap_or_default();
            return cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
        }
    }
    0
}

/// `-[NSURL description]` — returns the URL string.
pub extern "C" fn shim_nsurl_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsurl_absolute_string(obj, _cmd)
}

/// `-[NSURL standardizedURL]`
pub extern "C" fn shim_nsurl_standardized_url(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSURL absoluteURL]` — returns self since our URLs are always absolute.
pub extern "C" fn shim_nsurl_absolute_url(obj: u64, _cmd: u64) -> u64 {
    obj
}

/// `-[NSURL URLByDeletingLastPathComponent]` — returns a new URL with the last path component removed.
pub extern "C" fn shim_nsurl_url_by_deleting_last_path_component(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(url_str) = extract_absolute_url_string(obj) else {
        return 0;
    };
    let is_file = if is_synthetic_url(obj) {
        url_is_file(obj)
    } else {
        url_str.starts_with("file://")
    };

    let new_url = if let Some(rest) = url_str.strip_prefix("file://") {
        let parent = std::path::Path::new(rest)
            .parent()
            .unwrap_or(std::path::Path::new("/"));
        let mut p = parent.to_string_lossy().to_string();
        if p != "/" && !p.ends_with('/') {
            p.push('/');
        }
        format!("file://{}", p)
    } else if let Some(scheme_end) = url_str.find("://") {
        let after_scheme = &url_str[scheme_end + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            let authority = &after_scheme[..slash_idx];
            let path_part = &after_scheme[slash_idx..];
            let parent = std::path::Path::new(path_part)
                .parent()
                .unwrap_or(std::path::Path::new("/"));
            let mut p = parent.to_string_lossy().to_string();
            if p != "/" && !p.ends_with('/') {
                p.push('/');
            }
            format!("{}://{}{}", &url_str[..scheme_end], authority, p)
        } else {
            format!("{}://{}/", &url_str[..scheme_end], after_scheme)
        }
    } else {
        let parent = std::path::Path::new(&url_str)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        parent.to_string_lossy().to_string()
    };

    let c_str = CString::new(new_url).unwrap_or_default();
    let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    alloc_url(ns_str, is_file)
}

/// `-[NSURL URLByDeletingPathExtension]` — returns a new URL with the path extension removed.
pub extern "C" fn shim_nsurl_url_by_deleting_path_extension(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(url_str) = extract_absolute_url_string(obj) else {
        return 0;
    };
    let is_file = if is_synthetic_url(obj) {
        url_is_file(obj)
    } else {
        url_str.starts_with("file://")
    };

    let new_url = if let Some(rest) = url_str.strip_prefix("file://") {
        let no_ext = std::path::Path::new(rest)
            .with_extension("")
            .to_string_lossy()
            .to_string();
        format!("file://{}", no_ext)
    } else if let Some(scheme_end) = url_str.find("://") {
        let after_scheme = &url_str[scheme_end + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            let authority = &after_scheme[..slash_idx];
            let path_part = &after_scheme[slash_idx..];
            let no_ext = std::path::Path::new(path_part)
                .with_extension("")
                .to_string_lossy()
                .to_string();
            format!("{}://{}{}", &url_str[..scheme_end], authority, no_ext)
        } else {
            url_str.clone()
        }
    } else {
        std::path::Path::new(&url_str)
            .with_extension("")
            .to_string_lossy()
            .to_string()
    };

    let c_str = CString::new(new_url).unwrap_or_default();
    let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    alloc_url(ns_str, is_file)
}

/// `-[NSURL URLByAppendingPathExtension:]` — returns a new URL with the extension appended.
pub extern "C" fn shim_nsurl_url_by_appending_path_extension(obj: u64, _cmd: u64, ext: u64) -> u64 {
    if obj == 0 || ext == 0 {
        return 0;
    }
    let Some(url_str) = extract_absolute_url_string(obj) else {
        return 0;
    };
    let Some(ext_str) = cfstring::extract_cfstring_cstr(ext) else {
        return 0;
    };
    let is_file = if is_synthetic_url(obj) {
        url_is_file(obj)
    } else {
        url_str.starts_with("file://")
    };

    let new_url = format!("{}.{}", url_str.trim_end_matches('/'), ext_str);
    let c_str = CString::new(new_url).unwrap_or_default();
    let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    alloc_url(ns_str, is_file)
}

/// `-[NSURL pathComponents]` — returns path components as an NSArray.
pub extern "C" fn shim_nsurl_path_components(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(path) = extract_url_path_string(obj) else {
        return 0;
    };

    let mut components: Vec<u64> = Vec::new();

    if path.starts_with('/') {
        components.push(cfstring::shim_cf_string_create_with_cstring(
            0,
            c"/".as_ptr(),
            0,
        ));
    }

    for part in path.split('/').filter(|s| !s.is_empty()) {
        let c_str = CString::new(part).unwrap_or_default();
        components.push(cfstring::shim_cf_string_create_with_cstring(
            0,
            c_str.as_ptr(),
            0,
        ));
    }

    crate::nsarray::make_nsarray(&components)
}

/// `-[NSURL checkResourceIsReachableAndReturnError:]` — checks if URL resource exists.
pub extern "C" fn shim_nsurl_check_resource_is_reachable(obj: u64, _cmd: u64, _error: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let Some(path) = extract_url_path_string(obj) else {
        return 0;
    };
    let is_file = if is_synthetic_url(obj) {
        url_is_file(obj)
    } else {
        extract_absolute_url_string(obj)
            .map(|s| s.starts_with("file://"))
            .unwrap_or(false)
    };
    if !is_file {
        return 0;
    }
    u8::from(std::path::Path::new(path.as_str()).exists())
}

/// `-[NSURL initWithString:]` — initializes a URL from a string.
pub extern "C" fn shim_nsurl_init_with_string(obj: u64, _cmd: u64, str_arg: u64) -> u64 {
    if obj == 0 || str_arg == 0 {
        return 0;
    }
    let is_file = cfstring::extract_cfstring_cstr(str_arg)
        .map(|s| s.starts_with("file://"))
        .unwrap_or(false);
    unsafe {
        *(obj as *mut u64) = nsobj::nsurl_class_addr();
        *((obj as usize + STRING_OFF) as *mut u64) = str_arg;
        *((obj as usize + IS_FILE_OFF) as *mut u64) = is_file as u64;
    }
    obj
}

/// `-[NSURL initWithString:relativeToURL:]` — initializes a URL from a string relative to a base.
pub extern "C" fn shim_nsurl_init_with_string_relative(
    obj: u64,
    _cmd: u64,
    str_arg: u64,
    base: u64,
) -> u64 {
    if obj == 0 || str_arg == 0 {
        return 0;
    }
    // If base is nil or string already has a scheme, treat as absolute
    if base == 0 {
        return shim_nsurl_init_with_string(obj, _cmd, str_arg);
    }

    let Some(str_val) = cfstring::extract_cfstring_cstr(str_arg) else {
        return 0;
    };
    // If the string already contains a scheme, use it directly
    if str_val.contains("://") {
        return shim_nsurl_init_with_string(obj, _cmd, str_arg);
    }

    // Resolve relative to base
    let Some(base_str) = extract_absolute_url_string(base) else {
        return shim_nsurl_init_with_string(obj, _cmd, str_arg);
    };

    let resolved = if str_val.starts_with('/') {
        // Absolute path relative to base authority
        if let Some(scheme_end) = base_str.find("://") {
            let after_scheme = &base_str[scheme_end + 3..];
            if let Some(slash_idx) = after_scheme.find('/') {
                format!("{}{}", &base_str[..scheme_end + 3 + slash_idx], str_val)
            } else {
                format!("{}{}", base_str, str_val)
            }
        } else {
            str_val.to_string()
        }
    } else {
        // Relative path
        let base_trimmed = if base_str.ends_with('/') {
            base_str.clone()
        } else if let Some(last_slash) = base_str.rfind('/') {
            base_str[..=last_slash].to_string()
        } else {
            format!("{}/", base_str)
        };
        format!("{}{}", base_trimmed, str_val)
    };

    let c_str = CString::new(resolved).unwrap_or_default();
    let ns_str = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
    let is_file = cfstring::extract_cfstring_cstr(ns_str)
        .map(|s| s.starts_with("file://"))
        .unwrap_or(false);
    unsafe {
        *(obj as *mut u64) = nsobj::nsurl_class_addr();
        *((obj as usize + STRING_OFF) as *mut u64) = ns_str;
        *((obj as usize + IS_FILE_OFF) as *mut u64) = is_file as u64;
    }
    obj
}

/// `-[NSURL resourceSpecifier]` — returns everything after the scheme: part.
pub extern "C" fn shim_nsurl_resource_specifier(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let Some(url_str) = extract_absolute_url_string(obj) else {
        return 0;
    };
    // resourceSpecifier is everything after "scheme:", i.e. includes the "//"
    let spec = if let Some(colon_idx) = url_str.find(':') {
        &url_str[colon_idx + 1..]
    } else {
        &url_str
    };
    let c_str = CString::new(spec).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[cfg(target_os = "macos")]
    fn host_cf_symbol(name: &[u8]) -> *mut libc::c_void {
        super::host_cf_symbol(name)
    }

    #[cfg(target_os = "macos")]
    fn host_cf_release(obj: u64) {
        super::host_cf_release(obj)
    }

    #[cfg(target_os = "macos")]
    fn host_cfurl_create_with_file_system_path(path: &str, is_directory: bool) -> u64 {
        type HostCFStringCreateWithCStringFn = unsafe extern "C" fn(u64, *const c_char, u32) -> u64;
        type HostCFURLCreateWithFileSystemPathFn = unsafe extern "C" fn(u64, u64, i64, u8) -> u64;

        let cf_string_create: HostCFStringCreateWithCStringFn =
            unsafe { std::mem::transmute(host_cf_symbol(b"CFStringCreateWithCString\0")) };
        let cf_url_create_with_file_system_path: HostCFURLCreateWithFileSystemPathFn =
            unsafe { std::mem::transmute(host_cf_symbol(b"CFURLCreateWithFileSystemPath\0")) };

        let cstr = CString::new(path).unwrap();
        let host_path = unsafe {
            cf_string_create(0, cstr.as_ptr(), crate::cfstring::K_CF_STRING_ENCODING_UTF8)
        };
        let url =
            unsafe { cf_url_create_with_file_system_path(0, host_path, 0, u8::from(is_directory)) };
        host_cf_release(host_path);
        url
    }

    // ── Construction helpers ───────────────────────────────────────

    /// Build an NSURL via +[NSURL URLWithString:] from a &str.
    fn url_from_str(s: &str) -> u64 {
        let ns = make_string(s);
        shim_nsurl_url_with_string(0, 0, ns)
    }

    /// Build a file URL via +[NSURL fileURLWithPath:] from a &str.
    fn file_url_from_str(path: &str) -> u64 {
        let ns = make_string(path);
        shim_nsurl_file_url_with_path(0, 0, ns)
    }

    // ── +[NSURL URLWithString:] ────────────────────────────────────

    #[test]
    fn url_with_string_basic() {
        let url = url_from_str("https://example.com/foo");
        assert_ne!(url, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(url, 0),
            "https://example.com/foo",
        );
        free_obj(url);
    }

    #[test]
    fn url_with_string_nil() {
        assert_eq!(shim_nsurl_url_with_string(0, 0, 0), 0);
    }

    #[test]
    fn url_with_string_file_scheme_detected() {
        let url = url_from_str("file:///tmp/test.txt");
        assert_ne!(url, 0);
        assert_eq!(shim_nsurl_is_file_url(url, 0), 1);
        free_obj(url);
    }

    #[test]
    fn url_with_string_http_not_file() {
        let url = url_from_str("http://example.com");
        assert_ne!(url, 0);
        assert_eq!(shim_nsurl_is_file_url(url, 0), 0);
        free_obj(url);
    }

    #[test]
    fn url_with_string_empty() {
        let url = url_from_str("");
        assert_ne!(url, 0);
        // Should still carry the empty string
        assert_string_eq(shim_nsurl_absolute_string(url, 0), "");
        free_obj(url);
    }

    // ── +[NSURL fileURLWithPath:] ──────────────────────────────────

    #[test]
    fn file_url_with_path_basic() {
        let url = file_url_from_str("/tmp/hello.txt");
        assert_ne!(url, 0);
        assert_string_eq(shim_nsurl_absolute_string(url, 0), "file:///tmp/hello.txt");
        assert_eq!(shim_nsurl_is_file_url(url, 0), 1);
        free_obj(url);
    }

    #[test]
    fn file_url_with_path_nil() {
        assert_eq!(shim_nsurl_file_url_with_path(0, 0, 0), 0);
    }

    #[test]
    fn file_url_with_path_root() {
        let url = file_url_from_str("/");
        assert_ne!(url, 0);
        assert_string_eq(shim_nsurl_absolute_string(url, 0), "file:///");
        free_obj(url);
    }

    #[test]
    fn file_url_with_path_is_directory_adds_trailing_slash() {
        let path = make_string("/tmp/folder");
        let url = shim_nsurl_file_url_with_path_is_directory(0, 0, path, 1);
        assert_ne!(url, 0);
        assert_string_eq(shim_nsurl_absolute_string(url, 0), "file:///tmp/folder/");
        assert_eq!(shim_nsurl_is_file_url(url, 0), 1);
        free_obj(url);
    }

    #[test]
    fn file_url_with_path_is_directory_false_trims_extra_trailing_slash() {
        let path = make_string("/tmp/file.txt/");
        let url = shim_nsurl_file_url_with_path_is_directory(0, 0, path, 0);
        assert_ne!(url, 0);
        assert_string_eq(shim_nsurl_absolute_string(url, 0), "file:///tmp/file.txt");
        free_obj(url);
    }

    // ── -[NSURL absoluteString] ────────────────────────────────────

    #[test]
    fn absolute_string_nil() {
        assert_eq!(shim_nsurl_absolute_string(0, 0), 0);
    }

    #[test]
    fn absolute_string_roundtrip() {
        let url = url_from_str("https://user:pass@host:443/path?q=1#frag");
        assert_string_eq(
            shim_nsurl_absolute_string(url, 0),
            "https://user:pass@host:443/path?q=1#frag",
        );
        free_obj(url);
    }

    // ── -[NSURL path] ─────────────────────────────────────────────

    #[test]
    fn path_nil() {
        assert_eq!(shim_nsurl_path(0, 0), 0);
    }

    #[test]
    fn path_file_url() {
        let url = file_url_from_str("/usr/local/bin");
        let p = shim_nsurl_path(url, 0);
        assert_ne!(p, 0);
        assert_string_eq(p, "/usr/local/bin");
        free_obj(url);
    }

    #[test]
    fn path_http_url() {
        let url = url_from_str("https://example.com/a/b/c");
        let p = shim_nsurl_path(url, 0);
        assert_ne!(p, 0);
        assert_string_eq(p, "/a/b/c");
        free_obj(url);
    }

    #[test]
    fn path_http_url_no_path() {
        let url = url_from_str("https://example.com");
        let p = shim_nsurl_path(url, 0);
        assert_ne!(p, 0);
        // No slash after host -> empty path
        assert_string_eq(p, "");
        free_obj(url);
    }

    // ── -[NSURL lastPathComponent] ─────────────────────────────────

    #[test]
    fn last_path_component_nil() {
        assert_eq!(shim_nsurl_last_path_component(0, 0), 0);
    }

    #[test]
    fn last_path_component_file_url() {
        let url = file_url_from_str("/tmp/docs/README.md");
        assert_string_eq(shim_nsurl_last_path_component(url, 0), "README.md");
        free_obj(url);
    }

    #[test]
    fn last_path_component_http_url() {
        let url = url_from_str("https://example.com/a/b/index.html");
        assert_string_eq(shim_nsurl_last_path_component(url, 0), "index.html");
        free_obj(url);
    }

    #[test]
    fn last_path_component_root_file() {
        let url = file_url_from_str("/");
        // "file:///" -> last component after splitting by '/' is ""
        assert_string_eq(shim_nsurl_last_path_component(url, 0), "");
        free_obj(url);
    }

    // ── -[NSURL pathExtension] ─────────────────────────────────────

    #[test]
    fn path_extension_nil() {
        assert_eq!(shim_nsurl_path_extension(0, 0), 0);
    }

    #[test]
    fn path_extension_txt() {
        let url = file_url_from_str("/tmp/file.txt");
        assert_string_eq(shim_nsurl_path_extension(url, 0), "txt");
        free_obj(url);
    }

    #[test]
    fn path_extension_none() {
        let url = file_url_from_str("/tmp/Makefile");
        assert_string_eq(shim_nsurl_path_extension(url, 0), "");
        free_obj(url);
    }

    #[test]
    fn path_extension_double() {
        let url = file_url_from_str("/tmp/archive.tar.gz");
        assert_string_eq(shim_nsurl_path_extension(url, 0), "gz");
        free_obj(url);
    }

    #[test]
    fn path_extension_dotfile() {
        let url = file_url_from_str("/tmp/.hidden");
        // ".hidden" has a dot at position 0 — the code excludes dot==0
        assert_string_eq(shim_nsurl_path_extension(url, 0), "");
        free_obj(url);
    }

    // ── -[NSURL URLByAppendingPathComponent:] ──────────────────────

    #[test]
    fn append_path_component_nil_obj() {
        let comp = make_string("child");
        assert_eq!(shim_nsurl_url_by_appending_path_component(0, 0, comp), 0);
    }

    #[test]
    fn append_path_component_nil_component() {
        let url = file_url_from_str("/tmp");
        assert_eq!(shim_nsurl_url_by_appending_path_component(url, 0, 0), 0);
        free_obj(url);
    }

    #[test]
    fn append_path_component_basic() {
        let url = file_url_from_str("/tmp");
        let comp = make_string("test.txt");
        let result = shim_nsurl_url_by_appending_path_component(url, 0, comp);
        assert_ne!(result, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "file:///tmp/test.txt",
        );
        assert_eq!(shim_nsurl_is_file_url(result, 0), 1);
        free_obj(result);
        free_obj(url);
    }

    #[test]
    fn append_path_component_trailing_slash() {
        let url = url_from_str("file:///tmp/");
        let comp = make_string("data");
        let result = shim_nsurl_url_by_appending_path_component(url, 0, comp);
        assert_ne!(result, 0);
        // Base already ends with '/', so no double-slash
        assert_string_eq(shim_nsurl_absolute_string(result, 0), "file:///tmp/data");
        free_obj(result);
        free_obj(url);
    }

    // ── -[NSURL isFileURL] ─────────────────────────────────────────

    #[test]
    fn is_file_url_nil() {
        assert_eq!(shim_nsurl_is_file_url(0, 0), 0);
    }

    #[test]
    fn is_file_url_yes() {
        let url = file_url_from_str("/etc/hosts");
        assert_eq!(shim_nsurl_is_file_url(url, 0), 1);
        free_obj(url);
    }

    #[test]
    fn is_file_url_no() {
        let url = url_from_str("https://example.com");
        assert_eq!(shim_nsurl_is_file_url(url, 0), 0);
        free_obj(url);
    }

    // ── -[NSURL scheme] ───────────────────────────────────────────

    #[test]
    fn scheme_nil() {
        assert_eq!(shim_nsurl_scheme(0, 0), 0);
    }

    #[test]
    fn scheme_https() {
        let url = url_from_str("https://example.com/path");
        assert_string_eq(shim_nsurl_scheme(url, 0), "https");
        free_obj(url);
    }

    #[test]
    fn scheme_file() {
        let url = file_url_from_str("/tmp");
        assert_string_eq(shim_nsurl_scheme(url, 0), "file");
        free_obj(url);
    }

    #[test]
    fn scheme_custom() {
        let url = url_from_str("myapp://callback?code=123");
        assert_string_eq(shim_nsurl_scheme(url, 0), "myapp");
        free_obj(url);
    }

    #[test]
    fn scheme_no_scheme() {
        let url = url_from_str("just-a-string");
        assert_eq!(shim_nsurl_scheme(url, 0), 0);
        free_obj(url);
    }

    // ── -[NSURL description] ──────────────────────────────────────

    #[test]
    fn description_nil() {
        assert_eq!(shim_nsurl_description(0, 0), 0);
    }

    #[test]
    fn description_returns_url_string() {
        let url = url_from_str("https://example.com/test");
        let desc = shim_nsurl_description(url, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "https://example.com/test");
        free_obj(url);
    }

    #[test]
    fn standardized_url_returns_self() {
        let url = url_from_str("https://example.com/a/../b");
        assert_ne!(url, 0);
        assert_eq!(shim_nsurl_standardized_url(url, 0), url);
        free_obj(url);
    }

    // ── URL with all components ────────────────────────────────────

    #[test]
    fn full_url_all_components() {
        let url = url_from_str("https://user:pass@host.com:8080/path/to/resource?q=1&r=2#section");
        assert_ne!(url, 0);

        assert_string_eq(shim_nsurl_scheme(url, 0), "https");
        assert_string_eq(shim_nsurl_path(url, 0), "/path/to/resource?q=1&r=2#section");
        assert_string_eq(
            shim_nsurl_last_path_component(url, 0),
            "resource?q=1&r=2#section",
        );
        assert_eq!(shim_nsurl_is_file_url(url, 0), 0);

        free_obj(url);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn host_cfurl_path_bridge() {
        let url = host_cfurl_create_with_file_system_path("/tmp/host-url-bridge", false);
        assert_ne!(url, 0);

        let path = shim_nsurl_path(url, 0);
        assert_ne!(path, 0);
        assert_string_eq(path, "/tmp/host-url-bridge");

        host_cf_release(url);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn host_cfurl_absolute_string_bridge() {
        let url = host_cfurl_create_with_file_system_path("/tmp/host-url-absolute", false);
        assert_ne!(url, 0);

        let absolute = shim_nsurl_absolute_string(url, 0);
        assert_ne!(absolute, 0);
        assert_string_eq(absolute, "file:///tmp/host-url-absolute");

        host_cf_release(url);
    }

    // ── -[NSURL absoluteURL] ──────────────────────────────────────

    #[test]
    fn absolute_url_returns_self() {
        let url = url_from_str("https://example.com/path");
        assert_eq!(shim_nsurl_absolute_url(url, 0), url);
        free_obj(url);
    }

    #[test]
    fn absolute_url_nil() {
        assert_eq!(shim_nsurl_absolute_url(0, 0), 0);
    }

    // ── -[NSURL URLByDeletingLastPathComponent] ────────────────────

    #[test]
    fn delete_last_path_component_nil() {
        assert_eq!(shim_nsurl_url_by_deleting_last_path_component(0, 0), 0);
    }

    #[test]
    fn delete_last_path_component_file() {
        let url = file_url_from_str("/tmp/docs/README.md");
        let result = shim_nsurl_url_by_deleting_last_path_component(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(shim_nsurl_absolute_string(result, 0), "file:///tmp/docs/");
        free_obj(result);
        free_obj(url);
    }

    #[test]
    fn delete_last_path_component_root() {
        let url = file_url_from_str("/");
        let result = shim_nsurl_url_by_deleting_last_path_component(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(shim_nsurl_absolute_string(result, 0), "file:///");
        free_obj(result);
        free_obj(url);
    }

    #[test]
    fn delete_last_path_component_http() {
        let url = url_from_str("https://example.com/a/b/c");
        let result = shim_nsurl_url_by_deleting_last_path_component(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "https://example.com/a/b/",
        );
        free_obj(result);
        free_obj(url);
    }

    // ── -[NSURL URLByDeletingPathExtension] ────────────────────────

    #[test]
    fn delete_path_extension_nil() {
        assert_eq!(shim_nsurl_url_by_deleting_path_extension(0, 0), 0);
    }

    #[test]
    fn delete_path_extension_file() {
        let url = file_url_from_str("/tmp/file.txt");
        let result = shim_nsurl_url_by_deleting_path_extension(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(shim_nsurl_absolute_string(result, 0), "file:///tmp/file");
        free_obj(result);
        free_obj(url);
    }

    #[test]
    fn delete_path_extension_no_extension() {
        let url = file_url_from_str("/tmp/Makefile");
        let result = shim_nsurl_url_by_deleting_path_extension(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "file:///tmp/Makefile",
        );
        free_obj(result);
        free_obj(url);
    }

    #[test]
    fn delete_path_extension_double() {
        let url = file_url_from_str("/tmp/archive.tar.gz");
        let result = shim_nsurl_url_by_deleting_path_extension(url, 0);
        assert_ne!(result, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "file:///tmp/archive.tar",
        );
        free_obj(result);
        free_obj(url);
    }

    // ── -[NSURL URLByAppendingPathExtension:] ──────────────────────

    #[test]
    fn append_path_extension_nil_obj() {
        let ext = make_string("txt");
        assert_eq!(shim_nsurl_url_by_appending_path_extension(0, 0, ext), 0);
    }

    #[test]
    fn append_path_extension_nil_ext() {
        let url = file_url_from_str("/tmp/file");
        assert_eq!(shim_nsurl_url_by_appending_path_extension(url, 0, 0), 0);
        free_obj(url);
    }

    #[test]
    fn append_path_extension_basic() {
        let url = file_url_from_str("/tmp/file");
        let ext = make_string("txt");
        let result = shim_nsurl_url_by_appending_path_extension(url, 0, ext);
        assert_ne!(result, 0);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "file:///tmp/file.txt",
        );
        free_obj(result);
        free_obj(url);
    }

    // ── -[NSURL pathComponents] ────────────────────────────────────

    #[test]
    fn path_components_nil() {
        assert_eq!(shim_nsurl_path_components(0, 0), 0);
    }

    #[test]
    fn path_components_file_url() {
        let url = file_url_from_str("/tmp/docs/README.md");
        let arr = shim_nsurl_path_components(url, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 4);
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), "/");
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1),
            "tmp",
        );
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 2),
            "docs",
        );
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(arr, 0, 3),
            "README.md",
        );
        free_obj(url);
    }

    #[test]
    fn path_components_root() {
        let url = file_url_from_str("/");
        let arr = shim_nsurl_path_components(url, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        assert_string_eq(crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0), "/");
        free_obj(url);
    }

    // ── -[NSURL checkResourceIsReachableAndReturnError:] ───────────

    #[test]
    fn check_resource_reachable_nil() {
        assert_eq!(shim_nsurl_check_resource_is_reachable(0, 0, 0), 0);
    }

    #[test]
    fn check_resource_reachable_exists() {
        // /tmp always exists
        let url = file_url_from_str("/tmp");
        assert_eq!(shim_nsurl_check_resource_is_reachable(url, 0, 0), 1);
        free_obj(url);
    }

    #[test]
    fn check_resource_reachable_missing() {
        let url = file_url_from_str("/nonexistent_path_abc_xyz_123");
        assert_eq!(shim_nsurl_check_resource_is_reachable(url, 0, 0), 0);
        free_obj(url);
    }

    #[test]
    fn check_resource_reachable_not_file_url() {
        let url = url_from_str("https://example.com");
        assert_eq!(shim_nsurl_check_resource_is_reachable(url, 0, 0), 0);
        free_obj(url);
    }

    // ── -[NSURL initWithString:] ───────────────────────────────────

    #[test]
    fn init_with_string_nil_obj() {
        assert_eq!(
            shim_nsurl_init_with_string(0, 0, make_string("http://x")),
            0
        );
    }

    #[test]
    fn init_with_string_nil_str() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        assert_eq!(shim_nsurl_init_with_string(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_string_basic() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        let s = make_string("https://example.com/path");
        let result = shim_nsurl_init_with_string(obj, 0, s);
        assert_eq!(result, obj);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "https://example.com/path",
        );
        assert_eq!(shim_nsurl_is_file_url(result, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_string_file_url() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        let s = make_string("file:///tmp/test");
        let result = shim_nsurl_init_with_string(obj, 0, s);
        assert_eq!(result, obj);
        assert_eq!(shim_nsurl_is_file_url(result, 0), 1);
        free_obj(obj);
    }

    // ── -[NSURL initWithString:relativeToURL:] ─────────────────────

    #[test]
    fn init_with_string_relative_nil_obj() {
        assert_eq!(
            shim_nsurl_init_with_string_relative(0, 0, make_string("x"), 0),
            0
        );
    }

    #[test]
    fn init_with_string_relative_nil_str() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        assert_eq!(shim_nsurl_init_with_string_relative(obj, 0, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn init_with_string_relative_no_base() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        let s = make_string("https://example.com");
        let result = shim_nsurl_init_with_string_relative(obj, 0, s, 0);
        assert_eq!(result, obj);
        assert_string_eq(shim_nsurl_absolute_string(result, 0), "https://example.com");
        free_obj(obj);
    }

    #[test]
    fn init_with_string_relative_with_base() {
        let obj = alloc_obj(24, nsobj::nsurl_class_addr());
        let base = url_from_str("https://example.com/dir/");
        let s = make_string("page.html");
        let result = shim_nsurl_init_with_string_relative(obj, 0, s, base);
        assert_eq!(result, obj);
        assert_string_eq(
            shim_nsurl_absolute_string(result, 0),
            "https://example.com/dir/page.html",
        );
        free_obj(obj);
        free_obj(base);
    }

    // ── -[NSURL resourceSpecifier] ─────────────────────────────────

    #[test]
    fn resource_specifier_nil() {
        assert_eq!(shim_nsurl_resource_specifier(0, 0), 0);
    }

    #[test]
    fn resource_specifier_http() {
        let url = url_from_str("https://example.com/path?q=1");
        assert_string_eq(
            shim_nsurl_resource_specifier(url, 0),
            "//example.com/path?q=1",
        );
        free_obj(url);
    }

    #[test]
    fn resource_specifier_file() {
        let url = file_url_from_str("/tmp/file.txt");
        assert_string_eq(shim_nsurl_resource_specifier(url, 0), "///tmp/file.txt");
        free_obj(url);
    }

    #[test]
    fn resource_specifier_no_scheme() {
        let url = url_from_str("just-a-string");
        // No colon found → returns the entire string
        assert_string_eq(shim_nsurl_resource_specifier(url, 0), "just-a-string");
        free_obj(url);
    }
}
