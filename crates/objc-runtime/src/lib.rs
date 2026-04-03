//! ObjC runtime for burdettie-os.
//!
//! Provides class registration, selector interning, method dispatch
//! (`objc_msgSend`), and object allocation for Mach-O binaries with
//! Objective-C metadata.
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
// Public shim functions take raw pointers from emulated code — inherently unsafe at the FFI boundary.
#![allow(
    clippy::fn_to_numeric_cast,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
)]
//!
//! This is a minimal, single-threaded, lab-grade ObjC runtime — it
//! implements enough to run simple ObjC binaries without Foundation.

pub(crate) mod blocks;
pub mod backend;
mod class;
pub mod refcount;
pub mod shims;

pub use class::{ClassInfo, MethodEntry, ObjcRuntime, ProtocolInfo, OBJC_RUNTIME};
pub use shims::ObjcShims;
pub use shims::{
    host_nsexception_ehtype_addr,
    shim_nsobject_autorelease, shim_nsobject_dealloc, shim_nsobject_release, shim_nsobject_retain,
    shim_nsobject_retain_count, shim_objc_exception_throw,
    shim_class_conforms_to_protocol,
};

// Re-export for KVC/KVO dispatch — allows calling guest methods through the trampoline.
pub use shims::objc_msg_send_addr;

use std::sync::{Arc, OnceLock};

/// Global host symbol resolver for exception-bridge functions.
static HOST_SYMBOLS: OnceLock<Arc<dyn platform_core::symbols::HostSymbolResolver>> =
    OnceLock::new();

/// Set the global host symbol resolver. Called once during runtime initialization.
pub fn set_host_symbols(resolver: Arc<dyn platform_core::symbols::HostSymbolResolver>) {
    if HOST_SYMBOLS.set(resolver).is_err() {
        log::debug!("objc-runtime: set_host_symbols called more than once (harmless if reusing runtime)");
    }
}

/// Get the global host symbol resolver.
#[allow(dead_code)]
pub(crate) fn host_symbols() -> Option<&'static Arc<dyn platform_core::symbols::HostSymbolResolver>> {
    HOST_SYMBOLS.get()
}

/// Global heap allocator for ObjC object memory.
static PAL_ALLOC: OnceLock<Arc<dyn platform_core::alloc::Allocator>> = OnceLock::new();

/// Global threading primitives.
static PAL_THREADING: OnceLock<Arc<dyn platform_core::threading::Threading>> = OnceLock::new();

/// Set the global heap allocator. Called once during runtime initialization.
pub fn set_allocator(alloc: Arc<dyn platform_core::alloc::Allocator>) {
    if PAL_ALLOC.set(alloc).is_err() {
        log::debug!("objc-runtime: set_allocator called more than once (harmless if reusing runtime)");
    }
}

/// Set the global threading provider. Called once during runtime initialization.
pub fn set_threading(threading: Arc<dyn platform_core::threading::Threading>) {
    if PAL_THREADING.set(threading).is_err() {
        log::debug!("objc-runtime: set_threading called more than once (harmless if reusing runtime)");
    }
}

/// Access the platform heap allocator from shim functions.
/// Falls back to libc if not initialized (for tests).
pub(crate) fn pal_alloc() -> &'static dyn platform_core::alloc::Allocator {
    #[cfg(target_os = "macos")]
    {
        use platform_darwin_host::DarwinAllocator;
        static FALLBACK: DarwinAllocator = DarwinAllocator;
        PAL_ALLOC.get().map(|a| a.as_ref()).unwrap_or(&FALLBACK)
    }
    #[cfg(target_os = "linux")]
    {
        use platform_linux::LinuxAllocator;
        static FALLBACK: LinuxAllocator = LinuxAllocator;
        PAL_ALLOC.get().map(|a| a.as_ref()).unwrap_or(&FALLBACK)
    }
}

/// Access the platform threading primitives from shim functions.
/// Falls back to POSIX pthreads if not initialized (for tests).
pub(crate) fn pal_threading() -> &'static dyn platform_core::threading::Threading {
    #[cfg(target_os = "macos")]
    {
        use platform_darwin_host::PosixThreading;
        static FALLBACK: PosixThreading = PosixThreading;
        PAL_THREADING.get().map(|t| t.as_ref()).unwrap_or(&FALLBACK)
    }
    #[cfg(target_os = "linux")]
    {
        use platform_linux::LinuxThreading;
        static FALLBACK: LinuxThreading = LinuxThreading;
        PAL_THREADING.get().map(|t| t.as_ref()).unwrap_or(&FALLBACK)
    }
}
