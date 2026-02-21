---
title: "Global Legal Crawler & MCP RAG System"
status: completed
version: "1.0"
---

# Solution Design Document

## Validation Checklist

### CRITICAL GATES (Must Pass)

- [x] All required sections are complete
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Architecture pattern is clearly stated with rationale
- [x] All architecture decisions confirmed by user
- [x] Every interface has specification

### QUALITY CHECKS (Should Pass)

- [x] All context sources are listed with relevance ratings
- [x] Project commands are discovered from actual project files
- [x] Constraints → Strategy → Design → Implementation path is logical
- [x] Every component in diagram has directory mapping
- [x] Error handling covers all error types
- [x] Quality requirements are specific and measurable
- [x] Component names consistent across diagrams
- [x] A developer could implement from this design

---

## Constraints

CON-1 **Language**: Backend in Rust (Axum 0.8). Frontend in Next.js 14+ (TypeScript, shadcn/ui). No Python in the new system.

CON-2 **Database**: PostgreSQL 16 + pgvector. HNSW indexes. sqlx 0.8 for queries. No ORM abstraction layer.

CON-3 **Deployment**: Docker-based. Target GCP (Cloud Run + GCS) or Azure (Container Apps + Blob Storage). EU region. No Kubernetes for v1.

CON-4 **Team**: 1–3 engineers. Architecture must minimize operational complexity. One shared PostgreSQL database — no distributed systems overhead.

CON-5 **Open source**: No proprietary runtime dependencies that prevent community self-hosting. API keys and secrets via environment variables or Vault — no hardcoded secrets.

CON-6 **Performance**: Search p95 < 2s (F1 AC), MCP p95 < 3s under 100 concurrent sessions (F4 AC). Embedding pipeline must batch API calls — no sequential single-call pattern.

CON-7 **i18n**: Bilingual EN+DE from day one. next-intl 4.x, not retrofitted. All UI strings in translation files.

CON-8 **Security**: Source credentials encrypted at rest (pgcrypto or secrets manager). MCP endpoint open (no auth) for v1. REST API uses API key auth (Bearer token, hashed in DB).


---

## Implementation Context

### Required Context Sources

#### Documentation Context
```yaml
- doc: docs/specs/001-global-legal-crawler-mcp-rag/product-requirements.md
  relevance: CRITICAL
  why: "All features F1-F14 and acceptance criteria drive this design"

- doc: packages/mcp-server/migrations/init.sql
  relevance: HIGH
  why: "Current schema being replaced — informs migration decisions"

- url: https://docs.rs/rmcp/latest/rmcp/
  relevance: HIGH
  why: "Official Rust MCP SDK — transport and tool registration patterns"

- url: https://docs.rs/sqlx/latest/sqlx/
  relevance: HIGH
  why: "Primary database access layer — query macro patterns, migration runner"

- url: https://docs.rs/pgvector/latest/pgvector/
  relevance: HIGH
  why: "pgvector Rust type — Vector type registration, HNSW index creation"

- url: https://modelcontextprotocol.io/specification
  relevance: HIGH
  why: "MCP spec — Streamable HTTP transport, tool definitions, JSON-RPC 2.0"

- url: https://docs.rs/apalis/latest/apalis/
  relevance: MEDIUM
  why: "Async job queue — ingestion pipeline task definitions"

- url: https://docs.rs/object_store/latest/object_store/
  relevance: MEDIUM
  why: "Cloud object storage abstraction — GCS and Azure Blob"

- url: https://next-intl-docs.vercel.app/
  relevance: MEDIUM
  why: "next-intl App Router patterns — locale routing, RSC message loading"
```

#### Code Context
```yaml
- file: packages/mcp-server/migrations/init.sql
  relevance: HIGH
  why: "7 existing tables — laws, law_chunks, case_law, case_law_chunks, crawl_runs, api_keys, related_laws"

- file: packages/crawler/src/sources/base.py
  relevance: HIGH
  why: "BaseCrawler abstraction — rate limiting, retry, robots.txt patterns to replicate in Rust"

- file: packages/crawler/src/processing/chunker.py
  relevance: HIGH
  why: "Chunking strategy — token-based (tiktoken, 512 tokens, 50 overlap) to replicate with text-splitter"

- file: packages/mcp-server/src/mcp/tools.py
  relevance: HIGH
  why: "5 existing MCP tools — search_laws, get_law_by_id, search_case_law, get_legal_changes, get_related_laws"

- file: packages/frontend/src/lib/api.ts
  relevance: MEDIUM
  why: "Current API client — endpoint shapes the new REST API must be compatible with or replace"

- file: docker-compose.yml
  relevance: MEDIUM
  why: "Current service topology — postgres, mcp-server, crawler, frontend (4 services)"
```

#### External APIs
```yaml
- service: Google Vertex AI (text-embedding-004)
  doc: https://cloud.google.com/vertex-ai/generative-ai/docs/embeddings/get-text-embeddings
  relevance: HIGH
  why: "Primary embedding provider. 768-dim multilingual. Batch endpoint supports up to 250 texts per call."

- service: Google Vertex AI (Gemini 2.5 Pro)
  doc: https://cloud.google.com/vertex-ai/generative-ai/docs/model-reference/gemini
  relevance: HIGH
  why: "LLM for AI summary generation. Used in F3 document provenance."

- service: Mistral Document AI
  doc: https://docs.mistral.ai/capabilities/document/
  relevance: MEDIUM
  why: "PDF extraction for F10 PDF Upload and OCR. mistral-ocr-latest returns structured markdown."
```

### Implementation Boundaries

- **Must Preserve**: Existing crawl data in PostgreSQL (migration path from current schema required). Frontend look and feel (shadcn/ui component library stays).
- **Can Modify**: All backend code (full Rust rewrite). Database schema (with migration scripts). Docker Compose topology.
- **Must Not Touch**: PostgreSQL data during active migration phase (read-only until cutover).

### External Interfaces

#### System Context Diagram

```mermaid
graph TB
    LP[Legal Professional]
    CO[Compliance Officer]
    DEV[Developer / Integrator]
    AI[LLM Client - Claude/ChatGPT/Cursor]
    ADM[System Administrator]

    subgraph "Legal MCP Platform"
        FE[Frontend - Next.js]
        API[API Service - Rust/Axum]
        CRW[Crawler Service - Rust]
        DB[(PostgreSQL + pgvector)]
        OBJ[(Object Storage - GCS/Azure)]
    end

    GOV[Government Legal Portals]
    VERTEX[Google Vertex AI]
    MISTRAL[Mistral Document AI]

    LP --> FE
    CO --> FE
    ADM --> FE
    DEV --> API
    AI --> API

    FE --> API
    API --> DB
    API --> OBJ
    API --> VERTEX

    CRW --> GOV
    CRW --> DB
    CRW --> OBJ
    CRW --> VERTEX
    CRW --> MISTRAL
```

#### Interface Specifications

```yaml
inbound:
  - name: "Browser (Frontend)"
    type: HTTPS
    format: REST JSON
    authentication: Session cookie (admin routes) / none (public search)
    data_flow: "Search queries, document views, source management"

  - name: "MCP Client (LLM)"
    type: HTTP
    format: JSON-RPC 2.0 (MCP Streamable HTTP)
    authentication: None (v1 — open access)
    path: POST /mcp
    data_flow: "Tool calls: search_laws, get_law_by_id, get_sources, get_legal_changes"

  - name: "REST API Consumer (Developer)"
    type: HTTPS
    format: REST JSON
    authentication: Bearer API key (hashed in DB)
    path: /api/v1/*
    data_flow: "Search, document retrieval, source listing, change tracking"

outbound:
  - name: "Government Legal Portals"
    type: HTTPS
    format: HTML / XML / SPARQL
    authentication: Per-source (none, API key, or login — stored encrypted)
    data_flow: "Legal document discovery and fetching"
    criticality: HIGH

  - name: "Vertex AI Embedding"
    type: HTTPS
    format: REST JSON
    authentication: GCP Service Account (ADC)
    data_flow: "Batch text → vector embeddings (768-dim)"
    criticality: HIGH

  - name: "Vertex AI Gemini"
    type: HTTPS
    format: REST JSON
    authentication: GCP Service Account (ADC)
    data_flow: "Document text → 2-3 sentence AI summary"
    criticality: MEDIUM

  - name: "Mistral Document AI"
    type: HTTPS
    format: REST JSON
    authentication: API Key (env var)
    data_flow: "PDF file → structured markdown"
    criticality: MEDIUM

data:
  - name: "PostgreSQL + pgvector"
    type: PostgreSQL 16
    connection: sqlx connection pool (max 20 per service)
    data_flow: "All application state — sources, documents, chunks, embeddings, jobs, API keys"

  - name: "Object Storage"
    type: GCS or Azure Blob (via object_store)
    connection: HTTP SDK
    data_flow: "Original document files (PDF, HTML, XML snapshots)"

  - name: "apalis Job Queue"
    type: PostgreSQL-backed (apalis tables in same DB)
    connection: apalis worker pool
    data_flow: "Ingestion pipeline tasks: parse, embed, index"
```

