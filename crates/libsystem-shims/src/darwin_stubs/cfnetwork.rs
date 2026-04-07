//! Apple CFNetwork.framework — real DNS + benign HTTP stubs.
//!
//! CFHost DNS resolution is backed by hickory-resolver (pure-Rust DNS).
//! CFHTTPMessage and other HTTP APIs remain safe stubs since NSURLSession
//! already provides real HTTP via ureq.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct CfNetworkStubs;

// ── Sentinels ───────────────────────────────────────────────────

/// Generic non-NULL CFNetwork object sentinel.
static CFNETWORK_SENTINEL: u64 = 0x4346_4E57_0001;

// ── kCF* data sentinels (string constants) ──────────────────────

static K_CF_STREAM_HTTP_RESPONSE: u64 = 0x4346_4E57_0100;
static K_CF_STREAM_HTTP_FINAL_URL: u64 = 0x4346_4E57_0101;
static K_CF_STREAM_SSL_SETTINGS: u64 = 0x4346_4E57_0102;
static K_CF_HTTP_VERSION_1_0: u64 = 0x4346_4E57_0103;
static K_CF_HTTP_VERSION_1_1: u64 = 0x4346_4E57_0104;
static K_CF_HTTP_VERSION_2_0: u64 = 0x4346_4E57_0105;
static K_CF_STREAM_HTTP_AUTOREDIRECT: u64 = 0x4346_4E57_0106;
static K_CF_NETWORK_PROXIES_HTTP_ENABLE: u64 = 0x4346_4E57_0107;
static K_CF_NETWORK_PROXIES_HTTP_PROXY: u64 = 0x4346_4E57_0108;
static K_CF_NETWORK_PROXIES_HTTP_PORT: u64 = 0x4346_4E57_0109;

// ── Real CFHost DNS infrastructure ──────────────────────────────

static NEXT_CFHOST_HANDLE: AtomicU64 = AtomicU64::new(0x4346_4800_0001);

struct CfHostEntry {
    hostname: String,
    /// Resolved IPv4/IPv6 addresses (stored as sockaddr bytes in macOS layout).
    resolved_addrs: Option<Vec<Vec<u8>>>,
    /// CFArrayRef pointing to the resolved addresses (allocated on first access).
    cached_array: u64,
}

static CFHOST_STORE: Mutex<Option<HashMap<u64, CfHostEntry>>> = Mutex::new(None);

fn cfhost_store() -> std::sync::MutexGuard<'static, Option<HashMap<u64, CfHostEntry>>> {
    CFHOST_STORE.lock().unwrap_or_else(|e| e.into_inner())
}

