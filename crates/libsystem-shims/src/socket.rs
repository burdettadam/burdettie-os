//! POSIX socket syscall interception shims.
//!
//! These shims sit in front of the twelve core BSD socket calls so that raw
//! network activity from emulated binaries is accounted for and policy-checked
//! before reaching the host network stack.
//!
//! Policy:
//!   - `AF_UNIX` (local IPC / XPC): always allowed
//!   - `AF_INET`  to `127.0.0.1` / `0.0.0.0` (bind): allowed
//!   - `AF_INET6` to `::1` / `::` (bind): allowed
//!   - Everything else: denied with `EOPNOTSUPP`
//!
//! Address validation happens at `connect()` / `bind()` / `sendto()` time.
//! `socket()` permits creation of AF_UNIX/AF_INET/AF_INET6 sockets since the
//! destination is not yet known.  Post-validation calls (`send`, `recv`,
//! `listen`, `accept`, etc.) pass through unconditionally.

use crate::table::ShimTable;

// ── errno helpers ───────────────────────────────────────────────────────────

/// Cross-platform errno pointer (macOS: __error, Linux: __errno_location).
#[cfg(target_os = "macos")]
unsafe fn errno_ptr() -> *mut i32 {
    extern "C" {
        fn __error() -> *mut i32;
    }
    unsafe { __error() }
}
#[cfg(target_os = "linux")]
unsafe fn errno_ptr() -> *mut i32 {
    unsafe { libc::__errno_location() }
}

/// Set errno to `EOPNOTSUPP` and return `-1` (i32).
fn deny_i32() -> i32 {
    unsafe { *errno_ptr() = libc::EOPNOTSUPP; }
    -1
}

/// Set errno to `EOPNOTSUPP` and return `-1` (i64).
fn deny_i64() -> i64 {
    unsafe { *errno_ptr() = libc::EOPNOTSUPP; }
    -1
}

// ── Address policy helpers ──────────────────────────────────────────────────

/// Read the `sa_family` field from a raw `sockaddr` pointer.
///
/// Uses `libc::sockaddr` so the platform-specific layout (macOS has a
/// leading `sa_len` byte, Linux does not) is handled by the C definition.
fn read_sa_family(addr: u64) -> u16 {
    if addr == 0 {
        return 0;
    }
    let sa = addr as *const libc::sockaddr;
    unsafe { (*sa).sa_family as u16 }
}

/// Return `true` if the address is allowed by our localhost-only policy.
///
/// - `AF_UNIX`: always allowed (local IPC).
/// - `AF_INET`: allowed only for `127.0.0.1` or `INADDR_ANY` (`0.0.0.0`,
///   used by `bind`).
/// - `AF_INET6`: allowed only for `::1` or `::` (any, used by `bind`).
/// - Null `addr`: treated as "connected socket" — already validated.
/// - Everything else: denied.
fn is_allowed_address(addr: u64, addrlen: u32) -> bool {
    if addr == 0 {
        return true; // null addr → already-connected socket
    }
    let family = read_sa_family(addr);
    match family as i32 {
        libc::AF_UNIX => true,
        libc::AF_INET => {
            if (addrlen as usize) < std::mem::size_of::<libc::sockaddr_in>() {
                return false;
            }
            let sin = unsafe { &*(addr as *const libc::sockaddr_in) };
            let ip = u32::from_be(sin.sin_addr.s_addr);
            ip == 0x7f00_0001 // 127.0.0.1
                || ip == 0    // INADDR_ANY (0.0.0.0) — for bind
        }
        libc::AF_INET6 => {
            if (addrlen as usize) < std::mem::size_of::<libc::sockaddr_in6>() {
                return false;
            }
            let sin6 = unsafe { &*(addr as *const libc::sockaddr_in6) };
            let octets = sin6.sin6_addr.s6_addr;
            let is_loopback = octets == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
            let is_any = octets == [0u8; 16];
            is_loopback || is_any
        }
        _ => false,
    }
}

// ── Shim implementations ────────────────────────────────────────────────────

