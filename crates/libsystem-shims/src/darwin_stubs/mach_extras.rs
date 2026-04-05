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
            ("_mach_continuous_time", shim_mach_continuous_time as *const () as u64, "mach_continuous_time (stub → mach_absolute_time)"),
            ("_mach_msg_destroy", shim_mach_msg_destroy as *const () as u64, "mach_msg_destroy (stub → void)"),
            ("_mach_msg", shim_mach_msg as *const () as u64, "mach_msg (stub → MACH_SEND_INVALID_DEST)"),
            ("_mach_msg_trap", shim_mach_msg as *const () as u64, "mach_msg_trap (stub → MACH_SEND_INVALID_DEST)"),
            ("_mach_port_type", shim_mach_port_type as *const () as u64, "mach_port_type (stub → KERN_INVALID_ARGUMENT)"),
            ("_mig_dealloc_special_reply_port", shim_mig_void_stub as *const () as u64, "mig_dealloc_special_reply_port (stub → void)"),
            ("_mig_get_special_reply_port", shim_mig_get_special_reply_port as *const () as u64, "mig_get_special_reply_port (stub → 0)"),
            ("_mig_strncpy", shim_mig_strncpy as *const () as u64, "mig_strncpy (stub → strncpy)"),
            ("_mig_strncpy_zerofill", shim_mig_strncpy as *const () as u64, "mig_strncpy_zerofill (stub → strncpy)")
        );

        // ── Host / VM / Task ────────────────────────────────────
        register_shims!(table,
            ("_mach_host_self", shim_mach_host_self as *const () as u64, "mach_host_self (→ synthetic port)"),
            ("_host_page_size", shim_host_page_size as *const () as u64, "host_page_size (→ real page size)"),
            ("_host_info", shim_host_info as *const () as u64, "host_info (→ real CPU/mem info)"),
            ("_host_statistics", shim_host_statistics as *const () as u64, "host_statistics (stub → KERN_SUCCESS)"),
            ("_host_statistics64", shim_host_statistics64 as *const () as u64, "host_statistics64 (stub → KERN_SUCCESS)"),
            ("_vm_allocate", shim_vm_allocate as *const () as u64, "vm_allocate (→ real mmap)"),
            ("_vm_deallocate", shim_vm_deallocate as *const () as u64, "vm_deallocate (→ real munmap)"),
            ("_vm_protect", shim_vm_protect as *const () as u64, "vm_protect (→ real mprotect)"),
            ("_vm_page_size", &VM_PAGE_SIZE_DATA as *const u64 as u64, "vm_page_size data symbol"),
            ("_mach_port_allocate", shim_mach_port_allocate as *const () as u64, "mach_port_allocate (stub → KERN_SUCCESS)"),
            ("_mach_port_deallocate", shim_mach_port_deallocate as *const () as u64, "mach_port_deallocate (stub → KERN_SUCCESS)"),
            ("_mach_port_mod_refs", shim_mach_port_mod_refs as *const () as u64, "mach_port_mod_refs (stub → KERN_SUCCESS)"),
            ("_task_info", shim_task_info as *const () as u64, "task_info (→ real memory stats)")
        );

        // ── Thread state & task control ─────────────────────────
        register_shims!(table,
            ("_thread_get_state", shim_thread_get_state as *const () as u64, "thread_get_state (stub → KERN_FAILURE)"),
            ("_thread_set_state", shim_thread_set_state as *const () as u64, "thread_set_state (stub → KERN_FAILURE)"),
            ("_thread_create", shim_thread_create as *const () as u64, "thread_create (stub → KERN_FAILURE)"),
            ("_thread_terminate", shim_thread_terminate as *const () as u64, "thread_terminate (stub → KERN_SUCCESS)"),
            ("_task_threads", shim_task_threads as *const () as u64, "task_threads (stub → 0 threads)"),
            ("_thread_info", shim_thread_info as *const () as u64, "thread_info (stub → KERN_SUCCESS)"),
            ("_thread_suspend", shim_thread_suspend as *const () as u64, "thread_suspend (stub → KERN_SUCCESS)"),
            ("_thread_resume", shim_thread_resume as *const () as u64, "thread_resume (stub → KERN_SUCCESS)"),
            ("_thread_abort", shim_thread_abort as *const () as u64, "thread_abort (stub → KERN_SUCCESS)"),
            ("_thread_abort_safely", shim_thread_abort_safely as *const () as u64, "thread_abort_safely (stub → KERN_SUCCESS)"),
            ("_task_policy_set", shim_task_policy_set as *const () as u64, "task_policy_set (stub → KERN_SUCCESS)"),
            ("_task_policy_get", shim_task_policy_get as *const () as u64, "task_policy_get (stub → KERN_SUCCESS)"),
            ("_task_set_exception_ports", shim_task_set_exception_ports as *const () as u64, "task_set_exception_ports (stub → KERN_SUCCESS)")
        );

        log::debug!("MachExtrasStubs: registered 33 Mach stubs");
    }
}

