//! Runtime configuration.

use std::path::PathBuf;
use std::sync::Arc;

use platform_core::PlatformProvider;
use trust_policy::TrustPolicy;

/// Controls which shim categories are installed.
#[derive(Debug, Clone)]
pub struct ShimSet {
    /// Install process identity shims (getprogname, _NSGetExecutablePath, etc.)
    pub process: bool,
    /// Install TLS shims (_tlv_bootstrap, _tlv_atexit)
    pub tls: bool,
    /// Install dispatch shims (dispatch_once, dispatch_once_f)
    pub dispatch: bool,
    /// Install pthread stubs (pthread_self, pthread_mutex_*)
    pub pthread: bool,
    /// Install mach primitives (mach_task_self_, mach_absolute_time)
    pub mach: bool,
    /// Install lifecycle shims (atexit, __cxa_atexit)
    pub lifecycle: bool,
    /// Install ObjC runtime shims (objc_msgSend, objc_getClass, etc.)
    pub objc: bool,
    /// Install Foundation/CF-lite shims (NSLog, NSObject class, CFConstantString)
    pub foundation: bool,
}

impl Default for ShimSet {
    fn default() -> Self {
        Self {
            process: true,
            tls: true,
            dispatch: true,
            pthread: true,
            mach: true,
            lifecycle: true,
            objc: true,
            foundation: true,
        }
    }
}

/// Configuration knobs for a runtime execution.
pub struct RuntimeConfig {
    /// Security policy to evaluate before execution.
    /// `None` means skip security evaluation entirely.
    pub security_policy: Option<TrustPolicy>,
    /// Whether to allow dlsym fallback to the host process.
    pub host_fallback: bool,
    /// Which shim categories to install.
    pub shim_set: ShimSet,
    /// Environment variables available to the running binary.
    pub env_overrides: Vec<(String, String)>,
    /// Process name returned by getprogname.
    pub process_name: String,
    /// Process path returned by _NSGetExecutablePath.
    pub process_path: PathBuf,
    /// Runtime event tracer for structured observability.
    /// When `Some`, collects events throughout the pipeline.
    pub tracer: Option<crate::tracer::RuntimeTracer>,
    /// Platform abstraction provider.
    /// Supplies host services (memory mapping, symbol resolution, clocks, etc.).
    /// Defaults to `DarwinHostPlatform` when created via `permissive()` or `secure()`.
    pub platform: Arc<dyn PlatformProvider>,
}

impl RuntimeConfig {
    /// Minimal config for running a binary with host fallback and no security.
    pub fn permissive(binary_path: &std::path::Path) -> Self {
        let name = binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();
        Self {
            security_policy: None,
            host_fallback: true,
            shim_set: ShimSet::default(),
            env_overrides: Vec::new(),
            process_name: name,
            process_path: binary_path.to_path_buf(),
            tracer: None,
            platform: Arc::new(platform_darwin_host::DarwinHostPlatform::new()),
        }
    }

    /// Config with full security evaluation using Apple-default policy.
    pub fn secure(binary_path: &std::path::Path) -> Self {
        let mut cfg = Self::permissive(binary_path);
        cfg.security_policy = Some(TrustPolicy::apple_default());
        cfg.host_fallback = true;
        cfg
    }
}
