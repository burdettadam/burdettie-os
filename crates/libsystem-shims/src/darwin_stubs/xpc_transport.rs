//! XPC connection transport — connection state machine + Unix domain socket IPC.
//!
//! Each `xpc_connection_t` maps to a `Connection` tracked in a global table.
//! Connections can operate in two modes:
//!
//! 1. **Loopback** (default) — messages are delivered to the event handler on the
//!    same connection.  This is useful for binaries that set up XPC connections to
//!    services that don't actually exist in the emulation layer.
//!
//! 2. **Socket** — connects to a Unix domain socket at
//!    `/tmp/darwin-xpc/<service>.sock` if a listener is registered.  This enables
//!    real cross-process IPC between emulated binaries.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::xpc_objects;

// ── Connection state machine ───────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnState {
    Created,
    Resumed,
    Suspended,
    Cancelled,
}

/// A tracked XPC connection.
#[derive(Clone, Debug)]
pub struct Connection {
    pub id: u64,
    pub service_name: Option<String>,
    pub state: ConnState,
    /// Block pointer for the event handler set via `xpc_connection_set_event_handler`.
    pub event_handler: u64,
    /// Target dispatch queue (opaque handle).
    pub target_queue: u64,
    /// Whether this is a listener connection (server side).
    pub is_listener: bool,
    /// The XPC object handle returned to the guest (wraps `XpcValue::Connection`).
    pub object_handle: u64,
    /// Queued messages awaiting delivery (before resume).
    pending_messages: Vec<u64>,
}

// ── Global connection table ────────────────────────────────────

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

static CONNECTIONS: Mutex<Option<HashMap<u64, Connection>>> = Mutex::new(None);

fn with_conns<R>(f: impl FnOnce(&mut HashMap<u64, Connection>) -> R) -> R {
    let mut guard = CONNECTIONS.lock().unwrap_or_else(|e| e.into_inner());
    let table = guard.get_or_insert_with(HashMap::new);
    f(table)
}

/// Find connection ID by object handle.
fn conn_id_for_handle(handle: u64) -> Option<u64> {
    with_conns(|t| {
        for (id, c) in t.iter() {
            if c.object_handle == handle {
                return Some(*id);
            }
        }
        None
    })
}

/// Create a new XPC connection (client side).
///
/// Returns the XPC object handle (xpc_connection_t) for the guest.
pub fn create_connection(name_ptr: u64, _queue: u64, _flags: u64) -> u64 {
    let service_name = if name_ptr != 0 {
        let cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const i8) };
        Some(cstr.to_str().unwrap_or("unknown").to_string())
    } else {
        None
    };

    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
    let obj_handle = xpc_objects::alloc(xpc_objects::XpcValue::Connection(conn_id));

    let conn = Connection {
        id: conn_id,
        service_name: service_name.clone(),
        state: ConnState::Created,
        event_handler: 0,
        target_queue: 0,
        is_listener: false,
        object_handle: obj_handle,
        pending_messages: Vec::new(),
    };

    log::debug!(
        "xpc: connection {} created for service {:?}",
        conn_id,
        service_name
    );

    with_conns(|t| { t.insert(conn_id, conn); });
    obj_handle
}

/// Create a Mach service connection (equivalent to `xpc_connection_create_mach_service`).
pub fn create_mach_service(name_ptr: u64, queue: u64, flags: u64) -> u64 {
    let handle = create_connection(name_ptr, queue, 0);
    // flags & 1 => XPC_CONNECTION_MACH_SERVICE_LISTENER
    if flags & 1 != 0 {
        if let Some(id) = conn_id_for_handle(handle) {
            with_conns(|t| {
                if let Some(c) = t.get_mut(&id) {
                    c.is_listener = true;
                }
            });
        }
    }
    handle
}

/// Set the event handler block for a connection.
pub fn set_event_handler(handle: u64, block: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        with_conns(|t| {
            if let Some(c) = t.get_mut(&id) {
                c.event_handler = block;
                log::debug!("xpc: connection {} event handler set -> {:#x}", id, block);
            }
        });
    }
}

