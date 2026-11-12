use crate::ids::{ChunkId, ObjectId, VersionId};
use crate::provenance::Provenance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryObjectType {
    DurableMemory,
    GeneratedSummary,
    RepoFact,
}

impl MemoryObjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DurableMemory => "durable_memory",
            Self::GeneratedSummary => "generated_summary",
            Self::RepoFact => "repo_fact",
        }
    }

    /// Parse a canonical stored object type name, returning `None` for unknown values.
    #[allow(clippy::should_implement_trait)] // Retain the existing Option-returning API alongside FromStr.
    pub fn from_str(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

/// An unrecognized memory object type name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseMemoryObjectTypeError;

impl std::fmt::Display for ParseMemoryObjectTypeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unrecognized memory object type")
    }
}

impl std::error::Error for ParseMemoryObjectTypeError {}

impl std::str::FromStr for MemoryObjectType {
    type Err = ParseMemoryObjectTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "durable_memory" => Ok(Self::DurableMemory),
            "generated_summary" => Ok(Self::GeneratedSummary),
            "repo_fact" => Ok(Self::RepoFact),
            _ => Err(ParseMemoryObjectTypeError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Shared,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub workspace_id: Option<String>,
    pub visibility: Visibility,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            workspace_id: None,
            visibility: Visibility::Private,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryObject {
    pub object_id: ObjectId,
    pub version_id: VersionId,
    pub object_type: MemoryObjectType,
    pub title: String,
    pub body: String,
    pub scope: Scope,
    pub provenance: Provenance,
    pub content_hash: String,
    pub byte_length: u64,
    pub line_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryChunk {
    pub chunk_id: ChunkId,
    pub object_id: ObjectId,
    pub version_id: VersionId,
    pub chunk_index: u32,
    pub text: String,
    pub provenance: Provenance,
    pub content_hash: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub top_k: usize,
    pub workspace_id: Option<String>,
    pub object_types: Vec<MemoryObjectType>,
    pub latency_budget_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub chunk_id: ChunkId,
    pub object_id: ObjectId,
    pub score: f32,
    pub lexical_score: Option<f32>,
    pub semantic_score: Option<f32>,
    pub snippet: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextPack {
    pub hits: Vec<Hit>,
    pub token_estimate: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSummary {
    pub sync_id: i64,
    pub scanned_files: u64,
    pub scanned_bytes: u64,
    pub indexed_objects: u64,
    pub indexed_chunks: u64,
    pub inactive_objects: u64,
    pub inactive_chunks: u64,
}
