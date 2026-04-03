//! ScrollView — a scrollable container for content larger than its frame.
//!
//! `ScrollView` wraps a content region with a viewport. The `content_offset`
//! determines which portion of the content is visible. Scroll bars are
//! drawn when content exceeds the visible area.

use graphics_core::{Color, Point, Rect, Renderer};
use ui_view::View;

/// A scrollable view that clips content to a viewport.
pub struct ScrollView {
    view: View,
    /// Offset into the content area (how far scrolled).
    content_offset: Point,
    /// The total size of the scrollable content.
    content_size: (f64, f64),
    /// Whether horizontal scrolling is enabled.
    horizontal_scroll_enabled: bool,
    /// Whether vertical scrolling is enabled.
    vertical_scroll_enabled: bool,
    /// Whether to draw scroll indicators.
    shows_indicators: bool,
    /// Scroll indicator color.
    indicator_color: Color,
}

impl ScrollView {
    /// Create a scroll view with the given frame.
    pub fn new(frame: Rect) -> Self {
        Self {
            view: View::new(frame),
            content_offset: Point::ZERO,
            content_size: (frame.width, frame.height),
            horizontal_scroll_enabled: true,
            vertical_scroll_enabled: true,
            shows_indicators: true,
            indicator_color: Color::new(0.5, 0.5, 0.5, 0.6),
        }
    }

    /// The current scroll offset.
    pub fn content_offset(&self) -> Point {
        self.content_offset
    }

    /// Set the scroll offset, clamped to valid range.
    pub fn set_content_offset(&mut self, offset: Point) {
        self.content_offset = self.clamp_offset(offset);
        self.view.set_needs_display();
    }

    /// The total content size.
    pub fn content_size(&self) -> (f64, f64) {
        self.content_size
    }

    /// Set the content size.
    pub fn set_content_size(&mut self, width: f64, height: f64) {
        self.content_size = (width, height);
        // Re-clamp offset in case content shrank
        let offset = self.content_offset;
        self.content_offset = self.clamp_offset(offset);
        self.view.set_needs_display();
    }

    /// Scroll by a delta amount.
    pub fn scroll_by(&mut self, dx: f64, dy: f64) {
        let new_offset = Point::new(
            self.content_offset.x + dx,
            self.content_offset.y + dy,
        );
        self.set_content_offset(new_offset);
    }

    /// Whether content can scroll vertically (content taller than frame).
    pub fn can_scroll_vertically(&self) -> bool {
        self.vertical_scroll_enabled && self.content_size.1 > self.view.frame.height
    }

    /// Whether content can scroll horizontally (content wider than frame).
    pub fn can_scroll_horizontally(&self) -> bool {
        self.horizontal_scroll_enabled && self.content_size.0 > self.view.frame.width
    }

    /// Enable/disable vertical scrolling.
    pub fn set_vertical_scroll_enabled(&mut self, enabled: bool) {
        self.vertical_scroll_enabled = enabled;
    }

    /// Enable/disable horizontal scrolling.
    pub fn set_horizontal_scroll_enabled(&mut self, enabled: bool) {
        self.horizontal_scroll_enabled = enabled;
    }

    /// Show/hide scroll indicators.
    pub fn set_shows_indicators(&mut self, shows: bool) {
        self.shows_indicators = shows;
    }

    /// The visible rect in content coordinates.
    pub fn visible_rect(&self) -> Rect {
        Rect::new(
            self.content_offset.x,
            self.content_offset.y,
            self.view.frame.width,
            self.view.frame.height,
        )
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Draw the scroll view frame along with scroll indicators.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        if self.view.background_color.a > 0.0 {
            renderer.fill_rect(self.view.frame, self.view.background_color);
        }

        // Draw border
        renderer.stroke_rect(
            self.view.frame,
            Color::new(0.7, 0.7, 0.7, 1.0),
            1.0,
        );

        // Draw scroll indicators
        if self.shows_indicators {
            self.draw_indicators(renderer);
        }
    }

    /// Draw scroll indicator bars.
    fn draw_indicators(&self, renderer: &mut dyn Renderer) {
        let frame = self.view.frame;
        let indicator_thickness = 6.0;
        let min_indicator_length = 20.0;

        // Vertical indicator
        if self.can_scroll_vertically() {
            let visible_ratio = frame.height / self.content_size.1;
            let indicator_height =
                (frame.height * visible_ratio).max(min_indicator_length);
            let max_scroll = self.content_size.1 - frame.height;
            let scroll_ratio = if max_scroll > 0.0 {
                self.content_offset.y / max_scroll
            } else {
                0.0
            };
            let track_height = frame.height - indicator_height;
            let indicator_y = frame.y + track_height * scroll_ratio;
            let indicator_rect = Rect::new(
                frame.x + frame.width - indicator_thickness - 1.0,
                indicator_y,
                indicator_thickness,
                indicator_height,
            );
            renderer.fill_rect(indicator_rect, self.indicator_color);
        }

        // Horizontal indicator
        if self.can_scroll_horizontally() {
            let visible_ratio = frame.width / self.content_size.0;
            let indicator_width =
                (frame.width * visible_ratio).max(min_indicator_length);
            let max_scroll = self.content_size.0 - frame.width;
            let scroll_ratio = if max_scroll > 0.0 {
                self.content_offset.x / max_scroll
            } else {
                0.0
            };
            let track_width = frame.width - indicator_width;
            let indicator_x = frame.x + track_width * scroll_ratio;
            let indicator_rect = Rect::new(
                indicator_x,
                frame.y + frame.height - indicator_thickness - 1.0,
                indicator_width,
                indicator_thickness,
            );
            renderer.fill_rect(indicator_rect, self.indicator_color);
        }
    }

