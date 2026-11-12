//! NSKeyedArchiver / NSKeyedUnarchiver / NSCoder shims.
//!
//! NSKeyedArchiver layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: data_ptr    (u64 — accumulated NSData output)
//!   offset 16: requiring   (u8 — requiresSecureCoding flag)
//!
//! NSKeyedUnarchiver layout (24 bytes):
//!   offset  0: isa         (u64)
//!   offset  8: data_ptr    (u64 — NSData input)
//!   offset 16: requiring   (u8 — requiresSecureCoding flag)
//!
//! Key-value encoding/decoding uses a process-global store keyed by object
//! address so that we don't need to embed a HashMap pointer in the fixed-size
//! object layout.

use crate::cfstring;
use crate::nsobj;

use std::collections::HashMap;
use std::sync::Mutex;

const DATA_OFF: usize = 8;
const SECURE_OFF: usize = 16;

#[cfg(windows)]
fn host_read(fd: i32, buf: *mut libc::c_void, count: usize) -> isize {
    unsafe { libc::read(fd, buf, count.min(u32::MAX as usize) as u32) as isize }
}

#[cfg(not(windows))]
fn host_read(fd: i32, buf: *mut libc::c_void, count: usize) -> isize {
    unsafe { libc::read(fd, buf, count) as isize }
}

#[cfg(windows)]
fn host_write(fd: i32, buf: *const libc::c_void, count: usize) -> isize {
    unsafe { libc::write(fd, buf, count.min(u32::MAX as usize) as u32) as isize }
}

#[cfg(not(windows))]
fn host_write(fd: i32, buf: *const libc::c_void, count: usize) -> isize {
    unsafe { libc::write(fd, buf, count) as isize }
}

// ═══════════════════════════════════════════════════════════════
// Apple Keyed Archive Format Decoder
// ═══════════════════════════════════════════════════════════════
//
// Apple's NSKeyedArchiver format uses binary plist with this structure:
//   $archiver: "NSKeyedArchiver"
//   $version:  100000
//   $top:      { root: CF$UID(N) }
//   $objects:  [ "$null", obj1, obj2, ... ]
//
// Each object in $objects is either:
//   - A leaf value (string, number, data, date)
//   - A dict with "$class" key pointing to a class descriptor via CF$UID
//     Class descriptors: { "$classname": "Foo", "$classes": ["Foo","NSObject"] }

/// Per-unarchiver decode context for Apple keyed archives.
struct AppleArchiveContext {
    /// Raw plist objects from the `$objects` array.
    objects: Vec<plist::Value>,
    /// Cache: UID → already-decoded Foundation object pointer.
    decoded: HashMap<usize, u64>,
    /// Stack of object UIDs currently being decoded (for `initWithCoder:` dispatch).
    /// The top entry is the object whose `decodeObjectForKey:` calls should resolve against.
    current_uid_stack: Vec<usize>,
}

/// Global map: unarchiver address → Apple archive context.
static APPLE_ARCHIVE_CTX: Mutex<Option<HashMap<u64, AppleArchiveContext>>> = Mutex::new(None);

// ── NIB connection wiring ──────────────────────────────────────

/// A decoded IBOutlet or IBAction connection from a NIB archive.
#[derive(Debug, Clone)]
pub enum NibConnection {
    /// IBOutlet: call `setValue:forKey:` on source with label as key, destination as value.
    Outlet {
        source: u64,
        destination: u64,
        label: String,
    },
    /// IBAction: call `setTarget:` + `setAction:` on source (the control).
    Action {
        source: u64,
        destination: u64,
        label: String,
    },
}

/// Accumulated NIB connections from the most recent unarchive pass.
static NIB_CONNECTIONS: Mutex<Vec<NibConnection>> = Mutex::new(Vec::new());

/// Drain accumulated NIB connections (returns and clears the list).
pub fn drain_nib_connections() -> Vec<NibConnection> {
    let mut guard = NIB_CONNECTIONS.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *guard)
}

/// Push a NIB connection for later wiring.
fn push_nib_connection(conn: NibConnection) {
    let mut guard = NIB_CONNECTIONS.lock().unwrap_or_else(|e| e.into_inner());
    guard.push(conn);
}

/// Check if a plist value represents an Apple keyed archive.
fn is_apple_keyed_archive(dict: &plist::Dictionary) -> bool {
    matches!(
        dict.get("$archiver").and_then(|v| v.as_string()),
        Some("NSKeyedArchiver")
    ) && dict.contains_key("$objects")
        && dict.contains_key("$top")
}

/// Decode the root object from an Apple keyed archive.
///
/// Returns the decoded Foundation object, or 0 on failure.
fn decode_apple_archive(data_bytes: &[u8], unarchiver: u64) -> u64 {
    let Ok(value) = plist::Value::from_reader(std::io::Cursor::new(data_bytes)) else {
        return 0;
    };
    let Some(dict) = value.into_dictionary() else {
        return 0;
    };
    if !is_apple_keyed_archive(&dict) {
        return 0;
    }

    let Some(objects_val) = dict.get("$objects") else {
        return 0;
    };
    let Some(objects_array) = objects_val.as_array() else {
        return 0;
    };
    let Some(top_val) = dict.get("$top") else {
        return 0;
    };
    let Some(top_dict) = top_val.as_dictionary() else {
        return 0;
    };

    let objects: Vec<plist::Value> = objects_array.clone();

    // Create context
    let ctx = AppleArchiveContext {
        objects,
        decoded: HashMap::new(),
        current_uid_stack: Vec::new(),
    };

    {
        let mut guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(unarchiver, ctx);
    }

    // Resolve the root — typically "$top" has a "root" key with a CF$UID
    let root_uid = top_dict
        .get("root")
        .and_then(|v| v.as_uid())
        .map(|u| u.get() as usize);

    let result = if let Some(uid) = root_uid {
        decode_uid(unarchiver, uid)
    } else {
        // Some archives use different top-level key names
        // Try first non-null UID in $top
        let mut result = 0u64;
        for (_key, val) in top_dict {
            if let Some(uid) = val.as_uid() {
                result = decode_uid(unarchiver, uid.get() as usize);
                if result != 0 {
                    break;
                }
            }
        }
        result
    };

    // Clean up context
    {
        let mut guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            map.remove(&unarchiver);
        }
    }

    result
}

/// Decode a single UID from the `$objects` array.
fn decode_uid(unarchiver: u64, uid: usize) -> u64 {
    // UID 0 is always $null
    if uid == 0 {
        return 0;
    }

    // Check cache first
    {
        let guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_ref() {
            if let Some(ctx) = map.get(&unarchiver) {
                if let Some(&cached) = ctx.decoded.get(&uid) {
                    return cached;
                }
            }
        }
    }

    // Get the raw plist value for this UID
    let obj_value = {
        let guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        let Some(map) = guard.as_ref() else {
            return 0;
        };
        let Some(ctx) = map.get(&unarchiver) else {
            return 0;
        };
        if uid >= ctx.objects.len() {
            return 0;
        }
        ctx.objects[uid].clone()
    };

    let result = decode_plist_object(unarchiver, uid, &obj_value);

    // Cache the result
    if result != 0 {
        let mut guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            if let Some(ctx) = map.get_mut(&unarchiver) {
                ctx.decoded.insert(uid, result);
            }
        }
    }

    result
}

/// Decode a plist value into a Foundation object.
fn decode_plist_object(unarchiver: u64, uid: usize, value: &plist::Value) -> u64 {
    match value {
        // $null sentinel
        plist::Value::String(s) if s == "$null" => 0,

        // Leaf string
        plist::Value::String(s) => {
            let c = std::ffi::CString::new(s.as_str()).unwrap_or_default();
            cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
        }

        // Leaf integer
        plist::Value::Integer(n) => {
            if let Some(signed) = n.as_signed() {
                crate::nsnumber::make_nsnumber_int(signed)
            } else if let Some(unsigned) = n.as_unsigned() {
                crate::nsnumber::make_nsnumber_int(unsigned as i64)
            } else {
                0
            }
        }

        // Leaf real
        plist::Value::Real(f) => crate::nsnumber::make_nsnumber_double(*f),

        // Leaf boolean
        plist::Value::Boolean(b) => crate::nsnumber::make_nsnumber_bool(*b),

        // Leaf data
        plist::Value::Data(bytes) => crate::nsdata::make_nsdata(bytes),

        // Leaf date
        plist::Value::Date(d) => crate::nsdate::alloc_nsdate(date_to_ti(d)),

        // UID reference (shouldn't appear at top level, but handle gracefully)
        plist::Value::Uid(u) => decode_uid(unarchiver, u.get() as usize),

        // Array of values
        plist::Value::Array(items) => {
            let decoded: Vec<u64> = items
                .iter()
                .map(|item| {
                    if let Some(uid) = item.as_uid() {
                        decode_uid(unarchiver, uid.get() as usize)
                    } else {
                        decode_plist_object(unarchiver, uid, item)
                    }
                })
                .collect();
            crate::nsarray::make_nsarray(&decoded)
        }

        // Dictionary — may be a coded object or a plain dict
        plist::Value::Dictionary(dict) => {
            if dict.contains_key("$class") {
                decode_coded_object(unarchiver, uid, dict)
            } else {
                // Plain dictionary — decode key/value pairs
                let mut keys = Vec::with_capacity(dict.len());
                let mut vals = Vec::with_capacity(dict.len());
                for (k, v) in dict {
                    let key_ns = {
                        let c = std::ffi::CString::new(k.as_str()).unwrap_or_default();
                        cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
                    };
                    let val_ns = if let Some(uid_ref) = v.as_uid() {
                        decode_uid(unarchiver, uid_ref.get() as usize)
                    } else {
                        decode_plist_object(unarchiver, uid, v)
                    };
                    keys.push(key_ns);
                    vals.push(val_ns);
                }
                crate::nsdictionary::make_nsdictionary(&keys, &vals)
            }
        }

        _ => 0,
    }
}

/// Convert plist::Date to NSDate time interval (seconds since 2001-01-01).
fn date_to_ti(d: &plist::Date) -> f64 {
    // plist dates are XML date strings; convert to NSTimeInterval.
    // Apple reference date: 2001-01-01T00:00:00Z = Unix 978307200
    let xml = d.to_xml_format();
    // Try parsing ISO 8601
    if let Ok(secs_unix) = parse_iso8601_to_unix(&xml) {
        secs_unix - 978_307_200.0
    } else {
        0.0
    }
}

