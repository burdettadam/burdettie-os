//! CoreAnimation stub shims: CALayer, CATransaction, CADisplayLink.
//!
//! Modern Cocoa apps expect layer-backed views (`wantsLayer = YES`).
//! These stubs provide the minimum surface so guest code that accesses
//! `view.layer`, creates `CATransaction` groups, or queries
//! `CADisplayLink` does not crash.
//!
//! ## CALayer object layout (48 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: bounds_x            (f64)
//!   offset 16: bounds_y            (f64)
//!   offset 24: bounds_w            (f64)
//!   offset 32: bounds_h            (f64)
//!   offset 40: background_color    (u64 — CGColorRef, or 0)
//!
//! ## CATransaction — class-only, no instance layout.
//! ## CADisplayLink (24 bytes)
//!
//!   offset  0: isa                 (u64)
//!   offset  8: target              (u64)
//!   offset 16: selector            (u64)

const BOUNDS_X_OFF: usize = 8;
const BOUNDS_Y_OFF: usize = 16;
const BOUNDS_W_OFF: usize = 24;
const BOUNDS_H_OFF: usize = 32;
const BG_COLOR_OFF: usize = 40;
const LAYER_SIZE: usize = 48;

const DISPLAYLINK_TARGET_OFF: usize = 8;
const DISPLAYLINK_SEL_OFF: usize = 16;
const DISPLAYLINK_SIZE: usize = 24;

// ── CALayer ────────────────────────────────────────────────────

/// Allocate a new layer with zero bounds and no background color.
pub(crate) fn alloc_layer() -> u64 {
    let obj = crate::pal_alloc().calloc(1, LAYER_SIZE) as u64;
    if obj != 0 {
        unsafe {
            *(obj as *mut u64) = crate::nsobj::calayer_class_addr();
        }
    }
    obj
}

/// `-[CALayer init]`
pub extern "C" fn shim_calayer_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return alloc_layer();
    }
    obj
}

/// `-[CALayer bounds]` — returns origin.x (f64). Full CGRect via x8 in real ABI.
pub extern "C" fn shim_calayer_bounds(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + BOUNDS_X_OFF) as *const f64) }
}

/// `-[CALayer setBounds:]`
pub extern "C" fn shim_calayer_set_bounds(
    obj: u64, _cmd: u64,
    x: f64, y: f64, w: f64, h: f64,
) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + BOUNDS_X_OFF) as *mut f64) = x;
        *((obj as usize + BOUNDS_Y_OFF) as *mut f64) = y;
        *((obj as usize + BOUNDS_W_OFF) as *mut f64) = w;
        *((obj as usize + BOUNDS_H_OFF) as *mut f64) = h;
    }
}

/// `-[CALayer frame]` — same as bounds for non-transformed layers.
pub extern "C" fn shim_calayer_frame(obj: u64, _cmd: u64) -> f64 {
    shim_calayer_bounds(obj, _cmd)
}

/// `-[CALayer setFrame:]`
pub extern "C" fn shim_calayer_set_frame(
    obj: u64, _cmd: u64,
    x: f64, y: f64, w: f64, h: f64,
) {
    shim_calayer_set_bounds(obj, _cmd, x, y, w, h);
}

/// `-[CALayer position]` — returns center point x.
pub extern "C" fn shim_calayer_position(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe {
        let x = *((obj as usize + BOUNDS_X_OFF) as *const f64);
        let w = *((obj as usize + BOUNDS_W_OFF) as *const f64);
        x + w / 2.0
    }
}

/// `-[CALayer setPosition:]` — stub, adjusts bounds origin.
pub extern "C" fn shim_calayer_set_position(_obj: u64, _cmd: u64, _x: f64, _y: f64) {
    // no-op for now
}

/// `-[CALayer backgroundColor]`
pub extern "C" fn shim_calayer_background_color(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + BG_COLOR_OFF) as *const u64) }
}

