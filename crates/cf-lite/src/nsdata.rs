//! NSData shim: immutable byte buffer.
//!
//! Object layout (heap-allocated):
//!
//!   offset  0: isa        (u64 — pointer to NSData class)
//!   offset  8: length     (u64 — byte count)
//!   offset 16: bytes_ptr  (u64 — pointer to malloc'd byte buffer, or 0 for empty)
//!
//! Total: 24 bytes per NSData object.

const NSDATA_LENGTH_OFF: usize = 8;
const NSDATA_BYTES_OFF: usize = 16;

/// Create an NSData from a byte slice.
pub fn make_nsdata(bytes: &[u8]) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
        *((obj as usize + NSDATA_LENGTH_OFF) as *mut u64) = bytes.len() as u64;
    }
    if !bytes.is_empty() {
        let buf = crate::pal_alloc().malloc(bytes.len());
        if !buf.is_null() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len()); }
        }
        unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *mut u64) = buf as u64; }
    }
    obj
}

/// `+[NSData dataWithBytes:length:]`
pub extern "C" fn shim_nsdata_data_with_bytes_length(
    _cls: u64,
    _cmd: u64,
    bytes: u64,
    length: u64,
) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }

    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
    }
    unsafe {
        *((obj as usize + NSDATA_LENGTH_OFF) as *mut u64) = length;
    }

    if length > 0 && bytes != 0 {
        let buf = crate::pal_alloc().malloc(length as usize) as u64;
        if buf != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(bytes as *const u8, buf as *mut u8, length as usize);
            }
        }
        unsafe {
            *((obj as usize + NSDATA_BYTES_OFF) as *mut u64) = buf;
        }
    }

    log::debug!(
        "NSData dataWithBytes:length: → {:#x} ({} bytes)",
        obj,
        length
    );
    obj
}

/// `+[NSData data]` — empty data.
pub extern "C" fn shim_nsdata_data(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdata_class_addr();
    }
    obj
}

/// `-[NSData length]`
pub extern "C" fn shim_nsdata_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) }
}

/// `-[NSData bytes]` — returns raw pointer to byte buffer.
pub extern "C" fn shim_nsdata_bytes(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) }
}

