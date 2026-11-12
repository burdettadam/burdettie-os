use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MemoryIndexError {
    #[error("workspace root does not exist: {0}")]
    MissingWorkspaceRoot(PathBuf),
    #[error("required semantic memory source is missing: {0}")]
    MissingRequiredSource(PathBuf),
    #[error("path {path} is not inside workspace root {root}")]
    PathOutsideWorkspace { path: PathBuf, root: PathBuf },
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
