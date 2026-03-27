//! Message dispatch: objc_msgSend trampoline linkage + lookup callbacks.

mod fallback;
mod forwarding;
pub mod lookup;

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

// ── x86_64 assembly trampoline linkage ─────────────────────────────

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
extern "C" {
    fn shim_objc_msg_send_x86_64_asm();
    fn shim_objc_msg_send_stret_x86_64_asm();
    fn shim_objc_msg_send_fp2ret_x86_64_asm();
    fn shim_objc_msg_send_super_x86_64_asm();
    fn shim_objc_msg_send_super_stret_x86_64_asm();
    fn shim_objc_msg_send_super2_x86_64_asm();
    fn shim_objc_msg_send_super2_stret_x86_64_asm();
}

// Cfg predicate: native asm available on macOS for either arch.
// The fallback is used for all non-macOS targets (e.g. Linux).

pub fn objc_msg_send_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_rust as u64 }
}

pub fn objc_msg_send_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_stret_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_stret_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_rust as u64 }
}

pub fn objc_msg_send_fp2ret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_fp2ret_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_fp2ret_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_rust as u64 }
}

pub fn objc_msg_send_super_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_super_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_super_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_super_rust as u64 }
}

pub fn objc_msg_send_super_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_super_stret_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_super_stret_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_super_rust as u64 }
}

pub fn objc_msg_send_super2_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_super2_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_super2_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_super2_rust as u64 }
}

pub fn objc_msg_send_super2_stret_addr() -> u64 {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { shim_objc_msg_send_super2_stret_asm as u64 }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { shim_objc_msg_send_super2_stret_x86_64_asm as u64 }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "macos")))]
    { fallback::shim_objc_msg_send_super2_rust as u64 }
}
