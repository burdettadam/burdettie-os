//! DVTFoundation / IDEFoundation framework C function stubs.
//!
//! These are the C-linkage initialization and utility functions that
//! Xcode tools (xcodebuild, ibtool, actool, genstrings, etc.) call
//! during their startup sequence. Without stubs, these bind targets
//! hit trap addresses and crash.
//!
//! Init chain: DVTEnvironmentInitialize → DVTFoundationInitialize → IDEInitialize
//!
//! The stubs return success (0) or NULL as appropriate so the tools
//! can proceed past framework initialization.

use crate::table::ShimTable;

pub struct DvtFoundationStubs;

impl DvtFoundationStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Core initialization chain ───────────────────────────────
        register_shims!(
            table,
            (
                "_DVTEnvironmentInitialize",
                shim_dvt_void_stub as *const () as u64,
                "DVTEnvironmentInitialize (stub → void)"
            ),
            (
                "_DVTFoundationInitialize",
                shim_dvt_void_stub as *const () as u64,
                "DVTFoundationInitialize (stub → void)"
            ),
            (
                "_DVTFoundationInitializeRegisteredDefaults",
                shim_dvt_void_stub as *const () as u64,
                "DVTFoundationInitializeRegisteredDefaults (stub → void)"
            ),
            (
                "_IDEInitialize",
                shim_dvt_ret0_stub as *const () as u64,
                "IDEInitialize (stub → 0/success)"
            ),
            (
                "_IDEInitializeWithOptions",
                shim_dvt_ret0_stub as *const () as u64,
                "IDEInitializeWithOptions (stub → 0/success)"
            ),
        );

        // ── DVT logging ─────────────────────────────────────────────
        register_shims!(
            table,
            (
                "_DVTLogEnable",
                shim_dvt_void_stub as *const () as u64,
                "DVTLogEnable (stub → void)"
            ),
            (
                "_DVTLogSetLevel",
                shim_dvt_void_stub as *const () as u64,
                "DVTLogSetLevel (stub → void)"
            ),
            (
                "_DVTLogSetLogFileURL",
                shim_dvt_void_stub as *const () as u64,
                "DVTLogSetLogFileURL (stub → void)"
            ),
            (
                "_DVTLogCreate",
                shim_dvt_ret0_stub as *const () as u64,
                "DVTLogCreate (stub → NULL)"
            ),
        );

        // ── DVT developer paths ─────────────────────────────────────
        register_shims!(
            table,
            (
                "_DVTDeveloperPathsXcodeAppPath",
                shim_dvt_developer_xcode_app_path as *const () as u64,
                "DVTDeveloperPathsXcodeAppPath → /Applications/Xcode.app"
            ),
            (
                "_DVTDeveloperPathsSystemDeveloperDir",
                shim_dvt_developer_system_developer_dir as *const () as u64,
                "DVTDeveloperPathsSystemDeveloperDir → /Applications/Xcode.app/Contents/Developer"
            ),
            (
                "_DVTDeveloperPathsToolchainDir",
                shim_dvt_developer_toolchain_dir as *const () as u64,
                "DVTDeveloperPathsToolchainDir → toolchain path"
            ),
            (
                "_DVTDeveloperPathsSDKsDir",
                shim_dvt_developer_sdks_dir as *const () as u64,
                "DVTDeveloperPathsSDKsDir → SDKs path"
            ),
            (
                "_DVTDeveloperPathsPlatformsDir",
                shim_dvt_developer_platforms_dir as *const () as u64,
                "DVTDeveloperPathsPlatformsDir → Platforms path"
            ),
        );

        // ── DVT utility functions ───────────────────────────────────
        register_shims!(
            table,
            (
                "_DVTAssertionFailureHandler",
                shim_dvt_void_stub as *const () as u64,
                "DVTAssertionFailureHandler (stub → void)"
            ),
            (
                "_DVTAssertionHandler",
                shim_dvt_ret0_stub as *const () as u64,
                "DVTAssertionHandler (stub → NULL)"
            ),
            (
                "_DVTAddSIGHandlers",
                shim_dvt_void_stub as *const () as u64,
                "DVTAddSIGHandlers (stub → void)"
            ),
            (
                "_DVTModelObjectGraphSentinel",
                shim_dvt_ret0_stub as *const () as u64,
                "DVTModelObjectGraphSentinel (stub → NULL)"
            ),
            (
                "_DVTPlugInManagerSetUpOptions",
                shim_dvt_void_stub as *const () as u64,
                "DVTPlugInManagerSetUpOptions (stub → void)"
            ),
            (
                "_DVTPlugInManagerScan",
                shim_dvt_void_stub as *const () as u64,
                "DVTPlugInManagerScan (stub → void)"
            ),
        );

        // ── IDE utility functions ───────────────────────────────────
        register_shims!(
            table,
            (
                "_IDESetSafeToLoadFrameworks",
                shim_dvt_void_stub as *const () as u64,
                "IDESetSafeToLoadFrameworks (stub → void)"
            ),
            (
                "_IDERegisterStaticAnalyzerCheckers",
                shim_dvt_void_stub as *const () as u64,
                "IDERegisterStaticAnalyzerCheckers (stub → void)"
            ),
        );

        log::debug!("dvt_foundation: registered DVT/IDE C function stubs");
    }
}

// ── Stub implementations ─────────────────────────────────────────

/// Void stub — does nothing, returns nothing.
extern "C" fn shim_dvt_void_stub() {}

/// Return-zero stub — returns 0 (NULL / success / false).
extern "C" fn shim_dvt_ret0_stub() -> u64 {
    0
}

