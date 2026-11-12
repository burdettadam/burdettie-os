//! NSColor shim: color factory stubs for AppKit bootstrap.
//!
//! Object layout:
//!
//!   offset  0: isa  (u64)
//!   offset  8: r    (f64)
//!   offset 16: g    (f64)
//!   offset 24: b    (f64)
//!   offset 32: a    (f64)
//!
//! Total: 40 bytes.

fn alloc_color(r: f64, g: f64, b: f64, a: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nscolor_class_addr();
        *((obj as usize + 8) as *mut f64) = r;
        *((obj as usize + 16) as *mut f64) = g;
        *((obj as usize + 24) as *mut f64) = b;
        *((obj as usize + 32) as *mut f64) = a;
    }
    obj
}

/// `+[NSColor whiteColor]`
pub extern "C" fn shim_nscolor_white_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 1.0, 1.0, 1.0)
}

/// `+[NSColor blackColor]`
pub extern "C" fn shim_nscolor_black_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}

/// `+[NSColor redColor]`
pub extern "C" fn shim_nscolor_red_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 0.0, 0.0, 1.0)
}

/// `+[NSColor greenColor]`
pub extern "C" fn shim_nscolor_green_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 1.0, 0.0, 1.0)
}

/// `+[NSColor blueColor]`
pub extern "C" fn shim_nscolor_blue_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 1.0, 1.0)
}

/// `+[NSColor clearColor]`
pub extern "C" fn shim_nscolor_clear_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 0.0)
}

/// `+[NSColor grayColor]`
pub extern "C" fn shim_nscolor_gray_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.5, 0.5, 0.5, 1.0)
}

/// `+[NSColor colorWithRed:green:blue:alpha:]`
pub extern "C" fn shim_nscolor_with_rgba(
    _cls: u64,
    _cmd: u64,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) -> u64 {
    alloc_color(r, g, b, a)
}

/// `-[NSColor redComponent]`
pub extern "C" fn shim_nscolor_red_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 8) as *const f64) }
}

/// `-[NSColor greenComponent]`
pub extern "C" fn shim_nscolor_green_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 16) as *const f64) }
}

/// `-[NSColor blueComponent]`
pub extern "C" fn shim_nscolor_blue_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 24) as *const f64) }
}

/// `-[NSColor alphaComponent]`
pub extern "C" fn shim_nscolor_alpha_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 32) as *const f64) }
}

// ── System / semantic colors (AppKit Dark Mode–aware) ──────────

/// `+[NSColor controlColor]`
pub extern "C" fn shim_nscolor_control_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 1.0, 1.0, 1.0)
}
/// `+[NSColor controlBackgroundColor]`
pub extern "C" fn shim_nscolor_control_background_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 1.0, 1.0, 1.0)
}
/// `+[NSColor controlTextColor]`
pub extern "C" fn shim_nscolor_control_text_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}
/// `+[NSColor disabledControlTextColor]`
pub extern "C" fn shim_nscolor_disabled_control_text_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.5, 0.5, 0.5, 1.0)
}
/// `+[NSColor selectedControlColor]`
pub extern "C" fn shim_nscolor_selected_control_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.48, 1.0, 1.0)
}
/// `+[NSColor selectedTextBackgroundColor]`
pub extern "C" fn shim_nscolor_selected_text_background_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.71, 0.84, 1.0, 1.0)
}
/// `+[NSColor selectedTextColor]`
pub extern "C" fn shim_nscolor_selected_text_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}
/// `+[NSColor textColor]`
pub extern "C" fn shim_nscolor_text_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}
/// `+[NSColor textBackgroundColor]`
pub extern "C" fn shim_nscolor_text_background_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 1.0, 1.0, 1.0)
}
/// `+[NSColor windowBackgroundColor]`
pub extern "C" fn shim_nscolor_window_background_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.93, 0.93, 0.93, 1.0)
}
/// `+[NSColor windowFrameTextColor]`
pub extern "C" fn shim_nscolor_window_frame_text_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}
/// `+[NSColor labelColor]`
pub extern "C" fn shim_nscolor_label_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}
/// `+[NSColor secondaryLabelColor]`
pub extern "C" fn shim_nscolor_secondary_label_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.24, 0.24, 0.26, 0.6)
}
/// `+[NSColor tertiaryLabelColor]`
pub extern "C" fn shim_nscolor_tertiary_label_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.24, 0.24, 0.26, 0.3)
}
/// `+[NSColor quaternaryLabelColor]`
pub extern "C" fn shim_nscolor_quaternary_label_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.24, 0.24, 0.26, 0.18)
}
/// `+[NSColor separatorColor]`
pub extern "C" fn shim_nscolor_separator_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 0.1)
}
/// `+[NSColor gridColor]`
pub extern "C" fn shim_nscolor_grid_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.85, 0.85, 0.85, 1.0)
}
/// `+[NSColor highlightColor]`
pub extern "C" fn shim_nscolor_highlight_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 1.0, 1.0, 1.0)
}
/// `+[NSColor shadowColor]`
pub extern "C" fn shim_nscolor_shadow_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.0, 0.0, 1.0)
}

