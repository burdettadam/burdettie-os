//! Miscellaneous macOS-specific stubs.
//!
//! Groups small sets of symbols from various private/semi-private
//! frameworks: Carbon, sysmon, DataDetectors, container API,
//! NStatManager, ServiceManagement, CoreAnimation, AudioToolbox,
//! UserNotifications, os_transaction, and diagnostics helpers.

use crate::table::ShimTable;

pub struct MiscStubs;

impl MiscStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;

        // ── Carbon HI* (8 symbols) ─────────────────────────────────
        register_shims!(table,
            ("_HIToolboxInit", shim_void_stub as *const () as u64, "HIToolboxInit (stub → void)"),
            ("_HIApplicationMain", shim_void_stub as *const () as u64, "HIApplicationMain (stub → void)"),
            ("_HISizeOfTPField", shim_zero_stub as *const () as u64, "HISizeOfTPField (stub → 0)"),
            ("_HIGetScaleFactor", shim_one_f64_stub as *const () as u64, "HIGetScaleFactor (stub → 1.0)"),
            ("_HIThemeDrawButton", shim_zero_stub as *const () as u64, "HIThemeDrawButton (stub → 0)"),
            ("_HIThemeDrawFrame", shim_zero_stub as *const () as u64, "HIThemeDrawFrame (stub → 0)"),
            ("_HIThemeDrawTextBox", shim_zero_stub as *const () as u64, "HIThemeDrawTextBox (stub → 0)"),
            ("_HIWindowIsOrderedIn", shim_false_stub as *const () as u64, "HIWindowIsOrderedIn (stub → false)"),
        );

        // ── sysmon (8 symbols) ──────────────────────────────────────
        register_shims!(table,
            ("_sysmon_request_create", shim_null_stub as *const () as u64, "sysmon_request_create (stub → NULL)"),
            ("_sysmon_request_set_interval", shim_void2_stub as *const () as u64, "sysmon_request_set_interval (stub → void)"),
            ("_sysmon_request_set_flags", shim_void2_stub as *const () as u64, "sysmon_request_set_flags (stub → void)"),
            ("_sysmon_request_cancel", shim_void1_stub as *const () as u64, "sysmon_request_cancel (stub → void)"),
            ("_sysmon_request_submit", shim_err_stub as *const () as u64, "sysmon_request_submit (stub → -1)"),
            ("_sysmon_table_apply", shim_void2_stub as *const () as u64, "sysmon_table_apply (stub → void)"),
            ("_sysmon_row_get_value", shim_null_stub as *const () as u64, "sysmon_row_get_value (stub → NULL)"),
            ("_sysmon_release", shim_void1_stub as *const () as u64, "sysmon_release (stub → void)"),
        );

        // ── DataDetectors (5 symbols) ───────────────────────────────
        register_shims!(table,
            ("_DDScannerCreate", shim_null2_stub as *const () as u64, "DDScannerCreate (stub → NULL)"),
            ("_DDScannerScanQuery", shim_false2_stub as *const () as u64, "DDScannerScanQuery (stub → false)"),
            ("_DDScannerCopyResults", shim_null_stub as *const () as u64, "DDScannerCopyResults (stub → NULL)"),
            ("_DDResultGetType", shim_zero_stub as *const () as u64, "DDResultGetType (stub → 0)"),
            ("_DDResultGetRange", shim_zero_range_stub as *const () as u64, "DDResultGetRange (stub → {0,0})"),
        );

        // ── container API (9 symbols) ───────────────────────────────
        register_shims!(table,
            ("_container_create_or_lookup_path_for_current_user", shim_null3_stub as *const () as u64, "container_create_or_lookup (stub → NULL)"),
            ("_container_disk_usage", shim_zero_stub as *const () as u64, "container_disk_usage (stub → 0)"),
            ("_container_get_class", shim_zero_stub as *const () as u64, "container_get_class (stub → 0)"),
            ("_container_get_path", shim_null_stub as *const () as u64, "container_get_path (stub → NULL)"),
            ("_container_system_path_for_identifier", shim_null2_stub as *const () as u64, "container_system_path_for_identifier (stub → NULL)"),
            ("_container_system_group_path_for_identifier", shim_null2_stub as *const () as u64, "container_system_group_path_for_identifier (stub → NULL)"),
            ("_container_query_create", shim_null_stub as *const () as u64, "container_query_create (stub → NULL)"),
            ("_container_query_get_single_result", shim_null_stub as *const () as u64, "container_query_get_single_result (stub → NULL)"),
            ("_container_free", shim_void1_stub as *const () as u64, "container_free (stub → void)"),
        );

        // ── NStatManager (function stubs, constants in DataSymbolStore) ─
        register_shims!(table,
            ("_NStatManagerCreate", shim_null2_stub as *const () as u64, "NStatManagerCreate (stub → NULL)"),
            ("_NStatManagerSetInterfaceTraceFD", shim_void2_stub as *const () as u64, "NStatManagerSetInterfaceTraceFD (stub → void)"),
            ("_NStatManagerSetFlags", shim_void2_stub as *const () as u64, "NStatManagerSetFlags (stub → void)"),
            ("_NStatManagerQueryAllSources", shim_void1_stub as *const () as u64, "NStatManagerQueryAllSources (stub → void)"),
            ("_NStatManagerQueryAllSourcesDescriptions", shim_void2_stub as *const () as u64, "NStatManagerQueryAllSourcesDescriptions (stub → void)"),
            ("_NStatManagerSetQueuePriority", shim_void2_stub as *const () as u64, "NStatManagerSetQueuePriority (stub → void)"),
            ("_NStatManagerDestroy", shim_void1_stub as *const () as u64, "NStatManagerDestroy (stub → void)"),
            ("_NStatSourceQueryDescription", shim_void1_stub as *const () as u64, "NStatSourceQueryDescription (stub → void)"),
        );

        // ── ServiceManagement (3 symbols) ───────────────────────────
        register_shims!(table,
            ("_SMAppService", shim_null_stub as *const () as u64, "SMAppService (stub → NULL)"),
            ("_SMJobBless", shim_false2_stub as *const () as u64, "SMJobBless (stub → false)"),
            ("_SMLoginItemSetEnabled", shim_false2_stub as *const () as u64, "SMLoginItemSetEnabled (stub → false)"),
        );

        // ── CoreAnimation (2 symbols) ───────────────────────────────
        register_shims!(table,
            ("_CACurrentMediaTime", shim_ca_current_media_time as *const () as u64, "CACurrentMediaTime (stub → monotonic seconds)"),
            ("_CATransform3DIdentity", &CA_IDENTITY_TRANSFORM as *const _ as u64, "CATransform3DIdentity (stub data)"),
        );

        // ── AudioToolbox (1 symbol) ─────────────────────────────────
        register_shims!(table,
            ("_AudioServicesPlaySystemSound", shim_void1_stub as *const () as u64, "AudioServicesPlaySystemSound (stub → void)"),
        );

        // ── UserNotifications (2 symbols) ───────────────────────────
        register_shims!(table,
            ("_UNUserNotificationCenter", shim_null_stub as *const () as u64, "UNUserNotificationCenter (stub → NULL)"),
            ("_UNMutableNotificationContent", shim_null_stub as *const () as u64, "UNMutableNotificationContent (stub → NULL)"),
        );

        // ── os_transaction / diagnostics (4 symbols) ────────────────
        register_shims!(table,
            ("_os_transaction_create", shim_os_transaction_create as *const () as u64, "os_transaction_create (stub → sentinel)"),
            ("__os_assert_log", shim_void2_stub as *const () as u64, "_os_assert_log (stub → void)"),
            ("__os_assumes_log", shim_void1_stub as *const () as u64, "_os_assumes_log (stub → void)"),
            ("__os_crash", shim_os_crash as *const () as u64, "_os_crash (stub → abort)"),
        );

        // ── CoreFoundation / Foundation private helpers ──────────────
        // These private symbols are imported by plutil, security, and others.
        // __os_feature_enabled_impl: feature-flag query → return 0 (disabled)
        // __NSIsNSString: type-check helper → return 0 (safe default)
        // __CFGetProgname: return the process name as a C string
        // __CFBundleCopyInfoDictionaryForExecutableFileData: return NULL
        register_shims!(table,
            ("__os_feature_enabled_impl", shim_os_feature_enabled as *const () as u64, "_os_feature_enabled_impl (stub → 0/disabled)"),
            ("__NSIsNSString", shim_ns_is_ns_string as *const () as u64, "_NSIsNSString (stub → 0)"),
            ("__CFGetProgname", shim_cf_get_progname as *const () as u64, "_CFGetProgname (stub → \"tool\")"),
            ("__CFBundleCopyInfoDictionaryForExecutableFileData", shim_null2_stub as *const () as u64, "_CFBundleCopyInfoDictionaryForExecutableFileData (stub → NULL)"),
        );

        log::debug!("MiscStubs: registered Carbon/sysmon/DataDetectors/container/NStatManager/SM/CA/Audio/UN/os/CF stubs");
    }
}

