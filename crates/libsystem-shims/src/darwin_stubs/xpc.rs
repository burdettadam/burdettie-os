//! Apple libxpc — real object store + connection transport.
//!
//! XPC (inter-process communication) is heavily used by launchd, Xcode tools,
//! and any macOS service communication.  This module provides:
//!
//! - **Real XPC objects**: dictionaries, arrays, and primitives that actually
//!   store data (backed by `xpc_objects`).
//! - **Connection transport**: state-machine connections with event handlers,
//!   loopback message delivery, and service listener registry (backed by
//!   `xpc_transport`).
//! - **Stub-only**: Activity APIs and pipe (no launchd).

use crate::table::ShimTable;
use super::xpc_activity as activity;
use super::xpc_objects as obj;
use super::xpc_pipe as pipe;
use super::xpc_transport as transport;

pub struct XpcStubs;

impl XpcStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── XPC Object lifecycle ────────────────────────────────
        register_shims!(table,
            ("_xpc_retain", shim_xpc_retain as u64, "xpc_retain → identity (refcount not tracked)"),
            ("_xpc_release", shim_xpc_release as u64, "xpc_release → removes from object store"),
            ("_xpc_copy", shim_xpc_copy as u64, "xpc_copy → identity (shallow)"),
            ("_xpc_equal", shim_xpc_equal as u64, "xpc_equal → value comparison"),
            ("_xpc_hash", shim_xpc_hash as u64, "xpc_hash → value hash"),
            ("_xpc_get_type", shim_xpc_get_type as u64, "xpc_get_type → type tag pointer"),
            ("_xpc_copy_description", shim_xpc_copy_description as u64, "xpc_copy_description → malloc'd string")
        );

        // ── XPC Connection ──────────────────────────────────────
        register_shims!(table,
            ("_xpc_connection_create", shim_xpc_connection_create as u64, "xpc_connection_create → tracked connection"),
            ("_xpc_connection_create_mach_service", shim_xpc_connection_create_mach_service as u64, "xpc_connection_create_mach_service → tracked connection"),
            ("_xpc_connection_set_event_handler", shim_xpc_connection_set_event_handler as u64, "xpc_connection_set_event_handler → stores block"),
            ("_xpc_connection_resume", shim_xpc_connection_resume as u64, "xpc_connection_resume → delivers pending msgs"),
            ("_xpc_connection_suspend", shim_xpc_connection_suspend as u64, "xpc_connection_suspend → pauses delivery"),
            ("_xpc_connection_cancel", shim_xpc_connection_cancel as u64, "xpc_connection_cancel → sends INVALID error"),
            ("_xpc_connection_send_message", shim_xpc_connection_send_message as u64, "xpc_connection_send_message → loopback delivery"),
            ("_xpc_connection_send_message_with_reply", shim_xpc_connection_send_message_with_reply as u64, "xpc_connection_send_message_with_reply → calls handler"),
            ("_xpc_connection_send_message_with_reply_sync", shim_xpc_connection_send_message_with_reply_sync as u64, "xpc_connection_send_message_with_reply_sync → error reply"),
            ("_xpc_connection_send_barrier", shim_xpc_connection_send_barrier as u64, "xpc_connection_send_barrier → inline block"),
            ("_xpc_connection_set_target_queue", shim_xpc_connection_set_target_queue as u64, "xpc_connection_set_target_queue → stores queue"),
            ("_xpc_connection_get_pid", shim_xpc_connection_get_pid as u64, "xpc_connection_get_pid → getpid()")
        );

        // ── XPC Dictionary ──────────────────────────────────────
        register_shims!(table,
            ("_xpc_dictionary_create", shim_xpc_dictionary_create as u64, "xpc_dictionary_create → real dict object"),
            ("_xpc_dictionary_create_empty", shim_xpc_dictionary_create_empty as u64, "xpc_dictionary_create_empty → real dict object"),
            ("_xpc_dictionary_set_value", shim_xpc_dictionary_set_value as u64, "xpc_dictionary_set_value → stores pair"),
            ("_xpc_dictionary_get_value", shim_xpc_dictionary_get_value as u64, "xpc_dictionary_get_value → retrieves pair"),
            ("_xpc_dictionary_set_string", shim_xpc_dictionary_set_string as u64, "xpc_dictionary_set_string → stores string"),
            ("_xpc_dictionary_get_string", shim_xpc_dictionary_get_string as u64, "xpc_dictionary_get_string → returns C string ptr"),
            ("_xpc_dictionary_set_int64", shim_xpc_dictionary_set_int64 as u64, "xpc_dictionary_set_int64 → stores int64"),
            ("_xpc_dictionary_get_int64", shim_xpc_dictionary_get_int64 as u64, "xpc_dictionary_get_int64 → retrieves int64"),
            ("_xpc_dictionary_set_uint64", shim_xpc_dictionary_set_uint64 as u64, "xpc_dictionary_set_uint64 → stores uint64"),
            ("_xpc_dictionary_get_uint64", shim_xpc_dictionary_get_uint64 as u64, "xpc_dictionary_get_uint64 → retrieves uint64"),
            ("_xpc_dictionary_set_bool", shim_xpc_dictionary_set_bool as u64, "xpc_dictionary_set_bool → stores bool"),
            ("_xpc_dictionary_get_bool", shim_xpc_dictionary_get_bool as u64, "xpc_dictionary_get_bool → retrieves bool"),
            ("_xpc_dictionary_set_data", shim_xpc_dictionary_set_data as u64, "xpc_dictionary_set_data → stores data"),
            ("_xpc_dictionary_get_data", shim_xpc_dictionary_get_data as u64, "xpc_dictionary_get_data → retrieves data ptr"),
            ("_xpc_dictionary_set_fd", shim_xpc_dictionary_set_fd as u64, "xpc_dictionary_set_fd → stores fd"),
            ("_xpc_dictionary_get_count", shim_xpc_dictionary_get_count as u64, "xpc_dictionary_get_count → real count"),
            ("_xpc_dictionary_apply", shim_xpc_dictionary_apply as u64, "xpc_dictionary_apply → iterates pairs"),
            ("_xpc_dictionary_get_remote_connection", shim_xpc_dictionary_get_remote_connection as u64, "xpc_dictionary_get_remote_connection → NULL (loopback)")
        );

        // ── XPC Array ───────────────────────────────────────────
        register_shims!(table,
            ("_xpc_array_create", shim_xpc_array_create as u64, "xpc_array_create → real array object"),
            ("_xpc_array_get_count", shim_xpc_array_get_count as u64, "xpc_array_get_count → real count"),
            ("_xpc_array_get_value", shim_xpc_array_get_value as u64, "xpc_array_get_value → retrieves element"),
            ("_xpc_array_append_value", shim_xpc_array_append_value as u64, "xpc_array_append_value → appends element"),
            ("_xpc_array_apply", shim_xpc_array_apply as u64, "xpc_array_apply → iterates elements")
        );

        // ── XPC Primitive constructors ──────────────────────────
        register_shims!(table,
            ("_xpc_string_create", shim_xpc_string_create as u64, "xpc_string_create → real string object"),
            ("_xpc_string_get_string_ptr", shim_xpc_string_get_string_ptr as u64, "xpc_string_get_string_ptr → C string pointer"),
            ("_xpc_string_get_length", shim_xpc_string_get_length as u64, "xpc_string_get_length → byte count"),
            ("_xpc_int64_create", shim_xpc_int64_create as u64, "xpc_int64_create → real int64 object"),
            ("_xpc_int64_get_value", shim_xpc_int64_get_value as u64, "xpc_int64_get_value → stored value"),
            ("_xpc_uint64_create", shim_xpc_uint64_create as u64, "xpc_uint64_create → real uint64 object"),
            ("_xpc_uint64_get_value", shim_xpc_uint64_get_value as u64, "xpc_uint64_get_value → stored value"),
            ("_xpc_bool_create", shim_xpc_bool_create as u64, "xpc_bool_create → real bool object"),
            ("_xpc_bool_get_value", shim_xpc_bool_get_value as u64, "xpc_bool_get_value → stored value"),
            ("_xpc_data_create", shim_xpc_data_create as u64, "xpc_data_create → real data object"),
            ("_xpc_data_get_bytes_ptr", shim_xpc_data_get_bytes_ptr as u64, "xpc_data_get_bytes_ptr → byte pointer"),
            ("_xpc_data_get_length", shim_xpc_data_get_length as u64, "xpc_data_get_length → byte count"),
            ("_xpc_null_create", shim_xpc_null_create as u64, "xpc_null_create → real null object"),
            ("_xpc_date_create", shim_xpc_date_create as u64, "xpc_date_create → real date object"),
            ("_xpc_date_get_value", shim_xpc_date_get_value as u64, "xpc_date_get_value → stored nanos"),
            ("_xpc_uuid_create", shim_xpc_uuid_create as u64, "xpc_uuid_create → real uuid object"),
            ("_xpc_fd_create", shim_xpc_fd_create as u64, "xpc_fd_create → real fd object"),
            ("_xpc_double_create", shim_xpc_double_create as u64, "xpc_double_create → real double object"),
            ("_xpc_double_get_value", shim_xpc_double_get_value as u64, "xpc_double_get_value → stored value")
        );

        // ── XPC Activity (timer-based scheduler) ─────────────────
        register_shims!(table,
            ("_xpc_activity_register", shim_xpc_activity_register as u64, "xpc_activity_register → registers + fires check-in"),
            ("_xpc_activity_unregister", shim_xpc_activity_unregister as u64, "xpc_activity_unregister → removes activity"),
            ("_xpc_activity_set_criteria", shim_xpc_activity_set_criteria as u64, "xpc_activity_set_criteria → updates criteria dict"),
            ("_xpc_activity_get_state", shim_xpc_activity_get_state as u64, "xpc_activity_get_state → real state machine"),
            ("_xpc_activity_should_defer", shim_xpc_activity_should_defer as u64, "xpc_activity_should_defer → false (always run)")
        );

        // ── XPC Endpoint / Pipe ─────────────────────────────────
        register_shims!(table,
            ("_xpc_endpoint_create", shim_xpc_endpoint_create as u64, "xpc_endpoint_create → real endpoint object"),
            ("_xpc_pipe_create", shim_xpc_pipe_create as u64, "xpc_pipe_create → named pipe with handler registry"),
            ("_xpc_pipe_routine", shim_xpc_pipe_routine as u64, "xpc_pipe_routine → routes to handler or EPIPE")
        );

        // Install default pipe handlers for well-known services
        pipe::install_defaults();

        log::debug!("XpcStubs: registered ~70 XPC shims (real objects + transport + activity + pipe)");
    }
}