impl CfNetworkStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── CFHost (real DNS resolution via hickory-resolver) ───
        register_shims!(table,
            ("_CFHostCreateWithName", shim_cfhost_create_with_name as *const () as u64, "CFHostCreateWithName (real DNS)"),
            ("_CFHostStartInfoResolution", shim_cfhost_start_info_resolution as *const () as u64, "CFHostStartInfoResolution (real DNS lookup)"),
            ("_CFHostGetAddressing", shim_cfhost_get_addressing as *const () as u64, "CFHostGetAddressing (real resolved addresses)"),
            ("_CFHostGetNames", shim_cfnetwork_null2 as *const () as u64, "CFHostGetNames (stub → NULL)"),
            ("_CFHostSetClient", shim_cfnetwork_void3 as *const () as u64, "CFHostSetClient (stub → no-op)"),
            ("_CFHostScheduleWithRunLoop", shim_cfnetwork_void3 as *const () as u64, "CFHostScheduleWithRunLoop (stub → no-op)"),
            ("_CFHostUnscheduleFromRunLoop", shim_cfnetwork_void3 as *const () as u64, "CFHostUnscheduleFromRunLoop (stub → no-op)")
        );

        // ── CFHTTPMessage ───────────────────────────────────────
        register_shims!(table,
            ("_CFHTTPMessageCreateRequest", shim_cfnetwork_sentinel4 as *const () as u64, "CFHTTPMessageCreateRequest (stub → sentinel)"),
            ("_CFHTTPMessageCreateResponse", shim_cfnetwork_sentinel4 as *const () as u64, "CFHTTPMessageCreateResponse (stub → sentinel)"),
            ("_CFHTTPMessageCreateEmpty", shim_cfnetwork_sentinel2 as *const () as u64, "CFHTTPMessageCreateEmpty (stub → sentinel)"),
            ("_CFHTTPMessageAppendBytes", shim_cfnetwork_true3 as *const () as u64, "CFHTTPMessageAppendBytes (stub → true)"),
            ("_CFHTTPMessageCopyBody", shim_cfnetwork_null1 as *const () as u64, "CFHTTPMessageCopyBody (stub → NULL)"),
            ("_CFHTTPMessageCopyHeaderFieldValue", shim_cfnetwork_null2 as *const () as u64, "CFHTTPMessageCopyHeaderFieldValue (stub → NULL)"),
            ("_CFHTTPMessageCopyAllHeaderFields", shim_cfnetwork_null1 as *const () as u64, "CFHTTPMessageCopyAllHeaderFields (stub → NULL)"),
            ("_CFHTTPMessageCopyRequestURL", shim_cfnetwork_null1 as *const () as u64, "CFHTTPMessageCopyRequestURL (stub → NULL)"),
            ("_CFHTTPMessageCopyRequestMethod", shim_cfnetwork_null1 as *const () as u64, "CFHTTPMessageCopyRequestMethod (stub → NULL)"),
            ("_CFHTTPMessageGetResponseStatusCode", shim_cf_http_message_get_response_status_code as *const () as u64, "CFHTTPMessageGetResponseStatusCode (stub → 200)"),
            ("_CFHTTPMessageSetBody", shim_cfnetwork_void2 as *const () as u64, "CFHTTPMessageSetBody (stub → no-op)"),
            ("_CFHTTPMessageSetHeaderFieldValue", shim_cfnetwork_void3 as *const () as u64, "CFHTTPMessageSetHeaderFieldValue (stub → no-op)"),
            ("_CFHTTPMessageIsHeaderComplete", shim_cfnetwork_true1 as *const () as u64, "CFHTTPMessageIsHeaderComplete (stub → true)")
        );

        // ── CFHTTPStream ────────────────────────────────────────
        register_shims!(table,
            ("_CFReadStreamCreateForHTTPRequest", shim_cfnetwork_sentinel2 as *const () as u64, "CFReadStreamCreateForHTTPRequest (stub → sentinel)"),
            ("_CFHTTPReadStreamSetRedirectsAutomatically", shim_cfnetwork_void2 as *const () as u64, "CFHTTPReadStreamSetRedirectsAutomatically (stub → no-op)")
        );

        // ── CFNetDiagnostics ────────────────────────────────────
        register_shims!(table,
            ("_CFNetDiagnosticCreateWithStreams", shim_cfnetwork_sentinel3 as *const () as u64, "CFNetDiagnosticCreateWithStreams (stub → sentinel)"),
            ("_CFNetDiagnosticSetName", shim_cfnetwork_void2 as *const () as u64, "CFNetDiagnosticSetName (stub → no-op)"),
            ("_CFNetDiagnosticCopyNetworkStatusPassively", shim_cf_net_diagnostic_copy_status as *const () as u64, "CFNetDiagnosticCopyNetworkStatusPassively (stub → connected)")
        );

        // ── CFNetService (Bonjour) ──────────────────────────────
        register_shims!(table,
            ("_CFNetServiceCreate", shim_cfnetwork_sentinel4 as *const () as u64, "CFNetServiceCreate (stub → sentinel)"),
            ("_CFNetServiceBrowserCreate", shim_cfnetwork_sentinel2 as *const () as u64, "CFNetServiceBrowserCreate (stub → sentinel)"),
            ("_CFNetServiceRegisterWithOptions", shim_cfnetwork_true3 as *const () as u64, "CFNetServiceRegisterWithOptions (stub → true)"),
            ("_CFNetServiceBrowserSearchForServices", shim_cfnetwork_true4 as *const () as u64, "CFNetServiceBrowserSearchForServices (stub → true)"),
            ("_CFNetServiceCancel", shim_cfnetwork_void1 as *const () as u64, "CFNetServiceCancel (stub → no-op)"),
            ("_CFNetServiceBrowserStopSearch", shim_cfnetwork_void2 as *const () as u64, "CFNetServiceBrowserStopSearch (stub → no-op)"),
            ("_CFNetServiceGetName", shim_cfnetwork_null1 as *const () as u64, "CFNetServiceGetName (stub → NULL)")
        );

        // ── CFProxy (PAC / WPAD) ────────────────────────────────
        register_shims!(table,
            ("_CFNetworkCopyProxiesForURL", shim_cfnetwork_null2 as *const () as u64, "CFNetworkCopyProxiesForURL (stub → NULL)"),
            ("_CFNetworkCopyProxiesForAutoConfigurationScript", shim_cfnetwork_null3 as *const () as u64, "CFNetworkCopyProxiesForAutoConfigurationScript (stub → NULL)"),
            ("_CFNetworkCopySystemProxySettings", shim_cfnetwork_null0 as *const () as u64, "CFNetworkCopySystemProxySettings (stub → NULL)")
        );

        // ── CFSocketStream ──────────────────────────────────────
        register_shims!(table,
            ("_CFStreamCreatePairWithSocketToHost", shim_cf_stream_create_pair_with_host as *const () as u64, "CFStreamCreatePairWithSocketToHost (stub → no-op, NULLs)"),
            ("_CFStreamCreatePairWithSocket", shim_cf_stream_create_pair_with_socket as *const () as u64, "CFStreamCreatePairWithSocket (stub → no-op)")
        );

        // ── Data symbols (string constants) ─────────────────────
        register_shims!(table,
            ("_kCFStreamPropertyHTTPResponseHeader", &K_CF_STREAM_HTTP_RESPONSE as *const u64 as u64, "kCFStreamPropertyHTTPResponseHeader data symbol"),
            ("_kCFStreamPropertyHTTPFinalURL", &K_CF_STREAM_HTTP_FINAL_URL as *const u64 as u64, "kCFStreamPropertyHTTPFinalURL data symbol"),
            ("_kCFStreamPropertySSLSettings", &K_CF_STREAM_SSL_SETTINGS as *const u64 as u64, "kCFStreamPropertySSLSettings data symbol"),
            ("_kCFHTTPVersion1_0", &K_CF_HTTP_VERSION_1_0 as *const u64 as u64, "kCFHTTPVersion1_0 data symbol"),
            ("_kCFHTTPVersion1_1", &K_CF_HTTP_VERSION_1_1 as *const u64 as u64, "kCFHTTPVersion1_1 data symbol"),
            ("_kCFHTTPVersion2_0", &K_CF_HTTP_VERSION_2_0 as *const u64 as u64, "kCFHTTPVersion2_0 data symbol"),
            ("_kCFStreamPropertyHTTPShouldAutoredirect", &K_CF_STREAM_HTTP_AUTOREDIRECT as *const u64 as u64, "kCFStreamPropertyHTTPShouldAutoredirect data symbol"),
            ("_kCFNetworkProxiesHTTPEnable", &K_CF_NETWORK_PROXIES_HTTP_ENABLE as *const u64 as u64, "kCFNetworkProxiesHTTPEnable data symbol"),
            ("_kCFNetworkProxiesHTTPProxy", &K_CF_NETWORK_PROXIES_HTTP_PROXY as *const u64 as u64, "kCFNetworkProxiesHTTPProxy data symbol"),
            ("_kCFNetworkProxiesHTTPPort", &K_CF_NETWORK_PROXIES_HTTP_PORT as *const u64 as u64, "kCFNetworkProxiesHTTPPort data symbol")
        );

        // ── POSIX DNS shims (domain rewrite interception) ───────
        register_shims!(table,
            ("_getaddrinfo", shim_getaddrinfo as *const () as u64, "getaddrinfo (domain rewrite + forward to libc)"),
            ("_freeaddrinfo", shim_freeaddrinfo as *const () as u64, "freeaddrinfo (forward to libc)"),
            ("_getnameinfo", shim_getnameinfo as *const () as u64, "getnameinfo (forward to libc)"),
            ("_gethostbyname", shim_gethostbyname as *const () as u64, "gethostbyname (domain rewrite + forward to libc)"),
            ("_gethostbyname2", shim_gethostbyname2 as *const () as u64, "gethostbyname2 (domain rewrite + forward to libc)")
        );

        // ── libresolv stubs (prevent DNS leaks to host) ─────────
        register_shims!(table,
            ("_res_query", shim_res_query as *const () as u64, "res_query (stub → SERVFAIL)"),
            ("_res_search", shim_res_search as *const () as u64, "res_search (stub → SERVFAIL)"),
            ("_res_send", shim_res_send as *const () as u64, "res_send (stub → SERVFAIL)"),
            ("_res_init", shim_res_init as *const () as u64, "res_init (stub → success)"),
            ("_dn_expand", shim_dn_expand as *const () as u64, "dn_expand (stub → error)")
        );

        log::debug!("CfNetworkStubs: registered CFNetwork.framework stubs");
    }
}

