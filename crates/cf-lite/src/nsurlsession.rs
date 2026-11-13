//! NSURLSession shim: HTTP client (stub for basic data tasks).
//!
//! NSURLSession layout (16 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: config_ptr  (u64 — NSURLSessionConfiguration, 0=default)
//!
//! NSURLSessionConfiguration layout (16 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: timeout     (u64 — f64 bits, default 60.0)
//!
//! NSURLSessionDataTask layout (32 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: request_ptr (u64 — NSURLRequest)
//!   offset 16: state       (u64 — 0=suspended, 1=running, 2=canceling, 3=completed)
//!   offset 24: block_ptr   (u64 — completionHandler block, 0=none)

use crate::cfstring;
use crate::nsobj;
use crate::nsurl;
use crate::nsurlrequest;
use std::sync::OnceLock;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn http_agent() -> &'static ureq::Agent {
    static HTTP_AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    HTTP_AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent()
    })
}

// ═══════════════════════════════════════════════════════════════
// NSURLSessionConfiguration
// ═══════════════════════════════════════════════════════════════

fn alloc_config() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlsessionconfiguration_class_addr();
        // Default timeout: 60.0 seconds
        *((obj as usize + 8) as *mut u64) = 60.0_f64.to_bits();
    }
    obj
}

/// `+[NSURLSessionConfiguration defaultSessionConfiguration]`
pub extern "C" fn shim_nsurlsessionconfig_default(_cls: u64, _cmd: u64) -> u64 {
    alloc_config()
}

/// `+[NSURLSessionConfiguration ephemeralSessionConfiguration]`
pub extern "C" fn shim_nsurlsessionconfig_ephemeral(_cls: u64, _cmd: u64) -> u64 {
    alloc_config()
}

/// `-[NSURLSessionConfiguration timeoutIntervalForRequest]`
pub extern "C" fn shim_nsurlsessionconfig_timeout(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 60.0;
    }
    f64::from_bits(unsafe { *((obj as usize + 8) as *const u64) })
}

/// `-[NSURLSessionConfiguration setTimeoutIntervalForRequest:]`
pub extern "C" fn shim_nsurlsessionconfig_set_timeout(obj: u64, _cmd: u64, val: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + 8) as *mut u64) = val.to_bits();
    }
}

/// `+[NSURLSessionConfiguration backgroundSessionConfigurationWithIdentifier:]`
pub extern "C" fn shim_nsurlsessionconfig_background(
    _cls: u64,
    _cmd: u64,
    _identifier: u64,
) -> u64 {
    alloc_config()
}

/// `-[NSURLSessionConfiguration timeoutIntervalForResource]`
pub extern "C" fn shim_nsurlsessionconfig_timeout_resource(_obj: u64, _cmd: u64) -> f64 {
    604800.0 // 7 days
}

/// `-[NSURLSessionConfiguration setTimeoutIntervalForResource:]`
pub extern "C" fn shim_nsurlsessionconfig_set_timeout_resource(
    _obj: u64,
    _cmd: u64,
    _interval: f64,
) {
    // stub — not tracked
}

pub extern "C" fn shim_nsurlsessionconfig_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSURLSessionConfiguration>")
}

// ═══════════════════════════════════════════════════════════════
// NSURLSession
// ═══════════════════════════════════════════════════════════════

const SESSION_CONFIG_OFF: usize = 8;
const SESSION_DELEGATE_OFF: usize = 16;
const SESSION_QUEUE_OFF: usize = 24;
const SESSION_SIZE: usize = 32;

/// `+[NSURLSession sharedSession]`
pub extern "C" fn shim_nsurlsession_shared(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SESSION_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlsession_class_addr();
    }
    obj
}

/// `+[NSURLSession sessionWithConfiguration:]`
pub extern "C" fn shim_nsurlsession_with_config(_cls: u64, _cmd: u64, config: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SESSION_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlsession_class_addr();
        *((obj as usize + SESSION_CONFIG_OFF) as *mut u64) = config;
    }
    obj
}

/// `+[NSURLSession sessionWithConfiguration:delegate:delegateQueue:]`
pub extern "C" fn shim_nsurlsession_with_config_delegate(
    _cls: u64,
    _cmd: u64,
    config: u64,
    delegate: u64,
    queue: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SESSION_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlsession_class_addr();
        *((obj as usize + SESSION_CONFIG_OFF) as *mut u64) = config;
        *((obj as usize + SESSION_DELEGATE_OFF) as *mut u64) = delegate;
        *((obj as usize + SESSION_QUEUE_OFF) as *mut u64) = queue;
    }
    obj
}

/// `-[NSURLSession delegate]`
pub extern "C" fn shim_nsurlsession_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SESSION_DELEGATE_OFF) as *const u64) }
}

/// `-[NSURLSession delegateQueue]`
pub extern "C" fn shim_nsurlsession_delegate_queue(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SESSION_QUEUE_OFF) as *const u64) }
}

/// `-[NSURLSession configuration]`
pub extern "C" fn shim_nsurlsession_configuration(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SESSION_CONFIG_OFF) as *const u64) }
}

/// `-[NSURLSession dataTaskWithURL:]`
pub extern "C" fn shim_nsurlsession_data_task_with_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Create an NSURLRequest from the URL
    let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, url);
    alloc_data_task_for_session(req, obj)
}

