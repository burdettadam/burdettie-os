//! NSBundle shim: application bundle access.
//!
//! Object layout (heap-allocated, singleton for mainBundle):
//!
//!   offset  0: isa         (u64 — pointer to NSBundle class)
//!   offset  8: bundle_path (u64 — pointer to NSString with bundle path)
//!
//! Total: 16 bytes per NSBundle object.

use std::sync::OnceLock;

const NSBUNDLE_PATH_OFF: usize = 8;

static MAIN_BUNDLE: OnceLock<u64> = OnceLock::new();

pub fn get_or_create_main_bundle() -> u64 {
    *MAIN_BUNDLE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 16) as u64;
        if obj == 0 {
            return 0;
        }

        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsbundle_class_addr();
        }

        // Use the executable path from shim config, derive bundle dir
        let exe_path = libsystem_shims::process::process_path()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/usr/local/bin/unknown".to_string());

        // Detect .app bundle layout: Foo.app/Contents/MacOS/Foo
        // Walk up from the executable to find the .app root.
        let bundle_path = detect_app_bundle_root(&exe_path)
            .unwrap_or_else(|| {
                // Flat CLI: bundle path = directory containing the executable
                std::path::Path::new(&exe_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string())
            });

        let cstr = std::ffi::CString::new(bundle_path).unwrap_or_default();
        let nsstr = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
        unsafe {
            *((obj as usize + NSBUNDLE_PATH_OFF) as *mut u64) = nsstr;
        }

        obj
    })
}

/// Walk up from an executable path to detect a .app bundle root.
///
/// Standard layout: `Foo.app/Contents/MacOS/Foo`, but Xcode developer tools
/// also live deeper inside the bundle (for example
/// `Xcode.app/Contents/Developer/usr/bin/xcodebuild`).
///
/// We therefore walk ancestors until we find a directory ending in `.app`.
pub(crate) fn detect_app_bundle_root(exe_path: &str) -> Option<String> {
    let path = std::path::Path::new(exe_path);
    for ancestor in path.ancestors().skip(1) {
        let Some(name) = ancestor.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".app") {
            return Some(ancestor.to_string_lossy().to_string());
        }
    }
    None
}

/// `+[NSBundle mainBundle]` — returns the main bundle singleton.
pub extern "C" fn shim_nsbundle_main_bundle(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_main_bundle()
}

/// `-[NSBundle bundlePath]` — returns the bundle directory path.
pub extern "C" fn shim_nsbundle_bundle_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) }
}

fn nsstring_path_to_file_url(path_ptr: u64) -> u64 {
    if path_ptr == 0 {
        return 0;
    }

    crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path_ptr)
}

/// `-[NSBundle resourcePath]` — returns bundlePath + "/Contents/Resources"
/// (for a standard .app bundle), or just bundlePath for flat CLI bundles.
pub extern "C" fn shim_nsbundle_resource_path(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    if path_ptr == 0 {
        return 0;
    }

    // Read the bundle path string
    let utf8 = crate::cfstring::shim_nsstring_utf8string(path_ptr, 0);
    if utf8.is_null() {
        return path_ptr;
    }

    let path_str = unsafe { std::ffi::CStr::from_ptr(utf8) }
        .to_str()
        .unwrap_or("");

    // For .app bundles, Contents/Resources exists; for flat CLIs, just return bundlePath
    let resources = std::path::Path::new(path_str).join("Contents/Resources");
    if resources.exists() {
        let cstr = std::ffi::CString::new(resources.to_string_lossy().as_ref()).unwrap_or_default();
        crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
    } else {
        path_ptr
    }
}

/// `-[NSBundle bundleURL]` — returns the bundle directory as a file URL.
pub extern "C" fn shim_nsbundle_bundle_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    nsstring_path_to_file_url(shim_nsbundle_bundle_path(obj, 0))
}

/// `-[NSBundle executablePath]` — returns the executable path.
pub extern "C" fn shim_nsbundle_executable_path(_obj: u64, _cmd: u64) -> u64 {
    let exe_path = libsystem_shims::process::process_path()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/usr/local/bin/unknown".to_string());

    let cstr = std::ffi::CString::new(exe_path).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

/// `-[NSBundle executableURL]` — returns the executable path as a file URL.
pub extern "C" fn shim_nsbundle_executable_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    nsstring_path_to_file_url(shim_nsbundle_executable_path(obj, 0))
}

