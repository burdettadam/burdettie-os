//! NSMethodSignature shim — type encoding parser and signature queries.
//!
//! Layout (24 bytes):
//!   offset 0: ISA pointer (u64)
//!   offset 8: pointer to heap-allocated C string of the type encoding
//!   offset 16: numberOfArguments (u64)

use crate::nsobj;

/// Instance size for NSMethodSignature objects.
pub const INSTANCE_SIZE: u32 = 24;

// ── Type encoding parser ───────────────────────────────────────────

/// Parse an ObjC type encoding string into individual type components.
///
/// Examples:
///   "v@:" → ["v", "@", ":"]
///   "@@:@" → ["@", "@", ":", "@"]
///   "v24@0:8@@16" → ["v", "@", ":", "@", "@"]
///   "{_NSRange=QQ}@:@" → ["{_NSRange=QQ}", "@", ":", "@"]
pub fn parse_type_encoding(encoding: &str) -> Vec<String> {
    let mut types = Vec::new();
    let bytes = encoding.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip digits (stack offsets)
        if bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // Skip type qualifiers: r(const), n(in), N(inout), o(out), O(bycopy), R(byref), V(oneway)
        if matches!(bytes[i], b'r' | b'n' | b'N' | b'o' | b'O' | b'R' | b'V') {
            // Only skip if next char is not end-of-string and is a type char
            if i + 1 < bytes.len() && !bytes[i + 1].is_ascii_digit() {
                i += 1;
                continue;
            }
        }
        match bytes[i] {
            // Struct: {name=fields}
            b'{' => {
                let start = i;
                let mut depth = 1;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'{' {
                        depth += 1;
                    } else if bytes[i] == b'}' {
                        depth -= 1;
                    }
                    i += 1;
                }
                types.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
            }
            // Union: (name=fields)
            b'(' => {
                let start = i;
                let mut depth = 1;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'(' {
                        depth += 1;
                    } else if bytes[i] == b')' {
                        depth -= 1;
                    }
                    i += 1;
                }
                types.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
            }
            // Array: [count type]
            b'[' => {
                let start = i;
                let mut depth = 1;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'[' {
                        depth += 1;
                    } else if bytes[i] == b']' {
                        depth -= 1;
                    }
                    i += 1;
                }
                types.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
            }
            // Pointer to type
            b'^' => {
                // ^type — take ^ and the next type char (simplified)
                let start = i;
                i += 1;
                if i < bytes.len() {
                    if bytes[i] == b'{' {
                        // ^{struct} — include the whole struct
                        let mut depth = 1;
                        i += 1;
                        while i < bytes.len() && depth > 0 {
                            if bytes[i] == b'{' {
                                depth += 1;
                            } else if bytes[i] == b'}' {
                                depth -= 1;
                            }
                            i += 1;
                        }
                    } else {
                        i += 1; // single char after ^
                    }
                }
                types.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
            }
            // Simple types: v c i s l q C I S L Q f d B @ # : * ? b
            _ => {
                types.push(String::from(bytes[i] as char));
                i += 1;
            }
        }
    }
    types
}

/// Return the size in bytes for a single type encoding character.
pub fn size_of_type(type_char: u8) -> usize {
    match type_char {
        b'c' | b'C' | b'B' => 1,
        b's' | b'S' => 2,
        b'i' | b'I' | b'f' => 4,
        b'l' | b'L' => 4,
        b'q' | b'Q' | b'd' => 8,
        b'@' | b'#' | b':' | b'*' | b'^' | b'?' => 8, // pointers
        b'v' => 0,
        _ => 8, // default to pointer size
    }
}

// ── Shim functions ─────────────────────────────────────────────────

/// `+[NSMethodSignature signatureWithObjCTypes:]`
///
/// Allocates an NSMethodSignature, parses the type encoding, and stores
/// the encoding string and argument count.
pub extern "C" fn shim_signature_with_objc_types(_cls: u64, _cmd: u64, types_ptr: u64) -> u64 {
    if types_ptr == 0 {
        return 0;
    }
    let encoding = unsafe {
        std::ffi::CStr::from_ptr(types_ptr as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    if encoding.is_empty() {
        return 0;
    }

    let parsed = parse_type_encoding(encoding);
    // First type is the return type; remaining are arguments (including self and _cmd)
    let num_args = if parsed.len() > 1 {
        parsed.len() - 1
    } else {
        0
    };

    // Allocate the object
    let obj = crate::pal_alloc().calloc(1, INSTANCE_SIZE as usize) as u64;
    if obj == 0 {
        return 0;
    }

    // Set ISA
    let isa = nsobj::nsmethodsignature_class_addr();
    unsafe {
        *(obj as *mut u64) = isa;
    }

    // Copy the encoding string to a heap-allocated C string
    let c_encoding = std::ffi::CString::new(encoding).unwrap_or_default();
    let encoding_ptr = c_encoding.into_raw() as u64;
    unsafe {
        *((obj + 8) as *mut u64) = encoding_ptr;
        *((obj + 16) as *mut u64) = num_args as u64;
    }

    obj
}

/// `-[NSMethodSignature numberOfArguments]`
pub extern "C" fn shim_number_of_arguments(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj + 16) as *const u64) }
}

/// `-[NSMethodSignature methodReturnType]`
///
/// Returns a pointer to a C string describing the return type.
/// The pointer is valid as long as the NSMethodSignature exists.
pub extern "C" fn shim_method_return_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // The encoding C string is at offset 8; return type is the first char.
    // We return the same pointer — caller reads the first type character.
    unsafe { *((obj + 8) as *const u64) }
}

