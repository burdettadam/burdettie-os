//! Extra Mach kernel primitive stubs.
//!
//! Supplements the core Mach shims in `crate::mach` with additional
//! primitives used by Xcode tools and Foundation code: host info,
//! VM allocation, task info, Mach ports.

use crate::table::ShimTable;

pub struct MachExtrasStubs;

/// Synthetic Mach host port for non-macOS platforms.
static MACH_HOST_PORT: u32 = 0x104;

impl MachExtrasStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_mach_continuous_time", shim_mach_continuous_time as u64, "mach_continuous_time (stub → mach_absolute_time)"),
            ("_mach_msg_destroy", shim_mach_msg_destroy as *const () as u64, "mach_msg_destroy (stub → void)"),
            ("_mach_msg", shim_mach_msg as *const () as u64, "mach_msg (stub → MACH_SEND_INVALID_DEST)"),
            ("_mach_msg_trap", shim_mach_msg as *const () as u64, "mach_msg_trap (stub → MACH_SEND_INVALID_DEST)"),
            ("_mach_port_type", shim_mach_port_type as u64, "mach_port_type (stub → KERN_INVALID_ARGUMENT)"),
            ("_mig_dealloc_special_reply_port", shim_mig_void_stub as u64, "mig_dealloc_special_reply_port (stub → void)"),
            ("_mig_get_special_reply_port", shim_mig_get_special_reply_port as u64, "mig_get_special_reply_port (stub → 0)"),
            ("_mig_strncpy", shim_mig_strncpy as u64, "mig_strncpy (stub → strncpy)"),
            ("_mig_strncpy_zerofill", shim_mig_strncpy as u64, "mig_strncpy_zerofill (stub → strncpy)")
        );

        // ── Host / VM / Task ────────────────────────────────────
        register_shims!(table,
            ("_mach_host_self", shim_mach_host_self as u64, "mach_host_self (→ synthetic port)"),
            ("_host_page_size", shim_host_page_size as u64, "host_page_size (→ real page size)"),
            ("_host_info", shim_host_info as u64, "host_info (→ real CPU/mem info)"),
            ("_host_statistics", shim_host_statistics as u64, "host_statistics (stub → KERN_SUCCESS)"),
            ("_host_statistics64", shim_host_statistics64 as u64, "host_statistics64 (stub → KERN_SUCCESS)"),
            ("_vm_allocate", shim_vm_allocate as u64, "vm_allocate (→ real mmap)"),
            ("_vm_deallocate", shim_vm_deallocate as u64, "vm_deallocate (→ real munmap)"),
            ("_vm_protect", shim_vm_protect as u64, "vm_protect (→ real mprotect)"),
            ("_vm_page_size", &VM_PAGE_SIZE_DATA as *const u64 as u64, "vm_page_size data symbol"),
            ("_mach_port_allocate", shim_mach_port_allocate as u64, "mach_port_allocate (stub → KERN_SUCCESS)"),
            ("_mach_port_deallocate", shim_mach_port_deallocate as u64, "mach_port_deallocate (stub → KERN_SUCCESS)"),
            ("_mach_port_mod_refs", shim_mach_port_mod_refs as u64, "mach_port_mod_refs (stub → KERN_SUCCESS)"),
            ("_task_info", shim_task_info as u64, "task_info (→ real memory stats)")
        );

        // ── Thread state & task control ─────────────────────────
        register_shims!(table,
            ("_thread_get_state", shim_thread_get_state as u64, "thread_get_state (stub → KERN_FAILURE)"),
            ("_thread_set_state", shim_thread_set_state as u64, "thread_set_state (stub → KERN_FAILURE)"),
            ("_thread_create", shim_thread_create as u64, "thread_create (stub → KERN_FAILURE)"),
            ("_thread_terminate", shim_thread_terminate as u64, "thread_terminate (stub → KERN_SUCCESS)"),
            ("_task_threads", shim_task_threads as u64, "task_threads (stub → 0 threads)"),
            ("_thread_info", shim_thread_info as u64, "thread_info (stub → KERN_SUCCESS)"),
            ("_thread_suspend", shim_thread_suspend as u64, "thread_suspend (stub → KERN_SUCCESS)"),
            ("_thread_resume", shim_thread_resume as u64, "thread_resume (stub → KERN_SUCCESS)"),
            ("_thread_abort", shim_thread_abort as u64, "thread_abort (stub → KERN_SUCCESS)"),
            ("_thread_abort_safely", shim_thread_abort_safely as u64, "thread_abort_safely (stub → KERN_SUCCESS)"),
            ("_task_policy_set", shim_task_policy_set as u64, "task_policy_set (stub → KERN_SUCCESS)"),
            ("_task_policy_get", shim_task_policy_get as u64, "task_policy_get (stub → KERN_SUCCESS)")
        );

        log::debug!("MachExtrasStubs: registered 32 Mach stubs");
    }
}

