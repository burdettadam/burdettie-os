//! NSFont shim: font factory stubs for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa        (u64)
//!   offset  8: name_ptr   (u64 — NSString*)
//!   offset 16: point_size (f64)
//!
//! Total: 24 bytes.

const NAME_OFF: usize = 8;
const SIZE_OFF: usize = 16;

fn alloc_font(name: &str, size: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    let cstr = std::ffi::CString::new(name).unwrap_or_default();
    let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsfont_class_addr();
        *((obj as usize + NAME_OFF) as *mut u64) = nsstr;
        *((obj as usize + SIZE_OFF) as *mut f64) = size;
    }
    obj
}

/// `+[NSFont systemFontOfSize:]`
pub extern "C" fn shim_nsfont_system_font_of_size(_cls: u64, _cmd: u64, size: f64) -> u64 {
    alloc_font(".AppleSystemUIFont", size)
}

/// `+[NSFont boldSystemFontOfSize:]`
pub extern "C" fn shim_nsfont_bold_system_font_of_size(_cls: u64, _cmd: u64, size: f64) -> u64 {
    alloc_font(".AppleSystemUIFontBold", size)
}

/// `+[NSFont labelFontOfSize:]`
pub extern "C" fn shim_nsfont_label_font_of_size(_cls: u64, _cmd: u64, size: f64) -> u64 {
    alloc_font(".AppleSystemUIFont", size)
}

/// `+[NSFont systemFontSize]` — returns 13.0 (default macOS system font size).
pub extern "C" fn shim_nsfont_system_font_size(_cls: u64, _cmd: u64) -> f64 {
    13.0
}

/// `-[NSFont fontName]`
pub extern "C" fn shim_nsfont_font_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `-[NSFont pointSize]`
pub extern "C" fn shim_nsfont_point_size(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + SIZE_OFF) as *const f64) }
}

/// `-[NSFont familyName]` — same as fontName for stub.
pub extern "C" fn shim_nsfont_family_name(obj: u64, cmd: u64) -> u64 {
    shim_nsfont_font_name(obj, cmd)
}
