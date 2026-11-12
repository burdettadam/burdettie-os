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

// ── macOS `struct msghdr` layout (differs from Linux on ARM64) ──────────────
//
// macOS ARM64:
//   void            *msg_name;       offset 0,  8 bytes
//   socklen_t        msg_namelen;    offset 8,  4 bytes
//   [4 pad]                          offset 12
//   struct iovec    *msg_iov;        offset 16, 8 bytes
//   int              msg_iovlen;     offset 24, 4 bytes  ← KEY DIFFERENCE
//   [4 pad]                          offset 28
//   void            *msg_control;    offset 32, 8 bytes
//   socklen_t        msg_controllen; offset 40, 4 bytes  ← KEY DIFFERENCE
//   int              msg_flags;      offset 44, 4 bytes
//   Total: 48 bytes
//
// Linux ARM64:
//   void            *msg_name;       offset 0,  8 bytes
//   socklen_t        msg_namelen;    offset 8,  4 bytes
//   [4 pad]                          offset 12
//   struct iovec    *msg_iov;        offset 16, 8 bytes
//   size_t           msg_iovlen;     offset 24, 8 bytes
//   void            *msg_control;    offset 32, 8 bytes
//   size_t           msg_controllen; offset 40, 8 bytes
//   unsigned int     msg_flags;      offset 48, 4 bytes
//   Total: 56 bytes

/// Read fields from a macOS-layout msghdr at the given pointer.
#[cfg(target_os = "linux")]
struct DarwinMsghdr {
    msg_name: *mut libc::c_void,
    msg_namelen: u32,
    msg_iov: *mut libc::iovec,
    msg_iovlen: i32,
    msg_control: *mut libc::c_void,
    msg_controllen: u32,
    msg_flags: i32,
}

#[cfg(target_os = "linux")]
impl DarwinMsghdr {
    /// Read from a macOS-layout msghdr in guest memory.
    unsafe fn read_from(ptr: u64) -> Self {
        let base = ptr as *const u8;
        Self {
            msg_name: unsafe { *(base.add(0) as *const *mut libc::c_void) },
            msg_namelen: unsafe { *(base.add(8) as *const u32) },
            msg_iov: unsafe { *(base.add(16) as *const *mut libc::iovec) },
            msg_iovlen: unsafe { *(base.add(24) as *const i32) },
            msg_control: unsafe { *(base.add(32) as *const *mut libc::c_void) },
            msg_controllen: unsafe { *(base.add(40) as *const u32) },
            msg_flags: unsafe { *(base.add(44) as *const i32) },
        }
    }

    /// Convert to a Linux-layout msghdr for syscall use.
    fn to_linux(&self) -> libc::msghdr {
        libc::msghdr {
            msg_name: self.msg_name,
            msg_namelen: self.msg_namelen,
            msg_iov: self.msg_iov,
            msg_iovlen: self.msg_iovlen as usize,
            msg_control: self.msg_control,
            msg_controllen: self.msg_controllen as usize,
            msg_flags: self.msg_flags,
        }
    }

    /// Write output fields back to the macOS-layout msghdr after recvmsg.
    unsafe fn write_back(&self, ptr: u64, linux_msg: &libc::msghdr) {
        let base = ptr as *mut u8;
        // Write back only the fields that recvmsg modifies.
        unsafe {
            *(base.add(8) as *mut u32) = linux_msg.msg_namelen;
            *(base.add(40) as *mut u32) = linux_msg.msg_controllen as u32;
            *(base.add(44) as *mut i32) = linux_msg.msg_flags;
        }
    }
}

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

/// Reverse translation: Linux AF → macOS AF (for recvmsg return path).
fn linux_af_to_darwin(linux_af: u16) -> (u8, u8) {
    // Returns (sa_len, darwin_sa_family).
    match linux_af as i32 {
        libc::AF_INET => (16, DARWIN_AF_INET as u8), // sockaddr_in = 16 bytes
        libc::AF_INET6 => (28, DARWIN_AF_INET6 as u8), // sockaddr_in6 = 28 bytes
        libc::AF_UNIX => (106, DARWIN_AF_UNIX as u8), // sockaddr_un ~106 bytes
        _ => (0, linux_af as u8),                    // unknown — pass through
    }
}

/// Rewrite a Linux sockaddr (returned by recvmsg/recvfrom) in-place to macOS
/// layout:  Linux `[sa_family:u16_le]` → macOS `[sa_len:u8][sa_family:u8]`.
fn translate_sockaddr_linux_to_darwin(buf: *mut u8, len: u32) {
    if buf.is_null() || len < 2 {
        return;
    }
    unsafe {
        // Read Linux sa_family (u16 little-endian at offset 0).
        let family_lo = *buf;
        let family_hi = *buf.add(1);
        let linux_family = u16::from_le_bytes([family_lo, family_hi]);
        let (sa_len, darwin_family) = linux_af_to_darwin(linux_family);
        log::trace!(
            "translate_sockaddr_linux_to_darwin: Linux family={linux_family} → \
             macOS sa_len={sa_len} sa_family={darwin_family}"
        );
        // Overwrite: byte 0 = sa_len, byte 1 = sa_family (u8).
        *buf = sa_len;
        *buf.add(1) = darwin_family;
    }
}

