//! Cross-service API integration tests — T8.1
//!
//! Tests the full stack via HTTP against a running API service.
//!
//! # Running
//!
//! Start the stack first:
//!   docker compose up -d postgres api-service
//!
//! Then run:
//!   TEST_API_BASE_URL=http://localhost:8000 cargo test -p integration-tests --test api
//!
//! If TEST_API_BASE_URL is not set the tests are skipped automatically.

use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the base URL for the API under test, or None if not configured.
fn api_base() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("TEST_API_BASE_URL").ok()
}

/// Build a reqwest client with a short timeout.
fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build reqwest client")
}

/// Seed an API key via the admin endpoint (itself auth-protected).
/// We bootstrap by using an existing key from TEST_API_KEY env var,
/// or create a fresh one using direct DB access seeded by the test environment.
fn test_api_key() -> Option<String> {
    std::env::var("TEST_API_KEY").ok()
}

/// Skip helper — prints a message and returns without failing.
macro_rules! skip_without_server {
    ($base:expr) => {
        match $base {
            Some(b) => b,
            None => {
                eprintln!("SKIP: TEST_API_BASE_URL not set — skipping integration test");
                return;
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Health / readiness
// ---------------------------------------------------------------------------

/// AC: The /health endpoint returns 200 with `{"status":"ok"}`.
#[tokio::test]
async fn test_health_returns_ok() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("GET /health failed");

    assert_eq!(
        resp.status().as_u16(),
        200,
        "health check should return 200"
    );
    let body: Value = resp.json().await.expect("health body is not JSON");
    assert_eq!(body["status"], "ok", "health status should be 'ok'");
}

/// AC: The /ready endpoint returns 200 when DB is reachable.
#[tokio::test]
async fn test_ready_returns_ready() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/ready"))
        .send()
        .await
        .expect("GET /ready failed");

    assert_eq!(resp.status().as_u16(), 200, "ready check should return 200");
    let body: Value = resp.json().await.expect("ready body is not JSON");
    // The API returns {"status":"ok"} from both /health and /ready
    assert!(
        body["status"].is_string(),
        "ready response should have a status field"
    );
}

// ---------------------------------------------------------------------------
// Auth — AC-F7-2: Source mutation requires API key
// ---------------------------------------------------------------------------

/// Unauthenticated POST /api/v1/sources must return 401.
#[tokio::test]
async fn test_create_source_without_auth_is_401() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .post(format!("{base}/api/v1/sources"))
        .json(&json!({
            "name": "Auth Test Source",
            "jurisdiction": "DE",
            "language": "de",
            "base_url": "https://example.com",
            "crawler_type": "bundesrecht",
            "cron_schedule": "0 0 * * *"
        }))
        .send()
        .await
        .expect("POST /api/v1/sources failed");

    assert_eq!(
        resp.status().as_u16(),
        401,
        "unauthenticated mutation should be 401"
    );
}

/// Unauthenticated GET /api/v1/admin/stats must return 401.
#[tokio::test]
async fn test_admin_stats_without_auth_is_401() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .send()
        .await
        .expect("GET /api/v1/admin/stats failed");

    assert_eq!(
        resp.status().as_u16(),
        401,
        "unauthenticated admin access should be 401"
    );
}

/// Invalid Bearer token must return 401.
#[tokio::test]
async fn test_invalid_bearer_token_is_401() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .header("Authorization", "Bearer invalid-token-xyz")
        .send()
        .await
        .expect("GET /api/v1/admin/stats with bad token failed");

    assert_eq!(
        resp.status().as_u16(),
        401,
        "invalid bearer token should be 401"
    );
}

/// Valid Bearer token must allow access to admin endpoints.
/// Requires TEST_API_KEY env var.
#[tokio::test]
async fn test_valid_bearer_token_allows_admin_access() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set — skipping authenticated test");
            return;
        }
    };

    let resp = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .expect("GET /api/v1/admin/stats with valid token failed");

    assert_eq!(
        resp.status().as_u16(),
        200,
        "valid bearer token should allow admin access"
    );
    let body: Value = resp.json().await.expect("stats body is not JSON");
    assert!(
        body["document_count"].is_number(),
        "stats should include document_count"
    );
    assert!(
        body["source_count"].is_number(),
        "stats should include source_count"
    );
}

