//! Apple BSM (Basic Security Module) audit token introspection stubs.
//!
//! These are used by XPC, sandbox, and security framework code to check
//! process identity via `audit_token_t`. The token is a struct of 8
//! `u32` fields:
//!
//!   \[0\]=auid, \[1\]=euid, \[2\]=egid, \[3\]=ruid, \[4\]=rgid,
//!   \[5\]=pid, \[6\]=asid, \[7\]=pidversion
//!
//! Our shims accept the token as a `u64` pointer to the 32-byte struct.

use crate::table::ShimTable;

pub struct AuditStubs;

impl AuditStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_audit_token_to_pid", shim_audit_token_to_pid as u64, "audit_token_to_pid → field[5] or getpid()"),
            ("_audit_token_to_euid", shim_audit_token_to_euid as u64, "audit_token_to_euid → field[1] or geteuid()"),
            ("_audit_token_to_egid", shim_audit_token_to_egid as u64, "audit_token_to_egid → field[2] or getegid()"),
            ("_audit_token_to_ruid", shim_audit_token_to_ruid as u64, "audit_token_to_ruid → field[3] or getuid()"),
            ("_audit_token_to_rgid", shim_audit_token_to_rgid as u64, "audit_token_to_rgid → field[4] or getgid()"),
            ("_audit_token_to_auid", shim_audit_token_to_auid as u64, "audit_token_to_auid → field[0] or getuid()"),
            ("_audit_token_to_asid", shim_audit_token_to_asid as u64, "audit_token_to_asid → field[6] or getpid()"),
            ("_audit_token_to_au32", shim_audit_token_to_au32 as u64, "audit_token_to_au32 → unpack all 8 fields"),
            ("_audit_token_to_pidversion", shim_audit_token_to_pidversion as u64, "audit_token_to_pidversion → field[7] or 0"),
            ("_audit_session_self", shim_audit_session_self as u64, "audit_session_self → 0"),
            ("_audit_session_join", shim_audit_session_join as u64, "audit_session_join → 0")
        );
        log::debug!("AuditStubs: registered 11 stubs");
    }
}

// ── Helper ──────────────────────────────────────────────────────

/// Read a `u32` field from an `audit_token_t` at the given field index (0–7).
unsafe fn read_token_field(token: u64, index: usize) -> u32 {
    let ptr = token as *const u32;
    if ptr.is_null() {
        return 0;
    }
    unsafe { *ptr.add(index) }
}

// ── Shim functions ──────────────────────────────────────────────

/// `audit_token_to_pid(token) → pid` — field\[5\] (offset 20).
pub extern "C" fn shim_audit_token_to_pid(token: u64) -> i32 {
    if token == 0 {
        return unsafe { libc::getpid() };
    }
    unsafe { read_token_field(token, 5) as i32 }
}

/// `audit_token_to_euid(token) → euid` — field\[1\] (offset 4).
pub extern "C" fn shim_audit_token_to_euid(token: u64) -> u32 {
    if token == 0 {
        return unsafe { libc::geteuid() };
    }
    unsafe { read_token_field(token, 1) }
}

/// `audit_token_to_egid(token) → egid` — field\[2\] (offset 8).
pub extern "C" fn shim_audit_token_to_egid(token: u64) -> u32 {
    if token == 0 {
        return unsafe { libc::getegid() };
    }
    unsafe { read_token_field(token, 2) }
}

/// `audit_token_to_ruid(token) → ruid` — field\[3\] (offset 12).
pub extern "C" fn shim_audit_token_to_ruid(token: u64) -> u32 {
    if token == 0 {
        return unsafe { libc::getuid() };
    }
    unsafe { read_token_field(token, 3) }
}

/// `audit_token_to_rgid(token) → rgid` — field\[4\] (offset 16).
pub extern "C" fn shim_audit_token_to_rgid(token: u64) -> u32 {
    if token == 0 {
        return unsafe { libc::getgid() };
    }
    unsafe { read_token_field(token, 4) }
}

/// `audit_token_to_auid(token) → auid` — field\[0\] (offset 0).
pub extern "C" fn shim_audit_token_to_auid(token: u64) -> u32 {
    if token == 0 {
        return unsafe { libc::getuid() };
    }
    unsafe { read_token_field(token, 0) }
}

/// `audit_token_to_asid(token) → asid` — field\[6\] (offset 24).
pub extern "C" fn shim_audit_token_to_asid(token: u64) -> i32 {
    if token == 0 {
        return unsafe { libc::getpid() };
    }
    unsafe { read_token_field(token, 6) as i32 }
}

