//! Dylib path resolution with @rpath, @executable_path, @loader_path support.

use std::path::{Path, PathBuf};

use super::{SearchEnv, STANDARD_DYLIB_PATHS, STANDARD_FRAMEWORK_PATHS};

/// Resolves `@rpath`, `@loader_path`, and `@executable_path` prefixes.
/// Also handles DYLD_* environment search paths and framework bundles.
pub struct PathResolver {
    pub executable_path: PathBuf,
    pub loader_path: PathBuf,
    pub rpaths: Vec<PathBuf>,
    pub search_env: SearchEnv,
}

impl PathResolver {
    pub fn new(executable_path: PathBuf, rpaths: Vec<String>) -> Self {
        let loader_path = executable_path.clone();
        Self::new_with_loader(executable_path, loader_path, rpaths)
    }

    #[allow(clippy::needless_pass_by_value)] // public API convenience
    pub fn new_with_env(
        executable_path: PathBuf,
        rpaths: Vec<String>,
        search_env: SearchEnv,
    ) -> Self {
        let loader_path = executable_path.clone();
        Self::new_full(executable_path, loader_path, &rpaths, search_env)
    }

    #[allow(clippy::needless_pass_by_value)] // public API convenience
    pub fn new_with_loader(
        executable_path: PathBuf,
        loader_path: PathBuf,
        rpaths: Vec<String>,
    ) -> Self {
        Self::new_full(executable_path, loader_path, &rpaths, SearchEnv::default())
    }

    pub(crate) fn new_full(
        executable_path: PathBuf,
        loader_path: PathBuf,
        rpaths: &[String],
        search_env: SearchEnv,
    ) -> Self {
        let exe_dir = executable_path
            .parent()
            .unwrap_or(Path::new("/"))
            .to_path_buf();
        let loader_dir = loader_path.parent().unwrap_or(Path::new("/")).to_path_buf();

        let resolved_rpaths: Vec<PathBuf> = rpaths
            .iter()
            .map(|rp| {
                let expanded = rp
                    .replace("@executable_path", &exe_dir.to_string_lossy())
                    .replace("@loader_path", &loader_dir.to_string_lossy());
                PathBuf::from(expanded)
            })
            .collect();

        Self {
            executable_path,
            loader_path,
            rpaths: resolved_rpaths,
            search_env,
        }
    }

    /// Resolve a dylib install name to a candidate path on disk.
    pub fn resolve(&self, name: &str) -> Option<PathBuf> {
        let exe_dir = self.executable_path.parent().unwrap_or(Path::new("/"));
        let loader_dir = self.loader_path.parent().unwrap_or(Path::new("/"));

        if let Some(suffix) = name.strip_prefix("@rpath/") {
            for rp in &self.rpaths {
                let candidate = rp.join(suffix);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        } else if let Some(suffix) = name.strip_prefix("@executable_path/") {
            let candidate = exe_dir.join(suffix);
            if candidate.exists() {
                return Some(candidate);
            }
            return None;
        } else if let Some(suffix) = name.strip_prefix("@loader_path/") {
            let candidate = loader_dir.join(suffix);
            if candidate.exists() {
                return Some(candidate);
            }
            return None;
        } else {
            let p = PathBuf::from(name);
            if p.exists() {
                return Some(p);
            }
        }

        if let Some(found) = self.resolve_framework(name) {
            return Some(found);
        }

        let leaf = Path::new(name)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| name.to_string());

        for dir in &self.search_env.library_path {
            let candidate = dir.join(&leaf);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        for dir in STANDARD_DYLIB_PATHS {
            let candidate = PathBuf::from(dir).join(&leaf);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        for dir in &self.search_env.fallback_library_path {
            let candidate = dir.join(&leaf);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    /// Resolve a framework reference.
    fn resolve_framework(&self, name: &str) -> Option<PathBuf> {
        let fw_marker = ".framework/";
        let fw_pos = name.find(fw_marker)?;
        let fw_name_start = name[..fw_pos].rfind('/').map(|i| i + 1).unwrap_or(0);
        let fw_name = &name[fw_name_start..fw_pos];
        let in_bundle_suffix = &name[fw_pos + fw_marker.len()..];
        let bundle_subpath = format!("{fw_name}.framework/{in_bundle_suffix}");

        for dir in &self.search_env.framework_path {
            let candidate = dir.join(&bundle_subpath);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        for dir in STANDARD_FRAMEWORK_PATHS {
            let candidate = PathBuf::from(dir).join(&bundle_subpath);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        for dir in &self.search_env.fallback_framework_path {
            let candidate = dir.join(&bundle_subpath);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }
}