// ── XPC Activity ────────────────────────────────────────────────

pub extern "C" fn shim_xpc_activity_register(name: u64, criteria: u64, handler: u64) {
    activity::register(name, criteria, handler);
}

pub extern "C" fn shim_xpc_activity_unregister(name: u64) {
    activity::unregister(name);
}

pub extern "C" fn shim_xpc_activity_set_criteria(act: u64, criteria: u64) {
    activity::set_criteria(act, criteria);
}

pub extern "C" fn shim_xpc_activity_get_state(act: u64) -> u64 {
    activity::get_state(act)
}

pub extern "C" fn shim_xpc_activity_should_defer(act: u64) -> i32 {
    activity::should_defer(act) as i32
}

// ── XPC Object lifecycle ────────────────────────────────────────

pub extern "C" fn shim_xpc_retain(o: u64) -> u64 { o }

pub extern "C" fn shim_xpc_release(o: u64) { obj::release(o); }

pub extern "C" fn shim_xpc_copy(o: u64) -> u64 { o }

pub extern "C" fn shim_xpc_equal(a: u64, b: u64) -> i32 { obj::equal(a, b) as i32 }

pub extern "C" fn shim_xpc_hash(o: u64) -> u64 { obj::hash(o) }

pub extern "C" fn shim_xpc_get_type(o: u64) -> u64 { obj::get_type_tag(o) }

