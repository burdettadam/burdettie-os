//! Stubs for Apple's launchd APIs: `launch.h`, `bootstrap.h`, `vproc.h`.
//!
//! Delegates to the `launchd` crate which provides a full in-process
//! reimplementation of these APIs.

use crate::table::ShimTable;

// ═══════════════════════════════════════════════════════════════════
//  launch.h — launch_data_t + launch_msg + launch_get_fd
// ═══════════════════════════════════════════════════════════════════

pub extern "C" fn shim_launch_data_alloc(ty: u32) -> u64 {
    launchd::launch_data::alloc(ty) as u64
}

/// Forward to [`launchd::launch_data::copy`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::copy`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_copy(handle: u64) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::copy(handle as *mut launchd::launch_data::LaunchData) as u64 }
}

/// Forward to [`launchd::launch_data::get_type`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_type`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_type(handle: u64) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_type(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::free`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::free`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_free(handle: u64) {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::free(handle as *mut launchd::launch_data::LaunchData) };
}

// Dict operations
/// Forward to [`launchd::launch_data::dict_insert`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::dict_insert`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_dict_insert(
    dict: u64,
    value: u64,
    key: *const libc::c_char,
) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::dict_insert(
            dict as *mut launchd::launch_data::LaunchData,
            value as *mut launchd::launch_data::LaunchData,
            key,
        )
    }
}

/// Forward to [`launchd::launch_data::dict_lookup`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::dict_lookup`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_dict_lookup(dict: u64, key: *const libc::c_char) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::dict_lookup(dict as *mut launchd::launch_data::LaunchData, key) as u64
    }
}

/// Forward to [`launchd::launch_data::dict_remove`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::dict_remove`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_dict_remove(dict: u64, key: *const libc::c_char) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::dict_remove(dict as *mut launchd::launch_data::LaunchData, key) }
}

/// Forward to [`launchd::launch_data::dict_iterate`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::dict_iterate`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_dict_iterate(
    dict: u64,
    callback: launchd::launch_data::DictIterateCallback,
    context: *mut libc::c_void,
) {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::dict_iterate(
            dict as *mut launchd::launch_data::LaunchData,
            callback,
            context,
        )
    };
}

/// Forward to [`launchd::launch_data::dict_get_count`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::dict_get_count`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_dict_get_count(dict: u64) -> usize {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::dict_get_count(dict as *mut launchd::launch_data::LaunchData) }
}

// Array operations
/// Forward to [`launchd::launch_data::array_set_index`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::array_set_index`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_array_set_index(
    array: u64,
    value: u64,
    index: usize,
) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::array_set_index(
            array as *mut launchd::launch_data::LaunchData,
            value as *mut launchd::launch_data::LaunchData,
            index,
        )
    }
}

/// Forward to [`launchd::launch_data::array_get_index`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::array_get_index`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_array_get_index(array: u64, index: usize) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::array_get_index(array as *mut launchd::launch_data::LaunchData, index)
            as u64
    }
}

/// Forward to [`launchd::launch_data::array_get_count`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::array_get_count`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_array_get_count(array: u64) -> usize {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::array_get_count(array as *mut launchd::launch_data::LaunchData) }
}

// Value constructors
pub extern "C" fn shim_launch_data_new_fd(fd: i32) -> u64 {
    launchd::launch_data::new_fd(fd) as u64
}

pub extern "C" fn shim_launch_data_new_machport(port: u32) -> u64 {
    launchd::launch_data::new_machport(port) as u64
}

pub extern "C" fn shim_launch_data_new_integer(val: i64) -> u64 {
    launchd::launch_data::new_integer(val) as u64
}

pub extern "C" fn shim_launch_data_new_bool(val: bool) -> u64 {
    launchd::launch_data::new_bool(val) as u64
}

pub extern "C" fn shim_launch_data_new_real(val: f64) -> u64 {
    launchd::launch_data::new_real(val) as u64
}

/// Forward to [`launchd::launch_data::new_string`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::new_string`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_new_string(s: *const libc::c_char) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::new_string(s) as u64 }
}

/// Forward to [`launchd::launch_data::new_opaque`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::new_opaque`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_new_opaque(data: *const libc::c_void, len: usize) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::new_opaque(data, len) as u64 }
}

