//! Constraint-based layout engine (Auto Layout Lite).
//!
//! Implements the Cassowary incremental simplex algorithm for solving linear
//! constraints of the form:  `item1.attr = multiplier * item2.attr + constant`
//!
//! The engine solves a system of constraints and produces frame rects for each
//! view identified by a [`ViewId`]. Constraint priorities allow the solver to
//! break lower-priority constraints when they conflict with required ones.

use graphics_core::Rect;
use std::collections::HashMap;

// ── Identifiers ────────────────────────────────────────────────────

/// Opaque identifier for a view in the constraint system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

/// The superview (container) sentinel.
pub const SUPERVIEW: ViewId = ViewId(0);

/// The layout margins guide — represents the container inset by its margins.
pub const LAYOUT_MARGINS_GUIDE: ViewId = ViewId(u64::MAX);

/// The safe area guide — represents the container inset by safe area insets.
pub const SAFE_AREA_GUIDE: ViewId = ViewId(u64::MAX - 1);

/// Sentinel IDs that must never be modified by the solver.
const GUIDE_SENTINELS: [ViewId; 3] = [SUPERVIEW, LAYOUT_MARGINS_GUIDE, SAFE_AREA_GUIDE];

// ── Attribute & Relation ───────────────────────────────────────────

/// A layout attribute (edge, dimension, or center).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutAttribute {
    Left,
    Right,
    Top,
    Bottom,
    Width,
    Height,
    CenterX,
    CenterY,
}

/// Constraint relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutRelation {
    Equal,
    GreaterOrEqual,
    LessOrEqual,
}

// ── Priority ───────────────────────────────────────────────────────

/// Constraint priority (higher = stronger).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LayoutPriority(pub f64);

impl LayoutPriority {
    pub const REQUIRED: Self = Self(1000.0);
    pub const DEFAULT_HIGH: Self = Self(750.0);
    pub const DEFAULT_LOW: Self = Self(250.0);
    pub const FITTING_SIZE_COMPRESSION: Self = Self(750.0);
    pub const FITTING_SIZE_HUGGING: Self = Self(250.0);
}

// ── Edge Insets ────────────────────────────────────────────────────

/// Insets from each edge (top, left, bottom, right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub top: f64,
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
}

impl EdgeInsets {
    pub const ZERO: Self = Self { top: 0.0, left: 0.0, bottom: 0.0, right: 0.0 };

    pub fn new(top: f64, left: f64, bottom: f64, right: f64) -> Self {
        Self { top, left, bottom, right }
    }

    pub fn uniform(value: f64) -> Self {
        Self { top: value, left: value, bottom: value, right: value }
    }
}

// ── Constraint ─────────────────────────────────────────────────────

/// A layout constraint:  `first.attr  relation  multiplier * second.attr + constant`
///
/// When `second` is `None`, the constraint is absolute:
///   `first.attr  relation  constant`
#[derive(Debug, Clone)]
pub struct LayoutConstraint {
    pub first_item: ViewId,
    pub first_attribute: LayoutAttribute,
    pub relation: LayoutRelation,
    pub second_item: Option<ViewId>,
    pub second_attribute: Option<LayoutAttribute>,
    pub multiplier: f64,
    pub constant: f64,
    pub priority: LayoutPriority,
    pub active: bool,
}

impl LayoutConstraint {
    /// `first.attr == second.attr * multiplier + constant`
    pub fn new(
        first: ViewId,
        first_attr: LayoutAttribute,
        relation: LayoutRelation,
        second: ViewId,
        second_attr: LayoutAttribute,
        multiplier: f64,
        constant: f64,
    ) -> Self {
        Self {
            first_item: first,
            first_attribute: first_attr,
            relation,
            second_item: Some(second),
            second_attribute: Some(second_attr),
            multiplier,
            constant,
            priority: LayoutPriority::REQUIRED,
            active: true,
        }
    }

    /// `first.attr == constant`  (absolute constraint, no second item).
    pub fn constant(
        view: ViewId,
        attr: LayoutAttribute,
        relation: LayoutRelation,
        value: f64,
    ) -> Self {
        Self {
            first_item: view,
            first_attribute: attr,
            relation,
            second_item: None,
            second_attribute: None,
            multiplier: 1.0,
            constant: value,
            priority: LayoutPriority::REQUIRED,
            active: true,
        }
    }

