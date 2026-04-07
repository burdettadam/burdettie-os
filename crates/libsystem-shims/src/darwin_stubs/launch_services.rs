//! LaunchServices + Uniform Type Identifiers framework stubs.
//!
//! macOS app/URL/file-type registration API. Functions return
//! error codes (kLSApplicationNotFoundErr = -10814) or NULL.
//!
//! UTType stubs handle UTTypeCreatePreferredIdentifierForTag and the
//! common kUT* constant strings needed on Linux where CoreServices
//! is not available.

use crate::table::ShimTable;

/// kLSApplicationNotFoundErr
const LS_APP_NOT_FOUND: i32 = -10814;

pub struct LaunchServicesStubs;

impl LaunchServicesStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── LaunchServices ──────────────────────────────────────────
        register_shims!(table,
            ("_LSCopyDefaultApplicationURLForURL", shim_ls_null_stub as *const () as u64, "LSCopyDefaultApplicationURLForURL (stub → NULL)"),
            ("_LSCopyDefaultApplicationURLForContentType", shim_ls_null_stub as *const () as u64, "LSCopyDefaultApplicationURLForContentType (stub → NULL)"),
            ("_LSCopyApplicationURLsForBundleIdentifier", shim_ls_null_stub as *const () as u64, "LSCopyApplicationURLsForBundleIdentifier (stub → NULL)"),
            ("_LSCopyApplicationURLsForURL", shim_ls_null_stub as *const () as u64, "LSCopyApplicationURLsForURL (stub → NULL)"),
            ("_LSCanURLAcceptURL", shim_ls_false_stub as *const () as u64, "LSCanURLAcceptURL (stub → false)"),
            ("_LSOpenCFURLRef", shim_ls_err_stub as *const () as u64, "LSOpenCFURLRef (stub → error)"),
            ("_LSOpenFromURLSpec", shim_ls_err_stub as *const () as u64, "LSOpenFromURLSpec (stub → error)"),
            ("_LSRegisterURL", shim_ls_err_stub as *const () as u64, "LSRegisterURL (stub → error)"),
            ("_LSGetApplicationForInfo", shim_ls_err_stub as *const () as u64, "LSGetApplicationForInfo (stub → error)"),
            ("_LSGetApplicationForURL", shim_ls_err_stub as *const () as u64, "LSGetApplicationForURL (stub → error)"),
            ("_LSCopyItemInfoForURL", shim_ls_err_stub as *const () as u64, "LSCopyItemInfoForURL (stub → error)"),
            ("_LSCopyDisplayNameForURL", shim_ls_null_stub as *const () as u64, "LSCopyDisplayNameForURL (stub → NULL)"),
            ("_LSCopyKindStringForURL", shim_ls_null_stub as *const () as u64, "LSCopyKindStringForURL (stub → NULL)"),
            ("_LSCopyAllRoleHandlersForContentType", shim_ls_null_stub as *const () as u64, "LSCopyAllRoleHandlersForContentType (stub → NULL)"),
            ("_LSSetDefaultRoleHandlerForContentType", shim_ls_err_stub as *const () as u64, "LSSetDefaultRoleHandlerForContentType (stub → error)"),
            ("_LSCopyDefaultHandlerForURLScheme", shim_ls_null_stub as *const () as u64, "LSCopyDefaultHandlerForURLScheme (stub → NULL)"),
            ("_LSSetDefaultHandlerForURLScheme", shim_ls_err_stub as *const () as u64, "LSSetDefaultHandlerForURLScheme (stub → error)"),
            ("__LSCopyApplicationArrayInFrontToBackOrder", shim_ls_null_stub as *const () as u64, "_LSCopyApplicationArrayInFrontToBackOrder (stub → NULL)"),
        );

        // ── Uniform Type Identifiers ────────────────────────────────
        register_shims!(table,
            ("_UTTypeCreatePreferredIdentifierForTag", shim_ut_type_create_preferred_identifier as *const () as u64, "UTTypeCreatePreferredIdentifierForTag (extension→UTI)"),
            ("_UTTypeCopyPreferredTagWithClass", shim_ut_type_copy_preferred_tag as *const () as u64, "UTTypeCopyPreferredTagWithClass (UTI→extension)"),
            ("_UTTypeConformsTo", shim_ut_type_conforms_to as *const () as u64, "UTTypeConformsTo (stub → false)"),
            ("_UTTypeCopyDescription", shim_ls_null_stub as *const () as u64, "UTTypeCopyDescription (stub → NULL)"),
            ("_UTTypeCopyDeclaration", shim_ls_null_stub as *const () as u64, "UTTypeCopyDeclaration (stub → NULL)"),
            ("_UTTypeCreateAllIdentifiersForTag", shim_ls_null_stub as *const () as u64, "UTTypeCreateAllIdentifiersForTag (stub → NULL)"),
            ("_UTTypeEqual", shim_ut_type_equal as *const () as u64, "UTTypeEqual (CFString pointer compare)"),
        );

        // ── kUTTagClass constants (CFString pointers) ───────────────
        register_shims!(table,
            ("_kUTTagClassFilenameExtension", &KUT_TAG_FILENAME_EXT as *const _ as u64, "kUTTagClassFilenameExtension data"),
            ("_kUTTagClassMIMEType", &KUT_TAG_MIME_TYPE as *const _ as u64, "kUTTagClassMIMEType data"),
        );

        // ── kUTType constants (CFString pointers) ───────────────────
        register_shims!(table,
            ("_kUTTypeItem", &KUT_TYPE_ITEM as *const _ as u64, "kUTTypeItem data"),
            ("_kUTTypeContent", &KUT_TYPE_CONTENT as *const _ as u64, "kUTTypeContent data"),
            ("_kUTTypeData", &KUT_TYPE_DATA as *const _ as u64, "kUTTypeData data"),
            ("_kUTTypeText", &KUT_TYPE_TEXT as *const _ as u64, "kUTTypeText data"),
            ("_kUTTypePlainText", &KUT_TYPE_PLAIN_TEXT as *const _ as u64, "kUTTypePlainText data"),
            ("_kUTTypeUTF8PlainText", &KUT_TYPE_UTF8_PLAIN_TEXT as *const _ as u64, "kUTTypeUTF8PlainText data"),
            ("_kUTTypeImage", &KUT_TYPE_IMAGE as *const _ as u64, "kUTTypeImage data"),
            ("_kUTTypePNG", &KUT_TYPE_PNG as *const _ as u64, "kUTTypePNG data"),
            ("_kUTTypeJPEG", &KUT_TYPE_JPEG as *const _ as u64, "kUTTypeJPEG data"),
            ("_kUTTypePDF", &KUT_TYPE_PDF as *const _ as u64, "kUTTypePDF data"),
            ("_kUTTypeURL", &KUT_TYPE_URL as *const _ as u64, "kUTTypeURL data"),
            ("_kUTTypeFileURL", &KUT_TYPE_FILE_URL as *const _ as u64, "kUTTypeFileURL data"),
            ("_kUTTypeDirectory", &KUT_TYPE_DIRECTORY as *const _ as u64, "kUTTypeDirectory data"),
            ("_kUTTypeFolder", &KUT_TYPE_FOLDER as *const _ as u64, "kUTTypeFolder data"),
            ("_kUTTypeSourceCode", &KUT_TYPE_SOURCE_CODE as *const _ as u64, "kUTTypeSourceCode data"),
            ("_kUTTypePropertyList", &KUT_TYPE_PLIST as *const _ as u64, "kUTTypePropertyList data"),
            ("_kUTTypeXMLPropertyList", &KUT_TYPE_XML_PLIST as *const _ as u64, "kUTTypeXMLPropertyList data"),
            ("_kUTTypeBinaryPropertyList", &KUT_TYPE_BINARY_PLIST as *const _ as u64, "kUTTypeBinaryPropertyList data"),
            ("_kUTTypeJSON", &KUT_TYPE_JSON as *const _ as u64, "kUTTypeJSON data"),
            ("_kUTTypeXML", &KUT_TYPE_XML as *const _ as u64, "kUTTypeXML data"),
        );

        log::debug!("LaunchServicesStubs: registered {} LS + UTType stubs", 18 + 7 + 2 + 20);
    }
}