// Value setters
/// Forward to [`launchd::launch_data::set_fd`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_fd`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_fd(handle: u64, fd: i32) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::set_fd(handle as *mut launchd::launch_data::LaunchData, fd) }
}

/// Forward to [`launchd::launch_data::set_machport`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_machport`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_machport(handle: u64, port: u32) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::set_machport(handle as *mut launchd::launch_data::LaunchData, port)
    }
}

/// Forward to [`launchd::launch_data::set_integer`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_integer`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_integer(handle: u64, val: i64) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::set_integer(handle as *mut launchd::launch_data::LaunchData, val)
    }
}

/// Forward to [`launchd::launch_data::set_bool`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_bool`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_bool(handle: u64, val: bool) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::set_bool(handle as *mut launchd::launch_data::LaunchData, val) }
}

/// Forward to [`launchd::launch_data::set_real`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_real`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_real(handle: u64, val: f64) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::set_real(handle as *mut launchd::launch_data::LaunchData, val) }
}

/// Forward to [`launchd::launch_data::set_string`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_string`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_string(handle: u64, s: *const libc::c_char) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::set_string(handle as *mut launchd::launch_data::LaunchData, s) }
}

/// Forward to [`launchd::launch_data::set_opaque`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::set_opaque`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_set_opaque(
    handle: u64,
    data: *const libc::c_void,
    len: usize,
) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::set_opaque(handle as *mut launchd::launch_data::LaunchData, data, len)
    }
}

// Value getters
/// Forward to [`launchd::launch_data::get_fd`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_fd`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_fd(handle: u64) -> i32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_fd(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_machport`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_machport`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_machport(handle: u64) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_machport(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_integer`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_integer`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_integer(handle: u64) -> i64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_integer(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_bool`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_bool`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_bool(handle: u64) -> bool {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_bool(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_real`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_real`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_real(handle: u64) -> f64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_real(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_string`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_string`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_string(handle: u64) -> *const libc::c_char {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_string(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_opaque`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_opaque`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_opaque(handle: u64) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_opaque(handle as *mut launchd::launch_data::LaunchData) }
}

/// Forward to [`launchd::launch_data::get_opaque_size`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_opaque_size`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_opaque_size(handle: u64) -> usize {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_data::get_opaque_size(handle as *mut launchd::launch_data::LaunchData)
    }
}

/// Forward to [`launchd::launch_data::get_errno`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_data::get_errno`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_data_get_errno(handle: u64) -> i32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::launch_data::get_errno(handle as *mut launchd::launch_data::LaunchData) }
}

// launch_msg + launch_get_fd
/// Forward to [`launchd::launch_msg::launch_msg`].
///
/// # Safety
/// The caller must satisfy [`launchd::launch_msg::launch_msg`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_launch_msg(request: u64) -> u64 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::launch_msg::launch_msg(request as *mut launchd::launch_data::LaunchData) as u64
    }
}

pub extern "C" fn shim_launch_get_fd() -> i32 {
    launchd::launch_msg::launch_get_fd()
}

// ═══════════════════════════════════════════════════════════════════
//  bootstrap.h
// ═══════════════════════════════════════════════════════════════════

/// Forward to [`launchd::bootstrap::bootstrap_check_in`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_check_in`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_check_in(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_check_in(bp, name, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_look_up`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_look_up`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_look_up_launchd(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_look_up(bp, name, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_register`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_register`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_register(
    bp: u32,
    name: *const libc::c_char,
    sp: u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_register(bp, name, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_create_service`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_create_service`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_create_service(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_create_service(bp, name, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_create_server`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_create_server`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_create_server(
    bp: u32,
    cmd: *const libc::c_char,
    uid: u32,
    on_demand: bool,
    sp: *mut u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_create_server(bp, cmd, uid, on_demand, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_status`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_status`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_status(
    bp: u32,
    name: *const libc::c_char,
    status: *mut u32,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_status(bp, name, status) }
}

pub extern "C" fn shim_bootstrap_strerror(r: u32) -> *const libc::c_char {
    use launchd::constants::*;

    // The C API borrows static NUL-terminated storage, not a Rust str buffer.
    match r {
        BOOTSTRAP_SUCCESS => c"Success",
        BOOTSTRAP_NOT_PRIVILEGED => c"Not privileged",
        BOOTSTRAP_NAME_IN_USE => c"Name in use",
        BOOTSTRAP_UNKNOWN_SERVICE => c"Unknown service",
        BOOTSTRAP_SERVICE_ACTIVE => c"Service active",
        BOOTSTRAP_BAD_COUNT => c"Bad count",
        BOOTSTRAP_NO_MEMORY => c"No memory",
        BOOTSTRAP_NO_CHILDREN => c"No children",
        _ => c"Unknown bootstrap error",
    }
    .as_ptr()
}

/// Forward to [`launchd::bootstrap::bootstrap_subset`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_subset`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_subset(bp: u32, req: u32, sp: *mut u32) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_subset(bp, req, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_unprivileged`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_unprivileged`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_unprivileged(bp: u32, sp: *mut u32) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_unprivileged(bp, sp) }
}

/// Forward to [`launchd::bootstrap::bootstrap_parent`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_parent`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_parent(bp: u32, sp: *mut u32) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_parent(bp, sp) }
}

// bootstrap_priv.h
/// Forward to [`launchd::bootstrap::bootstrap_register2`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_register2`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_register2(
    bp: u32,
    name: *const libc::c_char,
    sp: u32,
    flags: u64,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_register2(bp, name, sp, flags) }
}

