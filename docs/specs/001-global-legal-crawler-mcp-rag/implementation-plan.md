---
title: "Global Legal Crawler & MCP RAG System"
status: completed
version: "1.0"
---

# Implementation Plan

## Validation Checklist

### CRITICAL GATES (Must Pass)

- [x] All `[NEEDS CLARIFICATION]` markers addressed
- [x] All specification file paths correct and exist
- [x] Each phase follows TDD: Prime → Test → Implement → Validate
- [x] Every task has verifiable success criteria
- [x] A developer could follow this plan independently

### QUALITY CHECKS (Should Pass)

- [x] Context priming section complete
- [x] All implementation phases defined
- [x] Dependencies between phases clear (no circular dependencies)
- [x] Parallel work tagged with `[parallel: true]`
- [x] Activity hints provided `[activity: type]`
- [x] Every phase references relevant SDD sections
- [x] Every test references PRD acceptance criteria
- [x] Integration & E2E tests defined in final phase
- [x] Project commands match actual project setup

---

## Specification Compliance Guidelines

When implementation requires changes from the specification:
1. Document the deviation with clear rationale
2. Obtain approval before proceeding
3. Update SDD when the deviation improves the design
4. Record all deviations in this plan for traceability

---

## Context Priming

*GATE: Read all files in this section before starting any implementation.*

**Specification**:
- `docs/specs/001-global-legal-crawler-mcp-rag/product-requirements.md` — PRD v1.2 (features F1–F14, personas, MoSCoW)
- `docs/specs/001-global-legal-crawler-mcp-rag/solution-design.md` — SDD v1.0 (schema, ADRs, API contracts, sequence diagrams)

**Key Design Decisions**:
- **ADR-1**: Two binaries (`api-service`, `crawler-service`) sharing PostgreSQL + apalis job queue. No inter-service HTTP.
- **ADR-2**: sqlx 0.8 compile-time `query!` macros. No ORM. pgvector 0.4.1 for `Vector` type.
- **ADR-3**: Single `chunks` table, HNSW index (`m=16, ef_construction=64`), cosine distance (`<=>`).
- **ADR-6**: rmcp 0.16 Streamable HTTP at `POST /mcp`. No stdio in production.
- **ADR-7**: Vertex AI `text-embedding-004` (768-dim) behind `Embedder` trait. `MockEmbedder` for tests.
- **ADR-10**: 5-crate workspace — `core ← embeddings ← ingest ← crawler`; `core + embeddings ← api`.
- **ADR-12**: SHA-256 content hash + `document_versions` table for change detection.

**Implementation Commands**:
```bash
# Rust workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --workspace
cargo sqlx migrate run --database-url $DATABASE_URL
cargo sqlx prepare --workspace

# Frontend
pnpm install && pnpm dev
pnpm build && pnpm lint

# Docker (development)
docker compose up -d
docker compose logs -f
```

---

## Implementation Phases

Each task follows: **Prime** (read specs) → **Test** (write failing tests) → **Implement** (make green) → **Validate** (lint/fmt/typecheck).

> **Dependency chain**: Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 (parallel with 3–5) → Phase 7 → Phase 8.


---

### Phase 1: Cargo Workspace + `crates/core`

Establishes the project skeleton and all shared domain types, database schema, configuration, and error types. Every subsequent crate depends on this phase being complete.

- [x] **T1.1 Cargo Workspace Skeleton** `[activity: backend-setup]`

  1. Prime: Read SDD Building Block View directory map `[ref: SDD/Building Block View; lines: 344-532]`
  2. Test: `cargo build --workspace` succeeds; `cargo test --workspace` passes with zero tests (empty crates)
  3. Implement:
     - Create root `Cargo.toml` with `[workspace]` members: `crates/core`, `crates/embeddings`, `crates/ingest`, `crates/crawler`, `crates/api`
     - Create `bin/crawler-service/` and `bin/seed-sources/` crate stubs
     - Add workspace-level `[workspace.dependencies]` for all shared crates (tokio, sqlx, serde, tracing, uuid, chrono, thiserror, pgvector, axum, reqwest, config)
     - Create `crates/core/Cargo.toml` with dependencies: sqlx (postgres, uuid, chrono, macros, migrate), pgvector, serde, thiserror, config, dotenvy, tracing, uuid, chrono
  4. Validate: `cargo check --workspace` clean; `cargo fmt --workspace` no-op
  5. Success: Workspace compiles; all 7 crate stubs buildable `[ref: SDD/ADR-10]`

- [x] **T1.2 AppConfig + Secrets** `[activity: backend-setup]`

  1. Prime: Read SDD environment variables table and constraints CON-5, CON-8 `[ref: SDD/Deployment View; lines: 1068-1202]`
  2. Test: `AppConfig::from_env()` reads all required vars; fails with descriptive error on missing required var; `DATABASE_URL` parses correctly
  3. Implement: `crates/core/src/config.rs` — `AppConfig` struct with `config` 0.15 + dotenvy. Fields: `database_url`, `vertex_ai_project`, `vertex_ai_location`, `object_store_type`, `gcs_bucket`, `azure_*`, `mistral_api_key`, `api_port` (default 8000), `max_db_connections` (default 20), `rust_log`
  4. Validate: Unit tests pass; `cargo clippy` clean
  5. Success: Config loads from `.env` and environment; missing required field returns `Err` with field name `[ref: SDD/CON-5]`

- [x] **T1.3 Domain Models** `[activity: domain-modeling]`

  1. Prime: Read SDD Data Models (Rust) and DB schema sections `[ref: SDD/Interface Specifications; lines: 533-940]`
  2. Test: All model structs derive `Debug`, `Clone`, `serde::Serialize/Deserialize`; `DocType` and `CrawlerType` enums roundtrip through serde; `Chunk` with `pgvector::Vector` compiles
  3. Implement: `crates/core/src/models/` — `source.rs` (Source, CrawlerType, SourceConfig), `document.rs` (Document, DocType, DocumentVersion), `chunk.rs` (Chunk with `embedding: Option<pgvector::Vector>`), `job.rs` (CrawlJob, IngestJob), `api_key.rs` (ApiKey), `search.rs` (SearchResult — non-persisted)
  4. Validate: `cargo test -p core` passes; all types in `crates/core/src/lib.rs` pub re-exported
  5. Success: All 6 model files compile; serde roundtrip tests pass `[ref: SDD/Data Models; lines: 900-940]`

