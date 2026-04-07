//! CF-lite: Foundation and CoreFoundation shims for burdettie-os.
//!
//! Provides the minimum set of Foundation/CF symbols needed to run
//! ObjC binaries that use NSLog, NSObject, and @"..." string literals.
//!
//! # Apple Stack Layer Organization
//!
//! This crate implements three layers of Apple's framework hierarchy:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  AppKit layer         (ns{alert,application,...})   │
//! ├─────────────────────────────────────────────────────┤
//! │  Foundation layer     (ns{array,data,string,...})   │
//! ├─────────────────────────────────────────────────────┤
//! │  CoreFoundation layer (cf_core, cfstring, cg_types) │
//! │  Geometry types live in `core-foundation-lite`      │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
// Public shim functions take raw pointers from emulated code — inherently unsafe at the FFI boundary.
#![allow(
    dead_code,                         // FFI shims referenced by pointer, not by Rust call
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

// ── CoreFoundation layer ────────────────────────────────────────
// Geometry types canonically defined in `core-foundation-lite` crate,
// re-exported through cg_types for backward compatibility.
pub mod cf_core;
pub(crate) mod cfstring;
pub mod cg_types;
pub(crate) mod format;
pub(crate) mod foundation_constants;

// ── Foundation layer (ObjC) ─────────────────────────────────────
pub(crate) mod foundation;
pub(crate) mod nsarray;
pub(crate) mod nsattributedstring;
pub(crate) mod nsbundle;
pub(crate) mod nscache;
pub(crate) mod nscalendar;
pub(crate) mod nscharacterset;
pub(crate) mod nscountedset;
pub(crate) mod nsdata;
pub(crate) mod nsdate;
pub(crate) mod nsdateformatter;
pub(crate) mod nsdictionary;
pub(crate) mod nsenumerator;
pub(crate) mod nserror;
pub(crate) mod nsexception;
pub(crate) mod nsfilehandle;
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
pub(crate) mod nsmutablecharacterset;
pub(crate) mod nsmutabledata;
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
pub(crate) mod nsdecimalnumber;
pub mod nsobj;
pub(crate) mod nsoperation;
pub(crate) mod nsorderedset;
pub(crate) mod nspredicate;
pub(crate) mod nsprocessinfo;
pub(crate) mod nspropertylistserialization;
pub(crate) mod nsregularexpression;
pub(crate) mod nsresponder;
pub(crate) mod nsrunloop;
pub(crate) mod nsscanner;
pub(crate) mod nsscreen;
pub(crate) mod nsset;
pub(crate) mod nssortdescriptor;
pub(crate) mod nsthread;
pub(crate) mod nstimer;
pub(crate) mod nstimezone;
pub(crate) mod nsurl;
pub(crate) mod nsurlcomponents;
pub(crate) mod nsurlrequest;
pub(crate) mod nsuserdefaults;
pub(crate) mod nsuuid;
pub(crate) mod nsvalue;
pub(crate) mod nsview;
pub(crate) mod nswindow;
pub(crate) mod nsworkspace;
pub(crate) mod nspasteboard;
pub(crate) mod nsmeasurement;
// Gap-analysis Phase C9
pub(crate) mod nsbytecountformatter;
pub(crate) mod nshost;
pub(crate) mod nsiso8601dateformatter;
pub(crate) mod nspipe;
pub(crate) mod nsprogress;
pub(crate) mod nstask;
pub(crate) mod pkbundlecomponentversion;
// Gap-analysis Phase C10
pub(crate) mod nskeyedarchiver;
pub(crate) mod nsstream;
pub(crate) mod nsurlsession;
pub(crate) mod nsxmlparser;
// Gap-analysis Phase C12: credential and cookie storage stubs
pub(crate) mod nscredential;
pub(crate) mod nsautoreleasepool;

