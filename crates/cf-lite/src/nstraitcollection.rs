//! NSTraitCollection shim: size classes, user interface idiom, and display scale.
//!
//! Object layout (40 bytes):
//!
//!   offset  0: isa                     (u64)
//!   offset  8: horizontal_size_class   (u64 — 0=unspecified, 1=compact, 2=regular)
//!   offset 16: vertical_size_class     (u64 — 0=unspecified, 1=compact, 2=regular)
//!   offset 24: user_interface_idiom    (u64 — 0=unspecified, 1=mac)
//!   offset 32: display_scale           (f64 — 1.0 or 2.0 for Retina)
//!
//! Total: 40 bytes.

const HORIZONTAL_SIZE_CLASS_OFF: usize = 8;
const VERTICAL_SIZE_CLASS_OFF: usize = 16;
const USER_INTERFACE_IDIOM_OFF: usize = 24;
const DISPLAY_SCALE_OFF: usize = 32;
pub(crate) const TRAIT_COLLECTION_SIZE: usize = 40;

/// Size class constants matching `NSUserInterfaceSizeClass`.
pub const SIZE_CLASS_UNSPECIFIED: u64 = 0;
pub const SIZE_CLASS_COMPACT: u64 = 1;
pub const SIZE_CLASS_REGULAR: u64 = 2;

/// User interface idiom constants.
pub const UI_IDIOM_UNSPECIFIED: u64 = 0;
pub const UI_IDIOM_MAC: u64 = 1;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut u64) = value; }
}

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut f64) = value; }
}

/// Allocate an NSTraitCollection with macOS desktop defaults.
pub(crate) fn make_default_trait_collection() -> u64 {
    let obj = crate::pal_alloc().calloc(1, TRAIT_COLLECTION_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nstraitcollection_class_addr(); }
    write_u64(obj, HORIZONTAL_SIZE_CLASS_OFF, SIZE_CLASS_REGULAR);
    write_u64(obj, VERTICAL_SIZE_CLASS_OFF, SIZE_CLASS_REGULAR);
    write_u64(obj, USER_INTERFACE_IDIOM_OFF, UI_IDIOM_MAC);
    write_f64(obj, DISPLAY_SCALE_OFF, 1.0);
    obj
}

// ── Class method shims ──────────────────────────────────────────

/// `+[NSTraitCollection traitCollectionWithHorizontalSizeClass:]`
pub extern "C" fn shim_nstraitcollection_with_horizontal_size_class(
    _cls: u64, _cmd: u64, size_class: u64,
) -> u64 {
    let obj = make_default_trait_collection();
    write_u64(obj, HORIZONTAL_SIZE_CLASS_OFF, size_class);
    obj
}

/// `+[NSTraitCollection traitCollectionWithVerticalSizeClass:]`
pub extern "C" fn shim_nstraitcollection_with_vertical_size_class(
    _cls: u64, _cmd: u64, size_class: u64,
) -> u64 {
    let obj = make_default_trait_collection();
    write_u64(obj, VERTICAL_SIZE_CLASS_OFF, size_class);
    obj
}

/// `+[NSTraitCollection traitCollectionWithUserInterfaceIdiom:]`
pub extern "C" fn shim_nstraitcollection_with_user_interface_idiom(
    _cls: u64, _cmd: u64, idiom: u64,
) -> u64 {
    let obj = make_default_trait_collection();
    write_u64(obj, USER_INTERFACE_IDIOM_OFF, idiom);
    obj
}

/// `+[NSTraitCollection traitCollectionWithDisplayScale:]`
pub extern "C" fn shim_nstraitcollection_with_display_scale(
    _cls: u64, _cmd: u64, scale: f64,
) -> u64 {
    let obj = make_default_trait_collection();
    write_f64(obj, DISPLAY_SCALE_OFF, scale);
    obj
}

/// `+[NSTraitCollection currentTraitCollection]`
pub extern "C" fn shim_nstraitcollection_current(_cls: u64, _cmd: u64) -> u64 {
    make_default_trait_collection()
}

// ── Instance method shims ───────────────────────────────────────

/// `-[NSTraitCollection horizontalSizeClass]`
pub extern "C" fn shim_nstraitcollection_horizontal_size_class(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, HORIZONTAL_SIZE_CLASS_OFF)
}

/// `-[NSTraitCollection verticalSizeClass]`
pub extern "C" fn shim_nstraitcollection_vertical_size_class(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, VERTICAL_SIZE_CLASS_OFF)
}

