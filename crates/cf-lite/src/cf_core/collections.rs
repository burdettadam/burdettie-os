//! CF collection functions: CFArray, CFDictionary, CFData, CFURL, CFBundle, CFBoolean.

use std::path::Path;

use crate::nsobj;

// ── CFArray C functions ────────────────────────────────────────────

/// `CFArrayRef CFArrayCreate(CFAllocatorRef, const void **values, CFIndex numValues, const CFArrayCallBacks *)`
pub extern "C" fn shim_cf_array_create(
    _alloc: u64,
    values: *const u64,
    num_values: i64,
    _callbacks: u64,
) -> u64 {
    if num_values <= 0 || values.is_null() {
        return crate::nsarray::make_nsarray(&[]);
    }
    let items = unsafe { std::slice::from_raw_parts(values, num_values as usize) };
    crate::nsarray::make_nsarray(items)
}

/// `CFIndex CFArrayGetCount(CFArrayRef theArray)`
pub extern "C" fn shim_cf_array_get_count(array: u64) -> i64 {
    if array == 0 {
        return 0;
    }
    unsafe { *((array as usize + 8) as *const u64) as i64 }
}

/// `const void *CFArrayGetValueAtIndex(CFArrayRef theArray, CFIndex idx)`
pub extern "C" fn shim_cf_array_get_value_at_index(array: u64, idx: i64) -> u64 {
    if array == 0 {
        return 0;
    }
    let count = unsafe { *((array as usize + 8) as *const u64) };
    if idx < 0 || idx as u64 >= count {
        return 0;
    }
    let elems = unsafe { *((array as usize + 16) as *const u64) };
    if elems == 0 {
        return 0;
    }
    unsafe { *((elems as usize + idx as usize * 8) as *const u64) }
}

// ── CFDictionary C functions ───────────────────────────────────────

/// `CFDictionaryRef CFDictionaryCreate(CFAllocatorRef, const void **keys, const void **values, CFIndex numValues, ...)`
pub extern "C" fn shim_cf_dictionary_create(
    _alloc: u64,
    keys: *const u64,
    values: *const u64,
    num_values: i64,
    _key_cbs: u64,
    _val_cbs: u64,
) -> u64 {
    if num_values <= 0 || keys.is_null() || values.is_null() {
        return crate::nsdictionary::make_nsdictionary(&[], &[]);
    }
    let n = num_values as usize;
    let k_slice = unsafe { std::slice::from_raw_parts(keys, n) };
    let v_slice = unsafe { std::slice::from_raw_parts(values, n) };
    crate::nsdictionary::make_nsdictionary(k_slice, v_slice)
}

/// `CFIndex CFDictionaryGetCount(CFDictionaryRef theDict)`
pub extern "C" fn shim_cf_dictionary_get_count(dict: u64) -> i64 {
    if dict == 0 {
        return 0;
    }
    unsafe { *((dict as usize + 8) as *const u64) as i64 }
}

/// `const void *CFDictionaryGetValue(CFDictionaryRef theDict, const void *key)`
pub extern "C" fn shim_cf_dictionary_get_value(dict: u64, key: u64) -> u64 {
    if dict == 0 || key == 0 {
        return 0;
    }
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

/// `Boolean CFDictionaryGetValueIfPresent(CFDictionaryRef theDict, const void *key, const void **value)`
pub extern "C" fn shim_cf_dictionary_get_value_if_present(
    dict: u64,
    key: u64,
    value_out: u64,
) -> u8 {
    let val = shim_cf_dictionary_get_value(dict, key);
    if val == 0 {
        return 0; // false
    }
    if value_out != 0 {
        unsafe {
            *(value_out as *mut u64) = val;
        }
    }
    1 // true
}

// ── CFData C functions ─────────────────────────────────────────────

/// `CFDataRef CFDataCreate(CFAllocatorRef, const UInt8 *bytes, CFIndex length)`
pub extern "C" fn shim_cf_data_create(_alloc: u64, bytes: *const u8, length: i64) -> u64 {
    if length <= 0 || bytes.is_null() {
        return crate::nsdata::make_nsdata(&[]);
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, length as usize) };
    crate::nsdata::make_nsdata(slice)
}

/// `const UInt8 *CFDataGetBytePtr(CFDataRef theData)`
pub extern "C" fn shim_cf_data_get_byte_ptr(data: u64) -> u64 {
    if data == 0 {
        return 0;
    }
    unsafe { *((data as usize + 16) as *const u64) }
}

/// `CFIndex CFDataGetLength(CFDataRef theData)`
pub extern "C" fn shim_cf_data_get_length(data: u64) -> i64 {
    if data == 0 {
        return 0;
    }
    unsafe { *((data as usize + 8) as *const u64) as i64 }
}

// ── CFURL C functions ──────────────────────────────────────────────

/// `CFURLRef CFURLCreateWithString(CFAllocatorRef, CFStringRef urlString, CFURLRef baseURL)`
pub extern "C" fn shim_cf_url_create_with_string(
    _alloc: u64,
    url_string: u64,
    _base_url: u64,
) -> u64 {
    if url_string == 0 {
        return 0;
    }
    // Delegate to NSURL's URLWithString: — same object, toll-free
    crate::nsurl::shim_nsurl_url_with_string(nsobj::nsurl_class_addr(), 0, url_string)
}

/// `CFStringRef CFURLGetString(CFURLRef anURL)`
pub extern "C" fn shim_cf_url_get_string(url: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_absolute_string(url, 0)
}

/// `CFURLRef CFURLCreateWithFileSystemPath(CFAllocatorRef, CFStringRef path, CFURLPathStyle, Boolean isDirectory)`
pub extern "C" fn shim_cf_url_create_with_file_system_path(
    _alloc: u64,
    path: u64,
    _path_style: i64,
    _is_dir: u8,
) -> u64 {
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, path)
}

/// `CFURLRef CFURLCopyAbsoluteURL(CFURLRef url)`
pub extern "C" fn shim_cf_url_copy_absolute_url(url: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_absolute_url(url, 0)
}

/// `CFStringRef CFURLCopyFileSystemPath(CFURLRef url, CFURLPathStyle pathStyle)`
pub extern "C" fn shim_cf_url_copy_file_system_path(url: u64, _path_style: i64) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_path(url, 0)
}

/// `CFStringRef CFURLCopyLastPathComponent(CFURLRef url)`
pub extern "C" fn shim_cf_url_copy_last_path_component(url: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_last_path_component(url, 0)
}

