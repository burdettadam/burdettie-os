//! `bootstrap.h` API — Mach bootstrap server port management.
//!
//! Implements the full public API from Apple's `<servers/bootstrap.h>` and
//! the private `<bootstrap_priv.h>`.  Uses an in-process port registry
//! instead of real Mach IPC.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::constants::*;

/// The global bootstrap port, matching Apple's `extern mach_port_t bootstrap_port`.
///
/// Set to a synthetic non-zero value so code that checks `bootstrap_port != 0`
/// considers it valid.
static BOOTSTRAP_PORT: AtomicU32 = AtomicU32::new(0x105);

/// Get the current bootstrap_port value.
pub fn get_bootstrap_port() -> u32 {
    BOOTSTRAP_PORT.load(Ordering::Relaxed)
}

/// Get a mutable pointer to the bootstrap_port, for use as a data symbol.
pub fn bootstrap_port_ptr() -> *mut u32 {
    // Safety: AtomicU32 has the same layout as u32.
    &BOOTSTRAP_PORT as *const AtomicU32 as *mut u32
}

// ── Port registry ───────────────────────────────────────────────────

struct BootstrapRegistryInner {
    /// service_name → (port, is_checked_in)
    services: HashMap<String, ServiceEntry>,
    /// Port allocator counter.
    next_port: u32,
    /// Subset bootstraps: child_port → parent_port.
    subsets: HashMap<u32, u32>,
}

#[derive(Debug, Clone)]
struct ServiceEntry {
    port: u32,
    checked_in: bool,
}

static REGISTRY: OnceLock<Mutex<BootstrapRegistryInner>> = OnceLock::new();

fn with_registry<R>(f: impl FnOnce(&mut BootstrapRegistryInner) -> R) -> R {
    let mutex = REGISTRY.get_or_init(|| {
        Mutex::new(BootstrapRegistryInner {
            services: HashMap::new(),
            next_port: 0x1000,
            subsets: HashMap::new(),
        })
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

fn alloc_port(inner: &mut BootstrapRegistryInner) -> u32 {
    let port = inner.next_port;
    inner.next_port += 1;
    port
}

// ═══════════════════════════════════════════════════════════════════
//  Public C API — matching <servers/bootstrap.h>
// ═══════════════════════════════════════════════════════════════════

/// `bootstrap_create_server(bp, server_cmd, server_uid, on_demand, &server_port)` → kern_return_t.
///
/// # Safety
/// `server_port` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
/// `_server_cmd` is ignored and is never dereferenced.
pub unsafe fn bootstrap_create_server(
    _bp: u32,
    _server_cmd: *const libc::c_char,
    _server_uid: u32,
    _on_demand: bool,
    server_port: *mut u32,
) -> u32 {
    if server_port.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let port = with_registry(alloc_port);
    unsafe {
        *server_port = port;
    }
    log::debug!("bootstrap_create_server → port {:#x}", port);
    BOOTSTRAP_SUCCESS
}

/// `bootstrap_subset(bp, requestor_port, &subset_port)` → kern_return_t.
///
/// # Safety
/// `subset_port` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_subset(bp: u32, _requestor_port: u32, subset_port: *mut u32) -> u32 {
    if subset_port.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let port = with_registry(|inner| {
        let p = alloc_port(inner);
        inner.subsets.insert(p, bp);
        p
    });
    unsafe {
        *subset_port = port;
    }
    log::debug!("bootstrap_subset({:#x}) → {:#x}", bp, port);
    BOOTSTRAP_SUCCESS
}

/// `bootstrap_unprivileged(bp, &unpriv_port)` → kern_return_t.
///
/// In our emulation, all ports are unprivileged, so we return the same port.
///
/// # Safety
/// `unpriv_port` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_unprivileged(bp: u32, unpriv_port: *mut u32) -> u32 {
    if unpriv_port.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    unsafe {
        *unpriv_port = bp;
    }
    BOOTSTRAP_SUCCESS
}

/// `bootstrap_parent(bp, &parent_port)` → kern_return_t.
///
/// # Safety
/// `parent_port` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_parent(bp: u32, parent_port: *mut u32) -> u32 {
    if parent_port.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let parent = with_registry(|inner| inner.subsets.get(&bp).copied().unwrap_or(bp));
    unsafe {
        *parent_port = parent;
    }
    BOOTSTRAP_SUCCESS
}