/// `-[NSURLSession dataTaskWithRequest:]`
pub extern "C" fn shim_nsurlsession_data_task_with_request(
    obj: u64,
    _cmd: u64,
    request: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    alloc_data_task_for_session(request, obj)
}

/// `-[NSURLSession invalidateAndCancel]`
pub extern "C" fn shim_nsurlsession_invalidate(_obj: u64, _cmd: u64) {
    log::debug!("NSURLSession invalidateAndCancel (stub)");
}

/// `-[NSURLSession finishTasksAndInvalidate]`
pub extern "C" fn shim_nsurlsession_finish_tasks(_obj: u64, _cmd: u64) {
    log::debug!("NSURLSession finishTasksAndInvalidate (stub)");
}

/// `-[NSURLSession uploadTaskWithRequest:fromData:]`
///
/// Creates an upload task: sets the HTTP body on the request and marks the
/// task type as upload.  On `resume`, the existing HTTP engine reads the body
/// from the request and sends it as a POST/PUT payload.
pub extern "C" fn shim_nsurlsession_upload_task_with_request(
    obj: u64,
    _cmd: u64,
    request: u64,
    data: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Attach the body data to the request so resume picks it up.
    if request != 0 && data != 0 {
        nsurlrequest::shim_nsmutableurlrequest_set_http_body(request, 0, data);
    }
    let task = alloc_data_task_for_session(request, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessionuploadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 1; // upload
        }
    }
    task
}

/// `-[NSURLSession uploadTaskWithRequest:fromData:completionHandler:]`
pub extern "C" fn shim_nsurlsession_upload_task_with_request_completion(
    obj: u64,
    _cmd: u64,
    request: u64,
    data: u64,
    block: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    if request != 0 && data != 0 {
        nsurlrequest::shim_nsmutableurlrequest_set_http_body(request, 0, data);
    }
    let task = alloc_data_task_for_session(request, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessionuploadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 1;
        }
        if block != 0 {
            unsafe {
                *((task as usize + TASK_BLOCK_OFF) as *mut u64) = block;
            }
        }
    }
    task
}

/// `-[NSURLSession downloadTaskWithRequest:]`
///
/// Creates a download task.  On `resume`, the response body is written to a
/// temporary file and the completion handler receives a file URL.
pub extern "C" fn shim_nsurlsession_download_task_with_request(
    obj: u64,
    _cmd: u64,
    request: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let task = alloc_data_task_for_session(request, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessiondownloadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 2; // download
        }
    }
    task
}

/// `-[NSURLSession downloadTaskWithURL:]`
pub extern "C" fn shim_nsurlsession_download_task_with_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, url);
    let task = alloc_data_task_for_session(req, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessiondownloadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 2;
        }
    }
    task
}

/// `-[NSURLSession downloadTaskWithRequest:completionHandler:]`
pub extern "C" fn shim_nsurlsession_download_task_with_request_completion(
    obj: u64,
    _cmd: u64,
    request: u64,
    block: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let task = alloc_data_task_for_session(request, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessiondownloadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 2;
        }
        if block != 0 {
            unsafe {
                *((task as usize + TASK_BLOCK_OFF) as *mut u64) = block;
            }
        }
    }
    task
}

/// `-[NSURLSession downloadTaskWithURL:completionHandler:]`
pub extern "C" fn shim_nsurlsession_download_task_with_url_completion(
    obj: u64,
    _cmd: u64,
    url: u64,
    block: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, url);
    let task = alloc_data_task_for_session(req, obj);
    if task != 0 {
        unsafe {
            *(task as *mut u64) = nsobj::nsurlsessiondownloadtask_class_addr();
            *((task as usize + TASK_TYPE_OFF) as *mut u64) = 2;
        }
        if block != 0 {
            unsafe {
                *((task as usize + TASK_BLOCK_OFF) as *mut u64) = block;
            }
        }
    }
    task
}

/// `-[NSURLSession resetWithCompletionHandler:]`
pub extern "C" fn shim_nsurlsession_reset_with_completion_handler(
    _obj: u64,
    _cmd: u64,
    block: u64,
) {
    if block != 0 {
        let invoke_ptr = unsafe { *((block as usize + 16) as *const u64) };
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64) = unsafe { std::mem::transmute(invoke_ptr) };
            invoke(block);
        }
    }
}

/// `-[NSURLSession getAllTasksWithCompletionHandler:]`
pub extern "C" fn shim_nsurlsession_all_tasks_with_completion_handler(
    _obj: u64,
    _cmd: u64,
    block: u64,
) {
    if block != 0 {
        let empty = crate::nsarray::make_nsarray(&[]);
        let invoke_ptr = unsafe { *((block as usize + 16) as *const u64) };
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(invoke_ptr) };
            invoke(block, empty);
        }
    }
}

pub extern "C" fn shim_nsurlsession_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSURLSession>")
}

// ═══════════════════════════════════════════════════════════════
// NSURLSessionDataTask
// ═══════════════════════════════════════════════════════════════

const TASK_REQUEST_OFF: usize = 8;
const TASK_STATE_OFF: usize = 16;
const TASK_BLOCK_OFF: usize = 24;
const TASK_SESSION_OFF: usize = 32;
const TASK_TYPE_OFF: usize = 40; // 0=data, 1=upload, 2=download
const TASK_SIZE: usize = 48;

