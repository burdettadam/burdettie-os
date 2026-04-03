//! TabView control — tabbed container for switching between panels.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::View;

/// A tabbed container that switches between panels.
pub struct TabView {
    view: View,
    tabs: Vec<String>,
    selected_tab: usize,
    font: Font,
    tab_height: f64,
    tab_color: Color,
    selected_tab_color: Color,
    text_color: Color,
    on_tab_change: Option<Box<dyn Fn(usize)>>,
}

impl TabView {
    /// Create a tab view with the given frame. Starts with no tabs.
    pub fn new(frame: Rect) -> Self {
        Self {
            view: View::new(frame),
            tabs: Vec::new(),
            selected_tab: 0,
            font: Font::default(),
            tab_height: 30.0,
            tab_color: Color::new(0.85, 0.85, 0.85, 1.0),
            selected_tab_color: Color::WHITE,
            text_color: Color::BLACK,
            on_tab_change: None,
        }
    }

    /// Add a tab with the given title.
    pub fn add_tab(&mut self, title: impl Into<String>) {
        self.tabs.push(title.into());
    }

    /// The tab titles.
    pub fn tabs(&self) -> &[String] {
        &self.tabs
    }

    /// Number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Remove the tab at `index`. Adjusts selection if needed.
    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.tabs.is_empty() {
                self.selected_tab = 0;
            } else if self.selected_tab >= self.tabs.len() {
                self.selected_tab = self.tabs.len() - 1;
            }
        }
    }

    /// The currently selected tab index.
    pub fn selected_tab(&self) -> usize {
        self.selected_tab
    }

    /// The title of the currently selected tab, if any.
    pub fn selected_tab_title(&self) -> Option<&str> {
        self.tabs.get(self.selected_tab).map(|s| s.as_str())
    }

    /// Select a tab by index. Clamped to valid range. Fires the change callback.
    pub fn select_tab(&mut self, index: usize) {
        if self.tabs.is_empty() {
            return;
        }
        let clamped = index.min(self.tabs.len() - 1);
        self.selected_tab = clamped;
        if let Some(ref cb) = self.on_tab_change {
            cb(clamped);
        }
        self.view.set_needs_display();
    }

    /// Set a callback invoked when the selected tab changes.
    pub fn set_on_tab_change(&mut self, handler: impl Fn(usize) + 'static) {
        self.on_tab_change = Some(Box::new(handler));
    }

    /// The rect of the tab bar (top portion of the frame).
    pub fn tab_bar_rect(&self) -> Rect {
        let f = self.view.frame;
        Rect::new(f.x, f.y, f.width, self.tab_height)
    }

    /// The rect of the content area below the tab bar.
    pub fn content_rect(&self) -> Rect {
        let f = self.view.frame;
        Rect::new(
            f.x,
            f.y + self.tab_height,
            f.width,
            (f.height - self.tab_height).max(0.0),
        )
    }

    /// The rect for an individual tab in the tab bar (equal-width division).
    pub fn tab_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.tabs.len() || self.tabs.is_empty() {
            return None;
        }
        let bar = self.tab_bar_rect();
        let tab_width = bar.width / self.tabs.len() as f64;
        Some(Rect::new(
            bar.x + tab_width * index as f64,
            bar.y,
            tab_width,
            bar.height,
        ))
    }

    /// Hit-test: which tab index is at `point`, if any.
    pub fn hit_test_tab(&self, point: Point) -> Option<usize> {
        if !self.tab_bar_rect().contains(point) {
            return None;
        }
        for i in 0..self.tabs.len() {
            if let Some(r) = self.tab_rect(i) {
                if r.contains(point) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable view access.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Draw the tab view: tab bar background, individual tabs, titles, and content border.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Tab bar background
        let bar = self.tab_bar_rect();
        renderer.fill_rect(bar, self.tab_color);

        // Individual tabs
        for i in 0..self.tabs.len() {
            if let Some(r) = self.tab_rect(i) {
                let color = if i == self.selected_tab {
                    self.selected_tab_color
                } else {
                    self.tab_color
                };
                renderer.fill_rect(r, color);
                renderer.stroke_rect(r, Color::new(0.7, 0.7, 0.7, 1.0), 1.0);

                // Center the title text in the tab rect
                let text_x = r.x + 4.0;
                let text_y = r.y + (r.height - self.font.size) / 2.0;
                renderer.draw_text(
                    &self.tabs[i],
                    Point::new(text_x, text_y),
                    &self.font,
                    self.text_color,
                );
            }
        }

        // Content area border
        let content = self.content_rect();
        renderer.stroke_rect(content, Color::new(0.7, 0.7, 0.7, 1.0), 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn tabview_default() {
        let tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        assert_eq!(tv.tab_count(), 0);
        assert_eq!(tv.selected_tab(), 0);
    }

    #[test]
    fn tabview_add_tabs() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("One");
        tv.add_tab("Two");
        tv.add_tab("Three");
        assert_eq!(tv.tab_count(), 3);
        assert_eq!(tv.tabs(), &["One", "Two", "Three"]);
    }

    #[test]
    fn tabview_select_tab() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");
        tv.select_tab(2);
        assert_eq!(tv.selected_tab(), 2);
        assert_eq!(tv.selected_tab_title(), Some("C"));
    }

    #[test]
    fn tabview_select_out_of_range() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("X");
        tv.add_tab("Y");
        tv.select_tab(99);
        assert_eq!(tv.selected_tab(), 1); // clamped to last
    }

    #[test]
    fn tabview_remove_tab() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");
        tv.select_tab(2);
        // Remove middle tab
        tv.remove_tab(1);
        assert_eq!(tv.tab_count(), 2);
        assert_eq!(tv.tabs(), &["A", "C"]);
        // selected_tab was 2, now should be 1 (clamped)
        assert_eq!(tv.selected_tab(), 1);
    }

    #[test]
    fn tabview_remove_last_tab_adjusts_selection() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("Only");
        tv.select_tab(0);
        tv.remove_tab(0);
        assert_eq!(tv.tab_count(), 0);
        assert_eq!(tv.selected_tab(), 0);
        assert_eq!(tv.selected_tab_title(), None);
    }

    #[test]
    fn tabview_on_tab_change() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");
        let changed = Rc::new(Cell::new(999usize));
        let changed_clone = changed.clone();
        tv.set_on_tab_change(move |idx| {
            changed_clone.set(idx);
        });
        tv.select_tab(1);
        assert_eq!(changed.get(), 1);
    }

    #[test]
    fn tabview_tab_bar_rect() {
        let tv = TabView::new(Rect::new(10.0, 20.0, 400.0, 300.0));
        let bar = tv.tab_bar_rect();
        assert_eq!(bar.x, 10.0);
        assert_eq!(bar.y, 20.0);
        assert_eq!(bar.width, 400.0);
        assert_eq!(bar.height, 30.0);
    }

    #[test]
    fn tabview_content_rect() {
        let tv = TabView::new(Rect::new(10.0, 20.0, 400.0, 300.0));
        let content = tv.content_rect();
        assert_eq!(content.x, 10.0);
        assert_eq!(content.y, 50.0); // 20 + 30 tab_height
        assert_eq!(content.width, 400.0);
        assert_eq!(content.height, 270.0); // 300 - 30
    }

    #[test]
    fn tabview_tab_rect() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 300.0, 200.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");

        let r0 = tv.tab_rect(0).unwrap();
        assert!((r0.x - 0.0).abs() < 0.01);
        assert!((r0.width - 100.0).abs() < 0.01);
        assert_eq!(r0.height, 30.0);

        let r1 = tv.tab_rect(1).unwrap();
        assert!((r1.x - 100.0).abs() < 0.01);

        let r2 = tv.tab_rect(2).unwrap();
        assert!((r2.x - 200.0).abs() < 0.01);

        assert!(tv.tab_rect(3).is_none());
    }

    #[test]
    fn tabview_hit_test_tab() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 300.0, 200.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");

        // Point in tab 1 (x=150, y=15 is inside tab bar)
        assert_eq!(tv.hit_test_tab(Point::new(150.0, 15.0)), Some(1));
    }

    #[test]
    fn tabview_hit_test_tab_outside() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 300.0, 200.0));
        tv.add_tab("A");
        tv.add_tab("B");

        // Point below tab bar
        assert_eq!(tv.hit_test_tab(Point::new(50.0, 100.0)), None);
    }

    #[test]
    fn tabview_draw_no_panic() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("Tab 1");
        tv.add_tab("Tab 2");
        tv.select_tab(0);
        let mut surface = graphics_core::Surface::new(400, 300);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        tv.draw(&mut renderer);
    }

    #[test]
    fn tabview_view_access() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        assert_eq!(tv.view().frame.width, 400.0);
        tv.view_mut().hidden = true;
        assert!(tv.view().hidden);
    }

    #[test]
    fn select_tab_empty_no_panic() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        // No tabs added — select_tab should not panic
        tv.select_tab(0);
        assert_eq!(tv.selected_tab(), 0);
    }

    #[test]
    fn remove_tab_out_of_range_no_panic() {
        let mut tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        tv.add_tab("A");
        tv.add_tab("B");
        tv.add_tab("C");
        tv.remove_tab(99);
        assert_eq!(tv.tab_count(), 3);
    }

    #[test]
    fn selected_tab_title_returns_none_when_empty() {
        let tv = TabView::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        assert!(tv.selected_tab_title().is_none());
    }
}