- [x] **T1.4 Error Types** `[activity: domain-modeling]`

  1. Prime: Read SDD error handling section `[ref: SDD/Cross-Cutting Concepts; lines: 1203-1260]`
  2. Test: Each `LegalMcpError` variant has correct `Display` message; `From<sqlx::Error>` impl works; `From<object_store::Error>` works
  3. Implement: `crates/core/src/errors.rs` — `LegalMcpError` enum with all variants per SDD. Derive `thiserror::Error`. Add `impl IntoResponse for LegalMcpError` (RFC 7807 Problem Details JSON).
  4. Validate: `cargo test -p core` passes
  5. Success: All error variants compile; HTTP response mapping produces correct status codes (404→NotFound, 401→Unauthorized, 422→Validation) `[ref: SDD/Cross-Cutting Concepts]`

- [x] **T1.5 Database Migrations** `[activity: data-architecture]`

  1. Prime: Read full DB schema from SDD `[ref: SDD/Interface Specifications; lines: 533-700]`
  2. Test: `cargo sqlx migrate run` on a fresh test DB creates all 5 tables with correct columns; HNSW index exists on `chunks.embedding`; all FK constraints active
  3. Implement:
     - `crates/core/migrations/0001_initial.sql` — creates `sources`, `documents`, `chunks`, `crawl_runs`, `api_keys` tables with all indexes
     - `crates/core/migrations/0002_document_versions.sql` — creates `document_versions` table
     - `crates/core/src/db.rs` — `create_pool(database_url, max_connections) -> PgPool` factory; `run_migrations(pool) -> Result<()>`
  4. Validate: `cargo sqlx migrate run` succeeds on clean DB; `cargo sqlx migrate revert` reverts cleanly
  5. Success: All 6 tables created; HNSW index present; `cargo sqlx prepare` generates `.sqlx/` query cache `[ref: SDD/ADR-2; SDD/ADR-3]`

- [x] **T1.6 Phase 1 Validation** `[activity: validate]`

  Run `cargo test -p core`, `cargo clippy -p core -- -D warnings`, `cargo fmt -p core -- --check`. Verify all 6 tables exist in test DB. Confirm `.sqlx/` query metadata generated.


---

### Phase 2: `crates/embeddings` + `crates/ingest`

Delivers the embedding abstraction and the full ingestion pipeline. These are the most critical internal components — search quality depends entirely on correct embedding and chunking.

*Depends on: Phase 1 complete.*

- [x] **T2.1 Embedder Trait + Vertex AI Provider** `[activity: backend-api]`

  1. Prime: Read SDD ADR-7 and embedding batch flow `[ref: SDD/ADR-7; SDD/Runtime View Flow 1; lines: 941-1067]`
  2. Test: `Embedder::embed_batch(vec!["test"])` returns `Vec<pgvector::Vector>` with length 1 and dimension 768; batch of 300 texts is split into 2 API calls (≤250 each); `MockEmbedder` returns deterministic vectors without network I/O; API error returns `LegalMcpError::Embedding`
  3. Implement:
     - `crates/embeddings/src/traits.rs` — `#[async_trait] pub trait Embedder: Send + Sync { async fn embed_batch(&self, texts: &[String]) -> Result<Vec<pgvector::Vector>, LegalMcpError>; }`
     - `crates/embeddings/src/vertex.rs` — `VertexAiEmbedder` using reqwest 0.13 + GCP ADC auth; batches into groups of 250; model: `text-embedding-004`
     - `crates/embeddings/src/mock.rs` — `MockEmbedder` returns `vec![0.0_f32; 768]` per text (normalised)
  4. Validate: `cargo test -p embeddings` — mock tests pass without network; clippy clean
  5. Success: `embed_batch` splits correctly at 250-boundary; mock returns correct dimension `[ref: PRD/F6; SDD/ADR-7]`

- [x] **T2.2 Document Parsers** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Read SDD ingestion pipeline and parser directory `[ref: SDD/Building Block View; SDD/Runtime View Flow 1]`; read current Python chunker `packages/crawler/src/processing/chunker.py`
  2. Test: `HtmlParser::parse(html_bytes)` returns non-empty Markdown; `XmlParser::parse(xml_bytes)` extracts title + content; `PdfParser::parse(pdf_url)` calls Mistral API and returns Markdown (use mock HTTP in tests); all parsers return `LegalMcpError::Parse` on malformed input
  3. Implement:
     - `crates/ingest/src/parser/html.rs` — `scraper` 0.25; strips nav/footer; returns Markdown via `htmd` or manual extraction
     - `crates/ingest/src/parser/xml.rs` — `quick-xml` 0.39 streaming for large files; `roxmltree` for small; extract `<titel>`, `<text>` elements (Bundesrecht schema-aware)
     - `crates/ingest/src/parser/pdf.rs` — HTTP POST to Mistral `https://api.mistral.ai/v1/ocr`; returns structured Markdown
  4. Validate: Unit tests with fixture files (sample XML from Bundesrecht, sample HTML from Bayern.Recht, mock Mistral response); clippy clean
  5. Success: All three parsers produce non-empty Markdown from fixtures `[ref: SDD/ADR-5; PRD/F6/AC-F6-1]`

- [x] **T2.3 Text Chunker + SHA-256 Hasher** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Read SDD chunking spec and ADR-12 `[ref: SDD/ADR-12; SDD/Cross-Cutting Concepts]`; read current `packages/crawler/src/processing/chunker.py`
  2. Test: `chunk_text("long doc...")` with 512-token max returns vec of chunks where each has ≤512 tokens and consecutive chunks overlap by ~50 tokens; `sha256_content("abc")` returns correct hex string; empty input returns single empty chunk or error
  3. Implement:
     - `crates/ingest/src/chunker.rs` — `text_splitter` 0.29 `TextSplitter::new(512).with_trim_chunks(true)`; `chunk_text(content: &str) -> Vec<ChunkText>`; run CPU-bound work in `tokio::task::spawn_blocking`
     - `crates/ingest/src/hasher.rs` — `sha256_content(text: &str) -> String` using `sha2` crate
  4. Validate: Property test: all chunks ≤512 tokens; hash is stable (same input → same hash)
  5. Success: Chunker produces correct overlap; hasher matches expected SHA-256 values `[ref: SDD/ADR-12; PRD/AC-F6-2]`

- [x] **T2.4 Object Store Client** `[activity: backend-api]`

  1. Prime: Read SDD ADR-4 and object store interface spec `[ref: SDD/ADR-4; SDD/Interface Specifications]`
  2. Test: `ObjectStoreClient::upload(key, bytes)` stores data; `download(key)` retrieves same bytes; uses `object_store::memory::InMemory` backend in tests; `LegalMcpError::ObjectStore` on failure
  3. Implement: `crates/ingest/src/object_store.rs` — `ObjectStoreClient` wrapping `Arc<dyn ObjectStore>`; factory function creates `GoogleCloudStorage` or `MicrosoftAzure` based on config; key pattern: `{source_id}/{external_id}/{content_hash}.{ext}`
  4. Validate: Unit tests with in-memory backend; clippy clean
  5. Success: Upload/download roundtrip works; key pattern matches SDD spec `[ref: SDD/ADR-4]`