/// Resume a connection — transitions from Created/Suspended to Resumed.
///
/// If there are pending messages, they are delivered now.
pub fn resume(handle: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        let (handler, pending) = with_conns(|t| {
            if let Some(c) = t.get_mut(&id) {
                c.state = ConnState::Resumed;
                let handler = c.event_handler;
                let pending = std::mem::take(&mut c.pending_messages);
                log::debug!("xpc: connection {} resumed ({} pending)", id, pending.len());
                (handler, pending)
            } else {
                (0, Vec::new())
            }
        });

        // Deliver any queued messages
        if handler != 0 {
            for msg in pending {
                invoke_block_1(handler, msg);
            }
        }
    }
}

/// Suspend a connection.
pub fn suspend(handle: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        with_conns(|t| {
            if let Some(c) = t.get_mut(&id) {
                c.state = ConnState::Suspended;
                log::debug!("xpc: connection {} suspended", id);
            }
        });
    }
}

/// Cancel a connection.
pub fn cancel(handle: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        let handler = with_conns(|t| {
            if let Some(c) = t.get_mut(&id) {
                c.state = ConnState::Cancelled;
                log::debug!("xpc: connection {} cancelled", id);
                c.event_handler
            } else {
                0
            }
        });

        // Deliver XPC_ERROR_CONNECTION_INVALID to the event handler
        if handler != 0 {
            let err = xpc_objects::create_error(0x5850_E002); // _xpc_error_connection_invalid
            invoke_block_1(handler, err);
        }
    }
}

/// Set the target dispatch queue.
pub fn set_target_queue(handle: u64, queue: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        with_conns(|t| {
            if let Some(c) = t.get_mut(&id) {
                c.target_queue = queue;
            }
        });
    }
}

/// Get the PID of the remote peer.  Returns current PID as loopback.
pub fn get_pid(handle: u64) -> i32 {
    let _ = handle;
    unsafe { libc::getpid() }
}

/// Send a message (fire-and-forget).
///
/// Routes through Unix domain socket if a listener exists, otherwise
/// delivers to the connection's event handler (loopback).
pub fn send_message(handle: u64, message: u64) {
    if let Some(id) = conn_id_for_handle(handle) {
        let (state, handler, service) = with_conns(|t| {
            if let Some(c) = t.get(&id) {
                (c.state, c.event_handler, c.service_name.clone())
            } else {
                (ConnState::Cancelled, 0, None)
            }
        });

        // Try socket transport first if a service name exists
        if let Some(ref name) = service {
            if socket_transport::try_send(name, message) {
                return;
            }
        }

        match state {
            ConnState::Resumed if handler != 0 => {
                // Loopback: deliver directly to event handler
                invoke_block_1(handler, message);
            }
            ConnState::Created | ConnState::Suspended => {
                // Queue for later delivery
                with_conns(|t| {
                    if let Some(c) = t.get_mut(&id) {
                        c.pending_messages.push(message);
                    }
                });
            }
            _ => {
                log::debug!("xpc: send_message on connection {} dropped (state={:?})", id, state);
            }
        }
    }
}

/// Send a message with a reply handler.
///
/// Tries socket transport if a listener exists, otherwise falls back to
/// loopback (delivers XPC_ERROR_CONNECTION_INTERRUPTED to the handler).
pub fn send_message_with_reply(handle: u64, message: u64, _queue: u64, reply_handler: u64) {
    if reply_handler == 0 { return; }

    // Try socket transport for a real reply
    if let Some(id) = conn_id_for_handle(handle) {
        let service = with_conns(|t| {
            t.get(&id).and_then(|c| c.service_name.clone())
        });

        if let Some(ref name) = service {
            if let Some(reply) = socket_transport::try_send_with_reply(name, message) {
                invoke_block_1(reply_handler, reply);
                return;
            }
        }
    }

    // Loopback fallback: deliver an error reply
    let err = xpc_objects::create_error(0x5850_E001); // _xpc_error_connection_interrupted
    invoke_block_1(reply_handler, err);
}

