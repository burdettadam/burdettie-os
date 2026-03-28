//! NSURLRequest / NSMutableURLRequest / NSURLResponse / NSHTTPURLResponse shims.
//!
//! NSURLRequest layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: url_ptr      (u64 — NSURL)
//!   offset 16: method_ptr   (u64 — NSString, "GET" default)
//!   offset 24: headers_ptr  (u64 — NSDictionary, 0=nil)
//!
//! NSURLResponse layout (32 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: url_ptr      (u64 — NSURL)
//!   offset 16: mime_ptr     (u64 — NSString)
//!   offset 24: length       (i64 — expected content length, -1=unknown)
//!
//! NSHTTPURLResponse layout (40 bytes):
//!   offset  0..32: same as NSURLResponse
//!   offset 32: status_code  (i64)

use crate::cfstring;
use crate::nsobj;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ═══════════════════════════════════════════════════════════════
// NSURLRequest
// ═══════════════════════════════════════════════════════════════

const REQ_URL_OFF: usize = 8;
const REQ_METHOD_OFF: usize = 16;
const REQ_HEADERS_OFF: usize = 24;

/// `+[NSURLRequest requestWithURL:]`
pub extern "C" fn shim_nsurlrequest_with_url(_cls: u64, _cmd: u64, url: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlrequest_class_addr();
        *((obj as usize + REQ_URL_OFF) as *mut u64) = url;
        *((obj as usize + REQ_METHOD_OFF) as *mut u64) = make_nsstring("GET");
    }
    obj
}

pub extern "C" fn shim_nsurlrequest_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_URL_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlrequest_http_method(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return make_nsstring("GET"); }
    unsafe { *((obj as usize + REQ_METHOD_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlrequest_all_http_header_fields(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_HEADERS_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlrequest_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let method = cfstring::extract_cfstring_cstr(shim_nsurlrequest_http_method(obj, 0))
        .unwrap_or("GET");
    let url_ptr = shim_nsurlrequest_url(obj, 0);
    let url = if url_ptr != 0 {
        cfstring::extract_cfstring_cstr(crate::nsurl::shim_nsurl_absolute_string(url_ptr, 0))
            .unwrap_or("(null)")
    } else {
        "(null)"
    };
    make_nsstring(&format!("<NSURLRequest: {method} {url}>"))
}

// ═══════════════════════════════════════════════════════════════
// NSMutableURLRequest
// ═══════════════════════════════════════════════════════════════

/// `+[NSMutableURLRequest requestWithURL:]`
pub extern "C" fn shim_nsmutableurlrequest_with_url(_cls: u64, _cmd: u64, url: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 32) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nsmutableurlrequest_class_addr();
        *((obj as usize + REQ_URL_OFF) as *mut u64) = url;
        *((obj as usize + REQ_METHOD_OFF) as *mut u64) = make_nsstring("GET");
    }
    obj
}

pub extern "C" fn shim_nsmutableurlrequest_set_http_method(obj: u64, _cmd: u64, method: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_METHOD_OFF) as *mut u64) = method; }
    0
}

pub extern "C" fn shim_nsmutableurlrequest_set_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_URL_OFF) as *mut u64) = url; }
    0
}

pub extern "C" fn shim_nsmutableurlrequest_set_value_for_header(
    obj: u64, _cmd: u64, value: u64, field: u64,
) -> u64 {
    if obj == 0 { return 0; }
    // Get or create headers dict
    let headers = unsafe { *((obj as usize + REQ_HEADERS_OFF) as *const u64) };
    if headers == 0 {
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(0, 0);
        unsafe { *((obj as usize + REQ_HEADERS_OFF) as *mut u64) = dict; }
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, value, field);
    } else {
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(headers, 0, value, field);
    }
    0
}

// ═══════════════════════════════════════════════════════════════
// NSURLResponse
// ═══════════════════════════════════════════════════════════════

const RESP_URL_OFF: usize = 8;
const RESP_MIME_OFF: usize = 16;
const RESP_LENGTH_OFF: usize = 24;

pub extern "C" fn shim_nsurlresponse_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + RESP_URL_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlresponse_mime_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + RESP_MIME_OFF) as *const u64) }
}

pub extern "C" fn shim_nsurlresponse_expected_content_length(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return -1; }
    unsafe { *((obj as usize + RESP_LENGTH_OFF) as *const i64) }
}

pub extern "C" fn shim_nsurlresponse_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_nsstring("<NSURLResponse>")
}

// ═══════════════════════════════════════════════════════════════
// NSHTTPURLResponse
// ═══════════════════════════════════════════════════════════════

const HTTP_STATUS_OFF: usize = 32;

/// Create an NSHTTPURLResponse (used internally, not from ObjC code).
pub fn make_http_response(url: u64, status: i64, mime: u64, length: i64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nshttpurlresponse_class_addr();
        *((obj as usize + RESP_URL_OFF) as *mut u64) = url;
        *((obj as usize + RESP_MIME_OFF) as *mut u64) = mime;
        *((obj as usize + RESP_LENGTH_OFF) as *mut i64) = length;
        *((obj as usize + HTTP_STATUS_OFF) as *mut i64) = status;
    }
    obj
}

pub extern "C" fn shim_nshttpurlresponse_status_code(obj: u64, _cmd: u64) -> i64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + HTTP_STATUS_OFF) as *const i64) }
}

/// `+[NSHTTPURLResponse localizedStringForStatusCode:]`
pub extern "C" fn shim_nshttpurlresponse_localized_string(
    _cls: u64, _cmd: u64, code: i64,
) -> u64 {
    let s = match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    };
    make_nsstring(s)
}

pub extern "C" fn shim_nshttpurlresponse_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let status = shim_nshttpurlresponse_status_code(obj, 0);
    make_nsstring(&format!("<NSHTTPURLResponse: {status}>"))
}