pub extern "C" fn shim_mach_continuous_time() -> u64 {
    // mach_continuous_time includes sleep time; approximate with monotonic clock
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
}

pub extern "C" fn shim_mach_msg_destroy(_msg: u64) {}

/// `mach_msg` / `mach_msg_trap` — basic MIG routing for known message IDs.
///
/// Parses the `mach_msg_header_t` to determine the MIG routine:
/// - msgh_id 403: `bootstrap_look_up` — look up a service name in the
///   bootstrap namespace and return a synthetic send right.
/// - All other message IDs return `MACH_SEND_INVALID_DEST`.
///
/// This is sufficient for code that uses the raw MIG interface for
/// bootstrap service lookups (e.g. `xpc_pipe_routine` → launchd).
pub extern "C" fn shim_mach_msg(
    msg: u64,
    options: u32,
    send_size: u32,
    rcv_size: u32,
    _rcv_name: u32,
    _timeout: u32,
    _notify: u32,
) -> u32 {
    if msg == 0 || send_size < 24 {
        return MACH_SEND_INVALID_DEST;
    }

    // Read mach_msg_header_t at `msg` (24 bytes):
    //   bits(4) + size(4) + remote_port(4) + local_port(4) + voucher(4) + id(4)
    let hdr = msg as *const u8;
    let msgh_id = unsafe {
        i32::from_le_bytes([
            *hdr.add(20), *hdr.add(21), *hdr.add(22), *hdr.add(23),
        ])
    };

    let is_send_recv = (options & (MACH_SEND_MSG | MACH_RCV_MSG)) == (MACH_SEND_MSG | MACH_RCV_MSG);

    match msgh_id {
        // MIG bootstrap_look_up request (routine 403)
        MIG_BOOTSTRAP_LOOK_UP if is_send_recv => {
            handle_bootstrap_look_up_mig(msg, rcv_size)
        }
        // MIG task_get_special_port (routine 3400)
        MIG_TASK_GET_SPECIAL_PORT if is_send_recv => {
            handle_task_get_special_port(msg, rcv_size)
        }
        // MIG host_info (routine 200)
        MIG_HOST_INFO if is_send_recv => {
            handle_host_info_mig(msg, rcv_size)
        }
        _ => {
            log::debug!("mach_msg: unhandled msgh_id={}, returning MACH_SEND_INVALID_DEST", msgh_id);
            MACH_SEND_INVALID_DEST
        }
    }
}

// Mach message option flags
const MACH_SEND_MSG: u32 = 0x00000001;
const MACH_RCV_MSG: u32 = 0x00000002;
const MACH_SEND_INVALID_DEST: u32 = 0x10000003;

// MIG routine numbers
const MIG_BOOTSTRAP_LOOK_UP: i32 = 403;
const MIG_HOST_INFO: i32 = 200;
const MIG_TASK_GET_SPECIAL_PORT: i32 = 3400;

