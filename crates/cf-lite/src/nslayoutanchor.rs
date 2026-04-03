//! NSLayoutAnchor shim: layout anchor classes for Auto Layout constraint creation.
//!
//! Covers `NSLayoutAnchor`, `NSLayoutXAxisAnchor`, `NSLayoutYAxisAnchor`, and
//! `NSLayoutDimension`.  All four share a single 24-byte object layout:
//!
//!   offset  0: isa        (u64)
//!   offset  8: item       (u64 — owning view pointer)
//!   offset 16: attribute  (u64 — NSLayoutAttribute enum value)
//!
//! NSLayoutAttribute values:
//!   1=Left, 2=Right, 3=Top, 4=Bottom, 5=Leading, 6=Trailing,
//!   7=Width, 8=Height, 9=CenterX, 10=CenterY
//!
//! NSLayoutRelation values:
//!   -1=LessThanOrEqual (0xFFFFFFFFFFFFFFFF as u64), 0=Equal, 1=GreaterThanOrEqual

const ITEM_OFF: usize = 8;
const ATTR_OFF: usize = 16;
pub(crate) const ANCHOR_SIZE: usize = 24;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + offset) as *mut u64) = value; }
}

// ── Relation constants ─────────────────────────────────────────

const REL_EQ: u64 = 0;
const REL_GE: u64 = 1;
const REL_LE: u64 = (-1i64) as u64;

// ── Internal helpers ───────────────────────────────────────────

fn make_constraint(
    first_item: u64,
    first_attr: u64,
    relation: u64,
    second_item: u64,
    second_attr: u64,
    multiplier: f64,
    constant: f64,
) -> u64 {
    crate::nslayoutconstraint::shim_nslayoutconstraint_create(
        0, 0, first_item, first_attr, relation,
        second_item, second_attr, multiplier, constant,
    )
}

/// Allocate a new anchor object storing the owning view and attribute.
pub(crate) fn make_anchor(item: u64, attribute: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, ANCHOR_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = crate::nsobj::nslayoutanchor_class_addr(); }
    write_u64(obj, ITEM_OFF, item);
    write_u64(obj, ATTR_OFF, attribute);
    obj
}

// ── NSLayoutAnchor constraint methods ──────────────────────────

/// `-[NSLayoutAnchor constraintEqualToAnchor:]`
pub extern "C" fn shim_nslayoutanchor_constraint_eq(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_EQ, other_item, other_attr, 1.0, 0.0)
}

/// `-[NSLayoutAnchor constraintEqualToAnchor:constant:]`
pub extern "C" fn shim_nslayoutanchor_constraint_eq_constant(obj: u64, _cmd: u64, other: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_EQ, other_item, other_attr, 1.0, constant)
}

/// `-[NSLayoutAnchor constraintGreaterThanOrEqualToAnchor:]`
pub extern "C" fn shim_nslayoutanchor_constraint_ge(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_GE, other_item, other_attr, 1.0, 0.0)
}

/// `-[NSLayoutAnchor constraintGreaterThanOrEqualToAnchor:constant:]`
pub extern "C" fn shim_nslayoutanchor_constraint_ge_constant(obj: u64, _cmd: u64, other: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_GE, other_item, other_attr, 1.0, constant)
}

/// `-[NSLayoutAnchor constraintLessThanOrEqualToAnchor:]`
pub extern "C" fn shim_nslayoutanchor_constraint_le(obj: u64, _cmd: u64, other: u64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_LE, other_item, other_attr, 1.0, 0.0)
}

/// `-[NSLayoutAnchor constraintLessThanOrEqualToAnchor:constant:]`
pub extern "C" fn shim_nslayoutanchor_constraint_le_constant(obj: u64, _cmd: u64, other: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_LE, other_item, other_attr, 1.0, constant)
}

// ── NSLayoutDimension extra methods ────────────────────────────

/// `-[NSLayoutDimension constraintEqualToConstant:]`
pub extern "C" fn shim_nslayoutdimension_constraint_eq_constant(obj: u64, _cmd: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_EQ, 0, 0, 1.0, constant)
}

/// `-[NSLayoutDimension constraintGreaterThanOrEqualToConstant:]`
pub extern "C" fn shim_nslayoutdimension_constraint_ge_constant(obj: u64, _cmd: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_GE, 0, 0, 1.0, constant)
}

/// `-[NSLayoutDimension constraintLessThanOrEqualToConstant:]`
pub extern "C" fn shim_nslayoutdimension_constraint_le_constant(obj: u64, _cmd: u64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_LE, 0, 0, 1.0, constant)
}

