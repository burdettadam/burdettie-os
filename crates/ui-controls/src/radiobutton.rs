//! RadioButton control — a mutually exclusive selection within a group.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A radio button control that belongs to a numbered group.
///
/// Only one radio button in a group should be selected at a time.
/// Use [`select_in_group`] to enforce mutual exclusivity across a slice.
pub struct RadioButton {
    view: View,
    title: String,
    font: Font,
    selected: bool,
    enabled: bool,
    group_id: u32,
    on_select: Option<Box<dyn Fn()>>,
}

impl RadioButton {
    /// Create a radio button with the given frame, title, and group identifier.
    /// Defaults to not selected and enabled.
    pub fn new(frame: Rect, title: impl Into<String>, group_id: u32) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::TRANSPARENT;
        Self {
            view,
            title: title.into(),
            font: Font::default(),
            selected: false,
            enabled: true,
            group_id,
            on_select: None,
        }
    }

    /// Get the current title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.view.set_needs_display();
    }

    /// Whether this radio button is currently selected.
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Set the selected state directly (does not fire the callback).
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.view.set_needs_display();
    }

    /// Mark this radio button as selected, fire the on_select callback,
    /// and request a redraw. Does nothing if disabled.
    pub fn select(&mut self) {
        if !self.enabled {
            return;
        }
        self.selected = true;
        if let Some(ref handler) = self.on_select {
            handler();
        }
        self.view.set_needs_display();
    }

    /// Mark this radio button as not selected.
    pub fn deselect(&mut self) {
        self.selected = false;
        self.view.set_needs_display();
    }

    /// The group identifier for mutual-exclusion logic.
    pub fn group_id(&self) -> u32 {
        self.group_id
    }

    /// Whether the radio button is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the radio button.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.view.set_needs_display();
    }

    /// Set the handler called when this radio button is selected via [`select`].
    pub fn set_on_select(&mut self, handler: impl Fn() + 'static) {
        self.on_select = Some(Box::new(handler));
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
    /// Layout: `[circle_diameter] [8px gap] [title text] [4px padding]`
    /// where circle diameter equals the font size.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        let circle_diameter = self.font.size;
        let gap = 8.0;
        let char_width = self.font.size * 0.6;
        let text_width = self.title.len() as f64 * char_width;
        let padding = 4.0;
        let width = circle_diameter + gap + text_width + padding;
        let height = self.font.size + 8.0;
        IntrinsicSize::new(width, height)
    }

    /// Draw the radio button: circle outline, filled center when selected,
    /// and the title text.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        let frame = self.view.frame;
        let radius = self.font.size / 2.0;
        let center_x = frame.x + radius;
        let center_y = frame.y + frame.height / 2.0;
        let circle_rect = Rect::new(
            center_x - radius,
            center_y - radius,
            radius * 2.0,
            radius * 2.0,
        );

        // Outer circle
        let border_color = if self.enabled {
            Color::new(0.3, 0.3, 0.3, 1.0)
        } else {
            Color::new(0.7, 0.7, 0.7, 1.0)
        };
        renderer.stroke_ellipse(circle_rect, border_color, 1.0);

        // Filled inner circle when selected
        if self.selected {
            let inner_radius = radius * 0.5;
            let center = Point::new(center_x, center_y);
            let fill_color = if self.enabled {
                Color::new(0.2, 0.4, 0.8, 1.0)
            } else {
                Color::new(0.6, 0.6, 0.6, 1.0)
            };
            renderer.fill_circle(center, inner_radius, fill_color);
        }

        // Title text
        let text_color = if self.enabled {
            Color::BLACK
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        let text_x = frame.x + radius * 2.0 + 8.0;
        let text_y = frame.y + (frame.height - self.font.size) / 2.0;
        renderer.draw_text(&self.title, Point::new(text_x, text_y), &self.font, text_color);
    }
}