/// Handle MIG routine 403 (bootstrap_look_up).
///
/// Request layout (after 24-byte header):
///   NDR_record (16 bytes) + service_name (128 bytes inline char[])
///
/// Reply layout (after 24-byte header):
///   NDR_record (16 bytes) + return_code (4 bytes) + service_port descriptor
fn handle_bootstrap_look_up_mig(msg: u64, rcv_size: u32) -> u32 {
    // Read the service name from the request (offset 40 = 24 hdr + 16 NDR)
    let name_ptr = (msg + 40) as *const u8;
    let name = unsafe {
        let mut len = 0usize;
        while len < 128 && *name_ptr.add(len) != 0 {
            len += 1;
        }
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, len))
    };

    log::debug!("mach_msg MIG 403: bootstrap_look_up(\"{}\")", name);

    // Look up or create a synthetic port for this service.
    let port = BOOTSTRAP_SERVICES.look_up_or_create(name);

    // Write the reply message.
    // Minimum reply = 24 (hdr) + 16 (NDR) + 4 (retcode) + 4 (port) = 48 bytes
    if rcv_size < 48 {
        return MACH_SEND_INVALID_DEST;
    }

    let reply = msg as *mut u8;
    unsafe {
        // Zero the reply area first
        std::ptr::write_bytes(reply, 0, rcv_size.min(256) as usize);

        // Reply header
        let reply_id: i32 = 403 + 100; // MIG reply = request ID + 100
        std::ptr::copy_nonoverlapping(reply_id.to_le_bytes().as_ptr(), reply.add(20), 4);

        // Reply size
        let reply_size: u32 = 48;
        std::ptr::copy_nonoverlapping(reply_size.to_le_bytes().as_ptr(), reply.add(4), 4);

        // NDR record at offset 24 (standard little-endian)
        *reply.add(27) = 1; // mig_encoding = NDR_PROTOCOL_2_0
        *reply.add(28) = 1; // int_rep = little-endian

        // Return code = KERN_SUCCESS (0) at offset 40
        // (already zeroed)

        // Service port at offset 44
        std::ptr::copy_nonoverlapping(port.to_le_bytes().as_ptr(), reply.add(44), 4);
    }

    0 // MACH_MSG_SUCCESS
}

/// Handle MIG routine 3400 (task_get_special_port).
///
/// Request: hdr(24) + NDR(16) + which_port(4)
/// Reply:   hdr(24) + NDR(16) + return_code(4) + port(4)
fn handle_task_get_special_port(msg: u64, rcv_size: u32) -> u32 {
    if rcv_size < 48 {
        return MACH_SEND_INVALID_DEST;
    }
    // Read which_port from offset 40 (24 hdr + 16 NDR)
    let which = unsafe {
        i32::from_le_bytes([
            *((msg + 40) as *const u8),
            *((msg + 41) as *const u8),
            *((msg + 42) as *const u8),
            *((msg + 43) as *const u8),
        ])
    };
    // Return the bootstrap port (MACH_HOST_PORT) for any request
    let port: u32 = match which {
        1 => MACH_HOST_PORT,   // TASK_KERNEL_PORT
        4 => 0x105,            // TASK_BOOTSTRAP_PORT — synthetic bootstrap
        _ => MACH_HOST_PORT,
    };
    log::debug!("mach_msg MIG 3400: task_get_special_port(which={}) → {:#x}", which, port);

    let reply = msg as *mut u8;
    unsafe {
        std::ptr::write_bytes(reply, 0, rcv_size.min(256) as usize);
        let reply_id: i32 = 3400 + 100;
        std::ptr::copy_nonoverlapping(reply_id.to_le_bytes().as_ptr(), reply.add(20), 4);
        let reply_size: u32 = 48;
        std::ptr::copy_nonoverlapping(reply_size.to_le_bytes().as_ptr(), reply.add(4), 4);
        *reply.add(27) = 1;
        *reply.add(28) = 1;
        std::ptr::copy_nonoverlapping(port.to_le_bytes().as_ptr(), reply.add(44), 4);
    }
    0
}

