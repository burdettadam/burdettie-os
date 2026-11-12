//! View tree, layout, and hit testing.
//!
//! Provides a `View` struct with a composable tree structure. Views have
//! frames (position in superview coordinates), bounds (local coordinate
//! space), background color, hidden flag, and a tag for identification.
//!
//! Hit testing walks the view tree in reverse (front-to-back) order to
//! find the deepest view containing a point.
//!
//! The [`layout`] module provides a constraint-based layout engine (Auto Layout
//! Lite) using the Cassowary algorithm for solving linear constraints.

pub mod layout;

use app_kit_lite::{Event, Responder};
use graphics_core::{Color, Point, Rect, Renderer};
use layout::{LayoutEngine, ViewId};

/// A view in the view tree.
pub struct View {
    /// Position and size in superview coordinates.
    pub frame: Rect,
    /// Local coordinate space (origin usually 0,0).
    pub bounds: Rect,
    /// Background color (drawn before subviews).
    pub background_color: Color,
    /// Whether the view is hidden.
    pub hidden: bool,
    /// Application-defined tag for identification.
    pub tag: usize,
    /// Optional layout identifier for constraint-based layout.
    pub layout_id: Option<ViewId>,
    /// Child views (drawn back-to-front: index 0 is backmost).
    subviews: Vec<View>,
    /// Whether the view needs a redraw.
    needs_display: bool,
}

impl View {
    /// Create a new view with the given frame.
    pub fn new(frame: Rect) -> Self {
        Self {
            frame,
            bounds: Rect::new(0.0, 0.0, frame.width, frame.height),
            background_color: Color::TRANSPARENT,
            hidden: false,
            tag: 0,
            layout_id: None,
            subviews: Vec::new(),
            needs_display: true,
        }
    }

    /// The view's frame (position and size in superview coordinates).
    pub fn frame(&self) -> Rect {
        self.frame
    }

    /// Set the view's frame.
    pub fn set_frame(&mut self, frame: Rect) {
        self.frame = frame;
        self.bounds = Rect::new(0.0, 0.0, frame.width, frame.height);
        self.set_needs_display();
    }

    /// Add a child view.
    pub fn add_subview(&mut self, view: View) {
        self.subviews.push(view);
        self.set_needs_display();
    }

    /// Remove the last child view matching the tag. Returns it if found.
    pub fn remove_subview_by_tag(&mut self, tag: usize) -> Option<View> {
        if let Some(pos) = self.subviews.iter().position(|v| v.tag == tag) {
            self.set_needs_display();
            Some(self.subviews.remove(pos))
        } else {
            None
        }
    }

    /// Access child views.
    pub fn subviews(&self) -> &[View] {
        &self.subviews
    }

    /// Mutable access to child views.
    pub fn subviews_mut(&mut self) -> &mut Vec<View> {
        &mut self.subviews
    }

    /// Mark the view as needing a redraw.
    pub fn set_needs_display(&mut self) {
        self.needs_display = true;
    }

    /// Whether the view needs a redraw.
    pub fn needs_display(&self) -> bool {
        self.needs_display
    }