/// Select the radio button at `index` and deselect all others that share the
/// same `group_id`. Panics if `index` is out of bounds.
pub fn select_in_group(buttons: &mut [RadioButton], index: usize) {
    let group = buttons[index].group_id;
    for (i, btn) in buttons.iter_mut().enumerate() {
        if btn.group_id == group {
            if i == index {
                btn.select();
            } else {
                btn.deselect();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn make_rb(title: &str, group: u32) -> RadioButton {
        RadioButton::new(Rect::new(0.0, 0.0, 200.0, 24.0), title, group)
    }

    #[test]
    fn radiobutton_default() {
        let rb = make_rb("Option A", 1);
        assert!(!rb.is_selected());
        assert_eq!(rb.group_id(), 1);
        assert!(rb.is_enabled());
        assert_eq!(rb.title(), "Option A");
    }

    #[test]
    fn radiobutton_select() {
        let mut rb = make_rb("A", 1);
        rb.select();
        assert!(rb.is_selected());
    }

    #[test]
    fn radiobutton_select_fires_callback() {
        let fired = Rc::new(Cell::new(false));
        let fired2 = fired.clone();
        let mut rb = make_rb("A", 1);
        rb.set_on_select(move || fired2.set(true));
        rb.select();
        assert!(fired.get());
    }

    #[test]
    fn radiobutton_deselect() {
        let mut rb = make_rb("A", 1);
        rb.select();
        assert!(rb.is_selected());
        rb.deselect();
        assert!(!rb.is_selected());
    }

    #[test]
    fn radiobutton_disabled() {
        let fired = Rc::new(Cell::new(false));
        let fired2 = fired.clone();
        let mut rb = make_rb("A", 1);
        rb.set_on_select(move || fired2.set(true));
        rb.set_enabled(false);
        rb.select();
        assert!(!rb.is_selected(), "select() on disabled should be no-op");
        assert!(!fired.get(), "callback should not fire when disabled");
    }

    #[test]
    fn radiobutton_select_in_group() {
        let mut buttons = vec![make_rb("A", 1), make_rb("B", 1), make_rb("C", 1)];
        select_in_group(&mut buttons, 1);
        assert!(!buttons[0].is_selected());
        assert!(buttons[1].is_selected());
        assert!(!buttons[2].is_selected());

        // Select another
        select_in_group(&mut buttons, 2);
        assert!(!buttons[0].is_selected());
        assert!(!buttons[1].is_selected());
        assert!(buttons[2].is_selected());
    }

    #[test]
    fn radiobutton_set_title() {
        let mut rb = make_rb("Old", 1);
        rb.set_title("New");
        assert_eq!(rb.title(), "New");
    }

    #[test]
    fn radiobutton_intrinsic_content_size() {
        let rb = make_rb("Test", 1);
        let size = rb.intrinsic_content_size();
        // circle_diameter(14) + gap(8) + 4 chars * 8.4 + padding(4) = 59.6
        assert!(size.width.unwrap() > 0.0);
        assert!(size.height.unwrap() > 0.0);
        // Height should be font.size + 8 = 22
        assert!((size.height.unwrap() - 22.0).abs() < 0.01);
    }

    #[test]
    fn radiobutton_draw_no_panic() {
        let mut rb = make_rb("Draw me", 1);
        rb.select();
        let mut surface = graphics_core::Surface::new(200, 50);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        rb.draw(&mut renderer);
        // Deselect and draw again
        rb.deselect();
        rb.draw(&mut renderer);
        // Disabled draw
        rb.set_enabled(false);
        rb.select(); // no-op since disabled
        rb.set_selected(true); // force selected while disabled for draw path
        rb.draw(&mut renderer);
    }

    #[test]
    fn radiobutton_view_access() {
        let mut rb = make_rb("V", 1);
        assert_eq!(rb.view().frame.width, 200.0);
        rb.view_mut().hidden = true;
        assert!(rb.view().hidden);
    }
}
