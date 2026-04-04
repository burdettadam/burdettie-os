//! macOS sandbox stubs.
//!
//! Stubs report "no sandbox" so all operations appear permitted.

use crate::table::ShimTable;

/// Synthetic token returned by `sandbox_extension_issue_*` stubs.
static MOCK_TOKEN: &[u8] = b"com.apple.sandbox.token.mock\0";

pub struct SandboxStubs;

impl SandboxStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_sandbox_check", shim_sandbox_check as *const () as u64, "sandbox_check (stub → 0, not sandboxed)"),
            ("_sandbox_extension_consume", shim_sandbox_extension_consume as *const () as u64, "sandbox_extension_consume (stub → 0)"),
            ("_sandbox_init", shim_sandbox_init as *const () as u64, "sandbox_init (stub → 0, accepted but not enforced)"),
            ("_sandbox_init_with_parameters", shim_sandbox_init_with_parameters as *const () as u64, "sandbox_init_with_parameters (stub → 0)"),
            ("_sandbox_free_error", shim_sandbox_free_error as *const () as u64, "sandbox_free_error (no-op)"),
            ("_sandbox_extension_issue_file", shim_sandbox_extension_issue_file as *const () as u64, "sandbox_extension_issue_file (stub → mock token)"),
            ("_sandbox_extension_issue_mach", shim_sandbox_extension_issue_mach as *const () as u64, "sandbox_extension_issue_mach (stub → mock token)"),
            ("_sandbox_extension_issue_generic", shim_sandbox_extension_issue_generic as *const () as u64, "sandbox_extension_issue_generic (stub → mock token)"),
            ("_sandbox_extension_release", shim_sandbox_extension_release as *const () as u64, "sandbox_extension_release (stub → 0)"),
            ("_sandbox_check_by_audit_token", shim_sandbox_check_by_audit_token as *const () as u64, "sandbox_check_by_audit_token (stub → 0, allowed)"),
            ("_sandbox_apply", shim_sandbox_apply as *const () as u64, "sandbox_apply (stub → 0, not enforced)"),
        );
        log::debug!("SandboxStubs: registered 11 stubs");
    }
}

/// Returns 0 = operation allowed (not sandboxed).
pub extern "C" fn shim_sandbox_check(_pid: i32, _operation: u64, _type: i32) -> i32 { 0 }

/// Returns 0 = success (extension consumed).
pub extern "C" fn shim_sandbox_extension_consume(_token: u64) -> i32 { 0 }

/// Returns 0 = success. Profile accepted but not enforced.
/// If `errorbuf` is non-null, clears it to null (no error).
pub extern "C" fn shim_sandbox_init(_profile: u64, _flags: u64, errorbuf: u64) -> i32 {
    if errorbuf != 0 {
        unsafe { *(errorbuf as *mut u64) = 0; }
    }
    0
}

/// Returns 0 = success. Same semantics as `sandbox_init`.
pub extern "C" fn shim_sandbox_init_with_parameters(_profile: u64, _flags: u64, _params: u64, errorbuf: u64) -> i32 {
    if errorbuf != 0 {
        unsafe { *(errorbuf as *mut u64) = 0; }
    }
    0
}

/// No-op — we never allocate error strings.
pub extern "C" fn shim_sandbox_free_error(_errorbuf: u64) {}

/// Returns a pointer to a static mock token string.
pub extern "C" fn shim_sandbox_extension_issue_file(_type: u64, _path: u64, _flags: u32) -> u64 {
    MOCK_TOKEN.as_ptr() as u64
}

/// Returns a pointer to a static mock token string.
pub extern "C" fn shim_sandbox_extension_issue_mach(_type: u64, _name: u64, _flags: u32) -> u64 {
    MOCK_TOKEN.as_ptr() as u64
}

/// Returns a pointer to a static mock token string.
pub extern "C" fn shim_sandbox_extension_issue_generic(_type: u64, _flags: u32) -> u64 {
    MOCK_TOKEN.as_ptr() as u64
}

/// Returns 0 = success (handle released).
pub extern "C" fn shim_sandbox_extension_release(_handle: i64) -> i32 { 0 }

/// Returns 0 = operation allowed (not sandboxed).
pub extern "C" fn shim_sandbox_check_by_audit_token(_token: u64, _operation: u64, _type: i32) -> i32 { 0 }

/// Returns 0 = success (sandbox "applied" but not enforced).
pub extern "C" fn shim_sandbox_apply(_container: u64) -> i32 { 0 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_check_allows() {
        assert_eq!(shim_sandbox_check(1, 0, 0), 0);
    }

    #[test]
    fn test_sandbox_init_succeeds() {
        assert_eq!(shim_sandbox_init(0, 0, 0), 0);
    }

    #[test]
    fn test_sandbox_init_clears_error() {
        let mut errptr: u64 = 0xDEAD;
        let errorbuf = &mut errptr as *mut u64 as u64;
        assert_eq!(shim_sandbox_init(0, 0, errorbuf), 0);
        assert_eq!(errptr, 0, "errorbuf should be cleared to null");
    }

    #[test]
    fn test_extension_issue_returns_token() {
        let token = shim_sandbox_extension_issue_file(0, 0, 0);
        assert_ne!(token, 0, "should return a non-null token pointer");
    }

    #[test]
    fn test_extension_release_succeeds() {
        assert_eq!(shim_sandbox_extension_release(1), 0);
    }

    #[test]
    fn test_sandbox_apply_succeeds() {
        assert_eq!(shim_sandbox_apply(0), 0);
    }
}