/// `bootstrap_register(bp, service_name, sp)` → kern_return_t.
///
/// Deprecated but still used by legacy code.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
pub unsafe fn bootstrap_register(_bp: u32, service_name: *const libc::c_char, sp: u32) -> u32 {
    let Some(name) = (unsafe { read_cstr(service_name) }) else {
        return BOOTSTRAP_BAD_COUNT;
    };
    with_registry(|inner| {
        if let Some(entry) = inner.services.get(&name) {
            if entry.checked_in {
                return BOOTSTRAP_NAME_IN_USE;
            }
        }
        inner.services.insert(
            name.clone(),
            ServiceEntry {
                port: sp,
                checked_in: true,
            },
        );
        log::debug!("bootstrap_register(\"{}\") → port {:#x}", name, sp);
        BOOTSTRAP_SUCCESS
    })
}

/// `bootstrap_create_service(bp, service_name, &sp)` → kern_return_t.
///
/// Deprecated.  Creates a port for the named service.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_create_service(
    _bp: u32,
    service_name: *const libc::c_char,
    sp: *mut u32,
) -> u32 {
    if sp.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let Some(name) = (unsafe { read_cstr(service_name) }) else {
        return BOOTSTRAP_BAD_COUNT;
    };
    with_registry(|inner| {
        if inner.services.contains_key(&name) {
            return BOOTSTRAP_SERVICE_ACTIVE;
        }
        let port = alloc_port(inner);
        inner.services.insert(
            name.clone(),
            ServiceEntry {
                port,
                checked_in: false,
            },
        );
        unsafe {
            *sp = port;
        }
        log::debug!("bootstrap_create_service(\"{}\") → port {:#x}", name, port);
        BOOTSTRAP_SUCCESS
    })
}

/// `bootstrap_check_in(bp, service_name, &sp)` → kern_return_t.
///
/// Returns the receive right for a declared service.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_check_in(_bp: u32, service_name: *const libc::c_char, sp: *mut u32) -> u32 {
    if sp.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let Some(name) = (unsafe { read_cstr(service_name) }) else {
        return BOOTSTRAP_UNKNOWN_SERVICE;
    };
    with_registry(|inner| {
        // If service already exists, check if it's checked in
        if let Some(entry) = inner.services.get_mut(&name) {
            if entry.checked_in {
                return BOOTSTRAP_SERVICE_ACTIVE;
            }
            entry.checked_in = true;
            unsafe {
                *sp = entry.port;
            }
            log::debug!("bootstrap_check_in(\"{}\") → port {:#x}", name, entry.port);
            return BOOTSTRAP_SUCCESS;
        }

        // Auto-create the service on check-in (matching Apple behavior for
        // jobs that declare MachServices in their plist)
        let port = alloc_port(inner);
        inner.services.insert(
            name.clone(),
            ServiceEntry {
                port,
                checked_in: true,
            },
        );
        unsafe {
            *sp = port;
        }
        log::debug!("bootstrap_check_in(\"{}\") → new port {:#x}", name, port);
        BOOTSTRAP_SUCCESS
    })
}

/// `bootstrap_look_up(bp, service_name, &sp)` → kern_return_t.
///
/// Returns a send right for the named service.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_look_up(_bp: u32, service_name: *const libc::c_char, sp: *mut u32) -> u32 {
    if sp.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let Some(name) = (unsafe { read_cstr(service_name) }) else {
        return BOOTSTRAP_UNKNOWN_SERVICE;
    };
    with_registry(|inner| {
        if let Some(entry) = inner.services.get(&name) {
            unsafe {
                *sp = entry.port;
            }
            log::debug!("bootstrap_look_up(\"{}\") → port {:#x}", name, entry.port);
            BOOTSTRAP_SUCCESS
        } else {
            // Auto-create a port for the service (matching Apple behavior for
            // lookups that happen before check-in)
            let port = alloc_port(inner);
            inner.services.insert(
                name.clone(),
                ServiceEntry {
                    port,
                    checked_in: false,
                },
            );
            unsafe {
                *sp = port;
            }
            log::debug!(
                "bootstrap_look_up(\"{}\") → auto-created port {:#x}",
                name,
                port
            );
            BOOTSTRAP_SUCCESS
        }
    })
}

