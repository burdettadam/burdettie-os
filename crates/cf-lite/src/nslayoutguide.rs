//! NSLayoutGuide shim: rectangular guide for Auto Layout.
//!
//! Object layout (24 bytes):
//!
//!   offset  0: isa            (u64)
//!   offset  8: owning_view    (u64 — the view this guide belongs to, or 0)
//!   offset 16: identifier     (u64 — NSString* or 0)
//!
//! Total: 24 bytes.
//!
//! NSLayoutGuide doesn't have a visual representation; it only provides
//! layout anchors that constrain against its owning view.

const OWNING_VIEW_OFF: usize = 8;
const IDENTIFIER_OFF: usize = 16;
pub(crate) const LAYOUT_GUIDE_SIZE: usize = 24;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

/// Allocate a new NSLayoutGuide for the given owning view.
pub(crate) fn make_layout_guide(owning_view: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, LAYOUT_GUIDE_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nslayoutguide_class_addr();
    }
    write_u64(obj, OWNING_VIEW_OFF, owning_view);
    obj
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSLayoutGuide owningView]`
pub extern "C" fn shim_nslayoutguide_owning_view(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, OWNING_VIEW_OFF)
}

/// `-[NSLayoutGuide setOwningView:]` (private/internal)
pub extern "C" fn shim_nslayoutguide_set_owning_view(obj: u64, _cmd: u64, view: u64) {
    write_u64(obj, OWNING_VIEW_OFF, view);
}

/// `-[NSLayoutGuide identifier]`
pub extern "C" fn shim_nslayoutguide_identifier(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IDENTIFIER_OFF)
}

/// `-[NSLayoutGuide setIdentifier:]`
pub extern "C" fn shim_nslayoutguide_set_identifier(obj: u64, _cmd: u64, ident: u64) {
    write_u64(obj, IDENTIFIER_OFF, ident);
}

// ── Anchor accessors ───────────────────────────────────────────
//
// All anchors delegate to the owning view's anchors, making the guide
// a transparent proxy.  If the guide has no owning view, returns nil.

/// `-[NSLayoutGuide leadingAnchor]`
pub extern "C" fn shim_nslayoutguide_leading_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 5)
}

/// `-[NSLayoutGuide trailingAnchor]`
pub extern "C" fn shim_nslayoutguide_trailing_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 6)
}

/// `-[NSLayoutGuide leftAnchor]`
pub extern "C" fn shim_nslayoutguide_left_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 1)
}

/// `-[NSLayoutGuide rightAnchor]`
pub extern "C" fn shim_nslayoutguide_right_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 2)
}

/// `-[NSLayoutGuide topAnchor]`
pub extern "C" fn shim_nslayoutguide_top_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 3)
}

/// `-[NSLayoutGuide bottomAnchor]`
pub extern "C" fn shim_nslayoutguide_bottom_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 4)
}

/// `-[NSLayoutGuide widthAnchor]`
pub extern "C" fn shim_nslayoutguide_width_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 7)
}

/// `-[NSLayoutGuide heightAnchor]`
pub extern "C" fn shim_nslayoutguide_height_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 8)
}

/// `-[NSLayoutGuide centerXAnchor]`
pub extern "C" fn shim_nslayoutguide_center_x_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 9)
}

/// `-[NSLayoutGuide centerYAnchor]`
pub extern "C" fn shim_nslayoutguide_center_y_anchor(obj: u64, _cmd: u64) -> u64 {
    let view = read_u64(obj, OWNING_VIEW_OFF);
    if view == 0 {
        return 0;
    }
    crate::nslayoutanchor::make_anchor(view, 10)
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn guide_isa() -> u64 {
        crate::nsobj::nslayoutguide_class_addr()
    }

    #[test]
    fn make_layout_guide_sets_fields() {
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 200.0, 100.0);
        let guide = make_layout_guide(view);
        assert_ne!(guide, 0);
        assert_eq!(unsafe { *(guide as *const u64) }, guide_isa());
        assert_eq!(shim_nslayoutguide_owning_view(guide, 0), view);
        assert_eq!(shim_nslayoutguide_identifier(guide, 0), 0);
        free_obj(guide);
        free_obj(view);
    }

    #[test]
    fn identifier_round_trip() {
        let guide = alloc_obj(LAYOUT_GUIDE_SIZE, guide_isa());
        shim_nslayoutguide_set_identifier(guide, 0, 0xBEEF);
        assert_eq!(shim_nslayoutguide_identifier(guide, 0), 0xBEEF);
        free_obj(guide);
    }

    #[test]
    fn anchor_accessors_delegate_to_owning_view() {
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 200.0, 100.0);
        let guide = make_layout_guide(view);

        let anchor = shim_nslayoutguide_leading_anchor(guide, 0);
        assert_ne!(anchor, 0);

        // Anchor's item should be the owning view.
        let item = crate::nslayoutanchor::shim_nslayoutanchor_item(anchor, 0);
        assert_eq!(item, view);

        free_obj(anchor);
        free_obj(guide);
        free_obj(view);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nslayoutguide_owning_view(0, 0), 0);
        shim_nslayoutguide_set_owning_view(0, 0, 0x1234);
        assert_eq!(shim_nslayoutguide_identifier(0, 0), 0);
        shim_nslayoutguide_set_identifier(0, 0, 0x5678);
        assert_eq!(shim_nslayoutguide_leading_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_trailing_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_top_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_bottom_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_width_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_height_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_center_x_anchor(0, 0), 0);
        assert_eq!(shim_nslayoutguide_center_y_anchor(0, 0), 0);
    }

    #[test]
    fn no_owning_view_returns_nil_anchors() {
        let guide = alloc_obj(LAYOUT_GUIDE_SIZE, guide_isa());
        // owning_view is 0 (calloc-zeroed).
        assert_eq!(shim_nslayoutguide_leading_anchor(guide, 0), 0);
        assert_eq!(shim_nslayoutguide_width_anchor(guide, 0), 0);
        free_obj(guide);
    }
}
