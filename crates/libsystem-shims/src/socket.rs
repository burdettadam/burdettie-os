//! POSIX socket syscall interception shims.
//!
//! These shims sit in front of the twelve core BSD socket calls so that
//! emulated macOS binaries can use networking on a Linux host.
//!
//! Translation:
//!   - macOS AF_* constants → Linux equivalents (e.g. AF_INET6: 30→10)
//!   - macOS sockaddr layout `[sa_len:u8][sa_family:u8]` → Linux `[sa_family:u16]`
//!   - macOS SOL_SOCKET (0xFFFF) → Linux SOL_SOCKET (1)
//!
//! `socket()` denies unsupported address families with `EOPNOTSUPP`.
//! All other calls pass through after ABI translation.

use crate::table::ShimTable;

// ── macOS → Linux constant translation ──────────────────────────────────────

// macOS AF_* constants differ from Linux.
const DARWIN_AF_UNIX: i32 = 1;
const DARWIN_AF_INET: i32 = 2;
const DARWIN_AF_INET6: i32 = 30;
const DARWIN_AF_ROUTE: i32 = 17;
const DARWIN_AF_SYSTEM: i32 = 40;

// macOS SOL_SOCKET = 0xFFFF, Linux SOL_SOCKET = 1.
const DARWIN_SOL_SOCKET: i32 = 0xFFFF;

/// Translate a macOS address-family constant to the Linux equivalent.
/// Returns `None` for families we don't support.
fn translate_af(darwin_af: i32) -> Option<i32> {
    match darwin_af {
        DARWIN_AF_UNIX => Some(libc::AF_UNIX),
        DARWIN_AF_INET => Some(libc::AF_INET),
        DARWIN_AF_INET6 => Some(libc::AF_INET6),
        #[cfg(target_os = "linux")]
        DARWIN_AF_ROUTE => Some(libc::AF_NETLINK),
        // Also pass through already-translated Linux values (when called
        // from host code or tests that use Linux constants directly).
        v if v == libc::AF_UNIX || v == libc::AF_INET || v == libc::AF_INET6 => Some(v),
        _ => None,
    }
}

/// Translate a macOS SOL_* level to the Linux equivalent.
fn translate_sockopt_level(darwin_level: i32) -> i32 {
    if darwin_level == DARWIN_SOL_SOCKET {
        libc::SOL_SOCKET
    } else {
        darwin_level
    }
}

// ── macOS → Linux sockaddr translation ──────────────────────────────────────
//
// macOS sockaddr layout:  [sa_len: u8][sa_family: u8][sa_data...]
// Linux sockaddr layout:  [sa_family: u16][sa_data...]
//
// We need to:
//   1. Read sa_family from the macOS byte position (offset 1)
//   2. Translate the AF_* constant (e.g. AF_INET6: 30→10)
//   3. Rewrite sa_family in Linux layout (u16 at offset 0)
//
// This is done in a stack-local copy to avoid mutating the guest's buffer.

/// Read the `sa_family` from a macOS-layout `sockaddr`, translate AF_* constants,
/// and write a Linux-layout sockaddr into `out_buf`. Returns the translated
/// byte count. On macOS host, this is a no-op copy.
#[cfg(target_os = "linux")]
fn translate_sockaddr(src: u64, src_len: u32, out_buf: &mut [u8]) -> u32 {
    if src == 0 || src_len == 0 || (src_len as usize) > out_buf.len() {
        return 0;
    }
    let n = src_len as usize;
    // Copy the raw macOS sockaddr bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(src as *const u8, out_buf.as_mut_ptr(), n);
    }
    if n < 2 {
        return src_len;
    }
    // macOS layout: byte 0 = sa_len, byte 1 = sa_family (u8).
    let darwin_family = out_buf[1] as i32;
    let linux_family = translate_af(darwin_family).unwrap_or(darwin_family);
    // Linux layout: bytes 0-1 = sa_family (u16, native endian).
    let fam_bytes = (linux_family as u16).to_ne_bytes();
    out_buf[0] = fam_bytes[0];
    out_buf[1] = fam_bytes[1];
    src_len
}

/// On macOS host, no translation needed — just return the pointer as-is.
#[cfg(target_os = "macos")]
fn translate_sockaddr(src: u64, src_len: u32, _out_buf: &mut [u8]) -> u32 {
    src_len
}

/// Helper: get the translated sockaddr pointer. On Linux, points to `buf`.
/// On macOS, points to the original source.
#[cfg(target_os = "linux")]
fn translated_sockaddr_ptr(src: u64, buf: &[u8], translated_len: u32) -> *const libc::sockaddr {
    let _ = (src, translated_len);
    buf.as_ptr() as *const libc::sockaddr
}

#[cfg(target_os = "macos")]
fn translated_sockaddr_ptr(src: u64, _buf: &[u8], _translated_len: u32) -> *const libc::sockaddr {
    src as *const libc::sockaddr
}

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

/// Read the `sa_family` from a guest (macOS-layout) `sockaddr` pointer.
///
/// macOS layout: `[sa_len: u8][sa_family: u8][data...]`
/// We read byte 1 for the family, then translate the AF_* constant.
#[cfg(target_os = "linux")]
fn read_guest_sa_family(addr: u64) -> i32 {
    if addr == 0 {
        return 0;
    }
    // macOS sockaddr: byte 1 is sa_family
    let darwin_family = unsafe { *((addr as usize + 1) as *const u8) } as i32;
    translate_af(darwin_family).unwrap_or(darwin_family)
}