// ── System tint colors (macOS 10.10+) ──────────────────────────

/// `+[NSColor systemRedColor]`
pub extern "C" fn shim_nscolor_system_red_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 0.23, 0.19, 1.0)
}
/// `+[NSColor systemGreenColor]`
pub extern "C" fn shim_nscolor_system_green_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.20, 0.78, 0.35, 1.0)
}
/// `+[NSColor systemBlueColor]`
pub extern "C" fn shim_nscolor_system_blue_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.48, 1.0, 1.0)
}
/// `+[NSColor systemOrangeColor]`
pub extern "C" fn shim_nscolor_system_orange_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 0.58, 0.0, 1.0)
}
/// `+[NSColor systemYellowColor]`
pub extern "C" fn shim_nscolor_system_yellow_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 0.80, 0.0, 1.0)
}
/// `+[NSColor systemPinkColor]`
pub extern "C" fn shim_nscolor_system_pink_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(1.0, 0.18, 0.33, 1.0)
}
/// `+[NSColor systemPurpleColor]`
pub extern "C" fn shim_nscolor_system_purple_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.69, 0.32, 0.87, 1.0)
}
/// `+[NSColor systemTealColor]`
pub extern "C" fn shim_nscolor_system_teal_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.35, 0.78, 0.98, 1.0)
}
/// `+[NSColor systemIndigoColor]`
pub extern "C" fn shim_nscolor_system_indigo_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.35, 0.34, 0.84, 1.0)
}
/// `+[NSColor systemBrownColor]`
pub extern "C" fn shim_nscolor_system_brown_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.64, 0.52, 0.37, 1.0)
}
/// `+[NSColor systemMintColor]`
pub extern "C" fn shim_nscolor_system_mint_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.0, 0.78, 0.75, 1.0)
}
/// `+[NSColor systemCyanColor]`
pub extern "C" fn shim_nscolor_system_cyan_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.33, 0.67, 0.93, 1.0)
}
/// `+[NSColor systemGrayColor]`
pub extern "C" fn shim_nscolor_system_gray_color(_cls: u64, _cmd: u64) -> u64 {
    alloc_color(0.56, 0.56, 0.58, 1.0)
}

/// `+[NSColor colorWithWhite:alpha:]`
pub extern "C" fn shim_nscolor_with_white_alpha(_cls: u64, _cmd: u64, w: f64, a: f64) -> u64 {
    alloc_color(w, w, w, a)
}

/// `+[NSColor colorWithHue:saturation:brightness:alpha:]`
pub extern "C" fn shim_nscolor_with_hsba(
    _cls: u64,
    _cmd: u64,
    h: f64,
    s: f64,
    b: f64,
    a: f64,
) -> u64 {
    let (r, g, bl) = hsb_to_rgb(h, s, b);
    alloc_color(r, g, bl, a)
}

// ── HSB instance methods ───────────────────────────────────────

/// `-[NSColor hueComponent]`
pub extern "C" fn shim_nscolor_hue_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let r = unsafe { *((obj as usize + 8) as *const f64) };
    let g = unsafe { *((obj as usize + 16) as *const f64) };
    let b = unsafe { *((obj as usize + 24) as *const f64) };
    let (h, _, _) = rgb_to_hsb(r, g, b);
    h
}

/// `-[NSColor saturationComponent]`
pub extern "C" fn shim_nscolor_saturation_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let r = unsafe { *((obj as usize + 8) as *const f64) };
    let g = unsafe { *((obj as usize + 16) as *const f64) };
    let b = unsafe { *((obj as usize + 24) as *const f64) };
    let (_, s, _) = rgb_to_hsb(r, g, b);
    s
}

/// `-[NSColor brightnessComponent]`
pub extern "C" fn shim_nscolor_brightness_component(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let r = unsafe { *((obj as usize + 8) as *const f64) };
    let g = unsafe { *((obj as usize + 16) as *const f64) };
    let b = unsafe { *((obj as usize + 24) as *const f64) };
    let (_, _, v) = rgb_to_hsb(r, g, b);
    v
}

