//! Ivar and property introspection shims.

use std::ffi::{c_char, CStr};

use crate::class::OBJC_RUNTIME;

/// Fake Ivar struct layout (32 bytes):
///   offset  0: name_ptr          (u64 — pointer to C string)
///   offset  8: type_encoding_ptr (u64 — pointer to C string)
///   offset 16: ivar_offset       (i64 — ptrdiff_t)
///   offset 24: size              (u32)
const IVAR_STRUCT_SIZE: usize = 32;

/// Fake Property struct layout (16 bytes):
///   offset  0: name_ptr       (u64 — pointer to C string)
///   offset  8: attributes_ptr (u64 — pointer to C string)
const PROPERTY_STRUCT_SIZE: usize = 16;

fn allocate_fake_ivar(name: &str, type_enc: &str, offset: u32, size: u32) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, IVAR_STRUCT_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }

    let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
    let type_cstr = std::ffi::CString::new(type_enc).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = name_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = type_cstr.into_raw() as u64;
        *((ptr as usize + 16) as *mut i64) = offset as i64;
        *((ptr as usize + 24) as *mut u32) = size;
    }
    ptr
}

fn allocate_fake_property(name: &str, attributes: &str) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, PROPERTY_STRUCT_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }

    let name_cstr = std::ffi::CString::new(name).unwrap_or_default();
    let attr_cstr = std::ffi::CString::new(attributes).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = name_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = attr_cstr.into_raw() as u64;
    }
    ptr
}

/// `Ivar *class_copyIvarList(Class cls, unsigned int *outCount)`
pub extern "C" fn shim_class_copy_ivar_list(cls: u64, out_count: *mut u32) -> u64 {
    if cls == 0 {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let Some(class) = rt.get_class_by_addr(cls) else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let count = class.ivars.len();
    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }
    if count == 0 {
        return 0;
    }

    // Allocate NULL-terminated array of Ivar pointers
    let array_size = (count + 1) * std::mem::size_of::<u64>();
    let array_ptr = crate::pal_alloc().calloc(1, array_size) as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, ivar) in class.ivars.iter().enumerate() {
        let fake = allocate_fake_ivar(&ivar.name, &ivar.type_encoding, ivar.offset, ivar.size);
        unsafe {
            *((array_ptr as usize + i * 8) as *mut u64) = fake;
        }
    }
    array_ptr
}

/// `Ivar class_getInstanceVariable(Class cls, const char *name)`
pub extern "C" fn shim_class_get_instance_variable(cls: u64, name: u64) -> u64 {
    if cls == 0 || name == 0 {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    let Some(class) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    class
        .ivars
        .iter()
        .find(|iv| iv.name == name_str)
        .map(|iv| allocate_fake_ivar(&iv.name, &iv.type_encoding, iv.offset, iv.size))
        .unwrap_or(0)
}

/// `const char *ivar_getName(Ivar ivar)`
pub extern "C" fn shim_ivar_get_name(ivar: u64) -> u64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *(ivar as *const u64) }
}

/// `const char *ivar_getTypeEncoding(Ivar ivar)`
pub extern "C" fn shim_ivar_get_type_encoding(ivar: u64) -> u64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *((ivar as usize + 8) as *const u64) }
}

/// `ptrdiff_t ivar_getOffset(Ivar ivar)`
pub extern "C" fn shim_ivar_get_offset(ivar: u64) -> i64 {
    if ivar == 0 {
        return 0;
    }
    unsafe { *((ivar as usize + 16) as *const i64) }
}

/// `objc_property_t *class_copyPropertyList(Class cls, unsigned int *outCount)`
pub extern "C" fn shim_class_copy_property_list(cls: u64, out_count: *mut u32) -> u64 {
    if cls == 0 {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let Some(class) = rt.get_class_by_addr(cls) else {
        if !out_count.is_null() {
            unsafe {
                *out_count = 0;
            }
        }
        return 0;
    };

    let count = class.properties.len();
    if !out_count.is_null() {
        unsafe {
            *out_count = count as u32;
        }
    }
    if count == 0 {
        return 0;
    }

    let array_size = (count + 1) * std::mem::size_of::<u64>();
    let array_ptr = crate::pal_alloc().calloc(1, array_size) as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, prop) in class.properties.iter().enumerate() {
        let fake = allocate_fake_property(&prop.name, &prop.attributes);
        unsafe {
            *((array_ptr as usize + i * 8) as *mut u64) = fake;
        }
    }
    array_ptr
}

/// `objc_property_t class_getProperty(Class cls, const char *name)`
pub extern "C" fn shim_class_get_property(cls: u64, name: u64) -> u64 {
    if cls == 0 || name == 0 {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };
    let Some(class) = rt.get_class_by_addr(cls) else {
        return 0;
    };

    class
        .properties
        .iter()
        .find(|p| p.name == name_str)
        .map(|p| allocate_fake_property(&p.name, &p.attributes))
        .unwrap_or(0)
}

/// `const char *property_getName(objc_property_t property)`
pub extern "C" fn shim_property_get_name(property: u64) -> u64 {
    if property == 0 {
        return 0;
    }
    unsafe { *(property as *const u64) }
}

/// `const char *property_getAttributes(objc_property_t property)`
pub extern "C" fn shim_property_get_attributes(property: u64) -> u64 {
    if property == 0 {
        return 0;
    }
    unsafe { *((property as usize + 8) as *const u64) }
}

/// `id object_getIvar(id obj, Ivar ivar)`
///
/// Reads the pointer-sized value at the ivar's offset within the object.
pub extern "C" fn shim_object_get_ivar(obj: u64, ivar: u64) -> u64 {
    if obj == 0 || ivar == 0 {
        return 0;
    }
    let offset = unsafe { *((ivar as usize + 16) as *const i64) };
    if offset < 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset as usize) as *const u64) }
}

/// `void object_setIvar(id obj, Ivar ivar, id value)`
///
/// Writes a pointer-sized value at the ivar's offset within the object.
pub extern "C" fn shim_object_set_ivar(obj: u64, ivar: u64, value: u64) {
    if obj == 0 || ivar == 0 {
        return;
    }
    let offset = unsafe { *((ivar as usize + 16) as *const i64) };
    if offset < 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset as usize) as *mut u64) = value;
    }
}
