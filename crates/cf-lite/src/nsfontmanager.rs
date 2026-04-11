//! NSFontManager shim: font discovery and enumeration for AppKit.
//!
//! Provides a shared font manager singleton that enumerates available
//! font families and fonts via `graphics-core`'s font system.

use std::sync::Mutex;

/// Cached singleton pointer for `+[NSFontManager sharedFontManager]`.
static SHARED_MANAGER: Mutex<u64> = Mutex::new(0);

/// `+[NSFontManager sharedFontManager]` — returns singleton.
pub extern "C" fn shim_nsfontmanager_shared(_cls: u64, _cmd: u64) -> u64 {
    let mut guard = SHARED_MANAGER.lock().unwrap_or_else(|e| e.into_inner());
    if *guard == 0 {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe { *(obj as *mut u64) = crate::nsobj::nsfontmanager_class_addr(); }
        }
        *guard = obj;
    }
    *guard
}

/// `+[NSFontManager setFontManagerFactory:]` — no-op stub.
pub extern "C" fn shim_nsfontmanager_set_factory(_cls: u64, _cmd: u64, _factory: u64) {
    log::debug!("NSFontManager setFontManagerFactory: (stub)");
}

/// `-[NSFontManager availableFontFamilies]` — returns NSArray of font family names.
pub extern "C" fn shim_nsfontmanager_available_font_families(
    _obj: u64,
    _cmd: u64,
) -> u64 {
    let families = cg_shims::bridge::font_family_names();
    build_nsarray_of_strings(&families)
}

/// `-[NSFontManager availableFonts]` — returns NSArray of all font names.
///
/// For simplicity, returns family names (same as availableFontFamilies).
/// Real AppKit returns individual font face names (e.g. "Helvetica-Bold").
pub extern "C" fn shim_nsfontmanager_available_fonts(_obj: u64, _cmd: u64) -> u64 {
    let families = cg_shims::bridge::font_family_names();
    build_nsarray_of_strings(&families)
}

/// `-[NSFontManager availableMembersOfFontFamily:]` — stub returning nil.
pub extern "C" fn shim_nsfontmanager_available_members(
    _obj: u64,
    _cmd: u64,
    _family: u64,
) -> u64 {
    log::debug!("NSFontManager availableMembersOfFontFamily: (stub → nil)");
    0
}

/// `-[NSFontManager fontWithFamily:traits:weight:size:]` — creates an NSFont.
pub extern "C" fn shim_nsfontmanager_font_with_family(
    _obj: u64,
    _cmd: u64,
    family_ptr: u64,
    _traits: u64,
    _weight: u64,
    size: f64,
) -> u64 {
    let name = crate::cfstring::extract_cfstring_cstr(family_ptr)
        .unwrap_or(".AppleSystemUIFont");
    crate::nsfont::alloc_font_pub(name, size)
}

/// `-[NSFontManager convertFont:]` — stub returns same font.
pub extern "C" fn shim_nsfontmanager_convert_font(
    _obj: u64,
    _cmd: u64,
    font: u64,
) -> u64 {
    font
}

/// `-[NSFontManager convertFont:toHaveTrait:]` — stub returns same font.
pub extern "C" fn shim_nsfontmanager_convert_font_to_trait(
    _obj: u64,
    _cmd: u64,
    font: u64,
    _trait: u64,
) -> u64 {
    font
}

/// `-[NSFontManager convertFont:toSize:]` — stub returns same font.
pub extern "C" fn shim_nsfontmanager_convert_font_to_size(
    _obj: u64,
    _cmd: u64,
    font: u64,
    _size: f64,
) -> u64 {
    font
}

/// `-[NSFontManager setSelectedFont:isMultiple:]` — no-op stub.
pub extern "C" fn shim_nsfontmanager_set_selected_font(
    _obj: u64,
    _cmd: u64,
    _font: u64,
    _is_multiple: u8,
) {
    log::debug!("NSFontManager setSelectedFont:isMultiple: (stub)");
}

// ── Helpers ────────────────────────────────────────────────────

/// Build an NSArray of NSString objects from a slice of Rust strings.
fn build_nsarray_of_strings(strings: &[String]) -> u64 {
    let ptrs: Vec<u64> = strings
        .iter()
        .map(|s| {
            let cstr = std::ffi::CString::new(s.as_str()).unwrap_or_default();
            crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
        })
        .collect();
    crate::nsarray::make_nsarray(&ptrs)
}

/// Reset shared state for runtime reuse.
pub fn reset_font_manager_state() {
    let mut guard = SHARED_MANAGER.lock().unwrap_or_else(|e| e.into_inner());
    *guard = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn shared_returns_same_pointer() {
        let a = shim_nsfontmanager_shared(0, 0);
        let b = shim_nsfontmanager_shared(0, 0);
        assert_ne!(a, 0, "sharedFontManager should not be nil");
        assert_eq!(a, b, "sharedFontManager should return same object");
    }

    #[test]
    fn available_font_families_returns_non_nil() {
        let mgr = shim_nsfontmanager_shared(0, 0);
        let arr = shim_nsfontmanager_available_font_families(mgr, 0);
        assert_ne!(arr, 0, "availableFontFamilies should not be nil");
    }

    #[test]
    fn available_fonts_returns_non_nil() {
        let mgr = shim_nsfontmanager_shared(0, 0);
        let arr = shim_nsfontmanager_available_fonts(mgr, 0);
        assert_ne!(arr, 0, "availableFonts should not be nil");
    }

    #[test]
    fn set_factory_no_crash() {
        shim_nsfontmanager_set_factory(0, 0, 0x1234);
    }

    #[test]
    fn convert_font_returns_same() {
        let font = alloc_obj_fake(24);
        let result = shim_nsfontmanager_convert_font(0, 0, font);
        assert_eq!(result, font);
        free_obj(font);
    }

    #[test]
    fn font_with_family_returns_font() {
        let name = make_string("Helvetica");
        let result = shim_nsfontmanager_font_with_family(0, 0, name, 0, 5, 14.0);
        assert_ne!(result, 0, "fontWithFamily should return non-nil");
        free_obj(name);
    }

    #[test]
    fn available_members_returns_nil() {
        let name = make_string("DejaVu Sans");
        let result = shim_nsfontmanager_available_members(0, 0, name);
        assert_eq!(result, 0, "stub should return nil");
        free_obj(name);
    }

    #[test]
    fn set_selected_font_no_crash() {
        shim_nsfontmanager_set_selected_font(0, 0, 0x1234, 0);
    }
}
