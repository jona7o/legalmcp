//! Source service — CRUD for sources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use legal_core::errors::LegalMcpError;

#[derive(Debug, Serialize)]
pub struct SourceRow {
    pub id: Uuid,
    pub name: String,
    pub jurisdiction: String,
    pub language: String,
    pub base_url: String,
    pub crawler_type: String,
    pub cron_schedule: String,
    pub config_json: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSourceRequest {
    pub name: String,
    pub jurisdiction: String,
    pub language: String,
    pub base_url: String,
    pub crawler_type: String,
    pub cron_schedule: Option<String>,
    pub config_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSourceRequest {
    pub enabled: Option<bool>,
    pub cron_schedule: Option<String>,
    pub config_json: Option<serde_json::Value>,
}

pub struct SourceService {
    pool: PgPool,
}

impl SourceService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_sources(&self) -> Result<Vec<SourceRow>, LegalMcpError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, jurisdiction, language, base_url, crawler_type,
                   cron_schedule, config_json, enabled, created_at, updated_at
            FROM sources
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SourceRow {
                id: r.id,
                name: r.name,
                jurisdiction: r.jurisdiction,
                language: r.language,
                base_url: r.base_url,
                crawler_type: r.crawler_type,
                cron_schedule: r.cron_schedule,
                config_json: r.config_json,
                enabled: r.enabled,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn get_source(&self, id: Uuid) -> Result<SourceRow, LegalMcpError> {
        let r = sqlx::query!(
            r#"
            SELECT id, name, jurisdiction, language, base_url, crawler_type,
                   cron_schedule, config_json, enabled, created_at, updated_at
            FROM sources
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| LegalMcpError::NotFound(format!("source {id}")))?;

        Ok(SourceRow {
            id: r.id,
            name: r.name,
            jurisdiction: r.jurisdiction,
            language: r.language,
            base_url: r.base_url,
            crawler_type: r.crawler_type,
            cron_schedule: r.cron_schedule,
            config_json: r.config_json,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    pub async fn create_source(
        &self,
        req: CreateSourceRequest,
    ) -> Result<SourceRow, LegalMcpError> {
        let cron = req.cron_schedule.unwrap_or_else(|| "0 2 * * *".into());
        let config = req.config_json.unwrap_or(serde_json::json!({}));

        let r = sqlx::query!(
            r#"
            INSERT INTO sources (name, jurisdiction, language, base_url, crawler_type, cron_schedule, config_json)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, name, jurisdiction, language, base_url, crawler_type,
                      cron_schedule, config_json, enabled, created_at, updated_at
            "#,
            req.name,
            req.jurisdiction,
            req.language,
            req.base_url,
            req.crawler_type,
            cron,
            config,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SourceRow {
            id: r.id,
            name: r.name,
            jurisdiction: r.jurisdiction,
            language: r.language,
            base_url: r.base_url,
            crawler_type: r.crawler_type,
            cron_schedule: r.cron_schedule,
            config_json: r.config_json,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    pub async fn update_source(
        &self,
        id: Uuid,
        req: UpdateSourceRequest,
    ) -> Result<SourceRow, LegalMcpError> {
        // Fetch existing to apply partial update.
        let existing = self.get_source(id).await?;

        let enabled = req.enabled.unwrap_or(existing.enabled);
        let cron = req.cron_schedule.unwrap_or(existing.cron_schedule);
        let config = req.config_json.unwrap_or(existing.config_json);

        let r = sqlx::query!(
            r#"
            UPDATE sources
            SET enabled = $2, cron_schedule = $3, config_json = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, jurisdiction, language, base_url, crawler_type,
                      cron_schedule, config_json, enabled, created_at, updated_at
            "#,
            id,
            enabled,
            cron,
            config,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SourceRow {
            id: r.id,
            name: r.name,
            jurisdiction: r.jurisdiction,
            language: r.language,
            base_url: r.base_url,
            crawler_type: r.crawler_type,
            cron_schedule: r.cron_schedule,
            config_json: r.config_json,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    pub async fn delete_source(&self, id: Uuid) -> Result<(), LegalMcpError> {
        // Verify the source exists first to return 404 for unknown IDs.
        let _ = self.get_source(id).await?;

        sqlx::query!("DELETE FROM sources WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
