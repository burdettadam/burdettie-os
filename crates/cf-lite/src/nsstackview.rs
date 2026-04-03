//! NSStackView shim: auto-layout stack container for arranging views.
//!
//! Inherits from NSView (80 bytes).
//!
//! Object layout (88 bytes):
//!
//!   offset  0: isa                  (u64 — inherited from NSView)
//!   [0..80): inherited NSView fields
//!   offset 80: spacing              (f64 — default 8.0)
//!   offset 88: flags                (u64 — bit 0: orientation
//!                                            0=horizontal, 1=vertical
//!                                          bits 1-2: distribution
//!                                            0=fill, 1=fillEqually,
//!                                            2=fillProportionally, 3=equalSpacing
//!                                          bits 3-4: alignment)
//!
//! Total: 96 bytes.
//!
//! Arranged subview list is stored in a global map keyed by stack-view pointer.

use std::collections::HashMap;
use std::sync::Mutex;

const SPACING_OFF: usize = 80;
const FLAGS_OFF: usize = 88;
pub(crate) const STACK_VIEW_SIZE: usize = 96;

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

// ── Arranged subview tracking ──────────────────────────────────

static ARRANGED_SUBVIEWS: Mutex<Option<HashMap<u64, Vec<u64>>>> = Mutex::new(None);

