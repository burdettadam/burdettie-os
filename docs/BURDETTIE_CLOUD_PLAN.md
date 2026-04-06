# BurdettieOS Cloud Infrastructure Plan

> Replace Apple's cloud services with configurable, self-hosted BurdettieOS equivalents.

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Guest Mach-O binary (Xcode tool / app)                        │
│    NSURLSession → *.apple.com                                  │
│    XPC → com.apple.analyticsd                                  │
│    os_log → unified logging                                    │
└────────┬──────────────┬──────────────────┬──────────────────────┘
         │              │                  │
    ┌────▼────┐   ┌─────▼──────┐    ┌─────▼──────┐
    │ Domain  │   │    XPC     │    │  os_log    │
    │ Rewrite │   │  Daemon    │    │  Pipeline  │
    │ Layer   │   │  Router    │    │            │
    └────┬────┘   └─────┬──────┘    └─────┬──────┘
         │              │                  │
    ┌────▼──────────────▼──────────────────▼──────┐
    │          burdettie-services (local)          │
    │     Rust HTTP server (tiny_http / rouille)   │
    │     /api/v1/log, /api/v1/analytics, ...      │
    │     Request recording + replay               │
    └─────────────────────┬───────────────────────┘
                          │
            ┌─────────────▼──────────────┐
            │   Configurable backend     │
            │   ● Local SQLite (default) │
            │   ● Remote REST endpoint   │
            │   ● S3-compatible store    │
            └────────────────────────────┘