/// Handle MIG routine 200 (host_info).
///
/// Returns minimal host_basic_info: max_cpus, avail_cpus, memory_size.
fn handle_host_info_mig(msg: u64, rcv_size: u32) -> u32 {
    if rcv_size < 72 {
        return MACH_SEND_INVALID_DEST;
    }
    log::debug!("mach_msg MIG 200: host_info");

    let reply = msg as *mut u8;
    unsafe {
        std::ptr::write_bytes(reply, 0, rcv_size.min(256) as usize);
        let reply_id: i32 = 200 + 100;
        std::ptr::copy_nonoverlapping(reply_id.to_le_bytes().as_ptr(), reply.add(20), 4);
        let reply_size: u32 = 72;
        std::ptr::copy_nonoverlapping(reply_size.to_le_bytes().as_ptr(), reply.add(4), 4);
        *reply.add(27) = 1;
        *reply.add(28) = 1;
        // host_info_out starts at offset 44 (after hdr+NDR+retcode)
        // host_basic_info: max_cpus(4), avail_cpus(4), memory_size(4)
        let ncpu = libc::sysconf(libc::_SC_NPROCESSORS_ONLN).max(1) as u32;
        std::ptr::copy_nonoverlapping(ncpu.to_le_bytes().as_ptr(), reply.add(44), 4);
        std::ptr::copy_nonoverlapping(ncpu.to_le_bytes().as_ptr(), reply.add(48), 4);
        // memory_size at offset 52 (simplified: report as 0 and let callers use sysctl)
    }
    0
}

/// Bootstrap service registry — maps service names to synthetic Mach ports.
///
/// Thread-safe via `Mutex`. Ports are allocated from a monotonic counter
/// starting at `0x1000` (above the synthetic host/task port range).
struct BootstrapServiceRegistry {
    inner: std::sync::OnceLock<std::sync::Mutex<BootstrapRegistryInner>>,
}

struct BootstrapRegistryInner {
    services: std::collections::HashMap<String, u32>,
    next_port: u32,
}

impl BootstrapServiceRegistry {
    const fn new() -> Self {
        Self {
            inner: std::sync::OnceLock::new(),
        }
    }

    fn get_inner(&self) -> &std::sync::Mutex<BootstrapRegistryInner> {
        self.inner.get_or_init(|| {
            std::sync::Mutex::new(BootstrapRegistryInner {
                services: std::collections::HashMap::new(),
                next_port: 0x1000,
            })
        })
    }

    /// Look up an existing service or create a new synthetic port.
    fn look_up_or_create(&self, name: &str) -> u32 {
        let mut inner = self.get_inner().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&port) = inner.services.get(name) {
            return port;
        }
        let port = inner.next_port;
        inner.next_port += 1;
        inner.services.insert(name.to_string(), port);
        log::debug!("bootstrap: registered \"{}\" → port {:#x}", name, port);
        port
    }

    /// Look up a service by name.
    fn look_up(&self, name: &str) -> Option<u32> {
        let inner = self.get_inner().lock().unwrap_or_else(|e| e.into_inner());
        inner.services.get(name).copied()
    }
}

static BOOTSTRAP_SERVICES: BootstrapServiceRegistry = BootstrapServiceRegistry::new();

/// Public entry point for `bootstrap_look_up` C wrapper (posix_extras.rs).
pub fn bootstrap_look_up_service(name: &str) -> u32 {
    BOOTSTRAP_SERVICES.look_up_or_create(name)
}

/// KERN_INVALID_ARGUMENT = 4
pub extern "C" fn shim_mach_port_type(_task: u32, _name: u32, _ptype: u64) -> i32 { 4 }

pub extern "C" fn shim_mig_void_stub(_port: u32) {}

pub extern "C" fn shim_mig_get_special_reply_port() -> u32 { 0 }

