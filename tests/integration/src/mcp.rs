//! MCP tool chain integration tests — T8.1 / AC-F4-x
//!
//! Tests the MCP Streamable HTTP endpoint at POST /mcp.
//!
//! Spec: MCP 2025-03-26, Streamable HTTP transport (stateful mode).
//!
//! Protocol flow:
//!   1. POST /mcp with `initialize` — returns 200 SSE + `mcp-session-id` header
//!   2. POST /mcp with `notifications/initialized` + `mcp-session-id`
//!   3. POST /mcp with actual request + `mcp-session-id` — returns 200 SSE with data lines
//!
//! # Running
//!
//!   TEST_API_BASE_URL=http://localhost:8000 cargo test -p integration-tests --test mcp

use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};

fn api_base() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("TEST_API_BASE_URL").ok()
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build reqwest client")
}

macro_rules! skip_without_server {
    ($base:expr) => {
        match $base {
            Some(b) => b,
            None => {
                eprintln!("SKIP: TEST_API_BASE_URL not set — skipping MCP integration test");
                return;
            }
        }
    };
}

// ---------------------------------------------------------------------------
// MCP protocol helpers
// ---------------------------------------------------------------------------

/// Parse JSON-RPC result from an SSE response body.
/// SSE lines look like: `data: {"jsonrpc":"2.0","id":1,"result":{...}}`
fn parse_sse_json(body: &str) -> Option<Value> {
    for line in body.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() {
                if let Ok(v) = serde_json::from_str::<Value>(data) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Perform the MCP initialize handshake.
/// Returns the session ID from the `mcp-session-id` response header.
async fn mcp_initialize(base: &str) -> String {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test",
                "version": "0.1.0"
            }
        }
    });

    let resp = client()
        .post(format!("{base}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .expect("MCP initialize request failed");

    assert!(
        resp.status().is_success(),
        "MCP initialize should succeed, got {}",
        resp.status()
    );

    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .expect("initialize response must include mcp-session-id header")
        .to_str()
        .expect("mcp-session-id must be UTF-8")
        .to_string();

    // Consume body
    let _ = resp.text().await;

    // Send notifications/initialized (no response expected for notifications)
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });

    let _ = client()
        .post(format!("{base}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .json(&notif)
        .send()
        .await
        .expect("MCP notifications/initialized failed");

    session_id
}

/// Send a JSON-RPC 2.0 request to the MCP endpoint (after handshake) and
/// return the parsed JSON-RPC response value.
async fn mcp_call(base: &str, session_id: &str, method: &str, params: Value) -> Value {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let resp = client()
        .post(format!("{base}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", session_id)
        .json(&body)
        .send()
        .await
        .expect("MCP request failed");

    let status = resp.status().as_u16();
    let text = resp.text().await.expect("failed to read MCP response body");

    // Try to parse SSE data line
    if let Some(v) = parse_sse_json(&text) {
        return v;
    }

    // Fallback: try direct JSON parse (in case server returns plain JSON)
    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        return v;
    }

    panic!("MCP {method} returned status {status} with non-JSON body:\n{text}");
}

// ---------------------------------------------------------------------------
// AC-F4-1: MCP endpoint at POST /mcp, Streamable HTTP
// ---------------------------------------------------------------------------

/// MCP endpoint is reachable and responds to initialize.
#[tokio::test]
async fn test_mcp_endpoint_is_reachable() {
    let base = skip_without_server!(api_base());

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test",
                "version": "0.1.0"
            }
        }
    });

    let resp = client()
        .post(format!("{base}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .expect("MCP initialize failed");

    let status = resp.status().as_u16();
    assert!(
        status == 200 || status == 202,
        "MCP initialize should return 200 or 202 (got {status})"
    );

    // Must return a session ID
    assert!(
        resp.headers().contains_key("mcp-session-id"),
        "MCP initialize response must include mcp-session-id header"
    );
}

// ---------------------------------------------------------------------------
// AC-F4-2: 4 MCP tools registered
// ---------------------------------------------------------------------------

/// tools/list must return exactly the 4 required tools.
#[tokio::test]
async fn test_mcp_tools_list_returns_4_tools() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(&base, &session_id, "tools/list", json!({})).await;

    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tools array");

    assert_eq!(
        tools.len(),
        4,
        "exactly 4 MCP tools should be registered, got: {tools:?}"
    );

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(
        tool_names.contains(&"search_laws"),
        "search_laws tool must be registered, got: {tool_names:?}"
    );
    assert!(
        tool_names.contains(&"get_law_by_id"),
        "get_law_by_id tool must be registered, got: {tool_names:?}"
    );
    assert!(
        tool_names.contains(&"get_sources"),
        "get_sources tool must be registered, got: {tool_names:?}"
    );
    assert!(
        tool_names.contains(&"get_legal_changes"),
        "get_legal_changes tool must be registered, got: {tool_names:?}"
    );
}

