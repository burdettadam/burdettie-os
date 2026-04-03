//! CoreFoundation-layer primitive types.
//!
//! This crate provides `#[repr(C)]` geometry and range types that mirror
//! Apple's CoreFoundation / CoreGraphics C types. These are the lowest-level
//! types in the Apple stack and have zero dependencies.
//!
//! # Apple Stack Alignment
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  AppKit / UIKit            (GUI layer)              │
//! ├─────────────────────────────────────────────────────┤
//! │  Foundation                 (ObjC layer)            │
//! ├─────────────────────────────────────────────────────┤
//! │  CoreFoundation             (C layer)               │
//! │  ── core-foundation-lite lives here ──              │
//! │  CGFloat, CGPoint, CGSize, CGRect, NSRange          │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # ABI Layout
//!
//! On ARM64:
//! - Structs ≤16 bytes (`CGPoint`, `CGSize`, `NSRange`) are returned in x0/x1
//! - Structs >16 bytes (`CGRect`, `NSEdgeInsets`) use x8 as the indirect
//!   result pointer, handled by the `objc_msgSend` trampoline

mod types;

pub use types::*;
