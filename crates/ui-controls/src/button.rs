//! Button control — triggers an action on click.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A clickable button control.
pub struct Button {
    view: View,
    title: String,
    font: Font,
    title_color: Color,
    enabled: bool,
    action: Option<Box<dyn Fn()>>,
}

impl Button {
    /// Create a button with the given frame and title.
    pub fn new(frame: Rect, title: impl Into<String>) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::new(0.85, 0.85, 0.85, 1.0);
        Self {
            view,
            title: title.into(),
            font: Font::default(),
            title_color: Color::BLACK,
            enabled: true,
            action: None,
        }
    }

    /// Get the current title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the button title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.view.set_needs_display();
    }

    /// Set the action closure invoked on click.
    pub fn set_action(&mut self, action: impl Fn() + 'static) {
        self.action = Some(Box::new(action));
    }

    /// Whether the button is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the button.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.view.set_needs_display();
    }

    /// Set the title text color.
    pub fn set_title_color(&mut self, color: Color) {
        self.title_color = color;
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

    /// Compute the intrinsic content size based on title length and font.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        let char_width = self.font.size * 0.6;
        let width = self.title.len() as f64 * char_width + 24.0; // +24 for button padding + border
        let height = self.font.size + 12.0; // +12 for top/bottom padding
        IntrinsicSize::new(width, height)
    }

    /// Simulate a click (triggers the action if enabled).
    pub fn perform_click(&self) {
        if self.enabled {
            if let Some(ref action) = self.action {
                action();
            }
        }
    }

    /// Draw the button.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        let bg = if self.enabled {
            self.view.background_color
        } else {
            Color::new(0.9, 0.9, 0.9, 1.0)
        };
        renderer.fill_rect(self.view.frame, bg);

        // Draw border
        let border_color = if self.enabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.8, 0.8, 0.8, 1.0)
        };
        renderer.stroke_rect(self.view.frame, border_color, 1.0);

        // Draw title centered (approximate)
        let text_color = if self.enabled {
            self.title_color
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        let origin = Point::new(
            self.view.frame.x + 8.0,
            self.view.frame.y + (self.view.frame.height - self.font.size) / 2.0,
        );
        renderer.draw_text(&self.title, origin, &self.font, text_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn button_basic() {
        let b = Button::new(Rect::new(0.0, 0.0, 80.0, 30.0), "OK");
        assert_eq!(b.title(), "OK");
        assert!(b.is_enabled());
    }

    #[test]
    fn button_action() {
        let clicked = Rc::new(Cell::new(false));
        let clicked2 = clicked.clone();
        let mut b = Button::new(Rect::new(0.0, 0.0, 80.0, 30.0), "Click");
        b.set_action(move || clicked2.set(true));
        b.perform_click();
        assert!(clicked.get());
    }

    #[test]
    fn button_disabled_no_action() {
        let clicked = Rc::new(Cell::new(false));
        let clicked2 = clicked.clone();
        let mut b = Button::new(Rect::new(0.0, 0.0, 80.0, 30.0), "Click");
        b.set_action(move || clicked2.set(true));
        b.set_enabled(false);
        b.perform_click();
        assert!(!clicked.get());
    }
}
