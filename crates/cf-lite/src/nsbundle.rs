//! NSBundle shim: application bundle access.
//!
//! Object layout (heap-allocated, singleton for mainBundle):
//!
//!   offset  0: isa         (u64 — pointer to NSBundle class)
//!   offset  8: bundle_path (u64 — pointer to NSString with bundle path)
//!
//! Total: 16 bytes per NSBundle object.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const NSBUNDLE_PATH_OFF: usize = 8;

static MAIN_BUNDLE: OnceLock<u64> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static TEST_MAIN_BUNDLE_OVERRIDE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn test_main_bundle_override() -> u64 {
    TEST_MAIN_BUNDLE_OVERRIDE.with(std::cell::Cell::get)
}

#[cfg(not(test))]
fn test_main_bundle_override() -> u64 {
    0
}

pub fn get_or_create_main_bundle() -> u64 {
    let override_bundle = test_main_bundle_override();
    if override_bundle != 0 {
        return override_bundle;
    }

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
        let bundle_path = detect_app_bundle_root(&exe_path).unwrap_or_else(|| {
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

#[cfg(test)]
pub(crate) fn set_main_bundle_path_for_test(path: &Path) -> u64 {
    let bundle_path = make_nsstring_from_path(path);
    let bundle =
        shim_nsbundle_bundle_with_path(crate::nsobj::nsbundle_class_addr(), 0, bundle_path);
    TEST_MAIN_BUNDLE_OVERRIDE.with(|slot| slot.set(bundle));
    bundle
}

#[cfg(test)]
pub(crate) fn clear_main_bundle_path_for_test() {
    TEST_MAIN_BUNDLE_OVERRIDE.with(|slot| slot.set(0));
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

fn nsstring_path_to_file_url_with_directory_hint(path_ptr: u64, is_directory: bool) -> u64 {
    if path_ptr == 0 {
        return 0;
    }

    crate::nsurl::shim_nsurl_file_url_with_path_is_directory(
        crate::nsobj::nsurl_class_addr(),
        0,
        path_ptr,
        u8::from(is_directory),
    )
}

fn nsstring_to_string(ptr: u64) -> Option<String> {
    crate::cfstring::extract_cfstring_cstr(ptr).map(str::to_string)
}

fn make_nsstring_from_string(value: &str) -> u64 {
    let cstr = std::ffi::CString::new(value).unwrap_or_default();
    crate::cfstring::alloc_nsstring_from_cstr(cstr.as_ptr())
}

fn make_nsstring_from_path(path: &Path) -> u64 {
    make_nsstring_from_string(path.to_string_lossy().as_ref())
}

fn file_url_from_path(path: &Path, is_directory: bool) -> u64 {
    let ns_path = make_nsstring_from_path(path);
    nsstring_path_to_file_url_with_directory_hint(ns_path, is_directory)
}

fn bundle_path_string(obj: u64) -> Option<String> {
    if obj == 0 {
        return None;
    }
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    nsstring_to_string(path_ptr)
}

fn resource_root_for_bundle_path(bundle_path: &str) -> PathBuf {
    let bundle = Path::new(bundle_path);
    let contents_resources = bundle.join("Contents").join("Resources");
    if contents_resources.exists() {
        contents_resources
    } else {
        bundle.to_path_buf()
    }
}

pub(crate) fn bundle_resource_root(obj: u64) -> Option<PathBuf> {
    bundle_path_string(obj).map(|path| resource_root_for_bundle_path(&path))
}

fn nsstring_argument(ptr: u64) -> Option<String> {
    if ptr == 0 {
        None
    } else {
        nsstring_to_string(ptr)
    }
}

fn clean_resource_subpath(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim_matches(|ch| ch == '/' || ch == '\\');
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn resource_filename(name: &str, ext: Option<&str>) -> Option<String> {
    if name.is_empty() {
        return None;
    }

    let ext = ext.unwrap_or("").trim_start_matches('.');
    if ext.is_empty() || name.ends_with(&format!(".{ext}")) {
        Some(name.to_string())
    } else {
        Some(format!("{name}.{ext}"))
    }
}

fn localization_dir_name(localization: &str) -> Option<String> {
    let trimmed = localization.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with(".lproj") {
        Some(trimmed.to_string())
    } else {
        Some(format!("{trimmed}.lproj"))
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|path| path == &candidate) {
        paths.push(candidate);
    }
}

fn append_optional_subdir(mut base: PathBuf, subdir: Option<&str>) -> PathBuf {
    if let Some(subdir) = subdir.and_then(clean_resource_subpath) {
        base.push(subdir);
    }
    base
}

fn resource_search_dirs(
    root: &Path,
    subdir: Option<&str>,
    localization: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(localization) = localization.and_then(localization_dir_name) {
        push_unique_path(
            &mut dirs,
            append_optional_subdir(root.join(localization), subdir),
        );
    }

    push_unique_path(
        &mut dirs,
        append_optional_subdir(root.to_path_buf(), subdir),
    );

    for localization in ["Base.lproj", "en.lproj"] {
        push_unique_path(
            &mut dirs,
            append_optional_subdir(root.join(localization), subdir),
        );
    }

    if let Ok(entries) = std::fs::read_dir(root) {
        let mut lproj_dirs = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with(".lproj"))
            })
            .collect::<Vec<_>>();
        lproj_dirs.sort();
        for dir in lproj_dirs {
            push_unique_path(&mut dirs, append_optional_subdir(dir, subdir));
        }
    }

    dirs
}

fn resolve_bundle_resource_path(
    obj: u64,
    name: u64,
    ext: u64,
    subdir: u64,
    localization: u64,
) -> Option<PathBuf> {
    let name = nsstring_to_string(name)?;
    let ext = nsstring_argument(ext);
    let subdir = nsstring_argument(subdir);
    let localization = nsstring_argument(localization);
    resolve_bundle_resource_path_parts(
        obj,
        &name,
        ext.as_deref(),
        subdir.as_deref(),
        localization.as_deref(),
    )
}

pub(crate) fn resolve_bundle_resource_path_parts(
    obj: u64,
    name: &str,
    ext: Option<&str>,
    subdir: Option<&str>,
    localization: Option<&str>,
) -> Option<PathBuf> {
    let filename = resource_filename(name, ext)?;
    let root = bundle_resource_root(obj)?;

    for dir in resource_search_dirs(&root, subdir, localization) {
        let candidate = dir.join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn collect_bundle_resource_paths(
    obj: u64,
    ext: u64,
    subdir: u64,
    localization: u64,
) -> Vec<PathBuf> {
    let ext = nsstring_argument(ext).map(|s| s.trim_start_matches('.').to_string());
    let subdir = nsstring_argument(subdir);
    let localization = nsstring_argument(localization);
    let Some(root) = bundle_resource_root(obj) else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    for dir in resource_search_dirs(&root, subdir.as_deref(), localization.as_deref()) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut entries = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                if let Some(ext) = ext.as_deref() {
                    if ext.is_empty() {
                        return true;
                    }
                    path.extension().and_then(|value| value.to_str()) == Some(ext)
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            push_unique_path(&mut paths, path);
        }
    }

    paths
}

fn decode_utf16_bytes(bytes: &[u8], big_endian: bool) -> Option<String> {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    let mut chunks = bytes.chunks_exact(2);
    for chunk in chunks.by_ref() {
        units.push(if big_endian {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_le_bytes([chunk[0], chunk[1]])
        });
    }
    if !chunks.remainder().is_empty() {
        return None;
    }
    String::from_utf16(&units).ok()
}

fn decode_strings_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(&bytes[3..]).ok().map(ToOwned::to_owned);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16_bytes(&bytes[2..], false);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16_bytes(&bytes[2..], true);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Some(text.to_owned());
    }

    if bytes.len() >= 4 && bytes.len() % 2 == 0 {
        let even_zeroes = bytes.iter().step_by(2).filter(|byte| **byte == 0).count();
        let odd_zeroes = bytes
            .iter()
            .skip(1)
            .step_by(2)
            .filter(|byte| **byte == 0)
            .count();
        let pairs = bytes.len() / 2;
        if odd_zeroes > pairs / 3 {
            return decode_utf16_bytes(bytes, false);
        }
        if even_zeroes > pairs / 3 {
            return decode_utf16_bytes(bytes, true);
        }
    }

    None
}

fn skip_strings_ws_and_comments(chars: &[char], mut index: usize) -> usize {
    loop {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }

        if index + 1 >= chars.len() || chars[index] != '/' {
            return index;
        }

        match chars[index + 1] {
            '/' => {
                index += 2;
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            '*' => {
                index += 2;
                while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                    index += 1;
                }
                index = (index + 2).min(chars.len());
            }
            _ => return index,
        }
    }
}

