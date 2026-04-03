//! Checkbox control — a toggleable on/off control with a label.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A checkbox control that can be toggled on or off.
pub struct Checkbox {
    view: View,
    title: String,
    font: Font,
    checked: bool,
    enabled: bool,
    on_change: Option<Box<dyn Fn(bool)>>,
}

impl Checkbox {
    /// Create a checkbox with the given frame and title. Defaults to unchecked and enabled.
    pub fn new(frame: Rect, title: impl Into<String>) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::new(0.95, 0.95, 0.95, 1.0);
        Self {
            view,
            title: title.into(),
            font: Font::default(),
            checked: false,
            enabled: true,
            on_change: None,
        }
    }

    /// Get the current title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the checkbox title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.view.set_needs_display();
    }

    /// Whether the checkbox is checked.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Set the checked state directly.
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        self.view.set_needs_display();
    }

    /// Toggle the checked state. If enabled, flips the value, fires the on_change
    /// callback, and marks the view for redisplay.
    pub fn toggle(&mut self) {
        if !self.enabled {
            return;
        }
        self.checked = !self.checked;
        if let Some(ref cb) = self.on_change {
            cb(self.checked);
        }
        self.view.set_needs_display();
    }

    /// Whether the checkbox is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the checkbox.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.view.set_needs_display();
    }

    /// Set the callback invoked when the checked state changes via `toggle`.
    pub fn set_on_change(&mut self, handler: impl Fn(bool) + 'static) {
        self.on_change = Some(Box::new(handler));
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Compute the intrinsic content size.
    ///
    /// Layout: `[box (font.size × font.size)] [8px gap] [title text] [4px padding each side]`
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        let box_size = self.font.size;
        let gap = 8.0;
        let char_width = self.font.size * 0.6;
        let text_width = self.title.len() as f64 * char_width;
        let padding = 8.0; // 4px each side
        let width = box_size + gap + text_width + padding;
        let height = self.font.size + 4.0;
        IntrinsicSize::new(width, height)
    }

    /// Draw the checkbox: a box (filled when checked, outlined when unchecked),
    /// an inner check mark rect when checked, and the title text.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        renderer.fill_rect(self.view.frame, self.view.background_color);

        let box_size = self.font.size;
        let box_x = self.view.frame.x + 4.0;
        let box_y = self.view.frame.y + (self.view.frame.height - box_size) / 2.0;
        let box_rect = Rect::new(box_x, box_y, box_size, box_size);

        let border_color = if self.enabled {
            Color::new(0.4, 0.4, 0.4, 1.0)
        } else {
            Color::new(0.7, 0.7, 0.7, 1.0)
        };

        if self.checked {
            // Filled box for checked state
            let fill = if self.enabled {
                Color::new(0.2, 0.5, 1.0, 1.0)
            } else {
                Color::new(0.6, 0.7, 0.85, 1.0)
            };
            renderer.fill_rect(box_rect, fill);

            // Inner check mark — small centered rect
            let inset = box_size * 0.25;
            let mark_rect = Rect::new(
                box_x + inset,
                box_y + inset,
                box_size - inset * 2.0,
                box_size - inset * 2.0,
            );
            renderer.fill_rect(mark_rect, Color::WHITE);
        } else {
            // Outlined box for unchecked state
            renderer.stroke_rect(box_rect, border_color, 1.0);
        }

        // Draw title text
        let text_color = if self.enabled {
            Color::BLACK
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        let text_x = box_x + box_size + 8.0;
        let text_y = self.view.frame.y + (self.view.frame.height - self.font.size) / 2.0;
        let origin = Point::new(text_x, text_y);
        renderer.draw_text(&self.title, origin, &self.font, text_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn checkbox_default() {
        let cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "Accept");
        assert!(!cb.is_checked());
        assert!(cb.is_enabled());
        assert_eq!(cb.title(), "Accept");
    }

    #[test]
    fn checkbox_toggle() {
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "Toggle");
        assert!(!cb.is_checked());
        cb.toggle();
        assert!(cb.is_checked());
        cb.toggle();
        assert!(!cb.is_checked());
    }

    #[test]
    fn checkbox_toggle_fires_callback() {
        let value = Rc::new(Cell::new(false));
        let value2 = value.clone();
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "CB");
        cb.set_on_change(move |checked| value2.set(checked));
        cb.toggle();
        assert!(value.get());
        cb.toggle();
        assert!(!value.get());
    }

    #[test]
    fn checkbox_disabled_toggle() {
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "CB");
        cb.set_enabled(false);
        cb.toggle();
        assert!(!cb.is_checked());
    }

    #[test]
    fn checkbox_set_checked() {
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "CB");
        cb.set_checked(true);
        assert!(cb.is_checked());
        cb.set_checked(false);
        assert!(!cb.is_checked());
    }

    #[test]
    fn checkbox_set_title() {
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "Old");
        cb.set_title("New");
        assert_eq!(cb.title(), "New");
        assert!(cb.view().needs_display());
    }

    #[test]
    fn checkbox_intrinsic_content_size() {
        let cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "Check");
        let size = cb.intrinsic_content_size();
        // box_size(14) + gap(8) + text(5 * 14*0.6 = 42) + padding(8) = 72
        assert!((size.width.unwrap() - 72.0).abs() < 0.01);
        // font.size(14) + 4 = 18
        assert!((size.height.unwrap() - 18.0).abs() < 0.01);
    }

    #[test]
    fn checkbox_draw_no_panic() {
        let mut surface = graphics_core::Surface::new(200, 50);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "Draw");
        cb.draw(&mut renderer);
        cb.set_checked(true);
        cb.draw(&mut renderer);
    }

    #[test]
    fn checkbox_view_access() {
        let mut cb = Checkbox::new(Rect::new(10.0, 20.0, 150.0, 24.0), "V");
        assert!((cb.view().frame.x - 10.0).abs() < f64::EPSILON);
        cb.view_mut().hidden = true;
        assert!(cb.view().hidden);
    }

    #[test]
    fn toggle_fires_callback() {
        let fired = Rc::new(Cell::new(false));
        let fired2 = fired.clone();
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "T");
        cb.set_on_change(move |_| fired2.set(true));
        cb.toggle();
        assert!(fired.get());
    }

    #[test]
    fn set_checked_does_not_fire_callback() {
        let fired = Rc::new(Cell::new(false));
        let fired2 = fired.clone();
        let mut cb = Checkbox::new(Rect::new(0.0, 0.0, 150.0, 24.0), "T");
        cb.set_on_change(move |_| fired2.set(true));
        cb.set_checked(true);
        assert!(cb.is_checked());
        // set_checked does NOT invoke on_change — only toggle does
        assert!(!fired.get());
    }
}