fn parse_iso8601_to_unix(s: &str) -> Result<f64, ()> {
    // Simple ISO 8601 parser for "YYYY-MM-DDTHH:MM:SSZ" format
    // plist dates are always in this format
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return Err(());
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    if date_parts.len() != 3 {
        return Err(());
    }
    let time_str = parts[1].trim_end_matches('Z');
    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 3 {
        return Err(());
    }

    let year: i64 = date_parts[0].parse().map_err(|_| ())?;
    let month: i64 = date_parts[1].parse().map_err(|_| ())?;
    let day: i64 = date_parts[2].parse().map_err(|_| ())?;
    let hour: i64 = time_parts[0].parse().map_err(|_| ())?;
    let min: i64 = time_parts[1].parse().map_err(|_| ())?;
    let sec: i64 = time_parts[2].parse().map_err(|_| ())?;

    // Days from epoch (1970) to given date — simplified (no DST)
    let mut days = 0i64;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for m in 0..(month - 1) as usize {
        days += month_days[m];
    }
    days += day - 1;

    Ok((days * 86400 + hour * 3600 + min * 60 + sec) as f64)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Decode an NSCoding object from a dict with `$class` key.
fn decode_coded_object(unarchiver: u64, uid: usize, dict: &plist::Dictionary) -> u64 {
    // Resolve class name: $class → UID → class descriptor dict → $classname
    let class_uid = dict
        .get("$class")
        .and_then(|v| v.as_uid())
        .map(|u| u.get() as usize);
    let Some(class_uid) = class_uid else {
        return 0;
    };

    let class_name = {
        let guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        let Some(map) = guard.as_ref() else {
            return 0;
        };
        let Some(ctx) = map.get(&unarchiver) else {
            return 0;
        };
        if class_uid >= ctx.objects.len() {
            return 0;
        }
        ctx.objects[class_uid]
            .as_dictionary()
            .and_then(|d| d.get("$classname"))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
    };
    let Some(class_name) = class_name else {
        return 0;
    };

    log::debug!(
        "Apple archive: decoding class '{}' at UID {}",
        class_name,
        uid
    );

    // Short-circuit Foundation types
    match class_name.as_str() {
        "NSString" | "NSMutableString" | "__NSCFString" | "__NSCFConstantString" => {
            return decode_foundation_string(unarchiver, dict);
        }
        "NSArray" | "NSMutableArray" | "__NSArrayI" | "__NSArrayM" | "__NSArray0" => {
            return decode_foundation_array(unarchiver, dict);
        }
        "NSDictionary"
        | "NSMutableDictionary"
        | "__NSDictionaryI"
        | "__NSDictionaryM"
        | "__NSSingleEntryDictionaryI"
        | "__NSDictionary0" => return decode_foundation_dictionary(unarchiver, dict),
        "NSData" | "NSMutableData" | "__NSCFData" | "_NSInlineData" | "NSConcreteData" => {
            return decode_foundation_data(dict);
        }
        "NSNumber" | "__NSCFNumber" | "NSConstantIntegerNumber" | "NSConstantDoubleNumber" => {
            return decode_foundation_number(dict);
        }
        "NSDate" | "__NSDate" | "__NSTaggedDate" => return decode_foundation_date(dict),
        "NSURL" => return decode_foundation_url(unarchiver, dict),
        "NSSet" | "NSMutableSet" | "__NSSetI" | "__NSSetM" => {
            return decode_foundation_set(unarchiver, dict);
        }
        "NSValue" | "NSConcreteValue" => return decode_foundation_value(dict),
        // NIB connection records — decode and store, return a placeholder.
        "NSNibOutletConnector" | "NSNibControlConnector" => {
            return decode_nib_connector(unarchiver, dict, &class_name);
        }
        _ => {}
    }

    // General ObjC class: allocate instance and call initWithCoder:
    decode_coded_object_via_runtime(unarchiver, uid, dict, &class_name)
}

// ── NIB connector decoder ──────────────────────────────────────

/// Decode an `NSNibOutletConnector` or `NSNibControlConnector` from an archive.
///
/// NIB connectors have three keys:
///   - `NSSource`      (UID → the object that receives the connection)
///   - `NSDestination` (UID → the target object)
///   - `NSLabel`       (UID → string: outlet name or action selector)
fn decode_nib_connector(unarchiver: u64, dict: &plist::Dictionary, class_name: &str) -> u64 {
    let source = dict
        .get("NSSource")
        .and_then(|v| v.as_uid())
        .map(|u| decode_uid(unarchiver, u.get() as usize))
        .unwrap_or(0);
    let destination = dict
        .get("NSDestination")
        .and_then(|v| v.as_uid())
        .map(|u| decode_uid(unarchiver, u.get() as usize))
        .unwrap_or(0);
    let label = dict
        .get("NSLabel")
        .and_then(|v| v.as_uid())
        .map(|u| decode_uid(unarchiver, u.get() as usize))
        .unwrap_or(0);
    let label_str = if label != 0 {
        crate::cfstring::extract_cfstring_cstr(label).unwrap_or_default()
    } else {
        ""
    };

    if source != 0 && destination != 0 && !label_str.is_empty() {
        let conn = if class_name == "NSNibOutletConnector" {
            NibConnection::Outlet {
                source,
                destination,
                label: label_str.to_string(),
            }
        } else {
            NibConnection::Action {
                source,
                destination,
                label: label_str.to_string(),
            }
        };
        log::debug!("NIB connector: {:?}", conn);
        push_nib_connection(conn);
    }
    // Return a non-zero placeholder so the connector isn't treated as a failure.
    // We use a sentinel value that won't be dereferenced.
    0xBEEF_C000_u64
}

// ── Foundation type decoders ───────────────────────────────────────

fn decode_foundation_string(unarchiver: u64, dict: &plist::Dictionary) -> u64 {
    // NS.string → UID pointing to the actual string value
    if let Some(uid_val) = dict.get("NS.string").and_then(|v| v.as_uid()) {
        return decode_uid(unarchiver, uid_val.get() as usize);
    }
    // Inline string (some archives embed the string directly)
    if let Some(s) = dict.get("NS.string").and_then(|v| v.as_string()) {
        let c = std::ffi::CString::new(s).unwrap_or_default();
        return cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0);
    }
    // NS.bytes → data containing UTF-8 string bytes
    if let Some(bytes) = dict.get("NS.bytes").and_then(|v| v.as_data()) {
        let s = String::from_utf8_lossy(bytes);
        let c = std::ffi::CString::new(s.as_ref()).unwrap_or_default();
        return cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0);
    }
    0
}

fn decode_foundation_array(unarchiver: u64, dict: &plist::Dictionary) -> u64 {
    let Some(ns_objects) = dict.get("NS.objects") else {
        return crate::nsarray::make_nsarray(&[]);
    };
    match ns_objects {
        plist::Value::Array(items) => {
            let decoded: Vec<u64> = items
                .iter()
                .map(|item| {
                    if let Some(uid) = item.as_uid() {
                        decode_uid(unarchiver, uid.get() as usize)
                    } else {
                        0
                    }
                })
                .collect();
            crate::nsarray::make_nsarray(&decoded)
        }
        _ => crate::nsarray::make_nsarray(&[]),
    }
}

fn decode_foundation_dictionary(unarchiver: u64, dict: &plist::Dictionary) -> u64 {
    let keys_val = dict.get("NS.keys");
    let vals_val = dict.get("NS.objects");

    let (keys_raw, vals_raw) = match (keys_val, vals_val) {
        (Some(plist::Value::Array(k)), Some(plist::Value::Array(v))) => (k, v),
        _ => return crate::nsdictionary::make_nsdictionary(&[], &[]),
    };

    let keys: Vec<u64> = keys_raw
        .iter()
        .map(|item| {
            if let Some(uid) = item.as_uid() {
                decode_uid(unarchiver, uid.get() as usize)
            } else {
                0
            }
        })
        .collect();
    let vals: Vec<u64> = vals_raw
        .iter()
        .map(|item| {
            if let Some(uid) = item.as_uid() {
                decode_uid(unarchiver, uid.get() as usize)
            } else {
                0
            }
        })
        .collect();

    crate::nsdictionary::make_nsdictionary(&keys, &vals)
}

fn decode_foundation_data(dict: &plist::Dictionary) -> u64 {
    if let Some(bytes) = dict.get("NS.data").and_then(|v| v.as_data()) {
        return crate::nsdata::make_nsdata(bytes);
    }
    if let Some(bytes) = dict.get("NS.bytes").and_then(|v| v.as_data()) {
        return crate::nsdata::make_nsdata(bytes);
    }
    crate::nsdata::make_nsdata(&[])
}

fn decode_foundation_number(dict: &plist::Dictionary) -> u64 {
    // NS.intval
    if let Some(n) = dict.get("NS.intval") {
        if let Some(plist::Value::Integer(iv)) = Some(n) {
            if let Some(v) = iv.as_signed() {
                return crate::nsnumber::make_nsnumber_int(v);
            }
            if let Some(v) = iv.as_unsigned() {
                return crate::nsnumber::make_nsnumber_int(v as i64);
            }
        }
        if let plist::Value::Real(f) = n {
            return crate::nsnumber::make_nsnumber_int(*f as i64);
        }
    }
    // NS.dblval
    if let Some(plist::Value::Real(f)) = dict.get("NS.dblval") {
        return crate::nsnumber::make_nsnumber_double(*f);
    }
    // NS.number — general
    if let Some(n) = dict.get("NS.number") {
        match n {
            plist::Value::Integer(iv) => {
                if let Some(v) = iv.as_signed() {
                    return crate::nsnumber::make_nsnumber_int(v);
                }
            }
            plist::Value::Real(f) => {
                return crate::nsnumber::make_nsnumber_double(*f);
            }
            plist::Value::Boolean(b) => {
                return crate::nsnumber::make_nsnumber_bool(*b);
            }
            _ => {}
        }
    }
    0
}

fn decode_foundation_date(dict: &plist::Dictionary) -> u64 {
    // NS.time — seconds since 2001-01-01 (NSDate reference date)
    if let Some(plist::Value::Real(ti)) = dict.get("NS.time") {
        return crate::nsdate::alloc_nsdate(*ti);
    }
    if let Some(plist::Value::Integer(iv)) = dict.get("NS.time") {
        if let Some(v) = iv.as_signed() {
            return crate::nsdate::alloc_nsdate(v as f64);
        }
    }
    crate::nsdate::alloc_nsdate(0.0)
}

fn decode_foundation_url(unarchiver: u64, dict: &plist::Dictionary) -> u64 {
    // NS.relative → UID of the URL string
    let relative = dict
        .get("NS.relative")
        .and_then(|v| v.as_uid())
        .map(|u| decode_uid(unarchiver, u.get() as usize))
        .unwrap_or(0);
    if relative == 0 {
        return 0;
    }
    // Allocate NSURL via initWithString:
    let url_obj = crate::pal_alloc().calloc(1, 32) as u64;
    if url_obj == 0 {
        return 0;
    }
    unsafe {
        *(url_obj as *mut u64) = nsobj::nsurl_class_addr();
    }
    crate::nsurl::shim_nsurl_init_with_string(url_obj, 0, relative)
}

