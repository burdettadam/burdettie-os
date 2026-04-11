//! Cassowary-based Auto Layout solver bridge.
//!
//! Translates `NSLayoutConstraint` objects (raw ObjC shim pointers) into
//! `cassowary::Constraint` values, solves the system, and writes the
//! resulting frames back to `NSView` objects.
//!
//! ## NSLayoutAttribute mapping (Apple values)
//!
//! | Value | Attribute         |
//! |-------|-------------------|
//! | 0     | NotAnAttribute    |
//! | 1     | Left / Leading    |
//! | 2     | Right / Trailing  |
//! | 3     | Top               |
//! | 4     | Bottom            |
//! | 5     | Leading (alias)   |
//! | 6     | Trailing (alias)  |
//! | 7     | Width             |
//! | 8     | Height            |
//! | 9     | CenterX           |
//! | 10    | CenterY           |
//!
//! ## NSLayoutRelation mapping
//!
//! | Value | Relation           |
//! |-------|-------------------|
//! | -1    | LessOrEqual       |
//! |  0    | Equal             |
//! |  1    | GreaterOrEqual    |

use std::collections::HashMap;
use std::sync::Mutex;

use cassowary::strength::{MEDIUM, REQUIRED, STRONG, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Expression, Solver, Variable};

// NSView frame offsets (must match nsview.rs).
const FRAME_X_OFF: usize = 24;
const FRAME_Y_OFF: usize = 32;
const FRAME_W_OFF: usize = 40;
const FRAME_H_OFF: usize = 48;
const FLAGS_OFF: usize = 72;

// NSLayoutConstraint field offsets (must match nslayoutconstraint.rs).
const C_FIRST_ITEM: usize = 8;
const C_FIRST_ATTR: usize = 16;
const C_RELATION: usize = 24;
const C_SECOND_ITEM: usize = 32;
const C_SECOND_ATTR: usize = 40;
const C_MULTIPLIER: usize = 48;
const C_CONSTANT: usize = 56;
const C_PRIORITY: usize = 64;
const C_ACTIVE: usize = 72;

// NSView flags.
const FLAG_NEEDS_LAYOUT: u64 = 1 << 3;

// ── Per-view Cassowary variables ──────────────────────────────

/// Four cassowary variables per view: x, y, width, height.
#[derive(Clone, Copy)]
struct ViewVars {
    x: Variable,
    y: Variable,
    w: Variable,
    h: Variable,
}

impl ViewVars {
    fn new() -> Self {
        Self {
            x: Variable::new(),
            y: Variable::new(),
            w: Variable::new(),
            h: Variable::new(),
        }
    }

    /// Return the Expression for a given `NSLayoutAttribute` integer.
    fn expr_for_attr(&self, attr: u64) -> Option<Expression> {
        match attr {
            1 | 5 => Some(Expression::from(self.x)),              // Left / Leading
            2 | 6 => Some(Expression::from(self.x) + self.w),     // Right / Trailing
            3     => Some(Expression::from(self.y)),               // Top
            4     => Some(Expression::from(self.y) + self.h),      // Bottom
            7     => Some(Expression::from(self.w)),               // Width
            8     => Some(Expression::from(self.h)),               // Height
            9     => Some(Expression::from(self.x) + self.w / 2.0), // CenterX
            10    => Some(Expression::from(self.y) + self.h / 2.0), // CenterY
            _     => None, // NotAnAttribute (0) or unknown
        }
    }
}

// ── Per-view constraint storage ───────────────────────────────

/// Constraint list stored on each NSView (addConstraint:/removeConstraint:).
/// Stored as a `Vec<u64>` of NSLayoutConstraint pointers.
struct ViewConstraintState {
    /// Raw NSLayoutConstraint* pointers added via addConstraint:.
    constraint_ptrs: Vec<u64>,
}

// ── Global layout state ───────────────────────────────────────

struct LayoutState {
    /// Map: NSView* → per-view constraint list.
    view_constraints: HashMap<u64, ViewConstraintState>,
}

