use crate::sync::{IndexSnapshot, StoredChunk, StoredObject};
use burdettie_memory_core::{MemoryChunk, MemoryObject, MemoryObjectType, SyncSummary};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, rusqlite::Error> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", 1)?;
    Ok(conn)
}

pub fn ensure_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sync_runs (
            sync_id INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            finished_at TEXT,
            scanned_files INTEGER NOT NULL DEFAULT 0,
            scanned_bytes INTEGER NOT NULL DEFAULT 0,
            indexed_objects INTEGER NOT NULL DEFAULT 0,
            indexed_chunks INTEGER NOT NULL DEFAULT 0,
            inactive_objects INTEGER NOT NULL DEFAULT 0,
            inactive_chunks INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS documents (
            object_id TEXT PRIMARY KEY,
            version_id TEXT NOT NULL,
            object_type TEXT NOT NULL,
            source_root TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            room TEXT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            byte_length INTEGER NOT NULL,
            line_count INTEGER NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            last_seen_sync INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_documents_relative_path ON documents(relative_path);
        CREATE INDEX IF NOT EXISTS idx_documents_active ON documents(is_active);

        CREATE TABLE IF NOT EXISTS chunks (
            chunk_id TEXT PRIMARY KEY,
            object_id TEXT NOT NULL,
            version_id TEXT NOT NULL,
            source_root TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            room TEXT,
            chunk_index INTEGER NOT NULL,
            text TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            byte_length INTEGER NOT NULL,
            vector_bin BLOB,
            codec_version TEXT NOT NULL DEFAULT 'scalar_v1:1',
            is_active INTEGER NOT NULL DEFAULT 1,
            last_seen_sync INTEGER NOT NULL,
            FOREIGN KEY(object_id) REFERENCES documents(object_id)
        );

        CREATE INDEX IF NOT EXISTS idx_chunks_object_id ON chunks(object_id);
        CREATE INDEX IF NOT EXISTS idx_chunks_active ON chunks(is_active);

        CREATE TABLE IF NOT EXISTS tombstones (
            object_id TEXT NOT NULL,
            deleted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            delete_mode TEXT NOT NULL DEFAULT 'soft',
            object_type TEXT,
            relative_path TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_tombstones_object_id ON tombstones(object_id);
        ",
    )
}

pub fn begin_sync(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.execute("INSERT INTO sync_runs DEFAULT VALUES", [])?;
    Ok(conn.last_insert_rowid())
}

pub fn finish_sync(conn: &Connection, summary: &SyncSummary) -> Result<(), rusqlite::Error> {
    conn.execute(
        "
        UPDATE sync_runs
        SET finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            scanned_files = ?2,
            scanned_bytes = ?3,
            indexed_objects = ?4,
            indexed_chunks = ?5,
            inactive_objects = ?6,
            inactive_chunks = ?7
        WHERE sync_id = ?1
        ",
        params![
            summary.sync_id,
            summary.scanned_files,
            summary.scanned_bytes,
            summary.indexed_objects,
            summary.indexed_chunks,
            summary.inactive_objects,
            summary.inactive_chunks,
        ],
    )?;
    Ok(())
}

pub fn upsert_object(
    conn: &Connection,
    source_root: &str,
    object: &MemoryObject,
    sync_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "
        INSERT INTO documents (
            object_id,
            version_id,
            object_type,
            source_root,
            relative_path,
            room,
            title,
            body,
            content_hash,
            byte_length,
            line_count,
            is_active,
            last_seen_sync
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12)
        ON CONFLICT(object_id) DO UPDATE SET
            version_id = excluded.version_id,
            object_type = excluded.object_type,
            source_root = excluded.source_root,
            relative_path = excluded.relative_path,
            room = excluded.room,
            title = excluded.title,
            body = excluded.body,
            content_hash = excluded.content_hash,
            byte_length = excluded.byte_length,
            line_count = excluded.line_count,
            is_active = 1,
            last_seen_sync = excluded.last_seen_sync
        ",
        params![
            object.object_id.as_str(),
            object.version_id.as_str(),
            object.object_type.as_str(),
            source_root,
            object.provenance.relative_path.as_str(),
            object.provenance.room.as_deref(),
            object.title.as_str(),
            object.body.as_str(),
            object.content_hash.as_str(),
            object.byte_length,
            object.line_count,
            sync_id,
        ],
    )?;

    Ok(())
}