/// `-[CALayer setBackgroundColor:]`
pub extern "C" fn shim_calayer_set_background_color(obj: u64, _cmd: u64, color: u64) {
    if obj == 0 {
        return;
    }
    unsafe { *((obj as usize + BG_COLOR_OFF) as *mut u64) = color; }
}

/// `-[CALayer setNeedsDisplay]` — no-op stub.
pub extern "C" fn shim_calayer_set_needs_display(_obj: u64, _cmd: u64) {}

/// `-[CALayer displayIfNeeded]` — no-op stub.
pub extern "C" fn shim_calayer_display_if_needed(_obj: u64, _cmd: u64) {}

/// `-[CALayer setNeedsLayout]` — no-op stub.
pub extern "C" fn shim_calayer_set_needs_layout(_obj: u64, _cmd: u64) {}

/// `-[CALayer layoutIfNeeded]` — no-op stub.
pub extern "C" fn shim_calayer_layout_if_needed(_obj: u64, _cmd: u64) {}

/// `-[CALayer addSublayer:]` — no-op stub.
pub extern "C" fn shim_calayer_add_sublayer(_obj: u64, _cmd: u64, _layer: u64) {}

/// `-[CALayer removeFromSuperlayer]` — no-op stub.
pub extern "C" fn shim_calayer_remove_from_superlayer(_obj: u64, _cmd: u64) {}

