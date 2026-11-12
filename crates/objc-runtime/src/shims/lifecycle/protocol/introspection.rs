//! Protocol method description and property introspection shims.

use std::ffi::{c_char, CStr, CString};

use crate::class::{MethodEntry, OBJC_RUNTIME};

use super::helpers::sentinel_to_protocol_name;

// ── Method description introspection ──────────────────────────────

/// Fake `objc_method_description` struct layout (16 bytes):
///   offset 0: SEL  name  (u64 — pointer to selector C string)
///   offset 8: char *types (u64 — pointer to type encoding C string)
const METHOD_DESC_SIZE: usize = 16;

fn allocate_method_desc(selector: &str, types: &str) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, METHOD_DESC_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }
    let sel_cstr = CString::new(selector).unwrap_or_default();
    let type_cstr = CString::new(types).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = sel_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = type_cstr.into_raw() as u64;
    }
    ptr
}

/// `struct objc_method_description *protocol_copyMethodDescriptionList(
///     Protocol *proto, BOOL isRequiredMethod, BOOL isInstanceMethod,
///     unsigned int *outCount)`
pub extern "C" fn shim_protocol_copy_method_description_list(
    proto: u64,
    is_required: u8,
    is_instance: u8,
    out_count: *mut u32,
) -> u64 {
    if proto == 0 {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let Some(proto_name) = sentinel_to_protocol_name(rt, proto) else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };
    let Some(info) = rt.get_protocol(&proto_name) else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let methods: &[MethodEntry] = match (is_required != 0, is_instance != 0) {
        (true, true) => &info.instance_methods,
        (true, false) => &info.class_methods,
        (false, true) => &info.optional_instance_methods,
        (false, false) => &info.optional_class_methods,
    };

    let count = methods.len();
    if !out_count.is_null() {
        unsafe { *out_count = count as u32 };
    }
    if count == 0 {
        return 0;
    }

    // Allocate array of objc_method_description structs (inline, not pointers)
    let total_size = count * METHOD_DESC_SIZE;
    let array_ptr = crate::pal_alloc().calloc(1, total_size) as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, m) in methods.iter().enumerate() {
        let sel_cstr = CString::new(m.selector.as_ref()).unwrap_or_default();
        let type_cstr = CString::new(m.type_encoding.as_ref()).unwrap_or_default();
        let base = array_ptr as usize + i * METHOD_DESC_SIZE;
        unsafe {
            *(base as *mut u64) = sel_cstr.into_raw() as u64;
            *((base + 8) as *mut u64) = type_cstr.into_raw() as u64;
        }
    }

    array_ptr
}

/// `struct objc_method_description protocol_getMethodDescription(
///     Protocol *proto, SEL aSel, BOOL isRequiredMethod, BOOL isInstanceMethod)`
///
/// Returns a struct (16 bytes) by value. We return a pointer to a heap-allocated
/// struct and let the caller read it as two u64 fields.
pub extern "C" fn shim_protocol_get_method_description(
    proto: u64,
    sel: u64,
    is_required: u8,
    is_instance: u8,
) -> u64 {
    if proto == 0 || sel == 0 {
        return allocate_method_desc("", "");
    }

    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        return allocate_method_desc("", "");
    };

    let Some(proto_name) = sentinel_to_protocol_name(rt, proto) else {
        return allocate_method_desc("", "");
    };
    let Some(info) = rt.get_protocol(&proto_name) else {
        return allocate_method_desc("", "");
    };

    let methods: &[MethodEntry] = match (is_required != 0, is_instance != 0) {
        (true, true) => &info.instance_methods,
        (true, false) => &info.class_methods,
        (false, true) => &info.optional_instance_methods,
        (false, false) => &info.optional_class_methods,
    };

    if let Some(m) = methods.iter().find(|m| m.selector == sel_str) {
        allocate_method_desc(&m.selector, &m.type_encoding)
    } else {
        allocate_method_desc("", "")
    }
}

// ── Protocol property introspection ───────────────────────────────

/// Fake Property struct layout (16 bytes) — same as ivar_property.rs.
const PROPERTY_STRUCT_SIZE: usize = 16;

fn allocate_fake_property(name: &str, attributes: &str) -> u64 {
    let ptr = crate::pal_alloc().calloc(1, PROPERTY_STRUCT_SIZE) as u64;
    if ptr == 0 {
        return 0;
    }
    let name_cstr = CString::new(name).unwrap_or_default();
    let attr_cstr = CString::new(attributes).unwrap_or_default();
    unsafe {
        *(ptr as *mut u64) = name_cstr.into_raw() as u64;
        *((ptr as usize + 8) as *mut u64) = attr_cstr.into_raw() as u64;
    }
    ptr
}

/// `objc_property_t *protocol_copyPropertyList(Protocol *proto, unsigned int *outCount)`
pub extern "C" fn shim_protocol_copy_property_list(proto: u64, out_count: *mut u32) -> u64 {
    if proto == 0 {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let Some(proto_name) = sentinel_to_protocol_name(rt, proto) else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };
    let Some(info) = rt.get_protocol(&proto_name) else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let count = info.properties.len();
    if !out_count.is_null() {
        unsafe { *out_count = count as u32 };
    }
    if count == 0 {
        return 0;
    }

    let array_size = (count + 1) * std::mem::size_of::<u64>();
    let array_ptr = crate::pal_alloc().calloc(1, array_size) as u64;
    if array_ptr == 0 {
        return 0;
    }

    for (i, prop) in info.properties.iter().enumerate() {
        let fake = allocate_fake_property(&prop.name, &prop.attributes);
        unsafe {
            *((array_ptr as usize + i * 8) as *mut u64) = fake;
        }
    }
    array_ptr
}

/// `objc_property_t protocol_getProperty(Protocol *proto, const char *name,
///     BOOL isRequiredProperty, BOOL isInstanceProperty)`
pub extern "C" fn shim_protocol_get_property(
    proto: u64,
    name: *const c_char,
    _is_required: u8,
    _is_instance: u8,
) -> u64 {
    if proto == 0 || name.is_null() {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let Some(proto_name) = sentinel_to_protocol_name(rt, proto) else {
        return 0;
    };
    let Some(info) = rt.get_protocol(&proto_name) else {
        return 0;
    };

    info.properties
        .iter()
        .find(|p| p.name == name_str)
        .map(|p| allocate_fake_property(&p.name, &p.attributes))
        .unwrap_or(0)
}