pub extern "C" fn shim_xpc_copy_description(o: u64) -> u64 { obj::copy_description(o) }

// ── XPC Connection ──────────────────────────────────────────────

pub extern "C" fn shim_xpc_connection_create(name: u64, queue: u64) -> u64 {
    transport::create_connection(name, queue, 0)
}

pub extern "C" fn shim_xpc_connection_create_mach_service(
    name: u64, queue: u64, flags: u64,
) -> u64 {
    transport::create_mach_service(name, queue, flags)
}

pub extern "C" fn shim_xpc_connection_set_event_handler(conn: u64, block: u64) {
    transport::set_event_handler(conn, block);
}

pub extern "C" fn shim_xpc_connection_resume(conn: u64) {
    transport::resume(conn);
}

pub extern "C" fn shim_xpc_connection_suspend(conn: u64) {
    transport::suspend(conn);
}

pub extern "C" fn shim_xpc_connection_cancel(conn: u64) {
    transport::cancel(conn);
}

pub extern "C" fn shim_xpc_connection_send_message(conn: u64, msg: u64) {
    transport::send_message(conn, msg);
}

pub extern "C" fn shim_xpc_connection_send_message_with_reply(
    conn: u64, msg: u64, queue: u64, handler: u64,
) {
    transport::send_message_with_reply(conn, msg, queue, handler);
}

