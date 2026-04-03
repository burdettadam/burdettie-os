use super::*;
use crate::color::Color;
use crate::geometry::Point;
use crate::gradient::{Gradient, GradientStop};
use crate::path::{BezierPath, PathElement};
use crate::shadow::Shadow;

// ── BezierPath tests ───────────────────────────────────────────────

#[test]
fn path_new_is_empty() {
    assert!(BezierPath::new().is_empty());
}

#[test]
fn path_move_to() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(1.0, 2.0));
    assert_eq!(p.elements().len(), 1);
    assert!(matches!(p.elements()[0], PathElement::MoveTo(_)));
}

#[test]
fn path_line_to() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.line_to(Point::new(10.0, 10.0));
    assert_eq!(p.elements().len(), 2);
    assert!(matches!(p.elements()[1], PathElement::LineTo(_)));
}

#[test]
fn path_quad_to() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.quad_to(Point::new(5.0, 10.0), Point::new(10.0, 0.0));
    assert_eq!(p.elements().len(), 2);
    assert!(matches!(p.elements()[1], PathElement::QuadTo { .. }));
}

#[test]
fn path_cubic_to() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.cubic_to(
        Point::new(3.0, 10.0),
        Point::new(7.0, 10.0),
        Point::new(10.0, 0.0),
    );
    assert_eq!(p.elements().len(), 2);
    assert!(matches!(p.elements()[1], PathElement::CubicTo { .. }));
}

#[test]
fn path_close() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.line_to(Point::new(10.0, 0.0));
    p.close();
    assert_eq!(p.elements().len(), 3);
    assert!(matches!(p.elements()[2], PathElement::Close));
}

#[test]
fn path_chaining() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0))
        .line_to(Point::new(10.0, 0.0))
        .line_to(Point::new(10.0, 10.0))
        .close();
    assert_eq!(p.elements().len(), 4);
}

#[test]
fn path_rect() {
    let p = BezierPath::rect(10.0, 20.0, 50.0, 60.0);
    // MoveTo + 3 LineTo + Close = 5
    assert_eq!(p.elements().len(), 5);
    assert!(matches!(p.elements()[0], PathElement::MoveTo(_)));
    assert!(matches!(p.elements()[1], PathElement::LineTo(_)));
    assert!(matches!(p.elements()[2], PathElement::LineTo(_)));
    assert!(matches!(p.elements()[3], PathElement::LineTo(_)));
    assert!(matches!(p.elements()[4], PathElement::Close));
}

#[test]
fn path_rounded_rect() {
    let p = BezierPath::rounded_rect(0.0, 0.0, 100.0, 80.0, 10.0);
    assert!(!p.is_empty());
    let has_cubic = p.elements().iter().any(|e| matches!(e, PathElement::CubicTo { .. }));
    assert!(has_cubic, "rounded rect should contain cubic curves");
}

#[test]
fn path_rounded_rect_zero_radius() {
    let rr = BezierPath::rounded_rect(10.0, 20.0, 50.0, 60.0, 0.0);
    let r = BezierPath::rect(10.0, 20.0, 50.0, 60.0);
    assert_eq!(rr.elements().len(), r.elements().len());
    // No cubics — it should be the same as a plain rect.
    let has_cubic = rr.elements().iter().any(|e| matches!(e, PathElement::CubicTo { .. }));
    assert!(!has_cubic);
}

#[test]
fn path_circle() {
    let p = BezierPath::circle(50.0, 50.0, 20.0);
    assert!(!p.is_empty());
    let cubic_count = p.elements().iter().filter(|e| matches!(e, PathElement::CubicTo { .. })).count();
    assert!(cubic_count >= 4, "circle should have at least 4 cubic segments");
}

#[test]
fn path_ellipse() {
    let p = BezierPath::ellipse(0.0, 0.0, 100.0, 60.0);
    assert!(!p.is_empty());
    let cubic_count = p.elements().iter().filter(|e| matches!(e, PathElement::CubicTo { .. })).count();
    assert!(cubic_count >= 4, "ellipse should have at least 4 cubic segments");
}

#[test]
fn path_flatten_line() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.line_to(Point::new(10.0, 5.0));
    let flat = p.flatten(0.25);
    assert_eq!(flat.len(), 1);
    let sub = &flat[0];
    assert_eq!(sub.first().unwrap().x, 0.0);
    assert_eq!(sub.first().unwrap().y, 0.0);
    assert_eq!(sub.last().unwrap().x, 10.0);
    assert_eq!(sub.last().unwrap().y, 5.0);
}

#[test]
fn path_flatten_quad() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.quad_to(Point::new(50.0, 100.0), Point::new(100.0, 0.0));
    let flat = p.flatten(0.25);
    assert_eq!(flat.len(), 1);
    // Curved path must flatten to more than a single segment.
    assert!(flat[0].len() > 2, "quad curve should produce multiple line segments");
}

#[test]
fn path_flatten_cubic() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.cubic_to(
        Point::new(0.0, 100.0),
        Point::new(100.0, 100.0),
        Point::new(100.0, 0.0),
    );
    let flat = p.flatten(0.25);
    assert_eq!(flat.len(), 1);
    assert!(flat[0].len() > 2, "cubic curve should produce multiple line segments");
}

#[test]
fn path_flatten_closed() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.line_to(Point::new(10.0, 0.0));
    p.line_to(Point::new(5.0, 10.0));
    p.close();
    let flat = p.flatten(0.25);
    assert_eq!(flat.len(), 1);
    let sub = &flat[0];
    let first = sub.first().unwrap();
    let last = sub.last().unwrap();
    assert!((first.x - last.x).abs() < 1e-12);
    assert!((first.y - last.y).abs() < 1e-12);
}