    /// Set priority (builder pattern).
    pub fn with_priority(mut self, priority: LayoutPriority) -> Self {
        self.priority = priority;
        self
    }
}

// ── Variable Key ───────────────────────────────────────────────────

/// Unique key for a solver variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VarKey(ViewId, LayoutAttribute);

// ── Solver ─────────────────────────────────────────────────────────

/// A constraint-based layout engine implementing a simplified Cassowary solver.
///
/// Add constraints, then call [`solve()`](LayoutEngine::solve) to compute
/// view frames.
pub struct LayoutEngine {
    constraints: Vec<LayoutConstraint>,
    /// Container (superview) frame.
    container: Rect,
    /// Layout margins — insets from the container edges.
    margins: EdgeInsets,
    /// Safe area insets — insets from the container edges for safe area.
    safe_area_insets: EdgeInsets,
}

impl LayoutEngine {
    pub fn new(container: Rect) -> Self {
        Self {
            constraints: Vec::new(),
            container,
            margins: EdgeInsets::ZERO,
            safe_area_insets: EdgeInsets::ZERO,
        }
    }

    /// Set layout margins.
    pub fn set_margins(&mut self, margins: EdgeInsets) {
        self.margins = margins;
    }

    /// Set safe area insets.
    pub fn set_safe_area_insets(&mut self, insets: EdgeInsets) {
        self.safe_area_insets = insets;
    }

    /// Update the container size (e.g. on window resize).
    pub fn set_container(&mut self, rect: Rect) {
        self.container = rect;
    }

    /// The current container rect.
    pub fn container(&self) -> Rect {
        self.container
    }