fn alloc_data_task(request: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, TASK_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsurlsessiondatatask_class_addr();
        *((obj as usize + TASK_REQUEST_OFF) as *mut u64) = request;
        // state = 0 (suspended); block_ptr = 0 (none); session = 0
    }
    obj
}

fn alloc_data_task_for_session(request: u64, session: u64) -> u64 {
    let obj = alloc_data_task(request);
    if obj != 0 && session != 0 {
        unsafe {
            *((obj as usize + TASK_SESSION_OFF) as *mut u64) = session;
        }
    }
    obj
}

/// `-[NSURLSession dataTaskWithURL:completionHandler:]`
///
/// Stores the block; no real HTTP is performed. On `resume`, the block is
/// called with `(nil, nil, nil)` so callers that gate on the error argument
/// will proceed gracefully.
pub extern "C" fn shim_nsurlsession_data_task_with_url_completion(
    obj: u64,
    _cmd: u64,
    url: u64,
    block: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, url);
    let task = alloc_data_task_for_session(req, obj);
    if task != 0 && block != 0 {
        unsafe {
            *((task as usize + TASK_BLOCK_OFF) as *mut u64) = block;
        }
    }
    task
}

/// `-[NSURLSession dataTaskWithRequest:completionHandler:]`
pub extern "C" fn shim_nsurlsession_data_task_with_request_completion(
    obj: u64,
    _cmd: u64,
    request: u64,
    block: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let task = alloc_data_task_for_session(request, obj);
    if task != 0 && block != 0 {
        unsafe {
            *((task as usize + TASK_BLOCK_OFF) as *mut u64) = block;
        }
    }
    task
}

/// `-[NSURLSessionDataTask resume]`
///
/// Performs a synchronous HTTP/HTTPS request using ureq and dispatches the
/// completionHandler block with:
///   - data/upload tasks: `(NSData, NSHTTPURLResponse, NSError)`
///   - download tasks:    `(NSURL, NSHTTPURLResponse, NSError)`
///
/// Falls back to a 200-stub response if no URL is available.
pub extern "C" fn shim_nsurlsessiondatatask_resume(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + TASK_STATE_OFF) as *mut u64) = 1;
    } // running

    let block = unsafe { *((obj as usize + TASK_BLOCK_OFF) as *const u64) };
    let request = unsafe { *((obj as usize + TASK_REQUEST_OFF) as *const u64) };
    let task_type = unsafe { *((obj as usize + TASK_TYPE_OFF) as *const u64) };

    if block == 0 {
        unsafe {
            *((obj as usize + TASK_STATE_OFF) as *mut u64) = 3;
        }
        return;
    }

    // Resolve the URL string from the NSURLRequest.
    let url_ns = if request != 0 {
        nsurlrequest::shim_nsurlrequest_url(request, 0)
    } else {
        0
    };
    let url_str = if url_ns != 0 {
        crate::cfstring::extract_cfstring_cstr(nsurl::shim_nsurl_absolute_string(url_ns, 0))
            .map(|s| s.to_string())
    } else {
        None
    };

    // Resolve the HTTP method.
    let method_ns = if request != 0 {
        nsurlrequest::shim_nsurlrequest_http_method(request, 0)
    } else {
        0
    };
    let method = crate::cfstring::extract_cfstring_cstr(method_ns)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "GET".to_string());

    // Resolve the HTTP body (for POST/PUT).
    let body_bytes: Option<Vec<u8>> = if request != 0 {
        let body_ns = nsurlrequest::shim_nsurlrequest_http_body(request, 0);
        if body_ns != 0 {
            let len = unsafe { *((body_ns as usize + 8) as *const u64) } as usize;
            let ptr = unsafe { *((body_ns as usize + 16) as *const u64) };
            if len > 0 && ptr != 0 {
                let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
                Some(slice.to_vec())
            } else {
                Some(vec![])
            }
        } else {
            None
        }
    } else {
        None
    };

    if task_type == 2 {
        // ── Download task: write body to temp file, fire completion with file URL ──
        resume_download_task(obj, block, url_ns, &url_str, &method, body_bytes.as_deref());
    } else {
        // ── Data/upload task: fire completion with NSData ──
        resume_data_task(obj, block, url_ns, &url_str, &method, body_bytes.as_deref());
    }
}

/// Resume for data and upload tasks — fires completion with `(NSData, NSHTTPURLResponse, NSError)`.
fn resume_data_task(
    obj: u64,
    block: u64,
    url_ns: u64,
    url_str: &Option<String>,
    method: &str,
    body: Option<&[u8]>,
) {
    let (data_obj, response_obj, error_obj) = if let Some(ref url) = url_str {
        match perform_http_request(url, method, body) {
            Ok((status, body)) => {
                let ns_data = crate::nsdata::make_nsdata(&body);
                let ns_resp =
                    nsurlrequest::make_http_response(url_ns, status as i64, 0, body.len() as i64);
                (ns_data, ns_resp, 0u64)
            }
            Err(e) => {
                log::warn!("NSURLSessionDataTask HTTP error for {url}: {e}");
                let ns_err_obj = crate::nserror::make_nserror("NSURLErrorDomain", -1004);
                (0u64, 0u64, ns_err_obj)
            }
        }
    } else {
        log::debug!("NSURLSessionDataTask resume — no URL, returning stub 200");
        let ns_resp = nsurlrequest::make_http_response(url_ns, 200, 0, -1);
        (0u64, ns_resp, 0u64)
    };

    unsafe {
        *((obj as usize + TASK_STATE_OFF) as *mut u64) = 3;
    }

    let invoke_ptr = unsafe { *((block as usize + 16) as *const u64) };
    if invoke_ptr != 0 {
        let invoke: extern "C" fn(u64, u64, u64, u64) = unsafe { std::mem::transmute(invoke_ptr) };
        invoke(block, data_obj, response_obj, error_obj);
    }
}