/// `-[NSBundle bundleIdentifier]` — returns nil for CLI tools.
pub extern "C" fn shim_nsbundle_bundle_identifier(_obj: u64, _cmd: u64) -> u64 {
    0 // nil — CLI tools typically don't have a bundle identifier
}

/// `+[NSBundle bundleWithPath:]` — returns a bundle object for any path.
pub extern "C" fn shim_nsbundle_bundle_with_path(_cls: u64, _cmd: u64, path: u64) -> u64 {
    if path == 0 { return 0; }
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsbundle_class_addr();
        *((obj as usize + NSBUNDLE_PATH_OFF) as *mut u64) = path;
    }
    obj
}

/// `+[NSBundle allFrameworks]` — returns bundles found under `Contents/SharedFrameworks`.
pub extern "C" fn shim_nsbundle_all_frameworks(_cls: u64, _cmd: u64) -> u64 {
    let main_bundle = get_or_create_main_bundle();
    if main_bundle == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }

    let bundle_path = shim_nsbundle_bundle_path(main_bundle, 0);
    let Some(bundle_path_str) = crate::cfstring::extract_cfstring_cstr(bundle_path) else {
        return crate::nsarray::make_nsarray(&[]);
    };

    let frameworks_dir = std::path::Path::new(bundle_path_str).join("Contents/SharedFrameworks");
    let mut bundles = Vec::new();

    if let Ok(entries) = std::fs::read_dir(frameworks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.ends_with(".framework") {
                continue;
            }

            let cstr = std::ffi::CString::new(path.to_string_lossy().as_ref()).unwrap_or_default();
            let ns_path = crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
            let bundle = shim_nsbundle_bundle_with_path(crate::nsobj::nsbundle_class_addr(), 0, ns_path);
            if bundle != 0 {
                bundles.push(bundle);
            }
        }
    }

    crate::nsarray::make_nsarray(&bundles)
}

/// Parse the Info.plist for a bundle object and return the NSDictionary (or 0).
fn load_info_dict(obj: u64) -> u64 {
    if obj == 0 { return 0; }
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    if path_ptr == 0 { return 0; }
    let path_str = match crate::cfstring::extract_cfstring_cstr(path_ptr) {
        Some(s) => s.to_string(),
        None => return 0,
    };
    // Try standard .app layout: bundle/Contents/Info.plist
    let app_plist = std::path::Path::new(&path_str)
        .join("Contents")
        .join("Info.plist");
    if let Some(p) = app_plist.to_str() {
        let result = crate::nspropertylistserialization::parse_plist_file(p);
        if result != 0 { return result; }
    }
    // Flat layout: bundle/Info.plist
    let flat_plist = std::path::Path::new(&path_str).join("Info.plist");
    if let Some(p) = flat_plist.to_str() {
        let result = crate::nspropertylistserialization::parse_plist_file(p);
        if result != 0 { return result; }
    }
    0
}

/// `-[NSBundle infoDictionary]` — parse and return the bundle's Info.plist.
pub extern "C" fn shim_nsbundle_info_dictionary(obj: u64, _cmd: u64) -> u64 {
    load_info_dict(obj)
}

/// `-[NSBundle objectForInfoDictionaryKey:]` — look up a key in infoDictionary.
pub extern "C" fn shim_nsbundle_object_for_info_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 { return 0; }
    let dict = load_info_dict(obj);
    if dict == 0 { return 0; }
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