pub extern "C" fn shim_xpc_connection_send_message_with_reply_sync(
    conn: u64, msg: u64,
) -> u64 {
    transport::send_message_with_reply_sync(conn, msg)
}

pub extern "C" fn shim_xpc_connection_send_barrier(conn: u64, block: u64) {
    transport::send_barrier(conn, block);
}

pub extern "C" fn shim_xpc_connection_set_target_queue(conn: u64, queue: u64) {
    transport::set_target_queue(conn, queue);
}

pub extern "C" fn shim_xpc_connection_get_pid(conn: u64) -> u64 {
    transport::get_pid(conn) as u64
}

// ── XPC Dictionary ──────────────────────────────────────────────

pub extern "C" fn shim_xpc_dictionary_create(
    keys: u64, values: u64, count: u64,
) -> u64 {
    let dict = obj::create_dictionary_empty();
    if keys != 0 && values != 0 && count > 0 {
        for i in 0..count {
            unsafe {
                let key_ptr = *((keys + i * 8) as *const u64);
                let val_handle = *((values + i * 8) as *const u64);
                if key_ptr != 0 {
                    let key = std::ffi::CStr::from_ptr(key_ptr as *const i8);
                    obj::dict_set(dict, key, val_handle);
                }
            }
        }
    }
    dict
}

pub extern "C" fn shim_xpc_dictionary_create_empty() -> u64 {
    obj::create_dictionary_empty()
}

pub extern "C" fn shim_xpc_dictionary_set_value(dict: u64, key: u64, val: u64) {
    if key == 0 { return; }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const i8) };
    obj::dict_set(dict, k, val);
}

pub extern "C" fn shim_xpc_dictionary_get_value(dict: u64, key: u64) -> u64 {
    if key == 0 { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const i8) };
    obj::dict_get(dict, k)
}

pub extern "C" fn shim_xpc_dictionary_set_string(dict: u64, key: u64, val: u64) {
    obj::dict_set_string(dict, key, val);
}

pub extern "C" fn shim_xpc_dictionary_get_string(dict: u64, key: u64) -> u64 {
    obj::dict_get_string(dict, key)
}

pub extern "C" fn shim_xpc_dictionary_set_int64(dict: u64, key: u64, val: u64) {
    obj::dict_set_int64(dict, key, val as i64);
}

pub extern "C" fn shim_xpc_dictionary_get_int64(dict: u64, key: u64) -> u64 {
    obj::dict_get_int64(dict, key) as u64
}

pub extern "C" fn shim_xpc_dictionary_set_uint64(dict: u64, key: u64, val: u64) {
    obj::dict_set_uint64(dict, key, val);
}

pub extern "C" fn shim_xpc_dictionary_get_uint64(dict: u64, key: u64) -> u64 {
    obj::dict_get_uint64(dict, key)
}