fn with_arranged<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<u64, Vec<u64>>) -> R,
{
    let mut guard = ARRANGED_SUBVIEWS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

/// Reset arranged-subview state (for test isolation).
#[cfg(test)]
fn reset_arranged() {
    let mut guard = ARRANGED_SUBVIEWS.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
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

/// `-[NSStackView addArrangedSubview:]` — tracks the view and adds it as a subview.
pub extern "C" fn shim_nsstackview_add_arranged_subview(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 || view == 0 {
        return;
    }
    with_arranged(|map| {
        let list = map.entry(obj).or_insert_with(Vec::new);
        if !list.contains(&view) {
            list.push(view);
        }
    });
    // Also add as a regular subview.
    crate::nsview::shim_nsview_add_subview(obj, 0, view);
    crate::layout_solver::set_needs_layout(obj, true);
}

/// `-[NSStackView removeArrangedSubview:]` — removes from arranged tracking (not from subviews).
pub extern "C" fn shim_nsstackview_remove_arranged_subview(obj: u64, _cmd: u64, view: u64) {
    if obj == 0 || view == 0 {
        return;
    }
    with_arranged(|map| {
        if let Some(list) = map.get_mut(&obj) {
            list.retain(|&v| v != view);
        }
    });
    crate::layout_solver::set_needs_layout(obj, true);
}

/// `-[NSStackView arrangedSubviews]` — returns NSArray of tracked arranged subviews.
pub extern "C" fn shim_nsstackview_arranged_subviews(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    with_arranged(|map| {
        if let Some(list) = map.get(&obj) {
            crate::nsarray::make_nsarray(list)
        } else {
            crate::nsarray::make_nsarray(&[])
        }
    })
}

/// `-[NSStackView insertArrangedSubview:atIndex:]`
pub extern "C" fn shim_nsstackview_insert_arranged_subview(obj: u64, _cmd: u64, view: u64, index: u64) {
    if obj == 0 || view == 0 {
        return;
    }
    with_arranged(|map| {
        let list = map.entry(obj).or_insert_with(Vec::new);
        let idx = (index as usize).min(list.len());
        if !list.contains(&view) {
            list.insert(idx, view);
        }
    });
    crate::nsview::shim_nsview_add_subview(obj, 0, view);
    crate::layout_solver::set_needs_layout(obj, true);
}

/// Perform stack layout: generate constraints from arranged subviews, orientation, spacing.
///
/// Called during layout pass. Creates sequential leading→trailing constraints between
/// arranged subviews with the configured spacing.
pub(crate) fn perform_stack_layout(obj: u64) {
    if obj == 0 {
        return;
    }

    let arranged: Vec<u64> = with_arranged(|map| {
        map.get(&obj).cloned().unwrap_or_default()
    });
    if arranged.is_empty() {
        return;
    }

    let orientation = shim_nsstackview_orientation(obj, 0);
    let spacing = shim_nsstackview_spacing(obj, 0);
    let distribution = shim_nsstackview_distribution(obj, 0);

    // Attribute indices based on orientation.
    let leading_attr: u64 = if orientation == 1 { 3 } else { 1 };  // Top / Left
    let trailing_attr: u64 = if orientation == 1 { 4 } else { 2 }; // Bottom / Right
    let size_attr: u64 = if orientation == 1 { 8 } else { 7 };     // Height / Width

    // 1. Pin first arranged subview's leading to stack view's leading.
    let first = arranged[0];
    let c_lead = crate::nslayoutconstraint::shim_nslayoutconstraint_create(
        0, 0, first, leading_attr, 0, obj, leading_attr, 1.0, 0.0,
    );
    if c_lead != 0 {
        crate::nslayoutconstraint::shim_nslayoutconstraint_set_active(c_lead, 0, 1);
        crate::layout_solver::add_constraint(obj, c_lead);
    }

    // 2. Chain each pair with spacing.
    for i in 1..arranged.len() {
        let prev = arranged[i - 1];
        let curr = arranged[i];
        // curr.leading = prev.trailing + spacing
        let c = crate::nslayoutconstraint::shim_nslayoutconstraint_create(
            0, 0, curr, leading_attr, 0, prev, trailing_attr, 1.0, spacing,
        );
        if c != 0 {
            crate::nslayoutconstraint::shim_nslayoutconstraint_set_active(c, 0, 1);
            crate::layout_solver::add_constraint(obj, c);
        }
    }

    // 3. Pin last arranged subview's trailing to stack view's trailing.
    let last = *arranged.last().unwrap();
    let c_trail = crate::nslayoutconstraint::shim_nslayoutconstraint_create(
        0, 0, obj, trailing_attr, 0, last, trailing_attr, 1.0, 0.0,
    );
    if c_trail != 0 {
        crate::nslayoutconstraint::shim_nslayoutconstraint_set_active(c_trail, 0, 1);
        crate::layout_solver::add_constraint(obj, c_trail);
    }

    // 4. fillEqually: all arranged subviews same size.
    if distribution == 1 && arranged.len() > 1 {
        for i in 1..arranged.len() {
            // arranged[i].size = arranged[0].size
            let c = crate::nslayoutconstraint::shim_nslayoutconstraint_create(
                0, 0, arranged[i], size_attr, 0, arranged[0], size_attr, 1.0, 0.0,
            );
            if c != 0 {
                crate::nslayoutconstraint::shim_nslayoutconstraint_set_active(c, 0, 1);
                crate::layout_solver::add_constraint(obj, c);
            }
        }
    }
}

// ── Class method shims ─────────────────────────────────────────

/// `+[NSStackView stackViewWithViews:]` — allocate a new stack view with default spacing 8.0.
pub extern "C" fn shim_nsstackview_stack_view_with_views(_cls: u64, _cmd: u64, views: u64) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, STACK_VIEW_SIZE);
    if ptr.is_null() {
        return 0;
    }
    let obj = ptr as u64;
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsstackview_class_addr();
    }
    write_f64(obj, SPACING_OFF, 8.0);

    // Add each view from the NSArray as an arranged subview.
    if views != 0 {
        let count = crate::nsarray::shim_nsarray_count(views, 0);
        for i in 0..count {
            let view = crate::nsarray::shim_nsarray_object_at_index(views, 0, i);
            if view != 0 {
                shim_nsstackview_add_arranged_subview(obj, 0, view);
            }
        }
    }

    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    /// Serialise stack-view tests — they share a process-global `ARRANGED_SUBVIEWS`.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    fn add_arranged_subview_tracks() {
        let _lock = TEST_LOCK.lock();
        reset_arranged();

        let sv = alloc_obj(STACK_VIEW_SIZE, crate::nsobj::nsstackview_class_addr());
        let v1 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);
        let v2 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);

        shim_nsstackview_add_arranged_subview(sv, 0, v1);
        shim_nsstackview_add_arranged_subview(sv, 0, v2);

        let arr = shim_nsstackview_arranged_subviews(sv, 0);
        let count = crate::nsarray::shim_nsarray_count(arr, 0);
        assert_eq!(count, 2);

        let first = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        let second = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 1);
        assert_eq!(first, v1);
        assert_eq!(second, v2);

        reset_arranged();
        free_obj(sv);
    }

    #[test]
    fn remove_arranged_subview_untracks() {
        let _lock = TEST_LOCK.lock();
        reset_arranged();

        let sv = alloc_obj(STACK_VIEW_SIZE, crate::nsobj::nsstackview_class_addr());
        let v1 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);
        let v2 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);

        shim_nsstackview_add_arranged_subview(sv, 0, v1);
        shim_nsstackview_add_arranged_subview(sv, 0, v2);
        shim_nsstackview_remove_arranged_subview(sv, 0, v1);

        let arr = shim_nsstackview_arranged_subviews(sv, 0);
        let count = crate::nsarray::shim_nsarray_count(arr, 0);
        assert_eq!(count, 1);

        let first = crate::nsarray::shim_nsarray_object_at_index(arr, 0, 0);
        assert_eq!(first, v2);

        reset_arranged();
        free_obj(sv);
    }

    #[test]
    fn factory_sets_correct_isa() {
        let _lock = TEST_LOCK.lock();
        reset_arranged();

        let views_array = crate::nsarray::make_nsarray(&[]);
        let sv = shim_nsstackview_stack_view_with_views(0, 0, views_array);
        assert_ne!(sv, 0);

        let isa = unsafe { *(sv as *const u64) };
        assert_eq!(isa, crate::nsobj::nsstackview_class_addr(),
            "ISA should be nsstackview, not nsview");

        assert!((shim_nsstackview_spacing(sv, 0) - 8.0).abs() < f64::EPSILON);

        reset_arranged();
    }

    #[test]
    fn perform_stack_layout_generates_constraints() {
        let _lock = TEST_LOCK.lock();
        reset_arranged();
        crate::layout_solver::reset_global_state();

        let sv = alloc_obj(STACK_VIEW_SIZE, crate::nsobj::nsstackview_class_addr());
        // Set frame for the stack view.
        unsafe {
            *((sv as usize + 24) as *mut f64) = 0.0;  // frame_x
            *((sv as usize + 32) as *mut f64) = 0.0;  // frame_y
            *((sv as usize + 40) as *mut f64) = 300.0; // frame_w
            *((sv as usize + 48) as *mut f64) = 100.0; // frame_h
        }
        write_f64(sv, SPACING_OFF, 10.0);

        let v1 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);
        let v2 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 50.0, 25.0);

        shim_nsstackview_add_arranged_subview(sv, 0, v1);
        shim_nsstackview_add_arranged_subview(sv, 0, v2);

        perform_stack_layout(sv);

        // Should have added constraints to the stack view.
        let constraints = crate::layout_solver::get_constraints(sv);
        let count = crate::nsarray::shim_nsarray_count(constraints, 0);
        // Expect: first.leading=sv.leading, v2.leading=v1.trailing+spacing, sv.trailing=v2.trailing
        assert!(count >= 3, "expected >= 3 constraints, got {count}");

        reset_arranged();
        crate::layout_solver::reset_global_state();
    }

    #[test]
    fn class_factory_default_spacing() {
        let _lock = TEST_LOCK.lock();
        reset_arranged();

        let sv = shim_nsstackview_stack_view_with_views(0, 0, 0);
        assert_ne!(sv, 0, "factory should allocate an object");
        assert_eq!(shim_nsstackview_spacing(sv, 0), 8.0);
        assert_eq!(shim_nsstackview_orientation(sv, 0), 0); // horizontal default
        assert_eq!(shim_nsstackview_distribution(sv, 0), 0); // fill default

        reset_arranged();
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