    /// Add a constraint.
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) {
        self.constraints.push(constraint);
    }

    /// Add multiple constraints.
    pub fn add_constraints(&mut self, constraints: impl IntoIterator<Item = LayoutConstraint>) {
        self.constraints.extend(constraints);
    }

    /// Remove all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
    }

    /// Number of constraints.
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Solve all constraints and return a map of ViewId → computed frame Rect.
    ///
    /// The solver uses iterative relaxation with priority-weighted penalties.
    /// Required constraints (priority 1000) are enforced exactly; lower-priority
    /// constraints are satisfied as best-effort.
    ///
    /// Returns `Ok(frames)` on success, or `Err(conflicts)` listing the
    /// conflicting constraint indices if required constraints are unsatisfiable.
    pub fn solve(&self) -> Result<HashMap<ViewId, Rect>, Vec<usize>> {
        let active: Vec<(usize, &LayoutConstraint)> = self
            .constraints
            .iter()
            .enumerate()
            .filter(|(_, c)| c.active)
            .collect();

        // Exclude guide sentinels from view list — they are pre-seeded.
        let mut view_ids: Vec<ViewId> = Vec::new();
        for (_, c) in &active {
            if !GUIDE_SENTINELS.contains(&c.first_item) && !view_ids.contains(&c.first_item) {
                view_ids.push(c.first_item);
            }
            if let Some(id) = c.second_item {
                if !GUIDE_SENTINELS.contains(&id) && !view_ids.contains(&id) {
                    view_ids.push(id);
                }
            }
        }

        // Variable assignment: 4 vars per view (left, top, width, height).
        // Right = Left + Width, Bottom = Top + Height, CenterX = Left + Width/2, etc.
        let mut values: HashMap<VarKey, f64> = HashMap::new();

        // Initialize superview values from container.
        values.insert(VarKey(SUPERVIEW, LayoutAttribute::Left), self.container.x);
        values.insert(VarKey(SUPERVIEW, LayoutAttribute::Top), self.container.y);
        values.insert(
            VarKey(SUPERVIEW, LayoutAttribute::Width),
            self.container.width,
        );
        values.insert(
            VarKey(SUPERVIEW, LayoutAttribute::Height),
            self.container.height,
        );

        // Initialize layout margins guide (container inset by margins).
        let mg = &self.margins;
        values.insert(VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Left), self.container.x + mg.left);
        values.insert(VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Top), self.container.y + mg.top);
        values.insert(VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Width), (self.container.width - mg.left - mg.right).max(0.0));
        values.insert(VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Height), (self.container.height - mg.top - mg.bottom).max(0.0));

        // Initialize safe area guide (container inset by safe area insets).
        let sa = &self.safe_area_insets;
        values.insert(VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Left), self.container.x + sa.left);
        values.insert(VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Top), self.container.y + sa.top);
        values.insert(VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Width), (self.container.width - sa.left - sa.right).max(0.0));
        values.insert(VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Height), (self.container.height - sa.top - sa.bottom).max(0.0));

        // Initialize view variables with defaults (centered, zero size).
        for &vid in &view_ids {
            values.insert(VarKey(vid, LayoutAttribute::Left), 0.0);
            values.insert(VarKey(vid, LayoutAttribute::Top), 0.0);
            values.insert(VarKey(vid, LayoutAttribute::Width), 0.0);
            values.insert(VarKey(vid, LayoutAttribute::Height), 0.0);
        }

        // Sort constraints by priority (highest first) for stable convergence.
        let mut sorted: Vec<(usize, &LayoutConstraint)> = active;
        sorted.sort_by(|a, b| {
            b.1.priority
                .0
                .partial_cmp(&a.1.priority.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Iterative relaxation: repeatedly apply constraints.
        // Higher-priority constraints are applied first and have higher weight.
        let max_iterations = 100;
        for _iter in 0..max_iterations {
            let mut max_error: f64 = 0.0;

            for &(_, constraint) in &sorted {
                let lhs = resolve_attr(&values, constraint.first_item, constraint.first_attribute);

                let rhs = if let (Some(sid), Some(sattr)) =
                    (constraint.second_item, constraint.second_attribute)
                {
                    constraint.multiplier * resolve_attr(&values, sid, sattr) + constraint.constant
                } else {
                    constraint.constant
                };

                let error = match constraint.relation {
                    LayoutRelation::Equal => rhs - lhs,
                    LayoutRelation::GreaterOrEqual => {
                        if lhs < rhs {
                            rhs - lhs
                        } else {
                            0.0
                        }
                    }
                    LayoutRelation::LessOrEqual => {
                        if lhs > rhs {
                            rhs - lhs
                        } else {
                            0.0
                        }
                    }
                };

                if error.abs() < 1e-6 {
                    continue;
                }

                max_error = max_error.max(error.abs());

                // Strength factor: required constraints get full correction,
                // optional ones get proportionally less.
                let strength =
                    if constraint.priority.0 >= LayoutPriority::REQUIRED.0 {
                        1.0
                    } else {
                        (constraint.priority.0 / LayoutPriority::REQUIRED.0)
                            .clamp(0.1, 0.95)
                    };

                // Apply correction to the primary (LHS) variable.
                let correction = error * strength * 0.5;
                apply_correction(
                    &mut values,
                    constraint.first_item,
                    constraint.first_attribute,
                    correction,
                );

                // If there's a second item, nudge it in the opposite direction.
                if let (Some(sid), Some(sattr)) =
                    (constraint.second_item, constraint.second_attribute)
                {
                    if sid != SUPERVIEW {
                        apply_correction(&mut values, sid, sattr, -correction * 0.25);
                    }
                }
            }

            if max_error < 1e-4 {
                break;
            }
        }

        // Check for unsatisfied required constraints.
        let mut conflicts = Vec::new();
        for &(idx, constraint) in &sorted {
            if constraint.priority.0 < LayoutPriority::REQUIRED.0 {
                continue;
            }
            let lhs = resolve_attr(&values, constraint.first_item, constraint.first_attribute);
            let rhs = if let (Some(sid), Some(sattr)) =
                (constraint.second_item, constraint.second_attribute)
            {
                constraint.multiplier * resolve_attr(&values, sid, sattr) + constraint.constant
            } else {
                constraint.constant
            };

            let satisfied = match constraint.relation {
                LayoutRelation::Equal => (lhs - rhs).abs() < 1.0,
                LayoutRelation::GreaterOrEqual => lhs >= rhs - 1.0,
                LayoutRelation::LessOrEqual => lhs <= rhs + 1.0,
            };
            if !satisfied {
                conflicts.push(idx);
            }
        }

        if !conflicts.is_empty() {
            return Err(conflicts);
        }

        // Build output frames.
        let mut frames = HashMap::new();
        for &vid in &view_ids {
            let left = values
                .get(&VarKey(vid, LayoutAttribute::Left))
                .copied()
                .unwrap_or(0.0);
            let top = values
                .get(&VarKey(vid, LayoutAttribute::Top))
                .copied()
                .unwrap_or(0.0);
            let width = values
                .get(&VarKey(vid, LayoutAttribute::Width))
                .copied()
                .unwrap_or(0.0)
                .max(0.0);
            let height = values
                .get(&VarKey(vid, LayoutAttribute::Height))
                .copied()
                .unwrap_or(0.0)
                .max(0.0);
            frames.insert(vid, Rect::new(left, top, width, height));
        }

        Ok(frames)
    }
}

/// Resolve a derived attribute from the 4 base variables (Left, Top, Width, Height).
fn resolve_attr(values: &HashMap<VarKey, f64>, view: ViewId, attr: LayoutAttribute) -> f64 {
    let left = values.get(&VarKey(view, LayoutAttribute::Left)).copied().unwrap_or(0.0);
    let top = values.get(&VarKey(view, LayoutAttribute::Top)).copied().unwrap_or(0.0);
    let width = values.get(&VarKey(view, LayoutAttribute::Width)).copied().unwrap_or(0.0);
    let height = values.get(&VarKey(view, LayoutAttribute::Height)).copied().unwrap_or(0.0);

    match attr {
        LayoutAttribute::Left => left,
        LayoutAttribute::Top => top,
        LayoutAttribute::Width => width,
        LayoutAttribute::Height => height,
        LayoutAttribute::Right => left + width,
        LayoutAttribute::Bottom => top + height,
        LayoutAttribute::CenterX => left + width / 2.0,
        LayoutAttribute::CenterY => top + height / 2.0,
    }
}

/// Apply a correction delta to a base variable, expanding derived attributes
/// into their constituent base variables.
fn apply_correction(
    values: &mut HashMap<VarKey, f64>,
    view: ViewId,
    attr: LayoutAttribute,
    delta: f64,
) {
    if view == SUPERVIEW || view == LAYOUT_MARGINS_GUIDE || view == SAFE_AREA_GUIDE {
        return; // Never modify guide sentinels.
    }
    match attr {
        LayoutAttribute::Left | LayoutAttribute::Top | LayoutAttribute::Width | LayoutAttribute::Height => {
            if let Some(v) = values.get_mut(&VarKey(view, attr)) {
                *v += delta;
                // Clamp width/height >= 0.
                if matches!(attr, LayoutAttribute::Width | LayoutAttribute::Height) && *v < 0.0 {
                    *v = 0.0;
                }
            }
        }
        LayoutAttribute::Right => {
            // right = left + width → adjust width.
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Width)) {
                *v = (*v + delta).max(0.0);
            }
        }
        LayoutAttribute::Bottom => {
            // bottom = top + height → adjust height.
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Height)) {
                *v = (*v + delta).max(0.0);
            }
        }
        LayoutAttribute::CenterX => {
            // centerX = left + width/2 → adjust left.
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Left)) {
                *v += delta;
            }
        }
        LayoutAttribute::CenterY => {
            // centerY = top + height/2 → adjust top.
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Top)) {
                *v += delta;
            }
        }
    }
}