static LAYOUT_STATE: Mutex<Option<LayoutState>> = Mutex::new(None);

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut LayoutState) -> R,
{
    let mut guard = LAYOUT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = guard.get_or_insert_with(|| LayoutState {
        view_constraints: HashMap::new(),
    });
    f(state)
}

// ── Raw memory helpers ────────────────────────────────────────

unsafe fn read_u64(ptr: u64, off: usize) -> u64 {
    *((ptr as usize + off) as *const u64)
}

unsafe fn read_f64(ptr: u64, off: usize) -> f64 {
    *((ptr as usize + off) as *const f64)
}

unsafe fn write_f64(ptr: u64, off: usize, val: f64) {
    *((ptr as usize + off) as *mut f64) = val;
}

unsafe fn read_flags(view: u64) -> u64 {
    read_u64(view, FLAGS_OFF)
}

unsafe fn write_flags(view: u64, flags: u64) {
    *((view as usize + FLAGS_OFF) as *mut u64) = flags;
}

// ── Public API for NSView shims ───────────────────────────────

/// `-[NSView addConstraint:]`
pub fn add_constraint(view: u64, constraint: u64) {
    if view == 0 || constraint == 0 {
        return;
    }
    with_state(|state| {
        let entry = state.view_constraints.entry(view).or_insert_with(|| {
            ViewConstraintState {
                constraint_ptrs: Vec::new(),
            }
        });
        if !entry.constraint_ptrs.contains(&constraint) {
            entry.constraint_ptrs.push(constraint);
        }
    });
    mark_needs_layout(view);
}

/// `-[NSView addConstraints:]`
pub fn add_constraints(view: u64, constraints_array: u64) {
    if view == 0 || constraints_array == 0 {
        return;
    }
    let count = crate::nsarray::shim_nsarray_count(constraints_array, 0);
    for i in 0..count {
        let c = crate::nsarray::shim_nsarray_object_at_index(constraints_array, 0, i);
        add_constraint(view, c);
    }
}

/// `-[NSView removeConstraint:]`
pub fn remove_constraint(view: u64, constraint: u64) {
    if view == 0 || constraint == 0 {
        return;
    }
    with_state(|state| {
        if let Some(entry) = state.view_constraints.get_mut(&view) {
            entry.constraint_ptrs.retain(|&c| c != constraint);
        }
    });
    mark_needs_layout(view);
}

/// Remove all constraints stored on a view (used by NSStackView to regenerate).
pub(crate) fn clear_view_constraints(view: u64) {
    if view == 0 {
        return;
    }
    with_state(|state| {
        if let Some(entry) = state.view_constraints.get_mut(&view) {
            entry.constraint_ptrs.clear();
        }
    });
}

/// `-[NSView removeConstraints:]`
pub fn remove_constraints(view: u64, constraints_array: u64) {
    if view == 0 || constraints_array == 0 {
        return;
    }
    let count = crate::nsarray::shim_nsarray_count(constraints_array, 0);
    for i in 0..count {
        let c = crate::nsarray::shim_nsarray_object_at_index(constraints_array, 0, i);
        remove_constraint(view, c);
    }
}

/// `-[NSView constraints]` — returns NSArray of stored constraint pointers.
pub fn get_constraints(view: u64) -> u64 {
    if view == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }
    with_state(|state| {
        if let Some(entry) = state.view_constraints.get(&view) {
            crate::nsarray::make_nsarray(&entry.constraint_ptrs)
        } else {
            crate::nsarray::make_nsarray(&[])
        }
    })
}

/// `-[NSView setNeedsLayout:]`
pub fn set_needs_layout(view: u64, flag: bool) {
    if view == 0 {
        return;
    }
    unsafe {
        let mut flags = read_flags(view);
        if flag {
            flags |= FLAG_NEEDS_LAYOUT;
        } else {
            flags &= !FLAG_NEEDS_LAYOUT;
        }
        write_flags(view, flags);
    }
}

