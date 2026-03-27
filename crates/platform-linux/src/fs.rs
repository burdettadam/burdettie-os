//! Linux filesystem via std::fs.

use platform_core::fs::FilesystemProvider;
use std::path::{Path, PathBuf};

pub struct LinuxFilesystem;

impl FilesystemProvider for LinuxFilesystem {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf> {
        std::fs::read_link(path)
    }

    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<String>> {
        let entries = std::fs::read_dir(path)?;
        let mut names = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        std::env::current_dir()
    }
}