/// `bootstrap_status(bp, service_name, &status)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `service_active` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_status(
    _bp: u32,
    service_name: *const libc::c_char,
    service_active: *mut u32,
) -> u32 {
    if service_active.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    let Some(name) = (unsafe { read_cstr(service_name) }) else {
        return BOOTSTRAP_UNKNOWN_SERVICE;
    };
    with_registry(|inner| match inner.services.get(&name) {
        Some(entry) => {
            let status = if entry.checked_in {
                BOOTSTRAP_STATUS_ACTIVE
            } else {
                BOOTSTRAP_STATUS_ON_DEMAND
            };
            unsafe {
                *service_active = status;
            }
            BOOTSTRAP_SUCCESS
        }
        None => BOOTSTRAP_UNKNOWN_SERVICE,
    })
}

/// `bootstrap_strerror(r)` → string description.
pub fn bootstrap_strerror(r: u32) -> &'static str {
    match r {
        BOOTSTRAP_SUCCESS => "Success",
        BOOTSTRAP_NOT_PRIVILEGED => "Not privileged",
        BOOTSTRAP_NAME_IN_USE => "Name in use",
        BOOTSTRAP_UNKNOWN_SERVICE => "Unknown service",
        BOOTSTRAP_SERVICE_ACTIVE => "Service active",
        BOOTSTRAP_BAD_COUNT => "Bad count",
        BOOTSTRAP_NO_MEMORY => "No memory",
        BOOTSTRAP_NO_CHILDREN => "No children",
        _ => "Unknown bootstrap error",
    }
}

// ── Private API (bootstrap_priv.h) ──────────────────────────────────

/// `bootstrap_register2(bp, service_name, sp, flags)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
pub unsafe fn bootstrap_register2(
    bp: u32,
    service_name: *const libc::c_char,
    sp: u32,
    _flags: u64,
) -> u32 {
    // Flags are ignored in our implementation
    unsafe { bootstrap_register(bp, service_name, sp) }
}

/// `bootstrap_look_up2(bp, service_name, &sp, target_pid, flags)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_look_up2(
    bp: u32,
    service_name: *const libc::c_char,
    sp: *mut u32,
    _target_pid: i32,
    _flags: u64,
) -> u32 {
    unsafe { bootstrap_look_up(bp, service_name, sp) }
}

/// `bootstrap_check_in2(bp, service_name, &sp, flags)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_check_in2(
    bp: u32,
    service_name: *const libc::c_char,
    sp: *mut u32,
    _flags: u64,
) -> u32 {
    unsafe { bootstrap_check_in(bp, service_name, sp) }
}

/// `bootstrap_look_up3(bp, service_name, &sp, target_pid, instance_id, flags)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
/// `_instance_id` is ignored and is never dereferenced.
pub unsafe fn bootstrap_look_up3(
    bp: u32,
    service_name: *const libc::c_char,
    sp: *mut u32,
    _target_pid: i32,
    _instance_id: *const u8,
    _flags: u64,
) -> u32 {
    unsafe { bootstrap_look_up(bp, service_name, sp) }
}

/// `bootstrap_check_in3(bp, service_name, &sp, instance_id, flags)` → kern_return_t.
///
/// # Safety
/// `service_name` must be null or point to a readable, NUL-terminated
/// C string that remains unchanged for the duration of the call.
/// `sp` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
/// `_instance_id` is ignored and is never dereferenced.
pub unsafe fn bootstrap_check_in3(
    bp: u32,
    service_name: *const libc::c_char,
    sp: *mut u32,
    _instance_id: *mut u8,
    _flags: u64,
) -> u32 {
    unsafe { bootstrap_check_in(bp, service_name, sp) }
}

/// `bootstrap_get_root(bp, &root)` → kern_return_t.
///
/// # Safety
/// `root` must be null or point to an aligned, writable `u32` that is
/// exclusively accessible for the duration of the call.
pub unsafe fn bootstrap_get_root(_bp: u32, root: *mut u32) -> u32 {
    if root.is_null() {
        return BOOTSTRAP_BAD_COUNT;
    }
    unsafe {
        *root = get_bootstrap_port();
    }
    BOOTSTRAP_SUCCESS
}

// ── Rust-native helpers ─────────────────────────────────────────────

/// Reset the bootstrap registry (for testing).
pub fn reset_registry() {
    with_registry(|inner| {
        inner.services.clear();
        inner.subsets.clear();
        inner.next_port = 0x1000;
    });
}

/// Look up a service by name (Rust API).
pub fn look_up_service(name: &str) -> Option<u32> {
    with_registry(|inner| inner.services.get(name).map(|e| e.port))
}