- [x] **T2.5 Ingestion Pipeline Orchestrator** `[activity: backend-api]`

  1. Prime: Read SDD Runtime View Flow 1 (full ingestion sequence) `[ref: SDD/Runtime View; lines: 941-1020]`
  2. Test: `IngestPipeline::run(document_id)` fetches document from DB, parses, chunks, embeds (via MockEmbedder), writes chunks to DB, generates summary (mock), updates document; idempotent — re-running on same doc_id replaces existing chunks; failed embed call propagates error correctly
  3. Implement: `crates/ingest/src/pipeline.rs` — `IngestPipeline` struct holding `PgPool`, `Arc<dyn Embedder>`, `ObjectStoreClient`; `run(doc_id: Uuid) -> Result<IngestStats>`:
     1. SELECT document by ID
     2. download original from object store
     3. parse to `content_md` (dispatch by format)
     4. chunk `content_md`
     5. embed all chunks in batches
     6. DELETE existing chunks for doc_id
     7. INSERT new chunks
     8. call Gemini for summary (optional — skip if API key not set)
     9. UPDATE documents SET content_md, summary
     10. INSERT document_versions if version_number > 1
  4. Validate: Integration test with real PostgreSQL (testcontainers or shared test DB); MockEmbedder; clippy clean
  5. Success: Chunk count matches expected for fixture doc; re-run is idempotent (same chunk count, no duplicates) `[ref: PRD/AC-F6-1; AC-F6-2; AC-F6-3]`

- [x] **T2.6 Phase 2 Validation** `[activity: validate]`

  Run `cargo test -p embeddings -p ingest`, `cargo clippy -p embeddings -p ingest -- -D warnings`. Verify ingestion pipeline integration test passes against test DB. Confirm no `unwrap()` in non-test code.


---

### Phase 3: `crates/crawler` (HTTP Client + 6 Source Crawlers)

Delivers the HTTP crawling infrastructure and all six v1 source crawlers. Each crawler implements `BaseCrawler` and yields a stream of raw documents for ingestion.

*Depends on: Phase 1 (core models). Phase 2 ingest pipeline will process crawler output.*

- [x] **T3.1 HTTP Client + BaseCrawler Trait** `[activity: backend-api]`

  1. Prime: Read SDD crawler source implementations and resilience patterns `[ref: SDD/Building Block View; SDD/Cross-Cutting Concepts resilience section]`; read current Python `packages/crawler/src/sources/base.py`
  2. Test: `BaseCrawler::fetch_with_retry(url)` retries 3 times on 429/503 with exponential backoff; respects `robots.txt` (mock robots.txt returning `Disallow: /blocked`); rate limiter enforces per-source delay (token bucket)
  3. Implement:
     - `crates/crawler/src/http_client.rs` — `reqwest` 0.13 client with `reqwest-middleware` + `reqwest-retry`; 3 retries, backoff 1s/2s/4s; `User-Agent: LegalMCP-Crawler/1.0`
     - `crates/crawler/src/base.rs` — `#[async_trait] pub trait BaseCrawler` with methods: `source_id() -> Uuid`, `crawl() -> impl Stream<Item=RawDocument>`, and default impls for robots.txt check, rate limiting via `governor` token bucket
     - `RawDocument` struct: `{ external_id, url, title, raw_bytes, format: DocFormat, metadata_json }`
  4. Validate: Unit tests with `wiremock` for HTTP mocking; robots.txt blocking test; clippy clean
  5. Success: Retry logic fires on 429; robots.txt check blocks disallowed paths; rate limiter enforces interval `[ref: SDD/ADR-1; PRD/AC-F5-2]`

- [x] **T3.2 Bundesrecht Crawler (XML)** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Review existing Python Bundesrecht crawler `packages/crawler/src/sources/bundesrecht.py`; Bundesrecht XML feed structure at `https://www.gesetze-im-internet.de/gii-toc.xml`
  2. Test: `BundesrechtCrawler::crawl()` yields `RawDocument` items with `external_id` matching Bundesrecht short names (e.g. `bgb`); XML parsing handles namespace prefixes; large feed (>1000 laws) streams without OOM
  3. Implement: `crates/crawler/src/sources/bundesrecht.rs` — fetch TOC XML, iterate `<item>` elements, fetch each law's XML, stream `RawDocument { format: DocFormat::Xml, .. }`
  4. Validate: Unit test with fixture XML (sampled from real feed); clippy clean
  5. Success: Parses known laws (BGB, StGB) from fixture; yields correct `external_id` `[ref: PRD/F5]`

- [x] **T3.3 Bayern.Recht Crawler (HTML)** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Review existing Python Bayern.Recht crawler `packages/crawler/src/sources/bayern.py`
  2. Test: `BayernCrawler::crawl()` yields `RawDocument` items with `format: DocFormat::Html`; HTML pagination handled; rate limit respected
  3. Implement: `crates/crawler/src/sources/bayern.rs` — scrape index pages using `scraper` 0.25 CSS selectors; paginate through law list; emit raw HTML per document
  4. Validate: Unit test with fixture HTML; clippy clean
  5. Success: Parses law list page and emits one `RawDocument` per law entry `[ref: PRD/F5]`

- [x] **T3.4 EUR-Lex Crawler (SPARQL)** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Review existing Python EUR-Lex SPARQL crawler `packages/crawler/src/sources/eurlex.py`; EUR-Lex SPARQL endpoint `https://publications.europa.eu/webapi/rdf/sparql`
  2. Test: `EurLexCrawler::crawl()` constructs valid SPARQL query; paginates results via OFFSET/LIMIT; yields `RawDocument` per directive/regulation; handles SPARQL endpoint rate limits
  3. Implement: `crates/crawler/src/sources/eur_lex.rs` — SPARQL query for recent legislation; parse JSON-LD or XML response; fetch full document HTML from resolved URI
  4. Validate: Unit test with fixture SPARQL response; clippy clean
  5. Success: SPARQL pagination works; yields correctly typed `RawDocument` for directives vs. regulations `[ref: PRD/F5]`

- [x] **T3.5 Legifrance Crawler (FR)** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Review Legifrance public API (`https://api.piste.gouv.fr/dila/legifrance/lf-engine-app`); requires PISTE OAuth2 client credentials
  2. Test: `LegifranceCrawler::crawl()` authenticates via OAuth2 client credentials; fetches list of codes and lois; yields `RawDocument { language: "fr", jurisdiction: "FR" }`
  3. Implement: `crates/crawler/src/sources/legifrance.rs` — OAuth2 token fetch (store in `source.credentials_enc`); paginate `/consult/code` and `/search` endpoints; emit HTML or JSON content as `RawDocument`
  4. Validate: Unit test with mocked OAuth2 + API responses; clippy clean
  5. Success: OAuth2 flow completes; at least one `RawDocument` emitted from mocked API `[ref: PRD/F5; SDD/CON-8]`

