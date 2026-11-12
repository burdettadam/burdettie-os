//! NSWorkspace shim: workspace singleton, opener, and app discovery helpers.
//!
//! URL/file/application opening uses the open-source `open` crate so the shim
//! stays Rust-native and cross-platform. Application lookup is intentionally
//! conservative: it scans an optional `BURDETTIE_NSWORKSPACE_APP_PATHS`
//! overlay plus common application roots and returns nil when no matching local
//! bundle or executable can be proven.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static SHARED_WORKSPACE: OnceLock<u64> = OnceLock::new();

fn get_or_create_shared_workspace() -> u64 {
    *SHARED_WORKSPACE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj == 0 {
            return 0;
        }
        unsafe {
            *(obj as *mut u64) = crate::nsobj::nsworkspace_class_addr();
        }
        obj
    })
}

/// `+[NSWorkspace sharedWorkspace]`.
pub extern "C" fn shim_nsworkspace_shared_workspace(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_shared_workspace()
}

/// `-[NSWorkspace openURL:]`.
pub extern "C" fn shim_nsworkspace_open_url(_obj: u64, _cmd: u64, url_obj: u64) -> i32 {
    let Some(url_str) = extract_url_string(url_obj) else {
        return 0;
    };
    platform_open(&url_str)
}

/// `-[NSWorkspace openFile:]`.
pub extern "C" fn shim_nsworkspace_open_file(_obj: u64, _cmd: u64, path_obj: u64) -> i32 {
    let Some(path_str) = nsstring_to_string(path_obj) else {
        return 0;
    };
    platform_open(&path_str)
}

/// `-[NSWorkspace openFile:withApplication:]`.
pub extern "C" fn shim_nsworkspace_open_file_with_app(
    _obj: u64,
    _cmd: u64,
    path_obj: u64,
    app_obj: u64,
) -> i32 {
    let Some(path_str) = nsstring_to_string(path_obj) else {
        return 0;
    };
    let app = nsstring_to_string(app_obj);
    platform_open_with_app(&path_str, app.as_deref())
}

/// `-[NSWorkspace openFile:withApplication:andDeactivate:]`.
pub extern "C" fn shim_nsworkspace_open_file_with_app_and_deactivate(
    obj: u64,
    cmd: u64,
    path_obj: u64,
    app_obj: u64,
    _deactivate: u8,
) -> i32 {
    shim_nsworkspace_open_file_with_app(obj, cmd, path_obj, app_obj)
}

/// `-[NSWorkspace openURLs:withApplicationAtURL:options:configuration:error:]`.
pub extern "C" fn shim_nsworkspace_open_urls_with_application_at_url_options_configuration_error(
    _obj: u64,
    _cmd: u64,
    urls: u64,
    app_url: u64,
    _options: u64,
    _configuration: u64,
    error: *mut u64,
) -> u8 {
    clear_error(error);
    let targets = nsurl_array_to_targets(urls);
    if targets.is_empty() {
        return 0;
    }
    let app = nsurl_to_path_or_string(app_url);
    let mut opened_any = false;
    for target in targets {
        opened_any |= platform_open_with_app(&target, app.as_deref()) != 0;
    }
    u8::from(opened_any)
}

/// `-[NSWorkspace launchApplication:]`.
pub extern "C" fn shim_nsworkspace_launch_application(_obj: u64, _cmd: u64, app_name: u64) -> i32 {
    let Some(app_name) = nsstring_to_string(app_name) else {
        return 0;
    };
    let target = resolve_application_path(&app_name)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or(app_name);
    platform_open(&target)
}

/// `-[NSWorkspace launchApplicationAtURL:options:configuration:error:]`.
pub extern "C" fn shim_nsworkspace_launch_application_at_url_options_configuration_error(
    _obj: u64,
    _cmd: u64,
    app_url: u64,
    _options: u64,
    _configuration: u64,
    error: *mut u64,
) -> u8 {
    clear_error(error);
    let Some(target) = nsurl_to_path_or_string(app_url) else {
        return 0;
    };
    u8::from(platform_open(&target) != 0)
}

