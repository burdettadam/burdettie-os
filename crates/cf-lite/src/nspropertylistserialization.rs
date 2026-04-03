//! NSPropertyListSerialization shim: plist ↔ Foundation object conversion.
//!
//! Supports XML plist generation plus XML/binary plist parsing.
//!
//! Foundation mapping:
//!   - NSString ↔ `<string>`
//!   - NSNumber (int) ↔ `<integer>`
//!   - NSNumber (double) ↔ `<real>`
//!   - NSNumber (bool) ↔ `<true/>` / `<false/>`
//!   - NSArray ↔ `<array>`
//!   - NSDictionary ↔ `<dict>`
//!   - NSData ↔ `<data>` (base64 — simplified: hex encoding)

use crate::cfstring;
use crate::nsarray;
use crate::nsdata;
use crate::nsdictionary;
use crate::nsnumber;
use crate::nsobj;

/// `NSPropertyListBinaryFormat_v1_0 = 200`
const PLIST_BINARY_FORMAT: u64 = 200;

/// Check if ISA matches a dictionary class (dynamic or constant).
fn is_dict_isa(isa: u64) -> bool {
    isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
        || isa == nsobj::nsconstantdictionary_class_addr()
}

/// Check if ISA matches a number class (dynamic or constant).
fn is_number_isa(isa: u64) -> bool {
    isa == nsobj::nsnumber_class_addr()
        || isa == nsobj::nsconstantintegernumber_class_addr()
}

/// `NSPropertyListXMLFormat_v1_0 = 100`
const _PLIST_XML_FORMAT: u64 = 100;

fn foundation_string(value: &str) -> u64 {
    let c_str = std::ffi::CString::new(value).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0)
}

fn plist_value_to_foundation(value: plist::Value) -> u64 {
    match value {
        plist::Value::String(value) => foundation_string(&value),
        plist::Value::Boolean(value) => nsnumber::make_nsnumber_bool(value),
        plist::Value::Integer(value) => {
            if let Some(signed) = value.as_signed() {
                nsnumber::make_nsnumber_int(signed)
            } else if let Some(unsigned) = value.as_unsigned() {
                if let Ok(signed) = i64::try_from(unsigned) {
                    nsnumber::make_nsnumber_int(signed)
                } else {
                    nsnumber::make_nsnumber_double(unsigned as f64)
                }
            } else {
                0
            }
        }
        plist::Value::Real(value) => nsnumber::make_nsnumber_double(value),
        plist::Value::Data(bytes) => nsdata::make_nsdata(&bytes),
        plist::Value::Array(items) => {
            let converted: Vec<u64> = items.into_iter().map(plist_value_to_foundation).collect();
            nsarray::make_nsarray(&converted)
        }
        plist::Value::Dictionary(dict) => {
            let mut keys = Vec::with_capacity(dict.len());
            let mut values = Vec::with_capacity(dict.len());
            for (key, value) in dict {
                keys.push(foundation_string(&key));
                values.push(plist_value_to_foundation(value));
            }
            nsdictionary::make_nsdictionary(&keys, &values)
        }
        _ => 0,
    }
}

fn parse_plist_bytes(bytes: &[u8]) -> u64 {
    if let Ok(value) = plist::Value::from_reader(std::io::Cursor::new(bytes)) {
        return plist_value_to_foundation(value);
    }

    let Ok(xml) = std::str::from_utf8(bytes) else {
        return 0;
    };
    parse_plist_xml(xml)
}

// ── Foundation → plist XML ─────────────────────────────────────────