/// `int socket(int domain, int type, int protocol)`
///
/// Allows creation of AF_UNIX / AF_INET / AF_INET6 sockets (address not yet
/// known).  All other families are denied.
pub extern "C" fn shim_socket(domain: i32, typ: i32, proto: i32) -> i32 {
    match domain {
        libc::AF_UNIX | libc::AF_INET | libc::AF_INET6 => {
            log::debug!("socket(domain={domain}) — passthrough");
            unsafe { libc::socket(domain, typ, proto) }
        }
        _ => {
            log::debug!("socket(domain={domain}) — denied (EOPNOTSUPP)");
            deny_i32()
        }
    }
}

/// `int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen)`
pub extern "C" fn shim_connect(fd: i32, addr: u64, len: u32) -> i32 {
    if is_allowed_address(addr, len) {
        log::debug!("connect(fd={fd}) — passthrough (localhost/unix)");
        unsafe { libc::connect(fd, addr as *const libc::sockaddr, len) }
    } else {
        log::debug!("connect(fd={fd}) — denied (non-localhost)");
        deny_i32()
    }
}

/// `int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen)`
pub extern "C" fn shim_bind(fd: i32, addr: u64, len: u32) -> i32 {
    if is_allowed_address(addr, len) {
        log::debug!("bind(fd={fd}) — passthrough (localhost/unix)");
        unsafe { libc::bind(fd, addr as *const libc::sockaddr, len) }
    } else {
        log::debug!("bind(fd={fd}) — denied (non-localhost)");
        deny_i32()
    }
}

/// `int listen(int sockfd, int backlog)`
pub extern "C" fn shim_listen(fd: i32, backlog: i32) -> i32 {
    log::debug!("listen(fd={fd}) — passthrough");
    unsafe { libc::listen(fd, backlog) }
}

/// `int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen)`
pub extern "C" fn shim_accept(fd: i32, addr: u64, len: u64) -> i32 {
    log::debug!("accept(fd={fd}) — passthrough");
    unsafe { libc::accept(fd, addr as *mut libc::sockaddr, len as *mut libc::socklen_t) }
}

/// `ssize_t send(int sockfd, const void *buf, size_t len, int flags)`
pub extern "C" fn shim_send(fd: i32, buf: u64, len: u64, flags: i32) -> i64 {
    log::debug!("send(fd={fd}) — passthrough");
    unsafe { libc::send(fd, buf as *const libc::c_void, len as usize, flags) as i64 }
}

/// `ssize_t recv(int sockfd, void *buf, size_t len, int flags)`
pub extern "C" fn shim_recv(fd: i32, buf: u64, len: u64, flags: i32) -> i64 {
    log::debug!("recv(fd={fd}) — passthrough");
    unsafe { libc::recv(fd, buf as *mut libc::c_void, len as usize, flags) as i64 }
}

/// `ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
///                  const struct sockaddr *dest_addr, socklen_t addrlen)`
pub extern "C" fn shim_sendto(
    fd: i32,
    buf: u64,
    len: u64,
    flags: i32,
    dest: u64,
    dest_len: u32,
) -> i64 {
    if dest != 0 && !is_allowed_address(dest, dest_len) {
        log::debug!("sendto(fd={fd}) — denied (non-localhost dest)");
        return deny_i64();
    }
    log::debug!("sendto(fd={fd}) — passthrough");
    unsafe {
        libc::sendto(
            fd,
            buf as *const libc::c_void,
            len as usize,
            flags,
            dest as *const libc::sockaddr,
            dest_len,
        ) as i64
    }
}

/// `ssize_t recvfrom(int sockfd, void *buf, size_t len, int flags,
///                    struct sockaddr *src_addr, socklen_t *addrlen)`
pub extern "C" fn shim_recvfrom(
    fd: i32,
    buf: u64,
    len: u64,
    flags: i32,
    src: u64,
    src_len: u64,
) -> i64 {
    log::debug!("recvfrom(fd={fd}) — passthrough");
    unsafe {
        libc::recvfrom(
            fd,
            buf as *mut libc::c_void,
            len as usize,
            flags,
            src as *mut libc::sockaddr,
            src_len as *mut libc::socklen_t,
        ) as i64
    }
}