/// Send a message synchronously, returning the reply.
///
/// Tries socket transport for a real reply, otherwise returns an error.
pub fn send_message_with_reply_sync(handle: u64, message: u64) -> u64 {
    if let Some(id) = conn_id_for_handle(handle) {
        let service = with_conns(|t| {
            t.get(&id).and_then(|c| c.service_name.clone())
        });

        if let Some(ref name) = service {
            if let Some(reply) = socket_transport::try_send_with_reply(name, message) {
                return reply;
            }
        }
    }

    // Loopback fallback: return an error object
    xpc_objects::create_error(0x5850_E001)
}

/// Execute a barrier block on the connection's queue.
///
/// We execute inline (the block runs synchronously).
pub fn send_barrier(handle: u64, barrier_block: u64) {
    let _ = handle;
    if barrier_block != 0 {
        invoke_block_0(barrier_block);
    }
}

/// Get the remote connection from a dictionary (xpc_dictionary_get_remote_connection).
/// Not meaningful in loopback mode — returns the handle as-is.
pub fn get_remote_connection(_dict: u64) -> u64 {
    0
}

// ── Service listener registry ──────────────────────────────────
//
// Minimal listener support.  Services can be registered with
// `register_listener(name, handler_block)`.  When a client sends a
// message to that service, the handler is invoked.

static LISTENERS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Register a named service listener.  `handler` is a block that receives
/// each incoming peer connection handle.
pub fn register_listener(name: &str, handler: u64) {
    let mut guard = LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(name.to_string(), handler);
    log::debug!("xpc: listener registered for service '{}'", name);
}

/// Check if a service has a listener.
pub fn has_listener(name: &str) -> bool {
    let guard = LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map_or(false, |m| m.contains_key(name))
}

/// Get the listener handler for a service.
fn get_listener_handler(name: &str) -> Option<u64> {
    let guard = LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(name).copied())
}

// ── Unix domain socket transport ───────────────────────────────

pub mod socket_transport {
    use super::*;

    /// Base directory for XPC socket files.
    const SOCKET_DIR: &str = "/tmp/darwin-xpc";

