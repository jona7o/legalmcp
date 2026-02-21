//! Health + readiness handlers.

use std::sync::Arc;

use axum::{extract::State, response::Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// `GET /health` — always returns 200 while the process is alive.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// `GET /ready` — returns 200 when the DB is reachable, 503 otherwise.
pub async fn ready(State(state): State<Arc<AppState>>) -> Result<Json<Value>, Json<Value>> {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => Ok(Json(json!({ "status": "ok" }))),
        Err(e) => Err(Json(json!({ "status": "error", "detail": e.to_string() }))),
    }
}
