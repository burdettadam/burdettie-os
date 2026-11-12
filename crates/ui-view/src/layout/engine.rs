//! Constraint solver: LayoutEngine, VarKey, helper functions.

use std::collections::HashMap;

use graphics_core::Rect;

use super::types::{
    EdgeInsets, LayoutAttribute, LayoutConstraint, LayoutPriority, LayoutRelation, ViewId,
    GUIDE_SENTINELS, LAYOUT_MARGINS_GUIDE, SAFE_AREA_GUIDE, SUPERVIEW,
};

/// Unique key for a solver variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VarKey(ViewId, LayoutAttribute);

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
        values.insert(
            VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Left),
            self.container.x + mg.left,
        );
        values.insert(
            VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Top),
            self.container.y + mg.top,
        );
        values.insert(
            VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Width),
            (self.container.width - mg.left - mg.right).max(0.0),
        );
        values.insert(
            VarKey(LAYOUT_MARGINS_GUIDE, LayoutAttribute::Height),
            (self.container.height - mg.top - mg.bottom).max(0.0),
        );

        // Initialize safe area guide (container inset by safe area insets).
        let sa = &self.safe_area_insets;
        values.insert(
            VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Left),
            self.container.x + sa.left,
        );
        values.insert(
            VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Top),
            self.container.y + sa.top,
        );
        values.insert(
            VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Width),
            (self.container.width - sa.left - sa.right).max(0.0),
        );
        values.insert(
            VarKey(SAFE_AREA_GUIDE, LayoutAttribute::Height),
            (self.container.height - sa.top - sa.bottom).max(0.0),
        );

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

        // Iterative relaxation.
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

                let strength = if constraint.priority.0 >= LayoutPriority::REQUIRED.0 {
                    1.0
                } else {
                    (constraint.priority.0 / LayoutPriority::REQUIRED.0).clamp(0.1, 0.95)
                };

                let correction = error * strength * 0.5;
                apply_correction(
                    &mut values,
                    constraint.first_item,
                    constraint.first_attribute,
                    correction,
                );

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
    let left = values
        .get(&VarKey(view, LayoutAttribute::Left))
        .copied()
        .unwrap_or(0.0);
    let top = values
        .get(&VarKey(view, LayoutAttribute::Top))
        .copied()
        .unwrap_or(0.0);
    let width = values
        .get(&VarKey(view, LayoutAttribute::Width))
        .copied()
        .unwrap_or(0.0);
    let height = values
        .get(&VarKey(view, LayoutAttribute::Height))
        .copied()
        .unwrap_or(0.0);

    match attr {
        LayoutAttribute::Left => left,
        LayoutAttribute::Top => top,
        LayoutAttribute::Width => width,
        LayoutAttribute::Height => height,
        LayoutAttribute::Right => left + width,
        LayoutAttribute::Bottom => top + height,
        LayoutAttribute::CenterX => left + width / 2.0,
        LayoutAttribute::CenterY => top + height / 2.0,
        LayoutAttribute::AspectRatio => {
            if height.abs() > 1e-9 {
                width / height
            } else {
                0.0
            }
        }
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
        LayoutAttribute::Left
        | LayoutAttribute::Top
        | LayoutAttribute::Width
        | LayoutAttribute::Height => {
            if let Some(v) = values.get_mut(&VarKey(view, attr)) {
                *v += delta;
                if matches!(attr, LayoutAttribute::Width | LayoutAttribute::Height) && *v < 0.0 {
                    *v = 0.0;
                }
            }
        }
        LayoutAttribute::Right => {
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Width)) {
                *v = (*v + delta).max(0.0);
            }
        }
        LayoutAttribute::Bottom => {
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Height)) {
                *v = (*v + delta).max(0.0);
            }
        }
        LayoutAttribute::CenterX => {
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Left)) {
                *v += delta;
            }
        }
        LayoutAttribute::CenterY => {
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Top)) {
                *v += delta;
            }
        }
        LayoutAttribute::AspectRatio => {
            // Adjust width to satisfy aspect ratio.
            if let Some(v) = values.get_mut(&VarKey(view, LayoutAttribute::Width)) {
                *v = (*v + delta).max(0.0);
            }
        }
    }
}