### Cross-Component Boundaries

- **API Contract**: `/api/v1/` endpoints are public contracts. Breaking changes require version bump to `/api/v2/`.
- **MCP Contract**: MCP tool names (`search_laws`, `get_law_by_id`, `get_sources`, `get_legal_changes`) are stable. New tools additive only.
- **Shared Database**: Both services share the same PostgreSQL instance. The `core` crate owns all schema definitions and migration files. Neither service may bypass the `core` crate's types.
- **No Direct Service-to-Service HTTP**: Crawler and API do not call each other over HTTP. All communication is via the shared database and apalis job queue tables.

### Project Commands

```bash
# Rust workspace (new system)
Build:    cargo build --workspace
Test:     cargo test --workspace
Lint:     cargo clippy --workspace -- -D warnings
Format:   cargo fmt --workspace
Migrate:  cargo sqlx migrate run --database-url $DATABASE_URL
Check:    cargo sqlx prepare --workspace  # compile-time query verification

# Frontend
Install:  pnpm install
Dev:      pnpm dev
Build:    pnpm build
Lint:     pnpm lint
Test:     pnpm test

# Docker (development)
Up:       docker compose up -d
Down:     docker compose down
Logs:     docker compose logs -f

# Database
Seed:     cargo run --bin seed-sources  # loads 6 default source configs
```


---

## Solution Strategy

### Architecture Pattern: Modular Monorepo with Two Binaries

The system is a **Rust Cargo workspace** with five internal crates sharing a single PostgreSQL database. Two binaries are deployed as separate containers:

| Binary | Crates Used | Responsibility |
|--------|------------|----------------|
| `api-service` | core, embeddings, api | REST API + MCP endpoint |
| `crawler-service` | core, embeddings, ingest, crawler | Crawl scheduling + ingestion pipeline |

This is **not** microservices. Both binaries share the same database schema (defined in `crates/core`), communicate via database tables and apalis job queue rows — never via HTTP. This minimises operational complexity for a 1–3 engineer team (CON-4) while allowing independent scaling of crawling vs. serving workloads.

### Key Technology Decisions

| Concern | Choice | Rationale |
|---------|--------|-----------|
| Backend language | Rust + Axum 0.8 | Performance, memory safety, single binary deployment |
| Vector search | pgvector HNSW | Avoid separate vector DB; SQL joins on metadata |
| Async job queue | apalis (PostgreSQL) | No Redis/RabbitMQ; same DB connection pool |
| Embedding | Vertex AI text-embedding-004 | 768-dim multilingual; fixes current gbert/Vertex mismatch |
| PDF extraction | Mistral Document AI | Native Rust PDF libs too immature for production |
| MCP transport | rmcp 0.16 Streamable HTTP | Official SDK; replaces broken 501 stdio endpoint |
| Object storage | object_store (GCS/Azure) | Original files preserved; crawl deduplication via SHA-256 |
| Scheduling | tokio-cron-scheduler 0.15 | PostgreSQL persistence; cron syntax |
| Frontend i18n | next-intl 4.x | App Router RSC support; EN+DE from day one |

### Migration Strategy

The existing Python system remains live during the build phase. Cutover is a hard switch:

1. Build and test new system in parallel with existing system running.
2. Run `migrate-schema` script: transforms existing `laws` + `case_law` rows into new `documents` + `chunks` tables (re-embedding required due to gbert→Vertex AI switch).
3. Deploy new Docker Compose services alongside old ones.
4. Smoke-test new system against production data.
5. DNS/reverse proxy cutover — old services stopped.

Re-embedding all existing chunks is unavoidable: the current system stores gbert embeddings (incompatible with Vertex AI text-embedding-004 vectors). This is a one-time migration cost.

### Simplicity Over Abstraction

- **No ORM**: sqlx compile-time checked queries only. No Diesel, no SeaORM.
- **No service mesh**: Direct database connection sharing.
- **No event bus**: apalis job tables are the only async communication channel.
- **No GraphQL**: REST + MCP is sufficient for all personas.

---

## Building Block View

### Level 1: System Components

```mermaid
graph TB
    subgraph "api-service binary"
        AXUM[Axum Router]
        REST[REST Handlers<br/>/api/v1/*]
        MCP_H[MCP Handler<br/>/mcp]
        AUTH[Auth Middleware<br/>API key validation]
    end

    subgraph "crawler-service binary"
        SCHED[Scheduler<br/>tokio-cron-scheduler]
        WORKER[apalis Workers<br/>Job processors]
        CRAWLERS[Source Crawlers<br/>Bundesrecht / Bayern / EUR-Lex<br/>Legifrance / Normattiva / BOE]
        INGEST[Ingestion Pipeline<br/>parse → chunk → embed → store]
    end

    subgraph "crates/core"
        MODELS[Domain Models<br/>Source, Document, Chunk, Job]
        MIGRATIONS[SQL Migrations<br/>sqlx migrate]
        ERRORS[Error Types<br/>LegalMcpError]
        CONFIG[Config<br/>AppConfig, Secrets]
    end

    subgraph "crates/embeddings"
        EMBED_T[Embedder Trait]
        VERTEX[VertexAiEmbedder]
        MOCK_E[MockEmbedder<br/>tests only]
    end

    subgraph "crates/ingest"
        PARSER[Document Parser<br/>HTML / XML / PDF]
        CHUNKER[Text Chunker<br/>text-splitter 0.29]
        HASHER[Change Detector<br/>SHA-256]
        OBJ[Object Store Client<br/>object_store]
    end

    subgraph "crates/crawler"
        BASE[BaseCrawler Trait<br/>rate limit / retry / robots.txt]
        SOURCES[Source Implementations<br/>6 crawlers]
        HTTP_C[HTTP Client<br/>reqwest 0.13 + middleware]
    end

    subgraph "crates/api"
        SEARCH[Search Service]
        DOC_SVC[Document Service]
        SRC_SVC[Source Service]
        MCP_SVC[MCP Tool Impl]
    end

    subgraph "Infrastructure"
        PG[(PostgreSQL 16<br/>+ pgvector)]
        APALIS[(apalis job tables<br/>in PostgreSQL)]
        OBJ_STORE[(GCS / Azure Blob)]
        VERTEX_AI[Vertex AI<br/>text-embedding-004<br/>Gemini 2.5 Pro]
        MISTRAL[Mistral<br/>Document AI]
    end

    AXUM --> REST
    AXUM --> MCP_H
    AXUM --> AUTH

    REST --> SEARCH
    REST --> DOC_SVC
    REST --> SRC_SVC
    MCP_H --> MCP_SVC
    MCP_SVC --> SEARCH
    MCP_SVC --> DOC_SVC

    SCHED --> WORKER
    WORKER --> CRAWLERS
    CRAWLERS --> INGEST

    SEARCH --> PG
    DOC_SVC --> PG
    SRC_SVC --> PG
    INGEST --> PG
    INGEST --> APALIS
    INGEST --> OBJ_STORE
    INGEST --> VERTEX_AI
    INGEST --> MISTRAL

    EMBED_T --> VERTEX
    INGEST --> EMBED_T
    SEARCH --> EMBED_T
```

### Level 2: Cargo Workspace Directory Map

