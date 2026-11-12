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
//! - `CGPoint` and `CGSize` are homogeneous floating-point aggregates returned
//!   in d0/d1.
//! - `CGRect` and `NSEdgeInsets` are four-element homogeneous floating-point
//!   aggregates returned in d0–d3, despite their 32-byte size.
//! - `NSRange` is an integer aggregate returned in x0/x1.

mod types;

pub use types::*;