/// `-[NSBundle pathForResource:ofType:]` — find a resource file in the bundle.
pub extern "C" fn shim_nsbundle_path_for_resource(
    obj: u64, _cmd: u64, name: u64, r#type: u64,
) -> u64 {
    if obj == 0 || name == 0 { return 0; }

    let name_str = match crate::cfstring::extract_cfstring_cstr(name) {
        Some(s) => s.to_string(),
        None => return 0,
    };
    let type_str = if r#type != 0 {
        crate::cfstring::extract_cfstring_cstr(r#type).map(|s| s.to_string())
    } else {
        None
    };

    // Build filename: "name" or "name.type"
    let filename = match &type_str {
        Some(ext) if !ext.is_empty() => format!("{name_str}.{ext}"),
        _ => name_str.clone(),
    };

    let res_path = shim_nsbundle_resource_path(obj, 0);
    if res_path == 0 { return 0; }
    let res_str = match crate::cfstring::extract_cfstring_cstr(res_path) {
        Some(s) => s.to_string(),
        None => return 0,
    };

    let full_path = std::path::Path::new(&res_str).join(&filename);
    if full_path.exists() {
        let cstr = std::ffi::CString::new(full_path.to_string_lossy().as_ref()).unwrap_or_default();
        return crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr());
    }
    0
}

/// `-[NSBundle loadNibNamed:owner:topLevelObjects:]`
///
/// Finds a `.nib` file in the bundle, unarchives it via NSKeyedUnarchiver,
/// and returns the decoded object graph. Returns YES if the nib was found
/// and loaded, NO otherwise.
pub extern "C" fn shim_nsbundle_load_nib_named(
    obj: u64, _cmd: u64, name: u64, owner: u64, top_out: u64,
) -> u8 {
    if obj == 0 || name == 0 { return 0; }

    let Some(nib_name) = crate::cfstring::extract_cfstring_cstr(name) else { return 0; };

    // Get bundle path
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    if path_ptr == 0 { return 0; }
    let Some(bundle_path) = crate::cfstring::extract_cfstring_cstr(path_ptr) else { return 0; };

    // Search for the .nib file in multiple locations
    let candidates = [
        format!("{}/Contents/Resources/{}.nib", bundle_path, nib_name),
        format!("{}/{}.nib", bundle_path, nib_name),
        format!("{}/Contents/Resources/Base.lproj/{}.nib", bundle_path, nib_name),
        format!("{}/Contents/Resources/en.lproj/{}.nib", bundle_path, nib_name),
    ];

    let mut nib_path = None;
    for candidate in &candidates {
        let p = std::path::Path::new(candidate);
        // NIB can be a directory (contains keyedobjects.nib) or a file
        if p.is_file() {
            nib_path = Some(candidate.clone());
            break;
        }
        let inner = p.join("keyedobjects.nib");
        if inner.is_file() {
            nib_path = Some(inner.to_string_lossy().to_string());
            break;
        }
        let inner2 = p.join("keyedobjects-101.nib");
        if inner2.is_file() {
            nib_path = Some(inner2.to_string_lossy().to_string());
            break;
        }
    }

    let Some(nib_file) = nib_path else {
        log::debug!("NSBundle loadNibNamed: '{}' not found in bundle '{}'", nib_name, bundle_path);
        return 0;
    };

    log::debug!("NSBundle loadNibNamed: loading '{}'", nib_file);

    // Read the nib file
    let Ok(bytes) = std::fs::read(&nib_file) else { return 0; };
    if bytes.is_empty() { return 0; }

    // Unarchive the nib data
    let data = crate::nsdata::make_nsdata(&bytes);
    let result = crate::nskeyedarchiver::shim_nskeyedunarchiver_unarchived_object(
        0, 0, 0, data, 0,
    );

    if result == 0 {
        log::debug!("NSBundle loadNibNamed: unarchive failed for '{}'", nib_file);
        return 0;
    }

    // Wire IBOutlet / IBAction connections decoded from the NIB.
    wire_nib_connections(owner);

    // If topLevelObjects: out-parameter was provided, write the result
    if top_out != 0 {
        // result might be an NSArray of top-level objects, or a single object
        let isa = unsafe { *(result as *const u64) };
        if isa == crate::nsobj::nsarray_class_addr()
            || isa == crate::nsobj::nsmutablearray_class_addr()
        {
            unsafe { *(top_out as *mut u64) = result; }
        } else {
            // Wrap single object in array
            let arr = crate::nsarray::make_nsarray(&[result]);
            unsafe { *(top_out as *mut u64) = arr; }
        }
    }

    log::debug!("NSBundle loadNibNamed: loaded '{}' successfully", nib_name);
    1 // YES
}

