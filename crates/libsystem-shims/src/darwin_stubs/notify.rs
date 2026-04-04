//! macOS notify(3) stubs.
//!
//! Inter-process notification API. Stubs return success codes
//! but do not actually register or deliver notifications.

use crate::table::ShimTable;

/// NOTIFY_STATUS_OK
const NOTIFY_STATUS_OK: u32 = 0;

pub struct NotifyStubs;

impl NotifyStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_notify_register_dispatch", shim_notify_register_dispatch as *const () as u64, "notify_register_dispatch (stub → OK)"),
            ("_notify_register_check", shim_notify_register_check as *const () as u64, "notify_register_check (stub → OK)"),
            ("_notify_get_state", shim_notify_get_state as *const () as u64, "notify_get_state (stub → OK, state=0)"),
            ("_notify_set_state", shim_notify_set_state as *const () as u64, "notify_set_state (stub → OK)"),
            ("_notify_post", shim_notify_post as *const () as u64, "notify_post (stub → OK)"),
            ("_notify_cancel", shim_notify_cancel as *const () as u64, "notify_cancel (stub → OK)"),
        );
        log::debug!("NotifyStubs: registered 6 stubs");
    }
}

pub extern "C" fn shim_notify_register_dispatch(
    _name: u64, _out_token: *mut i32, _queue: u64, _handler: u64,
) -> u32 {
    if !_out_token.is_null() {
        unsafe { *_out_token = 0; }
    }
    NOTIFY_STATUS_OK
}

pub extern "C" fn shim_notify_get_state(_token: i32, _state: *mut u64) -> u32 {
    if !_state.is_null() {
        unsafe { *_state = 0; }
    }
    NOTIFY_STATUS_OK
}

pub extern "C" fn shim_notify_cancel(_token: i32) -> u32 { NOTIFY_STATUS_OK }

/// `uint32_t notify_register_check(const char *name, int *out_token)`
pub extern "C" fn shim_notify_register_check(
    _name: u64, out_token: *mut i32,
) -> u32 {
    if !out_token.is_null() {
        unsafe { *out_token = 0; }
    }
    NOTIFY_STATUS_OK
}

/// `uint32_t notify_set_state(int token, uint64_t state)`
pub extern "C" fn shim_notify_set_state(_token: i32, _state: u64) -> u32 {
    NOTIFY_STATUS_OK
}

/// `uint32_t notify_post(const char *name)`
pub extern "C" fn shim_notify_post(_name: u64) -> u32 {
    NOTIFY_STATUS_OK
}