/// `-[NSWorkspace runningApplications]`.
pub extern "C" fn shim_nsworkspace_running_applications(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `-[NSWorkspace frontmostApplication]`.
pub extern "C" fn shim_nsworkspace_frontmost_application(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `-[NSWorkspace iconForFile:]`.
pub extern "C" fn shim_nsworkspace_icon_for_file(_obj: u64, _cmd: u64, _path: u64) -> u64 {
    0
}

/// `-[NSWorkspace fullPathForApplication:]`.
pub extern "C" fn shim_nsworkspace_full_path_for_application(
    _obj: u64,
    _cmd: u64,
    app_name: u64,
) -> u64 {
    let Some(app_name) = nsstring_to_string(app_name) else {
        return 0;
    };
    let Some(path) = resolve_application_path(&app_name) else {
        return 0;
    };
    make_nsstring(&path.to_string_lossy())
}

/// `-[NSWorkspace URLForApplicationWithBundleIdentifier:]`.
pub extern "C" fn shim_nsworkspace_url_for_application_with_bundle_identifier(
    _obj: u64,
    _cmd: u64,
    bundle_id: u64,
) -> u64 {
    let Some(bundle_id) = nsstring_to_string(bundle_id) else {
        return 0;
    };
    let Some(path) = find_application_by_bundle_identifier(&bundle_id) else {
        return 0;
    };
    file_url_for_path(&path)
}

/// `-[NSWorkspace URLsForApplicationsWithBundleIdentifier:]`.
pub extern "C" fn shim_nsworkspace_urls_for_applications_with_bundle_identifier(
    obj: u64,
    cmd: u64,
    bundle_id: u64,
) -> u64 {
    let url = shim_nsworkspace_url_for_application_with_bundle_identifier(obj, cmd, bundle_id);
    if url == 0 {
        crate::nsarray::make_nsarray(&[])
    } else {
        crate::nsarray::make_nsarray(&[url])
    }
}

/// `-[NSWorkspace URLForApplicationToOpenURL:]`.
pub extern "C" fn shim_nsworkspace_url_for_application_to_open_url(
    _obj: u64,
    _cmd: u64,
    _url: u64,
) -> u64 {
    0
}

/// `-[NSWorkspace applicationsForOpeningURL:]`.
pub extern "C" fn shim_nsworkspace_applications_for_opening_url(
    _obj: u64,
    _cmd: u64,
    _url: u64,
) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `-[NSWorkspace selectFile:inFileViewerRootedAtPath:]`.
pub extern "C" fn shim_nsworkspace_select_file_in_file_viewer_rooted_at_path(
    _obj: u64,
    _cmd: u64,
    full_path: u64,
    root_path: u64,
) -> u8 {
    let target = nsstring_to_string(full_path)
        .filter(|s| !s.is_empty())
        .or_else(|| nsstring_to_string(root_path).filter(|s| !s.is_empty()));
    let Some(target) = target else {
        return 0;
    };
    u8::from(platform_open(&target) != 0)
}

/// `-[NSWorkspace activateFileViewerSelectingURLs:]`.
pub extern "C" fn shim_nsworkspace_activate_file_viewer_selecting_urls(
    _obj: u64,
    _cmd: u64,
    urls: u64,
) {
    if let Some(target) = nsurl_array_to_targets(urls).into_iter().next() {
        let _ = platform_open(&target);
    }
}

/// `-[NSWorkspace noteFileSystemChanged:]`.
pub extern "C" fn shim_nsworkspace_note_file_system_changed(_obj: u64, _cmd: u64, _path: u64) {}

/// `-[NSWorkspace mountedLocalVolumePaths]`.
pub extern "C" fn shim_nsworkspace_mounted_local_volume_paths(_obj: u64, _cmd: u64) -> u64 {
    let roots = mounted_roots();
    let ns_roots: Vec<u64> = roots.iter().map(|root| make_nsstring(root)).collect();
    crate::nsarray::make_nsarray(&ns_roots)
}

/// `-[NSWorkspace typeOfFile:error:]`.
pub extern "C" fn shim_nsworkspace_type_of_file_error(
    _obj: u64,
    _cmd: u64,
    path: u64,
    error: *mut u64,
) -> u64 {
    clear_error(error);
    let Some(path) = nsstring_to_string(path) else {
        return 0;
    };
    make_nsstring(uti_for_path(&path))
}

/// `-[NSWorkspace localizedDescriptionForType:]`.
pub extern "C" fn shim_nsworkspace_localized_description_for_type(
    _obj: u64,
    _cmd: u64,
    type_name: u64,
) -> u64 {
    let Some(type_name) = nsstring_to_string(type_name) else {
        return 0;
    };
    make_nsstring(description_for_uti(&type_name))
}

fn platform_open(target: &str) -> i32 {
    platform_open_with_app(target, None)
}

fn platform_open_with_app(target: &str, app: Option<&str>) -> i32 {
    let target = target.trim();
    if target.is_empty() {
        return 0;
    }

    let result = match app.map(str::trim).filter(|app| !app.is_empty()) {
        Some(app) => open::with_detached(target, app),
        None => open::that_detached(target),
    };

    match result {
        Ok(_) => {
            log::debug!("NSWorkspace: opened {target:?}");
            1
        }
        Err(e) => {
            log::debug!("NSWorkspace: failed to open {target:?}: {e}");
            0
        }
    }
}

fn clear_error(error: *mut u64) {
    if !error.is_null() {
        unsafe {
            *error = 0;
        }
    }
}

fn make_nsstring(value: &str) -> u64 {
    let cstr = std::ffi::CString::new(value).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn nsstring_to_string(obj: u64) -> Option<String> {
    crate::cfstring::extract_cfstring_cstr(obj).map(str::to_string)
}

fn extract_url_string(url_obj: u64) -> Option<String> {
    if url_obj == 0 {
        return None;
    }
    let absolute = crate::nsurl::shim_nsurl_absolute_string(url_obj, 0);
    nsstring_to_string(absolute)
}

fn nsurl_to_path_or_string(url_obj: u64) -> Option<String> {
    if url_obj == 0 {
        return None;
    }
    let path = crate::nsurl::shim_nsurl_path(url_obj, 0);
    nsstring_to_string(path)
        .filter(|path| !path.is_empty())
        .or_else(|| extract_url_string(url_obj))
}

fn nsurl_array_to_targets(urls: u64) -> Vec<String> {
    let count = crate::nsarray::shim_nsarray_count(urls, 0);
    let mut targets = Vec::new();
    for index in 0..count {
        let url = crate::nsarray::shim_nsarray_object_at_index(urls, 0, index);
        if let Some(target) = nsurl_to_path_or_string(url).filter(|target| !target.is_empty()) {
            targets.push(target);
        }
    }
    targets
}

fn file_url_for_path(path: &Path) -> u64 {
    let path_str = path.to_string_lossy();
    let ns_path = make_nsstring(&path_str);
    crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, ns_path)
}

fn workspace_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(paths) = std::env::var_os("BURDETTIE_NSWORKSPACE_APP_PATHS") {
        roots.extend(std::env::split_paths(&paths));
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("Applications"));
        }
        roots.push(PathBuf::from("/Applications"));
        roots.push(PathBuf::from("/System/Applications"));
        roots.push(PathBuf::from("/System/Library/CoreServices"));
    }

    if let Some(path) = std::env::var_os("PATH") {
        roots.extend(std::env::split_paths(&path));
    }
    roots
}