/// `CFStringRef CFURLCopyPathExtension(CFURLRef url)`
pub extern "C" fn shim_cf_url_copy_path_extension(url: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_path_extension(url, 0)
}

/// `CFURLRef CFURLCreateCopyAppendingPathComponent(CFAllocatorRef alloc, CFURLRef url, CFStringRef pathComponent, Boolean isDirectory)`
pub extern "C" fn shim_cf_url_create_copy_appending_path_component(
    _alloc: u64,
    url: u64,
    path_component: u64,
    _is_directory: u8,
) -> u64 {
    if url == 0 || path_component == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_url_by_appending_path_component(url, 0, path_component)
}

/// `CFURLRef CFURLCreateCopyDeletingLastPathComponent(CFAllocatorRef alloc, CFURLRef url)`
pub extern "C" fn shim_cf_url_create_copy_deleting_last_path_component(
    _alloc: u64,
    url: u64,
) -> u64 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_url_by_deleting_last_path_component(url, 0)
}

/// `CFURLRef CFURLCreateFromFileSystemRepresentation(CFAllocatorRef alloc, const UInt8 *buffer, CFIndex bufLen, Boolean isDirectory)`
pub extern "C" fn shim_cf_url_create_from_file_system_representation(
    _alloc: u64,
    buffer: *const u8,
    buf_len: i64,
    is_directory: u8,
) -> u64 {
    if buffer.is_null() || buf_len < 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(buffer, buf_len as usize) };
    let path = String::from_utf8_lossy(slice);
    let cstr = std::ffi::CString::new(path.as_ref()).unwrap_or_default();
    let path_string = crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
    crate::nsurl::shim_nsurl_file_url_with_path_is_directory(
        nsobj::nsurl_class_addr(),
        0,
        path_string,
        is_directory,
    )
}

/// `CFURLRef CFURLCreateFilePathURL(CFAllocatorRef alloc, CFURLRef url, CFErrorRef *error)`
pub extern "C" fn shim_cf_url_create_file_path_url(_alloc: u64, url: u64, _error: u64) -> u64 {
    if url == 0 {
        return 0;
    }
    if crate::nsurl::shim_nsurl_is_file_url(url, 0) != 0 {
        return crate::nsurl::shim_nsurl_absolute_url(url, 0);
    }
    let path = crate::nsurl::shim_nsurl_path(url, 0);
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, path)
}

/// `CFURLRef CFURLCreateFileReferenceURL(CFAllocatorRef alloc, CFURLRef url, CFErrorRef *error)`
pub extern "C" fn shim_cf_url_create_file_reference_url(alloc: u64, url: u64, error: u64) -> u64 {
    shim_cf_url_create_file_path_url(alloc, url, error)
}

/// `Boolean CFURLResourceIsReachable(CFURLRef url, CFErrorRef *error)`
pub extern "C" fn shim_cf_url_resource_is_reachable(url: u64, error: u64) -> u8 {
    if url == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_check_resource_is_reachable(url, 0, error)
}

// ── CFBundle C functions ───────────────────────────────────────────

/// `CFBundleRef CFBundleGetMainBundle(void)`
pub extern "C" fn shim_cf_bundle_get_main_bundle() -> u64 {
    crate::nsbundle::shim_nsbundle_main_bundle(nsobj::nsbundle_class_addr(), 0)
}

/// `CFURLRef CFBundleCopyBundleURL(CFBundleRef bundle)`
pub extern "C" fn shim_cf_bundle_copy_bundle_url(bundle: u64) -> u64 {
    if bundle == 0 {
        return 0;
    }
    let path = crate::nsbundle::shim_nsbundle_bundle_path(bundle, 0);
    if path == 0 {
        return 0;
    }
    crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, path)
}

/// `CFDictionaryRef CFBundleCopyInfoDictionaryForURL(CFURLRef url)`
pub extern "C" fn shim_cf_bundle_copy_info_dictionary_for_url(url: u64) -> u64 {
    if url == 0 {
        log::debug!(
            "CFBundleCopyInfoDictionaryForURL: nil URL; falling back to process executable path"
        );
        if let Some(path) = libsystem_shims::process::process_path() {
            return copy_info_dictionary_for_path(path);
        }
        return 0;
    }

    let path_ns = crate::nsurl::shim_nsurl_path(url, 0);
    if path_ns == 0 {
        log::debug!("CFBundleCopyInfoDictionaryForURL: no path for URL {url:#x}");
        return 0;
    }

    let Some(path) = crate::cfstring::extract_cfstring_cstr(path_ns) else {
        log::debug!("CFBundleCopyInfoDictionaryForURL: non-string path for URL {url:#x}");
        return 0;
    };

    copy_info_dictionary_for_path(path)
}

fn copy_info_dictionary_for_path(path: &str) -> u64 {
    log::debug!("CFBundleCopyInfoDictionaryForURL: path={path}");

    let path_obj = Path::new(path);
    if path_obj.is_file() {
        let dict = crate::nspropertylistserialization::parse_embedded_plist_file(path);
        if dict != 0 {
            log::debug!(
                "CFBundleCopyInfoDictionaryForURL: using embedded plist from executable {path}"
            );
            return dict;
        }
    }

    let bundle_path = if path_obj
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".app"))
    {
        Some(path.to_string())
    } else if let Some(root) = crate::nsbundle::detect_app_bundle_root(path) {
        Some(root)
    } else if path_obj.join("Contents/Info.plist").exists() || path_obj.join("Info.plist").exists()
    {
        Some(path.to_string())
    } else {
        path_obj.parent().and_then(|parent| {
            let has_plist =
                parent.join("Contents/Info.plist").exists() || parent.join("Info.plist").exists();
            has_plist.then(|| parent.to_string_lossy().to_string())
        })
    };

    let Some(bundle_path) = bundle_path else {
        log::debug!("CFBundleCopyInfoDictionaryForURL: no bundle path resolved for {path}");
        return 0;
    };

    log::debug!("CFBundleCopyInfoDictionaryForURL: resolved bundle path={bundle_path}");

    let cstr = std::ffi::CString::new(bundle_path.clone()).unwrap_or_default();
    let bundle_path_ns = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
    if bundle_path_ns == 0 {
        return 0;
    }

    let bundle = crate::nsbundle::shim_nsbundle_bundle_with_path(
        nsobj::nsbundle_class_addr(),
        0,
        bundle_path_ns,
    );
    if bundle == 0 {
        log::debug!("CFBundleCopyInfoDictionaryForURL: bundleWithPath failed for {bundle_path}");
        return 0;
    }

    let dict = crate::nsbundle::shim_nsbundle_info_dictionary(bundle, 0);
    log::debug!("CFBundleCopyInfoDictionaryForURL: infoDictionary result={dict:#x}");
    dict
}

