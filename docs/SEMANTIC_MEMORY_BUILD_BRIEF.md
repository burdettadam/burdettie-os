# Semantic Memory Build Brief

> Build a first-class semantic memory subsystem for BurdettieOS with local,
> CPU-first retrieval and a TurboQuant-style warm-tier compression path.

## Goal

Deliver a reusable semantic memory subsystem that is owned by the repo, not by
one agent integration, with explicit storage, query, deletion, and tiering
contracts.

The success contract is:

- semantic memory is a first-class BurdettieOS subsystem with stable APIs
- local query path targets 25 ms and must stay below 40 ms P95 on the reference
  corpus
- phase-1 semantic corpus is limited to durable memory files, generated
  summaries, and repo facts
- raw conversation traces and raw tool logs stay cold and only influence
  retrieval through derived summaries or extracted facts
- every returned hit resolves to stable provenance
- the workspace bridge and MCP transport remain adapters, not the system of
  record

## Current Starting Point

- `memories/` already holds curated durable memory and repo-local notes.
- `mempalace.yaml` already defines room-level taxonomy that can seed semantic
  scopes.
- `.vscode/mcp.json` already exposes a workspace memory bridge transport, but it
  does not define semantic storage or retrieval behavior.
- `burdettie-services` already owns local HTTP transport and admin endpoints.
- `burdettie-log-pipeline` already captures structured runtime events.
- No workspace crate currently owns a semantic object model, semantic index,
  vector store, codec versioning, or deletion/redaction flow.

## What Moves Into Semantic Memory

Semantic memory is not the new home for all memory. It is the primary retrieval
layer for fuzzy, rankable, long-lived knowledge.

| Memory class | Source of truth | Direct semantic indexing | Notes |
|---|---|---:|---|
| DurableMemory | `memories/` | Yes | Curated preferences, rules, stable facts |
| GeneratedSummary | semantic subsystem | Yes | Derived from sessions, diffs, or traces |
| RepoFact | semantic subsystem | Yes | Extracted facts about files, symbols, services, and plans |
| Procedural rules and safety constraints | `memories/` and explicit config | Shadow only | Keep symbolic authority outside approximate retrieval |
| Raw conversation traces | bridge-managed logs / append-only stores | No | Cold only; summarize before promotion |
| Raw tool logs and request captures | recorder and log pipeline stores | No | Feed lower-priority summaries or facts |

This is the intended migration boundary: semantic memory becomes authoritative
for rankable knowledge objects, while exact instructions, raw traces, and audit
records remain outside the primary semantic index.

## Subsystem Placement

Semantic memory belongs beside the other Burdettie product subsystems, not in
the loader core and not inside the workspace bridge.

```text
memories/ + repo scanners + summary producers
                    |
                    v
        burdettie-memory-core
                    |
                    v
        burdettie-memory-index
          |            |           |
          |            |           +-- hot tier: in-memory exact or mixed precision
          |            +-------------- warm tier: compressed vectors + manifests
          +--------------------------- metadata, provenance, deletion state
                    |
                    v
            burdettie-services
          /_burdettie/admin/memory/*
                    |
                    v
          workspace bridge / MCP clients
```

### On-disk ownership

- Curated source material stays in `memories/`.
- Generated semantic state lives under `.mempalace/semantic/` because that tree
  is already local-only and gitignored.
- Short-lived rebuild scratch space may use `tmp/semantic-memory/`, but it is
  not the authoritative store.

Recommended local layout:

```text
.mempalace/
  semantic/
    metadata.sqlite
    manifest.json
    segments/
      hot/
      warm/
    tombstones/
    queues/
      reembed/
      compaction/
```

Raw conversation traces do not move into this store. The subsystem persists
derived summaries, extracted facts, vectors, manifests, and deletion state.

## New Crates

The first credible slice should add two crates, not a mini-platform.

### `burdettie-memory-core`

Owns the stable domain model and public Rust contracts.

```text
crates/burdettie-memory-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── ids.rs
    ├── model.rs
    ├── provenance.rs
    ├── query.rs
    ├── policy.rs
    └── traits.rs
```

Responsibilities:

- canonical object types: `DurableMemory`, `GeneratedSummary`, `RepoFact`
- stable IDs for objects, versions, and chunks
- provenance contracts: room, path, line/span, hash, source class
- query and hit types shared by all clients
- traits for embedding, indexing, storage, ranking, and tier planning

Non-responsibilities:

- no HTTP transport
- no SQLite schema
- no ANN implementation details
- no log or bridge ownership

### `burdettie-memory-index`

Owns ingestion, storage, ranking, and lifecycle management.

```text
crates/burdettie-memory-index/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── db/
    ├── sync/
    ├── chunking/
    ├── lexical/
    ├── vector/
    ├── codec/
    ├── tiering/
    ├── delete/
    └── metrics/
```

Responsibilities:

- scan and normalize in-scope sources into canonical objects
- maintain SQLite metadata, manifests, and chunk tables
- implement lexical retrieval and hybrid reranking
- own hot/warm tier policies and storage formats
- own vector metadata versioning: provider, model, dimension, codec version
- own deletion, redaction, tombstones, and compaction queues
- ship the TurboQuant-style warm-tier codec module when the baseline retrieval
  path is already proven

Non-responsibilities:

- no public transport routing
- no request recording tables in `burdettie-services`
- no raw log capture inside `burdettie-log-pipeline`

### Why not split a codec crate immediately

The TurboQuant-style math layer is a real subsystem boundary, but it does not
need its own workspace crate until the codec is reused outside semantic memory,
for example by a future model runtime or KV-cache path. In the first build
brief, keep `codec/` inside `burdettie-memory-index` and split it only when the
reuse is concrete.

## Changes In Existing Crates

| Existing crate | Change | Must not become |
|---|---|---|
| `burdettie-services` | Add memory admin and query handlers; translate HTTP or MCP payloads to core types | The semantic index owner |
| `burdettie-log-pipeline` | Add structured export or snapshot hooks that summary producers can read | The chunker, embedder, or ranker |
| `runtime-diagnostics` | Add latency, recall-debug, compaction, and tier counters | The semantic storage layer |
| `mempalace.yaml` | Remains the room taxonomy seed | The semantic index manifest |
| `.vscode/mcp.json` | Remains a thin transport adapter | The durable data store |

## API Contracts

### Internal Rust contracts

`burdettie-memory-core` should define these stable interfaces:

| Contract | Purpose | Notes |
|---|---|---|
| `MemoryObject` | Canonical semantic object | Carries object type, scope, version, and provenance |
| `MemoryChunk` | Searchable unit | Stable chunk ID with path or span provenance |
| `Query` | Search request | Includes scope, filters, latency budget, and top-k |
| `Hit` | Ranked result | Carries total score, lexical score, semantic score, snippet, and provenance |
| `ContextPack` | Final retrieval bundle | Bounded chunk set and token estimate |
| `Embedder` | Batched embedding provider | Versioned by provider, model, and dimension |
| `VectorCodec` | Encode and score vectors | Supports baseline float path and warm-tier compressed path |
| `MemoryStore` | Metadata and object persistence | Owns manifests, versions, and tombstones |
| `SemanticIndex` | Search and sync boundary | Owns hybrid retrieval, filters, and rerank |
| `TierPlanner` | Hot and warm policy | Owns promotion, demotion, and candidate caps |

Non-negotiable rules:

- every chunk must resolve back to a real source location or canonical object ID
- embedding metadata is stored with the vector so re-embed is explicit, never
  silent
- delete and redact requests must invalidate query visibility before background
  compaction finishes

### External service contracts

`burdettie-services` should expose a narrow admin and query surface:

| Endpoint | Purpose | Owner |
|---|---|---|
| `/_burdettie/admin/memory/status` | health, corpus counts, active manifest, queue depth | `burdettie-services` transport + `burdettie-memory-index` data |
| `/_burdettie/admin/memory/sync` | ingest or refresh in-scope sources | `burdettie-memory-index` |
| `/_burdettie/admin/memory/query` | bounded search over semantic corpus | `burdettie-memory-index` |
| `/_burdettie/admin/memory/object/<id>` | inspect canonical object and versions | `burdettie-memory-index` |
| `/_burdettie/admin/memory/delete` | tombstone, redact, or hard delete | `burdettie-memory-index` |

The MCP bridge should mirror those as `memory.status`, `memory.sync`,
`memory.query`, `memory.get_object`, and `memory.delete`, but the bridge must
remain adapter-thin.

Required query request shape:

- `query_text`
- `scope` with workspace and visibility filters
- `k`
- `latency_budget_ms`
- `filters` for object types, rooms, paths, or time ranges
- `retrieval_profile` with hot-first, warm-fallback, and candidate caps

Required query response shape:

- ranked hits with stable provenance
- score breakdown: lexical, semantic, total
- token estimate or context size estimate
- execution stats: latency, candidates scored, tier hits, active codec version

## Decomposition And Priority Order

| Phase | Slice | Why it comes now | MVP |
|---|---|---|---|
| P0 | Corpus ledger and canonical model | No retrieval work is credible until IDs, provenance, and sync are deterministic | Yes |
| P1 | Lexical query surface | Gives an immediately verifiable search baseline before embeddings or codecs complicate the stack | Yes |
| P2 | Semantic baseline ranking | Proves that embeddings and hybrid rerank help on real repo queries before introducing compression | Yes |
| P3 | Tiered storage and TurboQuant warm codec | Adds the actual long-term compression path after baseline ranking is measurable | Yes |
| P4 | Generated summaries and repo-fact producers | Expands corpus quality without flooding the index with raw traces | No |
| P5 | Deletion, redaction, backup, and compaction hardening | Required before the subsystem can be treated as durable infrastructure | No |