fn resolve_application_path(app_name: &str) -> Option<PathBuf> {
    resolve_application_path_in_roots(app_name, workspace_application_roots())
}

fn resolve_application_path_in_roots<I>(app_name: &str, roots: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    let app_name = app_name.trim();
    if app_name.is_empty() {
        return None;
    }

    let direct = PathBuf::from(app_name);
    if direct.exists() {
        return Some(normalize_existing_path(direct));
    }

    let app_bundle_name = if app_name.ends_with(".app") {
        app_name.to_string()
    } else {
        format!("{app_name}.app")
    };

    #[cfg(windows)]
    let executable_name = if app_name.ends_with(".exe") {
        app_name.to_string()
    } else {
        format!("{app_name}.exe")
    };

    for root in roots {
        let candidates = vec![root.join(app_name), root.join(&app_bundle_name)];
        #[cfg(windows)]
        let candidates = {
            let mut candidates = candidates;
            candidates.push(root.join(&executable_name));
            candidates
        };

        for candidate in candidates {
            if candidate.exists() {
                return Some(normalize_existing_path(candidate));
            }
        }
    }
    None
}

fn normalize_existing_path(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

fn find_application_by_bundle_identifier(bundle_id: &str) -> Option<PathBuf> {
    find_application_by_bundle_identifier_in_roots(bundle_id, workspace_application_roots())
}

fn find_application_by_bundle_identifier_in_roots<I>(bundle_id: &str, roots: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    let bundle_id = bundle_id.trim();
    if bundle_id.is_empty() {
        return None;
    }

    for root in roots {
        for candidate in application_candidates(&root) {
            if bundle_identifier_for_app(&candidate).as_deref() == Some(bundle_id) {
                return Some(normalize_existing_path(candidate));
            }
        }
    }
    None
}

fn application_candidates(root: &Path) -> Vec<PathBuf> {
    if root.extension().and_then(|ext| ext.to_str()) == Some("app") {
        return vec![root.to_path_buf()];
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("app"))
        .collect()
}

fn bundle_identifier_for_app(app_path: &Path) -> Option<String> {
    let candidates = [
        app_path.join("Contents").join("Info.plist"),
        app_path.join("Info.plist"),
    ];
    for plist_path in candidates {
        if !plist_path.exists() {
            continue;
        }
        let Ok(value) = plist::Value::from_file(&plist_path) else {
            continue;
        };
        let Some(dict) = value.as_dictionary() else {
            continue;
        };
        if let Some(plist::Value::String(bundle_id)) = dict.get("CFBundleIdentifier") {
            return Some(bundle_id.clone());
        }
    }
    None
}

fn mounted_roots() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        vec!["C:\\"]
    }
    #[cfg(not(windows))]
    {
        vec!["/"]
    }
}

