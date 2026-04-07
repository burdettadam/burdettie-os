//! NSLayoutConstraint shim: Auto Layout constraint for AppKit/UIKit.
//!
//! Object layout (96 bytes):
//!
//!   offset  0: isa              (u64)
//!   offset  8: first_item       (u64 — first view, or 0)
//!   offset 16: first_attribute  (u64 — NSLayoutAttribute enum value)
//!   offset 24: relation         (u64 — NSLayoutRelation enum value)
//!   offset 32: second_item      (u64 — second view, or 0)
//!   offset 40: second_attribute (u64 — NSLayoutAttribute enum value)
//!   offset 48: multiplier       (f64)
//!   offset 56: constant         (f64)
//!   offset 64: priority         (f64 — 1..1000)
//!   offset 72: active           (u64 — 0 or 1)
//!   offset 80: identifier       (u64 — NSString* or 0)
//!   offset 88: should_be_archived (u64 — 0 or 1)

const FIRST_ITEM_OFF: usize = 8;
const FIRST_ATTR_OFF: usize = 16;
const RELATION_OFF: usize = 24;
const SECOND_ITEM_OFF: usize = 32;
const SECOND_ATTR_OFF: usize = 40;
const MULTIPLIER_OFF: usize = 48;
const CONSTANT_OFF: usize = 56;
const PRIORITY_OFF: usize = 64;
const ACTIVE_OFF: usize = 72;
const IDENTIFIER_OFF: usize = 80;
const SHOULD_ARCHIVE_OFF: usize = 88;
pub(crate) const CONSTRAINT_SIZE: usize = 96;

/// NSView superview offset (must match nsview.rs SUPERVIEW_OFF).
const VIEW_SUPERVIEW_OFF: usize = 8;

/// Walk the superview chain to collect all ancestors (including `view` itself).
fn ancestor_chain(view: u64) -> Vec<u64> {
    let mut chain = Vec::new();
    let mut v = view;
    while v != 0 {
        chain.push(v);
        v = unsafe { *((v as usize + VIEW_SUPERVIEW_OFF) as *const u64) };
    }
    chain
}

/// Find the nearest common ancestor of two views.
/// If `b` is 0 (constant-only constraint), returns `a`.
/// If no common ancestor exists, returns `a` as fallback.
fn nearest_common_ancestor(a: u64, b: u64) -> u64 {
    if a == 0 { return b; }
    if b == 0 { return a; }
    if a == b { return a; }

    let chain_a = ancestor_chain(a);
    let chain_b = ancestor_chain(b);

    // Walk chain_b looking for the first view that's also in chain_a.
    for &v in &chain_b {
        if chain_a.contains(&v) {
            return v;
        }
    }
    // No common ancestor found — fall back to first item.
    a
}

/// Activate a single constraint: set the active flag, find the nearest
/// common ancestor of firstItem/secondItem, and install the constraint
/// into the layout solver on that view.
fn activate_constraint(c: u64) {
    if c == 0 { return; }
    let was_active = read_u64(c, ACTIVE_OFF) != 0;
    write_u64(c, ACTIVE_OFF, 1);

    if !was_active {
        let first = read_u64(c, FIRST_ITEM_OFF);
        let second = read_u64(c, SECOND_ITEM_OFF);
        let host_view = nearest_common_ancestor(first, second);
        if host_view != 0 {
            crate::layout_solver::add_constraint(host_view, c);
        }
    }
}

/// Deactivate a single constraint: clear the active flag and remove
/// from the layout solver.
fn deactivate_constraint(c: u64) {
    if c == 0 { return; }
    let was_active = read_u64(c, ACTIVE_OFF) != 0;
    write_u64(c, ACTIVE_OFF, 0);

    if was_active {
        let first = read_u64(c, FIRST_ITEM_OFF);
        let second = read_u64(c, SECOND_ITEM_OFF);
        let host_view = nearest_common_ancestor(first, second);
        if host_view != 0 {
            crate::layout_solver::remove_constraint(host_view, c);
        }
    }
}

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

