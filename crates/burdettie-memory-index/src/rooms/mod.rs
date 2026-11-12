use crate::MemoryIndexError;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct RoomCatalog {
    rooms: Vec<RoomRule>,
}

impl RoomCatalog {
    pub fn load(path: &Path) -> Result<Self, MemoryIndexError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw: RawCatalog = serde_yaml::from_str(&std::fs::read_to_string(path)?)?;
        let rooms = raw
            .rooms
            .into_iter()
            .map(|room| RoomRule {
                name: room.name,
                tokens: room
                    .keywords
                    .into_iter()
                    .map(|keyword| normalize_token(&keyword))
                    .collect(),
            })
            .collect();

        Ok(Self { rooms })
    }

    pub fn classify(&self, relative_path: &str) -> Option<String> {
        let first_component = relative_path.split('/').next()?;
        let token = normalize_token(first_component);

        self.rooms.iter().find_map(|room| {
            if normalize_token(&room.name) == token
                || room.tokens.iter().any(|entry| entry == &token)
            {
                Some(room.name.clone())
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone)]
struct RoomRule {
    name: String,
    tokens: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawCatalog {
    #[serde(default)]
    rooms: Vec<RawRoom>,
}

#[derive(Debug, Deserialize)]
struct RawRoom {
    name: String,
    #[serde(default)]
    keywords: Vec<String>,
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('/')
        .replace('-', "_")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_first_path_component() {
        let catalog = RoomCatalog {
            rooms: vec![
                RoomRule {
                    name: "documentation".to_string(),
                    tokens: vec!["docs".to_string()],
                },
                RoomRule {
                    name: "memories".to_string(),
                    tokens: vec!["memories".to_string()],
                },
            ],
        };

        assert_eq!(
            catalog.classify("docs/plan.md"),
            Some("documentation".to_string())
        );
        assert_eq!(
            catalog.classify("memories/repo/facts.md"),
            Some("memories".to_string())
        );
        assert_eq!(catalog.classify("crates/foo/src/lib.rs"), None);
    }
}
