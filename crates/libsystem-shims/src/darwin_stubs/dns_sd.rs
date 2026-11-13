//! dns_sd.h (Bonjour / mDNSResponder) stub shims for Linux.
//!
//! Implements DNSServiceBrowse, DNSServiceRegister, DNSServiceResolve, TXTRecord* helpers.
//! Service refs are opaque handles with pipe-based fds for event loop integration.
//! No real network discovery; all responses are synthetic.

use crate::table::ShimTable;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ── ServiceRef state ───────────────────────────────────────────

struct ServiceRef {
    handle: u64,
    fd: OwnedFd,
    // Keep the writer alive so a live service's read end does not report EOF.
    _write_fd: OwnedFd,
    kind: ServiceKind,
    callback: u64,
    context: u64,
    txt: Option<Vec<u8>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ServiceKind {
    Browse,
    Register,
    Resolve,
}

fn service_refs() -> &'static Mutex<HashMap<u64, ServiceRef>> {
    static STORE: OnceLock<Mutex<HashMap<u64, ServiceRef>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0xD155_0001);

fn alloc_service_ref(kind: ServiceKind, callback: u64, context: u64) -> u64 {
    let (fd_read, fd_write) = unsafe {
        let mut fds = [0; 2];
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            return 0;
        }
        (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1]))
    };
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let ref_ = ServiceRef {
        handle,
        fd: fd_read,
        _write_fd: fd_write,
        kind,
        callback,
        context,
        txt: None,
    };
    service_refs().lock().unwrap().insert(handle, ref_);
    handle
}

// ── TXTRecord helpers ──────────────────────────────────────────

#[no_mangle]
pub extern "C" fn TXTRecordCreate(_txt: *mut u8, _buffer_len: u16, _buffer: *mut u8) {
    // No-op: buffer is caller-owned
}

#[no_mangle]
pub extern "C" fn TXTRecordDeallocate(_txt: *mut u8) {
    // No-op
}

#[no_mangle]
pub extern "C" fn TXTRecordSetValue(
    _txt: *mut u8,
    _key: *const u8,
    _key_len: u8,
    _value_len: u8,
    _value: *const u8,
) -> i32 {
    // No-op: just pretend success
    0
}

#[no_mangle]
pub extern "C" fn TXTRecordGetLength(_txt: *const u8) -> u16 {
    0
}

#[no_mangle]
pub extern "C" fn TXTRecordGetBytesPtr(_txt: *const u8) -> *const u8 {
    std::ptr::null()
}

// ── DNSService* entry points ──────────────────────────────────