/// Forward to [`launchd::bootstrap::bootstrap_look_up2`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_look_up2`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_look_up2(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
    pid: i32,
    flags: u64,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_look_up2(bp, name, sp, pid, flags) }
}

/// Forward to [`launchd::bootstrap::bootstrap_check_in2`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_check_in2`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_check_in2(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
    flags: u64,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_check_in2(bp, name, sp, flags) }
}

/// Forward to [`launchd::bootstrap::bootstrap_look_up3`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_look_up3`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_look_up3(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
    pid: i32,
    iid: *const u8,
    flags: u64,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_look_up3(bp, name, sp, pid, iid, flags) }
}

/// Forward to [`launchd::bootstrap::bootstrap_check_in3`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_check_in3`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_check_in3(
    bp: u32,
    name: *const libc::c_char,
    sp: *mut u32,
    iid: *mut u8,
    flags: u64,
) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_check_in3(bp, name, sp, iid, flags) }
}

/// Forward to [`launchd::bootstrap::bootstrap_get_root`].
///
/// # Safety
/// The caller must satisfy [`launchd::bootstrap::bootstrap_get_root`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_bootstrap_get_root(bp: u32, root: *mut u32) -> u32 {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::bootstrap::bootstrap_get_root(bp, root) }
}

// ═══════════════════════════════════════════════════════════════════
//  vproc.h / vproc_priv.h
// ═══════════════════════════════════════════════════════════════════

pub extern "C" fn shim_vproc_strerror(err: *const libc::c_void) -> *const libc::c_char {
    // The opaque error is only compared with null, never dereferenced.
    if err.is_null() {
        c"Success".as_ptr()
    } else {
        c"vproc error".as_ptr()
    }
}

pub extern "C" fn shim_vproc_transaction_begin(vp: *mut libc::c_void) -> *mut libc::c_void {
    launchd::vproc::vproc_transaction_begin(vp)
}

pub extern "C" fn shim_vproc_transaction_end(vp: *mut libc::c_void, handle: *mut libc::c_void) {
    launchd::vproc::vproc_transaction_end(vp, handle);
}

pub extern "C" fn shim_vproc_standby_begin(vp: *mut libc::c_void) -> *mut libc::c_void {
    launchd::vproc::vproc_standby_begin(vp)
}

pub extern "C" fn shim_vproc_standby_end(vp: *mut libc::c_void, handle: *mut libc::c_void) {
    launchd::vproc::vproc_standby_end(vp, handle);
}

pub extern "C" fn shim_vprocmgr_lookup_vproc(label: *const libc::c_char) -> *mut libc::c_void {
    launchd::vproc::vprocmgr_lookup_vproc(label)
}

pub extern "C" fn shim_vproc_retain(vp: *mut libc::c_void) -> *mut libc::c_void {
    launchd::vproc::vproc_retain(vp)
}

pub extern "C" fn shim_vproc_release(vp: *mut libc::c_void) {
    launchd::vproc::vproc_release(vp);
}

/// Forward to [`launchd::vproc::vproc_swap_integer`].
///
/// # Safety
/// The caller must satisfy [`launchd::vproc::vproc_swap_integer`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_vproc_swap_integer(
    vp: *mut libc::c_void,
    key: u32,
    inval: *const i64,
    outval: *mut i64,
) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::vproc::vproc_swap_integer(vp, key, inval, outval) }
}

