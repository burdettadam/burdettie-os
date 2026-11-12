//! XPC Pipe — in-process message routing.
//!
//! Apple's `xpc_pipe` APIs provide a low-level IPC mechanism used by
//! tools like `xpcproxy` and `launchctl` to communicate with launchd.
//! Without a real launchd, we provide:
//!
//! - Named pipe registry: services can register pipe handlers.
//! - `xpc_pipe_routine` routes messages to registered handlers and
//!   writes replies back.  Unregistered pipes return EPIPE.
//! - `xpc_pipe_routine_with_flags` variant.

use std::collections::HashMap;
use std::sync::Mutex;

use super::xpc_objects;

/// EPIPE error code (no service on the other end).
const EPIPE: i32 = 32;

/// A registered pipe service handler.
///
/// The handler takes a request dictionary handle and returns a reply
/// dictionary handle (or 0 for no reply).
type PipeHandler = fn(request: u64) -> u64;

/// Global pipe registry: pipe name → handler function.
static PIPE_HANDLERS: Mutex<Option<HashMap<String, PipeHandler>>> = Mutex::new(None);

fn with_pipes<R>(f: impl FnOnce(&mut HashMap<String, PipeHandler>) -> R) -> R {
    let mut guard = PIPE_HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

/// Register a named pipe service with a Rust handler function.
///
/// When `xpc_pipe_routine` is called on a pipe created with this name,
/// the handler will be invoked with the request dictionary and can
/// return a reply dictionary.
pub fn register_pipe_handler(name: &str, handler: PipeHandler) {
    log::debug!("xpc_pipe: registering handler for '{}'", name);
    with_pipes(|t| {
        t.insert(name.to_string(), handler);
    });
}

/// Unregister a pipe handler.
pub fn unregister_pipe_handler(name: &str) {
    with_pipes(|t| {
        t.remove(name);
    });
}

/// Check if a pipe handler is registered.
pub fn has_pipe_handler(name: &str) -> bool {
    with_pipes(|t| t.contains_key(name))
}

/// Route a message directly to a pipe handler by service name.
///
/// Returns the reply handle (0 if no handler or handler returned 0).
/// Used by `xpc_transport` to route sync/async replies through pipe
/// handlers before falling back to socket transport.
pub fn route_to_handler(name: &str, request: u64) -> u64 {
    let handler = with_pipes(|t| t.get(name).copied());
    match handler {
        Some(h) => h(request),
        None => 0,
    }
}

/// Create a pipe object.
///
/// Returns a dictionary-backed handle with the pipe name and flags stored.
pub fn create(name_ptr: u64, flags: u64) -> u64 {
    let pipe = xpc_objects::create_dictionary_empty();
    if name_ptr != 0 {
        xpc_objects::dict_set_string(pipe, b"_pipe_name\0".as_ptr() as u64, name_ptr);
    }
    xpc_objects::dict_set_uint64(pipe, b"_pipe_flags\0".as_ptr() as u64, flags);
    log::debug!("xpc_pipe: created pipe handle {:#x}", pipe);
    pipe
}

/// `xpc_pipe_routine(pipe, request, reply_out)` → 0 on success, errno on failure.
///
/// Looks up the pipe's name in the handler registry.  If a handler is
/// registered, invokes it with the request and writes the reply to
/// `reply_out`.  Otherwise returns EPIPE.
pub fn routine(pipe: u64, request: u64, reply_out: u64) -> i32 {
    // Extract the pipe name from the pipe dict
    let name_ptr = xpc_objects::dict_get_string(pipe, b"_pipe_name\0".as_ptr() as u64);
    if name_ptr == 0 {
        log::debug!("xpc_pipe: routine on pipe with no name → EPIPE");
        return EPIPE;
    }

    let name = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const std::ffi::c_char) };
    let name_str = name.to_str().unwrap_or("");

    // Look up handler
    let handler = with_pipes(|t| t.get(name_str).copied());

    match handler {
        Some(h) => {
            log::debug!("xpc_pipe: routing to handler for '{}'", name_str);
            let reply = h(request);

            // Record XPC activity through the log pipeline
            burdettie_log_pipeline::emit(burdettie_log_pipeline::LogEvent {
                subsystem: "com.burdettie.xpc".into(),
                category: "pipe".into(),
                level: burdettie_log_pipeline::LogLevel::Info,
                message: format!("xpc_pipe_routine: {} → reply={:#x}", name_str, reply),
                sequence: 0,
                timestamp_ms: 0,
            });

            // Record to SQLite if the global recorder is available
            if let Some(recorder) = burdettie_services::global_recorder() {
                recorder.record_xpc(&burdettie_services::XpcRecord {
                    service: name_str.to_string(),
                    request_json: format!("{{\"handle\":\"{:#x}\"}}", request),
                    reply_json: format!("{{\"handle\":\"{:#x}\"}}", reply),
                    guest_binary: String::new(),
                });
            }

            // Write reply handle to reply_out pointer if non-NULL
            if reply_out != 0 && reply != 0 {
                unsafe {
                    *(reply_out as *mut u64) = reply;
                }
            }
            0 // success
        }
        None => {
            log::debug!("xpc_pipe: no handler for '{}' → EPIPE", name_str);
            EPIPE
        }
    }
}

