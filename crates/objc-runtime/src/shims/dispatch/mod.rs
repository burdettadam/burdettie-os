//! Message dispatch: objc_msgSend trampoline linkage + lookup callbacks.

mod fallback;
mod forwarding;
mod lookup;

// ── ARM64 assembly trampoline linkage ──────────────────────────────

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
extern "C" {
    fn shim_objc_msg_send_asm();
    fn shim_objc_msg_send_stret_asm();
    fn shim_objc_msg_send_fp2ret_asm();
    fn shim_objc_msg_send_super_asm();
    fn shim_objc_msg_send_super_stret_asm();
    fn shim_objc_msg_send_super2_asm();
    fn shim_objc_msg_send_super2_stret_asm();
}

pub fn objc_msg_send_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_rust as u64
    }
}

pub fn objc_msg_send_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_stret_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_rust as u64
    }
}

pub fn objc_msg_send_fp2ret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_fp2ret_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_rust as u64
    }
}

pub fn objc_msg_send_super_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_super_rust as u64
    }
}

pub fn objc_msg_send_super_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super_stret_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_super_rust as u64
    }
}

pub fn objc_msg_send_super2_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super2_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_super2_rust as u64
    }
}

pub fn objc_msg_send_super2_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        shim_objc_msg_send_super2_stret_asm as u64
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "macos")))]
    {
        fallback::shim_objc_msg_send_super2_rust as u64
    }
}
