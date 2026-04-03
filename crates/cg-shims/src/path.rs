//! CGPath / CGMutablePath — bridge-backed path objects.
//!
//! `CGPathCreateMutable` allocates a real `BezierPath` via the bridge module.
//! Building operations (`MoveTo`, `LineTo`, `CurveTo`, etc.) mutate the path.

use crate::bridge;
use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGPathCreateMutable", shim_cg_path_create as u64, "CGPathCreateMutable (bridge → real)"),
        ("_CGPathCreateCopy", shim_cg_path_copy as u64, "CGPathCreateCopy (bridge → clone)"),
        ("_CGPathCreateMutableCopy", shim_cg_path_copy as u64, "CGPathCreateMutableCopy (bridge → clone)"),
        ("_CGPathRetain", shim_cg_path_retain as u64, "CGPathRetain (bridge → same)"),
        ("_CGPathRelease", shim_cg_path_release as u64, "CGPathRelease (bridge → real)"),
        ("_CGPathMoveToPoint", shim_cg_path_move_to as u64, "CGPathMoveToPoint (bridge → real)"),
        ("_CGPathAddLineToPoint", shim_cg_path_line_to as u64, "CGPathAddLineToPoint (bridge → real)"),
        ("_CGPathAddCurveToPoint", shim_cg_path_curve_to as u64, "CGPathAddCurveToPoint (bridge → real)"),
        ("_CGPathAddRect", shim_cg_path_add_rect as u64, "CGPathAddRect (bridge → real)"),
        ("_CGPathAddEllipseInRect", shim_cg_path_add_ellipse as u64, "CGPathAddEllipseInRect (bridge → real)"),
        ("_CGPathCloseSubpath", shim_cg_path_close as u64, "CGPathCloseSubpath (bridge → real)"),
        ("_CGPathIsEmpty", shim_cg_path_is_empty as u64, "CGPathIsEmpty (bridge → real)"),
        ("_CGPathGetBoundingBox", shim_cg_path_void1 as u64, "CGPathGetBoundingBox (stub → zeroed)"),
        ("_CGPathContainsPoint", shim_cg_path_false as u64, "CGPathContainsPoint (stub → false)")
    );
}

/// CGPathCreateMutable → allocate a real BezierPath.
pub extern "C" fn shim_cg_path_create() -> u64 {
    bridge::path_create()
}

/// CGPathCreateCopy / CGPathCreateMutableCopy → clone path elements into a new handle.
pub extern "C" fn shim_cg_path_copy(path: u64) -> u64 {
    let new_handle = bridge::path_create();
    bridge::with_path(path, |src| {
        let elems: Vec<_> = src.elements().to_vec();
        bridge::with_path(new_handle, |dst| {
            for e in &elems {
                use graphics_core::PathElement;
                match e {
                    PathElement::MoveTo(p) => { dst.move_to(*p); }
                    PathElement::LineTo(p) => { dst.line_to(*p); }
                    PathElement::QuadTo { ctrl, end } => { dst.quad_to(*ctrl, *end); }
                    PathElement::CubicTo { ctrl1, ctrl2, end } => { dst.cubic_to(*ctrl1, *ctrl2, *end); }
                    PathElement::Close => { dst.close(); }
                }
            }
        });
    });
    new_handle
}

pub extern "C" fn shim_cg_path_retain(p: u64) -> u64 { p }

pub extern "C" fn shim_cg_path_release(p: u64) {
    bridge::path_release(p);
}

/// CGPathMoveToPoint(path, transform, x, y).
pub extern "C" fn shim_cg_path_move_to(path: u64, _transform: u64, x: f64, y: f64) {
    bridge::with_path(path, |p| { p.move_to(graphics_core::Point::new(x, y)); });
}

/// CGPathAddLineToPoint(path, transform, x, y).
pub extern "C" fn shim_cg_path_line_to(path: u64, _transform: u64, x: f64, y: f64) {
    bridge::with_path(path, |p| { p.line_to(graphics_core::Point::new(x, y)); });
}

/// CGPathAddCurveToPoint(path, transform, cp1x, cp1y, cp2x, cp2y, x, y).
pub extern "C" fn shim_cg_path_curve_to(
    path: u64, _transform: u64,
    cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64,
) {
    bridge::with_path(path, |p| {
        p.cubic_to(
            graphics_core::Point::new(cp1x, cp1y),
            graphics_core::Point::new(cp2x, cp2y),
            graphics_core::Point::new(x, y),
        );
    });

}

/// CGPathAddRect(path, transform, x, y, w, h).
pub extern "C" fn shim_cg_path_add_rect(path: u64, _transform: u64, x: f64, y: f64, w: f64, h: f64) {
    bridge::with_path(path, |p| {
        p.move_to(graphics_core::Point::new(x, y));
        p.line_to(graphics_core::Point::new(x + w, y));
        p.line_to(graphics_core::Point::new(x + w, y + h));
        p.line_to(graphics_core::Point::new(x, y + h));
        p.close();
    });

}

/// CGPathAddEllipseInRect — approximated as a rect for now.
pub extern "C" fn shim_cg_path_add_ellipse(path: u64, transform: u64, x: f64, y: f64, w: f64, h: f64) {
    shim_cg_path_add_rect(path, transform, x, y, w, h);
}

pub extern "C" fn shim_cg_path_close(path: u64) {
    bridge::with_path(path, |p| { p.close(); });
}

pub extern "C" fn shim_cg_path_is_empty(path: u64) -> i32 {
    if bridge::path_is_empty(path) { 1 } else { 0 }
}

// Stubs for not-yet-bridged operations
pub extern "C" fn shim_cg_path_void1(_a: u64) {}
pub extern "C" fn shim_cg_path_false(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 { 0 }