/// Forward to [`launchd::vproc::vproc_swap_complex`].
///
/// # Safety
/// The caller must satisfy [`launchd::vproc::vproc_swap_complex`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_vproc_swap_complex(
    vp: *mut libc::c_void,
    key: u32,
    inval: u64,
    outval: *mut u64,
) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe {
        launchd::vproc::vproc_swap_complex(
            vp,
            key,
            inval as *mut launchd::launch_data::LaunchData,
            outval as *mut *mut launchd::launch_data::LaunchData,
        )
    }
}

/// Forward to [`launchd::vproc::vproc_swap_string`].
///
/// # Safety
/// The caller must satisfy [`launchd::vproc::vproc_swap_string`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim_vproc_swap_string(
    vp: *mut libc::c_void,
    key: u32,
    instr: *const libc::c_char,
    outstr: *mut *mut libc::c_char,
) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::vproc::vproc_swap_string(vp, key, instr, outstr) }
}

/// Forward to [`launchd::vproc::_vproc_get_last_exit_status`].
///
/// # Safety
/// The caller must satisfy [`launchd::vproc::_vproc_get_last_exit_status`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim__vproc_get_last_exit_status(
    wstatus: *mut i32,
) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::vproc::_vproc_get_last_exit_status(wstatus) }
}

pub extern "C" fn shim__vproc_set_global_on_demand(val: bool) -> *const libc::c_void {
    launchd::vproc::_vproc_set_global_on_demand(val)
}

/// Forward to [`launchd::vproc::_vproc_send_signal_by_label`].
///
/// # Safety
/// The caller must satisfy [`launchd::vproc::_vproc_send_signal_by_label`]
/// for all pointers and ownership transfers. Integer handles represent the
/// same pointers and must satisfy those validity and aliasing requirements.
pub unsafe extern "C" fn shim__vproc_send_signal_by_label(
    label: *const libc::c_char,
    sig: i32,
) -> *const libc::c_void {
    // SAFETY: The caller upholds the target API contract; casts preserve the handles.
    unsafe { launchd::vproc::_vproc_send_signal_by_label(label, sig) }
}

pub extern "C" fn shim__vproc_transaction_begin() {
    launchd::vproc::_vproc_transaction_begin();
}

pub extern "C" fn shim__vproc_transaction_end() {
    launchd::vproc::_vproc_transaction_end();
}

pub extern "C" fn shim__vproc_transaction_count() -> usize {
    launchd::vproc::_vproc_transaction_count()
}

pub extern "C" fn shim__vproc_transactions_enable() {
    launchd::vproc::_vproc_transactions_enable();
}

pub extern "C" fn shim__vproc_standby_begin() {
    launchd::vproc::_vproc_standby_begin();
}

pub extern "C" fn shim__vproc_standby_end() {
    launchd::vproc::_vproc_standby_end();
}

pub extern "C" fn shim__vproc_standby_count() -> usize {
    launchd::vproc::_vproc_standby_count()
}

pub extern "C" fn shim__vproc_standby_timeout() -> usize {
    launchd::vproc::_vproc_standby_timeout()
}

pub extern "C" fn shim__vproc_pid_is_managed(pid: i32) -> bool {
    launchd::vproc::_vproc_pid_is_managed(pid)
}

pub extern "C" fn shim__vproc_transaction_try_exit(status: i32) {
    launchd::vproc::_vproc_transaction_try_exit(status);
}

// ═══════════════════════════════════════════════════════════════════
//  Registration
// ═══════════════════════════════════════════════════════════════════

pub struct LaunchdStubs;

