//! Constraint-based layout types: identifiers, attributes, constraints.

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
pub(super) const GUIDE_SENTINELS: [ViewId; 3] = [SUPERVIEW, LAYOUT_MARGINS_GUIDE, SAFE_AREA_GUIDE];

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
    /// Aspect ratio (width / height). Used with a multiplier constraint.
    AspectRatio,
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
    pub const HIGH: Self = Self(750.0);
    pub const LOW: Self = Self(250.0);
    pub const FITTING_SIZE_COMPRESSION: Self = Self(750.0);
    pub const FITTING_SIZE_HUGGING: Self = Self(250.0);
}

/// Per-view layout priorities for content hugging and compression resistance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutPriorities {
    pub hugging_horizontal: LayoutPriority,
    pub hugging_vertical: LayoutPriority,
    pub compression_resistance_horizontal: LayoutPriority,
    pub compression_resistance_vertical: LayoutPriority,
}

impl Default for LayoutPriorities {
    fn default() -> Self {
        Self {
            hugging_horizontal: LayoutPriority::LOW,
            hugging_vertical: LayoutPriority::LOW,
            compression_resistance_horizontal: LayoutPriority::HIGH,
            compression_resistance_vertical: LayoutPriority::HIGH,
        }
    }
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