// ── kCFBooleanTrue/False data symbols ──────────────────────────────

use std::sync::LazyLock;

/// Boolean singletons — real NSNumber objects with ISA + tag(2=bool) + value.
/// Layout: [isa: u64, tag: u64, value: u64] = 24 bytes.
static K_CF_BOOLEAN_TRUE_OBJ: LazyLock<[u64; 3]> =
    LazyLock::new(|| [nsobj::nsnumber_class_addr(), 2, 1]);
static K_CF_BOOLEAN_FALSE_OBJ: LazyLock<[u64; 3]> =
    LazyLock::new(|| [nsobj::nsnumber_class_addr(), 2, 0]);

/// Address of kCFBooleanTrue — a real NSNumber(bool=1) object.
pub fn kcf_boolean_true_addr() -> u64 {
    &*K_CF_BOOLEAN_TRUE_OBJ as *const [u64; 3] as u64
}

/// Address of kCFBooleanFalse — a real NSNumber(bool=0) object.
pub fn kcf_boolean_false_addr() -> u64 {
    &*K_CF_BOOLEAN_FALSE_OBJ as *const [u64; 3] as u64
}

/// `kCFNull` singleton — backed by `NSNull.null`.
pub fn kcf_null_addr() -> u64 {
    crate::nsnull::shim_nsnull_null(nsobj::nsnull_class_addr(), 0)
}

static K_CF_TYPE_ARRAY_CALLBACKS: [u64; 5] = [0; 5];

/// `kCFTypeArrayCallBacks` static callback table.
pub fn kcf_type_array_callbacks_addr() -> u64 {
    &K_CF_TYPE_ARRAY_CALLBACKS as *const [u64; 5] as u64
}

// ── CFNumber C functions ───────────────────────────────────────────

// CFNumber type constants — subset of `CFNumberType` enum.
// kCFNumberSInt8Type = 1, ..., kCFNumberDoubleType = 13, kCFNumberNSIntegerType = 15.

/// `CFNumberRef CFNumberCreate(CFAllocatorRef, CFNumberType, const void *valuePtr)`
pub extern "C" fn shim_cf_number_create(_alloc: u64, the_type: i32, value_ptr: *const u8) -> u64 {
    if value_ptr.is_null() {
        return 0;
    }
    match the_type {
        // SInt8 (1), SInt16 (2), SInt32 (3), CharType (7), ShortType (8), IntType (9)
        1 | 7 => {
            let v = unsafe { *(value_ptr as *const i8) } as i64;
            crate::nsnumber::make_nsnumber_int(v)
        }
        2 | 8 => {
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const i16) } as i64;
            crate::nsnumber::make_nsnumber_int(v)
        }
        3 | 9 => {
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const i32) } as i64;
            crate::nsnumber::make_nsnumber_int(v)
        }
        // SInt64 (4), LongType (10), LongLongType (11), NSIntegerType (15), CFIndexType (14)
        4 | 10 | 11 | 14 | 15 => {
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const i64) };
            crate::nsnumber::make_nsnumber_int(v)
        }
        // Float32Type (5), FloatType (12)
        5 | 12 => {
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const f32) } as f64;
            crate::nsnumber::make_nsnumber_double(v)
        }
        // Float64Type (6), DoubleType (13)
        6 | 13 => {
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const f64) };
            crate::nsnumber::make_nsnumber_double(v)
        }
        _ => {
            // Unknown type — try to read as i64
            let v = unsafe { std::ptr::read_unaligned(value_ptr as *const i64) };
            crate::nsnumber::make_nsnumber_int(v)
        }
    }
}

/// `Boolean CFNumberGetValue(CFNumberRef number, CFNumberType theType, void *valuePtr)`
pub extern "C" fn shim_cf_number_get_value(number: u64, the_type: i32, value_ptr: *mut u8) -> u8 {
    if number == 0 || value_ptr.is_null() {
        return 0;
    }
    let (tag, raw) = crate::nsnumber::read_number_pub(number);
    match the_type {
        1 | 7 => {
            let v: i8 = match tag {
                1 => f64::from_bits(raw) as i8,
                _ => raw as i64 as i8,
            };
            unsafe {
                *(value_ptr as *mut i8) = v;
            }
        }
        2 | 8 => {
            let v: i16 = match tag {
                1 => f64::from_bits(raw) as i16,
                _ => raw as i64 as i16,
            };
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut i16, v);
            }
        }
        3 | 9 => {
            let v: i32 = match tag {
                1 => f64::from_bits(raw) as i32,
                _ => raw as i64 as i32,
            };
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut i32, v);
            }
        }
        4 | 10 | 11 | 14 | 15 => {
            let v: i64 = match tag {
                1 => f64::from_bits(raw) as i64,
                _ => raw as i64,
            };
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut i64, v);
            }
        }
        5 | 12 => {
            let v: f32 = match tag {
                1 => f64::from_bits(raw) as f32,
                _ => raw as i64 as f32,
            };
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut f32, v);
            }
        }
        6 | 13 => {
            let v: f64 = match tag {
                1 => f64::from_bits(raw),
                _ => raw as i64 as f64,
            };
            unsafe {
                std::ptr::write_unaligned(value_ptr as *mut f64, v);
            }
        }
        _ => return 0,
    }
    1
}

/// `CFNumberType CFNumberGetType(CFNumberRef number)`
pub extern "C" fn shim_cf_number_get_type(number: u64) -> i32 {
    if number == 0 {
        return 0;
    }
    let (tag, _) = crate::nsnumber::read_number_pub(number);
    match tag {
        0 | 2 => 4, // SInt64 for int and bool
        1 => 13,    // kCFNumberDoubleType
        _ => 4,
    }
}

/// `CFComparisonResult CFNumberCompare(CFNumberRef, CFNumberRef, void *)`
pub extern "C" fn shim_cf_number_compare(a: u64, b: u64, _context: u64) -> i32 {
    if a == 0 || b == 0 {
        return 0;
    }
    let va = crate::nsnumber::shim_nsnumber_double_value(a, 0);
    let vb = crate::nsnumber::shim_nsnumber_double_value(b, 0);
    if va < vb {
        -1
    } else if va > vb {
        1
    } else {
        0
    }
}

/// `Boolean CFNumberIsFloatType(CFNumberRef number)`
pub extern "C" fn shim_cf_number_is_float_type(number: u64) -> u8 {
    if number == 0 {
        return 0;
    }
    let (tag, _) = crate::nsnumber::read_number_pub(number);
    u8::from(tag == 1) // TAG_DOUBLE
}