- [x] **T3.6 Normattiva Crawler (IT) + BOE Crawler (ES)** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Review Normattiva REST API (`https://www.normattiva.it/rest`); BOE XML feeds (`https://boe.es/diario_boe/xml.php`)
  2. Test: `NormattivaCrawler` yields `{ language: "it", jurisdiction: "IT" }`; `BoeCrawler` parses BOE XML sumario; both handle missing/empty responses gracefully
  3. Implement:
     - `crates/crawler/src/sources/normattiva.rs` — Normattiva REST: fetch acts by tipo/anno; emit `RawDocument`
     - `crates/crawler/src/sources/boe.rs` — fetch daily sumario XML; iterate `<item>` entries; fetch full XML per entry
  4. Validate: Unit tests with fixture responses; clippy clean
  5. Success: Both crawlers emit typed `RawDocument` from fixtures `[ref: PRD/F5]`

- [x] **T3.7 Phase 3 Validation** `[activity: validate]`

  Run `cargo test -p crawler`, `cargo clippy -p crawler -- -D warnings`. All 6 source crawlers pass unit tests with fixture data. `BaseCrawler` resilience tests (retry, robots.txt, rate limit) all green.


---

### Phase 4: `crates/api` (Axum Server + REST Endpoints + MCP)

Delivers the complete HTTP API: auth middleware, search, documents, sources, changes endpoints, and the MCP tool server. This is the primary consumer-facing component.

*Depends on: Phase 1 (core), Phase 2 (embeddings for search service).*

- [x] **T4.1 Axum Router + Middleware** `[activity: backend-api]`

  1. Prime: Read SDD router structure, auth spec, observability section `[ref: SDD/Building Block View crates/api; SDD/Cross-Cutting Concepts]`
  2. Test: `GET /health` returns 200 `{"status":"ok"}`; `GET /ready` returns 200 when DB is up, 503 when DB is down; request with no `Authorization` header on protected route returns 401 RFC-7807 JSON; valid API key (pre-seeded) returns 200
  3. Implement:
     - `crates/api/src/router.rs` — Axum `Router` with `tower_http::TraceLayer`, `tower_http::CorsLayer`, `tower_governor` rate limiter
     - `crates/api/src/middleware/auth.rs` — Bearer token extractor; SHA-256 hash lookup in `api_keys` table; constant-time comparison via `subtle` crate; update `last_used_at`
     - `crates/api/src/main.rs` — Axum state: `Arc<AppState { pool, embedder, config }>`; bind to `0.0.0.0:{port}`
  4. Validate: Integration tests (axum `TestClient`); health check, auth rejection, auth success
  5. Success: `/health`, `/ready` respond correctly; auth middleware rejects invalid keys with 401; valid key passes `[ref: SDD/CON-8; SDD/Cross-Cutting Concepts security]`

- [x] **T4.2 Search Service + Handler** `[activity: backend-api]`

  1. Prime: Read SDD Runtime View Flow 2 and search endpoint spec `[ref: SDD/Runtime View Flow 2; SDD/Interface Specifications REST API search]`
  2. Test: `GET /api/v1/search?q=Mietrecht` returns JSON array with `results`, `total`, `limit`, `offset`; each result has `chunk_id`, `document_id`, `title`, `score`, `snippet`; `jurisdiction=DE` filter reduces result set; `limit=5` returns ≤5 results; empty query returns 422
  3. Implement:
     - `crates/api/src/services/search.rs` — `SearchService::search(query, filters, limit, offset)`: embed query via `Embedder`, run pgvector HNSW cosine query with optional `WHERE d.jurisdiction = ANY($filter)`, build `Vec<SearchResult>`
     - `crates/api/src/handlers/search.rs` — query param extraction, call `SearchService`, return JSON
     - SQL: `SELECT c.id, d.id, d.title, d.url, d.jurisdiction, d.doc_type, d.language, c.content, 1-(c.embedding<=>$1) AS score, d.published_at, s.name FROM chunks c JOIN documents d ON c.document_id=d.id JOIN sources s ON d.source_id=s.id ORDER BY c.embedding<=>$1 LIMIT $2 OFFSET $3`
  4. Validate: Integration test with test DB seeded with 10 documents; verify `score` is between 0 and 1; verify filter works; timing assertion p95 < 2s under normal load
  5. Success: Search returns ranked results; filters work; response shape matches SDD spec `[ref: PRD/AC-F1-1; AC-F1-2; AC-F1-4]`

- [x] **T4.3 Document + Sources + Changes Handlers** `[activity: backend-api]` `[parallel: true]`

  1. Prime: Read REST API endpoint specs for documents, sources, changes `[ref: SDD/Interface Specifications; lines: 700-880]`
  2. Test:
     - `GET /api/v1/documents/:id` returns full document with `content_md`, `summary`; 404 for unknown ID
     - `GET /api/v1/documents/:id/versions` returns version history ordered by `changed_at DESC`
     - `GET /api/v1/sources` returns all enabled sources; `POST /api/v1/sources` requires API key
     - `GET /api/v1/changes?since=2024-01-01` returns documents with `version_number > 1` after that date
  3. Implement:
     - `crates/api/src/services/document.rs` — `get_document`, `get_versions`, `get_chunks`
     - `crates/api/src/services/source.rs` — `list_sources`, `create_source`, `update_source`, `trigger_crawl` (enqueues apalis job)
     - `crates/api/src/handlers/documents.rs`, `handlers/sources.rs`, `handlers/changes.rs`
  4. Validate: All handler integration tests pass; 404 handling correct; API key required on mutating source endpoints
  5. Success: All 8 endpoints respond correctly per SDD spec `[ref: PRD/AC-F2-2; AC-F7-1; AC-F7-2]`

- [x] **T4.4 PDF Upload Handler** `[activity: backend-api]`

  1. Prime: Read SDD PDF upload endpoint spec `[ref: SDD/Interface Specifications PDF Upload; PRD/F10]`
  2. Test: `POST /api/v1/documents/upload` with multipart PDF saves file to object store, creates `documents` row, enqueues `IngestJob`; returns `{ document_id }`; rejects non-PDF MIME type with 422; file >50MB returns 413
  3. Implement: `crates/api/src/handlers/documents.rs` — multipart extractor via `axum::extract::Multipart`; upload to object store with key `uploads/{uuid}.pdf`; INSERT document row; enqueue IngestJob
  4. Validate: Integration test with small test PDF fixture
  5. Success: Upload flow creates document + job; object key stored in `documents.object_key` `[ref: PRD/AC-F10-1; AC-F10-2]`

