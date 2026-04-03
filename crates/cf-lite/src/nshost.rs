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
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut std::ffi::c_char, buf.len()) };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    const SIZE: usize = 24;

    fn host_isa() -> u64 {
        crate::nsobj::nshost_class_addr()
    }

    // ── shim_nshost_current_host ───────────────────────────────

    #[test]
    fn current_host_returns_valid_object() {
        let h = shim_nshost_current_host(0, 0);
        assert_ne!(h, 0);
        // ISA should be set
        assert_eq!(read_field(h, 0), host_isa());
        // name field should be a non-null string
        let name_ptr = read_field(h, NAME_OFF);
        assert_ne!(name_ptr, 0);
        let name = read_string(name_ptr);
        assert!(!name.is_empty(), "hostname should not be empty");
        free_obj(h);
    }

    // ── shim_nshost_with_name ──────────────────────────────────

    #[test]
    fn with_name_stores_name() {
        let ns = make_string("myhost.local");
        let h = shim_nshost_with_name(0, 0, ns);
        assert_ne!(h, 0);
        assert_eq!(read_field(h, NAME_OFF), ns);
        assert_eq!(read_field(h, ADDR_OFF), 0);
        free_obj(h);
    }

    #[test]
    fn with_name_nil_name() {
        let h = shim_nshost_with_name(0, 0, 0);
        assert_ne!(h, 0);
        assert_eq!(read_field(h, NAME_OFF), 0);
        free_obj(h);
    }

    // ── shim_nshost_with_address ───────────────────────────────

    #[test]
    fn with_address_stores_address() {
        let addr = make_string("10.0.0.1");
        let h = shim_nshost_with_address(0, 0, addr);
        assert_ne!(h, 0);
        assert_eq!(read_field(h, NAME_OFF), 0);
        assert_eq!(read_field(h, ADDR_OFF), addr);
        free_obj(h);
    }

    #[test]
    fn with_address_nil_addr() {
        let h = shim_nshost_with_address(0, 0, 0);
        assert_ne!(h, 0);
        assert_eq!(read_field(h, ADDR_OFF), 0);
        free_obj(h);
    }

    // ── shim_nshost_name ───────────────────────────────────────

    #[test]
    fn name_returns_stored_name() {
        let ns = make_string("example.com");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, ns);
        assert_eq!(shim_nshost_name(h, 0), ns);
        free_obj(h);
    }

    #[test]
    fn name_nil_returns_zero() {
        assert_eq!(shim_nshost_name(0, 0), 0);
    }

    // ── shim_nshost_address ────────────────────────────────────

    #[test]
    fn address_returns_stored_address() {
        let addr = make_string("192.168.1.1");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, ADDR_OFF, addr);
        let result = shim_nshost_address(h, 0);
        assert_eq!(result, addr);
        free_obj(h);
    }

    #[test]
    fn address_nil_returns_zero() {
        assert_eq!(shim_nshost_address(0, 0), 0);
    }

    #[test]
    fn address_resolves_localhost() {
        let ns = make_string("localhost");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, ns);
        // addr_ptr is 0, so it should resolve via getaddrinfo
        let result = shim_nshost_address(h, 0);
        assert_ne!(result, 0);
        let addr_str = read_string(result);
        // localhost typically resolves to 127.0.0.1 or ::1 — shim returns an IP string
        assert!(!addr_str.is_empty());
        free_obj(h);
    }

    #[test]
    fn address_with_no_name_no_addr_returns_zero() {
        let h = alloc_obj(SIZE, host_isa());
        // both name and addr are 0 (from calloc)
        assert_eq!(shim_nshost_address(h, 0), 0);
        free_obj(h);
    }

    // ── shim_nshost_addresses ──────────────────────────────────

    #[test]
    fn addresses_with_stored_addr() {
        let addr = make_string("10.0.0.5");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, ADDR_OFF, addr);
        let arr = shim_nshost_addresses(h, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        free_obj(h);
    }

    #[test]
    fn addresses_nil_returns_empty_array() {
        let arr = shim_nshost_addresses(0, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    // ── shim_nshost_names ──────────────────────────────────────

    #[test]
    fn names_with_stored_name() {
        let ns = make_string("host.test");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, ns);
        let arr = shim_nshost_names(h, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 1);
        free_obj(h);
    }

    #[test]
    fn names_nil_returns_empty_array() {
        let arr = shim_nshost_names(0, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
    }

    #[test]
    fn names_no_name_returns_empty_array() {
        let h = alloc_obj(SIZE, host_isa());
        let arr = shim_nshost_names(h, 0);
        assert_ne!(arr, 0);
        assert_eq!(crate::nsarray::shim_nsarray_count(arr, 0), 0);
        free_obj(h);
    }

    // ── shim_nshost_localized_name ─────────────────────────────

    #[test]
    fn localized_name_delegates_to_name() {
        let ns = make_string("local.host");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, ns);
        assert_eq!(shim_nshost_localized_name(h, 0), ns);
        free_obj(h);
    }

    #[test]
    fn localized_name_nil_returns_zero() {
        assert_eq!(shim_nshost_localized_name(0, 0), 0);
    }

    // ── shim_nshost_is_equal ───────────────────────────────────

    #[test]
    fn is_equal_same_name() {
        let n1 = make_string("alpha");
        let n2 = make_string("alpha");
        let h1 = alloc_obj(SIZE, host_isa());
        let h2 = alloc_obj(SIZE, host_isa());
        write_field(h1, NAME_OFF, n1);
        write_field(h2, NAME_OFF, n2);
        assert_eq!(shim_nshost_is_equal(h1, 0, h2), 1);
        free_obj(h1);
        free_obj(h2);
    }

    #[test]
    fn is_equal_different_names() {
        let n1 = make_string("alpha");
        let n2 = make_string("beta");
        let h1 = alloc_obj(SIZE, host_isa());
        let h2 = alloc_obj(SIZE, host_isa());
        write_field(h1, NAME_OFF, n1);
        write_field(h2, NAME_OFF, n2);
        assert_eq!(shim_nshost_is_equal(h1, 0, h2), 0);
        free_obj(h1);
        free_obj(h2);
    }

    #[test]
    fn is_equal_nil_obj_returns_zero() {
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, make_string("x"));
        assert_eq!(shim_nshost_is_equal(0, 0, h), 0);
        free_obj(h);
    }

    #[test]
    fn is_equal_nil_other_returns_zero() {
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, make_string("x"));
        assert_eq!(shim_nshost_is_equal(h, 0, 0), 0);
        free_obj(h);
    }

    #[test]
    fn is_equal_both_nil_returns_zero() {
        assert_eq!(shim_nshost_is_equal(0, 0, 0), 0);
    }

    // ── shim_nshost_description ────────────────────────────────

    #[test]
    fn description_contains_hostname() {
        let ns = make_string("mybox");
        let h = alloc_obj(SIZE, host_isa());
        write_field(h, NAME_OFF, ns);
        let desc = shim_nshost_description(h, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHost: mybox>");
        free_obj(h);
    }

    #[test]
    fn description_nil_returns_zero() {
        assert_eq!(shim_nshost_description(0, 0), 0);
    }

    #[test]
    fn description_no_name_shows_unknown() {
        let h = alloc_obj(SIZE, host_isa());
        let desc = shim_nshost_description(h, 0);
        assert_ne!(desc, 0);
        assert_string_eq(desc, "<NSHost: unknown>");
        free_obj(h);
    }
}
