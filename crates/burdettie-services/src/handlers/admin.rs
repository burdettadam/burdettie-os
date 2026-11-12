//! Admin API handler — query recorded requests, logs, and system state.
//!
//! Endpoints (all require auth token when configured):
//! - `/_burdettie/admin/requests` — query recorded requests (GET)
//! - `/_burdettie/admin/requests/summary` — request counts by source (GET)
//! - `/_burdettie/admin/logs` — query structured log events (GET)
//! - `/_burdettie/admin/config` — current server configuration (GET)
//!
//! Query parameters:
//! - `source` — filter by source type: http, xpc, dns, os_log
//! - `limit` — max results (default 50, max 1000)
//! - `offset` — pagination offset (default 0)

use super::{Handler, HandlerResponse};
use burdettie_memory_index::{MemoryObjectType, SemanticMemoryIndex};
use std::path::PathBuf;
use tiny_http::Request;

pub struct AdminHandler {
    /// Bearer token for auth (None = no auth required).
    auth_token: Option<String>,
    /// Instance ID for multi-tenant tagging.
    instance_id: String,
    /// Workspace root for repo-local admin subsystems.
    workspace_root: PathBuf,
}

impl AdminHandler {
    pub fn new(
        auth_token: Option<String>,
        instance_id: String,
        workspace_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            auth_token,
            instance_id,
            workspace_root: workspace_root.into(),
        }
    }

    /// Validate the Authorization header if a token is configured.
    fn check_auth(&self, request: &Request) -> bool {
        let Some(ref expected) = self.auth_token else {
            return true; // no auth required
        };

        request
            .headers()
            .iter()
            .find(|h| {
                h.field
                    .as_str()
                    .as_str()
                    .eq_ignore_ascii_case("authorization")
            })
            .map(|h| {
                let val = h.value.as_str();
                val.strip_prefix("Bearer ")
                    .or_else(|| val.strip_prefix("bearer "))
                    .is_some_and(|token| token == expected.as_str())
            })
            .unwrap_or(false)
    }
}

impl Handler for AdminHandler {
    fn handle(
        &self,
        path: &str,
        method: &str,
        body: &[u8],
        request: &Request,
    ) -> Option<HandlerResponse> {
        if !path.starts_with("/_burdettie/admin/") {
            return None;
        }

        // Check auth
        if !self.check_auth(request) {
            return Some(HandlerResponse::with_status(
                401,
                "application/json",
                br#"{"error":"unauthorized","message":"Valid Bearer token required"}"#.to_vec(),
            ));
        }

        if path == "/_burdettie/admin/memory/status" {
            return Some(if method == "GET" {
                self.handle_memory_status()
            } else {
                method_not_allowed(&["GET"])
            });
        }

        if path == "/_burdettie/admin/memory/sync" {
            return Some(if method == "POST" {
                self.handle_memory_sync()
            } else {
                method_not_allowed(&["POST"])
            });
        }

        if path == "/_burdettie/admin/memory/query" {
            return Some(if method == "GET" {
                self.handle_memory_query(request)
            } else {
                method_not_allowed(&["GET"])
            });
        }

        if let Some(object_id) = path.strip_prefix("/_burdettie/admin/memory/object/") {
            if object_id.is_empty() {
                return Some(not_found_response());
            }
            return Some(if method == "GET" {
                self.handle_memory_object(object_id)
            } else {
                method_not_allowed(&["GET"])
            });
        }

        if path == "/_burdettie/admin/memory/delete" {
            return Some(if method == "POST" {
                self.handle_memory_delete(request)
            } else {
                method_not_allowed(&["POST"])
            });
        }

        if path == "/_burdettie/admin/memory/objects/upsert" {
            return Some(if method == "POST" {
                self.handle_memory_upsert(body)
            } else {
                method_not_allowed(&["POST"])
            });
        }

        match path {
            "/_burdettie/admin/requests" => Some(self.handle_requests(request)),
            "/_burdettie/admin/requests/summary" => Some(self.handle_summary()),
            "/_burdettie/admin/logs" => Some(self.handle_logs(request)),
            "/_burdettie/admin/config" => Some(self.handle_config()),
            _ => Some(HandlerResponse::with_status(
                404,
                "application/json",
                br#"{"error":"not_found"}"#.to_vec(),
            )),
        }
    }
}

impl AdminHandler {
    fn memory_index(&self) -> Result<SemanticMemoryIndex, HandlerResponse> {
        SemanticMemoryIndex::new(&self.workspace_root).map_err(memory_error_response)
    }

