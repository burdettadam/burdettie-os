//! libsystem_info user/group lookup shims.
//!
//! On macOS these resolve via dlsym but on Linux the directory service
//! integration differs.  We provide a shim layer that normalizes behavior
//! by delegating to the corresponding libc functions.

use std::ffi::c_char;

use crate::table::ShimTable;

pub struct SysInfoStubs;

impl SysInfoStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_getpwnam", shim_getpwnam as *const () as u64, "getpwnam user lookup by name"),
            ("_getpwuid", shim_getpwuid as *const () as u64, "getpwuid user lookup by uid"),
            ("_getpwnam_r", shim_getpwnam_r as *const () as u64, "getpwnam_r reentrant user lookup by name"),
            ("_getpwuid_r", shim_getpwuid_r as *const () as u64, "getpwuid_r reentrant user lookup by uid"),
            ("_getpwent", shim_getpwent as *const () as u64, "getpwent enumerate passwd entries"),
            ("_setpwent", shim_setpwent as *const () as u64, "setpwent rewind passwd enumeration"),
            ("_endpwent", shim_endpwent as *const () as u64, "endpwent close passwd enumeration"),
            ("_getgrnam", shim_getgrnam as *const () as u64, "getgrnam group lookup by name"),
            ("_getgrgid", shim_getgrgid as *const () as u64, "getgrgid group lookup by gid"),
            ("_getgrnam_r", shim_getgrnam_r as *const () as u64, "getgrnam_r reentrant group lookup by name"),
            ("_getgrgid_r", shim_getgrgid_r as *const () as u64, "getgrgid_r reentrant group lookup by gid"),
            ("_getgrent", shim_getgrent as *const () as u64, "getgrent enumerate group entries"),
            ("_setgrent", shim_setgrent as *const () as u64, "setgrent rewind group enumeration"),
            ("_endgrent", shim_endgrent as *const () as u64, "endgrent close group enumeration"),
            ("_gethostname", shim_gethostname as *const () as u64, "gethostname get system hostname"),
            ("_sethostname", shim_sethostname as *const () as u64, "sethostname stub (returns EPERM)"),
            ("_getgroups", shim_getgroups as *const () as u64, "getgroups get supplementary group IDs"),
            ("_getgrouplist_2", shim_getgrouplist_2 as *const () as u64, "getgrouplist_2 macOS extended group list"),
            ("_getaudit_addr", shim_getaudit_addr as *const () as u64, "getaudit_addr stub (returns ENOSYS)"),
        );
        log::debug!("SysInfoStubs: registered 19 stubs");
    }
}

// ---------------------------------------------------------------------------
// User lookups
// ---------------------------------------------------------------------------

pub extern "C" fn shim_getpwnam(name: u64) -> u64 {
    unsafe { libc::getpwnam(name as *const c_char) as u64 }
}

pub extern "C" fn shim_getpwuid(uid: u32) -> u64 {
    unsafe { libc::getpwuid(uid) as u64 }
}

pub extern "C" fn shim_getpwnam_r(
    name: u64,
    pwd: u64,
    buf: u64,
    bufsize: usize,
    result: u64,
) -> i32 {
    unsafe {
        libc::getpwnam_r(
            name as *const c_char,
            pwd as *mut libc::passwd,
            buf as *mut c_char,
            bufsize,
            result as *mut *mut libc::passwd,
        )
    }
}

pub extern "C" fn shim_getpwuid_r(
    uid: u32,
    pwd: u64,
    buf: u64,
    bufsize: usize,
    result: u64,
) -> i32 {
    unsafe {
        libc::getpwuid_r(
            uid,
            pwd as *mut libc::passwd,
            buf as *mut c_char,
            bufsize,
            result as *mut *mut libc::passwd,
        )
    }
}

pub extern "C" fn shim_getpwent() -> u64 {
    unsafe { libc::getpwent() as u64 }
}

pub extern "C" fn shim_setpwent() {
    unsafe { libc::setpwent() }
}

pub extern "C" fn shim_endpwent() {
    unsafe { libc::endpwent() }
}

// ---------------------------------------------------------------------------
// Group lookups
// ---------------------------------------------------------------------------

pub extern "C" fn shim_getgrnam(name: u64) -> u64 {
    unsafe { libc::getgrnam(name as *const c_char) as u64 }
}

pub extern "C" fn shim_getgrgid(gid: u32) -> u64 {
    unsafe { libc::getgrgid(gid) as u64 }
}

pub extern "C" fn shim_getgrnam_r(
    name: u64,
    grp: u64,
    buf: u64,
    bufsize: usize,
    result: u64,
) -> i32 {
    unsafe {
        libc::getgrnam_r(
            name as *const c_char,
            grp as *mut libc::group,
            buf as *mut c_char,
            bufsize,
            result as *mut *mut libc::group,
        )
    }
}

pub extern "C" fn shim_getgrgid_r(
    gid: u32,
    grp: u64,
    buf: u64,
    bufsize: usize,
    result: u64,
) -> i32 {
    unsafe {
        libc::getgrgid_r(
            gid,
            grp as *mut libc::group,
            buf as *mut c_char,
            bufsize,
            result as *mut *mut libc::group,
        )
    }
}

pub extern "C" fn shim_getgrent() -> u64 {
    unsafe { libc::getgrent() as u64 }
}

pub extern "C" fn shim_setgrent() {
    unsafe { libc::setgrent() }
}

pub extern "C" fn shim_endgrent() {
    unsafe { libc::endgrent() }
}

// ---------------------------------------------------------------------------
// Hostname
// ---------------------------------------------------------------------------

