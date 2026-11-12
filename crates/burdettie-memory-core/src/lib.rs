//! Canonical semantic memory types and contracts.

mod ids;
mod model;
mod provenance;
mod traits;

pub use ids::{
    content_hash_hex, stable_chunk_id, stable_object_id, stable_version_id, ChunkId, ObjectId,
    VersionId,
};
pub use model::{
    ContextPack, Hit, MemoryChunk, MemoryObject, MemoryObjectType, Query, Scope, SyncSummary,
    Visibility,
};
pub use provenance::{Provenance, SourceClass};
pub use traits::{Embedder, MemoryStore, SemanticIndex, TierPlanner, VectorCodec};
