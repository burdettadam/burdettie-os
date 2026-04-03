//! ProgressBar control — displays determinate or indeterminate progress.

use graphics_core::{Color, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A progress bar control.
pub struct ProgressBar {
    view: View,
    progress: f64,
    is_indeterminate: bool,
    bar_color: Color,
    track_color: Color,
}

impl ProgressBar {
    /// Create a progress bar with the given frame.
    pub fn new(frame: Rect) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::TRANSPARENT;
        Self {
            view,
            progress: 0.0,
            is_indeterminate: false,
            bar_color: Color::new(0.0, 0.48, 1.0, 1.0),
            track_color: Color::new(0.9, 0.9, 0.9, 1.0),
        }
    }

    /// Get the current progress (0.0..=1.0).
    pub fn progress(&self) -> f64 {
        self.progress
    }

    /// Set the progress, clamped to [0.0, 1.0].
    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
        self.view.set_needs_display();
    }

    /// Whether the progress bar is in indeterminate mode.
    pub fn is_indeterminate(&self) -> bool {
        self.is_indeterminate
    }

    /// Set indeterminate mode.
    pub fn set_indeterminate(&mut self, flag: bool) {
        self.is_indeterminate = flag;
        self.view.set_needs_display();
    }

    /// Set the bar (filled portion) color.
    pub fn set_bar_color(&mut self, color: Color) {
        self.bar_color = color;
    }

    /// Set the track (background) color.
    pub fn set_track_color(&mut self, color: Color) {
        self.track_color = color;
    }

    /// Whether progress has reached 1.0.
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Reset progress to 0.0.
    pub fn reset(&mut self) {
        self.progress = 0.0;
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

    /// Compute the intrinsic content size (no intrinsic width, height = 8.0).
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        IntrinsicSize::height_only(8.0)
    }

    /// Draw the progress bar.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        let frame = self.view.frame;

        // Draw track
        renderer.fill_rect(frame, self.track_color);

        if self.is_indeterminate {
            // Pulsing-style bar: 30% width, centered
            let bar_width = frame.width * 0.3;
            let bar_x = frame.x + (frame.width - bar_width) / 2.0;
            let bar_rect = Rect::new(bar_x, frame.y, bar_width, frame.height);
            renderer.fill_rect(bar_rect, self.bar_color);
        } else {
            // Determinate bar proportional to progress
            let bar_width = frame.width * self.progress;
            if bar_width > 0.0 {
                let bar_rect = Rect::new(frame.x, frame.y, bar_width, frame.height);
                renderer.fill_rect(bar_rect, self.bar_color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progressbar_default() {
        let pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        assert_eq!(pb.progress(), 0.0);
        assert!(!pb.is_indeterminate());
    }

    #[test]
    fn progressbar_set_progress() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(0.5);
        assert!((pb.progress() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn progressbar_clamp() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(1.5);
        assert!((pb.progress() - 1.0).abs() < f64::EPSILON);
        pb.set_progress(-0.5);
        assert!((pb.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progressbar_is_complete() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(1.0);
        assert!(pb.is_complete());
        pb.set_progress(0.99);
        assert!(!pb.is_complete());
    }

    #[test]
    fn progressbar_reset() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(0.7);
        pb.reset();
        assert_eq!(pb.progress(), 0.0);
    }

    #[test]
    fn progressbar_indeterminate() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_indeterminate(true);
        assert!(pb.is_indeterminate());
    }

    #[test]
    fn progressbar_colors() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_bar_color(Color::new(1.0, 0.0, 0.0, 1.0));
        pb.set_track_color(Color::new(0.5, 0.5, 0.5, 1.0));
        assert!((pb.bar_color.r - 1.0).abs() < f64::EPSILON);
        assert!((pb.track_color.r - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn progressbar_draw_no_panic() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_progress(0.5);
        let mut surface = graphics_core::Surface::new(200, 8);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        pb.draw(&mut renderer);
    }

    #[test]
    fn progressbar_draw_indeterminate_no_panic() {
        let mut pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        pb.set_indeterminate(true);
        let mut surface = graphics_core::Surface::new(200, 8);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        pb.draw(&mut renderer);
    }

    #[test]
    fn progressbar_view_access() {
        let mut pb = ProgressBar::new(Rect::new(10.0, 20.0, 200.0, 8.0));
        assert_eq!(pb.view().frame.x, 10.0);
        pb.view_mut().hidden = true;
        assert!(pb.view().hidden);
    }

    #[test]
    fn progressbar_intrinsic_content_size() {
        let pb = ProgressBar::new(Rect::new(0.0, 0.0, 200.0, 8.0));
        let size = pb.intrinsic_content_size();
        assert!(size.width.is_none());
        assert!((size.height.unwrap() - 8.0).abs() < f64::EPSILON);
    }
}