impl LaunchdStubs {
    pub fn install(table: &mut ShimTable) {
        // Also install default bootstrap services
        launchd::install_defaults();

        crate::register_shims!(
            table,
            // ── launch_data_t (launch.h) ────────────────────────
            (
                "_launch_data_alloc",
                shim_launch_data_alloc as *const () as u64,
                "launch_data_alloc"
            ),
            (
                "_launch_data_copy",
                shim_launch_data_copy as *const () as u64,
                "launch_data_copy"
            ),
            (
                "_launch_data_get_type",
                shim_launch_data_get_type as *const () as u64,
                "launch_data_get_type"
            ),
            (
                "_launch_data_free",
                shim_launch_data_free as *const () as u64,
                "launch_data_free"
            ),
            // dict ops
            (
                "_launch_data_dict_insert",
                shim_launch_data_dict_insert as *const () as u64,
                "launch_data_dict_insert"
            ),
            (
                "_launch_data_dict_lookup",
                shim_launch_data_dict_lookup as *const () as u64,
                "launch_data_dict_lookup"
            ),
            (
                "_launch_data_dict_remove",
                shim_launch_data_dict_remove as *const () as u64,
                "launch_data_dict_remove"
            ),
            (
                "_launch_data_dict_iterate",
                shim_launch_data_dict_iterate as *const () as u64,
                "launch_data_dict_iterate"
            ),
            (
                "_launch_data_dict_get_count",
                shim_launch_data_dict_get_count as *const () as u64,
                "launch_data_dict_get_count"
            ),
            // array ops
            (
                "_launch_data_array_set_index",
                shim_launch_data_array_set_index as *const () as u64,
                "launch_data_array_set_index"
            ),
            (
                "_launch_data_array_get_index",
                shim_launch_data_array_get_index as *const () as u64,
                "launch_data_array_get_index"
            ),
            (
                "_launch_data_array_get_count",
                shim_launch_data_array_get_count as *const () as u64,
                "launch_data_array_get_count"
            ),
            // value constructors
            (
                "_launch_data_new_fd",
                shim_launch_data_new_fd as *const () as u64,
                "launch_data_new_fd"
            ),
            (
                "_launch_data_new_machport",
                shim_launch_data_new_machport as *const () as u64,
                "launch_data_new_machport"
            ),
            (
                "_launch_data_new_integer",
                shim_launch_data_new_integer as *const () as u64,
                "launch_data_new_integer"
            ),
            (
                "_launch_data_new_bool",
                shim_launch_data_new_bool as *const () as u64,
                "launch_data_new_bool"
            ),
            (
                "_launch_data_new_real",
                shim_launch_data_new_real as *const () as u64,
                "launch_data_new_real"
            ),
            (
                "_launch_data_new_string",
                shim_launch_data_new_string as *const () as u64,
                "launch_data_new_string"
            ),
            (
                "_launch_data_new_opaque",
                shim_launch_data_new_opaque as *const () as u64,
                "launch_data_new_opaque"
            ),
            // value setters
            (
                "_launch_data_set_fd",
                shim_launch_data_set_fd as *const () as u64,
                "launch_data_set_fd"
            ),
            (
                "_launch_data_set_machport",
                shim_launch_data_set_machport as *const () as u64,
                "launch_data_set_machport"
            ),
            (
                "_launch_data_set_integer",
                shim_launch_data_set_integer as *const () as u64,
                "launch_data_set_integer"
            ),
            (
                "_launch_data_set_bool",
                shim_launch_data_set_bool as *const () as u64,
                "launch_data_set_bool"
            ),
            (
                "_launch_data_set_real",
                shim_launch_data_set_real as *const () as u64,
                "launch_data_set_real"
            ),
            (
                "_launch_data_set_string",
                shim_launch_data_set_string as *const () as u64,
                "launch_data_set_string"
            ),
            (
                "_launch_data_set_opaque",
                shim_launch_data_set_opaque as *const () as u64,
                "launch_data_set_opaque"
            ),
            // value getters
            (
                "_launch_data_get_fd",
                shim_launch_data_get_fd as *const () as u64,
                "launch_data_get_fd"
            ),
            (
                "_launch_data_get_machport",
                shim_launch_data_get_machport as *const () as u64,
                "launch_data_get_machport"
            ),
            (
                "_launch_data_get_integer",
                shim_launch_data_get_integer as *const () as u64,
                "launch_data_get_integer"
            ),
            (
                "_launch_data_get_bool",
                shim_launch_data_get_bool as *const () as u64,
                "launch_data_get_bool"
            ),
            (
                "_launch_data_get_real",
                shim_launch_data_get_real as *const () as u64,
                "launch_data_get_real"
            ),
            (
                "_launch_data_get_string",
                shim_launch_data_get_string as *const () as u64,
                "launch_data_get_string"
            ),
            (
                "_launch_data_get_opaque",
                shim_launch_data_get_opaque as *const () as u64,
                "launch_data_get_opaque"
            ),
            (
                "_launch_data_get_opaque_size",
                shim_launch_data_get_opaque_size as *const () as u64,
                "launch_data_get_opaque_size"
            ),
            (
                "_launch_data_get_errno",
                shim_launch_data_get_errno as *const () as u64,
                "launch_data_get_errno"
            ),
            // launch_msg + launch_get_fd
            (
                "_launch_msg",
                shim_launch_msg as *const () as u64,
                "launch_msg (in-process job registry)"
            ),
            (
                "_launch_get_fd",
                shim_launch_get_fd as *const () as u64,
                "launch_get_fd (-1, no IPC socket)"
            ),
            // ── bootstrap.h ─────────────────────────────────────
            (
                "_bootstrap_check_in",
                shim_bootstrap_check_in as *const () as u64,
                "bootstrap_check_in (launchd)"
            ),
            (
                "_bootstrap_look_up",
                shim_bootstrap_look_up_launchd as *const () as u64,
                "bootstrap_look_up (launchd)"
            ),
            (
                "_bootstrap_register",
                shim_bootstrap_register as *const () as u64,
                "bootstrap_register (launchd)"
            ),
            (
                "_bootstrap_create_service",
                shim_bootstrap_create_service as *const () as u64,
                "bootstrap_create_service (launchd)"
            ),
            (
                "_bootstrap_create_server",
                shim_bootstrap_create_server as *const () as u64,
                "bootstrap_create_server (launchd)"
            ),
            (
                "_bootstrap_status",
                shim_bootstrap_status as *const () as u64,
                "bootstrap_status (launchd)"
            ),
            (
                "_bootstrap_strerror",
                shim_bootstrap_strerror as *const () as u64,
                "bootstrap_strerror (launchd)"
            ),
            (
                "_bootstrap_subset",
                shim_bootstrap_subset as *const () as u64,
                "bootstrap_subset (launchd)"
            ),
            (
                "_bootstrap_unprivileged",
                shim_bootstrap_unprivileged as *const () as u64,
                "bootstrap_unprivileged (launchd)"
            ),
            (
                "_bootstrap_parent",
                shim_bootstrap_parent as *const () as u64,
                "bootstrap_parent (launchd)"
            ),
            // bootstrap_priv.h
            (
                "_bootstrap_register2",
                shim_bootstrap_register2 as *const () as u64,
                "bootstrap_register2 (launchd)"
            ),
            (
                "_bootstrap_look_up2",
                shim_bootstrap_look_up2 as *const () as u64,
                "bootstrap_look_up2 (launchd)"
            ),
            (
                "_bootstrap_check_in2",
                shim_bootstrap_check_in2 as *const () as u64,
                "bootstrap_check_in2 (launchd)"
            ),
            (
                "_bootstrap_look_up3",
                shim_bootstrap_look_up3 as *const () as u64,
                "bootstrap_look_up3 (launchd)"
            ),
            (
                "_bootstrap_check_in3",
                shim_bootstrap_check_in3 as *const () as u64,
                "bootstrap_check_in3 (launchd)"
            ),
            (
                "_bootstrap_get_root",
                shim_bootstrap_get_root as *const () as u64,
                "bootstrap_get_root (launchd)"
            ),
            // ── vproc.h ─────────────────────────────────────────
            (
                "_vproc_strerror",
                shim_vproc_strerror as *const () as u64,
                "vproc_strerror"
            ),
            (
                "_vproc_transaction_begin",
                shim_vproc_transaction_begin as *const () as u64,
                "vproc_transaction_begin"
            ),
            (
                "_vproc_transaction_end",
                shim_vproc_transaction_end as *const () as u64,
                "vproc_transaction_end"
            ),
            (
                "_vproc_standby_begin",
                shim_vproc_standby_begin as *const () as u64,
                "vproc_standby_begin"
            ),
            (
                "_vproc_standby_end",
                shim_vproc_standby_end as *const () as u64,
                "vproc_standby_end"
            ),
            (
                "_vprocmgr_lookup_vproc",
                shim_vprocmgr_lookup_vproc as *const () as u64,
                "vprocmgr_lookup_vproc"
            ),
            (
                "_vproc_retain",
                shim_vproc_retain as *const () as u64,
                "vproc_retain"
            ),
            (
                "_vproc_release",
                shim_vproc_release as *const () as u64,
                "vproc_release"
            ),
            // ── vproc_priv.h ────────────────────────────────────
            (
                "_vproc_swap_integer",
                shim_vproc_swap_integer as *const () as u64,
                "vproc_swap_integer"
            ),
            (
                "_vproc_swap_complex",
                shim_vproc_swap_complex as *const () as u64,
                "vproc_swap_complex"
            ),
            (
                "_vproc_swap_string",
                shim_vproc_swap_string as *const () as u64,
                "vproc_swap_string"
            ),
            (
                "__vproc_get_last_exit_status",
                shim__vproc_get_last_exit_status as *const () as u64,
                "_vproc_get_last_exit_status"
            ),
            (
                "__vproc_set_global_on_demand",
                shim__vproc_set_global_on_demand as *const () as u64,
                "_vproc_set_global_on_demand"
            ),
            (
                "__vproc_send_signal_by_label",
                shim__vproc_send_signal_by_label as *const () as u64,
                "_vproc_send_signal_by_label"
            ),
            (
                "__vproc_transaction_begin",
                shim__vproc_transaction_begin as *const () as u64,
                "_vproc_transaction_begin"
            ),
            (
                "__vproc_transaction_end",
                shim__vproc_transaction_end as *const () as u64,
                "_vproc_transaction_end"
            ),
            (
                "__vproc_transaction_count",
                shim__vproc_transaction_count as *const () as u64,
                "_vproc_transaction_count"
            ),
            (
                "__vproc_transactions_enable",
                shim__vproc_transactions_enable as *const () as u64,
                "_vproc_transactions_enable"
            ),
            (
                "__vproc_standby_begin",
                shim__vproc_standby_begin as *const () as u64,
                "_vproc_standby_begin"
            ),
            (
                "__vproc_standby_end",
                shim__vproc_standby_end as *const () as u64,
                "_vproc_standby_end"
            ),
            (
                "__vproc_standby_count",
                shim__vproc_standby_count as *const () as u64,
                "_vproc_standby_count"
            ),
            (
                "__vproc_standby_timeout",
                shim__vproc_standby_timeout as *const () as u64,
                "_vproc_standby_timeout"
            ),
            (
                "__vproc_pid_is_managed",
                shim__vproc_pid_is_managed as *const () as u64,
                "_vproc_pid_is_managed"
            ),
            (
                "__vproc_transaction_try_exit",
                shim__vproc_transaction_try_exit as *const () as u64,
                "_vproc_transaction_try_exit"
            ),
            // ── data symbols ────────────────────────────────────
            (
                "_bootstrap_port",
                launchd::bootstrap::bootstrap_port_ptr() as u64,
                "bootstrap_port data symbol (mach_port_t)"
            )
        );

        log::debug!("launchd_stubs: registered all launch/bootstrap/vproc shims");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use launchd::constants::*;
    use std::ffi::CStr;

    #[test]
    fn bootstrap_error_strings_are_terminated_and_preserve_messages() {
        for code in [
            BOOTSTRAP_SUCCESS,
            BOOTSTRAP_NOT_PRIVILEGED,
            BOOTSTRAP_NAME_IN_USE,
            BOOTSTRAP_UNKNOWN_SERVICE,
            BOOTSTRAP_SERVICE_ACTIVE,
            BOOTSTRAP_BAD_COUNT,
            BOOTSTRAP_NO_MEMORY,
            BOOTSTRAP_NO_CHILDREN,
            u32::MAX,
        ] {
            let pointer = shim_bootstrap_strerror(code);
            assert!(!pointer.is_null());
            // SAFETY: The wrapper returns a borrowed static C string for every code.
            let message = unsafe { CStr::from_ptr(pointer) };
            assert_eq!(
                message.to_bytes(),
                launchd::bootstrap::bootstrap_strerror(code).as_bytes()
            );
        }
    }

    #[test]
    fn vproc_error_strings_are_terminated_and_preserve_messages() {
        let sentinel = 1_u8;
        let error = (&sentinel as *const u8).cast::<libc::c_void>();
        for code in [std::ptr::null(), error] {
            let pointer = shim_vproc_strerror(code);
            assert!(!pointer.is_null());
            // SAFETY: Both error cases return borrowed static C strings.
            let message = unsafe { CStr::from_ptr(pointer) };
            assert_eq!(
                message.to_bytes(),
                launchd::vproc::vproc_strerror(code).as_bytes()
            );
        }
    }
}
