//! Integration tests for the API service.
//!
//! These tests exercise the full Axum router with a real PostgreSQL connection
//! and `MockEmbedder`.  They use `tower::ServiceExt` to call the router
//! in-process — no HTTP listener required.
//!
//! # Requirements
//! The `DATABASE_URL` environment variable (or a `.env` file at the workspace
//! root) must point to a running PostgreSQL instance with the migrations
//! applied.  The test suite seeds a temporary API key and cleans it up
//! afterwards.
//!
//! Run with:
//! ```
//! DATABASE_URL="postgres://legal:legal_dev_password@localhost:5433/legalmcp" \
//!   cargo test -p api --test api_integration_tests
//! ```

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt; // `oneshot` extension
use uuid::Uuid;

use api::{router::app_router, state::AppState};
use embeddings::MockEmbedder;
use ingest::object_store::ObjectStoreClient;
use legal_core::{config::AppConfig, db::create_pool};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Create application state backed by real PostgreSQL and `MockEmbedder`.
async fn make_test_state() -> Arc<AppState> {
    dotenvy::dotenv().ok();
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = create_pool(&database_url)
        .await
        .expect("failed to connect to test database");

    let config = AppConfig {
        database_url,
        vertex_ai_project: String::new(),
        vertex_ai_location: String::new(),
        google_application_credentials: String::new(),
        mistral_api_key: String::new(),
        object_store_type: "memory".into(),
        gcs_bucket: String::new(),
        azure_storage_account: String::new(),
        azure_storage_key: String::new(),
        azure_container: String::new(),
        api_port: 8000,
        max_db_connections: 5,
        rust_log: "error".into(),
    };

    let object_store =
        ObjectStoreClient::from_config(&config).expect("failed to create object store");
    AppState::new(pool, Arc::new(MockEmbedder), object_store, config)
}

/// Seed a single API key and return the raw key + its DB id.
async fn seed_api_key(state: &Arc<AppState>, name: &str) -> (String, Uuid) {
    use hex::ToHex;
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    let mut raw_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw_bytes);
    let raw_key: String = raw_bytes.encode_hex();
    let key_hash: String = Sha256::digest(raw_key.as_bytes()).encode_hex();
    let key_prefix = &raw_key[..8];

    // Use dynamic (non-macro) query to avoid needing this file in .sqlx/ cache.
    let row = sqlx::query(
        "INSERT INTO api_keys (name, key_hash, key_prefix, enabled) VALUES ($1, $2, $3, true) RETURNING id",
    )
    .bind(name)
    .bind(&key_hash)
    .bind(key_prefix)
    .fetch_one(&state.pool)
    .await
    .expect("failed to seed API key");

    use sqlx::Row;
    let id: Uuid = row.get("id");
    (raw_key, id)
}

/// Remove a seeded API key by id.
async fn cleanup_api_key(state: &Arc<AppState>, id: Uuid) {
    // Use dynamic (non-macro) query to avoid needing this file in .sqlx/ cache.
    sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .unwrap();
}

/// Parse the response body as JSON.
async fn body_json(body: Body) -> Value {
    let bytes = body
        .collect()
        .await
        .expect("failed to read response body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("response body is not valid JSON")
}

// ---------------------------------------------------------------------------
// Test 1 — Health check returns 200 with {"status":"ok"}
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_returns_200_ok() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], "ok");
}

// ---------------------------------------------------------------------------
// Test 2 — Auth middleware returns 401 for missing Bearer token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_protected_route_missing_token_returns_401() {
    let state = make_test_state().await;
    let app = app_router(state);

    // POST /api/v1/sources requires an API key (it is a mutating source endpoint).
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sources")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"name":"test","jurisdiction":"DE","language":"de","base_url":"http://example.com","crawler_type":"bundesrecht"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], 401);
}

// ---------------------------------------------------------------------------
// Test 3 — Valid API key returns 200 on a protected read endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_valid_api_key_allows_access() {
    let state = make_test_state().await;
    let (raw_key, key_id) = seed_api_key(&state, "integration-test-key-3").await;

    let app = app_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources")
                .header("Authorization", format!("Bearer {raw_key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // sources listing is public but should succeed (not 401) with a key too
    assert!(
        response.status() != StatusCode::UNAUTHORIZED,
        "expected non-401 with valid key, got {}",
        response.status()
    );

    cleanup_api_key(&state, key_id).await;
}

// ---------------------------------------------------------------------------
// Test 4 — Search endpoint returns 422 for empty query
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_search_empty_query_returns_422() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], 422, "expected 422 in problem detail body");
}

// ---------------------------------------------------------------------------
// Test 5 — Documents endpoint returns 404 for unknown UUID
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_document_unknown_uuid_returns_404() {
    let state = make_test_state().await;
    let app = app_router(state);

    let unknown_id = Uuid::new_v4();
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{unknown_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], 404);
    assert!(
        json["problem_type"]
            .as_str()
            .unwrap_or_default()
            .contains("not-found")
            || json["type"]
                .as_str()
                .unwrap_or_default()
                .contains("not-found"),
        "problem type should contain 'not-found', got: {json}"
    );
}

// ---------------------------------------------------------------------------
// Test 6 — Invalid Bearer token returns 401 on admin endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_invalid_api_key_returns_401() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/stats")
                .header("Authorization", "Bearer definitely-not-a-valid-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Test 7 — Readiness check returns 200 when DB is up
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ready_returns_200_when_db_up() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], "ok");
}

// ---------------------------------------------------------------------------
// Test 8 — Sources list is publicly accessible (GET, no auth required)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sources_list_is_public() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await;
    assert!(json.is_array(), "sources list should be a JSON array");
}

// ---------------------------------------------------------------------------
// Test 9 — Admin stats requires API key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_admin_stats_requires_api_key() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Test 10 — Admin stats returns valid stats when authenticated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_admin_stats_authenticated() {
    let state = make_test_state().await;
    let (raw_key, key_id) = seed_api_key(&state, "integration-test-stats-10").await;

    let app = app_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/stats")
                .header("Authorization", format!("Bearer {raw_key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await;
    assert!(
        json["document_count"].is_number(),
        "stats should include document_count"
    );
    assert!(
        json["chunk_count"].is_number(),
        "stats should include chunk_count"
    );
    assert!(
        json["source_count"].is_number(),
        "stats should include source_count"
    );

    cleanup_api_key(&state, key_id).await;
}

// ---------------------------------------------------------------------------
// Test 11 — Unknown route returns 404
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/this/does/not/exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Test 12 — Search with whitespace-only query returns 422
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_search_whitespace_query_returns_422() {
    let state = make_test_state().await;
    let app = app_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=%20%20%20") // URL-encoded "   "
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
