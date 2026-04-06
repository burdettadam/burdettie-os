//! CGAffineTransform shims — real 2D affine math.
//!
//! On ARM64, CGAffineTransform is 48 bytes (6 × f64) which exceeds the HFA
//! limit of 4 elements, so it is returned via x8 indirect (sret).
//! Functions that return CGAffineTransform take an implicit sret pointer
//! as their first argument in the host ABI.

// Tuple returns in extern "C" are fine for our ABI-compatible shim calls.
#![allow(improper_ctypes_definitions)]

use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGAffineTransformMakeTranslation", shim_affine_make_translation as *const () as u64, "CGAffineTransformMakeTranslation (bridge → real)"),
        ("_CGAffineTransformMakeScale", shim_affine_make_scale as *const () as u64, "CGAffineTransformMakeScale (bridge → real)"),
        ("_CGAffineTransformMakeRotation", shim_affine_make_rotation as *const () as u64, "CGAffineTransformMakeRotation (bridge → real)"),
        ("_CGAffineTransformConcat", shim_affine_concat as *const () as u64, "CGAffineTransformConcat (bridge → real)"),
        ("_CGAffineTransformInvert", shim_affine_invert as *const () as u64, "CGAffineTransformInvert (bridge → real)"),
        ("_CGAffineTransformIsIdentity", shim_affine_is_identity as *const () as u64, "CGAffineTransformIsIdentity (bridge → real)"),
        ("_CGPointApplyAffineTransform", shim_point_apply as *const () as u64, "CGPointApplyAffineTransform (bridge → real)"),
        ("_CGSizeApplyAffineTransform", shim_size_apply as *const () as u64, "CGSizeApplyAffineTransform (bridge → real)"),
        ("_CGRectApplyAffineTransform", shim_rect_apply as *const () as u64, "CGRectApplyAffineTransform (bridge → real)"),
        ("_CGAffineTransformIdentity", &IDENTITY_TRANSFORM as *const _ as u64, "CGAffineTransformIdentity (global constant)"),
        ("_CGAffineTransformTranslate", shim_affine_translate as *const () as u64, "CGAffineTransformTranslate (bridge → real)"),
        ("_CGAffineTransformScale", shim_affine_scale as *const () as u64, "CGAffineTransformScale (bridge → real)"),
        ("_CGAffineTransformRotate", shim_affine_rotate as *const () as u64, "CGAffineTransformRotate (bridge → real)")
    );
}

/// Layout-compatible with `CGAffineTransform { a, b, c, d, tx, ty }`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Affine {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

static IDENTITY_TRANSFORM: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

/// Write 6 f64s through an indirect sret pointer.
#[inline]
unsafe fn write_affine(sret: u64, t: Affine) {
    if sret == 0 { return; }
    let p = sret as *mut f64;
    *p = t.a;
    *p.add(1) = t.b;
    *p.add(2) = t.c;
    *p.add(3) = t.d;
    *p.add(4) = t.tx;
    *p.add(5) = t.ty;
}

/// Read a CGAffineTransform from a pointer (passed by invisible reference on ARM64).
#[inline]
unsafe fn read_affine(ptr: u64) -> Affine {
    if ptr == 0 { return IDENTITY_TRANSFORM; }
    let p = ptr as *const f64;
    Affine {
        a: *p,
        b: *p.add(1),
        c: *p.add(2),
        d: *p.add(3),
        tx: *p.add(4),
        ty: *p.add(5),
    }
}

/// Multiply two affine transforms: result = t1 * t2.
#[inline]
fn concat(t1: Affine, t2: Affine) -> Affine {
    Affine {
        a: t1.a * t2.a + t1.b * t2.c,
        b: t1.a * t2.b + t1.b * t2.d,
        c: t1.c * t2.a + t1.d * t2.c,
        d: t1.c * t2.b + t1.d * t2.d,
        tx: t1.tx * t2.a + t1.ty * t2.c + t2.tx,
        ty: t1.tx * t2.b + t1.ty * t2.d + t2.ty,
    }
}

// ── Make functions (sret in first arg, parameters in d0/d1) ─────

/// CGAffineTransformMakeTranslation(tx, ty) → sret
pub extern "C" fn shim_affine_make_translation(sret: u64, tx: f64, ty: f64) {
    unsafe { write_affine(sret, Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx, ty }); }
}

