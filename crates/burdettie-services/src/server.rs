//! HTTP server — runs in a background thread, serves BurdettieOS API endpoints.

use crate::config::ServiceConfig;
use crate::recorder::RequestRecorder;
use crate::router::Router;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Handle to a running service server.
pub struct ServerHandle {
    /// The port the server is listening on.
    pub port: u16,
    /// The full bind address (e.g. `127.0.0.1:12345`).
    pub addr: String,
    /// Signal to stop the server.
    stop: Arc<AtomicBool>,
    /// Background thread handle.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ServerHandle {
    /// Signal the server to stop and wait for it to finish.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    /// Check if the server is still running.
    pub fn is_running(&self) -> bool {
        self.thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Don't join on drop — the thread will exit on next request timeout.
    }
}

/// The service server.
pub struct ServiceServer;

impl ServiceServer {
    /// Start the service server in a background thread.
    ///
    /// Returns a `ServerHandle` with the bound port and a stop signal.
    /// The server processes requests until the handle is dropped or `stop()` is
    /// called.
    pub fn start(config: &ServiceConfig) -> Result<ServerHandle, String> {
        let server = tiny_http::Server::http(&config.bind_addr)
            .map_err(|e| format!("failed to bind {}: {e}", config.bind_addr))?;

        let addr = server
            .server_addr()
            .to_ip()
            .map(|a| a.to_string())
            .unwrap_or_else(|| config.bind_addr.clone());

        let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);

        // Set up the request recorder
        let recorder = if config.record_requests {
            match RequestRecorder::open(&config.record_db_path) {
                Ok(r) => {
                    log::info!(
                        "burdettie-services: recording requests to {}",
                        config.record_db_path
                    );
                    let arc = Arc::new(r);
                    // Install as global recorder so other crates can use it
                    crate::set_global_recorder(Arc::clone(&arc));
                    Some(arc)
                }
                Err(e) => {
                    log::warn!("burdettie-services: failed to open recorder: {e}");
                    None
                }
            }
        } else {
            None
        };

        let router = Arc::new(Router::new(recorder, config));
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        log::info!("burdettie-services: listening on {addr} (port {port})");

        let thread = std::thread::Builder::new()
            .name("burdettie-services".into())
            .spawn(move || {
                serve_loop(server, router, stop_clone);
            })
            .map_err(|e| format!("failed to spawn server thread: {e}"))?;

        Ok(ServerHandle {
            port,
            addr,
            stop,
            thread: Some(thread),
        })
    }
}