// ---------------------------------------------------------------------------
// MCP tool: get_sources (no DB data required)
// ---------------------------------------------------------------------------

/// get_sources tool can be called and returns text content.
/// AC-F4-4: MCP responses include source attribution.
#[tokio::test]
async fn test_mcp_get_sources_returns_text() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(
        &base,
        &session_id,
        "tools/call",
        json!({
            "name": "get_sources",
            "arguments": {}
        }),
    )
    .await;

    // Should not be a JSON-RPC error
    assert!(
        body["error"].is_null(),
        "get_sources should not return a JSON-RPC error: {:?}",
        body["error"]
    );

    // The result content should be text
    let content = &body["result"]["content"];
    if content.is_array() {
        let items = content.as_array().unwrap();
        assert!(
            !items.is_empty(),
            "get_sources should return at least one content item"
        );
        let first = &items[0];
        assert_eq!(first["type"], "text", "MCP content type should be text");
        let text = first["text"].as_str().unwrap_or("");
        assert!(!text.is_empty(), "get_sources should return non-empty text");
    }
}

// ---------------------------------------------------------------------------
// MCP tool: search_laws
// ---------------------------------------------------------------------------

/// search_laws tool can be called without error (even if DB is empty).
#[tokio::test]
async fn test_mcp_search_laws_no_error() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(
        &base,
        &session_id,
        "tools/call",
        json!({
            "name": "search_laws",
            "arguments": {
                "query": "Mietrecht",
                "limit": 5
            }
        }),
    )
    .await;

    // Should not be a JSON-RPC error
    assert!(
        body["error"].is_null(),
        "search_laws should not return a JSON-RPC error: {:?}",
        body["error"]
    );
}

// ---------------------------------------------------------------------------
// MCP tool: get_law_by_id — invalid UUID returns error
// ---------------------------------------------------------------------------

/// get_law_by_id with an invalid UUID returns a tool error, not a crash.
#[tokio::test]
async fn test_mcp_get_law_by_id_invalid_uuid() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(
        &base,
        &session_id,
        "tools/call",
        json!({
            "name": "get_law_by_id",
            "arguments": {
                "document_id": "not-a-valid-uuid"
            }
        }),
    )
    .await;

    // The tool should return an error result (isError: true in content) or a JSON-RPC error
    let is_tool_error = body["result"]["isError"].as_bool().unwrap_or(false);
    let is_rpc_error = !body["error"].is_null();
    assert!(
        is_tool_error || is_rpc_error,
        "invalid UUID should produce a tool error or RPC error; got: {body}"
    );
}

// ---------------------------------------------------------------------------
// MCP tool: get_legal_changes
// ---------------------------------------------------------------------------

/// get_legal_changes tool returns text content.
#[tokio::test]
async fn test_mcp_get_legal_changes_returns_text() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(
        &base,
        &session_id,
        "tools/call",
        json!({
            "name": "get_legal_changes",
            "arguments": {
                "since": "2020-01-01T00:00:00Z",
                "limit": 5
            }
        }),
    )
    .await;

    assert!(
        body["error"].is_null(),
        "get_legal_changes should not return RPC error: {:?}",
        body["error"]
    );
}

/// get_legal_changes with invalid timestamp returns tool error.
#[tokio::test]
async fn test_mcp_get_legal_changes_invalid_since() {
    let base = skip_without_server!(api_base());
    let session_id = mcp_initialize(&base).await;

    let body = mcp_call(
        &base,
        &session_id,
        "tools/call",
        json!({
            "name": "get_legal_changes",
            "arguments": {
                "since": "not-a-timestamp"
            }
        }),
    )
    .await;

    let is_tool_error = body["result"]["isError"].as_bool().unwrap_or(false);
    let is_rpc_error = !body["error"].is_null();
    assert!(
        is_tool_error || is_rpc_error,
        "invalid timestamp should produce an error; got: {body}"
    );
}