/// Repack the control message (cmsg) buffer from Linux layout to macOS layout.
///
/// Linux cmsghdr on arm64:  [cmsg_len:u64][cmsg_level:i32][cmsg_type:i32]  = 16 byte header, 8-byte aligned
/// macOS cmsghdr on arm64:  [cmsg_len:u32][cmsg_level:i32][cmsg_type:i32]  = 12 byte header, 4-byte aligned
///
/// We walk the Linux cmsg chain, translate level/type, and repack in-place
/// into macOS layout (which is always smaller, so it fits in the same buffer).
/// Returns the new macOS-layout total controllen.
#[cfg(target_os = "linux")]
fn translate_cmsg_linux_to_darwin(buf: *mut u8, total_len: usize) -> u32 {
    const LINUX_CMSG_HDR: usize = 16; // sizeof(struct cmsghdr) on Linux arm64
    const LINUX_CMSG_ALIGN: usize = 8;
    const DARWIN_CMSG_HDR: usize = 12; // sizeof(struct cmsghdr) on macOS arm64
    const DARWIN_CMSG_ALIGN: usize = 4;

    let mut linux_off = 0usize;
    let mut darwin_off = 0usize;

    while linux_off + LINUX_CMSG_HDR <= total_len {
        // Read Linux cmsghdr.
        let linux_len = unsafe { *(buf.add(linux_off) as *const u64) } as usize;
        if linux_len < LINUX_CMSG_HDR || linux_len > total_len - linux_off {
            break;
        }
        let linux_level = unsafe { *(buf.add(linux_off + 8) as *const i32) };
        let linux_type = unsafe { *(buf.add(linux_off + 12) as *const i32) };
        let data_len = linux_len - LINUX_CMSG_HDR;

        log::trace!("cmsg repack: Linux level={linux_level} type={linux_type} data_len={data_len}");

        // Translate level + type.
        let (darwin_level, darwin_type) = translate_cmsg_level_type(linux_level, linux_type);

        // macOS cmsg_len includes the 12-byte header + data (no alignment).
        let darwin_cmsg_len = (DARWIN_CMSG_HDR + data_len) as u32;

        // Write macOS cmsghdr at darwin_off.
        unsafe {
            *(buf.add(darwin_off) as *mut u32) = darwin_cmsg_len;
            *(buf.add(darwin_off + 4) as *mut i32) = darwin_level;
            *(buf.add(darwin_off + 8) as *mut i32) = darwin_type;
        }

        // Copy data payload (may overlap since we're shrinking, use copy).
        if data_len > 0 {
            unsafe {
                std::ptr::copy(
                    buf.add(linux_off + LINUX_CMSG_HDR),
                    buf.add(darwin_off + DARWIN_CMSG_HDR),
                    data_len,
                );
            }
        }

        // Advance Linux offset (aligned to 8).
        linux_off += (linux_len + LINUX_CMSG_ALIGN - 1) & !(LINUX_CMSG_ALIGN - 1);
        // Advance Darwin offset (aligned to 4).
        let darwin_total = DARWIN_CMSG_HDR + data_len;
        darwin_off += (darwin_total + DARWIN_CMSG_ALIGN - 1) & !(DARWIN_CMSG_ALIGN - 1);
    }

    log::trace!("cmsg repack: Linux {total_len} bytes → macOS {darwin_off} bytes");
    darwin_off as u32
}

/// Translate a single cmsg level+type from Linux to macOS.
#[cfg(target_os = "linux")]
fn translate_cmsg_level_type(linux_level: i32, linux_type: i32) -> (i32, i32) {
    if linux_level == libc::SOL_SOCKET {
        let darwin_type = match linux_type {
            libc::SCM_TIMESTAMP => 0x02, // macOS SCM_TIMESTAMP
            _ => linux_type,
        };
        return (DARWIN_SOL_SOCKET, darwin_type);
    }
    if linux_level == libc::IPPROTO_IP {
        let darwin_type = match linux_type {
            8  /* IP_PKTINFO */  => 26, // macOS IP_PKTINFO
            1  /* IP_TOS */      => 27, // macOS IP_RECVTOS
            2  /* IP_TTL */      => 24, // macOS IP_RECVTTL
            _ => linux_type,
        };
        return (linux_level, darwin_type);
    }
    (linux_level, linux_type)
}

/// Translate a macOS SOL_* level to the Linux equivalent.
fn translate_sockopt_level(darwin_level: i32) -> i32 {
    if darwin_level == DARWIN_SOL_SOCKET {
        libc::SOL_SOCKET
    } else {
        darwin_level
    }
}

// ── macOS socket option constants ────────────────────────────────────────────
// macOS SOL_SOCKET options (many differ from Linux).
const DARWIN_SO_DEBUG: i32 = 0x0001;
const DARWIN_SO_REUSEADDR: i32 = 0x0004;
const DARWIN_SO_KEEPALIVE: i32 = 0x0008;
const DARWIN_SO_DONTROUTE: i32 = 0x0010;
const DARWIN_SO_BROADCAST: i32 = 0x0020;
const DARWIN_SO_LINGER: i32 = 0x0080;
const DARWIN_SO_OOBINLINE: i32 = 0x0100;
const DARWIN_SO_REUSEPORT: i32 = 0x0200;
const DARWIN_SO_TIMESTAMP: i32 = 0x0400;
const DARWIN_SO_SNDBUF: i32 = 0x1001;
const DARWIN_SO_RCVBUF: i32 = 0x1002;
const DARWIN_SO_SNDLOWAT: i32 = 0x1003;
const DARWIN_SO_RCVLOWAT: i32 = 0x1004;
const DARWIN_SO_SNDTIMEO: i32 = 0x1005;
const DARWIN_SO_RCVTIMEO: i32 = 0x1006;
const DARWIN_SO_ERROR: i32 = 0x1007;
const DARWIN_SO_TYPE: i32 = 0x1008;
const DARWIN_SO_NOSIGPIPE: i32 = 0x1022; // macOS-only, no Linux equivalent
const DARWIN_SO_NREAD: i32 = 0x1020; // macOS-only (use FIONREAD ioctl)
const DARWIN_SO_NUMRCVPKT: i32 = 0x1112; // macOS-only

// macOS IPPROTO_IP option constants that differ.
const DARWIN_IP_RECVDSTADDR: i32 = 7; // macOS=7, Linux has IP_PKTINFO=8
const DARWIN_IP_RECVTOS: i32 = 27; // macOS=27, Linux IP_RECVTOS=13