/// Default handler for launchd domain queries.
///
/// Returns a minimal reply dictionary with an error string indicating
/// no launchd is available.  Registered automatically for common
/// launchd pipe names.
fn launchd_stub_handler(request: u64) -> u64 {
    let _ = request;
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, EPIPE as i64);
    xpc_objects::dict_set_string(
        reply,
        b"error-string\0".as_ptr() as u64,
        b"launchd not available in emulation\0".as_ptr() as u64,
    );
    reply
}

// ── BurdettieOS daemon handlers ─────────────────────────────────

/// Analytics daemon — accepts analytics submissions silently.
///
/// Apple's analyticsd collects crash/usage/diagnostic data.  We accept
/// every submission and return success so tools don't retry.
fn burdettie_analytics_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie analytics handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_bool(reply, b"success\0".as_ptr() as u64, true);
    reply
}

/// Diagnostics daemon — accepts diagnostic reports silently.
///
/// Apple's diagnosticd manages diagnostic data collection.  We return
/// success for all queries.
fn burdettie_diagnostics_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie diagnostics handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_bool(reply, b"success\0".as_ptr() as u64, true);
    reply
}

/// Symptom daemon — network symptom tracking.
///
/// Apple's symptomsd tracks network conditions and issues.
/// We return a healthy status.
fn burdettie_symptom_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie symptom handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_string(
        reply,
        b"status\0".as_ptr() as u64,
        b"healthy\0".as_ptr() as u64,
    );
    reply
}

/// Launch Services daemon — UTI/app association queries.
///
/// Apple's lsd provides Uniform Type Identifier lookups and app
/// registration.  We return "not found" cleanly so callers handle
/// the absence gracefully.
fn burdettie_launch_services_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie launch services handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // Return an empty result set — no registered apps
    xpc_objects::dict_set_int64(reply, b"result-count\0".as_ptr() as u64, 0);
    reply
}

/// Quarantine daemon — file quarantine attributes.
///
/// Apple's quarantine service manages Gatekeeper/quarantine extended
/// attributes.  We report all files as non-quarantined.
fn burdettie_quarantine_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie quarantine handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_bool(reply, b"quarantined\0".as_ptr() as u64, false);
    reply
}

/// CFPreferences daemon — preferences/defaults sync.
///
/// Apple's cfprefsd manages plist-based preference synchronization.
/// We return success (preferences are handled in-process by cf-lite).
fn burdettie_cfprefs_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie cfprefs handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_bool(reply, b"success\0".as_ptr() as u64, true);
    reply
}

/// PluginKit daemon — plugin discovery and management.
///
/// Apple's pkd manages app extensions and plugins.  We return an empty
/// list of available plugins.
fn burdettie_pluginkit_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie pluginkit handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_int64(reply, b"result-count\0".as_ptr() as u64, 0);
    reply
}

