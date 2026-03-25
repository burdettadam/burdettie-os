// ── Geometry types ─────────────────────────────────────────────────

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Origin point (top-left).
    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Whether the rect contains a point.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Intersect with another rect.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Inset the rect by a uniform amount on all sides.
    pub fn inset(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(
            self.x + dx,
            self.y + dy,
            (self.width - 2.0 * dx).max(0.0),
            (self.height - 2.0 * dy).max(0.0),
        )
    }
}
