//! App bundle resolution helpers.

use std::path::{Path, PathBuf};

/// Resolve an `.app` bundle path to the executable inside.
///
/// Given a path like `Foo.app`, reads `Contents/Info.plist` for
/// `CFBundleExecutable` and returns `Foo.app/Contents/MacOS/<exe>`.
/// Returns `None` if the path is not an `.app` bundle.
pub(super) fn resolve_bundle(bundle_path: &Path) -> Option<PathBuf> {
    let name = bundle_path.file_name()?.to_str()?;
    if !name.ends_with(".app") {
        return None;
    }

    let contents = bundle_path.join("Contents");
    let macos = contents.join("MacOS");

    // Try Info.plist for CFBundleExecutable
    let plist_path = contents.join("Info.plist");
    if let Ok(data) = std::fs::read_to_string(&plist_path) {
        if let Some(exe_name) = parse_cf_bundle_executable(&data) {
            let exe = macos.join(&exe_name);
            if exe.exists() {
                return Some(exe);
            }
        }
    }

    // Fallback: bundle name without .app suffix
    let stem = name.strip_suffix(".app")?;
    let exe = macos.join(stem);
    if exe.exists() {
        return Some(exe);
    }

    // Last resort: any executable in Contents/MacOS
    if let Ok(entries) = std::fs::read_dir(&macos) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

/// Minimal plist parser: extract CFBundleExecutable from Info.plist XML.
fn parse_cf_bundle_executable(plist_xml: &str) -> Option<String> {
    let key_tag = "<key>CFBundleExecutable</key>";
    let key_pos = plist_xml.find(key_tag)?;
    let after_key = &plist_xml[key_pos + key_tag.len()..];

    let string_start = after_key.find("<string>")?;
    let value_start = string_start + "<string>".len();
    let value_end = after_key[value_start..].find("</string>")?;

    let value = after_key[value_start..value_start + value_end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn non_app_path_returns_none() {
        assert!(resolve_bundle(Path::new("/tmp/foo")).is_none());
        assert!(resolve_bundle(Path::new("/tmp/foo.txt")).is_none());
    }

    #[test]
    fn parse_cf_bundle_executable_valid() {
        let plist =
            r#"<plist><dict><key>CFBundleExecutable</key><string>MyApp</string></dict></plist>"#;
        assert_eq!(parse_cf_bundle_executable(plist), Some("MyApp".to_string()));
    }

    #[test]
    fn parse_cf_bundle_executable_missing() {
        let plist = r#"<plist><dict><key>CFBundleName</key><string>Foo</string></dict></plist>"#;
        assert!(parse_cf_bundle_executable(plist).is_none());
    }

    #[test]
    fn parse_cf_bundle_executable_empty_string() {
        let plist =
            r#"<plist><dict><key>CFBundleExecutable</key><string>  </string></dict></plist>"#;
        // Trimmed empty should return None
        assert!(parse_cf_bundle_executable(plist).is_none());
    }

    #[test]
    fn resolve_bundle_nonexistent() {
        assert!(resolve_bundle(Path::new("/tmp/nonexistent_abc123.app")).is_none());
    }

    #[test]
    fn resolve_bundle_creates_temp() {
        let base = std::env::temp_dir().join("macho_runtime_test_bundle.app");
        let macos = base.join("Contents").join("MacOS");
        let contents = base.join("Contents");

        // Clean up any previous run
        let _ = fs::remove_dir_all(&base);

        fs::create_dir_all(&macos).expect("create MacOS dir");
        let plist = r#"<?xml version="1.0"?>
<plist version="1.0">
<dict>
<key>CFBundleExecutable</key>
<string>TestExe</string>
</dict>
</plist>"#;
        fs::write(contents.join("Info.plist"), plist).expect("write plist");
        fs::write(macos.join("TestExe"), b"fake-binary").expect("write exe");

        let resolved = resolve_bundle(&base);
        assert!(resolved.is_some(), "should resolve bundle");
        assert_eq!(resolved.unwrap().file_name().unwrap(), "TestExe");

        // Clean up
        let _ = fs::remove_dir_all(&base);
    }
}