/// Register a service by name with a specific port (Rust API).
pub fn register_service(name: &str, port: u32) {
    with_registry(|inner| {
        inner.services.insert(
            name.to_string(),
            ServiceEntry {
                port,
                checked_in: true,
            },
        );
    });
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Read an optional C string into owned storage.
///
/// # Safety
/// A non-null `ptr` must point to a readable, NUL-terminated C string that
/// remains unchanged while it is read. Null checking alone does not validate it.
unsafe fn read_cstr(ptr: *const libc::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let cs = unsafe { std::ffi::CStr::from_ptr(ptr) };
    cs.to_str().ok().map(|s| s.to_string())
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    // SAFETY: Test names are live C strings and output pointers refer to
    // exclusive local values; null pointers exercise documented error paths.

    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Generate a unique service name per test invocation to avoid
    /// inter-test interference from shared global state.
    fn unique_name(prefix: &str) -> std::ffi::CString {
        static CTR: AtomicU32 = AtomicU32::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        std::ffi::CString::new(format!("com.test.{}.{}", prefix, n)).unwrap()
    }

    #[test]
    fn bootstrap_port_is_nonzero() {
        assert_ne!(get_bootstrap_port(), 0);
    }

    #[test]
    fn check_in_and_look_up() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("checkin");
        let mut port: u32 = 0;
        let r = unsafe { bootstrap_check_in(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);

        let mut port2: u32 = 0;
        let r = unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port2) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, port2);
    }

    #[test]
    fn double_check_in_returns_active() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("dup");
        let mut port: u32 = 0;
        unsafe { bootstrap_check_in(0, name.as_ptr(), &mut port) };
        let r = unsafe { bootstrap_check_in(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SERVICE_ACTIVE);
    }

    #[test]
    fn create_service_and_check_in() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("create");
        let mut port: u32 = 0;
        let r = unsafe { bootstrap_create_service(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);

        // Check in with the created service
        let mut port2: u32 = 0;
        let r = unsafe { bootstrap_check_in(0, name.as_ptr(), &mut port2) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, port2);
    }

    #[test]
    fn duplicate_create_service_returns_active() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("dupcs");
        let mut port: u32 = 0;
        unsafe { bootstrap_create_service(0, name.as_ptr(), &mut port) };
        let r = unsafe { bootstrap_create_service(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SERVICE_ACTIVE);
    }

    #[test]
    fn register_and_look_up() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("reg");
        let r = unsafe { bootstrap_register(0, name.as_ptr(), 0x5000) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);

        let mut port: u32 = 0;
        let r = unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, 0x5000);
    }

    #[test]
    fn status_query() {
        let _state_guard = crate::lock_test_state();
        let name_unknown = unique_name("statusU");
        let name_svc = unique_name("statusS");

        // Unknown service
        let mut status: u32 = 0;
        let r = unsafe { bootstrap_status(0, name_unknown.as_ptr(), &mut status) };
        assert_eq!(r, BOOTSTRAP_UNKNOWN_SERVICE);

        // Create but don't check in → ON_DEMAND
        let mut port: u32 = 0;
        unsafe { bootstrap_create_service(0, name_svc.as_ptr(), &mut port) };
        let r = unsafe { bootstrap_status(0, name_svc.as_ptr(), &mut status) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(status, BOOTSTRAP_STATUS_ON_DEMAND);

        // Check in → ACTIVE
        unsafe { bootstrap_check_in(0, name_svc.as_ptr(), &mut port) };
        let r = unsafe { bootstrap_status(0, name_svc.as_ptr(), &mut status) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(status, BOOTSTRAP_STATUS_ACTIVE);
    }

    #[test]
    fn strerror_messages() {
        assert_eq!(bootstrap_strerror(BOOTSTRAP_SUCCESS), "Success");
        assert_eq!(
            bootstrap_strerror(BOOTSTRAP_UNKNOWN_SERVICE),
            "Unknown service"
        );
        assert_eq!(bootstrap_strerror(9999), "Unknown bootstrap error");
    }

    #[test]
    fn subset_and_parent() {
        let _state_guard = crate::lock_test_state();
        let root = get_bootstrap_port();
        let mut child: u32 = 0;
        unsafe { bootstrap_subset(root, 0, &mut child) };
        assert_ne!(child, 0);

        let mut parent: u32 = 0;
        unsafe { bootstrap_parent(child, &mut parent) };
        assert_eq!(parent, root);

        // Root's parent is itself
        unsafe { bootstrap_parent(root, &mut parent) };
        assert_eq!(parent, root);
    }

    #[test]
    fn unprivileged_returns_same_port() {
        let mut port: u32 = 0;
        let r = unsafe { bootstrap_unprivileged(0x105, &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, 0x105);
    }

    #[test]
    fn create_server() {
        let _state_guard = crate::lock_test_state();
        let mut port: u32 = 0;
        let r =
            unsafe { bootstrap_create_server(0, c"/usr/sbin/httpd".as_ptr(), 0, true, &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);
    }

    #[test]
    fn auto_create_on_look_up() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("autocreate");
        let mut port: u32 = 0;
        let r = unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);

        // Second lookup returns same port
        let mut port2: u32 = 0;
        unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port2) };
        assert_eq!(port, port2);
    }

    #[test]
    fn get_root() {
        let mut root: u32 = 0;
        let r = unsafe { bootstrap_get_root(0, &mut root) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(root, get_bootstrap_port());
    }

    // ── Additional coverage ─────────────────────────────────────

    #[test]
    fn null_service_name_returns_error() {
        let _state_guard = crate::lock_test_state();
        let mut port: u32 = 0;
        assert_eq!(
            unsafe { bootstrap_look_up(0, std::ptr::null(), &mut port) },
            BOOTSTRAP_UNKNOWN_SERVICE
        );
        assert_eq!(
            unsafe { bootstrap_check_in(0, std::ptr::null(), &mut port) },
            BOOTSTRAP_UNKNOWN_SERVICE
        );
        assert_eq!(
            unsafe { bootstrap_register(0, std::ptr::null(), 0x100) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_create_service(0, std::ptr::null(), &mut port) },
            BOOTSTRAP_BAD_COUNT
        );

        let mut status: u32 = 0;
        assert_eq!(
            unsafe { bootstrap_status(0, std::ptr::null(), &mut status) },
            BOOTSTRAP_UNKNOWN_SERVICE
        );
    }

    #[test]
    fn null_output_pointer_returns_bad_count() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("nullout");
        assert_eq!(
            unsafe { bootstrap_look_up(0, name.as_ptr(), std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_check_in(0, name.as_ptr(), std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_create_service(0, name.as_ptr(), std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_status(0, name.as_ptr(), std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_subset(0, 0, std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_unprivileged(0, std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_parent(0, std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe {
                bootstrap_create_server(0, c"/bin/sh".as_ptr(), 0, false, std::ptr::null_mut())
            },
            BOOTSTRAP_BAD_COUNT
        );
        assert_eq!(
            unsafe { bootstrap_get_root(0, std::ptr::null_mut()) },
            BOOTSTRAP_BAD_COUNT
        );
    }

    #[test]
    fn strerror_all_known_codes() {
        assert_eq!(bootstrap_strerror(BOOTSTRAP_SUCCESS), "Success");
        assert_eq!(
            bootstrap_strerror(BOOTSTRAP_NOT_PRIVILEGED),
            "Not privileged"
        );
        assert_eq!(bootstrap_strerror(BOOTSTRAP_NAME_IN_USE), "Name in use");
        assert_eq!(
            bootstrap_strerror(BOOTSTRAP_UNKNOWN_SERVICE),
            "Unknown service"
        );
        assert_eq!(
            bootstrap_strerror(BOOTSTRAP_SERVICE_ACTIVE),
            "Service active"
        );
        assert_eq!(bootstrap_strerror(BOOTSTRAP_BAD_COUNT), "Bad count");
        assert_eq!(bootstrap_strerror(BOOTSTRAP_NO_MEMORY), "No memory");
        assert_eq!(bootstrap_strerror(BOOTSTRAP_NO_CHILDREN), "No children");
        assert_eq!(bootstrap_strerror(0xFFFF), "Unknown bootstrap error");
    }

    #[test]
    fn register_checked_in_service_returns_name_in_use() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("regactive");
        let mut port: u32 = 0;
        // Check in creates and marks as checked_in
        unsafe { bootstrap_check_in(0, name.as_ptr(), &mut port) };
        // Trying to register should fail
        let r = unsafe { bootstrap_register(0, name.as_ptr(), 0x5000) };
        assert_eq!(r, BOOTSTRAP_NAME_IN_USE);
    }

    #[test]
    fn register_unchecked_service_overwrites() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("regover");
        let mut port: u32 = 0;
        // Create (not checked in)
        unsafe { bootstrap_create_service(0, name.as_ptr(), &mut port) };
        // Register should succeed (not checked in yet)
        let r = unsafe { bootstrap_register(0, name.as_ptr(), 0x6000) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        // Lookup should show the new port
        let mut port2: u32 = 0;
        unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port2) };
        assert_eq!(port2, 0x6000);
    }

    #[test]
    fn bootstrap_port_ptr_is_valid() {
        let ptr = bootstrap_port_ptr();
        assert!(!ptr.is_null());
        let val = unsafe { *ptr };
        assert_eq!(val, get_bootstrap_port());
    }

    #[test]
    fn look_up2_delegates_to_look_up() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("lu2");
        let mut port: u32 = 0;
        unsafe { bootstrap_register(0, name.as_ptr(), 0x7000) };
        let r = unsafe { bootstrap_look_up2(0, name.as_ptr(), &mut port, -1, 0) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, 0x7000);
    }

    #[test]
    fn check_in2_delegates_to_check_in() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("ci2");
        let mut port: u32 = 0;
        let r = unsafe { bootstrap_check_in2(0, name.as_ptr(), &mut port, 0) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);
    }

    #[test]
    fn look_up3_delegates_to_look_up() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("lu3");
        let mut port: u32 = 0;
        unsafe { bootstrap_register(0, name.as_ptr(), 0x8000) };
        let r = unsafe { bootstrap_look_up3(0, name.as_ptr(), &mut port, -1, std::ptr::null(), 0) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_eq!(port, 0x8000);
    }

    #[test]
    fn check_in3_delegates_to_check_in() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("ci3");
        let mut port: u32 = 0;
        let r =
            unsafe { bootstrap_check_in3(0, name.as_ptr(), &mut port, std::ptr::null_mut(), 0) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        assert_ne!(port, 0);
    }

    #[test]
    fn register2_delegates_to_register() {
        let _state_guard = crate::lock_test_state();
        let name = unique_name("reg2");
        let r = unsafe { bootstrap_register2(0, name.as_ptr(), 0x9000, 0) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
        let mut port: u32 = 0;
        unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port) };
        assert_eq!(port, 0x9000);
    }

    #[test]
    fn subset_hierarchy_deep_nesting() {
        let _state_guard = crate::lock_test_state();
        let root = get_bootstrap_port();
        let mut child: u32 = 0;
        unsafe { bootstrap_subset(root, 0, &mut child) };

        let mut grandchild: u32 = 0;
        unsafe { bootstrap_subset(child, 0, &mut grandchild) };

        // grandchild → child → root
        let mut parent: u32 = 0;
        unsafe { bootstrap_parent(grandchild, &mut parent) };
        assert_eq!(parent, child);

        unsafe { bootstrap_parent(child, &mut parent) };
        assert_eq!(parent, root);

        unsafe { bootstrap_parent(root, &mut parent) };
        assert_eq!(parent, root); // root's parent is itself
    }

    #[test]
    fn rust_helper_look_up_and_register() {
        let _state_guard = crate::lock_test_state();
        let svc = format!(
            "com.test.rust_api.{}",
            unique_name("rapi").to_str().unwrap()
        );
        assert!(look_up_service(&svc).is_none());
        register_service(&svc, 0xBEEF);
        assert_eq!(look_up_service(&svc), Some(0xBEEF));
    }

    #[test]
    fn empty_string_service_name() {
        let _state_guard = crate::lock_test_state();
        let name = std::ffi::CString::new("").unwrap();
        let mut port: u32 = 0;
        // Empty string is valid UTF-8 so read_cstr succeeds — should work
        let r = unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port) };
        assert_eq!(r, BOOTSTRAP_SUCCESS);
    }

    #[test]
    fn concurrent_registrations() {
        let _state_guard = crate::lock_test_state();
        use std::sync::Arc;
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|t| {
                let b = barrier.clone();
                std::thread::spawn(move || {
                    b.wait();
                    for i in 0..25 {
                        let name =
                            std::ffi::CString::new(format!("com.conc.t{}.s{}", t, i)).unwrap();
                        let mut port: u32 = 0;
                        let r = unsafe { bootstrap_look_up(0, name.as_ptr(), &mut port) };
                        assert_eq!(r, BOOTSTRAP_SUCCESS);
                        assert_ne!(port, 0);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
}
