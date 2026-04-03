//! 2D Bézier path: move, line, quadratic curve, cubic curve, close.
//!
//! Provides `BezierPath` — a CGPath/NSBezierPath equivalent that stores
//! a sequence of path commands. Renderers flatten curves into line segments
//! for rasterization.

use crate::geometry::Point;

/// A single drawing command in a Bézier path.
#[derive(Debug, Clone, Copy)]
pub enum PathElement {
    /// Move the current point (starts a new subpath).
    MoveTo(Point),
    /// Straight line from the current point.
    LineTo(Point),
    /// Quadratic Bézier curve: control point + end point.
    QuadTo { ctrl: Point, end: Point },
    /// Cubic Bézier curve: two control points + end point.
    CubicTo { ctrl1: Point, ctrl2: Point, end: Point },
    /// Close the current subpath (line back to the subpath start).
    Close,
}

/// A 2D vector path built from Bézier segments.
///
/// Equivalent to `CGMutablePath` / `NSBezierPath`. Used with `Renderer`
/// methods `fill_path` and `stroke_path`.
#[derive(Debug, Clone, Default)]
pub struct BezierPath {
    elements: Vec<PathElement>,
}

impl BezierPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, p: Point) -> &mut Self {
        self.elements.push(PathElement::MoveTo(p));
        self
    }

    pub fn line_to(&mut self, p: Point) -> &mut Self {
        self.elements.push(PathElement::LineTo(p));
        self
    }

    pub fn quad_to(&mut self, ctrl: Point, end: Point) -> &mut Self {
        self.elements.push(PathElement::QuadTo { ctrl, end });
        self
    }

    pub fn cubic_to(&mut self, ctrl1: Point, ctrl2: Point, end: Point) -> &mut Self {
        self.elements.push(PathElement::CubicTo { ctrl1, ctrl2, end });
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.elements.push(PathElement::Close);
        self
    }

    pub fn elements(&self) -> &[PathElement] {
        &self.elements
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    // ── Convenience shape constructors ─────────────────────────────

    /// Create a rectangle path.
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Self {
        let mut p = Self::new();
        p.move_to(Point::new(x, y));
        p.line_to(Point::new(x + w, y));
        p.line_to(Point::new(x + w, y + h));
        p.line_to(Point::new(x, y + h));
        p.close();
        p
    }

    /// Create a rounded rectangle path with a uniform corner radius.
    pub fn rounded_rect(x: f64, y: f64, w: f64, h: f64, radius: f64) -> Self {
        let r = radius.min(w * 0.5).min(h * 0.5);
        if r <= 0.0 {
            return Self::rect(x, y, w, h);
        }
        // κ for a quarter-circle cubic Bézier approximation.
        let k = r * 0.5522847498;
        let mut p = Self::new();
        // Start at top edge, after top-left radius
        p.move_to(Point::new(x + r, y));
        // Top edge → top-right corner
        p.line_to(Point::new(x + w - r, y));
        p.cubic_to(
            Point::new(x + w - r + k, y),
            Point::new(x + w, y + r - k),
            Point::new(x + w, y + r),
        );
        // Right edge → bottom-right corner
        p.line_to(Point::new(x + w, y + h - r));
        p.cubic_to(
            Point::new(x + w, y + h - r + k),
            Point::new(x + w - r + k, y + h),
            Point::new(x + w - r, y + h),
        );
        // Bottom edge → bottom-left corner
        p.line_to(Point::new(x + r, y + h));
        p.cubic_to(
            Point::new(x + r - k, y + h),
            Point::new(x, y + h - r + k),
            Point::new(x, y + h - r),
        );
        // Left edge → top-left corner
        p.line_to(Point::new(x, y + r));
        p.cubic_to(
            Point::new(x, y + r - k),
            Point::new(x + r - k, y),
            Point::new(x + r, y),
        );
        p.close();
        p
    }

    /// Create a circle path.
    pub fn circle(cx: f64, cy: f64, radius: f64) -> Self {
        Self::ellipse(cx - radius, cy - radius, radius * 2.0, radius * 2.0)
    }

    /// Create an ellipse inscribed in the given bounding box.
    pub fn ellipse(x: f64, y: f64, w: f64, h: f64) -> Self {
        let rx = w * 0.5;
        let ry = h * 0.5;
        let cx = x + rx;
        let cy = y + ry;
        let kx = rx * 0.5522847498;
        let ky = ry * 0.5522847498;

        let mut p = Self::new();
        // Start at top center
        p.move_to(Point::new(cx, cy - ry));
        // Top-right quadrant
        p.cubic_to(
            Point::new(cx + kx, cy - ry),
            Point::new(cx + rx, cy - ky),
            Point::new(cx + rx, cy),
        );
        // Bottom-right quadrant
        p.cubic_to(
            Point::new(cx + rx, cy + ky),
            Point::new(cx + kx, cy + ry),
            Point::new(cx, cy + ry),
        );
        // Bottom-left quadrant
        p.cubic_to(
            Point::new(cx - kx, cy + ry),
            Point::new(cx - rx, cy + ky),
            Point::new(cx - rx, cy),
        );
        // Top-left quadrant
        p.cubic_to(
            Point::new(cx - rx, cy),
            Point::new(cx - kx, cy - ry),
            Point::new(cx, cy - ry),
        );
        p.close();
        p
    }

    /// Flatten the path into line segments by recursively subdividing curves.
    ///
    /// `tolerance` controls the maximum deviation from the true curve in pixels.
    /// Returns a list of polyline subpaths (each subpath is a `Vec<Point>`).
    pub fn flatten(&self, tolerance: f64) -> Vec<Vec<Point>> {
        let mut subpaths: Vec<Vec<Point>> = Vec::new();
        let mut current: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        let mut cursor = Point::ZERO;

        for el in &self.elements {
            match *el {
                PathElement::MoveTo(p) => {
                    if !current.is_empty() {
                        subpaths.push(std::mem::take(&mut current));
                    }
                    current.push(p);
                    cursor = p;
                    start = p;
                }
                PathElement::LineTo(p) => {
                    current.push(p);
                    cursor = p;
                }
                PathElement::QuadTo { ctrl, end } => {
                    flatten_quad(cursor, ctrl, end, tolerance, &mut current);
                    cursor = end;
                }
                PathElement::CubicTo { ctrl1, ctrl2, end } => {
                    flatten_cubic(cursor, ctrl1, ctrl2, end, tolerance, &mut current);
                    cursor = end;
                }
                PathElement::Close => {
                    if !current.is_empty() {
                        current.push(start);
                        subpaths.push(std::mem::take(&mut current));
                    }
                    cursor = start;
                }
            }
        }
        if !current.is_empty() {
            subpaths.push(current);
        }
        subpaths
    }
}