fn uti_for_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt" | "text" | "md" | "markdown") => "public.plain-text",
        Some("rtf") => "public.rtf",
        Some("html" | "htm") => "public.html",
        Some("png") => "public.png",
        Some("jpg" | "jpeg") => "public.jpeg",
        Some("tif" | "tiff") => "public.tiff",
        Some("pdf") => "com.adobe.pdf",
        Some("app") => "com.apple.application-bundle",
        Some("plist") => "com.apple.property-list",
        Some("json") => "public.json",
        _ => "public.data",
    }
}

fn description_for_uti(type_name: &str) -> &'static str {
    match type_name {
        "public.plain-text" => "plain text",
        "public.rtf" => "rich text",
        "public.html" => "HTML document",
        "public.png" => "PNG image",
        "public.jpeg" => "JPEG image",
        "public.tiff" => "TIFF image",
        "com.adobe.pdf" => "PDF document",
        "com.apple.application-bundle" => "application",
        "com.apple.property-list" => "property list",
        "public.json" => "JSON document",
        "public.data" => "data",
        _ => "document",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn test_shared_workspace_returns_nonzero() {
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        assert_ne!(ws, 0, "sharedWorkspace must return a non-nil singleton");
    }

    #[test]
    fn test_shared_workspace_is_singleton() {
        let ws1 = shim_nsworkspace_shared_workspace(0, 0);
        let ws2 = shim_nsworkspace_shared_workspace(0, 0);
        assert_eq!(
            ws1, ws2,
            "sharedWorkspace must return the same pointer each time"
        );
    }

    #[test]
    fn test_shared_workspace_has_correct_isa() {
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        let isa = read_field(ws, 0);
        assert_eq!(
            isa,
            crate::nsobj::nsworkspace_class_addr(),
            "singleton ISA must be nsworkspace_class_addr()"
        );
    }

    #[test]
    fn test_shared_workspace_nil_cls_and_cmd() {
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        assert_ne!(ws, 0);
    }

    #[test]
    fn test_shared_workspace_nonzero_cls() {
        let ws = shim_nsworkspace_shared_workspace(0xCAFE, 0xBEEF);
        let ws2 = shim_nsworkspace_shared_workspace(0, 0);
        assert_eq!(
            ws, ws2,
            "sharedWorkspace ignores _cls and always returns the singleton"
        );
    }

    #[test]
    fn open_rejects_empty_targets_without_spawning() {
        assert_eq!(platform_open(""), 0);
        assert_eq!(platform_open_with_app("   ", Some("Finder")), 0);
    }

    #[test]
    fn url_targets_use_nsurl_helpers() {
        let url_str = make_string("https://example.invalid/doc");
        let url =
            crate::nsurl::shim_nsurl_url_with_string(crate::nsobj::nsurl_class_addr(), 0, url_str);
        assert_eq!(
            extract_url_string(url),
            Some(String::from("https://example.invalid/doc"))
        );
    }

    #[test]
    fn file_url_target_prefers_path_for_bulk_open() {
        let path = make_string("/tmp/workspace-target.txt");
        let url =
            crate::nsurl::shim_nsurl_file_url_with_path(crate::nsobj::nsurl_class_addr(), 0, path);
        let urls = crate::nsarray::make_nsarray(&[url]);
        assert_eq!(
            nsurl_array_to_targets(urls),
            vec![String::from("/tmp/workspace-target.txt")]
        );
    }

    #[test]
    fn resolves_application_bundle_from_roots() {
        let root =
            std::env::temp_dir().join(format!("cf-lite-workspace-app-root-{}", std::process::id()));
        let app = root.join("Sample.app");
        std::fs::create_dir_all(app.join("Contents")).expect("create sample app");

        let resolved = resolve_application_path_in_roots("Sample", vec![root.clone()])
            .expect("sample app should resolve");
        assert_eq!(resolved, normalize_existing_path(app.clone()));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn finds_application_by_bundle_identifier() {
        let root = std::env::temp_dir().join(format!(
            "cf-lite-workspace-bundle-root-{}",
            std::process::id()
        ));
        let app = root.join("Bundle.app");
        let contents = app.join("Contents");
        std::fs::create_dir_all(&contents).expect("create bundle contents");
        std::fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
 "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.Bundle</string>
</dict>
</plist>"#,
        )
        .expect("write Info.plist");

        let found = find_application_by_bundle_identifier_in_roots(
            "com.example.Bundle",
            vec![root.clone()],
        )
        .expect("bundle id should resolve");
        assert_eq!(found, normalize_existing_path(app));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn file_type_and_description_are_stable() {
        assert_eq!(uti_for_path("/tmp/readme.txt"), "public.plain-text");
        assert_eq!(
            uti_for_path("/Applications/Test.app"),
            "com.apple.application-bundle"
        );
        assert_eq!(description_for_uti("public.png"), "PNG image");
        assert_eq!(description_for_uti("unknown.type"), "document");
    }

    #[test]
    fn nil_and_empty_workspace_queries_are_safe() {
        assert_eq!(shim_nsworkspace_full_path_for_application(0, 0, 0), 0);
        assert_eq!(
            shim_nsworkspace_url_for_application_with_bundle_identifier(0, 0, 0),
            0
        );
        assert_eq!(
            crate::nsarray::shim_nsarray_count(
                shim_nsworkspace_applications_for_opening_url(0, 0, 0),
                0
            ),
            0
        );
        assert_eq!(
            shim_nsworkspace_select_file_in_file_viewer_rooted_at_path(0, 0, 0, 0),
            0
        );
    }
}