    /// Draw the view and its subviews into a renderer.
    ///
    /// The renderer should be pre-translated to the view's frame origin.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.hidden {
            return;
        }

        // Draw background
        if self.background_color.a > 0.0 {
            renderer.fill_rect(self.bounds, self.background_color);
        }

        // Draw content (subclasses override draw_content)
        self.draw_content(renderer);

        // Draw subviews
        for subview in &mut self.subviews {
            // Note: a real implementation would translate the renderer
            // coordinate space by subview.frame.origin. For now, subview
            // frames are in the parent's coordinate space.
            subview.draw(renderer);
        }

        self.needs_display = false;
    }

    /// Override point for custom drawing. Base implementation does nothing.
    pub fn draw_content(&self, _renderer: &mut dyn Renderer) {
        // Subclasses (Label, Button, etc.) override this
    }

    /// Hit test: find the deepest visible view containing the point.
    ///
    /// The point is in the coordinate space of this view's superview
    /// (i.e., the same space as `self.frame`).
    pub fn hit_test(&self, point: Point) -> Option<&View> {
        if self.hidden {
            return None;
        }

        // Check if point is within our frame
        if !self.frame.contains(point) {
            return None;
        }

        // Convert to local coordinates
        let local = Point::new(point.x - self.frame.x, point.y - self.frame.y);

        // Check subviews in reverse order (front-to-back)
        for subview in self.subviews.iter().rev() {
            if let Some(hit) = subview.hit_test(local) {
                return Some(hit);
            }
        }

        // No subview hit — we are the target
        Some(self)
    }

    /// Mutable hit test.
    pub fn hit_test_mut(&mut self, point: Point) -> Option<&mut View> {
        if self.hidden {
            return None;
        }

        if !self.frame.contains(point) {
            return None;
        }

        let local = Point::new(point.x - self.frame.x, point.y - self.frame.y);

        // Check subviews in reverse order — find index first to avoid borrow conflicts
        let hit_index = self
            .subviews
            .iter()
            .enumerate()
            .rev()
            .find(|(_, sv)| sv.hit_test(local).is_some())
            .map(|(i, _)| i);

        if let Some(idx) = hit_index {
            return self.subviews[idx].hit_test_mut(local);
        }

        Some(self)
    }

    /// Layout subviews. Override point for custom layout logic.
    pub fn layout_subviews(&mut self) {
        // Base implementation does nothing
    }

    /// Apply constraint-based layout to this view and all subviews that
    /// have a `layout_id`. The engine must already contain constraints
    /// referencing these IDs.
    pub fn apply_layout(&mut self, engine: &LayoutEngine) {
        if let Ok(frames) = engine.solve() {
            self.apply_layout_frames(&frames);
        }
    }

    /// Recursively apply solved layout frames to this view and subviews.
    fn apply_layout_frames(&mut self, frames: &std::collections::HashMap<ViewId, Rect>) {
        if let Some(id) = self.layout_id {
            if let Some(&rect) = frames.get(&id) {
                self.set_frame(rect);
            }
        }
        for subview in &mut self.subviews {
            subview.apply_layout_frames(frames);
        }
    }

    /// Find a subview by tag (recursive).
    pub fn view_with_tag(&self, tag: usize) -> Option<&View> {
        if self.tag == tag {
            return Some(self);
        }
        for subview in &self.subviews {
            if let Some(found) = subview.view_with_tag(tag) {
                return Some(found);
            }
        }
        None
    }
}

impl Responder for View {
    fn next_responder(&self) -> Option<&dyn Responder> {
        // In a full implementation, this would return the superview.
        // For now, views are the end of the chain.
        None
    }