/// `-[NSView needsLayout]`
pub fn needs_layout(view: u64) -> bool {
    if view == 0 {
        return false;
    }
    unsafe { read_flags(view) & FLAG_NEEDS_LAYOUT != 0 }
}

fn mark_needs_layout(view: u64) {
    set_needs_layout(view, true);
}

/// `-[NSView layoutIfNeeded]`
///
/// Collects all active constraints from this view and its descendants,
/// feeds them to a Cassowary solver, and writes the solved frames back.
pub fn layout_if_needed(view: u64) {
    if view == 0 {
        return;
    }
    // Only run if layout is actually needed.
    if !needs_layout(view) {
        return;
    }

    // Collect the full view subtree.
    let mut views = Vec::new();
    collect_subtree(view, &mut views);

    // For any NSStackView in the subtree, regenerate stack-layout constraints
    // before the solver runs so they participate in the solve step.
    let stack_isa = crate::nsobj::nsstackview_class_addr();
    for &v in &views {
        let isa = unsafe { *((v as usize) as *const u64) };
        if isa == stack_isa {
            clear_view_constraints(v);
            crate::nsstackview::perform_stack_layout(v);
        }
    }

    // Collect all active constraints from all views in the subtree.
    let all_constraints: Vec<u64> = with_state(|state| {
        let mut result = Vec::new();
        for &v in &views {
            if let Some(entry) = state.view_constraints.get(&v) {
                for &c in &entry.constraint_ptrs {
                    if unsafe { read_u64(c, C_ACTIVE) } != 0 {
                        result.push(c);
                    }
                }
            }
        }
        result
    });

    if all_constraints.is_empty() {
        // Nothing to solve — clear the flag and return.
        set_needs_layout(view, false);
        return;
    }

    // Build variable map: NSView* → ViewVars.
    let mut var_map: HashMap<u64, ViewVars> = HashMap::new();
    for &v in &views {
        var_map.entry(v).or_insert_with(ViewVars::new);
    }

    // Build cassowary solver.
    let mut solver = Solver::new();

    // Add non-negative width/height constraints for all views.
    for (&_view_ptr, vars) in &var_map {
        let _ = solver.add_constraint(vars.w |GE(REQUIRED)| 0.0);
        let _ = solver.add_constraint(vars.h |GE(REQUIRED)| 0.0);
    }

    // Seed the root view's frame as edit variables (STRONG), so the
    // solver can position children relative to a known container.
    if let Some(root_vars) = var_map.get(&view) {
        let (rx, ry, rw, rh) = unsafe {
            (
                read_f64(view, FRAME_X_OFF),
                read_f64(view, FRAME_Y_OFF),
                read_f64(view, FRAME_W_OFF),
                read_f64(view, FRAME_H_OFF),
            )
        };
        let _ = solver.add_edit_variable(root_vars.x, STRONG);
        let _ = solver.add_edit_variable(root_vars.y, STRONG);
        let _ = solver.add_edit_variable(root_vars.w, STRONG);
        let _ = solver.add_edit_variable(root_vars.h, STRONG);
        let _ = solver.suggest_value(root_vars.x, rx);
        let _ = solver.suggest_value(root_vars.y, ry);
        let _ = solver.suggest_value(root_vars.w, rw);
        let _ = solver.suggest_value(root_vars.h, rh);
    }

    // Translate each NSLayoutConstraint into a cassowary constraint.
    for &c_ptr in &all_constraints {
        if let Some(cass_constraint) = translate_constraint(c_ptr, &mut var_map) {
            let _ = solver.add_constraint(cass_constraint);
        }
    }

    // Read results and write back frames for non-root views.
    let changes: Vec<(Variable, f64)> = solver.fetch_changes().to_vec();
    let change_map: HashMap<Variable, f64> = changes.into_iter().collect();

    for (&view_ptr, vars) in &var_map {
        // Don't overwrite the root view's frame — it was the container input.
        if view_ptr == view {
            continue;
        }
        let x = change_map.get(&vars.x).copied().unwrap_or_else(|| unsafe { read_f64(view_ptr, FRAME_X_OFF) });
        let y = change_map.get(&vars.y).copied().unwrap_or_else(|| unsafe { read_f64(view_ptr, FRAME_Y_OFF) });
        let w = change_map.get(&vars.w).copied().unwrap_or_else(|| unsafe { read_f64(view_ptr, FRAME_W_OFF) });
        let h = change_map.get(&vars.h).copied().unwrap_or_else(|| unsafe { read_f64(view_ptr, FRAME_H_OFF) });
        unsafe {
            write_f64(view_ptr, FRAME_X_OFF, x);
            write_f64(view_ptr, FRAME_Y_OFF, y);
            write_f64(view_ptr, FRAME_W_OFF, w);
            write_f64(view_ptr, FRAME_H_OFF, h);
        }
    }

    // Clear needs-layout for all views in the subtree.
    for &v in &views {
        set_needs_layout(v, false);
    }

    log::debug!(
        "layout_if_needed: solved {} constraints for {} views",
        all_constraints.len(),
        views.len(),
    );
}

