//! CF-lite: Foundation and CoreFoundation shims for burdetty-os.
//!
//! Provides the minimum set of Foundation/CF symbols needed to run
//! ObjC binaries that use NSLog, NSObject, and @"..." string literals.
//!
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
// Public shim functions take raw pointers from emulated code — inherently unsafe at the FFI boundary.
#![allow(
    clippy::fn_to_numeric_cast,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::unwrap_used,            // shim FFI code uses unwrap where panic = irrecoverable
    clippy::expect_used,            // same — expect with context for init paths
    clippy::cast_possible_truncation,  // ObjC type-encoding casts are intentional
    clippy::cast_sign_loss,            // C ABI interop
    clippy::cast_possible_wrap,        // C ABI interop
)]
//! This is NOT a real CoreFoundation implementation. It provides:
//! - `NSLog` → `fprintf(stderr, ...)`
//! - `___CFConstantStringClassReference` → fake class data for @"..." literals
//! - `_OBJC_CLASS_$_NSObject` / `_OBJC_METACLASS_$_NSObject` → minimal class stubs

pub(crate) mod cfstring;
pub mod cg_types;
pub(crate) mod format;
pub(crate) mod foundation;
pub(crate) mod nsarray;
pub(crate) mod nsattributedstring;
pub(crate) mod nsbundle;
pub(crate) mod nscache;
pub(crate) mod nscalendar;
pub(crate) mod nscharacterset;
pub(crate) mod nsdata;
pub(crate) mod nsdate;
pub(crate) mod nsdateformatter;
pub(crate) mod nsdictionary;
pub(crate) mod nsenumerator;
pub(crate) mod nserror;
pub(crate) mod nsexception;
pub(crate) mod nsfilemanager;
pub(crate) mod nshashtable;
pub(crate) mod nsindexset;
pub(crate) mod nsinvocation;
pub(crate) mod nsjsonserialization;
pub(crate) mod nslocale;
pub(crate) mod nslock;
pub mod nslog;
pub(crate) mod nsmaptable;
pub(crate) mod nsmutablearray;
pub(crate) mod nsmutabledict;
pub(crate) mod nsmutableset;
pub(crate) mod nsmutablestring;
pub(crate) mod nsnotificationcenter;
pub(crate) mod nskeyvalueobserving;
pub(crate) mod nskeyvaluecoding;
pub(crate) mod nsmethodsignature;
pub(crate) mod nsnull;
pub(crate) mod nsnumber;
pub(crate) mod nsnumberformatter;
pub mod nsobj;
pub(crate) mod nsoperation;
pub(crate) mod nspredicate;
pub(crate) mod nsprocessinfo;
pub(crate) mod nspropertylistserialization;
pub(crate) mod nsregularexpression;
pub(crate) mod nsset;
pub(crate) mod nssortdescriptor;
pub(crate) mod nsthread;
pub(crate) mod nstimer;
pub(crate) mod nstimezone;
pub(crate) mod nsurl;
pub(crate) mod nsuserdefaults;
pub(crate) mod nsuuid;
pub(crate) mod nsvalue;

pub use foundation::FoundationShims;

use platform_core::PlatformProvider;
use std::sync::{Arc, OnceLock};

/// Global platform provider for Foundation shim functions.
///
/// Set once by the runtime (via `set_platform`) before Foundation classes
/// are registered.  Shim functions that need clock, rng, fs, or process
/// info access it through `platform()`.
static PLATFORM: OnceLock<Arc<dyn PlatformProvider>> = OnceLock::new();

/// Install the platform provider for Foundation shims.
pub fn set_platform(provider: Arc<dyn PlatformProvider>) {
    if PLATFORM.set(provider).is_err() {
        log::debug!("cf-lite: set_platform called more than once (harmless if reusing runtime)");
    }
}

/// Access the platform provider from shim functions.
pub(crate) fn platform() -> &'static dyn PlatformProvider {
    PLATFORM
        .get()
        .expect("cf-lite: platform not initialized")
        .as_ref()
}

/// Convenience accessor for the platform heap allocator.
/// Falls back to libc if the platform isn't initialized (unit tests).
pub(crate) fn pal_alloc() -> &'static dyn platform_core::alloc::Allocator {
    use platform_darwin_host::DarwinAllocator;
    static FALLBACK: DarwinAllocator = DarwinAllocator;
    PLATFORM
        .get()
        .map(|p| p.alloc())
        .unwrap_or(&FALLBACK)
}

/// Convenience accessor for the platform symbol resolver.
/// Falls back to libc dlsym if the platform isn't initialized (unit tests).
pub(crate) fn pal_symbols() -> &'static dyn platform_core::symbols::HostSymbolResolver {
    use platform_darwin_host::DarwinSymbolResolver;
    static FALLBACK: DarwinSymbolResolver = DarwinSymbolResolver::new();
    PLATFORM
        .get()
        .map(|p| p.symbols())
        .unwrap_or(&FALLBACK)
}

/// Convenience accessor for the platform threading primitives.
/// Falls back to POSIX pthreads if the platform isn't initialized (unit tests).
pub(crate) fn pal_threading() -> &'static dyn platform_core::threading::Threading {
    use platform_darwin_host::PosixThreading;
    static FALLBACK: PosixThreading = PosixThreading;
    PLATFORM
        .get()
        .map(|p| p.threading())
        .unwrap_or(&FALLBACK)
}
