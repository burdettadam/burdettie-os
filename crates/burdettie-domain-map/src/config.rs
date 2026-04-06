//! Configuration for the domain rewrite layer.

/// Configuration for domain rewriting.
#[derive(Debug, Clone)]
pub struct RewriteConfig {
    /// Whether domain rewriting is enabled.
    pub enabled: bool,
    /// Target host for rewritten URLs (default: `127.0.0.1`).
    pub target_host: String,
    /// Target port for rewritten URLs (auto-detected from server bind, or 0).
    pub target_port: u16,
    /// Domains that should never be rewritten (passthrough to real Apple).
    pub passthrough: Vec<String>,
}

impl RewriteConfig {
    /// Create a config from environment variables.
    ///
    /// | Variable | Default |
    /// |---|---|
    /// | `BURDETTIE_DOMAIN_REWRITE` | `true` |
    /// | `BURDETTIE_SERVICES_HOST` | `127.0.0.1` |
    /// | `BURDETTIE_SERVICES_PORT` | `0` (auto) |
    /// | `BURDETTIE_PASSTHROUGH` | (empty, comma-separated) |
    pub fn from_env() -> Self {
        let enabled = std::env::var("BURDETTIE_DOMAIN_REWRITE")
            .map(|v| !matches!(v.as_str(), "0" | "false" | "no" | "off"))
            .unwrap_or(true);

        let target_host = std::env::var("BURDETTIE_SERVICES_HOST")
            .unwrap_or_else(|_| "127.0.0.1".into());

        let target_port = std::env::var("BURDETTIE_SERVICES_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0u16);

        let passthrough = std::env::var("BURDETTIE_PASSTHROUGH")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self { enabled, target_host, target_port, passthrough }
    }

    /// Disabled config — no rewriting.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            target_host: "127.0.0.1".into(),
            target_port: 0,
            passthrough: vec![],
        }
    }

    /// Check if a hostname is in the passthrough list.
    pub fn is_passthrough(&self, hostname: &str) -> bool {
        let host_lower = hostname.to_ascii_lowercase();
        self.passthrough.iter().any(|p| host_lower == *p)
    }
}

impl Default for RewriteConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