// ---------------------------------------------------------------------------
// Sources — AC-F7-1: CRUD endpoints for sources
// ---------------------------------------------------------------------------

/// GET /api/v1/sources returns an array (public, no auth).
#[tokio::test]
async fn test_list_sources_is_public() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/sources"))
        .send()
        .await
        .expect("GET /api/v1/sources failed");

    assert_eq!(resp.status().as_u16(), 200, "list sources should be public");
    let body: Value = resp.json().await.expect("sources body is not JSON");
    assert!(body.is_array(), "sources response should be an array");
}

/// Full source CRUD cycle using an authenticated client.
#[tokio::test]
async fn test_source_crud_cycle() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set — skipping source CRUD test");
            return;
        }
    };

    let unique_name = format!("Integration Test Source {}", Uuid::new_v4());

    // CREATE
    let create_resp = client()
        .post(format!("{base}/api/v1/sources"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({
            "name": unique_name,
            "jurisdiction": "DE",
            "language": "de",
            "base_url": "https://example.com/test",
            "crawler_type": "bundesrecht",
            "cron_schedule": "0 2 * * *"
        }))
        .send()
        .await
        .expect("POST /api/v1/sources failed");

    assert_eq!(
        create_resp.status().as_u16(),
        201,
        "creating a source should return 201"
    );
    let created: Value = create_resp.json().await.expect("create body is not JSON");
    let source_id = created["id"]
        .as_str()
        .expect("created source should have an id");
    assert_eq!(created["name"], unique_name, "source name should match");
    assert_eq!(created["jurisdiction"], "DE");

    // READ
    let get_resp = client()
        .get(format!("{base}/api/v1/sources/{source_id}"))
        .send()
        .await
        .expect("GET /api/v1/sources/:id failed");

    assert_eq!(
        get_resp.status().as_u16(),
        200,
        "GET source should return 200"
    );
    let fetched: Value = get_resp.json().await.expect("get body is not JSON");
    assert_eq!(
        fetched["id"], created["id"],
        "fetched id should match created"
    );

    // UPDATE — disable the source
    let patch_resp = client()
        .patch(format!("{base}/api/v1/sources/{source_id}"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .expect("PATCH /api/v1/sources/:id failed");

    assert_eq!(
        patch_resp.status().as_u16(),
        200,
        "PATCH source should return 200"
    );
    let patched: Value = patch_resp.json().await.expect("patch body is not JSON");
    assert_eq!(
        patched["enabled"], false,
        "source should be disabled after PATCH"
    );
}

/// AC-F7-3: Disabling a source — verified above via PATCH enabled=false.

// ---------------------------------------------------------------------------
// Search — AC-F1-x
// ---------------------------------------------------------------------------

/// GET /api/v1/search is publicly accessible (no auth).
/// AC-F1-3: Public search endpoint.
#[tokio::test]
async fn test_search_is_public() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/search?q=test"))
        .send()
        .await
        .expect("GET /api/v1/search failed");

    // Should be 200 or 422 (empty DB is fine), but never 401 or 500.
    let status = resp.status().as_u16();
    assert!(
        status == 200 || status == 422,
        "search should be public (got {status})"
    );
}

/// AC-F1-4: Search result includes title, url, score, snippet fields.
#[tokio::test]
async fn test_search_result_shape() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/search?q=Gesetz&limit=1"))
        .send()
        .await
        .expect("search request failed");

    if resp.status().as_u16() != 200 {
        eprintln!(
            "SKIP: search returned {} — no data in DB yet",
            resp.status()
        );
        return;
    }

    let body: Value = resp.json().await.expect("search body is not JSON");
    let results = body["results"].as_array().expect("results should be array");

    if results.is_empty() {
        eprintln!("SKIP: search returned 0 results — no indexed documents yet");
        return;
    }

    let first = &results[0];
    assert!(first["title"].is_string(), "result must have title");
    assert!(first["url"].is_string(), "result must have url");
    assert!(first["score"].is_number(), "result must have score");
    assert!(first["snippet"].is_string(), "result must have snippet");
    assert!(
        first["jurisdiction"].is_string(),
        "result must have jurisdiction"
    );
    assert!(first["doc_type"].is_string(), "result must have doc_type");
}

