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
    with_pipes(|t| { t.insert(name.to_string(), handler); });
}

/// Unregister a pipe handler.
pub fn unregister_pipe_handler(name: &str) {
    with_pipes(|t| { t.remove(name); });
}

/// Check if a pipe handler is registered.
pub fn has_pipe_handler(name: &str) -> bool {
    with_pipes(|t| t.contains_key(name))
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

/// Install default pipe handlers for well-known service names.
pub fn install_defaults() {
    // launchd domain lookup — used by `launchctl`, `xpcproxy`, etc.
    register_pipe_handler("com.apple.xpc.launchd.domain-port", launchd_stub_handler);
    register_pipe_handler("com.apple.xpc.launchd", launchd_stub_handler);
    log::debug!("xpc_pipe: installed default launchd stub handlers");
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
        assert_eq!(xpc_objects::dict_get_int64(reply_handle, b"count\0".as_ptr() as u64), 2);
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
        assert_eq!(xpc_objects::dict_get_int64(reply, b"error\0".as_ptr() as u64), 32);
    }

    #[test]
    fn unregister_handler() {
        register_pipe_handler("com.test.unreg", echo_handler);
        assert!(has_pipe_handler("com.test.unreg"));
        unregister_pipe_handler("com.test.unreg");
        assert!(!has_pipe_handler("com.test.unreg"));
    }
}