fn decode_foundation_set(unarchiver: u64, dict: &plist::Dictionary) -> u64 {
    let Some(plist::Value::Array(items)) = dict.get("NS.objects") else {
        return crate::nsset::shim_nsset_set(0, 0);
    };
    let decoded: Vec<u64> = items
        .iter()
        .map(|item| {
            if let Some(uid) = item.as_uid() {
                decode_uid(unarchiver, uid.get() as usize)
            } else {
                0
            }
        })
        .collect();
    // Build array then set from array
    let arr = crate::nsarray::make_nsarray(&decoded);
    crate::nsset::shim_nsset_set_with_array(0, 0, arr)
}

fn decode_foundation_value(dict: &plist::Dictionary) -> u64 {
    // NSValue stores type-specific keys
    if let Some(s) = dict.get("NS.pointval").and_then(|v| v.as_string()) {
        // "{x, y}" format
        let obj = crate::pal_alloc().calloc(1, 64) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = nsobj::nsvalue_class_addr();
        }
        // Parse point and store
        if let Some((x, y)) = parse_point_string(s) {
            unsafe {
                *((obj as usize + 8) as *mut f64) = x;
                *((obj as usize + 16) as *mut f64) = y;
            }
        }
        return obj;
    }
    if let Some(s) = dict.get("NS.rectval").and_then(|v| v.as_string()) {
        let obj = crate::pal_alloc().calloc(1, 64) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = nsobj::nsvalue_class_addr();
        }
        if let Some((x, y, w, h)) = parse_rect_string(s) {
            unsafe {
                *((obj as usize + 8) as *mut f64) = x;
                *((obj as usize + 16) as *mut f64) = y;
                *((obj as usize + 24) as *mut f64) = w;
                *((obj as usize + 32) as *mut f64) = h;
            }
        }
        return obj;
    }
    0
}

fn parse_point_string(s: &str) -> Option<(f64, f64)> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x = parts[0].trim().parse().ok()?;
        let y = parts[1].trim().parse().ok()?;
        Some((x, y))
    } else {
        None
    }
}

fn parse_rect_string(s: &str) -> Option<(f64, f64, f64, f64)> {
    // "{{x, y}, {w, h}}" format
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    // Split on "}, {"
    let parts: Vec<&str> = s.split("}, {").collect();
    if parts.len() == 2 {
        let p1 = parts[0].trim().trim_start_matches('{');
        let p2 = parts[1].trim().trim_end_matches('}');
        let xy: Vec<&str> = p1.split(',').collect();
        let wh: Vec<&str> = p2.split(',').collect();
        if xy.len() == 2 && wh.len() == 2 {
            let x = xy[0].trim().parse().ok()?;
            let y = xy[1].trim().parse().ok()?;
            let w = wh[0].trim().parse().ok()?;
            let h = wh[1].trim().parse().ok()?;
            return Some((x, y, w, h));
        }
    }
    None
}

fn rect_from_value_object(obj: u64) -> Option<(f64, f64, f64, f64)> {
    if obj == 0 {
        return None;
    }

    if let Some(s) = cfstring::extract_cfstring_cstr(obj) {
        return parse_rect_string(s);
    }

    let isa = unsafe { *(obj as *const u64) };
    if isa != nsobj::nsvalue_class_addr() {
        return None;
    }

    Some(unsafe {
        (
            *((obj as usize + 8) as *const f64),
            *((obj as usize + 16) as *const f64),
            *((obj as usize + 24) as *const f64),
            *((obj as usize + 32) as *const f64),
        )
    })
}

pub(crate) fn decode_object_for_any_key(coder: u64, keys: &[&str]) -> u64 {
    if coder == 0 {
        return 0;
    }

    for key in keys {
        let key_obj = make_nsstring(key);
        let value = shim_nskeyedunarchiver_decode_object_for_key(coder, 0, key_obj);
        if value != 0 {
            return value;
        }
    }
    0
}

pub(crate) fn decode_string_for_any_key(coder: u64, keys: &[&str]) -> Option<String> {
    if coder == 0 {
        return None;
    }

    for key in keys {
        let key_obj = make_nsstring(key);
        let value = shim_nskeyedunarchiver_decode_object_for_key(coder, 0, key_obj);
        if value == 0 {
            continue;
        }
        if let Some(s) = cfstring::extract_cfstring_cstr(value) {
            return Some(s.to_string());
        }
    }
    None
}

pub(crate) fn contains_value_for_any_key(coder: u64, keys: &[&str]) -> bool {
    if coder == 0 {
        return false;
    }

    keys.iter().any(|key| {
        let key_obj = make_nsstring(key);
        shim_nskeyedunarchiver_contains_value_for_key(coder, 0, key_obj) != 0
    })
}

pub(crate) fn decode_integer_for_any_key(coder: u64, keys: &[&str]) -> Option<i64> {
    if !contains_value_for_any_key(coder, keys) {
        return None;
    }

    for key in keys {
        let key_obj = make_nsstring(key);
        if shim_nskeyedunarchiver_contains_value_for_key(coder, 0, key_obj) != 0 {
            return Some(shim_nskeyedunarchiver_decode_integer_for_key(
                coder, 0, key_obj,
            ));
        }
    }
    None
}

pub(crate) fn decode_bool_for_any_key(coder: u64, keys: &[&str]) -> Option<bool> {
    if !contains_value_for_any_key(coder, keys) {
        return None;
    }

    for key in keys {
        let key_obj = make_nsstring(key);
        if shim_nskeyedunarchiver_contains_value_for_key(coder, 0, key_obj) != 0 {
            return Some(shim_nskeyedunarchiver_decode_bool_for_key(coder, 0, key_obj) != 0);
        }
    }
    None
}

pub(crate) fn decode_rect_for_any_key(coder: u64, keys: &[&str]) -> Option<(f64, f64, f64, f64)> {
    if coder == 0 {
        return None;
    }

    for key in keys {
        let key_obj = make_nsstring(key);
        let value = shim_nskeyedunarchiver_decode_object_for_key(coder, 0, key_obj);
        if let Some(rect) = rect_from_value_object(value) {
            return Some(rect);
        }
    }
    None
}

/// Decode an object via ObjC runtime dispatch (for custom/guest classes).
fn decode_coded_object_via_runtime(
    unarchiver: u64,
    uid: usize,
    dict: &plist::Dictionary,
    class_name: &str,
) -> u64 {
    let class_name_c = std::ffi::CString::new(class_name).unwrap_or_default();
    let cls = get_class_by_name(class_name_c.as_ptr());
    if cls == 0 {
        log::debug!("Apple archive: class '{}' not found in runtime", class_name);
        return 0;
    }

    // Allocate instance
    let alloc_imp = lookup_imp(cls, "alloc");
    let instance = if alloc_imp != 0 {
        let csel = std::ffi::CString::new("alloc").unwrap();
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(alloc_imp) };
        f(cls, csel.as_ptr() as u64)
    } else {
        let obj = crate::pal_alloc().calloc(1, 128) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = cls;
            }
        }
        obj
    };
    if instance == 0 {
        return 0;
    }

    // Push this UID as current object for decode method dispatch
    {
        let mut guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            if let Some(ctx) = map.get_mut(&unarchiver) {
                ctx.current_uid_stack.push(uid);
            }
        }
    }

    // Also populate the flat ARCHIVER_STORE as fallback for decode methods
    // that use the old path. Decode nested UIDs before taking the store mutex:
    // storyboard/nib graphs recurse through this path heavily.
    let decoded_entries: Vec<(String, u64)> = dict
        .iter()
        .filter_map(|(k, v)| {
            if k == "$class" {
                return None;
            }
            let foundation_val = if let Some(uid_ref) = v.as_uid() {
                decode_uid(unarchiver, uid_ref.get() as usize)
            } else {
                decode_plist_object(unarchiver, uid, v)
            };
            Some((k.clone(), foundation_val))
        })
        .collect();

    {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.get_or_insert_with(HashMap::new);
        let entries = store.entry(unarchiver).or_default();
        for (key, value) in decoded_entries {
            entries.insert(key, value);
        }
    }

    // Call initWithCoder:
    let init_imp = lookup_imp(instance, "initWithCoder:");
    let result = if init_imp != 0 {
        log::debug!(
            "Apple archive: calling initWithCoder: on '{}' instance {:#x}",
            class_name,
            instance
        );
        let csel = std::ffi::CString::new("initWithCoder:").unwrap();
        let f: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(init_imp) };
        f(instance, csel.as_ptr() as u64, unarchiver)
    } else {
        log::debug!(
            "Apple archive: '{}' has no initWithCoder:, returning raw instance",
            class_name
        );
        instance
    };

    // Pop current UID
    {
        let mut guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            if let Some(ctx) = map.get_mut(&unarchiver) {
                ctx.current_uid_stack.pop();
            }
        }
    }

    // Clean up flat store entry
    {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = guard.as_mut() {
            m.remove(&unarchiver);
        }
    }

    result
}

/// Look up a key in the current Apple archive object and resolve its UID.
///
/// Called by `decodeObjectForKey:` and friends when an Apple archive context
/// is active. Returns the decoded Foundation object, or 0 if not found.
fn apple_decode_key(unarchiver: u64, key: &str) -> Option<u64> {
    let guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.as_ref()?;
    let ctx = map.get(&unarchiver)?;
    let uid = *ctx.current_uid_stack.last()?;
    let obj = ctx.objects.get(uid)?;
    let dict = obj.as_dictionary()?;

    let val = dict.get(key)?;
    // Drop the lock before recursing
    let uid_ref = val.as_uid().map(|u| u.get() as usize);
    let inline_val = if uid_ref.is_none() {
        Some(val.clone())
    } else {
        None
    };
    drop(guard);

    if let Some(uid_val) = uid_ref {
        Some(decode_uid(unarchiver, uid_val))
    } else if let Some(v) = inline_val {
        Some(decode_plist_object(unarchiver, uid, &v))
    } else {
        None
    }
}

// ── ObjC runtime FFI (method dispatch) ─────────────────────────────────────
//
// These symbols live in objc-runtime, which is only linked in the full binary.
// In standalone cf-lite tests they are not available, so all helpers fall back
// gracefully to 0 / null when the symbols cannot be resolved.

/// Look up IMP for `sel_str` on `obj` and return it, or 0 if the method
/// is not registered in our runtime's table.  Returns 0 in standalone
/// test builds where the ObjC runtime is not linked.
pub(crate) fn lookup_imp(obj: u64, sel_str: &str) -> u64 {
    if let Some(imp) = runtime_lookup_imp(obj, sel_str) {
        return imp;
    }

    let Ok(csel) = std::ffi::CString::new(sel_str) else {
        return 0;
    };
    let sel_val = csel.as_ptr() as u64;

    let Some(has_sym) = crate::pal_symbols().resolve("objc_has_method") else {
        return 0;
    };
    let has_fn: extern "C" fn(u64, u64) -> u8 = unsafe { std::mem::transmute(has_sym) };
    if has_fn(obj, sel_val) == 0 {
        return 0;
    }

    let Some(lookup_sym) = crate::pal_symbols().resolve("objc_msg_lookup_imp") else {
        return 0;
    };
    let lookup_fn: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(lookup_sym) };
    lookup_fn(obj, sel_val)
}