// ── Generic stub functions ──────────────────────────────────────────

pub extern "C" fn shim_void_stub() {}
pub extern "C" fn shim_void1_stub(_a: u64) {}
pub extern "C" fn shim_void2_stub(_a: u64, _b: u64) {}
pub extern "C" fn shim_null_stub(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_null2_stub(_a: u64, _b: u64) -> u64 { 0 }
pub extern "C" fn shim_null3_stub(_a: u64, _b: u64, _c: u64) -> u64 { 0 }
pub extern "C" fn shim_zero_stub(_a: u64) -> u64 { 0 }
pub extern "C" fn shim_false_stub(_a: u64) -> i32 { 0 }
pub extern "C" fn shim_false2_stub(_a: u64, _b: u64) -> i32 { 0 }
pub extern "C" fn shim_err_stub(_a: u64) -> i32 { -1 }
pub extern "C" fn shim_one_f64_stub() -> f64 { 1.0 }

/// DDResultGetRange returns CFRange {location, length} = {0, 0}
#[repr(C)]
pub struct CfRange { pub location: u64, pub length: u64 }
pub extern "C" fn shim_zero_range_stub(_a: u64) -> CfRange { CfRange { location: 0, length: 0 } }

/// CACurrentMediaTime → monotonic seconds as f64
pub extern "C" fn shim_ca_current_media_time() -> f64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 / 1_000_000_000.0
}

