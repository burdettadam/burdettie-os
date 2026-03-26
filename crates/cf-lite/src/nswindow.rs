//! NSWindow shim: window stub for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa              (u64)
//!   offset  8: content_view     (u64 — NSView*)
//!   offset 16: title_ptr        (u64 — NSString*)
//!   offset 24: style_mask       (u64)
//!   offset 32: frame_x          (f64)
//!   offset 40: frame_y          (f64)
//!   offset 48: frame_w          (f64)
//!   offset 56: frame_h          (f64)
//!
//! Total: 64 bytes.

use crate::cg_types::{CGRect, CGPoint, CGSize};

const CONTENT_VIEW_OFF: usize = 8;
const TITLE_OFF: usize = 16;
const STYLE_MASK_OFF: usize = 24;
const FRAME_X_OFF: usize = 32;

fn alloc_window(rect: CGRect, style: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 64) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nswindow_class_addr();
        *((obj as usize + STYLE_MASK_OFF) as *mut u64) = style;
        *((obj as usize + FRAME_X_OFF) as *mut f64) = rect.origin.x;
        *((obj as usize + FRAME_X_OFF + 8) as *mut f64) = rect.origin.y;
        *((obj as usize + FRAME_X_OFF + 16) as *mut f64) = rect.size.width;
        *((obj as usize + FRAME_X_OFF + 24) as *mut f64) = rect.size.height;
    }
    obj
}

/// `-[NSWindow initWithContentRect:styleMask:backing:defer:]`
pub extern "C" fn shim_nswindow_init_with_content_rect(
    _obj: u64,
    _cmd: u64,
    x: f64, y: f64, w: f64, h: f64, // CGRect passed as 4 doubles on ARM64
    style_mask: u64,
    _backing: u64,
    _defer: u8,
) -> u64 {
    let rect = CGRect {
        origin: CGPoint { x, y },
        size: CGSize { width: w, height: h },
    };
    log::debug!("NSWindow initWithContentRect:{} styleMask:{}", rect, style_mask);
    alloc_window(rect, style_mask)
}

/// `-[NSWindow contentView]`
pub extern "C" fn shim_nswindow_content_view(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + CONTENT_VIEW_OFF) as *const u64) }
}

/// `-[NSWindow setContentView:]`
pub extern "C" fn shim_nswindow_set_content_view(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + CONTENT_VIEW_OFF) as *mut u64) = view; }
}

/// `-[NSWindow makeKeyAndOrderFront:]` — stub.
pub extern "C" fn shim_nswindow_make_key_and_order_front(_obj: u64, _cmd: u64, _sender: u64) {
    log::debug!("NSWindow makeKeyAndOrderFront: (stub)");
}

/// `-[NSWindow title]`
pub extern "C" fn shim_nswindow_title(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + TITLE_OFF) as *const u64) }
}

/// `-[NSWindow setTitle:]`
pub extern "C" fn shim_nswindow_set_title(obj: u64, _cmd: u64, title: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + TITLE_OFF) as *mut u64) = title; }
}

/// `-[NSWindow styleMask]`
pub extern "C" fn shim_nswindow_style_mask(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + STYLE_MASK_OFF) as *const u64) }
}

/// `-[NSWindow frame]` — returns CGRect as x0,x1,d0,d1 (>16 bytes, uses x8 indirect on ARM64).
/// For now we return the origin.x in x0 as a simple stub.
pub extern "C" fn shim_nswindow_frame(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + FRAME_X_OFF) as *const f64) }
}

/// `-[NSWindow close]` — stub.
pub extern "C" fn shim_nswindow_close(_obj: u64, _cmd: u64) {
    log::debug!("NSWindow close (stub)");
}

/// `-[NSWindow orderFront:]` — stub.
pub extern "C" fn shim_nswindow_order_front(_obj: u64, _cmd: u64, _sender: u64) {
    log::debug!("NSWindow orderFront: (stub)");
}

/// `-[NSWindow isVisible]` — returns NO.
pub extern "C" fn shim_nswindow_is_visible(_obj: u64, _cmd: u64) -> u8 {
    0
}