/// AC-F1-2: Jurisdiction filter returns only matching results.
#[tokio::test]
async fn test_search_jurisdiction_filter() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/search?q=law&jurisdiction=EU"))
        .send()
        .await
        .expect("search with filter failed");

    if resp.status().as_u16() != 200 {
        return;
    }
    let body: Value = resp.json().await.expect("body is not JSON");
    let results = body["results"].as_array().expect("results is array");
    for r in results {
        assert_eq!(
            r["jurisdiction"], "EU",
            "all results should have jurisdiction=EU"
        );
    }
}

/// Empty query string returns 422 Unprocessable Entity.
#[tokio::test]
async fn test_search_empty_query_returns_422() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/search?q="))
        .send()
        .await
        .expect("empty query request failed");

    assert_eq!(resp.status().as_u16(), 422, "empty query should return 422");
}

// ---------------------------------------------------------------------------
// Document endpoints
// ---------------------------------------------------------------------------

/// GET /api/v1/documents/:id for an unknown UUID returns 404.
#[tokio::test]
async fn test_get_unknown_document_returns_404() {
    let base = skip_without_server!(api_base());
    let fake_id = Uuid::new_v4();
    let resp = client()
        .get(format!("{base}/api/v1/documents/{fake_id}"))
        .send()
        .await
        .expect("GET unknown document failed");

    assert_eq!(
        resp.status().as_u16(),
        404,
        "unknown document should be 404"
    );
    let body: Value = resp.json().await.expect("404 body is not JSON");
    // RFC 7807 Problem Details
    assert!(body["title"].is_string(), "404 should have RFC 7807 title");
    assert_eq!(body["status"], 404);
}

/// GET /api/v1/documents/:id/versions for an unknown UUID returns 200 with empty array.
/// (The endpoint returns all versions for a document ID; if none exist, returns [])
#[tokio::test]
async fn test_get_unknown_document_versions_returns_empty_array() {
    let base = skip_without_server!(api_base());
    let fake_id = Uuid::new_v4();
    let resp = client()
        .get(format!("{base}/api/v1/documents/{fake_id}/versions"))
        .send()
        .await
        .expect("GET unknown document versions failed");

    // Returns 200 with empty array — versions are scoped to document_id FK,
    // no versions exist for an unknown document so the result is [].
    assert_eq!(
        resp.status().as_u16(),
        200,
        "versions for unknown document returns 200 empty"
    );
    let body: Value = resp.json().await.expect("versions body not JSON");
    assert!(body.is_array(), "versions response should be an array");
}

// ---------------------------------------------------------------------------
// Changes — AC-F2-x
// ---------------------------------------------------------------------------

/// GET /api/v1/changes is public and returns an array.
/// AC-F2-2: /changes returns versions ordered by changed_at.
#[tokio::test]
async fn test_changes_endpoint_is_public_and_ordered() {
    let base = skip_without_server!(api_base());
    // `since` is a required query param (ISO-8601). Use a far-past date to get all changes.
    let resp = client()
        .get(format!(
            "{base}/api/v1/changes?since=2000-01-01T00:00:00Z&limit=5"
        ))
        .send()
        .await
        .expect("GET /api/v1/changes failed");

    assert_eq!(
        resp.status().as_u16(),
        200,
        "changes should be publicly accessible"
    );
    let body: Value = resp.json().await.expect("changes body is not JSON");

    // Response is either an array or an object with a `results` key
    let items = if body.is_array() {
        body.as_array().cloned().unwrap_or_default()
    } else {
        body["results"].as_array().cloned().unwrap_or_default()
    };

    // If there are items, verify ordering: changed_at should be descending.
    if items.len() >= 2 {
        let ts0 = items[0]["changed_at"].as_str().unwrap_or("");
        let ts1 = items[1]["changed_at"].as_str().unwrap_or("");
        assert!(
            ts0 >= ts1,
            "changes should be ordered by changed_at DESC: {ts0} >= {ts1}"
        );
    }
}

// ---------------------------------------------------------------------------
// Manual crawl trigger — POST /api/v1/sources/:id/crawl
// ---------------------------------------------------------------------------

