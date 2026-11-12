//! SplitView — two panes with an adjustable divider.
//!
//! Supports horizontal (side-by-side) and vertical (top/bottom) orientation.
//! The divider position can be set programmatically or (in a GUI backend)
//! dragged by the user.

use graphics_core::{Color, Point, Rect, Renderer};
use ui_view::View;

/// Orientation of the split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// Left | Right
    Horizontal,
    /// Top / Bottom
    Vertical,
}

/// A split view dividing space between two child areas.
pub struct SplitView {
    view: View,
    orientation: SplitOrientation,
    /// Position of the divider along the primary axis (absolute, from start).
    divider_position: f64,
    /// Thickness of the divider bar in points.
    divider_thickness: f64,
    /// Color of the divider bar.
    divider_color: Color,
    /// Minimum size for the first pane.
    min_first: f64,
    /// Minimum size for the second pane.
    min_second: f64,
}

impl SplitView {
    /// Create a split view with frame and orientation.
    pub fn new(frame: Rect, orientation: SplitOrientation) -> Self {
        let initial_pos = match orientation {
            SplitOrientation::Horizontal => frame.width / 2.0,
            SplitOrientation::Vertical => frame.height / 2.0,
        };
        Self {
            view: View::new(frame),
            orientation,
            divider_position: initial_pos,
            divider_thickness: 5.0,
            divider_color: Color::new(0.75, 0.75, 0.75, 1.0),
            min_first: 50.0,
            min_second: 50.0,
        }
    }

    /// The split orientation.
    pub fn orientation(&self) -> SplitOrientation {
        self.orientation
    }

    /// Current divider position.
    pub fn divider_position(&self) -> f64 {
        self.divider_position
    }

    /// Set the divider position, clamped to valid range.
    pub fn set_divider_position(&mut self, pos: f64) {
        self.divider_position = self.clamp_position(pos);
        self.view.set_needs_display();
    }

    /// Divider thickness in points.
    pub fn divider_thickness(&self) -> f64 {
        self.divider_thickness
    }

    /// Set divider thickness.
    pub fn set_divider_thickness(&mut self, t: f64) {
        self.divider_thickness = t;
        self.view.set_needs_display();
    }

    /// Set minimum sizes for the two panes.
    pub fn set_min_sizes(&mut self, first: f64, second: f64) {
        self.min_first = first;
        self.min_second = second;
        // Re-clamp
        let pos = self.divider_position;
        self.divider_position = self.clamp_position(pos);
    }

    /// Get the frame of the first (left/top) pane in the split view's coordinates.
    pub fn first_pane_frame(&self) -> Rect {
        let frame = self.view.frame;
        match self.orientation {
            SplitOrientation::Horizontal => {
                Rect::new(frame.x, frame.y, self.divider_position, frame.height)
            }
            SplitOrientation::Vertical => {
                Rect::new(frame.x, frame.y, frame.width, self.divider_position)
            }
        }
    }

    /// Get the frame of the second (right/bottom) pane.
    pub fn second_pane_frame(&self) -> Rect {
        let frame = self.view.frame;
        let after_divider = self.divider_position + self.divider_thickness;
        match self.orientation {
            SplitOrientation::Horizontal => Rect::new(
                frame.x + after_divider,
                frame.y,
                (frame.width - after_divider).max(0.0),
                frame.height,
            ),
            SplitOrientation::Vertical => Rect::new(
                frame.x,
                frame.y + after_divider,
                frame.width,
                (frame.height - after_divider).max(0.0),
            ),
        }
    }

    /// Get the divider rect.
    pub fn divider_rect(&self) -> Rect {
        let frame = self.view.frame;
        match self.orientation {
            SplitOrientation::Horizontal => Rect::new(
                frame.x + self.divider_position,
                frame.y,
                self.divider_thickness,
                frame.height,
            ),
            SplitOrientation::Vertical => Rect::new(
                frame.x,
                frame.y + self.divider_position,
                frame.width,
                self.divider_thickness,
            ),
        }
    }

    /// Check if a point hits the divider (for drag handling).
    pub fn hit_test_divider(&self, point: Point) -> bool {
        self.divider_rect().contains(point)
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable view access.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Draw the split view frame and divider.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Background
        if self.view.background_color.a > 0.0 {
            renderer.fill_rect(self.view.frame, self.view.background_color);
        }

        // Divider
        renderer.fill_rect(self.divider_rect(), self.divider_color);

        // Border
        renderer.stroke_rect(self.view.frame, Color::new(0.7, 0.7, 0.7, 1.0), 1.0);
    }

    fn clamp_position(&self, pos: f64) -> f64 {
        let total = match self.orientation {
            SplitOrientation::Horizontal => self.view.frame.width,
            SplitOrientation::Vertical => self.view.frame.height,
        };
        let max = (total - self.divider_thickness - self.min_second).max(self.min_first);
        pos.clamp(self.min_first, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_horizontal_basic() {
        let sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Horizontal,
        );
        assert_eq!(sv.orientation(), SplitOrientation::Horizontal);
        // Default: half
        assert!((sv.divider_position() - 200.0).abs() < 0.01);
    }

    #[test]
    fn split_vertical_basic() {
        let sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Vertical,
        );
        assert!((sv.divider_position() - 150.0).abs() < 0.01);
    }

    #[test]
    fn split_pane_frames_horizontal() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Horizontal,
        );
        sv.set_divider_position(150.0);

        let first = sv.first_pane_frame();
        assert_eq!(first.width, 150.0);
        assert_eq!(first.height, 300.0);

        let second = sv.second_pane_frame();
        assert!((second.x - 155.0).abs() < 0.01); // 150 + 5 divider
        assert!((second.width - 245.0).abs() < 0.01);
    }

    #[test]
    fn split_pane_frames_vertical() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Vertical,
        );
        sv.set_divider_position(100.0);

        let first = sv.first_pane_frame();
        assert_eq!(first.height, 100.0);

        let second = sv.second_pane_frame();
        assert!((second.y - 105.0).abs() < 0.01);
        assert!((second.height - 195.0).abs() < 0.01);
    }

    #[test]
    fn split_divider_clamp() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Horizontal,
        );
        // Try to push past max
        sv.set_divider_position(999.0);
        assert!(sv.divider_position() <= 400.0 - sv.divider_thickness() - 50.0);
        // Try negative
        sv.set_divider_position(-100.0);
        assert!(sv.divider_position() >= 50.0);
    }

    #[test]
    fn split_hit_test_divider() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Horizontal,
        );
        sv.set_divider_position(200.0);

        assert!(sv.hit_test_divider(Point::new(202.0, 150.0)));
        assert!(!sv.hit_test_divider(Point::new(100.0, 150.0)));
        assert!(!sv.hit_test_divider(Point::new(300.0, 150.0)));
    }

    #[test]
    fn split_min_sizes() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 200.0, 100.0),
            SplitOrientation::Horizontal,
        );
        sv.set_min_sizes(80.0, 80.0);
        // Setting to 50 should clamp to 80
        sv.set_divider_position(50.0);
        assert!((sv.divider_position() - 80.0).abs() < 0.01);
    }

    #[test]
    fn split_draw_does_not_panic() {
        let mut sv = SplitView::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            SplitOrientation::Horizontal,
        );
        sv.view_mut().background_color = Color::WHITE;
        let mut surface = graphics_core::Surface::new(400, 300);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        sv.draw(&mut renderer);
    }
}