fn parse_hex_escape(chars: &[char], index: &mut usize) -> Option<char> {
    if *index + 4 > chars.len() {
        return None;
    }
    let mut value = 0_u32;
    for _ in 0..4 {
        value = (value << 4) | chars[*index].to_digit(16)?;
        *index += 1;
    }
    char::from_u32(value)
}

fn parse_strings_quoted_token(chars: &[char], index: &mut usize) -> Option<String> {
    if chars.get(*index) != Some(&'"') {
        return None;
    }
    *index += 1;
    let mut value = String::new();
    while *index < chars.len() {
        let ch = chars[*index];
        *index += 1;
        match ch {
            '"' => return Some(value),
            '\\' if *index < chars.len() => {
                let escaped = chars[*index];
                *index += 1;
                match escaped {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    'U' | 'u' => {
                        if let Some(ch) = parse_hex_escape(chars, index) {
                            value.push(ch);
                        }
                    }
                    other => value.push(other),
                }
            }
            other => value.push(other),
        }
    }
    None
}

fn parse_strings_unquoted_token(chars: &[char], index: &mut usize) -> Option<String> {
    let start = *index;
    while *index < chars.len()
        && !chars[*index].is_whitespace()
        && chars[*index] != '='
        && chars[*index] != ';'
    {
        *index += 1;
    }
    if *index == start {
        None
    } else {
        Some(chars[start..*index].iter().collect::<String>())
    }
}

fn parse_strings_token(chars: &[char], index: &mut usize) -> Option<String> {
    if chars.get(*index) == Some(&'"') {
        parse_strings_quoted_token(chars, index)
    } else {
        parse_strings_unquoted_token(chars, index)
    }
}

fn parse_text_strings_table(text: &str) -> HashMap<String, String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut strings = HashMap::new();

    while index < chars.len() {
        index = skip_strings_ws_and_comments(&chars, index);
        let Some(key) = parse_strings_token(&chars, &mut index) else {
            break;
        };

        index = skip_strings_ws_and_comments(&chars, index);
        if chars.get(index) != Some(&'=') {
            while index < chars.len() && chars[index] != ';' && chars[index] != '\n' {
                index += 1;
            }
            if chars.get(index) == Some(&';') {
                index += 1;
            }
            continue;
        }
        index += 1;

        index = skip_strings_ws_and_comments(&chars, index);
        let Some(value) = parse_strings_token(&chars, &mut index) else {
            continue;
        };
        strings.insert(key, value);

        index = skip_strings_ws_and_comments(&chars, index);
        if chars.get(index) == Some(&';') {
            index += 1;
        }
    }

    strings
}

