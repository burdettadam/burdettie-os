//! Extra GCD dispatch stubs.
//!
//! Supplements the core dispatch shims in `crate::dispatch` with
//! additional primitives used by Xcode tools.

use crate::table::ShimTable;

pub struct DispatchExtrasStubs;

impl DispatchExtrasStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_dispatch_activate", shim_dispatch_void1 as *const () as u64, "dispatch_activate (stub → void)"),
            ("_dispatch_assert_queue", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue (stub → void)"),
            ("_dispatch_assert_queue_barrier", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue_barrier (stub → void)"),
            ("_dispatch_assert_queue_not", shim_dispatch_void1 as *const () as u64, "dispatch_assert_queue_not (stub → void)"),
            ("_dispatch_block_create", shim_dispatch_block_create as *const () as u64, "dispatch_block_create (stub → passthrough block)"),
            ("_dispatch_block_create_with_qos_class", shim_dispatch_block_create_qos as *const () as u64, "dispatch_block_create_with_qos_class (stub → passthrough)"),
            ("_dispatch_block_cancel", shim_dispatch_void1 as *const () as u64, "dispatch_block_cancel (stub → void)"),
            ("_dispatch_block_testcancel", shim_dispatch_zero1 as *const () as u64, "dispatch_block_testcancel (stub → 0)"),
            ("_dispatch_block_wait", shim_dispatch_zero2 as *const () as u64, "dispatch_block_wait (stub → 0)"),
            ("_dispatch_block_notify", shim_dispatch_void3 as *const () as u64, "dispatch_block_notify (stub → void)"),
            ("_dispatch_block_perform", shim_dispatch_void2 as *const () as u64, "dispatch_block_perform (stub → void)"),
            ("_dispatch_io_create", shim_dispatch_null3 as *const () as u64, "dispatch_io_create (stub → NULL)"),
            ("_dispatch_io_read", shim_dispatch_void4 as *const () as u64, "dispatch_io_read (stub → void)"),
            ("_dispatch_io_write", shim_dispatch_void4 as *const () as u64, "dispatch_io_write (stub → void)"),
            ("_dispatch_io_close", shim_dispatch_void2 as *const () as u64, "dispatch_io_close (stub → void)"),
            ("_dispatch_io_set_high_water", shim_dispatch_void2 as *const () as u64, "dispatch_io_set_high_water (stub → void)"),
            ("_dispatch_io_set_low_water", shim_dispatch_void2 as *const () as u64, "dispatch_io_set_low_water (stub → void)"),
            ("_dispatch_queue_attr_make_with_autorelease_frequency", shim_dispatch_first_arg as *const () as u64, "dispatch_queue_attr_make_with_autorelease_frequency (stub → attr passthrough)"),
            ("_dispatch_queue_get_specific", shim_dispatch_null1 as *const () as u64, "dispatch_queue_get_specific (stub → NULL)"),
            ("_dispatch_source_get_data", shim_dispatch_zero1 as *const () as u64, "dispatch_source_get_data (stub → 0)"),
            ("_dispatch_source_get_handle", shim_dispatch_zero1 as *const () as u64, "dispatch_source_get_handle (stub → 0)"),
            ("_dispatch_source_merge_data", shim_dispatch_void2 as *const () as u64, "dispatch_source_merge_data (stub → void)"),
        );
        log::debug!("DispatchExtrasStubs: registered 22 stubs");
    }
}

pub extern "C" fn shim_dispatch_void1(_a: u64) {}
pub extern "C" fn shim_dispatch_void2(_a: u64, _b: u64) {}
pub extern "C" fn shim_dispatch_void3(_a: u64, _b: u64, _c: u64) {}
pub extern "C" fn shim_dispatch_void4(_a: u64, _b: u64, _c: u64, _d: u64) {}
pub extern "C" fn shim_dispatch_zero1(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_zero2(_a: u64, _b: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_null1(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_null3(_a: u64, _b: u64, _c: u64) -> u64 { 0 }
pub extern "C" fn shim_dispatch_block_create(_flags: u64, _block: u64) -> u64 { _block }
pub extern "C" fn shim_dispatch_block_create_qos(_flags: u64, _qos: u64, _rel: i32, _block: u64) -> u64 { _block }
pub extern "C" fn shim_dispatch_first_arg(a: u64, _b: u64) -> u64 { a }
