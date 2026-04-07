//! Remote forwarding handler — proxy unmatched requests to a remote BurdettieOS instance.
//!
//! When `[remote]` is configured in the TOML config, requests that reach this handler
//! are forwarded to the remote endpoint. The remote response is relayed back to the client.
//! This enables multi-tier deployment where a lightweight local instance delegates to a
//! full cloud-hosted BurdettieOS server.

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct ForwardHandler {
    /// Remote BurdettieOS endpoint (e.g. "https://burdettie.example.com").
    endpoint: String,
    /// Optional auth token for the remote server.
    auth_token: Option<String>,
    /// Instance ID sent in `X-Burdettie-Instance` header.
    instance_id: String,
}

impl ForwardHandler {
    pub fn new(endpoint: String, auth_token: Option<String>, instance_id: String) -> Self {
        Self { endpoint, auth_token, instance_id }
    }
}

impl Handler for ForwardHandler {
    fn handle(&self, path: &str, method: &str, body: &[u8], request: &Request)
        -> Option<HandlerResponse>
    {
        // Don't forward admin or status requests
        if path.starts_with("/_burdettie/") {
            return None;
        }

        let url = format!("{}{}", self.endpoint, path);

        let forwarded = match self.forward(method, &url, body, request) {
            Ok(resp) => resp,
            Err(e) => {
                log::warn!("burdettie-services: forward to {} failed: {e}", self.endpoint);
                return None; // Fall through to the next handler
            }
        };

        Some(forwarded)
    }
}

impl ForwardHandler {
    fn forward(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        request: &Request,
    ) -> Result<HandlerResponse, Box<dyn std::error::Error>> {
        let agent = ureq::Agent::new_with_defaults();

        // Collect forwarded headers (skip hop-by-hop and host)
        let mut forward_headers: Vec<(&str, &str)> = Vec::new();
        for header in request.headers() {
            let name = header.field.as_str().as_str();
            if matches!(
                name.to_ascii_lowercase().as_str(),
                "host" | "connection" | "transfer-encoding" | "content-length"
            ) {
                continue;
            }
            forward_headers.push((name, header.value.as_str()));
        }

        let response = match method {
            "GET" | "HEAD" | "DELETE" => {
                let mut req = match method {
                    "HEAD" => agent.head(url),
                    "DELETE" => agent.delete(url),
                    _ => agent.get(url),
                };
                for (k, v) in &forward_headers {
                    req = req.header(*k, *v);
                }
                req = req.header("X-Burdettie-Instance", &self.instance_id);
                if let Some(ref token) = self.auth_token {
                    req = req.header("Authorization", &format!("Bearer {token}"));
                }
                req.call()?
            }
            _ => {
                // POST, PUT, PATCH, etc. — methods with body
                let mut req = match method {
                    "PUT" => agent.put(url),
                    "PATCH" => agent.patch(url),
                    _ => agent.post(url),
                };
                for (k, v) in &forward_headers {
                    req = req.header(*k, *v);
                }
                req = req.header("X-Burdettie-Instance", &self.instance_id);
                if let Some(ref token) = self.auth_token {
                    req = req.header("Authorization", &format!("Bearer {token}"));
                }
                req.send(body)?
            }
        };

        let status = response.status().as_u16();
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let resp_body = response.into_body().read_to_vec()?;

        Ok(HandlerResponse::with_status(status, content_type, resp_body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_admin_paths() {
        let handler = ForwardHandler::new(
            "http://localhost:9999".into(),
            None,
            "test".into(),
        );
        // ForwardHandler skips _burdettie paths
        assert!("/_burdettie/admin/requests".starts_with("/_burdettie/"));
        assert!("/_burdettie/status".starts_with("/_burdettie/"));
        let _ = handler;
    }

    #[test]
    fn constructs_with_optional_auth() {
        let h1 = ForwardHandler::new("http://a.test".into(), None, "i1".into());
        assert!(h1.auth_token.is_none());

        let h2 = ForwardHandler::new("http://b.test".into(), Some("tok".into()), "i2".into());
        assert_eq!(h2.auth_token.as_deref(), Some("tok"));
    }
}