fn parse_plist_strings_table(bytes: &[u8]) -> Option<HashMap<String, String>> {
    let plist = plist::Value::from_reader(std::io::Cursor::new(bytes)).ok()?;
    let plist::Value::Dictionary(dict) = plist else {
        return None;
    };

    let strings = dict
        .into_iter()
        .filter_map(|(key, value)| value.as_string().map(|value| (key, value.to_owned())))
        .collect::<HashMap<_, _>>();
    Some(strings)
}

fn load_strings_table(path: &Path) -> Option<HashMap<String, String>> {
    let bytes = std::fs::read(path).ok()?;
    if let Some(strings) = parse_plist_strings_table(&bytes) {
        return Some(strings);
    }
    let text = decode_strings_bytes(&bytes)?;
    Some(parse_text_strings_table(&text))
}

fn strings_table_name(table: u64) -> String {
    nsstring_to_string(table)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Localizable".to_string())
}

fn localized_string_lookup(obj: u64, key: u64, table: u64) -> Option<String> {
    let key = nsstring_to_string(key)?;
    let table_name = strings_table_name(table);
    let table_name = table_name.strip_suffix(".strings").unwrap_or(&table_name);
    let table_path =
        resolve_bundle_resource_path_parts(obj, table_name, Some("strings"), None, None)?;
    let strings = load_strings_table(&table_path)?;
    strings.get(&key).cloned()
}

fn localized_string_fallback(key: u64, value: u64) -> u64 {
    if value != 0 {
        if let Some(s) = crate::cfstring::extract_cfstring_cstr(value) {
            if !s.is_empty() {
                return value;
            }
        }
    }
    key
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn lproj_localization_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    name.strip_suffix(".lproj").map(ToOwned::to_owned)
}

fn bundle_development_region_from_info(obj: u64) -> Option<String> {
    let key = make_nsstring_from_string("CFBundleDevelopmentRegion");
    nsstring_to_string(shim_nsbundle_object_for_info_key(obj, 0, key))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn bundle_localization_names(obj: u64) -> Vec<String> {
    let mut names = Vec::new();

    if let Some(region) = bundle_development_region_from_info(obj) {
        push_unique_string(&mut names, region);
    }

    let Some(root) = bundle_resource_root(obj) else {
        return names;
    };

    if let Ok(entries) = std::fs::read_dir(root) {
        let mut lproj_names = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter_map(|path| lproj_localization_name(&path))
            .collect::<Vec<_>>();
        lproj_names.sort();
        for name in lproj_names {
            push_unique_string(&mut names, name);
        }
    }

    names
}

fn nsarray_from_strings(values: &[String]) -> u64 {
    let strings = values
        .iter()
        .map(|value| make_nsstring_from_string(value))
        .collect::<Vec<_>>();
    crate::nsarray::make_nsarray(&strings)
}

fn bundle_development_localization(obj: u64) -> Option<String> {
    bundle_development_region_from_info(obj).or_else(|| {
        let names = bundle_localization_names(obj);
        names
            .iter()
            .find(|name| name.as_str() == "Base")
            .or_else(|| names.iter().find(|name| name.as_str() == "en"))
            .or_else(|| names.first())
            .cloned()
            .or_else(|| Some("en".to_string()))
    })
}

fn bundle_preferred_localization_names(obj: u64) -> Vec<String> {
    let available = bundle_localization_names(obj);
    if available.is_empty() {
        return Vec::new();
    }

    let mut preferred = Vec::new();
    for candidate in [
        bundle_development_localization(obj).as_deref(),
        Some("en"),
        Some("Base"),
    ]
    .into_iter()
    .flatten()
    {
        if available.iter().any(|name| name == candidate) {
            push_unique_string(&mut preferred, candidate.to_string());
        }
    }
    for name in available {
        push_unique_string(&mut preferred, name);
    }
    preferred
}

/// `-[NSBundle resourcePath]` — returns bundlePath + "/Contents/Resources"
/// (for a standard .app bundle), or just bundlePath for flat CLI bundles.
pub extern "C" fn shim_nsbundle_resource_path(obj: u64, _cmd: u64) -> u64 {
    let Some(bundle_path) = bundle_path_string(obj) else {
        return 0;
    };
    make_nsstring_from_path(&resource_root_for_bundle_path(&bundle_path))
}

/// `-[NSBundle resourceURL]` - returns the resource directory as a file URL.
pub extern "C" fn shim_nsbundle_resource_url(obj: u64, _cmd: u64) -> u64 {
    let Some(root) = bundle_resource_root(obj) else {
        return 0;
    };
    file_url_from_path(&root, true)
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
pub extern "C" fn shim_nsbundle_bundle_identifier(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    let key = make_nsstring_from_string("CFBundleIdentifier");
    shim_nsbundle_object_for_info_key(obj, 0, key)
}

/// `+[NSBundle bundleWithPath:]` — returns a bundle object for any path.
pub extern "C" fn shim_nsbundle_bundle_with_path(_cls: u64, _cmd: u64, path: u64) -> u64 {
    if path == 0 {
        return 0;
    }
    let obj = crate::pal_alloc().calloc(1, 16) as u64;
    if obj == 0 {
        return 0;
    }
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
            let bundle =
                shim_nsbundle_bundle_with_path(crate::nsobj::nsbundle_class_addr(), 0, ns_path);
            if bundle != 0 {
                bundles.push(bundle);
            }
        }
    }

    crate::nsarray::make_nsarray(&bundles)
}

