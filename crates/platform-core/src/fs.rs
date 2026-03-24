//! Filesystem abstraction.

use std::path::{Path, PathBuf};

/// Filesystem operations needed by the runtime.
pub trait FilesystemProvider: Send + Sync {
    /// Read the entire contents of a file.
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;

    /// Check whether a path exists.
    fn exists(&self, path: &Path) -> bool;

    /// Check whether a path is a directory.
    fn is_dir(&self, path: &Path) -> bool;

    /// Read a symbolic link's target.
    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf>;

    /// List entries in a directory.
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<String>>;

    /// Get the current working directory.
    fn current_dir(&self) -> std::io::Result<PathBuf>;
}
