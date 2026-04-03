//! Protocol creation shims: objc_allocateProtocol, objc_registerProtocol, class_addProtocol.

use std::ffi::{c_char, CStr};

use crate::class::{ProtocolInfo, OBJC_RUNTIME};

use super::helpers::{protocol_name_to_sentinel, sentinel_to_protocol_name,
    STAGED_PROTOCOLS, StagedInit};

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

    let guard = match OBJC_RUNTIME.read() {
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
        let mut guard = match OBJC_RUNTIME.write() {
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

    let mut guard = match OBJC_RUNTIME.write() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use crate::class::{OBJC_RUNTIME, ObjcRuntime};

    fn ensure_runtime() {
        let mut guard = OBJC_RUNTIME.write().unwrap();
        guard.get_or_insert_with(ObjcRuntime::new);
    }

    #[test]
    fn allocate_protocol_null() {
        ensure_runtime();
        assert_eq!(shim_objc_allocate_protocol(std::ptr::null()), 0);
    }

    #[test]
    fn allocate_protocol_empty() {
        ensure_runtime();
        let name = CString::new("").unwrap();
        assert_eq!(shim_objc_allocate_protocol(name.as_ptr()), 0);
    }

    #[test]
    fn allocate_protocol_valid() {
        ensure_runtime();
        let name = CString::new("ProtoTestA").unwrap();
        let sentinel = shim_objc_allocate_protocol(name.as_ptr());
        assert_ne!(sentinel, 0);
    }

    #[test]
    fn register_protocol_null() {
        shim_objc_register_protocol(0);
    }

    #[test]
    fn add_protocol_null_cls() {
        assert_eq!(shim_class_add_protocol(0, 0x7000_0000_0000_1234), 0);
    }

    #[test]
    fn add_protocol_null_proto() {
        assert_eq!(shim_class_add_protocol(0x1234, 0), 0);
    }
}