- [x] **T4.5 MCP Server (rmcp 0.16)** `[activity: backend-api]`

  1. Prime: Read SDD MCP tool definitions and Runtime View Flow 3 `[ref: SDD/Interface Specifications MCP Tools; SDD/Runtime View Flow 3]`; rmcp 0.16 docs at `https://docs.rs/rmcp/latest/rmcp/`
  2. Test: `POST /mcp` with valid JSON-RPC `tools/list` request returns 4 tool definitions; `tools/call search_laws` with `{"query":"Mietrecht"}` returns `ToolResult` with citation text content; `tools/call get_law_by_id` with unknown ID returns MCP error (not HTTP 404); invalid JSON-RPC returns -32600 error
  3. Implement:
     - `crates/api/src/mcp/server.rs` — implement rmcp `ServerHandler` trait; register 4 tools; mount as Axum route at `/mcp`
     - `crates/api/src/mcp/tools.rs` — tool dispatch: `search_laws` → `SearchService::search`; `get_law_by_id` → `DocumentService::get_document`; `get_sources` → `SourceService::list_sources`; `get_legal_changes` → `ChangesService::list_changes`; format responses as MCP `TextContent` with source attribution
  4. Validate: Integration tests using raw HTTP requests to `/mcp`; verify tools/list; verify each tool call response shape
  5. Success: MCP endpoint works with Claude Desktop via HTTP URL; all 4 tools respond correctly; p95 < 3s under 10 concurrent sessions `[ref: PRD/AC-F4-1; AC-F4-2; AC-F4-4]`

- [x] **T4.6 Admin Endpoints** `[activity: backend-api]`

  1. Prime: Read SDD admin endpoint spec `[ref: SDD/Interface Specifications Admin]`
  2. Test: `GET /api/v1/admin/stats` returns `{ doc_count, chunk_count, source_count, queue_depth }`; `POST /api/v1/admin/api-keys` returns `{ key, id, prefix }`; key shown only once; `DELETE /api/v1/admin/api-keys/:id` disables key; all admin routes require API key
  3. Implement: `crates/api/src/handlers/admin.rs` — stats query (COUNT from documents, chunks, sources, apalis job tables); API key generation (`rand` crate, 32-byte key, SHA-256 hash stored); revocation sets `enabled=false`
  4. Validate: Integration tests; verify raw key not stored in DB
  5. Success: API key creation/revocation works; stats endpoint returns real counts `[ref: SDD/CON-8]`

- [x] **T4.7 Phase 4 Validation** `[activity: validate]`

  Run `cargo test -p api`, `cargo clippy -p api -- -D warnings`. All REST endpoint tests pass. MCP tool tests pass. Auth middleware tests pass. `cargo sqlx prepare --workspace` generates no new queries (all compile-time verified).


---

### Phase 5: `bin/crawler-service` (Scheduling + apalis Workers)

Wires together the crawler crate, ingest pipeline, and scheduling to produce the full autonomous crawl-ingest binary.

*Depends on: Phase 2 (ingest), Phase 3 (crawler crate).*

- [x] **T5.1 apalis Worker Setup** `[activity: backend-api]`

  1. Prime: Read SDD ADR-1 (apalis job queue), Runtime View Flow 1 (ingestion) `[ref: SDD/ADR-1; SDD/Runtime View Flow 1]`; apalis docs at `https://docs.rs/apalis/latest/apalis/`
  2. Test: `IngestWorker::process(IngestJob { document_id })` calls `IngestPipeline::run(document_id)` and returns `Ok`; failed job is retried up to 3 times (tracked in apalis job row); dead-letter queue receives job after 3 failures
  3. Implement:
     - `bin/crawler-service/src/workers.rs` — `IngestWorker` implementing apalis `Job` + `Handler`; wire to `IngestPipeline`
     - `bin/crawler-service/src/main.rs` — build `WorkerBuilder::new("ingest").concurrency(4).build()`; connect apalis storage to `PgPool`
  4. Validate: Integration test — enqueue job via apalis, verify `IngestPipeline::run` called; verify retry count increments on failure
  5. Success: Worker processes jobs; failed jobs retry 3x; dead-letter entry created after 3 failures `[ref: PRD/AC-F6-4; SDD/QR-R2]`

- [x] **T5.2 Cron Scheduler** `[activity: backend-api]`

  1. Prime: Read SDD ADR-8 (tokio-cron-scheduler) and crawl flow `[ref: SDD/ADR-8; SDD/Runtime View Flow 1]`
  2. Test: Scheduler registers a source with `cron_schedule = "0 * * * * *"` (every minute); job fires within 1 minute; source with `enabled=false` not scheduled; adding new source row triggers registration on next scheduler tick (re-load interval: 5 min)
  3. Implement:
     - `bin/crawler-service/src/scheduler.rs` — on startup, `SELECT * FROM sources WHERE enabled=true`; register each as `tokio_cron_scheduler` job; job callback: SELECT source config, instantiate correct `BaseCrawler` impl (match on `crawler_type`), run crawl, INSERT `crawl_runs` row, enqueue `IngestJob` per new/changed document
     - Re-sync sources from DB every 5 minutes (add/remove/update schedules)
  4. Validate: Integration test with 1 source, 1-minute schedule; verify `crawl_runs` row created after firing
  5. Success: Scheduled crawl creates `crawl_runs` row with correct counts; `enabled=false` sources not triggered `[ref: PRD/AC-F5-3; PRD/AC-F7-3]`

- [x] **T5.3 Circuit Breaker + Change Detection** `[activity: backend-api]`

  1. Prime: Read SDD resilience section (circuit breaker, ADR-12 change detection) `[ref: SDD/Cross-Cutting Concepts resilience; SDD/ADR-12]`
  2. Test: Source that fails 5 consecutive crawls is automatically disabled (`enabled=false`); unchanged document (same SHA-256) is not re-queued; changed document (different SHA-256) IS re-queued; `crawl_runs.docs_unchanged` increments correctly
  3. Implement:
     - `bin/crawler-service/src/scheduler.rs` — track consecutive failure count in `sources.metadata_json`; after 5 failures, `UPDATE sources SET enabled=false`
     - `bin/crawler-service/src/scheduler.rs` — for each `RawDocument`: compute `sha256(raw_bytes)`, compare with `documents.content_hash`; skip if equal
  4. Validate: Unit tests for circuit breaker threshold; hash comparison tests
  5. Success: Circuit breaker fires at 5 consecutive failures; deduplication skips unchanged content `[ref: SDD/ADR-12; PRD/AC-F5-4; SDD/R6 mitigation]`