fn runtime_lookup_imp(obj: u64, sel_str: &str) -> Option<u64> {
    if obj == 0 {
        return None;
    }

    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_ref()?;

    if let Some(dispatch_isa) = rt.dispatch_isa_for_receiver(obj) {
        return rt.msg_lookup(dispatch_isa, sel_str);
    }

    let raw_isa = unsafe { *(obj as *const u64) };
    let isa = if rt.is_known_isa(raw_isa) {
        raw_isa
    } else {
        normalize_runtime_isa(raw_isa)
    };
    rt.msg_lookup(isa, sel_str)
}

fn normalize_runtime_isa(raw_isa: u64) -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        raw_isa & 0x007f_ffff_ffff_fff8
    }
    #[cfg(target_arch = "x86_64")]
    {
        raw_isa & 0x0000_7fff_ffff_fff8
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        raw_isa
    }
}

// ── Runtime helpers resolved via dlsym ─────────────────────────────────────
//
// These functions live in objc-runtime, which isn't linked during standalone
// cf-lite tests.  We resolve them lazily via dlsym so that the linker doesn't
// complain about missing symbols.

/// Call `class_getName(cls)` if available, returning the C string pointer.
fn get_class_name_cstr(cls: u64) -> *const std::ffi::c_char {
    if let Some(name) = runtime_class_name_cstr(cls) {
        return name;
    }

    let Some(sym) = crate::pal_symbols().resolve("shim_class_get_name") else {
        return std::ptr::null();
    };
    let f: extern "C" fn(u64) -> *const std::ffi::c_char = unsafe { std::mem::transmute(sym) };
    f(cls)
}

fn runtime_class_name_cstr(cls: u64) -> Option<*const std::ffi::c_char> {
    let name = {
        let guard = objc_runtime::OBJC_RUNTIME
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let rt = guard.as_ref()?;
        rt.class_name_for_isa(cls).map(ToOwned::to_owned)?
    };

    let cstring = std::ffi::CString::new(name).ok()?;
    Some(Box::leak(cstring.into_boxed_c_str()).as_ptr())
}

/// Call `objc_getClass(name)` if available, returning the class address.
fn get_class_by_name(name: *const std::ffi::c_char) -> u64 {
    if let Some(cls) = runtime_class_by_name(name) {
        return cls;
    }

    let Some(sym) = crate::pal_symbols().resolve("shim_objc_get_class") else {
        return 0;
    };
    let f: extern "C" fn(*const std::ffi::c_char) -> u64 = unsafe { std::mem::transmute(sym) };
    f(name)
}

fn runtime_class_by_name(name: *const std::ffi::c_char) -> Option<u64> {
    if name.is_null() {
        return None;
    }
    let class_name = unsafe { std::ffi::CStr::from_ptr(name) }.to_str().ok()?;

    let guard = objc_runtime::OBJC_RUNTIME
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let rt = guard.as_ref()?;
    rt.get_class(class_name).map(|class| class.mapped_addr)
}

// ── Global encode/decode store ─────────────────────────────────────────────
//
// Maps archiver/unarchiver object address → (key string → encoded object u64).
// Integer/bool primitives are wrapped as NSNumber before storage.

static ARCHIVER_STORE: Mutex<Option<HashMap<u64, HashMap<String, u64>>>> = Mutex::new(None);

// ── NSCoder abstract base stubs ────────────────────────────────────────────

/// `-[NSCoder allowsKeyedCoding]` — always YES; subclasses are keyed coders.
pub extern "C" fn shim_nscoder_allows_keyed_coding(_obj: u64, _cmd: u64) -> u8 {
    1
}

/// `-[NSCoder description]`
pub extern "C" fn shim_nscoder_description(_obj: u64, _cmd: u64) -> u64 {
    let c = std::ffi::CString::new("<NSCoder>").unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn store_encode(archiver: u64, key: &str, value: u64) {
    let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.entry(archiver)
        .or_default()
        .insert(key.to_string(), value);
}

fn store_decode(archiver: u64, key: &str) -> u64 {
    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|m| m.get(&archiver))
        .and_then(|kv| kv.get(key))
        .copied()
        .unwrap_or(0)
}

fn key_string(ns_key: u64) -> String {
    cfstring::extract_cfstring_cstr(ns_key)
        .unwrap_or("")
        .to_string()
}

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

// ═══════════════════════════════════════════════════════════════
// NSKeyedArchiver
// ═══════════════════════════════════════════════════════════════

fn alloc_archiver() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nskeyedarchiver_class_addr();
    }
    obj
}

/// `+[NSKeyedArchiver alloc]`
pub extern "C" fn shim_nskeyedarchiver_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_archiver()
}

/// `-[NSKeyedArchiver initRequiringSecureCoding:]`
pub extern "C" fn shim_nskeyedarchiver_init_secure(obj: u64, _cmd: u64, secure: u8) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + SECURE_OFF) as *mut u8) = secure;
    }
    obj
}

/// `+[NSKeyedArchiver archivedDataWithRootObject:requiringSecureCoding:error:]`
///
/// If the root object responds to `encodeWithCoder:`, we dispatch through the
/// ObjC runtime so that custom classes can archive their properties.  After the
/// call, the encoded key-value pairs (stored in `ARCHIVER_STORE`) are collected
/// into an NSDictionary with a `$class` key holding the class name, then
/// serialised to binary plist and returned as NSData.
///
/// For plain plist types (NSString, NSNumber, …) that do not implement
/// `encodeWithCoder:`, we fall back to direct plist serialisation.
pub extern "C" fn shim_nskeyedarchiver_archived_data(
    _cls: u64,
    _cmd: u64,
    root: u64,
    _secure: u8,
    error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    if root == 0 {
        return crate::nsdata::shim_nsdata_data(0, 0);
    }

    // Try NSCoding protocol dispatch first
    let imp = lookup_imp(root, "encodeWithCoder:");
    if imp != 0 {
        // Create a temporary archiver object that will collect encoded pairs
        let archiver = alloc_archiver();
        if archiver != 0 {
            let csel = std::ffi::CString::new("encodeWithCoder:").unwrap();
            let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
            f(root, csel.as_ptr() as u64, archiver);

            // Collect encoded pairs into an NSDictionary
            let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(
                crate::nsobj::nsmutabledictionary_class_addr(),
                0,
            );
            if dict != 0 {
                // Store the class name by looking up class_getName via dlsym
                let isa = unsafe { *(root as *const u64) };
                let class_name_cstr = get_class_name_cstr(isa);
                if !class_name_cstr.is_null() {
                    let class_key = make_nsstring("$class");
                    let class_val =
                        cfstring::shim_cf_string_create_with_cstring(0, class_name_cstr, 0);
                    crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(
                        dict, 0, class_val, class_key,
                    );
                }

                // Copy all encoded key-value pairs
                {
                    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(entries) = guard.as_ref().and_then(|m| m.get(&archiver)) {
                        for (k, v) in entries {
                            let k_ns = make_nsstring(k);
                            crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(
                                dict, 0, *v, k_ns,
                            );
                        }
                    }
                }

                // Clean up archiver's store entry
                {
                    let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(m) = guard.as_mut() {
                        m.remove(&archiver);
                    }
                }

                // Serialise to plist data
                let plist_data = crate::nspropertylistserialization::shim_plist_data_with_object(
                    0, 0, dict, 200, /* binary */
                    0, 0,
                );
                if plist_data != 0 {
                    return plist_data;
                }
            }
        }
    }

    // Fallback: attempt plist serialization of the root directly
    let plist_data = crate::nspropertylistserialization::shim_plist_data_with_object(
        0, 0, root, 200, /* binary */
        0, 0,
    );
    if plist_data != 0 {
        return plist_data;
    }
    // Fallback: return empty data
    crate::nsdata::shim_nsdata_data(0, 0)
}

/// `+[NSKeyedArchiver archivedDataWithRootObject:]`  (deprecated but common)
pub extern "C" fn shim_nskeyedarchiver_archived_data_legacy(
    _cls: u64,
    _cmd: u64,
    root: u64,
) -> u64 {
    shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0)
}

/// `+[NSKeyedArchiver archiveRootObject:toFile:]`
///
/// Archives `root` to a file at `path`.  Uses the in-memory store to serialise
/// the root object's encoded key-value pairs as a flat plist-style byte blob,
/// then writes them to the file.  Returns YES on success.
pub extern "C" fn shim_nskeyedarchiver_archive_root_to_file(
    _cls: u64,
    _cmd: u64,
    root: u64,
    path: u64,
) -> u8 {
    let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
    if data == 0 {
        return 0;
    }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else {
        return 0;
    };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        )
    };
    if fd < 0 {
        return 0;
    }
    let len = crate::nsdata::shim_nsdata_length(data, 0);
    let bytes = crate::nsdata::shim_nsdata_bytes(data, 0);
    let mut written = 0usize;
    if len > 0 && bytes != 0 {
        let rc = host_write(fd, bytes as *const libc::c_void, len as usize);
        if rc > 0 {
            written = rc as usize;
        }
    }
    unsafe {
        libc::close(fd);
    }
    u8::from(written == len as usize)
}

/// `-[NSKeyedArchiver encodedData]`
pub extern "C" fn shim_nskeyedarchiver_encoded_data(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + DATA_OFF) as *const u64) }
}

/// `-[NSKeyedArchiver setRequiresSecureCoding:]`
pub extern "C" fn shim_nskeyedarchiver_set_requires_secure(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + SECURE_OFF) as *mut u8) = val;
    }
}

/// `-[NSKeyedArchiver requiresSecureCoding]`
pub extern "C" fn shim_nskeyedarchiver_requires_secure(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SECURE_OFF) as *const u8) }
}

/// `-[NSKeyedArchiver setOutputFormat:]` — no-op stub.
pub extern "C" fn shim_nskeyedarchiver_set_output_format(_obj: u64, _cmd: u64, _format: u64) {}

/// `-[NSKeyedArchiver outputFormat]` — returns NSPropertyListBinaryFormat_v1_0 (200).
pub extern "C" fn shim_nskeyedarchiver_output_format(_obj: u64, _cmd: u64) -> u64 {
    200
}

/// `-[NSKeyedArchiver finishEncoding]`
pub extern "C" fn shim_nskeyedarchiver_finish_encoding(_obj: u64, _cmd: u64) {
    log::debug!("NSKeyedArchiver finishEncoding (stub)");
}

// ── Key-value encode methods ────────────────────────────────────────────────

/// `-[NSKeyedArchiver encodeObject:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_object_for_key(
    obj: u64,
    _cmd: u64,
    value: u64,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    store_encode(obj, &key_string(key), value);
}