/// `-[NSLayoutDimension constraintEqualToAnchor:multiplier:]`
pub extern "C" fn shim_nslayoutdimension_constraint_eq_anchor_multiplier(obj: u64, _cmd: u64, other: u64, multiplier: f64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_EQ, other_item, other_attr, multiplier, 0.0)
}

/// `-[NSLayoutDimension constraintEqualToAnchor:multiplier:constant:]`
pub extern "C" fn shim_nslayoutdimension_constraint_eq_anchor_multiplier_constant(obj: u64, _cmd: u64, other: u64, multiplier: f64, constant: f64) -> u64 {
    if obj == 0 { return 0; }
    let other_item = read_u64(other, ITEM_OFF);
    let other_attr = read_u64(other, ATTR_OFF);
    make_constraint(read_u64(obj, ITEM_OFF), read_u64(obj, ATTR_OFF), REL_EQ, other_item, other_attr, multiplier, constant)
}

// ── Accessor shims ─────────────────────────────────────────────

/// `-[NSLayoutAnchor item]` (private API, but used internally)
pub extern "C" fn shim_nslayoutanchor_item(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, ITEM_OFF)
}

// ── NSView anchor accessors ────────────────────────────────────

/// `-[NSView leadingAnchor]`
pub extern "C" fn shim_nsview_leading_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 5)
}

/// `-[NSView trailingAnchor]`
pub extern "C" fn shim_nsview_trailing_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 6)
}

/// `-[NSView leftAnchor]`
pub extern "C" fn shim_nsview_left_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 1)
}

/// `-[NSView rightAnchor]`
pub extern "C" fn shim_nsview_right_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 2)
}

/// `-[NSView topAnchor]`
pub extern "C" fn shim_nsview_top_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 3)
}

/// `-[NSView bottomAnchor]`
pub extern "C" fn shim_nsview_bottom_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 4)
}

/// `-[NSView widthAnchor]`
pub extern "C" fn shim_nsview_width_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 7)
}

/// `-[NSView heightAnchor]`
pub extern "C" fn shim_nsview_height_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 8)
}

/// `-[NSView centerXAnchor]`
pub extern "C" fn shim_nsview_center_x_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 9)
}

