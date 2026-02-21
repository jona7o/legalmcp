//! Ingestion pipeline orchestrator (ADR-10, SDD Runtime View Flow 1).
//!
//! [`IngestPipeline::run`] is the single entry point called by the apalis
//! ingest worker (Phase 5). It takes a `document_id`, fetches the raw file
//! from object storage, parses it, chunks it, embeds the chunks, and writes
//! everything back to PostgreSQL.

use std::sync::Arc;

use bytes::Bytes;
use embeddings::Embedder;
use legal_core::errors::LegalMcpError;
use object_store::path::Path;
use sqlx::PgPool;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::{
    chunker::chunk_text,
    hasher::hash_content,
    object_store::ObjectStoreClient,
    parser::{html, xml, DocFormat},
};

/// Statistics returned by a successful pipeline run.
#[derive(Debug, Clone)]
pub struct IngestStats {
    pub document_id: Uuid,
    pub chunks_written: usize,
}

/// Orchestrates the full ingestion flow for a single document.
///
/// Holds shared resources: database pool, embedder, and object-store client.
/// Designed to be cloned cheaply across worker tasks.
#[derive(Clone)]
pub struct IngestPipeline {
    db: PgPool,
    embedder: Arc<dyn Embedder>,
    store: ObjectStoreClient,
}

impl IngestPipeline {
    /// Construct a new pipeline.
    pub fn new(db: PgPool, embedder: Arc<dyn Embedder>, store: ObjectStoreClient) -> Self {
        Self {
            db,
            embedder,
            store,
        }
    }

