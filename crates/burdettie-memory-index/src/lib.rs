//! Semantic memory corpus sync and metadata storage.

pub mod chunking;
pub mod codec;
pub mod db;
mod error;
pub mod rooms;
pub mod sync;
pub mod vector;

pub use chunking::ChunkPolicy;
pub use db::{ChunkHit, ObjectBody, Tombstone};
pub use error::MemoryIndexError;
pub use sync::{IndexSnapshot, SemanticMemoryIndex, SourceSpec, StoredChunk, StoredObject};

pub use burdettie_memory_core::MemoryObjectType;