// ── CFString expanded C functions ──────────────────────────────────

/// `Boolean CFStringGetCString(CFStringRef, char *buffer, CFIndex bufferSize, CFStringEncoding)`
pub extern "C" fn shim_cf_string_get_cstring(
    string: u64,
    buffer: *mut u8,
    buffer_size: i64,
    _encoding: u32,
) -> u8 {
    if string == 0 || buffer.is_null() || buffer_size <= 0 {
        return 0;
    }
    let c_str_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(string, 0);
    if c_str_ptr.is_null() {
        // Null-terminate the empty buffer
        unsafe {
            *buffer = 0;
        }
        return 0;
    }
    let src = unsafe { std::ffi::CStr::from_ptr(c_str_ptr) };
    let src_bytes = src.to_bytes_with_nul();
    let max = buffer_size as usize;
    if src_bytes.len() > max {
        // Doesn't fit — truncate and null-terminate
        unsafe {
            std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), buffer, max - 1);
            *buffer.add(max - 1) = 0;
        }
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), buffer, src_bytes.len());
    }
    1
}

/// `CFStringRef CFStringCreateCopy(CFAllocatorRef, CFStringRef)`
pub extern "C" fn shim_cf_string_create_copy(_alloc: u64, string: u64) -> u64 {
    if string == 0 {
        return 0;
    }
    // Our CFStrings are immutable with refcounting — just retain and return.
    super::shim_cf_retain(string)
}

/// `CFStringRef CFStringCreateWithBytesNoCopy(CFAllocatorRef alloc, const UInt8 *bytes, CFIndex numBytes, CFStringEncoding encoding, Boolean isExternalRepresentation, CFAllocatorRef contentsDeallocator)`
pub extern "C" fn shim_cf_string_create_with_bytes_no_copy(
    _alloc: u64,
    bytes: *const u8,
    num_bytes: i64,
    encoding: u32,
    _is_external_representation: u8,
    _contents_deallocator: u64,
) -> u64 {
    if num_bytes < 0 {
        return 0;
    }
    if bytes.is_null() {
        return if num_bytes == 0 {
            crate::cfstring::shim_cf_string_create_with_cstring(0, c"".as_ptr(), 0)
        } else {
            0
        };
    }
    if encoding != crate::cfstring::K_CF_STRING_ENCODING_UTF8 && encoding != 0 {
        log::warn!(
            "CFStringCreateWithBytesNoCopy: unsupported encoding {encoding:#x}, treating as UTF-8"
        );
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, num_bytes as usize) };
    let text = String::from_utf8_lossy(slice).replace('\0', "");
    let cstr = std::ffi::CString::new(text).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
}