/// `-[NSColor colorUsingColorSpace:]` — returns self (all colors stored in device RGB).
pub extern "C" fn shim_nscolor_color_using_color_space(obj: u64, _cmd: u64, _cs: u64) -> u64 {
    obj
}

/// `-[NSColor getRed:green:blue:alpha:]` — writes components into out-pointers.
pub extern "C" fn shim_nscolor_get_rgba(
    obj: u64,
    _cmd: u64,
    r: *mut f64,
    g: *mut f64,
    b: *mut f64,
    a: *mut f64,
) {
    if obj == 0 {
        return;
    }
    unsafe {
        if !r.is_null() {
            *r = *((obj as usize + 8) as *const f64);
        }
        if !g.is_null() {
            *g = *((obj as usize + 16) as *const f64);
        }
        if !b.is_null() {
            *b = *((obj as usize + 24) as *const f64);
        }
        if !a.is_null() {
            *a = *((obj as usize + 32) as *const f64);
        }
    }
}

// ── Color conversion helpers ───────────────────────────────────

fn rgb_to_hsb(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let brightness = max;
    if delta < 1e-10 {
        return (0.0, 0.0, brightness);
    }
    let saturation = delta / max;
    let hue = if (max - r).abs() < 1e-10 {
        (g - b) / delta + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-10 {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    } / 6.0;
    (hue, saturation, brightness)
}

// ── Drawing context integration ────────────────────────────────

fn color_components(obj: u64) -> (f64, f64, f64, f64) {
    if obj == 0 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    unsafe {
        (
            *((obj as usize + 8) as *const f64),
            *((obj as usize + 16) as *const f64),
            *((obj as usize + 24) as *const f64),
            *((obj as usize + 32) as *const f64),
        )
    }
}

/// `-[NSColor set]` — set both fill and stroke to this color in the current context.
pub extern "C" fn shim_nscolor_set(obj: u64, _cmd: u64) {
    shim_nscolor_set_fill(obj, _cmd);
    shim_nscolor_set_stroke(obj, _cmd);
}

/// `-[NSColor setFill]` — set fill color in the current CGContext.
pub extern "C" fn shim_nscolor_set_fill(obj: u64, _cmd: u64) {
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 || obj == 0 {
        return;
    }
    let (r, g, b, a) = color_components(obj);
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.fill_color = graphics_core::Color::new(r, g, b, a);
    });
}

/// `-[NSColor setStroke]` — set stroke color in the current CGContext.
pub extern "C" fn shim_nscolor_set_stroke(obj: u64, _cmd: u64) {
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 || obj == 0 {
        return;
    }
    let (r, g, b, a) = color_components(obj);
    cg_shims::bridge::with_context(cg_ctx, |c| {
        c.stroke_color = graphics_core::Color::new(r, g, b, a);
    });
}