pub extern "C" fn shim_gethostname(name: u64, len: usize) -> i32 {
    unsafe { libc::gethostname(name as *mut c_char, len) }
}

pub extern "C" fn shim_sethostname(_name: u64, _len: usize) -> i32 {
    #[cfg(target_os = "macos")]
    unsafe { *libc::__error() = libc::EPERM };
    #[cfg(not(target_os = "macos"))]
    unsafe { *libc::__errno_location() = libc::EPERM };
    -1
}

// ---------------------------------------------------------------------------
// Supplementary groups
// ---------------------------------------------------------------------------

/// `int getgroups(int gidsetsize, gid_t grouplist[])`
pub extern "C" fn shim_getgroups(gidsetsize: i32, grouplist: u64) -> i32 {
    unsafe { libc::getgroups(gidsetsize, grouplist as *mut libc::gid_t) }
}

/// `int getgrouplist_2(const char *name, gid_t basegid, gid_t **groups)`
///
/// macOS-specific: allocates and returns all groups for user.
/// We implement using Linux `getgrouplist` + malloc.
pub extern "C" fn shim_getgrouplist_2(name: u64, basegid: u32, groups_out: u64) -> i32 {
    if name == 0 || groups_out == 0 {
        return -1;
    }
    // On macOS, getgrouplist uses c_int for groups; on Linux it uses gid_t.
    // We handle this with cfg to satisfy both platforms.
    #[cfg(target_os = "macos")]
    {
        let mut ngroups: libc::c_int = 64;
        let mut buf: Vec<libc::c_int> = vec![0; ngroups as usize];
        let rc = unsafe {
            libc::getgrouplist(name as *const c_char, basegid as libc::c_int, buf.as_mut_ptr(), &mut ngroups)
        };
        if rc < 0 {
            buf.resize(ngroups as usize, 0);
            let rc2 = unsafe {
                libc::getgrouplist(name as *const c_char, basegid as libc::c_int, buf.as_mut_ptr(), &mut ngroups)
            };
            if rc2 < 0 { return -1; }
        }
        let alloc_size = (ngroups as usize) * std::mem::size_of::<u32>();
        let ptr = unsafe { libc::malloc(alloc_size) };
        if ptr.is_null() { return -1; }
        // Copy as u32 (gid_t) for the macOS ABI caller
        let out = ptr as *mut u32;
        for i in 0..ngroups as usize {
            unsafe { *out.add(i) = buf[i] as u32; }
        }
        unsafe { *(groups_out as *mut u64) = ptr as u64; }
        ngroups
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut ngroups: libc::c_int = 64;
        let mut buf: Vec<libc::gid_t> = vec![0; ngroups as usize];
        let rc = unsafe {
            libc::getgrouplist(name as *const c_char, basegid as libc::gid_t, buf.as_mut_ptr(), &mut ngroups)
        };
        if rc < 0 {
            buf.resize(ngroups as usize, 0);
            let rc2 = unsafe {
                libc::getgrouplist(name as *const c_char, basegid as libc::gid_t, buf.as_mut_ptr(), &mut ngroups)
            };
            if rc2 < 0 { return -1; }
        }
        let alloc_size = (ngroups as usize) * std::mem::size_of::<libc::gid_t>();
        let ptr = unsafe { libc::malloc(alloc_size) };
        if ptr.is_null() { return -1; }
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr() as *const u8, ptr as *mut u8, alloc_size);
            *(groups_out as *mut u64) = ptr as u64;
        }
        ngroups
    }
}

// ---------------------------------------------------------------------------
// BSM Audit (stub)
// ---------------------------------------------------------------------------

/// `int getaudit_addr(auditinfo_addr_t *info, int length)`
///
/// BSM audit not available on Linux — return ENOSYS.
pub extern "C" fn shim_getaudit_addr(_info: u64, _length: i32) -> i32 {
    unsafe {
        #[cfg(target_os = "macos")]
        { *libc::__error() = libc::ENOSYS; }
        #[cfg(not(target_os = "macos"))]
        { *libc::__errno_location() = libc::ENOSYS; }
    }
    -1
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getpwuid_root() {
        let result = shim_getpwuid(0);
        assert_ne!(result, 0, "getpwuid(0) should return non-null for root");
    }

    #[test]
    fn test_getpwnam_root() {
        let name = b"root\0".as_ptr() as u64;
        let result = shim_getpwnam(name);
        assert_ne!(result, 0, "getpwnam(\"root\") should return non-null");
    }

    #[test]
    fn test_getgrgid_zero() {
        let result = shim_getgrgid(0);
        assert_ne!(result, 0, "getgrgid(0) should return non-null (root/wheel)");
    }

    #[test]
    fn test_gethostname() {
        let mut buf = [0u8; 256];
        let rc = shim_gethostname(buf.as_mut_ptr() as u64, buf.len());
        assert_eq!(rc, 0, "gethostname should succeed");
        let hostname = unsafe {
            std::ffi::CStr::from_ptr(buf.as_ptr() as *const c_char)
        };
        assert!(!hostname.to_bytes().is_empty(), "hostname should be non-empty");
    }

    #[test]
    fn test_endpwent_no_crash() {
        shim_endpwent();
    }

    #[test]
    fn test_getpwuid_r_root() {
        let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut buf = [0u8; 4096];
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        let rc = shim_getpwuid_r(
            0,
            &mut pwd as *mut libc::passwd as u64,
            buf.as_mut_ptr() as u64,
            buf.len(),
            &mut result as *mut *mut libc::passwd as u64,
        );
        assert_eq!(rc, 0, "getpwuid_r for uid 0 should return 0");
        assert!(!result.is_null(), "result pointer should be non-null for root");
    }
}