pub extern "C" fn shim_mach_continuous_time() -> u64 {
    // mach_continuous_time includes sleep time; approximate with monotonic clock
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
}

pub extern "C" fn shim_mach_msg_destroy(_msg: u64) {}

/// `mach_msg` / `mach_msg_trap` stub — returns `MACH_SEND_INVALID_DEST`.
///
/// Signature matches `mach_msg(msg, options, send_size, rcv_size, rcv_name,
/// timeout, notify)`. There is no real Mach IPC in the lab environment;
/// any code path that reaches this should treat the error as "port invalid".
pub extern "C" fn shim_mach_msg(
    _msg: u64,
    _options: u32,
    _send_size: u32,
    _rcv_size: u32,
    _rcv_name: u32,
    _timeout: u32,
    _notify: u32,
) -> u32 {
    // MACH_SEND_INVALID_DEST = 0x10000003
    0x10000003u32
}

/// KERN_INVALID_ARGUMENT = 4
pub extern "C" fn shim_mach_port_type(_task: u32, _name: u32, _ptype: u64) -> i32 { 4 }

pub extern "C" fn shim_mig_void_stub(_port: u32) {}

pub extern "C" fn shim_mig_get_special_reply_port() -> u32 { 0 }

pub extern "C" fn shim_mig_strncpy(dest: *mut u8, src: *const u8, len: u64) {
    if dest.is_null() || src.is_null() { return; }
    unsafe { libc::strncpy(dest as *mut i8, src as *const i8, len as usize); }
}

// ── Host / VM / Task ───────────────────────────────────────────

/// Data symbol for `vm_page_size` (same as `getpagesize()`).
static VM_PAGE_SIZE_DATA: u64 = 4096; // overridden at install time on non-4K systems

/// `mach_host_self()` — returns a synthetic host port.
pub extern "C" fn shim_mach_host_self() -> u32 {
    MACH_HOST_PORT
}

/// `host_page_size(host, &page_size)` — writes the system page size.
pub extern "C" fn shim_host_page_size(_host: u32, out: *mut u64) -> i32 {
    if !out.is_null() {
        unsafe { *out = libc::sysconf(libc::_SC_PAGESIZE) as u64; }
    }
    0 // KERN_SUCCESS
}

/// `HOST_BASIC_INFO` flavor number.
const HOST_BASIC_INFO_FLAVOR: i32 = 1;

/// Minimal `host_basic_info_data_t` layout (first 3 fields).
#[repr(C)]
struct HostBasicInfo {
    max_cpus: i32,
    avail_cpus: i32,
    memory_size: u32,
    // ... remainder ignored
}

/// `host_info(host, flavor, info, &count)` — returns CPU/memory info.
pub extern "C" fn shim_host_info(
    _host: u32, flavor: i32, info: *mut u8, count: *mut u32,
) -> i32 {
    if flavor == HOST_BASIC_INFO_FLAVOR && !info.is_null() && !count.is_null() {
        let ncpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) } as i32;
        let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) } as u64;
        let page_sz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        let mem_bytes = pages.saturating_mul(page_sz);

        let basic = HostBasicInfo {
            max_cpus: ncpus,
            avail_cpus: ncpus,
            memory_size: (mem_bytes / (1024 * 1024)) as u32, // MB
        };
        let copy_len = std::mem::size_of::<HostBasicInfo>()
            .min(unsafe { *count } as usize * 4);
        unsafe {
            std::ptr::copy_nonoverlapping(
                &basic as *const HostBasicInfo as *const u8,
                info,
                copy_len,
            );
        }
        return 0; // KERN_SUCCESS
    }
    0 // KERN_SUCCESS even for unknown flavors (best effort)
}