/// Parse the Info.plist for a bundle object and return the NSDictionary (or 0).
fn load_info_dict(obj: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    let path_ptr = unsafe { *((obj as usize + NSBUNDLE_PATH_OFF) as *const u64) };
    if path_ptr == 0 {
        return 0;
    }
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
        if result != 0 {
            return result;
        }
    }
    // Flat layout: bundle/Info.plist
    let flat_plist = std::path::Path::new(&path_str).join("Info.plist");
    if let Some(p) = flat_plist.to_str() {
        let result = crate::nspropertylistserialization::parse_plist_file(p);
        if result != 0 {
            return result;
        }
    }
    0
}

/// `-[NSBundle infoDictionary]` — parse and return the bundle's Info.plist.
pub extern "C" fn shim_nsbundle_info_dictionary(obj: u64, _cmd: u64) -> u64 {
    load_info_dict(obj)
}

/// `-[NSBundle objectForInfoDictionaryKey:]` — look up a key in infoDictionary.
pub extern "C" fn shim_nsbundle_object_for_info_key(obj: u64, _cmd: u64, key: u64) -> u64 {
    if obj == 0 || key == 0 {
        return 0;
    }
    let dict = load_info_dict(obj);
    if dict == 0 {
        return 0;
    }
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

/// `-[NSBundle pathForResource:ofType:]` — find a resource file in the bundle.
pub extern "C" fn shim_nsbundle_path_for_resource(
    obj: u64,
    _cmd: u64,
    name: u64,
    r#type: u64,
) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, r#type, 0, 0) else {
        return 0;
    };
    make_nsstring_from_path(&path)
}

/// `-[NSBundle pathForResource:ofType:inDirectory:]`.
pub extern "C" fn shim_nsbundle_path_for_resource_in_directory(
    obj: u64,
    _cmd: u64,
    name: u64,
    r#type: u64,
    subdir: u64,
) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, r#type, subdir, 0) else {
        return 0;
    };
    make_nsstring_from_path(&path)
}

/// `-[NSBundle pathForResource:ofType:inDirectory:forLocalization:]`.
pub extern "C" fn shim_nsbundle_path_for_resource_in_directory_for_localization(
    obj: u64,
    _cmd: u64,
    name: u64,
    r#type: u64,
    subdir: u64,
    localization: u64,
) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, r#type, subdir, localization) else {
        return 0;
    };
    make_nsstring_from_path(&path)
}

pub(crate) fn resolve_nib_file_path(nib_path: &std::path::Path) -> Option<std::path::PathBuf> {
    if nib_path.is_file() {
        return Some(nib_path.to_path_buf());
    }

    for inner_name in ["keyedobjects.nib", "keyedobjects-101.nib"] {
        let inner = nib_path.join(inner_name);
        if inner.is_file() {
            return Some(inner);
        }
    }

    None
}

pub(crate) fn load_nib_file_at_path(nib_file: &std::path::Path, owner: u64, top_out: u64) -> u8 {
    log::debug!("NSBundle loadNibNamed: loading '{}'", nib_file.display());

    let Ok(bytes) = std::fs::read(nib_file) else {
        return 0;
    };
    if bytes.is_empty() {
        return 0;
    }

    let data = crate::nsdata::make_nsdata(&bytes);
    let result = crate::nskeyedarchiver::shim_nskeyedunarchiver_unarchived_object(0, 0, 0, data, 0);

    if result == 0 {
        log::debug!(
            "NSBundle loadNibNamed: unarchive failed for '{}'",
            nib_file.display()
        );
        return 0;
    }

    wire_nib_connections(owner);

    if top_out != 0 {
        let isa = unsafe { *(result as *const u64) };
        if isa == crate::nsobj::nsarray_class_addr()
            || isa == crate::nsobj::nsmutablearray_class_addr()
        {
            unsafe {
                *(top_out as *mut u64) = result;
            }
        } else {
            let arr = crate::nsarray::make_nsarray(&[result]);
            unsafe {
                *(top_out as *mut u64) = arr;
            }
        }
    }

    1
}

fn load_nib_named_from_bundle(obj: u64, name: u64, owner: u64, top_out: u64) -> u8 {
    if obj == 0 || name == 0 {
        return 0;
    }

    let Some(nib_name) = crate::cfstring::extract_cfstring_cstr(name) else {
        return 0;
    };
    let nib_name = nib_name.strip_suffix(".nib").unwrap_or(nib_name);

    let Some(nib_resource) =
        resolve_bundle_resource_path_parts(obj, nib_name, Some("nib"), None, None)
    else {
        log::debug!(
            "NSBundle loadNibNamed: '{}' not found in bundle '{}'",
            nib_name,
            bundle_path_string(obj).unwrap_or_default()
        );
        return 0;
    };
    let Some(nib_file) = resolve_nib_file_path(&nib_resource) else {
        log::debug!(
            "NSBundle loadNibNamed: '{}' resolved to non-loadable path '{}'",
            nib_name,
            nib_resource.display()
        );
        return 0;
    };

    let ok = load_nib_file_at_path(&nib_file, owner, top_out);
    if ok != 0 {
        log::debug!("NSBundle loadNibNamed: loaded '{}' successfully", nib_name);
    }
    ok
}

fn value_for_string_key(dict: u64, key: &str) -> u64 {
    if dict == 0 {
        return 0;
    }
    let key = make_nsstring_from_string(key);
    crate::nsdictionary::shim_nsdictionary_object_for_key(dict, 0, key)
}

fn owner_from_external_name_table(table: u64) -> u64 {
    let owner = value_for_string_key(table, "NSOwner");
    if owner != 0 {
        owner
    } else {
        value_for_string_key(table, "NSNibOwner")
    }
}