/// `-[NSKeyedArchiver encodeInteger:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_integer_for_key(
    obj: u64,
    _cmd: u64,
    value: i64,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let ns = crate::nsnumber::shim_nsnumber_number_with_integer(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeBool:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_bool_for_key(
    obj: u64,
    _cmd: u64,
    value: u8,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let ns = crate::nsnumber::shim_nsnumber_number_with_bool(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeDouble:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_double_for_key(
    obj: u64,
    _cmd: u64,
    value: f64,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let ns = crate::nsnumber::shim_nsnumber_number_with_double(0, 0, value);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeFloat:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_float_for_key(
    obj: u64,
    _cmd: u64,
    value: f32,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let ns = crate::nsnumber::shim_nsnumber_number_with_double(0, 0, value as f64);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeInt:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int_for_key(
    obj: u64,
    _cmd: u64,
    value: i32,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let ns = crate::nsnumber::shim_nsnumber_number_with_integer(0, 0, value as i64);
    store_encode(obj, &key_string(key), ns);
}

/// `-[NSKeyedArchiver encodeInt32:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int32_for_key(
    obj: u64,
    _cmd: u64,
    value: i32,
    key: u64,
) {
    shim_nskeyedarchiver_encode_int_for_key(obj, _cmd, value, key);
}

/// `-[NSKeyedArchiver encodeInt64:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_int64_for_key(
    obj: u64,
    _cmd: u64,
    value: i64,
    key: u64,
) {
    shim_nskeyedarchiver_encode_integer_for_key(obj, _cmd, value, key);
}

pub extern "C" fn shim_nskeyedarchiver_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSKeyedArchiver>")
}

// ═══════════════════════════════════════════════════════════════
// NSKeyedUnarchiver
// ═══════════════════════════════════════════════════════════════

fn alloc_unarchiver() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nskeyedunarchiver_class_addr();
    }
    obj
}

/// `+[NSKeyedUnarchiver alloc]`
pub extern "C" fn shim_nskeyedunarchiver_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_unarchiver()
}

/// `-[NSKeyedUnarchiver initForReadingFromData:error:]`
pub extern "C" fn shim_nskeyedunarchiver_init_from_data(
    obj: u64,
    _cmd: u64,
    data: u64,
    error_out: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    unsafe {
        *((obj as usize + DATA_OFF) as *mut u64) = data;
    }
    obj
}

/// `+[NSKeyedUnarchiver unarchivedObjectOfClass:fromData:error:]`
pub extern "C" fn shim_nskeyedunarchiver_unarchived_object(
    _cls: u64,
    _cmd: u64,
    _allowedclass: u64,
    data: u64,
    error_out: u64,
) -> u64 {
    if error_out != 0 {
        unsafe {
            *(error_out as *mut u64) = 0;
        }
    }
    if data == 0 {
        return 0;
    }

    // Get raw bytes to check for Apple keyed archive format
    let len = crate::nsdata::shim_nsdata_length(data, 0) as usize;
    let bytes_ptr = crate::nsdata::shim_nsdata_bytes(data, 0);
    if len > 0 && bytes_ptr != 0 {
        let raw_bytes = unsafe { std::slice::from_raw_parts(bytes_ptr as *const u8, len) };

        // Try Apple keyed archive format first
        if let Ok(value) = plist::Value::from_reader(std::io::Cursor::new(raw_bytes)) {
            if let Some(dict) = value.as_dictionary() {
                if is_apple_keyed_archive(dict) {
                    let unarchiver = alloc_unarchiver();
                    if unarchiver != 0 {
                        unsafe {
                            *((unarchiver as usize + DATA_OFF) as *mut u64) = data;
                        }
                        // Apple format is authoritative — return result even if 0 (null root)
                        return decode_apple_archive(raw_bytes, unarchiver);
                    }
                }
            }
        }
    }

    // Fall back to existing flat format decoder
    let dict = crate::nspropertylistserialization::shim_plist_with_data(0, 0, data, 0, 0, 0);
    if dict == 0 {
        return 0;
    }

    // Check if the result is a dictionary (NSCoding archive) — if not, return it as-is
    let isa = unsafe { *(dict as *const u64) };
    let dict_class = nsobj::nsdictionary_class_addr();
    let mdict_class = nsobj::nsmutabledictionary_class_addr();
    let cdict_class = nsobj::nsconstantdictionary_class_addr();
    let is_dict = isa == dict_class || isa == mdict_class || isa == cdict_class;
    if !is_dict {
        return dict;
    }

    // Check if this is an NSCoding archive (has "$class" key)
    let class_key = make_nsstring("$class");
    let class_val = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, class_key);
    if class_val == 0 {
        // Not an NSCoding archive — return the deserialized plist value directly
        return dict;
    }

    // Get the class name and look up the class
    let Some(class_name) = cfstring::extract_cfstring_cstr(class_val) else {
        return dict;
    };
    let class_name_c = std::ffi::CString::new(class_name).unwrap_or_default();

    let cls = get_class_by_name(class_name_c.as_ptr());
    if cls == 0 {
        return dict;
    }
    log::debug!(
        "NSKeyedUnarchiver: restoring class '{}' (cls={:#x})",
        class_name,
        cls
    );

    // Allocate an instance of the class
    let alloc_imp = lookup_imp(cls, "alloc");
    let instance = if alloc_imp != 0 {
        let csel = std::ffi::CString::new("alloc").unwrap();
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(alloc_imp) };
        f(cls, csel.as_ptr() as u64)
    } else {
        // Manual alloc: read instance_size from ClassInfo isn't directly accessible,
        // so use a reasonable default (128 bytes covers most Foundation objects)
        let obj = crate::pal_alloc().calloc(1, 128) as u64;
        if obj != 0 {
            unsafe {
                *(obj as *mut u64) = cls;
            }
        }
        obj
    };
    if instance == 0 {
        return dict;
    }

    // Create a temporary unarchiver and populate its store from the dict
    let unarchiver = alloc_unarchiver();
    if unarchiver == 0 {
        return dict;
    }
    unsafe {
        *((unarchiver as usize + DATA_OFF) as *mut u64) = data;
    }

    // Populate the unarchiver's ARCHIVER_STORE from the NSDictionary entries
    {
        let all_keys = crate::nsdictionary::shim_nsdictionary_all_keys(dict, 0);
        if all_keys != 0 {
            let count = crate::nsarray::shim_nsarray_count(all_keys, 0);
            for i in 0..count {
                let k = crate::nsarray::shim_nsarray_object_at_index(all_keys, 0, i);
                // Skip the "$class" meta-key (compare by string content, not pointer)
                if let Some(ks) = cfstring::extract_cfstring_cstr(k) {
                    if ks == "$class" {
                        continue;
                    }
                    let v = crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, k);
                    store_encode(unarchiver, ks, v);
                }
            }
        }
    }

    // Call initWithCoder: on the instance
    let init_imp = lookup_imp(instance, "initWithCoder:");
    let result = if init_imp != 0 {
        log::debug!(
            "NSKeyedUnarchiver: calling initWithCoder: on instance {:#x}",
            instance
        );
        let csel = std::ffi::CString::new("initWithCoder:").unwrap();
        let f: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(init_imp) };
        f(instance, csel.as_ptr() as u64, unarchiver)
    } else {
        log::debug!(
            "NSKeyedUnarchiver: '{}' has no initWithCoder:, returning raw instance",
            class_name
        );
        instance
    };
    log::debug!("NSKeyedUnarchiver: result={:#x}", result);

    // Clean up unarchiver store
    {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = guard.as_mut() {
            m.remove(&unarchiver);
        }
    }

    result
}

/// `+[NSKeyedUnarchiver unarchiveObjectWithData:]` (deprecated but common)
pub extern "C" fn shim_nskeyedunarchiver_unarchive_legacy(_cls: u64, _cmd: u64, data: u64) -> u64 {
    shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0)
}

/// `+[NSKeyedUnarchiver unarchiveObjectWithFile:]` — reads file into NSData, then unarchives.
pub extern "C" fn shim_nskeyedunarchiver_unarchive_from_file(
    _cls: u64,
    _cmd: u64,
    path: u64,
) -> u64 {
    if path == 0 {
        return 0;
    }
    let Some(p) = cfstring::extract_cfstring_cstr(path) else {
        return 0;
    };
    let c_path = std::ffi::CString::new(p).unwrap_or_default();
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return 0;
    }
    let mut result = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = host_read(fd, buf.as_mut_ptr().cast(), buf.len());
        if n <= 0 {
            break;
        }
        result.extend_from_slice(&buf[..n as usize]);
    }
    unsafe {
        libc::close(fd);
    }
    if result.is_empty() {
        return 0;
    }
    let data = crate::nsdata::make_nsdata(&result);
    shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0)
}

/// `-[NSKeyedUnarchiver setRequiresSecureCoding:]`
pub extern "C" fn shim_nskeyedunarchiver_set_requires_secure(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + SECURE_OFF) as *mut u8) = val;
    }
}

/// `-[NSKeyedUnarchiver requiresSecureCoding]`
pub extern "C" fn shim_nskeyedunarchiver_requires_secure(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + SECURE_OFF) as *const u8) }
}

/// `-[NSKeyedUnarchiver setDecodingFailurePolicy:]` — no-op stub.
pub extern "C" fn shim_nskeyedunarchiver_set_decoding_failure_policy(
    _obj: u64,
    _cmd: u64,
    _policy: i64,
) {
}

/// `-[NSKeyedUnarchiver decodingFailurePolicy]` — returns 0 (NSDecodingFailureRaiseException).
pub extern "C" fn shim_nskeyedunarchiver_decoding_failure_policy(_obj: u64, _cmd: u64) -> i64 {
    0
}

/// `-[NSKeyedUnarchiver finishDecoding]`
pub extern "C" fn shim_nskeyedunarchiver_finish_decoding(_obj: u64, _cmd: u64) {
    log::debug!("NSKeyedUnarchiver finishDecoding (stub)");
}

// ── Key-value decode methods ────────────────────────────────────────────────

/// `-[NSKeyedUnarchiver decodeObjectForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_object_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Try Apple archive context first
    let k = key_string(key);
    if let Some(result) = apple_decode_key(obj, &k) {
        return result;
    }
    // Fall back to flat store
    store_decode(obj, &k)
}

/// `-[NSKeyedUnarchiver decodeIntegerForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_integer_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> i64 {
    if obj == 0 {
        return 0;
    }
    let k = key_string(key);
    let ns = if let Some(v) = apple_decode_key(obj, &k) {
        v
    } else {
        store_decode(obj, &k)
    };
    if ns == 0 {
        return 0;
    }
    crate::nsnumber::shim_nsnumber_integer_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeBoolForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_bool_for_key(obj: u64, _cmd: u64, key: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let k = key_string(key);
    let ns = if let Some(v) = apple_decode_key(obj, &k) {
        v
    } else {
        store_decode(obj, &k)
    };
    if ns == 0 {
        return 0;
    }
    crate::nsnumber::shim_nsnumber_bool_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeDoubleForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_double_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    let k = key_string(key);
    let ns = if let Some(v) = apple_decode_key(obj, &k) {
        v
    } else {
        store_decode(obj, &k)
    };
    if ns == 0 {
        return 0.0;
    }
    crate::nsnumber::shim_nsnumber_double_value(ns, 0)
}

