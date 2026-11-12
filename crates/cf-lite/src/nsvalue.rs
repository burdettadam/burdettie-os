//! NSValue shim: wraps non-object values (NSRange, CGPoint, CGSize, CGRect, pointers, raw bytes).
//!
//! Object layout: 32 bytes
//!   [0..8):  ISA pointer
//!   [8..16): value_lo (u64) — first 8 bytes of stored value (e.g., range.location)
//!   [16..24): value_hi (u64) — second 8 bytes (e.g., range.length)
//!   [24..32): objc_type ptr (pointer to C string with ObjC type encoding)
//!
//! For types larger than 16 bytes (CGRect, NSEdgeInsets), we extend to 64 bytes:
//!   [0..8):   ISA pointer
//!   [8..16):  value bytes 0..8
//!   [16..24): value bytes 8..16
//!   [24..32): value bytes 16..24
//!   [32..40): value bytes 24..32
//!   [40..48): objc_type ptr
//!   [48..56): padding
//!   [56..64): padding

use crate::cfstring;
use crate::nsobj;

const NSVALUE_SIZE: usize = 64; // Extended to hold CGRect (32 bytes of value data)

/// Allocate a 64-byte NSValue on the heap.
/// Layout: [ISA:8][v0:8][v1:8][v2:8][v3:8][type_ptr:8][pad:16]
fn alloc_nsvalue(lo: u64, hi: u64, type_ptr: *const u8) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSVALUE_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    unsafe {
        *obj = nsobj::nsvalue_class_addr();
        *obj.add(1) = lo;
        *obj.add(2) = hi;
        // v2, v3 = 0 from calloc
        *obj.add(5) = type_ptr as u64;
    }
    obj as u64
}

/// Allocate a 64-byte NSValue holding up to 32 bytes of raw data.
fn alloc_nsvalue_raw(data: &[u8], type_ptr: *const u8) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSVALUE_SIZE) as *mut u64;
    if obj.is_null() {
        return 0;
    }
    unsafe {
        *obj = nsobj::nsvalue_class_addr();
        // Copy up to 32 bytes of value data into slots [1..5)
        let dst = (obj as *mut u8).add(8);
        let copy_len = data.len().min(32);
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, copy_len);
        *obj.add(5) = type_ptr as u64;
    }
    obj as u64
}

// ── Static type encoding strings ───────────────────────────────────

