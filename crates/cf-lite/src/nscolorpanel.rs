//! NSColorPanel / NSFontPanel stubs — prevent crashes when apps
//! look up `+[NSColorPanel sharedColorPanel]` or similar.
//!
//! These stub classes return a singleton or nil for most methods,
//! since the panels are not visually rendered.

use crate::nsobj;

// ── NSColorPanel ───────────────────────────────────────────────

static SHARED_COLOR_PANEL: std::sync::Mutex<u64> = std::sync::Mutex::new(0);

/// `+[NSColorPanel sharedColorPanel]`
pub extern "C" fn shim_nscolorpanel_shared(_cls: u64, _cmd: u64) -> u64 {
    let mut guard = SHARED_COLOR_PANEL.lock().unwrap();
    if *guard == 0 {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nscolorpanel_class_addr();
            }
        }
        *guard = obj;
    }
    *guard
}

/// `+[NSColorPanel sharedColorPanelExists]`
pub extern "C" fn shim_nscolorpanel_shared_exists(_cls: u64, _cmd: u64) -> u8 {
    let guard = SHARED_COLOR_PANEL.lock().unwrap();
    u8::from(*guard != 0)
}

/// `-[NSColorPanel orderFront:]`
pub extern "C" fn shim_nscolorpanel_order_front(_obj: u64, _cmd: u64, _sender: u64) {
    // No-op — we don't show the panel visually.
}

/// `-[NSColorPanel close]`
pub extern "C" fn shim_nscolorpanel_close(_obj: u64, _cmd: u64) {
    // No-op.
}

/// `-[NSColorPanel color]` — returns black.
pub extern "C" fn shim_nscolorpanel_color(_obj: u64, _cmd: u64) -> u64 {
    crate::nscolor::shim_nscolor_black_color(0, 0)
}

// ── NSFontPanel ────────────────────────────────────────────────

static SHARED_FONT_PANEL: std::sync::Mutex<u64> = std::sync::Mutex::new(0);

/// `+[NSFontPanel sharedFontPanel]`
pub extern "C" fn shim_nsfontpanel_shared(_cls: u64, _cmd: u64) -> u64 {
    let mut guard = SHARED_FONT_PANEL.lock().unwrap();
    if *guard == 0 {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = nsobj::nsfontpanel_class_addr();
            }
        }
        *guard = obj;
    }
    *guard
}

/// `+[NSFontPanel sharedFontPanelExists]`
pub extern "C" fn shim_nsfontpanel_shared_exists(_cls: u64, _cmd: u64) -> u8 {
    let guard = SHARED_FONT_PANEL.lock().unwrap();
    u8::from(*guard != 0)
}

/// `-[NSFontPanel orderFront:]`
pub extern "C" fn shim_nsfontpanel_order_front(_obj: u64, _cmd: u64, _sender: u64) {
    // No-op.
}

/// `-[NSFontPanel close]`
pub extern "C" fn shim_nsfontpanel_close(_obj: u64, _cmd: u64) {
    // No-op.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_panel_shared_returns_nonzero() {
        let panel = shim_nscolorpanel_shared(0, 0);
        assert_ne!(panel, 0);
        assert_eq!(shim_nscolorpanel_shared(0, 0), panel); // singleton
    }

    #[test]
    fn font_panel_shared_returns_nonzero() {
        let panel = shim_nsfontpanel_shared(0, 0);
        assert_ne!(panel, 0);
        assert_eq!(shim_nsfontpanel_shared(0, 0), panel); // singleton
    }

    #[test]
    fn nil_safety() {
        shim_nscolorpanel_order_front(0, 0, 0);
        shim_nscolorpanel_close(0, 0);
        shim_nsfontpanel_order_front(0, 0, 0);
        shim_nsfontpanel_close(0, 0);
    }
}