/// `-[CALayer sublayers]` — returns empty array.
pub extern "C" fn shim_calayer_sublayers(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `-[CALayer isHidden]` — returns NO.
pub extern "C" fn shim_calayer_is_hidden(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[CALayer setHidden:]` — no-op stub.
pub extern "C" fn shim_calayer_set_hidden(_obj: u64, _cmd: u64, _hidden: u8) {}

/// `-[CALayer opacity]` — returns 1.0.
pub extern "C" fn shim_calayer_opacity(_obj: u64, _cmd: u64) -> f32 {
    1.0
}

/// `-[CALayer setOpacity:]` — no-op stub.
pub extern "C" fn shim_calayer_set_opacity(_obj: u64, _cmd: u64, _opacity: f32) {}

/// `-[CALayer cornerRadius]` — returns 0.
pub extern "C" fn shim_calayer_corner_radius(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[CALayer setCornerRadius:]` — no-op stub.
pub extern "C" fn shim_calayer_set_corner_radius(_obj: u64, _cmd: u64, _r: f64) {}

/// `-[CALayer borderWidth]` — returns 0.
pub extern "C" fn shim_calayer_border_width(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[CALayer setBorderWidth:]` — no-op stub.
pub extern "C" fn shim_calayer_set_border_width(_obj: u64, _cmd: u64, _w: f64) {}

/// `-[CALayer borderColor]` — returns 0 (NULL).
pub extern "C" fn shim_calayer_border_color(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[CALayer setBorderColor:]` — no-op stub.
pub extern "C" fn shim_calayer_set_border_color(_obj: u64, _cmd: u64, _color: u64) {}

/// `-[CALayer shadowOpacity]` — returns 0.
pub extern "C" fn shim_calayer_shadow_opacity(_obj: u64, _cmd: u64) -> f32 {
    0.0
}

/// `-[CALayer setShadowOpacity:]` — no-op stub.
pub extern "C" fn shim_calayer_set_shadow_opacity(_obj: u64, _cmd: u64, _opacity: f32) {}

/// `-[CALayer contents]` — returns nil.
pub extern "C" fn shim_calayer_contents(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[CALayer setContents:]` — no-op stub.
pub extern "C" fn shim_calayer_set_contents(_obj: u64, _cmd: u64, _contents: u64) {}

/// `-[CALayer contentsScale]` — returns 1.0.
pub extern "C" fn shim_calayer_contents_scale(_obj: u64, _cmd: u64) -> f64 {
    1.0
}

/// `-[CALayer setContentsScale:]` — no-op stub.
pub extern "C" fn shim_calayer_set_contents_scale(_obj: u64, _cmd: u64, _scale: f64) {}

/// `-[CALayer masksToBounds]` — returns NO.
pub extern "C" fn shim_calayer_masks_to_bounds(_obj: u64, _cmd: u64) -> u8 {
    0
}

/// `-[CALayer setMasksToBounds:]` — no-op stub.
pub extern "C" fn shim_calayer_set_masks_to_bounds(_obj: u64, _cmd: u64, _flag: u8) {}

// ── CATransaction ──────────────────────────────────────────────

/// `+[CATransaction begin]` — no-op stub.
pub extern "C" fn shim_catransaction_begin(_cls: u64, _cmd: u64) {
    log::trace!("CATransaction begin (stub)");
}

/// `+[CATransaction commit]` — no-op stub.
pub extern "C" fn shim_catransaction_commit(_cls: u64, _cmd: u64) {
    log::trace!("CATransaction commit (stub)");
}

/// `+[CATransaction flush]` — no-op stub.
pub extern "C" fn shim_catransaction_flush(_cls: u64, _cmd: u64) {}

/// `+[CATransaction setDisableActions:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_disable_actions(_cls: u64, _cmd: u64, _flag: u8) {}

/// `+[CATransaction disableActions]` — returns NO.
pub extern "C" fn shim_catransaction_disable_actions(_cls: u64, _cmd: u64) -> u8 {
    0
}

/// `+[CATransaction animationDuration]` — returns 0.25.
pub extern "C" fn shim_catransaction_animation_duration(_cls: u64, _cmd: u64) -> f64 {
    0.25
}

/// `+[CATransaction setAnimationDuration:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_animation_duration(_cls: u64, _cmd: u64, _dur: f64) {}

/// `+[CATransaction setCompletionBlock:]` — no-op stub.
pub extern "C" fn shim_catransaction_set_completion_block(_cls: u64, _cmd: u64, _block: u64) {}

// ── CADisplayLink ──────────────────────────────────────────────

/// `+[CADisplayLink displayLinkWithTarget:selector:]`
pub extern "C" fn shim_cadisplaylink_create(
    _cls: u64, _cmd: u64, target: u64, selector: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, DISPLAYLINK_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::cadisplaylink_class_addr();
        *((obj as usize + DISPLAYLINK_TARGET_OFF) as *mut u64) = target;
        *((obj as usize + DISPLAYLINK_SEL_OFF) as *mut u64) = selector;
    }
    obj
}

/// `-[CADisplayLink addToRunLoop:forMode:]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_add_to_run_loop(
    _obj: u64, _cmd: u64, _run_loop: u64, _mode: u64,
) {}

/// `-[CADisplayLink invalidate]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_invalidate(_obj: u64, _cmd: u64) {}

/// `-[CADisplayLink isPaused]` — returns YES (paused by default).
pub extern "C" fn shim_cadisplaylink_is_paused(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[CADisplayLink setPaused:]` — no-op stub.
pub extern "C" fn shim_cadisplaylink_set_paused(_obj: u64, _cmd: u64, _paused: u8) {}

/// `-[CADisplayLink timestamp]` — returns 0.
pub extern "C" fn shim_cadisplaylink_timestamp(_obj: u64, _cmd: u64) -> f64 {
    0.0
}

/// `-[CADisplayLink duration]` — returns 1/60.
pub extern "C" fn shim_cadisplaylink_duration(_obj: u64, _cmd: u64) -> f64 {
    1.0 / 60.0
}

// ── CAMediaTimingFunction ──────────────────────────────────────

/// `+[CAMediaTimingFunction functionWithName:]` — returns a sentinel.
pub extern "C" fn shim_camediatimingfunction_with_name(_cls: u64, _cmd: u64, _name: u64) -> u64 {
    // Return a non-nil sentinel.
    static SENTINEL: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SENTINEL.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = crate::nsobj::camediatimingfunction_class_addr();
            }
        }
        obj
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // --- CALayer ---

    #[test]
    fn layer_alloc_has_isa() {
        let layer = alloc_layer();
        assert_ne!(layer, 0);
        let isa = unsafe { *(layer as *const u64) };
        assert_eq!(isa, crate::nsobj::calayer_class_addr());
        free_obj(layer);
    }

    #[test]
    fn layer_set_bounds_roundtrip() {
        let layer = alloc_layer();
        shim_calayer_set_bounds(layer, 0, 10.0, 20.0, 300.0, 400.0);
        let x = unsafe { *((layer as usize + BOUNDS_X_OFF) as *const f64) };
        let h = unsafe { *((layer as usize + BOUNDS_H_OFF) as *const f64) };
        assert!((x - 10.0).abs() < f64::EPSILON);
        assert!((h - 400.0).abs() < f64::EPSILON);
        free_obj(layer);
    }

    #[test]
    fn layer_background_color_roundtrip() {
        let layer = alloc_layer();
        shim_calayer_set_background_color(layer, 0, 0xCAFE);
        assert_eq!(shim_calayer_background_color(layer, 0), 0xCAFE);
        free_obj(layer);
    }

    #[test]
    fn layer_nil_safety() {
        assert!((shim_calayer_bounds(0, 0)).abs() < f64::EPSILON);
        shim_calayer_set_bounds(0, 0, 1.0, 2.0, 3.0, 4.0);
        assert_eq!(shim_calayer_background_color(0, 0), 0);
    }

    #[test]
    fn layer_opacity_default() {
        let layer = alloc_layer();
        assert!((shim_calayer_opacity(layer, 0) - 1.0).abs() < f32::EPSILON);
        free_obj(layer);
    }

    #[test]
    fn layer_contents_scale_default() {
        let layer = alloc_layer();
        assert!((shim_calayer_contents_scale(layer, 0) - 1.0).abs() < f64::EPSILON);
        free_obj(layer);
    }

    // --- CATransaction ---

    #[test]
    fn transaction_stubs_dont_crash() {
        shim_catransaction_begin(0, 0);
        shim_catransaction_commit(0, 0);
        shim_catransaction_flush(0, 0);
        shim_catransaction_set_disable_actions(0, 0, 1);
        assert_eq!(shim_catransaction_disable_actions(0, 0), 0);
        let dur = shim_catransaction_animation_duration(0, 0);
        assert!((dur - 0.25).abs() < f64::EPSILON);
    }

    // --- CADisplayLink ---

    #[test]
    fn displaylink_create_has_isa() {
        let link = shim_cadisplaylink_create(0, 0, 0xABC, 0xDEF);
        assert_ne!(link, 0);
        let isa = unsafe { *(link as *const u64) };
        assert_eq!(isa, crate::nsobj::cadisplaylink_class_addr());
        let target = unsafe { *((link as usize + DISPLAYLINK_TARGET_OFF) as *const u64) };
        assert_eq!(target, 0xABC);
        free_obj(link);
    }

    #[test]
    fn displaylink_stubs_dont_crash() {
        shim_cadisplaylink_add_to_run_loop(0, 0, 0, 0);
        shim_cadisplaylink_invalidate(0, 0);
        assert_eq!(shim_cadisplaylink_is_paused(0, 0), 1);
        let dur = shim_cadisplaylink_duration(0, 0);
        assert!((dur - 1.0 / 60.0).abs() < f64::EPSILON);
    }

    // --- CAMediaTimingFunction ---

    #[test]
    fn timing_function_returns_sentinel() {
        let result = shim_camediatimingfunction_with_name(0, 0, 0);
        assert_ne!(result, 0);
        // calling again returns same singleton
        let result2 = shim_camediatimingfunction_with_name(0, 0, 0);
        assert_eq!(result, result2);
    }
}
