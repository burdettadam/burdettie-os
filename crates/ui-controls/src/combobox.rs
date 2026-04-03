//! ComboBox control — dropdown selector with a list of items.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// A dropdown selector that lets the user pick from a list of items.
pub struct ComboBox {
    view: View,
    items: Vec<String>,
    selected_index: Option<usize>,
    font: Font,
    text_color: Color,
    placeholder: String,
    enabled: bool,
    on_selection_change: Option<Box<dyn Fn(Option<usize>)>>,
}

impl ComboBox {
    /// Create a combo box with the given frame. Starts with no items and no selection.
    pub fn new(frame: Rect) -> Self {
        let mut view = View::new(frame);
        view.background_color = Color::WHITE;
        Self {
            view,
            items: Vec::new(),
            selected_index: None,
            font: Font::default(),
            text_color: Color::BLACK,
            placeholder: String::new(),
            enabled: true,
            on_selection_change: None,
        }
    }

    /// Add an item to the end of the list.
    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    /// The current list of items.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Number of items in the list.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Remove the item at `index`. Adjusts `selected_index` accordingly.
    pub fn remove_item(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        if let Some(sel) = self.selected_index {
            if sel == index {
                self.selected_index = None;
            } else if sel > index {
                self.selected_index = Some(sel - 1);
            }
        }
        self.view.set_needs_display();
    }

    /// Remove all items and reset selection.
    pub fn remove_all_items(&mut self) {
        self.items.clear();
        self.selected_index = None;
        self.view.set_needs_display();
    }

    /// The currently selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// The currently selected item text, if any.
    pub fn selected_item(&self) -> Option<&str> {
        self.selected_index
            .and_then(|i| self.items.get(i))
            .map(|s| s.as_str())
    }

    /// Select the item at `index`. Does nothing if out of range.
    pub fn select_item(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.selected_index = Some(index);
        if let Some(ref cb) = self.on_selection_change {
            cb(self.selected_index);
        }
        self.view.set_needs_display();
    }

    /// Clear the selection.
    pub fn deselect(&mut self) {
        self.selected_index = None;
        self.view.set_needs_display();
    }