#[cfg(target_os = "macos")]
fn read_guest_sa_family(addr: u64) -> i32 {
    if addr == 0 {
        return 0;
    }
    let sa = addr as *const libc::sockaddr;
    unsafe { (*sa).sa_family as i32 }
}

// ── Shim implementations ────────────────────────────────────────────────────

/// `int socket(int domain, int type, int protocol)`
///
/// Translates macOS AF_* constants to Linux equivalents and creates the socket.
/// Denies unsupported address families.
pub extern "C" fn shim_socket(domain: i32, typ: i32, proto: i32) -> i32 {
    eprintln!("[SHIM] socket(domain={domain}, type={typ}, proto={proto})");
    if let Some(linux_domain) = translate_af(domain) {
        log::debug!("socket(domain={domain}→{linux_domain}) — passthrough");
        unsafe { libc::socket(linux_domain, typ, proto) }
    } else {
        log::debug!("socket(domain={domain}) — denied (EOPNOTSUPP)");
        deny_i32()
    }
}

/// `int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen)`
///
/// Translates macOS sockaddr layout (sa_len+sa_family) to Linux (sa_family u16)
/// and translates AF_* constants. All destinations are allowed — the guest binary
/// controls its own network access.
pub extern "C" fn shim_connect(fd: i32, addr: u64, len: u32) -> i32 {
    let mut buf = [0u8; 128];
    let tlen = translate_sockaddr(addr, len, &mut buf);
    if tlen == 0 && addr != 0 {
        return deny_i32();
    }
    let ptr = if addr == 0 {
        std::ptr::null()
    } else {
        translated_sockaddr_ptr(addr, &buf, tlen)
    };
    log::debug!("connect(fd={fd}) — passthrough");
    unsafe { libc::connect(fd, ptr, tlen) }
}

/// `int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen)`
pub extern "C" fn shim_bind(fd: i32, addr: u64, len: u32) -> i32 {
    let mut buf = [0u8; 128];
    let tlen = translate_sockaddr(addr, len, &mut buf);
    if tlen == 0 && addr != 0 {
        return deny_i32();
    }
    let ptr = if addr == 0 {
        std::ptr::null()
    } else {
        translated_sockaddr_ptr(addr, &buf, tlen)
    };
    log::debug!("bind(fd={fd}) — passthrough");
    unsafe { libc::bind(fd, ptr, tlen) }
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
    let mut sa_buf = [0u8; 128];
    let tlen = if dest != 0 {
        translate_sockaddr(dest, dest_len, &mut sa_buf)
    } else {
        0
    };
    let ptr = if dest == 0 {
        std::ptr::null()
    } else {
        translated_sockaddr_ptr(dest, &sa_buf, tlen)
    };
    log::debug!("sendto(fd={fd}) — passthrough");
    unsafe {
        libc::sendto(
            fd,
            buf as *const libc::c_void,
            len as usize,
            flags,
            ptr,
            tlen,
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
    let linux_level = translate_sockopt_level(level);
    log::debug!("getsockopt(fd={fd}, level={level}→{linux_level}) — passthrough");
    unsafe {
        libc::getsockopt(
            fd,
            linux_level,
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
    let linux_level = translate_sockopt_level(level);
    log::debug!("setsockopt(fd={fd}, level={level}→{linux_level}) — passthrough");
    unsafe { libc::setsockopt(fd, linux_level, optname, optval as *const libc::c_void, optlen) }
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
            ("_socket",     shim_socket as *const () as u64,     "socket — AF translation passthrough"),
            ("_connect",    shim_connect as *const () as u64,    "connect — sockaddr translation passthrough"),
            ("_bind",       shim_bind as *const () as u64,       "bind — sockaddr translation passthrough"),
            ("_listen",     shim_listen as *const () as u64,     "listen — passthrough"),
            ("_accept",     shim_accept as *const () as u64,     "accept — passthrough"),
            ("_send",       shim_send as *const () as u64,       "send — passthrough"),
            ("_recv",       shim_recv as *const () as u64,       "recv — passthrough"),
            ("_sendto",     shim_sendto as *const () as u64,     "sendto — sockaddr translation passthrough"),
            ("_recvfrom",   shim_recvfrom as *const () as u64,   "recvfrom — passthrough"),
            ("_getsockopt", shim_getsockopt as *const () as u64, "getsockopt — passthrough"),
            ("_setsockopt", shim_setsockopt as *const () as u64, "setsockopt — passthrough"),
            ("_shutdown",   shim_shutdown as *const () as u64,   "shutdown — passthrough"),
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
    fn test_connect_external_allowed() {
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
        // External addresses are now allowed through — connect may fail
        // (ECONNREFUSED, ETIMEDOUT, etc.) but NOT with EOPNOTSUPP.
        if ret == -1 {
            assert_ne!(
                get_errno(),
                libc::EOPNOTSUPP,
                "connect to 8.8.8.8 should not be denied by policy"
            );
        }
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