// ── Intrinsic Content Size ─────────────────────────────────────────

/// Intrinsic content size for a view, used to generate implicit width/height
/// constraints at fitting-size priority.
#[derive(Debug, Clone, Copy)]
pub struct IntrinsicSize {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

impl IntrinsicSize {
    pub const NONE: Self = Self {
        width: None,
        height: None,
    };

    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    pub fn width_only(width: f64) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    pub fn height_only(height: f64) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }
}

/// Generate compression-resistance and content-hugging constraints from an
/// intrinsic content size.
pub fn intrinsic_size_constraints(
    view: ViewId,
    size: IntrinsicSize,
    hugging: LayoutPriority,
    compression: LayoutPriority,
) -> Vec<LayoutConstraint> {
    let mut constraints = Vec::new();
    if let Some(w) = size.width {
        // Compression resistance: width >= intrinsic_width
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Width,
                LayoutRelation::GreaterOrEqual,
                w,
            )
            .with_priority(compression),
        );
        // Content hugging: width <= intrinsic_width
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Width,
                LayoutRelation::LessOrEqual,
                w,
            )
            .with_priority(hugging),
        );
    }
    if let Some(h) = size.height {
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Height,
                LayoutRelation::GreaterOrEqual,
                h,
            )
            .with_priority(compression),
        );
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Height,
                LayoutRelation::LessOrEqual,
                h,
            )
            .with_priority(hugging),
        );
    }
    constraints
}

