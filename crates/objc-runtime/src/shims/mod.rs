//! ObjC runtime shims: objc_msgSend, objc_getClass, etc.
//!
//! These are `extern "C"` functions registered in the `ShimTable`.
//! They reference the global `OBJC_RUNTIME` (populated during image
//! registration) to perform class lookup and method dispatch.
//!
//! On aarch64-apple, `objc_msgSend` uses an ARM64 assembly trampoline
//! (`trampoline.s`) that preserves the entire argument register file
//! (x0-x7, d0-d7) across the class-table lookup, then tail-calls the
//! IMP.

pub mod dispatch;
mod install;
mod install_advanced;
mod lifecycle;
#[cfg(test)]
mod tests;

use libsystem_shims::table::ShimTable;

// Re-export for KVC/KVO dispatch + internal use.
pub use dispatch::objc_msg_send_addr;

// Re-export NSObject memory-management method shims for Foundation registration.
pub use lifecycle::{
    host_nsexception_ehtype_addr,
    shim_nsobject_autorelease, shim_nsobject_dealloc, shim_nsobject_release, shim_nsobject_retain,
    shim_nsobject_retain_count,
};

// Re-export exception shim for NSException's -raise method.
pub use lifecycle::shim_objc_exception_throw;

// Re-export protocol shim for NSObject -conformsToProtocol:.
pub use lifecycle::shim_class_conforms_to_protocol;

/// A zero-filled 16 bytes that serves as the empty method cache.
static EMPTY_CACHE: [u8; 16] = [0u8; 16];

/// Install ObjC runtime shims into a `ShimTable`.
pub struct ObjcShims;

impl ObjcShims {
    pub fn install(table: &mut ShimTable) {
        install::install_dispatch_shims(table);
        install::install_arc_shims(table);
        install::install_introspection_shims(table);
        install::install_selector_shims(table);
        install_advanced::install_method_shims(table);
        install_advanced::install_ivar_property_shims(table);
        install_advanced::install_exception_shims(table);
        install_advanced::install_associated_shims(table);
        install_advanced::install_protocol_shims(table);
        install_advanced::install_dynamic_class_shims(table);
        log::debug!("ObjcShims: registered 112 ObjC runtime shims");
    }
}
