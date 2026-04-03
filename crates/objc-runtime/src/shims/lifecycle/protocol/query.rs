//! Protocol query shims: objc_getProtocol, protocol_getName, conformsToProtocol, etc.

use std::ffi::{c_char, CStr, CString};

use crate::class::OBJC_RUNTIME;

use super::helpers::{protocol_name_to_sentinel, sentinel_to_protocol_name,
};

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

    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(rt) = guard.as_ref() else { return 0 };

    // Find the protocol whose sentinel matches
    for (name, _) in rt.protocols_iter() {
        let sentinel = protocol_name_to_sentinel(name);
        if sentinel == proto {
            let cstr = CString::new(name).unwrap_or_default();
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

    let guard = match OBJC_RUNTIME.read() {
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

    let guard = match OBJC_RUNTIME.read() {
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
    fn get_protocol_null_name() {
        ensure_runtime();
        assert_eq!(shim_objc_get_protocol(std::ptr::null()), 0);
    }

    #[test]
    fn get_protocol_empty_name() {
        ensure_runtime();
        let name = CString::new("").unwrap();
        assert_eq!(shim_objc_get_protocol(name.as_ptr()), 0);
    }

    #[test]
    fn get_protocol_nonexistent() {
        ensure_runtime();
        let name = CString::new("NoSuchProtocol").unwrap();
        assert_eq!(shim_objc_get_protocol(name.as_ptr()), 0);
    }

    #[test]
    fn protocol_is_equal_same() {
        assert_eq!(shim_protocol_is_equal(0x7000_0000_0000_1234, 0x7000_0000_0000_1234), 1);
    }

    #[test]
    fn protocol_is_equal_different() {
        assert_eq!(shim_protocol_is_equal(0x7000_0000_0000_1234, 0x7000_0000_0000_5678), 0);
    }

    #[test]
    fn class_conforms_null_cls() {
        assert_eq!(shim_class_conforms_to_protocol(0, 0x7000_0000_0000_1234), 0);
    }

    #[test]
    fn class_conforms_null_proto() {
        assert_eq!(shim_class_conforms_to_protocol(0x1234, 0), 0);
    }

    #[test]
    fn class_copy_protocol_list_null() {
        let mut count: u32 = 99;
        let result = shim_class_copy_protocol_list(0, &mut count as *mut u32);
        assert_eq!(result, 0);
        assert_eq!(count, 0);
    }
}

