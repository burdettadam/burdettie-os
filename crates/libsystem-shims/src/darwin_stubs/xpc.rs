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

use super::xpc_activity as activity;
use super::xpc_objects as obj;
use super::xpc_pipe as pipe;
use super::xpc_transport as transport;
use crate::table::ShimTable;

pub struct XpcStubs;

impl XpcStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── XPC Object lifecycle ────────────────────────────────
        register_shims!(
            table,
            (
                "_xpc_retain",
                shim_xpc_retain as *const () as u64,
                "xpc_retain → identity (refcount not tracked)"
            ),
            (
                "_xpc_release",
                shim_xpc_release as *const () as u64,
                "xpc_release → removes from object store"
            ),
            (
                "_xpc_copy",
                shim_xpc_copy as *const () as u64,
                "xpc_copy → identity (shallow)"
            ),
            (
                "_xpc_equal",
                shim_xpc_equal as *const () as u64,
                "xpc_equal → value comparison"
            ),
            (
                "_xpc_hash",
                shim_xpc_hash as *const () as u64,
                "xpc_hash → value hash"
            ),
            (
                "_xpc_get_type",
                shim_xpc_get_type as *const () as u64,
                "xpc_get_type → type tag pointer"
            ),
            (
                "_xpc_copy_description",
                shim_xpc_copy_description as *const () as u64,
                "xpc_copy_description → malloc'd string"
            )
        );

        // ── XPC Connection ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_xpc_connection_create",
                shim_xpc_connection_create as *const () as u64,
                "xpc_connection_create → tracked connection"
            ),
            (
                "_xpc_connection_create_mach_service",
                shim_xpc_connection_create_mach_service as *const () as u64,
                "xpc_connection_create_mach_service → tracked connection"
            ),
            (
                "_xpc_connection_set_event_handler",
                shim_xpc_connection_set_event_handler as *const () as u64,
                "xpc_connection_set_event_handler → stores block"
            ),
            (
                "_xpc_connection_resume",
                shim_xpc_connection_resume as *const () as u64,
                "xpc_connection_resume → delivers pending msgs"
            ),
            (
                "_xpc_connection_suspend",
                shim_xpc_connection_suspend as *const () as u64,
                "xpc_connection_suspend → pauses delivery"
            ),
            (
                "_xpc_connection_cancel",
                shim_xpc_connection_cancel as *const () as u64,
                "xpc_connection_cancel → sends INVALID error"
            ),
            (
                "_xpc_connection_send_message",
                shim_xpc_connection_send_message as *const () as u64,
                "xpc_connection_send_message → loopback delivery"
            ),
            (
                "_xpc_connection_send_message_with_reply",
                shim_xpc_connection_send_message_with_reply as *const () as u64,
                "xpc_connection_send_message_with_reply → calls handler"
            ),
            (
                "_xpc_connection_send_message_with_reply_sync",
                shim_xpc_connection_send_message_with_reply_sync as *const () as u64,
                "xpc_connection_send_message_with_reply_sync → error reply"
            ),
            (
                "_xpc_connection_send_barrier",
                shim_xpc_connection_send_barrier as *const () as u64,
                "xpc_connection_send_barrier → inline block"
            ),
            (
                "_xpc_connection_set_target_queue",
                shim_xpc_connection_set_target_queue as *const () as u64,
                "xpc_connection_set_target_queue → stores queue"
            ),
            (
                "_xpc_connection_get_pid",
                shim_xpc_connection_get_pid as *const () as u64,
                "xpc_connection_get_pid → getpid()"
            )
        );

        // ── XPC Dictionary ──────────────────────────────────────
        register_shims!(
            table,
            (
                "_xpc_dictionary_create",
                shim_xpc_dictionary_create as *const () as u64,
                "xpc_dictionary_create → real dict object"
            ),
            (
                "_xpc_dictionary_create_empty",
                shim_xpc_dictionary_create_empty as *const () as u64,
                "xpc_dictionary_create_empty → real dict object"
            ),
            (
                "_xpc_dictionary_set_value",
                shim_xpc_dictionary_set_value as *const () as u64,
                "xpc_dictionary_set_value → stores pair"
            ),
            (
                "_xpc_dictionary_get_value",
                shim_xpc_dictionary_get_value as *const () as u64,
                "xpc_dictionary_get_value → retrieves pair"
            ),
            (
                "_xpc_dictionary_set_string",
                shim_xpc_dictionary_set_string as *const () as u64,
                "xpc_dictionary_set_string → stores string"
            ),
            (
                "_xpc_dictionary_get_string",
                shim_xpc_dictionary_get_string as *const () as u64,
                "xpc_dictionary_get_string → returns C string ptr"
            ),
            (
                "_xpc_dictionary_set_int64",
                shim_xpc_dictionary_set_int64 as *const () as u64,
                "xpc_dictionary_set_int64 → stores int64"
            ),
            (
                "_xpc_dictionary_get_int64",
                shim_xpc_dictionary_get_int64 as *const () as u64,
                "xpc_dictionary_get_int64 → retrieves int64"
            ),
            (
                "_xpc_dictionary_set_uint64",
                shim_xpc_dictionary_set_uint64 as *const () as u64,
                "xpc_dictionary_set_uint64 → stores uint64"
            ),
            (
                "_xpc_dictionary_get_uint64",
                shim_xpc_dictionary_get_uint64 as *const () as u64,
                "xpc_dictionary_get_uint64 → retrieves uint64"
            ),
            (
                "_xpc_dictionary_set_bool",
                shim_xpc_dictionary_set_bool as *const () as u64,
                "xpc_dictionary_set_bool → stores bool"
            ),
            (
                "_xpc_dictionary_get_bool",
                shim_xpc_dictionary_get_bool as *const () as u64,
                "xpc_dictionary_get_bool → retrieves bool"
            ),
            (
                "_xpc_dictionary_set_data",
                shim_xpc_dictionary_set_data as *const () as u64,
                "xpc_dictionary_set_data → stores data"
            ),
            (
                "_xpc_dictionary_get_data",
                shim_xpc_dictionary_get_data as *const () as u64,
                "xpc_dictionary_get_data → retrieves data ptr"
            ),
            (
                "_xpc_dictionary_set_fd",
                shim_xpc_dictionary_set_fd as *const () as u64,
                "xpc_dictionary_set_fd → stores fd"
            ),
            (
                "_xpc_dictionary_get_count",
                shim_xpc_dictionary_get_count as *const () as u64,
                "xpc_dictionary_get_count → real count"
            ),
            (
                "_xpc_dictionary_apply",
                shim_xpc_dictionary_apply as *const () as u64,
                "xpc_dictionary_apply → iterates pairs"
            ),
            (
                "_xpc_dictionary_get_remote_connection",
                shim_xpc_dictionary_get_remote_connection as *const () as u64,
                "xpc_dictionary_get_remote_connection → NULL (loopback)"
            )
        );

        // ── XPC Array ───────────────────────────────────────────
        register_shims!(
            table,
            (
                "_xpc_array_create",
                shim_xpc_array_create as *const () as u64,
                "xpc_array_create → real array object"
            ),
            (
                "_xpc_array_get_count",
                shim_xpc_array_get_count as *const () as u64,
                "xpc_array_get_count → real count"
            ),
            (
                "_xpc_array_get_value",
                shim_xpc_array_get_value as *const () as u64,
                "xpc_array_get_value → retrieves element"
            ),
            (
                "_xpc_array_append_value",
                shim_xpc_array_append_value as *const () as u64,
                "xpc_array_append_value → appends element"
            ),
            (
                "_xpc_array_apply",
                shim_xpc_array_apply as *const () as u64,
                "xpc_array_apply → iterates elements"
            )
        );

        // ── XPC Primitive constructors ──────────────────────────
        register_shims!(
            table,
            (
                "_xpc_string_create",
                shim_xpc_string_create as *const () as u64,
                "xpc_string_create → real string object"
            ),
            (
                "_xpc_string_get_string_ptr",
                shim_xpc_string_get_string_ptr as *const () as u64,
                "xpc_string_get_string_ptr → C string pointer"
            ),
            (
                "_xpc_string_get_length",
                shim_xpc_string_get_length as *const () as u64,
                "xpc_string_get_length → byte count"
            ),
            (
                "_xpc_int64_create",
                shim_xpc_int64_create as *const () as u64,
                "xpc_int64_create → real int64 object"
            ),
            (
                "_xpc_int64_get_value",
                shim_xpc_int64_get_value as *const () as u64,
                "xpc_int64_get_value → stored value"
            ),
            (
                "_xpc_uint64_create",
                shim_xpc_uint64_create as *const () as u64,
                "xpc_uint64_create → real uint64 object"
            ),
            (
                "_xpc_uint64_get_value",
                shim_xpc_uint64_get_value as *const () as u64,
                "xpc_uint64_get_value → stored value"
            ),
            (
                "_xpc_bool_create",
                shim_xpc_bool_create as *const () as u64,
                "xpc_bool_create → real bool object"
            ),
            (
                "_xpc_bool_get_value",
                shim_xpc_bool_get_value as *const () as u64,
                "xpc_bool_get_value → stored value"
            ),
            (
                "_xpc_data_create",
                shim_xpc_data_create as *const () as u64,
                "xpc_data_create → real data object"
            ),
            (
                "_xpc_data_get_bytes_ptr",
                shim_xpc_data_get_bytes_ptr as *const () as u64,
                "xpc_data_get_bytes_ptr → byte pointer"
            ),
            (
                "_xpc_data_get_length",
                shim_xpc_data_get_length as *const () as u64,
                "xpc_data_get_length → byte count"
            ),
            (
                "_xpc_null_create",
                shim_xpc_null_create as *const () as u64,
                "xpc_null_create → real null object"
            ),
            (
                "_xpc_date_create",
                shim_xpc_date_create as *const () as u64,
                "xpc_date_create → real date object"
            ),
            (
                "_xpc_date_get_value",
                shim_xpc_date_get_value as *const () as u64,
                "xpc_date_get_value → stored nanos"
            ),
            (
                "_xpc_uuid_create",
                shim_xpc_uuid_create as *const () as u64,
                "xpc_uuid_create → real uuid object"
            ),
            (
                "_xpc_fd_create",
                shim_xpc_fd_create as *const () as u64,
                "xpc_fd_create → real fd object"
            ),
            (
                "_xpc_double_create",
                shim_xpc_double_create as *const () as u64,
                "xpc_double_create → real double object"
            ),
            (
                "_xpc_double_get_value",
                shim_xpc_double_get_value as *const () as u64,
                "xpc_double_get_value → stored value"
            )
        );

        // ── XPC Activity (timer-based scheduler) ─────────────────
        register_shims!(
            table,
            (
                "_xpc_activity_register",
                shim_xpc_activity_register as *const () as u64,
                "xpc_activity_register → registers + fires check-in"
            ),
            (
                "_xpc_activity_unregister",
                shim_xpc_activity_unregister as *const () as u64,
                "xpc_activity_unregister → removes activity"
            ),
            (
                "_xpc_activity_set_criteria",
                shim_xpc_activity_set_criteria as *const () as u64,
                "xpc_activity_set_criteria → updates criteria dict"
            ),
            (
                "_xpc_activity_get_state",
                shim_xpc_activity_get_state as *const () as u64,
                "xpc_activity_get_state → real state machine"
            ),
            (
                "_xpc_activity_should_defer",
                shim_xpc_activity_should_defer as *const () as u64,
                "xpc_activity_should_defer → false (always run)"
            )
        );

        // ── XPC Endpoint / Pipe ─────────────────────────────────
        register_shims!(
            table,
            (
                "_xpc_endpoint_create",
                shim_xpc_endpoint_create as *const () as u64,
                "xpc_endpoint_create → real endpoint object"
            ),
            (
                "_xpc_pipe_create",
                shim_xpc_pipe_create as *const () as u64,
                "xpc_pipe_create → named pipe with handler registry"
            ),
            (
                "_xpc_pipe_routine",
                shim_xpc_pipe_routine as *const () as u64,
                "xpc_pipe_routine → routes to handler or EPIPE"
            ),
            (
                "_xpc_pipe_routine_with_flags",
                shim_xpc_pipe_routine_with_flags as *const () as u64,
                "xpc_pipe_routine_with_flags → routes to handler or EPIPE"
            )
        );

        // ── Missing XPC shims (gap closure) ─────────────────────
        register_shims!(
            table,
            (
                "_xpc_connection_create_from_endpoint",
                shim_xpc_connection_create_from_endpoint as *const () as u64,
                "xpc_connection_create_from_endpoint → tracked connection"
            ),
            (
                "_xpc_connection_get_name",
                shim_xpc_connection_get_name as *const () as u64,
                "xpc_connection_get_name → service name C string"
            ),
            (
                "_xpc_connection_get_euid",
                shim_xpc_connection_get_euid as *const () as u64,
                "xpc_connection_get_euid → current euid"
            ),
            (
                "_xpc_connection_get_egid",
                shim_xpc_connection_get_egid as *const () as u64,
                "xpc_connection_get_egid → current egid"
            ),
            (
                "_xpc_connection_get_asid",
                shim_xpc_connection_get_asid as *const () as u64,
                "xpc_connection_get_asid → 0"
            ),
            (
                "_xpc_connection_set_context",
                shim_xpc_connection_set_context as *const () as u64,
                "xpc_connection_set_context → stores opaque pointer"
            ),
            (
                "_xpc_connection_get_context",
                shim_xpc_connection_get_context as *const () as u64,
                "xpc_connection_get_context → retrieves opaque pointer"
            ),
            (
                "_xpc_connection_set_finalizer_f",
                shim_xpc_void2 as *const () as u64,
                "xpc_connection_set_finalizer_f → stub"
            ),
            (
                "_xpc_dictionary_create_reply",
                shim_xpc_dictionary_create_reply as *const () as u64,
                "xpc_dictionary_create_reply → empty reply dict"
            ),
            (
                "_xpc_dictionary_get_data_with_size",
                shim_xpc_dictionary_get_data_with_size as *const () as u64,
                "xpc_dictionary_get_data_with_size → data ptr + size"
            ),
            (
                "_xpc_dictionary_set_connection",
                shim_xpc_void3 as *const () as u64,
                "xpc_dictionary_set_connection → stub"
            ),
            (
                "_xpc_dictionary_dup_fd",
                shim_xpc_dictionary_dup_fd as *const () as u64,
                "xpc_dictionary_dup_fd → dup'd fd or -1"
            ),
            (
                "_xpc_dictionary_get_fd",
                shim_xpc_dictionary_get_fd as *const () as u64,
                "xpc_dictionary_get_fd → raw fd or -1"
            ),
            (
                "_xpc_dictionary_set_uuid",
                shim_xpc_dictionary_set_uuid as *const () as u64,
                "xpc_dictionary_set_uuid → stores uuid"
            ),
            (
                "_xpc_dictionary_get_uuid",
                shim_xpc_dictionary_get_uuid as *const () as u64,
                "xpc_dictionary_get_uuid → uuid bytes ptr"
            ),
            (
                "_xpc_dictionary_set_double",
                shim_xpc_dictionary_set_double as *const () as u64,
                "xpc_dictionary_set_double → stores double"
            ),
            (
                "_xpc_dictionary_get_double",
                shim_xpc_dictionary_get_double as *const () as u64,
                "xpc_dictionary_get_double → retrieves double"
            ),
            (
                "_xpc_dictionary_set_date",
                shim_xpc_dictionary_set_date as *const () as u64,
                "xpc_dictionary_set_date → stores date"
            ),
            (
                "_xpc_dictionary_get_date",
                shim_xpc_dictionary_get_date as *const () as u64,
                "xpc_dictionary_get_date → retrieves date"
            ),
            (
                "_xpc_array_set_value",
                shim_xpc_void3 as *const () as u64,
                "xpc_array_set_value → stub"
            ),
            (
                "_xpc_array_create_empty",
                shim_xpc_array_create_empty as *const () as u64,
                "xpc_array_create_empty → empty array"
            ),
            (
                "_xpc_shmem_create",
                shim_xpc_shmem_create as *const () as u64,
                "xpc_shmem_create → real shared memory region"
            ),
            (
                "_xpc_shmem_map",
                shim_xpc_shmem_map as *const () as u64,
                "xpc_shmem_map → maps shared memory region"
            ),
            (
                "_xpc_type_get_name",
                shim_xpc_type_get_name as *const () as u64,
                "xpc_type_get_name → type name string"
            ),
            (
                "_xpc_data_get_bytes",
                shim_xpc_data_get_bytes as *const () as u64,
                "xpc_data_get_bytes → copies bytes to buffer"
            )
        );

        // Install default pipe handlers for well-known services
        pipe::install_defaults();

        log::debug!(
            "XpcStubs: registered ~70 XPC shims (real objects + transport + activity + pipe)"
        );
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

