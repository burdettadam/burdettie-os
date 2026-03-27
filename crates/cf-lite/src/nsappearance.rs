//! NSAppearance stub — dark mode appearance queries.
//!
//! Many apps call `[NSAppearance currentAppearance]` or read appearance
//! names to detect dark mode. Our stub returns nil/Aqua for all queries.

/// `+[NSAppearance currentAppearance]` → nil (no appearance set).
pub extern "C" fn shim_nsappearance_current_appearance(_cls: u64, _cmd: u64) -> u64 {
    0
}

/// `+[NSAppearance currentDrawingAppearance]` → nil.
pub extern "C" fn shim_nsappearance_current_drawing_appearance(_cls: u64, _cmd: u64) -> u64 {
    0
}

/// `+[NSAppearance appearanceNamed:]` → returns a minimal appearance object.
pub extern "C" fn shim_nsappearance_appearance_named(_cls: u64, _cmd: u64, _name: u64) -> u64 {
    // Return nil — no custom appearances.
    0
}

/// `-[NSAppearance name]` → "NSAppearanceNameAqua" constant string.
pub extern "C" fn shim_nsappearance_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let cstr = c"NSAppearanceNameAqua";
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSAppearance bestMatchFromAppearancesWithNames:]` → first item or nil.
pub extern "C" fn shim_nsappearance_best_match(_obj: u64, _cmd: u64, _names: u64) -> u64 {
    // Return nil — let caller use default.
    0
}
