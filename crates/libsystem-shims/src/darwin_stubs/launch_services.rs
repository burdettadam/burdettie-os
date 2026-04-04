//! LaunchServices framework stubs.
//!
//! macOS app/URL/file-type registration API. All functions return
//! error codes (kLSApplicationNotFoundErr = -10814) or NULL.

use crate::table::ShimTable;

/// kLSApplicationNotFoundErr
const LS_APP_NOT_FOUND: i32 = -10814;

pub struct LaunchServicesStubs;

impl LaunchServicesStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_LSCopyDefaultApplicationURLForURL", shim_ls_null_stub as *const () as u64, "LSCopyDefaultApplicationURLForURL (stub → NULL)"),
            ("_LSCopyDefaultApplicationURLForContentType", shim_ls_null_stub as *const () as u64, "LSCopyDefaultApplicationURLForContentType (stub → NULL)"),
            ("_LSCopyApplicationURLsForBundleIdentifier", shim_ls_null_stub as *const () as u64, "LSCopyApplicationURLsForBundleIdentifier (stub → NULL)"),
            ("_LSCopyApplicationURLsForURL", shim_ls_null_stub as *const () as u64, "LSCopyApplicationURLsForURL (stub → NULL)"),
            ("_LSCanURLAcceptURL", shim_ls_false_stub as *const () as u64, "LSCanURLAcceptURL (stub → false)"),
            ("_LSOpenCFURLRef", shim_ls_err_stub as *const () as u64, "LSOpenCFURLRef (stub → error)"),
            ("_LSOpenFromURLSpec", shim_ls_err_stub as *const () as u64, "LSOpenFromURLSpec (stub → error)"),
            ("_LSRegisterURL", shim_ls_err_stub as *const () as u64, "LSRegisterURL (stub → error)"),
            ("_LSGetApplicationForInfo", shim_ls_err_stub as *const () as u64, "LSGetApplicationForInfo (stub → error)"),
            ("_LSGetApplicationForURL", shim_ls_err_stub as *const () as u64, "LSGetApplicationForURL (stub → error)"),
            ("_LSCopyItemInfoForURL", shim_ls_err_stub as *const () as u64, "LSCopyItemInfoForURL (stub → error)"),
            ("_LSCopyDisplayNameForURL", shim_ls_null_stub as *const () as u64, "LSCopyDisplayNameForURL (stub → NULL)"),
            ("_LSCopyKindStringForURL", shim_ls_null_stub as *const () as u64, "LSCopyKindStringForURL (stub → NULL)"),
            ("_LSCopyAllRoleHandlersForContentType", shim_ls_null_stub as *const () as u64, "LSCopyAllRoleHandlersForContentType (stub → NULL)"),
            ("_LSSetDefaultRoleHandlerForContentType", shim_ls_err_stub as *const () as u64, "LSSetDefaultRoleHandlerForContentType (stub → error)"),
            ("_LSCopyDefaultHandlerForURLScheme", shim_ls_null_stub as *const () as u64, "LSCopyDefaultHandlerForURLScheme (stub → NULL)"),
            ("_LSSetDefaultHandlerForURLScheme", shim_ls_err_stub as *const () as u64, "LSSetDefaultHandlerForURLScheme (stub → error)"),
            ("__LSCopyApplicationArrayInFrontToBackOrder", shim_ls_null_stub as *const () as u64, "_LSCopyApplicationArrayInFrontToBackOrder (stub → NULL)"),
        );
        log::debug!("LaunchServicesStubs: registered 18 stubs");
    }
}

pub extern "C" fn shim_ls_null_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> u64 { 0 }
pub extern "C" fn shim_ls_false_stub(_arg0: u64, _arg1: u64, _arg2: u64, _arg3: u64) -> u8 { 0 }
pub extern "C" fn shim_ls_err_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> i32 { LS_APP_NOT_FOUND }