/// CGAffineTransformMakeScale(sx, sy) → sret
pub extern "C" fn shim_affine_make_scale(sret: u64, sx: f64, sy: f64) {
    unsafe { write_affine(sret, Affine { a: sx, b: 0.0, c: 0.0, d: sy, tx: 0.0, ty: 0.0 }); }
}

/// CGAffineTransformMakeRotation(angle) → sret
pub extern "C" fn shim_affine_make_rotation(sret: u64, angle: f64) {
    let (s, c) = angle.sin_cos();
    unsafe { write_affine(sret, Affine { a: c, b: s, c: -s, d: c, tx: 0.0, ty: 0.0 }); }
}

// ── Transform operations (sret, transform_ptr, params) ──────────

/// CGAffineTransformTranslate(t, tx, ty) → sret
pub extern "C" fn shim_affine_translate(sret: u64, t_ptr: u64, tx: f64, ty: f64) {
    let t = unsafe { read_affine(t_ptr) };
    let m = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx, ty };
    unsafe { write_affine(sret, concat(t, m)); }
}

/// CGAffineTransformScale(t, sx, sy) → sret
pub extern "C" fn shim_affine_scale(sret: u64, t_ptr: u64, sx: f64, sy: f64) {
    let t = unsafe { read_affine(t_ptr) };
    let m = Affine { a: sx, b: 0.0, c: 0.0, d: sy, tx: 0.0, ty: 0.0 };
    unsafe { write_affine(sret, concat(t, m)); }
}

/// CGAffineTransformRotate(t, angle) → sret
pub extern "C" fn shim_affine_rotate(sret: u64, t_ptr: u64, angle: f64) {
    let t = unsafe { read_affine(t_ptr) };
    let (s, c) = angle.sin_cos();
    let m = Affine { a: c, b: s, c: -s, d: c, tx: 0.0, ty: 0.0 };
    unsafe { write_affine(sret, concat(t, m)); }
}

/// CGAffineTransformConcat(t1_ptr, t2_ptr) → sret
pub extern "C" fn shim_affine_concat(sret: u64, t1_ptr: u64, t2_ptr: u64) {
    let t1 = unsafe { read_affine(t1_ptr) };
    let t2 = unsafe { read_affine(t2_ptr) };
    unsafe { write_affine(sret, concat(t1, t2)); }
}

/// CGAffineTransformInvert(t_ptr) → sret
pub extern "C" fn shim_affine_invert(sret: u64, t_ptr: u64) {
    let t = unsafe { read_affine(t_ptr) };
    let det = t.a * t.d - t.b * t.c;
    if det.abs() < 1e-15 {
        // Singular matrix → return identity
        unsafe { write_affine(sret, IDENTITY_TRANSFORM); }
        return;
    }
    let inv_det = 1.0 / det;
    unsafe {
        write_affine(sret, Affine {
            a: t.d * inv_det,
            b: -t.b * inv_det,
            c: -t.c * inv_det,
            d: t.a * inv_det,
            tx: (t.c * t.ty - t.d * t.tx) * inv_det,
            ty: (t.b * t.tx - t.a * t.ty) * inv_det,
        });
    }
}

/// CGAffineTransformIsIdentity(t_ptr) → bool
pub extern "C" fn shim_affine_is_identity(t_ptr: u64) -> i32 {
    let t = unsafe { read_affine(t_ptr) };
    let is_id = (t.a - 1.0).abs() < 1e-12
        && t.b.abs() < 1e-12
        && t.c.abs() < 1e-12
        && (t.d - 1.0).abs() < 1e-12
        && t.tx.abs() < 1e-12
        && t.ty.abs() < 1e-12;
    is_id as i32
}

// ── Point / Size / Rect apply ───────────────────────────────────

/// CGPointApplyAffineTransform(point, t) → CGPoint.
/// On ARM64: CGPoint is HFA-2 (2×f64), returned in d0/d1.
/// Parameters: x in d0, y in d1, t_ptr in x0 (invisible ref for >16 byte struct).
pub extern "C" fn shim_point_apply(x: f64, y: f64, t_ptr: u64) -> (f64, f64) {
    let t = unsafe { read_affine(t_ptr) };
    (t.a * x + t.c * y + t.tx, t.b * x + t.d * y + t.ty)
}

/// CGSizeApplyAffineTransform(size, t) → CGSize.
/// Size transform ignores translation.
pub extern "C" fn shim_size_apply(w: f64, h: f64, t_ptr: u64) -> (f64, f64) {
    let t = unsafe { read_affine(t_ptr) };
    ((t.a * w + t.c * h).abs(), (t.b * w + t.d * h).abs())
}

