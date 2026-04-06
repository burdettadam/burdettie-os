//! Exhaustive list of known Apple service domains and their BurdettieOS mappings.

/// Category of Apple service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Telemetry,
    SoftwareUpdate,
    Xcode,
    Certificate,
    DeviceActivation,
    Authentication,
    Connectivity,
    ICloud,
    Content,
    Dns,
    Diagnostics,
    PushNotification,
}

/// A single Apple domain → BurdettieOS endpoint mapping.
#[derive(Debug, Clone)]
pub struct DomainEntry {
    /// The Apple domain pattern (lowercase). `*.foo.com` for wildcard.
    pub pattern: &'static str,
    /// The BurdettieOS API path prefix (e.g. `/api/v1/telemetry/xp`).
    pub burdettie_path: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Service category.
    pub category: Category,
}

/// All known Apple service domains relevant to macOS / Xcode CLI tools.
///
/// Sources:
/// - <https://support.apple.com/en-us/101555> (Apple enterprise network requirements)
/// - Observed traffic from Xcode tools under emulation
pub static APPLE_DOMAINS: &[DomainEntry] = &[
    // ── Telemetry / Diagnostics ─────────────────────────────────
    DomainEntry {
        pattern: "xp.apple.com",
        burdettie_path: "/api/v1/telemetry/xp",
        description: "Telemetry and diagnostics reporting",
        category: Category::Telemetry,
    },
    DomainEntry {
        pattern: "xp-cdn.apple.com",
        burdettie_path: "/api/v1/telemetry/xp-cdn",
        description: "Content caching reporting",
        category: Category::Telemetry,
    },
    DomainEntry {
        pattern: "metrics.icloud.com",
        burdettie_path: "/api/v1/telemetry/metrics",
        description: "iCloud device metrics",
        category: Category::Telemetry,
    },
    DomainEntry {
        pattern: "iphonesubmissions.apple.com",
        burdettie_path: "/api/v1/telemetry/submissions",
        description: "Analytics submissions",
        category: Category::Telemetry,
    },
    DomainEntry {
        pattern: "diagassets.apple.com",
        burdettie_path: "/api/v1/diagnostics/assets",
        description: "Hardware diagnostics assets",
        category: Category::Diagnostics,
    },

    // ── Software Updates ────────────────────────────────────────
    DomainEntry {
        pattern: "swscan.apple.com",
        burdettie_path: "/api/v1/swupdate/scan",
        description: "Software update scan",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "swcdn.apple.com",
        burdettie_path: "/api/v1/swupdate/cdn",
        description: "Software update CDN",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "swdist.apple.com",
        burdettie_path: "/api/v1/swupdate/dist",
        description: "Software update distribution",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "swdownload.apple.com",
        burdettie_path: "/api/v1/swupdate/download",
        description: "Software update downloads",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "mesu.apple.com",
        burdettie_path: "/api/v1/swupdate/catalog",
        description: "Software update catalogs",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "gdmf.apple.com",
        burdettie_path: "/api/v1/swupdate/gdmf",
        description: "Managed software update catalog",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "configuration.apple.com",
        burdettie_path: "/api/v1/config",
        description: "Service configuration / Rosetta 2 updates",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "oscdn.apple.com",
        burdettie_path: "/api/v1/recovery/cdn",
        description: "macOS Recovery CDN",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "osrecovery.apple.com",
        burdettie_path: "/api/v1/recovery",
        description: "macOS Recovery",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "appldnld.apple.com",
        burdettie_path: "/api/v1/swupdate/appldnld",
        description: "iOS/watchOS updates and carrier bundles",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "updates.cdn-apple.com",
        burdettie_path: "/api/v1/swupdate/updates-cdn",
        description: "Software update CDN downloads",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "updates-http.cdn-apple.com",
        burdettie_path: "/api/v1/swupdate/updates-http-cdn",
        description: "Software update CDN downloads (HTTP)",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "gg.apple.com",
        burdettie_path: "/api/v1/swupdate/gg",
        description: "OS updates",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "gs.apple.com",
        burdettie_path: "/api/v1/swupdate/gs",
        description: "OS updates",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "ig.apple.com",
        burdettie_path: "/api/v1/swupdate/ig",
        description: "macOS updates",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "skl.apple.com",
        burdettie_path: "/api/v1/swupdate/skl",
        description: "macOS updates",
        category: Category::SoftwareUpdate,
    },

    // ── Xcode Specific ──────────────────────────────────────────
    DomainEntry {
        pattern: "devimages-cdn.apple.com",
        burdettie_path: "/api/v1/xcode/devimages",
        description: "Xcode downloadable components (device images, etc.)",
        category: Category::Xcode,
    },
    DomainEntry {
        pattern: "download.developer.apple.com",
        burdettie_path: "/api/v1/xcode/download",
        description: "Xcode developer downloads",
        category: Category::Xcode,
    },
    DomainEntry {
        pattern: "developer.apple.com",
        burdettie_path: "/api/v1/xcode/developer",
        description: "Apple developer portal",
        category: Category::Xcode,
    },

    // ── Certificate Validation ──────────────────────────────────
    DomainEntry {
        pattern: "ocsp.apple.com",
        burdettie_path: "/api/v1/certs/ocsp",
        description: "OCSP certificate validation",
        category: Category::Certificate,
    },
    DomainEntry {
        pattern: "ocsp2.apple.com",
        burdettie_path: "/api/v1/certs/ocsp",
        description: "OCSP certificate validation (secondary)",
        category: Category::Certificate,
    },
    DomainEntry {
        pattern: "crl.apple.com",
        burdettie_path: "/api/v1/certs/crl",
        description: "Certificate revocation list",
        category: Category::Certificate,
    },
    DomainEntry {
        pattern: "certs.apple.com",
        burdettie_path: "/api/v1/certs/validate",
        description: "Certificate validation",
        category: Category::Certificate,
    },
    DomainEntry {
        pattern: "valid.apple.com",
        burdettie_path: "/api/v1/certs/validate",
        description: "Certificate validation",
        category: Category::Certificate,
    },

    // ── Device Activation / Setup ───────────────────────────────
    DomainEntry {
        pattern: "albert.apple.com",
        burdettie_path: "/api/v1/device/activate",
        description: "Device activation",
        category: Category::DeviceActivation,
    },
    DomainEntry {
        pattern: "static.ips.apple.com",
        burdettie_path: "/api/v1/device/ips",
        description: "Device setup",
        category: Category::DeviceActivation,
    },
    DomainEntry {
        pattern: "tbsc.apple.com",
        burdettie_path: "/api/v1/device/tbsc",
        description: "Device setup",
        category: Category::DeviceActivation,
    },
    DomainEntry {
        pattern: "humb.apple.com",
        burdettie_path: "/api/v1/device/humb",
        description: "Device setup",
        category: Category::DeviceActivation,
    },

    // ── Authentication ──────────────────────────────────────────
    DomainEntry {
        pattern: "gsa.apple.com",
        burdettie_path: "/api/v1/auth/gsa",
        description: "Apple Account authentication (GSA)",
        category: Category::Authentication,
    },
    DomainEntry {
        pattern: "idmsa.apple.com",
        burdettie_path: "/api/v1/auth/idmsa",
        description: "Apple Account authentication (IDMSA)",
        category: Category::Authentication,
    },
    DomainEntry {
        pattern: "account.apple.com",
        burdettie_path: "/api/v1/auth/account",
        description: "Apple Account management",
        category: Category::Authentication,
    },
    DomainEntry {
        pattern: "appleid.cdn-apple.com",
        burdettie_path: "/api/v1/auth/appleid-cdn",
        description: "Apple Account authentication CDN",
        category: Category::Authentication,
    },

    // ── Connectivity ────────────────────────────────────────────
    DomainEntry {
        pattern: "captive.apple.com",
        burdettie_path: "/api/v1/network/captive",
        description: "Captive portal / connectivity check",
        category: Category::Connectivity,
    },
    DomainEntry {
        pattern: "probe.icloud.com",
        burdettie_path: "/api/v1/network/probe",
        description: "iCloud connectivity probe",
        category: Category::Connectivity,
    },
    DomainEntry {
        pattern: "pong.icloud.com",
        burdettie_path: "/api/v1/network/pong",
        description: "iCloud connectivity test",
        category: Category::Connectivity,
    },

    // ── iCloud ──────────────────────────────────────────────────
    DomainEntry {
        pattern: "*.icloud.com",
        burdettie_path: "/api/v1/icloud",
        description: "iCloud services (wildcard)",
        category: Category::ICloud,
    },
    DomainEntry {
        pattern: "*.icloud-content.com",
        burdettie_path: "/api/v1/icloud/content",
        description: "iCloud content delivery",
        category: Category::ICloud,
    },
    DomainEntry {
        pattern: "*.apple-cloudkit.com",
        burdettie_path: "/api/v1/icloud/cloudkit",
        description: "CloudKit services / app notarization",
        category: Category::ICloud,
    },
    DomainEntry {
        pattern: "gateway.icloud.com",
        burdettie_path: "/api/v1/icloud/gateway",
        description: "CloudKit gateway (XProtect, Voice Control)",
        category: Category::ICloud,
    },

    // ── Content / App Store ─────────────────────────────────────
    DomainEntry {
        pattern: "*.itunes.apple.com",
        burdettie_path: "/api/v1/content/itunes",
        description: "iTunes / App Store content",
        category: Category::Content,
    },
    DomainEntry {
        pattern: "*.apps.apple.com",
        burdettie_path: "/api/v1/content/apps",
        description: "App Store content",
        category: Category::Content,
    },
    DomainEntry {
        pattern: "ppq.apple.com",
        burdettie_path: "/api/v1/apps/validate",
        description: "Enterprise app validation",
        category: Category::Content,
    },
    DomainEntry {
        pattern: "*.mzstatic.com",
        burdettie_path: "/api/v1/content/mzstatic",
        description: "App Store static content",
        category: Category::Content,
    },

    // ── DNS ─────────────────────────────────────────────────────
    DomainEntry {
        pattern: "doh.dns.apple.com",
        burdettie_path: "/api/v1/dns/resolve",
        description: "DNS-over-HTTPS",
        category: Category::Dns,
    },

    // ── Push Notifications ──────────────────────────────────────
    DomainEntry {
        pattern: "*.push.apple.com",
        burdettie_path: "/api/v1/push",
        description: "Apple Push Notification service",
        category: Category::PushNotification,
    },

    // ── Content Caching ─────────────────────────────────────────
    DomainEntry {
        pattern: "lcdn-registration.apple.com",
        burdettie_path: "/api/v1/cache/register",
        description: "Content caching registration",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "lcdn-locator.apple.com",
        burdettie_path: "/api/v1/cache/locate",
        description: "Content caching locator",
        category: Category::SoftwareUpdate,
    },
    DomainEntry {
        pattern: "suconfig.apple.com",
        burdettie_path: "/api/v1/cache/config",
        description: "Content caching configuration",
        category: Category::SoftwareUpdate,
    },

    // ── Time ────────────────────────────────────────────────────
    DomainEntry {
        pattern: "time.apple.com",
        burdettie_path: "/api/v1/time",
        description: "NTP time server",
        category: Category::Connectivity,
    },
    DomainEntry {
        pattern: "time-macos.apple.com",
        burdettie_path: "/api/v1/time",
        description: "macOS NTP time server",
        category: Category::Connectivity,
    },
];