/// `CFComparisonResult CFStringCompare(CFStringRef, CFStringRef, CFStringCompareFlags)`
pub extern "C" fn shim_cf_string_compare(a: u64, b: u64, flags: u64) -> i32 {
    if a == 0 && b == 0 {
        return 0;
    }
    if a == 0 {
        return -1;
    }
    if b == 0 {
        return 1;
    }
    let a_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(a, 0);
    let b_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(b, 0);
    if a_ptr.is_null() || b_ptr.is_null() {
        return 0;
    }
    let a_str = unsafe { std::ffi::CStr::from_ptr(a_ptr) }.to_string_lossy();
    let b_str = unsafe { std::ffi::CStr::from_ptr(b_ptr) }.to_string_lossy();

    let case_insensitive = flags & 1 != 0; // kCFCompareCaseInsensitive = 1
    let result = if case_insensitive {
        a_str.to_lowercase().cmp(&b_str.to_lowercase())
    } else {
        a_str.cmp(&b_str)
    };
    match result {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `CFStringRef CFStringCreateWithCharacters(CFAllocatorRef, const UniChar *, CFIndex)`
pub extern "C" fn shim_cf_string_create_with_characters(
    _alloc: u64,
    chars: *const u16,
    num_chars: i64,
) -> u64 {
    if chars.is_null() || num_chars <= 0 {
        return crate::cfstring::shim_cf_string_create_with_cstring(0, c"".as_ptr(), 0);
    }
    let slice = unsafe { std::slice::from_raw_parts(chars, num_chars as usize) };
    let s = String::from_utf16_lossy(slice);
    let cstr = std::ffi::CString::new(s).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
}

/// `UniChar CFStringGetCharacterAtIndex(CFStringRef, CFIndex)`
pub extern "C" fn shim_cf_string_get_character_at_index(string: u64, idx: i64) -> u16 {
    if string == 0 || idx < 0 {
        return 0;
    }
    let c_str_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(string, 0);
    if c_str_ptr.is_null() {
        return 0;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(c_str_ptr) }.to_string_lossy();
    let utf16: Vec<u16> = s.encode_utf16().collect();
    utf16.get(idx as usize).copied().unwrap_or(0)
}

/// `CFRange CFStringFind(CFStringRef string, CFStringRef stringToFind, CFStringCompareFlags)`
/// Returns a CFRange (location: i64, length: i64). Packed into two i64s.
/// We return location in the low 64 bits and length in the high 64 bits via
/// a pointer argument (Apple ABI returns large structs via sret pointer).
pub extern "C" fn shim_cf_string_find(string: u64, to_find: u64, flags: u64) -> u64 {
    // Returns kCFNotFound (-1) packed: location = -1, length = 0
    const NOT_FOUND: u64 = u64::MAX; // location=-1 as unsigned
    if string == 0 || to_find == 0 {
        return NOT_FOUND;
    }
    let a_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(string, 0);
    let b_ptr = crate::cfstring::shim_cf_string_get_cstring_ptr(to_find, 0);
    if a_ptr.is_null() || b_ptr.is_null() {
        return NOT_FOUND;
    }
    let haystack = unsafe { std::ffi::CStr::from_ptr(a_ptr) }.to_string_lossy();
    let needle = unsafe { std::ffi::CStr::from_ptr(b_ptr) }.to_string_lossy();

    let case_insensitive = flags & 1 != 0;
    let pos = if case_insensitive {
        haystack.to_lowercase().find(&needle.to_lowercase())
    } else {
        haystack.find(needle.as_ref())
    };

    match pos {
        Some(loc) => loc as u64, // location; length would be in high bits for full struct
        None => NOT_FOUND,
    }
}

// ── CFRunLoop C functions ──────────────────────────────────────────

/// `CFRunLoopRef CFRunLoopGetCurrent(void)`
pub extern "C" fn shim_cf_run_loop_get_current() -> u64 {
    crate::nsrunloop::shim_nsrunloop_current_run_loop(0, 0)
}

/// `CFRunLoopRef CFRunLoopGetMain(void)`
pub extern "C" fn shim_cf_run_loop_get_main() -> u64 {
    crate::nsrunloop::shim_nsrunloop_main_run_loop(0, 0)
}

/// `void CFRunLoopRun(void)`
pub extern "C" fn shim_cf_run_loop_run() {
    let rl = shim_cf_run_loop_get_current();
    crate::nsrunloop::shim_nsrunloop_run(rl, 0);
}

/// `SInt32 CFRunLoopRunInMode(CFStringRef mode, CFTimeInterval seconds, Boolean returnAfterSourceHandled)`
pub extern "C" fn shim_cf_run_loop_run_in_mode(
    _mode: u64,
    seconds: f64,
    return_after_source: u8,
) -> i32 {
    let deadline = if seconds > 1.0e9 {
        // distantFuture-like value — treat as indefinite
        None
    } else if seconds <= 0.0 {
        Some(std::time::Instant::now())
    } else {
        Some(std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds))
    };
    let single_pass = return_after_source != 0;
    crate::nsrunloop::run_loop_blocking(deadline, single_pass)
}

/// `void CFRunLoopStop(CFRunLoopRef rl)`
pub extern "C" fn shim_cf_run_loop_stop(_rl: u64) {
    log::debug!("CFRunLoopStop called");
    crate::nsrunloop::stop();
}

/// `void CFRunLoopWakeUp(CFRunLoopRef rl)`
pub extern "C" fn shim_cf_run_loop_wakeup(_rl: u64) {
    log::debug!("CFRunLoopWakeUp called");
    crate::nsrunloop::wakeup();
}

/// `Boolean CFRunLoopIsWaiting(CFRunLoopRef rl)`
pub extern "C" fn shim_cf_run_loop_is_waiting(_rl: u64) -> u8 {
    u8::from(crate::nsrunloop::is_waiting())
}

/// `void CFRunLoopAddTimer(CFRunLoopRef rl, CFRunLoopTimerRef timer, CFStringRef mode)`
pub extern "C" fn shim_cf_run_loop_add_timer(rl: u64, timer: u64, mode: u64) {
    let run_loop = if rl == 0 {
        shim_cf_run_loop_get_current()
    } else {
        rl
    };
    crate::nsrunloop::shim_nsrunloop_add_timer_for_mode(run_loop, 0, timer, mode);
}

/// `void CFRunLoopAddSource(CFRunLoopRef rl, CFRunLoopSourceRef source, CFStringRef mode)`
pub extern "C" fn shim_cf_run_loop_add_source(_rl: u64, source: u64, _mode: u64) {
    if source == 0 {
        return;
    }
    crate::nsrunloop::add_source(source);
}

/// `void CFRunLoopRemoveSource(CFRunLoopRef rl, CFRunLoopSourceRef source, CFStringRef mode)`
pub extern "C" fn shim_cf_run_loop_remove_source(_rl: u64, source: u64, _mode: u64) {
    if source == 0 {
        return;
    }
    crate::nsrunloop::remove_source(source);
}

/// `CFRunLoopSourceRef CFRunLoopSourceCreate(CFAllocatorRef alloc, CFIndex order, CFRunLoopSourceContext *ctx)`
///
/// Allocates a version-0 run loop source. The `ctx` points to a
/// `CFRunLoopSourceContext` struct:
/// ```c
/// struct CFRunLoopSourceContext {
///     CFIndex version;             // 0
///     void *info;
///     CFAllocatorRetainCallBack retain;
///     CFAllocatorReleaseCallBack release;
///     CFAllocatorCopyDescriptionCallBack copyDescription;
///     CFRunLoopEqualCallBack equal;
///     CFRunLoopHashCallBack hash;
///     void (*schedule)(void *info, CFRunLoopRef rl, CFStringRef mode);
///     void (*cancel)(void *info, CFRunLoopRef rl, CFStringRef mode);
///     void (*perform)(void *info);
/// };
/// ```
pub extern "C" fn shim_cf_run_loop_source_create(_alloc: u64, _order: i64, ctx: u64) -> u64 {
    crate::nsrunloop::create_source(ctx)
}

/// `void CFRunLoopSourceSignal(CFRunLoopSourceRef source)`
pub extern "C" fn shim_cf_run_loop_source_signal(source: u64) {
    if source == 0 {
        return;
    }
    crate::nsrunloop::signal_source(source);
}

/// `void CFRunLoopSourceInvalidate(CFRunLoopSourceRef source)`
pub extern "C" fn shim_cf_run_loop_source_invalidate(source: u64) {
    if source == 0 {
        return;
    }
    crate::nsrunloop::invalidate_source(source);
}

/// `Boolean CFRunLoopSourceIsValid(CFRunLoopSourceRef source)`
pub extern "C" fn shim_cf_run_loop_source_is_valid(source: u64) -> u8 {
    if source == 0 {
        return 0;
    }
    crate::nsrunloop::source_is_valid(source)
}

/// `CFRunLoopObserverRef CFRunLoopObserverCreate(CFAllocatorRef alloc, CFOptionFlags activities,
///     Boolean repeats, CFIndex order, CFRunLoopObserverCallBack callback, CFRunLoopObserverContext *ctx)`
pub extern "C" fn shim_cf_run_loop_observer_create(
    _alloc: u64,
    activities: u64,
    repeats: u8,
    _order: i64,
    callback: u64,
    ctx: u64,
) -> u64 {
    crate::nsrunloop::create_observer(activities, repeats != 0, callback, ctx)
}

/// `void CFRunLoopAddObserver(CFRunLoopRef rl, CFRunLoopObserverRef observer, CFStringRef mode)`
pub extern "C" fn shim_cf_run_loop_add_observer(_rl: u64, observer: u64, _mode: u64) {
    if observer == 0 {
        return;
    }
    crate::nsrunloop::add_observer(observer);
}

/// `void CFRunLoopRemoveObserver(CFRunLoopRef rl, CFRunLoopObserverRef observer, CFStringRef mode)`
pub extern "C" fn shim_cf_run_loop_remove_observer(_rl: u64, observer: u64, _mode: u64) {
    if observer == 0 {
        return;
    }
    crate::nsrunloop::remove_observer(observer);
}

/// `void CFRunLoopObserverInvalidate(CFRunLoopObserverRef observer)`
pub extern "C" fn shim_cf_run_loop_observer_invalidate(observer: u64) {
    if observer == 0 {
        return;
    }
    crate::nsrunloop::invalidate_observer(observer);
}

/// CFRunLoop mode constants — addresses of pre-allocated NSString objects.
pub fn kcf_run_loop_default_mode_addr() -> u64 {
    crate::nsrunloop::nsdefault_run_loop_mode_addr()
}

pub fn kcf_run_loop_common_modes_addr() -> u64 {
    crate::nsrunloop::nsrun_loop_common_modes_addr()
}

// ── CFAllocator stubs ──────────────────────────────────────────────

/// `CFAllocatorRef CFAllocatorGetDefault(void)`
pub extern "C" fn shim_cf_allocator_get_default() -> u64 {
    crate::cfstring::kcf_allocator_default_addr()
}

/// `CFTimeZoneRef CFTimeZoneCopySystem(void)`
pub extern "C" fn shim_cf_time_zone_copy_system() -> u64 {
    crate::nstimezone::shim_nstimezone_system_timezone(nsobj::nstimezone_class_addr(), 0)
}

/// `CFTimeZoneRef CFTimeZoneCopyDefault(void)`
pub extern "C" fn shim_cf_time_zone_copy_default() -> u64 {
    crate::nstimezone::shim_nstimezone_system_timezone(nsobj::nstimezone_class_addr(), 0)
}

/// `CFStringRef CFTimeZoneGetName(CFTimeZoneRef tz)`
pub extern "C" fn shim_cf_time_zone_get_name(time_zone: u64) -> u64 {
    crate::nstimezone::shim_nstimezone_name(time_zone, 0)
}

/// `CFTimeInterval CFTimeZoneGetSecondsFromGMT(CFTimeZoneRef tz, CFAbsoluteTime at)`
pub extern "C" fn shim_cf_time_zone_get_seconds_from_gmt(time_zone: u64, _at: f64) -> i64 {
    crate::nstimezone::shim_nstimezone_seconds_from_gmt(time_zone, 0)
}

/// `void *CFAllocatorAllocate(CFAllocatorRef, CFIndex size, CFOptionFlags)`
pub extern "C" fn shim_cf_allocator_allocate(_alloc: u64, size: i64, _flags: u64) -> u64 {
    if size <= 0 {
        return 0;
    }
    crate::pal_alloc().calloc(1, size as usize) as u64
}

/// `void CFAllocatorDeallocate(CFAllocatorRef, void *ptr)`
pub extern "C" fn shim_cf_allocator_deallocate(_alloc: u64, ptr: u64) {
    if ptr != 0 {
        unsafe {
            libc::free(ptr as *mut libc::c_void);
        }
    }
}

// ── CFStringGetBytes ───────────────────────────────────────────────

/// `CFIndex CFStringGetBytes(CFStringRef, CFRange range, CFStringEncoding, UInt8 lossByte,
///     Boolean isExternalRepresentation, UInt8 *buffer, CFIndex maxBufLen, CFIndex *usedBufLen)`
///
/// Converts characters from a CFString into a byte buffer.
/// We only support UTF-8 encoding.
pub extern "C" fn shim_cf_string_get_bytes(
    string: u64,
    range_loc: i64,
    range_len: i64,
    _encoding: u32,
    _loss_byte: u8,
    _is_external: u8,
    buffer: u64,
    max_buf_len: i64,
    used_buf_len: u64,
) -> i64 {
    if string == 0 || range_len <= 0 {
        if used_buf_len != 0 {
            unsafe {
                *(used_buf_len as *mut i64) = 0;
            }
        }
        return 0;
    }
    let parts = crate::cfstring::cfstring_parts(string);
    let Some((bytes, total_len)) = parts else {
        if used_buf_len != 0 {
            unsafe {
                *(used_buf_len as *mut i64) = 0;
            }
        }
        return 0;
    };

    let start = range_loc as usize;
    let end = (range_loc + range_len) as usize;
    let end = end.min(total_len);
    if start >= end {
        if used_buf_len != 0 {
            unsafe {
                *(used_buf_len as *mut i64) = 0;
            }
        }
        return 0;
    }
    let slice = &bytes[start..end];
    let copy_len = if buffer != 0 {
        let n = slice.len().min(max_buf_len as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(slice.as_ptr(), buffer as *mut u8, n);
        }
        n
    } else {
        slice.len()
    };
    if used_buf_len != 0 {
        unsafe {
            *(used_buf_len as *mut i64) = copy_len as i64;
        }
    }
    copy_len as i64
}

/// `Boolean CFStringGetFileSystemRepresentation(CFStringRef, char *buffer, CFIndex maxBufLen)`
pub extern "C" fn shim_cf_string_get_file_system_representation(
    string: u64,
    buffer: u64,
    max_buf_len: i64,
) -> u8 {
    if string == 0 || buffer == 0 || max_buf_len <= 0 {
        return 0;
    }
    let cstr = crate::cfstring::extract_cfstring_cstr(string);
    match cstr {
        Some(s) => {
            let bytes = s.as_bytes();
            let n = bytes.len().min((max_buf_len - 1) as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, n);
                *((buffer as *mut u8).add(n)) = 0; // NUL-terminate
            }
            1
        }
        None => 0,
    }
}