fn foundation_to_xml(obj: u64, indent: usize) -> String {
    if obj == 0 {
        return format!("{}<string></string>", "  ".repeat(indent));
    }

    let isa = unsafe { *(obj as *const u64) };
    let pad = "  ".repeat(indent);

    // NSString
    if isa == nsobj::nsstring_class_addr()
        || isa == crate::foundation::cfconstant_string_class_addr()
        || isa == nsobj::nsmutablestring_class_addr()
    {
        let s = cfstring::extract_cfstring_cstr(obj).unwrap_or("");
        let escaped = xml_escape(s);
        return format!("{pad}<string>{escaped}</string>");
    }

    // NSNumber (including NSConstantIntegerNumber)
    if is_number_isa(isa) {
        let (tag, raw) = nsnumber::read_number_pub(obj);
        return match tag {
            2 => { // bool
                if raw != 0 { format!("{pad}<true/>") } else { format!("{pad}<false/>") }
            }
            0 => format!("{pad}<integer>{}</integer>", raw as i64), // int
            1 => format!("{pad}<real>{}</real>", f64::from_bits(raw)), // double
            _ => format!("{pad}<integer>{}</integer>", raw as i64),
        };
    }

    // NSArray
    if isa == nsobj::nsarray_class_addr() || isa == nsobj::nsmutablearray_class_addr() {
        let count = unsafe { *((obj + 8) as *const u64) } as usize;
        let items_ptr = unsafe { *((obj + 16) as *const u64) };
        let mut lines = vec![format!("{pad}<array>")];
        for i in 0..count {
            let item = unsafe { *((items_ptr as *const u64).add(i)) };
            lines.push(foundation_to_xml(item, indent + 1));
        }
        lines.push(format!("{pad}</array>"));
        return lines.join("\n");
    }

    // NSDictionary (including NSConstantDictionary)
    if is_dict_isa(isa) {
        let count = nsdictionary::helpers_pub::dict_count_pub(obj) as usize;
        let keys_ptr = nsdictionary::helpers_pub::dict_keys_pub(obj);
        let vals_ptr = nsdictionary::helpers_pub::dict_vals_pub(obj);
        let mut lines = vec![format!("{pad}<dict>")];
        let inner_pad = "  ".repeat(indent + 1);
        for i in 0..count {
            let key = unsafe { *((keys_ptr as *const u64).add(i)) };
            let val = unsafe { *((vals_ptr as *const u64).add(i)) };
            let key_str = cfstring::extract_cfstring_cstr(key).unwrap_or("");
            let escaped_key = xml_escape(key_str);
            lines.push(format!("{inner_pad}<key>{escaped_key}</key>"));
            lines.push(foundation_to_xml(val, indent + 1));
        }
        lines.push(format!("{pad}</dict>"));
        return lines.join("\n");
    }

    // NSData
    if isa == nsobj::nsdata_class_addr() {
        let length = unsafe { *((obj + 8) as *const u64) } as usize;
        let bytes_ptr = unsafe { *((obj + 16) as *const u64) } as *const u8;
        if length > 0 && !bytes_ptr.is_null() {
            let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, length) };
            let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
            return format!("{pad}<data>{hex}</data>");
        }
        return format!("{pad}<data></data>");
    }

    format!("{pad}<string></string>")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Plist XML → Foundation ─────────────────────────────────────────