// ── Helpers ───────────────────────────────────────────────────

/// Collect all views in the subtree rooted at `view` (BFS).
fn collect_subtree(root: u64, out: &mut Vec<u64>) {
    if root == 0 {
        return;
    }
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(root);
    while let Some(v) = queue.pop_front() {
        out.push(v);
        // Read subview_count and subviews_ptr.
        let count = unsafe { read_u64(v, 64) } as usize; // SUBVIEW_COUNT_OFF
        let ptr = unsafe { read_u64(v, 56) };             // SUBVIEWS_PTR_OFF
        if ptr != 0 && count > 0 {
            for i in 0..count {
                let child = unsafe { *((ptr as usize + i * 8) as *const u64) };
                if child != 0 {
                    queue.push_back(child);
                }
            }
        }
    }
}

/// Map Apple's priority value (1..1000) to a cassowary strength.
fn priority_to_strength(priority: f64) -> f64 {
    if priority >= 1000.0 {
        REQUIRED
    } else if priority >= 750.0 {
        // Interpolate between STRONG and REQUIRED.
        STRONG + (priority - 750.0) / 250.0 * (REQUIRED - STRONG)
    } else if priority >= 500.0 {
        MEDIUM + (priority - 500.0) / 250.0 * (STRONG - MEDIUM)
    } else if priority >= 250.0 {
        WEAK + (priority - 250.0) / 250.0 * (MEDIUM - WEAK)
    } else {
        // Below 250 → scale within WEAK.
        (priority / 250.0) * WEAK
    }
}

/// Translate a single NSLayoutConstraint pointer into a `cassowary::Constraint`.
///
/// Returns `None` if the constraint refers to unknown attributes or views
/// that aren't in the variable map.
fn translate_constraint(
    c_ptr: u64,
    var_map: &mut HashMap<u64, ViewVars>,
) -> Option<cassowary::Constraint> {
    let first_item = unsafe { read_u64(c_ptr, C_FIRST_ITEM) };
    let first_attr = unsafe { read_u64(c_ptr, C_FIRST_ATTR) };
    let relation = unsafe { read_u64(c_ptr, C_RELATION) };
    let second_item = unsafe { read_u64(c_ptr, C_SECOND_ITEM) };
    let second_attr = unsafe { read_u64(c_ptr, C_SECOND_ATTR) };
    let multiplier = unsafe { read_f64(c_ptr, C_MULTIPLIER) };
    let constant = unsafe { read_f64(c_ptr, C_CONSTANT) };
    let priority = unsafe { read_f64(c_ptr, C_PRIORITY) };

    if first_item == 0 || first_attr == 0 {
        return None;
    }

    // Ensure view has variables.
    let first_vars = *var_map.entry(first_item).or_insert_with(ViewVars::new);

    let lhs = first_vars.expr_for_attr(first_attr)?;

    // Build RHS: multiplier * second.attr + constant, or just constant.
    let rhs = if second_item != 0 && second_attr != 0 {
        let second_vars = *var_map.entry(second_item).or_insert_with(ViewVars::new);
        let base = second_vars.expr_for_attr(second_attr)?;
        base * multiplier + constant
    } else {
        // No second item — constraint is of the form: attr == constant
        // (e.g. width = 100).
        Expression::from(constant)
    };

    let strength = priority_to_strength(priority);

    // Build the constraint based on relation:
    //   relation -1 = LE, 0 = EQ, 1 = GE
    let cass_constraint = match relation as i64 {
        -1 => lhs |LE(strength)| rhs,
         0 => lhs |EQ(strength)| rhs,
         1 => lhs |GE(strength)| rhs,
         _ => return None,
    };

    Some(cass_constraint)
}

