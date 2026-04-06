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
pub use recorder::{RequestRecorder, XpcRecord, DnsRecord, OsLogRecord};
pub use server::{ServiceServer, ServerHandle};

use std::sync::{Arc, OnceLock};

/// Global recorder instance shared across crates.
///
/// Set once during pipeline startup (see `macho-runtime/src/pipeline_native.rs`).
/// Other crates (e.g. libsystem-shims for XPC recording) can access it
/// via `burdettie_services::global_recorder()`.
static GLOBAL_RECORDER: OnceLock<Arc<RequestRecorder>> = OnceLock::new();

/// Install a recorder as the global singleton.
pub fn set_global_recorder(recorder: Arc<RequestRecorder>) {
    let _ = GLOBAL_RECORDER.set(recorder);
}

/// Get the global recorder (if one was installed).
pub fn global_recorder() -> Option<&'static Arc<RequestRecorder>> {
    GLOBAL_RECORDER.get()
}
