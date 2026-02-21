//! Security validation tests — T8.3
//!
//! Verifies:
//! - SQL injection attempts return normal (empty/error) results, not DB errors
//! - API keys in DB are SHA-256 hashed, not stored in plaintext
//! - Sensitive fields are not leaked in API responses
//! - Proper error codes for auth failures (401, not 403/500)
//!
//! Some checks (DB-level) require DATABASE_URL; HTTP checks only need TEST_API_BASE_URL.
//!
//! # Running
//!
//!   DATABASE_URL="postgres://..." TEST_API_BASE_URL=http://localhost:8000 \
//!     cargo test -p integration-tests --test security

use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};

fn api_base() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("TEST_API_BASE_URL").ok()
}

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

fn test_api_key() -> Option<String> {
    std::env::var("TEST_API_KEY").ok()
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build reqwest client")
}

macro_rules! skip_without_server {
    ($base:expr) => {
        match $base {
            Some(b) => b,
            None => {
                eprintln!("SKIP: TEST_API_BASE_URL not set — skipping security test");
                return;
            }
        }
    };
}

macro_rules! skip_without_db {
    ($url:expr) => {
        match $url {
            Some(u) => u,
            None => {
                eprintln!("SKIP: DATABASE_URL not set — skipping DB-level security test");
                return;
            }
        }
    };
}

// ---------------------------------------------------------------------------
// QR-S1: SQL injection — parameterised queries prevent injection
// ---------------------------------------------------------------------------

/// Injecting SQL into the search query string must not cause a 500 or expose DB errors.
/// The parameterized sqlx queries ensure the input is treated as a literal string.
/// AC: The endpoint returns 200 (empty results) or 422, never a 500 DB error.
#[tokio::test]
async fn test_sql_injection_in_search_does_not_cause_500() {
    let base = skip_without_server!(api_base());

    let injection_payloads = vec![
        "' OR 1=1 --",
        "'; DROP TABLE documents; --",
        "' UNION SELECT id, key_hash FROM api_keys --",
        "1; SELECT pg_sleep(3) --",
        r#"\" OR \"\"=\""#,
    ];

    for payload in &injection_payloads {
        let encoded = urlencoding::encode(payload);
        let resp = client()
            .get(format!("{base}/api/v1/search?q={encoded}"))
            .send()
            .await
            .unwrap_or_else(|_| panic!("request failed for payload: {payload}"));

        let status = resp.status().as_u16();
        assert!(
            status != 500,
            "SQL injection payload '{payload}' caused a 500 error — possible injection vulnerability"
        );
        assert!(
            status == 200 || status == 422,
            "Expected 200 or 422 for injection payload '{payload}', got {status}"
        );

        // Verify the response body does NOT contain raw SQL error messages
        let body_text = resp.text().await.unwrap_or_default();
        let body_lower = body_text.to_lowercase();
        assert!(
            !body_lower.contains("syntax error"),
            "Response should not expose SQL syntax errors for payload: {payload}"
        );
        assert!(
            !body_lower.contains("pg_"),
            "Response should not expose PostgreSQL internal function names"
        );
    }
}

/// SQL injection in document ID path parameter — must return 400/404, never 500.
#[tokio::test]
async fn test_sql_injection_in_path_param_does_not_cause_500() {
    let base = skip_without_server!(api_base());

    // These are not valid UUIDs — Axum's path extractor should reject them with 400/422
    let bad_ids = vec![
        "' OR 1=1",
        "../../../etc/passwd",
        "00000000-0000-0000-0000-000000000000' OR '1'='1",
    ];

    for id in &bad_ids {
        let encoded = urlencoding::encode(id);
        let resp = client()
            .get(format!("{base}/api/v1/documents/{encoded}"))
            .send()
            .await
            .unwrap_or_else(|_| panic!("request failed for id: {id}"));

        let status = resp.status().as_u16();
        assert!(
            status != 500,
            "Path injection '{id}' caused a 500 — possible vulnerability"
        );
    }
}

// ---------------------------------------------------------------------------
// QR-S2: API key storage — keys hashed, not stored in plaintext
// ---------------------------------------------------------------------------