pub extern "C" fn shim_xpc_dictionary_set_bool(dict: u64, key: u64, val: u64) {
    obj::dict_set_bool(dict, key, val != 0);
}

pub extern "C" fn shim_xpc_dictionary_get_bool(dict: u64, key: u64) -> i32 {
    obj::dict_get_bool(dict, key) as i32
}

pub extern "C" fn shim_xpc_dictionary_set_data(dict: u64, key: u64, data: u64, len: u64) {
    obj::dict_set_data(dict, key, data, len);
}

pub extern "C" fn shim_xpc_dictionary_get_data(dict: u64, key: u64) -> u64 {
    obj::dict_get_data(dict, key)
}

pub extern "C" fn shim_xpc_dictionary_set_fd(dict: u64, key: u64, fd: u64) {
    obj::dict_set_fd(dict, key, fd as i32);
}

pub extern "C" fn shim_xpc_dictionary_get_count(dict: u64) -> u64 {
    obj::dict_get_count(dict)
}

pub extern "C" fn shim_xpc_dictionary_apply(dict: u64, applier: u64) -> i32 {
    obj::dict_apply(dict, applier) as i32
}

pub extern "C" fn shim_xpc_dictionary_get_remote_connection(dict: u64) -> u64 {
    transport::get_remote_connection(dict)
}

// ── XPC Array ───────────────────────────────────────────────────

pub extern "C" fn shim_xpc_array_create(values: u64, count: u64) -> u64 {
    let arr = obj::create_array_empty();
    if values != 0 && count > 0 {
        for i in 0..count {
            unsafe {
                let val = *((values + i * 8) as *const u64);
                obj::array_append(arr, val);
            }
        }
    }
    arr
}

pub extern "C" fn shim_xpc_array_get_count(arr: u64) -> u64 {
    obj::array_get_count(arr)
}

pub extern "C" fn shim_xpc_array_get_value(arr: u64, index: u64) -> u64 {
    obj::array_get_value(arr, index)
}

pub extern "C" fn shim_xpc_array_append_value(arr: u64, val: u64) {
    obj::array_append(arr, val);
}

pub extern "C" fn shim_xpc_array_apply(arr: u64, applier: u64) -> i32 {
    obj::array_apply(arr, applier) as i32
}

// ── XPC Primitive constructors ──────────────────────────────────

pub extern "C" fn shim_xpc_string_create(ptr: u64) -> u64 {
    obj::create_string_from_ptr(ptr)
}

pub extern "C" fn shim_xpc_string_get_string_ptr(o: u64) -> u64 {
    obj::get_string_ptr(o)
}

pub extern "C" fn shim_xpc_string_get_length(o: u64) -> u64 {
    obj::get_string_length(o)
}

pub extern "C" fn shim_xpc_int64_create(val: u64) -> u64 {
    obj::create_int64(val as i64)
}

pub extern "C" fn shim_xpc_int64_get_value(o: u64) -> u64 {
    obj::get_int64_value(o) as u64
}

pub extern "C" fn shim_xpc_uint64_create(val: u64) -> u64 {
    obj::create_uint64(val)
}

pub extern "C" fn shim_xpc_uint64_get_value(o: u64) -> u64 {
    obj::get_uint64_value(o)
}

pub extern "C" fn shim_xpc_bool_create(val: u64) -> u64 {
    obj::create_bool(val != 0)
}

pub extern "C" fn shim_xpc_bool_get_value(o: u64) -> i32 {
    obj::get_bool_value(o) as i32
}

pub extern "C" fn shim_xpc_data_create(ptr: u64, len: u64) -> u64 {
    obj::create_data_from_ptr(ptr, len)
}

pub extern "C" fn shim_xpc_data_get_bytes_ptr(o: u64) -> u64 {
    obj::get_data_ptr(o)
}

pub extern "C" fn shim_xpc_data_get_length(o: u64) -> u64 {
    obj::get_data_length(o)
}

pub extern "C" fn shim_xpc_null_create() -> u64 {
    obj::create_null()
}