/// Security daemon — keychain access and credential management.
///
/// On real macOS, `securityd` handles keychain items, certificate trust
/// evaluation, and authorization.  We return "item not found" for queries
/// and "success" for operations that callers can survive without.
fn burdettie_securityd_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie securityd handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    // errSecItemNotFound = -25300
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, -25300);
    xpc_objects::dict_set_int64(reply, b"result\0".as_ptr() as u64, -25300);
    reply
}

/// Trust evaluation daemon — certificate chain validation.
///
/// On real macOS, `trustd` handles SecTrust XPC requests from client
/// processes.  BurdettieOS handles SecTrust directly in-process via
/// `trust-eval` (rustls-webpki + Mozilla roots), so this XPC endpoint
/// just returns success for any fallthrough requests.
fn burdettie_trustd_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie trustd handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // Trust evaluation result = kSecTrustResultProceed (1)
    xpc_objects::dict_set_int64(reply, b"trust-result\0".as_ptr() as u64, 1);
    reply
}

/// Mobile File Integrity — runtime code signing enforcement.
///
/// On real macOS, `amfid` / MobileFileIntegrity enforces code signing
/// on running processes.  We report all code as valid since trust-policy
/// gates handle this at load time.
fn burdettie_amfi_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie AMFI handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // Valid code signing status
    xpc_objects::dict_set_bool(reply, b"valid\0".as_ptr() as u64, true);
    xpc_objects::dict_set_int64(reply, b"status\0".as_ptr() as u64, 0);
    reply
}

/// Launch Services daemon — full app registration and UTI database.
///
/// On real macOS, `launchservicesd` manages the app database, UTI
/// associations, and default handler lookups.  We return empty results
/// for all queries — no apps registered.
fn burdettie_launchservicesd_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie launchservicesd handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // kLSApplicationNotFoundErr = -10814
    xpc_objects::dict_set_int64(reply, b"result\0".as_ptr() as u64, -10814);
    xpc_objects::dict_set_int64(reply, b"result-count\0".as_ptr() as u64, 0);
    reply
}

/// Darwin notification center — cross-process notification posting.
///
/// On real macOS, `notifyd` manages Darwin notifications (not
/// NSNotificationCenter).  Used for state change notifications between
/// processes (e.g., timezone changes, network changes).  We accept all
/// registrations and posts silently.
fn burdettie_notifyd_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie notifyd handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // Token for notification registration
    xpc_objects::dict_set_int64(reply, b"token\0".as_ptr() as u64, 1);
    xpc_objects::dict_set_int64(reply, b"status\0".as_ptr() as u64, 0);
    reply
}

/// Directory Services — user/group membership resolution.
///
/// On real macOS, `DirectoryService` resolves UIDs to usernames, group
/// membership, and similar identity queries.  We return the current
/// process identity.
fn burdettie_directory_service_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie directory service handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    // Return basic identity — current uid/gid
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    xpc_objects::dict_set_int64(reply, b"uid\0".as_ptr() as u64, uid as i64);
    xpc_objects::dict_set_int64(reply, b"gid\0".as_ptr() as u64, gid as i64);
    xpc_objects::dict_set_int64(reply, b"member\0".as_ptr() as u64, 1); // is member
    reply
}

/// Syslog — legacy unified log drain.
///
/// On real macOS, the syslog daemon collects system log messages.
/// Since BurdettieOS uses `burdettie-log-pipeline` for all os_log
/// events, this just returns success.
fn burdettie_syslog_handler(request: u64) -> u64 {
    let _ = request;
    log::debug!("xpc_pipe: burdettie syslog handler invoked");
    let reply = xpc_objects::create_dictionary_empty();
    xpc_objects::dict_set_int64(reply, b"error\0".as_ptr() as u64, 0);
    xpc_objects::dict_set_bool(reply, b"success\0".as_ptr() as u64, true);
    reply
}