/// Resume for download tasks — writes body to a temp file, fires completion
/// with `(NSURL, NSHTTPURLResponse, NSError)`.
fn resume_download_task(
    obj: u64,
    block: u64,
    url_ns: u64,
    url_str: &Option<String>,
    method: &str,
    body: Option<&[u8]>,
) {
    let (file_url, response_obj, error_obj) = if let Some(ref url) = url_str {
        match perform_http_request(url, method, body) {
            Ok((status, body_bytes)) => {
                let ns_resp = nsurlrequest::make_http_response(
                    url_ns,
                    status as i64,
                    0,
                    body_bytes.len() as i64,
                );
                // Write body to a temporary file.
                let file_url = write_download_to_temp(&body_bytes);
                (file_url, ns_resp, 0u64)
            }
            Err(e) => {
                log::warn!("NSURLSessionDownloadTask HTTP error for {url}: {e}");
                let ns_err_obj = crate::nserror::make_nserror("NSURLErrorDomain", -1004);
                (0u64, 0u64, ns_err_obj)
            }
        }
    } else {
        log::debug!("NSURLSessionDownloadTask resume — no URL, returning stub 200");
        let ns_resp = nsurlrequest::make_http_response(url_ns, 200, 0, 0);
        let file_url = write_download_to_temp(b"");
        (file_url, ns_resp, 0u64)
    };

    unsafe {
        *((obj as usize + TASK_STATE_OFF) as *mut u64) = 3;
    }

    let invoke_ptr = unsafe { *((block as usize + 16) as *const u64) };
    if invoke_ptr != 0 {
        let invoke: extern "C" fn(u64, u64, u64, u64) = unsafe { std::mem::transmute(invoke_ptr) };
        invoke(block, file_url, response_obj, error_obj);
    }
}

/// Write bytes to a uniquely-named temp file and return an NSURL file URL.
fn write_download_to_temp(data: &[u8]) -> u64 {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let name = format!(
        "burdettie-download-{}",
        std::process::id() as u64 * 1_000_000
            + DOWNLOAD_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let path = dir.join(name);
    match std::fs::File::create(&path) {
        Ok(mut f) => {
            let _ = f.write_all(data);
        }
        Err(e) => {
            log::warn!("Failed to write download temp file: {e}");
            return 0;
        }
    }
    let path_str = path.to_string_lossy();
    let ns_path = make_nsstring(&path_str);
    nsurl::shim_nsurl_file_url_with_path(0, 0, ns_path)
}

static DOWNLOAD_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Perform a synchronous HTTP/HTTPS request and return (status_code, body_bytes).
fn perform_http_request(
    url: &str,
    method: &str,
    body: Option<&[u8]>,
) -> Result<(i32, Vec<u8>), String> {
    // BurdettieOS domain rewrite — redirect Apple service URLs to local server.
    // The original URL is passed as X-Burdettie-Original-URL header for recording.
    let (effective_url, original_url) = match burdettie_domain_map::rewrite_url(url) {
        Some(rewritten) => (rewritten, Some(url.to_string())),
        None => (url.to_string(), None),
    };

    let agent = http_agent();
    // ureq v3 typestate: POST/PUT use .send(); others use .call().
    // Apply map_err inside each arm so the match produces Result<_, String> uniformly.
    let mut resp = match () {
        _ if method.eq_ignore_ascii_case("HEAD") => {
            let mut r = agent.head(&effective_url);
            if let Some(ref orig) = original_url {
                r = r.header("X-Burdettie-Original-URL", orig);
            }
            r.call().map_err(|e| e.to_string())
        }
        _ if method.eq_ignore_ascii_case("DELETE") => {
            let mut r = agent.delete(&effective_url);
            if let Some(ref orig) = original_url {
                r = r.header("X-Burdettie-Original-URL", orig);
            }
            r.call().map_err(|e| e.to_string())
        }
        _ if method.eq_ignore_ascii_case("POST") => {
            let mut r = agent.post(&effective_url);
            if let Some(ref orig) = original_url {
                r = r.header("X-Burdettie-Original-URL", orig);
            }
            r.send(body.unwrap_or(b"")).map_err(|e| e.to_string())
        }
        _ if method.eq_ignore_ascii_case("PUT") => {
            let mut r = agent.put(&effective_url);
            if let Some(ref orig) = original_url {
                r = r.header("X-Burdettie-Original-URL", orig);
            }
            r.send(body.unwrap_or(b"")).map_err(|e| e.to_string())
        }
        _ => {
            let mut r = agent.get(&effective_url);
            if let Some(ref orig) = original_url {
                r = r.header("X-Burdettie-Original-URL", orig);
            }
            r.call().map_err(|e| e.to_string())
        }
    }?;
    let status = resp.status().as_u16() as i32;
    let body_bytes = resp.body_mut().read_to_vec().map_err(|e| e.to_string())?;
    Ok((status, body_bytes))
}

/// `-[NSURLSessionDataTask cancel]`
pub extern "C" fn shim_nsurlsessiondatatask_cancel(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + TASK_STATE_OFF) as *mut u64) = 2;
    }
}

