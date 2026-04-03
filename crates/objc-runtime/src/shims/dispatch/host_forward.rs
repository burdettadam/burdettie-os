//! Host-object forwarding for objc_msgSend — DISABLED.
//!
//! This module previously forwarded messages for unknown-ISA objects to the
//! host macOS ObjC runtime.  All host forwarding has been removed in favour
//! of a fully self-contained guest runtime that can run on Linux/Docker.
//!
//! The `#[no_mangle]` statics below are kept because the ARM64 assembly
//! trampoline (`trampoline.s`) still references them at link time even
//! though the code path that calls it is dead.

use std::sync::atomic::AtomicU64;

/// Host `objc_msgSend` function pointer — always 0 (host forwarding disabled).
#[no_mangle]
pub static HOST_OBJC_MSG_SEND: AtomicU64 = AtomicU64::new(0);

/// Host `sel_registerName` function pointer — always 0 (host forwarding disabled).
#[no_mangle]
pub static HOST_SEL_REGISTER_NAME: AtomicU64 = AtomicU64::new(0);