pub extern "C" fn shim_ls_null_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> u64 { 0 }
pub extern "C" fn shim_ls_false_stub(_arg0: u64, _arg1: u64, _arg2: u64, _arg3: u64) -> u8 { 0 }
pub extern "C" fn shim_ls_err_stub(_arg0: u64, _arg1: u64, _arg2: u64) -> i32 { LS_APP_NOT_FOUND }

// ── UTType function implementations ────────────────────────────────
//
// Simple extension↔UTI mapping.  On macOS this queries the LaunchServices
// database; here we use a built-in table of common mappings.

/// Extension → UTI string mapping.
const EXTENSION_TO_UTI: &[(&str, &str)] = &[
    ("c", "public.c-source"),
    ("cpp", "public.c-plus-plus-source"),
    ("css", "public.css"),
    ("gif", "com.compuserve.gif"),
    ("h", "public.c-header"),
    ("htm", "public.html"),
    ("html", "public.html"),
    ("jpeg", "public.jpeg"),
    ("jpg", "public.jpeg"),
    ("js", "com.netscape.javascript-source"),
    ("json", "public.json"),
    ("m", "public.objective-c-source"),
    ("mm", "public.objective-c-plus-plus-source"),
    ("o", "public.object-code"),
    ("pdf", "com.adobe.pdf"),
    ("plist", "com.apple.property-list"),
    ("png", "public.png"),
    ("py", "public.python-script"),
    ("rb", "public.ruby-script"),
    ("rtf", "public.rtf"),
    ("sh", "public.shell-script"),
    ("swift", "public.swift-source"),
    ("tif", "public.tiff"),
    ("tiff", "public.tiff"),
    ("txt", "public.plain-text"),
    ("xml", "public.xml"),
    ("zip", "com.pkware.zip-archive"),
];