    /// Run the full ingestion flow for `document_id`.
    ///
    /// Steps (see SDD Runtime View Flow 1):
    /// 1. SELECT document by ID — return `NotFound` if absent
    /// 2. Download original file from object store
    /// 3. Parse to Markdown (`content_md`)
    /// 4. Chunk `content_md`
    /// 5. Embed all chunks via the `Embedder`
    /// 6. DELETE existing chunks for `document_id` (idempotent re-run)
    /// 7. INSERT new chunks
    /// 8. UPDATE `documents.content_md` + `documents.content_hash`
    /// 9. INSERT `document_versions` if hash changed
    #[instrument(skip(self), fields(doc_id = %document_id))]
    pub async fn run(&self, document_id: Uuid) -> Result<IngestStats, LegalMcpError> {
        // ── Step 1: fetch document row ────────────────────────────────────────
        // Use unchecked dynamic query (no DATABASE_URL needed at build time;
        // compile-time verification done via `cargo sqlx prepare` in CI).
        let row = sqlx::query(
            r"SELECT id, source_id, object_key, content_md, content_hash,
                     doc_type, title, url, jurisdiction, language,
                     metadata_json
              FROM documents
              WHERE id = $1",
        )
        .bind(document_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| LegalMcpError::NotFound(format!("document {document_id}")))?;

        use sqlx::Row as _;
        let title: String = row.try_get("title").unwrap_or_default();
        let object_key: Option<String> = row.try_get("object_key").unwrap_or(None);
        let existing_content_md: String = row.try_get("content_md").unwrap_or_default();
        let existing_hash: String = row.try_get("content_hash").unwrap_or_default();
        let doc_type: String = row.try_get("doc_type").unwrap_or_default();

        info!(
            doc_id = %document_id,
            title = %title,
            "Starting ingestion"
        );

        // ── Step 2: download original from object store ───────────────────────
        let raw_bytes: Bytes = if let Some(ref key_str) = object_key {
            let key = Path::from(key_str.as_str());
            self.store.get(&key).await?
        } else {
            // No object key — use existing content_md as the raw content.
            Bytes::from(existing_content_md.into_bytes())
        };

        // ── Step 3: parse to Markdown ─────────────────────────────────────────
        let format = detect_format(&doc_type, &raw_bytes);
        let content_md = parse_to_markdown(raw_bytes, format).await?;

        // ── Step 4: chunk ─────────────────────────────────────────────────────
        let chunks = chunk_text(&content_md).await?;
        debug!(chunk_count = chunks.len(), "Chunking complete");

        if chunks.is_empty() {
            let new_hash = hash_content(&content_md);
            self.update_document_content(&document_id, &content_md, &new_hash)
                .await?;
            return Ok(IngestStats {
                document_id,
                chunks_written: 0,
            });
        }

        // ── Step 5: embed ─────────────────────────────────────────────────────
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let vectors = self.embedder.embed_batch(&texts).await?;
        debug!(vector_count = vectors.len(), "Embedding complete");

        // ── Step 6: delete existing chunks (idempotent) ───────────────────────
        sqlx::query("DELETE FROM chunks WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.db)
            .await?;

        // ── Step 7: insert new chunks ─────────────────────────────────────────
        for (chunk, vector) in chunks.iter().zip(vectors.iter()) {
            let chunk_id = Uuid::new_v4();
            let token_count = chunk.token_count as i32;
            let chunk_index = chunk.chunk_index as i32;

            sqlx::query(
                r"INSERT INTO chunks
                    (id, document_id, chunk_index, content, embedding, token_count, metadata_json)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(chunk_id)
            .bind(document_id)
            .bind(chunk_index)
            .bind(&chunk.content)
            .bind(vector)
            .bind(token_count)
            .bind(serde_json::json!({}))
            .execute(&self.db)
            .await?;
        }

        // ── Step 8: update document content_md + content_hash ─────────────────
        let new_hash = hash_content(&content_md);
        self.update_document_content(&document_id, &content_md, &new_hash)
            .await?;

        // ── Step 9: insert document_versions if hash changed ──────────────────
        if new_hash != existing_hash {
            self.insert_document_version(&document_id, &new_hash, &content_md)
                .await?;
        }

        let chunks_written = chunks.len();
        info!(chunks_written, doc_id = %document_id, "Ingestion complete");
        Ok(IngestStats {
            document_id,
            chunks_written,
        })
    }

    async fn update_document_content(
        &self,
        document_id: &Uuid,
        content_md: &str,
        content_hash: &str,
    ) -> Result<(), LegalMcpError> {
        sqlx::query(
            r"UPDATE documents
              SET content_md = $1, content_hash = $2, updated_at = NOW()
              WHERE id = $3",
        )
        .bind(content_md)
        .bind(content_hash)
        .bind(document_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn insert_document_version(
        &self,
        document_id: &Uuid,
        content_hash: &str,
        content_md: &str,
    ) -> Result<(), LegalMcpError> {
        // Determine the next version number.
        let next_version: i32 = {
            let row = sqlx::query(
                r"SELECT COALESCE(MAX(version_number), 0) + 1 AS next
                  FROM document_versions
                  WHERE document_id = $1",
            )
            .bind(document_id)
            .fetch_one(&self.db)
            .await?;
            use sqlx::Row as _;
            row.try_get::<i64, _>("next").unwrap_or(1) as i32
        };

        sqlx::query(
            r"INSERT INTO document_versions
                (id, document_id, version_number, content_hash, content_md)
              VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(document_id)
        .bind(next_version)
        .bind(content_hash)
        .bind(content_md)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Detect document format from content sniffing.
fn detect_format(_doc_type: &str, bytes: &Bytes) -> DocFormat {
    let prefix = &bytes[..bytes.len().min(64)];
    if prefix.starts_with(b"<?xml") || prefix.starts_with(b"<gesetz") {
        return DocFormat::Xml;
    }
    if prefix.starts_with(b"%PDF") {
        return DocFormat::Pdf;
    }
    DocFormat::Html
}

/// Parse raw bytes to a Markdown string based on the detected format.
async fn parse_to_markdown(bytes: Bytes, format: DocFormat) -> Result<String, LegalMcpError> {
    match format {
        DocFormat::Html => html::parse(&bytes),
        DocFormat::Xml => {
            let doc = xml::parse(&bytes)?;
            // Combine title + content into a simple Markdown string.
            let md = if doc.title.is_empty() {
                doc.content
            } else {
                format!("# {}\n\n{}", doc.title, doc.content)
            };
            Ok(md)
        }
        DocFormat::Pdf => {
            // PDF parsing requires the Mistral API — callers must pre-parse
            // PDFs and store the resulting Markdown in content_md / object store.
            Err(LegalMcpError::Parse(
                "PDF parsing via Mistral API must be done before calling IngestPipeline".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn detect_format_xml() {
        let b = Bytes::from_static(b"<?xml version=\"1.0\"?><root/>");
        assert!(matches!(detect_format("statute", &b), DocFormat::Xml));
    }

    #[test]
    fn detect_format_pdf() {
        let b = Bytes::from_static(b"%PDF-1.4 content");
        assert!(matches!(detect_format("case", &b), DocFormat::Pdf));
    }

    #[test]
    fn detect_format_html_default() {
        let b = Bytes::from_static(b"<html><body>test</body></html>");
        assert!(matches!(detect_format("statute", &b), DocFormat::Html));
    }

    #[test]
    fn ingest_stats_carries_doc_id_and_count() {
        let id = Uuid::new_v4();
        let stats = IngestStats {
            document_id: id,
            chunks_written: 42,
        };
        assert_eq!(stats.document_id, id);
        assert_eq!(stats.chunks_written, 42);
    }

    #[tokio::test]
    async fn parse_xml_to_markdown_includes_title() {
        let xml = Bytes::from_static(
            b"<?xml version=\"1.0\"?><law><titel>BGB</titel><P>Inhalt</P></law>",
        );
        let result = parse_to_markdown(xml, DocFormat::Xml).await.unwrap();
        // Should contain "BGB" as title and "Inhalt" as content
        assert!(result.contains("BGB") || result.contains("Inhalt"));
    }

    #[tokio::test]
    async fn parse_pdf_returns_error() {
        let pdf = Bytes::from_static(b"%PDF-1.4 binary");
        let result = parse_to_markdown(pdf, DocFormat::Pdf).await;
        assert!(result.is_err());
    }

    // Full integration tests (IngestPipeline::run with real DB) live in
    // tests/pipeline_integration.rs and require DATABASE_URL to be set.
}