// ── AppKit layer (GUI) ──────────────────────────────────────────
// These modules implement stub AppKit classes. They are structurally
// separate from Foundation and will eventually migrate to app-kit-lite.
// XCStringsParser.framework shims
pub(crate) mod xcstringsdata_entry;
pub(crate) mod xcstringsdata_table;
pub(crate) mod xcstringsdata_file;
// AppKit stubs
pub(crate) mod nsalert;
pub(crate) mod nsapplication;
pub(crate) mod nsappearance;
pub(crate) mod nsbutton;
pub(crate) mod calayer;
pub(crate) mod nscolor;
pub(crate) mod nscolorwell;
pub(crate) mod nscombobox;
pub(crate) mod nscontrol;
pub(crate) mod nsevent;
pub(crate) mod nsfont;
pub(crate) mod nsgraphicscontext;
pub(crate) mod nsimage;
pub(crate) mod nsimageview;
pub(crate) mod nsmenu_shim;
pub(crate) mod nsnib;
pub(crate) mod nsbox;
pub(crate) mod nspanel;
pub(crate) mod nspopupbutton;
pub(crate) mod nsprogressindicator;
pub(crate) mod nsscrollview;
pub(crate) mod nssegmentedcontrol;
pub(crate) mod nsslider;
pub(crate) mod nssplitview;
pub(crate) mod nslayoutanchor;
pub(crate) mod nslayoutconstraint;
pub(crate) mod nslayoutconstraint_vfl;
pub(crate) mod nslayoutguide;
pub(crate) mod layout_solver;
pub(crate) mod nsstackview;
pub(crate) mod nstabview;
pub(crate) mod nstabviewitem;
pub(crate) mod nstextfield;
pub(crate) mod nstoolbar;
pub(crate) mod nstoolbaritem;
pub(crate) mod nsusernotificationcenter;
pub(crate) mod nsviewcontroller;
pub(crate) mod nswindowcontroller;
// Tier 2 Feature Parity: safe area, size classes, animation, table view, undo
pub(crate) mod nstraitcollection;
pub(crate) mod nsanimationcontext;
pub(crate) mod nsbezierpath;
pub(crate) mod nstableview;
pub(crate) mod nsundomanager;
pub(crate) mod nsclipview;
pub(crate) mod nstextview;
#[cfg(target_os = "macos")]
pub(crate) mod nsopenpanel;
pub(crate) mod nstrackingarea;
pub(crate) mod nscursor;
pub(crate) mod nscolorpanel;
pub(crate) mod nsstepper;

// GUI bridge: connects AppKit shims to a real windowing backend
pub mod gui_bridge;

// Framework stubs (CoreData, AVFoundation, CoreLocation, CoreBluetooth)
pub(crate) mod coredata_stubs;
pub(crate) mod avfoundation_stubs;
pub(crate) mod corelocation_stubs;

pub use foundation::FoundationShims;

#[cfg(test)]
pub(crate) mod shim_test_helpers;

use platform_core::PlatformProvider;
use std::sync::{Arc, OnceLock};

/// Global platform provider for Foundation shim functions.
///
/// Set once by the runtime (via `set_platform`) before Foundation classes
/// are registered.  Shim functions that need clock, rng, fs, or process
/// info access it through `platform()`.
static PLATFORM: OnceLock<Arc<dyn PlatformProvider>> = OnceLock::new();

/// Reset process-global Foundation shim state so the runtime can be reused
/// safely for multiple guest executions in the same host process.
pub fn reset_runtime_state_for_reuse() {
    nsapplication::reset_runtime_state_for_reuse();
    nskeyvalueobserving::clear_all_observers();
    nsnotificationcenter::clear_all_observers();
    nsrunloop::reset_runtime_state_for_reuse();
    layout_solver::reset_global_state();
    nsview::reset_layout_priority_state();
}

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
    #[cfg(target_os = "macos")]
    {
        use platform_darwin_host::DarwinAllocator;
        static FALLBACK: DarwinAllocator = DarwinAllocator;
        PLATFORM.get().map(|p| p.alloc()).unwrap_or(&FALLBACK)
    }
    #[cfg(target_os = "linux")]
    {
        use platform_linux::LinuxAllocator;
        static FALLBACK: LinuxAllocator = LinuxAllocator;
        PLATFORM.get().map(|p| p.alloc()).unwrap_or(&FALLBACK)
    }
}

/// Convenience accessor for the platform symbol resolver.
/// Falls back to libc dlsym if the platform isn't initialized (unit tests).
pub(crate) fn pal_symbols() -> &'static dyn platform_core::symbols::HostSymbolResolver {
    #[cfg(target_os = "macos")]
    {
        use platform_darwin_host::DarwinSymbolResolver;
        static FALLBACK: DarwinSymbolResolver = DarwinSymbolResolver::new();
        PLATFORM.get().map(|p| p.symbols()).unwrap_or(&FALLBACK)
    }
    #[cfg(target_os = "linux")]
    {
        use platform_linux::LinuxSymbolResolver;
        static FALLBACK: LinuxSymbolResolver = LinuxSymbolResolver::new();
        PLATFORM.get().map(|p| p.symbols()).unwrap_or(&FALLBACK)
    }
}

/// Convenience accessor for the platform threading primitives.
/// Falls back to POSIX pthreads if the platform isn't initialized (unit tests).
pub(crate) fn pal_threading() -> &'static dyn platform_core::threading::Threading {
    #[cfg(target_os = "macos")]
    {
        use platform_darwin_host::PosixThreading;
        static FALLBACK: PosixThreading = PosixThreading;
        PLATFORM.get().map(|p| p.threading()).unwrap_or(&FALLBACK)
    }
    #[cfg(target_os = "linux")]
    {
        use platform_linux::LinuxThreading;
        static FALLBACK: LinuxThreading = LinuxThreading;
        PLATFORM.get().map(|p| p.threading()).unwrap_or(&FALLBACK)
    }
}
