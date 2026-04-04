//! Apple System Logger (ASL) stubs.
//!
//! Legacy macOS logging API. Stubs provide minimal handle management
//! and route asl_send output to stderr via log::debug!.

use crate::table::ShimTable;

pub struct AslStubs;

/// Synthetic sentinel values for ASL handles and messages.
static ASL_HANDLE_SENTINEL: u64 = 0xA510_0001;
static ASL_MSG_SENTINEL: u64 = 0xA510_0002;

impl AslStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_asl_open", shim_asl_open as *const () as u64, "asl_open (stub → synthetic handle)"),
            ("_asl_new", shim_asl_new as *const () as u64, "asl_new (stub → synthetic msg)"),
            ("_asl_set", shim_asl_set as *const () as u64, "asl_set (stub → 0)"),
            ("_asl_send", shim_asl_send as *const () as u64, "asl_send (stub → 0, logs debug)"),
            ("_asl_free", shim_asl_free as *const () as u64, "asl_free (stub → void)"),
            ("_asl_format", shim_asl_format as *const () as u64, "asl_format (stub → NULL)"),
        );
        log::debug!("AslStubs: registered 6 stubs");
    }
}

pub extern "C" fn shim_asl_open(_ident: u64, _facility: u64, _opts: u32) -> u64 {
    &ASL_HANDLE_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_asl_new(_type: u32) -> u64 {
    &ASL_MSG_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_asl_set(_msg: u64, _key: u64, _val: u64) -> i32 { 0 }

pub extern "C" fn shim_asl_send(_client: u64, _msg: u64) -> i32 {
    log::debug!("asl_send: message logged (stub)");
    0
}

pub extern "C" fn shim_asl_free(_obj: u64) {}

pub extern "C" fn shim_asl_format(_msg: u64, _fmt: u64, _time_fmt: u32) -> u64 { 0 }
