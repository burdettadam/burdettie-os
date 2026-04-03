//! Linux memory mapping via mmap/mprotect/munmap.

use platform_core::memory::MemoryMapper;
use platform_core::{PalError, Protection};

pub struct LinuxMemoryMapper;

impl MemoryMapper for LinuxMemoryMapper {
    fn page_size(&self) -> usize {
        unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
    }

    fn map_anonymous(&self, size: usize) -> Result<*mut u8, PalError> {
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
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

    /// Map image memory using `memfd_create` so that RW→RX mprotect
    /// transitions succeed even with SELinux `deny_execmem` enabled.
    ///
    /// File-backed MAP_SHARED mappings are exempt from the anonymous-execmem
    /// restriction because the kernel can verify the backing store's
    /// security context.  If `memfd_create` is unavailable (pre-3.17
    /// kernels), falls back to anonymous mapping.
    #[cfg(target_os = "linux")]
    fn map_image_pages(&self, size: usize) -> Result<*mut u8, PalError> {
        // Create an anonymous file descriptor via memfd_create.
        let name = b"macho-image\0";
        let fd = unsafe {
            libc::memfd_create(name.as_ptr() as *const libc::c_char, libc::MFD_CLOEXEC)
        };
        if fd < 0 {
            // Kernel too old for memfd — fall back to anonymous mapping.
            log::debug!("memfd_create unavailable, falling back to anonymous mapping");
            return self.map_anonymous(size);
        }

        // Size the backing file.
        if unsafe { libc::ftruncate(fd, size as libc::off_t) } != 0 {
            let err = std::io::Error::last_os_error();
            unsafe { libc::close(fd); }
            return Err(PalError::Memory(format!("ftruncate (memfd) failed: {err}")));
        }

        // Map RW + SHARED so mprotect to RX is allowed on hardened kernels.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        // Seal the fd so no further size changes are possible, then close it.
        // The mapping holds a kernel reference to the underlying inode.
        unsafe {
            // F_ADD_SEALS with F_SEAL_SHRINK prevents truncation after mapping.
            libc::fcntl(fd, libc::F_ADD_SEALS, libc::F_SEAL_SHRINK);
            libc::close(fd);
        }

        if ptr == libc::MAP_FAILED {
            Err(PalError::Memory(format!(
                "mmap (memfd) failed for {} bytes: {}",
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