/// MIME type → UTI string mapping.
const MIME_TO_UTI: &[(&str, &str)] = &[
    ("application/json", "public.json"),
    ("application/pdf", "com.adobe.pdf"),
    ("application/xml", "public.xml"),
    ("application/zip", "com.pkware.zip-archive"),
    ("image/gif", "com.compuserve.gif"),
    ("image/jpeg", "public.jpeg"),
    ("image/png", "public.png"),
    ("image/tiff", "public.tiff"),
    ("text/css", "public.css"),
    ("text/html", "public.html"),
    ("text/plain", "public.plain-text"),
    ("text/xml", "public.xml"),
];

/// `CFStringRef UTTypeCreatePreferredIdentifierForTag(CFStringRef tagClass,
///     CFStringRef tag, CFStringRef conformingToUTI)`
///
/// Looks up a UTI from a file extension or MIME type.
/// Returns a CFString (allocated NSString) or NULL if unknown.
pub extern "C" fn shim_ut_type_create_preferred_identifier(
    tag_class: u64,
    tag: u64,
    _conforming_to: u64,
) -> u64 {
    if tag == 0 {
        return 0;
    }

    // Read the tag string
    let tag_str = extract_cfstring(tag);
    if tag_str.is_empty() {
        return 0;
    }

    // Determine which table to search based on tag_class
    let is_mime = is_mime_tag_class(tag_class);

    let uti = if is_mime {
        MIME_TO_UTI.iter()
            .find(|(mime, _)| mime.eq_ignore_ascii_case(&tag_str))
            .map(|(_, uti)| *uti)
    } else {
        // Default: filename extension
        EXTENSION_TO_UTI.iter()
            .find(|(ext, _)| ext.eq_ignore_ascii_case(&tag_str))
            .map(|(_, uti)| *uti)
    };

    match uti {
        Some(uti_str) => alloc_cfstring(uti_str),
        None => {
            // Generate a dynamic UTI: dyn.xxx for unknown types
            let dyn_uti = format!("dyn.{tag_str}");
            alloc_cfstring(&dyn_uti)
        }
    }
}

