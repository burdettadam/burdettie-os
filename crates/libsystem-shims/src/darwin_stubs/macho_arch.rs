//! NXArchInfo shims for mach-o architecture introspection.
//!
//! macOS provides `NXGetLocalArchInfo`, `NXGetArchInfoFromCpuType`, etc.
//! in `<mach-o/arch.h>` (libmacho).  On Linux we provide static tables
//! so tools like `arch` can identify the running architecture.

use std::ffi::c_char;

use crate::table::ShimTable;

// ── Apple CPU type constants (from <mach/machine.h>) ───────────────

const CPU_TYPE_X86_64: i32 = 0x0100_0007; // CPU_TYPE_X86 | CPU_ARCH_ABI64
const CPU_TYPE_ARM64: i32 = 0x0100_000C; // CPU_TYPE_ARM | CPU_ARCH_ABI64

const CPU_SUBTYPE_X86_64_ALL: i32 = 3;
const CPU_SUBTYPE_ARM64_ALL: i32 = 0;

// ── NXArchInfo layout ──────────────────────────────────────────────
//
// struct NXArchInfo {
//     const char *name;           // offset 0
//     cpu_type_t  cputype;        // offset 8   (i32)
//     cpu_subtype_t cpusubtype;   // offset 12  (i32)
//     const char *description;    // offset 16
// };

#[repr(C)]
struct NXArchInfo {
    name: *const c_char,
    cputype: i32,
    cpusubtype: i32,
    description: *const c_char,
}

// SAFETY: static data, read-only after init.
unsafe impl Sync for NXArchInfo {}

static ARM64_INFO: NXArchInfo = NXArchInfo {
    name: b"arm64\0".as_ptr() as *const c_char,
    cputype: CPU_TYPE_ARM64,
    cpusubtype: CPU_SUBTYPE_ARM64_ALL,
    description: b"arm64\0".as_ptr() as *const c_char,
};

static X86_64_INFO: NXArchInfo = NXArchInfo {
    name: b"x86_64\0".as_ptr() as *const c_char,
    cputype: CPU_TYPE_X86_64,
    cpusubtype: CPU_SUBTYPE_X86_64_ALL,
    description: b"x86_64\0".as_ptr() as *const c_char,
};

pub struct MachoArchStubs;

impl MachoArchStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_NXGetLocalArchInfo",
                shim_nx_get_local_arch_info as *const () as u64,
                "NXGetLocalArchInfo → arm64"
            ),
            (
                "_NXGetArchInfoFromCpuType",
                shim_nx_get_arch_info_from_cpu_type as *const () as u64,
                "NXGetArchInfoFromCpuType lookup"
            ),
            (
                "_NXGetArchInfoFromName",
                shim_nx_get_arch_info_from_name as *const () as u64,
                "NXGetArchInfoFromName lookup"
            ),
            (
                "_NXGetAllArchInfos",
                shim_nx_get_all_arch_infos as *const () as u64,
                "NXGetAllArchInfos → static table"
            )
        );
        log::debug!("MachoArchStubs: registered 4 stubs");
    }
}

/// `const NXArchInfo *NXGetLocalArchInfo(void)`
///
/// Returns the architecture of the running process.
pub extern "C" fn shim_nx_get_local_arch_info() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        &ARM64_INFO as *const NXArchInfo as u64
    }
    #[cfg(target_arch = "x86_64")]
    {
        &X86_64_INFO as *const NXArchInfo as u64
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        0
    }
}

/// `const NXArchInfo *NXGetArchInfoFromCpuType(cpu_type_t cputype, cpu_subtype_t cpusubtype)`
pub extern "C" fn shim_nx_get_arch_info_from_cpu_type(cputype: i32, _cpusubtype: i32) -> u64 {
    match cputype {
        CPU_TYPE_ARM64 => &ARM64_INFO as *const NXArchInfo as u64,
        CPU_TYPE_X86_64 => &X86_64_INFO as *const NXArchInfo as u64,
        _ => 0, // NULL — unknown architecture
    }
}

/// `const NXArchInfo *NXGetArchInfoFromName(const char *name)`
pub extern "C" fn shim_nx_get_arch_info_from_name(name: u64) -> u64 {
    if name == 0 {
        return 0;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(name as *const c_char) };
    match c_str.to_bytes() {
        b"arm64" | b"aarch64" => &ARM64_INFO as *const NXArchInfo as u64,
        b"x86_64" => &X86_64_INFO as *const NXArchInfo as u64,
        _ => 0,
    }
}

// Static null-terminated table: [arm64, x86_64, {null sentinel}]
static ARCH_TABLE: [NXArchInfo; 3] = [
    NXArchInfo {
        name: b"arm64\0".as_ptr() as *const c_char,
        cputype: CPU_TYPE_ARM64,
        cpusubtype: CPU_SUBTYPE_ARM64_ALL,
        description: b"arm64\0".as_ptr() as *const c_char,
    },
    NXArchInfo {
        name: b"x86_64\0".as_ptr() as *const c_char,
        cputype: CPU_TYPE_X86_64,
        cpusubtype: CPU_SUBTYPE_X86_64_ALL,
        description: b"x86_64\0".as_ptr() as *const c_char,
    },
    // Sentinel: name == NULL terminates the list
    NXArchInfo {
        name: std::ptr::null(),
        cputype: 0,
        cpusubtype: 0,
        description: std::ptr::null(),
    },
];

/// `const NXArchInfo *NXGetAllArchInfos(void)`
///
/// Returns a pointer to a null-terminated array of NXArchInfo.
pub extern "C" fn shim_nx_get_all_arch_infos() -> u64 {
    ARCH_TABLE.as_ptr() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_arch_info_returns_non_null() {
        let ptr = shim_nx_get_local_arch_info();
        assert_ne!(ptr, 0);
        let info = unsafe { &*(ptr as *const NXArchInfo) };
        let name = unsafe { std::ffi::CStr::from_ptr(info.name) };
        #[cfg(target_arch = "aarch64")]
        assert_eq!(name.to_bytes(), b"arm64");
        #[cfg(target_arch = "x86_64")]
        assert_eq!(name.to_bytes(), b"x86_64");
    }

    #[test]
    fn cpu_type_lookup_arm64() {
        let ptr = shim_nx_get_arch_info_from_cpu_type(CPU_TYPE_ARM64, CPU_SUBTYPE_ARM64_ALL);
        assert_ne!(ptr, 0);
        let info = unsafe { &*(ptr as *const NXArchInfo) };
        assert_eq!(info.cputype, CPU_TYPE_ARM64);
    }

    #[test]
    fn cpu_type_lookup_unknown_returns_null() {
        let ptr = shim_nx_get_arch_info_from_cpu_type(999, 0);
        assert_eq!(ptr, 0);
    }

    #[test]
    fn name_lookup() {
        let ptr = shim_nx_get_arch_info_from_name(b"arm64\0".as_ptr() as u64);
        assert_ne!(ptr, 0);
        let ptr2 = shim_nx_get_arch_info_from_name(b"unknown\0".as_ptr() as u64);
        assert_eq!(ptr2, 0);
    }

    #[test]
    fn all_arch_infos_has_sentinel() {
        let ptr = shim_nx_get_all_arch_infos();
        assert_ne!(ptr, 0);
        let table = ptr as *const NXArchInfo;
        // Third entry should be the null sentinel
        let sentinel = unsafe { &*table.add(2) };
        assert!(sentinel.name.is_null());
    }
}