fn parse_xml_value(xml: &str) -> (u64, &str) {
    let xml = xml.trim_start();

    if let Some(rest) = xml.strip_prefix("<true/>") {
        return (nsnumber::make_nsnumber_bool(true), rest);
    }
    if let Some(rest) = xml.strip_prefix("<false/>") {
        return (nsnumber::make_nsnumber_bool(false), rest);
    }
    if let Some(rest) = xml.strip_prefix("<string>") {
        if let Some(end) = rest.find("</string>") {
            let s = xml_unescape(&rest[..end]);
            let c_str = std::ffi::CString::new(s).unwrap_or_default();
            let ns = cfstring::shim_cf_string_create_with_cstring(0, c_str.as_ptr(), 0);
            return (ns, &rest[end + 9..]);
        }
    }
    if let Some(rest) = xml.strip_prefix("<integer>") {
        if let Some(end) = rest.find("</integer>") {
            let val: i64 = rest[..end].trim().parse().unwrap_or(0);
            return (nsnumber::make_nsnumber_int(val), &rest[end + 10..]);
        }
    }
    if let Some(rest) = xml.strip_prefix("<real>") {
        if let Some(end) = rest.find("</real>") {
            let val: f64 = rest[..end].trim().parse().unwrap_or(0.0);
            return (nsnumber::make_nsnumber_double(val), &rest[end + 7..]);
        }
    }
    if let Some(rest) = xml.strip_prefix("<array>") {
        let mut items = Vec::new();
        let mut remaining = rest.trim_start();
        while !remaining.starts_with("</array>") && !remaining.is_empty() {
            let (val, r) = parse_xml_value(remaining);
            items.push(val);
            remaining = r.trim_start();
        }
        if let Some(r) = remaining.strip_prefix("</array>") {
            return (nsarray::make_nsarray(&items), r);
        }
        return (nsarray::make_nsarray(&items), remaining);
    }
    if let Some(rest) = xml.strip_prefix("<dict>") {
        let mut keys = Vec::new();
        let mut vals = Vec::new();
        let mut remaining = rest.trim_start();
        while !remaining.starts_with("</dict>") && !remaining.is_empty() {
            // Parse <key>...</key>
            if let Some(r) = remaining.strip_prefix("<key>") {
                if let Some(end) = r.find("</key>") {
                    let key_str = xml_unescape(&r[..end]);
                    let c_key = std::ffi::CString::new(key_str).unwrap_or_default();
                    keys.push(cfstring::shim_cf_string_create_with_cstring(0, c_key.as_ptr(), 0));
                    remaining = r[end + 6..].trim_start();
                    let (val, r2) = parse_xml_value(remaining);
                    vals.push(val);
                    remaining = r2.trim_start();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if let Some(r) = remaining.strip_prefix("</dict>") {
            return (nsdictionary::make_nsdictionary(&keys, &vals), r);
        }
        return (nsdictionary::make_nsdictionary(&keys, &vals), remaining);
    }
    if let Some(rest) = xml.strip_prefix("<data>") {
        if let Some(end) = rest.find("</data>") {
            let hex = &rest[..end].trim();
            let bytes: Vec<u8> = (0..hex.len())
                .step_by(2)
                .filter_map(|i| {
                    if i + 2 <= hex.len() {
                        u8::from_str_radix(&hex[i..i + 2], 16).ok()
                    } else {
                        None
                    }
                })
                .collect();
            return (nsdata::make_nsdata(&bytes), &rest[end + 7..]);
        }
    }

    // Unknown — skip characters and return nil
    (0, &xml[xml.len().min(1)..])
}

fn xml_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn parse_plist_xml(xml: &str) -> u64 {
    let body_start = if let Some(start) = xml.find("<plist") {
        xml[start..].find('>').map(|j| start + j + 1).unwrap_or(0)
    } else {
        0
    };
    let body = &xml[body_start..];
    let body = body.strip_suffix("</plist>").unwrap_or(body).trim();
    let (result, _) = parse_xml_value(body);
    result
}

fn extract_xml_plist(bytes: &[u8]) -> Option<&[u8]> {
    let start = bytes.windows(5).position(|window| window == b"<?xml")?;
    let end_marker = b"</plist>";
    let end = bytes.windows(end_marker.len()).position(|window| window == end_marker)?;
    (end >= start).then_some(&bytes[start..end + end_marker.len()])
}

// ── Class methods ──────────────────────────────────────────────────

/// `+[NSPropertyListSerialization dataWithPropertyList:format:options:error:]`
pub extern "C" fn shim_plist_data_with_object(
    _cls: u64,
    _cmd: u64,
    plist: u64,
    _format: u64,
    _options: u64,
    error_out: u64,
) -> u64 {
    if plist == 0 {
        if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
        return 0;
    }

    let header = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                  <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                  <plist version=\"1.0\">\n";
    let footer = "\n</plist>\n";
    let body = foundation_to_xml(plist, 0);
    let xml = format!("{header}{body}{footer}");

    nsdata::make_nsdata(xml.as_bytes())
}

/// `+[NSPropertyListSerialization propertyListWithData:options:format:error:]`
pub extern "C" fn shim_plist_with_data(
    _cls: u64,
    _cmd: u64,
    data: u64,
    _options: u64,
    format_out: u64,
    error_out: u64,
) -> u64 {
    if data == 0 { return 0; }

    let length = unsafe { *((data + 8) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((data + 16) as *const u64) } as *const u8;
    if bytes_ptr.is_null() || length == 0 { return 0; }

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, length) };
    if format_out != 0 {
        let format = if slice.starts_with(b"bplist00") {
            PLIST_BINARY_FORMAT
        } else {
            100
        };
        unsafe { *(format_out as *mut u64) = format; }
    }

    let result = parse_plist_bytes(slice);
    if result == 0 && error_out != 0 {
        let err = crate::nserror::make_nserror(
            "NSCocoaErrorDomain",
            3840, // NSPropertyListReadCorruptError
        );
        unsafe { *(error_out as *mut u64) = err; }
        log::debug!("NSPropertyListSerialization: failed to parse plist data");
    }
    result
}

/// Parse a plist file at the given filesystem path and return a Foundation object.
/// Returns 0 on failure (not found, not parseable).
pub(crate) fn parse_plist_file(path: &str) -> u64 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return 0,
    };
    parse_plist_bytes(&bytes)
}