/// CGRectApplyAffineTransform(rect_ptr, t_ptr) → sret (CGRect = 32 bytes, via x8).
pub extern "C" fn shim_rect_apply(sret: u64, rect_ptr: u64, t_ptr: u64) {
    if sret == 0 { return; }
    let t = unsafe { read_affine(t_ptr) };
    let (rx, ry, rw, rh) = if rect_ptr != 0 {
        let p = rect_ptr as *const f64;
        unsafe { (*p, *p.add(1), *p.add(2), *p.add(3)) }
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };
    // Transform all 4 corners and compute the bounding box.
    let corners = [
        (t.a * rx + t.c * ry + t.tx, t.b * rx + t.d * ry + t.ty),
        (t.a * (rx + rw) + t.c * ry + t.tx, t.b * (rx + rw) + t.d * ry + t.ty),
        (t.a * rx + t.c * (ry + rh) + t.tx, t.b * rx + t.d * (ry + rh) + t.ty),
        (t.a * (rx + rw) + t.c * (ry + rh) + t.tx, t.b * (rx + rw) + t.d * (ry + rh) + t.ty),
    ];
    let min_x = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
    let min_y = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
    let max_x = corners.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
    let max_y = corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
    unsafe {
        let p = sret as *mut f64;
        *p = min_x;
        *p.add(1) = min_y;
        *p.add(2) = max_x - min_x;
        *p.add(3) = max_y - min_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsystem_shims::table::ShimTable;

    #[test]
    fn affine_stubs_registered() {
        let mut table = ShimTable::new();
        install(&mut table);
        assert!(table.resolve("_CGAffineTransformMakeTranslation").is_some());
        assert!(table.resolve("_CGAffineTransformConcat").is_some());
        assert!(table.resolve("_CGAffineTransformIdentity").is_some());
    }

    #[test]
    fn make_translation_writes_correct_values() {
        let mut buf = [0.0f64; 6];
        let sret = buf.as_mut_ptr() as u64;
        shim_affine_make_translation(sret, 10.0, 20.0);
        assert_eq!(buf, [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
    }

    #[test]
    fn make_scale_writes_correct_values() {
        let mut buf = [0.0f64; 6];
        let sret = buf.as_mut_ptr() as u64;
        shim_affine_make_scale(sret, 2.0, 3.0);
        assert_eq!(buf, [2.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn concat_produces_correct_result() {
        // Translation(5,10) concat with Scale(2,2) = Scale first, then translate
        let t1 = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 5.0, ty: 10.0 };
        let t2 = Affine { a: 2.0, b: 0.0, c: 0.0, d: 2.0, tx: 0.0, ty: 0.0 };
        let result = concat(t1, t2);
        assert!((result.a - 2.0).abs() < 1e-12);
        assert!((result.d - 2.0).abs() < 1e-12);
        assert!((result.tx - 10.0).abs() < 1e-12);
        assert!((result.ty - 20.0).abs() < 1e-12);
    }

    #[test]
    fn invert_roundtrip() {
        let mut buf1 = [0.0f64; 6];
        let mut buf2 = [0.0f64; 6];
        // Make a non-trivial transform
        let t = Affine { a: 2.0, b: 1.0, c: -1.0, d: 3.0, tx: 5.0, ty: 7.0 };
        let t_ptr = &t as *const _ as u64;
        shim_affine_invert(buf1.as_mut_ptr() as u64, t_ptr);
        // Concat t with t_inverse should give identity
        let t_inv_ptr = buf1.as_ptr() as u64;
        shim_affine_concat(buf2.as_mut_ptr() as u64, t_ptr, t_inv_ptr);
        assert!((buf2[0] - 1.0).abs() < 1e-10); // a
        assert!(buf2[1].abs() < 1e-10);           // b
        assert!(buf2[2].abs() < 1e-10);           // c
        assert!((buf2[3] - 1.0).abs() < 1e-10); // d
        assert!(buf2[4].abs() < 1e-10);           // tx
        assert!(buf2[5].abs() < 1e-10);           // ty
    }

    #[test]
    fn is_identity_check() {
        let id = IDENTITY_TRANSFORM;
        assert_eq!(shim_affine_is_identity(&id as *const _ as u64), 1);
        let t = Affine { a: 2.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };
        assert_eq!(shim_affine_is_identity(&t as *const _ as u64), 0);
    }
}
