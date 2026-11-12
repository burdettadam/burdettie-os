//! Message dispatch: objc_msgSend trampoline linkage + lookup callbacks.

mod fallback;
mod forwarding;
pub mod host_forward;
pub mod lookup;

// ── ARM64 assembly trampoline linkage ──────────────────────────────

#[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
extern "C" {
    fn shim_objc_msg_send_asm();
    fn shim_objc_msg_send_stret_asm();
    fn shim_objc_msg_send_fp2ret_asm();
    fn shim_objc_msg_send_super_asm();
    fn shim_objc_msg_send_super_stret_asm();
    fn shim_objc_msg_send_super2_asm();
    fn shim_objc_msg_send_super2_stret_asm();
}

// ── x86_64 assembly trampoline linkage ─────────────────────────────

#[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
extern "C" {
    fn shim_objc_msg_send_x86_64_asm();
    fn shim_objc_msg_send_stret_x86_64_asm();
    fn shim_objc_msg_send_fp2ret_x86_64_asm();
    fn shim_objc_msg_send_super_x86_64_asm();
    fn shim_objc_msg_send_super_stret_x86_64_asm();
    fn shim_objc_msg_send_super2_x86_64_asm();
    fn shim_objc_msg_send_super2_stret_x86_64_asm();
}

// Cfg predicate: native asm available on aarch64 macOS/Linux and x86_64 macOS.
// The fallback is used for all other targets.

pub fn objc_msg_send_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_rust as *const () as u64
    }
}

pub fn objc_msg_send_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_stret_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_stret_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_rust as *const () as u64
    }
}

pub fn objc_msg_send_fp2ret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_fp2ret_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_fp2ret_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_rust as *const () as u64
    }
}

pub fn objc_msg_send_super_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_super_rust as *const () as u64
    }
}

pub fn objc_msg_send_super_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super_stret_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super_stret_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_super_rust as *const () as u64
    }
}

pub fn objc_msg_send_super2_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super2_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super2_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_super2_rust as *const () as u64
    }
}

pub fn objc_msg_send_super2_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super2_stret_asm as *const () as u64
    }
    #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux")))]
    {
        shim_objc_msg_send_super2_stret_x86_64_asm as *const () as u64
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")),
        all(target_arch = "x86_64", any(target_os = "macos", target_os = "linux"))
    )))]
    {
        fallback::shim_objc_msg_send_super2_rust as *const () as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_addresses_are_non_zero() {
        assert_ne!(objc_msg_send_addr(), 0);
        assert_ne!(objc_msg_send_stret_addr(), 0);
        assert_ne!(objc_msg_send_super_addr(), 0);
        assert_ne!(objc_msg_send_super_stret_addr(), 0);
        assert_ne!(objc_msg_send_super2_addr(), 0);
        assert_ne!(objc_msg_send_super2_stret_addr(), 0);
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn x86_64_linux_uses_native_dispatch_trampolines() {
        let fallback_msg = fallback::shim_objc_msg_send_rust as *const () as u64;
        let fallback_super = fallback::shim_objc_msg_send_super_rust as *const () as u64;
        let fallback_super2 = fallback::shim_objc_msg_send_super2_rust as *const () as u64;

        assert_ne!(objc_msg_send_addr(), fallback_msg);
        assert_ne!(objc_msg_send_stret_addr(), fallback_msg);
        assert_ne!(objc_msg_send_fp2ret_addr(), fallback_msg);
        assert_ne!(objc_msg_send_super_addr(), fallback_super);
        assert_ne!(objc_msg_send_super_stret_addr(), fallback_super);
        assert_ne!(objc_msg_send_super2_addr(), fallback_super2);
        assert_ne!(objc_msg_send_super2_stret_addr(), fallback_super2);
    }
}