pub extern "C" fn shim_xpc_date_create(nanos: u64) -> u64 {
    obj::create_date(nanos as i64)
}

pub extern "C" fn shim_xpc_date_get_value(o: u64) -> u64 {
    obj::get_date_value(o) as u64
}

pub extern "C" fn shim_xpc_uuid_create(ptr: u64) -> u64 {
    obj::create_uuid_from_ptr(ptr)
}

pub extern "C" fn shim_xpc_fd_create(fd: u64) -> u64 {
    obj::create_fd(fd as i32)
}

pub extern "C" fn shim_xpc_double_create(val: u64) -> u64 {
    // Reinterpret u64 bits as f64
    let f = f64::from_bits(val);
    obj::create_double(f)
}

pub extern "C" fn shim_xpc_double_get_value(o: u64) -> f64 {
    obj::get_double_value(o)
}

// ── XPC Endpoint / Pipe ─────────────────────────────────────────

pub extern "C" fn shim_xpc_endpoint_create(conn: u64) -> u64 {
    obj::alloc(obj::XpcValue::Endpoint(conn))
}

pub extern "C" fn shim_xpc_pipe_create(name: u64, flags: u64) -> u64 {
    pipe::create(name, flags)
}

pub extern "C" fn shim_xpc_pipe_routine(
    pipe_handle: u64, msg: u64, reply_out: u64,
) -> i32 {
    pipe::routine(pipe_handle, msg, reply_out)
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::ShimTable;

    #[test]
    fn xpc_stubs_registered() {
        let mut table = ShimTable::new();
        XpcStubs::install(&mut table);
        assert!(table.resolve("_xpc_retain").is_some());
        assert!(table.resolve("_xpc_connection_create").is_some());
        assert!(table.resolve("_xpc_dictionary_create").is_some());
        assert!(table.resolve("_xpc_bool_get_value").is_some());
        assert!(table.resolve("_xpc_pipe_routine").is_some());
    }

    #[test]
    fn connection_returns_valid_handle() {
        let name = std::ffi::CString::new("com.test.xpc").unwrap();
        let h = shim_xpc_connection_create(name.as_ptr() as u64, 0);
        assert_ne!(h, 0);
        assert!(obj::is_valid(h));
    }

    #[test]
    fn dictionary_round_trip() {
        let dict = shim_xpc_dictionary_create_empty();
        assert_ne!(dict, 0);
        assert_eq!(shim_xpc_dictionary_get_count(dict), 0);

        // String round-trip
        shim_xpc_dictionary_set_string(dict, b"name\0".as_ptr() as u64, b"hello\0".as_ptr() as u64);
        let ptr = shim_xpc_dictionary_get_string(dict, b"name\0".as_ptr() as u64);
        assert_ne!(ptr, 0);
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const i8) };
        assert_eq!(s.to_str().unwrap(), "hello");

        // Int64 round-trip
        shim_xpc_dictionary_set_int64(dict, b"count\0".as_ptr() as u64, 42);
        assert_eq!(shim_xpc_dictionary_get_int64(dict, b"count\0".as_ptr() as u64), 42);

        // Bool round-trip
        shim_xpc_dictionary_set_bool(dict, b"flag\0".as_ptr() as u64, 1);
        assert_eq!(shim_xpc_dictionary_get_bool(dict, b"flag\0".as_ptr() as u64), 1);

        assert_eq!(shim_xpc_dictionary_get_count(dict), 3);
    }

    #[test]
    fn array_round_trip() {
        let arr = shim_xpc_array_create(0, 0);
        assert_ne!(arr, 0);
        assert_eq!(shim_xpc_array_get_count(arr), 0);

        let v1 = shim_xpc_int64_create(10);
        let v2 = shim_xpc_int64_create(20);
        shim_xpc_array_append_value(arr, v1);
        shim_xpc_array_append_value(arr, v2);
        assert_eq!(shim_xpc_array_get_count(arr), 2);
        assert_eq!(shim_xpc_array_get_value(arr, 0), v1);
    }

    #[test]
    fn string_round_trip() {
        let h = shim_xpc_string_create(b"test\0".as_ptr() as u64);
        assert_ne!(h, 0);
        let ptr = shim_xpc_string_get_string_ptr(h);
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const i8) };
        assert_eq!(s.to_str().unwrap(), "test");
        assert_eq!(shim_xpc_string_get_length(h), 4);
    }

    #[test]
    fn int64_round_trip() {
        let h = shim_xpc_int64_create((-99i64) as u64);
        let v = shim_xpc_int64_get_value(h) as i64;
        assert_eq!(v, -99);
    }

    #[test]
    fn bool_round_trip() {
        let t = shim_xpc_bool_create(1);
        let f = shim_xpc_bool_create(0);
        assert_eq!(shim_xpc_bool_get_value(t), 1);
        assert_eq!(shim_xpc_bool_get_value(f), 0);
    }

    #[test]
    fn double_round_trip() {
        let h = shim_xpc_double_create(3.14f64.to_bits());
        let v = shim_xpc_double_get_value(h);
        assert!((v - 3.14).abs() < 1e-10);
    }

    #[test]
    fn data_round_trip() {
        let data = [1u8, 2, 3, 4];
        let h = shim_xpc_data_create(data.as_ptr() as u64, 4);
        assert_eq!(shim_xpc_data_get_length(h), 4);
        let ptr = shim_xpc_data_get_bytes_ptr(h);
        assert_ne!(ptr, 0);
        let read = unsafe { std::slice::from_raw_parts(ptr as *const u8, 4) };
        assert_eq!(read, &[1, 2, 3, 4]);
    }

    #[test]
    fn type_tag_check() {
        let s = shim_xpc_string_create(b"x\0".as_ptr() as u64);
        assert_eq!(shim_xpc_get_type(s), obj::XPC_TYPE_TAG_STRING);
        let i = shim_xpc_int64_create(0);
        assert_eq!(shim_xpc_get_type(i), obj::XPC_TYPE_TAG_INT64);
        let d = shim_xpc_dictionary_create_empty();
        assert_eq!(shim_xpc_get_type(d), obj::XPC_TYPE_TAG_DICTIONARY);
    }

    #[test]
    fn copy_description_valid() {
        let h = shim_xpc_int64_create(42);
        let ptr = shim_xpc_copy_description(h);
        assert_ne!(ptr, 0);
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr as *const i8) };
        assert_eq!(cstr.to_str().unwrap(), "42");
    }

    #[test]
    fn pipe_routine_epipe() {
        // Unknown pipe (no name) → EPIPE
        let unknown = obj::create_dictionary_empty();
        assert_eq!(shim_xpc_pipe_routine(unknown, obj::create_dictionary_empty(), 0), 32);
    }

    #[test]
    fn pipe_routine_with_handler() {
        // Install defaults and test launchd stub
        pipe::install_defaults();
        let p = shim_xpc_pipe_create(b"com.apple.xpc.launchd\0".as_ptr() as u64, 0);
        let req = obj::create_dictionary_empty();
        let mut reply: u64 = 0;
        let result = shim_xpc_pipe_routine(p, req, &mut reply as *mut u64 as u64);
        assert_eq!(result, 0); // Handler exists
        assert_ne!(reply, 0);
    }

    #[test]
    fn activity_register_stores() {
        let name = std::ffi::CString::new("com.test.act.shim").unwrap();
        shim_xpc_activity_register(name.as_ptr() as u64, 0, 0);
        assert!(activity::has_activity("com.test.act.shim"));
    }

    #[test]
    fn activity_should_defer_false() {
        assert_eq!(shim_xpc_activity_should_defer(0), 0);
    }

    #[test]
    fn activity_get_state_returns_check_in() {
        assert_eq!(shim_xpc_activity_get_state(0), 0); // CHECK_IN
    }

    #[test]
    fn null_and_date() {
        let n = shim_xpc_null_create();
        assert_eq!(shim_xpc_get_type(n), obj::XPC_TYPE_TAG_NULL);
        let d = shim_xpc_date_create(1_000_000);
        assert_eq!(shim_xpc_date_get_value(d), 1_000_000);
    }
}
