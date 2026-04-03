//! NSWorkspace shim: workspace singleton with real URL/file opening.
//!
//! `openURL:` and `openFile:` delegate to the platform's native open command:
//! - macOS: `/usr/bin/open`
//! - Linux: `xdg-open`

use std::sync::OnceLock;

static SHARED_WORKSPACE: OnceLock<u64> = OnceLock::new();

fn get_or_create_shared_workspace() -> u64 {
    *SHARED_WORKSPACE.get_or_init(|| {
        let obj = crate::pal_alloc().calloc(1, 8) as u64;
        if obj == 0 { return 0; }
        unsafe { *(obj as *mut u64) = crate::nsobj::nsworkspace_class_addr(); }
        obj
    })
}

/// `+[NSWorkspace sharedWorkspace]`
pub extern "C" fn shim_nsworkspace_shared_workspace(_cls: u64, _cmd: u64) -> u64 {
    get_or_create_shared_workspace()
}

/// `−[NSWorkspace openURL:]` — opens a URL using the platform's default handler.
/// Returns 1 (YES) on success, 0 (NO) on failure.
pub extern "C" fn shim_nsworkspace_open_url(_obj: u64, _cmd: u64, url_obj: u64) -> i32 {
    let Some(url_str) = extract_url_string(url_obj) else { return 0; };
    platform_open(&url_str)
}

/// `−[NSWorkspace openFile:]` — opens a file path using the platform's default handler.
pub extern "C" fn shim_nsworkspace_open_file(_obj: u64, _cmd: u64, path_obj: u64) -> i32 {
    let Some(path_str) = crate::cfstring::extract_cfstring_cstr(path_obj) else { return 0; };
    platform_open(path_str)
}

/// `−[NSWorkspace openFile:withApplication:]` — opens a file with a specific app.
pub extern "C" fn shim_nsworkspace_open_file_with_app(
    _obj: u64, _cmd: u64, path_obj: u64, _app_obj: u64,
) -> i32 {
    // Ignore application name, just open the file
    let Some(path_str) = crate::cfstring::extract_cfstring_cstr(path_obj) else { return 0; };
    platform_open(path_str)
}

/// `−[NSWorkspace runningApplications]` — returns empty NSArray.
pub extern "C" fn shim_nsworkspace_running_applications(_obj: u64, _cmd: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}

/// `−[NSWorkspace frontmostApplication]` — returns nil.
pub extern "C" fn shim_nsworkspace_frontmost_application(_obj: u64, _cmd: u64) -> u64 { 0 }

/// `−[NSWorkspace iconForFile:]` — returns nil (no icon support).
pub extern "C" fn shim_nsworkspace_icon_for_file(_obj: u64, _cmd: u64, _path: u64) -> u64 { 0 }

// ── Platform open helper ───────────────────────────────────────

fn platform_open(target: &str) -> i32 {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(not(target_os = "macos"))]
    let cmd = "xdg-open";

    match std::process::Command::new(cmd)
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => {
            log::debug!("NSWorkspace: opened {target:?} via {cmd}");
            1 // YES
        }
        Err(e) => {
            log::debug!("NSWorkspace: failed to open {target:?}: {e}");
            0 // NO
        }
    }
}

/// Extract URL string from an NSURL object (reads the _urlString field at offset 8).
fn extract_url_string(url_obj: u64) -> Option<&'static str> {
    if url_obj == 0 { return None; }
    // NSURL stores an NSString at offset 8
    let nsstr = unsafe { *((url_obj as usize + 8) as *const u64) };
    crate::cfstring::extract_cfstring_cstr(nsstr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    // NSWorkspace is an 8-byte singleton: [ISA:8]
    // Allocated internally by get_or_create_shared_workspace(), not by tests.
    // We must NOT free it — it lives in a static OnceLock.

    #[test]
    fn test_shared_workspace_returns_nonzero() {
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        assert_ne!(ws, 0, "sharedWorkspace must return a non-nil singleton");
    }

    #[test]
    fn test_shared_workspace_is_singleton() {
        let ws1 = shim_nsworkspace_shared_workspace(0, 0);
        let ws2 = shim_nsworkspace_shared_workspace(0, 0);
        assert_eq!(ws1, ws2, "sharedWorkspace must return the same pointer each time");
    }

    #[test]
    fn test_shared_workspace_has_correct_isa() {
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        let isa = read_field(ws, 0);
        assert_eq!(isa, crate::nsobj::nsworkspace_class_addr(),
            "singleton ISA must be nsworkspace_class_addr()");
    }

    #[test]
    fn test_shared_workspace_nil_cls_and_cmd() {
        // Class method — _cls and _cmd are unused; passing 0 must still work.
        let ws = shim_nsworkspace_shared_workspace(0, 0);
        assert_ne!(ws, 0);
    }

    #[test]
    fn test_shared_workspace_nonzero_cls() {
        // Passing an arbitrary _cls value must still return the singleton.
        let ws = shim_nsworkspace_shared_workspace(0xCAFE, 0xBEEF);
        let ws2 = shim_nsworkspace_shared_workspace(0, 0);
        assert_eq!(ws, ws2, "sharedWorkspace ignores _cls and always returns the singleton");
    }
}