/// `-[NSURLSessionDataTask suspend]`
pub extern "C" fn shim_nsurlsessiondatatask_suspend(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + TASK_STATE_OFF) as *mut u64) = 0;
    }
}

/// `-[NSURLSessionDataTask state]`
pub extern "C" fn shim_nsurlsessiondatatask_state(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 3;
    }
    unsafe { *((obj as usize + TASK_STATE_OFF) as *const u64) }
}

/// `-[NSURLSessionDataTask originalRequest]`
pub extern "C" fn shim_nsurlsessiondatatask_original_request(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + TASK_REQUEST_OFF) as *const u64) }
}

/// `-[NSURLSessionDataTask taskIdentifier]`
pub extern "C" fn shim_nsurlsessiondatatask_task_id(obj: u64, _cmd: u64) -> u64 {
    // Return the object address as a unique identifier
    obj
}

pub extern "C" fn shim_nsurlsessiondatatask_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let state = unsafe { *((obj as usize + TASK_STATE_OFF) as *const u64) };
    let task_type = unsafe { *((obj as usize + TASK_TYPE_OFF) as *const u64) };
    let state_str = match state {
        0 => "suspended",
        1 => "running",
        2 => "canceling",
        _ => "completed",
    };
    let type_name = match task_type {
        1 => "NSURLSessionUploadTask",
        2 => "NSURLSessionDownloadTask",
        _ => "NSURLSessionDataTask",
    };
    make_nsstring(&format!("<{type_name}: {state_str}>"))
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // ── NSURLSessionConfiguration ────────────────────────────

    #[test]
    fn config_default_creates_valid_object() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        assert_ne!(cfg, 0);
        let isa = read_field(cfg, 0);
        assert_eq!(isa, nsobj::nsurlsessionconfiguration_class_addr());
        // Default timeout should be 60.0
        let t = shim_nsurlsessionconfig_timeout(cfg, 0);
        assert!((t - 60.0).abs() < f64::EPSILON);
        free_obj(cfg);
    }

    #[test]
    fn config_ephemeral_creates_valid_object() {
        let cfg =
            shim_nsurlsessionconfig_ephemeral(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        assert_ne!(cfg, 0);
        let t = shim_nsurlsessionconfig_timeout(cfg, 0);
        assert!((t - 60.0).abs() < f64::EPSILON);
        free_obj(cfg);
    }

    #[test]
    fn config_timeout_nil() {
        let t = shim_nsurlsessionconfig_timeout(0, 0);
        assert!((t - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn config_set_timeout() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        shim_nsurlsessionconfig_set_timeout(cfg, 0, 120.0);
        let t = shim_nsurlsessionconfig_timeout(cfg, 0);
        assert!((t - 120.0).abs() < f64::EPSILON);
        free_obj(cfg);
    }

    #[test]
    fn config_set_timeout_nil() {
        // Should not crash
        shim_nsurlsessionconfig_set_timeout(0, 0, 99.0);
    }

    #[test]
    fn config_description_valid() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        let desc = shim_nsurlsessionconfig_description(cfg, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLSessionConfiguration>");
        free_obj(cfg);
    }

    #[test]
    fn config_description_nil() {
        assert_eq!(shim_nsurlsessionconfig_description(0, 0), 0);
    }

    // ── NSURLSession ─────────────────────────────────────────

    #[test]
    fn session_shared_creates_valid_object() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        assert_ne!(s, 0);
        let isa = read_field(s, 0);
        assert_eq!(isa, nsobj::nsurlsession_class_addr());
        // config_ptr should be 0 for shared session
        assert_eq!(read_field(s, SESSION_CONFIG_OFF), 0);
        free_obj(s);
    }

    #[test]
    fn session_with_config() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        let s = shim_nsurlsession_with_config(nsobj::nsurlsession_class_addr(), 0, cfg);
        assert_ne!(s, 0);
        assert_eq!(read_field(s, SESSION_CONFIG_OFF), cfg);
        free_obj(s);
        free_obj(cfg);
    }

    #[test]
    fn session_configuration_getter() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        let s = shim_nsurlsession_with_config(nsobj::nsurlsession_class_addr(), 0, cfg);
        let got = shim_nsurlsession_configuration(s, 0);
        assert_eq!(got, cfg);
        free_obj(s);
        free_obj(cfg);
    }

    #[test]
    fn session_configuration_nil() {
        assert_eq!(shim_nsurlsession_configuration(0, 0), 0);
    }

    #[test]
    fn session_data_task_with_url_nil_session() {
        assert_eq!(shim_nsurlsession_data_task_with_url(0, 0, 0), 0);
    }

    #[test]
    fn session_data_task_with_url_creates_task() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let url = make_string("http://example.com");
        let task = shim_nsurlsession_data_task_with_url(s, 0, url);
        assert_ne!(task, 0);
        let isa = read_field(task, 0);
        assert_eq!(isa, nsobj::nsurlsessiondatatask_class_addr());
        // Initial state is suspended (0)
        assert_eq!(read_field(task, TASK_STATE_OFF), 0);
        // request should be non-zero
        assert_ne!(read_field(task, TASK_REQUEST_OFF), 0);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn session_data_task_with_request_nil_session() {
        assert_eq!(shim_nsurlsession_data_task_with_request(0, 0, 0), 0);
    }

    #[test]
    fn session_data_task_with_request_creates_task() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let task = shim_nsurlsession_data_task_with_request(s, 0, req);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_REQUEST_OFF), req);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn session_invalidate_nil() {
        // Must not crash
        shim_nsurlsession_invalidate(0, 0);
    }

    #[test]
    fn session_finish_tasks_nil() {
        // Must not crash
        shim_nsurlsession_finish_tasks(0, 0);
    }

    #[test]
    fn session_description_valid() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let desc = shim_nsurlsession_description(s, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLSession>");
        free_obj(s);
    }

    #[test]
    fn session_description_nil() {
        assert_eq!(shim_nsurlsession_description(0, 0), 0);
    }

    // ── NSURLSessionDataTask ─────────────────────────────────

    #[test]
    fn task_cancel_sets_state() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        assert_ne!(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 0); // suspended
        shim_nsurlsessiondatatask_cancel(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 2); // canceling
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_cancel_nil() {
        // Must not crash
        shim_nsurlsessiondatatask_cancel(0, 0);
    }

    #[test]
    fn task_suspend_sets_state() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        // Manually set to running first
        write_field(task, TASK_STATE_OFF, 1);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 1);
        shim_nsurlsessiondatatask_suspend(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 0); // suspended
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_suspend_nil() {
        // Must not crash
        shim_nsurlsessiondatatask_suspend(0, 0);
    }

    #[test]
    fn task_state_nil() {
        // nil returns 3 (completed)
        assert_eq!(shim_nsurlsessiondatatask_state(0, 0), 3);
    }

    #[test]
    fn task_original_request() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let task = shim_nsurlsession_data_task_with_request(s, 0, req);
        let got = shim_nsurlsessiondatatask_original_request(task, 0);
        assert_eq!(got, req);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_original_request_nil() {
        assert_eq!(shim_nsurlsessiondatatask_original_request(0, 0), 0);
    }

    #[test]
    fn task_id_returns_obj_address() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        assert_eq!(shim_nsurlsessiondatatask_task_id(task, 0), task);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_id_nil() {
        // obj=0 → returns 0 (obj itself)
        assert_eq!(shim_nsurlsessiondatatask_task_id(0, 0), 0);
    }

    #[test]
    fn task_description_suspended() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSURLSessionDataTask: suspended>");
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_description_running() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        write_field(task, TASK_STATE_OFF, 1);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_string_eq(desc, "<NSURLSessionDataTask: running>");
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_description_canceling() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        write_field(task, TASK_STATE_OFF, 2);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_string_eq(desc, "<NSURLSessionDataTask: canceling>");
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_description_completed() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        write_field(task, TASK_STATE_OFF, 3);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_string_eq(desc, "<NSURLSessionDataTask: completed>");
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn task_description_nil() {
        assert_eq!(shim_nsurlsessiondatatask_description(0, 0), 0);
    }

    // ── resume — network-dependent, only test nil safety ─────

    #[test]
    fn task_resume_nil() {
        // Must not crash — nil check returns early before any HTTP work
        shim_nsurlsessiondatatask_resume(0, 0);
    }

    #[test]
    fn task_resume_without_completion_handler_marks_task_completed() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);

        shim_nsurlsessiondatatask_resume(task, 0);

        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 3);
        free_obj(task);
        free_obj(s);
    }

    // shim_nsurlsessiondatatask_resume: full path performs real HTTP via ureq —
    // not tested in unit tests to avoid network dependency.

    // ── completion-handler variants ──────────────────────────

    #[test]
    fn data_task_with_url_completion_nil_session() {
        assert_eq!(
            shim_nsurlsession_data_task_with_url_completion(0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn data_task_with_url_completion_stores_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let fake_block: u64 = 0xCAFE;
        let task = shim_nsurlsession_data_task_with_url_completion(s, 0, 0, fake_block);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), fake_block);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn data_task_with_url_completion_zero_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url_completion(s, 0, 0, 0);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), 0);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn data_task_with_request_completion_nil_session() {
        assert_eq!(
            shim_nsurlsession_data_task_with_request_completion(0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn data_task_with_request_completion_stores_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let fake_block: u64 = 0xBEEF;
        let task = shim_nsurlsession_data_task_with_request_completion(s, 0, req, fake_block);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), fake_block);
        assert_eq!(read_field(task, TASK_REQUEST_OFF), req);
        free_obj(task);
        free_obj(s);
    }

    // ── upload/download task creation ──────────────────────────

    #[test]
    fn upload_task_nil_session() {
        assert_eq!(shim_nsurlsession_upload_task_with_request(0, 0, 0, 0), 0);
    }

    #[test]
    fn upload_task_creates_valid_task() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let body = crate::nsdata::make_nsdata(b"hello upload");
        let task = shim_nsurlsession_upload_task_with_request(s, 0, req, body);
        assert_ne!(task, 0);
        // isa is NSURLSessionUploadTask
        assert_eq!(
            read_field(task, 0),
            nsobj::nsurlsessionuploadtask_class_addr()
        );
        // task type = 1 (upload)
        assert_eq!(read_field(task, TASK_TYPE_OFF), 1);
        // state = suspended
        assert_eq!(read_field(task, TASK_STATE_OFF), 0);
        // request stored
        assert_eq!(read_field(task, TASK_REQUEST_OFF), req);
        // session stored
        assert_eq!(read_field(task, TASK_SESSION_OFF), s);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn upload_task_sets_body_on_request() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let body = crate::nsdata::make_nsdata(b"payload");
        let task = shim_nsurlsession_upload_task_with_request(s, 0, req, body);
        assert_ne!(task, 0);
        // Body should now be set on the request
        let got_body = crate::nsurlrequest::shim_nsurlrequest_http_body(req, 0);
        assert_eq!(got_body, body);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn upload_task_completion_nil_session() {
        assert_eq!(
            shim_nsurlsession_upload_task_with_request_completion(0, 0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn upload_task_completion_stores_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let body = crate::nsdata::make_nsdata(b"data");
        let fake_block: u64 = 0xBEEF;
        let task =
            shim_nsurlsession_upload_task_with_request_completion(s, 0, req, body, fake_block);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), fake_block);
        assert_eq!(read_field(task, TASK_TYPE_OFF), 1);
        assert_eq!(
            read_field(task, 0),
            nsobj::nsurlsessionuploadtask_class_addr()
        );
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_with_request_nil_session() {
        assert_eq!(shim_nsurlsession_download_task_with_request(0, 0, 0), 0);
    }

    #[test]
    fn download_task_with_request_creates_valid_task() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let task = shim_nsurlsession_download_task_with_request(s, 0, req);
        assert_ne!(task, 0);
        assert_eq!(
            read_field(task, 0),
            nsobj::nsurlsessiondownloadtask_class_addr()
        );
        assert_eq!(read_field(task, TASK_TYPE_OFF), 2);
        assert_eq!(read_field(task, TASK_STATE_OFF), 0);
        assert_eq!(read_field(task, TASK_REQUEST_OFF), req);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_with_url_nil_session() {
        assert_eq!(shim_nsurlsession_download_task_with_url(0, 0, 0), 0);
    }

    #[test]
    fn download_task_with_url_creates_valid_task() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let url = make_string("http://example.com/file.zip");
        let task = shim_nsurlsession_download_task_with_url(s, 0, url);
        assert_ne!(task, 0);
        assert_eq!(
            read_field(task, 0),
            nsobj::nsurlsessiondownloadtask_class_addr()
        );
        assert_eq!(read_field(task, TASK_TYPE_OFF), 2);
        assert_ne!(read_field(task, TASK_REQUEST_OFF), 0); // auto-created request
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_with_request_completion_nil_session() {
        assert_eq!(
            shim_nsurlsession_download_task_with_request_completion(0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn download_task_with_request_completion_stores_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let fake_block: u64 = 0xDEAD;
        let task = shim_nsurlsession_download_task_with_request_completion(s, 0, req, fake_block);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), fake_block);
        assert_eq!(read_field(task, TASK_TYPE_OFF), 2);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_with_url_completion_nil_session() {
        assert_eq!(
            shim_nsurlsession_download_task_with_url_completion(0, 0, 0, 0),
            0
        );
    }

    #[test]
    fn download_task_with_url_completion_stores_block() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let url = make_string("http://example.com/data.bin");
        let fake_block: u64 = 0xFACE;
        let task = shim_nsurlsession_download_task_with_url_completion(s, 0, url, fake_block);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_BLOCK_OFF), fake_block);
        assert_eq!(read_field(task, TASK_TYPE_OFF), 2);
        assert_eq!(
            read_field(task, 0),
            nsobj::nsurlsessiondownloadtask_class_addr()
        );
        free_obj(task);
        free_obj(s);
    }

    // ── upload/download resume without completion handler ─────

    #[test]
    fn upload_task_resume_without_handler_marks_completed() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let body = crate::nsdata::make_nsdata(b"x");
        let task = shim_nsurlsession_upload_task_with_request(s, 0, req, body);
        shim_nsurlsessiondatatask_resume(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 3);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_resume_without_handler_marks_completed() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_download_task_with_request(s, 0, 0);
        shim_nsurlsessiondatatask_resume(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 3);
        free_obj(task);
        free_obj(s);
    }

    // ── write_download_to_temp ───────────────────────────────

    #[test]
    fn write_download_to_temp_creates_file() {
        let url = write_download_to_temp(b"test download data");
        assert_ne!(url, 0);
        // Verify it's a file URL
        let is_file = crate::nsurl::shim_nsurl_is_file_url(url, 0);
        assert_ne!(is_file, 0);
        // Extract the path and verify the file exists with correct content
        let path_ns = crate::nsurl::shim_nsurl_path(url, 0);
        assert_ne!(path_ns, 0);
        let path_str =
            crate::cfstring::extract_cfstring_cstr(path_ns).expect("path should be extractable");
        let contents = std::fs::read(path_str).expect("temp file should exist");
        assert_eq!(contents, b"test download data");
        // Cleanup
        let _ = std::fs::remove_file(path_str);
    }

    #[test]
    fn write_download_to_temp_empty_data() {
        let url = write_download_to_temp(b"");
        assert_ne!(url, 0);
        let path_ns = crate::nsurl::shim_nsurl_path(url, 0);
        let path_str =
            crate::cfstring::extract_cfstring_cstr(path_ns).expect("path should be extractable");
        let contents = std::fs::read(path_str).expect("temp file should exist");
        assert!(contents.is_empty());
        let _ = std::fs::remove_file(path_str);
    }

    // ── description reflects task type ───────────────────────

    #[test]
    fn upload_task_description() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let body = crate::nsdata::make_nsdata(b"x");
        let task = shim_nsurlsession_upload_task_with_request(s, 0, req, body);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_string_eq(desc, "<NSURLSessionUploadTask: suspended>");
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_description() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_download_task_with_request(s, 0, 0);
        let desc = shim_nsurlsessiondatatask_description(task, 0);
        assert_string_eq(desc, "<NSURLSessionDownloadTask: suspended>");
        free_obj(task);
        free_obj(s);
    }

    // ── task lifecycle (cancel/suspend) works on upload/download ──

    #[test]
    fn upload_task_cancel() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let req = crate::nsurlrequest::shim_nsurlrequest_with_url(0, 0, 0);
        let task = shim_nsurlsession_upload_task_with_request(s, 0, req, 0);
        shim_nsurlsessiondatatask_cancel(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 2);
        free_obj(task);
        free_obj(s);
    }

    #[test]
    fn download_task_suspend() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_download_task_with_url(s, 0, 0);
        write_field(task, TASK_STATE_OFF, 1); // running
        shim_nsurlsessiondatatask_suspend(task, 0);
        assert_eq!(shim_nsurlsessiondatatask_state(task, 0), 0);
        free_obj(task);
        free_obj(s);
    }

    // ── reset / getAllTasks ───────────────────────────────────

    #[test]
    fn reset_with_completion_handler_nil_block() {
        // Must not crash
        shim_nsurlsession_reset_with_completion_handler(0, 0, 0);
    }

    #[test]
    fn all_tasks_with_completion_handler_nil_block() {
        // Must not crash
        shim_nsurlsession_all_tasks_with_completion_handler(0, 0, 0);
    }

    // ── NSURLSessionConfiguration: background + resource timeout ─

    #[test]
    fn config_background_creates_valid_object() {
        let id = make_string("com.test.bg");
        let cfg = shim_nsurlsessionconfig_background(
            nsobj::nsurlsessionconfiguration_class_addr(),
            0,
            id,
        );
        assert_ne!(cfg, 0);
        let isa = read_field(cfg, 0);
        assert_eq!(isa, nsobj::nsurlsessionconfiguration_class_addr());
        let t = shim_nsurlsessionconfig_timeout(cfg, 0);
        assert!((t - 60.0).abs() < f64::EPSILON);
        free_obj(cfg);
    }

    #[test]
    fn config_timeout_resource_returns_7_days() {
        let t = shim_nsurlsessionconfig_timeout_resource(0, 0);
        assert!((t - 604800.0).abs() < f64::EPSILON);
    }

    #[test]
    fn config_set_timeout_resource_noop() {
        // Must not crash
        shim_nsurlsessionconfig_set_timeout_resource(0, 0, 999.0);
    }

    // ── delegate session ─────────────────────────────────────

    #[test]
    fn session_with_delegate_stores_fields() {
        let cfg = shim_nsurlsessionconfig_default(nsobj::nsurlsessionconfiguration_class_addr(), 0);
        let fake_delegate: u64 = 0xD000;
        let fake_queue: u64 = 0xE000;
        let s = shim_nsurlsession_with_config_delegate(
            nsobj::nsurlsession_class_addr(),
            0,
            cfg,
            fake_delegate,
            fake_queue,
        );
        assert_ne!(s, 0);
        assert_eq!(read_field(s, SESSION_CONFIG_OFF), cfg);
        assert_eq!(read_field(s, SESSION_DELEGATE_OFF), fake_delegate);
        assert_eq!(read_field(s, SESSION_QUEUE_OFF), fake_queue);
        assert_eq!(shim_nsurlsession_delegate(s, 0), fake_delegate);
        assert_eq!(shim_nsurlsession_delegate_queue(s, 0), fake_queue);
        free_obj(s);
        free_obj(cfg);
    }

    #[test]
    fn session_delegate_nil() {
        assert_eq!(shim_nsurlsession_delegate(0, 0), 0);
        assert_eq!(shim_nsurlsession_delegate_queue(0, 0), 0);
    }

    #[test]
    fn shared_session_has_no_delegate() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        assert_eq!(shim_nsurlsession_delegate(s, 0), 0);
        assert_eq!(shim_nsurlsession_delegate_queue(s, 0), 0);
        free_obj(s);
    }

    #[test]
    fn task_stores_session_reference() {
        let s = shim_nsurlsession_shared(nsobj::nsurlsession_class_addr(), 0);
        let task = shim_nsurlsession_data_task_with_url(s, 0, 0);
        assert_ne!(task, 0);
        assert_eq!(read_field(task, TASK_SESSION_OFF), s);
        free_obj(task);
        free_obj(s);
    }
}