/// ObjC type encoding for NSRange: `{_NSRange=QQ}`
static RANGE_TYPE: &[u8] = b"{_NSRange=QQ}\0";
/// ObjC type encoding for a pointer: `^v`
static POINTER_TYPE: &[u8] = b"^v\0";
/// ObjC type encoding for CGPoint: `{CGPoint=dd}`
static CGPOINT_TYPE: &[u8] = b"{CGPoint=dd}\0";
/// ObjC type encoding for CGSize: `{CGSize=dd}`
static CGSIZE_TYPE: &[u8] = b"{CGSize=dd}\0";
/// ObjC type encoding for CGRect: `{CGRect={CGPoint=dd}{CGSize=dd}}`
static CGRECT_TYPE: &[u8] = b"{CGRect={CGPoint=dd}{CGSize=dd}}\0";

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSValue valueWithBytes:objCType:]`
///
/// Reads up to 16 bytes from the provided buffer based on the type encoding.
/// For simplicity we always copy 16 bytes (zero-padded by calloc).
pub extern "C" fn shim_nsvalue_value_with_bytes(
    _cls: u64,
    _cmd: u64,
    bytes: u64,
    objc_type: u64,
) -> u64 {
    if bytes == 0 {
        return alloc_nsvalue(0, 0, std::ptr::null());
    }
    let lo = unsafe { std::ptr::read_unaligned(bytes as *const u64) };
    let hi = unsafe { std::ptr::read_unaligned((bytes as *const u64).add(1)) };
    let type_ptr = if objc_type != 0 {
        objc_type as *const u8
    } else {
        std::ptr::null()
    };
    alloc_nsvalue(lo, hi, type_ptr)
}

/// `+[NSValue valueWithRange:]` — wraps an NSRange (location, length).
///
/// Called with two u64 args after _cmd: location and length.
pub extern "C" fn shim_nsvalue_value_with_range(
    _cls: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    alloc_nsvalue(location, length, RANGE_TYPE.as_ptr())
}

/// `+[NSValue valueWithPointer:]` — wraps a raw pointer.
pub extern "C" fn shim_nsvalue_value_with_pointer(_cls: u64, _cmd: u64, pointer: u64) -> u64 {
    alloc_nsvalue(pointer, 0, POINTER_TYPE.as_ptr())
}

/// `+[NSValue valueWithPoint:]` — wraps a CGPoint.
/// On ARM64, CGPoint (16 bytes) is passed in x2/x3 (two f64 args).
pub extern "C" fn shim_nsvalue_value_with_point(_cls: u64, _cmd: u64, x: f64, y: f64) -> u64 {
    let data = crate::cg_types::CGPoint { x, y };
    alloc_nsvalue_raw(
        unsafe { std::slice::from_raw_parts(&data as *const _ as *const u8, 16) },
        CGPOINT_TYPE.as_ptr(),
    )
}

/// `+[NSValue valueWithSize:]` — wraps a CGSize.
/// On ARM64, CGSize (16 bytes) is passed in x2/x3 (two f64 args).
pub extern "C" fn shim_nsvalue_value_with_size(
    _cls: u64,
    _cmd: u64,
    width: f64,
    height: f64,
) -> u64 {
    let data = crate::cg_types::CGSize { width, height };
    alloc_nsvalue_raw(
        unsafe { std::slice::from_raw_parts(&data as *const _ as *const u8, 16) },
        CGSIZE_TYPE.as_ptr(),
    )
}

/// `+[NSValue valueWithRect:]` — wraps a CGRect.
/// On ARM64, CGRect (32 bytes) is passed via x8 indirect pointer (the caller
/// pushes it on the stack and passes x8 pointing to it). The trampoline
/// preserves x8. But for our shim call, the runtime passes the four f64
/// components as args.
pub extern "C" fn shim_nsvalue_value_with_rect(
    _cls: u64,
    _cmd: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> u64 {
    let data = crate::cg_types::CGRect::new(x, y, width, height);
    alloc_nsvalue_raw(
        unsafe { std::slice::from_raw_parts(&data as *const _ as *const u8, 32) },
        CGRECT_TYPE.as_ptr(),
    )
}

// ── Instance methods ───────────────────────────────────────────────

/// `-[NSValue rangeValue]` — returns the stored NSRange.location.
///
/// In the ObjC ABI for arm64, an NSRange return is passed via x0/x1.
/// We return location in x0; the caller reads length separately.
pub extern "C" fn shim_nsvalue_range_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue pointerValue]` — returns the stored pointer.
pub extern "C" fn shim_nsvalue_pointer_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue pointValue]` — returns the stored CGPoint as two f64s.
/// On ARM64, CGPoint (16 bytes ≤ 16) is returned in x0/x1 (as bit patterns).
pub extern "C" fn shim_nsvalue_point_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Return first 8 bytes (x component as f64 bit pattern in x0)
    // The runtime/caller will read x1 = second 8 bytes for y
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue sizeValue]` — returns the stored CGSize.
pub extern "C" fn shim_nsvalue_size_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue rectValue]` — returns the stored CGRect.
/// On ARM64, CGRect (32 bytes > 16) is returned via x8 indirect pointer.
/// The caller sets x8 to a buffer; we write 32 bytes there.
/// However, since we can't easily write to x8 from a shim function,
/// we return the first 8 bytes in x0 for now. The fixture uses
/// direct memory reads on the NSValue object for full rect access.
pub extern "C" fn shim_nsvalue_rect_value(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(1)) }
}

/// `-[NSValue objCType]` — returns the ObjC type encoding C-string.
pub extern "C" fn shim_nsvalue_objc_type(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u64).add(5)) }
}

/// `-[NSValue description]` — returns an NSString description.
pub extern "C" fn shim_nsvalue_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let lo = unsafe { *((obj as *const u64).add(1)) };
    let hi = unsafe { *((obj as *const u64).add(2)) };
    let desc = format!("<NSValue: lo={lo}, hi={hi}>\0");
    let c_str = std::ffi::CString::new(desc.trim_end_matches('\0')).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

/// `-[NSValue initWithBytes:objCType:]` — initialize value from raw bytes + type encoding.
pub extern "C" fn shim_nsvalue_init_with_bytes_objctype(
    obj: u64,
    _cmd: u64,
    bytes: u64,
    type_str: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Set ISA (may already be set from alloc, but be safe)
    unsafe {
        *(obj as *mut u64) = nsobj::nsvalue_class_addr();
    }
    // Determine size from type encoding (simple heuristic: up to 32 bytes)
    let copy_len = if type_str != 0 {
        let c = unsafe { std::ffi::CStr::from_ptr(type_str as *const std::ffi::c_char) };
        let s = c.to_str().unwrap_or("");
        type_encoding_size(s).min(32)
    } else {
        8
    };
    // Copy bytes into value storage (offset 8)
    if bytes != 0 {
        unsafe {
            let dst = (obj as *mut u8).add(8);
            std::ptr::copy_nonoverlapping(bytes as *const u8, dst, copy_len);
        }
    }
    // Store type pointer at offset 40 (slot 5)
    unsafe {
        *((obj as *mut u64).add(5)) = type_str;
    }
    obj
}

/// `-[NSValue isEqualToValue:]` — compare stored bytes of two NSValue objects.
pub extern "C" fn shim_nsvalue_is_equal_to_value(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 {
        return 0;
    }
    if obj == other {
        return 1;
    }
    // Compare the 32 bytes of value storage (offsets 8..40)
    let a = unsafe { std::slice::from_raw_parts((obj as *const u8).add(8), 32) };
    let b = unsafe { std::slice::from_raw_parts((other as *const u8).add(8), 32) };
    if a == b {
        1
    } else {
        0
    }
}

/// Estimate byte size for a simple ObjC type encoding.
fn type_encoding_size(enc: &str) -> usize {
    match enc {
        "c" | "C" | "B" => 1,
        "s" | "S" => 2,
        "i" | "I" | "f" => 4,
        "q" | "Q" | "d" | "^v" | "@" | "#" | ":" => 8,
        _ if enc.contains("CGRect") || enc.contains("NSEdgeInsets") => 32,
        _ if enc.contains("CGPoint") | enc.contains("CGSize") | enc.contains("NSRange") => 16,
        _ => 8,
    }
}

// ── NSStringFrom* free functions ───────────────────────────────────
//
// These are C functions (not ObjC methods) that convert CG/NS types to
// NSString. On ARM64, CGPoint/NSRange (≤16 bytes) are passed in registers;
// CGRect (32 bytes) is passed indirectly via pointer.

/// `NSStringFromPoint(CGPoint)` → NSString like `"{x, y}"`.
/// ARM64 ABI: CGPoint (16 bytes) passed as two f64s in x0/x1 (d0/d1).
pub extern "C" fn shim_nsstringfrompoint(x: f64, y: f64) -> u64 {
    let s = format!("{{{x}, {y}}}");
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

/// `NSStringFromSize(CGSize)` → NSString like `"{width, height}"`.
pub extern "C" fn shim_nsstringfromsize(width: f64, height: f64) -> u64 {
    let s = format!("{{{width}, {height}}}");
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

/// `NSStringFromRect(CGRect)` → NSString like `"{{x, y}, {w, h}}"`.
/// ARM64 ABI: CGRect (32 bytes) passed as four f64s in d0-d3 (or x0-x3).
pub extern "C" fn shim_nsstringfromrect(x: f64, y: f64, w: f64, h: f64) -> u64 {
    let s = format!("{{{{{x}, {y}}}, {{{w}, {h}}}}}");
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

/// `NSStringFromRange(NSRange)` → NSString like `"{location, length}"`.
/// ARM64 ABI: NSRange (16 bytes) passed in x0/x1.
pub extern "C" fn shim_nsstringfromrange(location: u64, length: u64) -> u64 {
    let s = format!("{{{location}, {length}}}");
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn range_roundtrip() {
        let val = shim_nsvalue_value_with_range(0, 0, 10, 5);
        assert_ne!(val, 0);
        assert_eq!(shim_nsvalue_range_value(val, 0), 10);
        // hi slot = length
        assert_eq!(read_field(val, 16), 5);
        free_obj(val);
    }

    #[test]
    fn pointer_roundtrip() {
        let val = shim_nsvalue_value_with_pointer(0, 0, 0xCAFE_BABE);
        assert_eq!(shim_nsvalue_pointer_value(val, 0), 0xCAFE_BABE);
        free_obj(val);
    }

    #[test]
    fn point_roundtrip() {
        let val = shim_nsvalue_value_with_point(0, 0, 1.5, 2.5);
        assert_ne!(val, 0);
        let x_bits = shim_nsvalue_point_value(val, 0);
        assert_eq!(f64::from_bits(x_bits), 1.5);
        let y_bits = read_field(val, 16);
        assert_eq!(f64::from_bits(y_bits), 2.5);
        free_obj(val);
    }

    #[test]
    fn size_roundtrip() {
        let val = shim_nsvalue_value_with_size(0, 0, 100.0, 200.0);
        let w_bits = shim_nsvalue_size_value(val, 0);
        assert_eq!(f64::from_bits(w_bits), 100.0);
        let h_bits = read_field(val, 16);
        assert_eq!(f64::from_bits(h_bits), 200.0);
        free_obj(val);
    }

    #[test]
    fn rect_roundtrip() {
        let val = shim_nsvalue_value_with_rect(0, 0, 10.0, 20.0, 300.0, 400.0);
        // Read all four doubles from the value data
        let x = f64::from_bits(read_field(val, 8));
        let y = f64::from_bits(read_field(val, 16));
        let w = f64::from_bits(read_field(val, 24));
        let h = f64::from_bits(read_field(val, 32));
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        assert_eq!(w, 300.0);
        assert_eq!(h, 400.0);
        free_obj(val);
    }

    #[test]
    fn objc_type_encoding() {
        let range_val = shim_nsvalue_value_with_range(0, 0, 0, 0);
        let type_ptr = shim_nsvalue_objc_type(range_val, 0);
        assert_ne!(type_ptr, 0);
        let c_str = unsafe { std::ffi::CStr::from_ptr(type_ptr as *const std::ffi::c_char) };
        assert_eq!(c_str.to_str().unwrap(), "{_NSRange=QQ}");
        free_obj(range_val);
    }

    #[test]
    fn nil_returns_zero() {
        assert_eq!(shim_nsvalue_range_value(0, 0), 0);
        assert_eq!(shim_nsvalue_pointer_value(0, 0), 0);
        assert_eq!(shim_nsvalue_objc_type(0, 0), 0);
    }

    #[test]
    fn string_from_range() {
        let s = shim_nsstringfromrange(3, 7);
        assert_string_eq(s, "{3, 7}");
    }

    #[test]
    fn string_from_point() {
        let s = shim_nsstringfrompoint(1.5, 2.5);
        assert_string_eq(s, "{1.5, 2.5}");
    }

    #[test]
    fn string_from_size() {
        let s = shim_nsstringfromsize(100.0, 200.0);
        assert_string_eq(s, "{100, 200}");
    }

    #[test]
    fn string_from_rect() {
        let s = shim_nsstringfromrect(0.0, 0.0, 1024.0, 768.0);
        assert_string_eq(s, "{{0, 0}, {1024, 768}}");
    }

    #[test]
    fn init_with_bytes_roundtrip() {
        let obj = alloc_obj(NSVALUE_SIZE, crate::nsobj::nsvalue_class_addr());
        let data: u64 = 42;
        let type_enc = b"q\0";
        let result = shim_nsvalue_init_with_bytes_objctype(
            obj,
            0,
            &data as *const u64 as u64,
            type_enc.as_ptr() as u64,
        );
        assert_eq!(result, obj);
        assert_eq!(read_field(obj, 8), 42);
        free_obj(obj);
    }

    #[test]
    fn init_with_bytes_nil() {
        assert_eq!(shim_nsvalue_init_with_bytes_objctype(0, 0, 0, 0), 0);
    }

    #[test]
    fn is_equal_to_value_same() {
        let a = shim_nsvalue_value_with_range(0, 0, 10, 20);
        let b = shim_nsvalue_value_with_range(0, 0, 10, 20);
        assert_eq!(shim_nsvalue_is_equal_to_value(a, 0, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_value_different() {
        let a = shim_nsvalue_value_with_range(0, 0, 10, 20);
        let b = shim_nsvalue_value_with_range(0, 0, 10, 30);
        assert_eq!(shim_nsvalue_is_equal_to_value(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_value_nil() {
        assert_eq!(shim_nsvalue_is_equal_to_value(0, 0, 0), 0);
    }

    #[test]
    fn is_equal_to_value_self() {
        let a = shim_nsvalue_value_with_range(0, 0, 5, 5);
        assert_eq!(shim_nsvalue_is_equal_to_value(a, 0, a), 1);
        free_obj(a);
    }
}
