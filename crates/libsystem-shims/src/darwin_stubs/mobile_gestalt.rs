//! MobileGestalt stubs.
//!
//! Private Apple API for querying device characteristics.
//! All queries return NULL/0/false on non-macOS.

use crate::table::ShimTable;

pub struct MobileGestaltStubs;

impl MobileGestaltStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_MGCopyAnswer", shim_mg_copy_answer as u64, "MGCopyAnswer (stub → NULL)"),
            ("_MGGetBoolAnswer", shim_mg_get_bool as u64, "MGGetBoolAnswer (stub → false)"),
            ("_MGGetFloat64Answer", shim_mg_get_f64 as u64, "MGGetFloat64Answer (stub → 0.0)"),
            ("_MGGetSInt32Answer", shim_mg_get_i32 as u64, "MGGetSInt32Answer (stub → 0)"),
            ("_MGGetStringAnswer", shim_mg_copy_answer as u64, "MGGetStringAnswer (stub → NULL)"),
            ("_MGIsQuestionValid", shim_mg_get_bool as u64, "MGIsQuestionValid (stub → false)"),
            ("_MGRegisterMultipleForChanges", shim_mg_void_stub as u64, "MGRegisterMultipleForChanges (stub → void)"),
            ("_MGCopyMultipleAnswers", shim_mg_copy_answer as u64, "MGCopyMultipleAnswers (stub → NULL)"),
            ("_MGRegisterForBulkUpdates", shim_mg_void_stub as u64, "MGRegisterForBulkUpdates (stub → void)"),
        );
        log::debug!("MobileGestaltStubs: registered 9 stubs");
    }
}

pub extern "C" fn shim_mg_copy_answer(_question: u64) -> u64 { 0 }
pub extern "C" fn shim_mg_get_bool(_question: u64) -> i32 { 0 }
pub extern "C" fn shim_mg_get_f64(_question: u64) -> f64 { 0.0 }
pub extern "C" fn shim_mg_get_i32(_question: u64) -> i32 { 0 }
pub extern "C" fn shim_mg_void_stub(_arg0: u64, _arg1: u64) {}