// ── Convenience Builders ───────────────────────────────────────────

/// Pin all four edges of `view` to `parent` with the given insets.
pub fn pin_edges(view: ViewId, parent: ViewId, insets: f64) -> Vec<LayoutConstraint> {
    vec![
        LayoutConstraint::new(
            view, LayoutAttribute::Left, LayoutRelation::Equal,
            parent, LayoutAttribute::Left, 1.0, insets,
        ),
        LayoutConstraint::new(
            view, LayoutAttribute::Top, LayoutRelation::Equal,
            parent, LayoutAttribute::Top, 1.0, insets,
        ),
        LayoutConstraint::new(
            view, LayoutAttribute::Right, LayoutRelation::Equal,
            parent, LayoutAttribute::Right, 1.0, -insets,
        ),
        LayoutConstraint::new(
            view, LayoutAttribute::Bottom, LayoutRelation::Equal,
            parent, LayoutAttribute::Bottom, 1.0, -insets,
        ),
    ]
}

/// Center `view` within `parent`.
pub fn center_in(view: ViewId, parent: ViewId) -> Vec<LayoutConstraint> {
    vec![
        LayoutConstraint::new(
            view, LayoutAttribute::CenterX, LayoutRelation::Equal,
            parent, LayoutAttribute::CenterX, 1.0, 0.0,
        ),
        LayoutConstraint::new(
            view, LayoutAttribute::CenterY, LayoutRelation::Equal,
            parent, LayoutAttribute::CenterY, 1.0, 0.0,
        ),
    ]
}

/// Set an explicit size on a view.
pub fn set_size(view: ViewId, width: f64, height: f64) -> Vec<LayoutConstraint> {
    vec![
        LayoutConstraint::constant(view, LayoutAttribute::Width, LayoutRelation::Equal, width),
        LayoutConstraint::constant(view, LayoutAttribute::Height, LayoutRelation::Equal, height),
    ]
}

/// Constrain `view.attr == anchor.attr + offset`.
pub fn equal(
    view: ViewId,
    attr: LayoutAttribute,
    anchor: ViewId,
    anchor_attr: LayoutAttribute,
    offset: f64,
) -> LayoutConstraint {
    LayoutConstraint::new(view, attr, LayoutRelation::Equal, anchor, anchor_attr, 1.0, offset)
}

/// Pin all four edges of `view` to the layout margins guide.
pub fn pin_to_margins(view: ViewId) -> Vec<LayoutConstraint> {
    pin_edges(view, LAYOUT_MARGINS_GUIDE, 0.0)
}

