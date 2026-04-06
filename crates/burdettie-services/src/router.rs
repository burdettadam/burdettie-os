//! Request router — dispatches incoming HTTP requests to the appropriate handler.

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
    pub fn new(recorder: Option<Arc<RequestRecorder>>) -> Self {
        let handlers: Vec<Box<dyn Handler>> = vec![
            Box::new(handlers::telemetry::TelemetryHandler),
            Box::new(handlers::captive::CaptiveHandler),
            Box::new(handlers::device::DeviceHandler),
            Box::new(handlers::swupdate::SwupdateHandler),
            Box::new(handlers::certs::CertsHandler),
            Box::new(handlers::xcode::XcodeHandler),
            Box::new(handlers::auth::AuthHandler),
            Box::new(handlers::config::ConfigHandler),
            Box::new(handlers::dns::DnsHandler),
            Box::new(handlers::notarize::NotarizeHandler),
            // Catch-all must be last
            Box::new(handlers::catchall::CatchallHandler),
        ];

        Self { handlers, recorder }
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
                        guest_binary: String::new(), // filled by caller if known
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