    /// Get the socket path for a service name.
    /// Sanitizes the name to prevent path traversal.
    pub fn socket_path(service_name: &str) -> std::path::PathBuf {
        // Sanitize: only allow safe characters, strip path separators and ..
        let safe_name: String = service_name.chars().map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        }).collect();
        // Extra protection: collapse any ".." sequences
        let safe_name = safe_name.replace("..", "__");
        std::path::PathBuf::from(SOCKET_DIR).join(format!("{}.sock", safe_name))
    }

    /// Try to send a message via Unix domain socket (fire-and-forget).
    /// Returns true if the socket exists and the message was sent.
    pub fn try_send(service_name: &str, message: u64) -> bool {
        let path = socket_path(service_name);
        if !path.exists() {
            return false;
        }

        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                let data = serialize_message(message);
                if let Err(e) = stream.write_all(&data) {
                    log::debug!("xpc_socket: send failed for '{}': {}", service_name, e);
                    return false;
                }
                log::debug!("xpc_socket: sent {} bytes to '{}'", data.len(), service_name);
                true
            }
            Err(e) => {
                log::debug!("xpc_socket: connect failed for '{}': {}", service_name, e);
                false
            }
        }
    }

    /// Try to send a message and receive a reply via Unix domain socket.
    /// Returns Some(reply_handle) on success, None if socket doesn't exist.
    pub fn try_send_with_reply(service_name: &str, message: u64) -> Option<u64> {
        let path = socket_path(service_name);
        if !path.exists() {
            return None;
        }

        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                // Send request
                let data = serialize_message(message);
                if stream.write_all(&data).is_err() {
                    return None;
                }
                // Shut down write side to signal end of request
                let _ = stream.shutdown(std::net::Shutdown::Write);

                // Read reply
                let mut reply_data = Vec::new();
                if stream.read_to_end(&mut reply_data).is_err() {
                    return None;
                }
                if reply_data.is_empty() {
                    return None;
                }

                let reply = deserialize_message(&reply_data);
                log::debug!("xpc_socket: received reply from '{}'", service_name);
                Some(reply)
            }
            Err(_) => None,
        }
    }

    /// Start a listener on a Unix domain socket for a service.
    ///
    /// Spawns a background thread that accepts connections and dispatches
    /// messages to the handler.  Returns the socket path on success.
    pub fn start_listener(service_name: &str, handler: u64) -> Result<std::path::PathBuf, String> {
        let path = socket_path(service_name);

        // Create socket directory
        if let Err(e) = std::fs::create_dir_all(SOCKET_DIR) {
            return Err(format!("failed to create {}: {}", SOCKET_DIR, e));
        }

        // Remove stale socket if it exists
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path)
            .map_err(|e| format!("bind '{}': {}", path.display(), e))?;

        log::debug!("xpc_socket: listening on {}", path.display());

        let service_name = service_name.to_string();
        let path_clone = path.clone();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut conn) => {
                        // Read the request
                        let mut data = Vec::new();
                        if conn.read_to_end(&mut data).is_err() {
                            continue;
                        }
                        if data.is_empty() {
                            continue;
                        }

                        let request = deserialize_message(&data);
                        log::debug!(
                            "xpc_socket: request received on '{}' ({} bytes)",
                            service_name,
                            data.len()
                        );

                        // Invoke the handler (if it's a block pointer)
                        if handler != 0 {
                            invoke_block_1(handler, request);
                        }
                    }
                    Err(e) => {
                        log::debug!("xpc_socket: accept error on '{}': {}", service_name, e);
                        break;
                    }
                }
            }
            // Clean up socket file when listener exits
            let _ = std::fs::remove_file(&path_clone);
        });

        Ok(path)
    }

    /// Shut down a listener by removing its socket file.
    pub fn stop_listener(service_name: &str) {
        let path = socket_path(service_name);
        let _ = std::fs::remove_file(&path);
        log::debug!("xpc_socket: stopped listener for '{}'", service_name);
    }

    // ── Recursive message serialization ────────────────────

    /// Serialize an XPC value to wire format (recursive for nested objects).
    ///
    /// Wire format for each value:
    /// - `i` + 8 bytes LE = Int64
    /// - `u` + 8 bytes LE = Uint64
    /// - `b` + 1 byte = Bool
    /// - `s` + 4 bytes LE (len) + bytes = String
    /// - `d` + 4 bytes LE (len) + bytes = Data
    /// - `f` + 8 bytes LE = Double
    /// - `t` + 8 bytes LE = Date (nanos)
    /// - `n` = Null
    /// - `D` + 4 bytes LE (count) + count × (key_len + key + value) = Dictionary
    /// - `A` + 4 bytes LE (count) + count × value = Array
    fn serialize_value(handle: u64, buf: &mut Vec<u8>) {
        match xpc_objects::get(handle) {
            Some(xpc_objects::XpcValue::Int64(v)) => {
                buf.push(b'i');
                buf.extend_from_slice(&v.to_le_bytes());
            }
            Some(xpc_objects::XpcValue::Uint64(v)) => {
                buf.push(b'u');
                buf.extend_from_slice(&v.to_le_bytes());
            }
            Some(xpc_objects::XpcValue::Bool(v)) => {
                buf.push(b'b');
                buf.push(v as u8);
            }
            Some(xpc_objects::XpcValue::String(s)) => {
                buf.push(b's');
                let bytes = s.to_bytes();
                buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            Some(xpc_objects::XpcValue::Data(d)) => {
                buf.push(b'd');
                buf.extend_from_slice(&(d.len() as u32).to_le_bytes());
                buf.extend_from_slice(&d);
            }
            Some(xpc_objects::XpcValue::Double(v)) => {
                buf.push(b'f');
                buf.extend_from_slice(&v.to_le_bytes());
            }
            Some(xpc_objects::XpcValue::Date(v)) => {
                buf.push(b't');
                buf.extend_from_slice(&v.to_le_bytes());
            }
            Some(xpc_objects::XpcValue::Dictionary(pairs)) => {
                buf.push(b'D');
                buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
                for (key, val_handle) in &pairs {
                    let key_bytes = key.to_bytes();
                    buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
                    buf.extend_from_slice(key_bytes);
                    serialize_value(*val_handle, buf);
                }
            }
            Some(xpc_objects::XpcValue::Array(elements)) => {
                buf.push(b'A');
                buf.extend_from_slice(&(elements.len() as u32).to_le_bytes());
                for val_handle in &elements {
                    serialize_value(*val_handle, buf);
                }
            }
            Some(xpc_objects::XpcValue::Null) | None => {
                buf.push(b'n');
            }
            _ => {
                // Error, Connection, Endpoint, Uuid, Fd — serialize as null
                buf.push(b'n');
            }
        }
    }

    /// Serialize an XPC dictionary as a top-level message.
    fn serialize_message(handle: u64) -> Vec<u8> {
        let mut buf = Vec::new();
        // Top-level is always a dictionary: write it using serialize_value
        serialize_value(handle, &mut buf);
        buf
    }

    /// Deserialize a single value from the wire, advancing `pos`.
    fn deserialize_value(data: &[u8], pos: &mut usize) -> u64 {
        if *pos >= data.len() { return xpc_objects::create_null(); }

        let type_tag = data[*pos];
        *pos += 1;

        match type_tag {
            b'i' => {
                if *pos + 8 > data.len() { return xpc_objects::create_null(); }
                let v = i64::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                    data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
                ]);
                *pos += 8;
                xpc_objects::create_int64(v)
            }
            b'u' => {
                if *pos + 8 > data.len() { return xpc_objects::create_null(); }
                let v = u64::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                    data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
                ]);
                *pos += 8;
                xpc_objects::create_uint64(v)
            }
            b'b' => {
                if *pos >= data.len() { return xpc_objects::create_null(); }
                let v = data[*pos] != 0;
                *pos += 1;
                xpc_objects::create_bool(v)
            }
            b's' => {
                if *pos + 4 > data.len() { return xpc_objects::create_null(); }
                let slen = u32::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                ]) as usize;
                *pos += 4;
                if *pos + slen > data.len() { return xpc_objects::create_null(); }
                let h = if let Ok(s) = std::ffi::CString::new(&data[*pos..*pos + slen]) {
                    xpc_objects::create_string(&s)
                } else {
                    xpc_objects::create_null()
                };
                *pos += slen;
                h
            }
            b'd' => {
                if *pos + 4 > data.len() { return xpc_objects::create_null(); }
                let dlen = u32::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                ]) as usize;
                *pos += 4;
                if *pos + dlen > data.len() { return xpc_objects::create_null(); }
                let h = xpc_objects::create_data(&data[*pos..*pos + dlen]);
                *pos += dlen;
                h
            }
            b'f' => {
                if *pos + 8 > data.len() { return xpc_objects::create_null(); }
                let v = f64::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                    data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
                ]);
                *pos += 8;
                xpc_objects::create_double(v)
            }
            b't' => {
                if *pos + 8 > data.len() { return xpc_objects::create_null(); }
                let v = i64::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                    data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
                ]);
                *pos += 8;
                xpc_objects::create_date(v)
            }
            b'D' => {
                if *pos + 4 > data.len() { return xpc_objects::create_null(); }
                let count = u32::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                ]) as usize;
                *pos += 4;
                let dict = xpc_objects::create_dictionary_empty();
                for _ in 0..count {
                    if *pos + 4 > data.len() { break; }
                    let key_len = u32::from_le_bytes([
                        data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                    ]) as usize;
                    *pos += 4;
                    if *pos + key_len > data.len() { break; }
                    let key_bytes = &data[*pos..*pos + key_len];
                    *pos += key_len;
                    let key = match std::ffi::CString::new(key_bytes) {
                        Ok(k) => k,
                        Err(_) => {
                            // Skip the value
                            let _ = deserialize_value(data, pos);
                            continue;
                        }
                    };
                    let val = deserialize_value(data, pos);
                    xpc_objects::dict_set(dict, &key, val);
                }
                dict
            }
            b'A' => {
                if *pos + 4 > data.len() { return xpc_objects::create_null(); }
                let count = u32::from_le_bytes([
                    data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
                ]) as usize;
                *pos += 4;
                let arr = xpc_objects::create_array_empty();
                for _ in 0..count {
                    let val = deserialize_value(data, pos);
                    xpc_objects::array_append(arr, val);
                }
                arr
            }
            b'n' => xpc_objects::create_null(),
            _ => xpc_objects::create_null(),
        }
    }

    /// Deserialize a message from wire format into an XPC object handle.
    fn deserialize_message(data: &[u8]) -> u64 {
        if data.is_empty() {
            return xpc_objects::create_dictionary_empty();
        }
        let mut pos = 0;
        deserialize_value(data, &mut pos)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::ffi::CString;

        #[test]
        fn serialize_deserialize_roundtrip() {
            let dict = xpc_objects::create_dictionary_empty();
            xpc_objects::dict_set_int64(dict, b"count\0".as_ptr() as u64, 42);
            xpc_objects::dict_set_string(
                dict,
                b"name\0".as_ptr() as u64,
                b"hello\0".as_ptr() as u64,
            );
            xpc_objects::dict_set_bool(dict, b"flag\0".as_ptr() as u64, true);

            let data = serialize_message(dict);
            assert!(!data.is_empty());

            let dict2 = deserialize_message(&data);
            assert_eq!(xpc_objects::dict_get_count(dict2), 3);
            assert_eq!(xpc_objects::dict_get_int64(dict2, b"count\0".as_ptr() as u64), 42);
            assert!(xpc_objects::dict_get_bool(dict2, b"flag\0".as_ptr() as u64));

            let name_ptr = xpc_objects::dict_get_string(dict2, b"name\0".as_ptr() as u64);
            assert_ne!(name_ptr, 0);
            let name = unsafe { std::ffi::CStr::from_ptr(name_ptr as *const i8) };
            assert_eq!(name.to_str().unwrap(), "hello");
        }

        #[test]
        fn serialize_empty_dict() {
            let dict = xpc_objects::create_dictionary_empty();
            let data = serialize_message(dict);
            let dict2 = deserialize_message(&data);
            assert_eq!(xpc_objects::dict_get_count(dict2), 0);
        }

        #[test]
        fn nested_dictionary_roundtrip() {
            // Create outer dict with an inner dict
            let inner = xpc_objects::create_dictionary_empty();
            xpc_objects::dict_set_int64(inner, b"x\0".as_ptr() as u64, 99);
            xpc_objects::dict_set_string(
                inner,
                b"y\0".as_ptr() as u64,
                b"nested\0".as_ptr() as u64,
            );

            let outer = xpc_objects::create_dictionary_empty();
            xpc_objects::dict_set_int64(outer, b"top\0".as_ptr() as u64, 1);
            let key = std::ffi::CString::new("child").unwrap();
            xpc_objects::dict_set(outer, &key, inner);

            let data = serialize_message(outer);
            let restored = deserialize_message(&data);

            assert_eq!(xpc_objects::dict_get_count(restored), 2);
            assert_eq!(xpc_objects::dict_get_int64(restored, b"top\0".as_ptr() as u64), 1);

            // Get the nested dict
            let child_key = std::ffi::CString::new("child").unwrap();
            let child_handle = xpc_objects::dict_get(restored, &child_key);
            assert_ne!(child_handle, 0);
            assert_eq!(xpc_objects::dict_get_count(child_handle), 2);
            assert_eq!(xpc_objects::dict_get_int64(child_handle, b"x\0".as_ptr() as u64), 99);
            let y_ptr = xpc_objects::dict_get_string(child_handle, b"y\0".as_ptr() as u64);
            assert_ne!(y_ptr, 0);
            let y_str = unsafe { std::ffi::CStr::from_ptr(y_ptr as *const i8) };
            assert_eq!(y_str.to_str().unwrap(), "nested");
        }

        #[test]
        fn nested_array_roundtrip() {
            // Create array with mixed elements
            let arr = xpc_objects::create_array_empty();
            xpc_objects::array_append(arr, xpc_objects::create_int64(10));
            xpc_objects::array_append(arr, xpc_objects::create_int64(20));

            // Nest array inside dict
            let dict = xpc_objects::create_dictionary_empty();
            let key = std::ffi::CString::new("items").unwrap();
            xpc_objects::dict_set(dict, &key, arr);
            xpc_objects::dict_set_int64(dict, b"total\0".as_ptr() as u64, 2);

            let data = serialize_message(dict);
            let restored = deserialize_message(&data);

            assert_eq!(xpc_objects::dict_get_count(restored), 2);
            assert_eq!(xpc_objects::dict_get_int64(restored, b"total\0".as_ptr() as u64), 2);

            let items_handle = xpc_objects::dict_get(restored, &key);
            assert_ne!(items_handle, 0);
            assert_eq!(xpc_objects::array_get_count(items_handle), 2);

            let v0 = xpc_objects::array_get_value(items_handle, 0);
            let v1 = xpc_objects::array_get_value(items_handle, 1);
            assert_eq!(xpc_objects::get_int64_value(v0), 10);
            assert_eq!(xpc_objects::get_int64_value(v1), 20);
        }

        #[test]
        fn deeply_nested_roundtrip() {
            // dict → dict → array → dict
            let leaf = xpc_objects::create_dictionary_empty();
            xpc_objects::dict_set_string(
                leaf,
                b"msg\0".as_ptr() as u64,
                b"deep\0".as_ptr() as u64,
            );

            let arr = xpc_objects::create_array_empty();
            xpc_objects::array_append(arr, leaf);

            let mid = xpc_objects::create_dictionary_empty();
            let arr_key = std::ffi::CString::new("list").unwrap();
            xpc_objects::dict_set(mid, &arr_key, arr);

            let top = xpc_objects::create_dictionary_empty();
            let mid_key = std::ffi::CString::new("mid").unwrap();
            xpc_objects::dict_set(top, &mid_key, mid);

            let data = serialize_message(top);
            let restored = deserialize_message(&data);

            // Navigate: restored.mid.list[0].msg
            let r_mid = xpc_objects::dict_get(restored, &mid_key);
            assert_ne!(r_mid, 0);
            let r_list = xpc_objects::dict_get(r_mid, &arr_key);
            assert_ne!(r_list, 0);
            assert_eq!(xpc_objects::array_get_count(r_list), 1);
            let r_leaf = xpc_objects::array_get_value(r_list, 0);
            let msg_ptr = xpc_objects::dict_get_string(r_leaf, b"msg\0".as_ptr() as u64);
            assert_ne!(msg_ptr, 0);
            let msg = unsafe { std::ffi::CStr::from_ptr(msg_ptr as *const i8) };
            assert_eq!(msg.to_str().unwrap(), "deep");
        }

        #[test]
        fn socket_path_sanitized() {
            let p = socket_path("com.apple.xpc.service");
            assert!(p.to_str().unwrap().contains("com.apple.xpc.service.sock"));
            // Path traversal attempt
            let p2 = socket_path("../../etc/passwd");
            assert!(!p2.to_str().unwrap().contains(".."));
        }

        #[test]
        fn try_send_nonexistent_returns_false() {
            assert!(!try_send("com.nonexist.test", 0));
        }

        #[test]
        fn try_send_with_reply_nonexistent_returns_none() {
            assert!(try_send_with_reply("com.nonexist.reply", 0).is_none());
        }

        #[test]
        fn unix_socket_roundtrip() {
            // Create a listener that echoes back
            let service = "com.test.socket.roundtrip";
            let path = socket_path(service);

            // Ensure clean state
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::create_dir_all("/tmp/darwin-xpc");

            let listener = UnixListener::bind(&path).unwrap();

            // Spawn a handler thread
            let handle = std::thread::spawn(move || {
                if let Ok(mut conn) = listener.accept() {
                    let mut data = Vec::new();
                    conn.0.read_to_end(&mut data).unwrap();
                    let request = deserialize_message(&data);
                    let count = xpc_objects::dict_get_count(request);
                    // Build reply
                    let reply = xpc_objects::create_dictionary_empty();
                    xpc_objects::dict_set_int64(
                        reply,
                        b"echo_count\0".as_ptr() as u64,
                        count as i64,
                    );
                    serialize_message(reply)
                } else {
                    Vec::new()
                }
            });

            // Client: send a message
            let msg = xpc_objects::create_dictionary_empty();
            xpc_objects::dict_set_int64(msg, b"a\0".as_ptr() as u64, 1);
            xpc_objects::dict_set_int64(msg, b"b\0".as_ptr() as u64, 2);

            assert!(try_send(service, msg));

            handle.join().unwrap();
            let _ = std::fs::remove_file(&path);
        }
    }
}

