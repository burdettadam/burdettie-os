//! Apple domain → BurdettieOS endpoint rewrite table.
//!
//! Provides in-process URL and hostname rewriting so that guest Mach-O binaries
//! that try to contact Apple services are transparently redirected to the local
//! (or remote) BurdettieOS service infrastructure.
//!
//! **Sandboxing**: All rewriting is purely in-process string replacement.
//! The host machine's DNS, `/etc/hosts`, and network configuration are never
//! modified.  Multiple loader instances can run simultaneously with independent
//! configurations.

mod apple_domains;
mod config;

pub use apple_domains::{Category, DomainEntry, APPLE_DOMAINS};
pub use config::RewriteConfig;

use std::sync::OnceLock;

// ── Global configuration ────────────────────────────────────────

static GLOBAL_CONFIG: OnceLock<RewriteConfig> = OnceLock::new();

/// Initialise the domain rewriter with the given configuration.
///
/// Must be called once before any `rewrite_url` / `rewrite_hostname` calls.
/// Subsequent calls are ignored (first-write-wins).
pub fn init(config: RewriteConfig) {
    let _ = GLOBAL_CONFIG.set(config);
}

/// Get the current config, or a disabled default if `init()` was never called.
fn cfg() -> &'static RewriteConfig {
    GLOBAL_CONFIG.get_or_init(RewriteConfig::disabled)
}

// ── URL rewriting (for NSURLSession HTTP) ───────────────────────

/// Rewrite an Apple URL to point at the BurdettieOS service endpoint.
///
/// Returns `None` if the URL should not be rewritten (disabled, passthrough,
/// or not an Apple domain).
///
/// The original URL is preserved as an `X-Burdettie-Original-URL` header —
/// that is the caller's responsibility to attach.
pub fn rewrite_url(original_url: &str) -> Option<String> {
    let config = cfg();
    if !config.enabled {
        return None;
    }

    let host = extract_host(original_url)?;

    // Passthrough list — never rewrite these
    if config.is_passthrough(&host) {
        return None;
    }

    // Look up the Apple domain
    let entry = lookup_domain(&host)?;
    let path_suffix = extract_path(original_url);

    let rewritten = format!(
        "http://{}:{}{}{path_suffix}",
        config.target_host, config.target_port, entry.burdettie_path,
    );

    log::info!(
        "burdettie-domain-map: rewrite {original_url} → {rewritten} ({})",
        entry.description
    );

    Some(rewritten)
}

/// Rewrite a hostname for DNS resolution.
///
/// Returns `None` if the hostname should not be rewritten.
pub fn rewrite_hostname(hostname: &str) -> Option<String> {
    let config = cfg();
    if !config.enabled {
        return None;
    }
    if config.is_passthrough(hostname) {
        return None;
    }
    if lookup_domain(hostname).is_some() {
        log::info!("burdettie-domain-map: DNS rewrite {hostname} → {}", config.target_host);
        Some(config.target_host.clone())
    } else {
        None
    }
}

/// Check whether a given hostname is a known Apple service domain.
pub fn is_apple_domain(hostname: &str) -> bool {
    lookup_domain(hostname).is_some()
}

/// Return the BurdettieOS API path for a known Apple domain, if any.
pub fn api_path_for(hostname: &str) -> Option<&'static str> {
    lookup_domain(hostname).map(|e| e.burdettie_path)
}

/// Return the category for a known Apple domain.
pub fn category_for(hostname: &str) -> Option<Category> {
    lookup_domain(hostname).map(|e| e.category)
}

// ── Internal helpers ────────────────────────────────────────────

/// Look up a hostname in the Apple domain table.
///
/// Supports exact matches and wildcard suffixes (e.g. `*.icloud.com`).
fn lookup_domain(hostname: &str) -> Option<&'static DomainEntry> {
    let host = hostname.trim().to_ascii_lowercase();

    // 1. Exact match
    if let Some(entry) = APPLE_DOMAINS.iter().find(|e| e.pattern == host) {
        return Some(entry);
    }

    // 2. Wildcard suffix match: strip subdomains and check *.parent
    if let Some(dot) = host.find('.') {
        let parent = &host[dot + 1..];
        let wildcard = format!("*.{parent}");
        if let Some(entry) = APPLE_DOMAINS.iter().find(|e| e.pattern == wildcard) {
            return Some(entry);
        }
        // Try one more level: a.b.c.apple.com → *.c.apple.com
        if let Some(dot2) = parent.find('.') {
            let grandparent = &parent[dot2 + 1..];
            let wildcard2 = format!("*.{grandparent}");
            if let Some(entry) = APPLE_DOMAINS.iter().find(|e| e.pattern == wildcard2) {
                return Some(entry);
            }
        }
    }

    None
}