```
legalmcp/
├── Cargo.toml                    # workspace root
├── Cargo.lock
├── crates/
│   ├── core/                     # Shared domain models, config, errors, migrations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config.rs         # AppConfig, Secrets (config 0.15 + dotenvy)
│   │   │   ├── errors.rs         # LegalMcpError enum (thiserror)
│   │   │   ├── models/
│   │   │   │   ├── source.rs     # Source, SourceConfig
│   │   │   │   ├── document.rs   # Document, DocumentVersion
│   │   │   │   ├── chunk.rs      # Chunk (with pgvector::Vector field)
│   │   │   │   ├── job.rs        # CrawlJob, IngestJob
│   │   │   │   └── api_key.rs    # ApiKey (hashed)
│   │   │   └── db.rs             # PgPool factory, migration runner
│   │   └── migrations/           # All sqlx migration files (authoritative)
│   │       ├── 0001_initial.sql
│   │       ├── 0002_document_versions.sql
│   │       └── ...
│   │
│   ├── embeddings/               # Embedding abstraction + providers
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs         # Embedder trait: embed_batch(texts) -> Vec<Vector>
│   │       ├── vertex.rs         # VertexAiEmbedder (text-embedding-004)
│   │       └── mock.rs           # MockEmbedder for tests
│   │
│   ├── ingest/                   # Ingestion pipeline steps
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pipeline.rs       # IngestPipeline orchestrator
│   │       ├── parser/
│   │       │   ├── html.rs       # scraper 0.25
│   │       │   ├── xml.rs        # quick-xml 0.39 / roxmltree
│   │       │   └── pdf.rs        # Mistral Document AI client
│   │       ├── chunker.rs        # text-splitter (512 tokens, 50 overlap)
│   │       ├── hasher.rs         # SHA-256 content hash
│   │       └── object_store.rs   # Upload/download originals
│   │
│   ├── crawler/                  # HTTP crawlers per source
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── base.rs           # BaseCrawler trait (rate limit, retry, robots.txt)
│   │       ├── http_client.rs    # reqwest 0.13 + reqwest-middleware + reqwest-retry
│   │       └── sources/
│   │           ├── bundesrecht.rs    # XML feed crawler
│   │           ├── bayern.rs         # HTML crawler
│   │           ├── eur_lex.rs        # SPARQL endpoint
│   │           ├── legifrance.rs     # French legal portal
│   │           ├── normattiva.rs     # Italian legal portal
│   │           └── boe.rs            # Spanish BOE XML
│   │
│   └── api/                      # Axum HTTP server + MCP handler
│       └── src/
│           ├── lib.rs
│           ├── main.rs           # api-service binary entry point
│           ├── router.rs         # Axum route definitions
│           ├── middleware/
│           │   ├── auth.rs       # API key Bearer token validation
│           │   └── tracing.rs    # Request tracing (tower-http)
│           ├── handlers/
│           │   ├── search.rs     # GET /api/v1/search
│           │   ├── documents.rs  # GET /api/v1/documents/:id
│           │   ├── sources.rs    # GET/POST /api/v1/sources
│           │   └── changes.rs    # GET /api/v1/changes
│           ├── mcp/
│           │   ├── server.rs     # rmcp ServerHandler impl
│           │   └── tools.rs      # MCP tool definitions (4 tools)
│           └── services/
│               ├── search.rs     # Vector search + metadata filter
│               ├── document.rs   # Document retrieval + summaries
│               └── source.rs     # Source CRUD
│
├── bin/
│   ├── crawler-service/
│   │   └── src/main.rs           # crawler-service binary entry point
│   └── seed-sources/
│       └── src/main.rs           # One-shot DB seeder (6 sources)
│
├── packages/
│   └── frontend/                 # Next.js 14+ (kept, enhanced)
│       ├── messages/
│       │   ├── en.json           # English translations
│       │   └── de.json           # German translations
│       └── src/
│           ├── app/
│           │   └── [locale]/     # next-intl locale routing
│           └── lib/
│               └── api.ts        # REST API client
│
└── docker-compose.yml            # postgres, api-service, crawler-service, frontend
```

---

## Interface Specifications

### Database Schema

All migrations live in `crates/core/migrations/`. Schema is owned exclusively by `crates/core`.

#### Table: `sources`

```sql
CREATE TABLE sources (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL UNIQUE,          -- "Bundesrecht", "EUR-Lex", etc.
    jurisdiction    TEXT NOT NULL,                  -- "DE", "EU", "FR", "IT", "ES"
    language        TEXT NOT NULL,                  -- "de", "en", "fr", "it", "es"
    base_url        TEXT NOT NULL,
    crawler_type    TEXT NOT NULL,                  -- "bundesrecht" | "bayern" | "eur_lex" | ...
    cron_schedule   TEXT NOT NULL DEFAULT '0 2 * * *',  -- daily at 02:00 UTC
    config_json     JSONB NOT NULL DEFAULT '{}',    -- source-specific config (SPARQL endpoint, selectors, etc.)
    credentials_enc BYTEA,                          -- pgcrypto-encrypted credentials (if needed)
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### Table: `documents`

```sql
CREATE TABLE documents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id       UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    external_id     TEXT NOT NULL,                  -- Source-assigned ID (e.g. "BJNR001950896")
    url             TEXT NOT NULL,
    title           TEXT NOT NULL,
    content_md      TEXT NOT NULL,                  -- Full document as Markdown
    summary         TEXT,                           -- AI-generated summary (Gemini)
    doc_type        TEXT NOT NULL,                  -- "statute" | "regulation" | "case" | "directive"
    jurisdiction    TEXT NOT NULL,
    language        TEXT NOT NULL,
    published_at    TIMESTAMPTZ,
    effective_at    TIMESTAMPTZ,
    content_hash    TEXT NOT NULL,                  -- SHA-256 of content_md
    object_key      TEXT,                           -- Object storage key for original file
    metadata_json   JSONB NOT NULL DEFAULT '{}',    -- doc-type-specific fields
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_id, external_id)
);

CREATE INDEX idx_documents_source_id ON documents(source_id);
CREATE INDEX idx_documents_jurisdiction ON documents(jurisdiction);
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_published_at ON documents(published_at DESC);
CREATE INDEX idx_documents_content_hash ON documents(content_hash);
```

#### Table: `document_versions`

```sql
CREATE TABLE document_versions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id     UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    version_number  INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    content_md      TEXT NOT NULL,
    changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    change_summary  TEXT,                           -- AI-generated diff summary
    UNIQUE (document_id, version_number)
);

CREATE INDEX idx_document_versions_document_id ON document_versions(document_id);
CREATE INDEX idx_document_versions_changed_at ON document_versions(changed_at DESC);
```

#### Table: `chunks`

```sql
CREATE TABLE chunks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id     UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index     INTEGER NOT NULL,               -- Position within document
    content         TEXT NOT NULL,                  -- Chunk text
    embedding       vector(768),                    -- pgvector HNSW indexed
    token_count     INTEGER NOT NULL,
    metadata_json   JSONB NOT NULL DEFAULT '{}',    -- section, article, page refs
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id, chunk_index)
);

-- HNSW index for cosine similarity search (ADR-3)
CREATE INDEX idx_chunks_embedding_hnsw
    ON chunks USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_chunks_document_id ON chunks(document_id);
```

#### Table: `crawl_runs`

```sql
CREATE TABLE crawl_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id       UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ,
    status          TEXT NOT NULL DEFAULT 'running', -- "running" | "completed" | "failed"
    docs_discovered INTEGER NOT NULL DEFAULT 0,
    docs_new        INTEGER NOT NULL DEFAULT 0,
    docs_updated    INTEGER NOT NULL DEFAULT 0,
    docs_unchanged  INTEGER NOT NULL DEFAULT 0,
    error_message   TEXT,
    metadata_json   JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_crawl_runs_source_id ON crawl_runs(source_id);