// ── CFDataCreateWithBytesNoCopy ────────────────────────────────────

/// `CFDataRef CFDataCreateWithBytesNoCopy(CFAllocatorRef, const UInt8 *bytes, CFIndex length, CFAllocatorRef bytesDeallocator)`
///
/// We copy the bytes anyway since we can't rely on the caller's lifetime guarantees.
pub extern "C" fn shim_cf_data_create_with_bytes_no_copy(
    alloc: u64,
    bytes: *const u8,
    length: i64,
    _deallocator: u64,
) -> u64 {
    shim_cf_data_create(alloc, bytes, length)
}

// ── CFErrorCopyDescription ─────────────────────────────────────────

/// `CFStringRef CFErrorCopyDescription(CFErrorRef err)`
///
/// We don't model CFError deeply — return a generic error string.
pub extern "C" fn shim_cf_error_copy_description(err: u64) -> u64 {
    if err == 0 {
        let cstr = std::ffi::CString::new("(null error)").unwrap();
        return crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0);
    }
    let cstr = std::ffi::CString::new("CFError (no details available)").unwrap();
    crate::cfstring::shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
}

// ── CFURLCreateDataAndPropertiesFromResource ───────────────────────

/// `Boolean CFURLCreateDataAndPropertiesFromResource(CFAllocatorRef, CFURLRef url,
///     CFDataRef *resourceData, CFDictionaryRef *properties, CFArrayRef desiredProperties,
///     SInt32 *errorCode)`
///
/// Deprecated API used by sw_vers to read plist files from disk.
pub extern "C" fn shim_cf_url_create_data_and_properties_from_resource(
    _alloc: u64,
    url: u64,
    resource_data_out: u64,
    _properties_out: u64,
    _desired: u64,
    error_code_out: u64,
) -> u8 {
    if url == 0 || resource_data_out == 0 {
        if error_code_out != 0 {
            unsafe {
                *(error_code_out as *mut i32) = -1;
            }
        }
        return 0;
    }
    // Extract file path from CFURL
    let url_str = crate::cfstring::extract_cfstring_cstr(unsafe {
        *((url as usize + 16) as *const u64) // NSURL stores the string at offset 16
    });
    let path = match url_str {
        Some(s) => {
            // Strip file:// prefix if present
            if let Some(p) = s.strip_prefix("file://") {
                p
            } else {
                s
            }
        }
        None => {
            if error_code_out != 0 {
                unsafe {
                    *(error_code_out as *mut i32) = -1;
                }
            }
            return 0;
        }
    };
    match std::fs::read(path) {
        Ok(data) => {
            let cf_data = crate::nsdata::make_nsdata(&data);
            unsafe {
                *(resource_data_out as *mut u64) = cf_data;
            }
            if error_code_out != 0 {
                unsafe {
                    *(error_code_out as *mut i32) = 0;
                }
            }
            1
        }
        Err(_) => {
            if error_code_out != 0 {
                unsafe {
                    *(error_code_out as *mut i32) = -1;
                }
            }
            0
        }
    }
}