/// `-[NSBundle loadNibNamed:owner:topLevelObjects:]`
///
/// Finds a `.nib` file in the bundle, unarchives it via NSKeyedUnarchiver,
/// and returns the decoded object graph. Returns YES if the nib was found
/// and loaded, NO otherwise.
pub extern "C" fn shim_nsbundle_load_nib_named(
    obj: u64,
    _cmd: u64,
    name: u64,
    owner: u64,
    top_out: u64,
) -> u8 {
    load_nib_named_from_bundle(obj, name, owner, top_out)
}

/// `+[NSBundle loadNibNamed:owner:]`.
pub extern "C" fn shim_nsbundle_load_nib_named_owner(
    _cls: u64,
    _cmd: u64,
    name: u64,
    owner: u64,
) -> u8 {
    load_nib_named_from_bundle(get_or_create_main_bundle(), name, owner, 0)
}

/// `+[NSBundle loadNibFile:externalNameTable:withZone:]`.
pub extern "C" fn shim_nsbundle_load_nib_file_external_name_table_with_zone(
    _cls: u64,
    _cmd: u64,
    path: u64,
    external_name_table: u64,
    _zone: u64,
) -> u8 {
    let Some(path) = nsstring_to_string(path) else {
        return 0;
    };
    let Some(nib_file) = resolve_nib_file_path(Path::new(&path)) else {
        log::debug!("NSBundle loadNibFile: '{}' is not a loadable nib", path);
        return 0;
    };
    load_nib_file_at_path(
        &nib_file,
        owner_from_external_name_table(external_name_table),
        0,
    )
}

/// `+[NSBundle bundleForClass:]` — returns mainBundle.
pub extern "C" fn shim_nsbundle_bundle_for_class(_cls: u64, _cmd: u64, _class: u64) -> u64 {
    get_or_create_main_bundle()
}

/// `-[NSBundle URLForResource:withExtension:]`.
pub extern "C" fn shim_nsbundle_url_for_resource(obj: u64, _cmd: u64, name: u64, ext: u64) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, ext, 0, 0) else {
        return 0;
    };
    file_url_from_path(&path, path.is_dir())
}

/// `-[NSBundle URLForResource:withExtension:subdirectory:]`.
pub extern "C" fn shim_nsbundle_url_for_resource_subdirectory(
    obj: u64,
    _cmd: u64,
    name: u64,
    ext: u64,
    subdir: u64,
) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, ext, subdir, 0) else {
        return 0;
    };
    file_url_from_path(&path, path.is_dir())
}

/// `-[NSBundle URLForResource:withExtension:subdirectory:localization:]`.
pub extern "C" fn shim_nsbundle_url_for_resource_subdirectory_localization(
    obj: u64,
    _cmd: u64,
    name: u64,
    ext: u64,
    subdir: u64,
    localization: u64,
) -> u64 {
    let Some(path) = resolve_bundle_resource_path(obj, name, ext, subdir, localization) else {
        return 0;
    };
    file_url_from_path(&path, path.is_dir())
}

/// `-[NSBundle URLsForResourcesWithExtension:subdirectory:]`.
pub extern "C" fn shim_nsbundle_urls_for_resources_with_extension_subdirectory(
    obj: u64,
    _cmd: u64,
    ext: u64,
    subdir: u64,
) -> u64 {
    let urls = collect_bundle_resource_paths(obj, ext, subdir, 0)
        .iter()
        .map(|path| file_url_from_path(path, path.is_dir()))
        .filter(|url| *url != 0)
        .collect::<Vec<_>>();
    crate::nsarray::make_nsarray(&urls)
}

/// `-[NSBundle URLsForResourcesWithExtension:subdirectory:localization:]`.
pub extern "C" fn shim_nsbundle_urls_for_resources_with_extension_subdirectory_localization(
    obj: u64,
    _cmd: u64,
    ext: u64,
    subdir: u64,
    localization: u64,
) -> u64 {
    let urls = collect_bundle_resource_paths(obj, ext, subdir, localization)
        .iter()
        .map(|path| file_url_from_path(path, path.is_dir()))
        .filter(|url| *url != 0)
        .collect::<Vec<_>>();
    crate::nsarray::make_nsarray(&urls)
}

/// `-[NSBundle localizedStringForKey:value:table:]`
pub extern "C" fn shim_nsbundle_localized_string(
    obj: u64,
    _cmd: u64,
    key: u64,
    value: u64,
    table: u64,
) -> u64 {
    if key == 0 {
        return 0;
    }

    if obj != 0 {
        if let Some(localized) = localized_string_lookup(obj, key, table) {
            return make_nsstring_from_string(&localized);
        }
    }

    localized_string_fallback(key, value)
}

/// `-[NSBundle localizations]`.
pub extern "C" fn shim_nsbundle_localizations(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    nsarray_from_strings(&bundle_localization_names(obj))
}

/// `-[NSBundle preferredLocalizations]`.
pub extern "C" fn shim_nsbundle_preferred_localizations(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    nsarray_from_strings(&bundle_preferred_localization_names(obj))
}