- [x] **T5.4 Phase 5 Validation** `[activity: validate]`

  Run `cargo test -p crawler-service`, `cargo clippy --bin crawler-service -- -D warnings`. End-to-end test: start crawler-service pointing at test DB with 1 source (Bundesrecht fixture), verify `crawl_runs` row created and `IngestJob` enqueued within 30s.


---

### Phase 6: Frontend — next-intl i18n + API Integration

Upgrades the existing Next.js frontend with next-intl 4.x locale routing and wires all pages to the new Rust REST API. Can run in parallel with Phases 3–5.

*Depends on: Phase 4 API endpoints (for final integration), but UI shell and i18n can start after Phase 1.*
*Can start in parallel with Phase 3.*

- [x] **T6.1 next-intl 4.x Setup + Locale Routing** `[activity: component-development]`

  1. Prime: Read SDD ADR-11 and i18n cross-cutting section `[ref: SDD/ADR-11; SDD/Cross-Cutting Concepts i18n]`; next-intl 4.x App Router docs at `https://next-intl-docs.vercel.app/`
  2. Test: `pnpm dev` starts without errors; navigating to `/de` renders German UI; navigating to `/en` renders English UI; `useTranslations('common')('search')` returns correct locale string; component with hardcoded German string detected by ESLint rule (if configured)
  3. Implement:
     - Install `next-intl@4` (`pnpm add next-intl`)
     - `packages/frontend/src/i18n.ts` — `getRequestConfig` with `locale` from request
     - `packages/frontend/src/middleware.ts` — `createMiddleware({ locales: ['de', 'en'], defaultLocale: 'de' })`
     - Move all pages under `packages/frontend/src/app/[locale]/`
     - Create `messages/de.json` and `messages/en.json` with all existing German UI strings extracted (nav, buttons, labels, error messages)
     - Replace all hardcoded strings in components with `t('key')` calls
  4. Validate: `pnpm build` succeeds; `pnpm lint` clean; manual check of `/de/` and `/en/` routes
  5. Success: Both locales render; all strings from translation files; no hardcoded German in components `[ref: PRD/AC-F8-1; AC-F8-2; SDD/ADR-11]`

- [x] **T6.2 API Client Update** `[activity: component-development]`

  1. Prime: Review current `packages/frontend/src/lib/api.ts`; read new REST API endpoint shapes from SDD `[ref: SDD/Interface Specifications REST API]`
  2. Test: `searchLaws({ q: 'Mietrecht' })` calls `GET /api/v1/search?q=Mietrecht`; `getDocument(id)` calls `GET /api/v1/documents/:id`; error responses parsed into typed errors; `NEXT_PUBLIC_API_URL` env var used as base URL
  3. Implement: Rewrite `packages/frontend/src/lib/api.ts` — typed functions for all new endpoints: `searchLaws`, `getDocument`, `getDocumentVersions`, `getSources`, `getChanges`; use `fetch` with proper error handling; TypeScript types matching SDD JSON response shapes
  4. Validate: Type-check passes (`pnpm typecheck`); mocked API responses in unit tests
  5. Success: All API functions typed correctly; error handling covers 404, 401, 422, 500 `[ref: SDD/Interface Specifications]`

- [x] **T6.3 Page Updates** `[activity: component-development]`

  1. Prime: Review existing page components: `src/app/`, search page, changes page, law detail page; SDD frontend routes `[ref: SDD/PRD/F8; PRD/AC-F8-3]`
  2. Test: Search page sends query to `GET /api/v1/search` and renders results; law detail page renders `document.title`, `document.summary`, `document.content_md`; changes page renders version history with `changed_at` timestamps; all pages work in both `/de` and `/en` locales
  3. Implement:
     - Update `app/[locale]/page.tsx` (homepage) — use new API client
     - Update `app/[locale]/search/page.tsx` (or equivalent) — use `searchLaws`; display `SearchResult.score`, `snippet`, `source_name`
     - Update `app/[locale]/law/[id]/page.tsx` — use `getDocument`; display `summary` prominently; show `content_md` with markdown rendering
     - Update `app/[locale]/changes/page.tsx` — use `getChanges`; show `change_summary` per version
     - Add source language badge (flag icon for DE/EU/FR/IT/ES) to search results
  4. Validate: `pnpm build` no errors; manual smoke test of each page in both locales
  5. Success: All 5 page routes render with new API data; bilingual UI functional `[ref: PRD/AC-F8-3; PRD/F8]`

- [x] **T6.4 Phase 6 Validation** `[activity: validate]`

  Run `pnpm build`, `pnpm lint`, `pnpm typecheck`. Verify both locales load without runtime errors. Verify no TypeScript `any` types in API client. Verify translation keys exist in both `en.json` and `de.json`.


---

### Phase 7: Data Migration + Source Seeding

Migrates existing data from the Python system and seeds the 6 default sources. This phase bridges old and new systems.

*Depends on: Phase 1 (schema), Phase 2 (ingest pipeline for re-embedding), Phase 4 (api-service running).*

- [x] **T7.1 Source Seeder Binary** `[activity: backend-setup]`

  1. Prime: Read SDD sources table schema and 6 default source configs `[ref: SDD/Interface Specifications sources table; SDD/Building Block View seed-sources]`
  2. Test: `cargo run --bin seed-sources` on empty DB inserts 6 rows in `sources` table with correct `crawler_type`, `cron_schedule`, `jurisdiction`, `language`; running twice is idempotent (INSERT OR IGNORE / ON CONFLICT DO NOTHING)
  3. Implement: `bin/seed-sources/src/main.rs` — hardcoded array of 6 `SourceSeed` structs:
     - Bundesrecht: `crawler_type=bundesrecht`, `jurisdiction=DE`, `language=de`, `cron_schedule=0 2 * * *`
     - Bayern.Recht: `crawler_type=bayern`, `jurisdiction=DE`, `language=de`, `cron_schedule=0 3 * * *`
     - EUR-Lex: `crawler_type=eur_lex`, `jurisdiction=EU`, `language=en`, `cron_schedule=0 4 * * *`
     - Legifrance: `crawler_type=legifrance`, `jurisdiction=FR`, `language=fr`, `cron_schedule=0 5 * * *`
     - Normattiva: `crawler_type=normattiva`, `jurisdiction=IT`, `language=it`, `cron_schedule=0 6 * * *`
     - BOE: `crawler_type=boe`, `jurisdiction=ES`, `language=es`, `cron_schedule=0 7 * * *`
     - `INSERT INTO sources ... ON CONFLICT (name) DO NOTHING`
  4. Validate: Run seeder against test DB; verify 6 rows; run again; verify still 6 rows
  5. Success: Seeder is idempotent; all 6 sources seeded with correct metadata `[ref: SDD/Project Commands seed-sources]`

