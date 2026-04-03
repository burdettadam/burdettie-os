//! Printf-style format string expansion with ObjC `%@` support.

use crate::cfstring;

/// Format an ObjC object for `%@`.
///
/// If the object is a CFConstantString, extract the C string directly.
/// Tagged pointers are formatted guest-side.
/// Otherwise, look up and call the object's `-description` method through the
/// ObjC runtime — `description` returns a CFString we can read.
pub(crate) fn format_objc_object(obj: u64) -> String {
    // Null pointer guard.
    if obj == 0 {
        return "(null)".to_string();
    }

    // Tagged pointer: bit 63 set on arm64 means it's an inline-encoded
    // value (NSNumber, NSDate, etc.), not a real heap pointer.
    if (obj >> 63) != 0 {
        return format_tagged_pointer(obj);
    }

    // Alignment check: a valid ObjC object pointer must be 8-byte aligned.
    if obj & 0x7 != 0 {
        return format!("<invalid obj {:#x}>", obj);
    }

    let isa = unsafe { *(obj as *const u64) };

    // Fast path: try to read as a CFString (covers CFConstantString and
    // dynamically-created NSString/NSMutableString — they share the same
    // memory layout with the c_str pointer at offset +16).
    if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
        return s.to_string();
    }

    // Slow path: call -description via the ObjC runtime
    let guard = match objc_runtime::OBJC_RUNTIME.read() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if let Some(rt) = guard.as_ref() {
        if let Some(imp) = rt.msg_lookup(isa, "description") {
            let desc_fn: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(imp) };
            let desc_obj = desc_fn(obj, 0);
            if desc_obj != 0 {
                if let Some(s) = cfstring::extract_cfstring_cstr(desc_obj) {
                    return s.to_string();
                }
            }
        }
    }

    format!("<ObjC object {:#x}>", obj)
}

/// Format a tagged pointer guest-side using the runtime's known Foundation
/// tagged-pointer class mapping.

mod tagged;
use self::tagged::format_tagged_pointer;
pub(crate) use self::tagged::expand_format;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_format_plain_string() {
        assert_eq!(expand_format("hello world", &[]), "hello world");
    }

    #[test]
    fn expand_format_int() {
        assert_eq!(expand_format("value=%d", &[42]), "value=42");
    }

    #[test]
    fn expand_format_percent_escape() {
        assert_eq!(expand_format("100%%", &[]), "100%");
    }

    #[test]
    fn expand_format_obj_null() {
        assert_eq!(expand_format("obj=%@", &[0]), "obj=(null)");
    }

    #[test]
    fn expand_format_multiple_ints() {
        assert_eq!(expand_format("%d + %d = %d", &[3, 4, 7]), "3 + 4 = 7");
    }

    #[test]
    fn expand_format_float() {
        let f: f64 = 2.72;
        assert_eq!(expand_format("pi=%f", &[f.to_bits()]), "pi=2.720000");
    }

    #[test]
    fn expand_format_float_zero() {
        let f: f64 = 0.0;
        assert_eq!(expand_format("%f", &[f.to_bits()]), "0.000000");
    }

    #[test]
    fn expand_format_hex_lower() {
        assert_eq!(expand_format("0x%x", &[255]), "0xff");
    }

    #[test]
    fn expand_format_hex_upper() {
        assert_eq!(expand_format("0x%X", &[255]), "0xFF");
    }

    #[test]
    fn expand_format_pointer() {
        assert_eq!(expand_format("ptr=%p", &[0x1234]), "ptr=0x1234");
    }

    #[test]
    fn format_objc_object_tagged_nsnumber_uses_guest_decoder() {
        let tagged_number = (1u64 << 63) | (2u64 << 60) | 42;
        assert_eq!(format_objc_object(tagged_number), "42");
    }

    #[test]
    fn format_objc_object_tagged_nsstring_uses_guest_placeholder() {
        let tagged_string = (1u64 << 63) | (3u64 << 60) | 0x1234;
        assert_eq!(
            format_objc_object(tagged_string),
            format!("<tagged NSString {:#x}>", tagged_string)
        );
    }

    #[test]
    fn expand_format_octal() {
        assert_eq!(expand_format("oct=%o", &[8]), "oct=10");
    }

    #[test]
    fn expand_format_lld() {
        let v = (-123456789i64) as u64;
        assert_eq!(expand_format("big=%lld", &[v]), "big=-123456789");
    }

    #[test]
    fn expand_format_llu() {
        assert_eq!(
            expand_format("big=%llu", &[u64::MAX]),
            format!("big={}", u64::MAX)
        );
    }

    #[test]
    fn expand_format_zu() {
        assert_eq!(expand_format("size=%zu", &[1024]), "size=1024");
    }

    #[test]
    fn expand_format_zd() {
        let v = (-42i64) as u64;
        assert_eq!(expand_format("off=%zd", &[v]), "off=-42");
    }

    #[test]
    fn expand_format_scientific() {
        let f: f64 = 12345.0;
        let s = expand_format("%e", &[f.to_bits()]);
        assert!(
            s.contains("e") || s.contains("E"),
            "expected scientific notation, got: {}",
            s
        );
    }

    #[test]
    fn expand_format_lf() {
        let f: f64 = 2.5;
        assert_eq!(expand_format("val=%lf", &[f.to_bits()]), "val=2.500000");
    }

    #[test]
    fn expand_format_mixed_new_specifiers() {
        let f: f64 = 1.5;
        assert_eq!(
            expand_format("%d %x %f", &[10, 0xFF, f.to_bits()]),
            "10 ff 1.500000"
        );
    }
}