/// `-[NSBundle developmentLocalization]`.
pub extern "C" fn shim_nsbundle_development_localization(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    bundle_development_localization(obj)
        .map(|value| make_nsstring_from_string(&value))
        .unwrap_or(0)
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
    if connections.is_empty() {
        return;
    }

    log::debug!("NIB wiring: {} connections to process", connections.len());

    for conn in &connections {
        match conn {
            NibConnection::Outlet {
                source,
                destination,
                label,
            } => {
                // For file's-owner outlets: source might be the placeholder
                // file's owner object (typically UID 2 in NIB archives).
                // If `owner` was provided and source is non-zero, we use owner
                // as the target for KVC.
                let target = if owner != 0 { owner } else { *source };
                let key_c = std::ffi::CString::new(label.as_str()).unwrap_or_default();
                let key_ns = crate::cfstring::alloc_nsstring_from_cstr(key_c.as_ptr());
                log::debug!(
                    "NIB wiring outlet: target={:#x} key='{}' value={:#x}",
                    target,
                    label,
                    destination
                );
                crate::nskeyvaluecoding::shim_set_value_for_key(target, 0, *destination, key_ns);
            }
            NibConnection::Action {
                source,
                destination,
                label,
            } => {
                if *source == 0 {
                    continue;
                }
                // Set target on the control.
                let set_target_imp = crate::nskeyedarchiver::lookup_imp(*source, "setTarget:");
                if set_target_imp != 0 {
                    let f: extern "C" fn(u64, u64, u64) =
                        unsafe { std::mem::transmute(set_target_imp) };
                    f(*source, 0, *destination);
                }
                // Set action selector string on the control.
                let set_action_imp = crate::nskeyedarchiver::lookup_imp(*source, "setAction:");
                if set_action_imp != 0 {
                    let sel_c = std::ffi::CString::new(label.as_str()).unwrap_or_default();
                    let sel = crate::cfstring::alloc_nsstring_from_cstr(sel_c.as_ptr());
                    let f: extern "C" fn(u64, u64, u64) =
                        unsafe { std::mem::transmute(set_action_imp) };
                    f(*source, 0, sel);
                    log::debug!(
                        "NIB wiring action: source={:#x} target={:#x} action='{}'",
                        source,
                        destination,
                        label
                    );
                }
            }
        }
    }

    // Call awakeFromNib on all objects that respond to it.
    let mut seen = std::collections::HashSet::new();
    for conn in &connections {
        let objects = match conn {
            NibConnection::Outlet {
                source,
                destination,
                ..
            } => vec![*source, *destination],
            NibConnection::Action {
                source,
                destination,
                ..
            } => vec![*source, *destination],
        };
        for obj in objects {
            if obj == 0 || !seen.insert(obj) {
                continue;
            }
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

    fn unique_temp_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cf-lite-nsbundle-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn make_temp_app_bundle(label: &str) -> (std::path::PathBuf, u64) {
        let root = unique_temp_root(label).join("Sample.app");
        std::fs::create_dir_all(root.join("Contents").join("Resources"))
            .expect("create app resource tree");
        let (obj, _) = make_bundle_with_path(root.to_string_lossy().as_ref());
        (root, obj)
    }

    fn assert_url_path_eq(url: u64, expected: &std::path::Path) {
        assert_ne!(url, 0);
        assert_string_eq(
            crate::nsurl::shim_nsurl_path(url, 0),
            expected.to_string_lossy().as_ref(),
        );
    }

    fn nsarray_strings(array: u64) -> Vec<String> {
        let count = crate::nsarray::shim_nsarray_count(array, 0);
        (0..count)
            .map(|index| {
                read_string(crate::nsarray::shim_nsarray_object_at_index(
                    array, 0, index,
                ))
            })
            .collect()
    }

    fn apple_archive_string_bytes(value: &str) -> Vec<u8> {
        let mut top = plist::Dictionary::new();
        top.insert("root".to_string(), plist::Value::Uid(plist::Uid::new(1)));

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
        archive.insert(
            "$objects".to_string(),
            plist::Value::Array(vec![
                plist::Value::String("$null".to_string()),
                plist::Value::String(value.to_string()),
            ]),
        );

        let mut bytes = Vec::new();
        plist::to_writer_binary(&mut bytes, &plist::Value::Dictionary(archive))
            .expect("write test keyed archive");
        bytes
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
        let result =
            detect_app_bundle_root("/Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild");
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
        assert_string_eq(
            crate::nsurl::shim_nsurl_absolute_string(result, 0),
            "file:///Applications/Test.app",
        );
        free_obj(obj);
    }

    // --- shim_nsbundle_bundle_identifier ---

    #[test]
    fn bundle_identifier_no_plist_returns_nil() {
        let obj = alloc_obj_fake(NSBUNDLE_SIZE);
        assert_eq!(shim_nsbundle_bundle_identifier(obj, 0), 0);
        free_obj(obj);
    }

    #[test]
    fn bundle_identifier_reads_info_plist() {
        let (root, obj) = make_temp_app_bundle("identifier");
        let info = root.join("Contents").join("Info.plist");
        std::fs::write(
            &info,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.example.Sample</string>
</dict></plist>"#,
        )
        .expect("write Info.plist");

        let result = shim_nsbundle_bundle_identifier(obj, 0);
        assert_string_eq(result, "com.example.Sample");

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
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

    #[test]
    fn class_load_nib_named_nil_returns_no() {
        assert_eq!(shim_nsbundle_load_nib_named_owner(0, 0, 0, 0), 0);
    }

    #[test]
    fn load_nib_named_finds_base_localized_directory_nib() {
        let (root, obj) = make_temp_app_bundle("localized-nib");
        let nib_dir = root
            .join("Contents")
            .join("Resources")
            .join("Base.lproj")
            .join("MainMenu.nib");
        std::fs::create_dir_all(&nib_dir).expect("create nib directory");
        std::fs::write(
            nib_dir.join("keyedobjects.nib"),
            apple_archive_string_bytes("nib-root"),
        )
        .expect("write keyed nib");

        let mut top_level = 0_u64;
        let ok = shim_nsbundle_load_nib_named(
            obj,
            0,
            make_string("MainMenu"),
            0,
            (&mut top_level as *mut u64) as u64,
        );

        assert_eq!(ok, 1);
        assert_ne!(top_level, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(top_level, 0), 1);
        assert_string_eq(
            crate::nsarray::shim_nsarray_object_at_index(top_level, 0, 0),
            "nib-root",
        );

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn load_nib_file_external_name_table_loads_directory_nib() {
        let root = unique_temp_root("load-nib-file");
        let nib_dir = root.join("MainMenu.nib");
        std::fs::create_dir_all(&nib_dir).expect("create nib directory");
        std::fs::write(
            nib_dir.join("keyedobjects.nib"),
            apple_archive_string_bytes("nib-root"),
        )
        .expect("write keyed nib");

        let owner = 0xABCD_u64;
        let key = make_string("NSOwner");
        let table = crate::nsdictionary::make_nsdictionary(&[key], &[owner]);
        let ok = shim_nsbundle_load_nib_file_external_name_table_with_zone(
            0,
            0,
            make_string(nib_dir.to_string_lossy().as_ref()),
            table,
            0,
        );

        assert_eq!(ok, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn external_name_table_prefers_nsowner_then_nsnibowner() {
        let owner = 0x1111_u64;
        let fallback_owner = 0x2222_u64;
        let nsowner_key = make_string("NSOwner");
        let nsnibowner_key = make_string("NSNibOwner");
        let table = crate::nsdictionary::make_nsdictionary(
            &[nsnibowner_key, nsowner_key],
            &[fallback_owner, owner],
        );
        assert_eq!(owner_from_external_name_table(table), owner);

        let fallback_table =
            crate::nsdictionary::make_nsdictionary(&[nsnibowner_key], &[fallback_owner]);
        assert_eq!(
            owner_from_external_name_table(fallback_table),
            fallback_owner
        );
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
        let (obj, _) = make_bundle_with_path("/nonexistent/bundle/dir");
        let result = shim_nsbundle_resource_path(obj, 0);
        assert_string_eq(result, "/nonexistent/bundle/dir");
        free_obj(obj);
    }

    #[test]
    fn resource_path_prefers_contents_resources_for_app_bundle() {
        let (root, obj) = make_temp_app_bundle("resource-path");
        let expected = root.join("Contents").join("Resources");

        assert_string_eq(
            shim_nsbundle_resource_path(obj, 0),
            expected.to_string_lossy().as_ref(),
        );
        assert_string_eq(
            crate::nsurl::shim_nsurl_path(shim_nsbundle_resource_url(obj, 0), 0),
            &format!("{}/", expected.to_string_lossy()),
        );

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
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

    #[test]
    fn path_for_resource_finds_top_level_resource() {
        let (root, obj) = make_temp_app_bundle("top-resource");
        let file = root.join("Contents").join("Resources").join("Icon.icns");
        std::fs::write(&file, b"icon").expect("write resource");

        let name = make_string("Icon");
        let ext = make_string("icns");
        let result = shim_nsbundle_path_for_resource(obj, 0, name, ext);
        assert_string_eq(result, file.to_string_lossy().as_ref());

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn path_for_resource_does_not_duplicate_existing_extension() {
        let (root, obj) = make_temp_app_bundle("existing-ext");
        let file = root
            .join("Contents")
            .join("Resources")
            .join("Main.storyboardc");
        std::fs::create_dir_all(&file).expect("create storyboard resource");

        let name = make_string("Main.storyboardc");
        let ext = make_string("storyboardc");
        let result = shim_nsbundle_path_for_resource(obj, 0, name, ext);
        assert_string_eq(result, file.to_string_lossy().as_ref());

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn path_for_resource_searches_subdirectory_and_localization() {
        let (root, obj) = make_temp_app_bundle("localized-resource");
        let dir = root
            .join("Contents")
            .join("Resources")
            .join("fr.lproj")
            .join("Images");
        std::fs::create_dir_all(&dir).expect("create localized subdir");
        let file = dir.join("Welcome.txt");
        std::fs::write(&file, b"bonjour").expect("write localized resource");

        let result = shim_nsbundle_path_for_resource_in_directory_for_localization(
            obj,
            0,
            make_string("Welcome"),
            make_string("txt"),
            make_string("Images"),
            make_string("fr"),
        );
        assert_string_eq(result, file.to_string_lossy().as_ref());

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn path_for_resource_falls_back_to_base_localization() {
        let (root, obj) = make_temp_app_bundle("base-resource");
        let dir = root.join("Contents").join("Resources").join("Base.lproj");
        std::fs::create_dir_all(&dir).expect("create base localization");
        let file = dir.join("Main.nib");
        std::fs::write(&file, b"nib").expect("write base resource");

        let result =
            shim_nsbundle_path_for_resource(obj, 0, make_string("Main"), make_string("nib"));
        assert_string_eq(result, file.to_string_lossy().as_ref());

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
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
    fn url_for_resource_returns_file_url() {
        let (root, obj) = make_temp_app_bundle("resource-url");
        let file = root
            .join("Contents")
            .join("Resources")
            .join("Assets")
            .join("Logo.png");
        std::fs::create_dir_all(file.parent().unwrap()).expect("create assets dir");
        std::fs::write(&file, b"png").expect("write png resource");

        let url = shim_nsbundle_url_for_resource_subdirectory(
            obj,
            0,
            make_string("Logo"),
            make_string("png"),
            make_string("Assets"),
        );
        assert_url_path_eq(url, &file);

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn url_for_resource_honors_localization() {
        let (root, obj) = make_temp_app_bundle("localized-url");
        let file = root
            .join("Contents")
            .join("Resources")
            .join("de.lproj")
            .join("Strings")
            .join("Localizable.strings");
        std::fs::create_dir_all(file.parent().unwrap()).expect("create strings dir");
        std::fs::write(&file, b"hello = hallo;").expect("write strings resource");

        let url = shim_nsbundle_url_for_resource_subdirectory_localization(
            obj,
            0,
            make_string("Localizable"),
            make_string("strings"),
            make_string("Strings"),
            make_string("de.lproj"),
        );
        assert_url_path_eq(url, &file);

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn urls_for_resources_collects_matching_extension() {
        let (root, obj) = make_temp_app_bundle("resource-urls");
        let resources = root.join("Contents").join("Resources").join("Sounds");
        std::fs::create_dir_all(&resources).expect("create sounds dir");
        let first = resources.join("A.wav");
        let second = resources.join("B.wav");
        std::fs::write(&first, b"a").expect("write first wav");
        std::fs::write(&second, b"b").expect("write second wav");
        std::fs::write(resources.join("Ignored.txt"), b"x").expect("write ignored resource");

        let urls = shim_nsbundle_urls_for_resources_with_extension_subdirectory(
            obj,
            0,
            make_string("wav"),
            make_string("Sounds"),
        );
        assert_eq!(crate::nsarray::shim_nsarray_count(urls, 0), 2);
        assert_url_path_eq(
            crate::nsarray::shim_nsarray_object_at_index(urls, 0, 0),
            &first,
        );
        assert_url_path_eq(
            crate::nsarray::shim_nsarray_object_at_index(urls, 0, 1),
            &second,
        );

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
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

    #[test]
    fn localization_metadata_nil_bundle_returns_zero() {
        assert_eq!(shim_nsbundle_localizations(0, 0), 0);
        assert_eq!(shim_nsbundle_preferred_localizations(0, 0), 0);
        assert_eq!(shim_nsbundle_development_localization(0, 0), 0);
    }

    #[test]
    fn localization_metadata_reads_lproj_dirs_and_development_region() {
        let (root, obj) = make_temp_app_bundle("localization-metadata");
        let resources = root.join("Contents").join("Resources");
        std::fs::create_dir_all(resources.join("Base.lproj")).expect("create Base.lproj");
        std::fs::create_dir_all(resources.join("en.lproj")).expect("create en.lproj");
        std::fs::create_dir_all(resources.join("fr.lproj")).expect("create fr.lproj");
        std::fs::write(
            root.join("Contents").join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>fr</string>
</dict>
</plist>
"#,
        )
        .expect("write info plist");

        assert_string_eq(shim_nsbundle_development_localization(obj, 0), "fr");
        assert_eq!(
            nsarray_strings(shim_nsbundle_localizations(obj, 0)),
            vec!["fr", "Base", "en"]
        );
        assert_eq!(
            nsarray_strings(shim_nsbundle_preferred_localizations(obj, 0)),
            vec!["fr", "en", "Base"]
        );

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn localized_string_reads_default_text_strings_table() {
        let (root, obj) = make_temp_app_bundle("localized-strings");
        let dir = root.join("Contents").join("Resources").join("Base.lproj");
        std::fs::create_dir_all(&dir).expect("create localization dir");
        std::fs::write(
            dir.join("Localizable.strings"),
            r#"
                /* menu title */
                "HELLO_KEY" = "Hello from resources";
                "ESCAPED" = "Line\nBreak \U263A";
            "#,
        )
        .expect("write strings table");

        let result = shim_nsbundle_localized_string(obj, 0, make_string("HELLO_KEY"), 0, 0);
        assert_string_eq(result, "Hello from resources");

        let escaped = shim_nsbundle_localized_string(obj, 0, make_string("ESCAPED"), 0, 0);
        assert_string_eq(escaped, "Line\nBreak \u{263a}");

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn localized_string_reads_named_table_and_falls_back_when_missing() {
        let (root, obj) = make_temp_app_bundle("localized-named-strings");
        let dir = root.join("Contents").join("Resources").join("en.lproj");
        std::fs::create_dir_all(&dir).expect("create localization dir");
        std::fs::write(dir.join("Menus.strings"), r#""Open" = "Open File";"#)
            .expect("write named strings table");

        let result = shim_nsbundle_localized_string(
            obj,
            0,
            make_string("Open"),
            0,
            make_string("Menus.strings"),
        );
        assert_string_eq(result, "Open File");

        let fallback = make_string("Fallback");
        let missing = shim_nsbundle_localized_string(
            obj,
            0,
            make_string("Missing"),
            fallback,
            make_string("Menus"),
        );
        assert_eq!(missing, fallback);

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn localized_string_reads_utf16_strings_table() {
        let (root, obj) = make_temp_app_bundle("localized-utf16-strings");
        let dir = root.join("Contents").join("Resources").join("Base.lproj");
        std::fs::create_dir_all(&dir).expect("create localization dir");

        let mut bytes = vec![0xFF, 0xFE];
        for unit in r#""HELLO" = "Bonjour";"#.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(dir.join("Localizable.strings"), bytes).expect("write utf16 strings");

        let result = shim_nsbundle_localized_string(obj, 0, make_string("HELLO"), 0, 0);
        assert_string_eq(result, "Bonjour");

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn localized_string_reads_xml_plist_strings_table() {
        let (root, obj) = make_temp_app_bundle("localized-plist-strings");
        let file = root
            .join("Contents")
            .join("Resources")
            .join("Localizable.strings");
        let mut dict = plist::Dictionary::new();
        dict.insert(
            "TITLE".to_string(),
            plist::Value::String("Plist Title".to_string()),
        );
        plist::to_file_xml(&file, &plist::Value::Dictionary(dict)).expect("write plist strings");

        let result = shim_nsbundle_localized_string(obj, 0, make_string("TITLE"), 0, 0);
        assert_string_eq(result, "Plist Title");

        free_obj(obj);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }
}
