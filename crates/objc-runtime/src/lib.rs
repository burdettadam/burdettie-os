//! ObjC runtime for burdetty-os.
//!
//! Provides class registration, selector interning, method dispatch
//! (`objc_msgSend`), and object allocation for Mach-O binaries with
//! Objective-C metadata.
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
// Public shim functions take raw pointers from emulated code — inherently unsafe at the FFI boundary.
#![allow(clippy::fn_to_numeric_cast, clippy::not_unsafe_ptr_arg_deref)]
//!
//! This is a minimal, single-threaded, lab-grade ObjC runtime — it
//! implements enough to run simple ObjC binaries without Foundation.

pub(crate) mod blocks;
mod class;
pub mod refcount;
mod shims;

pub use class::{ClassInfo, MethodEntry, ObjcRuntime, OBJC_RUNTIME};
pub use shims::ObjcShims;
pub use shims::{
    shim_nsobject_autorelease, shim_nsobject_dealloc, shim_nsobject_release, shim_nsobject_retain,
    shim_nsobject_retain_count, shim_objc_exception_throw,
};

// Re-export for KVC/KVO dispatch — allows calling guest methods through the trampoline.
pub use shims::objc_msg_send_addr;