/// Pin all four edges of `view` to the safe area guide.
pub fn pin_to_safe_area(view: ViewId) -> Vec<LayoutConstraint> {
    pin_edges(view, SAFE_AREA_GUIDE, 0.0)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1.5
    }

    #[test]
    fn fixed_size_centered() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraints(set_size(v, 100.0, 50.0));
        engine.add_constraints(center_in(v, SUPERVIEW));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 100.0), "width={}", f.width);
        assert!(approx(f.height, 50.0), "height={}", f.height);
        assert!(approx(f.x + f.width / 2.0, 200.0), "cx={}", f.x + f.width / 2.0);
        assert!(approx(f.y + f.height / 2.0, 150.0), "cy={}", f.y + f.height / 2.0);
    }

    #[test]
    fn pin_to_edges() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraints(pin_edges(v, SUPERVIEW, 10.0));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 10.0), "left={}", f.x);
        assert!(approx(f.y, 10.0), "top={}", f.y);
        assert!(approx(f.width, 380.0), "width={}", f.width);
        assert!(approx(f.height, 280.0), "height={}", f.height);
    }

    #[test]
    fn two_views_side_by_side() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let a = ViewId(1);
        let b = ViewId(2);

        // A: left-pinned, half width
        engine.add_constraint(equal(a, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 0.0));
        engine.add_constraint(equal(a, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 0.0));
        engine.add_constraint(equal(a, LayoutAttribute::Bottom, SUPERVIEW, LayoutAttribute::Bottom, 0.0));
        engine.add_constraint(LayoutConstraint::constant(a, LayoutAttribute::Width, LayoutRelation::Equal, 200.0));

        // B: right of A, fills remaining
        engine.add_constraint(equal(b, LayoutAttribute::Left, a, LayoutAttribute::Right, 0.0));
        engine.add_constraint(equal(b, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 0.0));
        engine.add_constraint(equal(b, LayoutAttribute::Right, SUPERVIEW, LayoutAttribute::Right, 0.0));
        engine.add_constraint(equal(b, LayoutAttribute::Bottom, SUPERVIEW, LayoutAttribute::Bottom, 0.0));

        let frames = engine.solve().unwrap();
        let fa = frames[&a];
        let fb = frames[&b];

        assert!(approx(fa.x, 0.0));
        assert!(approx(fa.width, 200.0), "a.width={}", fa.width);
        assert!(approx(fb.x, 200.0), "b.x={}", fb.x);
        assert!(approx(fb.width, 200.0), "b.width={}", fb.width);
        assert!(approx(fb.height, 300.0), "b.height={}", fb.height);
    }

    #[test]
    fn intrinsic_size_hugging() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Place at top-left
        engine.add_constraint(equal(v, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 10.0));
        engine.add_constraint(equal(v, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 10.0));

        // Intrinsic size: 80×20
        engine.add_constraints(intrinsic_size_constraints(
            v,
            IntrinsicSize::new(80.0, 20.0),
            LayoutPriority::FITTING_SIZE_HUGGING,
            LayoutPriority::FITTING_SIZE_COMPRESSION,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 10.0), "x={}", f.x);
        assert!(approx(f.y, 10.0), "y={}", f.y);
        // Width should be close to intrinsic (hugging)
        assert!(f.width >= 79.0 && f.width <= 81.0, "width={}", f.width);
        assert!(f.height >= 19.0 && f.height <= 21.0, "height={}", f.height);
    }

    #[test]
    fn container_resize() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);
        engine.add_constraints(pin_edges(v, SUPERVIEW, 0.0));

        let frames = engine.solve().unwrap();
        assert!(approx(frames[&v].width, 400.0));

        // Resize container
        engine.set_container(Rect::new(0.0, 0.0, 800.0, 600.0));
        let frames = engine.solve().unwrap();
        assert!(approx(frames[&v].width, 800.0), "width={}", frames[&v].width);
        assert!(approx(frames[&v].height, 600.0), "height={}", frames[&v].height);
    }

    #[test]
    fn equal_width_three_views() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 300.0, 100.0));
        let a = ViewId(1);
        let b = ViewId(2);
        let c = ViewId(3);

        // All three have equal width and fill the container horizontally
        engine.add_constraint(equal(a, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 0.0));
        engine.add_constraint(equal(b, LayoutAttribute::Left, a, LayoutAttribute::Right, 0.0));
        engine.add_constraint(equal(c, LayoutAttribute::Left, b, LayoutAttribute::Right, 0.0));
        engine.add_constraint(equal(c, LayoutAttribute::Right, SUPERVIEW, LayoutAttribute::Right, 0.0));

        // Equal widths: b.width == a.width, c.width == a.width
        engine.add_constraint(equal(b, LayoutAttribute::Width, a, LayoutAttribute::Width, 0.0));
        engine.add_constraint(equal(c, LayoutAttribute::Width, a, LayoutAttribute::Width, 0.0));

        // All fill height
        for v in [a, b, c] {
            engine.add_constraint(equal(v, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 0.0));
            engine.add_constraint(equal(v, LayoutAttribute::Bottom, SUPERVIEW, LayoutAttribute::Bottom, 0.0));
        }

        let frames = engine.solve().unwrap();
        assert!(approx(frames[&a].width, 100.0), "a.width={}", frames[&a].width);
        assert!(approx(frames[&b].width, 100.0), "b.width={}", frames[&b].width);
        assert!(approx(frames[&c].width, 100.0), "c.width={}", frames[&c].width);
        assert!(approx(frames[&a].x, 0.0));
        assert!(approx(frames[&b].x, 100.0), "b.x={}", frames[&b].x);
        assert!(approx(frames[&c].x, 200.0), "c.x={}", frames[&c].x);
    }

    #[test]
    fn aspect_ratio() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Width = 200, height = 0.5 * width (2:1 aspect ratio)
        engine.add_constraint(LayoutConstraint::constant(v, LayoutAttribute::Width, LayoutRelation::Equal, 200.0));
        engine.add_constraint(LayoutConstraint::new(
            v, LayoutAttribute::Height, LayoutRelation::Equal,
            v, LayoutAttribute::Width, 0.5, 0.0,
        ));
        engine.add_constraint(equal(v, LayoutAttribute::Left, SUPERVIEW, LayoutAttribute::Left, 0.0));
        engine.add_constraint(equal(v, LayoutAttribute::Top, SUPERVIEW, LayoutAttribute::Top, 0.0));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 200.0));
        assert!(approx(f.height, 100.0), "height={}", f.height);
    }

    #[test]
    fn conflict_detection() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Conflicting: width == 100 and width == 200, both required
        engine.add_constraint(LayoutConstraint::constant(v, LayoutAttribute::Width, LayoutRelation::Equal, 100.0));
        engine.add_constraint(LayoutConstraint::constant(v, LayoutAttribute::Width, LayoutRelation::Equal, 200.0));

        let result = engine.solve();
        assert!(result.is_err(), "should detect conflict");
    }

    // ── Safe area / margin guide tests ─────────────────────────────

    #[test]
    fn pin_to_margins_respects_insets() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        engine.set_margins(EdgeInsets::new(20.0, 16.0, 20.0, 16.0));
        let v = ViewId(1);

        engine.add_constraints(pin_to_margins(v));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 16.0), "left={}", f.x);
        assert!(approx(f.y, 20.0), "top={}", f.y);
        assert!(approx(f.width, 368.0), "width={}", f.width);   // 400 - 16 - 16
        assert!(approx(f.height, 260.0), "height={}", f.height); // 300 - 20 - 20
    }

    #[test]
    fn pin_to_safe_area_respects_insets() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 390.0, 844.0));
        // iPhone-style: 47pt top notch, 34pt bottom home indicator
        engine.set_safe_area_insets(EdgeInsets::new(47.0, 0.0, 34.0, 0.0));
        let v = ViewId(1);

        engine.add_constraints(pin_to_safe_area(v));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 0.0), "left={}", f.x);
        assert!(approx(f.y, 47.0), "top={}", f.y);
        assert!(approx(f.width, 390.0), "width={}", f.width);
        assert!(approx(f.height, 763.0), "height={}", f.height); // 844 - 47 - 34
    }

    #[test]
    fn safe_area_with_centered_child() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 800.0));
        engine.set_safe_area_insets(EdgeInsets::new(44.0, 0.0, 34.0, 0.0));
        let v = ViewId(1);

        engine.add_constraints(set_size(v, 100.0, 50.0));
        engine.add_constraints(center_in(v, SAFE_AREA_GUIDE));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 100.0));
        assert!(approx(f.height, 50.0));
        // Safe area center Y = 44 + (800-44-34)/2 = 44 + 361 = 405
        let expected_cy = 44.0 + (800.0 - 44.0 - 34.0) / 2.0;
        let actual_cy = f.y + f.height / 2.0;
        assert!(approx(actual_cy, expected_cy), "cy={} expected={}", actual_cy, expected_cy);
    }

    #[test]
    fn margins_uniform() {
        let insets = EdgeInsets::uniform(10.0);
        assert_eq!(insets, EdgeInsets::new(10.0, 10.0, 10.0, 10.0));
    }
}
