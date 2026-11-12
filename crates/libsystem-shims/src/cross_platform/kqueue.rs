//! kqueue → epoll translation shim.
//!
//! macOS binaries use `kqueue()`/`kevent()` for I/O multiplexing. On Linux,
//! these don't exist natively. This module provides minimal shims that
//! translate kqueue operations to Linux `epoll`.
//!
//! ## Supported features
//!
//! - `EVFILT_READ` / `EVFILT_WRITE` on file descriptors (sockets, pipes)
//! - `EVFILT_TIMER` via Linux `timerfd_create`
//! - `EV_ADD`, `EV_DELETE`, `EV_ENABLE`, `EV_DISABLE`
//! - Timeout via `struct timespec` parameter to `kevent()`
//! - `udata` preservation across add/return cycles
//!
//! ## Limitations
//!
//! - `EVFILT_SIGNAL`, `EVFILT_VNODE` etc. are not supported
//! - `EV_CLEAR` is approximated via `EPOLLET`
//! - `kevent64` and `kevent_qos` are not shimmed

use crate::table::ShimTable;
#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_os = "linux")]
use std::sync::Mutex;

// ── macOS kqueue constants ──────────────────────────────────────────────────

// Event filters (negative values on macOS).
const EVFILT_READ: i16 = -1;
const EVFILT_WRITE: i16 = -2;
const EVFILT_TIMER: i16 = -7;
const EVFILT_USER: i16 = -10;

// Event flags.
const EV_ADD: u16 = 0x0001;
const EV_DELETE: u16 = 0x0002;
#[allow(dead_code)]
const EV_ENABLE: u16 = 0x0004;
#[allow(dead_code)]
const EV_DISABLE: u16 = 0x0008;
const EV_ONESHOT: u16 = 0x0010;
const EV_CLEAR: u16 = 0x0020;

const EV_ERROR: u16 = 0x4000;
#[allow(dead_code)]
const EV_EOF: u16 = 0x8000;

// Timer fflags (macOS NOTE_* constants for EVFILT_TIMER).
#[cfg(target_os = "linux")]
const NOTE_SECONDS: u32 = 0x00000001;
#[cfg(target_os = "linux")]
const NOTE_USECONDS: u32 = 0x00000002;
#[cfg(target_os = "linux")]
const NOTE_NSECONDS: u32 = 0x00000004;

// EVFILT_USER fflags.
#[cfg(target_os = "linux")]
const NOTE_TRIGGER: u32 = 0x01000000;

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

// ── Per-kqueue state for udata + timer tracking ─────────────────────────────

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct RegisteredEvent {
    udata: u64,
    filter: i16,
    /// For EVFILT_TIMER: the timerfd that was created (or -1 for socket events).
    timerfd: i32,
}

/// Key: (epoll_fd, ident, filter) → registered event metadata.
/// ident is the fd for READ/WRITE, or an arbitrary identifier for TIMER.
#[cfg(target_os = "linux")]
type EventKey = (i32, u64, i16);

#[cfg(target_os = "linux")]
static EVENT_REGISTRY: Mutex<Option<HashMap<EventKey, RegisteredEvent>>> = Mutex::new(None);

/// Maps timerfd → (epoll_fd, original_ident) for reverse lookup in epoll_wait results.
#[cfg(target_os = "linux")]
static TIMERFD_MAP: Mutex<Option<HashMap<i32, (i32, u64)>>> = Mutex::new(None);

/// Maps eventfd → (epoll_fd, original_ident) for EVFILT_USER reverse lookup.
#[cfg(target_os = "linux")]
static USERFD_MAP: Mutex<Option<HashMap<i32, (i32, u64)>>> = Mutex::new(None);

