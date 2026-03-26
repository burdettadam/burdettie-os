//! NSColor shim: color factory stubs for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa  (u64)
//!   offset  8: r    (f64)
//!   offset 16: g    (f64)
//!   offset 24: b    (f64)
//!   offset 32: a    (f64)
//!
//! Total: 40 bytes.

fn alloc_color(r: f64, g: f64, b: f64, a: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nscolor_class_addr();
        *((obj as usize + 8) as *mut f64) = r;
        *((obj as usize + 16) as *mut f64) = g;
        *((obj as usize + 24) as *mut f64) = b;
        *((obj as usize + 32) as *mut f64) = a;
    }
    obj
}

/// `+[NSColor whiteColor]`
pub extern "C" fn shim_nscolor_white_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(1.0, 1.0, 1.0, 1.0) }

/// `+[NSColor blackColor]`
pub extern "C" fn shim_nscolor_black_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(0.0, 0.0, 0.0, 1.0) }

/// `+[NSColor redColor]`
pub extern "C" fn shim_nscolor_red_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(1.0, 0.0, 0.0, 1.0) }

/// `+[NSColor greenColor]`
pub extern "C" fn shim_nscolor_green_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(0.0, 1.0, 0.0, 1.0) }

/// `+[NSColor blueColor]`
pub extern "C" fn shim_nscolor_blue_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(0.0, 0.0, 1.0, 1.0) }

/// `+[NSColor clearColor]`
pub extern "C" fn shim_nscolor_clear_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(0.0, 0.0, 0.0, 0.0) }

/// `+[NSColor grayColor]`
pub extern "C" fn shim_nscolor_gray_color(_cls: u64, _cmd: u64) -> u64 { alloc_color(0.5, 0.5, 0.5, 1.0) }

/// `+[NSColor colorWithRed:green:blue:alpha:]`
pub extern "C" fn shim_nscolor_with_rgba(_cls: u64, _cmd: u64, r: f64, g: f64, b: f64, a: f64) -> u64 {
    alloc_color(r, g, b, a)
}

/// `-[NSColor redComponent]`
pub extern "C" fn shim_nscolor_red_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + 8) as *const f64) }
}

/// `-[NSColor greenComponent]`
pub extern "C" fn shim_nscolor_green_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + 16) as *const f64) }
}

/// `-[NSColor blueComponent]`
pub extern "C" fn shim_nscolor_blue_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + 24) as *const f64) }
}

/// `-[NSColor alphaComponent]`
pub extern "C" fn shim_nscolor_alpha_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + 32) as *const f64) }
}