/// `-[NSTraitCollection userInterfaceIdiom]`
pub extern "C" fn shim_nstraitcollection_user_interface_idiom(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, USER_INTERFACE_IDIOM_OFF)
}

/// `-[NSTraitCollection displayScale]`
pub extern "C" fn shim_nstraitcollection_display_scale(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, DISPLAY_SCALE_OFF)
}

/// `-[NSTraitCollection hasDifferentColorAppearanceComparedToTraitCollection:]`
pub extern "C" fn shim_nstraitcollection_has_different_color_appearance(
    _obj: u64, _cmd: u64, _other: u64,
) -> u8 {
    0 // always same in our shim
}

// ── NSView traitCollection accessor ─────────────────────────────

/// `-[NSView traitCollection]` — returns default macOS desktop traits.
pub extern "C" fn shim_nsview_trait_collection(_obj: u64, _cmd: u64) -> u64 {
    make_default_trait_collection()
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn tc_isa() -> u64 {
        crate::nsobj::nstraitcollection_class_addr()
    }

    #[test]
    fn default_trait_collection_fields() {
        let tc = make_default_trait_collection();
        assert_ne!(tc, 0);
        assert_eq!(unsafe { *(tc as *const u64) }, tc_isa());
        assert_eq!(shim_nstraitcollection_horizontal_size_class(tc, 0), SIZE_CLASS_REGULAR);
        assert_eq!(shim_nstraitcollection_vertical_size_class(tc, 0), SIZE_CLASS_REGULAR);
        assert_eq!(shim_nstraitcollection_user_interface_idiom(tc, 0), UI_IDIOM_MAC);
        assert!((shim_nstraitcollection_display_scale(tc, 0) - 1.0).abs() < f64::EPSILON);
        free_obj(tc);
    }

    #[test]
    fn factory_horizontal_size_class() {
        let tc = shim_nstraitcollection_with_horizontal_size_class(0, 0, SIZE_CLASS_COMPACT);
        assert_eq!(shim_nstraitcollection_horizontal_size_class(tc, 0), SIZE_CLASS_COMPACT);
        assert_eq!(shim_nstraitcollection_vertical_size_class(tc, 0), SIZE_CLASS_REGULAR);
        free_obj(tc);
    }

    #[test]
    fn factory_vertical_size_class() {
        let tc = shim_nstraitcollection_with_vertical_size_class(0, 0, SIZE_CLASS_COMPACT);
        assert_eq!(shim_nstraitcollection_vertical_size_class(tc, 0), SIZE_CLASS_COMPACT);
        free_obj(tc);
    }

    #[test]
    fn factory_user_interface_idiom() {
        let tc = shim_nstraitcollection_with_user_interface_idiom(0, 0, UI_IDIOM_MAC);
        assert_eq!(shim_nstraitcollection_user_interface_idiom(tc, 0), UI_IDIOM_MAC);
        free_obj(tc);
    }

    #[test]
    fn factory_display_scale() {
        let tc = shim_nstraitcollection_with_display_scale(0, 0, 2.0);
        assert!((shim_nstraitcollection_display_scale(tc, 0) - 2.0).abs() < f64::EPSILON);
        free_obj(tc);
    }

    #[test]
    fn current_trait_collection_returns_default() {
        let tc = shim_nstraitcollection_current(0, 0);
        assert_ne!(tc, 0);
        assert_eq!(shim_nstraitcollection_horizontal_size_class(tc, 0), SIZE_CLASS_REGULAR);
        free_obj(tc);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstraitcollection_horizontal_size_class(0, 0), 0);
        assert_eq!(shim_nstraitcollection_vertical_size_class(0, 0), 0);
        assert_eq!(shim_nstraitcollection_user_interface_idiom(0, 0), 0);
        assert_eq!(shim_nstraitcollection_display_scale(0, 0), 0.0);
    }

    #[test]
    fn has_different_color_appearance_always_false() {
        let tc = make_default_trait_collection();
        assert_eq!(shim_nstraitcollection_has_different_color_appearance(tc, 0, tc), 0);
        free_obj(tc);
    }

    #[test]
    fn view_trait_collection_returns_default() {
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 100.0);
        let tc = shim_nsview_trait_collection(view, 0);
        assert_ne!(tc, 0);
        assert_eq!(shim_nstraitcollection_horizontal_size_class(tc, 0), SIZE_CLASS_REGULAR);
        assert_eq!(shim_nstraitcollection_user_interface_idiom(tc, 0), UI_IDIOM_MAC);
        free_obj(tc);
        free_obj(view);
    }
}