// ── _CFCopySupplementalVersionDictionary ───────────────────────────

/// Private Apple function that returns the SystemVersion.plist as a CFDictionary.
/// sw_vers uses this to get macOS version info.
///
/// Reads `/System/Library/CoreServices/SystemVersion.plist` and returns
/// the parsed dictionary.  Returns NULL if the file is missing or unparseable.
pub extern "C" fn shim_cf_copy_supplemental_version_dictionary() -> u64 {
    const PLIST_PATH: &str = "/System/Library/CoreServices/SystemVersion.plist";

    let data = match std::fs::read(PLIST_PATH) {
        Ok(d) => d,
        Err(e) => {
            log::debug!("_CFCopySupplementalVersionDictionary: cannot read {PLIST_PATH}: {e}");
            return 0;
        }
    };

    // Parse the plist (XML or binary) via NSPropertyListSerialization
    let cf_data = crate::nsdata::make_nsdata(&data);
    let dict = crate::nspropertylistserialization::shim_plist_with_data(
        crate::nsobj::nspropertylistserialization_class_addr(),
        0, // _cmd
        cf_data,
        0, // options
        0, // format out
        0, // error out
    );

    if dict != 0 {
        log::debug!("_CFCopySupplementalVersionDictionary: loaded from {PLIST_PATH}");
    } else {
        log::debug!("_CFCopySupplementalVersionDictionary: plist parse failed");
    }
    dict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[cfg(target_os = "macos")]
    fn host_cf_symbol(name: &[u8]) -> *mut libc::c_void {
        static COREFOUNDATION_HANDLE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

        let handle = *COREFOUNDATION_HANDLE.get_or_init(|| unsafe {
            libc::dlopen(
                c"/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation".as_ptr(),
                libc::RTLD_LAZY,
            ) as usize
        }) as *mut libc::c_void;

        assert!(
            !handle.is_null(),
            "failed to open CoreFoundation for host test helper"
        );

        let symbol = unsafe { libc::dlsym(handle, name.as_ptr() as *const std::ffi::c_char) };
        let symbol = if symbol.is_null() {
            unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr() as *const std::ffi::c_char) }
        } else {
            symbol
        };
        assert!(
            !symbol.is_null(),
            "missing host symbol: {}",
            String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
        );
        symbol
    }

    #[cfg(target_os = "macos")]
    fn host_cf_release(obj: u64) {
        type HostCFReleaseFn = unsafe extern "C" fn(u64);

        if obj == 0 {
            return;
        }

        let cf_release: HostCFReleaseFn =
            unsafe { std::mem::transmute(host_cf_symbol(b"CFRelease\0")) };
        unsafe { cf_release(obj) };
    }

    #[cfg(target_os = "macos")]
    fn host_cfurl_create_with_file_system_path(path: &str, is_directory: bool) -> u64 {
        type HostCFStringCreateWithCStringFn =
            unsafe extern "C" fn(u64, *const std::ffi::c_char, u32) -> u64;
        type HostCFURLCreateWithFileSystemPathFn = unsafe extern "C" fn(u64, u64, i64, u8) -> u64;

        let cf_string_create: HostCFStringCreateWithCStringFn =
            unsafe { std::mem::transmute(host_cf_symbol(b"CFStringCreateWithCString\0")) };
        let cf_url_create_with_file_system_path: HostCFURLCreateWithFileSystemPathFn =
            unsafe { std::mem::transmute(host_cf_symbol(b"CFURLCreateWithFileSystemPath\0")) };

        let cstr = std::ffi::CString::new(path).unwrap();
        let host_path = unsafe {
            cf_string_create(0, cstr.as_ptr(), crate::cfstring::K_CF_STRING_ENCODING_UTF8)
        };
        let url =
            unsafe { cf_url_create_with_file_system_path(0, host_path, 0, u8::from(is_directory)) };
        host_cf_release(host_path);
        url
    }

    #[test]
    fn array_create_empty() {
        let arr = shim_cf_array_create(0, std::ptr::null(), 0, 0);
        assert_ne!(arr, 0);
        assert_eq!(shim_cf_array_get_count(arr), 0);
        free_obj(arr);
    }

    #[test]
    fn array_create_with_items() {
        let a = make_string("hello");
        let b = make_string("world");
        let items = [a, b];
        let arr = shim_cf_array_create(0, items.as_ptr(), 2, 0);
        assert_ne!(arr, 0);
        assert_eq!(shim_cf_array_get_count(arr), 2);
        assert_eq!(shim_cf_array_get_value_at_index(arr, 0), a);
        assert_eq!(shim_cf_array_get_value_at_index(arr, 1), b);
        free_obj(arr);
    }

    #[test]
    fn array_get_count_nil() {
        assert_eq!(shim_cf_array_get_count(0), 0);
    }

    #[test]
    fn array_get_value_out_of_bounds_and_nil() {
        let items: [u64; 1] = [42];
        let arr = shim_cf_array_create(0, items.as_ptr(), 1, 0);
        assert_eq!(shim_cf_array_get_value_at_index(arr, 5), 0);
        assert_eq!(shim_cf_array_get_value_at_index(arr, -1), 0);
        assert_eq!(shim_cf_array_get_value_at_index(0, 0), 0);
        free_obj(arr);
    }

    #[test]
    fn data_create_roundtrip() {
        let bytes: [u8; 3] = [10, 20, 30];
        let data = shim_cf_data_create(0, bytes.as_ptr(), 3);
        assert_ne!(data, 0);
        assert_eq!(shim_cf_data_get_length(data), 3);
        let ptr = shim_cf_data_get_byte_ptr(data);
        assert_ne!(ptr, 0);
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, 3) };
        assert_eq!(slice, &bytes);
        free_obj(data);
    }

    #[test]
    fn data_nil_safety() {
        assert_eq!(shim_cf_data_get_length(0), 0);
        assert_eq!(shim_cf_data_get_byte_ptr(0), 0);
    }

    #[test]
    fn dictionary_create_and_get() {
        let k = make_string("mykey");
        let v = make_string("myval");
        let keys = [k];
        let vals = [v];
        let dict = shim_cf_dictionary_create(0, keys.as_ptr(), vals.as_ptr(), 1, 0, 0);
        assert_ne!(dict, 0);
        assert_eq!(shim_cf_dictionary_get_count(dict), 1);
        assert_eq!(shim_cf_dictionary_get_value(dict, k), v);
        free_obj(dict);
    }

    #[test]
    fn dictionary_nil_safety() {
        assert_eq!(shim_cf_dictionary_get_count(0), 0);
        assert_eq!(shim_cf_dictionary_get_value(0, 0), 0);
    }

    #[test]
    fn boolean_singletons_distinct_and_non_zero() {
        let t = kcf_boolean_true_addr();
        let f = kcf_boolean_false_addr();
        assert_ne!(t, 0);
        assert_ne!(f, 0);
        assert_ne!(t, f);
    }

    #[test]
    fn cf_constants_cover_null_and_array_callbacks() {
        assert_ne!(kcf_null_addr(), 0);
        assert_ne!(kcf_type_array_callbacks_addr(), 0);
    }

    #[test]
    fn string_create_with_bytes_no_copy_roundtrips_utf8() {
        let string = shim_cf_string_create_with_bytes_no_copy(
            0,
            b"wezterm".as_ptr(),
            7,
            crate::cfstring::K_CF_STRING_ENCODING_UTF8,
            0,
            0,
        );

        assert_ne!(string, 0);
        crate::shim_test_helpers::assert_string_eq(string, "wezterm");
    }

    #[test]
    fn url_copy_and_path_component_helpers_roundtrip() {
        let path = make_string("/tmp/demo/config");
        let component = make_string("wezterm.lua");
        let url = shim_cf_url_create_with_file_system_path(0, path, 0, 0);
        let appended = shim_cf_url_create_copy_appending_path_component(0, url, component, 0);
        let trimmed = shim_cf_url_create_copy_deleting_last_path_component(0, appended);

        crate::shim_test_helpers::assert_string_eq(
            shim_cf_url_copy_file_system_path(appended, 0),
            "/tmp/demo/config/wezterm.lua",
        );
        crate::shim_test_helpers::assert_string_eq(
            shim_cf_url_copy_last_path_component(appended),
            "wezterm.lua",
        );
        assert_eq!(shim_cf_url_copy_absolute_url(url), url);
        crate::shim_test_helpers::assert_string_eq(
            shim_cf_url_copy_file_system_path(trimmed, 0),
            "/tmp/demo/config/",
        );
    }

    #[test]
    fn url_create_from_file_system_representation_builds_file_url() {
        let url =
            shim_cf_url_create_from_file_system_representation(0, b"/tmp/wezterm".as_ptr(), 12, 0);
        crate::shim_test_helpers::assert_string_eq(
            shim_cf_url_copy_file_system_path(url, 0),
            "/tmp/wezterm",
        );
    }

    #[test]
    fn bundle_copy_info_dictionary_for_url_detects_bundle_from_executable_path() {
        let root = std::env::temp_dir().join(format!(
            "cf_lite_bundle_info_dict_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t")
        ));
        let app_dir = root.join("TestXcode.app");
        let contents_dir = app_dir.join("Contents");
        let tool_dir = contents_dir.join("Developer/usr/bin");
        std::fs::create_dir_all(&tool_dir).unwrap();

        let plist = contents_dir.join("Info.plist");
        std::fs::write(
            &plist,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.TestXcode</string>
</dict>
</plist>
"#,
        )
        .unwrap();

        let exe_path = tool_dir.join("testtool");
        let exe_ns = crate::shim_test_helpers::make_string(exe_path.to_string_lossy().as_ref());
        let url = crate::nsurl::shim_nsurl_file_url_with_path(nsobj::nsurl_class_addr(), 0, exe_ns);
        let dict = shim_cf_bundle_copy_info_dictionary_for_url(url);

        assert_ne!(dict, 0);
        let key = crate::shim_test_helpers::make_string("CFBundleIdentifier");
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key);
        crate::shim_test_helpers::assert_string_eq(value, "com.example.TestXcode");

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bundle_copy_info_dictionary_for_host_url_detects_bundle_from_executable_path() {
        let root = std::env::temp_dir().join(format!(
            "cf_lite_bundle_info_host_url_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t")
        ));
        let app_dir = root.join("TestXcode.app");
        let contents_dir = app_dir.join("Contents");
        let tool_dir = contents_dir.join("Developer/usr/bin");
        std::fs::create_dir_all(&tool_dir).unwrap();

        let plist = contents_dir.join("Info.plist");
        std::fs::write(
            &plist,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.HostURLTestXcode</string>
</dict>
</plist>
"#,
        )
        .unwrap();

        let exe_path = tool_dir.join("testtool");
        std::fs::write(&exe_path, b"#!/bin/sh\n").unwrap();

        let url =
            host_cfurl_create_with_file_system_path(exe_path.to_string_lossy().as_ref(), false);
        let dict = shim_cf_bundle_copy_info_dictionary_for_url(url);

        assert_ne!(dict, 0);
        let key = crate::shim_test_helpers::make_string("CFBundleIdentifier");
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key);
        crate::shim_test_helpers::assert_string_eq(value, "com.example.HostURLTestXcode");

        host_cf_release(url);
        let _ = std::fs::remove_dir_all(root);
    }
}
