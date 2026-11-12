//! DiskArbitration framework stubs.
//!
//! macOS-only disk management API. All functions return NULL/0 since
//! there is no disk arbitration subsystem on Linux.

use crate::table::ShimTable;

pub struct DiskArbitrationStubs;

impl DiskArbitrationStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_DASessionCreate",
                shim_da_session_create as *const () as u64,
                "DASessionCreate (stub → NULL)"
            ),
            (
                "_DASessionSetDispatchQueue",
                shim_da_void_stub as *const () as u64,
                "DASessionSetDispatchQueue (stub → void)"
            ),
            (
                "_DADiskCreateFromBSDName",
                shim_da_null_stub as *const () as u64,
                "DADiskCreateFromBSDName (stub → NULL)"
            ),
            (
                "_DADiskCreateFromIOMedia",
                shim_da_null_stub as *const () as u64,
                "DADiskCreateFromIOMedia (stub → NULL)"
            ),
            (
                "_DADiskCopyDescription",
                shim_da_null_stub as *const () as u64,
                "DADiskCopyDescription (stub → NULL)"
            ),
            (
                "_DADiskCopyWholeDisk",
                shim_da_null_stub as *const () as u64,
                "DADiskCopyWholeDisk (stub → NULL)"
            ),
            (
                "_DADiskGetBSDName",
                shim_da_null_stub as *const () as u64,
                "DADiskGetBSDName (stub → NULL)"
            ),
            (
                "_DADiskCopyIOMedia",
                shim_da_null_stub as *const () as u64,
                "DADiskCopyIOMedia (stub → NULL)"
            ),
            (
                "_DARegisterDiskAppearedCallback",
                shim_da_void_stub as *const () as u64,
                "DARegisterDiskAppearedCallback (stub → void)"
            ),
            (
                "_DARegisterDiskDisappearedCallback",
                shim_da_void_stub as *const () as u64,
                "DARegisterDiskDisappearedCallback (stub → void)"
            ),
            (
                "_DARegisterDiskDescriptionChangedCallback",
                shim_da_void_stub as *const () as u64,
                "DARegisterDiskDescriptionChangedCallback (stub → void)"
            ),
            (
                "_DAUnregisterCallback",
                shim_da_void_stub as *const () as u64,
                "DAUnregisterCallback (stub → void)"
            ),
            (
                "_DADiskGetOptions",
                shim_da_zero_stub as *const () as u64,
                "DADiskGetOptions (stub → 0)"
            ),
            (
                "_DADiskSetOptions",
                shim_da_zero_stub as *const () as u64,
                "DADiskSetOptions (stub → 0)"
            ),
            (
                "_DASessionScheduleWithRunLoop",
                shim_da_void_stub as *const () as u64,
                "DASessionScheduleWithRunLoop (stub → void)"
            ),
        );
        log::debug!("DiskArbitrationStubs: registered 15 stubs");
    }
}

pub extern "C" fn shim_da_session_create(_alloc: u64) -> u64 {
    0
}
pub extern "C" fn shim_da_null_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> u64 {
    0
}
pub extern "C" fn shim_da_void_stub(_arg0: u64, _arg1: u64, _arg2: u64) {}
pub extern "C" fn shim_da_zero_stub(_arg0: u64, _arg1: u64) -> u64 {
    0
}