// ── Per-kqueue lifecycle state for idle detection ───────────────────────────
//
// Tracks whether a kqueue has ever had non-pipe fds (sockets, timers) and how
// many consecutive kevent calls have occurred with only pipe fds remaining.
// This enables a proper exit heuristic for programs like nslookup (BIND 9):
//
// 1. Normal operation: sockets registered → pipe_only_streak stays at 0
// 2. DNS work completes: sockets removed, only self-pipe remains
// 3. First few pipe-only calls: cap timeout to 50ms (allow task cleanup)
// 4. After PIPE_ONLY_SUPPRESS_THRESHOLD: suppress pipe events entirely
//    → forces event loop to see "no activity" → triggers exit path
//
// This replaces the previous simple 50ms heuristic with a proper lifecycle
// model that distinguishes "legitimate pipe-only" from "stuck after sockets".

/// After this many consecutive kevent calls with only pipe fds (and the kqueue
/// previously had non-pipe fds), start suppressing pipe events entirely.
#[cfg(target_os = "linux")]
const PIPE_ONLY_SUPPRESS_THRESHOLD: u32 = 10;

/// Per-kqueue lifecycle state.
#[cfg(target_os = "linux")]
struct KqueueLifecycle {
    /// Whether this kqueue has ever had a non-pipe fd registered
    /// (socket, timerfd, eventfd, etc.).
    had_non_pipe: bool,
    /// Consecutive kevent calls where only pipe fds are registered
    /// (after previously having had non-pipe fds).
    pipe_only_streak: AtomicU32,
}

/// Maps kqueue fd → lifecycle state.
#[cfg(target_os = "linux")]
static KQUEUE_LIFECYCLE: Mutex<Option<HashMap<i32, KqueueLifecycle>>> = Mutex::new(None);

/// Record that a non-pipe fd was registered on this kqueue.
#[cfg(target_os = "linux")]
fn lifecycle_mark_non_pipe(kq: i32) {
    let mut guard = KQUEUE_LIFECYCLE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(kq).or_insert_with(|| KqueueLifecycle {
        had_non_pipe: false,
        pipe_only_streak: AtomicU32::new(0),
    });
    entry.had_non_pipe = true;
    entry.pipe_only_streak.store(0, Ordering::Relaxed);
}

/// Check lifecycle state and update streak counter.
/// Returns `true` if pipe events should be suppressed (stuck pattern detected).
#[cfg(target_os = "linux")]
fn lifecycle_check_pipe_only(kq: i32, is_pipe_only: bool) -> bool {
    let mut guard = KQUEUE_LIFECYCLE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(kq).or_insert_with(|| KqueueLifecycle {
        had_non_pipe: false,
        pipe_only_streak: AtomicU32::new(0),
    });

    if !is_pipe_only || !entry.had_non_pipe {
        // Not in pipe-only mode, or never had sockets — reset and allow normal behavior
        entry.pipe_only_streak.store(0, Ordering::Relaxed);
        return false;
    }

    // Pipe-only after having had sockets — increment streak
    let streak = entry.pipe_only_streak.fetch_add(1, Ordering::Relaxed) + 1;
    streak >= PIPE_ONLY_SUPPRESS_THRESHOLD
}

#[cfg(target_os = "linux")]
fn registry_insert(key: EventKey, val: RegisteredEvent) {
    let mut guard = EVENT_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(HashMap::new).insert(key, val);
}

#[cfg(target_os = "linux")]
fn registry_lookup(key: &EventKey) -> Option<RegisteredEvent> {
    let guard = EVENT_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.get(key).copied()
}

#[cfg(target_os = "linux")]
fn registry_remove(key: &EventKey) -> Option<RegisteredEvent> {
    let mut guard = EVENT_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.remove(key)
}

/// Count how many events are registered for a given epoll/kqueue fd.
#[cfg(target_os = "linux")]
fn registry_count_for_kq(kq: i32) -> usize {
    let guard = EVENT_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(map) => map.keys().filter(|(epfd, _, _)| *epfd == kq).count(),
        None => 0,
    }
}