// ── Generic building-block stubs ────────────────────────────────

pub extern "C" fn shim_cfnetwork_void1(_a: u64) {}
pub extern "C" fn shim_cfnetwork_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_cfnetwork_void3(_a: u64, _b: u64, _c: u64) {}

pub extern "C" fn shim_cfnetwork_null0() -> u64 { 0 }
pub extern "C" fn shim_cfnetwork_null1(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_cfnetwork_null2(_a: u64, _b: u64) -> u64 { 0 }
pub extern "C" fn shim_cfnetwork_null3(_a: u64, _b: u64, _c: u64) -> u64 { 0 }

pub extern "C" fn shim_cfnetwork_true1(_a: u64) -> i32 { 1 }
pub extern "C" fn shim_cfnetwork_true3(_a: u64, _b: u64, _c: u64) -> i32 { 1 }
pub extern "C" fn shim_cfnetwork_true4(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 { 1 }

pub extern "C" fn shim_cfnetwork_sentinel2(_a: u64, _b: u64) -> u64 {
    &CFNETWORK_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_cfnetwork_sentinel3(_a: u64, _b: u64, _c: u64) -> u64 {
    &CFNETWORK_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_cfnetwork_sentinel4(_a: u64, _b: u64, _c: u64, _d: u64) -> u64 {
    &CFNETWORK_SENTINEL as *const u64 as u64
}

// ── Specific shim functions ─────────────────────────────────────

// ── Real CFHost DNS implementation ──────────────────────────────

/// CFHostCreateWithName(allocator, hostname_cfstring) → CFHostRef
pub extern "C" fn shim_cfhost_create_with_name(_alloc: u64, name: u64) -> u64 {
    let hostname = if name != 0 {
        // Read C string pointer from CFString at offset +16
        let cstr_ptr = unsafe { *((name as usize + 16) as *const u64) };
        if cstr_ptr > 0x1000 {
            unsafe { std::ffi::CStr::from_ptr(cstr_ptr as *const std::ffi::c_char) }
                .to_str()
                .unwrap_or("localhost")
                .to_string()
        } else {
            "localhost".to_string()
        }
    } else {
        "localhost".to_string()
    };

    let handle = NEXT_CFHOST_HANDLE.fetch_add(1, Ordering::Relaxed);
    let entry = CfHostEntry {
        hostname: hostname.clone(),
        resolved_addrs: None,
        cached_array: 0,
    };
    let mut guard = cfhost_store();
    guard.get_or_insert_with(HashMap::new).insert(handle, entry);
    log::trace!("CFHostCreateWithName: '{hostname}' → handle={handle:#x}");
    handle
}

/// CFHostStartInfoResolution(host, infoType, &error) → Boolean
///
/// Real DNS lookup using libc getaddrinfo (synchronous, uses system resolver).
pub extern "C" fn shim_cfhost_start_info_resolution(host: u64, _info_type: i32, _error: *mut u64) -> i32 {
    let mut guard = cfhost_store();
    let store = match guard.as_mut() {
        Some(s) => s,
        None => return 0,
    };
    let entry = match store.get_mut(&host) {
        Some(e) => e,
        None => return 0,
    };

    // BurdettieOS DNS rewrite — redirect Apple domains to local service
    let resolve_hostname = match burdettie_domain_map::rewrite_hostname(&entry.hostname) {
        Some(rewritten) => rewritten,
        None => entry.hostname.clone(),
    };

    // Use libc getaddrinfo for portable DNS resolution (works on both macOS and Linux)
    let hostname_cstr = match std::ffi::CString::new(resolve_hostname.as_str()) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = libc::AF_UNSPEC;
    hints.ai_socktype = libc::SOCK_STREAM;

    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe {
        libc::getaddrinfo(
            hostname_cstr.as_ptr(),
            std::ptr::null(),
            &hints,
            &mut result,
        )
    };

    if rc != 0 || result.is_null() {
        log::debug!("CFHostStartInfoResolution: getaddrinfo failed for '{}' (resolving as '{}', rc={rc})", entry.hostname, resolve_hostname);
        entry.resolved_addrs = Some(Vec::new());
        return 0;
    }

    let mut addrs = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let ai = unsafe { &*current };
        if !ai.ai_addr.is_null() && ai.ai_addrlen > 0 {
            let sa_bytes = unsafe {
                std::slice::from_raw_parts(ai.ai_addr as *const u8, ai.ai_addrlen as usize)
            };
            addrs.push(sa_bytes.to_vec());
        }
        current = ai.ai_next;
    }
    unsafe { libc::freeaddrinfo(result); }

    log::debug!("CFHostStartInfoResolution: '{}' → {} addresses", entry.hostname, addrs.len());
    entry.resolved_addrs = Some(addrs);
    1 // TRUE
}

/// CFHostGetAddressing(host, &hasBeenResolved) → CFArrayRef
///
/// Returns a CFArray of sockaddr data objects from the resolved addresses.
pub extern "C" fn shim_cfhost_get_addressing(host: u64, resolved: *mut i32) -> u64 {
    let mut guard = cfhost_store();
    let store = match guard.as_mut() {
        Some(s) => s,
        None => {
            if !resolved.is_null() { unsafe { *resolved = 0; } }
            return 0;
        }
    };
    let entry = match store.get_mut(&host) {
        Some(e) => e,
        None => {
            if !resolved.is_null() { unsafe { *resolved = 0; } }
            return 0;
        }
    };

    match &entry.resolved_addrs {
        None => {
            if !resolved.is_null() { unsafe { *resolved = 0; } }
            0
        }
        Some(addrs) => {
            if !resolved.is_null() { unsafe { *resolved = 1; } }
            if addrs.is_empty() {
                return 0;
            }
            // Return cached array if already built
            if entry.cached_array != 0 {
                return entry.cached_array;
            }
            // Build a CFArray of CFData objects containing sockaddr bytes
            let count = addrs.len();
            // Allocate array: [isa:8][count:8][items_ptr:8][pad:8]
            let array = unsafe { libc::calloc(1, 32) as u64 };
            if array == 0 { return 0; }

            let items = unsafe { libc::calloc(count, 8) as *mut u64 };
            if items.is_null() { return 0; }

            for (i, sa_bytes) in addrs.iter().enumerate() {
                // CFData: [isa:8][bytes_ptr:8][length:8]
                let data_obj = unsafe { libc::calloc(1, 24) as u64 };
                if data_obj != 0 && !sa_bytes.is_empty() {
                    let buf = unsafe { libc::malloc(sa_bytes.len()) as *mut u8 };
                    if !buf.is_null() {
                        unsafe {
                            std::ptr::copy_nonoverlapping(sa_bytes.as_ptr(), buf, sa_bytes.len());
                            *((data_obj as usize + 8) as *mut u64) = buf as u64;
                            *((data_obj as usize + 16) as *mut u64) = sa_bytes.len() as u64;
                        }
                    }
                }
                unsafe { items.add(i).write(data_obj); }
            }

            unsafe {
                *((array as usize + 8) as *mut u64) = count as u64;
                *((array as usize + 16) as *mut u64) = items as u64;
            }

            entry.cached_array = array;
            array
        }
    }
}

/// CFHTTPMessageGetResponseStatusCode(message) → CFIndex (200)
pub extern "C" fn shim_cf_http_message_get_response_status_code(_msg: u64) -> i64 {
    200
}

/// CFNetDiagnosticCopyNetworkStatusPassively(diagnostic, &status) → CFStringRef
/// Sets status to kCFNetDiagnosticConnectionUp (0) and returns non-NULL sentinel.
pub extern "C" fn shim_cf_net_diagnostic_copy_status(_diag: u64, status: *mut i32) -> u64 {
    if !status.is_null() {
        unsafe { *status = 0; } // kCFNetDiagnosticConnectionUp
    }
    &CFNETWORK_SENTINEL as *const u64 as u64
}

/// CFStreamCreatePairWithSocketToHost(alloc, host, port, &readStream, &writeStream)
/// Sets both out-params to NULL.
pub extern "C" fn shim_cf_stream_create_pair_with_host(
    _alloc: u64,
    _host: u64,
    _port: u32,
    read_stream: *mut u64,
    write_stream: *mut u64,
) {
    if !read_stream.is_null() {
        unsafe { *read_stream = 0; }
    }
    if !write_stream.is_null() {
        unsafe { *write_stream = 0; }
    }
}

/// CFStreamCreatePairWithSocket(alloc, sock, &readStream, &writeStream)
pub extern "C" fn shim_cf_stream_create_pair_with_socket(
    _alloc: u64,
    _sock: i32,
    read_stream: *mut u64,
    write_stream: *mut u64,
) {
    if !read_stream.is_null() {
        unsafe { *read_stream = 0; }
    }
    if !write_stream.is_null() {
        unsafe { *write_stream = 0; }
    }
}

// ── POSIX DNS shim implementations ──────────────────────────────

/// Read a C string from the guest at `ptr`, returning an owned String.
fn read_cstr(ptr: *const std::ffi::c_char) -> Option<String> {
    if ptr.is_null() { return None; }
    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
    cstr.to_str().ok().map(|s| s.to_string())
}

/// getaddrinfo(node, service, hints, res) → int
///
/// Intercepts DNS lookups: applies BurdettieOS domain rewrite for Apple
/// domains (redirecting to the local proxy), then forwards to libc.
pub extern "C" fn shim_getaddrinfo(
    node: *const std::ffi::c_char,
    service: *const std::ffi::c_char,
    hints: *const libc::addrinfo,
    res: *mut *mut libc::addrinfo,
) -> i32 {
    let hostname = read_cstr(node);
    let resolve_hostname = hostname.as_deref().and_then(|h| {
        burdettie_domain_map::rewrite_hostname(h)
    });

    if let Some(ref rewritten) = resolve_hostname {
        log::info!("getaddrinfo: domain rewrite {} → {rewritten}", hostname.as_deref().unwrap_or("?"));
        let cstr = match std::ffi::CString::new(rewritten.as_str()) {
            Ok(c) => c,
            Err(_) => return libc::EAI_FAIL,
        };
        return unsafe { libc::getaddrinfo(cstr.as_ptr(), service, hints, res) };
    }

    // No rewrite — pass through to host libc
    unsafe { libc::getaddrinfo(node, service, hints, res) }
}

/// freeaddrinfo(res) — direct forward to libc.
pub extern "C" fn shim_freeaddrinfo(res: *mut libc::addrinfo) {
    unsafe { libc::freeaddrinfo(res); }
}

/// getnameinfo — direct forward to libc (reverse DNS doesn't need rewrite).
pub extern "C" fn shim_getnameinfo(
    sa: *const libc::sockaddr,
    salen: libc::socklen_t,
    host: *mut std::ffi::c_char,
    hostlen: libc::socklen_t,
    serv: *mut std::ffi::c_char,
    servlen: libc::socklen_t,
    flags: i32,
) -> i32 {
    unsafe { libc::getnameinfo(sa, salen, host, hostlen, serv, servlen, flags) }
}

/// gethostbyname(name) → *hostent
///
/// Returns NULL (host not found). Guest should use getaddrinfo instead.
/// Legacy gethostbyname is not thread-safe and deprecated.
pub extern "C" fn shim_gethostbyname(name: *const std::ffi::c_char) -> u64 {
    let hostname = read_cstr(name).unwrap_or_default();
    log::debug!("gethostbyname: '{hostname}' → NULL (use getaddrinfo)");
    0
}

/// gethostbyname2(name, af) → *hostent
///
/// Returns NULL (host not found). Guest should use getaddrinfo instead.
pub extern "C" fn shim_gethostbyname2(name: *const std::ffi::c_char, _af: i32) -> u64 {
    let hostname = read_cstr(name).unwrap_or_default();
    log::debug!("gethostbyname2: '{hostname}' → NULL (use getaddrinfo)");
    0
}

// ── libresolv stub implementations ──────────────────────────────
//
// These prevent DNS queries via libresolv from leaking to the host's
// real DNS resolver, bypassing BurdettieOS domain interception.
// Returns -1 (SERVFAIL equivalent) for all wire-format queries.

/// res_query(dname, class, type, answer, anslen) → int
///
/// Stub returns -1 (error). Guest binaries that need DNS should use
/// getaddrinfo (which we intercept) instead of raw libresolv.
pub extern "C" fn shim_res_query(
    dname: *const std::ffi::c_char,
    _class: i32,
    _type: i32,
    _answer: *mut u8,
    _anslen: i32,
) -> i32 {
    let name = read_cstr(dname).unwrap_or_default();
    log::debug!("res_query: blocked query for '{name}' (use getaddrinfo instead)");
    -1
}

/// res_search(dname, class, type, answer, anslen) → int
pub extern "C" fn shim_res_search(
    dname: *const std::ffi::c_char,
    _class: i32,
    _type: i32,
    _answer: *mut u8,
    _anslen: i32,
) -> i32 {
    let name = read_cstr(dname).unwrap_or_default();
    log::debug!("res_search: blocked query for '{name}' (use getaddrinfo instead)");
    -1
}

/// res_send(msg, msglen, answer, anslen) → int
pub extern "C" fn shim_res_send(
    _msg: *const u8,
    _msglen: i32,
    _answer: *mut u8,
    _anslen: i32,
) -> i32 {
    log::debug!("res_send: blocked raw DNS send");
    -1
}

/// res_init() → int  (always succeeds, no real resolver state needed)
pub extern "C" fn shim_res_init() -> i32 {
    0
}

/// dn_expand(msg, eomorig, comp_dn, exp_dn, length) → int
pub extern "C" fn shim_dn_expand(
    _msg: *const u8,
    _eomorig: *const u8,
    _comp_dn: *const u8,
    _exp_dn: *mut u8,
    _length: i32,
) -> i32 {
    -1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn cfnetwork_stubs_registered() {
        let mut table = ShimTable::new();
        CfNetworkStubs::install(&mut table);
        assert!(table.resolve("_CFHostCreateWithName").is_some());
        assert!(table.resolve("_CFHTTPMessageCreateRequest").is_some());
        assert!(table.resolve("_CFNetworkCopySystemProxySettings").is_some());
    }

    #[test]
    fn http_message_status_code_returns_200() {
        assert_eq!(shim_cf_http_message_get_response_status_code(0), 200);
    }

    #[test]
    fn data_sentinels_are_non_null() {
        let mut table = ShimTable::new();
        CfNetworkStubs::install(&mut table);
        assert!(table.resolve("_kCFHTTPVersion1_1").is_some());
        let addr = table.resolve("_kCFHTTPVersion1_1").unwrap();
        assert_ne!(addr, 0);
    }

    #[test]
    fn posix_dns_shims_registered() {
        let mut table = ShimTable::new();
        CfNetworkStubs::install(&mut table);
        assert!(table.resolve("_getaddrinfo").is_some());
        assert!(table.resolve("_freeaddrinfo").is_some());
        assert!(table.resolve("_getnameinfo").is_some());
        assert!(table.resolve("_gethostbyname").is_some());
        assert!(table.resolve("_gethostbyname2").is_some());
    }

    #[test]
    fn libresolv_stubs_registered() {
        let mut table = ShimTable::new();
        CfNetworkStubs::install(&mut table);
        assert!(table.resolve("_res_query").is_some());
        assert!(table.resolve("_res_search").is_some());
        assert!(table.resolve("_res_send").is_some());
        assert!(table.resolve("_res_init").is_some());
        assert!(table.resolve("_dn_expand").is_some());
    }

    #[test]
    fn res_query_returns_error() {
        let name = std::ffi::CString::new("apple.com").unwrap();
        let mut buf = [0u8; 512];
        assert_eq!(shim_res_query(name.as_ptr(), 1, 1, buf.as_mut_ptr(), 512), -1);
    }

    #[test]
    fn res_search_returns_error() {
        let name = std::ffi::CString::new("apple.com").unwrap();
        let mut buf = [0u8; 512];
        assert_eq!(shim_res_search(name.as_ptr(), 1, 1, buf.as_mut_ptr(), 512), -1);
    }

    #[test]
    fn res_send_returns_error() {
        let msg = [0u8; 12];
        let mut buf = [0u8; 512];
        assert_eq!(shim_res_send(msg.as_ptr(), 12, buf.as_mut_ptr(), 512), -1);
    }

    #[test]
    fn res_init_returns_success() {
        assert_eq!(shim_res_init(), 0);
    }

    #[test]
    fn getaddrinfo_resolves_non_apple_domain() {
        // Non-Apple domains should pass through to host libc
        let node = std::ffi::CString::new("localhost").unwrap();
        let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
        hints.ai_family = libc::AF_UNSPEC;
        hints.ai_socktype = libc::SOCK_STREAM;
        let mut res: *mut libc::addrinfo = std::ptr::null_mut();
        let rc = shim_getaddrinfo(
            node.as_ptr(),
            std::ptr::null(),
            &hints,
            &mut res,
        );
        assert_eq!(rc, 0);
        assert!(!res.is_null());
        shim_freeaddrinfo(res);
    }
}