/// `CFStringRef UTTypeCopyPreferredTagWithClass(CFStringRef uti,
///     CFStringRef tagClass)`
///
/// Reverse lookup: given a UTI, return the preferred tag (extension or MIME).
pub extern "C" fn shim_ut_type_copy_preferred_tag(uti: u64, tag_class: u64) -> u64 {
    if uti == 0 {
        return 0;
    }
    let uti_str = extract_cfstring(uti);
    if uti_str.is_empty() {
        return 0;
    }

    let is_mime = is_mime_tag_class(tag_class);

    if is_mime {
        for (mime, u) in MIME_TO_UTI {
            if *u == uti_str {
                return alloc_cfstring(mime);
            }
        }
    } else {
        for (ext, u) in EXTENSION_TO_UTI {
            if *u == uti_str {
                return alloc_cfstring(ext);
            }
        }
    }

    0 // not found
}

/// `Boolean UTTypeConformsTo(CFStringRef uti, CFStringRef conformsToUTI)`
pub extern "C" fn shim_ut_type_conforms_to(_uti: u64, _conforms_to: u64) -> u8 {
    // Simplified: only return true for exact match
    if _uti == 0 || _conforms_to == 0 {
        return 0;
    }
    let a = extract_cfstring(_uti);
    let b = extract_cfstring(_conforms_to);
    if a == b { 1 } else { 0 }
}

/// `Boolean UTTypeEqual(CFStringRef uti1, CFStringRef uti2)`
pub extern "C" fn shim_ut_type_equal(uti1: u64, uti2: u64) -> u8 {
    if uti1 == 0 || uti2 == 0 {
        return 0;
    }
    let a = extract_cfstring(uti1);
    let b = extract_cfstring(uti2);
    if a.eq_ignore_ascii_case(&b) { 1 } else { 0 }
}

// ── UTType constant strings ────────────────────────────────────────
//
// These are CFString data symbols. We allocate them lazily as static
// NUL-terminated byte arrays and return their address. On Apple they'd
// be CFStringRef globals — here the address is just used as an identity
// token, and any code calling CFStringGetCStringPtr on them will read
// the embedded string.

macro_rules! utf_const {
    ($name:ident, $value:literal) => {
        static $name: &[u8] = concat!($value, "\0").as_bytes();
    };
}

// kUTTagClass constants
utf_const!(KUT_TAG_FILENAME_EXT, "public.filename-extension");
utf_const!(KUT_TAG_MIME_TYPE, "public.mime-type");

// kUTType constants
utf_const!(KUT_TYPE_ITEM, "public.item");
utf_const!(KUT_TYPE_CONTENT, "public.content");
utf_const!(KUT_TYPE_DATA, "public.data");
utf_const!(KUT_TYPE_TEXT, "public.text");
utf_const!(KUT_TYPE_PLAIN_TEXT, "public.plain-text");
utf_const!(KUT_TYPE_UTF8_PLAIN_TEXT, "public.utf8-plain-text");
utf_const!(KUT_TYPE_IMAGE, "public.image");
utf_const!(KUT_TYPE_PNG, "public.png");
utf_const!(KUT_TYPE_JPEG, "public.jpeg");
utf_const!(KUT_TYPE_PDF, "com.adobe.pdf");
utf_const!(KUT_TYPE_URL, "public.url");
utf_const!(KUT_TYPE_FILE_URL, "public.file-url");
utf_const!(KUT_TYPE_DIRECTORY, "public.directory");
utf_const!(KUT_TYPE_FOLDER, "public.folder");
utf_const!(KUT_TYPE_SOURCE_CODE, "public.source-code");
utf_const!(KUT_TYPE_PLIST, "com.apple.property-list");
utf_const!(KUT_TYPE_XML_PLIST, "com.apple.xml-property-list");
utf_const!(KUT_TYPE_BINARY_PLIST, "com.apple.binary-property-list");
utf_const!(KUT_TYPE_JSON, "public.json");
utf_const!(KUT_TYPE_XML, "public.xml");

// ── String helpers ─────────────────────────────────────────────────

/// Check if a tag_class CFString represents MIME types.
fn is_mime_tag_class(tag_class: u64) -> bool {
    if tag_class == 0 {
        return false;
    }
    // Check if tag_class points to our MIME sentinel
    if tag_class == &KUT_TAG_MIME_TYPE as *const _ as u64 {
        return true;
    }
    // Also check the string content
    let s = extract_cfstring(tag_class);
    s.contains("mime")
}

