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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // --- shim_nsappearance_current_appearance ---

    #[test]
    fn current_appearance_returns_nil() {
        assert_eq!(shim_nsappearance_current_appearance(0, 0), 0);
    }

    #[test]
    fn current_appearance_with_class_ptr() {
        // Even with a non-zero class pointer, still returns nil.
        assert_eq!(shim_nsappearance_current_appearance(0x1234, 0), 0);
    }

    // --- shim_nsappearance_current_drawing_appearance ---

    #[test]
    fn current_drawing_appearance_returns_nil() {
        assert_eq!(shim_nsappearance_current_drawing_appearance(0, 0), 0);
    }

    #[test]
    fn current_drawing_appearance_with_class_ptr() {
        assert_eq!(shim_nsappearance_current_drawing_appearance(0x1234, 0), 0);
    }

    // --- shim_nsappearance_appearance_named ---

    #[test]
    fn appearance_named_returns_nil() {
        assert_eq!(shim_nsappearance_appearance_named(0, 0, 0), 0);
    }

    #[test]
    fn appearance_named_with_name_returns_nil() {
        let name = make_string("NSAppearanceNameDarkAqua");
        assert_eq!(shim_nsappearance_appearance_named(0, 0, name), 0);
        free_obj(name);
    }

    // --- shim_nsappearance_name ---

    #[test]
    fn name_nil_returns_zero() {
        assert_eq!(shim_nsappearance_name(0, 0), 0);
    }

    #[test]
    fn name_returns_aqua_string() {
        // NSAppearance has no documented inline layout in this stub,
        // so use a fake object — the shim only checks obj != 0.
        let obj = alloc_obj_fake(8);
        let result = shim_nsappearance_name(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "NSAppearanceNameAqua");
        free_obj(result);
        free_obj(obj);
    }

    #[test]
    fn name_returns_fresh_string_each_call() {
        let obj = alloc_obj_fake(8);
        let r1 = shim_nsappearance_name(obj, 0);
        let r2 = shim_nsappearance_name(obj, 0);
        // Each call allocates a new string.
        assert_ne!(r1, r2);
        assert_string_eq(r1, "NSAppearanceNameAqua");
        assert_string_eq(r2, "NSAppearanceNameAqua");
        free_obj(r1);
        free_obj(r2);
        free_obj(obj);
    }

    // --- shim_nsappearance_best_match ---

    #[test]
    fn best_match_nil_obj_returns_nil() {
        assert_eq!(shim_nsappearance_best_match(0, 0, 0), 0);
    }

    #[test]
    fn best_match_with_obj_returns_nil() {
        let obj = alloc_obj_fake(8);
        assert_eq!(shim_nsappearance_best_match(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn best_match_with_names_returns_nil() {
        let obj = alloc_obj_fake(8);
        let name = make_string("NSAppearanceNameAqua");
        assert_eq!(shim_nsappearance_best_match(obj, 0, name), 0);
        free_obj(name);
        free_obj(obj);
    }
}