/// Main server loop — processes requests until stopped.
fn serve_loop(server: tiny_http::Server, router: Arc<Router>, stop: Arc<AtomicBool>) {
    loop {
        if stop.load(Ordering::Acquire) {
            log::debug!("burdettie-services: stop signal received");
            break;
        }

        // Use recv_timeout so we can check the stop flag periodically.
        let mut request = match server.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(Some(req)) => req,
            Ok(None) => continue, // timeout, check stop flag
            Err(e) => {
                log::warn!("burdettie-services: accept error: {e}");
                continue;
            }
        };

        // Read the request body
        let mut body = Vec::new();
        let content_length = request
            .headers()
            .iter()
            .find(|h| {
                h.field
                    .as_str()
                    .as_str()
                    .eq_ignore_ascii_case("content-length")
            })
            .and_then(|h| h.value.as_str().parse::<usize>().ok())
            .unwrap_or(0);

        if content_length > 0 {
            body.resize(content_length, 0);
            let reader = request.as_reader();
            let _ = reader.read_exact(&mut body);
        }

        let path = request.url().to_string();
        let method = request.method().as_str().to_string();

        // Extract original URL from X-Burdettie-Original-URL header if present
        let original_url = request
            .headers()
            .iter()
            .find(|h| {
                h.field
                    .as_str()
                    .as_str()
                    .eq_ignore_ascii_case("x-burdettie-original-url")
            })
            .map(|h| h.value.as_str().to_string())
            .unwrap_or_else(|| path.clone());

        let response = router.route(&path, &method, &body, &request, &original_url);

        if let Err(e) = request.respond(response.into_tiny_response()) {
            log::warn!("burdettie-services: response error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn server_starts_and_stops() {
        let config = ServiceConfig {
            bind_addr: "127.0.0.1:0".into(),
            record_requests: false,
            record_db_path: String::new(),
            replay_db_path: None,
            log_format: crate::config::LogFormat::Text,
            log_sink: crate::config::LogSink::Stdout,
            admin: crate::config::AdminConfig {
                enabled: false,
                token: None,
            },
            remote: crate::config::RemoteConfig {
                endpoint: None,
                auth_token: None,
            },
            instance_id: "test".into(),
            workspace_root: ".".into(),
        };

        let handle = ServiceServer::start(&config).expect("server should start");
        assert!(handle.port > 0, "should bind to a real port");
        assert!(handle.is_running());

        // Make a test request to the captive endpoint
        let url = format!("http://{}/api/v1/network/captive", handle.addr);
        let resp = ureq::get(&url).call();
        match resp {
            Ok(mut r) => {
                assert_eq!(r.status().as_u16(), 200);
                let body = r.body_mut().read_to_string().unwrap_or_default();
                assert!(
                    body.contains("Success"),
                    "captive should return Success HTML"
                );
            }
            Err(e) => panic!("request failed: {e}"),
        }

        // Test telemetry endpoint
        let url = format!("http://{}/api/v1/telemetry/xp/report", handle.addr);
        let resp = ureq::post(&url).send(b"test telemetry data");
        match resp {
            Ok(mut r) => {
                assert_eq!(r.status().as_u16(), 200);
                let body = r.body_mut().read_to_string().unwrap_or_default();
                assert!(body.contains("burdettie"));
            }
            Err(e) => panic!("request failed: {e}"),
        }

        handle.stop();
    }

    #[test]
    fn server_with_recording() {
        let config = ServiceConfig {
            bind_addr: "127.0.0.1:0".into(),
            record_requests: true,
            record_db_path: ":memory:".into(),
            replay_db_path: None,
            log_format: crate::config::LogFormat::Text,
            log_sink: crate::config::LogSink::Stdout,
            admin: crate::config::AdminConfig {
                enabled: false,
                token: None,
            },
            remote: crate::config::RemoteConfig {
                endpoint: None,
                auth_token: None,
            },
            instance_id: "test".into(),
            workspace_root: ".".into(),
        };

        let handle = ServiceServer::start(&config).expect("server should start");
        assert!(handle.port > 0);

        // Hit the device activation endpoint
        let url = format!("http://{}/api/v1/device/activate", handle.addr);
        let resp = ureq::post(&url).send(b"<plist>activation request</plist>");
        match resp {
            Ok(mut r) => {
                assert_eq!(r.status().as_u16(), 200);
                let body = r.body_mut().read_to_string().unwrap_or_default();
                assert!(body.contains("Activated"));
            }
            Err(e) => panic!("request failed: {e}"),
        }

        handle.stop();
    }

    #[test]
    fn server_exposes_memory_admin_status_and_sync() {
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join("mempalace.yaml"),
            "wing: test\nrooms:\n  - name: memories\n    keywords:\n      - memories\n",
        )
        .expect("write room catalog");
        fs::create_dir_all(temp.path().join("memories")).expect("create memories dir");
        fs::write(
            temp.path().join("memories/preferences.md"),
            "- prefers terse output\n",
        )
        .expect("write memory source");

        let config = ServiceConfig {
            bind_addr: "127.0.0.1:0".into(),
            record_requests: false,
            record_db_path: String::new(),
            replay_db_path: None,
            log_format: crate::config::LogFormat::Text,
            log_sink: crate::config::LogSink::Stdout,
            admin: crate::config::AdminConfig {
                enabled: true,
                token: None,
            },
            remote: crate::config::RemoteConfig {
                endpoint: None,
                auth_token: None,
            },
            instance_id: "test".into(),
            workspace_root: temp.path().display().to_string(),
        };

        let handle = ServiceServer::start(&config).expect("server should start");

        let status_url = format!("http://{}/_burdettie/admin/memory/status", handle.addr);
        let mut status = ureq::get(&status_url).call().expect("status request");
        assert_eq!(status.status().as_u16(), 200);
        let status_body = status.body_mut().read_to_string().expect("status body");
        let status_json: serde_json::Value =
            serde_json::from_str(&status_body).expect("status json");
        assert_eq!(status_json["objects"]["active"], 0);
        assert_eq!(status_json["last_sync_id"], 0);

        let sync_url = format!("http://{}/_burdettie/admin/memory/sync", handle.addr);
        let mut sync = ureq::post(&sync_url).send(b"").expect("sync request");
        assert_eq!(sync.status().as_u16(), 200);
        let sync_body = sync.body_mut().read_to_string().expect("sync body");
        let sync_json: serde_json::Value = serde_json::from_str(&sync_body).expect("sync json");
        assert_eq!(sync_json["sync"]["scanned_files"], 1);
        assert_eq!(sync_json["sync"]["indexed_objects"], 1);

        let mut status_after = ureq::get(&status_url).call().expect("status after sync");
        assert_eq!(status_after.status().as_u16(), 200);
        let status_after_body = status_after
            .body_mut()
            .read_to_string()
            .expect("status after body");
        let status_after_json: serde_json::Value =
            serde_json::from_str(&status_after_body).expect("status after json");
        assert_eq!(status_after_json["objects"]["active"], 1);
        assert!(
            status_after_json["chunks"]["active"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );

        handle.stop();
    }
}
