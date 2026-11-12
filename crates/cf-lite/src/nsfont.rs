//! NSFont shim: font factory stubs for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa        (u64)
//!   offset  8: name_ptr   (u64 — NSString*)
//!   offset 16: point_size (f64)
//!
//! Total: 24 bytes.
//!
//! Font metrics are synthesized from the point size using standard
//! typographic ratios (matching macOS system font proportions).

const NAME_OFF: usize = 8;
const SIZE_OFF: usize = 16;

fn alloc_font(name: &str, size: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    let cstr = std::ffi::CString::new(name).unwrap_or_default();
    let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsfont_class_addr();
        *((obj as usize + NAME_OFF) as *mut u64) = nsstr;
        *((obj as usize + SIZE_OFF) as *mut f64) = size;
    }
    obj
}

/// Public wrapper for other shim modules (e.g. NSFontManager).
pub fn alloc_font_pub(name: &str, size: f64) -> u64 {
    alloc_font(name, size)
}

/// Read the pointSize from a font object (or default to 13.0 for nil).
fn font_size(obj: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + SIZE_OFF) as *const f64) }
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
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `-[NSFont pointSize]`
pub extern "C" fn shim_nsfont_point_size(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + SIZE_OFF) as *const f64) }
}

/// `-[NSFont familyName]` — same as fontName for stub.
pub extern "C" fn shim_nsfont_family_name(obj: u64, cmd: u64) -> u64 {
    shim_nsfont_font_name(obj, cmd)
}

/// `+[NSFont fontWithName:size:]`
pub extern "C" fn shim_nsfont_font_with_name_size(
    _cls: u64,
    _cmd: u64,
    name: u64,
    size: f64,
) -> u64 {
    let s = crate::cfstring::extract_cfstring_cstr(name);
    alloc_font(s.unwrap_or(".AppleSystemUIFont"), size)
}

/// `+[NSFont monospacedDigitSystemFontOfSize:weight:]`
pub extern "C" fn shim_nsfont_monospaced_digit_system_font(
    _cls: u64,
    _cmd: u64,
    size: f64,
    _weight: f64,
) -> u64 {
    alloc_font(".AppleSystemUIFontMonospacedDigit", size)
}

/// `+[NSFont monospacedSystemFontOfSize:weight:]`
pub extern "C" fn shim_nsfont_monospaced_system_font(
    _cls: u64,
    _cmd: u64,
    size: f64,
    _weight: f64,
) -> u64 {
    alloc_font("Menlo-Regular", size)
}

/// `+[NSFont systemFontOfSize:weight:]`
pub extern "C" fn shim_nsfont_system_font_of_size_weight(
    _cls: u64,
    _cmd: u64,
    size: f64,
    _weight: f64,
) -> u64 {
    alloc_font(".AppleSystemUIFont", size)
}

// ── Font metrics ───────────────────────────────────────────────────
//
// These return synthetic values based on the point size, matching
// macOS SF Pro Text proportions at common sizes.

/// `-[NSFont ascender]` — distance above baseline.
pub extern "C" fn shim_nsfont_ascender(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * 0.77 // ~10pt for 13pt system font
}

/// `-[NSFont descender]` — distance below baseline (negative).
pub extern "C" fn shim_nsfont_descender(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * -0.23 // ~3pt for 13pt system font
}

/// `-[NSFont capHeight]` — height of capital letters.
pub extern "C" fn shim_nsfont_cap_height(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * 0.71
}

/// `-[NSFont xHeight]` — height of lowercase 'x'.
pub extern "C" fn shim_nsfont_x_height(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * 0.52
}

/// `-[NSFont leading]` — recommended inter-line spacing.
pub extern "C" fn shim_nsfont_leading(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * 0.03
}

/// `-[NSFont underlinePosition]`
pub extern "C" fn shim_nsfont_underline_position(obj: u64, _cmd: u64) -> f64 {
    font_size(obj) * -0.08
}

/// `-[NSFont underlineThickness]`
pub extern "C" fn shim_nsfont_underline_thickness(obj: u64, _cmd: u64) -> f64 {
    (font_size(obj) * 0.07).max(0.5)
}

/// `-[NSFont isFixedPitch]`
pub extern "C" fn shim_nsfont_is_fixed_pitch(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let name = unsafe { *((obj as usize + NAME_OFF) as *const u64) };
    let s = crate::cfstring::extract_cfstring_cstr(name);
    u8::from(s.is_some_and(|n| n.contains("Mono") || n.contains("Courier") || n.contains("Menlo")))
}

