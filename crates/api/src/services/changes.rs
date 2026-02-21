//! Changes service — returns documents that changed after a given timestamp.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use legal_core::errors::LegalMcpError;

/// One changed-document entry returned by the changes endpoint.
#[derive(Debug, Serialize)]
pub struct ChangedDocument {
    pub document_id: Uuid,
    pub source_id: Uuid,
    pub title: String,
    pub url: String,
    pub jurisdiction: String,
    pub language: String,
    pub version_number: i32,
    pub content_hash: String,
    pub changed_at: DateTime<Utc>,
    pub change_summary: Option<String>,
}

pub struct ChangesService {
    pool: PgPool,
}

impl ChangesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Return documents that have at least one `document_versions` entry with
    /// `version_number > 1` *and* `changed_at >= since`.
    ///
    /// Results are ordered newest-first, limited to `limit` rows with `offset`.
    pub async fn list_changes(
        &self,
        since: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChangedDocument>, LegalMcpError> {
        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);

        // Use dynamic query because the date filter is always present and
        // version_number filter is static.  sqlx query! would need the DB
        // at compile time; defer to runtime binding here.
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT
                dv.id             AS version_id,
                dv.document_id,
                dv.version_number,
                dv.content_hash,
                dv.changed_at,
                dv.change_summary,
                d.source_id,
                d.title,
                d.url,
                d.jurisdiction,
                d.language
            FROM document_versions dv
            JOIN documents d ON dv.document_id = d.id
            WHERE dv.version_number > 1
              AND dv.changed_at >= $1
            ORDER BY dv.changed_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(since)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|r| ChangedDocument {
                document_id: r.get("document_id"),
                source_id: r.get("source_id"),
                title: r.get("title"),
                url: r.get("url"),
                jurisdiction: r.get("jurisdiction"),
                language: r.get("language"),
                version_number: r.get("version_number"),
                content_hash: r.get("content_hash"),
                changed_at: r.get("changed_at"),
                change_summary: r.get("change_summary"),
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_clamping_changes() {
        assert_eq!(0_i64.clamp(1, 200), 1);
        assert_eq!(500_i64.clamp(1, 200), 200);
        assert_eq!(50_i64.clamp(1, 200), 50);
    }
}