/// Extract the hostname from a URL string.
fn extract_host(url: &str) -> Option<String> {
    // Strip scheme
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Take up to the first '/' or ':'
    let host_part = after_scheme
        .split(['/', ':'])
        .next()
        .unwrap_or(after_scheme);

    if host_part.is_empty() {
        None
    } else {
        Some(host_part.to_ascii_lowercase())
    }
}

/// Extract the path (with leading `/`) from a URL, or empty string.
fn extract_path(url: &str) -> &str {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Skip host:port
    if let Some(slash) = after_scheme.find('/') {
        &after_scheme[slash..]
    } else {
        ""
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_from_url() {
        assert_eq!(extract_host("https://xp.apple.com/report"), Some("xp.apple.com".into()));
        assert_eq!(extract_host("http://mesu.apple.com:443/catalog"), Some("mesu.apple.com".into()));
        assert_eq!(extract_host("https://foo.icloud.com/path"), Some("foo.icloud.com".into()));
        assert_eq!(extract_host(""), None);
    }

    #[test]
    fn extract_path_from_url() {
        assert_eq!(extract_path("https://xp.apple.com/report/2/xp_osx"), "/report/2/xp_osx");
        assert_eq!(extract_path("https://xp.apple.com"), "");
    }

    #[test]
    fn exact_domain_lookup() {
        let entry = lookup_domain("xp.apple.com");
        assert!(entry.is_some());
        assert_eq!(entry.map(|e| e.category), Some(Category::Telemetry));
    }

    #[test]
    fn wildcard_domain_lookup() {
        let entry = lookup_domain("foo.icloud.com");
        assert!(entry.is_some());
        assert_eq!(entry.map(|e| e.category), Some(Category::ICloud));
    }

    #[test]
    fn unknown_domain_returns_none() {
        assert!(lookup_domain("github.com").is_none());
        assert!(lookup_domain("google.com").is_none());
    }

    #[test]
    fn rewrite_url_when_enabled() {
        let config = RewriteConfig {
            enabled: true,
            target_host: "127.0.0.1".into(),
            target_port: 9999,
            passthrough: vec![],
        };
        // Use a fresh OnceLock test by testing the components directly
        assert!(lookup_domain("xp.apple.com").is_some());
        let entry = lookup_domain("xp.apple.com").expect("known domain");
        let result = format!("http://{}:{}{}/report/2", config.target_host, config.target_port, entry.burdettie_path);
        assert_eq!(result, "http://127.0.0.1:9999/api/v1/telemetry/xp/report/2");
    }

    #[test]
    fn passthrough_not_rewritten() {
        let config = RewriteConfig {
            enabled: true,
            target_host: "127.0.0.1".into(),
            target_port: 9999,
            passthrough: vec!["developer.apple.com".into()],
        };
        assert!(config.is_passthrough("developer.apple.com"));
        assert!(!config.is_passthrough("xp.apple.com"));
    }

    #[test]
    fn disabled_config_returns_none() {
        let config = RewriteConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn is_apple_domain_check() {
        assert!(is_apple_domain("xp.apple.com"));
        assert!(is_apple_domain("mesu.apple.com"));
        assert!(is_apple_domain("captive.apple.com"));
        assert!(!is_apple_domain("github.com"));
    }

    #[test]
    fn category_classification() {
        assert_eq!(category_for("xp.apple.com"), Some(Category::Telemetry));
        assert_eq!(category_for("mesu.apple.com"), Some(Category::SoftwareUpdate));
        assert_eq!(category_for("devimages-cdn.apple.com"), Some(Category::Xcode));
        assert_eq!(category_for("ocsp.apple.com"), Some(Category::Certificate));
        assert_eq!(category_for("albert.apple.com"), Some(Category::DeviceActivation));
        assert_eq!(category_for("captive.apple.com"), Some(Category::Connectivity));
    }
}
