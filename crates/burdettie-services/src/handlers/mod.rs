//! Request handlers for BurdettieOS service endpoints.

pub mod telemetry;
pub mod captive;
pub mod device;
pub mod swupdate;
pub mod certs;
pub mod xcode;
pub mod auth;
pub mod config;
pub mod dns;
pub mod notarize;
pub mod catchall;

use tiny_http::{Request, Response};

/// Trait for service endpoint handlers.
pub trait Handler: Send + Sync {
    /// Handle a request and return a response.
    ///
    /// Returns `None` if this handler does not handle the given path.
    fn handle(&self, path: &str, method: &str, body: &[u8], request: &Request)
        -> Option<HandlerResponse>;
}

/// A handler response ready to send back.
pub struct HandlerResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl HandlerResponse {
    /// Create a 200 JSON response.
    pub fn json(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: 200,
            content_type: "application/json".into(),
            body: body.into(),
        }
    }

    /// Create a 200 HTML response.
    pub fn html(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: 200,
            content_type: "text/html; charset=utf-8".into(),
            body: body.into(),
        }
    }

    /// Create a 200 XML/plist response.
    pub fn plist_xml(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: 200,
            content_type: "application/xml".into(),
            body: body.into(),
        }
    }

    /// Create a 200 response with arbitrary content type.
    pub fn ok(content_type: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: 200,
            content_type: content_type.into(),
            body: body.into(),
        }
    }

    /// Create a response with a custom status code.
    pub fn with_status(status: u16, content_type: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status_code: status,
            content_type: content_type.into(),
            body: body.into(),
        }
    }

    /// Convert to a tiny_http Response.
    pub fn into_tiny_response(self) -> Response<std::io::Cursor<Vec<u8>>> {
        let len = self.body.len();
        Response::from_data(self.body)
            .with_status_code(self.status_code)
            .with_header(
                tiny_http::Header::from_bytes(b"Content-Type", self.content_type.as_bytes())
                    .expect("valid header"),
            )
            .with_header(
                tiny_http::Header::from_bytes(b"Content-Length", format!("{len}").as_bytes())
                    .expect("valid header"),
            )
            .with_header(
                tiny_http::Header::from_bytes(b"Server", b"BurdettieOS/1.0")
                    .expect("valid header"),
            )
    }
}