/// `-[NSFont displayName]` — same as fontName for stub.
pub extern "C" fn shim_nsfont_display_name(obj: u64, cmd: u64) -> u64 {
    shim_nsfont_font_name(obj, cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn test_alloc_font() {
        let font = alloc_font("Helvetica", 14.0);
        assert_ne!(font, 0);
        let name_ptr = read_field(font, NAME_OFF);
        assert_ne!(name_ptr, 0);
        assert_string_eq(name_ptr, "Helvetica");
        let size = unsafe { *((font as usize + SIZE_OFF) as *const f64) };
        assert_eq!(size, 14.0);
        let isa = read_field(font, 0);
        assert_eq!(isa, crate::nsobj::nsfont_class_addr());
        free_obj(font);
    }

    #[test]
    fn test_system_font_of_size() {
        let font = shim_nsfont_system_font_of_size(0, 0, 12.0);
        assert_ne!(font, 0);
        let name = shim_nsfont_font_name(font, 0);
        assert_string_eq(name, ".AppleSystemUIFont");
        assert_eq!(shim_nsfont_point_size(font, 0), 12.0);
        free_obj(font);
    }

    #[test]
    fn test_bold_system_font_of_size() {
        let font = shim_nsfont_bold_system_font_of_size(0, 0, 18.0);
        assert_ne!(font, 0);
        let name = shim_nsfont_font_name(font, 0);
        assert_string_eq(name, ".AppleSystemUIFontBold");
        assert_eq!(shim_nsfont_point_size(font, 0), 18.0);
        free_obj(font);
    }

    #[test]
    fn test_label_font_of_size() {
        let font = shim_nsfont_label_font_of_size(0, 0, 10.0);
        assert_ne!(font, 0);
        let name = shim_nsfont_font_name(font, 0);
        assert_string_eq(name, ".AppleSystemUIFont");
        assert_eq!(shim_nsfont_point_size(font, 0), 10.0);
        free_obj(font);
    }

    #[test]
    fn test_system_font_size() {
        let sz = shim_nsfont_system_font_size(0, 0);
        assert_eq!(sz, 13.0);
    }

    #[test]
    fn test_font_name() {
        let font = alloc_font("Courier", 9.0);
        let name = shim_nsfont_font_name(font, 0);
        assert_string_eq(name, "Courier");
        free_obj(font);
    }

    #[test]
    fn test_font_name_nil() {
        assert_eq!(shim_nsfont_font_name(0, 0), 0);
    }

    #[test]
    fn test_point_size() {
        let font = alloc_font("Menlo", 15.5);
        assert_eq!(shim_nsfont_point_size(font, 0), 15.5);
        free_obj(font);
    }

    #[test]
    fn test_point_size_nil() {
        assert_eq!(shim_nsfont_point_size(0, 0), 0.0);
    }

    #[test]
    fn test_family_name() {
        let font = alloc_font("Times", 11.0);
        let family = shim_nsfont_family_name(font, 0);
        assert_string_eq(family, "Times");
        free_obj(font);
    }

    #[test]
    fn test_family_name_nil() {
        assert_eq!(shim_nsfont_family_name(0, 0), 0);
    }

    #[test]
    fn test_system_font_zero_size() {
        let font = shim_nsfont_system_font_of_size(0, 0, 0.0);
        assert_ne!(font, 0);
        assert_eq!(shim_nsfont_point_size(font, 0), 0.0);
        free_obj(font);
    }

    #[test]
    fn test_font_with_name_size() {
        let name = make_string("Helvetica");
        let font = shim_nsfont_font_with_name_size(0, 0, name, 14.0);
        assert_ne!(font, 0);
        assert_string_eq(shim_nsfont_font_name(font, 0), "Helvetica");
        assert_eq!(shim_nsfont_point_size(font, 0), 14.0);
        free_obj(font);
    }

    #[test]
    fn test_ascender_positive() {
        let font = shim_nsfont_system_font_of_size(0, 0, 13.0);
        let asc = shim_nsfont_ascender(font, 0);
        assert!(asc > 0.0, "ascender should be positive, got {asc}");
        assert!(asc < 13.0, "ascender should be less than point size");
        free_obj(font);
    }

    #[test]
    fn test_descender_negative() {
        let font = shim_nsfont_system_font_of_size(0, 0, 13.0);
        let desc = shim_nsfont_descender(font, 0);
        assert!(desc < 0.0, "descender should be negative, got {desc}");
        free_obj(font);
    }

    #[test]
    fn test_cap_height() {
        let font = shim_nsfont_system_font_of_size(0, 0, 13.0);
        let ch = shim_nsfont_cap_height(font, 0);
        assert!(ch > 0.0 && ch < 13.0);
        free_obj(font);
    }

    #[test]
    fn test_x_height() {
        let font = shim_nsfont_system_font_of_size(0, 0, 13.0);
        let xh = shim_nsfont_x_height(font, 0);
        assert!(xh > 0.0 && xh < shim_nsfont_cap_height(font, 0));
        free_obj(font);
    }

    #[test]
    fn test_leading() {
        let font = shim_nsfont_system_font_of_size(0, 0, 13.0);
        let lead = shim_nsfont_leading(font, 0);
        assert!(lead >= 0.0 && lead < 2.0);
        free_obj(font);
    }

    #[test]
    fn test_is_fixed_pitch() {
        let mono = alloc_font("Menlo-Regular", 12.0);
        assert_eq!(shim_nsfont_is_fixed_pitch(mono, 0), 1);
        free_obj(mono);

        let prop = alloc_font("Helvetica", 12.0);
        assert_eq!(shim_nsfont_is_fixed_pitch(prop, 0), 0);
        free_obj(prop);
    }

    #[test]
    fn test_metrics_nil_safety() {
        assert_eq!(shim_nsfont_ascender(0, 0), 0.0);
        assert_eq!(shim_nsfont_descender(0, 0), 0.0);
        assert_eq!(shim_nsfont_cap_height(0, 0), 0.0);
        assert_eq!(shim_nsfont_x_height(0, 0), 0.0);
        assert_eq!(shim_nsfont_leading(0, 0), 0.0);
    }
}
