//! NSColorWell shim: color picker button control for AppKit.
//!
//! Object layout (64 bytes, inherits NSControl at 48):
//!
//!   offset  0: isa        (u64)
//!   offset  8: color_ptr  (u64 — NSColor*)
//!   offset 16: flags      (u64 — bit 0: enabled, bit 1: bordered)
//!
//! Total: 64 bytes.

const COLOR_OFF: usize = 8;
const FLAGS_OFF: usize = 16;
pub(crate) const COLOR_WELL_SIZE: usize = 64;

const FLAG_IS_ENABLED: u64 = 1 << 0;
const FLAG_IS_BORDERED: u64 = 1 << 1;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

fn read_flags(obj: u64) -> u64 {
    read_u64(obj, FLAGS_OFF)
}

fn write_flags(obj: u64, flags: u64) {
    write_u64(obj, FLAGS_OFF, flags);
}

fn set_flag(obj: u64, flag: u64, val: bool) {
    if obj == 0 {
        return;
    }
    let mut flags = read_flags(obj);
    if val {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_flags(obj, flags);
}

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

fn default_color() -> u64 {
    crate::nscolor::shim_nscolor_black_color(0, 0)
}

// ── Instance method shims ──────────────────────────────────────

/// `-[NSColorWell color]`
pub extern "C" fn shim_nscolorwell_color(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let color = read_u64(obj, COLOR_OFF);
    if color != 0 {
        return color;
    }
    let color = default_color();
    write_u64(obj, COLOR_OFF, color);
    color
}

/// `-[NSColorWell setColor:]`
pub extern "C" fn shim_nscolorwell_set_color(obj: u64, _cmd: u64, color: u64) {
    write_u64(obj, COLOR_OFF, color);
}

/// `-[NSColorWell takeColorFrom:]`
pub extern "C" fn shim_nscolorwell_take_color_from(obj: u64, _cmd: u64, sender: u64) {
    if obj == 0 {
        return;
    }
    let color = read_u64(sender, COLOR_OFF);
    write_u64(obj, COLOR_OFF, color);
}

/// `-[NSColorWell activate:]` — no-op stub.
pub extern "C" fn shim_nscolorwell_activate(_obj: u64, _cmd: u64, _exclusive: u8) {}

/// `-[NSColorWell isEnabled]`
pub extern "C" fn shim_nscolorwell_is_enabled(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_ENABLED)
}

/// `-[NSColorWell setEnabled:]`
pub extern "C" fn shim_nscolorwell_set_enabled(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_ENABLED, val != 0);
}

/// `-[NSColorWell isBordered]`
pub extern "C" fn shim_nscolorwell_is_bordered(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_BORDERED)
}

/// `-[NSColorWell setBordered:]`
pub extern "C" fn shim_nscolorwell_set_bordered(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_BORDERED, val != 0);
}

/// `-[NSColorWell deactivate]` — no-op stub.
pub extern "C" fn shim_nscolorwell_deactivate(_obj: u64, _cmd: u64) {}

// ── Drawing ────────────────────────────────────────────────────

/// `-[NSColorWell drawRect:]` — renders the color swatch.
pub extern "C" fn shim_nscolorwell_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let color_ptr = read_u64(obj, COLOR_OFF);
    let (r, g, b, a) = if color_ptr != 0 {
        (
            crate::nscolor::shim_nscolor_red_component(color_ptr, 0),
            crate::nscolor::shim_nscolor_green_component(color_ptr, 0),
            crate::nscolor::shim_nscolor_blue_component(color_ptr, 0),
            crate::nscolor::shim_nscolor_alpha_component(color_ptr, 0),
        )
    } else {
        (0.0, 0.0, 0.0, 1.0) // default black
    };

    let is_bordered = read_flags(obj) & FLAG_IS_BORDERED != 0;

    cg_shims::bridge::with_context(cg_ctx, |c| {
        if is_bordered {
            // Draw gray border then inset the swatch.
            c.stroke_color = graphics_core::Color::new(0.5, 0.5, 0.5, 1.0);
            c.stroke_rect(x, y, w, h);
            c.fill_color = graphics_core::Color::new(r, g, b, a);
            c.fill_rect(x + 2.0, y + 2.0, w - 4.0, h - 4.0);
        } else {
            c.fill_color = graphics_core::Color::new(r, g, b, a);
            c.fill_rect(x, y, w, h);
        }
    });
}