/// `-[NSMethodSignature methodReturnLength]`
pub extern "C" fn shim_method_return_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let enc_ptr = unsafe { *((obj + 8) as *const u64) };
    if enc_ptr == 0 {
        return 0;
    }
    let encoding = unsafe {
        std::ffi::CStr::from_ptr(enc_ptr as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    let parsed = parse_type_encoding(encoding);
    if parsed.is_empty() {
        return 0;
    }
    size_of_type(parsed[0].as_bytes()[0]) as u64
}

/// `-[NSMethodSignature getArgumentTypeAtIndex:]`
///
/// Returns a pointer to a C string for the argument type at the given index.
/// Index 0 = self, index 1 = _cmd, index 2+ = method arguments.
pub extern "C" fn shim_get_argument_type_at_index(obj: u64, _cmd: u64, index: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let enc_ptr = unsafe { *((obj + 8) as *const u64) };
    if enc_ptr == 0 {
        return 0;
    }
    let encoding = unsafe {
        std::ffi::CStr::from_ptr(enc_ptr as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    let parsed = parse_type_encoding(encoding);
    // Argument at index N corresponds to parsed[N+1] (parsed[0] is return type)
    let type_index = (index as usize) + 1;
    if type_index >= parsed.len() {
        return 0;
    }
    // Return a pointer to a leaked C string of this type
    let c_type = std::ffi::CString::new(parsed[type_index].as_str()).unwrap_or_default();
    c_type.into_raw() as u64
}

/// `-[NSMethodSignature frameLength]`
///
/// Returns total frame size (sum of all argument sizes).
pub extern "C" fn shim_frame_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let enc_ptr = unsafe { *((obj + 8) as *const u64) };
    if enc_ptr == 0 {
        return 0;
    }
    let encoding = unsafe {
        std::ffi::CStr::from_ptr(enc_ptr as *const std::ffi::c_char)
            .to_str()
            .unwrap_or("")
    };
    let parsed = parse_type_encoding(encoding);
    let mut total: usize = 0;
    for (i, t) in parsed.iter().enumerate() {
        if i == 0 {
            continue; // skip return type
        }
        total += size_of_type(t.as_bytes()[0]);
    }
    total as u64
}

/// `-[NSMethodSignature dealloc]`
pub extern "C" fn shim_dealloc(obj: u64, _cmd: u64) {
    if obj == 0 {
        return;
    }
    // Free the encoding C string
    let enc_ptr = unsafe { *((obj + 8) as *const u64) };
    if enc_ptr != 0 {
        unsafe {
            drop(std::ffi::CString::from_raw(
                enc_ptr as *mut std::ffi::c_char,
            ));
        }
    }
    crate::pal_alloc().free(obj as *mut u8);
}

// ── Helper for creating NSMethodSignature from Rust ────────────────

/// Create an NSMethodSignature from a Rust type encoding string.
/// Used internally by methodSignatureForSelector: and the forwarding chain.
pub fn create_method_signature(encoding: &str) -> u64 {
    let c_encoding = std::ffi::CString::new(encoding).unwrap_or_default();
    shim_signature_with_objc_types(
        nsobj::nsmethodsignature_class_addr(),
        0,
        c_encoding.as_ptr() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_encoding() {
        assert_eq!(parse_type_encoding("v@:"), vec!["v", "@", ":"]);
        assert_eq!(parse_type_encoding("@@:@"), vec!["@", "@", ":", "@"]);
        assert_eq!(parse_type_encoding("B@:#"), vec!["B", "@", ":", "#"]);
    }

    #[test]
    fn parse_encoding_with_offsets() {
        assert_eq!(
            parse_type_encoding("v24@0:8@@16"),
            vec!["v", "@", ":", "@", "@"]
        );
    }

    #[test]
    fn parse_struct_encoding() {
        assert_eq!(
            parse_type_encoding("{_NSRange=QQ}@:@"),
            vec!["{_NSRange=QQ}", "@", ":", "@"]
        );
    }

    #[test]
    fn parse_pointer_encoding() {
        assert_eq!(parse_type_encoding("v@:^v"), vec!["v", "@", ":", "^v"]);
        assert_eq!(parse_type_encoding("@@:^@"), vec!["@", "@", ":", "^@"]);
    }

    #[test]
    fn size_of_basic_types() {
        assert_eq!(size_of_type(b'v'), 0);
        assert_eq!(size_of_type(b'c'), 1);
        assert_eq!(size_of_type(b'i'), 4);
        assert_eq!(size_of_type(b'q'), 8);
        assert_eq!(size_of_type(b'@'), 8);
        assert_eq!(size_of_type(b':'), 8);
        assert_eq!(size_of_type(b'B'), 1);
        assert_eq!(size_of_type(b'f'), 4);
        assert_eq!(size_of_type(b'd'), 8);
    }

    #[test]
    fn create_signature_roundtrip() {
        let sig = create_method_signature("@@:@");
        assert_ne!(sig, 0);
        // numberOfArguments should be 3 (self, _cmd, one arg)
        assert_eq!(shim_number_of_arguments(sig, 0), 3);
        // methodReturnLength should be 8 (id/@)
        assert_eq!(shim_method_return_length(sig, 0), 8);
        // Clean up
        shim_dealloc(sig, 0);
    }
}
