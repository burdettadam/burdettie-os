//! NSHost shim: hostname and address resolution.
//!
//! Object layout (24 bytes):
//!   offset  0: isa       (u64)
//!   offset  8: name_ptr  (u64 — NSString hostname)
//!   offset 16: addr_ptr  (u64 — NSString address, 0 for name-based)

use crate::cfstring;
use crate::nsobj;

const NAME_OFF: usize = 8;
const ADDR_OFF: usize = 16;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_host(name: u64, addr: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, 24) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = nsobj::nshost_class_addr();
        *((obj as usize + NAME_OFF) as *mut u64) = name;
        *((obj as usize + ADDR_OFF) as *mut u64) = addr;
    }
    obj
}

// ── Class methods ──────────────────────────────────────────────

/// `+[NSHost currentHost]`
pub extern "C" fn shim_nshost_current_host(_cls: u64, _cmd: u64) -> u64 {
    let mut buf = [0u8; 256];
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut i8, buf.len()) };
    let name = if rc == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..len]).to_string()
    } else {
        "localhost".to_string()
    };
    alloc_host(make_nsstring(&name), 0)
}

/// `+[NSHost hostWithName:]`
pub extern "C" fn shim_nshost_with_name(_cls: u64, _cmd: u64, name: u64) -> u64 {
    alloc_host(name, 0)
}

/// `+[NSHost hostWithAddress:]`
pub extern "C" fn shim_nshost_with_address(_cls: u64, _cmd: u64, addr: u64) -> u64 {
    alloc_host(0, addr)
}

// ── Instance methods ───────────────────────────────────────────

/// `-[NSHost name]`
pub extern "C" fn shim_nshost_name(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + NAME_OFF) as *const u64) }
}

/// `-[NSHost address]`
pub extern "C" fn shim_nshost_address(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let addr = unsafe { *((obj as usize + ADDR_OFF) as *const u64) };
    if addr != 0 { return addr; }
    // Resolve name to address via getaddrinfo
    let name_ptr = unsafe { *((obj as usize + NAME_OFF) as *const u64) };
    if name_ptr == 0 { return 0; }
    let Some(name) = cfstring::extract_cfstring_cstr(name_ptr) else { return 0; };
    let c_name = std::ffi::CString::new(name).unwrap_or_default();
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe { libc::getaddrinfo(c_name.as_ptr(), std::ptr::null(), std::ptr::null(), &mut result) };
    if rc != 0 || result.is_null() { return make_nsstring("127.0.0.1"); }
    let addr_str = unsafe {
        let ai = &*result;
        if ai.ai_family == libc::AF_INET && !ai.ai_addr.is_null() {
            let sin = &*(ai.ai_addr as *const libc::sockaddr_in);
            let ip = sin.sin_addr.s_addr.to_be();
            format!("{}.{}.{}.{}", (ip >> 24) & 0xFF, (ip >> 16) & 0xFF, (ip >> 8) & 0xFF, ip & 0xFF)
        } else {
            "127.0.0.1".to_string()
        }
    };
    unsafe { libc::freeaddrinfo(result); }
    make_nsstring(&addr_str)
}

/// `-[NSHost addresses]` — returns NSArray of address strings.
pub extern "C" fn shim_nshost_addresses(obj: u64, _cmd: u64) -> u64 {
    let addr = shim_nshost_address(obj, 0);
    if addr == 0 { return crate::nsarray::make_nsarray(&[]); }
    crate::nsarray::make_nsarray(&[addr])
}

/// `-[NSHost names]` — returns NSArray of name strings.
pub extern "C" fn shim_nshost_names(obj: u64, _cmd: u64) -> u64 {
    let name = shim_nshost_name(obj, 0);
    if name == 0 { return crate::nsarray::make_nsarray(&[]); }
    crate::nsarray::make_nsarray(&[name])
}

/// `-[NSHost localizedName]`
pub extern "C" fn shim_nshost_localized_name(obj: u64, _cmd: u64) -> u64 {
    shim_nshost_name(obj, 0)
}

/// `-[NSHost isEqualToHost:]`
pub extern "C" fn shim_nshost_is_equal(obj: u64, _cmd: u64, other: u64) -> u8 {
    if obj == 0 || other == 0 { return 0; }
    let n1 = cfstring::extract_cfstring_cstr(shim_nshost_name(obj, 0));
    let n2 = cfstring::extract_cfstring_cstr(shim_nshost_name(other, 0));
    u8::from(n1 == n2)
}

pub extern "C" fn shim_nshost_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let name = cfstring::extract_cfstring_cstr(shim_nshost_name(obj, 0))
        .unwrap_or("unknown");
    make_nsstring(&format!("<NSHost: {name}>"))
}
