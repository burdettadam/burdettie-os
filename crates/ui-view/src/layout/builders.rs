//! Intrinsic size and convenience layout builder functions.

use super::types::{
    LayoutAttribute, LayoutConstraint, LayoutPriority, LayoutRelation, LAYOUT_MARGINS_GUIDE,
    SAFE_AREA_GUIDE, ViewId,
};

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

/// Generate an aspect ratio constraint: `width = ratio * height`.
///
/// The `ratio` is width / height, e.g. 2.0 means width is twice the height.
pub fn aspect_ratio(view: ViewId, ratio: f64) -> LayoutConstraint {
    LayoutConstraint::new(
        view,
        LayoutAttribute::Width,
        LayoutRelation::Equal,
        view,
        LayoutAttribute::Height,
        ratio,
        0.0,
    )
}

/// Generate compression-resistance constraints from an intrinsic size
/// using per-axis priorities from a [`LayoutPriorities`](super::LayoutPriorities) struct.
pub fn intrinsic_size_with_priorities(
    view: ViewId,
    size: IntrinsicSize,
    priorities: &super::types::LayoutPriorities,
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
            .with_priority(priorities.compression_resistance_horizontal),
        );
        // Content hugging: width <= intrinsic_width
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Width,
                LayoutRelation::LessOrEqual,
                w,
            )
            .with_priority(priorities.hugging_horizontal),
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
            .with_priority(priorities.compression_resistance_vertical),
        );
        constraints.push(
            LayoutConstraint::constant(
                view,
                LayoutAttribute::Height,
                LayoutRelation::LessOrEqual,
                h,
            )
            .with_priority(priorities.hugging_vertical),
        );
    }
    constraints
}