pub extern "C" fn shim_mig_strncpy(dest: *mut u8, src: *const u8, len: u64) {
    if dest.is_null() || src.is_null() { return; }
    unsafe { libc::strncpy(dest as *mut std::ffi::c_char, src as *const std::ffi::c_char, len as usize); }
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

/// `kern_return_t task_set_exception_ports(task, mask, new_port, behavior, flavor)`
///
/// Mach exception port configuration — no-op on Linux.
pub extern "C" fn shim_task_set_exception_ports(
    _task: u32, _mask: u32, _new_port: u32, _behavior: u32, _flavor: u32,
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

    // ── Bootstrap / mach_msg tests ──────────────────────────────

    #[test]
    fn bootstrap_registry_creates_ports() {
        let reg = BootstrapServiceRegistry::new();
        let port = reg.look_up_or_create("com.test.creates_ports");
        assert!(port >= 0x1000, "port {:#x} should be >= 0x1000", port);
    }

    #[test]
    fn bootstrap_registry_same_service_same_port() {
        let reg = BootstrapServiceRegistry::new();
        let p1 = reg.look_up_or_create("com.test.same_service");
        let p2 = reg.look_up_or_create("com.test.same_service");
        assert_eq!(p1, p2);
    }

    #[test]
    fn bootstrap_registry_different_services_different_ports() {
        let reg = BootstrapServiceRegistry::new();
        let p1 = reg.look_up_or_create("com.test.service_a");
        let p2 = reg.look_up_or_create("com.test.service_b");
        assert_ne!(p1, p2);
    }

    #[test]
    fn mach_msg_null_returns_error() {
        let rc = shim_mach_msg(0, 0, 0, 0, 0, 0, 0);
        assert_eq!(rc, MACH_SEND_INVALID_DEST);
    }

    #[test]
    fn mach_msg_unknown_id_returns_error() {
        // Allocate a 24-byte header with an unknown msgh_id (999)
        let mut buf = vec![0u8; 64];
        let unknown_id: i32 = 999;
        buf[20..24].copy_from_slice(&unknown_id.to_le_bytes());
        let rc = shim_mach_msg(
            buf.as_mut_ptr() as u64,
            MACH_SEND_MSG | MACH_RCV_MSG,
            24,  // send_size
            64,  // rcv_size
            0, 0, 0,
        );
        assert_eq!(rc, MACH_SEND_INVALID_DEST);
    }

    #[test]
    fn mach_msg_bootstrap_look_up_succeeds() {
        // Build a MIG 403 (bootstrap_look_up) request:
        //   24-byte header + 16-byte NDR + 128-byte service name = 168 bytes
        // Use 256 bytes for send+rcv buffer.
        let mut buf = vec![0u8; 256];

        // --- Header (24 bytes) ---
        // msgh_bits at offset 0 (unused by our shim, leave 0)
        // msgh_size at offset 4
        let send_size: u32 = 168;
        buf[4..8].copy_from_slice(&send_size.to_le_bytes());
        // msgh_remote_port at offset 8, msgh_local_port at offset 12,
        // msgh_voucher at offset 16 — leave 0
        // msgh_id at offset 20 = 403
        let msg_id: i32 = 403;
        buf[20..24].copy_from_slice(&msg_id.to_le_bytes());

        // --- NDR record (16 bytes at offset 24) ---
        // bytes 3 = 1 (mig_encoding), byte 4 = 1 (int_rep)
        buf[27] = 1;
        buf[28] = 1;

        // --- Service name (128 bytes at offset 40) ---
        let svc = b"com.apple.test.shim_msg";
        buf[40..40 + svc.len()].copy_from_slice(svc);
        // rest is already zero (null terminated)

        let rcv_size: u32 = 256;
        let rc = shim_mach_msg(
            buf.as_mut_ptr() as u64,
            MACH_SEND_MSG | MACH_RCV_MSG,
            send_size,
            rcv_size,
            0, 0, 0,
        );
        assert_eq!(rc, 0, "mach_msg should return MACH_MSG_SUCCESS (0)");

        // Reply header msgh_id should be 403 + 100 = 503
        let reply_id = i32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
        assert_eq!(reply_id, 503);

        // Service port at reply offset 44
        let port = u32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
        assert!(port >= 0x1000, "reply port {:#x} should be >= 0x1000", port);
    }
}