#[test]
fn path_flatten_empty() {
    let p = BezierPath::new();
    let flat = p.flatten(0.25);
    assert!(flat.is_empty());
}

#[test]
fn path_flatten_tolerance() {
    let mut p = BezierPath::new();
    p.move_to(Point::new(0.0, 0.0));
    p.cubic_to(
        Point::new(0.0, 100.0),
        Point::new(100.0, 100.0),
        Point::new(100.0, 0.0),
    );
    let coarse = p.flatten(5.0);
    let fine = p.flatten(0.1);
    assert!(
        fine[0].len() > coarse[0].len(),
        "smaller tolerance should produce more segments"
    );
}

// ── Gradient tests ─────────────────────────────────────────────────

#[test]
fn gradient_linear_two() {
    let g = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::BLACK,
        Color::WHITE,
    );
    assert_eq!(g.sample(0.0), Color::BLACK);
    assert_eq!(g.sample(1.0), Color::WHITE);
}

#[test]
fn gradient_linear_midpoint() {
    let g = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::BLACK,
        Color::WHITE,
    );
    let mid = g.sample(0.5);
    assert!((mid.r - 0.5).abs() < 1e-9);
    assert!((mid.g - 0.5).abs() < 1e-9);
    assert!((mid.b - 0.5).abs() < 1e-9);
    assert!((mid.a - 1.0).abs() < 1e-9);
}

#[test]
fn gradient_radial_two() {
    let g = Gradient::radial_two(Point::new(50.0, 50.0), 40.0, Color::RED, Color::BLUE);
    assert_eq!(g.sample(0.0), Color::RED);
    assert_eq!(g.sample(1.0), Color::BLUE);
}

#[test]
fn gradient_sample_clamps() {
    let g = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::RED,
        Color::BLUE,
    );
    assert_eq!(g.sample(-1.0), Color::RED);
    assert_eq!(g.sample(2.0), Color::BLUE);
}

#[test]
fn gradient_sample_empty_stops() {
    let g = Gradient::linear(Point::new(0.0, 0.0), Point::new(1.0, 0.0), vec![]);
    assert_eq!(g.sample(0.5), Color::TRANSPARENT);
}

#[test]
fn gradient_sample_single_stop() {
    let g = Gradient::linear(
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        vec![GradientStop::new(0.5, Color::GREEN)],
    );
    assert_eq!(g.sample(0.0), Color::GREEN);
    assert_eq!(g.sample(0.5), Color::GREEN);
    assert_eq!(g.sample(1.0), Color::GREEN);
}

#[test]
fn gradient_parameter_at_linear() {
    let g = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::BLACK,
        Color::WHITE,
    );
    let t = g.parameter_at(50.0, 0.0);
    assert!((t - 0.5).abs() < 1e-9);
    // Beyond start
    assert!((g.parameter_at(-10.0, 0.0)).abs() < 1e-9);
    // Beyond end
    assert!((g.parameter_at(200.0, 0.0) - 1.0).abs() < 1e-9);
}

#[test]
fn gradient_parameter_at_radial() {
    let g = Gradient::radial_two(Point::new(0.0, 0.0), 100.0, Color::BLACK, Color::WHITE);
    // Point at distance 50 from center → t = 0.5
    let t = g.parameter_at(50.0, 0.0);
    assert!((t - 0.5).abs() < 1e-9);
    // At center → 0
    assert!((g.parameter_at(0.0, 0.0)).abs() < 1e-9);
}

#[test]
fn gradient_color_at() {
    let g = Gradient::linear_two(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Color::BLACK,
        Color::WHITE,
    );
    let c = g.color_at(50.0, 0.0);
    assert!((c.r - 0.5).abs() < 1e-9);
}

#[test]
fn gradient_multi_stop() {
    let g = Gradient::linear(
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(0.5, Color::GREEN),
            GradientStop::new(1.0, Color::BLUE),
        ],
    );
    // At 0.0 → RED
    assert_eq!(g.sample(0.0), Color::RED);
    // At 0.5 → GREEN
    assert_eq!(g.sample(0.5), Color::GREEN);
    // At 1.0 → BLUE
    assert_eq!(g.sample(1.0), Color::BLUE);
    // At 0.25 → midpoint between RED and GREEN
    let mid = g.sample(0.25);
    assert!((mid.r - 0.5).abs() < 1e-9);
    assert!((mid.g - 0.5).abs() < 1e-9);
    assert!((mid.b).abs() < 1e-9);
}

#[test]
fn gradient_stop_new() {
    let s = GradientStop::new(0.5, Color::RED);
    assert!((s.offset - 0.5).abs() < 1e-12);
    assert_eq!(s.color, Color::RED);
}

// ── Shadow tests ───────────────────────────────────────────────────

#[test]
fn shadow_new() {
    let s = Shadow::new(1.0, 2.0, 3.0, Color::BLACK);
    assert!((s.offset_x - 1.0).abs() < 1e-12);
    assert!((s.offset_y - 2.0).abs() < 1e-12);
    assert!((s.blur_radius - 3.0).abs() < 1e-12);
    assert_eq!(s.color, Color::BLACK);
}

#[test]
fn shadow_standard() {
    let s = Shadow::standard();
    assert!((s.offset_x).abs() < 1e-12);
    assert!((s.offset_y - 3.0).abs() < 1e-12);
    assert!((s.blur_radius - 6.0).abs() < 1e-12);
    assert!((s.color.a - 0.3).abs() < 1e-9);
}
