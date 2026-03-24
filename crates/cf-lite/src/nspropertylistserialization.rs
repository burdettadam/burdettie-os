//! NSPropertyListSerialization shim: plist ↔ Foundation object conversion.
//!
//! Supports XML plist format only (simplified). Uses basic XML generation
//! and parsing without external dependencies.
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

/// `NSPropertyListXMLFormat_v1_0 = 100`
const _PLIST_XML_FORMAT: u64 = 100;

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

    // NSNumber
    if isa == nsobj::nsnumber_class_addr() {
        let tag = unsafe { *((obj + 8) as *const u64) };
        let raw = unsafe { *((obj + 16) as *const u64) };
        return match tag {
            2 => { // bool
                if raw != 0 { format!("{pad}<true/>") } else { format!("{pad}<false/>") }
            }
            0 => format!("{pad}<integer>{}</integer>", raw as i64), // int
            1 => format!("{pad}<real>{}</real>", f64::from_bits(raw)), // double
            _ => format!("{pad}<integer>0</integer>"),
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

    // NSDictionary
    if isa == nsobj::nsdictionary_class_addr() || isa == nsobj::nsmutabledictionary_class_addr() {
        let count = unsafe { *((obj + 8) as *const u64) } as usize;
        let keys_ptr = unsafe { *((obj + 16) as *const u64) };
        let vals_ptr = unsafe { *((obj + 24) as *const u64) };
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
    _format: u64,
    error_out: u64,
) -> u64 {
    if data == 0 { return 0; }

    let length = unsafe { *((data + 8) as *const u64) } as usize;
    let bytes_ptr = unsafe { *((data + 16) as *const u64) } as *const u8;
    if bytes_ptr.is_null() || length == 0 { return 0; }

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, length) };
    let xml = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => {
            if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
            return 0;
        }
    };

    // Skip XML header/plist wrapper to find the first value element
    let body = xml
        .find("<plist")
        .and_then(|i| xml[i..].find('>').map(|j| &xml[i + j + 1..]))
        .unwrap_or(xml);
    let body = body.strip_suffix("</plist>").unwrap_or(body);

    let (result, _) = parse_xml_value(body.trim());
    if result == 0 && error_out != 0 {
        unsafe { *(error_out as *mut u64) = 0; }
    }
    result
}

/// `+[NSPropertyListSerialization propertyListIsValidForFormat:]` (simplified)
pub extern "C" fn shim_plist_is_valid(_cls: u64, _cmd: u64, plist: u64, _format: u64) -> u8 {
    if plist == 0 { return 0; }
    let isa = unsafe { *(plist as *const u64) };
    if isa == nsobj::nsarray_class_addr()
        || isa == nsobj::nsmutablearray_class_addr()
        || isa == nsobj::nsdictionary_class_addr()
        || isa == nsobj::nsmutabledictionary_class_addr()
        || isa == nsobj::nsstring_class_addr()
        || isa == nsobj::nsnumber_class_addr()
        || isa == nsobj::nsdata_class_addr()
    {
        1
    } else {
        0
    }
}
