//! macOS filesystem via std::fs.

use platform_core::fs::FilesystemProvider;
use std::path::{Path, PathBuf};

pub struct DarwinFilesystem;

impl FilesystemProvider for DarwinFilesystem {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exists_root() {
        let fs = DarwinFilesystem;
        assert!(fs.exists(Path::new("/")));
    }

    #[test]
    fn exists_nonexistent() {
        let fs = DarwinFilesystem;
        assert!(!fs.exists(Path::new("/nonexistent_path_xyz")));
    }

    #[test]
    fn is_dir_root() {
        let fs = DarwinFilesystem;
        assert!(fs.is_dir(Path::new("/")));
    }

    #[test]
    fn is_dir_false_for_file() {
        let fs = DarwinFilesystem;
        assert!(!fs.is_dir(Path::new("Cargo.toml")));
    }

    #[test]
    fn current_dir_ok() {
        let fs = DarwinFilesystem;
        assert!(fs.current_dir().is_ok());
    }

    #[test]
    fn read_dir_root_nonempty() {
        let fs = DarwinFilesystem;
        let entries = fs.read_dir(Path::new("/")).unwrap();
        assert!(!entries.is_empty());
    }
}
