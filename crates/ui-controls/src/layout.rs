//! Layout helpers — StackView and layout utilities.
//!
//! Provides `StackView` (horizontal/vertical stacking with spacing)
//! and helper functions for common layout patterns.

use graphics_core::{Rect, Renderer};
use ui_view::layout::{
    self as constraint, LayoutAttribute, LayoutEngine, ViewId,
    SUPERVIEW,
};
use ui_view::View;

/// Axis along which a stack view arranges its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// How children are distributed along the primary axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distribution {
    /// Each child keeps its own size (natural).
    Natural,
    /// Children are resized equally to fill the available space.
    FillEqually,
}

/// A stack view that arranges child views along an axis.
pub struct StackView {
    view: View,
    axis: Axis,
    spacing: f64,
    distribution: Distribution,
    padding: f64,
    /// Child frames managed by the layout engine.
    child_frames: Vec<Rect>,
}

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
            Axis::Vertical => {
                match self.distribution {
                    Distribution::FillEqually => {
                        let child_h =
                            ((inner_h - total_spacing) / count as f64).max(0.0);
                        let mut y = inner_y;
                        for _ in 0..count {
                            self.child_frames.push(Rect::new(
                                inner_x, y, inner_w, child_h,
                            ));
                            y += child_h + self.spacing;
                        }
                    }
                    Distribution::Natural => {
                        let mut y = inner_y;
                        let subviews = self.view.subviews();
                        for sv in subviews {
                            let child_h = sv.frame.height;
                            self.child_frames.push(Rect::new(
                                inner_x, y, inner_w, child_h,
                            ));
                            y += child_h + self.spacing;
                        }
                    }
                }
            }
            Axis::Horizontal => {
                match self.distribution {
                    Distribution::FillEqually => {
                        let child_w =
                            ((inner_w - total_spacing) / count as f64).max(0.0);
                        let mut x = inner_x;
                        for _ in 0..count {
                            self.child_frames.push(Rect::new(
                                x, inner_y, child_w, inner_h,
                            ));
                            x += child_w + self.spacing;
                        }
                    }
                    Distribution::Natural => {
                        let mut x = inner_x;
                        let subviews = self.view.subviews();
                        for sv in subviews {
                            let child_w = sv.frame.width;
                            self.child_frames.push(Rect::new(
                                x, inner_y, child_w, inner_h,
                            ));
                            x += child_w + self.spacing;
                        }
                    }
                }
            }
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
                        vid, LayoutAttribute::Left,
                        SUPERVIEW, LayoutAttribute::Left,
                        self.padding,
                    ));
                    engine.add_constraint(constraint::equal(
                        vid, LayoutAttribute::Right,
                        SUPERVIEW, LayoutAttribute::Right,
                        -self.padding,
                    ));
                    // Chain vertically.
                    if i == 0 {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Top,
                            SUPERVIEW, LayoutAttribute::Top,
                            self.padding,
                        ));
                    } else {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Top,
                            ids[i - 1], LayoutAttribute::Bottom,
                            self.spacing,
                        ));
                    }
                    if self.distribution == Distribution::FillEqually && i > 0 {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Height,
                            ids[0], LayoutAttribute::Height,
                            0.0,
                        ));
                    }
                }
                Axis::Horizontal => {
                    // Pin top/bottom to superview with padding.
                    engine.add_constraint(constraint::equal(
                        vid, LayoutAttribute::Top,
                        SUPERVIEW, LayoutAttribute::Top,
                        self.padding,
                    ));
                    engine.add_constraint(constraint::equal(
                        vid, LayoutAttribute::Bottom,
                        SUPERVIEW, LayoutAttribute::Bottom,
                        -self.padding,
                    ));
                    // Chain horizontally.
                    if i == 0 {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Left,
                            SUPERVIEW, LayoutAttribute::Left,
                            self.padding,
                        ));
                    } else {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Left,
                            ids[i - 1], LayoutAttribute::Right,
                            self.spacing,
                        ));
                    }
                    if self.distribution == Distribution::FillEqually && i > 0 {
                        engine.add_constraint(constraint::equal(
                            vid, LayoutAttribute::Width,
                            ids[0], LayoutAttribute::Width,
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
                        last, LayoutAttribute::Bottom,
                        SUPERVIEW, LayoutAttribute::Bottom,
                        -self.padding,
                    ));
                }
                Axis::Horizontal => {
                    engine.add_constraint(constraint::equal(
                        last, LayoutAttribute::Right,
                        SUPERVIEW, LayoutAttribute::Right,
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

// ── Utility functions ──────────────────────────────────────────────

/// Lay out rectangles vertically with given spacing, starting at `origin`.
/// Returns the computed frames.
pub fn layout_vertical(
    origin: (f64, f64),
    widths_heights: &[(f64, f64)],
    spacing: f64,
) -> Vec<Rect> {
    let mut frames = Vec::with_capacity(widths_heights.len());
    let mut y = origin.1;
    for &(w, h) in widths_heights {
        frames.push(Rect::new(origin.0, y, w, h));
        y += h + spacing;
    }
    frames
}

/// Lay out rectangles horizontally with given spacing, starting at `origin`.
/// Returns the computed frames.
pub fn layout_horizontal(
    origin: (f64, f64),
    widths_heights: &[(f64, f64)],
    spacing: f64,
) -> Vec<Rect> {
    let mut frames = Vec::with_capacity(widths_heights.len());
    let mut x = origin.0;
    for &(w, h) in widths_heights {
        frames.push(Rect::new(x, origin.1, w, h));
        x += w + spacing;
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphics_core::Color;

    #[test]
    fn stack_vertical_fill_equally() {
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 200.0, 300.0), Axis::Vertical);
        stack.set_spacing(10.0);
        stack.set_distribution(Distribution::FillEqually);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));

        // 300 - 20 (spacing) = 280, 280/3 ≈ 93.33
        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 3);
        // First child at y=0
        assert!((subs[0].frame.y - 0.0).abs() < 0.01);
        // All same height
        let h0 = subs[0].frame.height;
        assert!((subs[1].frame.height - h0).abs() < 0.01);
        assert!((subs[2].frame.height - h0).abs() < 0.01);
    }

    #[test]
    fn stack_horizontal_fill_equally() {
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 300.0, 50.0), Axis::Horizontal);
        stack.set_spacing(10.0);
        stack.set_distribution(Distribution::FillEqually);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 50.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 50.0, 30.0)));

        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 2);
        // 300 - 10 = 290, 290/2 = 145 each
        assert!((subs[0].frame.width - 145.0).abs() < 0.01);
        assert!((subs[1].frame.width - 145.0).abs() < 0.01);
    }

    #[test]
    fn stack_vertical_natural() {
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 200.0, 400.0), Axis::Vertical);
        stack.set_spacing(5.0);
        stack.set_distribution(Distribution::Natural);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 200.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 200.0, 50.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 200.0, 20.0)));

        let subs = stack.view().subviews();
        // Natural: keep original heights
        assert!((subs[0].frame.height - 30.0).abs() < 0.01);
        assert!((subs[1].frame.height - 50.0).abs() < 0.01);
        assert!((subs[2].frame.height - 20.0).abs() < 0.01);
        // Y positions: 0, 35, 90
        assert!((subs[0].frame.y - 0.0).abs() < 0.01);
        assert!((subs[1].frame.y - 35.0).abs() < 0.01);
        assert!((subs[2].frame.y - 90.0).abs() < 0.01);
    }

    #[test]
    fn stack_with_padding() {
        let mut stack = StackView::new(Rect::new(10.0, 10.0, 200.0, 300.0), Axis::Vertical);
        stack.set_padding(10.0);
        stack.set_spacing(0.0);
        stack.set_distribution(Distribution::FillEqually);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));

        let subs = stack.view().subviews();
        // With 10px padding: inner starts at 20, width = 180, height = 280
        assert!((subs[0].frame.x - 20.0).abs() < 0.01);
        assert!((subs[0].frame.y - 20.0).abs() < 0.01);
        assert!((subs[0].frame.width - 180.0).abs() < 0.01);
    }

    #[test]
    fn layout_vertical_helper() {
        let frames = layout_vertical((10.0, 20.0), &[(100.0, 30.0), (100.0, 40.0)], 5.0);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], Rect::new(10.0, 20.0, 100.0, 30.0));
        assert_eq!(frames[1], Rect::new(10.0, 55.0, 100.0, 40.0));
    }

    #[test]
    fn layout_horizontal_helper() {
        let frames = layout_horizontal((10.0, 20.0), &[(50.0, 30.0), (60.0, 30.0)], 10.0);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], Rect::new(10.0, 20.0, 50.0, 30.0));
        assert_eq!(frames[1], Rect::new(70.0, 20.0, 60.0, 30.0));
    }

    #[test]
    fn stack_empty() {
        let stack = StackView::new(Rect::new(0.0, 0.0, 200.0, 200.0), Axis::Vertical);
        assert_eq!(stack.arranged_subview_count(), 0);
    }

    #[test]
    fn stack_draw_does_not_panic() {
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 200.0, 200.0), Axis::Vertical);
        stack.view_mut().background_color = Color::WHITE;
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 30.0)));
        let mut surface = graphics_core::Surface::new(200, 200);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        stack.draw(&mut renderer);
    }

    #[test]
    fn stack_constraint_fill_equally() {
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 300.0, 100.0), Axis::Horizontal);
        stack.set_spacing(10.0);
        stack.set_distribution(Distribution::FillEqually);

        let mut a = View::new(Rect::new(0.0, 0.0, 50.0, 30.0));
        a.layout_id = Some(ViewId(1));
        let mut b = View::new(Rect::new(0.0, 0.0, 50.0, 30.0));
        b.layout_id = Some(ViewId(2));
        stack.add_arranged_subview(a);
        stack.add_arranged_subview(b);

        stack.layout_with_constraints();

        let subs = stack.view().subviews();
        // 300 - 10 spacing = 290, 290 / 2 = 145 each
        assert!((subs[0].frame.width - 145.0).abs() < 2.0, "a.w={}", subs[0].frame.width);
        assert!((subs[1].frame.width - 145.0).abs() < 2.0, "b.w={}", subs[1].frame.width);
        assert!((subs[0].frame.x).abs() < 2.0, "a.x={}", subs[0].frame.x);
        assert!((subs[1].frame.x - 155.0).abs() < 2.0, "b.x={}", subs[1].frame.x);
    }

    #[test]
    fn stack_constraint_fallback_without_ids() {
        // Without layout_ids, layout_with_constraints falls back to layout()
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 200.0, 300.0), Axis::Vertical);
        stack.set_distribution(Distribution::FillEqually);
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 100.0, 50.0)));

        stack.layout_with_constraints();

        // Fallback to simple layout still works
        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 2);
        assert!(subs[0].frame.height > 0.0);
    }
}