/// Extract a string from a CFString/NSString pointer.
///
/// Tries several layouts:
/// 1. Raw C string pointer (from our utf_const! macro — points to NUL-terminated bytes)
/// 2. NSString layout: [isa:8][data_ptr or inline data]
///
/// Returns an owned String so we're safe across pointer lifetimes.
fn extract_cfstring(ptr: u64) -> String {
    if ptr == 0 {
        return String::new();
    }

    // First try: direct C string (for our static constants)
    let p = ptr as *const u8;
    let mut len = 0usize;
    unsafe {
        // Check if it looks like a NUL-terminated ASCII string
        while len < 256 {
            let byte = *p.add(len);
            if byte == 0 { break; }
            if byte > 127 { break; } // not ASCII — might be ISA pointer
            len += 1;
        }
        if len > 0 && len < 256 {
            let slice = std::slice::from_raw_parts(p, len);
            if let Ok(s) = std::str::from_utf8(slice) {
                // Sanity: if it looks like a real string (not a pointer), return it
                if s.chars().all(|c| c.is_ascii_graphic() || c == ' ' || c == '.' || c == '-' || c == '/') {
                    return s.to_string();
                }
            }
        }
    }

    // Fall back: treat as NSString/CFString with isa+data layout
    // [0..8) = isa pointer, [8..16) = data pointer or length
    // We attempt to read the data pointer at offset 8
    unsafe {
        let data_ptr = *((ptr + 8) as *const u64);
        if data_ptr != 0 && data_ptr > 0x1000 {
            let dp = data_ptr as *const u8;
            let mut dlen = 0;
            while dlen < 256 {
                if *dp.add(dlen) == 0 { break; }
                dlen += 1;
            }
            if dlen > 0 {
                let slice = std::slice::from_raw_parts(dp, dlen);
                if let Ok(s) = std::str::from_utf8(slice) {
                    return s.to_string();
                }
            }
        }
    }

    String::new()
}

