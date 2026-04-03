//! Apple CFNetwork.framework stubs.
//!
//! CFNetwork is the C-level networking layer underneath NSURLSession.
//! Xcode CLI tools and Foundation code paths import these symbols.
//! All operations are safe no-ops: connections never open, DNS lookups
//! succeed vacuously, and HTTP messages return benign defaults.

use crate::table::ShimTable;

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

impl CfNetworkStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── CFHost (DNS resolution) ─────────────────────────────
        register_shims!(table,
            ("_CFHostCreateWithName", shim_cfnetwork_sentinel2 as u64, "CFHostCreateWithName (stub → sentinel)"),
            ("_CFHostStartInfoResolution", shim_cfnetwork_true3 as u64, "CFHostStartInfoResolution (stub → true)"),
            ("_CFHostGetAddressing", shim_cfnetwork_null2 as u64, "CFHostGetAddressing (stub → NULL)"),
            ("_CFHostGetNames", shim_cfnetwork_null2 as u64, "CFHostGetNames (stub → NULL)"),
            ("_CFHostSetClient", shim_cfnetwork_void3 as u64, "CFHostSetClient (stub → no-op)"),
            ("_CFHostScheduleWithRunLoop", shim_cfnetwork_void3 as u64, "CFHostScheduleWithRunLoop (stub → no-op)"),
            ("_CFHostUnscheduleFromRunLoop", shim_cfnetwork_void3 as u64, "CFHostUnscheduleFromRunLoop (stub → no-op)")
        );

        // ── CFHTTPMessage ───────────────────────────────────────
        register_shims!(table,
            ("_CFHTTPMessageCreateRequest", shim_cfnetwork_sentinel4 as u64, "CFHTTPMessageCreateRequest (stub → sentinel)"),
            ("_CFHTTPMessageCreateResponse", shim_cfnetwork_sentinel4 as u64, "CFHTTPMessageCreateResponse (stub → sentinel)"),
            ("_CFHTTPMessageCreateEmpty", shim_cfnetwork_sentinel2 as u64, "CFHTTPMessageCreateEmpty (stub → sentinel)"),
            ("_CFHTTPMessageAppendBytes", shim_cfnetwork_true3 as u64, "CFHTTPMessageAppendBytes (stub → true)"),
            ("_CFHTTPMessageCopyBody", shim_cfnetwork_null1 as u64, "CFHTTPMessageCopyBody (stub → NULL)"),
            ("_CFHTTPMessageCopyHeaderFieldValue", shim_cfnetwork_null2 as u64, "CFHTTPMessageCopyHeaderFieldValue (stub → NULL)"),
            ("_CFHTTPMessageCopyAllHeaderFields", shim_cfnetwork_null1 as u64, "CFHTTPMessageCopyAllHeaderFields (stub → NULL)"),
            ("_CFHTTPMessageCopyRequestURL", shim_cfnetwork_null1 as u64, "CFHTTPMessageCopyRequestURL (stub → NULL)"),
            ("_CFHTTPMessageCopyRequestMethod", shim_cfnetwork_null1 as u64, "CFHTTPMessageCopyRequestMethod (stub → NULL)"),
            ("_CFHTTPMessageGetResponseStatusCode", shim_cf_http_message_get_response_status_code as u64, "CFHTTPMessageGetResponseStatusCode (stub → 200)"),
            ("_CFHTTPMessageSetBody", shim_cfnetwork_void2 as u64, "CFHTTPMessageSetBody (stub → no-op)"),
            ("_CFHTTPMessageSetHeaderFieldValue", shim_cfnetwork_void3 as u64, "CFHTTPMessageSetHeaderFieldValue (stub → no-op)"),
            ("_CFHTTPMessageIsHeaderComplete", shim_cfnetwork_true1 as u64, "CFHTTPMessageIsHeaderComplete (stub → true)")
        );

        // ── CFHTTPStream ────────────────────────────────────────
        register_shims!(table,
            ("_CFReadStreamCreateForHTTPRequest", shim_cfnetwork_sentinel2 as u64, "CFReadStreamCreateForHTTPRequest (stub → sentinel)"),
            ("_CFHTTPReadStreamSetRedirectsAutomatically", shim_cfnetwork_void2 as u64, "CFHTTPReadStreamSetRedirectsAutomatically (stub → no-op)")
        );

        // ── CFNetDiagnostics ────────────────────────────────────
        register_shims!(table,
            ("_CFNetDiagnosticCreateWithStreams", shim_cfnetwork_sentinel3 as u64, "CFNetDiagnosticCreateWithStreams (stub → sentinel)"),
            ("_CFNetDiagnosticSetName", shim_cfnetwork_void2 as u64, "CFNetDiagnosticSetName (stub → no-op)"),
            ("_CFNetDiagnosticCopyNetworkStatusPassively", shim_cf_net_diagnostic_copy_status as u64, "CFNetDiagnosticCopyNetworkStatusPassively (stub → connected)")
        );

        // ── CFNetService (Bonjour) ──────────────────────────────
        register_shims!(table,
            ("_CFNetServiceCreate", shim_cfnetwork_sentinel4 as u64, "CFNetServiceCreate (stub → sentinel)"),
            ("_CFNetServiceBrowserCreate", shim_cfnetwork_sentinel2 as u64, "CFNetServiceBrowserCreate (stub → sentinel)"),
            ("_CFNetServiceRegisterWithOptions", shim_cfnetwork_true3 as u64, "CFNetServiceRegisterWithOptions (stub → true)"),
            ("_CFNetServiceBrowserSearchForServices", shim_cfnetwork_true4 as u64, "CFNetServiceBrowserSearchForServices (stub → true)"),
            ("_CFNetServiceCancel", shim_cfnetwork_void1 as u64, "CFNetServiceCancel (stub → no-op)"),
            ("_CFNetServiceBrowserStopSearch", shim_cfnetwork_void2 as u64, "CFNetServiceBrowserStopSearch (stub → no-op)"),
            ("_CFNetServiceGetName", shim_cfnetwork_null1 as u64, "CFNetServiceGetName (stub → NULL)")
        );

        // ── CFProxy (PAC / WPAD) ────────────────────────────────
        register_shims!(table,
            ("_CFNetworkCopyProxiesForURL", shim_cfnetwork_null2 as u64, "CFNetworkCopyProxiesForURL (stub → NULL)"),
            ("_CFNetworkCopyProxiesForAutoConfigurationScript", shim_cfnetwork_null3 as u64, "CFNetworkCopyProxiesForAutoConfigurationScript (stub → NULL)"),
            ("_CFNetworkCopySystemProxySettings", shim_cfnetwork_null0 as u64, "CFNetworkCopySystemProxySettings (stub → NULL)")
        );

        // ── CFSocketStream ──────────────────────────────────────
        register_shims!(table,
            ("_CFStreamCreatePairWithSocketToHost", shim_cf_stream_create_pair_with_host as u64, "CFStreamCreatePairWithSocketToHost (stub → no-op, NULLs)"),
            ("_CFStreamCreatePairWithSocket", shim_cf_stream_create_pair_with_socket as u64, "CFStreamCreatePairWithSocket (stub → no-op)")
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
}