    /// Set the placeholder text shown when no item is selected.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
        self.view.set_needs_display();
    }

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Whether the combo box is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the combo box.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.view.set_needs_display();
    }

    /// Set a callback invoked when the selection changes.
    pub fn set_on_selection_change(&mut self, handler: impl Fn(Option<usize>) + 'static) {
        self.on_selection_change = Some(Box::new(handler));
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Compute the intrinsic content size. Height is font-based; no intrinsic width.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        let height = self.font.size + 8.0;
        IntrinsicSize::height_only(height)
    }

    /// Draw the combo box.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        let frame = self.view.frame;

        // Background
        let bg = if self.enabled {
            self.view.background_color
        } else {
            Color::new(0.93, 0.93, 0.93, 1.0)
        };
        renderer.fill_rect(frame, bg);

        // Border
        let border_color = if self.enabled {
            Color::new(0.7, 0.7, 0.7, 1.0)
        } else {
            Color::new(0.85, 0.85, 0.85, 1.0)
        };
        renderer.stroke_rect(frame, border_color, 1.0);

        // Text: selected item or placeholder
        let origin = Point::new(frame.x + 4.0, frame.y + 2.0);
        let text_color = if self.enabled {
            self.text_color
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };

        if let Some(text) = self.selected_item() {
            renderer.draw_text(text, origin, &self.font, text_color);
        } else if !self.placeholder.is_empty() {
            let placeholder_color = Color::new(0.6, 0.6, 0.6, 1.0);
            renderer.draw_text(&self.placeholder, origin, &self.font, placeholder_color);
        }

        // Dropdown arrow (small triangle via two lines)
        let arrow_x = frame.x + frame.width - 14.0;
        let arrow_y = frame.y + frame.height / 2.0;
        let arrow_color = if self.enabled {
            Color::new(0.3, 0.3, 0.3, 1.0)
        } else {
            Color::new(0.6, 0.6, 0.6, 1.0)
        };
        renderer.draw_line(
            Point::new(arrow_x, arrow_y - 3.0),
            Point::new(arrow_x + 5.0, arrow_y + 3.0),
            arrow_color,
            1.0,
        );
        renderer.draw_line(
            Point::new(arrow_x + 5.0, arrow_y + 3.0),
            Point::new(arrow_x + 10.0, arrow_y - 3.0),
            arrow_color,
            1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn combobox_default() {
        let cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        assert_eq!(cb.item_count(), 0);
        assert!(cb.selected_index().is_none());
        assert!(cb.selected_item().is_none());
        assert!(cb.is_enabled());
    }

    #[test]
    fn combobox_add_items() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("Alpha");
        cb.add_item("Beta");
        cb.add_item("Gamma");
        assert_eq!(cb.item_count(), 3);
        assert_eq!(cb.items(), &["Alpha", "Beta", "Gamma"]);
    }

    #[test]
    fn combobox_select() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.add_item("C");
        cb.select_item(1);
        assert_eq!(cb.selected_index(), Some(1));
        assert_eq!(cb.selected_item(), Some("B"));
    }

    #[test]
    fn combobox_select_out_of_range() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.add_item("C");
        cb.select_item(1);
        cb.select_item(5); // out of range — should be ignored
        assert_eq!(cb.selected_index(), Some(1));
    }

    #[test]
    fn combobox_deselect() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("X");
        cb.select_item(0);
        assert!(cb.selected_index().is_some());
        cb.deselect();
        assert!(cb.selected_index().is_none());
    }

    #[test]
    fn combobox_remove_item() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.add_item("C");
        cb.select_item(2); // "C"
        cb.remove_item(0); // remove "A" → "C" is now at index 1
        assert_eq!(cb.item_count(), 2);
        assert_eq!(cb.selected_index(), Some(1));
        assert_eq!(cb.selected_item(), Some("C"));

        // Remove selected item itself
        cb.remove_item(1);
        assert!(cb.selected_index().is_none());
    }

    #[test]
    fn combobox_remove_all() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.select_item(0);
        cb.remove_all_items();
        assert_eq!(cb.item_count(), 0);
        assert!(cb.selected_index().is_none());
    }

    #[test]
    fn combobox_on_selection_change() {
        let called = Rc::new(Cell::new(None));
        let called2 = called.clone();
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.set_on_selection_change(move |idx| called2.set(idx));
        cb.select_item(1);
        assert_eq!(called.get(), Some(1));
    }

    #[test]
    fn combobox_placeholder() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        assert_eq!(cb.placeholder(), "");
        cb.set_placeholder("Choose...");
        assert_eq!(cb.placeholder(), "Choose...");
    }

    #[test]
    fn combobox_disabled() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.set_enabled(false);
        assert!(!cb.is_enabled());
        // Selection changes still work programmatically
        cb.select_item(1);
        assert_eq!(cb.selected_index(), Some(1));
    }

    #[test]
    fn combobox_draw_no_panic() {
        let mut surface = graphics_core::Surface::new(200, 30);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.set_placeholder("Pick one");
        cb.draw(&mut renderer);
    }

    #[test]
    fn combobox_draw_with_selection_no_panic() {
        let mut surface = graphics_core::Surface::new(200, 30);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("First");
        cb.add_item("Second");
        cb.select_item(0);
        cb.draw(&mut renderer);
    }

    #[test]
    fn combobox_view_access() {
        let mut cb = ComboBox::new(Rect::new(10.0, 20.0, 200.0, 24.0));
        assert_eq!(cb.view().frame.x, 10.0);
        cb.view_mut().hidden = true;
        assert!(cb.view().hidden);
    }

    #[test]
    fn combobox_intrinsic_content_size() {
        let cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        let size = cb.intrinsic_content_size();
        assert!(size.width.is_none());
        assert!(size.height.is_some());
        let h = size.height.unwrap();
        assert!(h > 0.0);
        assert!((h - (cb.font.size + 8.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn remove_item_out_of_range_no_panic() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.add_item("C");
        cb.remove_item(10);
        assert_eq!(cb.item_count(), 3);
    }

    #[test]
    fn select_item_out_of_range() {
        let mut cb = ComboBox::new(Rect::new(0.0, 0.0, 200.0, 24.0));
        cb.add_item("A");
        cb.add_item("B");
        cb.add_item("C");
        // No prior selection
        assert!(cb.selected_index().is_none());
        cb.select_item(99);
        // Out-of-range select is ignored — still None
        assert!(cb.selected_index().is_none());
    }
}