/// `+[NSLayoutConstraint constraintWithItem:attribute:relatedBy:toItem:attribute:multiplier:constant:]`
pub extern "C" fn shim_nslayoutconstraint_create(
    _cls: u64,
    _cmd: u64,
    first_item: u64,
    first_attr: u64,
    relation: u64,
    second_item: u64,
    second_attr: u64,
    multiplier: f64,
    constant: f64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CONSTRAINT_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nslayoutconstraint_class_addr(); }
    write_u64(obj, FIRST_ITEM_OFF, first_item);
    write_u64(obj, FIRST_ATTR_OFF, first_attr);
    write_u64(obj, RELATION_OFF, relation);
    write_u64(obj, SECOND_ITEM_OFF, second_item);
    write_u64(obj, SECOND_ATTR_OFF, second_attr);
    write_f64(obj, MULTIPLIER_OFF, multiplier);
    write_f64(obj, CONSTANT_OFF, constant);
    write_f64(obj, PRIORITY_OFF, 1000.0); // required by default
    obj
}

/// `+[NSLayoutConstraint activateConstraints:]`
pub extern "C" fn shim_nslayoutconstraint_activate_constraints(
    _cls: u64,
    _cmd: u64,
    constraints_array: u64,
) {
    if constraints_array == 0 { return; }
    let count = crate::nsarray::shim_nsarray_count(constraints_array, 0);
    for i in 0..count {
        let c = crate::nsarray::shim_nsarray_object_at_index(constraints_array, 0, i);
        activate_constraint(c);
    }
}

/// `+[NSLayoutConstraint deactivateConstraints:]`
pub extern "C" fn shim_nslayoutconstraint_deactivate_constraints(
    _cls: u64,
    _cmd: u64,
    constraints_array: u64,
) {
    if constraints_array == 0 { return; }
    let count = crate::nsarray::shim_nsarray_count(constraints_array, 0);
    for i in 0..count {
        let c = crate::nsarray::shim_nsarray_object_at_index(constraints_array, 0, i);
        deactivate_constraint(c);
    }
}

/// `-[NSLayoutConstraint isActive]`
pub extern "C" fn shim_nslayoutconstraint_is_active(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, ACTIVE_OFF) != 0)
}

/// `-[NSLayoutConstraint setActive:]`
pub extern "C" fn shim_nslayoutconstraint_set_active(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    if flag != 0 {
        activate_constraint(obj);
    } else {
        deactivate_constraint(obj);
    }
}

/// `-[NSLayoutConstraint constant]`
pub extern "C" fn shim_nslayoutconstraint_constant(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, CONSTANT_OFF)
}

/// `-[NSLayoutConstraint setConstant:]`
pub extern "C" fn shim_nslayoutconstraint_set_constant(obj: u64, _cmd: u64, value: f64) {
    write_f64(obj, CONSTANT_OFF, value);
}

/// `-[NSLayoutConstraint multiplier]`
pub extern "C" fn shim_nslayoutconstraint_multiplier(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, MULTIPLIER_OFF)
}

/// `-[NSLayoutConstraint priority]`
pub extern "C" fn shim_nslayoutconstraint_priority(obj: u64, _cmd: u64) -> f64 {
    read_f64(obj, PRIORITY_OFF)
}

/// `-[NSLayoutConstraint setPriority:]`
pub extern "C" fn shim_nslayoutconstraint_set_priority(obj: u64, _cmd: u64, value: f64) {
    let clamped = value.clamp(1.0, 1000.0);
    write_f64(obj, PRIORITY_OFF, clamped);
}

/// `-[NSLayoutConstraint firstItem]`
pub extern "C" fn shim_nslayoutconstraint_first_item(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, FIRST_ITEM_OFF)
}

/// `-[NSLayoutConstraint secondItem]`
pub extern "C" fn shim_nslayoutconstraint_second_item(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SECOND_ITEM_OFF)
}

