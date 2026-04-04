//! CGAffineTransform stubs.

use libsystem_shims::table::ShimTable;

pub fn install(table: &mut ShimTable) {
    use libsystem_shims::register_shims;
    register_shims!(table,
        ("_CGAffineTransformMakeTranslation", shim_cg_affine_identity as *const () as u64, "CGAffineTransformMakeTranslation (stub → identity)"),
        ("_CGAffineTransformMakeScale", shim_cg_affine_identity as *const () as u64, "CGAffineTransformMakeScale (stub → identity)"),
        ("_CGAffineTransformMakeRotation", shim_cg_affine_identity as *const () as u64, "CGAffineTransformMakeRotation (stub → identity)"),
        ("_CGAffineTransformConcat", shim_cg_affine_identity as *const () as u64, "CGAffineTransformConcat (stub → identity)"),
        ("_CGAffineTransformInvert", shim_cg_affine_identity as *const () as u64, "CGAffineTransformInvert (stub → identity)"),
        ("_CGAffineTransformIsIdentity", shim_cg_affine_true as *const () as u64, "CGAffineTransformIsIdentity (stub → true)"),
        ("_CGPointApplyAffineTransform", shim_cg_point_passthrough as *const () as u64, "CGPointApplyAffineTransform (stub → passthrough)"),
        ("_CGSizeApplyAffineTransform", shim_cg_point_passthrough as *const () as u64, "CGSizeApplyAffineTransform (stub → passthrough)"),
        ("_CGRectApplyAffineTransform", shim_cg_affine_identity as *const () as u64, "CGRectApplyAffineTransform (stub → passthrough)")
    );
}

/// Returns identity transform values via x0/x1 (packed as two u64 on ARM64)
pub extern "C" fn shim_cg_affine_identity(_a: u64, _b: u64) -> u64 { 0 }
pub extern "C" fn shim_cg_affine_true(_a: u64) -> i32 { 1 }
pub extern "C" fn shim_cg_point_passthrough(x: u64, _y: u64) -> u64 { x }
