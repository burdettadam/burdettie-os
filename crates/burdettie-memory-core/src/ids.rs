use crate::model::MemoryObjectType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }
    };
}

define_id_type!(ObjectId);
define_id_type!(VersionId);
define_id_type!(ChunkId);

pub fn content_hash_hex(bytes: &[u8]) -> String {
    digest_parts_hex([bytes].as_slice())
}

pub fn stable_object_id(object_type: MemoryObjectType, relative_path: &str) -> ObjectId {
    ObjectId::new(digest_strings_hex(
        ["object", object_type.as_str(), relative_path].as_slice(),
    ))
}

pub fn stable_version_id(content_hash: &str) -> VersionId {
    VersionId::new(digest_strings_hex(["version", content_hash].as_slice()))
}

pub fn stable_chunk_id(
    object_id: &ObjectId,
    chunk_index: u32,
    line_start: u32,
    line_end: u32,
    content_hash: &str,
) -> ChunkId {
    ChunkId::new(digest_strings_hex(
        [
            "chunk",
            object_id.as_str(),
            &chunk_index.to_string(),
            &line_start.to_string(),
            &line_end.to_string(),
            content_hash,
        ]
        .as_slice(),
    ))
}

fn digest_strings_hex(parts: &[&str]) -> String {
    let slices: Vec<&[u8]> = parts.iter().map(|part| part.as_bytes()).collect();
    digest_parts_hex(slices.as_slice())
}

fn digest_parts_hex(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            hasher.update([0u8]);
        }
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_are_repeatable() {
        let object_id =
            stable_object_id(MemoryObjectType::DurableMemory, "memories/preferences.md");
        let content_hash = content_hash_hex(b"hello world");
        let version_id = stable_version_id(&content_hash);
        let chunk_id = stable_chunk_id(&object_id, 0, 1, 3, &content_hash);

        assert_eq!(
            object_id,
            stable_object_id(MemoryObjectType::DurableMemory, "memories/preferences.md")
        );
        assert_eq!(version_id, stable_version_id(&content_hash));
        assert_eq!(
            chunk_id,
            stable_chunk_id(&object_id, 0, 1, 3, &content_hash)
        );
    }
}