CREATE INDEX idx_crawl_runs_started_at ON crawl_runs(started_at DESC);
```

#### Table: `api_keys`

```sql
CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    key_hash        TEXT NOT NULL UNIQUE,           -- SHA-256 of raw key
    key_prefix      TEXT NOT NULL,                  -- First 8 chars of raw key (display only)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ,
    enabled         BOOLEAN NOT NULL DEFAULT true
);
```

---

### REST API Endpoints

Base path: `/api/v1/`. All responses are `application/json`.

Authentication: `Authorization: Bearer <api-key>` required on all non-public endpoints. Public endpoints (search, document retrieval) are rate-limited but open in v1.

#### Search

```
GET /api/v1/search
```

Query parameters:

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | yes | Natural language query |
| `jurisdiction` | string | no | Filter: "DE", "EU", "FR", "IT", "ES" |
| `doc_type` | string | no | Filter: "statute", "regulation", "case", "directive" |
| `language` | string | no | Filter: "de", "en", "fr", "it", "es" |
| `limit` | int | no | Max results (default 10, max 50) |
| `offset` | int | no | Pagination offset (default 0) |

Response:
```json
{
  "results": [
    {
      "chunk_id": "uuid",
      "document_id": "uuid",
      "title": "Bürgerliches Gesetzbuch",
      "url": "https://...",
      "jurisdiction": "DE",
      "doc_type": "statute",
      "language": "de",
      "snippet": "...matched text excerpt...",
      "score": 0.923,
      "published_at": "2024-01-01T00:00:00Z",
      "source_name": "Bundesrecht"
    }
  ],
  "total": 847,
  "limit": 10,
  "offset": 0,
  "query_embedding_ms": 45,
  "search_ms": 120
}
```

#### Documents

```
GET /api/v1/documents/:id
```

Returns full document including `content_md`, `summary`, metadata.

```
GET /api/v1/documents/:id/versions
```

Returns version history with `changed_at` and `change_summary`.

```
GET /api/v1/documents/:id/chunks
```

Returns all chunks for a document (without embeddings).

#### Sources

```
GET  /api/v1/sources              # List all sources (public)
POST /api/v1/sources              # Create source (requires API key)
GET  /api/v1/sources/:id          # Get source + last crawl run stats
PATCH /api/v1/sources/:id         # Update source config (requires API key)
DELETE /api/v1/sources/:id        # Delete source (requires API key)
POST /api/v1/sources/:id/crawl    # Trigger manual crawl (requires API key)
```

#### Changes

```
GET /api/v1/changes
```

Query parameters: `jurisdiction`, `doc_type`, `since` (ISO datetime), `limit`, `offset`.

Returns documents with `version_number > 1` ordered by `changed_at DESC`.

#### PDF Upload

```
POST /api/v1/documents/upload
Content-Type: multipart/form-data
```

Fields: `file` (PDF), `title`, `jurisdiction`, `doc_type`, `language`. Returns created `document_id`.

#### Admin

```
GET  /api/v1/admin/crawl-runs     # Recent crawl run history
GET  /api/v1/admin/stats          # System stats (doc count, chunk count, queue depth)
POST /api/v1/admin/api-keys       # Create API key
DELETE /api/v1/admin/api-keys/:id # Revoke API key
```

---

### MCP Tool Definitions

MCP endpoint: `POST /mcp` (Streamable HTTP transport via rmcp 0.16).

All tools follow JSON-RPC 2.0. No authentication for v1 (CON-8).

#### Tool: `search_laws`

```json
{
  "name": "search_laws",
  "description": "Search European legal documents using semantic vector search. Returns ranked results with document metadata and text excerpts.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "Natural language search query" },
      "jurisdiction": { "type": "string", "enum": ["DE", "EU", "FR", "IT", "ES"], "description": "Optional jurisdiction filter" },
      "doc_type": { "type": "string", "enum": ["statute", "regulation", "case", "directive"], "description": "Optional document type filter" },
      "limit": { "type": "integer", "default": 10, "maximum": 20 }
    },
    "required": ["query"]
  }
}
```

#### Tool: `get_law_by_id`

```json
{
  "name": "get_law_by_id",
  "description": "Retrieve a specific legal document by its ID, including full text and AI summary.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "document_id": { "type": "string", "description": "Document UUID" }
    },
    "required": ["document_id"]
  }
}
```

#### Tool: `get_sources`

```json
{
  "name": "get_sources",
  "description": "List available legal data sources with jurisdiction and language coverage.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

#### Tool: `get_legal_changes`

```json
{
  "name": "get_legal_changes",
  "description": "Get recently changed or newly published legal documents.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "jurisdiction": { "type": "string" },
      "since": { "type": "string", "format": "date-time", "description": "ISO 8601 datetime. Defaults to 30 days ago." },
      "limit": { "type": "integer", "default": 10, "maximum": 50 }
    }
  }
}
```

---

### Data Models (Rust)

Key types in `crates/core/src/models/`:

```rust
// source.rs
pub struct Source {
    pub id: Uuid,
    pub name: String,
    pub jurisdiction: String,
    pub language: String,
    pub base_url: String,
    pub crawler_type: CrawlerType,
    pub cron_schedule: String,
    pub config_json: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum CrawlerType {
    Bundesrecht,
    Bayern,
    EurLex,
    Legifrance,
    Normattiva,
    Boe,
    Custom(String),
}

// document.rs
pub struct Document {
    pub id: Uuid,
    pub source_id: Uuid,
    pub external_id: String,
    pub url: String,
    pub title: String,
    pub content_md: String,
    pub summary: Option<String>,
    pub doc_type: DocType,
    pub jurisdiction: String,
    pub language: String,
    pub published_at: Option<DateTime<Utc>>,
    pub effective_at: Option<DateTime<Utc>>,
    pub content_hash: String,
    pub object_key: Option<String>,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum DocType {
    Statute,
    Regulation,
    Case,
    Directive,
}

// chunk.rs
pub struct Chunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: i32,
    pub content: String,
    pub embedding: Option<pgvector::Vector>,
    pub token_count: i32,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// Search result (not persisted)
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub title: String,
    pub url: String,
    pub jurisdiction: String,
    pub doc_type: DocType,
    pub language: String,
    pub snippet: String,
    pub score: f32,
    pub published_at: Option<DateTime<Utc>>,
    pub source_name: String,
}
```

---

## Runtime View

### Flow 1: Scheduled Crawl → Ingestion Pipeline

```mermaid
sequenceDiagram
    participant SCHED as tokio-cron-scheduler
    participant CRW as BaseCrawler (source impl)
    participant HASH as SHA-256 Hasher
    participant OBJ as Object Store
    participant APALIS as apalis Queue (PostgreSQL)
    participant WORKER as Ingest Worker
    participant PARSER as Document Parser
    participant CHUNKER as Text Chunker
    participant EMBED as Vertex AI Embedder
    participant DB as PostgreSQL

    SCHED->>CRW: trigger(source_id) at cron time
    CRW->>DB: INSERT crawl_runs (status=running)
    CRW->>GOV: HTTP fetch document list (XML/SPARQL/HTML)
    GOV-->>CRW: document URLs + metadata
    loop For each discovered document
        CRW->>GOV: HTTP fetch document content
        GOV-->>CRW: raw content (HTML/XML/PDF)
        CRW->>HASH: sha256(raw_content)
        HASH-->>CRW: content_hash
        CRW->>DB: SELECT content_hash FROM documents WHERE external_id=?
        alt content_hash changed or new document
            CRW->>OBJ: upload(key, raw_content)
            CRW->>DB: UPSERT documents (content_hash, object_key, ...)
            CRW->>APALIS: enqueue IngestJob(document_id)
        else unchanged
            CRW->>DB: UPDATE crawl_runs (docs_unchanged++)
        end
    end
    CRW->>DB: UPDATE crawl_runs (status=completed)

    Note over WORKER: apalis worker pool picks up jobs
    WORKER->>DB: SELECT document WHERE id=?
    WORKER->>OBJ: download(object_key) → raw_content
    WORKER->>PARSER: parse(raw_content, format)
    Note over PARSER: HTML→scraper, XML→quick-xml, PDF→Mistral API
    PARSER-->>WORKER: content_md (Markdown)
    WORKER->>CHUNKER: chunk(content_md, max_tokens=512, overlap=50)
    CHUNKER-->>WORKER: Vec<ChunkText>
    WORKER->>EMBED: embed_batch(chunk_texts) → Vec<Vector(768)>
    Note over EMBED: Batches of ≤250 texts, Vertex AI text-embedding-004
    EMBED-->>WORKER: Vec<Vector>
    WORKER->>DB: DELETE chunks WHERE document_id=?
    WORKER->>DB: INSERT chunks (content, embedding, token_count, ...)
    WORKER->>GEMINI: generate_summary(content_md[0..3000])
    GEMINI-->>WORKER: summary_text
    WORKER->>DB: UPDATE documents SET summary=?, content_md=?
    alt version_number > 1
        WORKER->>DB: INSERT document_versions (content_hash, content_md, ...)
    end
```

### Flow 2: Vector Search (REST / MCP)

```mermaid
sequenceDiagram
    participant CLIENT as Client (Browser / LLM)
    participant AUTH as Auth Middleware
    participant HANDLER as Search Handler
    participant SVC as Search Service
    participant EMBED as Vertex AI Embedder
    participant DB as PostgreSQL (pgvector)

    CLIENT->>HANDLER: GET /api/v1/search?q=...&jurisdiction=DE
    HANDLER->>AUTH: validate Bearer token (API key) or skip (public)
    AUTH-->>HANDLER: ok / 401

    HANDLER->>SVC: search(query, filters, limit, offset)
    SVC->>EMBED: embed_batch([query]) → Vector(768)
    Note over EMBED: Single query embedding ~50ms
    EMBED-->>SVC: query_vector

    SVC->>DB: SELECT chunks + documents via HNSW cosine search
    Note over DB: SELECT c.id, d.title, d.url, d.jurisdiction,\nd.doc_type, d.language, c.content,\n1 - (c.embedding <=> $1) AS score\nFROM chunks c JOIN documents d ON c.document_id = d.id\nWHERE d.jurisdiction = ANY($2) -- optional filter\nORDER BY c.embedding <=> $1\nLIMIT $3 OFFSET $4
    DB-->>SVC: Vec<SearchRow>

    SVC->>SVC: build SearchResult (snippet from chunk content)
    SVC-->>HANDLER: Vec<SearchResult>
    HANDLER-->>CLIENT: JSON response (results, total, timing)
```

### Flow 3: MCP Tool Call

```mermaid
sequenceDiagram
    participant LLM as LLM Client (Claude/Cursor)
    participant RMCP as rmcp ServerHandler
    participant TOOLS as MCP Tool Dispatcher
    participant SVC as Search / Document Service
    participant DB as PostgreSQL

    LLM->>RMCP: POST /mcp (JSON-RPC: tools/call search_laws)
    Note over RMCP: rmcp 0.16 Streamable HTTP transport
    RMCP->>TOOLS: dispatch("search_laws", params)
    TOOLS->>SVC: search(query, jurisdiction, doc_type, limit)
    SVC->>DB: embed query + HNSW search (same as Flow 2)
    DB-->>SVC: Vec<SearchResult>
    SVC-->>TOOLS: results
    TOOLS->>TOOLS: format as MCP TextContent with citations
    TOOLS-->>RMCP: ToolResult { content: [...] }
    RMCP-->>LLM: JSON-RPC response

    Note over LLM: For get_law_by_id tool:
    LLM->>RMCP: POST /mcp (tools/call get_law_by_id)
    RMCP->>TOOLS: dispatch("get_law_by_id", {id, include_full_text})
    TOOLS->>SVC: get_document(id)
    SVC->>DB: SELECT documents WHERE id=?
    DB-->>SVC: Document
    alt include_full_text=true
        SVC->>DB: SELECT chunks WHERE document_id=? ORDER BY chunk_index
        DB-->>SVC: Vec<Chunk>
        SVC->>TOOLS: Document + full text
    else
        SVC->>TOOLS: Document + summary only
    end
    TOOLS-->>RMCP: ToolResult { content: [title, url, summary, metadata] }
    RMCP-->>LLM: JSON-RPC response
```

---

## Deployment View

### Docker Compose (Development + Production v1)

```yaml
# docker-compose.yml (production-equivalent)
services:
  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_DB: legalmcp
      POSTGRES_USER: legalmcp
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "legalmcp"]
      interval: 10s
      retries: 5

  api-service:
    build:
      context: .
      dockerfile: Dockerfile.api
    environment:
      DATABASE_URL: postgresql://legalmcp:${POSTGRES_PASSWORD}@postgres:5432/legalmcp
      VERTEX_AI_PROJECT: ${VERTEX_AI_PROJECT}
      VERTEX_AI_LOCATION: europe-west3
      GOOGLE_APPLICATION_CREDENTIALS: /secrets/gcp-sa.json
      RUST_LOG: info
    ports:
      - "8000:8000"
    depends_on:
      postgres:
        condition: service_healthy
    volumes:
      - ${GCP_SA_KEY_PATH}:/secrets/gcp-sa.json:ro

  crawler-service:
    build:
      context: .
      dockerfile: Dockerfile.crawler
    environment:
      DATABASE_URL: postgresql://legalmcp:${POSTGRES_PASSWORD}@postgres:5432/legalmcp
      VERTEX_AI_PROJECT: ${VERTEX_AI_PROJECT}
      VERTEX_AI_LOCATION: europe-west3
      GOOGLE_APPLICATION_CREDENTIALS: /secrets/gcp-sa.json
      MISTRAL_API_KEY: ${MISTRAL_API_KEY}
      OBJECT_STORE_TYPE: gcs  # or "azure"
      GCS_BUCKET: ${GCS_BUCKET}
      RUST_LOG: info
    depends_on:
      postgres:
        condition: service_healthy
    volumes:
      - ${GCP_SA_KEY_PATH}:/secrets/gcp-sa.json:ro

  frontend:
    build:
      context: packages/frontend
      dockerfile: Dockerfile
    environment:
      NEXT_PUBLIC_API_URL: http://api-service:8000
    ports:
      - "3000:3000"
    depends_on:
      - api-service

volumes:
  postgres_data:
```

### Dockerfile Strategy

Two separate Dockerfiles using Rust multi-stage builds:

```dockerfile
# Dockerfile.api (and similar for Dockerfile.crawler)
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Build only the target binary (leverages crate-level caching)
RUN cargo build --release --bin api-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api-service /usr/local/bin/
EXPOSE 8000
CMD ["api-service"]
```

Final image size target: < 50MB (Rust static binary + minimal debian).

### Environment Variables

| Variable | Service | Required | Description |
|----------|---------|----------|-------------|
| `DATABASE_URL` | both | yes | PostgreSQL connection string |
| `VERTEX_AI_PROJECT` | both | yes | GCP project ID |
| `VERTEX_AI_LOCATION` | both | yes | GCP region (europe-west3 for EU) |
| `GOOGLE_APPLICATION_CREDENTIALS` | both | yes | Path to GCP service account JSON |
| `MISTRAL_API_KEY` | crawler | yes | Mistral Document AI API key |
| `OBJECT_STORE_TYPE` | crawler | yes | "gcs" or "azure" |
| `GCS_BUCKET` | crawler | if GCS | GCS bucket name |
| `AZURE_STORAGE_ACCOUNT` | crawler | if Azure | Azure storage account |
| `AZURE_STORAGE_KEY` | crawler | if Azure | Azure storage key |
| `AZURE_CONTAINER` | crawler | if Azure | Azure blob container name |
| `API_PORT` | api | no | HTTP port (default 8000) |
| `RUST_LOG` | both | no | Log level (default "info") |
| `MAX_DB_CONNECTIONS` | both | no | Pool size (default 20) |

### Cloud Run Deployment (GCP — recommended)

```
GCP Project (EU region: europe-west3)
├── Cloud Run Services
│   ├── legalmcp-api         (api-service, min 1, max 10 instances)
│   └── legalmcp-crawler     (crawler-service, min 0, max 2 instances)
├── Cloud SQL
│   └── PostgreSQL 16 + pgvector extension
├── Google Cloud Storage
│   └── legalmcp-documents bucket
└── Secret Manager
    ├── db-password
    ├── mistral-api-key
    └── gcp-sa-key
```

Alternatively, Azure Container Apps (same topology, Azure Blob, Azure Database for PostgreSQL Flexible Server).

---

## Cross-Cutting Concepts

### Error Handling

All errors flow through `LegalMcpError` in `crates/core/src/errors.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LegalMcpError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Crawl error for source {source}: {message}")]
    Crawl { source: String, message: String },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("External API error [{service}]: {message}")]
    ExternalApi { service: String, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(String),
}
```

HTTP error responses follow RFC 7807 Problem Details:
```json
{
  "type": "https://legalmcp.example.com/errors/not-found",
  "title": "Document Not Found",
  "status": 404,
  "detail": "No document with id 'abc123' exists"
}
```

MCP errors return JSON-RPC 2.0 error objects with standard codes (-32600 to -32099).

### Observability

- **Structured logging**: `tracing` crate with JSON output (`tracing-subscriber` + `tracing_json`). All log lines include `service`, `request_id`, `trace_id`.
- **Request tracing**: `tower-http` `TraceLayer` on all Axum routes. Each request gets a `X-Request-Id` UUID header.
- **Metrics**: `/metrics` endpoint (Prometheus format) via `metrics` + `metrics-exporter-prometheus` crates. Key metrics:
  - `legalmcp_search_duration_seconds` (histogram)
  - `legalmcp_embedding_duration_seconds` (histogram)
  - `legalmcp_crawl_documents_total` (counter, labels: source, outcome)
  - `legalmcp_mcp_tool_calls_total` (counter, labels: tool)
- **Health checks**: `GET /health` (liveness) and `GET /ready` (readiness — checks DB connectivity).
- **Log levels**: `RUST_LOG=info` in production. `RUST_LOG=debug` for development.

### Security

- **API key storage**: SHA-256 hash stored in `api_keys.key_hash`. Raw key shown only at creation. Constant-time comparison to prevent timing attacks.
- **Source credentials**: Encrypted at rest using `pgcrypto` symmetric encryption (`pgp_sym_encrypt`). Decryption key from environment variable.
- **SQL injection**: Impossible — sqlx compile-time checked queries with `query!` macro (no string interpolation).
- **Rate limiting**: `tower_governor` middleware on search endpoints (100 req/min per IP for public, 1000/min for API key holders).
- **CORS**: Configured in Axum to allow frontend origin only. MCP endpoint allows `*` (LLM clients from any origin).
- **TLS**: Terminated at reverse proxy / cloud load balancer level. Services communicate over plain HTTP within Docker network.
- **Secrets**: Never in code, Dockerfile, or git. Always from environment variables or mounted secrets.

### Performance

- **Connection pooling**: `sqlx::PgPool` with `max_connections = 20` per service. Shared pool ref via `Arc<PgPool>` in Axum state.
- **Embedding batching**: `embed_batch()` takes `Vec<String>`, batches into groups of 250 (Vertex AI limit). Never calls embedding API per-document sequentially.
- **HNSW tuning**: `m=16, ef_construction=64` for index build; set `SET hnsw.ef_search = 100` at query time for recall/speed balance.
- **Async throughout**: All I/O is `async/await` with Tokio. No blocking calls on async threads.
- **Chunker parallelism**: `tokio::task::spawn_blocking` for CPU-bound text-splitting operations.
- **apalis concurrency**: Worker pool processes ingestion jobs concurrently (configurable, default 4 workers per service).

### i18n

- **Frontend**: next-intl 4.x with App Router locale routing (`/[locale]/...`). Default locale: `de`. Supported: `en`, `de`.
- **Translation files**: `packages/frontend/messages/en.json` and `de.json`. All UI strings translated. No hardcoded text in components.
- **Backend**: Language-aware only in search filters (`language` query param). No backend translation — all i18n is frontend responsibility.
- **Legal content**: Legal text is stored in its source language (German laws in German, French laws in French). The UI language is independent of the content language. Cross-language search is supported by multilingual embeddings (text-embedding-004).

### Resilience

- **Crawler retries**: `reqwest-retry` with exponential backoff (3 retries, 1s/2s/4s delays). Per-source rate limiting via `governor` token bucket.
- **robots.txt**: `BaseCrawler` checks `robots.txt` before crawling. Respects `Crawl-Delay` header.
- **Embedding retries**: `reqwest-middleware` with retry policy for Vertex AI transient errors (429, 503).
- **apalis job retry**: Failed ingestion jobs retry up to 3 times with exponential backoff before moving to dead-letter queue.
- **Circuit breaker**: Crawlers that fail >5 consecutive times are automatically disabled (status set to `enabled=false`) until manually re-enabled.
- **Graceful shutdown**: Both services handle `SIGTERM` — drain in-flight requests, complete current job step, close DB pool.

---

## Architecture Decisions

### ADR-1: Two-Binary Architecture with apalis Job Queue

**Status**: Accepted

**Context**: The system needs both a real-time HTTP API and a background crawling+ingestion pipeline. The crawling workload is batch-oriented and I/O-heavy; the API workload is latency-sensitive. Combining both in one process risks crawl jobs blocking API request handling.

**Decision**: Two separate Rust binaries (`api-service`, `crawler-service`) deployed as separate Docker containers. They communicate exclusively via shared PostgreSQL tables and apalis job queue rows. No direct HTTP between services.

**Consequences**:
- (+) Independent scaling of API vs. crawl workload
- (+) Crawler failures don't affect API availability
- (+) No Redis, RabbitMQ, or additional infrastructure needed
- (-) Shared database is a coupling point — schema changes affect both binaries
- (-) Slightly more complex deployment than a single binary

---

### ADR-2: sqlx 0.8 + pgvector Crate (No ORM)

**Status**: Accepted

**Context**: Options were: sqlx (compile-time verified SQL), Diesel (ORM), SeaORM (async ORM), or raw postgres driver. The schema includes pgvector `vector(768)` columns requiring native type support.

**Decision**: sqlx 0.8 with compile-time `query!` macros and the `pgvector` crate for `Vector` type binding. No ORM.

**Consequences**:
- (+) SQL errors caught at compile time, not runtime
- (+) No abstraction overhead — queries are exactly what gets sent to PostgreSQL
- (+) pgvector 0.4.1 supports `Vector` type in sqlx `FromRow` derives
- (-) More verbose than ORM for CRUD operations
- (-) Schema changes require updating both migration SQL and Rust structs manually

---

### ADR-3: HNSW Index, Single `chunks` Table

**Status**: Accepted

**Context**: Current system uses IVFFlat indexes and separate `law_chunks`/`case_law_chunks` tables. IVFFlat requires re-indexing when data grows significantly. Separate tables require UNION queries for cross-type search.

**Decision**: Single `chunks` table with a `document_id` FK (no type split). HNSW index (`m=16, ef_construction=64`) on `embedding` column. Document type is stored in `documents.doc_type`.

**Consequences**:
- (+) HNSW does not require re-indexing — supports incremental inserts
- (+) Single table simplifies cross-type search query
- (+) Better recall at given speed compared to IVFFlat
- (-) HNSW index is larger in memory than IVFFlat
- (-) All chunks from all sources share one table (may need partitioning at very large scale — V2 concern)

---

### ADR-4: Object Storage via `object_store` Crate

**Status**: Accepted

**Context**: Original document files (HTML, XML, PDF) must be preserved for audit, re-processing, and version diffing. Local disk storage is incompatible with multi-container deployment.

**Decision**: `object_store` 0.11 crate with runtime-configurable backend (GCS or Azure Blob). Original files stored with key pattern `{source_id}/{external_id}/{content_hash}.{ext}`.

**Consequences**:
- (+) Cloud-native; no disk persistence concerns
- (+) GCS/Azure abstraction allows deploying on either cloud
- (+) Enables re-processing documents without re-crawling
- (-) Adds external API dependency for ingestion
- (-) Object storage costs scale with document corpus size

---

### ADR-5: Mistral Document AI for PDF Extraction

**Status**: Accepted

**Context**: Several sources (especially case law portals) publish documents as PDFs. Native Rust PDF libraries (`lopdf`, `pdf-extract`) are insufficient for production: they fail on complex layouts, scanned PDFs (OCR), and multi-column legal formatting. `pdfium-render` requires native system library.

**Decision**: Mistral Document AI (`mistral-ocr-latest` model). Sends PDF binary → receives structured Markdown. API call per PDF document.

**Consequences**:
- (+) Handles complex layouts and OCR natively
- (+) Returns structured Markdown directly — no post-processing needed
- (+) API-based — no native library maintenance
- (-) Adds per-PDF API cost (~$1/1000 pages)
- (-) External dependency for ingestion pipeline
- (-) Latency of ~2-5s per PDF document

---

### ADR-6: rmcp 0.16 for MCP (Streamable HTTP Transport)

**Status**: Accepted

**Context**: Current system has `/mcp` endpoint returning 501 Not Implemented. Only stdio transport works, which is incompatible with web deployment. The MCP specification added Streamable HTTP transport in 2024.

**Decision**: `rmcp` 0.16 (official Rust MCP SDK from Anthropic). Implements `ServerHandler` trait. Mounts at `/mcp` as Axum route. Streamable HTTP transport only — no stdio in production.

**Consequences**:
- (+) Official SDK — spec-compliant tool definitions, JSON-RPC 2.0
- (+) Streamable HTTP works with any MCP client (Claude, Cursor, etc.)
- (+) Integrates as Axum handler — same server, same port as REST API
- (-) rmcp 0.16 API is still stabilizing — may have breaking changes
- (-) No stdio support for local MCP client testing (workaround: Claude Desktop via HTTP URL)

---

### ADR-7: Vertex AI text-embedding-004 via `Embedder` Trait

**Status**: Accepted

**Context**: Current system has a critical bug: crawler uses `deepset/gbert-base` (local sentence-transformers, 768-dim), while API uses Vertex AI `text-embedding-004` (768-dim). Embeddings are incompatible — queries never match stored chunks. The new system must use a single consistent embedding provider.

**Decision**: Vertex AI `text-embedding-004` as sole provider. Wrapped behind an `Embedder` trait (`crates/embeddings`) to allow swapping in tests or future provider changes. Batch endpoint supports 250 texts/call.

**Consequences**:
- (+) Fixes the embedding mismatch bug permanently
- (+) 768-dim multilingual model handles DE/EN/FR/IT/ES content
- (+) Batch API reduces call count by ~250x vs sequential calls
- (-) Requires GCP project + service account for all environments (including CI)
- (-) MockEmbedder needed for unit tests (avoids API calls in tests)

---

### ADR-8: tokio-cron-scheduler 0.15 with PostgreSQL Persistence

**Status**: Accepted

**Context**: Crawl sources each need independent cron schedules (daily, weekly, etc.). Schedules must survive service restarts. Options were: tokio-cron-scheduler, croner, apalis scheduled jobs, or external cron (Kubernetes CronJob, Cloud Scheduler).

**Decision**: `tokio-cron-scheduler` 0.15 with PostgreSQL job store. Each `Source` row stores a `cron_schedule` string. On startup, crawler-service registers all enabled sources as cron jobs.

**Consequences**:
- (+) No Kubernetes or Cloud Scheduler dependency
- (+) Schedule state persists across restarts (PostgreSQL)
- (+) Dynamic: adding/editing a source row updates the schedule
- (-) Cron runs only on a single crawler-service instance (no distributed lock for multi-instance). Acceptable for v1 (max_instances=1 for crawler).

---

### ADR-9: Unified `documents` + `chunks` Schema (No Type Splitting)

**Status**: Accepted

**Context**: Current schema has `laws`/`law_chunks` and `case_law`/`case_law_chunks` as separate tables, reflecting the original Python design. Adding new doc types (e.g. directives, regulations) would require new tables each time.

**Decision**: Single `documents` table with `doc_type` discriminator column (`statute`, `regulation`, `case`, `directive`). Single `chunks` table. Type-specific metadata stored in `metadata_json: JSONB`.

**Consequences**:
- (+) Adding a new doc type = no schema migration (just new `doc_type` value)
- (+) Simpler cross-type search queries
- (+) `metadata_json` is flexible for type-specific fields
- (-) No compile-time column validation for type-specific metadata fields
- (-) May need JSON schema validation in application layer for metadata

---

### ADR-10: 5-Crate Cargo Workspace

**Status**: Accepted

**Context**: All Rust code could be in a single crate (simple but hard to test in isolation) or in many fine-grained crates (flexible but high overhead). A middle ground is needed.

**Decision**: 5 crates: `core` (domain + DB), `embeddings` (embedding abstraction), `ingest` (pipeline steps), `crawler` (HTTP + source impls), `api` (Axum server + MCP). Clear dependency graph: `core` ← `embeddings` ← `ingest` ← `crawler`; `core` + `embeddings` ← `api`.

**Consequences**:
- (+) `core` can be tested without network I/O
- (+) `embeddings` can be mocked in tests for `ingest` and `api`
- (+) Clear boundaries prevent circular dependencies
- (-) More `Cargo.toml` files to maintain
- (-) Compile times longer than single crate (partially mitigated by workspace caching)

---

### ADR-11: next-intl 4.x for Frontend i18n

**Status**: Accepted

**Context**: Current frontend is German-only with hardcoded strings. Retrofitting i18n after launch is expensive. The product targets both German-speaking legal professionals (primary) and international users via the English MCP/API layer.

**Decision**: next-intl 4.x with App Router locale routing (`/[locale]/...`). Default locale `de`. All UI strings in `messages/en.json` and `messages/de.json`. No hardcoded text in React components.

**Consequences**:
- (+) Bilingual from day one — no future migration cost
- (+) next-intl 4.x has first-class App Router and RSC support
- (+) Locale routing is clean (`/de/search`, `/en/search`)
- (-) Requires wrapping all text in `t('key')` calls — more verbose
- (-) Translation keys must be maintained in sync across both locale files

---

### ADR-12: SHA-256 Hash + `document_versions` Table for Change Detection

**Status**: Accepted

**Context**: Legal documents change over time (amendments, corrections). The system must detect changes and track version history for compliance use cases (F2, F12). Options were: last-modified HTTP header, ETag, or content hash.

**Decision**: SHA-256 hash of `content_md` (after extraction and normalisation). On each crawl, compare stored `documents.content_hash` with newly computed hash. On change: create `document_versions` entry, re-chunk and re-embed, update `documents`.

**Consequences**:
- (+) Works for any source regardless of HTTP header support
- (+) Content-based — immune to spurious last-modified changes
- (+) Full version history in `document_versions` enables change feed (F12)
- (-) Requires computing hash for every crawled document (CPU-cheap, acceptable)
- (-) Full re-embedding on any content change (cost: ~$0.001 per document)

---

## Quality Requirements

### Performance

| ID | Requirement | Measurement | Target |
|----|-------------|-------------|--------|
| QR-P1 | Search response time (REST) | p95 end-to-end | < 2s |
| QR-P2 | MCP tool response time | p95 for `search_laws` | < 3s |
| QR-P3 | Embedding API call | p95 per batch (250 texts) | < 5s |
| QR-P4 | Ingestion throughput | Documents processed/hour | > 500/hour |
| QR-P5 | Concurrent MCP sessions | Without degradation | 100 sessions |
| QR-P6 | API cold start | Docker container start time | < 5s |

### Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| QR-R1 | API service uptime | 99.5% monthly |
| QR-R2 | Crawl job retry | Failed jobs retry 3x before dead-letter |
| QR-R3 | Data durability | No document loss on service restart |
| QR-R4 | Graceful shutdown | Drain in-flight requests within 30s |
| QR-R5 | Embedding failure | Pipeline continues with other docs; failed doc retried |

### Maintainability

| ID | Requirement | Target |
|----|-------------|--------|
| QR-M1 | Test coverage | > 80% for core + api crates |
| QR-M2 | CI pipeline | All PRs: clippy + fmt + test + sqlx prepare check |
| QR-M3 | Compile-time safety | Zero `unwrap()` in non-test code (use `?` operator) |
| QR-M4 | Adding a new source | < 1 day engineering time (implement BaseCrawler trait) |
| QR-M5 | Schema migration | Zero-downtime (additive-only for v1, no column drops) |

### Security

| ID | Requirement | Target |
|----|-------------|--------|
| QR-S1 | SQL injection | Zero (enforced by sqlx compile-time macros) |
| QR-S2 | Secret exposure | No secrets in logs, error messages, or git history |
| QR-S3 | API key brute force | Rate limit to 100 req/min per IP |
| QR-S4 | Credential encryption | Source credentials encrypted at rest (pgcrypto) |

### Compatibility

| ID | Requirement | Target |
|----|-------------|--------|
| QR-C1 | MCP protocol version | Compatible with MCP spec 2024-11 (Streamable HTTP) |
| QR-C2 | Browser support | Last 2 versions of Chrome, Firefox, Safari, Edge |
| QR-C3 | PostgreSQL version | PostgreSQL 16 + pgvector 0.7+ |
| QR-C4 | Rust edition | Rust 2021 edition, MSRV 1.82 |

---

## Acceptance Criteria

Expressed in EARS (Easy Approach to Requirements Syntax) format, linked to PRD features.

### F1: Semantic Legal Document Search

**AC-F1-1** (Event-driven): WHEN a user submits a search query, THEN the system SHALL return semantically relevant results within 2 seconds (p95), ordered by cosine similarity score DESC.

**AC-F1-2** (Constraint): The system SHALL support optional filters for jurisdiction (DE/EU/FR/IT/ES), doc_type (statute/regulation/case/directive), and language without degrading the 2s p95 target.

**AC-F1-3** (Ubiquitous): The search endpoint SHALL be publicly accessible without authentication, subject to rate limiting (100 req/min per IP).

**AC-F1-4** (Event-driven): WHEN search returns results, THEN each result SHALL include: document title, source URL, jurisdiction, doc_type, language, published_at, similarity score, and a text excerpt (snippet) from the matching chunk.

### F2: Document Versioning & Change Tracking

**AC-F2-1** (Event-driven): WHEN a crawler detects a changed document (SHA-256 hash mismatch), THEN the system SHALL preserve the previous version in `document_versions` and re-embed the updated content.

**AC-F2-2** (Ubiquitous): The `GET /api/v1/changes` endpoint SHALL return documents with version changes ordered by `changed_at DESC`, filterable by jurisdiction and since-date.

**AC-F2-3** (Constraint): Version history SHALL be retained indefinitely (no automatic purge for v1).

### F3: AI-Powered Document Summaries

**AC-F3-1** (Event-driven): WHEN a new document is ingested, THEN the system SHALL generate a 2–3 sentence AI summary using Gemini 2.5 Pro and store it in `documents.summary`.

**AC-F3-2** (Ubiquitous): Summary SHALL be returned in `GET /api/v1/documents/:id` responses and in `get_law_by_id` MCP tool responses.

### F4: MCP Integration

**AC-F4-1** (Ubiquitous): The system SHALL expose an MCP-compliant endpoint at `POST /mcp` using Streamable HTTP transport (rmcp 0.16).

**AC-F4-2** (Ubiquitous): The MCP server SHALL implement 4 tools: `search_laws`, `get_law_by_id`, `get_sources`, `get_legal_changes`.

**AC-F4-3** (Performance): WHEN an LLM client calls `search_laws`, THEN the tool SHALL respond within 3 seconds (p95) under 100 concurrent sessions.

**AC-F4-4** (Constraint): MCP tool responses SHALL include source attribution (title, URL, jurisdiction) for all returned documents.

### F5: Multi-Source Legal Crawler

**AC-F5-1** (Ubiquitous): The system SHALL include crawlers for: Bundesrecht (XML), Bayern.Recht (HTML), EUR-Lex (SPARQL), Legifrance (FR), Normattiva (IT), BOE (ES).

**AC-F5-2** (Ubiquitous): Each crawler SHALL respect `robots.txt` and implement per-source rate limiting and exponential backoff retry.

**AC-F5-3** (Event-driven): WHEN a crawl completes, THEN the system SHALL record a `crawl_runs` row with discovered/new/updated/unchanged counts and completion status.

**AC-F5-4** (Constraint): Crawlers SHALL NOT store or process content that has not changed since the last crawl (SHA-256 deduplication).

### F6: Automated Ingestion Pipeline

**AC-F6-1** (Ubiquitous): The ingestion pipeline SHALL process: HTML → scraper, XML → quick-xml, PDF → Mistral Document AI, all normalised to Markdown.

**AC-F6-2** (Ubiquitous): Chunks SHALL be produced with max 512 tokens and 50-token overlap using `text-splitter`.

**AC-F6-3** (Ubiquitous): Embeddings SHALL be generated via Vertex AI `text-embedding-004` (768-dim) in batches of ≤250 texts per API call.

**AC-F6-4** (Event-driven): WHEN an ingestion job fails, THEN it SHALL be retried up to 3 times with exponential backoff before being marked as failed.

### F7: Source Management API

**AC-F7-1** (Ubiquitous): The REST API SHALL expose CRUD endpoints for sources at `/api/v1/sources`.

**AC-F7-2** (Ubiquitous): Source creation and modification SHALL require a valid API key (Bearer token).

**AC-F7-3** (Event-driven): WHEN a source's `enabled` field is set to `false`, THEN the crawler SHALL stop scheduling crawl runs for that source immediately (next scheduler tick).

### F8: Bilingual Frontend (EN+DE)

**AC-F8-1** (Ubiquitous): The frontend SHALL support locale routing at `/de/...` and `/en/...` with next-intl 4.x.

**AC-F8-2** (Constraint): No hardcoded UI strings SHALL exist in React components — all text SHALL come from `messages/{locale}.json` translation files.

**AC-F8-3** (Ubiquitous): The frontend SHALL preserve all existing shadcn/ui components and page routes: `/`, `/chat`, `/changes`, `/law/[id]`, `/case/[id]`.

### F10: PDF Upload & OCR

**AC-F10-1** (Event-driven): WHEN a user uploads a PDF via `POST /api/v1/documents/upload`, THEN the system SHALL extract text via Mistral Document AI and ingest the document through the standard pipeline.

**AC-F10-2** (Constraint): Uploaded PDFs SHALL be stored in object storage before processing. Processing failure SHALL not result in data loss.

---

## Risks and Technical Debt

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R1 | rmcp 0.16 API instability (pre-1.0 crate) | Medium | High | Pin exact version. Abstract MCP handler behind interface. Monitor upstream releases. |
| R2 | Mistral Document AI API outage or pricing change | Low | Medium | Object storage preserves originals. Can swap to alternative PDF extractor (pdfium-render) as fallback. |
| R3 | Vertex AI embedding quota limits during bulk migration re-embed | Medium | Medium | Batch size 250, exponential backoff, spread migration over multiple days. |
| R4 | Government portal robots.txt blocks or rate limit changes | Medium | Medium | Per-source configurable rate limits in `sources.config_json`. Manual override possible. |
| R5 | PostgreSQL HNSW index memory pressure at large scale (>10M chunks) | Low | High | Monitor index size. Partition by jurisdiction at V2 if needed. |
| R6 | tokio-cron-scheduler distributed locking gap (multiple crawler-service instances) | Low | Medium | Restrict crawler-service to max_instances=1 in v1 deployment config. |
| R7 | French/Italian/Spanish portals using non-standard HTML requiring bespoke scraping | High | Medium | BaseCrawler trait allows full customization. Budget extra time per new source crawler. |
| R8 | pgcrypto key rotation complexity | Low | Low | Document key rotation procedure. For v1, single symmetric key from env var is acceptable. |

### Known Technical Debt

| ID | Item | Priority | Planned For |
|----|------|----------|-------------|
| TD-1 | No distributed lock for cron scheduling — single crawler instance required | High | V2 (add `pg_advisory_lock` or leader election) |
| TD-2 | `metadata_json` type-specific fields not validated at DB level — only in application layer | Medium | V2 (JSON Schema validation or separate type tables) |
| TD-3 | MCP authentication is open (no auth) — vulnerable to abuse | Medium | V2 (add optional Bearer token auth to MCP endpoint) |
| TD-4 | `document_versions` has no size cap — unbounded growth over time | Low | V2 (add TTL-based archiving or configurable retention) |
| TD-5 | Frontend test coverage is not specified | Medium | V1.1 (add Playwright E2E tests for critical paths) |
| TD-6 | No automated accessibility audit in CI pipeline | Low | V1.1 (add axe-core checks) |

---

## Glossary

| Term | Definition |
|------|-----------|
| **HNSW** | Hierarchical Navigable Small World — a graph-based approximate nearest neighbour index algorithm used by pgvector. Better recall and no re-indexing vs. IVFFlat. |
| **pgvector** | PostgreSQL extension that adds a `vector` data type and distance operators (`<=>` cosine, `<->` L2, `<#>` inner product) for embedding search. |
| **Embedding** | A fixed-length numeric vector (768 dimensions) representing the semantic meaning of a text. Similar texts produce similar vectors (low cosine distance). |
| **Chunk** | A fragment of a legal document (max 512 tokens, 50-token overlap), the unit of indexing and retrieval. Each chunk has its own embedding vector. |
| **Ingestion Pipeline** | The multi-step process: fetch raw document → extract text → chunk → embed → store in PostgreSQL. Runs asynchronously via apalis. |
| **MCP** | Model Context Protocol — an open standard (by Anthropic) for connecting LLM clients to data sources and tools via JSON-RPC 2.0. |
| **Streamable HTTP** | The MCP transport protocol for web deployment: HTTP POST with optional streaming responses. Replaced the original stdio-only transport. |
| **rmcp** | Official Rust SDK for the Model Context Protocol (crate: `rmcp`). Provides `ServerHandler` trait and Axum integration. |
| **apalis** | Rust background job processing library backed by PostgreSQL. Used for async ingestion pipeline task execution. |
| **sqlx** | Rust async SQL toolkit with compile-time query verification via `query!` macros. No ORM — raw SQL. |
| **object_store** | Rust crate providing a unified interface to cloud object storage (GCS, Azure Blob, S3, local filesystem). |
| **text-splitter** | Rust crate implementing recursive character text splitting (similar to LangChain's `RecursiveCharacterTextSplitter`). Used for chunking. |
| **tokio-cron-scheduler** | Rust crate for scheduling tasks with cron syntax, backed by PostgreSQL for persistence across restarts. |
| **Bundesrecht** | German federal law portal (gesetze-im-internet.de). Publishes laws as XML feeds. |
| **Bayern.Recht** | Bavarian state law portal (gesetze-bayern.de). Publishes HTML. |
| **EUR-Lex** | Official EU law portal. Exposes a SPARQL endpoint and REST API for querying EU legislation. |
| **Legifrance** | French official legal portal. Publishes French laws, decrees, regulations. |
| **Normattiva** | Italian official legal portal. Publishes Italian laws (leggi). |
| **BOE** | Boletín Oficial del Estado — Spanish official state gazette. Publishes Spanish laws as XML. |
| **SHA-256** | Cryptographic hash function. Used to compute a content fingerprint for change detection without comparing full document text. |
| **ADC** | Application Default Credentials — GCP credential discovery mechanism. Service accounts authenticate via `GOOGLE_APPLICATION_CREDENTIALS` env var. |
| **EARS** | Easy Approach to Requirements Syntax — a structured natural language pattern for writing testable acceptance criteria (WHEN/THEN/SHALL). |
| **CON** | Constraint — a non-negotiable design boundary (CON-1 through CON-8 in this document). |
| **QR** | Quality Requirement — a measurable non-functional requirement (QR-P1, QR-R1, etc.). |