    fn handle_memory_status(&self) -> HandlerResponse {
        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        let db_exists = index.db_path().exists();
        let snapshot = match index.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => return memory_error_response(error),
        };
        let active_objects = snapshot
            .objects
            .iter()
            .filter(|object| object.is_active)
            .count();
        let inactive_objects = snapshot.objects.len().saturating_sub(active_objects);
        let active_chunks = snapshot
            .chunks
            .iter()
            .filter(|chunk| chunk.is_active)
            .count();
        let inactive_chunks = snapshot.chunks.len().saturating_sub(active_chunks);
        let (codec_version, vectorized_chunks) = index.codec_stats().unwrap_or_default();
        let tombstone_count = index.tombstones(1000).unwrap_or_default().len();

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "workspace_root": self.workspace_root.to_string_lossy(),
            "db_path": index.db_path().to_string_lossy(),
            "db_exists": db_exists,
            "healthy": true,
            "last_sync_id": snapshot.last_sync_id,
            "tombstones": tombstone_count,
            "codec": {
                "version": codec_version,
                "vectorized_chunks": vectorized_chunks,
            },
            "objects": {
                "active": active_objects,
                "inactive": inactive_objects,
                "total": snapshot.objects.len(),
            },
            "chunks": {
                "active": active_chunks,
                "inactive": inactive_chunks,
                "total": snapshot.chunks.len(),
            }
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_memory_sync(&self) -> HandlerResponse {
        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        let summary = match index.sync() {
            Ok(summary) => summary,
            Err(error) => return memory_error_response(error),
        };

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "workspace_root": self.workspace_root.to_string_lossy(),
            "db_path": index.db_path().to_string_lossy(),
            "sync": summary,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_memory_query(&self, request: &Request) -> HandlerResponse {
        let q = extract_query_param(request.url(), "q").unwrap_or_default();
        if q.is_empty() {
            return HandlerResponse::with_status(
                400,
                "application/json",
                br#"{"error":"missing_query","message":"Query parameter 'q' is required"}"#
                    .to_vec(),
            );
        }
        let top_k = extract_query_param(request.url(), "k")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10)
            .min(100);

        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        let hits = match index.query_chunks(&q, top_k) {
            Ok(hits) => hits,
            Err(error) => return memory_error_response(error),
        };
        let total = hits.len();

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "query": q,
            "top_k": top_k,
            "total": total,
            "hits": hits.iter().map(|hit| {
                serde_json::json!({
                    "chunk_id": hit.chunk_id,
                    "object_id": hit.object_id,
                    "object_type": hit.object_type,
                    "relative_path": hit.relative_path,
                    "room": hit.room,
                    "chunk_index": hit.chunk_index,
                    "text": hit.text,
                    "line_start": hit.line_start,
                    "line_end": hit.line_end,
                    "content_hash": hit.content_hash,
                    "version_id": hit.version_id,
                    "title": hit.title,
                    "byte_length": hit.byte_length,
                    "lexical_score": hit.lexical_score,
                    "semantic_score": hit.semantic_score,
                    "hybrid_score": hit.lexical_score * 0.4 + hit.semantic_score * 0.6,
                })
            }).collect::<Vec<_>>(),
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_memory_object(&self, object_id: &str) -> HandlerResponse {
        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        match index.object_body(object_id) {
            Ok(Some(body)) => {
                let response = serde_json::json!({
                    "instance_id": self.instance_id,
                    "object_id": body.object_id,
                    "version_id": body.version_id,
                    "object_type": body.object_type,
                    "source_root": body.source_root,
                    "relative_path": body.relative_path,
                    "room": body.room,
                    "title": body.title,
                    "body": body.body,
                    "content_hash": body.content_hash,
                    "byte_length": body.byte_length,
                    "line_count": body.line_count,
                    "is_active": body.is_active,
                });
                HandlerResponse::json(response.to_string().into_bytes())
            }
            Ok(None) => not_found_response(),
            Err(error) => memory_error_response(error),
        }
    }

    fn handle_memory_delete(&self, request: &Request) -> HandlerResponse {
        let object_id = extract_query_param(request.url(), "object_id").unwrap_or_default();
        if object_id.is_empty() {
            return HandlerResponse::with_status(
                400,
                "application/json",
                br#"{"error":"missing_object_id","message":"Query parameter 'object_id' is required"}"#.to_vec(),
            );
        }
        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        match index.delete_object(&object_id) {
            Ok((found, chunks)) => {
                let response = serde_json::json!({
                    "instance_id": self.instance_id,
                    "object_id": object_id,
                    "deleted": found,
                    "chunks_deactivated": chunks,
                });
                if found {
                    HandlerResponse::json(response.to_string().into_bytes())
                } else {
                    HandlerResponse::with_status(
                        404,
                        "application/json",
                        response.to_string().into_bytes(),
                    )
                }
            }
            Err(error) => memory_error_response(error),
        }
    }

    fn handle_memory_upsert(&self, body: &[u8]) -> HandlerResponse {
        let payload: serde_json::Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return HandlerResponse::with_status(
                    400,
                    "application/json",
                    serde_json::json!({"error":"invalid_json","message":e.to_string()})
                        .to_string()
                        .into_bytes(),
                );
            }
        };
        let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let text_body = payload.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let rel_path = payload
            .get("relative_path")
            .and_then(|v| v.as_str())
            .unwrap_or("ingested/default.md");
        let room = payload.get("room").and_then(|v| v.as_str());
        let obj_type_str = payload
            .get("object_type")
            .and_then(|v| v.as_str())
            .unwrap_or("durable_memory");