/// Check if all registered fds for a kqueue are pipes (not sockets).
///
/// Programs like nslookup (BIND 9 ISC event library) use a self-pipe for
/// inter-thread wakeup.  After DNS work completes, the DNS socket is
/// deregistered but the self-pipe read end stays in the kqueue.  If we
/// block with `epoll_wait(-1)` and nothing will ever write to the pipe
/// again, the program hangs forever.
///
/// When *only* pipe fds remain, we cap the timeout so the event loop can
/// iterate and check its own "shutting_down" / "done" flag.
#[cfg(target_os = "linux")]
fn registry_only_pipes_for_kq(kq: i32) -> bool {
    let guard = EVENT_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let map = match guard.as_ref() {
        Some(m) => m,
        None => return false,
    };
    let mut count = 0usize;
    for ((epfd, ident, filter), reg) in map.iter() {
        if *epfd != kq {
            continue;
        }
        // Timer entries have their own timerfd — not a pipe.
        if reg.timerfd >= 0 || *filter == EVFILT_TIMER {
            return false;
        }
        // EVFILT_USER entries use eventfd — not a pipe.
        if *filter == EVFILT_USER {
            return false;
        }
        let fd = *ident as i32;
        // Use fstat to check if the fd is a pipe (S_IFIFO).
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::fstat(fd, &mut stat) };
        if ret != 0 || (stat.st_mode & libc::S_IFMT) != libc::S_IFIFO {
            return false;
        }
        count += 1;
    }
    count > 0
}

#[cfg(target_os = "linux")]
fn timerfd_map_insert(tfd: i32, epfd: i32, ident: u64) {
    let mut guard = TIMERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(tfd, (epfd, ident));
}

#[cfg(target_os = "linux")]
fn timerfd_map_remove(tfd: i32) -> Option<(i32, u64)> {
    let mut guard = TIMERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.remove(&tfd)
}

#[cfg(target_os = "linux")]
fn timerfd_map_lookup_by_fd(tfd: i32) -> Option<(i32, u64)> {
    let guard = TIMERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.get(&tfd).copied()
}

#[cfg(target_os = "linux")]
fn userfd_map_insert(ufd: i32, epfd: i32, ident: u64) {
    let mut guard = USERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(ufd, (epfd, ident));
}

#[cfg(target_os = "linux")]
fn userfd_map_remove(ufd: i32) -> Option<(i32, u64)> {
    let mut guard = USERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut()?.remove(&ufd)
}

