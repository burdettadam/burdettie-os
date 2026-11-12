use crate::model::{ContextPack, MemoryChunk, MemoryObject, Query, SyncSummary};

pub trait MemoryStore {
    type Error;

    fn upsert_object(&mut self, object: &MemoryObject) -> Result<(), Self::Error>;
    fn replace_chunks(
        &mut self,
        object: &MemoryObject,
        chunks: &[MemoryChunk],
    ) -> Result<(), Self::Error>;
    fn finish_sync(&mut self, summary: &SyncSummary) -> Result<(), Self::Error>;
}

pub trait SemanticIndex {
    type Error;

    fn sync(&mut self) -> Result<SyncSummary, Self::Error>;
    fn query(&self, query: &Query) -> Result<ContextPack, Self::Error>;
}

pub trait Embedder {
    fn provider(&self) -> &str;
    fn model(&self) -> &str;
    fn dimension(&self) -> usize;
}

pub trait VectorCodec {
    fn version(&self) -> &str;
}

pub trait TierPlanner {
    fn hot_candidate_cap(&self) -> usize;
    fn warm_candidate_cap(&self) -> usize;
}