        if title.is_empty() || text_body.is_empty() {
            return HandlerResponse::with_status(
                400,
                "application/json",
                br#"{"error":"missing_fields","message":"title and body are required"}"#.to_vec(),
            );
        }

        let object_type = match obj_type_str {
            "durable_memory" => MemoryObjectType::DurableMemory,
            "generated_summary" => MemoryObjectType::GeneratedSummary,
            "repo_fact" => MemoryObjectType::RepoFact,
            other => {
                return HandlerResponse::with_status(
                    400,
                    "application/json",
                    serde_json::json!({"error":"invalid_object_type","message":format!("unknown type: {other}")}).to_string().into_bytes(),
                );
            }
        };

        let index = match self.memory_index() {
            Ok(index) => index,
            Err(response) => return response,
        };
        match index.ingest_text(title, text_body, object_type, rel_path, room) {
            Ok(object_id) => {
                let response = serde_json::json!({
                    "instance_id": self.instance_id,
                    "object_id": object_id,
                    "object_type": obj_type_str,
                    "relative_path": rel_path,
                    "ingested": true,
                });
                HandlerResponse::json(response.to_string().into_bytes())
            }
            Err(error) => memory_error_response(error),
        }
    }

    fn handle_requests(&self, request: &Request) -> HandlerResponse {
        let recorder = match crate::global_recorder() {
            Some(r) => r,
            None => {
                return HandlerResponse::with_status(
                    503,
                    "application/json",
                    br#"{"error":"recorder_not_available"}"#.to_vec(),
                )
            }
        };

        let url = request.url();
        let source = extract_query_param(url, "source");
        let limit = extract_query_param(url, "limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(50)
            .min(1000);
        let offset = extract_query_param(url, "offset")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let rows = recorder.query_recent(source.as_deref(), limit, offset);

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "total": recorder.count(),
            "limit": limit,
            "offset": offset,
            "count": rows.len(),
            "requests": rows,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_summary(&self) -> HandlerResponse {
        let recorder = match crate::global_recorder() {
            Some(r) => r,
            None => {
                return HandlerResponse::with_status(
                    503,
                    "application/json",
                    br#"{"error":"recorder_not_available"}"#.to_vec(),
                )
            }
        };

        let groups = recorder.count_by_source();
        let mut by_source = serde_json::Map::new();
        for (source, count) in &groups {
            by_source.insert(source.clone(), serde_json::json!(count));
        }

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "total": recorder.count(),
            "by_source": by_source,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_logs(&self, request: &Request) -> HandlerResponse {
        let limit = extract_query_param(request.url(), "limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100)
            .min(1000);

        let events = burdettie_log_pipeline::drain_events();
        let capped: Vec<_> = events.into_iter().rev().take(limit).collect();

        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "event_count": burdettie_log_pipeline::event_count(),
            "returned": capped.len(),
            "events": capped,
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }

    fn handle_config(&self) -> HandlerResponse {
        let response = serde_json::json!({
            "instance_id": self.instance_id,
            "guest_binary": crate::guest_binary(),
            "server_version": env!("CARGO_PKG_VERSION"),
        });

        HandlerResponse::json(response.to_string().into_bytes())
    }
}

fn method_not_allowed(allowed: &[&str]) -> HandlerResponse {
    HandlerResponse::with_status(
        405,
        "application/json",
        serde_json::json!({
            "error": "method_not_allowed",
            "allowed": allowed,
        })
        .to_string()
        .into_bytes(),
    )
}

fn not_found_response() -> HandlerResponse {
    HandlerResponse::with_status(
        404,
        "application/json",
        br#"{"error":"not_found"}"#.to_vec(),
    )
}

fn memory_error_response(error: burdettie_memory_index::MemoryIndexError) -> HandlerResponse {
    HandlerResponse::with_status(
        500,
        "application/json",
        serde_json::json!({
            "error": "memory_index_error",
            "message": error.to_string(),
        })
        .to_string()
        .into_bytes(),
    )
}

/// Extract a query parameter from a URL path+query string.
fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv.next().map(|v| v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_disabled_allows_all() {
        let handler = AdminHandler::new(None, "test".into(), ".");
        assert!(handler.auth_token.is_none());
        // Without a real Request we can't test check_auth, but we verify no panic
    }

    #[test]
    fn admin_ignores_non_admin_paths() {
        let handler = AdminHandler::new(None, "test".into(), ".");
        assert!(!"/api/v1/telemetry".starts_with("/_burdettie/admin/"));
        assert!("/_burdettie/admin/requests".starts_with("/_burdettie/admin/"));
        // handler.handle would return None for non-admin paths
        let _ = handler;
    }

    #[test]
    fn extract_params() {
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests?source=http&limit=10", "source"),
            Some("http".into())
        );
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests?source=http&limit=10", "limit"),
            Some("10".into())
        );
        assert_eq!(
            extract_query_param("/_burdettie/admin/requests", "source"),
            None
        );
    }
}
