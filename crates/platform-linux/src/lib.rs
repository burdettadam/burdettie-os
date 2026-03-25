//! Linux host implementation of `platform-core` traits (stub).
//!
//! Currently provides only `LinuxUnwindRegistry`.  The remaining PAL traits
//! (clock, rng, alloc, …) will be implemented in Phase E1.

mod unwind;

pub use unwind::LinuxUnwindRegistry;