/// `-[NSKeyedUnarchiver decodeFloatForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_float_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> f32 {
    shim_nskeyedunarchiver_decode_double_for_key(obj, _cmd, key) as f32
}

/// `-[NSKeyedUnarchiver decodeInt32ForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_int32_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> i32 {
    shim_nskeyedunarchiver_decode_integer_for_key(obj, _cmd, key) as i32
}

/// `-[NSKeyedUnarchiver decodeInt64ForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_int64_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> i64 {
    shim_nskeyedunarchiver_decode_integer_for_key(obj, _cmd, key)
}

pub extern "C" fn shim_nskeyedunarchiver_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSKeyedUnarchiver>")
}

/// `-[NSKeyedUnarchiver containsValueForKey:]`
pub extern "C" fn shim_nskeyedunarchiver_contains_value_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
) -> u8 {
    if obj == 0 {
        return 0;
    }
    let k = key_string(key);
    // Check Apple archive context first
    {
        let guard = APPLE_ARCHIVE_CTX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_ref() {
            if let Some(ctx) = map.get(&obj) {
                if let Some(&uid) = ctx.current_uid_stack.last() {
                    if let Some(dict) = ctx.objects.get(uid).and_then(|v| v.as_dictionary()) {
                        return u8::from(dict.contains_key(&k));
                    }
                }
            }
        }
    }
    // Fall back to flat store
    let guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let found = guard
        .as_ref()
        .and_then(|m| m.get(&obj))
        .map_or(false, |kv| kv.contains_key(&k));
    u8::from(found)
}

/// `-[NSKeyedUnarchiver decodeObjectOfClass:forKey:]` — same as `decodeObjectForKey:`.
pub extern "C" fn shim_nskeyedunarchiver_decode_object_of_class_for_key(
    obj: u64,
    _cmd: u64,
    _cls: u64,
    key: u64,
) -> u64 {
    shim_nskeyedunarchiver_decode_object_for_key(obj, 0, key)
}

// ── Bytes encode/decode ─────────────────────────────────────────────────────

/// `-[NSKeyedArchiver encodeBytes:length:forKey:]`
pub extern "C" fn shim_nskeyedarchiver_encode_bytes_for_key(
    obj: u64,
    _cmd: u64,
    bytes: u64,
    length: u64,
    key: u64,
) {
    if obj == 0 {
        return;
    }
    let data = if bytes != 0 && length > 0 {
        let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, length as usize) };
        crate::nsdata::make_nsdata(slice)
    } else {
        crate::nsdata::shim_nsdata_data(0, 0)
    };
    store_encode(obj, &key_string(key), data);
}