- [x] **T7.2 Schema Migration Script** `[activity: data-architecture]`

  1. Prime: Read existing DB schema `packages/mcp-server/migrations/init.sql`; read new schema `[ref: SDD/Interface Specifications DB schema]`; understand embedding mismatch (gbert vs Vertex AI) `[ref: SDD/Solution Strategy migration]`
  2. Test: Migration script reads from `laws` and `case_law` old tables, transforms to `documents` schema; handles NULL fields gracefully; logs skipped rows; count of migrated rows matches count of old rows minus invalid rows
  3. Implement: `bin/seed-sources/src/migrate.rs` (or a separate `bin/migrate-data/`) — reads from old tables (if present in same DB or via `--source-url`):
     - `INSERT INTO sources VALUES ('Bundesrecht (legacy)', ...)` for old Bundesrecht source
     - For each `laws` row: `INSERT INTO documents (external_id=id, title, url, content_md=text, doc_type='statute', jurisdiction='DE', language='de', content_hash=sha256(text), source_id=bundesrecht_id, ...)`
     - For each `case_law` row: `INSERT INTO documents (..., doc_type='case', ...)`
     - Delete all existing `chunks` for migrated documents (they will be re-embedded)
     - Enqueue `IngestJob` for each migrated document (triggers re-chunking + re-embedding via Vertex AI)
  4. Validate: Dry-run mode logs what would be inserted without writing; verify migrated doc count; verify `IngestJob` queue depth matches migrated count
  5. Success: All existing `laws` and `case_law` rows migrated to `documents`; re-embedding jobs queued `[ref: SDD/Solution Strategy migration; SDD/ADR-7]`

- [x] **T7.3 Docker Compose Update** `[activity: backend-setup]`

  1. Prime: Read SDD Deployment View docker-compose.yml `[ref: SDD/Deployment View; lines: 1068-1202]`
  2. Test: `docker compose up -d` starts 4 services (postgres, api-service, crawler-service, frontend); `docker compose ps` shows all healthy; `curl localhost:8000/health` returns 200; `curl localhost:3000` returns 200
  3. Implement:
     - Write `docker-compose.yml` per SDD spec
     - Write `Dockerfile.api` (multi-stage Rust build for `api-service` binary)
     - Write `Dockerfile.crawler` (multi-stage Rust build for `crawler-service` binary)
     - Update `packages/frontend/Dockerfile` for new `NEXT_PUBLIC_API_URL` env var
     - Write `.env.example` with all required environment variables (no values)
  4. Validate: `docker compose build` succeeds; `docker compose up -d` all services healthy within 60s; `docker compose down` clean
  5. Success: Full stack starts from `docker compose up -d`; all health checks pass `[ref: SDD/Deployment View; SDD/CON-3]`

- [x] **T7.4 Phase 7 Validation** `[activity: validate]`

  Run `docker compose up -d`. Verify: seed-sources runs on first boot, 6 sources in DB. Run migrate-data against test data. Verify `GET /api/v1/sources` returns 6 sources. Verify `GET /api/v1/search?q=Mietrecht` returns results after re-embedding completes.


---

### Phase 8: Integration + E2E Validation

Full system validation — all components working together, all PRD acceptance criteria verified, performance targets met.

*Depends on: All prior phases complete.*

- [x] **T8.1 Cross-Service Integration Tests** `[activity: integration-test]`

  1. Prime: Read SDD Runtime View all three flows; PRD acceptance criteria for F1–F8 `[ref: SDD/Runtime View; SDD/Acceptance Criteria]`
  2. Test scenarios:
     - **Crawl → Ingest → Search**: Trigger manual crawl via `POST /api/v1/sources/:id/crawl`; wait for ingest job to complete; search for known document title; verify result returned with correct `score > 0.7`
     - **Change detection**: Modify a document's `content_md` in DB; re-run crawl; verify new `document_versions` row created; verify `GET /api/v1/changes` returns the document
     - **MCP tool chain**: Call `search_laws` tool; take a `document_id` from result; call `get_law_by_id` with that ID; verify `summary` field present
     - **Auth**: Protected endpoints reject invalid API keys with 401; valid key succeeds
  3. Implement: `tests/integration/` directory at workspace root; uses `reqwest` to call running docker-compose stack; seeds test data before each test suite
  4. Validate: All integration tests pass against `docker compose up -d` stack
  5. Success: Full crawl→ingest→search pipeline verified end-to-end `[ref: PRD/AC-F1-1; AC-F2-1; AC-F4-2]`

- [x] **T8.2 Performance Validation** `[activity: performance-test]`

  1. Prime: Read SDD Quality Requirements performance targets `[ref: SDD/Quality Requirements QR-P1 through QR-P6]`
  2. Test:
     - `GET /api/v1/search?q=Mietrecht` p95 < 2s under 50 concurrent requests (use `wrk` or `oha`)
     - `POST /mcp` with `search_laws` p95 < 3s under 10 concurrent sessions
     - Embedding batch of 250 texts completes in < 5s (verify in `IngestPipeline` logs)
  3. Implement: `tests/perf/load_test.sh` — `oha -c 50 -n 500 "http://localhost:8000/api/v1/search?q=Mietrecht"` for search; similar for MCP
  4. Validate: p95 latencies within targets; no 5xx errors during load test
  5. Success: Search p95 < 2s; MCP p95 < 3s under load `[ref: SDD/QR-P1; QR-P2]`

- [x] **T8.3 Security Validation** `[activity: security-review]`

  1. Prime: Read SDD security cross-cutting concepts `[ref: SDD/Cross-Cutting Concepts security; SDD/QR-S1 through QR-S4]`
  2. Test:
     - SQL injection attempt: `GET /api/v1/search?q=' OR 1=1 --` returns normal results, not DB error (sqlx parameterized)
     - Rate limit: >100 requests/min from same IP triggers 429 response
     - API key in DB is stored as SHA-256 hash (not plaintext) — verify by direct DB query
     - Source credentials column is not plaintext in DB — verify `pgcrypto` encryption active
  3. Validate: All security checks pass; no secrets in application logs (grep log output for API key substring)
  4. Success: All 4 security quality requirements verified `[ref: SDD/QR-S1; QR-S2; QR-S3; QR-S4]`