/// `-[NSLayoutConstraint firstAttribute]`
pub extern "C" fn shim_nslayoutconstraint_first_attribute(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, FIRST_ATTR_OFF)
}

/// `-[NSLayoutConstraint secondAttribute]`
pub extern "C" fn shim_nslayoutconstraint_second_attribute(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, SECOND_ATTR_OFF)
}

/// `-[NSLayoutConstraint relation]`
pub extern "C" fn shim_nslayoutconstraint_relation(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, RELATION_OFF)
}

/// `-[NSLayoutConstraint identifier]`
pub extern "C" fn shim_nslayoutconstraint_identifier(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IDENTIFIER_OFF)
}

/// `-[NSLayoutConstraint setIdentifier:]`
pub extern "C" fn shim_nslayoutconstraint_set_identifier(obj: u64, _cmd: u64, value: u64) {
    write_u64(obj, IDENTIFIER_OFF, value);
}

/// `-[NSLayoutConstraint shouldBeArchived]`
pub extern "C" fn shim_nslayoutconstraint_should_be_archived(obj: u64, _cmd: u64) -> u8 {
    u8::from(read_u64(obj, SHOULD_ARCHIVE_OFF) != 0)
}

/// `-[NSLayoutConstraint setShouldBeArchived:]`
pub extern "C" fn shim_nslayoutconstraint_set_should_be_archived(obj: u64, _cmd: u64, flag: u8) {
    write_u64(obj, SHOULD_ARCHIVE_OFF, u64::from(flag != 0));
}

/// `-[NSView setTranslatesAutoresizingMaskIntoConstraints:]` — stored in flags bit 2.
pub extern "C" fn shim_nsview_set_translates_autoresize(obj: u64, _cmd: u64, flag: u8) {
    if obj == 0 { return; }
    let flags_off = 72; // nsview FLAGS_OFF
    let mut flags = unsafe { *((obj as usize + flags_off) as *const u64) };
    // Bit 2 encodes "translates DISABLED" (inverted: bit=0 means YES, bit=1 means NO)
    if flag != 0 {
        flags &= !(1 << 2); // YES → clear bit
    } else {
        flags |= 1 << 2;    // NO  → set bit
    }
    unsafe { *((obj as usize + flags_off) as *mut u64) = flags; }
}

/// `-[NSView translatesAutoresizingMaskIntoConstraints]` — defaults true (bit 2).
pub extern "C" fn shim_nsview_translates_autoresize(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 { return 1; }
    let flags_off = 72; // nsview FLAGS_OFF
    let flags = unsafe { *((obj as usize + flags_off) as *const u64) };
    // Default is YES when bit is NOT set (Apple convention — translates=YES unless explicitly cleared)
    u8::from(flags & (1 << 2) == 0)
}

/// `-[NSView addConstraint:]` — delegates to Cassowary layout solver.
pub extern "C" fn shim_nsview_add_constraint(obj: u64, _cmd: u64, constraint: u64) {
    crate::layout_solver::add_constraint(obj, constraint);
}

/// `-[NSView addConstraints:]` — delegates to Cassowary layout solver.
pub extern "C" fn shim_nsview_add_constraints(obj: u64, _cmd: u64, constraints: u64) {
    crate::layout_solver::add_constraints(obj, constraints);
}

/// `-[NSView removeConstraint:]` — delegates to Cassowary layout solver.
pub extern "C" fn shim_nsview_remove_constraint(obj: u64, _cmd: u64, constraint: u64) {
    crate::layout_solver::remove_constraint(obj, constraint);
}

/// `-[NSView removeConstraints:]` — delegates to Cassowary layout solver.
pub extern "C" fn shim_nsview_remove_constraints(obj: u64, _cmd: u64, constraints: u64) {
    crate::layout_solver::remove_constraints(obj, constraints);
}