/// Trigger crawl for unknown source returns 404.
#[tokio::test]
async fn test_trigger_crawl_unknown_source_returns_404() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set");
            return;
        }
    };

    let fake_id = Uuid::new_v4();
    let resp = client()
        .post(format!("{base}/api/v1/sources/{fake_id}/crawl"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .expect("POST trigger crawl failed");

    assert_eq!(
        resp.status().as_u16(),
        404,
        "trigger crawl on unknown source should be 404"
    );
}

/// Trigger crawl for valid source returns 202 Accepted.
#[tokio::test]
async fn test_trigger_crawl_valid_source_returns_202() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set");
            return;
        }
    };

    // First get a real source id
    let sources_resp = client()
        .get(format!("{base}/api/v1/sources"))
        .send()
        .await
        .expect("list sources failed");

    let sources: Value = sources_resp.json().await.expect("sources body not JSON");
    let sources_arr = sources.as_array().expect("sources is array");

    if sources_arr.is_empty() {
        eprintln!("SKIP: no sources configured — skipping trigger crawl test");
        return;
    }

    let source_id = sources_arr[0]["id"].as_str().expect("source has id");

    let resp = client()
        .post(format!("{base}/api/v1/sources/{source_id}/crawl"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .expect("POST trigger crawl failed");

    assert_eq!(
        resp.status().as_u16(),
        202,
        "trigger crawl should return 202 Accepted"
    );
    let body: Value = resp.json().await.expect("crawl response not JSON");
    assert!(
        body["source_id"].is_string(),
        "response should include source_id"
    );
    assert!(
        body["message"].is_string(),
        "response should include message"
    );
}

// ---------------------------------------------------------------------------
// Admin CRUD — API keys
// ---------------------------------------------------------------------------

/// Full API key lifecycle: create → list → revoke.
#[tokio::test]
async fn test_api_key_lifecycle() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set — skipping api key lifecycle test");
            return;
        }
    };

    let auth = format!("Bearer {key}");

    // CREATE a new API key
    let create_resp = client()
        .post(format!("{base}/api/v1/admin/api-keys"))
        .header("Authorization", &auth)
        .json(&json!({ "name": format!("test-key-{}", Uuid::new_v4()) }))
        .send()
        .await
        .expect("create api key failed");

    assert_eq!(
        create_resp.status().as_u16(),
        201,
        "create API key should return 201"
    );
    let created: Value = create_resp.json().await.expect("create key body not JSON");
    let new_key_id = created["id"].as_str().expect("created key has id");
    let new_raw_key = created["key"].as_str().expect("created key has raw key");
    assert_eq!(new_raw_key.len(), 64, "raw key should be 64 hex chars");

    // LIST keys — new key should appear
    let list_resp = client()
        .get(format!("{base}/api/v1/admin/api-keys"))
        .header("Authorization", &auth)
        .send()
        .await
        .expect("list api keys failed");

    assert_eq!(list_resp.status().as_u16(), 200);
    let keys: Value = list_resp.json().await.expect("keys list not JSON");
    let keys_arr = keys.as_array().expect("keys is array");
    let found = keys_arr
        .iter()
        .any(|k| k["id"].as_str() == Some(new_key_id));
    assert!(found, "newly created key should appear in the list");

    // Keys list should NOT include raw key value
    for k in keys_arr {
        assert!(
            k["key"].is_null()
                || !k
                    .get("key")
                    .is_some_and(|v| v.as_str().is_some_and(|s| s.len() == 64)),
            "listed keys must not include plaintext raw key"
        );
    }

    // REVOKE the new key
    let revoke_resp = client()
        .delete(format!("{base}/api/v1/admin/api-keys/{new_key_id}"))
        .header("Authorization", &auth)
        .send()
        .await
        .expect("revoke api key failed");

    assert_eq!(
        revoke_resp.status().as_u16(),
        204,
        "revoke API key should return 204"
    );

    // Verify revoked key no longer works
    let revoked_check = client()
        .get(format!("{base}/api/v1/admin/stats"))
        .header("Authorization", format!("Bearer {new_raw_key}"))
        .send()
        .await
        .expect("check with revoked key failed");

    assert_eq!(
        revoked_check.status().as_u16(),
        401,
        "revoked key should be rejected with 401"
    );
}

// ---------------------------------------------------------------------------
// 404 fallback
// ---------------------------------------------------------------------------

/// Unknown routes return 404.
#[tokio::test]
async fn test_unknown_route_returns_404() {
    let base = skip_without_server!(api_base());
    let resp = client()
        .get(format!("{base}/api/v1/does-not-exist"))
        .send()
        .await
        .expect("unknown route request failed");

    assert_eq!(
        resp.status().as_u16(),
        404,
        "unknown route should return 404"
    );
}

