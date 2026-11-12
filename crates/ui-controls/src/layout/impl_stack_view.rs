use graphics_core::{Rect, Renderer};
use ui_view::layout::{self as constraint, LayoutAttribute, LayoutEngine, ViewId, SUPERVIEW};
use ui_view::View;

use super::{Axis, Distribution, StackView};

impl StackView {
    /// Create a stack view with direction and frame.
    pub fn new(frame: Rect, axis: Axis) -> Self {
        Self {
            view: View::new(frame),
            axis,
            spacing: 8.0,
            distribution: Distribution::Natural,
            padding: 0.0,
            child_frames: Vec::new(),
        }
    }

    /// The stacking axis.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Set the axis.
    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
        self.layout();
    }

    /// Spacing between children.
    pub fn spacing(&self) -> f64 {
        self.spacing
    }

    /// Set spacing between children.
    pub fn set_spacing(&mut self, spacing: f64) {
        self.spacing = spacing;
        self.layout();
    }

    /// Padding around the edges.
    pub fn padding(&self) -> f64 {
        self.padding
    }

    /// Set padding.
    pub fn set_padding(&mut self, padding: f64) {
        self.padding = padding;
        self.layout();
    }

    /// The distribution mode.
    pub fn distribution(&self) -> Distribution {
        self.distribution
    }

    /// Set distribution mode.
    pub fn set_distribution(&mut self, dist: Distribution) {
        self.distribution = dist;
        self.layout();
    }

    /// Add a child view. Its frame will be repositioned by the layout.
    pub fn add_arranged_subview(&mut self, view: View) {
        self.view.add_subview(view);
        self.layout();
    }

    /// Number of arranged subviews.
    pub fn arranged_subview_count(&self) -> usize {
        self.view.subviews().len()
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Recompute child layout based on axis, spacing, distribution, and padding.
    pub fn layout(&mut self) {
        let count = self.view.subviews().len();
        if count == 0 {
            self.child_frames.clear();
            return;
        }

        let frame = self.view.frame;
        let inner_x = frame.x + self.padding;
        let inner_y = frame.y + self.padding;
        let inner_w = (frame.width - 2.0 * self.padding).max(0.0);
        let inner_h = (frame.height - 2.0 * self.padding).max(0.0);

        let total_spacing = self.spacing * (count as f64 - 1.0).max(0.0);

        self.child_frames.clear();

        match self.axis {
            Axis::Vertical => match self.distribution {
                Distribution::FillEqually => {
                    let child_h = ((inner_h - total_spacing) / count as f64).max(0.0);
                    let mut y = inner_y;
                    for _ in 0..count {
                        self.child_frames
                            .push(Rect::new(inner_x, y, inner_w, child_h));
                        y += child_h + self.spacing;
                    }
                }
                Distribution::Natural => {
                    let mut y = inner_y;
                    let subviews = self.view.subviews();
                    for sv in subviews {
                        let child_h = sv.frame.height;
                        self.child_frames
                            .push(Rect::new(inner_x, y, inner_w, child_h));
                        y += child_h + self.spacing;
                    }
                }
                Distribution::FillProportionally => {
                    let subviews = self.view.subviews();
                    let total_intrinsic: f64 = subviews.iter().map(|sv| sv.frame.height).sum();
                    let available = inner_h - total_spacing;
                    let mut y = inner_y;
                    for sv in subviews {
                        let child_h = if total_intrinsic > 0.0 {
                            (sv.frame.height / total_intrinsic * available).max(0.0)
                        } else {
                            (available / count as f64).max(0.0)
                        };
                        self.child_frames
                            .push(Rect::new(inner_x, y, inner_w, child_h));
                        y += child_h + self.spacing;
                    }
                }
                Distribution::EqualSpacing => {
                    let subviews = self.view.subviews();
                    let total_child: f64 = subviews.iter().map(|sv| sv.frame.height).sum();
                    let remaining = (inner_h - total_child).max(0.0);
                    let gap = if count > 1 {
                        remaining / (count as f64 - 1.0)
                    } else {
                        0.0
                    };
                    let mut y = inner_y;
                    for sv in subviews {
                        let child_h = sv.frame.height;
                        self.child_frames
                            .push(Rect::new(inner_x, y, inner_w, child_h));
                        y += child_h + gap;
                    }
                }
                Distribution::EqualCentering => {
                    let subviews = self.view.subviews();
                    let center_spacing = if count > 1 {
                        inner_h / count as f64
                    } else {
                        inner_h
                    };
                    for (i, sv) in subviews.iter().enumerate() {
                        let child_h = sv.frame.height;
                        let center_y = inner_y + center_spacing * (i as f64 + 0.5);
                        self.child_frames.push(Rect::new(
                            inner_x,
                            center_y - child_h / 2.0,
                            inner_w,
                            child_h,
                        ));
                    }
                }
            },
            Axis::Horizontal => match self.distribution {
                Distribution::FillEqually => {
                    let child_w = ((inner_w - total_spacing) / count as f64).max(0.0);
                    let mut x = inner_x;
                    for _ in 0..count {
                        self.child_frames
                            .push(Rect::new(x, inner_y, child_w, inner_h));
                        x += child_w + self.spacing;
                    }
                }
                Distribution::Natural => {
                    let mut x = inner_x;
                    let subviews = self.view.subviews();
                    for sv in subviews {
                        let child_w = sv.frame.width;
                        self.child_frames
                            .push(Rect::new(x, inner_y, child_w, inner_h));
                        x += child_w + self.spacing;
                    }
                }
                Distribution::FillProportionally => {
                    let subviews = self.view.subviews();
                    let total_intrinsic: f64 = subviews.iter().map(|sv| sv.frame.width).sum();
                    let available = inner_w - total_spacing;
                    let mut x = inner_x;
                    for sv in subviews {
                        let child_w = if total_intrinsic > 0.0 {
                            (sv.frame.width / total_intrinsic * available).max(0.0)
                        } else {
                            (available / count as f64).max(0.0)
                        };
                        self.child_frames
                            .push(Rect::new(x, inner_y, child_w, inner_h));
                        x += child_w + self.spacing;
                    }
                }
                Distribution::EqualSpacing => {
                    let subviews = self.view.subviews();
                    let total_child: f64 = subviews.iter().map(|sv| sv.frame.width).sum();
                    let remaining = (inner_w - total_child).max(0.0);
                    let gap = if count > 1 {
                        remaining / (count as f64 - 1.0)
                    } else {
                        0.0
                    };
                    let mut x = inner_x;
                    for sv in subviews {
                        let child_w = sv.frame.width;
                        self.child_frames
                            .push(Rect::new(x, inner_y, child_w, inner_h));
                        x += child_w + gap;
                    }
                }
                Distribution::EqualCentering => {
                    let subviews = self.view.subviews();
                    let center_spacing = if count > 1 {
                        inner_w / count as f64
                    } else {
                        inner_w
                    };
                    for (i, sv) in subviews.iter().enumerate() {
                        let child_w = sv.frame.width;
                        let center_x = inner_x + center_spacing * (i as f64 + 0.5);
                        self.child_frames.push(Rect::new(
                            center_x - child_w / 2.0,
                            inner_y,
                            child_w,
                            inner_h,
                        ));
                    }
                }
            },
        }

        // Apply computed frames
        let subviews = self.view.subviews_mut();
        for (i, frame) in self.child_frames.iter().enumerate() {
            if i < subviews.len() {
                subviews[i].set_frame(*frame);
            }
        }
    }

    /// Draw the stack view and its children.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        self.view.draw(renderer);
    }

    /// Recompute layout using the constraint engine.
    ///
    /// Generates constraints that model the stacking behaviour (axis, spacing,
    /// distribution, padding) and solves them with [`LayoutEngine`]. Each child
    /// view must have a `layout_id` set; if any child is missing an ID, this
    /// method falls back to the simple `layout()` path.
    pub fn layout_with_constraints(&mut self) {
        let count = self.view.subviews().len();
        if count == 0 {
            self.child_frames.clear();
            return;
        }

        // Collect child layout_ids — fall back to simple layout if any are missing.
        let ids: Vec<ViewId> = self
            .view
            .subviews()
            .iter()
            .filter_map(|sv| sv.layout_id)
            .collect();
        if ids.len() != count {
            self.layout();
            return;
        }

        let mut engine = LayoutEngine::new(self.view.frame);

        // All children pin to superview cross-axis and are chained along primary axis.
        for (i, &vid) in ids.iter().enumerate() {
            match self.axis {
                Axis::Vertical => {
                    // Pin left/right to superview with padding.
                    engine.add_constraint(constraint::equal(
                        vid,
                        LayoutAttribute::Left,
                        SUPERVIEW,
                        LayoutAttribute::Left,
                        self.padding,
                    ));
                    engine.add_constraint(constraint::equal(
                        vid,
                        LayoutAttribute::Right,
                        SUPERVIEW,
                        LayoutAttribute::Right,
                        -self.padding,
                    ));
                    // Chain vertically.
                    if i == 0 {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Top,
                            SUPERVIEW,
                            LayoutAttribute::Top,
                            self.padding,
                        ));
                    } else {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Top,
                            ids[i - 1],
                            LayoutAttribute::Bottom,
                            self.spacing,
                        ));
                    }
                    if self.distribution == Distribution::FillEqually && i > 0 {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Height,
                            ids[0],
                            LayoutAttribute::Height,
                            0.0,
                        ));
                    }
                }
                Axis::Horizontal => {
                    // Pin top/bottom to superview with padding.
                    engine.add_constraint(constraint::equal(
                        vid,
                        LayoutAttribute::Top,
                        SUPERVIEW,
                        LayoutAttribute::Top,
                        self.padding,
                    ));
                    engine.add_constraint(constraint::equal(
                        vid,
                        LayoutAttribute::Bottom,
                        SUPERVIEW,
                        LayoutAttribute::Bottom,
                        -self.padding,
                    ));
                    // Chain horizontally.
                    if i == 0 {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Left,
                            SUPERVIEW,
                            LayoutAttribute::Left,
                            self.padding,
                        ));
                    } else {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Left,
                            ids[i - 1],
                            LayoutAttribute::Right,
                            self.spacing,
                        ));
                    }
                    if self.distribution == Distribution::FillEqually && i > 0 {
                        engine.add_constraint(constraint::equal(
                            vid,
                            LayoutAttribute::Width,
                            ids[0],
                            LayoutAttribute::Width,
                            0.0,
                        ));
                    }
                }
            }
        }

        // Last child pins to superview trailing edge (for FillEqually).
        if self.distribution == Distribution::FillEqually {
            let last = *ids.last().unwrap();
            match self.axis {
                Axis::Vertical => {
                    engine.add_constraint(constraint::equal(
                        last,
                        LayoutAttribute::Bottom,
                        SUPERVIEW,
                        LayoutAttribute::Bottom,
                        -self.padding,
                    ));
                }
                Axis::Horizontal => {
                    engine.add_constraint(constraint::equal(
                        last,
                        LayoutAttribute::Right,
                        SUPERVIEW,
                        LayoutAttribute::Right,
                        -self.padding,
                    ));
                }
            }
        }

        // Solve and apply.
        if let Ok(frames) = engine.solve() {
            self.child_frames.clear();
            let subviews = self.view.subviews_mut();
            for (i, &vid) in ids.iter().enumerate() {
                if let Some(&rect) = frames.get(&vid) {
                    if i < subviews.len() {
                        subviews[i].set_frame(rect);
                    }
                    self.child_frames.push(rect);
                }
            }
        } else {
            // Conflict — fall back to simple layout.
            self.layout();
        }
    }
}
