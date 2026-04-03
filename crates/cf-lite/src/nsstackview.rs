//! NSStackView shim: auto-layout stack container for arranging views.
//!
//! Inherits from NSView (72 bytes).
//!
//! Object layout (88 bytes):
//!
//!   offset  0: isa                  (u64 — inherited from NSView)
//!   [0..72): inherited NSView fields
//!   offset 72: spacing              (f64 — default 8.0)
//!   offset 80: flags                (u64 — bit 0: orientation
//!                                            0=horizontal, 1=vertical
//!                                          bits 1-2: distribution
//!                                            0=fill, 1=fillEqually,
//!                                            2=fillProportionally, 3=equalSpacing
//!                                          bits 3-4: alignment)
//!
//! Total: 88 bytes.

const SPACING_OFF: usize = 72;
const FLAGS_OFF: usize = 80;
pub(crate) const STACK_VIEW_SIZE: usize = 88;

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

fn read_f64(obj: u64, offset: usize) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + offset) as *const f64) }
}

fn write_f64(obj: u64, offset: usize, value: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut f64) = value;
    }
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSStackView orientation]`
pub extern "C" fn shim_nsstackview_orientation(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_u64(obj, FLAGS_OFF) & 1
}

/// `-[NSStackView setOrientation:]`
pub extern "C" fn shim_nsstackview_set_orientation(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    write_u64(obj, FLAGS_OFF, (flags & !1) | (val & 1));
}

/// `-[NSStackView spacing]`
pub extern "C" fn shim_nsstackview_spacing(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, SPACING_OFF)
}

/// `-[NSStackView setSpacing:]`
pub extern "C" fn shim_nsstackview_set_spacing(obj: u64, _cmd: u64, val: f64) {
    write_f64(obj, SPACING_OFF, val);
}

/// `-[NSStackView distribution]`
pub extern "C" fn shim_nsstackview_distribution(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    (read_u64(obj, FLAGS_OFF) >> 1) & 0x3
}

/// `-[NSStackView setDistribution:]`
pub extern "C" fn shim_nsstackview_set_distribution(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    let cleared = flags & !0b110; // clear bits 1-2
    write_u64(obj, FLAGS_OFF, cleared | ((val & 0x3) << 1));
}

/// `-[NSStackView alignment]`
pub extern "C" fn shim_nsstackview_alignment(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    (read_u64(obj, FLAGS_OFF) >> 3) & 0x3
}

/// `-[NSStackView setAlignment:]`
pub extern "C" fn shim_nsstackview_set_alignment(obj: u64, _cmd: u64, val: u64) {
    if obj == 0 {
        return;
    }
    let flags = read_u64(obj, FLAGS_OFF);
    let cleared = flags & !(0x3 << 3); // clear bits 3-4
    write_u64(obj, FLAGS_OFF, cleared | ((val & 0x3) << 3));
}

/// `-[NSStackView addArrangedSubview:]` — no-op stub.
pub extern "C" fn shim_nsstackview_add_arranged_subview(_obj: u64, _cmd: u64, _view: u64) {}

/// `-[NSStackView removeArrangedSubview:]` — no-op stub.
pub extern "C" fn shim_nsstackview_remove_arranged_subview(_obj: u64, _cmd: u64, _view: u64) {}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSStackView stackViewWithViews:]` — allocate a new stack view with default spacing 8.0.
pub extern "C" fn shim_nsstackview_stack_view_with_views(_cls: u64, _cmd: u64, _views: u64) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, STACK_VIEW_SIZE);
    if ptr.is_null() {
        return 0;
    }
    let obj = ptr as u64;
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsview_class_addr();
    }
    write_f64(obj, SPACING_OFF, 8.0);
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsstackview_orientation(0, 0), 0);
        shim_nsstackview_set_orientation(0, 0, 1);
        assert_eq!(shim_nsstackview_spacing(0, 0), 0.0);
        shim_nsstackview_set_spacing(0, 0, 4.0);
        assert_eq!(shim_nsstackview_distribution(0, 0), 0);
        shim_nsstackview_set_distribution(0, 0, 1);
        assert_eq!(shim_nsstackview_alignment(0, 0), 0);
        shim_nsstackview_set_alignment(0, 0, 1);
        shim_nsstackview_add_arranged_subview(0, 0, 0x1234);
        shim_nsstackview_remove_arranged_subview(0, 0, 0x1234);
    }

    #[test]
    fn orientation_toggle() {
        let sv = alloc_obj(STACK_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nsstackview_orientation(sv, 0), 0); // horizontal
        shim_nsstackview_set_orientation(sv, 0, 1);
        assert_eq!(shim_nsstackview_orientation(sv, 0), 1); // vertical
        shim_nsstackview_set_orientation(sv, 0, 0);
        assert_eq!(shim_nsstackview_orientation(sv, 0), 0);
        free_obj(sv);
    }

    #[test]
    fn spacing_round_trip() {
        let sv = alloc_obj(STACK_VIEW_SIZE, 0xDEAD);
        // calloc zeros memory, so default is 0.0 for raw alloc
        shim_nsstackview_set_spacing(sv, 0, 12.5);
        assert_eq!(shim_nsstackview_spacing(sv, 0), 12.5);
        shim_nsstackview_set_spacing(sv, 0, 0.0);
        assert_eq!(shim_nsstackview_spacing(sv, 0), 0.0);
        free_obj(sv);
    }

    #[test]
    fn distribution_round_trip() {
        let sv = alloc_obj(STACK_VIEW_SIZE, 0xDEAD);
        assert_eq!(shim_nsstackview_distribution(sv, 0), 0); // fill
        shim_nsstackview_set_distribution(sv, 0, 2); // fillProportionally
        assert_eq!(shim_nsstackview_distribution(sv, 0), 2);
        shim_nsstackview_set_distribution(sv, 0, 3); // equalSpacing
        assert_eq!(shim_nsstackview_distribution(sv, 0), 3);
        free_obj(sv);
    }

    #[test]
    fn class_factory_default_spacing() {
        let sv = shim_nsstackview_stack_view_with_views(0, 0, 0);
        assert_ne!(sv, 0, "factory should allocate an object");
        assert_eq!(shim_nsstackview_spacing(sv, 0), 8.0);
        assert_eq!(shim_nsstackview_orientation(sv, 0), 0); // horizontal default
        assert_eq!(shim_nsstackview_distribution(sv, 0), 0); // fill default
        free_obj(sv);
    }

    #[test]
    fn flags_fields_independent() {
        let sv = alloc_obj(STACK_VIEW_SIZE, 0xDEAD);
        shim_nsstackview_set_orientation(sv, 0, 1);
        shim_nsstackview_set_distribution(sv, 0, 2);
        shim_nsstackview_set_alignment(sv, 0, 3);
        // Each field should be independently stored
        assert_eq!(shim_nsstackview_orientation(sv, 0), 1);
        assert_eq!(shim_nsstackview_distribution(sv, 0), 2);
        assert_eq!(shim_nsstackview_alignment(sv, 0), 3);
        // Changing one shouldn't clobber the others
        shim_nsstackview_set_orientation(sv, 0, 0);
        assert_eq!(shim_nsstackview_distribution(sv, 0), 2);
        assert_eq!(shim_nsstackview_alignment(sv, 0), 3);
        free_obj(sv);
    }
}
