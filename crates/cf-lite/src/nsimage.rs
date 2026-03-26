//! NSImage shim: image factory stubs for AppKit bootstrap.
//!
//! Returns nil for all image loading — this is a graceful degradation
//! stub so apps don't crash on image references.

/// `+[NSImage imageNamed:]` — stub, always returns nil.
pub extern "C" fn shim_nsimage_image_named(_cls: u64, _cmd: u64, _name: u64) -> u64 {
    log::debug!("NSImage imageNamed: (stub, returning nil)");
    0
}

/// `-[NSImage size]` — returns 0.0 stub.
pub extern "C" fn shim_nsimage_size(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[NSImage name]` — returns nil.
pub extern "C" fn shim_nsimage_name(_obj: u64, _cmd: u64) -> u64 {
    0
}
