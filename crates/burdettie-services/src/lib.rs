//! BurdettieOS local service server.
//!
//! A synchronous HTTP server that replaces Apple cloud services with local
//! handlers.  Runs in a background thread alongside the Mach-O loader.
//!
//! # Architecture
//!
//! ```text
//! Guest Mach-O ──► NSURLSession ──► domain-map rewrite ──► tiny_http server
//!                                                              │
//!                                                    ┌────────┴────────┐
//!                                                    │ Router          │
//!                                                    │  ├─ telemetry   │
//!                                                    │  ├─ captive     │
//!                                                    │  ├─ activate    │
//!                                                    │  └─ ...         │
//!                                                    └────────┬────────┘
//!                                                             │
//!                                                    ┌────────▼────────┐
//!                                                    │ Request         │
//!                                                    │ Recorder        │
//!                                                    │ (SQLite)        │
//!                                                    └─────────────────┘
//! ```

pub mod config;
pub mod handlers;
pub mod recorder;
pub mod router;
pub mod server;

pub use config::ServiceConfig;
pub use server::{ServiceServer, ServerHandle};
