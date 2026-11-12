//! NSAccessibility protocol shims.
//!
//! Provides the basic `NSAccessibility` informal-protocol methods that
//! AppKit objects respond to. These are registered on NSView and subclasses
//! so guest code calling `accessibilityRole`, `accessibilityLabel`, etc.
//! gets meaningful responses.

use crate::accessibility_bridge;
use crate::cfstring;
use crate::nsobj;

// ── Helper: ISA reader ─────────────────────────────────────────────

fn read_isa(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *(obj as *const u64) }
}

// ── accessibilityRole ──────────────────────────────────────────────

/// `-[NSView accessibilityRole]` — returns an NSString like "AXButton".
pub extern "C" fn shim_accessibility_role(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let isa = read_isa(obj);
    let role_str =
        if isa == nsobj::nsbutton_class_addr() || isa == nsobj::nspopupbutton_class_addr() {
            "AXButton"
        } else if isa == nsobj::nstextfield_class_addr() {
            "AXTextField"
        } else if isa == nsobj::nstextview_class_addr() {
            "AXTextArea"
        } else if isa == nsobj::nsimageview_class_addr() {
            "AXImage"
        } else if isa == nsobj::nsslider_class_addr() {
            "AXSlider"
        } else if isa == nsobj::nsprogressindicator_class_addr() {
            "AXProgressIndicator"
        } else if isa == nsobj::nsscrollview_class_addr() {
            "AXScrollArea"
        } else if isa == nsobj::nstableview_class_addr() {
            "AXTable"
        } else if isa == nsobj::nssegmentedcontrol_class_addr() {
            "AXRadioGroup"
        } else if isa == nsobj::nscombobox_class_addr() {
            "AXComboBox"
        } else if isa == nsobj::nswindow_class_addr() {
            "AXWindow"
        } else {
            "AXGroup"
        };

    let c_str = std::ffi::CString::new(role_str).unwrap_or_default();
    cfstring::alloc_nsstring_from_cstr(c_str.as_ptr())
}

// ── accessibilityRoleDescription ───────────────────────────────────

/// `-[NSView accessibilityRoleDescription]`
pub extern "C" fn shim_accessibility_role_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let isa = read_isa(obj);
    let desc = if isa == nsobj::nsbutton_class_addr() || isa == nsobj::nspopupbutton_class_addr() {
        "button"
    } else if isa == nsobj::nstextfield_class_addr() {
        "text field"
    } else if isa == nsobj::nstextview_class_addr() {
        "text area"
    } else if isa == nsobj::nsimageview_class_addr() {
        "image"
    } else if isa == nsobj::nsslider_class_addr() {
        "slider"
    } else if isa == nsobj::nsscrollview_class_addr() {
        "scroll area"
    } else if isa == nsobj::nstableview_class_addr() {
        "table"
    } else if isa == nsobj::nswindow_class_addr() {
        "window"
    } else {
        "group"
    };
    let c_str = std::ffi::CString::new(desc).unwrap_or_default();
    cfstring::alloc_nsstring_from_cstr(c_str.as_ptr())
}

// ── accessibilityLabel ─────────────────────────────────────────────

/// `-[NSView accessibilityLabel]` — returns the view's accessible name.
pub extern "C" fn shim_accessibility_label(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    if let Some(label) = accessibility_bridge::label_for_view(obj) {
        let c_str = std::ffi::CString::new(label).unwrap_or_default();
        cfstring::alloc_nsstring_from_cstr(c_str.as_ptr())
    } else {
        0
    }
}

// ── accessibilityValue ─────────────────────────────────────────────

/// `-[NSView accessibilityValue]` — returns the current value for controls.
pub extern "C" fn shim_accessibility_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // For text fields / controls, the value is stringValue (offset 80).
    let isa = read_isa(obj);
    if isa == nsobj::nstextfield_class_addr()
        || isa == nsobj::nscontrol_class_addr()
        || isa == nsobj::nscombobox_class_addr()
    {
        let sv = unsafe { *((obj as usize + 80) as *const u64) };
        return sv; // NSString* or 0
    }
    0
}

// ── isAccessibilityElement ─────────────────────────────────────────