/// Reset all layout state (called from `reset_runtime_state_for_reuse`).
pub(crate) fn reset_global_state() {
    let mut guard = LAYOUT_STATE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

/// Flush pending layouts for all views that have `needs_layout` set.
///
/// Called by the run loop before waiting (`kCFRunLoopBeforeWaiting`) to
/// coalesce layout passes, just as Apple's real implementation does.
pub(crate) fn flush_pending_layouts() {
    let views_needing_layout: Vec<u64> = with_state(|state| {
        state
            .view_constraints
            .keys()
            .copied()
            .filter(|&v| v != 0 && needs_layout(v))
            .collect()
    });
    for view in views_needing_layout {
        layout_if_needed(view);
    }
}

/// Alias for tests.
#[cfg(test)]
pub(crate) fn reset() {
    reset_global_state();
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;
    use crate::nslayoutconstraint::*;
    use crate::nsview::VIEW_SIZE;

    /// Serialise layout-solver tests — they share a process-global `LAYOUT_STATE`.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn make_view(x: f64, y: f64, w: f64, h: f64) -> u64 {
        let v = alloc_obj(VIEW_SIZE, crate::nsobj::nsview_class_addr());
        unsafe {
            write_f64(v, FRAME_X_OFF, x);
            write_f64(v, FRAME_Y_OFF, y);
            write_f64(v, FRAME_W_OFF, w);
            write_f64(v, FRAME_H_OFF, h);
        }
        v
    }

    /// Add child to parent's subview list (minimal — no window propagation).
    fn add_subview_raw(parent: u64, child: u64) {
        unsafe {
            // Set child's superview.
            *((child as usize + 8) as *mut u64) = parent; // SUPERVIEW_OFF

            // Append to parent's subview array.
            let count = read_u64(parent, 64) as usize; // SUBVIEW_COUNT_OFF
            let old_ptr = read_u64(parent, 56);         // SUBVIEWS_PTR_OFF
            let new_count = count + 1;
            let new_ptr = if old_ptr == 0 {
                crate::pal_alloc().calloc(new_count, 8) as u64
            } else {
                libc::realloc(old_ptr as *mut libc::c_void, new_count * 8) as u64
            };
            *((new_ptr as usize + count * 8) as *mut u64) = child;
            *((parent as usize + 56) as *mut u64) = new_ptr; // SUBVIEWS_PTR_OFF
            *((parent as usize + 64) as *mut u64) = new_count as u64; // SUBVIEW_COUNT_OFF
        }
    }

    fn read_frame(v: u64) -> (f64, f64, f64, f64) {
        unsafe {
            (
                read_f64(v, FRAME_X_OFF),
                read_f64(v, FRAME_Y_OFF),
                read_f64(v, FRAME_W_OFF),
                read_f64(v, FRAME_H_OFF),
            )
        }
    }

    fn make_width_constraint(view: u64, width: f64) -> u64 {
        // attr 7 = Width, relation 0 = EQ, no second item.
        // Mark active directly (not via setActive: which auto-installs).
        let c = shim_nslayoutconstraint_create(0, 0, view, 7, 0, 0, 0, 1.0, width);
        unsafe { *((c as usize + 72) as *mut u64) = 1; } // C_ACTIVE
        c
    }

    fn make_height_constraint(view: u64, height: f64) -> u64 {
        // attr 8 = Height, relation 0 = EQ, no second item.
        let c = shim_nslayoutconstraint_create(0, 0, view, 8, 0, 0, 0, 1.0, height);
        unsafe { *((c as usize + 72) as *mut u64) = 1; } // C_ACTIVE
        c
    }

    // attr: 1=Left, 2=Right, 3=Top, 4=Bottom
    fn make_eq_constraint(
        view_a: u64, attr_a: u64,
        view_b: u64, attr_b: u64,
        constant: f64,
    ) -> u64 {
        let c = shim_nslayoutconstraint_create(
            0, 0, view_a, attr_a, 0, view_b, attr_b, 1.0, constant,
        );
        unsafe { *((c as usize + 72) as *mut u64) = 1; } // C_ACTIVE
        c
    }

    #[test]
    fn solver_width_height_constraint() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        let cw = make_width_constraint(child, 100.0);
        let ch = make_height_constraint(child, 50.0);
        add_constraint(parent, cw);
        add_constraint(parent, ch);

        layout_if_needed(parent);

        let (_, _, w, h) = read_frame(child);
        assert!((w - 100.0).abs() < 1.0, "width should be ~100, got {w}");
        assert!((h - 50.0).abs() < 1.0, "height should be ~50, got {h}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solver_pin_to_parent_edges() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        // child.left == parent.left (attr 1)
        let c1 = make_eq_constraint(child, 1, parent, 1, 0.0);
        // child.right == parent.right (attr 2)
        let c2 = make_eq_constraint(child, 2, parent, 2, 0.0);
        // child.top == parent.top (attr 3)
        let c3 = make_eq_constraint(child, 3, parent, 3, 0.0);
        // child.bottom == parent.bottom (attr 4)
        let c4 = make_eq_constraint(child, 4, parent, 4, 0.0);

        add_constraint(parent, c1);
        add_constraint(parent, c2);
        add_constraint(parent, c3);
        add_constraint(parent, c4);

        layout_if_needed(parent);

        let (x, y, w, h) = read_frame(child);
        assert!((x - 0.0).abs() < 1.0, "x should be ~0, got {x}");
        assert!((y - 0.0).abs() < 1.0, "y should be ~0, got {y}");
        assert!((w - 400.0).abs() < 1.0, "width should be ~400, got {w}");
        assert!((h - 300.0).abs() < 1.0, "height should be ~300, got {h}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solver_center_in_parent() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        // child.width = 100
        let cw = make_width_constraint(child, 100.0);
        // child.height = 80
        let ch = make_height_constraint(child, 80.0);
        // child.centerX == parent.centerX (attr 9)
        let cx = make_eq_constraint(child, 9, parent, 9, 0.0);
        // child.centerY == parent.centerY (attr 10)
        let cy = make_eq_constraint(child, 10, parent, 10, 0.0);

        add_constraint(parent, cw);
        add_constraint(parent, ch);
        add_constraint(parent, cx);
        add_constraint(parent, cy);

        layout_if_needed(parent);

        let (x, y, w, h) = read_frame(child);
        assert!((w - 100.0).abs() < 1.0, "width should be ~100, got {w}");
        assert!((h - 80.0).abs() < 1.0, "height should be ~80, got {h}");
        // Centered: x = (400-100)/2 = 150, y = (300-80)/2 = 110
        assert!((x - 150.0).abs() < 1.0, "x should be ~150, got {x}");
        assert!((y - 110.0).abs() < 1.0, "y should be ~110, got {y}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solver_with_insets() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        // child.left == parent.left + 10
        let c1 = make_eq_constraint(child, 1, parent, 1, 10.0);
        // child.right == parent.right - 10  → child.right == parent.right + (-10)
        let c2 = make_eq_constraint(child, 2, parent, 2, -10.0);
        // child.top == parent.top + 20
        let c3 = make_eq_constraint(child, 3, parent, 3, 20.0);
        // child.bottom == parent.bottom - 20
        let c4 = make_eq_constraint(child, 4, parent, 4, -20.0);

        add_constraint(parent, c1);
        add_constraint(parent, c2);
        add_constraint(parent, c3);
        add_constraint(parent, c4);

        layout_if_needed(parent);

        let (x, y, w, h) = read_frame(child);
        assert!((x - 10.0).abs() < 1.0, "x should be ~10, got {x}");
        assert!((y - 20.0).abs() < 1.0, "y should be ~20, got {y}");
        assert!((w - 380.0).abs() < 1.0, "width should be ~380, got {w}");
        assert!((h - 260.0).abs() < 1.0, "height should be ~260, got {h}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solver_inactive_constraints_ignored() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(50.0, 50.0, 10.0, 10.0);
        add_subview_raw(parent, child);

        // Add width = 200 but DON'T activate it.
        let c = shim_nslayoutconstraint_create(0, 0, child, 7, 0, 0, 0, 1.0, 200.0);
        // Not activated — isActive remains 0.
        add_constraint(parent, c);

        layout_if_needed(parent);

        // Child frame should be unchanged (no active constraints solved).
        let (_, _, w, _) = read_frame(child);
        assert!((w - 10.0).abs() < 1.0, "width should be unchanged at ~10, got {w}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solver_remove_constraint() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        let cw = make_width_constraint(child, 100.0);
        add_constraint(parent, cw);
        layout_if_needed(parent);

        let (_, _, w, _) = read_frame(child);
        assert!((w - 100.0).abs() < 1.0, "width should be ~100");

        // Remove the constraint, reset child frame, re-layout.
        remove_constraint(parent, cw);
        unsafe { write_f64(child, FRAME_W_OFF, 0.0); }
        layout_if_needed(parent);

        // No constraints → frame should stay at 0.
        let (_, _, w2, _) = read_frame(child);
        assert!((w2 - 0.0).abs() < 1.0, "width should be ~0 after constraint removed, got {w2}");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn get_constraints_returns_added() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);

        let c1 = make_width_constraint(child, 100.0);
        let c2 = make_height_constraint(child, 50.0);
        add_constraint(parent, c1);
        add_constraint(parent, c2);

        let arr = get_constraints(parent);
        let count = crate::nsarray::shim_nsarray_count(arr, 0);
        assert_eq!(count, 2, "should have 2 constraints");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn needs_layout_flag() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let view = make_view(0.0, 0.0, 100.0, 100.0);

        assert!(!needs_layout(view), "should not need layout initially");
        set_needs_layout(view, true);
        assert!(needs_layout(view), "should need layout after set");
        set_needs_layout(view, false);
        assert!(!needs_layout(view), "should not need layout after clear");

        free_obj(view);
    }

    #[test]
    fn priority_mapping() {
        // REQUIRED
        assert_eq!(priority_to_strength(1000.0), REQUIRED);
        // STRONG range
        let s750 = priority_to_strength(750.0);
        assert!(s750 >= STRONG && s750 <= REQUIRED);
        // MEDIUM range
        let s500 = priority_to_strength(500.0);
        assert!(s500 >= MEDIUM && s500 <= STRONG);
        // WEAK range
        let s250 = priority_to_strength(250.0);
        assert!(s250 >= WEAK && s250 <= MEDIUM);
        // Low
        let s1 = priority_to_strength(1.0);
        assert!(s1 > 0.0 && s1 < WEAK);
    }

    #[test]
    fn solver_two_children_side_by_side() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let left = make_view(0.0, 0.0, 0.0, 0.0);
        let right = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, left);
        add_subview_raw(parent, right);

        // left.left == parent.left
        add_constraint(parent, make_eq_constraint(left, 1, parent, 1, 0.0));
        // left.width = 200
        add_constraint(parent, make_width_constraint(left, 200.0));
        // left.height = 300
        add_constraint(parent, make_height_constraint(left, 300.0));
        // left.top == parent.top
        add_constraint(parent, make_eq_constraint(left, 3, parent, 3, 0.0));

        // right.left == left.right
        add_constraint(parent, make_eq_constraint(right, 1, left, 2, 0.0));
        // right.right == parent.right
        add_constraint(parent, make_eq_constraint(right, 2, parent, 2, 0.0));
        // right.top == parent.top
        add_constraint(parent, make_eq_constraint(right, 3, parent, 3, 0.0));
        // right.height = 300
        add_constraint(parent, make_height_constraint(right, 300.0));

        layout_if_needed(parent);

        let (lx, ly, lw, lh) = read_frame(left);
        assert!((lx - 0.0).abs() < 1.0, "left.x = {lx}");
        assert!((ly - 0.0).abs() < 1.0, "left.y = {ly}");
        assert!((lw - 200.0).abs() < 1.0, "left.w = {lw}");
        assert!((lh - 300.0).abs() < 1.0, "left.h = {lh}");

        let (rx, ry, rw, rh) = read_frame(right);
        assert!((rx - 200.0).abs() < 1.0, "right.x = {rx}");
        assert!((ry - 0.0).abs() < 1.0, "right.y = {ry}");
        assert!((rw - 200.0).abs() < 1.0, "right.w = {rw}");
        assert!((rh - 300.0).abs() < 1.0, "right.h = {rh}");

        free_obj(right);
        free_obj(left);
        free_obj(parent);
    }

    #[test]
    fn flush_pending_layouts_clears_needs_layout() {
        let _lock = TEST_LOCK.lock();
        reset();

        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 100.0, 50.0);
        add_subview_raw(parent, child);

        // Add a width constraint and mark needs layout.
        let c = make_width_constraint(child, 200.0);
        add_constraint(parent, c);
        set_needs_layout(parent, true);
        assert!(needs_layout(parent));

        // Flush should clear the flag.
        flush_pending_layouts();
        assert!(!needs_layout(parent), "flush should clear needs_layout");

        free_obj(child);
        free_obj(parent);
    }

    #[test]
    fn solve_via_set_active_auto_install() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let parent = make_view(0.0, 0.0, 400.0, 300.0);
        let child = make_view(0.0, 0.0, 0.0, 0.0);
        add_subview_raw(parent, child);

        // Use setActive: (modern API) — auto-installs constraint onto the view.
        let cw = shim_nslayoutconstraint_create(0, 0, child, 7, 0, 0, 0, 1.0, 100.0);
        shim_nslayoutconstraint_set_active(cw, 0, 1);
        let ch = shim_nslayoutconstraint_create(0, 0, child, 8, 0, 0, 0, 1.0, 50.0);
        shim_nslayoutconstraint_set_active(ch, 0, 1);

        // Pin to parent edges via setActive:.
        let cl = shim_nslayoutconstraint_create(0, 0, child, 9, 0, parent, 9, 1.0, 0.0); // centerX
        shim_nslayoutconstraint_set_active(cl, 0, 1);
        let ct = shim_nslayoutconstraint_create(0, 0, child, 10, 0, parent, 10, 1.0, 0.0); // centerY
        shim_nslayoutconstraint_set_active(ct, 0, 1);

        // Layout should solve from parent down.
        layout_if_needed(parent);

        let (x, y, w, h) = read_frame(child);
        assert!((w - 100.0).abs() < 1.0, "width should be ~100, got {w}");
        assert!((h - 50.0).abs() < 1.0, "height should be ~50, got {h}");
        assert!((x - 150.0).abs() < 1.0, "x should be ~150 (centered), got {x}");
        assert!((y - 125.0).abs() < 1.0, "y should be ~125 (centered), got {y}");

        free_obj(child);
        free_obj(parent);
    }
}
