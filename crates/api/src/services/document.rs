//! Document service — fetch documents, versions, and chunks from the DB.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use legal_core::errors::LegalMcpError;

/// Full document response (no embeddings).
#[derive(Debug, Serialize)]
pub struct DocumentRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub external_id: String,
    pub url: String,
    pub title: String,
    pub content_md: String,
    pub summary: Option<String>,
    pub doc_type: String,
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

/// Document version row.
#[derive(Debug, Serialize)]
pub struct DocumentVersionRow {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_number: i32,
    pub content_hash: String,
    pub changed_at: DateTime<Utc>,
    pub change_summary: Option<String>,
}

/// Chunk row (no embedding vector).
#[derive(Debug, Serialize)]
pub struct ChunkRow {
    pub id: Uuid,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: i32,
    pub metadata_json: serde_json::Value,
}

pub struct DocumentService {
    pool: PgPool,
}

impl DocumentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_document(&self, id: Uuid) -> Result<DocumentRow, LegalMcpError> {
        let row = sqlx::query!(
            r#"
            SELECT id, source_id, external_id, url, title, content_md, summary,
                   doc_type, jurisdiction, language, published_at, effective_at,
                   content_hash, object_key, metadata_json,
                   created_at, updated_at
            FROM documents
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| LegalMcpError::NotFound(format!("document {id}")))?;

        Ok(DocumentRow {
            id: row.id,
            source_id: row.source_id,
            external_id: row.external_id,
            url: row.url,
            title: row.title,
            content_md: row.content_md,
            summary: row.summary,
            doc_type: row.doc_type,
            jurisdiction: row.jurisdiction,
            language: row.language,
            published_at: row.published_at,
            effective_at: row.effective_at,
            content_hash: row.content_hash,
            object_key: row.object_key,
            metadata_json: row.metadata_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn get_versions(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<DocumentVersionRow>, LegalMcpError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, document_id, version_number, content_hash, changed_at, change_summary
            FROM document_versions
            WHERE document_id = $1
            ORDER BY changed_at DESC
            "#,
            document_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DocumentVersionRow {
                id: r.id,
                document_id: r.document_id,
                version_number: r.version_number,
                content_hash: r.content_hash,
                changed_at: r.changed_at,
                change_summary: r.change_summary,
            })
            .collect())
    }

    pub async fn get_chunks(&self, document_id: Uuid) -> Result<Vec<ChunkRow>, LegalMcpError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, chunk_index, content, token_count, metadata_json
            FROM chunks
            WHERE document_id = $1
            ORDER BY chunk_index
            "#,
            document_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ChunkRow {
                id: r.id,
                chunk_index: r.chunk_index,
                content: r.content,
                token_count: r.token_count,
                metadata_json: r.metadata_json,
            })
            .collect())
    }
}
