-- Migration 0001: Initial schema
-- Creates core tables: sources, documents, chunks, crawl_runs, api_keys
-- Requires: pgvector extension

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS vector;

-- -------------------------
-- sources
-- -------------------------
CREATE TABLE sources (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL UNIQUE,
    jurisdiction    TEXT NOT NULL,
    language        TEXT NOT NULL,
    base_url        TEXT NOT NULL,
    crawler_type    TEXT NOT NULL,
    cron_schedule   TEXT NOT NULL DEFAULT '0 2 * * *',
    config_json     JSONB NOT NULL DEFAULT '{}',
    credentials_enc BYTEA,
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- -------------------------
-- documents
-- -------------------------
CREATE TABLE documents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id       UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    external_id     TEXT NOT NULL,
    url             TEXT NOT NULL,
    title           TEXT NOT NULL,
    content_md      TEXT NOT NULL,
    summary         TEXT,
    doc_type        TEXT NOT NULL,
    jurisdiction    TEXT NOT NULL,
    language        TEXT NOT NULL,
    published_at    TIMESTAMPTZ,
    effective_at    TIMESTAMPTZ,
    content_hash    TEXT NOT NULL,
    object_key      TEXT,
    metadata_json   JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_id, external_id)
);

CREATE INDEX idx_documents_source_id     ON documents(source_id);
CREATE INDEX idx_documents_jurisdiction  ON documents(jurisdiction);
CREATE INDEX idx_documents_doc_type      ON documents(doc_type);
CREATE INDEX idx_documents_published_at  ON documents(published_at DESC);
CREATE INDEX idx_documents_content_hash  ON documents(content_hash);

-- -------------------------
-- chunks
-- -------------------------
CREATE TABLE chunks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id     UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index     INTEGER NOT NULL,
    content         TEXT NOT NULL,
    embedding       vector(768),
    token_count     INTEGER NOT NULL,
    metadata_json   JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id, chunk_index)
);

-- HNSW index for cosine similarity search (ADR-3: m=16, ef_construction=64)
CREATE INDEX idx_chunks_embedding_hnsw
    ON chunks USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_chunks_document_id ON chunks(document_id);

-- -------------------------
-- crawl_runs
-- -------------------------
CREATE TABLE crawl_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id       UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ,
    status          TEXT NOT NULL DEFAULT 'running',
    docs_discovered INTEGER NOT NULL DEFAULT 0,
    docs_new        INTEGER NOT NULL DEFAULT 0,
    docs_updated    INTEGER NOT NULL DEFAULT 0,
    docs_unchanged  INTEGER NOT NULL DEFAULT 0,
    error_message   TEXT,
    metadata_json   JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_crawl_runs_source_id  ON crawl_runs(source_id);
CREATE INDEX idx_crawl_runs_started_at ON crawl_runs(started_at DESC);

-- -------------------------
-- api_keys
-- -------------------------
CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    key_prefix   TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ,
    enabled      BOOLEAN NOT NULL DEFAULT true
);