/// Translate a macOS socket option name to the Linux equivalent.
/// Returns `Some(linux_optname)` for translatable options,
/// `None` for macOS-only options that should be silently ignored.
#[cfg(target_os = "linux")]
fn translate_sockopt_name(level: i32, darwin_optname: i32) -> Option<i32> {
    if level == libc::SOL_SOCKET {
        match darwin_optname {
            DARWIN_SO_DEBUG => Some(libc::SO_DEBUG),
            DARWIN_SO_REUSEADDR => Some(libc::SO_REUSEADDR),
            DARWIN_SO_KEEPALIVE => Some(libc::SO_KEEPALIVE),
            DARWIN_SO_DONTROUTE => Some(libc::SO_DONTROUTE),
            DARWIN_SO_BROADCAST => Some(libc::SO_BROADCAST),
            DARWIN_SO_LINGER => Some(libc::SO_LINGER),
            DARWIN_SO_OOBINLINE => Some(libc::SO_OOBINLINE),
            DARWIN_SO_REUSEPORT => Some(libc::SO_REUSEPORT),
            DARWIN_SO_TIMESTAMP => Some(libc::SO_TIMESTAMP),
            DARWIN_SO_SNDBUF => Some(libc::SO_SNDBUF),
            DARWIN_SO_RCVBUF => Some(libc::SO_RCVBUF),
            DARWIN_SO_SNDLOWAT => Some(libc::SO_SNDLOWAT),
            DARWIN_SO_RCVLOWAT => Some(libc::SO_RCVLOWAT),
            DARWIN_SO_SNDTIMEO => Some(libc::SO_SNDTIMEO),
            DARWIN_SO_RCVTIMEO => Some(libc::SO_RCVTIMEO),
            DARWIN_SO_ERROR => Some(libc::SO_ERROR),
            DARWIN_SO_TYPE => Some(libc::SO_TYPE),
            DARWIN_SO_NOSIGPIPE | DARWIN_SO_NREAD | DARWIN_SO_NUMRCVPKT => None,
            _ => Some(darwin_optname),
        }
    } else if level == libc::IPPROTO_IP {
        match darwin_optname {
            DARWIN_IP_RECVDSTADDR => Some(libc::IP_PKTINFO),
            DARWIN_IP_RECVTOS => Some(libc::IP_RECVTOS),
            _ => Some(darwin_optname),
        }
    } else if level == libc::IPPROTO_IPV6 {
        // macOS and Linux IPV6_* option values differ significantly.
        match darwin_optname {
            4  /* IPV6_UNICAST_HOPS */   => Some(libc::IPV6_UNICAST_HOPS),    // 16
            9  /* IPV6_MULTICAST_IF */   => Some(libc::IPV6_MULTICAST_IF),    // 17
            10 /* IPV6_MULTICAST_HOPS */ => Some(libc::IPV6_MULTICAST_HOPS),  // 18
            11 /* IPV6_MULTICAST_LOOP */ => Some(libc::IPV6_MULTICAST_LOOP),  // 19
            27 /* IPV6_V6ONLY */         => Some(libc::IPV6_V6ONLY),          // 26
            35 /* IPV6_RECVTCLASS */     => Some(libc::IPV6_RECVTCLASS),      // 66
            36 /* IPV6_TCLASS */         => Some(libc::IPV6_TCLASS),          // 67
            37 /* IPV6_RECVHOPLIMIT */   => Some(51), // IPV6_RECVHOPLIMIT
            46 /* IPV6_PKTINFO */        => Some(libc::IPV6_PKTINFO),         // 50
            47 /* IPV6_HOPLIMIT */       => Some(52), // IPV6_HOPLIMIT
            61 /* IPV6_RECVPKTINFO */    => Some(libc::IPV6_RECVPKTINFO),     // 49
            _ => Some(darwin_optname),
        }
    } else {
        Some(darwin_optname)
    }
}