/// `-[NSView constraints]` — returns NSArray of constraints added to this view.
pub extern "C" fn shim_nsview_constraints(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    crate::layout_solver::get_constraints(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn isa() -> u64 {
        crate::nsobj::nslayoutconstraint_class_addr()
    }

    fn make_constraint() -> u64 {
        alloc_obj(CONSTRAINT_SIZE, isa())
    }

    // ── shim_nslayoutconstraint_create ──────────────────────────

    #[test]
    fn create_basic() {
        let view_a = alloc_obj_fake(80);
        let view_b = alloc_obj_fake(80);
        let c = shim_nslayoutconstraint_create(0, 0, view_a, 1, 0, view_b, 1, 1.0, 10.0);
        assert_ne!(c, 0);
        assert_eq!(read_field(c, 0), isa());
        assert_eq!(read_field(c, FIRST_ITEM_OFF), view_a);
        assert_eq!(read_field(c, FIRST_ATTR_OFF), 1);
        assert_eq!(read_field(c, RELATION_OFF), 0);
        assert_eq!(read_field(c, SECOND_ITEM_OFF), view_b);
        assert_eq!(read_field(c, SECOND_ATTR_OFF), 1);
        assert_eq!(shim_nslayoutconstraint_multiplier(c, 0), 1.0);
        assert_eq!(shim_nslayoutconstraint_constant(c, 0), 10.0);
        assert_eq!(shim_nslayoutconstraint_priority(c, 0), 1000.0);
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 0);
        free_obj(c);
        free_obj(view_b);
        free_obj(view_a);
    }

    #[test]
    fn create_with_nil_second() {
        let view_a = alloc_obj_fake(80);
        let c = shim_nslayoutconstraint_create(0, 0, view_a, 7, 0, 0, 0, 1.0, 200.0);
        assert_ne!(c, 0);
        assert_eq!(read_field(c, SECOND_ITEM_OFF), 0);
        assert_eq!(shim_nslayoutconstraint_constant(c, 0), 200.0);
        free_obj(c);
        free_obj(view_a);
    }

    // ── active / setActive ──────────────────────────────────────

    #[test]
    fn active_default_is_no() {
        let c = make_constraint();
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 0);
        free_obj(c);
    }

    #[test]
    fn set_active_yes() {
        let c = make_constraint();
        shim_nslayoutconstraint_set_active(c, 0, 1);
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 1);
        free_obj(c);
    }

    #[test]
    fn set_active_no() {
        let c = make_constraint();
        shim_nslayoutconstraint_set_active(c, 0, 1);
        shim_nslayoutconstraint_set_active(c, 0, 0);
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 0);
        free_obj(c);
    }

    // ── constant / setConstant ──────────────────────────────────

    #[test]
    fn constant_set_get() {
        let c = make_constraint();
        shim_nslayoutconstraint_set_constant(c, 0, 42.5);
        assert_eq!(shim_nslayoutconstraint_constant(c, 0), 42.5);
        free_obj(c);
    }

    // ── priority / setPriority ──────────────────────────────────

    #[test]
    fn priority_clamps_to_range() {
        let c = make_constraint();
        shim_nslayoutconstraint_set_priority(c, 0, 2000.0);
        assert_eq!(shim_nslayoutconstraint_priority(c, 0), 1000.0);
        shim_nslayoutconstraint_set_priority(c, 0, -50.0);
        assert_eq!(shim_nslayoutconstraint_priority(c, 0), 1.0);
        free_obj(c);
    }

    #[test]
    fn priority_normal() {
        let c = make_constraint();
        shim_nslayoutconstraint_set_priority(c, 0, 750.0);
        assert_eq!(shim_nslayoutconstraint_priority(c, 0), 750.0);
        free_obj(c);
    }

    // ── identifier ──────────────────────────────────────────────

    #[test]
    fn identifier_default_nil() {
        let c = make_constraint();
        assert_eq!(shim_nslayoutconstraint_identifier(c, 0), 0);
        free_obj(c);
    }

    #[test]
    fn identifier_set_get() {
        let c = make_constraint();
        let s = make_string("myConstraint");
        shim_nslayoutconstraint_set_identifier(c, 0, s);
        assert_eq!(shim_nslayoutconstraint_identifier(c, 0), s);
        free_obj(c);
    }

    // ── activate/deactivateConstraints ──────────────────────────

    #[test]
    fn activate_constraints_batch() {
        let c1 = make_constraint();
        let c2 = make_constraint();
        let arr = crate::nsarray::make_nsarray(&[c1, c2]);
        shim_nslayoutconstraint_activate_constraints(0, 0, arr);
        assert_eq!(shim_nslayoutconstraint_is_active(c1, 0), 1);
        assert_eq!(shim_nslayoutconstraint_is_active(c2, 0), 1);
        shim_nslayoutconstraint_deactivate_constraints(0, 0, arr);
        assert_eq!(shim_nslayoutconstraint_is_active(c1, 0), 0);
        assert_eq!(shim_nslayoutconstraint_is_active(c2, 0), 0);
        free_obj(c2);
        free_obj(c1);
    }

    #[test]
    fn activate_nil_array_noop() {
        shim_nslayoutconstraint_activate_constraints(0, 0, 0);
        shim_nslayoutconstraint_deactivate_constraints(0, 0, 0);
    }

    // ── nil receiver safety ─────────────────────────────────────

    #[test]
    fn nil_receiver_safety() {
        assert_eq!(shim_nslayoutconstraint_is_active(0, 0), 0);
        assert_eq!(shim_nslayoutconstraint_constant(0, 0), 0.0);
        assert_eq!(shim_nslayoutconstraint_multiplier(0, 0), 0.0);
        assert_eq!(shim_nslayoutconstraint_priority(0, 0), 0.0);
        assert_eq!(shim_nslayoutconstraint_first_item(0, 0), 0);
        assert_eq!(shim_nslayoutconstraint_second_item(0, 0), 0);
        assert_eq!(shim_nslayoutconstraint_identifier(0, 0), 0);
        shim_nslayoutconstraint_set_active(0, 0, 1);
        shim_nslayoutconstraint_set_constant(0, 0, 1.0);
        shim_nslayoutconstraint_set_priority(0, 0, 500.0);
        shim_nslayoutconstraint_set_identifier(0, 0, 0xBEEF);
    }

    // ── translatesAutoresizingMaskIntoConstraints ───────────────

    #[test]
    fn translates_autoresize_default_yes() {
        let view = alloc_obj_fake(80);
        // calloc zeroes flags → bit 2 is 0 → translates returns YES
        assert_eq!(shim_nsview_translates_autoresize(view, 0), 1);
        free_obj(view);
    }

    #[test]
    fn set_translates_autoresize_no() {
        let view = alloc_obj_fake(80);
        shim_nsview_set_translates_autoresize(view, 0, 0);
        assert_eq!(shim_nsview_translates_autoresize(view, 0), 0);
        free_obj(view);
    }

    #[test]
    fn set_translates_autoresize_yes() {
        let view = alloc_obj_fake(80);
        shim_nsview_set_translates_autoresize(view, 0, 0);
        shim_nsview_set_translates_autoresize(view, 0, 1);
        assert_eq!(shim_nsview_translates_autoresize(view, 0), 1);
        free_obj(view);
    }

    // ── NSView constraint stubs ─────────────────────────────────

    #[test]
    fn view_constraints_returns_empty() {
        let view = alloc_obj_fake(80);
        let arr = shim_nsview_constraints(view, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(view);
    }

    #[test]
    fn view_add_remove_constraint_noop() {
        let view = alloc_obj_fake(80);
        let c = make_constraint();
        shim_nsview_add_constraint(view, 0, c);
        shim_nsview_add_constraints(view, 0, 0);
        shim_nsview_remove_constraint(view, 0, c);
        shim_nsview_remove_constraints(view, 0, 0);
        free_obj(c);
        free_obj(view);
    }

    // ── setActive: auto-install ─────────────────────────────────

    /// Serialise tests that touch the process-global layout solver.
    static LAYOUT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn set_active_auto_installs_to_first_item() {
        let _guard = LAYOUT_TEST_LOCK.lock().unwrap();
        crate::layout_solver::reset_global_state();
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 100.0);
        // Width = 200, no second item → installs on firstItem.
        let c = shim_nslayoutconstraint_create(0, 0, view, 7, 0, 0, 0, 1.0, 200.0);
        shim_nslayoutconstraint_set_active(c, 0, 1);
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 1);
        // Should now be in the constraint list of `view`.
        let arr = crate::layout_solver::get_constraints(view);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        free_obj(c);
        free_obj(view);
        crate::layout_solver::reset_global_state();
    }

    #[test]
    fn set_active_auto_installs_to_common_ancestor() {
        let _guard = LAYOUT_TEST_LOCK.lock().unwrap();
        crate::layout_solver::reset_global_state();
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let child = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, child);
        // child.left == parent.left → installs on parent (nearest common ancestor).
        let c = shim_nslayoutconstraint_create(0, 0, child, 1, 0, parent, 1, 1.0, 0.0);
        shim_nslayoutconstraint_set_active(c, 0, 1);
        let arr = crate::layout_solver::get_constraints(parent);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        free_obj(c);
        crate::layout_solver::reset_global_state();
    }

    #[test]
    fn deactivate_removes_from_solver() {
        let _guard = LAYOUT_TEST_LOCK.lock().unwrap();
        crate::layout_solver::reset_global_state();
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 100.0);
        let c = shim_nslayoutconstraint_create(0, 0, view, 7, 0, 0, 0, 1.0, 200.0);
        shim_nslayoutconstraint_set_active(c, 0, 1);
        assert_eq!(crate::nsarray::shim_nsarray_count(crate::layout_solver::get_constraints(view), 0), 1);
        shim_nslayoutconstraint_set_active(c, 0, 0);
        assert_eq!(shim_nslayoutconstraint_is_active(c, 0), 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(crate::layout_solver::get_constraints(view), 0), 0);
        free_obj(c);
        free_obj(view);
        crate::layout_solver::reset_global_state();
    }

    #[test]
    fn activate_constraints_batch_auto_installs() {
        let _guard = LAYOUT_TEST_LOCK.lock().unwrap();
        crate::layout_solver::reset_global_state();
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let child = crate::nsview::alloc_view_with_frame(0.0, 0.0, 0.0, 0.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, child);

        let cw = shim_nslayoutconstraint_create(0, 0, child, 7, 0, 0, 0, 1.0, 100.0);
        let ch = shim_nslayoutconstraint_create(0, 0, child, 8, 0, 0, 0, 1.0, 50.0);
        let arr = crate::nsarray::make_nsarray(&[cw, ch]);
        shim_nslayoutconstraint_activate_constraints(0, 0, arr);

        // Width-only constraint auto-installs on child (firstItem, no secondItem).
        let child_constraints = crate::layout_solver::get_constraints(child);
        assert_eq!(crate::nsarray::shim_nsarray_count(child_constraints, 0), 2);
        free_obj(ch);
        free_obj(cw);
        crate::layout_solver::reset_global_state();
    }

    #[test]
    fn set_active_idempotent() {
        let _guard = LAYOUT_TEST_LOCK.lock().unwrap();
        crate::layout_solver::reset_global_state();
        let view = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 100.0);
        let c = shim_nslayoutconstraint_create(0, 0, view, 7, 0, 0, 0, 1.0, 200.0);
        shim_nslayoutconstraint_set_active(c, 0, 1);
        shim_nslayoutconstraint_set_active(c, 0, 1); // double activate
        // Should still only have 1 constraint, not 2.
        let arr = crate::layout_solver::get_constraints(view);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        free_obj(c);
        free_obj(view);
        crate::layout_solver::reset_global_state();
    }
}