/// `-[NSView centerYAnchor]`
pub extern "C" fn shim_nsview_center_y_anchor(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    make_anchor(obj, 10)
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn anchor_isa() -> u64 {
        crate::nsobj::nslayoutanchor_class_addr()
    }

    fn view_isa() -> u64 {
        crate::nsobj::nsview_class_addr()
    }

    const VIEW_SIZE: usize = 80;

    // ── make_anchor ────────────────────────────────────────────

    #[test]
    fn make_anchor_stores_item_and_attr() {
        let view = alloc_obj(VIEW_SIZE, view_isa());
        let anchor = make_anchor(view, 5);
        assert_ne!(anchor, 0);
        assert_eq!(read_u64(anchor, ITEM_OFF), view);
        assert_eq!(read_u64(anchor, ATTR_OFF), 5);
        // ISA should be set
        let isa = unsafe { *(anchor as *const u64) };
        assert_eq!(isa, anchor_isa());
        free_obj(anchor);
        free_obj(view);
    }

    // ── constraint_eq ──────────────────────────────────────────

    #[test]
    fn constraint_eq_creates_constraint() {
        let v1 = alloc_obj(VIEW_SIZE, view_isa());
        let v2 = alloc_obj(VIEW_SIZE, view_isa());
        let a1 = make_anchor(v1, 5); // leading
        let a2 = make_anchor(v2, 5); // leading

        let c = shim_nslayoutanchor_constraint_eq(a1, 0, a2);
        assert_ne!(c, 0);

        // Read constraint fields at known offsets (from nslayoutconstraint.rs)
        let first_item = unsafe { *((c as usize + 8) as *const u64) };
        let first_attr = unsafe { *((c as usize + 16) as *const u64) };
        let relation   = unsafe { *((c as usize + 24) as *const u64) };
        let second_item = unsafe { *((c as usize + 32) as *const u64) };
        let second_attr = unsafe { *((c as usize + 40) as *const u64) };

        assert_eq!(first_item, v1);
        assert_eq!(first_attr, 5);
        assert_eq!(relation, REL_EQ); // 0
        assert_eq!(second_item, v2);
        assert_eq!(second_attr, 5);

        free_obj(c);
        free_obj(a2);
        free_obj(a1);
        free_obj(v2);
        free_obj(v1);
    }

    // ── dimension_eq_constant ──────────────────────────────────

    #[test]
    fn dimension_eq_constant() {
        let view = alloc_obj(VIEW_SIZE, view_isa());
        let anchor = make_anchor(view, 7); // width

        let c = shim_nslayoutdimension_constraint_eq_constant(anchor, 0, 200.0);
        assert_ne!(c, 0);

        let first_item  = unsafe { *((c as usize + 8) as *const u64) };
        let first_attr  = unsafe { *((c as usize + 16) as *const u64) };
        let second_item = unsafe { *((c as usize + 32) as *const u64) };
        let constant    = unsafe { *((c as usize + 56) as *const f64) };

        assert_eq!(first_item, view);
        assert_eq!(first_attr, 7); // width
        assert_eq!(second_item, 0); // no second item
        assert!((constant - 200.0).abs() < f64::EPSILON);

        free_obj(c);
        free_obj(anchor);
        free_obj(view);
    }

    // ── view_anchor_accessors ──────────────────────────────────

    #[test]
    fn view_anchor_accessors() {
        let view = alloc_obj(VIEW_SIZE, view_isa());

        let cases: &[(extern "C" fn(u64, u64) -> u64, u64)] = &[
            (shim_nsview_leading_anchor, 5),
            (shim_nsview_trailing_anchor, 6),
            (shim_nsview_left_anchor, 1),
            (shim_nsview_right_anchor, 2),
            (shim_nsview_top_anchor, 3),
            (shim_nsview_bottom_anchor, 4),
            (shim_nsview_width_anchor, 7),
            (shim_nsview_height_anchor, 8),
            (shim_nsview_center_x_anchor, 9),
            (shim_nsview_center_y_anchor, 10),
        ];

        for &(accessor, expected_attr) in cases {
            let anchor = accessor(view, 0);
            assert_ne!(anchor, 0, "anchor for attr {expected_attr} was nil");
            assert_eq!(read_u64(anchor, ITEM_OFF), view, "item mismatch for attr {expected_attr}");
            assert_eq!(read_u64(anchor, ATTR_OFF), expected_attr, "attr mismatch for attr {expected_attr}");
            free_obj(anchor);
        }

        free_obj(view);
    }

    // ── nil safety ─────────────────────────────────────────────

    #[test]
    fn nil_safety() {
        // All NSLayoutAnchor methods must handle obj=0 gracefully
        assert_eq!(shim_nslayoutanchor_constraint_eq(0, 0, 0), 0);
        assert_eq!(shim_nslayoutanchor_constraint_eq_constant(0, 0, 0, 10.0), 0);
        assert_eq!(shim_nslayoutanchor_constraint_ge(0, 0, 0), 0);
        assert_eq!(shim_nslayoutanchor_constraint_ge_constant(0, 0, 0, 10.0), 0);
        assert_eq!(shim_nslayoutanchor_constraint_le(0, 0, 0), 0);
        assert_eq!(shim_nslayoutanchor_constraint_le_constant(0, 0, 0, 10.0), 0);

        // NSLayoutDimension methods
        assert_eq!(shim_nslayoutdimension_constraint_eq_constant(0, 0, 100.0), 0);
        assert_eq!(shim_nslayoutdimension_constraint_ge_constant(0, 0, 100.0), 0);
        assert_eq!(shim_nslayoutdimension_constraint_le_constant(0, 0, 100.0), 0);
        assert_eq!(shim_nslayoutdimension_constraint_eq_anchor_multiplier(0, 0, 0, 2.0), 0);
        assert_eq!(shim_nslayoutdimension_constraint_eq_anchor_multiplier_constant(0, 0, 0, 2.0, 10.0), 0);

        // Accessor
        assert_eq!(shim_nslayoutanchor_item(0, 0), 0);

        // NSView anchor accessors
        assert_eq!(shim_nsview_leading_anchor(0, 0), 0);
        assert_eq!(shim_nsview_trailing_anchor(0, 0), 0);
        assert_eq!(shim_nsview_left_anchor(0, 0), 0);
        assert_eq!(shim_nsview_right_anchor(0, 0), 0);
        assert_eq!(shim_nsview_top_anchor(0, 0), 0);
        assert_eq!(shim_nsview_bottom_anchor(0, 0), 0);
        assert_eq!(shim_nsview_width_anchor(0, 0), 0);
        assert_eq!(shim_nsview_height_anchor(0, 0), 0);
        assert_eq!(shim_nsview_center_x_anchor(0, 0), 0);
        assert_eq!(shim_nsview_center_y_anchor(0, 0), 0);
    }
}