/// `-[NSKeyedUnarchiver decodeBytesForKey:returnedLength:]`
pub extern "C" fn shim_nskeyedunarchiver_decode_bytes_for_key(
    obj: u64,
    _cmd: u64,
    key: u64,
    length_out: *mut u64,
) -> u64 {
    if obj == 0 {
        if !length_out.is_null() {
            unsafe {
                *length_out = 0;
            }
        }
        return 0;
    }
    let data = store_decode(obj, &key_string(key));
    if data == 0 {
        if !length_out.is_null() {
            unsafe {
                *length_out = 0;
            }
        }
        return 0;
    }
    let len = crate::nsdata::shim_nsdata_length(data, 0);
    if !length_out.is_null() {
        unsafe {
            *length_out = len;
        }
    }
    crate::nsdata::shim_nsdata_bytes(data, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const ARCHIVER_SIZE: usize = 24;
    const UNARCHIVER_SIZE: usize = 24;

    fn archiver_isa() -> u64 {
        crate::nsobj::nskeyedarchiver_class_addr()
    }

    fn unarchiver_isa() -> u64 {
        crate::nsobj::nskeyedunarchiver_class_addr()
    }

    /// Helper: clear global ARCHIVER_STORE entries for a given object address.
    fn clear_store(addr: u64) {
        let mut guard = ARCHIVER_STORE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(map) = guard.as_mut() {
            map.remove(&addr);
        }
    }

    // ── NSCoder ────────────────────────────────────────────────────

    #[test]
    fn nscoder_allows_keyed_coding() {
        assert_eq!(shim_nscoder_allows_keyed_coding(0, 0), 1);
        assert_eq!(shim_nscoder_allows_keyed_coding(0xABC, 0), 1);
    }

    #[test]
    fn nscoder_description() {
        let desc = shim_nscoder_description(0, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSCoder>");
    }

    // ── NSKeyedArchiver alloc / init ───────────────────────────────

    #[test]
    fn archiver_alloc() {
        let obj = shim_nskeyedarchiver_alloc(0, 0);
        assert_ne!(obj, 0);
        // ISA should be set to the archiver class
        assert_eq!(read_field(obj, 0), archiver_isa());
        free_obj(obj);
    }

    #[test]
    fn archiver_init_secure_nil() {
        assert_eq!(shim_nskeyedarchiver_init_secure(0, 0, 1), 0);
    }

    #[test]
    fn archiver_init_secure_sets_flag() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let ret = shim_nskeyedarchiver_init_secure(obj, 0, 1);
        assert_eq!(ret, obj);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 1);
        free_obj(obj);
    }

    // ── requiresSecureCoding get/set ──────────────────────────────

    #[test]
    fn archiver_requires_secure_nil() {
        assert_eq!(shim_nskeyedarchiver_requires_secure(0, 0), 0);
    }

    #[test]
    fn archiver_set_requires_secure_nil() {
        // Should not crash
        shim_nskeyedarchiver_set_requires_secure(0, 0, 1);
    }

    #[test]
    fn archiver_requires_secure_roundtrip() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 0);
        shim_nskeyedarchiver_set_requires_secure(obj, 0, 1);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 1);
        shim_nskeyedarchiver_set_requires_secure(obj, 0, 0);
        assert_eq!(shim_nskeyedarchiver_requires_secure(obj, 0), 0);
        free_obj(obj);
    }

    // ── encodedData ───────────────────────────────────────────────

    #[test]
    fn archiver_encoded_data_nil() {
        assert_eq!(shim_nskeyedarchiver_encoded_data(0, 0), 0);
    }

    #[test]
    fn archiver_encoded_data_reads_field() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        write_field(obj, DATA_OFF, 0xCAFE);
        assert_eq!(shim_nskeyedarchiver_encoded_data(obj, 0), 0xCAFE);
        free_obj(obj);
    }

    // ── finishEncoding ────────────────────────────────────────────

    #[test]
    fn archiver_finish_encoding_nil() {
        // Should not crash
        shim_nskeyedarchiver_finish_encoding(0, 0);
    }

    #[test]
    fn archiver_finish_encoding() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        shim_nskeyedarchiver_finish_encoding(obj, 0);
        free_obj(obj);
    }

    // ── description (archiver) ────────────────────────────────────

    #[test]
    fn archiver_description_nil() {
        assert_eq!(shim_nskeyedarchiver_description(0, 0), 0);
    }

    #[test]
    fn archiver_description() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let desc = shim_nskeyedarchiver_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSKeyedArchiver>");
        free_obj(obj);
    }

    // ── encode / decode roundtrip (object) ────────────────────────

    #[test]
    fn encode_object_for_key_nil() {
        // Should not crash
        shim_nskeyedarchiver_encode_object_for_key(0, 0, 42, 0);
    }

    #[test]
    fn encode_decode_object_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("myKey");
        let value = make_string("hello");

        shim_nskeyedarchiver_encode_object_for_key(archiver, 0, value, key);

        // Decode using the same address acts as the unarchiver store key
        let decoded = store_decode(archiver, "myKey");
        assert_eq!(decoded, value);
        assert_string_eq(decoded, "hello");

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (integer) ───────────────────────

    #[test]
    fn encode_integer_for_key_nil() {
        shim_nskeyedarchiver_encode_integer_for_key(0, 0, 42, 0);
    }

    #[test]
    fn encode_decode_integer_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("count");

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 99, key);

        // Use decode on the same address
        let decoded = shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key);
        assert_eq!(decoded, 99);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (bool) ──────────────────────────

    #[test]
    fn encode_bool_for_key_nil() {
        shim_nskeyedarchiver_encode_bool_for_key(0, 0, 1, 0);
    }

    #[test]
    fn encode_decode_bool_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("flag");

        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 1, key);
        let decoded = shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, key);
        assert_eq!(decoded, 1);

        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 0, key);
        let decoded = shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, key);
        assert_eq!(decoded, 0);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (double) ────────────────────────

    #[test]
    fn encode_double_for_key_nil() {
        shim_nskeyedarchiver_encode_double_for_key(0, 0, 3.14, 0);
    }

    #[test]
    fn encode_decode_double_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("pi");

        shim_nskeyedarchiver_encode_double_for_key(archiver, 0, 3.14, key);
        let decoded = shim_nskeyedunarchiver_decode_double_for_key(archiver, 0, key);
        assert!((decoded - 3.14).abs() < 1e-10);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (float) ─────────────────────────

    #[test]
    fn encode_float_for_key_nil() {
        shim_nskeyedarchiver_encode_float_for_key(0, 0, 2.5, 0);
    }

    #[test]
    fn encode_decode_float_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("ratio");

        shim_nskeyedarchiver_encode_float_for_key(archiver, 0, 2.5f32, key);
        let decoded = shim_nskeyedunarchiver_decode_float_for_key(archiver, 0, key);
        assert!((decoded - 2.5f32).abs() < 1e-5);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (int / int32) ───────────────────

    #[test]
    fn encode_int_for_key_nil() {
        shim_nskeyedarchiver_encode_int_for_key(0, 0, 7, 0);
    }

    #[test]
    fn encode_decode_int32_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("small");

        shim_nskeyedarchiver_encode_int32_for_key(archiver, 0, -42, key);
        let decoded = shim_nskeyedunarchiver_decode_int32_for_key(archiver, 0, key);
        assert_eq!(decoded, -42);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── encode / decode roundtrip (int64) ─────────────────────────

    #[test]
    fn encode_decode_int64_roundtrip() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("big");

        shim_nskeyedarchiver_encode_int64_for_key(archiver, 0, i64::MAX, key);
        let decoded = shim_nskeyedunarchiver_decode_int64_for_key(archiver, 0, key);
        assert_eq!(decoded, i64::MAX);

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── decode missing key returns zero ───────────────────────────

    #[test]
    fn decode_missing_key_returns_zero() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let key = make_string("nonexistent");

        assert_eq!(shim_nskeyedunarchiver_decode_object_for_key(obj, 0, key), 0);
        assert_eq!(
            shim_nskeyedunarchiver_decode_integer_for_key(obj, 0, key),
            0
        );
        assert_eq!(shim_nskeyedunarchiver_decode_bool_for_key(obj, 0, key), 0);
        assert!((shim_nskeyedunarchiver_decode_double_for_key(obj, 0, key)).abs() < 1e-10);
        assert!((shim_nskeyedunarchiver_decode_float_for_key(obj, 0, key)).abs() < 1e-5);
        assert_eq!(shim_nskeyedunarchiver_decode_int32_for_key(obj, 0, key), 0);
        assert_eq!(shim_nskeyedunarchiver_decode_int64_for_key(obj, 0, key), 0);

        free_obj(obj);
    }

    // ── NSKeyedUnarchiver alloc ───────────────────────────────────

    #[test]
    fn unarchiver_alloc() {
        let obj = shim_nskeyedunarchiver_alloc(0, 0);
        assert_ne!(obj, 0);
        assert_eq!(read_field(obj, 0), unarchiver_isa());
        free_obj(obj);
    }

    // ── initForReadingFromData ────────────────────────────────────

    #[test]
    fn unarchiver_init_from_data_nil() {
        assert_eq!(shim_nskeyedunarchiver_init_from_data(0, 0, 0, 0), 0);
    }

    #[test]
    fn unarchiver_init_from_data_sets_field() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let fake_data: u64 = 0xBEEF;
        let ret = shim_nskeyedunarchiver_init_from_data(obj, 0, fake_data, 0);
        assert_eq!(ret, obj);
        assert_eq!(read_field(obj, DATA_OFF), fake_data);
        free_obj(obj);
    }

    #[test]
    fn unarchiver_init_from_data_clears_error() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let mut error_out: u64 = 0xBAD;
        let ret = shim_nskeyedunarchiver_init_from_data(
            obj,
            0,
            0xBEEF,
            &mut error_out as *mut u64 as u64,
        );
        assert_eq!(ret, obj);
        assert_eq!(error_out, 0);
        free_obj(obj);
    }

    // ── requiresSecureCoding (unarchiver) ─────────────────────────

    #[test]
    fn unarchiver_requires_secure_nil() {
        assert_eq!(shim_nskeyedunarchiver_requires_secure(0, 0), 0);
    }

    #[test]
    fn unarchiver_set_requires_secure_nil() {
        shim_nskeyedunarchiver_set_requires_secure(0, 0, 1);
    }

    #[test]
    fn unarchiver_requires_secure_roundtrip() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 0);
        shim_nskeyedunarchiver_set_requires_secure(obj, 0, 1);
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 1);
        shim_nskeyedunarchiver_set_requires_secure(obj, 0, 0);
        assert_eq!(shim_nskeyedunarchiver_requires_secure(obj, 0), 0);
        free_obj(obj);
    }

    // ── finishDecoding ────────────────────────────────────────────

    #[test]
    fn unarchiver_finish_decoding_nil() {
        shim_nskeyedunarchiver_finish_decoding(0, 0);
    }

    #[test]
    fn unarchiver_finish_decoding() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        shim_nskeyedunarchiver_finish_decoding(obj, 0);
        free_obj(obj);
    }

    // ── decode nil safety ─────────────────────────────────────────

    #[test]
    fn decode_object_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_object_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_integer_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_integer_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_bool_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_bool_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_double_for_key_nil() {
        assert!((shim_nskeyedunarchiver_decode_double_for_key(0, 0, 0)).abs() < 1e-10);
    }

    #[test]
    fn decode_float_for_key_nil() {
        assert!((shim_nskeyedunarchiver_decode_float_for_key(0, 0, 0)).abs() < 1e-5);
    }

    #[test]
    fn decode_int32_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_int32_for_key(0, 0, 0), 0);
    }

    #[test]
    fn decode_int64_for_key_nil() {
        assert_eq!(shim_nskeyedunarchiver_decode_int64_for_key(0, 0, 0), 0);
    }

    // ── description (unarchiver) ──────────────────────────────────

    #[test]
    fn unarchiver_description_nil() {
        assert_eq!(shim_nskeyedunarchiver_description(0, 0), 0);
    }

    #[test]
    fn unarchiver_description() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        let desc = shim_nskeyedunarchiver_description(obj, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSKeyedUnarchiver>");
        free_obj(obj);
    }

    // ── archived_data nil/null root ───────────────────────────────

    #[test]
    fn archived_data_nil_root_returns_empty_data() {
        let data = shim_nskeyedarchiver_archived_data(0, 0, 0, 0, 0);
        // nil root → empty NSData (shim_nsdata_data returns an empty NSData)
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, 0);
    }

    #[test]
    fn archived_data_clears_error_out() {
        let mut err: u64 = 0xBAD;
        let _data = shim_nskeyedarchiver_archived_data(0, 0, 0, 0, &mut err as *mut u64 as u64);
        assert_eq!(err, 0);
    }

    #[test]
    fn archived_data_with_string_root() {
        let root = make_string("testValue");
        let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
        assert_ne!(data, 0);
        // A string should yield non-empty plist data
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert!(len > 0);
    }

    // ── archived_data_legacy ──────────────────────────────────────

    #[test]
    fn archived_data_legacy_nil_root() {
        let data = shim_nskeyedarchiver_archived_data_legacy(0, 0, 0);
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert_eq!(len, 0);
    }

    // ── unarchived_object ─────────────────────────────────────────

    #[test]
    fn unarchived_object_nil_data() {
        assert_eq!(shim_nskeyedunarchiver_unarchived_object(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn unarchived_object_clears_error_out() {
        let mut err: u64 = 0xBAD;
        let _ = shim_nskeyedunarchiver_unarchived_object(0, 0, 0, 0, &mut err as *mut u64 as u64);
        assert_eq!(err, 0);
    }

    // ── unarchive_legacy ──────────────────────────────────────────

    #[test]
    fn unarchive_legacy_nil_data() {
        assert_eq!(shim_nskeyedunarchiver_unarchive_legacy(0, 0, 0), 0);
    }

    // ── archive_root_to_file nil safety ───────────────────────────

    #[test]
    fn archive_root_to_file_nil_root() {
        // nil root → shim creates empty data → write succeeds only if path valid
        // but we pass nil path, so it should return 0
        assert_eq!(shim_nskeyedarchiver_archive_root_to_file(0, 0, 0, 0), 0);
    }

    #[test]
    fn archive_root_to_file_nil_path() {
        let root = make_string("something");
        assert_eq!(shim_nskeyedarchiver_archive_root_to_file(0, 0, root, 0), 0);
    }

    // ── unarchive_from_file nil safety ────────────────────────────

    #[test]
    fn unarchive_from_file_nil_path() {
        assert_eq!(shim_nskeyedunarchiver_unarchive_from_file(0, 0, 0), 0);
    }

    // ── archive → unarchive roundtrip via plist ───────────────────

    #[test]
    fn archive_unarchive_string_roundtrip() {
        let root = make_string("roundtrip");
        let data = shim_nskeyedarchiver_archived_data(0, 0, root, 0, 0);
        assert_ne!(data, 0);
        let len = crate::nsdata::shim_nsdata_length(data, 0);
        assert!(len > 0);
        let unarchived = shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0);
        // The plist round trip should recover the string
        if unarchived != 0 {
            assert_string_eq(unarchived, "roundtrip");
        }
    }

    // ── multiple keys on same archiver ────────────────────────────

    #[test]
    fn encode_multiple_keys() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let k1 = make_string("name");
        let k2 = make_string("age");
        let k3 = make_string("active");

        let v1 = make_string("Alice");
        shim_nskeyedarchiver_encode_object_for_key(archiver, 0, v1, k1);
        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 30, k2);
        shim_nskeyedarchiver_encode_bool_for_key(archiver, 0, 1, k3);

        assert_eq!(
            shim_nskeyedunarchiver_decode_object_for_key(archiver, 0, k1),
            v1
        );
        assert_eq!(
            shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, k2),
            30
        );
        assert_eq!(
            shim_nskeyedunarchiver_decode_bool_for_key(archiver, 0, k3),
            1
        );

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── overwrite key ─────────────────────────────────────────────

    #[test]
    fn encode_overwrite_key() {
        let archiver = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        let key = make_string("val");

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 1, key);
        assert_eq!(
            shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key),
            1
        );

        shim_nskeyedarchiver_encode_integer_for_key(archiver, 0, 2, key);
        assert_eq!(
            shim_nskeyedunarchiver_decode_integer_for_key(archiver, 0, key),
            2
        );

        clear_store(archiver);
        free_obj(archiver);
    }

    // ── outputFormat get/set ──────────────────────────────────────

    #[test]
    fn archiver_output_format_returns_binary() {
        assert_eq!(shim_nskeyedarchiver_output_format(0, 0), 200);
    }

    #[test]
    fn archiver_set_output_format_noop() {
        let obj = alloc_obj(ARCHIVER_SIZE, archiver_isa());
        shim_nskeyedarchiver_set_output_format(obj, 0, 100); // XML
                                                             // getter always returns 200 (stub)
        assert_eq!(shim_nskeyedarchiver_output_format(obj, 0), 200);
        free_obj(obj);
    }

    #[test]
    fn archiver_set_output_format_nil() {
        // Should not crash
        shim_nskeyedarchiver_set_output_format(0, 0, 200);
    }

    // ── decodingFailurePolicy get/set (unarchiver) ────────────────

    #[test]
    fn unarchiver_decoding_failure_policy_returns_zero() {
        assert_eq!(shim_nskeyedunarchiver_decoding_failure_policy(0, 0), 0);
    }

    #[test]
    fn unarchiver_set_decoding_failure_policy_noop() {
        let obj = alloc_obj(UNARCHIVER_SIZE, unarchiver_isa());
        shim_nskeyedunarchiver_set_decoding_failure_policy(obj, 0, 1);
        // getter always returns 0 (stub)
        assert_eq!(shim_nskeyedunarchiver_decoding_failure_policy(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn unarchiver_set_decoding_failure_policy_nil() {
        // Should not crash
        shim_nskeyedunarchiver_set_decoding_failure_policy(0, 0, 1);
    }

    // ── Apple keyed archive format ────────────────────────────────

    /// Helper: build an Apple keyed archive as binary plist bytes.
    fn build_apple_archive(objects: Vec<plist::Value>, root_uid: u64) -> Vec<u8> {
        let mut top = plist::Dictionary::new();
        top.insert(
            "root".to_string(),
            plist::Value::Uid(plist::Uid::new(root_uid)),
        );

        let mut archive = plist::Dictionary::new();
        archive.insert(
            "$archiver".to_string(),
            plist::Value::String("NSKeyedArchiver".to_string()),
        );
        archive.insert(
            "$version".to_string(),
            plist::Value::Integer(100_000.into()),
        );
        archive.insert("$top".to_string(), plist::Value::Dictionary(top));
        archive.insert("$objects".to_string(), plist::Value::Array(objects));

        let mut buf = Vec::new();
        plist::to_writer_binary(&mut buf, &plist::Value::Dictionary(archive)).unwrap();
        buf
    }

    fn unarchive_bytes(bytes: &[u8]) -> u64 {
        let data = crate::nsdata::make_nsdata(bytes);
        shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0)
    }

    #[test]
    fn apple_archive_string_leaf() {
        // $objects: ["$null", "Hello World"]
        // $top: { root: UID(1) }
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::String("Hello World".to_string()),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_string_eq(result, "Hello World");
    }

    #[test]
    fn apple_archive_integer_leaf() {
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Integer(42.into()),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsnumber::shim_nsnumber_integer_value(result, 0), 42);
    }

    #[test]
    fn apple_archive_bool_leaf() {
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Boolean(true),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsnumber::shim_nsnumber_bool_value(result, 0), 1);
    }

    #[test]
    fn apple_archive_real_leaf() {
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Real(3.14),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        let val = crate::nsnumber::shim_nsnumber_double_value(result, 0);
        assert!((val - 3.14).abs() < 0.001);
    }

    #[test]
    fn apple_archive_data_leaf() {
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Data(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(result, 0), 4);
    }

    #[test]
    fn apple_archive_null_uid() {
        // UID 0 is always $null → should return 0
        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::String("unused".to_string()),
        ];
        let bytes = build_apple_archive(objects, 0);
        let result = unarchive_bytes(&bytes);
        assert_eq!(result, 0);
    }

    #[test]
    fn apple_archive_nsstring_coded() {
        // NSString coded object: {$class: UID(3), NS.string: UID(2)}
        // $objects[2] = "Hello"
        // $objects[3] = class descriptor
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(3)));
        obj_dict.insert(
            "NS.string".to_string(),
            plist::Value::Uid(plist::Uid::new(2)),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSString".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSString".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::String("Hello".to_string()),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_string_eq(result, "Hello");
    }

    #[test]
    fn apple_archive_nsarray_coded() {
        // NSArray: {$class: UID(4), NS.objects: [UID(2), UID(3)]}
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(4)));
        obj_dict.insert(
            "NS.objects".to_string(),
            plist::Value::Array(vec![
                plist::Value::Uid(plist::Uid::new(2)),
                plist::Value::Uid(plist::Uid::new(3)),
            ]),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSArray".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSArray".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::String("first".to_string()),
            plist::Value::String("second".to_string()),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(result, 0), 2);
        let e0 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        assert_string_eq(e0, "first");
        let e1 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 1);
        assert_string_eq(e1, "second");
    }

    #[test]
    fn apple_archive_nsdictionary_coded() {
        // NSDictionary: {$class: UID(5), NS.keys: [UID(2)], NS.objects: [UID(3)]}
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(5)));
        obj_dict.insert(
            "NS.keys".to_string(),
            plist::Value::Array(vec![plist::Value::Uid(plist::Uid::new(2))]),
        );
        obj_dict.insert(
            "NS.objects".to_string(),
            plist::Value::Array(vec![plist::Value::Uid(plist::Uid::new(3))]),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSDictionary".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSDictionary".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::String("key1".to_string()),
            plist::Value::String("value1".to_string()),
            plist::Value::Integer(999.into()),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsdictionary::shim_nsdictionary_count(result, 0), 1);
    }

    #[test]
    fn apple_archive_nsdata_coded() {
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(2)));
        obj_dict.insert("NS.data".to_string(), plist::Value::Data(vec![1, 2, 3]));

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSData".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSData".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsdata::shim_nsdata_length(result, 0), 3);
    }

    #[test]
    fn apple_archive_nsnumber_int_coded() {
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(2)));
        obj_dict.insert("NS.intval".to_string(), plist::Value::Integer(42.into()));

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSNumber".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSNumber".to_string()),
                plist::Value::String("NSValue".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsnumber::shim_nsnumber_integer_value(result, 0), 42);
    }

    #[test]
    fn apple_archive_nsnumber_double_coded() {
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(2)));
        obj_dict.insert("NS.dblval".to_string(), plist::Value::Real(2.718));

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSNumber".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![plist::Value::String("NSNumber".to_string())]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        let val = crate::nsnumber::shim_nsnumber_double_value(result, 0);
        assert!((val - 2.718).abs() < 0.001);
    }

    #[test]
    fn apple_archive_nsdate_coded() {
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(2)));
        obj_dict.insert("NS.time".to_string(), plist::Value::Real(694224000.0)); // 2023-01-05

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSDate".to_string()),
        );
        class_desc.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSDate".to_string()),
                plist::Value::String("NSObject".to_string()),
            ]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
    }

    #[test]
    fn apple_archive_nested_string_array() {
        // Root is NSArray of [NSString("a"), NSMutableString("b")]
        // $objects: $null, array_dict, NSString_dict, "a", NSString_class, NSMutableString_dict, "b", NSMutableString_class
        let mut str_a_dict = plist::Dictionary::new();
        str_a_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(4)));
        str_a_dict.insert(
            "NS.string".to_string(),
            plist::Value::Uid(plist::Uid::new(3)),
        );

        let mut str_b_dict = plist::Dictionary::new();
        str_b_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(7)));
        str_b_dict.insert(
            "NS.string".to_string(),
            plist::Value::Uid(plist::Uid::new(6)),
        );

        let mut nsstring_class = plist::Dictionary::new();
        nsstring_class.insert(
            "$classname".to_string(),
            plist::Value::String("NSString".to_string()),
        );
        nsstring_class.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![plist::Value::String("NSString".to_string())]),
        );

        let mut nsmutstring_class = plist::Dictionary::new();
        nsmutstring_class.insert(
            "$classname".to_string(),
            plist::Value::String("NSMutableString".to_string()),
        );
        nsmutstring_class.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("NSMutableString".to_string()),
                plist::Value::String("NSString".to_string()),
            ]),
        );

        let mut array_dict = plist::Dictionary::new();
        array_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(8)));
        array_dict.insert(
            "NS.objects".to_string(),
            plist::Value::Array(vec![
                plist::Value::Uid(plist::Uid::new(2)), // str_a_dict
                plist::Value::Uid(plist::Uid::new(5)), // str_b_dict
            ]),
        );

        let mut nsarray_class = plist::Dictionary::new();
        nsarray_class.insert(
            "$classname".to_string(),
            plist::Value::String("NSArray".to_string()),
        );
        nsarray_class.insert(
            "$classes".to_string(),
            plist::Value::Array(vec![plist::Value::String("NSArray".to_string())]),
        );

        let objects = vec![
            plist::Value::String("$null".to_string()),   // 0
            plist::Value::Dictionary(array_dict),        // 1 - root
            plist::Value::Dictionary(str_a_dict),        // 2
            plist::Value::String("a".to_string()),       // 3
            plist::Value::Dictionary(nsstring_class),    // 4
            plist::Value::Dictionary(str_b_dict),        // 5
            plist::Value::String("b".to_string()),       // 6
            plist::Value::Dictionary(nsmutstring_class), // 7
            plist::Value::Dictionary(nsarray_class),     // 8
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(result, 0), 2);
        let e0 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        assert_string_eq(e0, "a");
        let e1 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 1);
        assert_string_eq(e1, "b");
    }

    #[test]
    fn apple_archive_is_detected_correctly() {
        // Non-Apple archive data should NOT use Apple decoder
        let xml = b"<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><string>hello</string></plist>";
        let data = crate::nsdata::make_nsdata(xml);
        let result = shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0);
        assert_ne!(result, 0);
        assert_string_eq(result, "hello");
    }

    #[test]
    fn apple_archive_nsstring_with_bytes() {
        // NSString with NS.bytes instead of NS.string
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(2)));
        obj_dict.insert(
            "NS.bytes".to_string(),
            plist::Value::Data(b"utf8text".to_vec()),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSString".to_string()),
        );
        class_desc.insert("$classes".to_string(), plist::Value::Array(vec![]));

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_string_eq(result, "utf8text");
    }

    #[test]
    fn apple_archive_nsset_coded() {
        let mut obj_dict = plist::Dictionary::new();
        obj_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(4)));
        obj_dict.insert(
            "NS.objects".to_string(),
            plist::Value::Array(vec![
                plist::Value::Uid(plist::Uid::new(2)),
                plist::Value::Uid(plist::Uid::new(3)),
            ]),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSSet".to_string()),
        );
        class_desc.insert("$classes".to_string(), plist::Value::Array(vec![]));

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(obj_dict),
            plist::Value::String("alpha".to_string()),
            plist::Value::String("beta".to_string()),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsset::shim_nsset_count(result, 0), 2);
    }

    #[test]
    fn apple_archive_shared_uid_dedup() {
        // Same UID referenced twice → should only decode once (cache hit)
        let mut array_dict = plist::Dictionary::new();
        array_dict.insert("$class".to_string(), plist::Value::Uid(plist::Uid::new(3)));
        array_dict.insert(
            "NS.objects".to_string(),
            plist::Value::Array(vec![
                plist::Value::Uid(plist::Uid::new(2)),
                plist::Value::Uid(plist::Uid::new(2)), // same UID again
            ]),
        );

        let mut class_desc = plist::Dictionary::new();
        class_desc.insert(
            "$classname".to_string(),
            plist::Value::String("NSArray".to_string()),
        );
        class_desc.insert("$classes".to_string(), plist::Value::Array(vec![]));

        let objects = vec![
            plist::Value::String("$null".to_string()),
            plist::Value::Dictionary(array_dict),
            plist::Value::String("shared".to_string()),
            plist::Value::Dictionary(class_desc),
        ];
        let bytes = build_apple_archive(objects, 1);
        let result = unarchive_bytes(&bytes);
        assert_ne!(result, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(result, 0), 2);
        let e0 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        let e1 = crate::nsarray::shim_nsarray_object_at_index(result, 0, 1);
        // Both should point to the same decoded string
        assert_eq!(e0, e1);
        assert_string_eq(e0, "shared");
    }

    #[test]
    fn drain_nib_connections_empty_by_default() {
        let conns = drain_nib_connections();
        assert!(conns.is_empty());
    }

    #[test]
    fn push_and_drain_outlet_connection() {
        push_nib_connection(NibConnection::Outlet {
            source: 0x1000,
            destination: 0x2000,
            label: "myOutlet".to_string(),
        });
        let conns = drain_nib_connections();
        assert_eq!(conns.len(), 1);
        match &conns[0] {
            NibConnection::Outlet {
                source,
                destination,
                label,
            } => {
                assert_eq!(*source, 0x1000);
                assert_eq!(*destination, 0x2000);
                assert_eq!(label, "myOutlet");
            }
            _ => panic!("expected Outlet"),
        }
        // Drain again should be empty.
        assert!(drain_nib_connections().is_empty());
    }

    #[test]
    fn push_and_drain_action_connection() {
        push_nib_connection(NibConnection::Action {
            source: 0x3000,
            destination: 0x4000,
            label: "buttonClicked:".to_string(),
        });
        let conns = drain_nib_connections();
        assert_eq!(conns.len(), 1);
        match &conns[0] {
            NibConnection::Action {
                source,
                destination,
                label,
            } => {
                assert_eq!(*source, 0x3000);
                assert_eq!(*destination, 0x4000);
                assert_eq!(label, "buttonClicked:");
            }
            _ => panic!("expected Action"),
        }
    }
}