/// Install default pipe handlers for well-known service names.
pub fn install_defaults() {
    // launchd domain lookup — used by `launchctl`, `xpcproxy`, etc.
    register_pipe_handler("com.apple.xpc.launchd.domain-port", launchd_stub_handler);
    register_pipe_handler("com.apple.xpc.launchd", launchd_stub_handler);

    // BurdettieOS daemon handlers — replace Apple system services
    register_pipe_handler("com.apple.analyticsd", burdettie_analytics_handler);
    register_pipe_handler("com.apple.diagnosticd", burdettie_diagnostics_handler);
    register_pipe_handler("com.apple.symptomsd", burdettie_symptom_handler);
    register_pipe_handler("com.apple.lsd", burdettie_launch_services_handler);
    register_pipe_handler(
        "com.apple.coreservices.quarantine",
        burdettie_quarantine_handler,
    );
    register_pipe_handler("com.apple.cfprefsd.daemon", burdettie_cfprefs_handler);
    register_pipe_handler("com.apple.cfprefsd.agent", burdettie_cfprefs_handler);
    register_pipe_handler("com.apple.pluginkit.pkd", burdettie_pluginkit_handler);

    // Security & trust — keychain, certificate validation, code signing
    register_pipe_handler("com.apple.securityd", burdettie_securityd_handler);
    register_pipe_handler("com.apple.trustd", burdettie_trustd_handler);
    register_pipe_handler("com.apple.MobileFileIntegrity", burdettie_amfi_handler);

    // System services — launch services, notifications, directory, syslog
    register_pipe_handler(
        "com.apple.coreservices.launchservicesd",
        burdettie_launchservicesd_handler,
    );
    register_pipe_handler("com.apple.notifyd", burdettie_notifyd_handler);
    register_pipe_handler(
        "com.apple.system.DirectoryService.membership_v1",
        burdettie_directory_service_handler,
    );
    register_pipe_handler("com.apple.syslog", burdettie_syslog_handler);

    log::debug!("xpc_pipe: installed default launchd + 15 BurdettieOS daemon handlers");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_handler(request: u64) -> u64 {
        // Echo: create a reply with the request's count
        let reply = xpc_objects::create_dictionary_empty();
        let count = xpc_objects::dict_get_count(request);
        xpc_objects::dict_set_int64(reply, b"count\0".as_ptr() as u64, count as i64);
        reply
    }

    #[test]
    fn register_and_route() {
        register_pipe_handler("com.test.echo", echo_handler);
        assert!(has_pipe_handler("com.test.echo"));

        let pipe = create(b"com.test.echo\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        xpc_objects::dict_set_int64(request, b"a\0".as_ptr() as u64, 1);
        xpc_objects::dict_set_int64(request, b"b\0".as_ptr() as u64, 2);

        let mut reply_handle: u64 = 0;
        let result = routine(pipe, request, &mut reply_handle as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply_handle, 0);
        // The echo handler returns count of request entries
        assert_eq!(
            xpc_objects::dict_get_int64(reply_handle, b"count\0".as_ptr() as u64),
            2
        );
    }

    #[test]
    fn unregistered_pipe_returns_epipe() {
        let pipe = create(b"com.test.nohandler\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let result = routine(pipe, request, 0);
        assert_eq!(result, EPIPE);
    }

    #[test]
    fn null_name_pipe_returns_epipe() {
        let pipe = xpc_objects::create_dictionary_empty();
        let request = xpc_objects::create_dictionary_empty();
        let result = routine(pipe, request, 0);
        assert_eq!(result, EPIPE);
    }

    #[test]
    fn install_defaults_registers_launchd() {
        install_defaults();
        assert!(has_pipe_handler("com.apple.xpc.launchd.domain-port"));
        assert!(has_pipe_handler("com.apple.xpc.launchd"));
    }

    #[test]
    fn launchd_stub_returns_error_reply() {
        install_defaults();
        let pipe = create(b"com.apple.xpc.launchd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0); // Handler exists, returns success
        assert_ne!(reply, 0);
        // Check error field in reply
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            32
        );
    }

    #[test]
    fn unregister_handler() {
        register_pipe_handler("com.test.unreg", echo_handler);
        assert!(has_pipe_handler("com.test.unreg"));
        unregister_pipe_handler("com.test.unreg");
        assert!(!has_pipe_handler("com.test.unreg"));
    }

    #[test]
    fn burdettie_analytics_handler_returns_success() {
        install_defaults();
        assert!(has_pipe_handler("com.apple.analyticsd"));
        let pipe = create(b"com.apple.analyticsd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        assert!(xpc_objects::dict_get_bool(
            reply,
            b"success\0".as_ptr() as u64
        ));
    }

    #[test]
    fn burdettie_diagnostics_handler_returns_success() {
        install_defaults();
        assert!(has_pipe_handler("com.apple.diagnosticd"));
        let pipe = create(b"com.apple.diagnosticd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
    }

    #[test]
    fn burdettie_quarantine_reports_not_quarantined() {
        install_defaults();
        let pipe = create(b"com.apple.coreservices.quarantine\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert!(!xpc_objects::dict_get_bool(
            reply,
            b"quarantined\0".as_ptr() as u64
        ));
    }

    #[test]
    fn burdettie_securityd_returns_item_not_found() {
        install_defaults();
        let pipe = create(b"com.apple.securityd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        // errSecItemNotFound = -25300
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            -25300
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"result\0".as_ptr() as u64),
            -25300
        );
    }

    #[test]
    fn burdettie_trustd_returns_proceed() {
        install_defaults();
        let pipe = create(b"com.apple.trustd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"trust-result\0".as_ptr() as u64),
            1
        );
    }

    #[test]
    fn burdettie_amfi_returns_valid() {
        install_defaults();
        let pipe = create(b"com.apple.MobileFileIntegrity\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        assert!(xpc_objects::dict_get_bool(
            reply,
            b"valid\0".as_ptr() as u64
        ));
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"status\0".as_ptr() as u64),
            0
        );
    }

    #[test]
    fn burdettie_launchservicesd_returns_not_found() {
        install_defaults();
        let pipe = create(
            b"com.apple.coreservices.launchservicesd\0".as_ptr() as u64,
            0,
        );
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        // kLSApplicationNotFoundErr = -10814
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"result\0".as_ptr() as u64),
            -10814
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"result-count\0".as_ptr() as u64),
            0
        );
    }

    #[test]
    fn burdettie_notifyd_returns_token() {
        install_defaults();
        let pipe = create(b"com.apple.notifyd\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"token\0".as_ptr() as u64),
            1
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"status\0".as_ptr() as u64),
            0
        );
    }

    #[test]
    fn burdettie_directory_service_returns_identity() {
        install_defaults();
        let pipe = create(
            b"com.apple.system.DirectoryService.membership_v1\0".as_ptr() as u64,
            0,
        );
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        let uid = unsafe { libc::getuid() } as i64;
        let gid = unsafe { libc::getgid() } as i64;
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"uid\0".as_ptr() as u64),
            uid
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"gid\0".as_ptr() as u64),
            gid
        );
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"member\0".as_ptr() as u64),
            1
        );
    }

    #[test]
    fn burdettie_syslog_returns_success() {
        install_defaults();
        let pipe = create(b"com.apple.syslog\0".as_ptr() as u64, 0);
        let request = xpc_objects::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = routine(pipe, request, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0);
        assert_ne!(reply, 0);
        assert_eq!(
            xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64),
            0
        );
        assert!(xpc_objects::dict_get_bool(
            reply,
            b"success\0".as_ptr() as u64
        ));
    }

    #[test]
    fn all_burdettie_handlers_registered() {
        install_defaults();
        let expected = [
            "com.apple.analyticsd",
            "com.apple.diagnosticd",
            "com.apple.symptomsd",
            "com.apple.lsd",
            "com.apple.coreservices.quarantine",
            "com.apple.cfprefsd.daemon",
            "com.apple.cfprefsd.agent",
            "com.apple.pluginkit.pkd",
            "com.apple.securityd",
            "com.apple.trustd",
            "com.apple.MobileFileIntegrity",
            "com.apple.coreservices.launchservicesd",
            "com.apple.notifyd",
            "com.apple.system.DirectoryService.membership_v1",
            "com.apple.syslog",
        ];
        for name in &expected {
            assert!(has_pipe_handler(name), "missing handler for {name}");
        }
    }
}