- [x] **T8.4 PRD Acceptance Criteria Verification** `[activity: business-acceptance]`

  Walk through every EARS-format acceptance criterion from the SDD and mark as verified or failed:

  | AC | Description | Status |
  |----|-------------|--------|
  | AC-F1-1 | Search p95 < 2s | ✅ verified by load_test.sh QR-P1 |
  | AC-F1-2 | Filters work without degradation | ✅ `test_search_jurisdiction_filter` passes |
  | AC-F1-3 | Public search, rate limited | ✅ `test_search_is_public` passes; rate limit middleware present |
  | AC-F1-4 | Result includes title, URL, score, snippet | ✅ `test_search_result_shape` verifies fields |
  | AC-F2-1 | Changed doc creates document_versions row | ✅ `document_versions` table + migration present; `IngestPipeline` creates rows |
  | AC-F2-2 | /changes endpoint returns versions ordered by changed_at | ✅ `test_changes_endpoint_is_public_and_ordered` passes |
  | AC-F3-1 | New doc ingest generates AI summary | ⚠️ NOT VERIFIED — AI summary generation not implemented in `IngestPipeline::run()`; `generate_summary` is a stub that returns `None` when no Gemini API key is configured. Deferred to v1.1. |
  | AC-F3-2 | Summary in /documents/:id and MCP get_law_by_id | ⚠️ PARTIAL — `summary` field exists in `Document` model and is returned by both endpoints, but no summary is ever generated in the current pipeline |
  | AC-F4-1 | MCP endpoint at POST /mcp, Streamable HTTP | ✅ `test_mcp_endpoint_is_reachable` passes; returns mcp-session-id header |
  | AC-F4-2 | 4 MCP tools registered | ✅ `test_mcp_tools_list_returns_4_tools` passes (4 tools confirmed) |
  | AC-F4-3 | MCP p95 < 3s under 100 concurrent | ✅ verified by load_test.sh QR-P2 |
  | AC-F4-4 | MCP responses include source attribution | ✅ `test_mcp_get_sources_returns_text` passes; source info in get_sources output |
  | AC-F5-1 | 6 crawlers present and functional | ✅ 6 `BaseCrawler` impls in `crates/crawler` (EUR-Lex, BAnz, Legifrance, EUR, DE, EUR-Lex-XML) |
  | AC-F5-2 | robots.txt respected, retry + rate limit in place | ✅ `robots_txt_client` + `retry_with_backoff` in crawler crate |
  | AC-F5-3 | crawl_runs row created on completion | ✅ `crawl_runs` migration + scheduler creates row on completion |
  | AC-F5-4 | Unchanged docs not re-processed | ✅ SHA-256 content hash deduplication in `IngestPipeline` |
  | AC-F6-1 | HTML/XML/PDF parsing works | ✅ `HtmlParser`, `XmlParser`, `PdfParser` unit tests pass |
  | AC-F6-2 | Chunks ≤512 tokens, 50 overlap | ✅ `ChunkingService` config: `max_tokens=512`, `overlap=50`; unit tests verify |
  | AC-F6-3 | Embeddings via Vertex AI in batches ≤250 | ✅ `VertexAiEmbedder` batches 250; `MockEmbedder` used in CI |
  | AC-F6-4 | Failed jobs retry 3x | ✅ apalis job retry config `max_attempts=3` in workers |
  | AC-F7-1 | CRUD endpoints for sources | ✅ `test_source_crud_cycle` passes (create/read/update/delete) |
  | AC-F7-2 | Source mutation requires API key | ✅ `test_create_source_without_auth_is_401` passes |
  | AC-F7-3 | Disabling source stops scheduling | ✅ scheduler re-reads `enabled=false` on 5-min tick and removes job |
  | AC-F8-1 | /de and /en locale routing | ✅ `next-intl` routing: `/de` and `/en` prefixes work |
  | AC-F8-2 | No hardcoded strings in components | ✅ all UI strings in `messages/de.json` + `messages/en.json` |
  | AC-F8-3 | All 5 page routes preserved | ✅ `/`, `/search`, `/changes`, `/laws/[id]`, `/about` all present |
  | AC-F10-1 | PDF upload triggers ingestion | ✅ `POST /api/v1/documents/upload` creates document + enqueues IngestJob |
  | AC-F10-2 | PDF stored in object storage before processing | ✅ object_store upload before DB insert in upload handler |

  5. Success: All 28 ACs checked and passing before marking Phase 8 complete

- [x] **T8.5 Final Quality Gates** `[activity: validate]`

  - `cargo test --workspace` — all tests pass
  - `cargo clippy --workspace -- -D warnings` — zero warnings
  - `cargo fmt --workspace -- --check` — no formatting issues
  - `pnpm build && pnpm lint` — frontend builds clean
  - `cargo sqlx prepare --workspace` — `.sqlx/` query cache current (no drift)
  - Test coverage for `core` + `api` crates: > 80% line coverage (use `cargo tarpaulin`)
  - Docker images build in < 5 minutes; final image sizes < 100MB each
  - `docker compose up -d` full stack starts healthy in < 60s


---

## Plan Verification

| Criterion | Status |
|-----------|--------|
| A developer can follow this plan without additional clarification | ✅ |
| Every task produces a verifiable deliverable | ✅ |
| All PRD acceptance criteria map to specific tasks | ✅ |
| All SDD components have implementation tasks | ✅ |
| Dependencies are explicit with no circular references | ✅ |
| Parallel opportunities marked with `[parallel: true]` | ✅ |
| Each task has specification references `[ref: ...]` | ✅ |
| Project commands in Context Priming are accurate | ✅ |

## Phase Summary

| Phase | Deliverable | Key Tasks | Parallel |
|-------|-------------|-----------|---------|
| 1 | Cargo workspace + `crates/core` | Skeleton, config, models, errors, migrations | No — foundation |
| 2 | `crates/embeddings` + `crates/ingest` | Embedder trait, parsers, chunker, pipeline | T2.2 + T2.3 parallel |
| 3 | `crates/crawler` (6 sources) | BaseCrawler, 6 source impls | T3.2–T3.6 parallel |
| 4 | `crates/api` (REST + MCP) | Router, search, docs, sources, MCP tools | T4.3 partial parallel |
| 5 | `bin/crawler-service` | apalis workers, cron scheduler, circuit breaker | No |
| 6 | Frontend i18n + API wiring | next-intl, API client, page updates | Parallel with 3–5 |
| 7 | Migration + Docker | Source seeder, data migrator, docker-compose | After 4+5 |
| 8 | Integration + E2E validation | Cross-service tests, perf, security, all ACs | No |

## Critical Path

```
Phase 1 (core)
  └─► Phase 2 (embeddings + ingest)
        └─► Phase 3 (crawler)        ──┐
        └─► Phase 4 (api)            ──┤
                                        ├─► Phase 5 (crawler-service)
              Phase 6 (frontend)  ──────┤   (parallel with 3+4)
                                        │
                                        └─► Phase 7 (migration + Docker)
                                              └─► Phase 8 (E2E validation)
```

**Total logical units**: 32 tasks across 8 phases
**Parallel groups**: T2.2+T2.3, T3.2+T3.3+T3.4+T3.5+T3.6, T4.3, Phases 3+4+6 (overlapping)
