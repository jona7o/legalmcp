//! Auth middleware — Bearer API-key validation.
//!
//! The raw key sent by the client is SHA-256 hashed and compared (constant-
//! time) against the `key_hash` column in `api_keys`.  If the key is valid the
//! `last_used_at` timestamp is updated.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use hex::ToHex;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use legal_core::errors::{LegalMcpError, ProblemDetail};
use sqlx::PgPool;

use crate::state::AppState;

/// Axum middleware that validates a `Bearer <token>` API key.
///
/// Extracts the pool from the shared `AppState` and validates the Bearer
/// token against the `api_keys` table using a constant-time comparison.
pub async fn require_api_key(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::response::Json<ProblemDetail>)> {
    let raw = extract_bearer(req.headers()).ok_or_else(|| {
        let pd = LegalMcpError::Unauthorized.to_problem_detail();
        (StatusCode::UNAUTHORIZED, axum::response::Json(pd))
    })?;

    validate_key(&state.pool, &raw).await.map_err(|e| {
        let pd = e.to_problem_detail();
        let status = StatusCode::from_u16(pd.status).unwrap_or(StatusCode::UNAUTHORIZED);
        (status, axum::response::Json(pd))
    })?;

    Ok(next.run(req).await)
}

/// Extract the raw token string from an `Authorization: Bearer <token>` header.
fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<String> {
    let val = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    val.strip_prefix("Bearer ").map(|s| s.to_owned())
}

/// Hash the raw key and look it up in `api_keys`.
async fn validate_key(pool: &PgPool, raw: &str) -> Result<(), LegalMcpError> {
    let hash: String = Sha256::digest(raw.as_bytes()).encode_hex();

    let row = sqlx::query!(
        "SELECT key_hash FROM api_keys WHERE enabled = true AND (expires_at IS NULL OR expires_at > NOW())"
    )
    .fetch_all(pool)
    .await?;

    // Constant-time comparison against every enabled key.
    let hash_bytes = hash.as_bytes();
    let matched = row.iter().any(|r| {
        let db_bytes = r.key_hash.as_bytes();
        // Both are hex strings of the same length (64 chars); ct_eq is safe here.
        if hash_bytes.len() != db_bytes.len() {
            return false;
        }
        hash_bytes.ct_eq(db_bytes).into()
    });

    if !matched {
        return Err(LegalMcpError::Unauthorized);
    }

    // Best-effort update of last_used_at (ignore errors).
    let _ = sqlx::query!(
        "UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1",
        hash
    )
    .execute(pool)
    .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_extract_bearer_present() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer my-secret-key".parse().unwrap(),
        );
        assert_eq!(extract_bearer(&headers), Some("my-secret-key".to_owned()));
    }

    #[test]
    fn test_extract_bearer_missing() {
        assert_eq!(extract_bearer(&HeaderMap::new()), None);
    }

    #[test]
    fn test_extract_bearer_wrong_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Basic dXNlcjpwYXNz".parse().unwrap(),
        );
        assert_eq!(extract_bearer(&headers), None);
    }

    #[test]
    fn test_key_hashes_to_sha256() {
        // SHA-256("test") = 9f86d081...
        let hash: String = Sha256::digest(b"test").encode_hex();
        assert_eq!(hash.len(), 64);
        assert!(hash.starts_with("9f86d081"));
    }
}
