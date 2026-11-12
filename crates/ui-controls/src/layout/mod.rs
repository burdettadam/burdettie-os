//! Layout helpers — StackView and layout utilities.
//!
//! Provides `StackView` (horizontal/vertical stacking with spacing)
//! and helper functions for common layout patterns.

use graphics_core::Rect;
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
    /// Children sized proportional to their intrinsic (original) size,
    /// scaled to fill the available space.
    FillProportionally,
    /// Equal gaps between items; items keep their natural size.
    EqualSpacing,
    /// Equal distance between item centers; items keep their natural size.
    EqualCentering,
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

mod impl_stack_view;

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
        assert!(
            (subs[0].frame.width - 145.0).abs() < 2.0,
            "a.w={}",
            subs[0].frame.width
        );
        assert!(
            (subs[1].frame.width - 145.0).abs() < 2.0,
            "b.w={}",
            subs[1].frame.width
        );
        assert!((subs[0].frame.x).abs() < 2.0, "a.x={}", subs[0].frame.x);
        assert!(
            (subs[1].frame.x - 155.0).abs() < 2.0,
            "b.x={}",
            subs[1].frame.x
        );
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

    // ── New distribution mode tests ────────────────────────────────

    #[test]
    fn stack_fill_equally_three_items() {
        // 3 items in 300px horizontal stack with 0 spacing → each gets 100px
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 300.0, 50.0), Axis::Horizontal);
        stack.set_spacing(0.0);
        stack.set_distribution(Distribution::FillEqually);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 40.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 60.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 80.0, 30.0)));

        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 3);
        assert!(
            (subs[0].frame.width - 100.0).abs() < 0.01,
            "w0={}",
            subs[0].frame.width
        );
        assert!(
            (subs[1].frame.width - 100.0).abs() < 0.01,
            "w1={}",
            subs[1].frame.width
        );
        assert!(
            (subs[2].frame.width - 100.0).abs() < 0.01,
            "w2={}",
            subs[2].frame.width
        );
    }

    #[test]
    fn stack_equal_spacing_three_items() {
        // 3 items of 50px each in 300px horizontal stack → equal gaps
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 300.0, 50.0), Axis::Horizontal);
        stack.set_spacing(0.0);
        stack.set_distribution(Distribution::EqualSpacing);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 50.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 50.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 50.0, 30.0)));

        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 3);
        // Total child width = 150, remaining = 150, 2 gaps → 75 each
        assert!((subs[0].frame.width - 50.0).abs() < 0.01);
        assert!((subs[1].frame.width - 50.0).abs() < 0.01);
        assert!((subs[2].frame.width - 50.0).abs() < 0.01);
        assert!((subs[0].frame.x - 0.0).abs() < 0.01);
        assert!(
            (subs[1].frame.x - 125.0).abs() < 0.01,
            "x1={}",
            subs[1].frame.x
        );
        assert!(
            (subs[2].frame.x - 250.0).abs() < 0.01,
            "x2={}",
            subs[2].frame.x
        );
    }

    #[test]
    fn stack_equal_centering_three_items() {
        // 3 items center-spaced in 300px horizontal stack
        let mut stack = StackView::new(Rect::new(0.0, 0.0, 300.0, 50.0), Axis::Horizontal);
        stack.set_spacing(0.0);
        stack.set_distribution(Distribution::EqualCentering);

        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 40.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 40.0, 30.0)));
        stack.add_arranged_subview(View::new(Rect::new(0.0, 0.0, 40.0, 30.0)));

        let subs = stack.view().subviews();
        assert_eq!(subs.len(), 3);
        // center_spacing = 300 / 3 = 100
        // centers at 50, 150, 250 → x positions: 30, 130, 230
        let c0 = subs[0].frame.x + subs[0].frame.width / 2.0;
        let c1 = subs[1].frame.x + subs[1].frame.width / 2.0;
        let c2 = subs[2].frame.x + subs[2].frame.width / 2.0;
        assert!((c0 - 50.0).abs() < 0.01, "c0={}", c0);
        assert!((c1 - 150.0).abs() < 0.01, "c1={}", c1);
        assert!((c2 - 250.0).abs() < 0.01, "c2={}", c2);
    }
}
