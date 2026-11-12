//! Layout constraint system for ui-view.
//!
//! Split into focused sub-modules:
//! - `types`   — ViewId, LayoutAttribute, LayoutConstraint, EdgeInsets, etc.
//! - `engine`  — Cassowary-inspired constraint solver (LayoutEngine)
//! - `builders` — Convenience factories: pin_edges, center_in, set_size, etc.

pub mod builders;
pub mod engine;
pub mod types;

pub use builders::*;
pub use engine::*;
pub use types::*;

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use graphics_core::Rect;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1.5
    }

    #[test]
    fn fixed_size_centered() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraints(set_size(v, 100.0, 50.0));
        engine.add_constraints(center_in(v, SUPERVIEW));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 100.0), "width={}", f.width);
        assert!(approx(f.height, 50.0), "height={}", f.height);
        assert!(
            approx(f.x + f.width / 2.0, 200.0),
            "cx={}",
            f.x + f.width / 2.0
        );
        assert!(
            approx(f.y + f.height / 2.0, 150.0),
            "cy={}",
            f.y + f.height / 2.0
        );
    }

    #[test]
    fn pin_to_edges() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraints(pin_edges(v, SUPERVIEW, 10.0));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 10.0), "left={}", f.x);
        assert!(approx(f.y, 10.0), "top={}", f.y);
        assert!(approx(f.width, 380.0), "width={}", f.width);
        assert!(approx(f.height, 280.0), "height={}", f.height);
    }

    #[test]
    fn two_views_side_by_side() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let a = ViewId(1);
        let b = ViewId(2);

        // A: left-pinned, half width
        engine.add_constraint(equal(
            a,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            a,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));
        engine.add_constraint(equal(
            a,
            LayoutAttribute::Bottom,
            SUPERVIEW,
            LayoutAttribute::Bottom,
            0.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            a,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            200.0,
        ));

        // B: right of A, fills remaining
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Left,
            a,
            LayoutAttribute::Right,
            0.0,
        ));
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Right,
            SUPERVIEW,
            LayoutAttribute::Right,
            0.0,
        ));
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Bottom,
            SUPERVIEW,
            LayoutAttribute::Bottom,
            0.0,
        ));

        let frames = engine.solve().unwrap();
        let fa = frames[&a];
        let fb = frames[&b];

        assert!(approx(fa.x, 0.0));
        assert!(approx(fa.width, 200.0), "a.width={}", fa.width);
        assert!(approx(fb.x, 200.0), "b.x={}", fb.x);
        assert!(approx(fb.width, 200.0), "b.width={}", fb.width);
        assert!(approx(fb.height, 300.0), "b.height={}", fb.height);
    }

    #[test]
    fn intrinsic_size_hugging() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Place at top-left
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            10.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            10.0,
        ));

        // Intrinsic size: 80×20
        engine.add_constraints(intrinsic_size_constraints(
            v,
            IntrinsicSize::new(80.0, 20.0),
            LayoutPriority::FITTING_SIZE_HUGGING,
            LayoutPriority::FITTING_SIZE_COMPRESSION,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 10.0), "x={}", f.x);
        assert!(approx(f.y, 10.0), "y={}", f.y);
        // Width should be close to intrinsic (hugging)
        assert!(f.width >= 79.0 && f.width <= 81.0, "width={}", f.width);
        assert!(f.height >= 19.0 && f.height <= 21.0, "height={}", f.height);
    }

    #[test]
    fn container_resize() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);
        engine.add_constraints(pin_edges(v, SUPERVIEW, 0.0));

        let frames = engine.solve().unwrap();
        assert!(approx(frames[&v].width, 400.0));

        // Resize container
        engine.set_container(Rect::new(0.0, 0.0, 800.0, 600.0));
        let frames = engine.solve().unwrap();
        assert!(
            approx(frames[&v].width, 800.0),
            "width={}",
            frames[&v].width
        );
        assert!(
            approx(frames[&v].height, 600.0),
            "height={}",
            frames[&v].height
        );
    }

    #[test]
    fn equal_width_three_views() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 300.0, 100.0));
        let a = ViewId(1);
        let b = ViewId(2);
        let c = ViewId(3);

        // All three have equal width and fill the container horizontally
        engine.add_constraint(equal(
            a,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Left,
            a,
            LayoutAttribute::Right,
            0.0,
        ));
        engine.add_constraint(equal(
            c,
            LayoutAttribute::Left,
            b,
            LayoutAttribute::Right,
            0.0,
        ));
        engine.add_constraint(equal(
            c,
            LayoutAttribute::Right,
            SUPERVIEW,
            LayoutAttribute::Right,
            0.0,
        ));

        // Equal widths: b.width == a.width, c.width == a.width
        engine.add_constraint(equal(
            b,
            LayoutAttribute::Width,
            a,
            LayoutAttribute::Width,
            0.0,
        ));
        engine.add_constraint(equal(
            c,
            LayoutAttribute::Width,
            a,
            LayoutAttribute::Width,
            0.0,
        ));

        // All fill height
        for v in [a, b, c] {
            engine.add_constraint(equal(
                v,
                LayoutAttribute::Top,
                SUPERVIEW,
                LayoutAttribute::Top,
                0.0,
            ));
            engine.add_constraint(equal(
                v,
                LayoutAttribute::Bottom,
                SUPERVIEW,
                LayoutAttribute::Bottom,
                0.0,
            ));
        }

        let frames = engine.solve().unwrap();
        assert!(
            approx(frames[&a].width, 100.0),
            "a.width={}",
            frames[&a].width
        );
        assert!(
            approx(frames[&b].width, 100.0),
            "b.width={}",
            frames[&b].width
        );
        assert!(
            approx(frames[&c].width, 100.0),
            "c.width={}",
            frames[&c].width
        );
        assert!(approx(frames[&a].x, 0.0));
        assert!(approx(frames[&b].x, 100.0), "b.x={}", frames[&b].x);
        assert!(approx(frames[&c].x, 200.0), "c.x={}", frames[&c].x);
    }

    #[test]
    fn aspect_ratio_manual() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Width = 200, height = 0.5 * width (2:1 aspect ratio)
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            200.0,
        ));
        engine.add_constraint(LayoutConstraint::new(
            v,
            LayoutAttribute::Height,
            LayoutRelation::Equal,
            v,
            LayoutAttribute::Width,
            0.5,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 200.0));
        assert!(approx(f.height, 100.0), "height={}", f.height);
    }

    #[test]
    fn layout_engine_clear() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(10);
        for i in 0..5 {
            engine.add_constraint(LayoutConstraint::constant(
                v,
                LayoutAttribute::Width,
                LayoutRelation::Equal,
                (i * 10) as f64,
            ));
        }
        assert_eq!(engine.constraint_count(), 5);
        engine.clear();
        assert_eq!(engine.constraint_count(), 0);
    }

    #[test]
    fn layout_engine_constraint_count() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        let v = ViewId(1);
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Left,
            LayoutRelation::Equal,
            0.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Top,
            LayoutRelation::Equal,
            0.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            50.0,
        ));
        assert_eq!(engine.constraint_count(), 3);
    }

    #[test]
    fn layout_engine_container_getter() {
        let container = Rect::new(10.0, 20.0, 300.0, 200.0);
        let engine = LayoutEngine::new(container);
        let c = engine.container();
        assert_eq!(c.x, 10.0);
        assert_eq!(c.y, 20.0);
        assert_eq!(c.width, 300.0);
        assert_eq!(c.height, 200.0);
    }

    #[test]
    fn layout_engine_set_container() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        engine.set_container(Rect::new(5.0, 10.0, 800.0, 600.0));
        let c = engine.container();
        assert_eq!(c.x, 5.0);
        assert_eq!(c.y, 10.0);
        assert_eq!(c.width, 800.0);
        assert_eq!(c.height, 600.0);
    }

    #[test]
    fn conflict_detection() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Conflicting: width == 100 and width == 200, both required
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            100.0,
        ));
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            200.0,
        ));

        let result = engine.solve();
        assert!(result.is_err(), "should detect conflict");
    }

    // ── Safe area / margin guide tests ─────────────────────────────

    #[test]
    fn pin_to_margins_respects_insets() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        engine.set_margins(EdgeInsets::new(20.0, 16.0, 20.0, 16.0));
        let v = ViewId(1);

        engine.add_constraints(pin_to_margins(v));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 16.0), "left={}", f.x);
        assert!(approx(f.y, 20.0), "top={}", f.y);
        assert!(approx(f.width, 368.0), "width={}", f.width); // 400 - 16 - 16
        assert!(approx(f.height, 260.0), "height={}", f.height); // 300 - 20 - 20
    }

    #[test]
    fn pin_to_safe_area_respects_insets() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 390.0, 844.0));
        // iPhone-style: 47pt top notch, 34pt bottom home indicator
        engine.set_safe_area_insets(EdgeInsets::new(47.0, 0.0, 34.0, 0.0));
        let v = ViewId(1);

        engine.add_constraints(pin_to_safe_area(v));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.x, 0.0), "left={}", f.x);
        assert!(approx(f.y, 47.0), "top={}", f.y);
        assert!(approx(f.width, 390.0), "width={}", f.width);
        assert!(approx(f.height, 763.0), "height={}", f.height); // 844 - 47 - 34
    }

    #[test]
    fn safe_area_with_centered_child() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 800.0));
        engine.set_safe_area_insets(EdgeInsets::new(44.0, 0.0, 34.0, 0.0));
        let v = ViewId(1);

        engine.add_constraints(set_size(v, 100.0, 50.0));
        engine.add_constraints(center_in(v, SAFE_AREA_GUIDE));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 100.0));
        assert!(approx(f.height, 50.0));
        // Safe area center Y = 44 + (800-44-34)/2 = 44 + 361 = 405
        let expected_cy = 44.0 + (800.0 - 44.0 - 34.0) / 2.0;
        let actual_cy = f.y + f.height / 2.0;
        assert!(
            approx(actual_cy, expected_cy),
            "cy={} expected={}",
            actual_cy,
            expected_cy
        );
    }

    #[test]
    fn margins_uniform() {
        let insets = EdgeInsets::uniform(10.0);
        assert_eq!(insets, EdgeInsets::new(10.0, 10.0, 10.0, 10.0));
    }

    #[test]
    fn layout_empty_solve() {
        let engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        // No constraints added — solve should succeed with empty map
        let frames = engine.solve().unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn layout_safe_area_guide() {
        // Use SAFE_AREA_GUIDE directly in equal() to constrain a view's left edge
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 800.0));
        engine.set_safe_area_insets(EdgeInsets::new(50.0, 10.0, 30.0, 10.0));
        let v = ViewId(1);

        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SAFE_AREA_GUIDE,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SAFE_AREA_GUIDE,
            LayoutAttribute::Top,
            0.0,
        ));
        engine.add_constraints(set_size(v, 80.0, 40.0));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        // Safe area left = 0 + 10 = 10, top = 0 + 50 = 50
        assert!(approx(f.x, 10.0), "x={}", f.x);
        assert!(approx(f.y, 50.0), "y={}", f.y);
        assert!(approx(f.width, 80.0));
        assert!(approx(f.height, 40.0));
    }

    #[test]
    fn layout_margins_guide() {
        // Use LAYOUT_MARGINS_GUIDE directly in equal() to constrain a view's left edge
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 300.0, 200.0));
        engine.set_margins(EdgeInsets::new(8.0, 16.0, 8.0, 16.0));
        let v = ViewId(1);

        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            LAYOUT_MARGINS_GUIDE,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            LAYOUT_MARGINS_GUIDE,
            LayoutAttribute::Top,
            0.0,
        ));
        engine.add_constraints(set_size(v, 60.0, 30.0));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        // Margins left = 16, top = 8
        assert!(approx(f.x, 16.0), "x={}", f.x);
        assert!(approx(f.y, 8.0), "y={}", f.y);
        assert!(approx(f.width, 60.0));
        assert!(approx(f.height, 30.0));
    }

    // ── AspectRatio tests ──────────────────────────────────────────

    #[test]
    fn aspect_ratio_constraint_width_from_height() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Set height = 100, aspect = 2.0 → width should be 200
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Height,
            LayoutRelation::Equal,
            100.0,
        ));
        engine.add_constraint(aspect_ratio(v, 2.0));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.height, 100.0), "height={}", f.height);
        assert!(approx(f.width, 200.0), "width={}", f.width);
    }

    #[test]
    fn aspect_ratio_constraint_height_from_width() {
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        // Set width = 300, aspect = 1.5 → height = 300 / 1.5 = 200
        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Width,
            LayoutRelation::Equal,
            300.0,
        ));
        engine.add_constraint(aspect_ratio(v, 1.5));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 300.0), "width={}", f.width);
        assert!(approx(f.height, 200.0), "height={}", f.height);
    }

    // ── Compression resistance tests ───────────────────────────────

    #[test]
    fn compression_resistance_prevents_shrink() {
        // View with intrinsic 100px and HIGH compression resistance should not shrink
        // below 100 even in a constrained container.
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 80.0, 80.0));
        let v = ViewId(1);

        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        // Intrinsic size 100×100 with HIGH compression resistance
        let priorities = LayoutPriorities {
            compression_resistance_horizontal: LayoutPriority::HIGH,
            compression_resistance_vertical: LayoutPriority::HIGH,
            ..Default::default()
        };
        engine.add_constraints(intrinsic_size_with_priorities(
            v,
            IntrinsicSize::new(100.0, 100.0),
            &priorities,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        // With HIGH compression resistance, view should stay >= 100
        assert!(f.width >= 99.0, "width={} should be >= 100", f.width);
        assert!(f.height >= 99.0, "height={} should be >= 100", f.height);
    }

    #[test]
    fn compression_resistance_low_allows_shrink() {
        // View with LOW compression resistance can shrink below intrinsic size
        // when a higher-priority constraint demands it.
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        // Intrinsic size 100×100 but LOW compression resistance + LOW hugging
        engine.add_constraints(intrinsic_size_constraints(
            v,
            IntrinsicSize::new(100.0, 100.0),
            LayoutPriority::LOW, // hugging
            LayoutPriority::LOW, // compression
        ));

        // Now add a DEFAULT_HIGH constraint forcing width to 60
        engine.add_constraint(
            LayoutConstraint::constant(v, LayoutAttribute::Width, LayoutRelation::Equal, 60.0)
                .with_priority(LayoutPriority::DEFAULT_HIGH),
        );
        engine.add_constraint(
            LayoutConstraint::constant(v, LayoutAttribute::Height, LayoutRelation::Equal, 60.0)
                .with_priority(LayoutPriority::DEFAULT_HIGH),
        );

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        // HIGH-priority size constraint wins over LOW compression resistance → shrinks toward 60
        assert!(
            f.width < 80.0,
            "width={} should be < 80 (shrunk below intrinsic)",
            f.width
        );
        assert!(
            f.height < 80.0,
            "height={} should be < 80 (shrunk below intrinsic)",
            f.height
        );
    }

    #[test]
    fn aspect_ratio_with_centering() {
        // Aspect ratio + center_in container.
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraint(LayoutConstraint::constant(
            v,
            LayoutAttribute::Height,
            LayoutRelation::Equal,
            100.0,
        ));
        engine.add_constraint(aspect_ratio(v, 2.0)); // width = 200
        engine.add_constraints(center_in(v, SUPERVIEW));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        assert!(approx(f.width, 200.0), "width={}", f.width);
        assert!(approx(f.height, 100.0), "height={}", f.height);
        let cx = f.x + f.width / 2.0;
        let cy = f.y + f.height / 2.0;
        assert!(approx(cx, 200.0), "cx={}", cx);
        assert!(approx(cy, 150.0), "cy={}", cy);
    }

    #[test]
    fn layout_priorities_default() {
        let p = LayoutPriorities::default();
        assert_eq!(p.hugging_horizontal, LayoutPriority::LOW);
        assert_eq!(p.hugging_vertical, LayoutPriority::LOW);
        assert_eq!(p.compression_resistance_horizontal, LayoutPriority::HIGH);
        assert_eq!(p.compression_resistance_vertical, LayoutPriority::HIGH);
    }

    #[test]
    fn compression_resistance_interacts_with_hugging() {
        // Both compression resistance (HIGH) and hugging (HIGH) at intrinsic size
        // should keep the view at exactly its intrinsic size.
        let mut engine = LayoutEngine::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let v = ViewId(1);

        engine.add_constraint(equal(
            v,
            LayoutAttribute::Left,
            SUPERVIEW,
            LayoutAttribute::Left,
            0.0,
        ));
        engine.add_constraint(equal(
            v,
            LayoutAttribute::Top,
            SUPERVIEW,
            LayoutAttribute::Top,
            0.0,
        ));

        let priorities = LayoutPriorities {
            hugging_horizontal: LayoutPriority::HIGH,
            hugging_vertical: LayoutPriority::HIGH,
            compression_resistance_horizontal: LayoutPriority::HIGH,
            compression_resistance_vertical: LayoutPriority::HIGH,
        };
        engine.add_constraints(intrinsic_size_with_priorities(
            v,
            IntrinsicSize::new(120.0, 80.0),
            &priorities,
        ));

        let frames = engine.solve().unwrap();
        let f = frames[&v];
        // Both hugging and compression at HIGH → stays near intrinsic
        assert!(f.width >= 119.0 && f.width <= 121.0, "width={}", f.width);
        assert!(f.height >= 79.0 && f.height <= 81.0, "height={}", f.height);
    }
}
