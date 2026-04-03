//! macOS memory mapping via mmap/mprotect/munmap.

use platform_core::memory::MemoryMapper;
use platform_core::{PalError, Protection};

pub struct DarwinMemoryMapper;

impl MemoryMapper for DarwinMemoryMapper {
    fn page_size(&self) -> usize {
        unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
    }

    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError> {
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            Err(PalError::Memory(format!(
                "mmap failed for {} bytes: {}",
                size,
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(ptr as *mut u8)
        }
    }

    fn protect(&self, addr: *mut u8, size: usize, prot: Protection) -> Result<(), PalError> {
        let flags = prot_to_mmap(prot);
        let ret = unsafe { libc::mprotect(addr as *mut libc::c_void, size, flags) };
        if ret != 0 {
            Err(PalError::Memory(format!(
                "mprotect failed: {}",
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(())
        }
    }

    fn unmap(&self, addr: *mut u8, size: usize) -> Result<(), PalError> {
        let ret = unsafe { libc::munmap(addr as *mut libc::c_void, size) };
        if ret != 0 {
            Err(PalError::Memory(format!(
                "munmap failed: {}",
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(())
        }
    }
}

fn prot_to_mmap(p: Protection) -> libc::c_int {
    let mut flags = libc::PROT_NONE;
    if p.read {
        flags |= libc::PROT_READ;
    }
    if p.write {
        flags |= libc::PROT_WRITE;
    }
    if p.execute {
        flags |= libc::PROT_EXEC;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_power_of_two() {
        let m = DarwinMemoryMapper;
        let ps = m.page_size();
        assert!(ps > 0);
        assert!(ps.is_power_of_two(), "page_size {ps} is not a power of 2");
    }

    #[test]
    fn map_anonymous_succeeds_and_zeroed() {
        let m = DarwinMemoryMapper;
        let size = m.page_size();
        let ptr = m.map_anonymous(size).unwrap();
        assert!(!ptr.is_null());
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        assert!(slice.iter().all(|&b| b == 0));
        m.unmap(ptr, size).unwrap();
    }

    #[test]
    fn protect_read_only() {
        let m = DarwinMemoryMapper;
        let size = m.page_size();
        let ptr = m.map_anonymous(size).unwrap();
        m.protect(ptr, size, Protection::READ_ONLY).unwrap();
        // Can still read
        let _val = unsafe { *ptr };
        m.unmap(ptr, size).unwrap();
    }

    #[test]
    fn unmap_succeeds() {
        let m = DarwinMemoryMapper;
        let size = m.page_size();
        let ptr = m.map_anonymous(size).unwrap();
        assert!(m.unmap(ptr, size).is_ok());
    }
}