    /// Clamp an offset to the valid scrollable range.
    fn clamp_offset(&self, offset: Point) -> Point {
        let max_x = if self.horizontal_scroll_enabled {
            (self.content_size.0 - self.view.frame.width).max(0.0)
        } else {
            0.0
        };
        let max_y = if self.vertical_scroll_enabled {
            (self.content_size.1 - self.view.frame.height).max(0.0)
        } else {
            0.0
        };
        Point::new(
            offset.x.clamp(0.0, max_x),
            offset.y.clamp(0.0, max_y),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollview_basic() {
        let sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(sv.content_offset(), Point::ZERO);
        assert_eq!(sv.content_size(), (200.0, 100.0));
    }

    #[test]
    fn scrollview_set_content_size() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 500.0);
        assert!(sv.can_scroll_vertically());
        assert!(!sv.can_scroll_horizontally());
    }

    #[test]
    fn scrollview_scroll_by() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 500.0);
        sv.scroll_by(0.0, 50.0);
        assert_eq!(sv.content_offset().y, 50.0);
    }

    #[test]
    fn scrollview_clamp_offset() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 300.0);
        // Try to scroll past the max
        sv.set_content_offset(Point::new(0.0, 999.0));
        assert_eq!(sv.content_offset().y, 200.0); // 300 - 100
        // Try negative
        sv.set_content_offset(Point::new(0.0, -50.0));
        assert_eq!(sv.content_offset().y, 0.0);
    }

    #[test]
    fn scrollview_visible_rect() {
        let mut sv = ScrollView::new(Rect::new(10.0, 20.0, 200.0, 100.0));
        sv.set_content_size(400.0, 500.0);
        sv.set_content_offset(Point::new(30.0, 40.0));
        let vis = sv.visible_rect();
        assert_eq!(vis.x, 30.0);
        assert_eq!(vis.y, 40.0);
        assert_eq!(vis.width, 200.0);
        assert_eq!(vis.height, 100.0);
    }

    #[test]
    fn scrollview_disable_horizontal() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(500.0, 500.0);
        sv.set_horizontal_scroll_enabled(false);
        assert!(!sv.can_scroll_horizontally());
        // Horizontal offset stays 0 after scroll_by
        sv.scroll_by(50.0, 0.0);
        assert_eq!(sv.content_offset().x, 0.0);
    }

    #[test]
    fn scrollview_content_smaller_than_frame() {
        let sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        // Default content_size == frame size, no scrolling possible
        assert!(!sv.can_scroll_vertically());
        assert!(!sv.can_scroll_horizontally());
    }

    #[test]
    fn scrollview_draw() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 300.0);
        sv.view_mut().background_color = Color::WHITE;
        let mut surface = graphics_core::Surface::new(200, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        sv.draw(&mut renderer); // should not panic
    }

    #[test]
    fn scrollview_clamp_offset_negative() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(400.0, 500.0);
        sv.set_content_offset(Point::new(-10.0, -20.0));
        assert_eq!(sv.content_offset().x, 0.0);
        assert_eq!(sv.content_offset().y, 0.0);
    }

    #[test]
    fn scrollview_clamp_offset_max() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 500.0);
        sv.set_content_offset(Point::new(0.0, 600.0));
        assert_eq!(sv.content_offset().y, 400.0);
    }

    #[test]
    fn scrollview_clamp_offset_horizontal() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(400.0, 100.0);
        sv.set_content_offset(Point::new(300.0, 0.0));
        assert_eq!(sv.content_offset().x, 200.0);
    }

    #[test]
    fn scrollview_disable_vertical() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 500.0);
        sv.set_vertical_scroll_enabled(false);
        assert!(!sv.can_scroll_vertically());
    }

    #[test]
    fn scrollview_scroll_by_clamps() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 500.0);
        sv.scroll_by(0.0, 9999.0);
        assert_eq!(sv.content_offset().y, 400.0);
    }

    #[test]
    fn scrollview_content_shrink_reclamps() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 1000.0);
        sv.set_content_offset(Point::new(0.0, 800.0));
        assert_eq!(sv.content_offset().y, 800.0);
        // Shrink content — offset must be re-clamped
        sv.set_content_size(200.0, 300.0);
        assert_eq!(sv.content_offset().y, 200.0);
    }

    #[test]
    fn scrollview_no_scroll_when_content_fits() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(200.0, 100.0);
        assert!(!sv.can_scroll_vertically());
        assert!(!sv.can_scroll_horizontally());
    }

    #[test]
    fn scrollview_shows_indicators() {
        let sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        assert!(sv.shows_indicators);
        let mut sv = sv;
        sv.set_shows_indicators(false);
        assert!(!sv.shows_indicators);
    }

    #[test]
    fn scrollview_view_access() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(sv.view().frame.width, 200.0);
        sv.view_mut().background_color = Color::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(sv.view().background_color.r, 1.0);
    }

    #[test]
    fn scrollview_draw_no_panic() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        let mut surface = graphics_core::Surface::new(200, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        sv.draw(&mut renderer);
    }

    #[test]
    fn scrollview_draw_with_indicators_no_panic() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.set_content_size(400.0, 500.0);
        sv.set_content_offset(Point::new(50.0, 100.0));
        let mut surface = graphics_core::Surface::new(200, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        sv.draw(&mut renderer);
    }

    #[test]
    fn scrollview_draw_hidden_no_panic() {
        let mut sv = ScrollView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        sv.view_mut().hidden = true;
        let mut surface = graphics_core::Surface::new(200, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        sv.draw(&mut renderer);
    }
}
