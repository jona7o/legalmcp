//! Changes handler — `GET /api/v1/changes`.
//!
//! Returns documents that have been updated after a given ISO-8601 timestamp.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use legal_core::errors::ProblemDetail;

use crate::{
    services::changes::{ChangedDocument, ChangesService},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ChangesQuery {
    /// ISO-8601 timestamp — only changes at or after this instant are returned.
    pub since: DateTime<Utc>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// `GET /api/v1/changes`
pub async fn list_changes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ChangesQuery>,
) -> Result<Json<Vec<ChangedDocument>>, (StatusCode, Json<ProblemDetail>)> {
    ChangesService::new(state.pool.clone())
        .list_changes(params.since, params.limit, params.offset)
        .await
        .map(Json)
        .map_err(|e| {
            let pd = e.to_problem_detail();
            let status =
                StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, Json(pd))
        })
}
