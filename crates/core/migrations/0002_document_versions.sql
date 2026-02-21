-- Migration 0002: Document versions table
-- Tracks content changes per document (ADR-12: SHA-256 content hash)

CREATE TABLE document_versions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id    UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    content_hash   TEXT NOT NULL,
    content_md     TEXT NOT NULL,
    changed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    change_summary TEXT,
    UNIQUE (document_id, version_number)
);

CREATE INDEX idx_document_versions_document_id ON document_versions(document_id);
CREATE INDEX idx_document_versions_changed_at  ON document_versions(changed_at DESC);