/// `+[NSBundle bundleForClass:]` — returns mainBundle.
pub extern "C" fn shim_nsbundle_bundle_for_class(_cls: u64, _cmd: u64, _class: u64) -> u64 {
    get_or_create_main_bundle()
}

/// `-[NSBundle URLForResource:withExtension:]` — stub, returns nil.
pub extern "C" fn shim_nsbundle_url_for_resource(
    _obj: u64, _cmd: u64, _name: u64, _ext: u64,
) -> u64 {
    0
}

/// `-[NSBundle localizedStringForKey:value:table:]`
pub extern "C" fn shim_nsbundle_localized_string(
    _obj: u64, _cmd: u64, key: u64, value: u64, _table: u64,
) -> u64 {
    // If a non-nil, non-empty value is provided, return it as fallback
    if value != 0 {
        if let Some(s) = crate::cfstring::extract_cfstring_cstr(value) {
            if !s.is_empty() {
                return value;
            }
        }
    }
    // Otherwise return the key itself (standard Foundation fallback)
    if key != 0 { key } else { 0 }
}

// ── NIB connection wiring ──────────────────────────────────────

/// Wire IBOutlet and IBAction connections decoded from a NIB archive.
///
/// Outlet connections call `setValue:forKey:` on the source with the label
/// as the key and the destination as the value.
///
/// Action connections call `setTarget:` and `setAction:` on the source
/// (typically an NSControl) with the destination as the target.
fn wire_nib_connections(owner: u64) {
    use crate::nskeyedarchiver::NibConnection;

    let connections = crate::nskeyedarchiver::drain_nib_connections();
    if connections.is_empty() { return; }

    log::debug!("NIB wiring: {} connections to process", connections.len());

    for conn in &connections {
        match conn {
            NibConnection::Outlet { source, destination, label } => {
                // For file's-owner outlets: source might be the placeholder
                // file's owner object (typically UID 2 in NIB archives).
                // If `owner` was provided and source is non-zero, we use owner
                // as the target for KVC.
                let target = if owner != 0 { owner } else { *source };
                let key_c = std::ffi::CString::new(label.as_str()).unwrap_or_default();
                let key_ns = crate::cfstring::alloc_nsstring_from_cstr(key_c.as_ptr());
                log::debug!("NIB wiring outlet: target={:#x} key='{}' value={:#x}",
                    target, label, destination);
                crate::nskeyvaluecoding::shim_set_value_for_key(
                    target, 0, *destination, key_ns,
                );
            }
            NibConnection::Action { source, destination, label } => {
                if *source == 0 { continue; }
                // Set target on the control.
                let set_target_imp = crate::nskeyedarchiver::lookup_imp(*source, "setTarget:");
                if set_target_imp != 0 {
                    let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(set_target_imp) };
                    f(*source, 0, *destination);
                }
                // Set action selector string on the control.
                let set_action_imp = crate::nskeyedarchiver::lookup_imp(*source, "setAction:");
                if set_action_imp != 0 {
                    let sel_c = std::ffi::CString::new(label.as_str()).unwrap_or_default();
                    let sel = crate::cfstring::alloc_nsstring_from_cstr(sel_c.as_ptr());
                    let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(set_action_imp) };
                    f(*source, 0, sel);
                    log::debug!("NIB wiring action: source={:#x} target={:#x} action='{}'",
                        source, destination, label);
                }
            }
        }
    }

    // Call awakeFromNib on all objects that respond to it.
    let mut seen = std::collections::HashSet::new();
    for conn in &connections {
        let objects = match conn {
            NibConnection::Outlet { source, destination, .. } => vec![*source, *destination],
            NibConnection::Action { source, destination, .. } => vec![*source, *destination],
        };
        for obj in objects {
            if obj == 0 || !seen.insert(obj) { continue; }
            let awake_imp = crate::nskeyedarchiver::lookup_imp(obj, "awakeFromNib");
            if awake_imp != 0 {
                let f: extern "C" fn(u64, u64) = unsafe { std::mem::transmute(awake_imp) };
                f(obj, 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const NSBUNDLE_SIZE: usize = 16;

    /// Helper: create an NSBundle object with the given path string stored.
    fn make_bundle_with_path(path: &str) -> (u64, u64) {
        let path_str = make_string(path);
        let obj = alloc_obj(NSBUNDLE_SIZE, crate::nsobj::nsbundle_class_addr());
        write_field(obj, NSBUNDLE_PATH_OFF, path_str);
        (obj, path_str)
    }

    // --- detect_app_bundle_root (private helper, pure string logic) ---

    #[test]
    fn detect_app_bundle_root_standard_layout() {
        let result = detect_app_bundle_root("/Applications/Foo.app/Contents/MacOS/Foo");
        assert_eq!(result, Some("/Applications/Foo.app".to_string()));
    }

    #[test]
    fn detect_app_bundle_root_nested_path() {
        let result = detect_app_bundle_root("/Users/me/Build/Bar.app/Contents/MacOS/bar");
        assert_eq!(result, Some("/Users/me/Build/Bar.app".to_string()));
    }

    #[test]
    fn detect_app_bundle_root_xcode_developer_tool_layout() {
        let result = detect_app_bundle_root(
            "/Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild",
        );
        assert_eq!(result, Some("/Applications/Xcode.app".to_string()));
    }

    #[test]
    fn detect_app_bundle_root_flat_binary() {
        assert_eq!(detect_app_bundle_root("/usr/local/bin/tool"), None);
    }

    #[test]
    fn detect_app_bundle_root_nonstandard_contents_subdir() {
        assert_eq!(
            detect_app_bundle_root("/Foo.app/Contents/bin/foo"),
            Some("/Foo.app".to_string())
        );
    }

    #[test]
    fn detect_app_bundle_root_nonstandard_app_subdir() {
        assert_eq!(
            detect_app_bundle_root("/Foo.app/MacOS/foo"),
            Some("/Foo.app".to_string())
        );
    }

    #[test]
    fn detect_app_bundle_root_not_dot_app_extension() {
        assert_eq!(
            detect_app_bundle_root("/Foo.framework/Contents/MacOS/foo"),
            None
        );
    }

    #[test]
    fn detect_app_bundle_root_root_level_app() {
        let result = detect_app_bundle_root("/Test.app/Contents/MacOS/Test");
        assert_eq!(result, Some("/Test.app".to_string()));
    }

    // --- shim_nsbundle_bundle_path ---

    #[test]
    fn bundle_path_returns_stored_path() {
        let (obj, path_str) = make_bundle_with_path("/some/bundle/path");
        let result = shim_nsbundle_bundle_path(obj, 0);
        assert_eq!(result, path_str);
        assert_string_eq(result, "/some/bundle/path");
        free_obj(obj);
    }

    #[test]
    fn bundle_path_nil_returns_zero() {
        assert_eq!(shim_nsbundle_bundle_path(0, 0), 0);
    }

    #[test]
    fn bundle_url_returns_file_url() {
        let (obj, _) = make_bundle_with_path("/Applications/Test.app");
        let result = shim_nsbundle_bundle_url(obj, 0);
        assert_ne!(result, 0);
        assert_string_eq(crate::nsurl::shim_nsurl_absolute_string(result, 0), "file:///Applications/Test.app");
        free_obj(obj);
    }

    // --- shim_nsbundle_bundle_identifier ---

    #[test]
    fn bundle_identifier_always_returns_nil() {
        let obj = alloc_obj_fake(NSBUNDLE_SIZE);
        assert_eq!(shim_nsbundle_bundle_identifier(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn bundle_identifier_nil_returns_zero() {
        assert_eq!(shim_nsbundle_bundle_identifier(0, 0), 0);
    }

    // --- shim_nsbundle_bundle_with_path ---

    #[test]
    fn bundle_with_path_creates_valid_object() {
        let path_str = make_string("/test/bundle");
        let obj = shim_nsbundle_bundle_with_path(0, 0, path_str);
        assert_ne!(obj, 0);
        // ISA must be NSBundle class
        let isa = read_field(obj, 0);
        assert_eq!(isa, crate::nsobj::nsbundle_class_addr());
        // Path field must store the provided path
        let stored = read_field(obj, NSBUNDLE_PATH_OFF);
        assert_eq!(stored, path_str);
        assert_string_eq(stored, "/test/bundle");
        free_obj(obj);
    }

    #[test]
    fn bundle_with_path_nil_path_returns_zero() {
        assert_eq!(shim_nsbundle_bundle_with_path(0, 0, 0), 0);
    }

    // --- shim_nsbundle_load_nib_named ---

    #[test]
    fn load_nib_named_stub_returns_no() {
        let obj = alloc_obj_fake(NSBUNDLE_SIZE);
        let name = make_string("MainMenu");
        assert_eq!(shim_nsbundle_load_nib_named(obj, 0, name, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn load_nib_named_nil_returns_no() {
        assert_eq!(shim_nsbundle_load_nib_named(0, 0, 0, 0, 0), 0);
    }

    // --- shim_nsbundle_resource_path ---

    #[test]
    fn resource_path_nil_returns_zero() {
        assert_eq!(shim_nsbundle_resource_path(0, 0), 0);
    }

    #[test]
    fn resource_path_nil_path_field_returns_zero() {
        // Object with zeroed path field
        let obj = alloc_obj(NSBUNDLE_SIZE, crate::nsobj::nsbundle_class_addr());
        assert_eq!(shim_nsbundle_resource_path(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn resource_path_no_contents_resources_returns_bundle_path() {
        // When Contents/Resources doesn't exist, falls back to bundle path
        let (obj, path_str) = make_bundle_with_path("/nonexistent/bundle/dir");
        let result = shim_nsbundle_resource_path(obj, 0);
        assert_eq!(result, path_str);
        assert_string_eq(result, "/nonexistent/bundle/dir");
        free_obj(obj);
    }

    // --- shim_nsbundle_executable_path ---

    #[test]
    fn executable_path_returns_nonempty_nsstring() {
        // Calls libsystem_shims::process::process_path() which falls back
        // to "/usr/local/bin/unknown" if unavailable.
        let result = shim_nsbundle_executable_path(0, 0);
        assert_ne!(result, 0);
        let s = read_string(result);
        assert!(!s.is_empty());
    }

    #[test]
    fn executable_url_returns_file_url() {
        let result = shim_nsbundle_executable_url(1, 0);
        assert_ne!(result, 0);
        assert_eq!(crate::nsurl::shim_nsurl_is_file_url(result, 0), 1);
    }

    // --- shim_nsbundle_info_dictionary ---

    #[test]
    fn info_dictionary_nil_returns_zero() {
        assert_eq!(shim_nsbundle_info_dictionary(0, 0), 0);
    }

    #[test]
    fn info_dictionary_no_plist_on_disk_returns_zero() {
        let (obj, _) = make_bundle_with_path("/nonexistent/no/plist/here");
        assert_eq!(shim_nsbundle_info_dictionary(obj, 0), 0);
        free_obj(obj);
    }

    // --- shim_nsbundle_object_for_info_key ---

    #[test]
    fn object_for_info_key_nil_obj_returns_zero() {
        let key = make_string("CFBundleName");
        assert_eq!(shim_nsbundle_object_for_info_key(0, 0, key), 0);
    }

    #[test]
    fn object_for_info_key_nil_key_returns_zero() {
        let obj = alloc_obj_fake(NSBUNDLE_SIZE);
        assert_eq!(shim_nsbundle_object_for_info_key(obj, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn object_for_info_key_both_nil_returns_zero() {
        assert_eq!(shim_nsbundle_object_for_info_key(0, 0, 0), 0);
    }

    #[test]
    fn object_for_info_key_no_plist_returns_zero() {
        let (obj, _) = make_bundle_with_path("/nonexistent/path");
        let key = make_string("CFBundleIdentifier");
        assert_eq!(shim_nsbundle_object_for_info_key(obj, 0, key), 0);
        free_obj(obj);
    }

    // --- shim_nsbundle_path_for_resource ---

    #[test]
    fn path_for_resource_nil_obj_returns_zero() {
        let name = make_string("icon");
        assert_eq!(shim_nsbundle_path_for_resource(0, 0, name, 0), 0);
    }

    #[test]
    fn path_for_resource_nil_name_returns_zero() {
        let obj = alloc_obj_fake(NSBUNDLE_SIZE);
        assert_eq!(shim_nsbundle_path_for_resource(obj, 0, 0, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn path_for_resource_nonexistent_file_returns_zero() {
        let (obj, _) = make_bundle_with_path("/nonexistent/bundle");
        let name = make_string("missing");
        let ext = make_string("txt");
        assert_eq!(shim_nsbundle_path_for_resource(obj, 0, name, ext), 0);
        free_obj(obj);
    }

    #[test]
    fn path_for_resource_nil_type_nonexistent_returns_zero() {
        let (obj, _) = make_bundle_with_path("/nonexistent/bundle");
        let name = make_string("nofile");
        assert_eq!(shim_nsbundle_path_for_resource(obj, 0, name, 0), 0);
        free_obj(obj);
    }

    // --- shim_nsbundle_main_bundle ---

    #[test]
    fn main_bundle_returns_valid_object() {
        let result = shim_nsbundle_main_bundle(0, 0);
        assert_ne!(result, 0);
        let isa = read_field(result, 0);
        assert_eq!(isa, crate::nsobj::nsbundle_class_addr());
        // Do NOT free — it's a singleton managed by OnceLock
    }

    #[test]
    fn main_bundle_is_singleton() {
        let a = shim_nsbundle_main_bundle(0, 0);
        let b = shim_nsbundle_main_bundle(0, 0);
        assert_eq!(a, b, "mainBundle must return the same pointer every time");
    }

    #[test]
    fn main_bundle_has_nonempty_path() {
        let bundle = shim_nsbundle_main_bundle(0, 0);
        let path = shim_nsbundle_bundle_path(bundle, 0);
        assert_ne!(path, 0);
        let s = read_string(path);
        assert!(!s.is_empty());
    }

    #[test]
    fn all_frameworks_returns_array() {
        let result = shim_nsbundle_all_frameworks(0, 0);
        assert_ne!(result, 0);
    }

    // ── New selector tests ─────────────────────────────────────────

    #[test]
    fn bundle_for_class_returns_main_bundle() {
        let main = shim_nsbundle_main_bundle(0, 0);
        let result = shim_nsbundle_bundle_for_class(0, 0, 0xDEAD);
        assert_eq!(result, main);
    }

    #[test]
    fn url_for_resource_returns_nil() {
        let (obj, _) = make_bundle_with_path("/some/path");
        let name = make_string("icon");
        let ext = make_string("png");
        assert_eq!(shim_nsbundle_url_for_resource(obj, 0, name, ext), 0);
        free_obj(obj);
    }

    #[test]
    fn localized_string_returns_key_when_no_value() {
        let (obj, _) = make_bundle_with_path("/some/path");
        let key = make_string("HELLO_KEY");
        let result = shim_nsbundle_localized_string(obj, 0, key, 0, 0);
        assert_eq!(result, key);
        assert_string_eq(result, "HELLO_KEY");
        free_obj(obj);
    }

    #[test]
    fn localized_string_returns_value_when_provided() {
        let (obj, _) = make_bundle_with_path("/some/path");
        let key = make_string("KEY");
        let value = make_string("Fallback Value");
        let result = shim_nsbundle_localized_string(obj, 0, key, value, 0);
        assert_eq!(result, value);
        assert_string_eq(result, "Fallback Value");
        free_obj(obj);
    }

    #[test]
    fn localized_string_empty_value_returns_key() {
        let (obj, _) = make_bundle_with_path("/some/path");
        let key = make_string("KEY");
        let value = make_string("");
        let result = shim_nsbundle_localized_string(obj, 0, key, value, 0);
        assert_eq!(result, key);
        free_obj(obj);
    }

    #[test]
    fn localized_string_nil_key_returns_zero() {
        assert_eq!(shim_nsbundle_localized_string(0, 0, 0, 0, 0), 0);
    }
}
