//! dns_sd.h (Bonjour / mDNSResponder) stub shims for Linux.
//!
//! Implements DNSServiceBrowse, DNSServiceRegister, DNSServiceResolve, TXTRecord* helpers.
//! Service refs are opaque handles with pipe-based fds for event loop integration.
//! No real network discovery; all responses are synthetic.

use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::table::ShimTable;

// ── ServiceRef state ───────────────────────────────────────────

struct ServiceRef {
    handle: u64,
    fd: RawFd,
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
        (fds[0], fds[1])
    };
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let ref_ = ServiceRef {
        handle,
        fd: fd_read,
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
    txt: *mut u8,
    key: *const u8,
    key_len: u8,
    value_len: u8,
    value: *const u8,
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
    unsafe { *sd_ref = handle; }
    0
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
    unsafe { *sd_ref = handle; }
    0
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
    unsafe { *sd_ref = handle; }
    0
}

#[no_mangle]
pub extern "C" fn DNSServiceRefSockFD(sd_ref: u64) -> i32 {
    let refs = service_refs().lock().unwrap();
    refs.get(&sd_ref).map(|r| r.fd).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn DNSServiceProcessResult(sd_ref: u64) -> i32 {
    // Synchronously invoke callback with synthetic data
    let mut refs = service_refs().lock().unwrap();
    if let Some(r) = refs.get(&sd_ref) {
        // For now, just call the callback with nulls
        if r.callback != 0 {
            let cb: extern "C" fn(u64, u32, u32, u32, *const u8, *const u8, *const u8, *const u8, u16, *const u8, u64) =
                unsafe { std::mem::transmute(r.callback as usize) };
            cb(sd_ref, 0, 0, 0, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, std::ptr::null(), r.context);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn DNSServiceRefDeallocate(sd_ref: u64) {
    let mut refs = service_refs().lock().unwrap();
    if let Some(r) = refs.remove(&sd_ref) {
        unsafe { libc::close(r.fd); }
    }
}

// ── Install ───────────────────────────────────────────────────

pub struct DnsSdStubs;

impl DnsSdStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_DNSServiceBrowse", DNSServiceBrowse as *const () as u64, "DNSServiceBrowse → stub"),
            ("_DNSServiceRegister", DNSServiceRegister as *const () as u64, "DNSServiceRegister → stub"),
            ("_DNSServiceResolve", DNSServiceResolve as *const () as u64, "DNSServiceResolve → stub"),
            ("_DNSServiceRefSockFD", DNSServiceRefSockFD as *const () as u64, "DNSServiceRefSockFD → stub"),
            ("_DNSServiceProcessResult", DNSServiceProcessResult as *const () as u64, "DNSServiceProcessResult → stub"),
            ("_DNSServiceRefDeallocate", DNSServiceRefDeallocate as *const () as u64, "DNSServiceRefDeallocate → stub"),
            ("_TXTRecordCreate", TXTRecordCreate as *const () as u64, "TXTRecordCreate → stub"),
            ("_TXTRecordDeallocate", TXTRecordDeallocate as *const () as u64, "TXTRecordDeallocate → stub"),
            ("_TXTRecordSetValue", TXTRecordSetValue as *const () as u64, "TXTRecordSetValue → stub"),
            ("_TXTRecordGetLength", TXTRecordGetLength as *const () as u64, "TXTRecordGetLength → stub"),
            ("_TXTRecordGetBytesPtr", TXTRecordGetBytesPtr as *const () as u64, "TXTRecordGetBytesPtr → stub")
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
        assert_eq!(DNSServiceBrowse(&mut sd_ref, 0, 0, std::ptr::null(), std::ptr::null(), 0, 0), 0);
        assert!(sd_ref != 0);
        let fd = DNSServiceRefSockFD(sd_ref);
        assert!(fd > 0);
        DNSServiceRefDeallocate(sd_ref);
    }

    #[test]
    fn process_result_invokes_callback() {
        let _g = TEST_LOCK.lock().unwrap();
        static mut CALLED: bool = false;
        extern "C" fn cb(_sd_ref: u64, _flags: u32, _iface: u32, _err: u32, _fullname: *const u8, _host: *const u8, _regtype: *const u8, _domain: *const u8, _port: u16, _txt: *const u8, _context: u64) {
            unsafe { CALLED = true; }
        }
        let mut sd_ref: u64 = 0;
        assert_eq!(DNSServiceBrowse(&mut sd_ref, 0, 0, std::ptr::null(), std::ptr::null(), cb as u64, 0), 0);
        assert_eq!(DNSServiceProcessResult(sd_ref), 0);
        unsafe { assert!(CALLED); }
        DNSServiceRefDeallocate(sd_ref);
    }
}