#[cfg(target_os = "macos")]
fn translate_sockopt_name(_level: i32, darwin_optname: i32) -> Option<i32> {
    Some(darwin_optname)
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
fn translate_sockaddr(_src: u64, src_len: u32, _out_buf: &mut [u8]) -> u32 {
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
    unsafe {
        *errno_ptr() = libc::EOPNOTSUPP;
    }
    -1
}

/// Set errno to `EOPNOTSUPP` and return `-1` (i64).
fn deny_i64() -> i64 {
    unsafe {
        *errno_ptr() = libc::EOPNOTSUPP;
    }
    -1
}

/// Translate the current Linux errno to the macOS equivalent.
/// Should be called after a libc call that returned -1.
/// Many errno values are the same, but key networking/threading ones differ.
#[cfg(target_os = "linux")]
fn translate_errno_to_darwin() {
    let e = unsafe { *errno_ptr() };
    let darwin_e = linux_errno_to_darwin(e);
    if darwin_e != e {
        unsafe {
            *errno_ptr() = darwin_e;
        }
    }
}

/// Map a Linux errno value to its macOS equivalent.
#[cfg(target_os = "linux")]
pub fn linux_errno_to_darwin(linux_errno: i32) -> i32 {
    // Only translate values that actually differ.
    // macOS errno.h vs Linux errno.h for key values:
    match linux_errno {
        11  /* EAGAIN/EWOULDBLOCK */  => 35,
        97  /* EAFNOSUPPORT */        => 47,
        99  /* EADDRNOTAVAIL */       => 49,
        98  /* EADDRINUSE */          => 48,
        100 /* ENETDOWN */            => 50,
        101 /* ENETUNREACH */         => 51,
        102 /* ENETRESET */           => 52,
        103 /* ECONNABORTED */        => 53,
        104 /* ECONNRESET */          => 54,
        105 /* ENOBUFS */             => 55,
        106 /* EISCONN */             => 56,
        107 /* ENOTCONN */            => 57,
        110 /* ETIMEDOUT */           => 60,
        111 /* ECONNREFUSED */        => 61,
        113 /* EHOSTUNREACH */        => 65,
        114 /* EALREADY */            => 37,
        115 /* EINPROGRESS */         => 36,
        95  /* EOPNOTSUPP */          => 45,
        92  /* ENOPROTOOPT */         => 42,
        93  /* EPROTONOSUPPORT */     => 43,
        _ => linux_errno, // Same on both, or not networking-related
    }
}

/// Helper: call function, translate errno on error, return result.
/// For functions returning i32 where -1 indicates error.
#[cfg(target_os = "linux")]
fn with_errno_i32(f: impl FnOnce() -> i32) -> i32 {
    let ret = f();
    if ret < 0 {
        translate_errno_to_darwin();
    }
    ret
}

/// Helper: call function, translate errno on error, return result.
/// For functions returning i64/ssize_t where -1 indicates error.
#[cfg(target_os = "linux")]
fn with_errno_i64(f: impl FnOnce() -> i64) -> i64 {
    let ret = f();
    if ret < 0 {
        translate_errno_to_darwin();
    }
    ret
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
    if let Some(linux_domain) = translate_af(domain) {
        let fd = unsafe { libc::socket(linux_domain, typ, proto) };
        log::debug!("socket(domain={domain}→{linux_domain}, type={typ}) → fd={fd}");
        fd
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
    #[cfg(target_os = "linux")]
    return with_errno_i32(|| unsafe { libc::connect(fd, ptr, tlen) });
    #[cfg(target_os = "macos")]
    unsafe {
        libc::connect(fd, ptr, tlen)
    }
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
    let ret = unsafe { libc::bind(fd, ptr, tlen) };
    log::debug!("bind(fd={fd}) returned {ret}");
    ret
}

/// `int listen(int sockfd, int backlog)`
pub extern "C" fn shim_listen(fd: i32, backlog: i32) -> i32 {
    log::debug!("listen(fd={fd}) — passthrough");
    unsafe { libc::listen(fd, backlog) }
}

/// `int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen)`
pub extern "C" fn shim_accept(fd: i32, addr: u64, len: u64) -> i32 {
    log::debug!("accept(fd={fd}) — passthrough + sockaddr translation");
    let ret = unsafe { libc::accept(fd, addr as *mut libc::sockaddr, len as *mut libc::socklen_t) };
    #[cfg(target_os = "linux")]
    if ret >= 0 && addr != 0 {
        let actual_len = if len != 0 {
            unsafe { *(len as *const libc::socklen_t) }
        } else {
            0
        };
        translate_sockaddr_linux_to_darwin(addr as *mut u8, actual_len);
    }
    ret
}

/// `int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen)`
pub extern "C" fn shim_getsockname(fd: i32, addr: u64, len: u64) -> i32 {
    log::debug!("getsockname(fd={fd}) — passthrough + sockaddr translation");
    let ret =
        unsafe { libc::getsockname(fd, addr as *mut libc::sockaddr, len as *mut libc::socklen_t) };
    #[cfg(target_os = "linux")]
    if ret >= 0 && addr != 0 {
        let actual_len = if len != 0 {
            unsafe { *(len as *const libc::socklen_t) }
        } else {
            0
        };
        translate_sockaddr_linux_to_darwin(addr as *mut u8, actual_len);
    }
    ret
}

/// `int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen)`
pub extern "C" fn shim_getpeername(fd: i32, addr: u64, len: u64) -> i32 {
    log::debug!("getpeername(fd={fd}) — passthrough + sockaddr translation");
    let ret =
        unsafe { libc::getpeername(fd, addr as *mut libc::sockaddr, len as *mut libc::socklen_t) };
    #[cfg(target_os = "linux")]
    if ret >= 0 && addr != 0 {
        let actual_len = if len != 0 {
            unsafe { *(len as *const libc::socklen_t) }
        } else {
            0
        };
        translate_sockaddr_linux_to_darwin(addr as *mut u8, actual_len);
    }
    ret
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
    #[cfg(target_os = "linux")]
    return with_errno_i64(|| unsafe {
        libc::sendto(
            fd,
            buf as *const libc::c_void,
            len as usize,
            flags,
            ptr,
            tlen,
        ) as i64
    });
    #[cfg(target_os = "macos")]
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
    log::debug!("recvfrom(fd={fd}) — passthrough + errno/sockaddr translation");
    #[cfg(target_os = "linux")]
    {
        let ret = with_errno_i64(|| unsafe {
            libc::recvfrom(
                fd,
                buf as *mut libc::c_void,
                len as usize,
                flags,
                src as *mut libc::sockaddr,
                src_len as *mut libc::socklen_t,
            ) as i64
        });
        if ret >= 0 && src != 0 {
            // Read the actual returned length from the addrlen out-param.
            let actual_len = if src_len != 0 {
                unsafe { *(src_len as *const libc::socklen_t) }
            } else {
                0
            };
            translate_sockaddr_linux_to_darwin(src as *mut u8, actual_len);
        }
        ret
    }
    #[cfg(target_os = "macos")]
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
    let Some(linux_optname) = translate_sockopt_name(linux_level, optname) else {
        log::debug!("getsockopt(fd={fd}, level={level}, opt={optname}) — no-op (macOS-only)");
        unsafe {
            *errno_ptr() = libc::ENOPROTOOPT;
        }
        return -1;
    };
    log::debug!(
        "getsockopt(fd={fd}, level={level}→{linux_level}, opt={optname}→{linux_optname}) — passthrough"
    );
    unsafe {
        libc::getsockopt(
            fd,
            linux_level,
            linux_optname,
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
    let Some(linux_optname) = translate_sockopt_name(linux_level, optname) else {
        // macOS-only option with no Linux equivalent — silently succeed.
        log::debug!("setsockopt(fd={fd}, level={level}, opt={optname}) — no-op (macOS-only)");
        return 0;
    };
    log::debug!(
        "setsockopt(fd={fd}, level={level}→{linux_level}, opt={optname}→{linux_optname}) — passthrough"
    );
    unsafe {
        libc::setsockopt(
            fd,
            linux_level,
            linux_optname,
            optval as *const libc::c_void,
            optlen,
        )
    }
}

/// `int shutdown(int sockfd, int how)`
pub extern "C" fn shim_shutdown(fd: i32, how: i32) -> i32 {
    log::debug!("shutdown(fd={fd}) — passthrough");
    #[cfg(target_os = "linux")]
    return with_errno_i32(|| unsafe { libc::shutdown(fd, how) });
    #[cfg(target_os = "macos")]
    unsafe {
        libc::shutdown(fd, how)
    }
}

// ── read/write shims (errno translation) ────────────────────────────────────
// These override the safe-list to translate Linux errno to macOS equiv.

/// `ssize_t read(int fd, void *buf, size_t count)`
pub extern "C" fn shim_read(fd: i32, buf: u64, count: u64) -> i64 {
    #[cfg(target_os = "linux")]
    {
        let ret = with_errno_i64(|| unsafe {
            libc::read(fd, buf as *mut libc::c_void, count as usize) as i64
        });
        log::trace!("read(fd={fd}, count={count}) → {ret}");
        ret
    }
    #[cfg(target_os = "macos")]
    unsafe {
        libc::read(fd, buf as *mut libc::c_void, count as usize) as i64
    }
}

/// `ssize_t write(int fd, const void *buf, size_t count)`
pub extern "C" fn shim_write(fd: i32, buf: u64, count: u64) -> i64 {
    #[cfg(target_os = "linux")]
    {
        let ret = with_errno_i64(|| unsafe {
            libc::write(fd, buf as *const libc::c_void, count as usize) as i64
        });
        log::trace!("write(fd={fd}, count={count}) → {ret}");
        ret
    }
    #[cfg(target_os = "macos")]
    unsafe {
        libc::write(fd, buf as *const libc::c_void, count as usize) as i64
    }
}

/// `int close(int fd)`
pub extern "C" fn shim_close(fd: i32) -> i32 {
    #[cfg(target_os = "linux")]
    return with_errno_i32(|| unsafe { libc::close(fd) });
    #[cfg(target_os = "macos")]
    unsafe {
        libc::close(fd)
    }
}

// ── sendmsg/recvmsg shims (sockaddr + errno translation) ────────────────────

/// `ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags)`
///
/// Reads the macOS-layout msghdr, builds a Linux-layout msghdr with
/// translated sockaddr, then calls Linux sendmsg.
pub extern "C" fn shim_sendmsg(fd: i32, msg_ptr: u64, flags: i32) -> i64 {
    #[cfg(target_os = "linux")]
    {
        if msg_ptr == 0 {
            return with_errno_i64(|| unsafe { libc::sendmsg(fd, std::ptr::null(), flags) as i64 });
        }
        // Read macOS-layout msghdr and convert to Linux layout.
        let darwin_msg = unsafe { DarwinMsghdr::read_from(msg_ptr) };
        let mut linux_msg = darwin_msg.to_linux();

        let mut sa_buf = [0u8; 128];
        let has_name = !darwin_msg.msg_name.is_null() && darwin_msg.msg_namelen > 0;
        if has_name {
            let tlen = translate_sockaddr(
                darwin_msg.msg_name as u64,
                darwin_msg.msg_namelen,
                &mut sa_buf,
            );
            linux_msg.msg_name = sa_buf.as_mut_ptr() as *mut libc::c_void;
            linux_msg.msg_namelen = tlen;
            log::debug!(
                "sendmsg(fd={fd}) — sockaddr translated (family={}, namelen={tlen}, addr_bytes={:?})",
                {
                    if tlen >= 2 {
                        u16::from_ne_bytes([sa_buf[0], sa_buf[1]])
                    } else {
                        0
                    }
                },
                &sa_buf[..std::cmp::min(tlen as usize, 16)]
            );
            let ret = with_errno_i64(|| unsafe { libc::sendmsg(fd, &linux_msg, flags) as i64 });
            log::debug!("sendmsg(fd={fd}) returned {ret}");
            return ret;
        }
        log::debug!(
            "sendmsg(fd={fd}) — no name, iovlen={}",
            darwin_msg.msg_iovlen
        );
        with_errno_i64(|| unsafe { libc::sendmsg(fd, &linux_msg, flags) as i64 })
    }
    #[cfg(target_os = "macos")]
    unsafe {
        libc::sendmsg(fd, msg_ptr as *const libc::msghdr, flags) as i64
    }
}

/// `ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags)`
///
/// Reads the macOS-layout msghdr, builds a Linux msghdr for the syscall,
/// then writes results back to the macOS layout and translates sockaddr + cmsg.
pub extern "C" fn shim_recvmsg(fd: i32, msg_ptr: u64, flags: i32) -> i64 {
    log::debug!("recvmsg(fd={fd}) — msghdr-translated + errno translation");
    #[cfg(target_os = "linux")]
    {
        if msg_ptr == 0 {
            return with_errno_i64(|| unsafe {
                libc::recvmsg(fd, std::ptr::null_mut(), flags) as i64
            });
        }
        // Read macOS-layout msghdr and convert to Linux layout.
        let darwin_msg = unsafe { DarwinMsghdr::read_from(msg_ptr) };
        let mut linux_msg = darwin_msg.to_linux();

        let ret = with_errno_i64(|| unsafe { libc::recvmsg(fd, &mut linux_msg, flags) as i64 });
        if ret >= 0 {
            // Write back the output fields to the macOS-layout msghdr.
            unsafe {
                darwin_msg.write_back(msg_ptr, &linux_msg);
            }

            // Translate the returned source sockaddr from Linux → macOS layout.
            if !linux_msg.msg_name.is_null() && linux_msg.msg_namelen >= 2 {
                translate_sockaddr_linux_to_darwin(
                    linux_msg.msg_name as *mut u8,
                    linux_msg.msg_namelen,
                );
            }
            // Translate control message (cmsg) buffer from Linux to macOS layout.
            // This repacks the buffer in-place and returns the new (smaller) length.
            if !linux_msg.msg_control.is_null() && linux_msg.msg_controllen > 0 {
                let darwin_controllen = translate_cmsg_linux_to_darwin(
                    linux_msg.msg_control as *mut u8,
                    linux_msg.msg_controllen,
                );
                // Patch the macOS msghdr controllen to the repacked size.
                unsafe {
                    let base = msg_ptr as *mut u8;
                    *(base.add(40) as *mut u32) = darwin_controllen;
                }
            }
            log::debug!(
                "recvmsg(fd={fd}) → {ret} bytes, namelen={}, controllen={}, flags={:#x}",
                linux_msg.msg_namelen,
                linux_msg.msg_controllen,
                linux_msg.msg_flags
            );
        }
        ret
    }
    #[cfg(target_os = "macos")]
    unsafe {
        libc::recvmsg(fd, msg_ptr as *mut libc::msghdr, flags) as i64
    }
}

// ── fcntl shim (O_NONBLOCK translation) ─────────────────────────────────────

// macOS open/fcntl file status flags that differ from Linux.
#[cfg(target_os = "linux")]
const DARWIN_O_NONBLOCK: i32 = 0x0004;
#[cfg(target_os = "linux")]
const DARWIN_O_APPEND: i32 = 0x0008;
#[cfg(target_os = "linux")]
const DARWIN_O_ASYNC: i32 = 0x0040;
#[cfg(target_os = "linux")]
const DARWIN_FIOCLEX: u64 = 0x2000_6601;
#[cfg(target_os = "linux")]
const DARWIN_FIONREAD: u64 = 0x4004_667f;
#[cfg(target_os = "linux")]
const DARWIN_FIONBIO: u64 = 0x8004_667e;
#[cfg(target_os = "linux")]
const DARWIN_TIOCEXCL: u64 = 0x2000_740d;
#[cfg(target_os = "linux")]
const DARWIN_TIOCGWINSZ: u64 = 0x4008_7468;

/// Translate macOS file-status flags (F_SETFL arg) to Linux equivalents.
/// Flags below 0x100 differ significantly between macOS and Linux, so we
/// do a bit-by-bit translation for the known ones and pass the rest through.
#[cfg(target_os = "linux")]
fn translate_file_flags(darwin_flags: i32) -> i32 {
    let mut linux_flags = 0i32;
    if darwin_flags & DARWIN_O_NONBLOCK != 0 {
        linux_flags |= libc::O_NONBLOCK;
    }
    if darwin_flags & DARWIN_O_APPEND != 0 {
        linux_flags |= libc::O_APPEND;
    }
    if darwin_flags & DARWIN_O_ASYNC != 0 {
        linux_flags |= libc::O_ASYNC;
    }
    // Pass through any higher flags (O_CLOEXEC, etc. are typically set via F_SETFD)
    linux_flags |= darwin_flags & !0x00FF;
    linux_flags
}

/// Translate Linux file-status flags back to macOS for F_GETFL return.
#[cfg(target_os = "linux")]
fn translate_file_flags_to_darwin(linux_flags: i32) -> i32 {
    let mut darwin_flags = 0i32;
    if linux_flags & libc::O_NONBLOCK != 0 {
        darwin_flags |= DARWIN_O_NONBLOCK;
    }
    if linux_flags & libc::O_APPEND != 0 {
        darwin_flags |= DARWIN_O_APPEND;
    }
    if linux_flags & libc::O_ASYNC != 0 {
        darwin_flags |= DARWIN_O_ASYNC;
    }
    darwin_flags |= linux_flags & !0x0FFF;
    darwin_flags
}

#[cfg(target_os = "linux")]
// libc's ioctl constants are c_ulong on GNU and c_int on musl. Keep the
// conversion to the guest's u64 representation on both libc families.
#[allow(clippy::unnecessary_cast)]
fn translate_ioctl_request(request: u64) -> u64 {
    match request {
        DARWIN_FIOCLEX => libc::FIOCLEX as u64,
        DARWIN_FIONREAD => libc::FIONREAD as u64,
        DARWIN_FIONBIO => libc::FIONBIO as u64,
        DARWIN_TIOCEXCL => libc::TIOCEXCL as u64,
        DARWIN_TIOCGWINSZ => libc::TIOCGWINSZ as u64,
        _ => request,
    }
}

/// `int ioctl(int fd, unsigned long request, ...)`
///
/// Apple ARM64 ABI puts variadic arguments on the stack, not in registers.
/// This mirrors `shim_fcntl` so the first variadic argument is loaded into x2.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
#[unsafe(naked)]
pub extern "C" fn shim_ioctl() -> i32 {
    core::arch::naked_asm!(
        "ldr  x2, [sp]",
        "b    {impl_fn}",
        impl_fn = sym shim_ioctl_impl,
    )
}

#[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
pub extern "C" fn shim_ioctl(fd: i32, request: u64, arg: u64) -> i32 {
    shim_ioctl_impl(fd, request, arg)
}

#[cfg(target_os = "macos")]
pub extern "C" fn shim_ioctl(fd: i32, request: u64, arg: u64) -> i32 {
    unsafe { libc::ioctl(fd, request as libc::c_ulong, arg as libc::c_ulong) }
}

#[cfg(target_os = "linux")]
// libc's ioctl constants differ between GNU (c_ulong) and musl (c_int), so
// request comparisons must normalize them to the guest's u64 representation.
#[allow(clippy::unnecessary_cast)]
extern "C" fn shim_ioctl_impl(fd: i32, request: u64, arg: u64) -> i32 {
    let translated_request = translate_ioctl_request(request);
    log::debug!("ioctl(fd={fd}, request={request:#x}) → {translated_request:#x}");

    let ret = match translated_request {
        v if v == libc::FIOCLEX as u64 || v == libc::TIOCEXCL as u64 => {
            with_errno_i32(|| unsafe { libc::ioctl(fd, translated_request as libc::c_ulong) })
        }
        v if v == libc::FIONREAD as u64 || v == libc::FIONBIO as u64 => with_errno_i32(|| unsafe {
            libc::ioctl(
                fd,
                translated_request as libc::c_ulong,
                arg as *mut libc::c_int,
            )
        }),
        v if v == libc::TIOCGWINSZ as u64 => with_errno_i32(|| unsafe {
            libc::ioctl(
                fd,
                translated_request as libc::c_ulong,
                arg as *mut libc::winsize,
            )
        }),
        _ => with_errno_i32(|| unsafe {
            libc::ioctl(
                fd,
                translated_request as libc::c_ulong,
                arg as libc::c_ulong,
            )
        }),
    };

    log::debug!("ioctl(fd={fd}, request={request:#x}) returned {ret}");
    ret
}

/// `int fcntl(int fd, int cmd, ...)`
///
/// Apple ARM64 ABI puts variadic arguments on the stack, not in registers.
/// This naked trampoline loads the first variadic arg from the stack into
/// x2 and tail-calls the real implementation.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
#[unsafe(naked)]
pub extern "C" fn shim_fcntl() -> i32 {
    core::arch::naked_asm!(
        "ldr  x2, [sp]",           // Load first variadic arg from Apple stack
        "b    {impl_fn}",          // Tail-call implementation
        impl_fn = sym shim_fcntl_impl,
    )
}

/// Non-aarch64 Linux hosts call the translated implementation directly.
#[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
pub extern "C" fn shim_fcntl(fd: i32, cmd: i32, arg: i64) -> i32 {
    shim_fcntl_impl(fd, cmd, arg)
}

/// macOS-host shim (no ABI translation needed).
#[cfg(target_os = "macos")]
pub extern "C" fn shim_fcntl(fd: i32, cmd: i32, arg: i64) -> i32 {
    unsafe { libc::fcntl(fd, cmd, arg as i32) }
}

/// Implementation of fcntl with O_NONBLOCK translation.
/// Called via the naked trampoline on Linux aarch64.
#[cfg(target_os = "linux")]
extern "C" fn shim_fcntl_impl(fd: i32, cmd: i32, arg: i64) -> i32 {
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    match cmd {
        F_GETFL => {
            let ret = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if ret < 0 {
                return ret;
            }
            let darwin_ret = translate_file_flags_to_darwin(ret);
            log::debug!("fcntl(fd={fd}, F_GETFL) → {ret:#x} → darwin {darwin_ret:#x}");
            darwin_ret
        }
        F_SETFL => {
            let darwin_flags = arg as i32;
            let linux_flags = translate_file_flags(darwin_flags);
            log::debug!("fcntl(fd={fd}, F_SETFL, {darwin_flags:#x}) → linux {linux_flags:#x}");
            unsafe { libc::fcntl(fd, libc::F_SETFL, linux_flags) }
        }
        _ => {
            log::debug!("fcntl(fd={fd}, cmd={cmd}) — passthrough");
            unsafe { libc::fcntl(fd, cmd, arg as i32) }
        }
    }
}

// ── Registration ────────────────────────────────────────────────────────────

pub struct SocketShims;

impl SocketShims {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_socket",
                shim_socket as *const () as u64,
                "socket — AF translation passthrough"
            ),
            (
                "_connect",
                shim_connect as *const () as u64,
                "connect — sockaddr translation passthrough"
            ),
            (
                "_bind",
                shim_bind as *const () as u64,
                "bind — sockaddr translation passthrough"
            ),
            (
                "_listen",
                shim_listen as *const () as u64,
                "listen — passthrough"
            ),
            (
                "_accept",
                shim_accept as *const () as u64,
                "accept — sockaddr translation"
            ),
            ("_send", shim_send as *const () as u64, "send — passthrough"),
            ("_recv", shim_recv as *const () as u64, "recv — passthrough"),
            (
                "_sendto",
                shim_sendto as *const () as u64,
                "sendto — sockaddr translation passthrough"
            ),
            (
                "_recvfrom",
                shim_recvfrom as *const () as u64,
                "recvfrom — sockaddr + errno translation"
            ),
            (
                "_getsockopt",
                shim_getsockopt as *const () as u64,
                "getsockopt — passthrough"
            ),
            (
                "_setsockopt",
                shim_setsockopt as *const () as u64,
                "setsockopt — passthrough"
            ),
            (
                "_shutdown",
                shim_shutdown as *const () as u64,
                "shutdown — passthrough"
            ),
            (
                "_getsockname",
                shim_getsockname as *const () as u64,
                "getsockname — sockaddr translation"
            ),
            (
                "_getpeername",
                shim_getpeername as *const () as u64,
                "getpeername — sockaddr translation"
            ),
            (
                "_ioctl",
                shim_ioctl as *const () as u64,
                "ioctl — Darwin request translation"
            ),
            (
                "_fcntl",
                shim_fcntl as *const () as u64,
                "fcntl — O_NONBLOCK translation"
            ),
            (
                "_read",
                shim_read as *const () as u64,
                "read — errno translation"
            ),
            (
                "_write",
                shim_write as *const () as u64,
                "write — errno translation"
            ),
            (
                "_close",
                shim_close as *const () as u64,
                "close — errno translation"
            ),
            (
                "_sendmsg",
                shim_sendmsg as *const () as u64,
                "sendmsg — sockaddr + errno translation"
            ),
            (
                "_recvmsg",
                shim_recvmsg as *const () as u64,
                "recvmsg — msghdr + cmsg + errno translation"
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shims consume Darwin sockaddr bytes even when tests run on Linux.
    #[repr(C)]
    struct DarwinSockaddrIn {
        len: u8,
        family: u8,
        port: u16,
        address: u32,
        padding: [u8; 8],
    }

    fn darwin_ipv4_address(address: [u8; 4], port: u16) -> DarwinSockaddrIn {
        DarwinSockaddrIn {
            len: std::mem::size_of::<DarwinSockaddrIn>() as u8,
            family: DARWIN_AF_INET as u8,
            port: port.to_be(),
            address: u32::from_ne_bytes(address),
            padding: [0; 8],
        }
    }

    /// Read current errno value.
    fn get_errno() -> i32 {
        unsafe { *errno_ptr() }
    }

    /// Clear errno before a test call.
    fn clear_errno() {
        unsafe {
            *errno_ptr() = 0;
        }
    }

    /// Close a file descriptor, ignoring errors.
    fn close_fd(fd: i32) {
        if fd >= 0 {
            unsafe {
                libc::close(fd);
            }
        }
    }

    // ── socket() tests ──────────────────────────────────────────────────

    #[test]
    fn test_socket_unix_allowed() {
        clear_errno();
        let fd = shim_socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
        assert!(
            fd >= 0,
            "AF_UNIX socket should succeed, got fd={fd} errno={}",
            get_errno()
        );
        close_fd(fd);
    }

    #[test]
    fn test_socket_inet_allowed() {
        clear_errno();
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(
            fd >= 0,
            "AF_INET socket should succeed, got fd={fd} errno={}",
            get_errno()
        );
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
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let sa = darwin_ipv4_address([127, 0, 0, 1], listener.local_addr().unwrap().port());

        clear_errno();
        let ret = shim_connect(
            fd,
            &sa as *const DarwinSockaddrIn as u64,
            std::mem::size_of::<DarwinSockaddrIn>() as u32,
        );
        assert_eq!(
            ret,
            0,
            "loopback connect should succeed, errno={}",
            get_errno()
        );
        close_fd(fd);
    }

    #[test]
    fn test_connect_external_allowed() {
        let sa = darwin_ipv4_address([8, 8, 8, 8], 53);
        // An invalid descriptor makes the host return EBADF immediately. That
        // proves the external address reached the host call without sending
        // network traffic or depending on a remote service.
        clear_errno();
        let ret = shim_connect(
            -1,
            &sa as *const DarwinSockaddrIn as u64,
            std::mem::size_of::<DarwinSockaddrIn>() as u32,
        );
        assert_eq!(ret, -1);
        assert_eq!(get_errno(), libc::EBADF);
    }

    // ── bind() tests ────────────────────────────────────────────────────

    #[test]
    fn test_bind_localhost_allowed() {
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let sa = darwin_ipv4_address([127, 0, 0, 1], 0);

        clear_errno();
        let ret = shim_bind(
            fd,
            &sa as *const DarwinSockaddrIn as u64,
            std::mem::size_of::<DarwinSockaddrIn>() as u32,
        );
        assert_eq!(
            ret,
            0,
            "bind to 127.0.0.1:0 should succeed, errno={}",
            get_errno()
        );
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
        assert_eq!(
            ret,
            0,
            "getsockopt SO_REUSEADDR should succeed, errno={}",
            get_errno()
        );
        close_fd(fd);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_ioctl_fioclex_translates() {
        // Descriptor flags do not depend on Unix-domain transport support.
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        clear_errno();
        let ret = shim_ioctl(fd, DARWIN_FIOCLEX, 0);
        assert_eq!(
            ret,
            0,
            "ioctl FIOCLEX should succeed, errno={}",
            get_errno()
        );

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert_ne!(flags & libc::FD_CLOEXEC, 0, "FD_CLOEXEC should be set");
        close_fd(fd);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_ioctl_fionbio_translates() {
        // Descriptor flags do not depend on Unix-domain transport support.
        let fd = shim_socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0);

        let mut enabled: i32 = 1;
        clear_errno();
        let ret = shim_ioctl(fd, DARWIN_FIONBIO, &mut enabled as *mut i32 as u64);
        assert_eq!(
            ret,
            0,
            "ioctl FIONBIO should succeed, errno={}",
            get_errno()
        );

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        assert_ne!(flags & libc::O_NONBLOCK, 0, "O_NONBLOCK should be set");
        close_fd(fd);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_ioctl_fionread_translates() {
        let mut fds: [i32; 2] = [-1, -1];
        let ret =
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
        assert_eq!(ret, 0, "socketpair failed, errno={}", get_errno());

        let msg = b"hello";
        let sent = unsafe { libc::write(fds[0], msg.as_ptr() as *const libc::c_void, msg.len()) };
        assert_eq!(sent, msg.len() as isize, "write should enqueue all bytes");

        let mut available: i32 = 0;
        clear_errno();
        let ret = shim_ioctl(fds[1], DARWIN_FIONREAD, &mut available as *mut i32 as u64);
        assert_eq!(
            ret,
            0,
            "ioctl FIONREAD should succeed, errno={}",
            get_errno()
        );
        assert_eq!(
            available,
            msg.len() as i32,
            "FIONREAD should report queued bytes"
        );

        close_fd(fds[0]);
        close_fd(fds[1]);
    }

    // ── send/recv over unix socketpair ──────────────────────────────────

    #[test]
    fn test_send_recv_unix_pair() {
        let mut fds: [i32; 2] = [-1, -1];
        let ret =
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
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
        assert!(table.resolve("_ioctl").is_some());
        assert_eq!(table.len(), 21);
    }
}