/// `audit_token_to_au32` — unpack all 8 fields into individual output pointers.
///
/// Each output pointer is a `*mut u32` (passed as `u64`). If non-null the
/// corresponding field is written. If the token pointer itself is null,
/// current-process values are used as defaults.
pub extern "C" fn shim_audit_token_to_au32(
    token: u64,
    auid_out: u64,
    euid_out: u64,
    egid_out: u64,
    ruid_out: u64,
    rgid_out: u64,
    pid_out: u64,
    asid_out: u64,
    pidversion_out: u64,
) {
    let vals: [u32; 8] = if token == 0 {
        unsafe {
            [
                libc::getuid(),                // auid
                libc::geteuid(),               // euid
                libc::getegid(),               // egid
                libc::getuid(),                // ruid
                libc::getgid(),                // rgid
                libc::getpid() as u32,         // pid
                libc::getpid() as u32,         // asid (approximation)
                0,                             // pidversion
            ]
        }
    } else {
        unsafe {
            [
                read_token_field(token, 0),
                read_token_field(token, 1),
                read_token_field(token, 2),
                read_token_field(token, 3),
                read_token_field(token, 4),
                read_token_field(token, 5),
                read_token_field(token, 6),
                read_token_field(token, 7),
            ]
        }
    };

    let ptrs = [
        auid_out, euid_out, egid_out, ruid_out,
        rgid_out, pid_out, asid_out, pidversion_out,
    ];
    for (i, &p) in ptrs.iter().enumerate() {
        if p != 0 {
            unsafe { *(p as *mut u32) = vals[i]; }
        }
    }
}

/// `audit_token_to_pidversion(token) → pidversion` — field\[7\] (offset 28).
pub extern "C" fn shim_audit_token_to_pidversion(token: u64) -> i32 {
    if token == 0 {
        return 0;
    }
    unsafe { read_token_field(token, 7) as i32 }
}

/// `audit_session_self()` — returns 0 (no real audit session).
pub extern "C" fn shim_audit_session_self() -> u64 {
    0
}

/// `audit_session_join(port)` — returns 0 (success).
pub extern "C" fn shim_audit_session_join(_port: u64) -> i32 {
    0
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_token_to_pid() {
        let token: [u32; 8] = [100, 501, 20, 501, 20, 12345, 1000, 42];
        let token_ptr = token.as_ptr() as u64;
        assert_eq!(shim_audit_token_to_pid(token_ptr), 12345);
    }

    #[test]
    fn test_audit_token_to_euid() {
        let token: [u32; 8] = [100, 501, 20, 501, 20, 12345, 1000, 42];
        let token_ptr = token.as_ptr() as u64;
        assert_eq!(shim_audit_token_to_euid(token_ptr), 501);
    }

    #[test]
    fn test_null_token_returns_current_pid() {
        let pid = shim_audit_token_to_pid(0);
        let expected = unsafe { libc::getpid() };
        assert_eq!(pid, expected);
    }

    #[test]
    fn test_null_token_returns_current_euid() {
        let euid = shim_audit_token_to_euid(0);
        let expected = unsafe { libc::geteuid() };
        assert_eq!(euid, expected);
    }

    #[test]
    fn test_audit_token_to_au32_unpacks_all() {
        let token: [u32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
        let token_ptr = token.as_ptr() as u64;

        let mut auid: u32 = 0;
        let mut euid: u32 = 0;
        let mut egid: u32 = 0;
        let mut ruid: u32 = 0;
        let mut rgid: u32 = 0;
        let mut pid: u32 = 0;
        let mut asid: u32 = 0;
        let mut pidversion: u32 = 0;

        shim_audit_token_to_au32(
            token_ptr,
            &mut auid as *mut u32 as u64,
            &mut euid as *mut u32 as u64,
            &mut egid as *mut u32 as u64,
            &mut ruid as *mut u32 as u64,
            &mut rgid as *mut u32 as u64,
            &mut pid as *mut u32 as u64,
            &mut asid as *mut u32 as u64,
            &mut pidversion as *mut u32 as u64,
        );

        assert_eq!(auid, 10);
        assert_eq!(euid, 20);
        assert_eq!(egid, 30);
        assert_eq!(ruid, 40);
        assert_eq!(rgid, 50);
        assert_eq!(pid, 60);
        assert_eq!(asid, 70);
        assert_eq!(pidversion, 80);
    }

    #[test]
    fn test_audit_session_self() {
        assert_eq!(shim_audit_session_self(), 0);
    }

    #[test]
    fn test_pidversion_from_token() {
        let token: [u32; 8] = [0, 0, 0, 0, 0, 0, 0, 99];
        let token_ptr = token.as_ptr() as u64;
        assert_eq!(shim_audit_token_to_pidversion(token_ptr), 99);
    }
}