/// `-[NSColorWell mouseDown:]` — activates the color well.
pub extern "C" fn shim_nscolorwell_mouse_down(obj: u64, _cmd: u64, _event: u64) {
    if obj == 0 {
        return;
    }
    if read_flags(obj) & FLAG_IS_ENABLED == 0 {
        return;
    }
    // Activate the well (which would normally open the color panel).
    shim_nscolorwell_activate(obj, 0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn read_f64(obj: u64, offset: usize) -> f64 {
        unsafe { *((obj as usize + offset) as *const f64) }
    }

    fn assert_black_color(obj: u64) {
        assert_ne!(obj, 0, "expected non-null color");
        assert_eq!(read_field(obj, 0), crate::nsobj::nscolor_class_addr());
        assert_eq!(read_f64(obj, 8), 0.0);
        assert_eq!(read_f64(obj, 16), 0.0);
        assert_eq!(read_f64(obj, 24), 0.0);
        assert_eq!(read_f64(obj, 32), 1.0);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nscolorwell_color(0, 0), 0);
        shim_nscolorwell_set_color(0, 0, 0x1234);
        shim_nscolorwell_take_color_from(0, 0, 0x1234);
        shim_nscolorwell_activate(0, 0, 1);
        assert_eq!(shim_nscolorwell_is_enabled(0, 0), 0);
        shim_nscolorwell_set_enabled(0, 0, 1);
        assert_eq!(shim_nscolorwell_is_bordered(0, 0), 0);
        shim_nscolorwell_set_bordered(0, 0, 1);
        shim_nscolorwell_deactivate(0, 0);
        shim_nscolorwell_draw_rect(0, 0, 0.0, 0.0, 50.0, 50.0);
        shim_nscolorwell_mouse_down(0, 0, 0);
    }

    #[test]
    fn color_returns_default_black_when_unset() {
        let well = alloc_obj_fake(COLOR_WELL_SIZE);
        let color = shim_nscolorwell_color(well, 0);
        assert_black_color(color);
        assert_eq!(read_field(well, COLOR_OFF), color);
        free_obj(color);
        free_obj(well);
    }

    #[test]
    fn set_color_round_trip() {
        let well = alloc_obj_fake(COLOR_WELL_SIZE);
        let color = crate::nscolor::shim_nscolor_red_color(0, 0);
        shim_nscolorwell_set_color(well, 0, color);
        assert_eq!(shim_nscolorwell_color(well, 0), color);
        free_obj(color);
        free_obj(well);
    }

    #[test]
    fn take_color_from_sender_copies_color_pointer() {
        let receiver = alloc_obj_fake(COLOR_WELL_SIZE);
        let sender = alloc_obj_fake(COLOR_WELL_SIZE);
        let color = crate::nscolor::shim_nscolor_blue_color(0, 0);
        write_field(sender, COLOR_OFF, color);

        shim_nscolorwell_take_color_from(receiver, 0, sender);

        assert_eq!(shim_nscolorwell_color(receiver, 0), color);
        free_obj(color);
        free_obj(sender);
        free_obj(receiver);
    }

    #[test]
    fn enabled_flag_round_trip() {
        let well = alloc_obj_fake(COLOR_WELL_SIZE);
        assert_eq!(shim_nscolorwell_is_enabled(well, 0), 0);
        shim_nscolorwell_set_enabled(well, 0, 1);
        assert_eq!(shim_nscolorwell_is_enabled(well, 0), 1);
        shim_nscolorwell_set_enabled(well, 0, 0);
        assert_eq!(shim_nscolorwell_is_enabled(well, 0), 0);
        free_obj(well);
    }

    #[test]
    fn bordered_flag_round_trip() {
        let well = alloc_obj_fake(COLOR_WELL_SIZE);
        assert_eq!(shim_nscolorwell_is_bordered(well, 0), 0);
        shim_nscolorwell_set_bordered(well, 0, 1);
        assert_eq!(shim_nscolorwell_is_bordered(well, 0), 1);
        shim_nscolorwell_set_bordered(well, 0, 0);
        assert_eq!(shim_nscolorwell_is_bordered(well, 0), 0);
        free_obj(well);
    }

    #[test]
    fn activate_and_deactivate_are_noops() {
        let well = alloc_obj_fake(COLOR_WELL_SIZE);
        let color = crate::nscolor::shim_nscolor_green_color(0, 0);
        shim_nscolorwell_set_color(well, 0, color);
        shim_nscolorwell_set_enabled(well, 0, 1);
        shim_nscolorwell_set_bordered(well, 0, 1);

        shim_nscolorwell_activate(well, 0, 1);
        shim_nscolorwell_deactivate(well, 0);

        assert_eq!(shim_nscolorwell_color(well, 0), color);
        assert_eq!(shim_nscolorwell_is_enabled(well, 0), 1);
        assert_eq!(shim_nscolorwell_is_bordered(well, 0), 1);
        free_obj(color);
        free_obj(well);
    }
}