#[cfg(target_os = "linux")]
fn userfd_map_lookup_by_fd(ufd: i32) -> Option<(i32, u64)> {
    let guard = USERFD_MAP.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.get(&ufd).copied()
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
    changelist: u64, // *const DarwinKevent
    nchanges: i32,
    eventlist: u64, // *mut DarwinKevent
    nevents: i32,
    timeout: u64, // *const libc::timespec
) -> i32 {
    log::debug!(
        "kevent(kq={kq}, nchanges={nchanges}, nevents={nevents}, timeout_ptr={timeout:#x})"
    );

    // Phase 1: Apply changelist (register/deregister events via epoll_ctl).
    if changelist != 0 && nchanges > 0 {
        for i in 0..nchanges as usize {
            let kev = unsafe {
                &*((changelist as usize + i * std::mem::size_of::<DarwinKevent>())
                    as *const DarwinKevent)
            };
            log::debug!(
                "  changelist[{i}]: ident={} filter={} flags={:#06x} udata={:#x}",
                kev.ident,
                kev.filter,
                kev.flags,
                kev.udata
            );
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

    // If no events are registered on this kqueue and we'd block forever,
    // return 0 immediately. This prevents tools like nslookup from hanging
    // indefinitely in an empty event loop after their work is done.
    if timeout_ms != 0 && registry_count_for_kq(kq) == 0 {
        log::debug!("  kevent: no events registered on kq={kq}, returning 0 (idle)");
        return 0;
    }

    // ── Pipe-only lifecycle detection ───────────────────────────────
    //
    // Programs like nslookup (BIND 9) use kqueue with both network sockets
    // and a self-pipe for inter-thread wakeup.  After DNS work completes:
    //   - Network sockets are removed from the kqueue
    //   - The self-pipe remains, and a worker thread keeps writing to it
    //   - The event loop processes pipe events indefinitely
    //
    // Strategy:
    //   Phase 1 (streak < threshold): Cap infinite → 50ms timeout.
    //     This gives the app a chance to discover it has no more work.
    //   Phase 2 (streak >= threshold): Suppress pipe events entirely.
    //     After 10 iterations with no socket activity, the app is stuck.
    //     Stop delivering pipe events to force the "nothing happened" path.
    let pipe_only = timeout_ms == -1 && registry_only_pipes_for_kq(kq);
    let suppress_pipe_events = lifecycle_check_pipe_only(kq, pipe_only);

    if suppress_pipe_events {
        log::debug!("  kevent: pipe-only streak exceeded threshold on kq={kq}, suppressing events");
        // Sleep briefly for CPU courtesy, then return 0 events.
        // This forces the event loop to see "nothing happened" and check
        // its exit condition rather than perpetually processing pipe wakeups.
        unsafe { libc::usleep(50_000) }; // 50ms
        return 0;
    }

    let timeout_ms = if pipe_only {
        log::debug!("  kevent: only pipe fds remain on kq={kq}, capping timeout to 50ms");
        50
    } else {
        timeout_ms
    };

    log::debug!("  epoll_wait(kq={kq}, timeout_ms={timeout_ms})");

    // epoll_wait into a temporary buffer, then translate to kevent format.
    let max_events = nevents.min(64) as usize;
    let mut epoll_events = vec![libc::epoll_event { events: 0, u64: 0 }; max_events];

    let n =
        unsafe { libc::epoll_wait(kq, epoll_events.as_mut_ptr(), max_events as i32, timeout_ms) };
    log::debug!("  epoll_wait returned {n}");
    if n < 0 {
        return -1;
    }

    // Translate epoll_events back to Darwin kevent format.
    // A single epoll event with EPOLLIN|EPOLLOUT may produce two Darwin
    // kevents (one READ + one WRITE), so we track output position separately.
    let mut out_idx: usize = 0;
    let out_max = nevents as usize;

    for i in 0..n as usize {
        if out_idx >= out_max {
            break;
        }
        let epev = &epoll_events[i];
        let fd = epev.u64 as i32;

        // ── timerfd → EVFILT_TIMER ──────────────────────────────────
        if let Some((_epfd, original_ident)) = timerfd_map_lookup_by_fd(fd) {
            // Drain the timerfd so it doesn't re-fire immediately.
            let mut buf = [0u8; 8];
            unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 8) };

            let udata = registry_lookup(&(kq, original_ident, EVFILT_TIMER))
                .map(|r| r.udata)
                .unwrap_or(0);

            let out = unsafe {
                &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                    as *mut DarwinKevent)
            };
            *out = DarwinKevent {
                ident: original_ident,
                filter: EVFILT_TIMER,
                flags: 0,
                fflags: 0,
                data: 1,
                udata,
            };
            out_idx += 1;
            log::debug!(
                "  event[{out_idx}]: timerfd {fd} → EVFILT_TIMER ident={original_ident} udata={udata:#x}"
            );
            continue;
        }

        // ── eventfd → EVFILT_USER ───────────────────────────────────
        if let Some((_epfd, original_ident)) = userfd_map_lookup_by_fd(fd) {
            // Drain the eventfd so it doesn't re-fire immediately.
            let mut buf = [0u8; 8];
            unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 8) };

            let udata = registry_lookup(&(kq, original_ident, EVFILT_USER))
                .map(|r| r.udata)
                .unwrap_or(0);

            let out = unsafe {
                &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                    as *mut DarwinKevent)
            };
            *out = DarwinKevent {
                ident: original_ident,
                filter: EVFILT_USER,
                flags: 0,
                fflags: 0,
                data: 0,
                udata,
            };
            out_idx += 1;
            log::debug!(
                "  event[{out_idx}]: eventfd {fd} → EVFILT_USER ident={original_ident} udata={udata:#x}"
            );
            continue;
        }

        // ── socket/pipe fd events ───────────────────────────────────
        let fd_u64 = fd as u64;

        if epev.events & (libc::EPOLLERR as u32) != 0 {
            let udata = registry_lookup(&(kq, fd_u64, EVFILT_READ))
                .map(|r| r.udata)
                .unwrap_or(0);
            let out = unsafe {
                &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                    as *mut DarwinKevent)
            };
            *out = DarwinKevent {
                ident: fd_u64,
                filter: EVFILT_READ,
                flags: EV_ERROR,
                fflags: 0,
                data: 0,
                udata,
            };
            out_idx += 1;
        } else {
            // Emit separate events for READ and WRITE (macOS kevent semantics).
            if epev.events & (libc::EPOLLIN as u32) != 0 && out_idx < out_max {
                let udata = registry_lookup(&(kq, fd_u64, EVFILT_READ))
                    .map(|r| r.udata)
                    .unwrap_or(0);
                // macOS kevent reports available bytes in `data` for EVFILT_READ.
                let mut nbytes: i32 = 0;
                unsafe { libc::ioctl(fd, libc::FIONREAD, &mut nbytes) };
                let out = unsafe {
                    &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                        as *mut DarwinKevent)
                };
                *out = DarwinKevent {
                    ident: fd_u64,
                    filter: EVFILT_READ,
                    flags: 0,
                    fflags: 0,
                    data: nbytes as i64,
                    udata,
                };
                out_idx += 1;
            }
            if epev.events & (libc::EPOLLOUT as u32) != 0 && out_idx < out_max {
                let udata = registry_lookup(&(kq, fd_u64, EVFILT_WRITE))
                    .map(|r| r.udata)
                    .unwrap_or(0);
                let out = unsafe {
                    &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                        as *mut DarwinKevent)
                };
                *out = DarwinKevent {
                    ident: fd_u64,
                    filter: EVFILT_WRITE,
                    flags: 0,
                    fflags: 0,
                    data: 0,
                    udata,
                };
                out_idx += 1;
            }
            // If neither EPOLLIN nor EPOLLOUT set, emit a generic READ event.
            if epev.events & ((libc::EPOLLIN | libc::EPOLLOUT) as u32) == 0 && out_idx < out_max {
                let udata = registry_lookup(&(kq, fd_u64, EVFILT_READ))
                    .map(|r| r.udata)
                    .unwrap_or(0);
                let out = unsafe {
                    &mut *((eventlist as usize + out_idx * std::mem::size_of::<DarwinKevent>())
                        as *mut DarwinKevent)
                };
                *out = DarwinKevent {
                    ident: fd_u64,
                    filter: EVFILT_READ,
                    flags: 0,
                    fflags: 0,
                    data: 0,
                    udata,
                };
                out_idx += 1;
            }
        }
    }

    log::debug!("  kevent returning {out_idx} events (epoll had {n})");
    out_idx as i32
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
    if kev.flags & EV_DELETE != 0 {
        // ── Remove ──────────────────────────────────────────────────
        if kev.filter == EVFILT_TIMER {
            // Clean up the timerfd for this timer ident.
            if let Some(reg) = registry_remove(&(epfd, kev.ident, EVFILT_TIMER)) {
                if reg.timerfd >= 0 {
                    unsafe {
                        libc::epoll_ctl(
                            epfd,
                            libc::EPOLL_CTL_DEL,
                            reg.timerfd,
                            std::ptr::null_mut(),
                        );
                        libc::close(reg.timerfd);
                    }
                    timerfd_map_remove(reg.timerfd);
                }
            }
        } else if kev.filter == EVFILT_USER {
            // Clean up the eventfd for this user ident.
            if let Some(reg) = registry_remove(&(epfd, kev.ident, EVFILT_USER)) {
                if reg.timerfd >= 0 {
                    // timerfd field repurposed as eventfd for EVFILT_USER.
                    unsafe {
                        libc::epoll_ctl(
                            epfd,
                            libc::EPOLL_CTL_DEL,
                            reg.timerfd,
                            std::ptr::null_mut(),
                        );
                        libc::close(reg.timerfd);
                    }
                    userfd_map_remove(reg.timerfd);
                }
            }
        } else {
            let fd = kev.ident as i32;
            registry_remove(&(epfd, kev.ident, kev.filter));
            unsafe {
                libc::epoll_ctl(epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
            }
        }
        return;
    }

    // ── EVFILT_USER NOTE_TRIGGER (no EV_ADD/EV_DELETE) ──────────────
    // When an already-registered user event is triggered, write to its
    // eventfd so the next epoll_wait wakes up and delivers the event.
    if kev.filter == EVFILT_USER && kev.fflags & NOTE_TRIGGER != 0 {
        if let Some(reg) = registry_lookup(&(epfd, kev.ident, EVFILT_USER)) {
            if reg.timerfd >= 0 {
                let val: u64 = 1;
                unsafe {
                    libc::write(reg.timerfd, &val as *const u64 as *const libc::c_void, 8);
                }
                log::debug!(
                    "  EVFILT_USER: triggered ident={} (eventfd={})",
                    kev.ident,
                    reg.timerfd
                );
            }
        }
        return;
    }

    if kev.flags & EV_ADD != 0 {
        match kev.filter {
            // ── EVFILT_READ / EVFILT_WRITE → epoll ──────────────────
            EVFILT_READ | EVFILT_WRITE => {
                let fd = kev.ident as i32;

                // Track socket lifecycle: if this fd is NOT a pipe, mark
                // the kqueue as having had non-pipe activity.  This enables
                // the pipe-only idle detection in the wait phase.
                {
                    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                    let ret = unsafe { libc::fstat(fd, &mut stat) };
                    if ret == 0 && (stat.st_mode & libc::S_IFMT) != libc::S_IFIFO {
                        lifecycle_mark_non_pipe(epfd);
                    }
                }

                let mut events: u32 = match kev.filter {
                    EVFILT_READ => libc::EPOLLIN as u32,
                    EVFILT_WRITE => libc::EPOLLOUT as u32,
                    _ => unreachable!(),
                };
                if kev.flags & EV_ONESHOT != 0 {
                    events |= libc::EPOLLONESHOT as u32;
                }
                if kev.flags & EV_CLEAR != 0 {
                    events |= libc::EPOLLET as u32;
                }

                // Store udata so we can return it when the event fires.
                registry_insert(
                    (epfd, kev.ident, kev.filter),
                    RegisteredEvent {
                        udata: kev.udata,
                        filter: kev.filter,
                        timerfd: -1,
                    },
                );

                let mut ev = libc::epoll_event {
                    events,
                    u64: fd as u64,
                };

                // Try EPOLL_CTL_ADD first; if it already exists, use MOD.
                let ret = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, fd, &mut ev) };
                if ret < 0 {
                    let err = unsafe { *libc::__errno_location() };
                    if err == libc::EEXIST {
                        unsafe {
                            libc::epoll_ctl(epfd, libc::EPOLL_CTL_MOD, fd, &mut ev);
                        }
                    }
                }
            }

            // ── EVFILT_TIMER → timerfd + epoll ──────────────────────
            EVFILT_TIMER => {
                // Timers are non-pipe activity — mark lifecycle.
                lifecycle_mark_non_pipe(epfd);

                // Convert kev.data to nanoseconds based on fflags.
                let ns: u64 = if kev.fflags & NOTE_NSECONDS != 0 {
                    kev.data as u64
                } else if kev.fflags & NOTE_USECONDS != 0 {
                    (kev.data as u64).saturating_mul(1_000)
                } else if kev.fflags & NOTE_SECONDS != 0 {
                    (kev.data as u64).saturating_mul(1_000_000_000)
                } else {
                    // Default: milliseconds.
                    (kev.data as u64).saturating_mul(1_000_000)
                };

                let secs = (ns / 1_000_000_000) as i64;
                let nsecs = (ns % 1_000_000_000) as i64;

                log::debug!(
                    "  EVFILT_TIMER: ident={} data={} → {}s {}ns oneshot={}",
                    kev.ident,
                    kev.data,
                    secs,
                    nsecs,
                    kev.flags & EV_ONESHOT != 0
                );

                // Create a timerfd.
                let tfd = unsafe {
                    libc::timerfd_create(
                        libc::CLOCK_MONOTONIC,
                        libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
                    )
                };
                if tfd < 0 {
                    log::warn!("timerfd_create failed: {}", std::io::Error::last_os_error());
                    return;
                }

                // Arm the timer. If EV_ONESHOT, use it_interval = 0 (one-shot).
                // Otherwise, make it repeating.
                let is_oneshot = kev.flags & EV_ONESHOT != 0;
                let interval = if is_oneshot {
                    libc::timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    }
                } else {
                    libc::timespec {
                        tv_sec: secs,
                        tv_nsec: nsecs,
                    }
                };
                let spec = libc::itimerspec {
                    it_interval: interval,
                    it_value: libc::timespec {
                        tv_sec: secs,
                        tv_nsec: nsecs,
                    },
                };
                let ret = unsafe { libc::timerfd_settime(tfd, 0, &spec, std::ptr::null_mut()) };
                if ret < 0 {
                    log::warn!(
                        "timerfd_settime failed: {}",
                        std::io::Error::last_os_error()
                    );
                    unsafe {
                        libc::close(tfd);
                    }
                    return;
                }

                // Register the timerfd with epoll so epoll_wait sees it.
                let mut ev = libc::epoll_event {
                    events: libc::EPOLLIN as u32,
                    u64: tfd as u64,
                };
                let ret = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, tfd, &mut ev) };
                if ret < 0 {
                    log::warn!(
                        "epoll_ctl(timer) failed: {}",
                        std::io::Error::last_os_error()
                    );
                    unsafe {
                        libc::close(tfd);
                    }
                    return;
                }

                // Store metadata for reverse lookup.
                registry_insert(
                    (epfd, kev.ident, EVFILT_TIMER),
                    RegisteredEvent {
                        udata: kev.udata,
                        filter: EVFILT_TIMER,
                        timerfd: tfd,
                    },
                );
                timerfd_map_insert(tfd, epfd, kev.ident);
            }

            // ── EVFILT_USER → eventfd + epoll ───────────────────────
            EVFILT_USER => {
                // User events are non-pipe activity — mark lifecycle.
                lifecycle_mark_non_pipe(epfd);
                log::debug!(
                    "  EVFILT_USER: ident={} fflags={:#x} udata={:#x}",
                    kev.ident,
                    kev.fflags,
                    kev.udata
                );

                // Create an eventfd (semaphore mode, non-blocking).
                let efd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
                if efd < 0 {
                    log::warn!("eventfd failed: {}", std::io::Error::last_os_error());
                    return;
                }

                // Register the eventfd with epoll.
                let mut ev = libc::epoll_event {
                    events: libc::EPOLLIN as u32,
                    u64: efd as u64,
                };
                let ret = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, efd, &mut ev) };
                if ret < 0 {
                    log::warn!(
                        "epoll_ctl(user) failed: {}",
                        std::io::Error::last_os_error()
                    );
                    unsafe {
                        libc::close(efd);
                    }
                    return;
                }

                // Store metadata — reuse timerfd field for the eventfd.
                registry_insert(
                    (epfd, kev.ident, EVFILT_USER),
                    RegisteredEvent {
                        udata: kev.udata,
                        filter: EVFILT_USER,
                        timerfd: efd,
                    },
                );
                userfd_map_insert(efd, epfd, kev.ident);

                // If NOTE_TRIGGER is set on the add call itself, fire immediately.
                if kev.fflags & NOTE_TRIGGER != 0 {
                    let val: u64 = 1;
                    unsafe {
                        libc::write(efd, &val as *const u64 as *const libc::c_void, 8);
                    }
                    log::debug!("  EVFILT_USER: triggered on add (ident={})", kev.ident);
                }
            }

            _ => {
                log::debug!("kevent: unsupported filter {}", kev.filter);
            }
        }
    }
}

// ── Registration ────────────────────────────────────────────────────────────

pub struct KqueueShims;

impl KqueueShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_kqueue",
                shim_kqueue as *const () as u64,
                "kqueue → epoll_create1"
            ),
            (
                "_kevent",
                shim_kevent as *const () as u64,
                "kevent → epoll_ctl/wait"
            ),
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
        unsafe {
            libc::close(fd);
        }
    }
}
