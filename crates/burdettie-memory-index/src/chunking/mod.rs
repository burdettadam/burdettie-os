use burdettie_memory_core::{content_hash_hex, stable_chunk_id, MemoryChunk, MemoryObject};

#[derive(Debug, Clone, Copy)]
pub struct ChunkPolicy {
    pub max_chars: usize,
    pub max_lines: usize,
}

impl Default for ChunkPolicy {
    fn default() -> Self {
        Self {
            max_chars: 2_048,
            max_lines: 32,
        }
    }
}

pub fn chunk_object(object: &MemoryObject, policy: ChunkPolicy) -> Vec<MemoryChunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut chunk_start_line = 1u32;
    let mut chunk_line_count = 0usize;
    let mut last_line = 0u32;

    for (index, piece) in object.body.split_inclusive('\n').enumerate() {
        let line_number = (index + 1) as u32;
        let next_char_count = current.len() + piece.len();
        if !current.is_empty()
            && (chunk_line_count >= policy.max_lines || next_char_count > policy.max_chars)
        {
            chunks.push(build_chunk(
                object,
                chunks.len() as u32,
                chunk_start_line,
                last_line.max(chunk_start_line),
                std::mem::take(&mut current),
            ));
            chunk_start_line = line_number;
            chunk_line_count = 0;
        }

        current.push_str(piece);
        chunk_line_count += 1;
        last_line = line_number;
    }

    if !current.is_empty() {
        chunks.push(build_chunk(
            object,
            chunks.len() as u32,
            chunk_start_line,
            last_line.max(chunk_start_line),
            current,
        ));
    }

    chunks
}

fn build_chunk(
    object: &MemoryObject,
    chunk_index: u32,
    line_start: u32,
    line_end: u32,
    text: String,
) -> MemoryChunk {
    let byte_length = text.len() as u64;
    let content_hash = content_hash_hex(text.as_bytes());
    let chunk_id = stable_chunk_id(
        &object.object_id,
        chunk_index,
        line_start,
        line_end,
        &content_hash,
    );
    let mut provenance = object.provenance.clone();
    provenance.line_start = Some(line_start);
    provenance.line_end = Some(line_end);
    provenance.content_hash = content_hash.clone();

    MemoryChunk {
        chunk_id,
        object_id: object.object_id.clone(),
        version_id: object.version_id.clone(),
        chunk_index,
        text,
        provenance,
        content_hash,
        byte_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burdettie_memory_core::{
        stable_object_id, stable_version_id, MemoryObject, MemoryObjectType, Provenance, Scope,
        SourceClass,
    };

    #[test]
    fn chunking_respects_line_boundaries() {
        let body = "one\ntwo\nthree\nfour\nfive\n".to_string();
        let byte_length = body.len() as u64;
        let content_hash = content_hash_hex(body.as_bytes());
        let object = MemoryObject {
            object_id: stable_object_id(MemoryObjectType::DurableMemory, "memories/test.md"),
            version_id: stable_version_id(&content_hash),
            object_type: MemoryObjectType::DurableMemory,
            title: "test.md".to_string(),
            body,
            scope: Scope::default(),
            provenance: Provenance {
                source_class: SourceClass::Filesystem,
                room: Some("memories".to_string()),
                relative_path: "memories/test.md".to_string(),
                line_start: None,
                line_end: None,
                content_hash: content_hash.clone(),
            },
            content_hash,
            byte_length,
            line_count: 5,
        };

        let chunks = chunk_object(
            &object,
            ChunkPolicy {
                max_chars: 64,
                max_lines: 2,
            },
        );

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].provenance.line_start, Some(1));
        assert_eq!(chunks[0].provenance.line_end, Some(2));
        assert_eq!(chunks[1].provenance.line_start, Some(3));
        assert_eq!(chunks[2].provenance.line_end, Some(5));
    }
}
