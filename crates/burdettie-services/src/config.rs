//! Service configuration — read from TOML file, env vars, or constructed programmatically.
//!
//! Loading order (later overrides earlier):
//! 1. Defaults
//! 2. TOML file (`BURDETTIE_CONFIG` env var, default `./burdettie.toml`)
//! 3. Environment variables

/// Configuration for the BurdettieOS service server.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Bind address for the HTTP server (e.g. `127.0.0.1:0` for random port).
    pub bind_addr: String,
    /// Whether to record all requests to SQLite.
    pub record_requests: bool,
    /// Path to the SQLite database for request recording.
    pub record_db_path: String,
    /// Path to a SQLite database for request replay (optional).
    pub replay_db_path: Option<String>,
    /// Log format for the unified logging pipeline.
    pub log_format: LogFormat,
    /// Log output sink.
    pub log_sink: LogSink,
    /// Admin API configuration.
    pub admin: AdminConfig,
    /// Remote forwarding configuration.
    pub remote: RemoteConfig,
    /// Instance identifier for multi-tenant tagging.
    pub instance_id: String,
    /// Workspace root used by repo-local admin subsystems.
    pub workspace_root: String,
}

/// Admin API settings.
#[derive(Debug, Clone)]
pub struct AdminConfig {
    /// Whether the admin API is enabled.
    pub enabled: bool,
    /// Bearer token required for admin endpoints (None = no auth).
    pub token: Option<String>,
}

/// Remote forwarding settings.
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    /// Remote BurdettieOS endpoint (e.g. "https://burdettie.example.com").
    pub endpoint: Option<String>,
    /// Auth token for the remote server.
    pub auth_token: Option<String>,
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON Lines (one JSON object per line).
    Json,
    /// Key=value logfmt.
    Logfmt,
    /// Human-readable text.
    Text,
}

/// Log output sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogSink {
    /// Write to stdout.
    Stdout,
    /// Write to a file.
    File(String),
    /// POST to an HTTP endpoint.
    Http(String),
}

