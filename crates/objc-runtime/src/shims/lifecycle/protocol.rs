//! Protocol introspection shims: objc_getProtocol, protocol_getName,
//! class_conformsToProtocol, protocol_conformsToProtocol, etc.

use std::ffi::{c_char, CStr, CString};

use crate::class::{MethodEntry, ProtocolInfo, OBJC_RUNTIME};

/// `Protocol *objc_getProtocol(const char *name)`
///
/// Returns a non-zero sentinel address if the protocol is registered,
/// or NULL if it is not found. The sentinel encodes the protocol index.
pub extern "C" fn shim_objc_get_protocol(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    if rt.get_protocol(name_str).is_some() {
        // Return a stable sentinel: hash-based fake address in high memory.
        // We use a simple FNV-like hash to generate a unique non-zero address.
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name_str.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        // Ensure non-zero and in a recognisable range
        h | 0x7000_0000_0000_0000
    } else {
        0
    }
}

/// `const char *protocol_getName(Protocol *proto)`
///
/// Since we use sentinel addresses from `objc_getProtocol`, we look up
/// the protocol by scanning for a matching sentinel.
pub extern "C" fn shim_protocol_get_name(proto: u64) -> u64 {
    if proto == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    // Find the protocol whose sentinel matches
    for (name, _) in rt.protocols_iter() {
        let sentinel = protocol_name_to_sentinel(name);
        if sentinel == proto {
            let cstr = CString::new(name.as_str()).unwrap_or_default();
            return cstr.into_raw() as u64;
        }
    }
    0
}

/// `BOOL protocol_isEqual(Protocol *proto, Protocol *other)`
pub extern "C" fn shim_protocol_is_equal(proto: u64, other: u64) -> u8 {
    if proto == other {
        1
    } else {
        0
    }
}

/// `BOOL class_conformsToProtocol(Class cls, Protocol *protocol)`
pub extern "C" fn shim_class_conforms_to_protocol(cls: u64, protocol: u64) -> u8 {
    if cls == 0 || protocol == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    // Find the protocol name from the sentinel
    let Some(proto_name) = sentinel_to_protocol_name(rt, protocol) else {
        return 0;
    };

    // Find the class name from addr
    let Some(class_info) = rt.get_class_by_addr(cls) else {
        return 0;
    };
    let class_name = class_info.name.clone();

    if rt.class_conforms_to_protocol(&class_name, &proto_name) {
        1
    } else {
        0
    }
}

/// `BOOL protocol_conformsToProtocol(Protocol *proto, Protocol *other)`
pub extern "C" fn shim_protocol_conforms_to_protocol(proto: u64, other: u64) -> u8 {
    if proto == 0 || other == 0 {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    let Some(proto_name) = sentinel_to_protocol_name(rt, proto) else {
        return 0;
    };
    let Some(other_name) = sentinel_to_protocol_name(rt, other) else {
        return 0;
    };

    if rt.protocol_conforms_to_protocol(&proto_name, &other_name) {
        1
    } else {
        0
    }
}

/// `Protocol * __unsafe_unretained *class_copyProtocolList(Class cls, unsigned int *outCount)`
pub extern "C" fn shim_class_copy_protocol_list(cls: u64, out_count: *mut u32) -> u64 {
    if cls == 0 {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let Some(class_info) = rt.get_class_by_addr(cls) else {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    };

    let protocol_names = &class_info.protocol_names;
    let count = protocol_names.len();

    if !out_count.is_null() {
        unsafe { *out_count = count as u32 };
    }
    if count == 0 {
        return 0;
    }

    // Allocate an array of Protocol* (sentinels)
    let layout = std::alloc::Layout::array::<u64>(count).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) } as *mut u64;
    if ptr.is_null() {
        return 0;
    }

    for (i, name) in protocol_names.iter().enumerate() {
        unsafe { *ptr.add(i) = protocol_name_to_sentinel(name) };
    }

    ptr as u64
}

/// `Protocol * __unsafe_unretained *protocol_copyProtocolList(Protocol *proto, unsigned int *outCount)`
pub extern "C" fn shim_protocol_copy_protocol_list(proto: u64, out_count: *mut u32) -> u64 {
    if proto == 0 {
        if !out_count.is_null() {
            unsafe { *out_count = 0 };
        }
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
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

    let adopted = &info.adopted_protocols;
    let count = adopted.len();

    if !out_count.is_null() {
        unsafe { *out_count = count as u32 };
    }
    if count == 0 {
        return 0;
    }

    let layout = std::alloc::Layout::array::<u64>(count).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) } as *mut u64;
    if ptr.is_null() {
        return 0;
    }

    for (i, name) in adopted.iter().enumerate() {
        unsafe { *ptr.add(i) = protocol_name_to_sentinel(name) };
    }

    ptr as u64
}