/// Recursively flatten a quadratic Bézier into line segments.
fn flatten_quad(p0: Point, ctrl: Point, p2: Point, tol: f64, out: &mut Vec<Point>) {
    // Check if the control point is close enough to the line p0→p2.
    let mx = (p0.x + p2.x) * 0.5;
    let my = (p0.y + p2.y) * 0.5;
    let dx = ctrl.x - mx;
    let dy = ctrl.y - my;
    if dx * dx + dy * dy <= tol * tol {
        out.push(p2);
        return;
    }
    // De Casteljau subdivision at t=0.5
    let q0 = Point::new((p0.x + ctrl.x) * 0.5, (p0.y + ctrl.y) * 0.5);
    let q1 = Point::new((ctrl.x + p2.x) * 0.5, (ctrl.y + p2.y) * 0.5);
    let mid = Point::new((q0.x + q1.x) * 0.5, (q0.y + q1.y) * 0.5);
    flatten_quad(p0, q0, mid, tol, out);
    flatten_quad(mid, q1, p2, tol, out);
}

/// Recursively flatten a cubic Bézier into line segments.
fn flatten_cubic(
    p0: Point, c1: Point, c2: Point, p3: Point,
    tol: f64, out: &mut Vec<Point>,
) {
    // Flatness test: max distance of control points from the chord p0→p3.
    let ux = 3.0 * c1.x - 2.0 * p0.x - p3.x;
    let uy = 3.0 * c1.y - 2.0 * p0.y - p3.y;
    let vx = 3.0 * c2.x - 2.0 * p3.x - p0.x;
    let vy = 3.0 * c2.y - 2.0 * p3.y - p0.y;
    let max_sq = (ux * ux).max(vx * vx) + (uy * uy).max(vy * vy);
    if max_sq <= 16.0 * tol * tol {
        out.push(p3);
        return;
    }
    // De Casteljau at t=0.5
    let m01 = Point::new((p0.x + c1.x) * 0.5, (p0.y + c1.y) * 0.5);
    let m12 = Point::new((c1.x + c2.x) * 0.5, (c1.y + c2.y) * 0.5);
    let m23 = Point::new((c2.x + p3.x) * 0.5, (c2.y + p3.y) * 0.5);
    let m012 = Point::new((m01.x + m12.x) * 0.5, (m01.y + m12.y) * 0.5);
    let m123 = Point::new((m12.x + m23.x) * 0.5, (m12.y + m23.y) * 0.5);
    let mid = Point::new((m012.x + m123.x) * 0.5, (m012.y + m123.y) * 0.5);
    flatten_cubic(p0, m01, m012, mid, tol, out);
    flatten_cubic(mid, m123, m23, p3, tol, out);
}

impl BezierPath {
    /// Compute the axis-aligned bounding box of all control points in the path.
    pub fn bounding_box(&self) -> crate::geometry::Rect {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        let mut visit = |p: Point| {
            if p.x < min_x { min_x = p.x; }
            if p.y < min_y { min_y = p.y; }
            if p.x > max_x { max_x = p.x; }
            if p.y > max_y { max_y = p.y; }
        };

        for el in &self.elements {
            match *el {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => visit(p),
                PathElement::QuadTo { ctrl, end } => { visit(ctrl); visit(end); }
                PathElement::CubicTo { ctrl1, ctrl2, end } => {
                    visit(ctrl1); visit(ctrl2); visit(end);
                }
                PathElement::Close => {}
            }
        }

        if min_x > max_x {
            return crate::geometry::Rect::ZERO;
        }
        crate::geometry::Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}