fn hsb_to_rgb(h: f64, s: f64, b: f64) -> (f64, f64, f64) {
    if s < 1e-10 {
        return (b, b, b);
    }
    let h = (h - h.floor()) * 6.0;
    let i = h.floor() as u32;
    let f = h - h.floor();
    let p = b * (1.0 - s);
    let q = b * (1.0 - s * f);
    let t = b * (1.0 - s * (1.0 - f));
    match i % 6 {
        0 => (b, t, p),
        1 => (q, b, p),
        2 => (p, b, t),
        3 => (p, q, b),
        4 => (t, p, b),
        _ => (b, p, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = 40;

    fn read_f64(obj: u64, offset: usize) -> f64 {
        unsafe { *((obj as usize + offset) as *const f64) }
    }

    fn assert_color(obj: u64, r: f64, g: f64, b: f64, a: f64) {
        assert_ne!(obj, 0, "expected non-null color");
        assert_eq!(read_f64(obj, 8), r, "red mismatch");
        assert_eq!(read_f64(obj, 16), g, "green mismatch");
        assert_eq!(read_f64(obj, 24), b, "blue mismatch");
        assert_eq!(read_f64(obj, 32), a, "alpha mismatch");
    }

    // --- factory class methods ---

    #[test]
    fn white_color() {
        let c = shim_nscolor_white_color(0, 0);
        assert_color(c, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(read_field(c, 0), crate::nsobj::nscolor_class_addr());
        free_obj(c);
    }

    #[test]
    fn black_color() {
        let c = shim_nscolor_black_color(0, 0);
        assert_color(c, 0.0, 0.0, 0.0, 1.0);
        free_obj(c);
    }

    #[test]
    fn red_color() {
        let c = shim_nscolor_red_color(0, 0);
        assert_color(c, 1.0, 0.0, 0.0, 1.0);
        free_obj(c);
    }

    #[test]
    fn green_color() {
        let c = shim_nscolor_green_color(0, 0);
        assert_color(c, 0.0, 1.0, 0.0, 1.0);
        free_obj(c);
    }

    #[test]
    fn blue_color() {
        let c = shim_nscolor_blue_color(0, 0);
        assert_color(c, 0.0, 0.0, 1.0, 1.0);
        free_obj(c);
    }

    #[test]
    fn clear_color() {
        let c = shim_nscolor_clear_color(0, 0);
        assert_color(c, 0.0, 0.0, 0.0, 0.0);
        free_obj(c);
    }

    #[test]
    fn gray_color() {
        let c = shim_nscolor_gray_color(0, 0);
        assert_color(c, 0.5, 0.5, 0.5, 1.0);
        free_obj(c);
    }

    // --- colorWithRed:green:blue:alpha: ---

    #[test]
    fn with_rgba_basic() {
        let c = shim_nscolor_with_rgba(0, 0, 0.2, 0.4, 0.6, 0.8);
        assert_color(c, 0.2, 0.4, 0.6, 0.8);
        free_obj(c);
    }

    #[test]
    fn with_rgba_zeros() {
        let c = shim_nscolor_with_rgba(0, 0, 0.0, 0.0, 0.0, 0.0);
        assert_color(c, 0.0, 0.0, 0.0, 0.0);
        free_obj(c);
    }

    #[test]
    fn with_rgba_ones() {
        let c = shim_nscolor_with_rgba(0, 0, 1.0, 1.0, 1.0, 1.0);
        assert_color(c, 1.0, 1.0, 1.0, 1.0);
        free_obj(c);
    }

    // --- component getters ---

    #[test]
    fn red_component() {
        let c = alloc_color(0.25, 0.5, 0.75, 1.0);
        assert_eq!(shim_nscolor_red_component(c, 0), 0.25);
        free_obj(c);
    }

    #[test]
    fn green_component() {
        let c = alloc_color(0.25, 0.5, 0.75, 1.0);
        assert_eq!(shim_nscolor_green_component(c, 0), 0.5);
        free_obj(c);
    }

    #[test]
    fn blue_component() {
        let c = alloc_color(0.25, 0.5, 0.75, 1.0);
        assert_eq!(shim_nscolor_blue_component(c, 0), 0.75);
        free_obj(c);
    }

    #[test]
    fn alpha_component() {
        let c = alloc_color(0.25, 0.5, 0.75, 0.9);
        assert_eq!(shim_nscolor_alpha_component(c, 0), 0.9);
        free_obj(c);
    }

    // --- nil safety for component getters ---

    #[test]
    fn red_component_nil() {
        assert_eq!(shim_nscolor_red_component(0, 0), 0.0);
    }

    #[test]
    fn green_component_nil() {
        assert_eq!(shim_nscolor_green_component(0, 0), 0.0);
    }

    #[test]
    fn blue_component_nil() {
        assert_eq!(shim_nscolor_blue_component(0, 0), 0.0);
    }

    #[test]
    fn alpha_component_nil() {
        assert_eq!(shim_nscolor_alpha_component(0, 0), 0.0);
    }

    // --- ISA is correct for all factory methods ---

    #[test]
    fn factory_methods_set_correct_isa() {
        let isa = crate::nsobj::nscolor_class_addr();
        let colors = [
            shim_nscolor_black_color(0, 0),
            shim_nscolor_red_color(0, 0),
            shim_nscolor_green_color(0, 0),
            shim_nscolor_blue_color(0, 0),
            shim_nscolor_clear_color(0, 0),
            shim_nscolor_gray_color(0, 0),
            shim_nscolor_with_rgba(0, 0, 0.1, 0.2, 0.3, 0.4),
        ];
        for c in &colors {
            assert_eq!(read_field(*c, 0), isa, "ISA mismatch for color at {c:#x}");
        }
        for c in colors {
            free_obj(c);
        }
    }

    // --- round-trip: create via factory, read via component getter ---

    #[test]
    fn roundtrip_components() {
        let c = shim_nscolor_with_rgba(0, 0, 0.1, 0.2, 0.3, 0.4);
        assert_eq!(shim_nscolor_red_component(c, 0), 0.1);
        assert_eq!(shim_nscolor_green_component(c, 0), 0.2);
        assert_eq!(shim_nscolor_blue_component(c, 0), 0.3);
        assert_eq!(shim_nscolor_alpha_component(c, 0), 0.4);
        free_obj(c);
    }
}
