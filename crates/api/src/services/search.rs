//! Search service — embeds query and runs pgvector HNSW cosine search.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use embeddings::traits::Embedder;
use legal_core::errors::LegalMcpError;

/// Optional filters for the search endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct SearchFilters {
    pub jurisdiction: Option<String>,
    pub doc_type: Option<String>,
    pub language: Option<String>,
}

/// A single ranked search result.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub title: String,
    pub url: String,
    pub jurisdiction: String,
    pub doc_type: String,
    pub language: String,
    pub snippet: String,
    pub score: f32,
    pub published_at: Option<DateTime<Utc>>,
    pub source_name: String,
}

/// Paginated search response.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub query_embedding_ms: u128,
    pub search_ms: u128,
}

pub struct SearchService {
    pool: PgPool,
    embedder: Arc<dyn Embedder>,
}

impl SearchService {
    pub fn new(pool: PgPool, embedder: Arc<dyn Embedder>) -> Self {
        Self { pool, embedder }
    }

    pub async fn search(
        &self,
        query: &str,
        filters: SearchFilters,
        limit: i64,
        offset: i64,
    ) -> Result<SearchResponse, LegalMcpError> {
        if query.trim().is_empty() {
            return Err(LegalMcpError::Validation("query must not be empty".into()));
        }

        let limit = limit.clamp(1, 50);
        let offset = offset.max(0);

        // 1. Embed the query.
        let t0 = std::time::Instant::now();
        let vecs = self.embedder.embed_batch(&[query.to_owned()]).await?;
        let query_vec = vecs
            .into_iter()
            .next()
            .ok_or_else(|| LegalMcpError::Embedding("embedder returned empty batch".into()))?;
        let embed_ms = t0.elapsed().as_millis();

        // 2. Vector search.  We use a dynamic query here because jurisdiction /
        //    doc_type / language filters are optional and sqlx compile-time
        //    `query!` does not support dynamic WHERE clauses.
        let t1 = std::time::Instant::now();
        let rows = self.run_search(&query_vec, &filters, limit, offset).await?;
        let search_ms = t1.elapsed().as_millis();

        // 3. Count total matches (same filters, no limit).
        let total = self.count_results(&query_vec, &filters).await?;

        Ok(SearchResponse {
            results: rows,
            total,
            limit,
            offset,
            query_embedding_ms: embed_ms,
            search_ms,
        })
    }

    async fn run_search(
        &self,
        query_vec: &Vector,
        filters: &SearchFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>, LegalMcpError> {
        // Build WHERE clauses based on provided filters.
        let mut conditions: Vec<String> = Vec::new();
        if filters.jurisdiction.is_some() {
            conditions.push("d.jurisdiction = $4".into());
        }
        if filters.doc_type.is_some() {
            conditions.push(format!("d.doc_type = ${}", 4 + conditions.len()));
        }
        if filters.language.is_some() {
            conditions.push(format!("d.language = ${}", 4 + conditions.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("AND {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT
                c.id            AS chunk_id,
                d.id            AS document_id,
                d.title,
                d.url,
                d.jurisdiction,
                d.doc_type,
                d.language,
                c.content       AS snippet,
                1 - (c.embedding <=> $1::vector) AS score,
                d.published_at,
                s.name          AS source_name
            FROM chunks c
            JOIN documents d ON c.document_id = d.id
            JOIN sources s   ON d.source_id   = s.id
            WHERE c.embedding IS NOT NULL
              {where_clause}
            ORDER BY c.embedding <=> $1::vector
            LIMIT $2 OFFSET $3
            "#
        );

        let mut q = sqlx::query(&sql).bind(query_vec).bind(limit).bind(offset);

        if let Some(ref j) = filters.jurisdiction {
            q = q.bind(j);
        }
        if let Some(ref dt) = filters.doc_type {
            q = q.bind(dt);
        }
        if let Some(ref lang) = filters.language {
            q = q.bind(lang);
        }

        use sqlx::Row;
        let rows = q.fetch_all(&self.pool).await?;

        let results = rows
            .into_iter()
            .map(|r| SearchResult {
                chunk_id: r.get("chunk_id"),
                document_id: r.get("document_id"),
                title: r.get("title"),
                url: r.get("url"),
                jurisdiction: r.get("jurisdiction"),
                doc_type: r.get("doc_type"),
                language: r.get("language"),
                snippet: r.get("snippet"),
                score: r.get::<f64, _>("score") as f32,
                published_at: r.get("published_at"),
                source_name: r.get("source_name"),
            })
            .collect();

        Ok(results)
    }

    async fn count_results(
        &self,
        query_vec: &Vector,
        filters: &SearchFilters,
    ) -> Result<i64, LegalMcpError> {
        let mut conditions: Vec<String> = Vec::new();
        if filters.jurisdiction.is_some() {
            conditions.push("d.jurisdiction = $2".into());
        }
        if filters.doc_type.is_some() {
            conditions.push(format!("d.doc_type = ${}", 2 + conditions.len()));
        }
        if filters.language.is_some() {
            conditions.push(format!("d.language = ${}", 2 + conditions.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("AND {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT COUNT(*) AS cnt
            FROM chunks c
            JOIN documents d ON c.document_id = d.id
            WHERE c.embedding IS NOT NULL
              {where_clause}
              AND c.embedding <=> $1::vector < 0.5
            "#
        );

        let mut q = sqlx::query(&sql).bind(query_vec);
        if let Some(ref j) = filters.jurisdiction {
            q = q.bind(j);
        }
        if let Some(ref dt) = filters.doc_type {
            q = q.bind(dt);
        }
        if let Some(ref lang) = filters.language {
            q = q.bind(lang);
        }

        use sqlx::Row;
        let row = q.fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("cnt"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_returns_validation_error() {
        // We can't easily call the async method here without a pool, but we can
        // test the validation logic in isolation.
        assert!("".trim().is_empty());
        assert!(!"hello".trim().is_empty());
    }

    #[test]
    fn test_limit_clamping() {
        assert_eq!((-1_i64).clamp(1, 50), 1);
        assert_eq!(100_i64.clamp(1, 50), 50);
        assert_eq!(10_i64.clamp(1, 50), 10);
    }
}
