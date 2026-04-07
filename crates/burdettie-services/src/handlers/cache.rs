//! Content caching handler.
//!
//! macOS content caching (`lcdn-registration`, `lcdn-locator`, `suconfig`)
//! allows enterprise networks to cache Apple software updates locally.
//! In BurdettieOS we return "no caching peers available" responses.
//!
//! Endpoints:
//! - `/api/v1/cache/register` — lcdn-registration.apple.com
//! - `/api/v1/cache/locate`   — lcdn-locator.apple.com
//! - `/api/v1/cache/config`   — suconfig.apple.com

use super::{Handler, HandlerResponse};
use tiny_http::Request;

pub struct CacheHandler;

impl Handler for CacheHandler {
    fn handle(&self, path: &str, _method: &str, _body: &[u8], _request: &Request)
        -> Option<HandlerResponse>
    {
        if !path.starts_with("/api/v1/cache/") {
            return None;
        }

        log::debug!("burdettie-services: cache {path}");

        if path.starts_with("/api/v1/cache/register") {
            // Registration endpoint — acknowledge but don't register
            return Some(HandlerResponse::json(
                br#"{"status":"ok","registered":false,"message":"Content caching not available in BurdettieOS","server":"burdettie"}"#.to_vec(),
            ));
        }

        if path.starts_with("/api/v1/cache/locate") {
            // Locator endpoint — no caching peers
            return Some(HandlerResponse::json(
                br#"{"servers":[],"server":"burdettie"}"#.to_vec(),
            ));
        }

        // suconfig — empty caching configuration
        Some(HandlerResponse::plist_xml(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>ContentCachingEnabled</key>
    <false/>
</dict>
</plist>"#.to_vec(),
        ))
    }
}
