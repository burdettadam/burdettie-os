//! NSURLRequest / NSMutableURLRequest / NSURLResponse / NSHTTPURLResponse shims.
//!
//! NSURLRequest layout (40 bytes):
//!   offset  0: isa          (u64)
//!   offset  8: url_ptr      (u64 — NSURL)
//!   offset 16: method_ptr   (u64 — NSString, "GET" default)
//!   offset 24: headers_ptr  (u64 — NSDictionary, 0=nil)
//!   offset 32: body_ptr     (u64 — NSData, 0=nil)
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
const REQ_BODY_OFF: usize = 32;

/// `+[NSURLRequest requestWithURL:]`
pub extern "C" fn shim_nsurlrequest_with_url(_cls: u64, _cmd: u64, url: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
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

/// `-[NSURLRequest HTTPBody]`
pub extern "C" fn shim_nsurlrequest_http_body(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_BODY_OFF) as *const u64) }
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
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
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

/// `-[NSMutableURLRequest setHTTPBody:]`
pub extern "C" fn shim_nsmutableurlrequest_set_http_body(obj: u64, _cmd: u64, data: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + REQ_BODY_OFF) as *mut u64) = data; }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── Helper: create an NSURL from a &str ───────────────────────
    fn make_url(s: &str) -> u64 {
        let ns = make_string(s);
        crate::nsurl::shim_nsurl_url_with_string(0, 0, ns)
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_with_url
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_with_url_basic() {
        let url = make_url("https://example.com");
        let req = shim_nsurlrequest_with_url(0, 0, url);
        assert_ne!(req, 0);
        // ISA should be nsurlrequest
        assert_eq!(read_field(req, 0), nsobj::nsurlrequest_class_addr());
        // URL field should match
        assert_eq!(shim_nsurlrequest_url(req, 0), url);
        // Default method should be GET
        assert_string_eq(shim_nsurlrequest_http_method(req, 0), "GET");
        // Headers and body default to nil
        assert_eq!(shim_nsurlrequest_all_http_header_fields(req, 0), 0);
        assert_eq!(shim_nsurlrequest_http_body(req, 0), 0);
        free_obj(req);
    }

    #[test]
    fn request_with_url_nil_url() {
        let req = shim_nsurlrequest_with_url(0, 0, 0);
        assert_ne!(req, 0); // object is still allocated
        assert_eq!(shim_nsurlrequest_url(req, 0), 0);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_url
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_url_nil_obj() {
        assert_eq!(shim_nsurlrequest_url(0, 0), 0);
    }

    #[test]
    fn request_url_returns_stored_url() {
        let url = make_url("https://rust-lang.org");
        let req = shim_nsurlrequest_with_url(0, 0, url);
        assert_eq!(shim_nsurlrequest_url(req, 0), url);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_http_method
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_http_method_nil_obj() {
        // nil receiver returns "GET" (the default)
        let m = shim_nsurlrequest_http_method(0, 0);
        assert_ne!(m, 0);
        assert_string_eq(m, "GET");
    }

    #[test]
    fn request_http_method_default_get() {
        let req = shim_nsurlrequest_with_url(0, 0, 0);
        assert_string_eq(shim_nsurlrequest_http_method(req, 0), "GET");
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_all_http_header_fields
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_headers_nil_obj() {
        assert_eq!(shim_nsurlrequest_all_http_header_fields(0, 0), 0);
    }

    #[test]
    fn request_headers_default_nil() {
        let req = shim_nsurlrequest_with_url(0, 0, 0);
        assert_eq!(shim_nsurlrequest_all_http_header_fields(req, 0), 0);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_http_body
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_body_nil_obj() {
        assert_eq!(shim_nsurlrequest_http_body(0, 0), 0);
    }

    #[test]
    fn request_body_default_nil() {
        let req = shim_nsurlrequest_with_url(0, 0, 0);
        assert_eq!(shim_nsurlrequest_http_body(req, 0), 0);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLRequest — shim_nsurlrequest_description
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn request_description_nil_obj() {
        assert_eq!(shim_nsurlrequest_description(0, 0), 0);
    }

    #[test]
    fn request_description_with_url() {
        let url = make_url("https://example.com/path");
        let req = shim_nsurlrequest_with_url(0, 0, url);
        let desc = shim_nsurlrequest_description(req, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLRequest: GET https://example.com/path>");
        free_obj(req);
    }

    #[test]
    fn request_description_nil_url() {
        let req = shim_nsurlrequest_with_url(0, 0, 0);
        let desc = shim_nsurlrequest_description(req, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLRequest: GET (null)>");
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSMutableURLRequest — shim_nsmutableurlrequest_with_url
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutable_request_with_url_basic() {
        let url = make_url("https://api.example.com");
        let req = shim_nsmutableurlrequest_with_url(0, 0, url);
        assert_ne!(req, 0);
        assert_eq!(read_field(req, 0), nsobj::nsmutableurlrequest_class_addr());
        assert_eq!(shim_nsurlrequest_url(req, 0), url);
        assert_string_eq(shim_nsurlrequest_http_method(req, 0), "GET");
        free_obj(req);
    }

    #[test]
    fn mutable_request_with_nil_url() {
        let req = shim_nsmutableurlrequest_with_url(0, 0, 0);
        assert_ne!(req, 0);
        assert_eq!(shim_nsurlrequest_url(req, 0), 0);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSMutableURLRequest — shim_nsmutableurlrequest_set_http_method
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn set_http_method_nil_obj() {
        assert_eq!(shim_nsmutableurlrequest_set_http_method(0, 0, 0), 0);
    }

    #[test]
    fn set_http_method_basic() {
        let req = shim_nsmutableurlrequest_with_url(0, 0, 0);
        let post = make_string("POST");
        shim_nsmutableurlrequest_set_http_method(req, 0, post);
        assert_string_eq(shim_nsurlrequest_http_method(req, 0), "POST");
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSMutableURLRequest — shim_nsmutableurlrequest_set_url
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn set_url_nil_obj() {
        assert_eq!(shim_nsmutableurlrequest_set_url(0, 0, 0), 0);
    }

    #[test]
    fn set_url_basic() {
        let req = shim_nsmutableurlrequest_with_url(0, 0, 0);
        let url = make_url("https://new.example.com");
        shim_nsmutableurlrequest_set_url(req, 0, url);
        assert_eq!(shim_nsurlrequest_url(req, 0), url);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSMutableURLRequest — shim_nsmutableurlrequest_set_value_for_header
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn set_value_for_header_nil_obj() {
        // Should not crash
        assert_eq!(shim_nsmutableurlrequest_set_value_for_header(0, 0, 0, 0), 0);
    }

    #[test]
    fn set_value_for_header_with_existing_dict() {
        let req = shim_nsmutableurlrequest_with_url(0, 0, 0);
        // Pre-create a headers dict so set_value_for_header appends to it
        let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
            nsobj::nsmutabledictionary_class_addr(), 0,
        );
        write_field(req, REQ_HEADERS_OFF, dict);
        let key = make_string("Content-Type");
        let val = make_string("application/json");
        shim_nsmutableurlrequest_set_value_for_header(req, 0, val, key);
        // Verify headers dict is still the same object
        assert_eq!(read_field(req, REQ_HEADERS_OFF), dict);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSMutableURLRequest — shim_nsmutableurlrequest_set_http_body
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn set_http_body_nil_obj() {
        assert_eq!(shim_nsmutableurlrequest_set_http_body(0, 0, 0), 0);
    }

    #[test]
    fn set_http_body_basic() {
        let req = shim_nsmutableurlrequest_with_url(0, 0, 0);
        let fake_data: u64 = 0xBEEF;
        shim_nsmutableurlrequest_set_http_body(req, 0, fake_data);
        assert_eq!(shim_nsurlrequest_http_body(req, 0), fake_data);
        free_obj(req);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLResponse — shim_nsurlresponse_url
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn response_url_nil_obj() {
        assert_eq!(shim_nsurlresponse_url(0, 0), 0);
    }

    #[test]
    fn response_url_basic() {
        let url = make_url("https://example.com");
        let resp = alloc_obj(32, nsobj::nsurlresponse_class_addr());
        write_field(resp, RESP_URL_OFF, url);
        assert_eq!(shim_nsurlresponse_url(resp, 0), url);
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLResponse — shim_nsurlresponse_mime_type
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn response_mime_nil_obj() {
        assert_eq!(shim_nsurlresponse_mime_type(0, 0), 0);
    }

    #[test]
    fn response_mime_basic() {
        let mime = make_string("text/html");
        let resp = alloc_obj(32, nsobj::nsurlresponse_class_addr());
        write_field(resp, RESP_MIME_OFF, mime);
        assert_eq!(shim_nsurlresponse_mime_type(resp, 0), mime);
        assert_string_eq(shim_nsurlresponse_mime_type(resp, 0), "text/html");
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLResponse — shim_nsurlresponse_expected_content_length
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn response_content_length_nil_obj() {
        assert_eq!(shim_nsurlresponse_expected_content_length(0, 0), -1);
    }

    #[test]
    fn response_content_length_basic() {
        let resp = alloc_obj(32, nsobj::nsurlresponse_class_addr());
        // Write i64 length via write_field (u64 transmute is fine for positive values)
        write_field(resp, RESP_LENGTH_OFF, 4096);
        assert_eq!(shim_nsurlresponse_expected_content_length(resp, 0), 4096);
        free_obj(resp);
    }

    #[test]
    fn response_content_length_unknown() {
        let resp = alloc_obj(32, nsobj::nsurlresponse_class_addr());
        // Write -1 as i64 (all 1s in u64 representation)
        write_field(resp, RESP_LENGTH_OFF, (-1_i64) as u64);
        assert_eq!(shim_nsurlresponse_expected_content_length(resp, 0), -1);
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSURLResponse — shim_nsurlresponse_description
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn response_description_nil_obj() {
        assert_eq!(shim_nsurlresponse_description(0, 0), 0);
    }

    #[test]
    fn response_description_basic() {
        let resp = alloc_obj(32, nsobj::nsurlresponse_class_addr());
        let desc = shim_nsurlresponse_description(resp, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLResponse>");
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSHTTPURLResponse — make_http_response
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn make_http_response_basic() {
        let url = make_url("https://example.com");
        let mime = make_string("application/json");
        let resp = make_http_response(url, 200, mime, 1024);
        assert_ne!(resp, 0);
        assert_eq!(read_field(resp, 0), nsobj::nshttpurlresponse_class_addr());
        assert_eq!(shim_nsurlresponse_url(resp, 0), url);
        assert_string_eq(shim_nsurlresponse_mime_type(resp, 0), "application/json");
        assert_eq!(shim_nsurlresponse_expected_content_length(resp, 0), 1024);
        assert_eq!(shim_nshttpurlresponse_status_code(resp, 0), 200);
        free_obj(resp);
    }

    #[test]
    fn make_http_response_negative_length() {
        let resp = make_http_response(0, 404, 0, -1);
        assert_ne!(resp, 0);
        assert_eq!(shim_nsurlresponse_expected_content_length(resp, 0), -1);
        assert_eq!(shim_nshttpurlresponse_status_code(resp, 0), 404);
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSHTTPURLResponse — shim_nshttpurlresponse_status_code
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn http_response_status_code_nil_obj() {
        assert_eq!(shim_nshttpurlresponse_status_code(0, 0), 0);
    }

    #[test]
    fn http_response_status_code_basic() {
        let resp = make_http_response(0, 503, 0, -1);
        assert_eq!(shim_nshttpurlresponse_status_code(resp, 0), 503);
        free_obj(resp);
    }

    // ═══════════════════════════════════════════════════════════════
    // NSHTTPURLResponse — shim_nshttpurlresponse_localized_string
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn localized_string_known_codes() {
        let cases: &[(i64, &str)] = &[
            (200, "OK"),
            (201, "Created"),
            (204, "No Content"),
            (301, "Moved Permanently"),
            (302, "Found"),
            (304, "Not Modified"),
            (400, "Bad Request"),
            (401, "Unauthorized"),
            (403, "Forbidden"),
            (404, "Not Found"),
            (405, "Method Not Allowed"),
            (500, "Internal Server Error"),
            (502, "Bad Gateway"),
            (503, "Service Unavailable"),
        ];
        for &(code, expected) in cases {
            let s = shim_nshttpurlresponse_localized_string(0, 0, code);
            assert_ne!(s, 0, "localized string for {code} was nil");
            assert_string_eq(s, expected);
        }
    }

    #[test]
    fn localized_string_unknown_code() {
        let s = shim_nshttpurlresponse_localized_string(0, 0, 999);
        assert_ne!(s, 0);
        assert_string_eq(s, "Unknown");
    }

    // ═══════════════════════════════════════════════════════════════
    // NSHTTPURLResponse — shim_nshttpurlresponse_description
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn http_response_description_nil_obj() {
        assert_eq!(shim_nshttpurlresponse_description(0, 0), 0);
    }

    #[test]
    fn http_response_description_basic() {
        let resp = make_http_response(0, 200, 0, -1);
        let desc = shim_nshttpurlresponse_description(resp, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHTTPURLResponse: 200>");
        free_obj(resp);
    }

    #[test]
    fn http_response_description_error_status() {
        let resp = make_http_response(0, 500, 0, -1);
        let desc = shim_nshttpurlresponse_description(resp, 0);
        assert_string_eq(desc, "<NSHTTPURLResponse: 500>");
        free_obj(resp);
    }
}