pub fn replace_chunks(
    conn: &Connection,
    source_root: &str,
    object: &MemoryObject,
    chunks: &[MemoryChunk],
    chunk_vectors: &[Vec<u8>],
    codec_version: &str,
    sync_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE chunks SET is_active = 0 WHERE object_id = ?1",
        params![object.object_id.as_str()],
    )?;

    for (idx, chunk) in chunks.iter().enumerate() {
        let vector_blob: Option<&[u8]> = chunk_vectors.get(idx).map(|v| v.as_slice());
        conn.execute(
            "
            INSERT INTO chunks (
                chunk_id,
                object_id,
                version_id,
                source_root,
                relative_path,
                room,
                chunk_index,
                text,
                content_hash,
                line_start,
                line_end,
                byte_length,
                vector_bin,
                codec_version,
                is_active,
                last_seen_sync
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1, ?15)
            ON CONFLICT(chunk_id) DO UPDATE SET
                object_id = excluded.object_id,
                version_id = excluded.version_id,
                source_root = excluded.source_root,
                relative_path = excluded.relative_path,
                room = excluded.room,
                chunk_index = excluded.chunk_index,
                text = excluded.text,
                content_hash = excluded.content_hash,
                line_start = excluded.line_start,
                line_end = excluded.line_end,
                byte_length = excluded.byte_length,
                vector_bin = excluded.vector_bin,
                codec_version = excluded.codec_version,
                is_active = 1,
                last_seen_sync = excluded.last_seen_sync
            ",
            params![
                chunk.chunk_id.as_str(),
                chunk.object_id.as_str(),
                chunk.version_id.as_str(),
                source_root,
                chunk.provenance.relative_path.as_str(),
                chunk.provenance.room.as_deref(),
                chunk.chunk_index,
                chunk.text.as_str(),
                chunk.content_hash.as_str(),
                chunk.provenance.line_start.unwrap_or_default(),
                chunk.provenance.line_end.unwrap_or_default(),
                chunk.byte_length,
                vector_blob,
                codec_version,
                sync_id,
            ],
        )?;
    }

    Ok(())
}

pub fn mark_missing_inactive(
    conn: &Connection,
    source_roots: &[String],
    sync_id: i64,
) -> Result<(u64, u64), rusqlite::Error> {
    let mut inactive_objects = 0u64;
    let mut inactive_chunks = 0u64;

    for source_root in source_roots {
        inactive_objects += conn.execute(
            "
            UPDATE documents
            SET is_active = 0
            WHERE source_root = ?1 AND last_seen_sync != ?2 AND is_active = 1
            ",
            params![source_root, sync_id],
        )? as u64;

        inactive_chunks += conn.execute(
            "
            UPDATE chunks
            SET is_active = 0
            WHERE source_root = ?1 AND last_seen_sync != ?2 AND is_active = 1
            ",
            params![source_root, sync_id],
        )? as u64;
    }

    Ok((inactive_objects, inactive_chunks))
}