/// `-[NSView isAccessibilityElement]` — always YES for views.
pub extern "C" fn shim_is_accessibility_element(_obj: u64, _cmd: u64) -> u8 {
    1 // YES
}

// ── accessibilityChildren ──────────────────────────────────────────

/// `-[NSView accessibilityChildren]` — returns subviews as NSArray.
pub extern "C" fn shim_accessibility_children(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let children = crate::nsview::snapshot_subviews_pub(obj);
    crate::nsarray::make_nsarray(&children)
}

// ── accessibilityParent ────────────────────────────────────────────

/// `-[NSView accessibilityParent]` — returns superview.
pub extern "C" fn shim_accessibility_parent(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    crate::nsview::shim_nsview_superview(obj, 0)
}

// ── accessibilityFrame ─────────────────────────────────────────────

/// `-[NSView accessibilityFrame]` — returns frame as CGRect (4 doubles).
/// Note: ObjC returns NSRect by value (4 × f64). We return via stret convention.
pub extern "C" fn shim_accessibility_frame(obj: u64, _cmd: u64) -> (f64, f64, f64, f64) {
    if obj == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let frame = crate::nsview::view_frame(obj);
    (
        frame.origin.x,
        frame.origin.y,
        frame.size.width,
        frame.size.height,
    )
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nsview::VIEW_SIZE;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_returns_zero() {
        assert_eq!(shim_accessibility_role(0, 0), 0);
        assert_eq!(shim_accessibility_label(0, 0), 0);
        assert_eq!(shim_accessibility_value(0, 0), 0);
        assert_eq!(shim_accessibility_parent(0, 0), 0);
    }

    #[test]
    fn button_role_string() {
        let btn = alloc_obj(112, nsobj::nsbutton_class_addr());
        let role = shim_accessibility_role(btn, 0);
        assert_ne!(role, 0);
        let s = crate::cfstring::extract_cfstring_cstr(role);
        assert_eq!(s, Some("AXButton"));
        free_obj(btn);
    }

    #[test]
    fn view_role_string() {
        let view = alloc_obj(VIEW_SIZE, nsobj::nsview_class_addr());
        let role = shim_accessibility_role(view, 0);
        let s = crate::cfstring::extract_cfstring_cstr(role);
        assert_eq!(s, Some("AXGroup"));
        free_obj(view);
    }

    #[test]
    fn textfield_role_description() {
        let tf = alloc_obj(96, nsobj::nstextfield_class_addr());
        let desc = shim_accessibility_role_description(tf, 0);
        let s = crate::cfstring::extract_cfstring_cstr(desc);
        assert_eq!(s, Some("text field"));
        free_obj(tf);
    }

    #[test]
    fn button_label() {
        let btn = alloc_obj(112, nsobj::nsbutton_class_addr());
        let title = make_string("OK");
        write_field(btn, 80, title); // TITLE_OFF
        let label = shim_accessibility_label(btn, 0);
        assert_ne!(label, 0);
        let s = crate::cfstring::extract_cfstring_cstr(label);
        assert_eq!(s, Some("OK"));
        free_obj(btn);
    }

    #[test]
    fn textfield_value() {
        let tf = alloc_obj(96, nsobj::nstextfield_class_addr());
        let sv = make_string("Hello");
        write_field(tf, 80, sv); // STRING_VALUE_OFF
        let val = shim_accessibility_value(tf, 0);
        assert_eq!(val, sv);
        free_obj(tf);
    }

    #[test]
    fn is_element_always_true() {
        let view = alloc_obj(VIEW_SIZE, nsobj::nsview_class_addr());
        assert_eq!(shim_is_accessibility_element(view, 0), 1);
        free_obj(view);
    }

    #[test]
    fn parent_returns_superview() {
        let parent = alloc_obj(VIEW_SIZE, nsobj::nsview_class_addr());
        let child = alloc_obj(VIEW_SIZE, nsobj::nsview_class_addr());
        crate::nsview::shim_nsview_add_subview(parent, 0, child);
        assert_eq!(shim_accessibility_parent(child, 0), parent);
        free_obj(child);
        free_obj(parent);
    }
}