// ── DVT developer path helpers ───────────────────────────────────
//
// These return CFString pointers (via the same alloc_cfstring helper
// used by CoreText stubs). The paths match the Docker layout at
// /opt/xcode-tools/ and fall back to /Applications/Xcode.app on macOS.

fn alloc_cfstring(s: &str) -> u64 {
    let len = s.len();
    let ptr = unsafe { libc::calloc(1, 32 + len + 1) } as u64;
    if ptr == 0 {
        return 0;
    }
    let buf = ptr as *mut u8;
    unsafe {
        // Write simple inline string storage:
        // [0..8):  ISA (placeholder 0x1)
        // [8..16): length as u64
        // [16..24): pointer to inline chars at offset 32
        // [24..32): capacity
        // [32..):  UTF-8 bytes + NUL
        let isa = 0x1u64;
        std::ptr::copy_nonoverlapping(&isa as *const u64 as *const u8, buf, 8);
        let len_u64 = len as u64;
        std::ptr::copy_nonoverlapping(&len_u64 as *const u64 as *const u8, buf.add(8), 8);
        let chars_ptr = ptr + 32;
        std::ptr::copy_nonoverlapping(&chars_ptr as *const u64 as *const u8, buf.add(16), 8);
        std::ptr::copy_nonoverlapping(s.as_ptr(), buf.add(32), len);
        *buf.add(32 + len) = 0; // NUL terminator
    }
    ptr
}

/// Returns the Xcode.app path as a CFString.
///
/// On Docker/Linux: /opt/xcode-tools  
/// On macOS: reads DEVELOPER_DIR or defaults to /Applications/Xcode.app
extern "C" fn shim_dvt_developer_xcode_app_path() -> u64 {
    let path = xcode_app_path();
    alloc_cfstring(&path)
}

extern "C" fn shim_dvt_developer_system_developer_dir() -> u64 {
    let path = format!("{}/Contents/Developer", xcode_app_path());
    alloc_cfstring(&path)
}

extern "C" fn shim_dvt_developer_toolchain_dir() -> u64 {
    let path = format!(
        "{}/Contents/Developer/Toolchains/XcodeDefault.xctoolchain",
        xcode_app_path()
    );
    alloc_cfstring(&path)
}

extern "C" fn shim_dvt_developer_sdks_dir() -> u64 {
    let path = format!(
        "{}/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs",
        xcode_app_path()
    );
    alloc_cfstring(&path)
}

extern "C" fn shim_dvt_developer_platforms_dir() -> u64 {
    let path = format!("{}/Contents/Developer/Platforms", xcode_app_path());
    alloc_cfstring(&path)
}

/// Determine the Xcode application path.
fn xcode_app_path() -> String {
    // Check DEVELOPER_DIR first (standard env var)
    if let Ok(dev_dir) = std::env::var("DEVELOPER_DIR") {
        // DEVELOPER_DIR typically points to .../Contents/Developer
        // Strip /Contents/Developer to get the .app path
        if let Some(app) = dev_dir.strip_suffix("/Contents/Developer") {
            return app.to_string();
        }
        return dev_dir;
    }
    // Docker layout
    if std::path::Path::new("/opt/xcode-tools").exists() {
        return "/opt/xcode-tools".to_string();
    }
    // macOS default
    "/Applications/Xcode.app".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_stub_does_not_crash() {
        shim_dvt_void_stub();
    }

    #[test]
    fn ret0_stub_returns_zero() {
        assert_eq!(shim_dvt_ret0_stub(), 0);
    }

    #[test]
    fn xcode_app_path_not_empty() {
        let p = xcode_app_path();
        assert!(!p.is_empty());
    }

    #[test]
    fn developer_paths_return_cfstrings() {
        let path = shim_dvt_developer_xcode_app_path();
        assert_ne!(path, 0, "xcode_app_path should return non-NULL CFString");

        let sys_dev = shim_dvt_developer_system_developer_dir();
        assert_ne!(sys_dev, 0);

        let toolchain = shim_dvt_developer_toolchain_dir();
        assert_ne!(toolchain, 0);

        let sdks = shim_dvt_developer_sdks_dir();
        assert_ne!(sdks, 0);

        let platforms = shim_dvt_developer_platforms_dir();
        assert_ne!(platforms, 0);
    }

    #[test]
    fn alloc_cfstring_stores_bytes() {
        let s = "Hello DVT";
        let ptr = alloc_cfstring(s);
        assert_ne!(ptr, 0);
        // Read back the string bytes from offset 32
        unsafe {
            let buf = ptr as *const u8;
            let stored_len = *(buf.add(8) as *const u64);
            assert_eq!(stored_len, s.len() as u64);
            let chars_ptr = *(buf.add(16) as *const u64) as *const u8;
            let slice = std::slice::from_raw_parts(chars_ptr, s.len());
            assert_eq!(std::str::from_utf8(slice).unwrap(), s);
        }
    }

    #[test]
    fn stubs_install_without_panic() {
        let mut table = crate::table::ShimTable::new();
        DvtFoundationStubs::install(&mut table);
        // Verify key symbols are registered
        assert!(table.resolve("_DVTEnvironmentInitialize").is_some());
        assert!(table.resolve("_DVTFoundationInitialize").is_some());
        assert!(table.resolve("_IDEInitialize").is_some());
        assert!(table.resolve("_DVTDeveloperPathsXcodeAppPath").is_some());
        assert!(table
            .resolve("_DVTDeveloperPathsSystemDeveloperDir")
            .is_some());
    }
}