/// Parse an XML plist embedded inside a non-plist file (for example a Mach-O
/// `__TEXT,__info_plist` section stored in an executable).
pub(crate) fn parse_embedded_plist_file(path: &str) -> u64 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return 0,
    };
    let Some(xml_bytes) = extract_xml_plist(&bytes) else {
        return 0;
    };
    let Ok(xml) = std::str::from_utf8(xml_bytes) else {
        return 0;
    };
    parse_plist_xml(xml)
}

/// `+[NSPropertyListSerialization propertyListIsValidForFormat:]` (simplified)
pub extern "C" fn shim_plist_is_valid(_cls: u64, _cmd: u64, plist: u64, _format: u64) -> u8 {
    if plist == 0 { return 0; }
    let isa = unsafe { *(plist as *const u64) };
    if isa == nsobj::nsarray_class_addr()
        || isa == nsobj::nsmutablearray_class_addr()
        || is_dict_isa(isa)
        || isa == nsobj::nsstring_class_addr()
        || is_number_isa(isa)
        || isa == nsobj::nsdata_class_addr()
    {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn data_with_nil_plist_returns_zero() {
        assert_eq!(shim_plist_data_with_object(0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn plist_with_nil_data_returns_zero() {
        assert_eq!(shim_plist_with_data(0, 0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn parse_plist_file_nonexistent_returns_zero() {
        assert_eq!(parse_plist_file("/tmp/cf_lite_missing_plist_9999999.plist"), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_plist_file_reads_binary_plist() {
        let path = std::env::temp_dir().join(format!(
            "cf_lite_binary_plist_{}_{}.plist",
            std::process::id(),
            std::thread::current().name().unwrap_or("t")
        ));
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleShortVersionString</key>
  <string>26.4</string>
</dict>
</plist>
"#;
        std::fs::write(&path, xml).unwrap();

        let status = std::process::Command::new("/usr/bin/plutil")
            .arg("-convert")
            .arg("binary1")
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success(), "plutil should create a binary plist for the test");

        let dict = parse_plist_file(path.to_string_lossy().as_ref());
        assert_ne!(dict, 0);

        let key = make_string("CFBundleShortVersionString");
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key);
        assert_string_eq(value, "26.4");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn parse_embedded_plist_file_extracts_xml_from_binary_wrapper() {
        let path = std::env::temp_dir().join(format!(
            "cf_lite_embedded_plist_{}_{}.bin",
            std::process::id(),
            std::thread::current().name().unwrap_or("t")
        ));
        let payload = br#"\x00\x01garbage<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.embedded</string>
</dict>
</plist>tail"#;
        std::fs::write(&path, payload).unwrap();

        let dict = parse_embedded_plist_file(path.to_string_lossy().as_ref());
        assert_ne!(dict, 0);

        let key = make_string("CFBundleIdentifier");
        let value = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key);
        assert_string_eq(value, "com.example.embedded");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn plist_roundtrip_string() {
        let s = make_string("roundtrip");
        let data = shim_plist_data_with_object(0, 0, s, 100, 0, 0);
        assert_ne!(data, 0);
        let parsed = shim_plist_with_data(0, 0, data, 0, 0, 0);
        assert_ne!(parsed, 0);
        assert_string_eq(parsed, "roundtrip");
        free_obj(data);
    }

    #[test]
    fn plist_is_valid_nil_returns_zero() {
        assert_eq!(shim_plist_is_valid(0, 0, 0, 100), 0);
    }

    #[test]
    fn plist_is_valid_number_returns_one() {
        // NSNumber is recognized as a valid plist object.
        let n = crate::nsnumber::make_nsnumber_int(42);
        assert_ne!(n, 0);
        assert_eq!(shim_plist_is_valid(0, 0, n, 100), 1);
    }
}
