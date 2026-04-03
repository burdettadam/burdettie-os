//! Runtime configuration.

use std::path::PathBuf;
use std::sync::Arc;

use platform_core::PlatformProvider;
use trust_policy::TrustPolicy;

/// Return the default platform provider for the current OS.
#[cfg(target_os = "macos")]
fn default_platform() -> impl PlatformProvider {
    platform_darwin_host::DarwinHostPlatform::new()
}

/// Return the default platform provider for the current OS.
#[cfg(target_os = "linux")]
fn default_platform() -> impl PlatformProvider {
    platform_linux::LinuxHostPlatform::new()
}

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
    /// Install pointer authentication shims (ptrauth_strip, ptrauth_sign_unauthenticated)
    pub ptrauth: bool,
    /// Install ObjC runtime shims (objc_msgSend, objc_getClass, etc.)
    pub objc: bool,
    /// Install Foundation/CF-lite shims (NSLog, NSObject class, CFConstantString)
    pub foundation: bool,
    /// Install random number shims (arc4random, arc4random_buf, arc4random_uniform)
    pub random: bool,
    /// Install macOS-specific platform stubs (Security, Authorization, Network, Carbon,
    /// sysmon, DataDetectors, container, NStatManager, ServiceManagement, AudioToolbox, etc.)
    /// Only meaningful on macOS; on Linux this is always false.
    pub darwin_stubs: bool,
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
            ptrauth: true,
            objc: true,
            foundation: true,
            random: true,
            #[cfg(target_os = "macos")]
            darwin_stubs: true,
            #[cfg(not(target_os = "macos"))]
            darwin_stubs: false,
        }
    }
}

/// Configuration knobs for a runtime execution.
pub struct RuntimeConfig {
    /// Security policy to evaluate before execution.
    /// `None` means skip security evaluation entirely.
    pub security_policy: Option<TrustPolicy>,
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
    /// Defaults to the host platform when created via `permissive()` or `secure()`.
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
            shim_set: ShimSet::default(),
            env_overrides: Vec::new(),
            process_name: name,
            process_path: binary_path.to_path_buf(),
            tracer: None,
            platform: Arc::new(default_platform()),
        }
    }

    /// Config with full security evaluation using Apple-default policy.
    pub fn secure(binary_path: &std::path::Path) -> Self {
        let mut cfg = Self::permissive(binary_path);
        cfg.security_policy = Some(TrustPolicy::apple_default());
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn shim_set_default_all_true() {
        let s = ShimSet::default();
        assert!(s.process);
        assert!(s.tls);
        assert!(s.dispatch);
        assert!(s.pthread);
        assert!(s.mach);
        assert!(s.lifecycle);
        assert!(s.ptrauth);
        assert!(s.objc);
        assert!(s.foundation);
        assert!(s.random);
    }

    #[test]
    fn shim_set_clone() {
        let mut original = ShimSet::default();
        original.objc = false;
        let cloned = original.clone();
        assert_eq!(cloned.objc, false);
        assert_eq!(cloned.process, true);
    }

    #[test]
    fn shim_set_debug() {
        let s = ShimSet::default();
        let dbg = format!("{:?}", s);
        assert!(dbg.contains("ShimSet"));
        assert!(dbg.contains("process"));
    }

    #[test]
    fn shim_set_selective_disable() {
        let mut s = ShimSet::default();
        s.dispatch = false;
        s.foundation = false;
        assert!(!s.dispatch);
        assert!(!s.foundation);
        assert!(s.process);
        assert!(s.tls);
    }

    #[test]
    fn permissive_config_has_no_policy() {
        let cfg = RuntimeConfig::permissive(Path::new("/tmp/test_binary"));
        assert!(cfg.security_policy.is_none());
        assert_eq!(cfg.process_name, "test_binary");
        assert!(cfg.shim_set.process);
    }

    #[test]
    fn secure_config_has_policy() {
        let cfg = RuntimeConfig::secure(Path::new("/tmp/secure_app"));
        assert!(cfg.security_policy.is_some());
    }

    #[test]
    fn secure_config_path() {
        let cfg = RuntimeConfig::secure(Path::new("/usr/local/bin/myapp"));
        assert_eq!(cfg.process_path, Path::new("/usr/local/bin/myapp"));
        assert_eq!(cfg.process_name, "myapp");
    }

    #[test]
    fn permissive_vs_secure() {
        let perm = RuntimeConfig::permissive(Path::new("/tmp/app"));
        let sec = RuntimeConfig::secure(Path::new("/tmp/app"));
        assert!(perm.security_policy.is_none());
        assert!(sec.security_policy.is_some());
        // Both should have the same defaults for shims and process info
        assert_eq!(perm.process_name, sec.process_name);
        assert!(perm.shim_set.objc);
        assert!(sec.shim_set.objc);
    }

    #[test]
    fn shim_set_all_disabled() {
        let s = ShimSet {
            process: false,
            tls: false,
            dispatch: false,
            pthread: false,
            mach: false,
            lifecycle: false,
            ptrauth: false,
            objc: false,
            foundation: false,
            random: false,
            darwin_stubs: false,
        };
        assert!(!s.process);
        assert!(!s.tls);
        assert!(!s.dispatch);
        assert!(!s.objc);
        assert!(!s.foundation);
    }
}
