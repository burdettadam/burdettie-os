//! Rosetta translation stubs.
//!
//! Stubs report that Rosetta translation is unavailable.

use crate::table::ShimTable;

pub struct RosettaStubs;

impl RosettaStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_rosetta_is_translation_available", shim_rosetta_false as u64, "rosetta_is_translation_available (stub → false)"),
            ("_rosetta_has_been_previously_installed", shim_rosetta_false as u64, "rosetta_has_been_previously_installed (stub → false)"),
            ("_rosetta_translate_binaries", shim_rosetta_err as u64, "rosetta_translate_binaries (stub → -1)"),
        );
        log::debug!("RosettaStubs: registered 3 stubs");
    }
}

pub extern "C" fn shim_rosetta_false() -> i32 { 0 }
pub extern "C" fn shim_rosetta_err(_arg0: u64, _arg1: u64) -> i32 { -1 }