```

## 2. Domain Mapping — Apple → BurdettieOS

The domain rewrite layer intercepts HTTP requests at the `perform_http_request()` 
chokepoint in `nsurlsession.rs` and rewrites Apple domains to BurdettieOS endpoints.

### 2.1 Relevant Apple Domains (macOS / Xcode CLI scope)

| Apple Domain | Purpose | BurdettieOS Endpoint | Priority |
|---|---|---|---|
| **Software Updates** | | | |
| `swscan.apple.com` | macOS update catalog | `/api/v1/swupdate/scan` | P1 |
| `swcdn.apple.com` | macOS update downloads | `/api/v1/swupdate/cdn` | P2 |
| `swdist.apple.com` | macOS update distribution | `/api/v1/swupdate/dist` | P2 |
| `swdownload.apple.com` | macOS update downloads | `/api/v1/swupdate/download` | P2 |
| `mesu.apple.com` | Software update catalogs | `/api/v1/swupdate/catalog` | P1 |
| `gdmf.apple.com` | Software update catalog | `/api/v1/swupdate/gdmf` | P2 |
| `configuration.apple.com` | Rosetta 2 / config | `/api/v1/config` | P1 |
| `oscdn.apple.com` | macOS recovery | `/api/v1/recovery/cdn` | P3 |
| `osrecovery.apple.com` | macOS recovery | `/api/v1/recovery` | P3 |
| `xp.apple.com` | Telemetry/diagnostics | `/api/v1/telemetry/xp` | P1 |
| **Xcode Specific** | | | |
| `devimages-cdn.apple.com` | Xcode components | `/api/v1/xcode/devimages` | P1 |
| `download.developer.apple.com` | Xcode downloads | `/api/v1/xcode/download` | P1 |
| **Certificate Validation** | | | |
| `ocsp.apple.com` | OCSP | `/api/v1/certs/ocsp` | P2 |
| `ocsp2.apple.com` | OCSP | `/api/v1/certs/ocsp` | P2 |
| `crl.apple.com` | CRL | `/api/v1/certs/crl` | P2 |
| `certs.apple.com` | Cert validation | `/api/v1/certs/validate` | P2 |
| `valid.apple.com` | Cert validation | `/api/v1/certs/validate` | P2 |
| **Device/Account** | | | |
| `albert.apple.com` | Device activation | `/api/v1/device/activate` | P1 |
| `gsa.apple.com` | Apple ID auth | `/api/v1/auth/gsa` | P2 |
| `idmsa.apple.com` | Apple ID auth | `/api/v1/auth/idmsa` | P2 |
| `captive.apple.com` | Connectivity check | `/api/v1/network/captive` | P1 |
| **Diagnostics** | | | |
| `diagassets.apple.com` | Hardware diagnostics | `/api/v1/diagnostics/assets` | P3 |
| `metrics.icloud.com` | iCloud metrics | `/api/v1/telemetry/metrics` | P1 |
| `iphonesubmissions.apple.com` | Analytics | `/api/v1/telemetry/submissions` | P2 |
| **Content** | | | |
| `*.itunes.apple.com` | App Store content | `/api/v1/content/itunes` | P3 |
| `ppq.apple.com` | Enterprise app validate | `/api/v1/apps/validate` | P2 |
| `api.apple-cloudkit.com` | Notarization | `/api/v1/notarize` | P2 |
| **DNS** | | | |
| `doh.dns.apple.com` | DNS-over-HTTPS | `/api/v1/dns/resolve` | P2 |

### 2.2 Domain Rewrite Strategy

Requests are rewritten, **not blocked**. The original Apple URL is preserved in a 
`X-Burdettie-Original-URL` header for request recording.

```
https://xp.apple.com/report/2/xp_osx_install
  → http://127.0.0.1:{PORT}/api/v1/telemetry/xp/report/2/xp_osx_install
     (with X-Burdettie-Original-URL: https://xp.apple.com/report/2/xp_osx_install)
```

## 3. New Crates

### 3.1 `burdettie-services` — Local Service Server

A synchronous Rust HTTP server that runs alongside the loader.

```
crates/burdettie-services/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Server lifecycle (start/stop/config)
│   ├── server.rs           # HTTP server (tiny_http)
│   ├── config.rs           # TOML configuration
│   ├── router.rs           # URL → handler dispatch
│   ├── recorder.rs         # Request/response recording to SQLite
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── telemetry.rs    # /api/v1/telemetry/* — accept + store
│   │   ├── swupdate.rs     # /api/v1/swupdate/* — catalog stubs
│   │   ├── xcode.rs        # /api/v1/xcode/*    — component stubs
│   │   ├── certs.rs        # /api/v1/certs/*    — OCSP/CRL responder
│   │   ├── config.rs       # /api/v1/config     — configuration plist
│   │   ├── device.rs       # /api/v1/device/*   — activation
│   │   ├── auth.rs         # /api/v1/auth/*     — Apple ID stubs
│   │   ├── captive.rs      # /api/v1/network/*  — connectivity check
│   │   ├── dns.rs          # /api/v1/dns/*      — DoH resolver
│   │   └── notarize.rs     # /api/v1/notarize   — notarization
│   └── replay.rs           # Replay recorded responses for testing
```

**Key dependencies (all open-source, all synchronous):**
- `tiny_http` — zero-async HTTP server (1 crate, no tokio)
- `rusqlite` (already in workspace) — request recording store
- `serde` + `serde_json` (already in workspace) — request/response serialization
- `toml` (already in workspace) — configuration
- `plist` (already in workspace) — Apple plist responses

### 3.2 `burdettie-domain-map` — Domain Rewrite Library

Thin library that maps Apple hostnames to BurdettieOS endpoints. Used by both 
`nsurlsession.rs` (HTTP rewrite) and `cfnetwork.rs` (DNS rewrite).

```
crates/burdettie-domain-map/
├── Cargo.toml
├── src/
│   ├── lib.rs          # RewriteTable: domain → local endpoint mapping
│   ├── apple_domains.rs # Exhaustive list of known Apple domains + categories
│   └── sandbox.rs      # Network namespace / iptables isolation (Linux)
```

### 3.3 `burdettie-log-pipeline` — Unified Logging

Replace the current `os_log → log::debug!()` passthrough with a structured pipeline.

```
crates/burdettie-log-pipeline/
├── Cargo.toml  
├── src/
│   ├── lib.rs           # Pipeline init, shared sink
│   ├── collector.rs     # Receives os_log events (subsystem, category, level, msg)
│   ├── formatter.rs     # Structured output (JSON lines, logfmt, Apple .tracev3)
│   ├── sink.rs          # Output targets: file, HTTP POST, stdout
│   └── query.rs         # log show / log stream equivalent for querying stored logs
```

## 4. Integration Points — What Changes in Existing Code

### 4.1 NSURLSession Domain Rewrite (nsurlsession.rs)

The single chokepoint `perform_http_request()` gets a domain rewrite check:

```rust
fn perform_http_request(url: &str, method: &str, body: Option<&[u8]>) -> Result<(i32, Vec<u8>), String> {
    // NEW: Rewrite Apple domains to local BurdettieOS services
    let rewritten_url = burdettie_domain_map::rewrite_url(url);
    let agent = http_agent();
    // ... use rewritten_url instead of url ...
}
```

**Sandboxing**: The rewrite only triggers inside the loader process. The host 
machine's DNS and HTTP remain untouched — no `/etc/hosts` modification, no iptables 
rules by default. The rewrite is purely in-process string replacement before the 
`ureq` call.

### 4.2 XPC Daemon Routing (xpc_pipe.rs, xpc_transport.rs)

Replace the loopback/error fallback with routing to BurdettieOS daemon handlers:

```rust
// xpc_pipe.rs — register BurdettieOS handlers alongside launchd stubs
pub fn install_defaults() {
    register_pipe_handler("com.apple.xpc.launchd.domain-port", launchd_stub_handler);
    register_pipe_handler("com.apple.xpc.launchd", launchd_stub_handler);
    // NEW: BurdettieOS daemon handlers
    register_pipe_handler("com.apple.analyticsd", burdettie_analytics_handler);
    register_pipe_handler("com.apple.diagnosticd", burdettie_diagnostics_handler);
    register_pipe_handler("com.apple.symptomsd", burdettie_symptom_handler);
    register_pipe_handler("com.apple.lsd", burdettie_launch_services_handler);
    register_pipe_handler("com.apple.coreservices.quarantine", burdettie_quarantine_handler);
}
```

For the XPC socket transport, BurdettieOS daemons can be registered as Unix domain 
socket listeners at `/tmp/darwin-xpc/com.apple.*.sock`, making them indistinguishable 
from real Apple daemons to the guest binary.

### 4.3 os_log Unified Pipeline (os_log.rs)

Replace `log::debug!()` routing with structured pipeline:

```rust
pub extern "C" fn shim_os_log_impl(
    _dso: u64, log_obj: u64, log_type: u8, fmt: u64, buf: u64, size: u32,
) {
    // NEW: Route to BurdettieOS log pipeline with full metadata
    burdettie_log_pipeline::emit(LogEvent {
        subsystem: get_subsystem(log_obj),
        category: get_category(log_obj),
        level: LogLevel::from_apple(log_type),
        format_string: read_cstring(fmt),
        timestamp: std::time::SystemTime::now(),
    });
}
```

### 4.4 DNS Sandboxing (cfnetwork.rs, system_configuration.rs)

For DNS, domain rewriting happens at the `getaddrinfo` call site:

```rust
// In shim_cfhost_start_info_resolution
let hostname = /* extract from CFHost */;
let resolved_hostname = burdettie_domain_map::rewrite_hostname(&hostname);
// getaddrinfo uses resolved_hostname, not the original
```

This keeps DNS resolution inside the process. The host's `/etc/resolv.conf` is 
never modified.

## 5. Configuration System

### 5.1 Config File: `burdettie.toml`

```toml
[services]
# Local server bind address (default: 127.0.0.1:0 = random port)
bind = "127.0.0.1:0"
# Enable request recording
record_requests = true
# SQLite path for recorded requests
record_db = "/tmp/burdettie-requests.db"

[domain_rewrite]
# Enable/disable domain rewriting (default: true)
enabled = true
# Override the target host for all rewrites
target_host = "127.0.0.1"
# target_port is auto-detected from [services].bind unless overridden
# target_port = 8080

# Custom domain mappings (extends built-in Apple → BurdettieOS map)
[domain_rewrite.custom]
"my-internal-service.corp" = "http://internal.corp:9090"

# Passthrough domains (never rewrite, allow real Apple traffic)
[domain_rewrite.passthrough]
domains = [
    # "developer.apple.com",  # uncomment to allow real Xcode downloads
]

[logging]
# Pipeline output format: "json", "logfmt", "text"
format = "json"
# Output target: "stdout", "file", "http"
sink = "stdout"
# For file sink
# file_path = "/var/log/burdettie/unified.log"
# For HTTP sink (forward to remote logging infrastructure)
# http_endpoint = "https://logs.my-deployment.com/ingest"

[xpc_daemons]
# Enable BurdettieOS daemon handlers
analytics = true
diagnostics = true
launch_services = true
```

### 5.2 Environment Variables

| Variable | Purpose | Default |
|---|---|---|
| `BURDETTIE_CONFIG` | Path to `burdettie.toml` | `./burdettie.toml` |
| `BURDETTIE_SERVICES_BIND` | Override server bind address | `127.0.0.1:0` |
| `BURDETTIE_DOMAIN_REWRITE` | `true`/`false` — enable domain rewrite | `true` |
| `BURDETTIE_LOG_FORMAT` | `json`/`logfmt`/`text` | `text` |
| `BURDETTIE_LOG_SINK` | `stdout`/`file:/path`/`http://url` | `stdout` |
| `BURDETTIE_RECORD_REQUESTS` | `true`/`false` — record all requests | `true` |
| `BURDETTIE_PASSTHROUGH` | Comma-separated domains to pass through | (empty) |

## 6. Request Recording & Protocol Discovery

Every intercepted request (HTTP, XPC, DNS) is recorded to SQLite for analysis:

```sql
CREATE TABLE requests (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    source TEXT NOT NULL,          -- 'http', 'xpc', 'dns', 'os_log'
    -- HTTP fields
    original_url TEXT,
    rewritten_url TEXT,
    method TEXT,
    request_headers TEXT,          -- JSON
    request_body BLOB,
    response_status INTEGER,
    response_headers TEXT,         -- JSON
    response_body BLOB,
    -- XPC fields
    xpc_service TEXT,
    xpc_request TEXT,              -- JSON serialized
    xpc_reply TEXT,                -- JSON serialized
    -- DNS fields
    dns_query TEXT,
    dns_resolved TEXT,             -- JSON array of addresses
    -- Metadata
    guest_binary TEXT,             -- which Mach-O tool made this request
    duration_ms INTEGER
);
```

This is the foundation for understanding Apple's protocols — record what tools 
actually ask for, then build handlers that produce compatible responses.

## 7. Sandboxing Strategy

### 7.1 In-Process Isolation (Default — Works Everywhere)

Domain rewriting happens entirely inside the loader process:
- `perform_http_request()` rewrites URLs before `ureq` sends them
- `getaddrinfo()` calls rewrite hostnames before the syscall
- Host machine DNS, HTTP, and network configuration are **never modified**
- Multiple loader instances can run simultaneously with different configs

### 7.2 Network Namespace Isolation (Linux — Optional)

For stronger isolation, the Docker container can use a dedicated network namespace:

```dockerfile
# In Dockerfile — no ports exposed to host by default
# All Apple domains resolve to 127.0.0.1 inside the container
RUN echo "127.0.0.1 xp.apple.com swscan.apple.com mesu.apple.com" >> /etc/hosts
```

Or with `unshare(CLONE_NEWNET)` at loader startup to create a fully isolated 
network namespace where only localhost is available.

### 7.3 Configurable Passthrough

Domains listed in `[domain_rewrite.passthrough]` skip rewriting entirely, 
allowing real Apple traffic when needed (e.g., downloading real Xcode components 
during development).

## 8. Implementation Phases

### Phase 1: Foundation (MVP)
1. Create `burdettie-domain-map` crate with Apple domain table + URL rewriter
2. Create `burdettie-services` crate with `tiny_http` server skeleton
3. Wire `perform_http_request()` to domain rewriter
4. Implement request recording to SQLite
5. Add `/api/v1/telemetry/xp` handler (accept + store — primary phone-home)
6. Add `/api/v1/network/captive` handler (return `<HTML><BODY>Success</BODY></HTML>`)
7. Add `/api/v1/device/activate` handler (return success plist)
8. Add `BURDETTIE_*` env vars for config
9. Tests: verify Apple domains rewrite, verify requests hit local server

### Phase 2: XPC Daemons + Logging
1. Register BurdettieOS pipe handlers for `com.apple.analyticsd`, `diagnosticd`, etc.
2. Implement structured os_log pipeline with JSON output
3. Add file and HTTP log sinks
4. XPC request recording to same SQLite store
5. Add `/api/v1/swupdate/scan` with minimal valid catalog plist
6. Add `/api/v1/xcode/devimages` for Xcode component metadata

### Phase 3: Protocol Fidelity
1. Analyze recorded requests to understand Apple's actual protocol format
2. Build response generators that produce Apple-compatible plists/JSON
3. OCSP responder using local cert store
4. Software update catalog serving
5. Notarization ticket validation

### Phase 4: Remote Deployment
1. Configuration for remote `burdettie-services` endpoint
2. Authentication for remote service access
3. S3-compatible storage backend for recorded data
4. Multi-tenant support (multiple loader instances → one remote server)
5. Dashboard / admin API for viewing recorded requests

## 9. Open Source Dependencies

| Need | Crate | License | Notes |
|---|---|---|---|
| HTTP server | `tiny_http` | MIT/Apache-2.0 | Zero-async, ~2K lines |
| SQLite | `rusqlite` (existing) | MIT | Already in workspace |
| Config parsing | `toml` (existing) | MIT/Apache-2.0 | Already in workspace |
| Plist generation | `plist` (existing) | MIT | Already in workspace |
| JSON | `serde_json` (existing) | MIT/Apache-2.0 | Already in workspace |
| TLS certs | `x509-parser` (existing) | MIT/Apache-2.0 | Already in workspace |
| OCSP | `x509-ocsp` | MIT/Apache-2.0 | OCSP request/response parsing |
| DNS | `hickory-resolver` (existing) | MIT/Apache-2.0 | Already in workspace |
| Time | `chrono` or `time` | MIT/Apache-2.0 | Log timestamps |

Almost everything needed is already in the workspace. Only `tiny_http` and 
optionally `x509-ocsp` are new.