    fn key_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.key_down(event);
        }
    }

    fn key_up(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.key_up(event);
        }
    }

    fn mouse_down(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.mouse_down(event);
        }
    }

    fn mouse_up(&self, event: &Event) {
        if let Some(next) = self.next_responder() {
            next.mouse_up(event);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_basic() {
        let v = View::new(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert_eq!(v.frame.x, 10.0);
        assert_eq!(v.bounds.width, 100.0);
        assert!(!v.hidden);
        assert!(v.needs_display());
    }

    #[test]
    fn view_add_subview() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = View::new(Rect::new(10.0, 10.0, 50.0, 50.0));
        parent.add_subview(child);
        assert_eq!(parent.subviews().len(), 1);
    }

    #[test]
    fn view_hit_test_self() {
        let v = View::new(Rect::new(10.0, 10.0, 100.0, 100.0));
        // Point inside
        let hit = v.hit_test(Point::new(50.0, 50.0));
        assert!(hit.is_some());
        // Point outside
        let hit = v.hit_test(Point::new(5.0, 5.0));
        assert!(hit.is_none());
    }

    #[test]
    fn view_hit_test_subview() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let mut child = View::new(Rect::new(50.0, 50.0, 40.0, 40.0));
        child.tag = 42;
        parent.add_subview(child);

        // Point inside child
        let hit = parent.hit_test(Point::new(60.0, 60.0));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().tag, 42);

        // Point outside child but inside parent
        let hit = parent.hit_test(Point::new(10.0, 10.0));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().tag, 0); // parent's tag
    }

    #[test]
    fn view_hit_test_frontmost_wins() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let mut back = View::new(Rect::new(10.0, 10.0, 80.0, 80.0));
        back.tag = 1;
        let mut front = View::new(Rect::new(10.0, 10.0, 80.0, 80.0));
        front.tag = 2;

        parent.add_subview(back);
        parent.add_subview(front);

        // Point in overlapping area — front (last added) should win
        let hit = parent.hit_test(Point::new(50.0, 50.0));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().tag, 2);
    }

    #[test]
    fn view_hit_test_hidden() {
        let mut v = View::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        v.hidden = true;
        let hit = v.hit_test(Point::new(50.0, 50.0));
        assert!(hit.is_none());
    }

    #[test]
    fn view_find_by_tag() {
        let mut root = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        root.tag = 1;

        let mut child = View::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        child.tag = 2;

        let mut grandchild = View::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        grandchild.tag = 3;

        child.add_subview(grandchild);
        root.add_subview(child);

        assert!(root.view_with_tag(1).is_some());
        assert!(root.view_with_tag(2).is_some());
        assert!(root.view_with_tag(3).is_some());
        assert!(root.view_with_tag(99).is_none());
    }

    #[test]
    fn view_remove_subview_by_tag() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let mut child = View::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        child.tag = 42;
        parent.add_subview(child);
        assert_eq!(parent.subviews().len(), 1);

        let removed = parent.remove_subview_by_tag(42);
        assert!(removed.is_some());
        assert_eq!(parent.subviews().len(), 0);
    }

    #[test]
    fn view_draw_clears_needs_display() {
        let mut v = View::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        v.background_color = Color::WHITE;
        assert!(v.needs_display());

        let mut surface = graphics_core::Surface::new(100, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        v.draw(&mut renderer);

        assert!(!v.needs_display());
    }

    #[test]
    fn view_draw_calls_subviews() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = View::new(Rect::new(10.0, 10.0, 50.0, 50.0));
        parent.add_subview(child);

        // Both should need display initially
        assert!(parent.needs_display());
        assert!(parent.subviews()[0].needs_display());

        let mut surface = graphics_core::Surface::new(200, 200);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        parent.draw(&mut renderer);

        // After draw, both parent and child should have needs_display cleared
        assert!(!parent.needs_display());
        assert!(!parent.subviews()[0].needs_display());
    }

    #[test]
    fn view_hit_test_returns_none_for_outside() {
        let v = View::new(Rect::new(50.0, 50.0, 100.0, 100.0));
        // All four sides outside
        assert!(v.hit_test(Point::new(0.0, 0.0)).is_none());
        assert!(v.hit_test(Point::new(200.0, 75.0)).is_none());
        assert!(v.hit_test(Point::new(75.0, 200.0)).is_none());
        assert!(v.hit_test(Point::new(49.0, 75.0)).is_none());
    }

    #[test]
    fn view_hit_test_returns_self_for_inside() {
        let mut v = View::new(Rect::new(10.0, 10.0, 80.0, 80.0));
        v.tag = 7;
        let hit = v.hit_test(Point::new(50.0, 50.0));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().tag, 7);
    }

    #[test]
    fn view_hidden_skips_hit_test() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        parent.hidden = true;
        let mut child = View::new(Rect::new(10.0, 10.0, 50.0, 50.0));
        child.tag = 99;
        parent.add_subview(child);

        // Even though the point is inside both parent and child, hidden parent returns None
        assert!(parent.hit_test(Point::new(30.0, 30.0)).is_none());
    }

    #[test]
    fn view_bounds_default() {
        let v = View::new(Rect::new(50.0, 100.0, 200.0, 150.0));
        // Bounds origin should be (0,0), dimensions should match frame
        assert_eq!(v.bounds.x, 0.0);
        assert_eq!(v.bounds.y, 0.0);
        assert_eq!(v.bounds.width, 200.0);
        assert_eq!(v.bounds.height, 150.0);
    }

    #[test]
    fn view_hit_test_mut_returns_mutable() {
        let mut parent = View::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        let mut child = View::new(Rect::new(10.0, 10.0, 80.0, 80.0));
        child.tag = 55;
        parent.add_subview(child);

        let hit = parent.hit_test_mut(Point::new(50.0, 50.0));
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.tag, 55);
        hit.tag = 99;
        // Verify mutation persisted
        assert_eq!(parent.subviews()[0].tag, 99);
    }

    #[test]
    fn view_apply_layout() {
        use layout::{LayoutAttribute, LayoutConstraint, LayoutEngine, LayoutRelation, ViewId};

        let mut root = View::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let mut child = View::new(Rect::new(0.0, 0.0, 0.0, 0.0));
        child.layout_id = Some(ViewId(1));
        root.add_subview(child);

        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        engine.add_constraint(LayoutConstraint::constant(
            ViewId(1),
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            100.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            ViewId(1),
            LayoutAttribute::Height,
            LayoutRelation::Equal,
            50.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            ViewId(1),
            LayoutAttribute::Left,
            LayoutRelation::Equal,
            10.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            ViewId(1),
            LayoutAttribute::Top,
            LayoutRelation::Equal,
            20.0,
        ));

        root.apply_layout(&engine);

        let child_frame = root.subviews()[0].frame;
        assert!(
            (child_frame.width - 100.0).abs() < 2.0,
            "w={}",
            child_frame.width
        );
        assert!(
            (child_frame.height - 50.0).abs() < 2.0,
            "h={}",
            child_frame.height
        );
        assert!((child_frame.x - 10.0).abs() < 2.0, "x={}", child_frame.x);
        assert!((child_frame.y - 20.0).abs() < 2.0, "y={}", child_frame.y);
    }

    #[test]
    fn view_set_needs_display() {
        let mut v = View::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        // Draw to clear needs_display
        let mut surface = graphics_core::Surface::new(100, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        v.draw(&mut renderer);
        assert!(!v.needs_display());

        v.set_needs_display();
        assert!(v.needs_display());
    }
}
