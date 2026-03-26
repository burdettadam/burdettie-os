//! NSScreen shim: screen geometry stub.
//!
//! Object layout (singleton):
//!
//!   offset  0: isa (u64)
//!
//! Total: 8 bytes. Frame values are returned directly, not stored.

use std::sync::OnceLock;

static MAIN_SCREEN: OnceLock<u64> = OnceLock::new();

fn get_or_create_main_screen() -> u64 {
    *MAIN_SCREEN.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj == 0 { return 0; }
        unsafe { *(obj as *mut u64) = crate::nsobj::nsscreen_class_addr(); }
        obj
    })
}

/// `+[NSScreen mainScreen]`
pub extern "C" fn shim_nsscreen_main_screen(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_screen()
}

/// `+[NSScreen screens]` — returns array with main screen.
pub extern "C" fn shim_nsscreen_screens(_cls: u64, _cmd: u64) -> u64 {
    let main = get_or_create_main_screen();
    crate::nsarray::make_nsarray(&[main])
}

/// `-[NSScreen frame]` — returns x=0 as simplified f64 stub.
/// Real CGRect return needs x8 indirect; for now return origin.x.
pub extern "C" fn shim_nsscreen_frame(_obj: u64, _cmd: u64) -> f64 {
    0.0 // Origin x of {0, 0, 1920, 1080}
}

/// `-[NSScreen visibleFrame]` — same stub as frame.
pub extern "C" fn shim_nsscreen_visible_frame(_obj: u64, _cmd: u64) -> f64 {
    0.0
}
