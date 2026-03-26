//! NSView shim: view stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa           (u64)
//!   offset  8: superview     (u64 — parent NSView*, or 0)
//!   offset 16: frame_x       (f64)
//!   offset 24: frame_y       (f64)
//!   offset 32: frame_w       (f64)
//!   offset 40: frame_h       (f64)
//!
//! Total: 48 bytes.

use crate::cg_types::{CGRect, CGPoint, CGSize};

const SUPERVIEW_OFF: usize = 8;
const FRAME_X_OFF: usize = 16;

/// `-[NSView initWithFrame:]`
pub extern "C" fn shim_nsview_init_with_frame(
    _obj: u64,
    _cmd: u64,
    x: f64, y: f64, w: f64, h: f64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 48) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsview_class_addr();
        *((obj as usize + FRAME_X_OFF) as *mut f64) = x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = y;
        *((obj as usize + FRAME_X_OFF + 16) as *mut f64) = w;
        *((obj as usize + FRAME_X_OFF + 24) as *mut f64) = h;
    }
    let _ = CGRect { origin: CGPoint { x, y }, size: CGSize { width: w, height: h } };
    obj
}

/// `-[NSView addSubview:]` — stub.
pub extern "C" fn shim_nsview_add_subview(_obj: u64, _cmd: u64, _subview: u64) {
    log::debug!("NSView addSubview: (stub)");
}

/// `-[NSView setNeedsDisplay:]` — stub.
pub extern "C" fn shim_nsview_set_needs_display(_obj: u64, _cmd: u64, _flag: u8) {
    log::debug!("NSView setNeedsDisplay: (stub)");
}

/// `-[NSView superview]`
pub extern "C" fn shim_nsview_superview(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + SUPERVIEW_OFF) as *const u64) }
}

/// `-[NSView frame]` — returns first f64 of frame (x) as simplified stub.
pub extern "C" fn shim_nsview_frame(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + FRAME_X_OFF) as *const f64) }
}

/// `-[NSView bounds]` — same as frame for this stub.
pub extern "C" fn shim_nsview_bounds(obj: u64, _cmd: u64) -> f64 {
    shim_nsview_frame(obj, _cmd)
}

/// `-[NSView window]` — stub, returns 0.
pub extern "C" fn shim_nsview_window(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSView isFlipped]` — returns NO.
pub extern "C" fn shim_nsview_is_flipped(_obj: u64, _cmd: u64) -> u8 {
    0
}
