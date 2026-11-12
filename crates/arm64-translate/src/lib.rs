//! ARM64 (AArch64) binary translator / interpreter.
//!
//! Phase E3 — "the moonshot": execute ARM64 Mach-O machine code on any
//! host architecture via interpretation. This enables running macOS ARM64
//! binaries on x86_64 hosts without Rosetta.
//!
//! # Architecture
//!
//! - **`cpu`** — CPU state: 31 general-purpose registers, SP, PC, NZCV flags
//! - **`decode`** — A64 instruction decoder (fixed 32-bit encoding)
//! - **`interp`** — Fetch-decode-execute interpreter loop
//! - **`mem`** — Memory model: segment-backed reads/writes + interpreter stack
//! - **`ffi`** — Foreign function interface bridge: ARM64 calling convention → host

pub mod cpu;
pub mod decode;
pub mod ffi;
pub mod interp;
pub mod jit;
pub mod mem;

#[cfg(test)]
pub mod test_helpers;

pub use cpu::CpuState;
pub use decode::Instruction;
pub use ffi::{call_host_capturing, FfiBridge, FfiCallArgs, FfiResult};
pub use interp::{ExitReason, Interpreter};
pub use mem::Memory;