// ── Runtime protocol creation ──────────────────────────────────────

/// `Protocol *objc_allocateProtocol(const char *name)`
///
/// Allocates a new, empty protocol. The protocol is not yet registered;
/// call `objc_registerProtocol` after configuring it.
pub extern "C" fn shim_objc_allocate_protocol(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");
    if name_str.is_empty() {
        return 0;
    }

    let guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    // Already registered → return NULL (Apple semantics).
    if rt.get_protocol(name_str).is_some() {
        return 0;
    }
    drop(guard);

    // Stage the protocol: use the sentinel address as a handle for the
    // not-yet-registered protocol. The name is stored in a global map so
    // `objc_registerProtocol` can retrieve it.
    let sentinel = protocol_name_to_sentinel(name_str);
    STAGED_PROTOCOLS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            sentinel,
            ProtocolInfo {
                name: name_str.to_string(),
                instance_methods: Vec::new(),
                class_methods: Vec::new(),
                optional_instance_methods: Vec::new(),
                optional_class_methods: Vec::new(),
                properties: Vec::new(),
                adopted_protocols: Vec::new(),
            },
        );
    sentinel
}

/// `void objc_registerProtocol(Protocol *proto)`
///
/// Moves a previously allocated protocol into the runtime's registry.
pub extern "C" fn shim_objc_register_protocol(proto: u64) {
    if proto == 0 {
        return;
    }

    let info = STAGED_PROTOCOLS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&proto);

    if let Some(info) = info {
        let mut guard = match OBJC_RUNTIME.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(rt) = guard.as_mut() {
            rt.register_protocol(info);
        }
    }
}

/// `BOOL class_addProtocol(Class cls, Protocol *protocol)`
///
/// Adds a protocol conformance to a class. Returns YES if the protocol
/// was added, NO if the class already conforms or the args are invalid.
pub extern "C" fn shim_class_add_protocol(cls: u64, protocol: u64) -> u8 {
    if cls == 0 || protocol == 0 {
        return 0;
    }

    let mut guard = match OBJC_RUNTIME.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_mut() else { return 0 };

    let Some(proto_name) = sentinel_to_protocol_name(rt, protocol) else {
        return 0;
    };
    let Some(class) = rt.get_class_by_addr_mut(cls) else {
        return 0;
    };

    if class.protocol_names.iter().any(|p| p == &proto_name) {
        return 0; // already conforms
    }
    class.protocol_names.push(proto_name);
    1
}

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

    let guard = match OBJC_RUNTIME.lock() {
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
        let sel_cstr = CString::new(m.selector.as_str()).unwrap_or_default();
        let type_cstr = CString::new(m.type_encoding.as_str()).unwrap_or_default();
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

    let guard = match OBJC_RUNTIME.lock() {
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

    let guard = match OBJC_RUNTIME.lock() {
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

    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("");

    let guard = match OBJC_RUNTIME.lock() {
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

// ── Staged protocol storage ───────────────────────────────────────

use std::collections::HashMap;
use std::sync::Mutex;

/// Protocols that have been allocated but not yet registered.
static STAGED_PROTOCOLS: Mutex<Option<HashMap<u64, ProtocolInfo>>> = Mutex::new(None);

/// Trait to initialize-once and access staged protocols.
trait StagedInit {
    fn insert(&mut self, key: u64, val: ProtocolInfo);
    fn remove(&mut self, key: &u64) -> Option<ProtocolInfo>;
}

impl StagedInit for std::sync::MutexGuard<'_, Option<HashMap<u64, ProtocolInfo>>> {
    fn insert(&mut self, key: u64, val: ProtocolInfo) {
        self.get_or_insert_with(HashMap::new).insert(key, val);
    }
    fn remove(&mut self, key: &u64) -> Option<ProtocolInfo> {
        self.as_mut()?.remove(key)
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn protocol_name_to_sentinel(name: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h | 0x7000_0000_0000_0000
}

fn sentinel_to_protocol_name(
    rt: &crate::class::ObjcRuntime,
    sentinel: u64,
) -> Option<String> {
    for (name, _) in rt.protocols_iter() {
        if protocol_name_to_sentinel(name) == sentinel {
            return Some(name.clone());
        }
    }
    None
}