/// `-[NSData description]` — returns NSString like `<hexdump>`.
pub extern "C" fn shim_nsdata_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let length = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };

    let hex = if length == 0 || bytes_ptr == 0 {
        String::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) };
        slice.iter().map(|b| format!("{b:02x}")).collect()
    };

    let desc = format!("<{hex}>");
    let cstr = std::ffi::CString::new(desc).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSData writeToFile:atomically:]`
///
/// Writes the data to the specified file path. Returns 1 on success, 0 on failure.
pub extern "C" fn shim_nsdata_write_to_file_atomically(
    obj: u64,
    _cmd: u64,
    path_obj: u64,
    atomically: u8,
) -> u8 {
    if obj == 0 || path_obj == 0 {
        return 0;
    }
    let path_str = match crate::cfstring::extract_cfstring_cstr(path_obj) {
        Some(s) => s,
        None => return 0,
    };

    let length = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };

    let data = if length == 0 || bytes_ptr == 0 {
        &[] as &[u8]
    } else {
        unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) }
    };

    log::debug!(
        "NSData writeToFile:atomically: path={} len={} atomic={}",
        path_str,
        length,
        atomically
    );

    let path = std::path::Path::new(path_str);
    let result = if atomically != 0 {
        // Write to a temp file then rename for atomicity
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, data)
            .and_then(|_| std::fs::rename(&tmp_path, path))
    } else {
        std::fs::write(path, data)
    };

    match result {
        Ok(()) => 1,
        Err(e) => {
            log::error!("NSData writeToFile failed: {}: {}", path_str, e);
            0
        }
    }
}

/// `-[NSData isEqualToData:]`
pub extern "C" fn shim_nsdata_is_equal_to_data(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == other {
        return 1;
    }
    if obj == 0 || other == 0 {
        return 0;
    }

    let len_a = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) };
    let len_b = unsafe { *((other as usize + NSDATA_LENGTH_OFF) as *const u64) };
    if len_a != len_b {
        return 0;
    }
    if len_a == 0 {
        return 1;
    }

    let ptr_a = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };
    let ptr_b = unsafe { *((other as usize + NSDATA_BYTES_OFF) as *const u64) };
    if ptr_a == 0 || ptr_b == 0 {
        return 0;
    }

    let slice_a = unsafe { std::slice::from_raw_parts(ptr_a as *const u8, len_a as usize) };
    let slice_b = unsafe { std::slice::from_raw_parts(ptr_b as *const u8, len_a as usize) };
    u8::from(slice_a == slice_b)
}

// ── Base64 encode / decode ─────────────────────────────────────────

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(B64_ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode_byte(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    // Strip whitespace
    let clean: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b' ' && b != b'\t').collect();
    if clean.is_empty() {
        return Some(Vec::new());
    }
    if clean.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let a = base64_decode_byte(chunk[0])?;
        let b = base64_decode_byte(chunk[1])?;
        let triple: u32;
        let bytes_out: usize;
        if chunk[2] == b'=' {
            triple = ((a as u32) << 18) | ((b as u32) << 12);
            bytes_out = 1;
        } else if chunk[3] == b'=' {
            let c = base64_decode_byte(chunk[2])?;
            triple = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6);
            bytes_out = 2;
        } else {
            let c = base64_decode_byte(chunk[2])?;
            let d = base64_decode_byte(chunk[3])?;
            triple = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);
            bytes_out = 3;
        }
        out.push((triple >> 16) as u8);
        if bytes_out > 1 {
            out.push((triple >> 8) as u8);
        }
        if bytes_out > 2 {
            out.push(triple as u8);
        }
    }
    Some(out)
}

/// Helper: read raw byte slice from an NSData object.
fn nsdata_bytes_slice(obj: u64) -> &'static [u8] {
    let length = unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *const u64) };
    if length == 0 || bytes_ptr == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, length) }
}

// ── New instance methods ───────────────────────────────────────────

/// `-[NSData base64EncodedStringWithOptions:]`
pub extern "C" fn shim_nsdata_base64_encoded_string_with_options(
    obj: u64,
    _cmd: u64,
    _options: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let encoded = base64_encode(nsdata_bytes_slice(obj));
    let c_str = std::ffi::CString::new(encoded).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(c_str.as_ptr())
}

/// `-[NSData initWithBase64EncodedString:options:]`
pub extern "C" fn shim_nsdata_init_with_base64_encoded_string(
    obj: u64,
    _cmd: u64,
    string: u64,
    _options: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    if string == 0 {
        return 0;
    }
    let text = match crate::cfstring::extract_cfstring_cstr(string) {
        Some(s) => s,
        None => return 0,
    };
    let decoded = match base64_decode(text) {
        Some(d) => d,
        None => return 0,
    };
    let len = decoded.len();
    if len > 0 {
        let buf = crate::pal_alloc().malloc(len);
        if !buf.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(decoded.as_ptr(), buf, len);
            }
            unsafe { *((obj as usize + NSDATA_BYTES_OFF) as *mut u64) = buf as u64; }
        }
    }
    unsafe { *((obj as usize + NSDATA_LENGTH_OFF) as *mut u64) = len as u64; }
    obj
}

/// `-[NSData base64EncodedDataWithOptions:]`
pub extern "C" fn shim_nsdata_base64_encoded_data_with_options(
    obj: u64,
    _cmd: u64,
    _options: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let encoded = base64_encode(nsdata_bytes_slice(obj));
    make_nsdata(encoded.as_bytes())
}

/// `-[NSData subdataWithRange:]` — range passed as two u64 (location, length).
pub extern "C" fn shim_nsdata_subdata_with_range(
    obj: u64,
    _cmd: u64,
    location: u64,
    length: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    let data = nsdata_bytes_slice(obj);
    let loc = location as usize;
    let len = length as usize;
    if loc > data.len() || loc + len > data.len() {
        return make_nsdata(&[]);
    }
    make_nsdata(&data[loc..loc + len])
}

/// `-[NSData getBytes:length:]`
pub extern "C" fn shim_nsdata_get_bytes_length(
    obj: u64,
    _cmd: u64,
    dest: u64,
    length: u64,
) -> u64 {
    if obj == 0 || dest == 0 {
        return 0;
    }
    let data = nsdata_bytes_slice(obj);
    let copy_len = (length as usize).min(data.len());
    if copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), dest as *mut u8, copy_len);
        }
    }
    0
}

/// `-[NSData getBytes:range:]` — range as two u64 (location, length).
pub extern "C" fn shim_nsdata_get_bytes_range(
    obj: u64,
    _cmd: u64,
    dest: u64,
    location: u64,
    length: u64,
) -> u64 {
    if obj == 0 || dest == 0 {
        return 0;
    }
    let data = nsdata_bytes_slice(obj);
    let loc = location as usize;
    let len = length as usize;
    if loc > data.len() || loc + len > data.len() {
        return 0;
    }
    if len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(data[loc..].as_ptr(), dest as *mut u8, len);
        }
    }
    0
}

/// `+[NSData dataWithContentsOfFile:]` — class method.
pub extern "C" fn shim_nsdata_data_with_contents_of_file(
    _cls: u64,
    _cmd: u64,
    path_obj: u64,
) -> u64 {
    if path_obj == 0 {
        return 0;
    }
    let path_str = match crate::cfstring::extract_cfstring_cstr(path_obj) {
        Some(s) => s,
        None => return 0,
    };
    match std::fs::read(path_str) {
        Ok(bytes) => make_nsdata(&bytes),
        Err(e) => {
            log::error!("NSData dataWithContentsOfFile: failed: {}: {}", path_str, e);
            0
        }
    }
}

/// `-[NSData writeToFile:options:error:]`
pub extern "C" fn shim_nsdata_write_to_file_options_error(
    obj: u64,
    _cmd: u64,
    path_obj: u64,
    _options: u64,
    _error: u64,
) -> u8 {
    if obj == 0 || path_obj == 0 {
        return 0;
    }
    let path_str = match crate::cfstring::extract_cfstring_cstr(path_obj) {
        Some(s) => s,
        None => return 0,
    };
    let data = nsdata_bytes_slice(obj);
    match std::fs::write(path_str, data) {
        Ok(()) => 1,
        Err(e) => {
            log::error!("NSData writeToFile:options:error: failed: {}: {}", path_str, e);
            0
        }
    }
}

/// `-[NSData writeToURL:atomically:]`
pub extern "C" fn shim_nsdata_write_to_url_atomically(
    obj: u64,
    _cmd: u64,
    url: u64,
    _atomically: u8,
) -> u8 {
    if obj == 0 || url == 0 {
        return 0;
    }
    // Get the path NSString from the URL, then extract C string.
    let path_nsstring = crate::nsurl::shim_nsurl_path(url, 0);
    if path_nsstring == 0 {
        return 0;
    }
    let path_str = match crate::cfstring::extract_cfstring_cstr(path_nsstring) {
        Some(s) => s,
        None => return 0,
    };
    let data = nsdata_bytes_slice(obj);
    match std::fs::write(path_str, data) {
        Ok(()) => 1,
        Err(e) => {
            log::error!("NSData writeToURL:atomically: failed: {}: {}", path_str, e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn data_with_bytes_roundtrip() {
        let bytes: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        let data = shim_nsdata_data_with_bytes_length(0, 0, bytes.as_ptr() as u64, 4);
        assert_ne!(data, 0);
        assert_eq!(shim_nsdata_length(data, 0), 4);
        let ptr = shim_nsdata_bytes(data, 0);
        assert_ne!(ptr, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 4) };
        assert_eq!(slice, &bytes);
        free_obj(data);
    }

    #[test]
    fn empty_data() {
        let data = shim_nsdata_data(0, 0);
        assert_ne!(data, 0);
        assert_eq!(shim_nsdata_length(data, 0), 0);
        free_obj(data);
    }

    #[test]
    fn make_nsdata_helper() {
        let data = make_nsdata(b"hello");
        assert_eq!(shim_nsdata_length(data, 0), 5);
        let ptr = shim_nsdata_bytes(data, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 5) };
        assert_eq!(slice, b"hello");
        free_obj(data);
    }

    #[test]
    fn make_nsdata_empty() {
        let data = make_nsdata(b"");
        assert_eq!(shim_nsdata_length(data, 0), 0);
        free_obj(data);
    }

    #[test]
    fn is_equal_to_data_same_content() {
        let a = make_nsdata(b"abc");
        let b = make_nsdata(b"abc");
        assert_eq!(shim_nsdata_is_equal_to_data(a, 0, b), 1);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_data_different() {
        let a = make_nsdata(b"abc");
        let b = make_nsdata(b"xyz");
        assert_eq!(shim_nsdata_is_equal_to_data(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_data_different_lengths() {
        let a = make_nsdata(b"ab");
        let b = make_nsdata(b"abc");
        assert_eq!(shim_nsdata_is_equal_to_data(a, 0, b), 0);
        free_obj(a);
        free_obj(b);
    }

    #[test]
    fn is_equal_to_data_self() {
        let a = make_nsdata(b"test");
        assert_eq!(shim_nsdata_is_equal_to_data(a, 0, a), 1);
        free_obj(a);
    }

    #[test]
    fn nil_returns_zero() {
        assert_eq!(shim_nsdata_length(0, 0), 0);
        assert_eq!(shim_nsdata_bytes(0, 0), 0);
        // nil == nil via pointer equality (obj == other check fires first)
        assert_eq!(shim_nsdata_is_equal_to_data(0, 0, 0), 1);
        // nil vs non-nil
        let a = make_nsdata(b"x");
        assert_eq!(shim_nsdata_is_equal_to_data(0, 0, a), 0);
        assert_eq!(shim_nsdata_is_equal_to_data(a, 0, 0), 0);
        free_obj(a);
    }

    #[test]
    fn description_format() {
        let data = make_nsdata(b"\xDE\xAD");
        let desc = shim_nsdata_description(data, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<dead>");
        free_obj(data);
    }

    // ── base64 encode / decode ─────────────────────────────────────

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_hello() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn base64_roundtrip() {
        let input = b"The quick brown fox jumps over the lazy dog";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn base64_decode_invalid() {
        // Odd length (not multiple of 4)
        assert!(base64_decode("abc").is_none());
    }

    #[test]
    fn base64_decode_empty() {
        assert_eq!(base64_decode("").unwrap(), b"");
    }

    // ── base64EncodedStringWithOptions: ────────────────────────────

    #[test]
    fn base64_encoded_string() {
        let data = make_nsdata(b"Hello");
        let s = shim_nsdata_base64_encoded_string_with_options(data, 0, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "SGVsbG8=");
        free_obj(data);
    }

    #[test]
    fn base64_encoded_string_nil() {
        assert_eq!(shim_nsdata_base64_encoded_string_with_options(0, 0, 0), 0);
    }

    #[test]
    fn base64_encoded_string_empty() {
        let data = make_nsdata(b"");
        let s = shim_nsdata_base64_encoded_string_with_options(data, 0, 0);
        assert_ne!(s, 0);
        assert_string_eq(s, "");
        free_obj(data);
    }

    // ── initWithBase64EncodedString:options: ───────────────────────

    #[test]
    fn init_with_base64_roundtrip() {
        // Allocate a bare NSData object, then init it from base64
        let obj = alloc_obj(24, crate::nsobj::nsdata_class_addr());
        let b64 = make_string("SGVsbG8="); // "Hello"
        let result = shim_nsdata_init_with_base64_encoded_string(obj, 0, b64, 0);
        assert_eq!(result, obj);
        assert_eq!(shim_nsdata_length(obj, 0), 5);
        let ptr = shim_nsdata_bytes(obj, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 5) };
        assert_eq!(slice, b"Hello");
        free_obj(obj);
    }

    #[test]
    fn init_with_base64_nil_obj() {
        assert_eq!(shim_nsdata_init_with_base64_encoded_string(0, 0, 0, 0), 0);
    }

    #[test]
    fn init_with_base64_nil_string() {
        let obj = alloc_obj(24, crate::nsobj::nsdata_class_addr());
        assert_eq!(shim_nsdata_init_with_base64_encoded_string(obj, 0, 0, 0), 0);
        free_obj(obj);
    }

    // ── base64EncodedDataWithOptions: ──────────────────────────────

    #[test]
    fn base64_encoded_data() {
        let data = make_nsdata(b"Hello");
        let encoded = shim_nsdata_base64_encoded_data_with_options(data, 0, 0);
        assert_ne!(encoded, 0);
        let len = shim_nsdata_length(encoded, 0) as usize;
        let ptr = shim_nsdata_bytes(encoded, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        assert_eq!(slice, b"SGVsbG8=");
        free_obj(data);
        free_obj(encoded);
    }

    #[test]
    fn base64_encoded_data_nil() {
        assert_eq!(shim_nsdata_base64_encoded_data_with_options(0, 0, 0), 0);
    }

    // ── subdataWithRange: ──────────────────────────────────────────

    #[test]
    fn subdata_with_range() {
        let data = make_nsdata(b"Hello, World!");
        let sub = shim_nsdata_subdata_with_range(data, 0, 7, 5);
        assert_ne!(sub, 0);
        assert_eq!(shim_nsdata_length(sub, 0), 5);
        let ptr = shim_nsdata_bytes(sub, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 5) };
        assert_eq!(slice, b"World");
        free_obj(data);
        free_obj(sub);
    }

    #[test]
    fn subdata_with_range_empty() {
        let data = make_nsdata(b"abc");
        let sub = shim_nsdata_subdata_with_range(data, 0, 1, 0);
        assert_ne!(sub, 0);
        assert_eq!(shim_nsdata_length(sub, 0), 0);
        free_obj(data);
        free_obj(sub);
    }

    #[test]
    fn subdata_with_range_out_of_bounds() {
        let data = make_nsdata(b"abc");
        let sub = shim_nsdata_subdata_with_range(data, 0, 0, 100);
        // Out of bounds → empty data
        assert_ne!(sub, 0);
        assert_eq!(shim_nsdata_length(sub, 0), 0);
        free_obj(data);
        free_obj(sub);
    }

    #[test]
    fn subdata_with_range_nil() {
        assert_eq!(shim_nsdata_subdata_with_range(0, 0, 0, 5), 0);
    }

    // ── getBytes:length: ───────────────────────────────────────────

    #[test]
    fn get_bytes_length() {
        let data = make_nsdata(b"ABCDE");
        let buf = alloc_u64_buf(2); // 16 bytes
        shim_nsdata_get_bytes_length(data, 0, buf as u64, 3);
        let slice = unsafe { std::slice::from_raw_parts(buf as *const u8, 3) };
        assert_eq!(slice, b"ABC");
        free_buf(buf);
        free_obj(data);
    }

    #[test]
    fn get_bytes_length_clamps() {
        let data = make_nsdata(b"AB");
        let buf = alloc_u64_buf(2);
        shim_nsdata_get_bytes_length(data, 0, buf as u64, 100);
        let slice = unsafe { std::slice::from_raw_parts(buf as *const u8, 2) };
        assert_eq!(slice, b"AB");
        free_buf(buf);
        free_obj(data);
    }

    #[test]
    fn get_bytes_length_nil() {
        assert_eq!(shim_nsdata_get_bytes_length(0, 0, 1, 1), 0);
    }

    // ── getBytes:range: ────────────────────────────────────────────

    #[test]
    fn get_bytes_range() {
        let data = make_nsdata(b"Hello, World!");
        let buf = alloc_u64_buf(2);
        shim_nsdata_get_bytes_range(data, 0, buf as u64, 7, 5);
        let slice = unsafe { std::slice::from_raw_parts(buf as *const u8, 5) };
        assert_eq!(slice, b"World");
        free_buf(buf);
        free_obj(data);
    }

    #[test]
    fn get_bytes_range_nil() {
        assert_eq!(shim_nsdata_get_bytes_range(0, 0, 1, 0, 1), 0);
    }

    // ── dataWithContentsOfFile: ────────────────────────────────────

    #[test]
    fn data_with_contents_of_file_roundtrip() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("cf_lite_nsdata_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_read.bin");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"test-data-payload").unwrap();
        }
        let path_str = path.to_str().unwrap();
        let ns_path = make_string(path_str);
        let data = shim_nsdata_data_with_contents_of_file(0, 0, ns_path);
        assert_ne!(data, 0);
        assert_eq!(shim_nsdata_length(data, 0), 17);
        let ptr = shim_nsdata_bytes(data, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 17) };
        assert_eq!(slice, b"test-data-payload");
        free_obj(data);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn data_with_contents_of_file_nil() {
        assert_eq!(shim_nsdata_data_with_contents_of_file(0, 0, 0), 0);
    }

    #[test]
    fn data_with_contents_of_file_missing() {
        let ns_path = make_string("/tmp/cf_lite_nsdata_test_DOES_NOT_EXIST_12345.bin");
        assert_eq!(shim_nsdata_data_with_contents_of_file(0, 0, ns_path), 0);
    }

    // ── writeToFile:options:error: ─────────────────────────────────

    #[test]
    fn write_to_file_options_error() {
        let dir = std::env::temp_dir().join("cf_lite_nsdata_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_write_opts.bin");
        let data = make_nsdata(b"write-opts-test");
        let ns_path = make_string(path.to_str().unwrap());
        let result = shim_nsdata_write_to_file_options_error(data, 0, ns_path, 0, 0);
        assert_eq!(result, 1);
        let contents = std::fs::read(&path).unwrap();
        assert_eq!(contents, b"write-opts-test");
        free_obj(data);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_to_file_options_error_nil() {
        assert_eq!(shim_nsdata_write_to_file_options_error(0, 0, 0, 0, 0), 0);
    }

    // ── writeToURL:atomically: ─────────────────────────────────────

    #[test]
    fn write_to_url_atomically() {
        let dir = std::env::temp_dir().join("cf_lite_nsdata_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_write_url.bin");
        let data = make_nsdata(b"url-write-test");
        // Create a file URL via NSURL
        let ns_path = make_string(path.to_str().unwrap());
        let url = crate::nsurl::shim_nsurl_file_url_with_path(0, 0, ns_path);
        assert_ne!(url, 0);
        let result = shim_nsdata_write_to_url_atomically(data, 0, url, 1);
        assert_eq!(result, 1);
        let contents = std::fs::read(&path).unwrap();
        assert_eq!(contents, b"url-write-test");
        free_obj(data);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_to_url_atomically_nil() {
        assert_eq!(shim_nsdata_write_to_url_atomically(0, 0, 0, 0), 0);
    }
}
