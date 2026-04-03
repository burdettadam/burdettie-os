//! Slider control — continuous value selection within a range.

use graphics_core::{Color, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A slider control for selecting a continuous value within a range.
pub struct Slider {
    view: View,
    value: f64,
    min_value: f64,
    max_value: f64,
    enabled: bool,
    track_color: Color,
    knob_color: Color,
    on_change: Option<Box<dyn Fn(f64)>>,
}

impl Slider {
    /// Create a slider with the given frame and value range.
    /// Value starts at `min`, enabled, gray track, blue knob.
    pub fn new(frame: Rect, min: f64, max: f64) -> Self {
        Self {
            view: View::new(frame),
            value: min,
            min_value: min,
            max_value: max,
            enabled: true,
            track_color: Color::new(0.7, 0.7, 0.7, 1.0),
            knob_color: Color::new(0.2, 0.5, 1.0, 1.0),
            on_change: None,
        }
    }

    /// Get the current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the value, clamped to [min, max]. Triggers on_change callback.
    pub fn set_value(&mut self, value: f64) {
        self.value = value.clamp(self.min_value, self.max_value);
        self.view.set_needs_display();
        if let Some(ref cb) = self.on_change {
            cb(self.value);
        }
    }

    /// Get the minimum value.
    pub fn min_value(&self) -> f64 {
        self.min_value
    }

    /// Get the maximum value.
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Set a new range and re-clamp the current value.
    pub fn set_range(&mut self, min: f64, max: f64) {
        self.min_value = min;
        self.max_value = max;
        self.value = self.value.clamp(self.min_value, self.max_value);
        self.view.set_needs_display();
    }

    /// Return the normalized value in [0.0, 1.0]. Returns 0.0 if min == max.
    pub fn normalized_value(&self) -> f64 {
        let range = self.max_value - self.min_value;
        if range == 0.0 {
            0.0
        } else {
            (self.value - self.min_value) / range
        }
    }

    /// Whether the slider is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the slider.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.view.set_needs_display();
    }

    /// Set the on-change callback.
    pub fn set_on_change(&mut self, handler: impl Fn(f64) + 'static) {
        self.on_change = Some(Box::new(handler));
    }

    /// Set the track color.
    pub fn set_track_color(&mut self, color: Color) {
        self.track_color = color;
        self.view.set_needs_display();
    }

    /// Set the knob color.
    pub fn set_knob_color(&mut self, color: Color) {
        self.knob_color = color;
        self.view.set_needs_display();
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Intrinsic content size: no intrinsic width, height = 22.0.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        IntrinsicSize::height_only(22.0)
    }

    /// Draw the slider: track, filled portion, and knob.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        let frame = self.view.frame;
        let track_height = 4.0;
        let track_y = frame.y + (frame.height - track_height) / 2.0;

        // Draw full track
        let track_rect = Rect::new(frame.x, track_y, frame.width, track_height);
        renderer.fill_rect(track_rect, self.track_color);

        // Draw filled portion
        let t = self.normalized_value();
        let filled_width = frame.width * t;
        if filled_width > 0.0 {
            let filled_rect = Rect::new(frame.x, track_y, filled_width, track_height);
            renderer.fill_rect(filled_rect, self.knob_color);
        }

        // Draw knob
        let knob_size = 16.0;
        let knob_x = frame.x + (frame.width - knob_size) * t;
        let knob_y = frame.y + (frame.height - knob_size) / 2.0;
        let knob_rect = Rect::new(knob_x, knob_y, knob_size, knob_size);
        let knob_draw_color = if self.enabled {
            self.knob_color
        } else {
            Color::new(0.6, 0.6, 0.6, 1.0)
        };
        renderer.fill_rect(knob_rect, knob_draw_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn slider_default() {
        let s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        assert_eq!(s.value(), 0.0);
        assert_eq!(s.min_value(), 0.0);
        assert_eq!(s.max_value(), 100.0);
        assert!(s.is_enabled());
    }

    #[test]
    fn slider_set_value() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(50.0);
        assert_eq!(s.value(), 50.0);
    }

    #[test]
    fn slider_clamp_low() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 10.0, 90.0);
        s.set_value(5.0);
        assert_eq!(s.value(), 10.0);
    }

    #[test]
    fn slider_clamp_high() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 10.0, 90.0);
        s.set_value(100.0);
        assert_eq!(s.value(), 90.0);
    }

    #[test]
    fn slider_normalized() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(50.0);
        assert!((s.normalized_value() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn slider_normalized_zero_range() {
        let s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 50.0, 50.0);
        assert_eq!(s.normalized_value(), 0.0);
    }

    #[test]
    fn slider_set_range() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(80.0);
        s.set_range(0.0, 50.0);
        assert_eq!(s.value(), 50.0); // re-clamped
        assert_eq!(s.min_value(), 0.0);
        assert_eq!(s.max_value(), 50.0);
    }

    #[test]
    fn slider_on_change_callback() {
        let val = Rc::new(Cell::new(0.0));
        let val2 = val.clone();
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_on_change(move |v| val2.set(v));
        s.set_value(42.0);
        assert_eq!(val.get(), 42.0);
    }

    #[test]
    fn slider_disabled() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_enabled(false);
        assert!(!s.is_enabled());
        // Programmatic set_value still works when disabled
        s.set_value(75.0);
        assert_eq!(s.value(), 75.0);
    }

    #[test]
    fn slider_draw_no_panic() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(50.0);
        let mut surface = graphics_core::Surface::new(200, 22);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        s.draw(&mut renderer); // should not panic
    }

    #[test]
    fn slider_view_access() {
        let mut s = Slider::new(Rect::new(10.0, 20.0, 200.0, 22.0), 0.0, 1.0);
        assert_eq!(s.view().frame.x, 10.0);
        s.view_mut().hidden = true;
        assert!(s.view().hidden);
    }

    #[test]
    fn slider_track_knob_colors() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_track_color(Color::RED);
        s.set_knob_color(Color::GREEN);
        // Verify colors are stored (draw uses them)
        assert_eq!(s.track_color.r, Color::RED.r);
        assert_eq!(s.knob_color.g, Color::GREEN.g);
    }

    #[test]
    fn set_range_clamps_current_value() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(80.0);
        assert_eq!(s.value(), 80.0);
        s.set_range(0.0, 50.0);
        assert_eq!(s.value(), 50.0);
    }

    #[test]
    fn set_value_below_min_clamps() {
        let mut s = Slider::new(Rect::new(0.0, 0.0, 200.0, 22.0), 0.0, 100.0);
        s.set_value(-10.0);
        assert_eq!(s.value(), 0.0);
    }
}
