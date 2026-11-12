---
description: "Foundation shim file conventions in cf-lite. Use when: editing crates/cf-lite/src/*.rs, writing shim functions, adding ObjC method shims."
applyTo: "crates/cf-lite/src/**/*.rs"
---

# cf-lite Shim Conventions

## Object Layout

Every shim file documents its object layout at the top:
```rust
//! Object layout: N bytes
//!   [0..8):  ISA pointer
//!   [8..16): field1 (type)
//!   ...
```

Field offsets are `const` values: `const FIELD_OFF: usize = 8;`

## Shim Function Signature

ObjC instance methods: `pub extern "C" fn shim_<class>_<method>(obj: u64, _cmd: u64, ...) -> u64`
ObjC class methods: `pub extern "C" fn shim_<class>_<method>(_cls: u64, _cmd: u64, ...) -> u64`

- First arg is always receiver (obj) or class (_cls)
- Second arg is always selector (_cmd), usually unused
- Return u64 (object pointer) or primitive
- Every shim MUST handle `obj == 0` (nil) gracefully — return 0 or no-op

## Allocation

Use `crate::pal_alloc().calloc(1, SIZE)` for new objects. Set ISA via `crate::nsobj::<class>_class_addr()`.

## Testing

Tests go inline: `#[cfg(test)] mod tests { use super::*; use crate::shim_test_helpers::*; }`
Run: `cargo test -p cf-lite -- <module>::tests`

## Build

Cargo writes to `target/` by default. For a custom build location, use
`CARGO_TARGET_DIR` and resolve the directory through `cargo metadata`.
