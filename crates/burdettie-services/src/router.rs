//! Request router — dispatches incoming HTTP requests to the appropriate handler.

use crate::config::ServiceConfig;
use crate::handlers::{self, Handler, HandlerResponse};
use crate::recorder::{HttpRecord, RequestRecorder};
use std::sync::Arc;
use std::time::Instant;

/// Routes requests through an ordered list of handlers.
pub struct Router {
    handlers: Vec<Box<dyn Handler>>,
    recorder: Option<Arc<RequestRecorder>>,
}

impl Router {
    /// Create a router with the default set of handlers.
    ///
    /// If `replay_db` is `Some`, a `ReplayHandler` is prepended so that
    /// previously-recorded responses are served first.
    pub fn new(recorder: Option<Arc<RequestRecorder>>, config: &ServiceConfig) -> Self {
        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();

        // Replay handler goes first — if it matches, other handlers are skipped
        if let Some(ref db_path) = config.replay_db_path {
            if let Some(replay) = handlers::replay::ReplayHandler::open(db_path) {
                log::info!("burdettie-services: replay handler active from {db_path}");
                handler_list.push(Box::new(replay));
            }
        }

        // Status endpoint — always available
        handlers::status::init_start_time();
        handler_list.push(Box::new(handlers::status::StatusHandler));

        // Admin API (before domain-specific handlers)
        if config.admin.enabled {
            handler_list.push(Box::new(handlers::admin::AdminHandler::new(
                config.admin.token.clone(),
                config.instance_id.clone(),
            )));
        }

        // Standard handlers in priority order
        handler_list.push(Box::new(handlers::telemetry::TelemetryHandler));
        handler_list.push(Box::new(handlers::captive::CaptiveHandler));
        handler_list.push(Box::new(handlers::device::DeviceHandler));
        handler_list.push(Box::new(handlers::swupdate::SwupdateHandler));
        handler_list.push(Box::new(handlers::certs::CertsHandler));
        handler_list.push(Box::new(handlers::xcode::XcodeHandler));
        handler_list.push(Box::new(handlers::auth::AuthHandler));
        handler_list.push(Box::new(handlers::dns::DnsHandler));
        handler_list.push(Box::new(handlers::notarize::NotarizeHandler));
        handler_list.push(Box::new(handlers::icloud::ICloudHandler));
        handler_list.push(Box::new(handlers::push::PushHandler));
        handler_list.push(Box::new(handlers::cache::CacheHandler));
        handler_list.push(Box::new(handlers::time::TimeHandler));
        handler_list.push(Box::new(handlers::diagnostics::DiagnosticsHandler));
        handler_list.push(Box::new(handlers::recovery::RecoveryHandler));
        handler_list.push(Box::new(handlers::content::ContentHandler));
        // Remote forwarding — proxy unmatched requests to remote BurdettieOS
        if let Some(ref endpoint) = config.remote.endpoint {
            handler_list.push(Box::new(handlers::forward::ForwardHandler::new(
                endpoint.clone(),
                config.remote.auth_token.clone(),
                config.instance_id.clone(),
            )));
        }
        // Catch-all must be last
        handler_list.push(Box::new(handlers::catchall::CatchallHandler));

        Self { handlers: handler_list, recorder }
    }

    /// Route a request and return a response.
    pub fn route(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        request: &tiny_http::Request,
        original_url: &str,
    ) -> HandlerResponse {
        let start = Instant::now();
        handlers::status::inc_total();

        for handler in &self.handlers {
            if let Some(response) = handler.handle(path, method, body, request) {
                let duration = start.elapsed();

                // Record the request if recording is enabled
                if let Some(ref recorder) = self.recorder {
                    recorder.record_http(&HttpRecord {
                        original_url: original_url.to_string(),
                        rewritten_url: format!("http://localhost{path}"),
                        method: method.to_string(),
                        request_headers: format_request_headers(request),
                        request_body: body.to_vec(),
                        response_status: i32::from(response.status_code),
                        response_headers: format!("{{\"content-type\":\"{}\"}}", response.content_type),
                        response_body: response.body.clone(),
                        guest_binary: crate::guest_binary().to_string(),
                        duration_ms: duration.as_millis() as i64,
                    });
                }

                return response;
            }
        }

        // No handler matched — return 404
        log::warn!("burdettie-services: no handler for {method} {path}");
        HandlerResponse::with_status(
            404,
            "application/json",
            br#"{"error":"not_found","server":"burdettie"}"#.to_vec(),
        )
    }
}

/// Extract request headers as a JSON string.
fn format_request_headers(request: &tiny_http::Request) -> String {
    let mut headers = serde_json::Map::new();
    for h in request.headers() {
        headers.insert(
            h.field.as_str().as_str().to_string(),
            serde_json::Value::String(h.value.as_str().to_string()),
        );
    }
    serde_json::to_string(&headers).unwrap_or_else(|_| "{}".into())
}