/// `int getsockopt(int sockfd, int level, int optname, void *optval, socklen_t *optlen)`
pub extern "C" fn shim_getsockopt(
    fd: i32,
    level: i32,
    optname: i32,
    optval: u64,
    optlen: u64,
) -> i32 {
    log::debug!("getsockopt(fd={fd}) — passthrough");
    unsafe {
        libc::getsockopt(
            fd,
            level,
            optname,
            optval as *mut libc::c_void,
            optlen as *mut libc::socklen_t,
        )
    }
}

/// `int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen)`
pub extern "C" fn shim_setsockopt(
    fd: i32,
    level: i32,
    optname: i32,
    optval: u64,
    optlen: u32,
) -> i32 {
    log::debug!("setsockopt(fd={fd}) — passthrough");
    unsafe { libc::setsockopt(fd, level, optname, optval as *const libc::c_void, optlen) }
}

/// `int shutdown(int sockfd, int how)`
pub extern "C" fn shim_shutdown(fd: i32, how: i32) -> i32 {
    log::debug!("shutdown(fd={fd}) — passthrough");
    unsafe { libc::shutdown(fd, how) }
}

// ── Registration ────────────────────────────────────────────────────────────

pub struct SocketShims;

impl SocketShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_socket",     shim_socket as u64,     "socket — localhost/unix passthrough"),
            ("_connect",    shim_connect as u64,    "connect — localhost/unix passthrough"),
            ("_bind",       shim_bind as u64,       "bind — localhost/unix passthrough"),
            ("_listen",     shim_listen as u64,     "listen — passthrough"),
            ("_accept",     shim_accept as u64,     "accept — passthrough"),
            ("_send",       shim_send as u64,       "send — passthrough"),
            ("_recv",       shim_recv as u64,       "recv — passthrough"),
            ("_sendto",     shim_sendto as u64,     "sendto — localhost/unix passthrough"),
            ("_recvfrom",   shim_recvfrom as u64,   "recvfrom — passthrough"),
            ("_getsockopt", shim_getsockopt as u64, "getsockopt — passthrough"),
            ("_setsockopt", shim_setsockopt as u64, "setsockopt — passthrough"),
            ("_shutdown",   shim_shutdown as u64,   "shutdown — passthrough"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read current errno value.
    fn get_errno() -> i32 {
        unsafe { *errno_ptr() }
    }

    /// Clear errno before a test call.
    fn clear_errno() {
        unsafe { *errno_ptr() = 0; }
    }

    /// Close a file descriptor, ignoring errors.
    fn close_fd(fd: i32) {
        if fd >= 0 {
            unsafe { libc::close(fd); }
        }
    }

    // ── socket() tests ──────────────────────────────────────────────────

    #[test]
    fn test_socket_unix_allowed() {
        clear_errno();
        let fd = shim_socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
        assert!(fd >= 0, "AF_UNIX socket should succeed, got fd={fd} errno={}", get_errno());
        close_fd(fd);
    }

    #[test]
    fn test_socket_inet_allowed() {
        clear_errno();
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0, "AF_INET socket should succeed, got fd={fd} errno={}", get_errno());
        close_fd(fd);
    }

    #[test]
    fn test_socket_unsupported_denied() {
        clear_errno();
        // AF_APPLETALK = 16
        let fd = shim_socket(16, libc::SOCK_STREAM, 0);
        assert_eq!(fd, -1);
        assert_eq!(get_errno(), libc::EOPNOTSUPP);
    }

    // ── connect() tests ─────────────────────────────────────────────────

    #[test]
    fn test_connect_localhost_allowed() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let mut sa: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        #[cfg(target_os = "macos")]
        {
            sa.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
        }
        sa.sin_family = libc::AF_INET as u8;
        sa.sin_port = 0; // port 0
        sa.sin_addr.s_addr = u32::to_be(0x7f00_0001); // 127.0.0.1

        clear_errno();
        let ret = shim_connect(
            fd,
            &sa as *const libc::sockaddr_in as u64,
            std::mem::size_of::<libc::sockaddr_in>() as u32,
        );
        // connect() will fail (ECONNREFUSED — nothing listening) but NOT
        // with EOPNOTSUPP, proving the policy allowed the attempt.
        if ret == -1 {
            assert_ne!(
                get_errno(),
                libc::EOPNOTSUPP,
                "connect to 127.0.0.1 should not be denied by policy"
            );
        }
        close_fd(fd);
    }

    #[test]
    fn test_connect_external_denied() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let mut sa: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        #[cfg(target_os = "macos")]
        {
            sa.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
        }
        sa.sin_family = libc::AF_INET as u8;
        sa.sin_port = 53u16.to_be(); // DNS port
        sa.sin_addr.s_addr = u32::to_be(0x0808_0808); // 8.8.8.8

        clear_errno();
        let ret = shim_connect(
            fd,
            &sa as *const libc::sockaddr_in as u64,
            std::mem::size_of::<libc::sockaddr_in>() as u32,
        );
        assert_eq!(ret, -1);
        assert_eq!(get_errno(), libc::EOPNOTSUPP);
        close_fd(fd);
    }

    // ── bind() tests ────────────────────────────────────────────────────

    #[test]
    fn test_bind_localhost_allowed() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let mut sa: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        #[cfg(target_os = "macos")]
        {
            sa.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
        }
        sa.sin_family = libc::AF_INET as u8;
        sa.sin_port = 0; // ephemeral
        sa.sin_addr.s_addr = u32::to_be(0x7f00_0001); // 127.0.0.1

        clear_errno();
        let ret = shim_bind(
            fd,
            &sa as *const libc::sockaddr_in as u64,
            std::mem::size_of::<libc::sockaddr_in>() as u32,
        );
        assert_eq!(ret, 0, "bind to 127.0.0.1:0 should succeed, errno={}", get_errno());
        close_fd(fd);
    }

    // ── shutdown() test ─────────────────────────────────────────────────

    #[test]
    fn test_shutdown_passthrough() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        clear_errno();
        let ret = shim_shutdown(fd, libc::SHUT_RDWR);
        // Will likely return -1/ENOTCONN (not connected), but NOT EOPNOTSUPP.
        if ret == -1 {
            assert_ne!(
                get_errno(),
                libc::EOPNOTSUPP,
                "shutdown should passthrough, not return policy-denied"
            );
        }
        close_fd(fd);
    }

    // ── getsockopt() test ───────────────────────────────────────────────

    #[test]
    fn test_getsockopt_passthrough() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let mut val: i32 = 0;
        let mut len: libc::socklen_t = std::mem::size_of::<i32>() as libc::socklen_t;

        clear_errno();
        let ret = shim_getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &mut val as *mut i32 as u64,
            &mut len as *mut libc::socklen_t as u64,
        );
        assert_eq!(ret, 0, "getsockopt SO_REUSEADDR should succeed, errno={}", get_errno());
        close_fd(fd);
    }

    // ── send/recv over unix socketpair ──────────────────────────────────

    #[test]
    fn test_send_recv_unix_pair() {
        let mut fds: [i32; 2] = [-1, -1];
        let ret = unsafe {
            libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr())
        };
        assert_eq!(ret, 0, "socketpair failed, errno={}", get_errno());

        let msg = b"hello";
        let sent = shim_send(fds[0], msg.as_ptr() as u64, msg.len() as u64, 0);
        assert_eq!(sent, 5, "send should return 5");

        let mut buf = [0u8; 16];
        let recvd = shim_recv(fds[1], buf.as_mut_ptr() as u64, buf.len() as u64, 0);
        assert_eq!(recvd, 5, "recv should return 5");
        assert_eq!(&buf[..5], b"hello");

        close_fd(fds[0]);
        close_fd(fds[1]);
    }

    // ── registration test ───────────────────────────────────────────────

    #[test]
    fn socket_shims_registered() {
        let mut table = ShimTable::new();
        SocketShims::install(&mut table);
        assert!(table.resolve("_socket").is_some());
        assert!(table.resolve("_connect").is_some());
        assert!(table.resolve("_bind").is_some());
        assert!(table.resolve("_listen").is_some());
        assert!(table.resolve("_accept").is_some());
        assert!(table.resolve("_send").is_some());
        assert!(table.resolve("_recv").is_some());
        assert!(table.resolve("_sendto").is_some());
        assert!(table.resolve("_recvfrom").is_some());
        assert!(table.resolve("_getsockopt").is_some());
        assert!(table.resolve("_setsockopt").is_some());
        assert!(table.resolve("_shutdown").is_some());
        assert_eq!(table.len(), 12);
    }
}
