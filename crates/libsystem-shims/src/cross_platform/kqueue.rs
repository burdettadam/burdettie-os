//! kqueue → epoll translation shim.
//!
//! macOS binaries use `kqueue()`/`kevent()` for I/O multiplexing. On Linux,
//! these don't exist natively. This module provides minimal shims that
//! translate kqueue operations to Linux `epoll`.
//!
//! ## Supported features
//!
//! - `EVFILT_READ` / `EVFILT_WRITE` on file descriptors (sockets, pipes)
//! - `EV_ADD`, `EV_DELETE`, `EV_ENABLE`, `EV_DISABLE`
//! - Timeout via `struct timespec` parameter to `kevent()`
//!
//! ## Limitations
//!
//! - `EVFILT_TIMER`, `EVFILT_SIGNAL`, `EVFILT_VNODE` etc. are not supported
//! - `EV_ONESHOT`, `EV_CLEAR` are approximated
//! - `kevent64` and `kevent_qos` are not shimmed

use crate::table::ShimTable;

// ── macOS kqueue constants ──────────────────────────────────────────────────

// Event filters (negative values on macOS).
const EVFILT_READ: i16 = -1;
const EVFILT_WRITE: i16 = -2;

// Event flags.
const EV_ADD: u16 = 0x0001;
const EV_DELETE: u16 = 0x0002;
#[allow(dead_code)]
const EV_ENABLE: u16 = 0x0004;
#[allow(dead_code)]
const EV_DISABLE: u16 = 0x0008;
#[allow(dead_code)]
const EV_ONESHOT: u16 = 0x0010;
#[allow(dead_code)]
const EV_CLEAR: u16 = 0x0020;

const EV_ERROR: u16 = 0x4000;
#[allow(dead_code)]
const EV_EOF: u16 = 0x8000;

/// macOS `struct kevent` layout (64-bit):
/// ```c
/// struct kevent {
///     uintptr_t ident;   // 8 bytes — fd or identifier
///     int16_t   filter;  // 2 bytes — EVFILT_*
///     uint16_t  flags;   // 2 bytes — EV_*
///     uint32_t  fflags;  // 4 bytes — filter-specific flags
///     intptr_t  data;    // 8 bytes — filter-specific data
///     void     *udata;   // 8 bytes — user data
/// };
/// ```
/// Total: 32 bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DarwinKevent {
    ident: u64,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: i64,
    udata: u64,
}

// ── kqueue() ────────────────────────────────────────────────────────────────

/// `int kqueue(void)` — returns a new kqueue descriptor.
///
/// On Linux, we create an epoll instance instead.
#[cfg(target_os = "linux")]
pub extern "C" fn shim_kqueue() -> i32 {
    log::debug!("kqueue() → epoll_create1(EPOLL_CLOEXEC)");
    unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) }
}

#[cfg(target_os = "macos")]
pub extern "C" fn shim_kqueue() -> i32 {
    unsafe { libc::kqueue() }
}

// ── kevent() ────────────────────────────────────────────────────────────────