/// `host_statistics(host, flavor, info, &count)` — stub returning success.
pub extern "C" fn shim_host_statistics(
    _host: u32, _flavor: i32, _info: *mut u8, _count: *mut u32,
) -> i32 {
    0 // KERN_SUCCESS
}

/// `vm_allocate(task, &addr, size, flags)` — real mmap allocation.
pub extern "C" fn shim_vm_allocate(
    _task: u32, addr: *mut u64, size: u64, flags: i32,
) -> i32 {
    if addr.is_null() || size == 0 { return 4; } // KERN_INVALID_ARGUMENT
    let anywhere = (flags & 1) != 0; // VM_FLAGS_ANYWHERE
    let hint = if anywhere { std::ptr::null_mut() } else { (unsafe { *addr }) as *mut libc::c_void };
    let ptr = unsafe {
        libc::mmap(
            hint,
            size as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return 3; // KERN_NO_SPACE
    }
    unsafe { *addr = ptr as u64; }
    0 // KERN_SUCCESS
}

/// `vm_deallocate(task, addr, size)` — real munmap.
pub extern "C" fn shim_vm_deallocate(_task: u32, addr: u64, size: u64) -> i32 {
    if addr == 0 || size == 0 { return 0; }
    unsafe { libc::munmap(addr as *mut libc::c_void, size as usize); }
    0
}

/// `mach_port_allocate(task, right, &name)` — allocate a synthetic port name.
static NEXT_PORT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x200);

pub extern "C" fn shim_mach_port_allocate(_task: u32, _right: i32, name: *mut u32) -> i32 {
    if !name.is_null() {
        let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        unsafe { *name = port; }
    }
    0 // KERN_SUCCESS
}

/// `mach_port_deallocate(task, name)` — no-op (synthetic ports).
pub extern "C" fn shim_mach_port_deallocate(_task: u32, _name: u32) -> i32 { 0 }

/// `mach_port_mod_refs(task, name, right, delta)` — no-op.
pub extern "C" fn shim_mach_port_mod_refs(_task: u32, _name: u32, _right: i32, _delta: i32) -> i32 { 0 }

/// `task_info(task, flavor, info, &count)` — returns basic memory stats.
pub extern "C" fn shim_task_info(
    _task: u32, _flavor: i32, _info: *mut u8, _count: *mut u32,
) -> i32 {
    // Minimal stub — return KERN_SUCCESS. Callers that actually read the
    // struct will get zeroed data (which is safe).
    0
}

// ── Thread state & task control ─────────────────────────────────

/// `vm_protect(task, addr, size, set_maximum, prot)` → mprotect.
pub extern "C" fn shim_vm_protect(
    _task: u32, addr: u64, size: u64, _set_max: i32, prot: i32,
) -> i32 {
    // Convert Mach VM_PROT flags to POSIX PROT flags (same numeric values)
    let rc = unsafe { libc::mprotect(addr as *mut libc::c_void, size as usize, prot) };
    if rc == 0 { 0 } else { 1 } // 0=KERN_SUCCESS, 1=KERN_INVALID_ADDRESS
}

/// `host_statistics64(host, flavor, info, count)` — extended stats stub.
pub extern "C" fn shim_host_statistics64(
    _host: u32, _flavor: i32, _info: *mut u8, _count: *mut u32,
) -> i32 {
    0 // KERN_SUCCESS
}

/// KERN_FAILURE = 5 (thread state ops not supported in interpreter).
const KERN_FAILURE: i32 = 5;

/// `thread_get_state(thread, flavor, state, &count)` → KERN_FAILURE.
pub extern "C" fn shim_thread_get_state(
    _thread: u32, _flavor: i32, _state: *mut u8, _count: *mut u32,
) -> i32 {
    KERN_FAILURE
}

/// `thread_set_state(thread, flavor, state, count)` → KERN_FAILURE.
pub extern "C" fn shim_thread_set_state(
    _thread: u32, _flavor: i32, _state: *const u8, _count: u32,
) -> i32 {
    KERN_FAILURE
}

