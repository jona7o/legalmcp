//! Admin service — stats, API key creation and revocation.
//!
//! API keys are generated as 32 cryptographically-random bytes encoded in hex
//! (64-char lowercase string).  Only the SHA-256 hash is persisted; the raw
//! key is returned once and never stored.

use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use hex::ToHex;

use legal_core::errors::LegalMcpError;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Aggregate stats for the admin dashboard.
#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub document_count: i64,
    pub chunk_count: i64,
    pub source_count: i64,
}

/// Response returned once when a new API key is created.
/// The `key` field is the raw plaintext — it is never stored and never shown
/// again after this response.
#[derive(Debug, Serialize)]
pub struct CreatedApiKey {
    pub id: Uuid,
    pub name: String,
    /// Raw (plaintext) API key — return to caller only.
    pub key: String,
    /// First 8 chars of the key, stored for display.
    pub prefix: String,
    pub created_at: DateTime<Utc>,
}

/// Summarised API key row (no raw key).
#[derive(Debug, Serialize)]
pub struct ApiKeyRow {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    /// Optional ISO-8601 expiry.
    pub expires_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct AdminService {
    pool: PgPool,
}

impl AdminService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Return aggregate counts for the admin stats endpoint.
    pub async fn stats(&self) -> Result<AdminStats, LegalMcpError> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT \
             (SELECT COUNT(*) FROM documents) AS doc_count, \
             (SELECT COUNT(*) FROM chunks)    AS chunk_count, \
             (SELECT COUNT(*) FROM sources)   AS source_count",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AdminStats {
            document_count: row.get::<i64, _>("doc_count"),
            chunk_count: row.get::<i64, _>("chunk_count"),
            source_count: row.get::<i64, _>("source_count"),
        })
    }

    /// Generate a new API key, persist the hash, return the plaintext once.
    pub async fn create_api_key(
        &self,
        req: CreateApiKeyRequest,
    ) -> Result<CreatedApiKey, LegalMcpError> {
        // Generate 32 random bytes → 64-char lowercase hex string.
        let mut raw_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw_bytes);
        let raw_key: String = raw_bytes.encode_hex();
        let prefix = raw_key[..8].to_owned();

        // SHA-256 the raw key before storing.
        let key_hash: String = Sha256::digest(raw_key.as_bytes()).encode_hex();

        let row = sqlx::query!(
            r#"
            INSERT INTO api_keys (name, key_hash, key_prefix, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, key_prefix, created_at
            "#,
            req.name,
            key_hash,
            prefix,
            req.expires_at,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(CreatedApiKey {
            id: row.id,
            name: row.name,
            key: raw_key,
            prefix: row.key_prefix,
            created_at: row.created_at,
        })
    }

    /// List all API keys (without raw values).
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeyRow>, LegalMcpError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, key_prefix, created_at, last_used_at, expires_at, enabled
            FROM api_keys
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ApiKeyRow {
                id: r.id,
                name: r.name,
                key_prefix: r.key_prefix,
                created_at: r.created_at,
                last_used_at: r.last_used_at,
                expires_at: r.expires_at,
                enabled: r.enabled,
            })
            .collect())
    }

    /// Soft-delete: set `enabled = false` on the given key.
    ///
    /// Returns `LegalMcpError::NotFound` if the ID does not exist.
    pub async fn revoke_api_key(&self, id: Uuid) -> Result<(), LegalMcpError> {
        let result = sqlx::query!("UPDATE api_keys SET enabled = false WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(LegalMcpError::NotFound(format!("api_key {id}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_is_64_hex_chars() {
        let mut raw_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw_bytes);
        let key: String = raw_bytes.encode_hex();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prefix_is_first_8_chars() {
        let key = "abcdef1234567890".repeat(4); // 64 chars
        let prefix = &key[..8];
        assert_eq!(prefix, "abcdef12");
    }

    #[test]
    fn sha256_key_is_64_chars() {
        let raw = "some-api-key";
        let hash: String = Sha256::digest(raw.as_bytes()).encode_hex();
        assert_eq!(hash.len(), 64);
    }
}