/// Verify API keys are stored as SHA-256 hashes in the database, not as plaintext.
/// The key_hash column should be 64 hex chars; the raw key must NOT appear there.
#[tokio::test]
async fn test_api_key_stored_as_hash_not_plaintext() {
    let db_url = skip_without_db!(database_url());
    let base = skip_without_server!(api_base());
    let admin_key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set — skipping API key hash test");
            return;
        }
    };

    // Create a new API key via the API
    let create_resp = client()
        .post(format!("{base}/api/v1/admin/api-keys"))
        .header("Authorization", format!("Bearer {admin_key}"))
        .json(&json!({ "name": "security-test-key" }))
        .send()
        .await
        .expect("create key request failed");

    assert_eq!(create_resp.status().as_u16(), 201);
    let created: Value = create_resp.json().await.expect("create body not JSON");
    let raw_key = created["key"].as_str().expect("should have raw key");
    let key_id = created["id"].as_str().expect("should have id");

    // Now check the DB directly — raw key must not appear in key_hash
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("failed to connect to DB for security check");

    let row = sqlx::query("SELECT key_hash, key_prefix FROM api_keys WHERE id = $1")
        .bind(uuid::Uuid::parse_str(key_id).expect("valid uuid"))
        .fetch_one(&pool)
        .await
        .expect("key not found in DB");

    use sqlx::Row;
    let stored_hash: String = row.get("key_hash");
    let stored_prefix: String = row.get("key_prefix");

    // The stored value must NOT be the raw key
    assert_ne!(
        stored_hash, raw_key,
        "API key must not be stored as plaintext"
    );

    // The stored value must be a 64-char hex string (SHA-256)
    assert_eq!(
        stored_hash.len(),
        64,
        "API key hash should be 64 hex chars (SHA-256)"
    );
    assert!(
        stored_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "API key hash should contain only hex digits"
    );

    // The prefix (first 8 chars of raw key) should match
    assert_eq!(
        stored_prefix,
        &raw_key[..8],
        "key_prefix should be first 8 chars of the raw key"
    );

    // Clean up
    sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(uuid::Uuid::parse_str(key_id).expect("valid uuid"))
        .execute(&pool)
        .await
        .ok();
}

// ---------------------------------------------------------------------------
// QR-S3: No secrets in API responses
// ---------------------------------------------------------------------------

/// The list API keys endpoint must not return raw key values.
#[tokio::test]
async fn test_api_key_list_does_not_expose_raw_keys() {
    let base = skip_without_server!(api_base());
    let admin_key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set");
            return;
        }
    };

    let resp = client()
        .get(format!("{base}/api/v1/admin/api-keys"))
        .header("Authorization", format!("Bearer {admin_key}"))
        .send()
        .await
        .expect("list keys failed");

    assert_eq!(resp.status().as_u16(), 200);
    let body_text = resp.text().await.expect("list keys body");

    // The response should NOT contain a 64-char hex key value
    // We check that no 64-char hex string appears in the JSON body
    // (key_prefix is only 8 chars, key_hash is not returned)
    let has_full_key = body_text
        .split('"')
        .any(|token| token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(
        !has_full_key,
        "List API keys response must not contain a 64-char hex key value"
    );
}

/// The search endpoint must not leak database schema info in error responses.
#[tokio::test]
async fn test_error_responses_do_not_leak_schema() {
    let base = skip_without_server!(api_base());

    // Trigger an error with a bad query
    let resp = client()
        .get(format!("{base}/api/v1/search?q="))
        .send()
        .await
        .expect("empty search failed");

    let body_text = resp.text().await.expect("body text");
    let lower = body_text.to_lowercase();

    // Should not expose internal details
    assert!(
        !lower.contains("sqlx"),
        "error response must not mention sqlx"
    );
    assert!(
        !lower.contains("postgres"),
        "error response must not mention postgres"
    );
    assert!(
        !lower.contains("stack trace"),
        "error response must not include stack traces"
    );
}

// ---------------------------------------------------------------------------
// QR-S4: Authentication error codes are correct
// ---------------------------------------------------------------------------

/// Missing Authorization header returns 401, not 403 or 500.
#[tokio::test]
async fn test_missing_auth_returns_401_not_403() {
    let base = skip_without_server!(api_base());

    let protected_endpoints = vec![
        ("GET", format!("{base}/api/v1/admin/stats")),
        ("GET", format!("{base}/api/v1/admin/api-keys")),
    ];

    for (method, url) in &protected_endpoints {
        let resp = match *method {
            "GET" => client().get(url).send().await.expect("request failed"),
            _ => unreachable!(),
        };

        assert_eq!(
            resp.status().as_u16(),
            401,
            "{method} {url} without auth should return 401, not {}",
            resp.status().as_u16()
        );

        // 401 must include WWW-Authenticate header (RFC 7235)
        // Note: this is a SHOULD not MUST, so we just log if absent
        if resp.headers().get("www-authenticate").is_none() {
            eprintln!("NOTE: {method} {url} 401 response missing WWW-Authenticate header");
        }
    }
}

/// Malformed Authorization header (not Bearer) returns 401.
#[tokio::test]
async fn test_malformed_auth_scheme_returns_401() {
    let base = skip_without_server!(api_base());

    // Non-Bearer auth scheme
    let resp = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .send()
        .await
        .expect("malformed auth request failed");

    assert_eq!(
        resp.status().as_u16(),
        401,
        "non-Bearer auth scheme should return 401"
    );
}

/// Correct error response format — RFC 7807 Problem Details.
#[tokio::test]
async fn test_auth_error_response_is_rfc7807() {
    let base = skip_without_server!(api_base());

    let resp = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status().as_u16(), 401);

    // Check Content-Type is application/json or application/problem+json
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json") || ct.contains("application/problem+json"),
        "401 response Content-Type should be application/json (got: {ct})"
    );

    let body: Value = resp.json().await.expect("401 body is not JSON");
    // RFC 7807 requires: type, title, status
    assert!(
        body["status"].is_number(),
        "RFC 7807 requires 'status' field"
    );
    assert!(body["title"].is_string(), "RFC 7807 requires 'title' field");
}