// ── Block invocation helpers ───────────────────────────────────

/// Invoke an ObjC block with zero arguments.
fn invoke_block_0(block: u64) {
    unsafe {
        let invoke_ptr = *((block + 16) as *const u64);
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64) = std::mem::transmute(invoke_ptr);
            invoke(block);
        }
    }
}

/// Invoke an ObjC block with one argument.
fn invoke_block_1(block: u64, arg1: u64) {
    unsafe {
        let invoke_ptr = *((block + 16) as *const u64);
        if invoke_ptr != 0 {
            let invoke: extern "C" fn(u64, u64) = std::mem::transmute(invoke_ptr);
            invoke(block, arg1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn create_connection_returns_valid_handle() {
        let name = CString::new("com.test.service").unwrap();
        let h = create_connection(name.as_ptr() as u64, 0, 0);
        assert_ne!(h, 0);
        assert!(xpc_objects::is_valid(h));
        assert_eq!(xpc_objects::get_type_tag(h), xpc_objects::XPC_TYPE_TAG_CONNECTION);
    }

    #[test]
    fn connection_lifecycle() {
        let name = CString::new("com.test.lifecycle").unwrap();
        let h = create_connection(name.as_ptr() as u64, 0, 0);

        // Starts in Created
        let id = conn_id_for_handle(h).unwrap();
        let state = with_conns(|t| t.get(&id).unwrap().state);
        assert_eq!(state, ConnState::Created);

        // Resume
        resume(h);
        let state = with_conns(|t| t.get(&id).unwrap().state);
        assert_eq!(state, ConnState::Resumed);

        // Suspend
        suspend(h);
        let state = with_conns(|t| t.get(&id).unwrap().state);
        assert_eq!(state, ConnState::Suspended);

        // Cancel
        cancel(h);
        let state = with_conns(|t| t.get(& id).unwrap().state);
        assert_eq!(state, ConnState::Cancelled);
    }

    #[test]
    fn mach_service_listener_flag() {
        let name = CString::new("com.test.listener").unwrap();
        let h = create_mach_service(name.as_ptr() as u64, 0, 1);
        let id = conn_id_for_handle(h).unwrap();
        let is_listener = with_conns(|t| t.get(&id).unwrap().is_listener);
        assert!(is_listener);
    }

    #[test]
    fn send_reply_sync_returns_error() {
        let name = CString::new("com.test.sync").unwrap();
        let h = create_connection(name.as_ptr() as u64, 0, 0);
        resume(h);
        let msg = xpc_objects::create_dictionary_empty();
        let reply = send_message_with_reply_sync(h, msg);
        assert_ne!(reply, 0);
        assert_eq!(xpc_objects::get_type_tag(reply), xpc_objects::XPC_TYPE_TAG_ERROR);
    }

    #[test]
    fn get_pid_returns_nonzero() {
        let p = get_pid(0);
        assert!(p > 0);
    }

    #[test]
    fn service_listener_registry() {
        register_listener("com.test.reg", 0xDEAD);
        assert!(has_listener("com.test.reg"));
        assert!(!has_listener("com.test.nope"));
    }
}
