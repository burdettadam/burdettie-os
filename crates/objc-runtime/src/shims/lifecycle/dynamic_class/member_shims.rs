use std::ffi::{c_char, CStr};

use crate::class::{IvarEntry, PropertyEntry, OBJC_RUNTIME};

use super::{StagedClassInit, STAGED_CLASSES};

// ── class_addIvar / class_addProperty ─────────────────────────────

/// `BOOL class_addIvar(Class cls, const char *name, size_t size,
///     uint8_t alignment, const char *types)`
///
/// Adds an instance variable to an unregistered class. Cannot be called
/// on classes that have already been registered.
pub extern "C" fn shim_class_add_ivar(
    cls: u64,
    name: *const c_char,
    size: u64,
    _alignment: u8,
    types: *const c_char,
) -> u8 {
    if cls == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }
    let type_str = if types.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(types) }.to_str().unwrap_or("")
    };

    let mut staged = STAGED_CLASSES.lock().unwrap_or_else(|e| e.into_inner());

    let Some(pair) = staged.get_mut(&cls) else {
        // Not staged — refuse (Apple semantics: can't add ivars after registration)
        return 0;
    };

    // Duplicate name check
    if pair.class.ivars.iter().any(|iv| iv.name == name_str) {
        return 0;
    }

    let offset = pair.class.instance_size;
    let ivar_size = size as u32;

    pair.class.ivars.push(IvarEntry {
        name: name_str.to_string(),
        type_encoding: type_str.to_string(),
        offset,
        size: ivar_size,
    });
    pair.class.instance_size += ivar_size;

    1
}

/// `BOOL class_addProperty(Class cls, const char *name,
///     const objc_property_attribute_t *attributes, unsigned int attributeCount)`
///
/// Adds a property to a class. Works on both staged and registered classes.
pub extern "C" fn shim_class_add_property(
    cls: u64,
    name: *const c_char,
    _attributes: u64,
    _attribute_count: u32,
) -> u8 {
    if cls == 0 || name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    // Try staged classes first.
    {
        let mut staged = STAGED_CLASSES.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pair) = staged.get_mut(&cls) {
            if pair.class.properties.iter().any(|p| p.name == name_str) {
                return 0;
            }
            pair.class.properties.push(PropertyEntry {
                name: name_str.to_string(),
                attributes: String::new(), // TODO: parse attribute structs if needed
            });
            return 1;
        }
    }

    // Fall back to registered classes.
    let mut guard = match OBJC_RUNTIME.write() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };
    let Some(class) = rt.get_class_by_addr_mut(cls) else {
        return 0;
    };

    if class.properties.iter().any(|p| p.name == name_str) {
        return 0;
    }
    class.properties.push(PropertyEntry {
        name: name_str.to_string(),
        attributes: String::new(),
    });
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::{ObjcRuntime, OBJC_RUNTIME};
    use crate::shims::lifecycle::dynamic_class::shim_objc_allocate_class_pair;
    use std::ffi::CString;
    use std::ptr;

    /// Ensure the runtime is initialized before allocating class pairs.
    fn ensure_runtime() {
        let mut g = OBJC_RUNTIME.write().unwrap();
        g.get_or_insert_with(ObjcRuntime::new);
    }

    #[test]
    fn add_ivar_null_cls() {
        let name = CString::new("x").unwrap();
        assert_eq!(shim_class_add_ivar(0, name.as_ptr(), 8, 3, ptr::null()), 0);
    }

    #[test]
    fn add_ivar_null_name() {
        assert_eq!(
            shim_class_add_ivar(0x1234, ptr::null(), 8, 3, ptr::null()),
            0
        );
    }

    #[test]
    fn add_ivar_to_staged() {
        ensure_runtime();
        let cls_name = CString::new("MemberTestIvarA").unwrap();
        let cls = shim_objc_allocate_class_pair(0, cls_name.as_ptr(), 0);
        assert_ne!(cls, 0, "allocateClassPair must succeed");

        let ivar_name = CString::new("_value").unwrap();
        let types = CString::new("i").unwrap();
        let result = shim_class_add_ivar(cls, ivar_name.as_ptr(), 4, 2, types.as_ptr());
        assert_eq!(result, 1);
    }

    #[test]
    fn add_ivar_duplicate() {
        ensure_runtime();
        let cls_name = CString::new("MemberTestIvarB").unwrap();
        let cls = shim_objc_allocate_class_pair(0, cls_name.as_ptr(), 0);
        assert_ne!(cls, 0);

        let ivar_name = CString::new("_dup").unwrap();
        let types = CString::new("i").unwrap();
        assert_eq!(
            shim_class_add_ivar(cls, ivar_name.as_ptr(), 4, 2, types.as_ptr()),
            1
        );
        // Second add with the same name must fail.
        assert_eq!(
            shim_class_add_ivar(cls, ivar_name.as_ptr(), 4, 2, types.as_ptr()),
            0
        );
    }

    #[test]
    fn add_property_null_cls() {
        let name = CString::new("prop").unwrap();
        assert_eq!(shim_class_add_property(0, name.as_ptr(), 0, 0), 0);
    }

    #[test]
    fn add_property_null_name() {
        assert_eq!(shim_class_add_property(0x1234, ptr::null(), 0, 0), 0);
    }
}