pub extern "C" fn shim_xpc_retain(o: u64) -> u64 {
    o
}

pub extern "C" fn shim_xpc_release(o: u64) {
    obj::release(o);
}

pub extern "C" fn shim_xpc_copy(o: u64) -> u64 {
    o
}

pub extern "C" fn shim_xpc_equal(a: u64, b: u64) -> i32 {
    obj::equal(a, b) as i32
}

pub extern "C" fn shim_xpc_hash(o: u64) -> u64 {
    obj::hash(o)
}

pub extern "C" fn shim_xpc_get_type(o: u64) -> u64 {
    obj::get_type_tag(o)
}

pub extern "C" fn shim_xpc_copy_description(o: u64) -> u64 {
    obj::copy_description(o)
}

// ── XPC Connection ──────────────────────────────────────────────

pub extern "C" fn shim_xpc_connection_create(name: u64, queue: u64) -> u64 {
    transport::create_connection(name, queue, 0)
}

pub extern "C" fn shim_xpc_connection_create_mach_service(
    name: u64,
    queue: u64,
    flags: u64,
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
    conn: u64,
    msg: u64,
    queue: u64,
    handler: u64,
) {
    transport::send_message_with_reply(conn, msg, queue, handler);
}

pub extern "C" fn shim_xpc_connection_send_message_with_reply_sync(conn: u64, msg: u64) -> u64 {
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

pub extern "C" fn shim_xpc_dictionary_create(keys: u64, values: u64, count: u64) -> u64 {
    let dict = obj::create_dictionary_empty();
    if keys != 0 && values != 0 && count > 0 {
        for i in 0..count {
            unsafe {
                let key_ptr = *((keys + i * 8) as *const u64);
                let val_handle = *((values + i * 8) as *const u64);
                if key_ptr != 0 {
                    let key = std::ffi::CStr::from_ptr(key_ptr as *const std::ffi::c_char);
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
    if key == 0 {
        return;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    obj::dict_set(dict, k, val);
}

pub extern "C" fn shim_xpc_dictionary_get_value(dict: u64, key: u64) -> u64 {
    if key == 0 {
        return 0;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
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

pub extern "C" fn shim_xpc_pipe_routine(pipe_handle: u64, msg: u64, reply_out: u64) -> i32 {
    pipe::routine(pipe_handle, msg, reply_out)
}

pub extern "C" fn shim_xpc_pipe_routine_with_flags(
    pipe_handle: u64,
    msg: u64,
    reply_out: u64,
    _flags: u64,
) -> i32 {
    pipe::routine(pipe_handle, msg, reply_out)
}

// ── Missing XPC shims (gap closure) ─────────────────────────────

pub extern "C" fn shim_xpc_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_xpc_void3(_a: u64, _b: u64, _c: u64) {}

pub extern "C" fn shim_xpc_connection_create_from_endpoint(endpoint: u64, queue: u64) -> u64 {
    // Just create a fresh connection — the endpoint carries a connection id
    // but in our loopback model connections are independent.
    transport::create_connection(0, queue, 0)
}

pub extern "C" fn shim_xpc_connection_get_name(conn: u64) -> u64 {
    transport::get_service_name_ptr(conn)
}

pub extern "C" fn shim_xpc_connection_get_euid(_conn: u64) -> u64 {
    unsafe { libc::geteuid() as u64 }
}

pub extern "C" fn shim_xpc_connection_get_egid(_conn: u64) -> u64 {
    unsafe { libc::getegid() as u64 }
}

pub extern "C" fn shim_xpc_connection_get_asid(_conn: u64) -> u64 {
    0 // audit session ID — not meaningful in emulation
}

pub extern "C" fn shim_xpc_connection_set_context(conn: u64, ctx: u64) {
    transport::set_context(conn, ctx);
}

pub extern "C" fn shim_xpc_connection_get_context(conn: u64) -> u64 {
    transport::get_context(conn)
}

/// `xpc_dictionary_t xpc_dictionary_create_reply(xpc_object_t original)`
///
/// Creates a reply dictionary for an incoming request. We just return a
/// fresh empty dictionary since our transport doesn't track message IDs.
pub extern "C" fn shim_xpc_dictionary_create_reply(_original: u64) -> u64 {
    obj::create_dictionary_empty()
}

/// `const void *xpc_dictionary_get_data_with_size(dict, key, length_out)`
pub extern "C" fn shim_xpc_dictionary_get_data_with_size(
    dict: u64,
    key: u64,
    length_out: u64,
) -> u64 {
    if key == 0 {
        return 0;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return 0;
    }

    obj::with_table(|t| {
        if let Some(obj::XpcValue::Data(bytes)) = t.get(&handle) {
            if length_out != 0 {
                unsafe { *(length_out as *mut u64) = bytes.len() as u64 };
            }
            bytes.as_ptr() as u64
        } else {
            0
        }
    })
}

pub extern "C" fn shim_xpc_dictionary_dup_fd(dict: u64, key: u64) -> i32 {
    if key == 0 {
        return -1;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return -1;
    }

    obj::with_table(|t| {
        if let Some(obj::XpcValue::Fd(fd)) = t.get(&handle) {
            let duped = unsafe { libc::dup(*fd) };
            if duped < 0 {
                -1
            } else {
                duped
            }
        } else {
            -1
        }
    })
}

pub extern "C" fn shim_xpc_dictionary_get_fd(dict: u64, key: u64) -> i32 {
    if key == 0 {
        return -1;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return -1;
    }

    obj::with_table(|t| {
        if let Some(obj::XpcValue::Fd(fd)) = t.get(&handle) {
            *fd
        } else {
            -1
        }
    })
}

pub extern "C" fn shim_xpc_dictionary_set_uuid(dict: u64, key: u64, uuid_ptr: u64) {
    if key == 0 {
        return;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let h = obj::create_uuid_from_ptr(uuid_ptr);
    obj::dict_set(dict, k, h);
}

pub extern "C" fn shim_xpc_dictionary_get_uuid(dict: u64, key: u64) -> u64 {
    if key == 0 {
        return 0;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return 0;
    }

    obj::with_table(|t| {
        if let Some(obj::XpcValue::Uuid(bytes)) = t.get(&handle) {
            bytes.as_ptr() as u64
        } else {
            0
        }
    })
}

pub extern "C" fn shim_xpc_dictionary_set_double(dict: u64, key: u64, val: f64) {
    if key == 0 {
        return;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let h = obj::create_double(val);
    obj::dict_set(dict, k, h);
}

pub extern "C" fn shim_xpc_dictionary_get_double(dict: u64, key: u64) -> f64 {
    if key == 0 {
        return 0.0;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return 0.0;
    }

    obj::with_table(|t| match t.get(&handle) {
        Some(obj::XpcValue::Double(v)) => *v,
        _ => 0.0,
    })
}

pub extern "C" fn shim_xpc_dictionary_set_date(dict: u64, key: u64, val: i64) {
    if key == 0 {
        return;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let h = obj::create_date(val);
    obj::dict_set(dict, k, h);
}

pub extern "C" fn shim_xpc_dictionary_get_date(dict: u64, key: u64) -> i64 {
    if key == 0 {
        return 0;
    }
    let k = unsafe { std::ffi::CStr::from_ptr(key as *const std::ffi::c_char) };
    let handle = obj::dict_get(dict, k);
    if handle == 0 {
        return 0;
    }
    obj::get_date_value(handle)
}

pub extern "C" fn shim_xpc_array_create_empty() -> u64 {
    obj::create_array_empty()
}

/// `xpc_object_t xpc_shmem_create(void *region, size_t length)`
///
/// Creates a shared memory object. In our emulation, this wraps an
/// mmap'd anonymous region. The region pointer is stored as Data bytes.
pub extern "C" fn shim_xpc_shmem_create(region: u64, length: u64) -> u64 {
    if region == 0 || length == 0 {
        return obj::create_null();
    }
    // Store the region address and length in a dictionary for later retrieval
    let shmem = obj::create_dictionary_empty();
    obj::dict_set_uint64(shmem, b"_shmem_addr\0".as_ptr() as u64, region);
    obj::dict_set_uint64(shmem, b"_shmem_size\0".as_ptr() as u64, length);
    shmem
}

/// `size_t xpc_shmem_map(xpc_object_t shmem, void **region)`
///
/// Maps the shared memory region and writes the address to *region.
/// Returns the size. In our emulation we just pass through the original pointer.
pub extern "C" fn shim_xpc_shmem_map(shmem: u64, region_out: u64) -> u64 {
    let addr = obj::dict_get_uint64(shmem, b"_shmem_addr\0".as_ptr() as u64);
    let size = obj::dict_get_uint64(shmem, b"_shmem_size\0".as_ptr() as u64);

    if region_out != 0 && addr != 0 {
        unsafe { *(region_out as *mut u64) = addr };
    }
    size
}

/// `const char *xpc_type_get_name(xpc_type_t type)`
pub extern "C" fn shim_xpc_type_get_name(type_tag: u64) -> u64 {
    let name: &[u8] = match type_tag {
        obj::XPC_TYPE_TAG_DICTIONARY => b"dictionary\0",
        obj::XPC_TYPE_TAG_ARRAY => b"array\0",
        obj::XPC_TYPE_TAG_STRING => b"string\0",
        obj::XPC_TYPE_TAG_INT64 => b"int64\0",
        obj::XPC_TYPE_TAG_UINT64 => b"uint64\0",
        obj::XPC_TYPE_TAG_BOOL => b"bool\0",
        obj::XPC_TYPE_TAG_DATA => b"data\0",
        obj::XPC_TYPE_TAG_DATE => b"date\0",
        obj::XPC_TYPE_TAG_UUID => b"uuid\0",
        obj::XPC_TYPE_TAG_FD => b"fd\0",
        obj::XPC_TYPE_TAG_NULL => b"null\0",
        obj::XPC_TYPE_TAG_ERROR => b"error\0",
        obj::XPC_TYPE_TAG_CONNECTION => b"connection\0",
        obj::XPC_TYPE_TAG_ENDPOINT => b"endpoint\0",
        _ => b"unknown\0",
    };
    name.as_ptr() as u64
}

/// `size_t xpc_data_get_bytes(xpc_object_t data_obj, void *buffer,
///     size_t offset, size_t length)`
///
/// Copies up to `length` bytes from `offset` into `buffer`. Returns bytes copied.
pub extern "C" fn shim_xpc_data_get_bytes(
    data_obj: u64,
    buffer: u64,
    offset: u64,
    length: u64,
) -> u64 {
    if buffer == 0 {
        return 0;
    }

    obj::with_table(|t| {
        if let Some(obj::XpcValue::Data(bytes)) = t.get(&data_obj) {
            let off = offset as usize;
            let len = length as usize;
            if off >= bytes.len() {
                return 0;
            }
            let end = (off + len).min(bytes.len());
            let count = end - off;
            unsafe {
                std::ptr::copy_nonoverlapping(bytes[off..end].as_ptr(), buffer as *mut u8, count);
            }
            count as u64
        } else {
            0
        }
    })
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
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(s.to_str().unwrap(), "hello");

        // Int64 round-trip
        shim_xpc_dictionary_set_int64(dict, b"count\0".as_ptr() as u64, 42);
        assert_eq!(
            shim_xpc_dictionary_get_int64(dict, b"count\0".as_ptr() as u64),
            42
        );

        // Bool round-trip
        shim_xpc_dictionary_set_bool(dict, b"flag\0".as_ptr() as u64, 1);
        assert_eq!(
            shim_xpc_dictionary_get_bool(dict, b"flag\0".as_ptr() as u64),
            1
        );

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
        let s = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
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
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(cstr.to_str().unwrap(), "42");
    }

    #[test]
    fn pipe_routine_epipe() {
        // Unknown pipe (no name) → EPIPE
        let unknown = obj::create_dictionary_empty();
        assert_eq!(
            shim_xpc_pipe_routine(unknown, obj::create_dictionary_empty(), 0),
            32
        );
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
