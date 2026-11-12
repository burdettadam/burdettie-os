//! Raw FFI bindings to Apple's libdispatch (Grand Central Dispatch).
//!
//! This crate links against the system's libdispatch and exposes the C API
//! as `extern "C"` functions. On macOS, libdispatch is part of libSystem.
//! On Linux, it's provided by the Swift toolchain or `libdispatch-dev`.
//!
//! These bindings cover the subset of GCD used by Xcode CLI tools.
//! For the full API, see Apple's dispatch(3) man page.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_long, c_ulong, c_void};

// ── Opaque types ────────────────────────────────────────────────

/// Opaque dispatch object (base type for all dispatch objects).
pub enum dispatch_object_s {}
pub type dispatch_object_t = *mut dispatch_object_s;

/// Dispatch queue.
#[repr(C)]
pub struct dispatch_queue_s {
    _opaque: [u8; 0],
}
pub type dispatch_queue_t = *mut dispatch_queue_s;

/// Dispatch queue attributes.
pub enum dispatch_queue_attr_s {}
pub type dispatch_queue_attr_t = *const dispatch_queue_attr_s;

/// Dispatch group.
pub enum dispatch_group_s {}
pub type dispatch_group_t = *mut dispatch_group_s;

/// Dispatch semaphore.
pub enum dispatch_semaphore_s {}
pub type dispatch_semaphore_t = *mut dispatch_semaphore_s;

/// Dispatch source.
pub enum dispatch_source_s {}
pub type dispatch_source_t = *mut dispatch_source_s;

/// Dispatch source type.
#[repr(C)]
pub struct dispatch_source_type_s {
    _opaque: [u8; 0],
}
pub type dispatch_source_type_t = *const dispatch_source_type_s;

/// Dispatch data.
pub enum dispatch_data_s {}
pub type dispatch_data_t = *mut dispatch_data_s;

/// Dispatch I/O channel.
pub enum dispatch_io_s {}
pub type dispatch_io_t = *mut dispatch_io_s;

/// Block type used by dispatch functions.
pub type dispatch_block_t = *const c_void;

/// Function type for dispatch_*_f variants.
pub type dispatch_function_t = Option<unsafe extern "C" fn(*mut c_void)>;

/// dispatch_once predicate.
pub type dispatch_once_t = c_long;

/// dispatch_time_t.
pub type dispatch_time_t = u64;

/// Time constants.
pub const DISPATCH_TIME_NOW: dispatch_time_t = 0;
pub const DISPATCH_TIME_FOREVER: dispatch_time_t = !0;

/// Queue priority constants.
pub const DISPATCH_QUEUE_PRIORITY_HIGH: c_long = 2;
pub const DISPATCH_QUEUE_PRIORITY_DEFAULT: c_long = 0;
pub const DISPATCH_QUEUE_PRIORITY_LOW: c_long = -2;
pub const DISPATCH_QUEUE_PRIORITY_BACKGROUND: c_long = i16::MIN as c_long;