/// CATransform3DIdentity — 4×4 identity matrix (16 × f64)
static CA_IDENTITY_TRANSFORM: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// os_transaction_create — synthetic non-NULL token
static OS_TRANSACTION_SENTINEL: u64 = 0x0573_AC71;

pub extern "C" fn shim_os_transaction_create(_description: u64) -> u64 {
    &OS_TRANSACTION_SENTINEL as *const u64 as u64
}

pub extern "C" fn shim_os_crash(_msg: u64) {
    log::error!("_os_crash called by guest binary");
    std::process::abort();
}

// ── CoreFoundation / Foundation private helper stubs ───────────────

/// _os_feature_enabled_impl(category, feature, default) → int
/// Returns 0 (feature disabled). Binaries that use this typically guard
/// optional behaviour, so returning 0 is the safe conservative choice.
pub extern "C" fn shim_os_feature_enabled(_cat: u64, _feat: u64, _default: i32) -> i32 {
    0
}

/// _NSIsNSString(cls) → BOOL
/// Type-check helper used by some CF internals. Return 0 (false) so the
/// caller falls back to the generic code path.
pub extern "C" fn shim_ns_is_ns_string(_cls: u64) -> i32 {
    0
}

/// _CFGetProgname() → const char *
/// Returns the process name as a null-terminated C string.  Callers only
/// use this for logging/display, so a generic string is safe.
pub extern "C" fn shim_cf_get_progname() -> u64 {
    static NAME: &[u8] = b"darwin-compat\0";
    NAME.as_ptr() as u64
}