/// Allocate a CFString/NSString from a Rust string.
/// Uses the cf-lite allocator if available, otherwise a simple CString leak.
fn alloc_cfstring(s: &str) -> u64 {
    let cstr = std::ffi::CString::new(s).unwrap_or_default();
    // Leak the CString so the pointer remains valid.
    // This is intentional — UTType results are cached/retained by callers.
    let ptr = cstr.into_raw();
    ptr as u64
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_services_stubs_registered() {
        let mut table = ShimTable::new();
        LaunchServicesStubs::install(&mut table);
        assert!(table.resolve("_LSCopyDefaultApplicationURLForURL").is_some());
        assert!(table.resolve("_LSOpenCFURLRef").is_some());
        assert!(table.resolve("_LSGetApplicationForInfo").is_some());
    }

    #[test]
    fn ls_null_stub_returns_zero() {
        assert_eq!(shim_ls_null_stub(0, 0, 0), 0);
    }

    #[test]
    fn ls_err_stub_returns_not_found() {
        assert_eq!(shim_ls_err_stub(0, 0, 0), LS_APP_NOT_FOUND);
    }

    // ── UTType tests ────────────────────────────────────────────

    #[test]
    fn ut_type_stubs_registered() {
        let mut table = ShimTable::new();
        LaunchServicesStubs::install(&mut table);
        assert!(table.resolve("_UTTypeCreatePreferredIdentifierForTag").is_some());
        assert!(table.resolve("_UTTypeCopyPreferredTagWithClass").is_some());
        assert!(table.resolve("_UTTypeConformsTo").is_some());
        assert!(table.resolve("_UTTypeEqual").is_some());
        assert!(table.resolve("_kUTTypeItem").is_some());
        assert!(table.resolve("_kUTTagClassFilenameExtension").is_some());
        assert!(table.resolve("_kUTTagClassMIMEType").is_some());
    }

    #[test]
    fn ut_type_constant_count() {
        let mut table = ShimTable::new();
        LaunchServicesStubs::install(&mut table);
        // 18 LS + 7 UTType functions + 2 tag class + 20 type constants = 47
        assert!(table.len() >= 47, "expected ≥47 stubs, got {}", table.len());
    }

    #[test]
    fn ut_extension_to_uti_known() {
        // "txt" → "public.plain-text"
        let tag = b"txt\0";
        let ext_class = KUT_TAG_FILENAME_EXT.as_ptr() as u64;
        let result = shim_ut_type_create_preferred_identifier(ext_class, tag.as_ptr() as u64, 0);
        assert_ne!(result, 0);
        let uti = unsafe { std::ffi::CStr::from_ptr(result as *const std::ffi::c_char) };
        assert_eq!(uti.to_str().unwrap(), "public.plain-text");
        // Clean up leaked CString
        unsafe { drop(std::ffi::CString::from_raw(result as *mut std::ffi::c_char)); }
    }

    #[test]
    fn ut_extension_to_uti_unknown() {
        let tag = b"xyz123\0";
        let ext_class = KUT_TAG_FILENAME_EXT.as_ptr() as u64;
        let result = shim_ut_type_create_preferred_identifier(ext_class, tag.as_ptr() as u64, 0);
        assert_ne!(result, 0);
        let uti = unsafe { std::ffi::CStr::from_ptr(result as *const std::ffi::c_char) };
        assert_eq!(uti.to_str().unwrap(), "dyn.xyz123");
        unsafe { drop(std::ffi::CString::from_raw(result as *mut std::ffi::c_char)); }
    }

    #[test]
    fn ut_mime_to_uti() {
        let tag = b"image/png\0";
        let mime_class = KUT_TAG_MIME_TYPE.as_ptr() as u64;
        let result = shim_ut_type_create_preferred_identifier(mime_class, tag.as_ptr() as u64, 0);
        assert_ne!(result, 0);
        let uti = unsafe { std::ffi::CStr::from_ptr(result as *const std::ffi::c_char) };
        assert_eq!(uti.to_str().unwrap(), "public.png");
        unsafe { drop(std::ffi::CString::from_raw(result as *mut std::ffi::c_char)); }
    }

    #[test]
    fn ut_uti_to_extension() {
        let uti = b"public.jpeg\0";
        let ext_class = KUT_TAG_FILENAME_EXT.as_ptr() as u64;
        let result = shim_ut_type_copy_preferred_tag(uti.as_ptr() as u64, ext_class);
        assert_ne!(result, 0);
        let ext = unsafe { std::ffi::CStr::from_ptr(result as *const std::ffi::c_char) };
        assert_eq!(ext.to_str().unwrap(), "jpeg");
        unsafe { drop(std::ffi::CString::from_raw(result as *mut std::ffi::c_char)); }
    }

    #[test]
    fn ut_type_equal_same() {
        let a = b"public.png\0";
        let b = b"public.png\0";
        assert_eq!(shim_ut_type_equal(a.as_ptr() as u64, b.as_ptr() as u64), 1);
    }

    #[test]
    fn ut_type_equal_different() {
        let a = b"public.png\0";
        let b = b"public.jpeg\0";
        assert_eq!(shim_ut_type_equal(a.as_ptr() as u64, b.as_ptr() as u64), 0);
    }

    #[test]
    fn ut_type_conforms_exact_match() {
        let a = b"public.png\0";
        assert_eq!(shim_ut_type_conforms_to(a.as_ptr() as u64, a.as_ptr() as u64), 1);
    }

    #[test]
    fn ut_nil_inputs_safe() {
        assert_eq!(shim_ut_type_create_preferred_identifier(0, 0, 0), 0);
        assert_eq!(shim_ut_type_copy_preferred_tag(0, 0), 0);
        assert_eq!(shim_ut_type_conforms_to(0, 0), 0);
        assert_eq!(shim_ut_type_equal(0, 0), 0);
    }

    #[test]
    fn kut_constants_are_non_null() {
        assert_ne!(&KUT_TAG_FILENAME_EXT as *const _ as u64, 0);
        assert_ne!(&KUT_TAG_MIME_TYPE as *const _ as u64, 0);
        assert_ne!(&KUT_TYPE_ITEM as *const _ as u64, 0);
        assert_ne!(&KUT_TYPE_JSON as *const _ as u64, 0);
    }
}