// ---------------------------------------------------------------------------
// E2E: Create source → trigger crawl → verify crawl_run — T8.1
// ---------------------------------------------------------------------------

/// End-to-end test: create a source, trigger a manual crawl, and verify
/// that the system accepts the crawl request (202 Accepted).
///
/// Note: In a test environment without real crawlers or embedders, we cannot
/// verify that documents are actually ingested and searchable. This test
/// validates the pipeline trigger path: source creation → crawl trigger →
/// accepted by the API. The crawl itself will fail gracefully in the background
/// because there is no real crawler for the test source.
#[tokio::test]
async fn test_e2e_create_source_trigger_crawl() {
    let base = skip_without_server!(api_base());
    let key = match test_api_key() {
        Some(k) => k,
        None => {
            eprintln!("SKIP: TEST_API_KEY not set — skipping E2E test");
            return;
        }
    };

    let auth = format!("Bearer {key}");
    let unique_name = format!("E2E Test Source {}", Uuid::new_v4());

    // Step 1: Create a new source
    let create_resp = client()
        .post(format!("{base}/api/v1/sources"))
        .header("Authorization", &auth)
        .json(&json!({
            "name": unique_name,
            "jurisdiction": "DE",
            "language": "de",
            "base_url": "https://example.com/e2e-test",
            "crawler_type": "bundesrecht",
            "cron_schedule": "0 0 1 1 *"
        }))
        .send()
        .await
        .expect("POST /api/v1/sources failed");

    assert_eq!(
        create_resp.status().as_u16(),
        201,
        "source creation should return 201"
    );
    let created: Value = create_resp.json().await.expect("create body not JSON");
    let source_id = created["id"]
        .as_str()
        .expect("created source should have id");

    // Step 2: Verify source is readable
    let get_resp = client()
        .get(format!("{base}/api/v1/sources/{source_id}"))
        .send()
        .await
        .expect("GET source failed");

    assert_eq!(get_resp.status().as_u16(), 200);
    let source: Value = get_resp.json().await.expect("source body not JSON");
    assert_eq!(source["name"], unique_name);
    assert_eq!(source["jurisdiction"], "DE");

    // Step 3: Trigger manual crawl (requires auth)
    let crawl_resp = client()
        .post(format!("{base}/api/v1/sources/{source_id}/crawl"))
        .header("Authorization", &auth)
        .send()
        .await
        .expect("POST trigger crawl failed");

    assert_eq!(
        crawl_resp.status().as_u16(),
        202,
        "trigger crawl should return 202 Accepted"
    );
    let crawl_body: Value = crawl_resp.json().await.expect("crawl response not JSON");
    assert_eq!(
        crawl_body["source_id"], source_id,
        "crawl response should reference the source"
    );

    // Step 4: Verify the crawl trigger without auth is rejected (401)
    let unauth_crawl = client()
        .post(format!("{base}/api/v1/sources/{source_id}/crawl"))
        .send()
        .await
        .expect("unauthenticated crawl request failed");

    assert_eq!(
        unauth_crawl.status().as_u16(),
        401,
        "crawl trigger without auth should be 401"
    );

    // Step 5: Verify upload without auth is rejected (401)
    let unauth_upload = client()
        .post(format!("{base}/api/v1/documents/upload"))
        .send()
        .await
        .expect("unauthenticated upload request failed");

    assert_eq!(
        unauth_upload.status().as_u16(),
        401,
        "document upload without auth should be 401"
    );

    // Step 6: Clean up — delete the test source
    let delete_resp = client()
        .delete(format!("{base}/api/v1/sources/{source_id}"))
        .header("Authorization", &auth)
        .send()
        .await
        .expect("DELETE source failed");

    // Accept either 204 (no content) or 200 for successful deletion
    let delete_status = delete_resp.status().as_u16();
    assert!(
        delete_status == 200 || delete_status == 204,
        "delete source should return 200 or 204, got {delete_status}"
    );

    // Step 7: Verify source is gone
    let verify_resp = client()
        .get(format!("{base}/api/v1/sources/{source_id}"))
        .send()
        .await
        .expect("GET deleted source failed");

    assert_eq!(
        verify_resp.status().as_u16(),
        404,
        "deleted source should return 404"
    );
}