#[no_mangle]
pub extern "C" fn DNSServiceBrowse(
    sd_ref: *mut u64,
    _flags: u32,
    _iface: u32,
    _regtype: *const u8,
    _domain: *const u8,
    callback: u64,
    context: u64,
) -> i32 {
    if sd_ref.is_null() {
        return -1;
    }
    let handle = alloc_service_ref(ServiceKind::Browse, callback, context);
    unsafe {
        *sd_ref = handle;
    }
    if handle == 0 {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn DNSServiceRegister(
    sd_ref: *mut u64,
    _flags: u32,
    _iface: u32,
    _name: *const u8,
    _regtype: *const u8,
    _domain: *const u8,
    _host: *const u8,
    _port: u16,
    _txt_len: u16,
    _txt: *const u8,
    callback: u64,
    context: u64,
) -> i32 {
    if sd_ref.is_null() {
        return -1;
    }
    let handle = alloc_service_ref(ServiceKind::Register, callback, context);
    unsafe {
        *sd_ref = handle;
    }
    if handle == 0 {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn DNSServiceResolve(
    sd_ref: *mut u64,
    _flags: u32,
    _iface: u32,
    _name: *const u8,
    _regtype: *const u8,
    _domain: *const u8,
    callback: u64,
    context: u64,
) -> i32 {
    if sd_ref.is_null() {
        return -1;
    }
    let handle = alloc_service_ref(ServiceKind::Resolve, callback, context);
    unsafe {
        *sd_ref = handle;
    }
    if handle == 0 {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn DNSServiceRefSockFD(sd_ref: u64) -> i32 {
    let refs = service_refs().lock().unwrap();
    refs.get(&sd_ref).map(|r| r.fd.as_raw_fd()).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn DNSServiceProcessResult(sd_ref: u64) -> i32 {
    // Synchronously invoke callback with synthetic data
    let callback_context = {
        let refs = service_refs().lock().unwrap();
        refs.get(&sd_ref).map(|r| (r.callback, r.context))
    };
    // A callback may query or deallocate its own service reference.
    if let Some((callback, context)) = callback_context {
        // For now, just call the callback with nulls
        if callback != 0 {
            let cb: extern "C" fn(
                u64,
                u32,
                u32,
                u32,
                *const u8,
                *const u8,
                *const u8,
                *const u8,
                u16,
                *const u8,
                u64,
            ) = unsafe { std::mem::transmute(callback as usize) };
            cb(
                sd_ref,
                0,
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                context,
            );
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn DNSServiceRefDeallocate(sd_ref: u64) {
    let mut refs = service_refs().lock().unwrap();
    // Dropping the service closes both owned pipe descriptors.
    refs.remove(&sd_ref);
}

// ── Install ───────────────────────────────────────────────────

pub struct DnsSdStubs;

impl DnsSdStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_DNSServiceBrowse",
                DNSServiceBrowse as *const () as u64,
                "DNSServiceBrowse → stub"
            ),
            (
                "_DNSServiceRegister",
                DNSServiceRegister as *const () as u64,
                "DNSServiceRegister → stub"
            ),
            (
                "_DNSServiceResolve",
                DNSServiceResolve as *const () as u64,
                "DNSServiceResolve → stub"
            ),
            (
                "_DNSServiceRefSockFD",
                DNSServiceRefSockFD as *const () as u64,
                "DNSServiceRefSockFD → stub"
            ),
            (
                "_DNSServiceProcessResult",
                DNSServiceProcessResult as *const () as u64,
                "DNSServiceProcessResult → stub"
            ),
            (
                "_DNSServiceRefDeallocate",
                DNSServiceRefDeallocate as *const () as u64,
                "DNSServiceRefDeallocate → stub"
            ),
            (
                "_TXTRecordCreate",
                TXTRecordCreate as *const () as u64,
                "TXTRecordCreate → stub"
            ),
            (
                "_TXTRecordDeallocate",
                TXTRecordDeallocate as *const () as u64,
                "TXTRecordDeallocate → stub"
            ),
            (
                "_TXTRecordSetValue",
                TXTRecordSetValue as *const () as u64,
                "TXTRecordSetValue → stub"
            ),
            (
                "_TXTRecordGetLength",
                TXTRecordGetLength as *const () as u64,
                "TXTRecordGetLength → stub"
            ),
            (
                "_TXTRecordGetBytesPtr",
                TXTRecordGetBytesPtr as *const () as u64,
                "TXTRecordGetBytesPtr → stub"
            )
        );
        log::debug!("DnsSdStubs: registered 11 shims");
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn browse_register_resolve_fd() {
        let _g = TEST_LOCK.lock().unwrap();
        let mut sd_ref: u64 = 0;
        assert_eq!(
            DNSServiceBrowse(&mut sd_ref, 0, 0, std::ptr::null(), std::ptr::null(), 0, 0),
            0
        );
        assert!(sd_ref != 0);
        let fd = DNSServiceRefSockFD(sd_ref);
        assert!(fd >= 0);
        assert_eq!(DNSServiceProcessResult(sd_ref), 0); // No callback registered.
        let monitor = service_refs()
            .lock()
            .unwrap()
            .get(&sd_ref)
            .unwrap()
            .fd
            .try_clone()
            .unwrap();
        let mut poll_fd = libc::pollfd {
            fd: monitor.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // A live service must retain its writer instead of reporting EOF.
        assert_eq!(unsafe { libc::poll(&mut poll_fd, 1, 0) }, 0);
        DNSServiceRefDeallocate(sd_ref);
        assert_eq!(DNSServiceRefSockFD(sd_ref), -1);
        assert_eq!(DNSServiceProcessResult(sd_ref), 0); // Unknown handle remains a no-op.
                                                        // The duplicate read end observes closure without depending on raw fd
                                                        // numbers staying unused while other tests allocate descriptors.
        assert_eq!(unsafe { libc::poll(&mut poll_fd, 1, 0) }, 1);
        assert_ne!(poll_fd.revents & libc::POLLHUP, 0);
    }

    #[test]
    fn process_result_invokes_callback() {
        let _g = TEST_LOCK.lock().unwrap();
        static mut CALLED: bool = false;
        extern "C" fn cb(
            _sd_ref: u64,
            _flags: u32,
            _iface: u32,
            _err: u32,
            _fullname: *const u8,
            _host: *const u8,
            _regtype: *const u8,
            _domain: *const u8,
            _port: u16,
            _txt: *const u8,
            _context: u64,
        ) {
            unsafe {
                CALLED = true;
            }
        }
        let mut sd_ref: u64 = 0;
        assert_eq!(
            DNSServiceBrowse(
                &mut sd_ref,
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                cb as *const () as u64,
                0
            ),
            0
        );
        assert_eq!(DNSServiceProcessResult(sd_ref), 0);
        unsafe {
            assert!(CALLED);
        }
        DNSServiceRefDeallocate(sd_ref);
    }

    #[test]
    fn callback_can_query_and_deallocate_its_service() {
        let _g = TEST_LOCK.lock().unwrap();
        static CALLBACK_OK: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        CALLBACK_OK.store(false, Ordering::SeqCst);

        extern "C" fn callback(
            sd_ref: u64,
            _flags: u32,
            _iface: u32,
            _err: u32,
            _fullname: *const u8,
            _host: *const u8,
            _regtype: *const u8,
            _domain: *const u8,
            _port: u16,
            _txt: *const u8,
            _context: u64,
        ) {
            // Fail outside the C callback instead of hanging or unwinding
            // through the ABI if ProcessResult still holds the registry lock.
            let Ok(guard) = service_refs().try_lock() else {
                return;
            };
            drop(guard);

            let was_live = DNSServiceRefSockFD(sd_ref) >= 0;
            DNSServiceRefDeallocate(sd_ref);
            CALLBACK_OK.store(
                was_live && DNSServiceRefSockFD(sd_ref) == -1,
                Ordering::SeqCst,
            );
        }

        let mut sd_ref = 0;
        assert_eq!(
            DNSServiceBrowse(
                &mut sd_ref,
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                callback as *const () as u64,
                0,
            ),
            0,
        );
        let monitor = service_refs()
            .lock()
            .unwrap()
            .get(&sd_ref)
            .unwrap()
            .fd
            .try_clone()
            .unwrap();

        let status = DNSServiceProcessResult(sd_ref);
        // Also clean up when the callback could not acquire the lock.
        DNSServiceRefDeallocate(sd_ref);
        assert_eq!(status, 0);
        assert!(CALLBACK_OK.load(Ordering::SeqCst));
        assert_eq!(DNSServiceRefSockFD(sd_ref), -1);

        let mut poll_fd = libc::pollfd {
            fd: monitor.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(unsafe { libc::poll(&mut poll_fd, 1, 0) }, 1);
        assert_ne!(poll_fd.revents & libc::POLLHUP, 0);
    }
}