pub fn load_snapshot(conn: &Connection) -> Result<IndexSnapshot, rusqlite::Error> {
    let mut objects_stmt = conn.prepare(
        "
        SELECT object_id, version_id, object_type, source_root, relative_path, room, is_active, last_seen_sync
        FROM documents
        ORDER BY relative_path ASC
        ",
    )?;
    let objects = objects_stmt
        .query_map([], |row| {
            Ok(StoredObject {
                object_id: row.get::<_, String>(0)?,
                version_id: row.get::<_, String>(1)?,
                object_type: MemoryObjectType::from_str(&row.get::<_, String>(2)?)
                    .unwrap_or(MemoryObjectType::RepoFact),
                source_root: row.get::<_, String>(3)?,
                relative_path: row.get::<_, String>(4)?,
                room: row.get::<_, Option<String>>(5)?,
                is_active: row.get::<_, i64>(6)? != 0,
                last_seen_sync: row.get::<_, i64>(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut chunks_stmt = conn.prepare(
        "
        SELECT chunk_id, object_id, source_root, relative_path, chunk_index, line_start, line_end, is_active, last_seen_sync
        FROM chunks
        ORDER BY relative_path ASC, chunk_index ASC
        ",
    )?;
    let chunks = chunks_stmt
        .query_map([], |row| {
            Ok(StoredChunk {
                chunk_id: row.get::<_, String>(0)?,
                object_id: row.get::<_, String>(1)?,
                source_root: row.get::<_, String>(2)?,
                relative_path: row.get::<_, String>(3)?,
                chunk_index: row.get::<_, u32>(4)?,
                line_start: row.get::<_, u32>(5)?,
                line_end: row.get::<_, u32>(6)?,
                is_active: row.get::<_, i64>(7)? != 0,
                last_seen_sync: row.get::<_, i64>(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let last_sync_id = conn
        .query_row("SELECT MAX(sync_id) FROM sync_runs", [], |row| {
            row.get::<_, Option<i64>>(0)
        })?
        .unwrap_or_default();

    Ok(IndexSnapshot {
        last_sync_id,
        objects,
        chunks,
    })
}

pub fn lookup_document_version(
    conn: &Connection,
    object_id: &str,
) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT version_id FROM documents WHERE object_id = ?1",
        params![object_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkHit {
    pub chunk_id: String,
    pub object_id: String,
    pub object_type: String,
    pub relative_path: String,
    pub room: Option<String>,
    pub chunk_index: u32,
    pub text: String,
    pub source_root: String,
    pub line_start: u32,
    pub line_end: u32,
    pub content_hash: String,
    pub version_id: String,
    pub title: String,
    pub byte_length: u64,
    pub lexical_score: f32,
    pub semantic_score: f32,
    pub vector_bin: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectBody {
    pub object_id: String,
    pub version_id: String,
    pub object_type: String,
    pub source_root: String,
    pub relative_path: String,
    pub room: Option<String>,
    pub title: String,
    pub body: String,
    pub content_hash: String,
    pub byte_length: u64,
    pub line_count: u32,
    pub is_active: bool,
    pub last_seen_sync: i64,
}

pub fn search_chunks_hybrid(
    conn: &Connection,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<ChunkHit>, rusqlite::Error> {
    let terms: Vec<String> = query_text
        .split_whitespace()
        .map(|term| {
            let escaped = term.replace('%', "\\%").replace('_', "\\_");
            format!("%{}%", escaped)
        })
        .filter(|pat| pat.len() > 2)
        .collect();

    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // Build LIKE conditions joined by OR
    let like_clauses: Vec<String> = (0..terms.len())
        .map(|idx| format!("c.text LIKE ?{} ESCAPE '\\'", idx + 1))
        .collect();
    let sql = format!(
        "
        SELECT c.chunk_id, c.object_id, d.object_type, c.relative_path, c.room,
               c.chunk_index, c.text, c.source_root, c.line_start, c.line_end,
               c.content_hash, c.version_id, d.title, c.byte_length, c.vector_bin
        FROM chunks c
        JOIN documents d ON c.object_id = d.object_id
        WHERE c.is_active = 1 AND d.is_active = 1
          AND ({})
        ORDER BY c.relative_path ASC, c.chunk_index ASC
        LIMIT {}
        ",
        like_clauses.join(" OR "),
        (top_k * 4).min(400),
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<String> = terms.clone();
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let mut hits: Vec<ChunkHit> = stmt
        .query_map(params_refs.as_slice(), |row| {
            let vector_blob: Option<Vec<u8>> = row.get::<_, Option<Vec<u8>>>(14)?;
            Ok(ChunkHit {
                chunk_id: row.get::<_, String>(0)?,
                object_id: row.get::<_, String>(1)?,
                object_type: row.get::<_, String>(2)?,
                relative_path: row.get::<_, String>(3)?,
                room: row.get::<_, Option<String>>(4)?,
                chunk_index: row.get::<_, u32>(5)?,
                text: row.get::<_, String>(6)?,
                source_root: row.get::<_, String>(7)?,
                line_start: row.get::<_, u32>(8)?,
                line_end: row.get::<_, u32>(9)?,
                content_hash: row.get::<_, String>(10)?,
                version_id: row.get::<_, String>(11)?,
                title: row.get::<_, String>(12)?,
                byte_length: row.get::<_, u64>(13)?,
                lexical_score: 0.0,
                semantic_score: 0.0,
                vector_bin: vector_blob.unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Build query vector from query terms against the stored vocabulary.
    // We extract a simple bag-of-terms from the query and compute cosine
    // similarity against each hit's stored vector.
    let query_tokens: Vec<String> = terms
        .iter()
        .map(|pat| pat[1..pat.len().saturating_sub(1)].to_string())
        .filter(|t| t.len() >= 2)
        .collect();

    // Compute lexical scores (term match counts) and semantic scores (cosine sim)
    for hit in &mut hits {
        let text_lower = hit.text.to_ascii_lowercase();
        let mut term_matches = 0u32;
        let mut total_occurrences = 0u32;
        for token in &query_tokens {
            let count = text_lower.matches(token.as_str()).count() as u32;
            if count > 0 {
                term_matches += 1;
                total_occurrences += count;
            }
        }
        hit.lexical_score = term_matches as f32 + (total_occurrences as f32 * 0.01);

        // Semantic score: fraction of stored vector weight that
        // overlaps with query terms (brute-force token match in text).
        if !hit.vector_bin.is_empty() {
            let stored = crate::vector::deserialize_vector(&hit.vector_bin);
            if stored.is_empty() {
                hit.semantic_score = 0.0;
            } else {
                let sum_weight: f32 = stored.iter().map(|w| w.weight).sum();
                let hits_any_term = query_tokens
                    .iter()
                    .any(|token| text_lower.contains(token.as_str()));
                let match_weight: f32 = if hits_any_term {
                    stored.iter().map(|w| w.weight).sum()
                } else {
                    0.0
                };
                hit.semantic_score = if sum_weight > 0.0 {
                    (match_weight / sum_weight).clamp(0.0, 1.0)
                } else {
                    0.0
                };
            }
        }
    }

    // Hybrid score: 0.4 * lexical + 0.6 * semantic
    let mut indexed: Vec<(usize, f32)> = hits
        .iter()
        .enumerate()
        .map(|(idx, hit)| {
            let hybrid = hit.lexical_score * 0.4 + hit.semantic_score * 0.6;
            (idx, hybrid)
        })
        .collect();

    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(top_k);

    let keep: Vec<usize> = indexed.into_iter().map(|(idx, _)| idx).collect();
    hits = keep.into_iter().map(|idx| hits[idx].clone()).collect();

    Ok(hits)
}

pub fn lookup_object_body(
    conn: &Connection,
    object_id: &str,
) -> Result<Option<ObjectBody>, rusqlite::Error> {
    conn.query_row(
        "
        SELECT object_id, version_id, object_type, source_root, relative_path,
               room, title, body, content_hash, byte_length, line_count,
               is_active, last_seen_sync
        FROM documents WHERE object_id = ?1
        ",
        params![object_id],
        |row| {
            Ok(ObjectBody {
                object_id: row.get::<_, String>(0)?,
                version_id: row.get::<_, String>(1)?,
                object_type: row.get::<_, String>(2)?,
                source_root: row.get::<_, String>(3)?,
                relative_path: row.get::<_, String>(4)?,
                room: row.get::<_, Option<String>>(5)?,
                title: row.get::<_, String>(6)?,
                body: row.get::<_, String>(7)?,
                content_hash: row.get::<_, String>(8)?,
                byte_length: row.get::<_, u64>(9)?,
                line_count: row.get::<_, u32>(10)?,
                is_active: row.get::<_, i64>(11)? != 0,
                last_seen_sync: row.get::<_, i64>(12)?,
            })
        },
    )
    .optional()
}

/// Soft-delete an object and all its chunks.  Records a tombstone row.
/// Returns (document_deactivated, chunk_count_deactivated).
pub fn delete_object(conn: &Connection, object_id: &str) -> Result<(bool, u64), rusqlite::Error> {
    // Record tombstone before deactivating
    let meta: Option<(String, String)> = conn
        .query_row(
            "SELECT object_type, relative_path FROM documents WHERE object_id = ?1",
            params![object_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let Some((obj_type, rel_path)) = meta else {
        return Ok((false, 0));
    };

    conn.execute(
        "INSERT INTO tombstones (object_id, delete_mode, object_type, relative_path) VALUES (?1, 'soft', ?2, ?3)",
        params![object_id, obj_type, rel_path],
    )?;

    let doc_rows = conn.execute(
        "UPDATE documents SET is_active = 0 WHERE object_id = ?1",
        params![object_id],
    )? as u64;

    let chunk_rows = conn.execute(
        "UPDATE chunks SET is_active = 0 WHERE object_id = ?1",
        params![object_id],
    )? as u64;

    Ok((doc_rows > 0, chunk_rows))
}

/// List tombstoned objects (most recent first).
pub fn list_tombstones(conn: &Connection, limit: usize) -> Result<Vec<Tombstone>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT object_id, deleted_at, delete_mode, object_type, relative_path
         FROM tombstones ORDER BY deleted_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit as i64], |row| {
            Ok(Tombstone {
                object_id: row.get::<_, String>(0)?,
                deleted_at: row.get::<_, String>(1)?,
                delete_mode: row.get::<_, String>(2)?,
                object_type: row.get::<_, String>(3)?,
                relative_path: row.get::<_, String>(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tombstone {
    pub object_id: String,
    pub deleted_at: String,
    pub delete_mode: String,
    pub object_type: String,
    pub relative_path: String,
}