### P0. Corpus ledger and canonical model

Ship `burdettie-memory-core` and the SQLite-backed metadata skeleton in
`burdettie-memory-index`.

Must include:

- canonical objects and chunk IDs
- room mapping seeded from `mempalace.yaml`
- sync of `memories/` and explicit repo-fact fixture inputs
- idempotent re-sync behavior
- provenance that resolves to file path and line/span where applicable

### P1. Lexical query surface

Ship a usable search path before embedding anything.

Must include:

- lexical index over canonical chunks
- room, path, and object-type filters
- `status`, `sync`, `query`, and `object` endpoints
- result snippets with provenance

This phase is where the subsystem first proves that it can answer repo-scoped
memory queries without the bridge owning storage.

### P2. Semantic baseline ranking

Add provider-agnostic embeddings, vector metadata, and hybrid reranking using a
baseline float or mixed-precision scoring path.

Must include:

- `Embedder` abstraction with explicit version metadata
- brute-force semantic scoring over the current corpus
- lexical plus semantic rerank with score breakdowns
- gold-query evaluation harness for repo memory questions

Do not introduce ANN structures or compression until this phase proves that
semantic ranking materially improves retrieval quality.

### P3. Tiered storage and TurboQuant warm codec

Add the actual first-class semantic memory compression path.

Must include:

- hot and warm tier contracts
- manifest-based vector segment activation
- codec versioning
- TurboQuant-style online warm-tier codec for compressed storage and search
- per-query candidate caps and tier fallbacks that enforce the latency budget

The first credible warm-tier design is hybrid:

- hot tier keeps higher precision vectors for a bounded active set
- warm tier stores compressed vectors and metadata for the full primary corpus
- rerank stays small and precise

### P4. Generated summaries and repo-fact producers

Promote new knowledge classes into the system without indexing raw noise.

Must include:

- generated summary ingest from curated session or diff summarizers
- repo-fact extraction for files, symbols, services, and plans
- lower-priority import hooks from `burdettie-log-pipeline` and service recorder
- source weighting so curated `memories/` content outranks low-signal derived data

### P5. Deletion and operational hardening

Finish the durability contract.

Must include:

- tombstones and immediate visibility filtering
- background compaction
- backup and restore by manifest
- re-embed queue management
- admin metrics and consistency checks

## MVP Boundary

### Included in MVP

- first-class semantic subsystem with dedicated crates
- canonical support for `DurableMemory`, `GeneratedSummary`, and `RepoFact`
- lexical plus semantic hybrid retrieval with stable provenance
- local state under `.mempalace/semantic/`
- transport hooks in `burdettie-services`
- hot and warm tier contracts
- TurboQuant-style warm-tier codec path
- latency budgeting and execution stats in query responses

### Explicitly out of scope for MVP

- indexing raw conversation traces directly into the primary semantic store
- turning `burdettie-services` into the semantic index owner
- turning `burdettie-log-pipeline` into an embedder or retriever
- external vector databases or remote ANN services as the system of record
- model-runtime or KV-cache unification work
- UI-heavy browsing tools beyond admin and MCP surfaces

## Acceptance Criteria

### Global exit criteria

The build brief is complete only when all of these are true:

1. Query results always return stable provenance.
2. Delete or redact requests hide objects from results immediately, even before
   compaction.
3. The warm-tier codec is versioned and swappable without rewriting the public
   query contract.
4. Reference-corpus query latency stays within the 25 ms target and 40 ms P95
   budget.
5. Raw traces remain cold and only enter retrieval through derived objects.

### Phase acceptance checks

| Phase | Acceptance criteria |
|---|---|
| P0 | Re-running sync produces identical object and chunk IDs for unchanged sources |
| P1 | A gold set of repo and memory queries returns the correct file in top-5 using lexical retrieval only |
| P2 | Hybrid ranking improves gold-set recall over P1 without losing provenance stability |
| P3 | Warm-tier compressed search matches the P2 top-k set closely enough to preserve task quality while meeting the latency budget |
| P4 | Derived summaries and repo facts are searchable, but curated memory still dominates when both answer the same query |
| P5 | Restore by manifest, tombstone replay, and re-embed queues all survive process restart without data loss |

## Fastest Credible Verification Path

Use a repo-local proof path before expanding the subsystem:

1. Build a gold query set from `memories/`, the major plan docs, and a small set
   of known repo facts.
2. Verify P1 lexical retrieval on that gold set.
3. Verify P2 hybrid retrieval against the same set before adding compression.
4. Add P3 warm-tier compression and compare top-k stability, latency, and bytes
   per vector against the P2 baseline.
5. Run delete and redact scenarios and confirm immediate result suppression.

The wrong move is to jump straight to ANN or external vector infrastructure.
The right move is to prove deterministic corpus ownership, then prove search
quality, then compress.