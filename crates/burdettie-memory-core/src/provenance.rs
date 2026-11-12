use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    Filesystem,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source_class: SourceClass,
    pub room: Option<String>,
    pub relative_path: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub content_hash: String,
}