impl ServiceConfig {
    /// Build configuration from TOML file + environment variables.
    ///
    /// TOML values are used as defaults; env vars always override.
    /// Loading order: 1) Defaults, 2) TOML file, 3) Env vars.
    pub fn from_env() -> Self {
        let toml = load_toml_config().unwrap_or_default();

        let bind_addr = std::env::var("BURDETTIE_SERVICES_BIND")
            .ok()
            .or(toml.services.bind)
            .unwrap_or_else(|| "127.0.0.1:0".into());

        let record_requests = std::env::var("BURDETTIE_RECORD_REQUESTS")
            .ok()
            .map(|v| !matches!(v.as_str(), "0" | "false" | "no" | "off"))
            .or(toml.services.record_requests)
            .unwrap_or(true);

        let record_db_path = std::env::var("BURDETTIE_RECORD_DB")
            .ok()
            .or(toml.services.record_db)
            .unwrap_or_else(|| "/tmp/burdettie-requests.db".into());

        let replay_db_path = std::env::var("BURDETTIE_REPLAY_DB")
            .ok()
            .or(toml.services.replay_db);

        let log_format = std::env::var("BURDETTIE_LOG_FORMAT")
            .ok()
            .or(toml.logging.format)
            .map(|s| match s.to_ascii_lowercase().as_str() {
                "json" => LogFormat::Json,
                "logfmt" => LogFormat::Logfmt,
                _ => LogFormat::Text,
            })
            .unwrap_or(LogFormat::Text);

        let log_sink = std::env::var("BURDETTIE_LOG_SINK")
            .ok()
            .or_else(|| {
                if let Some(ref path) = toml.logging.file_path {
                    Some(format!("file:{path}"))
                } else if let Some(ref url) = toml.logging.http_endpoint {
                    Some(url.clone())
                } else {
                    toml.logging.sink
                }
            })
            .map(|s| {
                if s.starts_with("file:") {
                    LogSink::File(s[5..].to_string())
                } else if s.starts_with("http://") || s.starts_with("https://") {
                    LogSink::Http(s.to_string())
                } else {
                    LogSink::Stdout
                }
            })
            .unwrap_or(LogSink::Stdout);

        let admin_enabled = std::env::var("BURDETTIE_ADMIN_ENABLED")
            .ok()
            .map(|v| !matches!(v.as_str(), "0" | "false" | "no" | "off"))
            .or(toml.admin.enabled)
            .unwrap_or(true);

        let admin_token = std::env::var("BURDETTIE_ADMIN_TOKEN")
            .ok()
            .or(toml.admin.token);

        let remote_endpoint = std::env::var("BURDETTIE_REMOTE_ENDPOINT")
            .ok()
            .or(toml.remote.endpoint);

        let remote_auth_token = std::env::var("BURDETTIE_REMOTE_TOKEN")
            .ok()
            .or(toml.remote.auth_token);

        let instance_id = std::env::var("BURDETTIE_INSTANCE_ID")
            .ok()
            .or(toml.remote.instance_id)
            .unwrap_or_else(|| "default".into());

        let workspace_root = std::env::var("BURDETTIE_WORKSPACE_ROOT")
            .ok()
            .or(toml.services.workspace_root)
            .unwrap_or_else(|| ".".into());

        Self {
            bind_addr,
            record_requests,
            record_db_path,
            replay_db_path,
            log_format,
            log_sink,
            admin: AdminConfig {
                enabled: admin_enabled,
                token: admin_token,
            },
            remote: RemoteConfig {
                endpoint: remote_endpoint,
                auth_token: remote_auth_token,
            },
            instance_id,
            workspace_root,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// ── TOML configuration file support ─────────────────────────────

/// TOML file structure matching the plan's `burdettie.toml` format.
#[derive(Debug, serde::Deserialize, Default)]
struct TomlConfig {
    #[serde(default)]
    services: TomlServices,
    #[serde(default)]
    logging: TomlLogging,
    #[serde(default)]
    domain_rewrite: TomlDomainRewrite,
    #[serde(default)]
    admin: TomlAdmin,
    #[serde(default)]
    remote: TomlRemote,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlServices {
    bind: Option<String>,
    record_requests: Option<bool>,
    record_db: Option<String>,
    replay_db: Option<String>,
    workspace_root: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlLogging {
    format: Option<String>,
    sink: Option<String>,
    file_path: Option<String>,
    http_endpoint: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlDomainRewrite {
    enabled: Option<bool>,
    target_host: Option<String>,
    target_port: Option<u16>,
    #[serde(default)]
    passthrough: TomlPassthrough,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlPassthrough {
    #[serde(default)]
    domains: Vec<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlAdmin {
    enabled: Option<bool>,
    token: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
struct TomlRemote {
    endpoint: Option<String>,
    auth_token: Option<String>,
    instance_id: Option<String>,
}

/// Try to load a TOML configuration file.
/// Checks `BURDETTIE_CONFIG` env var, then `./burdettie.toml`.
fn load_toml_config() -> Option<TomlConfig> {
    let config_path = std::env::var("BURDETTIE_CONFIG").unwrap_or_else(|_| "burdettie.toml".into());

    let contents = std::fs::read_to_string(&config_path).ok()?;
    match toml::from_str::<TomlConfig>(&contents) {
        Ok(cfg) => {
            log::info!("burdettie-services: loaded config from {config_path}");
            Some(cfg)
        }
        Err(e) => {
            log::warn!("burdettie-services: failed to parse {config_path}: {e}");
            None
        }
    }
}

impl ServiceConfig {
    /// Return domain rewrite settings from TOML (for burdettie-domain-map).
    pub fn domain_rewrite_from_toml() -> (Option<bool>, Option<String>, Option<u16>, Vec<String>) {
        let toml = load_toml_config().unwrap_or_default();
        (
            toml.domain_rewrite.enabled,
            toml.domain_rewrite.target_host,
            toml.domain_rewrite.target_port,
            toml.domain_rewrite.passthrough.domains,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_defaults() {
        // No TOML file, no env vars → should use defaults without panicking
        let cfg = ServiceConfig::from_env();
        assert!(!cfg.bind_addr.is_empty());
        assert!(cfg.record_requests);
        assert_eq!(cfg.log_format, LogFormat::Text);
        assert_eq!(cfg.log_sink, LogSink::Stdout);
    }

    #[test]
    fn toml_parsing() {
        let toml_str = r#"
[services]
bind = "127.0.0.1:9999"
record_requests = false
record_db = "/tmp/test.db"

[logging]
format = "json"
sink = "stdout"

[domain_rewrite]
enabled = true
target_host = "10.0.0.1"
target_port = 8080

[domain_rewrite.passthrough]
domains = ["developer.apple.com"]
"#;
        let cfg: TomlConfig = toml::from_str(toml_str).expect("valid TOML");
        assert_eq!(cfg.services.bind.as_deref(), Some("127.0.0.1:9999"));
        assert_eq!(cfg.services.record_requests, Some(false));
        assert_eq!(cfg.domain_rewrite.enabled, Some(true));
        assert_eq!(cfg.domain_rewrite.target_port, Some(8080));
        assert_eq!(
            cfg.domain_rewrite.passthrough.domains,
            vec!["developer.apple.com"]
        );
    }

    #[test]
    fn empty_toml_parses() {
        let cfg: TomlConfig = toml::from_str("").expect("empty TOML");
        assert!(cfg.services.bind.is_none());
        assert!(cfg.logging.format.is_none());
        assert!(cfg.domain_rewrite.passthrough.domains.is_empty());
    }
}