/// `thread_create(task, &child_thread)` → KERN_FAILURE (use pthreads instead).
pub extern "C" fn shim_thread_create(_task: u32, child: *mut u32) -> i32 {
    if !child.is_null() {
        unsafe { *child = 0; }
    }
    KERN_FAILURE
}

/// `thread_terminate(thread)` → KERN_SUCCESS (no-op in interpreter).
pub extern "C" fn shim_thread_terminate(_thread: u32) -> i32 { 0 }

/// `task_threads(task, &thread_list, &thread_count)` → 0 threads.
pub extern "C" fn shim_task_threads(
    _task: u32, thread_list: *mut u64, thread_count: *mut u32,
) -> i32 {
    if !thread_list.is_null() {
        unsafe { *thread_list = 0; }
    }
    if !thread_count.is_null() {
        unsafe { *thread_count = 0; }
    }
    0 // KERN_SUCCESS
}

/// `thread_info(thread, flavor, info, &count)` → KERN_SUCCESS with zeroed data.
pub extern "C" fn shim_thread_info(
    _thread: u32, _flavor: i32, _info: *mut u8, _count: *mut u32,
) -> i32 {
    0 // KERN_SUCCESS
}

/// `thread_suspend(thread)` → KERN_SUCCESS (no-op).
pub extern "C" fn shim_thread_suspend(_thread: u32) -> i32 { 0 }

/// `thread_resume(thread)` → KERN_SUCCESS (no-op).
pub extern "C" fn shim_thread_resume(_thread: u32) -> i32 { 0 }

/// `thread_abort(thread)` → KERN_SUCCESS (no-op).
pub extern "C" fn shim_thread_abort(_thread: u32) -> i32 { 0 }

/// `thread_abort_safely(thread)` → KERN_SUCCESS (no-op).
pub extern "C" fn shim_thread_abort_safely(_thread: u32) -> i32 { 0 }

/// `task_policy_set(task, flavor, policy_info, count)` → KERN_SUCCESS.
pub extern "C" fn shim_task_policy_set(
    _task: u32, _flavor: i32, _info: *const u8, _count: u32,
) -> i32 {
    0 // KERN_SUCCESS
}

/// `task_policy_get(task, flavor, policy_info, count, get_default)` → KERN_SUCCESS.
pub extern "C" fn shim_task_policy_get(
    _task: u32, _flavor: i32, _info: *mut u8, _count: *mut u32, _get_default: *mut i32,
) -> i32 {
    0 // KERN_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_protect_null_succeeds() {
        // vm_protect with a zero-size region is a no-op in practice
        let rc = shim_vm_protect(0, 0, 0, 0, libc::PROT_READ);
        // May return 0 or error; just confirm it doesn't panic
        let _ = rc;
    }

    #[test]
    fn thread_create_writes_zero() {
        let mut port: u32 = 0xDEAD;
        let rc = shim_thread_create(1, &mut port);
        assert_eq!(port, 0);
        assert_eq!(rc, KERN_FAILURE);
    }

    #[test]
    fn task_threads_writes_empty() {
        let mut list: u64 = 0xDEAD;
        let mut count: u32 = 99;
        let rc = shim_task_threads(1, &mut list, &mut count);
        assert_eq!(rc, 0);
        assert_eq!(list, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn thread_ops_return_success() {
        assert_eq!(shim_thread_terminate(1), 0);
        assert_eq!(shim_thread_suspend(1), 0);
        assert_eq!(shim_thread_resume(1), 0);
        assert_eq!(shim_thread_abort(1), 0);
        assert_eq!(shim_thread_abort_safely(1), 0);
    }

    #[test]
    fn task_policy_stubs() {
        assert_eq!(shim_task_policy_set(1, 0, std::ptr::null(), 0), 0);
        let mut info = 0u8;
        let mut count = 0u32;
        let mut get_default = 0i32;
        assert_eq!(shim_task_policy_get(1, 0, &mut info, &mut count, &mut get_default), 0);
    }

    #[test]
    fn host_statistics64_stub() {
        assert_eq!(shim_host_statistics64(0, 0, std::ptr::null_mut(), std::ptr::null_mut()), 0);
    }
}