extern "C" {
    // ── Queue Management ────────────────────────────────────────

    pub fn dispatch_queue_create(
        label: *const c_char,
        attr: dispatch_queue_attr_t,
    ) -> dispatch_queue_t;

    pub fn dispatch_get_global_queue(identifier: c_long, flags: c_ulong) -> dispatch_queue_t;

    /// `dispatch_get_main_queue()` is a macro on macOS expanding to
    /// `&_dispatch_main_q`. We expose the actual symbol.
    pub static _dispatch_main_q: dispatch_queue_s;

    pub fn dispatch_queue_get_label(queue: dispatch_queue_t) -> *const c_char;

    pub fn dispatch_set_target_queue(object: dispatch_object_t, queue: dispatch_queue_t);

    // ── Async / Sync ────────────────────────────────────────────

    pub fn dispatch_async(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_sync(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_async_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_sync_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_barrier_async(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_barrier_sync(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_after(when: dispatch_time_t, queue: dispatch_queue_t, block: dispatch_block_t);

    // ── Once ────────────────────────────────────────────────────

    pub fn dispatch_once(predicate: *mut dispatch_once_t, block: dispatch_block_t);
    pub fn dispatch_once_f(
        predicate: *mut dispatch_once_t,
        context: *mut c_void,
        function: dispatch_function_t,
    );

    // ── Groups ──────────────────────────────────────────────────

    pub fn dispatch_group_create() -> dispatch_group_t;
    pub fn dispatch_group_async(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        block: dispatch_block_t,
    );
    pub fn dispatch_group_wait(group: dispatch_group_t, timeout: dispatch_time_t) -> c_long;
    pub fn dispatch_group_notify(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        block: dispatch_block_t,
    );
    pub fn dispatch_group_enter(group: dispatch_group_t);
    pub fn dispatch_group_leave(group: dispatch_group_t);

    // ── Semaphores ──────────────────────────────────────────────

    pub fn dispatch_semaphore_create(value: c_long) -> dispatch_semaphore_t;
    pub fn dispatch_semaphore_wait(dsema: dispatch_semaphore_t, timeout: dispatch_time_t)
        -> c_long;
    pub fn dispatch_semaphore_signal(dsema: dispatch_semaphore_t) -> c_long;

    // ── Time ────────────────────────────────────────────────────

    pub fn dispatch_time(when: dispatch_time_t, delta: i64) -> dispatch_time_t;
    pub fn dispatch_walltime(when: *const libc::timespec, delta: i64) -> dispatch_time_t;

    // ── Object lifecycle ────────────────────────────────────────

    pub fn dispatch_retain(object: dispatch_object_t);
    pub fn dispatch_release(object: dispatch_object_t);
    pub fn dispatch_activate(object: dispatch_object_t);
    pub fn dispatch_suspend(object: dispatch_object_t);
    pub fn dispatch_resume(object: dispatch_object_t);

    // ── Sources ─────────────────────────────────────────────────

    pub fn dispatch_source_create(
        type_: dispatch_source_type_t,
        handle: usize,
        mask: c_ulong,
        queue: dispatch_queue_t,
    ) -> dispatch_source_t;
    pub fn dispatch_source_set_event_handler(source: dispatch_source_t, handler: dispatch_block_t);
    pub fn dispatch_source_set_cancel_handler(source: dispatch_source_t, handler: dispatch_block_t);
    pub fn dispatch_source_cancel(source: dispatch_source_t);
    pub fn dispatch_source_get_data(source: dispatch_source_t) -> c_ulong;
    pub fn dispatch_source_get_handle(source: dispatch_source_t) -> usize;
    pub fn dispatch_source_merge_data(source: dispatch_source_t, value: c_ulong);

    // ── Source type constants ────────────────────────────────────

    pub static _dispatch_source_type_data_add: dispatch_source_type_s;
    pub static _dispatch_source_type_data_or: dispatch_source_type_s;
    pub static _dispatch_source_type_data_replace: dispatch_source_type_s;
    pub static _dispatch_source_type_mach_send: dispatch_source_type_s;
    pub static _dispatch_source_type_memorypressure: dispatch_source_type_s;
    pub static _dispatch_source_type_timer: dispatch_source_type_s;
    pub static _dispatch_source_type_read: dispatch_source_type_s;
    pub static _dispatch_source_type_write: dispatch_source_type_s;
    pub static _dispatch_source_type_proc: dispatch_source_type_s;
    pub static _dispatch_source_type_signal: dispatch_source_type_s;
    pub static _dispatch_source_type_vnode: dispatch_source_type_s;
    pub static _dispatch_source_type_mach_recv: dispatch_source_type_s;

    // ── Dispatch I/O ────────────────────────────────────────────

    pub fn dispatch_read(
        fd: i32,
        length: usize,
        queue: dispatch_queue_t,
        handler: dispatch_block_t,
    );
    pub fn dispatch_write(
        fd: i32,
        data: dispatch_data_t,
        queue: dispatch_queue_t,
        handler: dispatch_block_t,
    );

    // ── Queue-specific data ─────────────────────────────────────

    pub fn dispatch_queue_set_specific(
        queue: dispatch_queue_t,
        key: *const c_void,
        context: *mut c_void,
        destructor: Option<unsafe extern "C" fn(*mut c_void)>,
    );
    pub fn dispatch_queue_get_specific(queue: dispatch_queue_t, key: *const c_void) -> *mut c_void;
    pub fn dispatch_get_specific(key: *const c_void) -> *mut c_void;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_main_queue() {
        unsafe {
            let q = &_dispatch_main_q as *const _ as *mut dispatch_queue_s;
            assert!(!q.is_null());
        }
    }

    #[test]
    fn can_create_semaphore() {
        unsafe {
            let sem = dispatch_semaphore_create(0);
            assert!(!sem.is_null());
            dispatch_semaphore_signal(sem);
            let result = dispatch_semaphore_wait(sem, DISPATCH_TIME_NOW);
            assert_eq!(result, 0);
            dispatch_release(sem as dispatch_object_t);
        }
    }

    #[test]
    fn can_create_group() {
        unsafe {
            let group = dispatch_group_create();
            assert!(!group.is_null());
            let result = dispatch_group_wait(group, DISPATCH_TIME_NOW);
            assert_eq!(result, 0); // empty group returns immediately
            dispatch_release(group as dispatch_object_t);
        }
    }
}