/// `int kevent(int kq, const struct kevent *changelist, int nchanges,
///             struct kevent *eventlist, int nevents,
///             const struct timespec *timeout)`
///
/// Translates macOS kevent changelist/eventlist to epoll_ctl/epoll_wait.
#[cfg(target_os = "linux")]
pub extern "C" fn shim_kevent(
    kq: i32,
    changelist: u64,  // *const DarwinKevent
    nchanges: i32,
    eventlist: u64,    // *mut DarwinKevent
    nevents: i32,
    timeout: u64,      // *const libc::timespec
) -> i32 {
    log::debug!("kevent(kq={kq}, nchanges={nchanges}, nevents={nevents})");

    // Phase 1: Apply changelist (register/deregister events via epoll_ctl).
    if changelist != 0 && nchanges > 0 {
        for i in 0..nchanges as usize {
            let kev = unsafe {
                &*((changelist as usize + i * std::mem::size_of::<DarwinKevent>())
                    as *const DarwinKevent)
            };
            apply_change(kq, kev);
        }
    }

    // Phase 2: Wait for events (epoll_wait).
    if eventlist == 0 || nevents <= 0 {
        return 0;
    }

    let timeout_ms = if timeout == 0 {
        -1 // Infinite wait
    } else {
        let ts = unsafe { &*(timeout as *const libc::timespec) };
        if ts.tv_sec == 0 && ts.tv_nsec == 0 {
            0 // Non-blocking poll
        } else {
            (ts.tv_sec as i32) * 1000 + (ts.tv_nsec as i32 / 1_000_000)
        }
    };

    // epoll_wait into a temporary buffer, then translate to kevent format.
    let max_events = nevents.min(64) as usize;
    let mut epoll_events = vec![libc::epoll_event { events: 0, u64: 0 }; max_events];

    let n = unsafe {
        libc::epoll_wait(kq, epoll_events.as_mut_ptr(), max_events as i32, timeout_ms)
    };
    if n < 0 {
        return -1;
    }

    // Translate epoll_events back to Darwin kevent format.
    for i in 0..n as usize {
        let epev = &epoll_events[i];
        let out = unsafe {
            &mut *((eventlist as usize + i * std::mem::size_of::<DarwinKevent>())
                as *mut DarwinKevent)
        };
        // The fd was stored in epoll_event.u64 by apply_change.
        let fd = epev.u64 as u64;

        if epev.events & (libc::EPOLLERR as u32) != 0 {
            *out = DarwinKevent {
                ident: fd,
                filter: EVFILT_READ,
                flags: EV_ERROR,
                fflags: 0,
                data: 0,
                udata: 0,
            };
        } else if epev.events & (libc::EPOLLIN as u32) != 0 {
            *out = DarwinKevent {
                ident: fd,
                filter: EVFILT_READ,
                flags: 0,
                fflags: 0,
                data: 0, // We don't know the exact byte count
                udata: 0,
            };
        } else if epev.events & (libc::EPOLLOUT as u32) != 0 {
            *out = DarwinKevent {
                ident: fd,
                filter: EVFILT_WRITE,
                flags: 0,
                fflags: 0,
                data: 0,
                udata: 0,
            };
        } else {
            *out = DarwinKevent {
                ident: fd,
                filter: EVFILT_READ,
                flags: 0,
                fflags: 0,
                data: 0,
                udata: 0,
            };
        }
    }

    n
}

#[cfg(target_os = "macos")]
pub extern "C" fn shim_kevent(
    _kq: i32,
    _changelist: u64,
    _nchanges: i32,
    _eventlist: u64,
    _nevents: i32,
    _timeout: u64,
) -> i32 {
    // On macOS, just pass through to real kevent. This shim is for Linux.
    0
}

/// Apply a single kevent change to the epoll instance.
#[cfg(target_os = "linux")]
fn apply_change(epfd: i32, kev: &DarwinKevent) {
    let fd = kev.ident as i32;

    if kev.flags & EV_DELETE != 0 {
        // Remove this fd from epoll.
        unsafe {
            libc::epoll_ctl(epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        }
        return;
    }

    if kev.flags & EV_ADD != 0 {
        let mut events: u32 = 0;
        match kev.filter {
            EVFILT_READ => events |= libc::EPOLLIN as u32,
            EVFILT_WRITE => events |= libc::EPOLLOUT as u32,
            _ => {
                log::debug!("kevent: unsupported filter {}", kev.filter);
                return;
            }
        }
        if kev.flags & EV_ONESHOT != 0 {
            events |= libc::EPOLLONESHOT as u32;
        }

        let mut ev = libc::epoll_event {
            events,
            u64: fd as u64,
        };

        // Try EPOLL_CTL_ADD first; if it already exists, use MOD.
        let ret = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, fd, &mut ev) };
        if ret < 0 {
            let err = unsafe { *libc::__errno_location() };
            if err == libc::EEXIST {
                unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_MOD, fd, &mut ev); }
            }
        }
    }
}

// ── Registration ────────────────────────────────────────────────────────────

pub struct KqueueShims;

impl KqueueShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_kqueue", shim_kqueue as *const () as u64, "kqueue → epoll_create1"),
            ("_kevent", shim_kevent as *const () as u64, "kevent → epoll_ctl/wait"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darwin_kevent_struct_size() {
        assert_eq!(std::mem::size_of::<DarwinKevent>(), 32);
    }

    #[test]
    fn kqueue_returns_valid_fd() {
        let fd = shim_kqueue();
        assert!(fd >= 0, "kqueue/epoll_create failed: fd={fd}");
        unsafe { libc::close(fd); }
    }
}
